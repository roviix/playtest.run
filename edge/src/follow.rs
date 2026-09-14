//! 关注、关注页、确认与退订（DESIGN §3.6、§3.10、§4.10）。
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
//! 门禁页只给邮箱这一种，浏览器通知只在根域（广场、关注页）上做，SW 是我们自己域上
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
        .ok_or(Invalid("This link is incomplete, please go back and try again."))?;
    if let Some(slug) = only_slug {
        let mine = matches!(&target, FollowTarget::Site { slug: s } if s == slug);
        if !mine {
            return Err(Invalid("This link is incomplete, please go back and try again."));
        }
    }

    let email = field(form::EMAIL).map(|e| e.trim().to_string());
    let push = field(form::PUSH);
    // 浏览器已经问过用户了，所以推送订阅优先：同一个人可能既有 `pt_me` 又刚点开通知。
    let channel = if let Some(raw) = push {
        let subscription: PushSubscription = serde_json::from_str(&raw)
            .map_err(|_| Invalid("Browser notifications could not be enabled, try email instead."))?;
        FollowChannel::Push { subscription }
    } else if let Some(token) = me_token {
        FollowChannel::Me {
            me_token: token.to_string(),
        }
    } else {
        let email = email.clone().ok_or(Invalid("Please enter an email address."))?;
        if !looks_like_email(&email) {
            return Err(Invalid("Please check your email address."));
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

pub async fn preview(api_base: Option<&str>, token: &str) -> Option<String> {
    match post::<_,serde_json::Value>(api_base,routes::PREVIEW,&ConfirmRequest { token:token.to_string() }).await {
        Call::Ok(value) => value.get("email").and_then(|email|email.as_str()).map(str::to_string),
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

/// 关注页要显示的东西。
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

/// 关掉这台设备的浏览器通知。关注不动。
pub async fn push_off(api_base: Option<&str>, me_token: &str) -> Mine {
    let request = MeRequest {
        me_token: me_token.to_string(),
    };
    into_mine(post(api_base, routes::ME_PUSH_OFF, &request).await)
}

/// 门禁页即时提交一句话体验反馈。
pub async fn submit_feedback(api_base: Option<&str>, slug: &str, session: &str, text: &str) -> Result<(), u16> {
    let req = playtest_common::ingest::FeedbackRequest {
        session: session.to_string(),
        slug: slug.to_string(),
        text: text.to_string(),
        seconds_in: None,
        source: Some("gate".to_string()),
    };
    match post::<_, playtest_common::ingest::FeedbackAccepted>(
            api_base,
            playtest_common::ingest::routes::FEEDBACK,
            &req,
        )
        .await {
        Call::Ok(_) => Ok(()),
        Call::Rejected(status) => Err(status),
        Call::Unavailable => Err(503),
    }
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
    let collection = matches!(sub.request.target, FollowTarget::Collection { .. });
    let (status, headline, detail) = match outcome {
        Outcome::Answered(FollowResponse::ConfirmSent) => {
            let masked = sub.email.as_deref().map(mask_email).unwrap_or_default();
            (
                StatusCode::OK,
                format!("Confirmation email sent to {masked}. 确认信已发出。"),
                "Click the link in the email to confirm. Check your spam folder if it does not arrive.",
            )
        }
        Outcome::Answered(FollowResponse::Subscribed) if plaza => (
            StatusCode::OK,
            "You are all set! We will notify you when new projects arrive.".to_string(),
            "",
        ),
        Outcome::Answered(FollowResponse::Subscribed) if collection => (
            StatusCode::OK,
            "You are all set! We will send you a weekly digest of new submissions.".to_string(),
            "",
        ),
        Outcome::Answered(FollowResponse::Subscribed) => (
            StatusCode::OK,
            "You are all set! We will notify you when a new version is released.".to_string(),
            "",
        ),
        Outcome::Answered(FollowResponse::AlreadyFollowing) => {
            (StatusCode::OK, "You are already following this.".to_string(), "")
        }
        // 503 是给机器看的（页面上那个按钮据此知道自己没成），玩家看到的仍是一句人话。
        Outcome::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Unable to process right now. Please try again later.".to_string(),
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
<input type=\"email\" name=\"{email}\" required placeholder=\"Your email\" autocomplete=\"email\" \
aria-label=\"Your email\"><button type=\"submit\">Send another link</button></form>\n",
            action = root_paths::ME_ACTION,
            send = ACTION_SEND_LINK,
            email = form::EMAIL,
        )
    } else {
        String::new()
    };
    let body = format!(
        "<h1>This link is no longer valid.</h1>\n{again}\
<p class=\"meta\"><a href=\"/\">Explore projects</a></p>\n"
    );
    shell("Link expired", "", &body)
}

/// 退订。一句话说完，不问为什么，也不放「再想想」——那是挽留，不是尊重。
pub fn unsubscribed_page(done: bool) -> String {
    if !done {
        return one_liner("Unable to unsubscribe right now. Please try again later.", "", "/");
    }
    let body = "<h1>Unsubscribed. You will not receive any more notifications.</h1>\n\
<p class=\"meta\"><a href=\"/\">Explore projects</a></p>\n";
    shell("Unsubscribed", "", body)
}

fn one_liner(headline: &str, detail: &str, back: &str) -> String {
    let detail = if detail.is_empty() {
        String::new()
    } else {
        format!("<p class=\"lead\">{}</p>\n", esc(detail))
    };
    let body = format!(
        "<h1>{}</h1>\n{detail}<p class=\"meta\"><a href=\"{}\">Back</a></p>\n",
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
<input type=\"email\" name=\"{email}\" required placeholder=\"Your email\" autocomplete=\"email\" \
aria-label=\"Your email\">\n<button type=\"submit\">{button}</button>\n</form>\n</details>",
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
/// `row` 为真时它在关注页那张列表里，左边已经有「浏览器通知」这个标了，
/// 自己就只说状态与动作；为假时它自己站着，得把话说全。
pub fn push_button(caps: &Capabilities, target: &FollowTarget, already: bool, row: bool) -> String {
    let Some(key) = &caps.push_public_key else {
        return String::new();
    };
    if already {
        let state = if row {
            "Enabled"
        } else {
            "Browser notifications: Enabled"
        };
        return format!(
            "<form method=\"post\" action=\"{action}\" class=\"pushed\">\n\
<input type=\"hidden\" name=\"action\" value=\"{off}\">\n\
<span>{state}</span><button type=\"submit\">Disable</button>\n</form>",
            action = root_paths::ME_ACTION,
            off = ACTION_PUSH_OFF,
        );
    }
    format!(
        "<button type=\"button\" id=\"pt-push\" class=\"ghost\" hidden data-key=\"{key}\" \
data-target=\"{target}\">{label}</button>",
        key = esc(key),
        target = esc(&target.form_value()),
        label = if row { "Enable" } else { "Browser notifications" },
    )
}

/// 上面那个按钮背后的脚本。根域的 CSP 只放行带 nonce 的脚本，调用方负责带上。
pub const PUSH_SCRIPT: &str = "(function(){\
var b=document.getElementById('pt-push');if(!b)return;\
if(!('serviceWorker'in navigator)||!('PushManager'in window)||!('Notification'in window))return;\
b.hidden=false;if(b.parentNode)b.parentNode.hidden=false;var u=document.getElementById('pt-push-unavailable');if(u)u.hidden=true;var status=document.createElement('p');status.className='notice-note';status.setAttribute('role','status');b.parentNode.after(status);\
function key(s){var p=new Array((4-s.length%4)%4+1).join('=');\
var raw=atob((s+p).replace(/-/g,'+').replace(/_/g,'/'));var out=new Uint8Array(raw.length);\
for(var i=0;i<raw.length;i++){out[i]=raw.charCodeAt(i)}return out}\
b.onclick=function(){b.disabled=true;status.textContent='Enabling notifications…';\
navigator.serviceWorker.register('/_playtest/sw.js').then(function(r){\
return r.pushManager.subscribe({userVisibleOnly:true,applicationServerKey:key(b.getAttribute('data-key'))})})\
.then(function(s){return fetch('/follow',{method:'POST',credentials:'same-origin',\
headers:{'content-type':'application/x-www-form-urlencoded'},\
body:'target='+encodeURIComponent(b.getAttribute('data-target'))+'&from=me&push='\
+encodeURIComponent(JSON.stringify(s.toJSON()))})})\
.then(function(r){if(r.ok){location.assign('/me')}else{b.disabled=false;status.textContent='Unable to enable notifications right now. Please try again later.'}},function(){b.disabled=false;status.textContent='Notifications not enabled. Please check browser permissions.'})};\
})();";

/// 根域上我们自己的 Service Worker：只做两件事——收到推送弹一条、点了打开那条链接。
/// 不缓存任何东西（它在根域上，根域只有广场和关注页，都是服务端直出的）。
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

// ---------------------------------------------------------------- 关注页

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

/// 关注列表和通知设置分开呈现；邮箱只在用户主动打开对话框后填写。
pub fn me_page(page: &MePage<'_>) -> String {
    let mut body = String::from("<div class=\"drawer\">");
    body.push_str(&match page.view {
        Some(view) => signed_in(view, page.caps),
        None => signed_out(page.caps),
    });
    let push_script = if body.contains("id=\"pt-push\"") {
        format!(
            "<script nonce=\"{}\">{PUSH_SCRIPT}</script>\n",
            esc(page.nonce)
        )
    } else {
        String::new()
    };
    body.push_str(&format!("</div>\n{push_script}"));
    plaza::wrap("Following", "", Here::Mine, &body)
}

/// 同一份紧凑对话框用于找回、设置和自愿关注；锚点保留无脚本路径。
pub fn notification_dialog(id: &str, title: &str, content: &str) -> String {
    format!(
        "<dialog id=\"{id}\" class=\"account-dialog\" aria-labelledby=\"{id}-title\">\
<div class=\"notice-head\"><h2 id=\"{id}-title\">{title}</h2>\
<a href=\"#\" class=\"dialog-close-btn\" data-close-dialog aria-label=\"Close\">×</a></div>\
{content}</dialog>",
        id = esc(id),
        title = esc(title),
    )
}

/// 关注、周报与已有邮箱登录共用输入组件；hidden 只由本站生成。
pub fn email_form(action: &str, hidden: &str, label: &str, note: &str) -> String {
    email_form_with_placeholder(action, hidden, label, note, "name@example.com")
}

pub fn email_form_with_placeholder(
    action: &str,
    hidden: &str,
    label: &str,
    note: &str,
    placeholder: &str,
) -> String {
    let note_p = if note.is_empty() {
        String::new()
    } else {
        format!("<p class=\"notice-note\">{}</p>", esc(note))
    };
    format!(
        "<form method=\"post\" action=\"{}\" class=\"notice-form\">{hidden}<input type=\"email\" name=\"email\" required aria-label=\"Email\" placeholder=\"{}\" autocomplete=\"email\"><button type=\"submit\">{}</button>{note_p}</form>",
        esc(action),
        esc(placeholder),
        esc(label)
    )
}

const BELL: &str = "<svg class=\"icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9M10 21h4\"/></svg>";

fn heading(channel: Option<&str>) -> String {
    let channel = channel
        .map(|value| format!("<span class=\"follow-account\">{}</span>", esc(value)))
        .unwrap_or_default();
    format!("<header class=\"follow-head\"><div><h1>Following</h1>{channel}</div><a class=\"notification-bell\" href=\"#notification-settings\" data-dialog=\"notification-settings\" aria-label=\"Notification Settings\" title=\"Notification Settings\">{BELL}<span>Notification Settings</span></a></header>")
}

fn weekly_settings(view: Option<&MeView>, caps: &Capabilities) -> String {
    let subscribed = view.is_some_and(|view| {
        view.follows
            .iter()
            .any(|item| matches!(item.target, FollowTarget::Plaza))
    });
    let mut settings = String::new();

    if let Some(masked) = view.and_then(|view| view.email_masked.as_deref()) {
        // 已登录且有邮箱：显示紧凑身份条与一键开关通道
        let switch_btn = if caps.email {
            format!("<a class=\"notice-switch-link\" href=\"#follow-login\" data-dialog=\"follow-login\">Switch</a>")
        } else {
            String::new()
        };
        settings.push_str(&format!(
            "<div class=\"notice-account-bar\"><div class=\"notice-account-info\"><span class=\"notice-dot\" aria-hidden=\"true\"></span><span class=\"notice-account-label\">Email</span><span class=\"notice-email\">{}</span></div>{}</div>",
            esc(masked),
            switch_btn
        ));

        settings.push_str("<div class=\"notice-group\">");
        if subscribed {
            settings.push_str(&format!(
                "<div class=\"notice-channel subscribed\"><div class=\"notice-meta\"><span class=\"notice-title\">Weekly Digest</span><span class=\"notice-desc\">Curated projects & updates</span></div><div class=\"notice-actions\"><span class=\"notice-chip pos\"><span class=\"chip-dot\"></span>Subscribed</span><form method=\"post\" action=\"{}\"><input type=\"hidden\" name=\"action\" value=\"{ACTION_UNFOLLOW}\"><input type=\"hidden\" name=\"target\" value=\"plaza\"><button class=\"ghost action-btn\" type=\"submit\">Unsubscribe</button></form></div></div>",
                root_paths::ME_ACTION
            ));
        } else {
            settings.push_str(&format!(
                "<div class=\"notice-channel\"><div class=\"notice-meta\"><span class=\"notice-title\">Weekly Digest</span><span class=\"notice-desc\">Curated projects & updates</span></div>{}</div>",
                one_click(
                    root_paths::FOLLOW,
                    &FollowTarget::Plaza,
                    root_paths::ME,
                    FROM_ME,
                    "Subscribe",
                    "notice-subscribe"
                )
            ));
        }

        let pushed = view.is_some_and(|view| view.push);
        let push = push_button(caps, &FollowTarget::Plaza, pushed, true);
        if !push.is_empty() {
            settings.push_str(&format!(
                "<div class=\"notice-channel\"{}>\
                    <div class=\"notice-meta\"><span class=\"notice-title\">Desktop Push</span><span class=\"notice-desc\">{}</span></div>\
                    <div class=\"notice-actions\">{}{push}</div>\
                </div>",
                if pushed { "" } else { " hidden" },
                if pushed {
                    "System alerts when playtest is closed"
                } else {
                    "Receive Plaza digest via browser notifications"
                },
                if pushed { "<span class=\"notice-chip pos\"><span class=\"chip-dot\"></span>Active</span>" } else { "" }
            ));
        }
        settings.push_str("</div>");
        settings.push_str("<p class=\"notice-foot-note\">No spam. Unsubscribe anytime with one click.</p>");
    } else {
        // 未登录状态：彻底摆脱套娃！直接展示一体化行内订阅卡片与登录入口
        if caps.email {
            settings.push_str(&format!(
                "<div class=\"notice-subscribe-card\">\
                    <div class=\"notice-subscribe-head\">\
                        <span class=\"notice-title\">Weekly digest</span>\
                        <span class=\"notice-desc\">Curated projects and version updates delivered to your inbox every week.</span>\
                    </div>\
                    <form class=\"notice-subscribe-form\" method=\"post\" action=\"{}\">\
                        <input type=\"hidden\" name=\"target\" value=\"plaza\">\
                        <input type=\"hidden\" name=\"from\" value=\"me\">\
                        <input type=\"hidden\" name=\"to\" value=\"/me\">\
                        <div class=\"notice-input-bar\">\
                            <input type=\"email\" name=\"email\" required placeholder=\"your@email.com\" autocomplete=\"email\" aria-label=\"Email for weekly digest\">\
                            <button type=\"submit\" class=\"notice-subscribe-btn\">Subscribe</button>\
                        </div>\
                        <p class=\"notice-card-hint\">Single-use confirmation link. No password required.</p>\
                    </form>\
                </div>",
                root_paths::FOLLOW
            ));
        }

        let push = push_button(caps, &FollowTarget::Plaza, false, true);
        if !push.is_empty() {
            settings.push_str(&format!(
                "<div class=\"notice-channel\" hidden><div class=\"notice-meta\"><span class=\"notice-title\">Desktop Push</span><span class=\"notice-desc\">Receive Plaza digest via browser notifications</span></div><div class=\"notice-actions\">{push}</div></div>"
            ));
        }

        if !caps.email && caps.push_public_key.is_some() {
            settings.push_str(
                "<p id=\"pt-push-unavailable\" class=\"notice-note\">This browser does not support push notifications.</p>",
            );
        }

        settings.push_str("<div class=\"notice-footer-bar\"><span>Already have an account?</span><a href=\"/console/?login=1&amp;return_to=%2Fme\" data-account-login>Sign in →</a></div>");
    }

    if settings.is_empty() {
        settings.push_str("<p class=\"notice-note\">Notifications currently unavailable.</p>");
    }

    notification_dialog("notification-settings", "Notification Settings", &settings)
}

fn signed_in(view: &MeView, caps: &Capabilities) -> String {
    let channel = view.email_masked.as_deref().or(if view.push {
        Some("Browser notifications")
    } else {
        None
    });
    let mut out = heading(channel);
    let follows: Vec<_> = view
        .follows
        .iter()
        .filter(|item| !matches!(item.target, FollowTarget::Plaza))
        .collect();
    if follows.is_empty() {
        out.push_str("<div class=\"follow-empty\"><span class=\"follow-empty-icon\" aria-hidden=\"true\"><svg class=\"icon\" viewBox=\"0 0 24 24\"><path d=\"M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z\"/></svg></span><h2>No followed projects or collections yet</h2><p class=\"muted\">Follow projects on the Plaza to track new versions and updates.</p><a class=\"restore-follow\" href=\"/\">Explore Plaza</a></div>");
    } else {
        out.push_str("<ul class=\"mine\">");
        for item in follows {
            let (title, url, detail) = match &item.target {
                FollowTarget::Site { slug } => (
                    item.title.as_deref().unwrap_or(slug),
                    root_paths::project_path(slug),
                    "Project updates",
                ),
                FollowTarget::Collection { slug } => (
                    item.title.as_deref().unwrap_or(slug),
                    format!("/c/{slug}"),
                    "Collection updates",
                ),
                FollowTarget::Plaza => unreachable!(),
            };
            let initial = title.chars().next().unwrap_or('·').to_string();
            out.push_str(&format!("<li><a href=\"{}\"><span class=\"follow-art\" aria-hidden=\"true\">{}</span><span class=\"follow-name\">{}<small>{detail}</small></span></a><form method=\"post\" action=\"{action}\"><input type=\"hidden\" name=\"action\" value=\"{unfollow}\"><input type=\"hidden\" name=\"{target}\" value=\"{value}\"><button type=\"submit\" aria-label=\"Unfollow {label}\">Unfollow</button></form></li>", esc(&url), esc(&initial), esc(title), action=root_paths::ME_ACTION, unfollow=ACTION_UNFOLLOW, target=form::TARGET, value=esc(&item.target.form_value()), label=esc(title)));
        }
        out.push_str("</ul>");
    }
    out.push_str(&weekly_settings(Some(view), caps));
    out
}

fn signed_out(caps: &Capabilities) -> String {
    let mut out = heading(None);
    out.push_str("<div class=\"follow-empty\"><span class=\"follow-empty-icon\" aria-hidden=\"true\"><svg class=\"icon\" viewBox=\"0 0 24 24\"><path d=\"M6 4h12v17l-6-4-6 4Z\"/></svg></span><h2>Sign in to view followed projects</h2>");
    out.push_str("<a class=\"restore-follow\" href=\"/console/?login=1&amp;return_to=%2Fme\" data-account-login>Sign in</a>");
    out.push_str("</div>");
    out.push_str(&weekly_settings(None, caps));
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
        assert_eq!(err.0, "Please enter an email address.");
        let err = parse("target=plaza&email=nope", None, None).unwrap_err();
        assert!(err.0.contains("check your email"));
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
        assert!(html.contains("Confirmation email sent to z***@example.com."));
        assert!(html.contains("spam folder"));
        // 邮箱不原样回显在页面上。
        assert!(!html.contains("zhong@example.com"));

        let (_, html) = result_page(&Outcome::Answered(FollowResponse::Subscribed), &sub, "/");
        assert!(html.contains("You are all set! We will notify you when a new version is released."));
        let (_, html) = result_page(
            &Outcome::Answered(FollowResponse::AlreadyFollowing),
            &sub,
            "/",
        );
        assert!(html.contains("You are already following this."));

        let (status, html) = result_page(&Outcome::Unavailable, &sub, "/level/3");
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(html.contains("Unable to process right now. Please try again later."));
        assert!(html.contains("href=\"/level/3\""));
        // 不吓人：正文里没有一个内部词（样式在 head 里，不看）。
        let body = html.split("</head>").nth(1).unwrap();
        for word in ["控制面", "500", "错误", "失败", "超时"] {
            assert!(!body.contains(word), "「{word}」不是给玩家看的");
        }

        let plaza = parse("target=plaza&email=a%40b.co", None, None).unwrap();
        let (_, html) = result_page(&Outcome::Answered(FollowResponse::Subscribed), &plaza, "/");
        assert!(html.contains("You are all set! We will notify you when new projects arrive."));
    }

    #[test]
    fn the_email_row_disappears_when_we_cannot_send_mail() {
        let nothing = Capabilities::default();
        assert!(email_details(
            &nothing,
            "Notify me on new versions",
            "/_playtest/follow",
            &site(),
            "/",
            FROM_GATE,
            "Notify me"
        )
        .is_empty());

        let html = email_details(
            &caps(),
            "Notify me on new versions",
            "/_playtest/follow",
            &site(),
            "/",
            FROM_GATE,
            "Notify me",
        );
        assert!(html.contains("<summary>Notify me on new versions</summary>"));
        assert!(html.contains("action=\"/_playtest/follow\""));
        assert!(html.contains("value=\"site:brisk-otter-41\""));
        assert!(html.contains("type=\"email\""));
        // 原生表单：没有 JS 也能提交。
        assert!(!html.contains("<script"));
    }

    #[test]
    fn the_push_button_only_exists_when_there_is_a_key() {
        assert!(push_button(&caps(), &FollowTarget::Plaza, false, false).is_empty());
        let with_key = Capabilities {
            email: true,
            push_public_key: Some("BKey".into()),
            ..Default::default()
        };
        let html = push_button(&with_key, &FollowTarget::Plaza, false, false);
        // 默认藏着，由脚本在真的能用的浏览器上显出来。
        assert!(html.contains("hidden"));
        assert!(html.contains("data-key=\"BKey\""));
        let on = push_button(&with_key, &FollowTarget::Plaza, true, false);
        assert!(on.contains("Enabled"));
        assert!(on.contains("push_off"));
    }

    #[test]
    fn me_without_a_key_is_a_local_drawer() {
        let html = me_page(&MePage {
            view: None,
            caps: &caps(),
            nonce: "n0nce",
        });
        assert!(html.contains("<h1>Following</h1>"));
        assert!(html.contains("Weekly digest"));
        assert!(html.contains(">Continue with Email</button>"));
        assert!(html.contains("placeholder=\"your@email.com\""));
        let restore = html
            .split("id=\"account-login\"")
            .nth(1)
            .unwrap()
            .split("</dialog>")
            .next()
            .unwrap();
        assert!(restore.contains("data-account-email"));
        assert!(restore.contains("data-account-github"));
        assert!(!restore.contains("value=\"plaza\""));
        assert!(html.contains("Sign in to view followed projects"));
        let visible = html.split("<dialog").next().unwrap();
        assert!(!visible.contains("Subscribe to Digest"));
        assert!(!visible.contains("type=\"email\""));
        assert!(restore.contains("data-close-dialog"));
        assert!(!html.contains("follow-welcome"));
        assert!(!html.contains("给我留个入口"));
        assert!(html.contains("action=\"/follow\""));
        assert!(html.contains("value=\"plaza\""));
        assert!(html.contains("aria-current=\"page\""));
        assert!(html.contains("Following<span class=\"nav-dot\"></span>"));
        assert!(html.contains("href=\"/\""));
        assert!(html.contains("Plaza</a>"));
        assert!(!html.contains("看看有什么新东西"));
        // 没有推送能力时这一页一行脚本都没有：邮箱是原生表单。
        assert!(!html.contains("<script"));
        // 这一页不是个人主页：没有这些东西（DESIGN §3.10）。
        for word in [">注册<", "昵称", "创建账号"] {
            assert!(!html.contains(word), "「{word}」不该出现");
        }
    }

    #[test]
    fn weekly_channels_share_one_entry_without_hiding_push_only_access() {
        let mut channels = caps();
        channels.push_public_key = Some("BKey".into());
        let html = signed_out(&channels);
        assert!(!html.contains("class=\"follow-weekly\""));
        assert_eq!(html.matches("id=\"pt-push\"").count(), 1);
        let weekly = html
            .split("id=\"notification-settings\"")
            .nth(1)
            .unwrap()
            .split("</dialog>")
            .next()
            .unwrap();
        assert!(weekly.contains("id=\"pt-push\""));
        assert!(weekly.contains("type=\"email\""));
        assert!(weekly.contains("class=\"notice-subscribe-form\""));
        assert!(!weekly.contains("data-dialog=\"follow-login\""));
        assert!(weekly.contains("class=\"notice-channel\" hidden"));
        channels.email = false;
        let html = signed_out(&channels);
        assert!(!html.contains("class=\"follow-weekly\""));
        assert_eq!(html.matches("id=\"pt-push\"").count(), 1);
        assert!(!html.contains("type=\"email\""));
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
        assert!(html.contains(">Plaza</a>"));
        assert_eq!(html.matches(">Unfollow</button>").count(), 1);
        assert!(html.contains(">Unsubscribe</button>"));
        assert!(html.contains("value=\"site:brisk-otter-41\""));
        assert!(html.contains("data-account-login"));
        assert!(!html.contains("No followed projects"));
    }

    #[test]
    fn empty_follows_keep_discovery_in_the_sidebar() {
        let view = MeView {
            email_masked: Some("z***@example.com".into()),
            push: false,
            follows: vec![],
        };
        let html = me_page(&MePage {
            view: Some(&view),
            caps: &caps(),
            nonce: "n0nce",
        });
        assert!(html.contains("<h1>Following</h1>"));
        assert!(html.contains("No followed projects or collections yet"));
        assert!(!html.contains("去广场看看"));
        assert!(html.contains(">Plaza</a>"));
    }

    #[test]
    fn me_with_only_plaza_still_points_at_a_work() {
        let view = MeView {
            email_masked: Some("z***@example.com".into()),
            push: false,
            follows: vec![FollowView {
                target: FollowTarget::Plaza,
                title: None,
                url: None,
                since: "2026-09-08T00:00:00Z".into(),
            }],
        };
        let html = me_page(&MePage {
            view: Some(&view),
            caps: &caps(),
            nonce: "n0nce",
        });
        assert!(html.contains(">Plaza</a>"));
        assert!(html.contains("No followed projects"));
        assert!(html.contains("No followed projects or collections yet"));
        assert!(!html.contains("<ul class=\"mine\">"));
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
