//! 关注、「我的」、确认与退订（DESIGN §3.6、§3.10、§4.10）。
//!
//! 这是边缘对控制面**仅有的同步写调用**（DESIGN §4.1）：玩家按下「告诉我」之后站在
//! 页面前等一句回话，所以这一条不像事件那样先落盘再批量送。控制面不在的时候如实说
//! 「现在登记不了，稍后再试」——不假装成功，也不给他一页红色的报错。
//!
//! 表单永远交给玩家自己所在的那个域：作品子域上是 [`edge_paths::FOLLOW`]，根域上是
//! [`root_paths::FOLLOW`]。玩家的浏览器从不把邮箱直接交给另一个域。
//!
//! **作品子域上不做浏览器通知。** Web Push 要先注册一个 Service Worker，而作品自己
//! 可能已经注册了一个（Godot / Unity 的 PWA 导出、Vite 的 PWA 插件都会装）。边缘往
//! 作品域上注册 SW 会顶掉它、还可能把旧文件缓存住——那是把别人的作品弄坏。所以子域的
//! 门禁页只给邮箱这一种，浏览器通知只在根域（广场、「我的」）上做，SW 是我们自己域上
//! 我们自己的那一个。

use std::time::Duration;

use axum::http::StatusCode;
use playtest_common::capabilities::Capabilities;
use playtest_common::follow::{
    form, looks_like_email, mask_email, root_paths, routes, ConfirmRequest, ConfirmResponse,
    FollowChannel, FollowRequest, FollowResponse, FollowTarget, MeRequest, MeView,
    PushSubscription, SendLinkRequest, UnfollowRequest, UnsubscribeRequest,
};

use crate::html::{esc, shell};
use crate::plaza::{self, Here};

/// 控制面必须在这么久之内答话。玩家在等，超过五秒不如直接说「稍后再试」。
const TIMEOUT: Duration = Duration::from_secs(5);

/// 表单里 `from` 的取值（进控制面的 `follows.source`，只用来看哪个入口有效）。
/// 只认这几个，别的当没有——它是玩家能改的字段。
pub const FROM_GATE: &str = "gate";
pub const FROM_PLAZA: &str = "plaza";
pub const FROM_ME: &str = "me";
pub const FROM_SDK: &str = "sdk";
const SOURCES: [&str; 4] = [FROM_GATE, FROM_PLAZA, FROM_ME, FROM_SDK];

/// 根域上我们自己的 Service Worker。只有它能弹通知，所以它必须在根域上，
/// 而且必须只有这么小——它会活得比任何一次访问都久。
pub const SW_PATH: &str = "/_playtest/sw.js";

/// 玩家提交的那张表单，解析之后。
#[derive(Debug)]
pub struct Submission {
    pub request: FollowRequest,
    /// 办完回哪去，同源相对路径。
    pub to: String,
    /// 只为了在结果页上打码回显，不往别处传。
    pub email: Option<String>,
}

/// 表单不合法时给玩家看的那一句。说人话，不说字段名。
#[derive(Debug)]
pub struct Invalid(pub &'static str);

/// 解析玩家交上来的表单。
///
/// `only_slug` 是作品子域上的那个 slug：子域上只能关注自己，别的一律不认——
/// 否则任何一个作品的页面都能借玩家的手去关注另一个作品。根域上传 `None`。
pub fn parse(
    raw: &str,
    only_slug: Option<&str>,
    me_token: Option<&str>,
) -> Result<Submission, Invalid> {
    let field = |key: &str| crate::app::field(raw, key).filter(|v| !v.trim().is_empty());

    let target = field(form::TARGET)
        .as_deref()
        .and_then(FollowTarget::parse)
        .ok_or(Invalid("这条链接不完整，回上一页再点一次。"))?;
    if let Some(slug) = only_slug {
        let mine = matches!(&target, FollowTarget::Site { slug: s } if s == slug);
        if !mine {
            return Err(Invalid("这条链接不完整，回上一页再点一次。"));
        }
    }

    let email = field(form::EMAIL).map(|e| e.trim().to_string());
    let push = field(form::PUSH);
    // 浏览器已经问过用户了，所以推送订阅优先：同一个人可能既有 `pt_me` 又刚点开通知。
    let channel = if let Some(raw) = push {
        let subscription: PushSubscription = serde_json::from_str(&raw)
            .map_err(|_| Invalid("这个浏览器的通知没开成，换邮箱试试。"))?;
        FollowChannel::Push { subscription }
    } else if let Some(token) = me_token {
        FollowChannel::Me {
            me_token: token.to_string(),
        }
    } else {
        let email = email.clone().ok_or(Invalid("留个邮箱吧，一行就够。"))?;
        if !looks_like_email(&email) {
            return Err(Invalid("这个邮箱看着不像能收信的，检查一下。"));
        }
        FollowChannel::Email { email }
    };

    let from = field(form::FROM).filter(|f| SOURCES.contains(&f.as_str()));
    let to = crate::app::same_origin_target(&field(form::TO).unwrap_or_default());
    Ok(Submission {
        request: FollowRequest {
            target,
            channel,
            from,
        },
        to,
        email,
    })
}

/// 控制面这一趟的结果。
pub enum Outcome {
    Answered(FollowResponse),
    /// 控制面不在、超时、或者答得不成形。玩家只会看到一句「现在登记不了」。
    Unavailable,
}

pub async fn register(api_base: Option<&str>, request: &FollowRequest) -> Outcome {
    match post(api_base, routes::FOLLOW, request).await {
        Call::Ok(response) => Outcome::Answered(response),
        _ => Outcome::Unavailable,
    }
}

pub async fn send_link(api_base: Option<&str>, email: &str) -> Outcome {
    let request = SendLinkRequest {
        email: email.to_string(),
    };
    match post(api_base, routes::ME_SEND_LINK, &request).await {
        Call::Ok(response) => Outcome::Answered(response),
        _ => Outcome::Unavailable,
    }
}

/// 确认信里那条链接。成功时拿到要种进 `pt_me` 的令牌。
pub async fn confirm(api_base: Option<&str>, token: &str) -> Option<ConfirmResponse> {
    let request = ConfirmRequest {
        token: token.to_string(),
    };
    match post(api_base, routes::CONFIRM, &request).await {
        Call::Ok(response) => Some(response),
        _ => None,
    }
}

/// 信底那个一键退订。点了就退，不问为什么。
pub async fn unsubscribe(api_base: Option<&str>, token: &str) -> bool {
    let request = UnsubscribeRequest {
        token: token.to_string(),
    };
    matches!(
        post::<_, MeView>(api_base, routes::UNSUBSCRIBE, &request).await,
        Call::Ok(_)
    )
}

/// 「我的」要显示的东西。
pub enum Mine {
    View(MeView),
    /// 这把钥匙控制面不认（退订过、或者根本是伪造的）：清掉 cookie，按没有处理。
    Stale,
    Unavailable,
}

pub async fn view(api_base: Option<&str>, me_token: &str) -> Mine {
    let request = MeRequest {
        me_token: me_token.to_string(),
    };
    into_mine(post(api_base, routes::ME_VIEW, &request).await)
}

pub async fn unfollow(api_base: Option<&str>, me_token: &str, target: FollowTarget) -> Mine {
    let request = UnfollowRequest {
        me_token: me_token.to_string(),
        target,
    };
    into_mine(post(api_base, routes::ME_UNFOLLOW, &request).await)
}

fn into_mine(call: Call<MeView>) -> Mine {
    match call {
        Call::Ok(view) => Mine::View(view),
        Call::Rejected(status) if status == 401 || status == 404 => Mine::Stale,
        _ => Mine::Unavailable,
    }
}

// ---------------------------------------------------------------- 给控制面打电话

enum Call<R> {
    Ok(R),
    /// 控制面在，但不接受这一条。
    Rejected(u16),
    Unavailable,
}

/// 内网上的一次 JSON POST。
///
/// 和 `ship.rs` 里那一个是两回事，所以没有合并：那边是后台批量、失败了下一轮再来、
/// 不关心响应体；这边是玩家在等，超时短、要解析回答、要区分「控制面说不」和「控制面不在」。
async fn post<B: serde::Serialize, R: serde::de::DeserializeOwned>(
    api_base: Option<&str>,
    path: &str,
    body: &B,
) -> Call<R> {
    // 没配控制面地址（本机一个人调试的常态）：如实说不行，不假装成功。
    let Some(api_base) = api_base else {
        return Call::Unavailable;
    };
    match try_post(api_base, path, body).await {
        Ok(call) => call,
        Err(err) => {
            // 玩家看到的是一句「现在登记不了」，运维要能在日志里看到到底怎么了。
            tracing::warn!(%err, path, "关注这一条没送到控制面");
            Call::Unavailable
        }
    }
}

async fn try_post<B: serde::Serialize, R: serde::de::DeserializeOwned>(
    api_base: &str,
    path: &str,
    body: &B,
) -> anyhow::Result<Call<R>> {
    let answer = crate::upstream::post_json(api_base, path, body, TIMEOUT).await?;
    if !answer.ok() {
        // 控制面明确说了不行（限速、参数不对、令牌过期）。这和「控制面不在」不一样：
        // 玩家该看到的话不同，重试有没有意义也不同。
        return Ok(Call::Rejected(answer.status));
    }
    match serde_json::from_slice::<R>(&answer.body) {
        Ok(parsed) => Ok(Call::Ok(parsed)),
        Err(err) => {
            tracing::warn!(%err, path, "控制面的回答看不懂");
            Ok(Call::Unavailable)
        }
    }
}

// ---------------------------------------------------------------- 玩家看到的页

/// 结果页：一句话加一个「返回」。这几句要经得起「第一次用的人读一遍就懂」。
pub fn result_page(outcome: &Outcome, sub: &Submission, back: &str) -> (StatusCode, String) {
    let plaza = matches!(sub.request.target, FollowTarget::Plaza);
    let (status, headline, detail) = match outcome {
        Outcome::Answered(FollowResponse::ConfirmSent) => {
            let masked = sub.email.as_deref().map(mask_email).unwrap_or_default();
            (
                StatusCode::OK,
                format!("确认信已发到 {masked}。"),
                "点里面的链接就算关注了；没收到看看垃圾箱。",
            )
        }
        Outcome::Answered(FollowResponse::Subscribed) if plaza => (
            StatusCode::OK,
            "好，广场有新东西时会通知你。".to_string(),
            "",
        ),
        Outcome::Answered(FollowResponse::Subscribed) => (
            StatusCode::OK,
            "好，这个作品有新版本时会通知你。".to_string(),
            "",
        ),
        Outcome::Answered(FollowResponse::AlreadyFollowing) => {
            (StatusCode::OK, "你已经关注着它了。".to_string(), "")
        }
        // 503 是给机器看的（页面上那个按钮据此知道自己没成），玩家看到的仍是一句人话。
        Outcome::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "现在登记不了，稍后再试。".to_string(),
            "",
        ),
    };
    (status, one_liner(&headline, detail, back))
}

/// 表单本身就不对。400，但页面上只有一句人话。
pub fn invalid_page(why: &Invalid, back: &str) -> String {
    one_liner(why.0, "", back)
}

/// 确认信里那条链接用过了、过期了、或者控制面此刻不在。三种情况对玩家是同一件事：
/// 这条链接现在不管用，再要一条就是了——所以不分开解释。
///
/// 但也不能替它编一个理由：说「已经用过了」而其实是我们这头没接上，就是撒谎。
/// 所以这句话只说眼下的事实。
pub fn confirm_failed_page(caps: &Capabilities) -> String {
    let again = if caps.email {
        format!(
            "<form method=\"post\" action=\"{action}\" class=\"row\">\
<input type=\"hidden\" name=\"action\" value=\"{send}\">\
<input type=\"email\" name=\"{email}\" required placeholder=\"你的邮箱\" autocomplete=\"email\" \
aria-label=\"你的邮箱\"><button type=\"submit\">再发一条</button></form>\n",
            action = root_paths::ME_ACTION,
            send = ACTION_SEND_LINK,
            email = form::EMAIL,
        )
    } else {
        String::new()
    };
    let body = format!(
        "<h1>这条链接现在不管用了。</h1>\n{again}\
<p class=\"meta\"><a href=\"/\">看看有什么新东西</a></p>\n"
    );
    shell("这条链接不管用了", "", &body)
}

/// 退订。一句话说完，不问为什么，也不放「再想想」——那是挽留，不是尊重。
pub fn unsubscribed_page(done: bool) -> String {
    if !done {
        return one_liner("现在退订不了，稍后再试。", "", "/");
    }
    let body = "<h1>已退订，不会再收到任何通知。</h1>\n\
<p class=\"meta\"><a href=\"/\">看看有什么新东西</a></p>\n";
    shell("已退订", "", body)
}

fn one_liner(headline: &str, detail: &str, back: &str) -> String {
    let detail = if detail.is_empty() {
        String::new()
    } else {
        format!("<p class=\"lead\">{}</p>\n", esc(detail))
    };
    let body = format!(
        "<h1>{}</h1>\n{detail}<p class=\"meta\"><a href=\"{}\">返回</a></p>\n",
        esc(headline),
        esc(back)
    );
    shell(headline, "", &body)
}

/// 「有新版本时告诉我」：一个原生表单，点开才展开，没有 JS 也能用。
/// `caps.email` 为假时返回空串——做不到的不显示、不解释（AGENTS 第 4 条）。
pub fn email_details(
    caps: &Capabilities,
    summary: &str,
    action: &str,
    target: &FollowTarget,
    to: &str,
    from: &str,
    button: &str,
) -> String {
    if !caps.email {
        return String::new();
    }
    format!(
        "<details class=\"tell\"><summary>{summary}</summary>\n\
<form method=\"post\" action=\"{action}\" class=\"row\">\n{hidden}\
<input type=\"email\" name=\"{email}\" required placeholder=\"你的邮箱\" autocomplete=\"email\" \
aria-label=\"你的邮箱\">\n<button type=\"submit\">{button}</button>\n</form>\n</details>",
        summary = esc(summary),
        action = esc(action),
        hidden = hidden_fields(target, to, from),
        email = form::EMAIL,
        button = esc(button),
    )
}

/// 已经有 `pt_me` 的人：关注是一下点击，不再要邮箱（DESIGN §3.6）。
pub fn one_click(
    action: &str,
    target: &FollowTarget,
    to: &str,
    from: &str,
    label: &str,
    class: &str,
) -> String {
    format!(
        "<form method=\"post\" action=\"{action}\" class=\"{class}\">\n{hidden}\
<button type=\"submit\">{label}</button>\n</form>",
        action = esc(action),
        class = esc(class),
        hidden = hidden_fields(target, to, from),
        label = esc(label),
    )
}

fn hidden_fields(target: &FollowTarget, to: &str, from: &str) -> String {
    format!(
        "<input type=\"hidden\" name=\"{t}\" value=\"{tv}\">\
<input type=\"hidden\" name=\"{f}\" value=\"{fv}\">\
<input type=\"hidden\" name=\"{o}\" value=\"{ov}\">\n",
        t = form::TARGET,
        tv = esc(&target.form_value()),
        f = form::FROM,
        fv = esc(from),
        o = form::TO,
        ov = esc(to),
    )
}

/// 「用浏览器通知」。只在根域上出现（模块头说明了为什么），且只在这个浏览器真的能用时
/// 才由脚本把它显出来——iOS 上不加到主屏幕就没有推送，与其显示一个点了没反应的按钮，
/// 不如不显示（DESIGN §3.6）。
pub fn push_button(caps: &Capabilities, target: &FollowTarget, already: bool) -> String {
    let Some(key) = &caps.push_public_key else {
        return String::new();
    };
    if already {
        return format!(
            "<form method=\"post\" action=\"{action}\" class=\"pushed\">\n\
<input type=\"hidden\" name=\"action\" value=\"{off}\">\n\
<span>浏览器通知：已开启</span><button type=\"submit\">关掉</button>\n</form>",
            action = root_paths::ME_ACTION,
            off = ACTION_PUSH_OFF,
        );
    }
    format!(
        "<button type=\"button\" id=\"pt-push\" class=\"ghost\" hidden data-key=\"{key}\" \
data-target=\"{target}\">用浏览器通知</button>",
        key = esc(key),
        target = esc(&target.form_value()),
    )
}

/// 上面那个按钮背后的脚本。根域的 CSP 只放行带 nonce 的脚本，调用方负责带上。
pub const PUSH_SCRIPT: &str = "(function(){\
var b=document.getElementById('pt-push');if(!b)return;\
if(!('serviceWorker'in navigator)||!('PushManager'in window)||!('Notification'in window))return;\
b.hidden=false;\
function key(s){var p=new Array((4-s.length%4)%4+1).join('=');\
var raw=atob((s+p).replace(/-/g,'+').replace(/_/g,'/'));var out=new Uint8Array(raw.length);\
for(var i=0;i<raw.length;i++){out[i]=raw.charCodeAt(i)}return out}\
b.onclick=function(){b.disabled=true;\
navigator.serviceWorker.register('/_playtest/sw.js').then(function(r){\
return r.pushManager.subscribe({userVisibleOnly:true,applicationServerKey:key(b.getAttribute('data-key'))})})\
.then(function(s){return fetch('/follow',{method:'POST',credentials:'same-origin',\
headers:{'content-type':'application/x-www-form-urlencoded'},\
body:'target='+encodeURIComponent(b.getAttribute('data-target'))+'&from=me&push='\
+encodeURIComponent(JSON.stringify(s.toJSON()))})})\
.then(function(r){if(r.ok){b.textContent='已开启'}else{b.disabled=false}},function(){b.disabled=false})};\
})();";

/// 根域上我们自己的 Service Worker：只做两件事——收到推送弹一条、点了打开那条链接。
/// 不缓存任何东西（它在根域上，根域只有广场和「我的」，都是服务端直出的）。
///
// TODO(contract): 推送负载的字段名（title / body / url）现在只写在这里和控制面的发信实现里，
// 应该进 common::follow 一个 `PushPayload` 类型，两边都从那里取。
pub const SW_JS: &str = "self.addEventListener('push',function(e){\
var d={};try{d=e.data?e.data.json():{}}catch(x){}\
e.waitUntil(self.registration.showNotification(d.title||'playtest',\
{body:d.body||'',data:{url:d.url||'/'},tag:d.tag||undefined}))});\
self.addEventListener('notificationclick',function(e){e.notification.close();\
var u=(e.notification.data&&e.notification.data.url)||'/';\
e.waitUntil(clients.matchAll({type:'window'}).then(function(list){\
for(var i=0;i<list.length;i++){if(list[i].url===u&&'focus'in list[i])return list[i].focus()}\
if(clients.openWindow)return clients.openWindow(u)}))});\n";

// ---------------------------------------------------------------- 「我的」

/// `/me/action` 表单里 `action` 的取值。
pub const ACTION_UNFOLLOW: &str = "unfollow";
pub const ACTION_SEND_LINK: &str = "send_link";
pub const ACTION_PUSH_OFF: &str = "push_off";

pub struct MePage<'a> {
    /// 有 `pt_me` 且控制面认了它，才有这个。
    pub view: Option<&'a MeView>,
    pub caps: &'a Capabilities,
    pub nonce: &'a str,
}

/// 「我的」（DESIGN §3.10）：一个抽屉，不是一个 profile。
/// 没有头像、没有昵称、没有历史记录，也没有「创建账号」这四个字。
pub fn me_page(page: &MePage<'_>) -> String {
    let mut body = String::from("<div class=\"drawer\"><h1>我的</h1>\n");
    match page.view {
        Some(view) => body.push_str(&signed_in(view, page.caps)),
        None => body.push_str(&signed_out(page.caps)),
    }
    // 没有 `pt_me` 的人也能开浏览器通知：推送订阅本身就是一个身份，不需要先有邮箱。
    let push = push_button(
        page.caps,
        &FollowTarget::Plaza,
        page.view.is_some_and(|v| v.push),
    );
    let push_script = if push.contains("id=\"pt-push\"") {
        format!(
            "<script nonce=\"{}\">{PUSH_SCRIPT}</script>\n",
            esc(page.nonce)
        )
    } else {
        String::new()
    };
    if !push.is_empty() {
        body.push_str(&format!("<div class=\"more\">{push}</div>\n"));
    }
    body.push_str(&format!("</div>\n{push_script}"));
    plaza::wrap("我的", "", Here::Mine, &body)
}

fn signed_in(view: &MeView, caps: &Capabilities) -> String {
    let mut out = String::new();
    match &view.email_masked {
        Some(masked) => out.push_str(&format!(
            "<p class=\"lead\">这台设备连着 {}。</p>\n",
            esc(masked)
        )),
        None => out.push_str("<p class=\"lead\">这台设备用浏览器通知收消息。</p>\n"),
    }

    if view.follows.is_empty() {
        out.push_str("<p class=\"summary\">还没有关注任何东西。在作品页上点「有新版本时告诉我」，它就会出现在这里。</p>\n");
    } else {
        out.push_str("<ul class=\"mine\">\n");
        for item in &view.follows {
            let link = match (&item.title, &item.url) {
                (Some(title), Some(url)) => {
                    format!("<a href=\"{}\">{}</a>", esc(url), esc(title))
                }
                _ => "<a href=\"/\">广场</a>".to_string(),
            };
            out.push_str(&format!(
                "<li>{link}\n<form method=\"post\" action=\"{action}\">\
<input type=\"hidden\" name=\"action\" value=\"{unfollow}\">\
<input type=\"hidden\" name=\"{target}\" value=\"{value}\">\
<button type=\"submit\">取消</button></form></li>\n",
                action = root_paths::ME_ACTION,
                unfollow = ACTION_UNFOLLOW,
                target = form::TARGET,
                value = esc(&item.target.form_value()),
            ));
        }
        out.push_str("</ul>\n");
    }

    // 换一台设备就是再点一次链接。没有密码、没有「登录」两个字（DESIGN §3.6）。
    if caps.email && view.email_masked.is_some() {
        out.push_str(&format!(
            "<div class=\"more\"><details class=\"tell\"><summary>换一台设备</summary>\n\
<form method=\"post\" action=\"{action}\" class=\"row\">\
<input type=\"hidden\" name=\"action\" value=\"{send}\">\
<input type=\"email\" name=\"{email}\" required placeholder=\"你的邮箱\" autocomplete=\"email\" \
aria-label=\"你的邮箱\">\
<button type=\"submit\">发一条链接给我</button></form>\n</details></div>\n",
            action = root_paths::ME_ACTION,
            send = ACTION_SEND_LINK,
            email = form::EMAIL,
        ));
    }
    out
}

/// 没有钥匙的人看到的：一句话说这里会放什么，一个邮箱输入。留下就是关注广场（DESIGN §3.10）。
fn signed_out(caps: &Capabilities) -> String {
    let mut out = String::from(
        "<p class=\"lead\">你关注的作品会出现在这里；留个邮箱，换一台设备也能找到它们。</p>\n",
    );
    if caps.email {
        out.push_str(&format!(
            "<form method=\"post\" action=\"{action}\" class=\"row\">\n{hidden}\
<input type=\"email\" name=\"{email}\" required placeholder=\"你的邮箱\" autocomplete=\"email\" \
aria-label=\"你的邮箱\"><button type=\"submit\">给我留个入口</button>\n</form>\n\
<p class=\"summary\">这也是关注广场：每周一一封，说说这周新出了什么。</p>\n",
            action = root_paths::FOLLOW,
            hidden = hidden_fields(&FollowTarget::Plaza, root_paths::ME, FROM_ME),
            email = form::EMAIL,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use playtest_common::follow::FollowView;

    fn caps() -> Capabilities {
        Capabilities {
            email: true,
            ..Default::default()
        }
    }

    fn site() -> FollowTarget {
        FollowTarget::Site {
            slug: "brisk-otter-41".into(),
        }
    }

    #[test]
    fn a_site_form_must_be_about_that_site() {
        let raw = "target=site%3Abrisk-otter-41&email=a%40b.co&from=gate&to=%2F";
        let sub = parse(raw, Some("brisk-otter-41"), None).unwrap();
        assert_eq!(sub.request.target, site());
        assert_eq!(sub.request.from.as_deref(), Some("gate"));
        assert_eq!(sub.to, "/");
        assert!(matches!(
            sub.request.channel,
            FollowChannel::Email { ref email } if email == "a@b.co"
        ));

        // 借玩家的手去关注别人：不认。
        assert!(parse(raw, Some("wise-mink-28"), None).is_err());
        // 子域上也不能关注广场。
        assert!(parse("target=plaza&email=a%40b.co", Some("brisk-otter-41"), None).is_err());
        // 根域上两种都行。
        assert!(parse("target=plaza&email=a%40b.co", None, None).is_ok());
    }

    #[test]
    fn a_key_in_the_cookie_makes_it_one_click() {
        let sub = parse(
            "target=plaza&from=plaza",
            None,
            Some("k".repeat(32).as_str()),
        )
        .unwrap();
        assert!(matches!(sub.request.channel, FollowChannel::Me { .. }));
        // 没有钥匙、也没有邮箱：说人话，不说「缺少字段」。
        let err = parse("target=plaza", None, None).unwrap_err();
        assert_eq!(err.0, "留个邮箱吧，一行就够。");
        let err = parse("target=plaza&email=nope", None, None).unwrap_err();
        assert!(err.0.contains("不像能收信"));
    }

    #[test]
    fn a_push_subscription_wins_and_must_be_the_real_shape() {
        let json = r#"{"endpoint":"https://push.example/x","keys":{"p256dh":"a","auth":"b"}}"#;
        let raw = format!(
            "target=plaza&push={}",
            percent_encoding::utf8_percent_encode(json, percent_encoding::NON_ALPHANUMERIC)
        );
        let sub = parse(&raw, None, Some("k")).unwrap();
        assert!(matches!(sub.request.channel, FollowChannel::Push { .. }));
        assert!(parse("target=plaza&push=%7B%7D", None, None).is_err());
    }

    #[test]
    fn the_source_and_the_return_path_are_both_checked() {
        // `from` 是玩家能改的字段，只认我们自己的那几个值。
        let sub = parse(
            "target=plaza&email=a%40b.co&from=%E9%BB%91%E5%AE%A2",
            None,
            None,
        )
        .unwrap();
        assert_eq!(sub.request.from, None);
        // 开放重定向：回到 `/`。
        let sub = parse(
            "target=plaza&email=a%40b.co&to=https%3A%2F%2Fevil.example",
            None,
            None,
        )
        .unwrap();
        assert_eq!(sub.to, "/");
    }

    #[test]
    fn every_answer_reads_like_a_person() {
        let sub = parse(
            "target=site%3Abrisk-otter-41&email=zhong%40example.com",
            None,
            None,
        )
        .unwrap();
        let (status, html) =
            result_page(&Outcome::Answered(FollowResponse::ConfirmSent), &sub, "/");
        assert_eq!(status, StatusCode::OK);
        assert!(html.contains("确认信已发到 z***@example.com。"));
        assert!(html.contains("没收到看看垃圾箱"));
        // 邮箱不原样回显在页面上。
        assert!(!html.contains("zhong@example.com"));

        let (_, html) = result_page(&Outcome::Answered(FollowResponse::Subscribed), &sub, "/");
        assert!(html.contains("好，这个作品有新版本时会通知你。"));
        let (_, html) = result_page(
            &Outcome::Answered(FollowResponse::AlreadyFollowing),
            &sub,
            "/",
        );
        assert!(html.contains("你已经关注着它了。"));

        let (status, html) = result_page(&Outcome::Unavailable, &sub, "/level/3");
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(html.contains("现在登记不了，稍后再试。"));
        assert!(html.contains("href=\"/level/3\""));
        // 不吓人：正文里没有一个内部词（样式在 head 里，不看）。
        let body = html.split("</head>").nth(1).unwrap();
        for word in ["控制面", "500", "错误", "失败", "超时"] {
            assert!(!body.contains(word), "「{word}」不是给玩家看的");
        }

        let plaza = parse("target=plaza&email=a%40b.co", None, None).unwrap();
        let (_, html) = result_page(&Outcome::Answered(FollowResponse::Subscribed), &plaza, "/");
        assert!(html.contains("好，广场有新东西时会通知你。"));
    }

    #[test]
    fn the_email_row_disappears_when_we_cannot_send_mail() {
        let nothing = Capabilities::default();
        assert!(email_details(
            &nothing,
            "有新版本时告诉我",
            "/_playtest/follow",
            &site(),
            "/",
            FROM_GATE,
            "告诉我"
        )
        .is_empty());

        let html = email_details(
            &caps(),
            "有新版本时告诉我",
            "/_playtest/follow",
            &site(),
            "/",
            FROM_GATE,
            "告诉我",
        );
        assert!(html.contains("<summary>有新版本时告诉我</summary>"));
        assert!(html.contains("action=\"/_playtest/follow\""));
        assert!(html.contains("value=\"site:brisk-otter-41\""));
        assert!(html.contains("type=\"email\""));
        // 原生表单：没有 JS 也能提交。
        assert!(!html.contains("<script"));
    }

    #[test]
    fn the_push_button_only_exists_when_there_is_a_key() {
        assert!(push_button(&caps(), &FollowTarget::Plaza, false).is_empty());
        let with_key = Capabilities {
            email: true,
            push_public_key: Some("BKey".into()),
            ..Default::default()
        };
        let html = push_button(&with_key, &FollowTarget::Plaza, false);
        // 默认藏着，由脚本在真的能用的浏览器上显出来。
        assert!(html.contains("hidden"));
        assert!(html.contains("data-key=\"BKey\""));
        let on = push_button(&with_key, &FollowTarget::Plaza, true);
        assert!(on.contains("已开启"));
        assert!(on.contains("push_off"));
    }

    #[test]
    fn me_without_a_key_is_a_local_drawer() {
        let html = me_page(&MePage {
            view: None,
            caps: &caps(),
            nonce: "n0nce",
        });
        assert!(html.contains("你关注的作品会出现在这里"));
        assert!(html.contains("action=\"/follow\""));
        assert!(html.contains("value=\"plaza\""));
        assert!(html.contains("aria-current=\"page\""));
        assert!(html.contains("我的<span class=\"nav-dot\"></span>"));
        assert!(html.contains("href=\"/\""));
        assert!(html.contains("广场</a>"));
        assert!(!html.contains("看看有什么新东西"));
        // 没有推送能力时这一页一行脚本都没有：邮箱是原生表单。
        assert!(!html.contains("<script"));
        // 「我的」不是个人主页：没有这些东西（DESIGN §3.10）。
        for word in [">登录<", ">注册<", "昵称", "创建账号"] {
            assert!(!html.contains(word), "「{word}」不该出现");
        }
    }

    #[test]
    fn me_with_a_key_lists_what_you_follow_and_how_to_stop() {
        let view = MeView {
            email_masked: Some("z***@example.com".into()),
            push: false,
            follows: vec![
                FollowView {
                    target: site(),
                    title: Some("小球大冒险".into()),
                    url: Some("http://brisk-otter-41.localhost:8443/".into()),
                    since: "2026-09-08T00:00:00Z".into(),
                },
                FollowView {
                    target: FollowTarget::Plaza,
                    title: None,
                    url: None,
                    since: "2026-09-08T00:00:00Z".into(),
                },
            ],
        };
        let html = me_page(&MePage {
            view: Some(&view),
            caps: &caps(),
            nonce: "n0nce",
        });
        assert!(html.contains("z***@example.com"));
        assert!(html.contains("小球大冒险"));
        assert!(html.contains(">广场</a>"));
        assert_eq!(html.matches(">取消</button>").count(), 2);
        assert!(html.contains("value=\"site:brisk-otter-41\""));
        assert!(html.contains("发一条链接给我"));
    }

    #[test]
    fn the_service_worker_only_shows_and_opens() {
        assert!(SW_JS.contains("showNotification"));
        assert!(SW_JS.contains("notificationclick"));
        // 不缓存任何东西：根域上只有服务端直出的两页。
        assert!(!SW_JS.contains("caches"));
        assert!(SW_JS.len() < 1024);
    }
}
