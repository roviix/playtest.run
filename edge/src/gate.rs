//! 门禁页（DESIGN §3.3）。
//!
//! 它不是 ngrok 那种警告页，是产品的一部分：「开始」这一下是浏览器要的用户手势
//! （之后游戏的音频才能播）、是会话的起点、是版本的告示牌、是举报入口。
//! 所以它必须一秒内出现——整页内联，不引任何外部脚本或字体。
//!
//! 这一页上不出现开发者域名：玩家路径与开发者路径是两个域名（AGENTS 第 7 条）。

use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use playtest_common::capabilities::Capabilities;
use playtest_common::limits::MAX_PLAYER_NAME_CHARS;
use playtest_common::live::{SiteLive, PUBLIC_FEEDBACK_ON_GATE};
use playtest_common::manifest::{GateMode, Manifest, WorkKind};
use playtest_common::{
    CARD_WIDE_HEIGHT, CARD_WIDE_PATH, CARD_WIDE_WIDTH, RESERVED_PATH_PREFIX, SHARE_PATH,
};

use crate::html::{copy_row, esc, shell_hero, COPY_JS};
use crate::when;
use playtest_common::wording::{action_verb, audience_noun, invite_verb};

const PATH_ENCODE: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}');

/// 封面在作品自己的域上的路径（DESIGN §3.3）。
pub const COVER_PATH: &str = "/_playtest/cover";

const BELL_ICON: &str = "<svg width=\"16\" height=\"16\" viewBox=\"0 0 20 20\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.4\" stroke-linecap=\"round\" stroke-linejoin=\"round\" aria-hidden=\"true\"><path d=\"M4 13.5h12l-1.5-2.2V8a4.5 4.5 0 0 0-9 0v3.3L4 13.5ZM8 16a2.2 2.2 0 0 0 4 0\"/></svg>";
const SHARE_ICON: &str = "<svg width=\"16\" height=\"16\" viewBox=\"0 0 20 20\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.4\" stroke-linecap=\"round\" stroke-linejoin=\"round\" aria-hidden=\"true\"><path d=\"M10 12V2.5m-3.2 3.2L10 2.5l3.2 3.2M5.5 8H4v8.5h12V8h-1.5\"/></svg>";
const CLOCK_ICON: &str = "<svg width=\"13\" height=\"13\" viewBox=\"0 0 20 20\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.4\" stroke-linecap=\"round\" aria-hidden=\"true\"><circle cx=\"10\" cy=\"10\" r=\"7\"/><path d=\"M10 6v4l2.5 1.5\"/></svg>";
const PRESENTATION_CSS: &str = include_str!("../../ui/presentation.css");

fn public_file_url(origin: &str, path: &str) -> String {
    let encoded = path
        .split('/')
        .map(|part| utf8_percent_encode(part, PATH_ENCODE).to_string())
        .collect::<Vec<_>>()
        .join("/");
    format!("{}/{encoded}", origin.trim_end_matches('/'))
}

/// 要不要拦这一次请求。三条硬线里的两条在这里（DESIGN §3.3）：
///
/// 只拦顶层文档导航——js / wasm / 贴图永远直接出，游戏点过「开始」之后才开始加载，
/// 拦资源等于把加载器打断在半路。**子资源永不拿到「200 + HTML」**：浏览器不会报网络错，
/// 最好的结果是一个看不懂的语法错，最坏是白屏加沉默。在游戏里 200 比 404 坏得多。
///
/// 第三条硬线不在代码里而在代码的缺席里：**没有任何请求头能绕过它**。
/// 判据只有请求语义（`Sec-Fetch-Dest`、`Accept`、路径形状）和门禁自己种的 cookie，
/// 没有 `X-Playtest-No-Gate` 这种东西——需要它就说明门禁站错了位置。
pub fn should_show(
    gate: GateMode,
    path: &str,
    is_html: bool,
    navigation: bool,
    has_cookie: bool,
    from: Option<&str>,
) -> bool {
    if gate == GateMode::Never || !is_html || !navigation || looks_like_a_resource(path) {
        return false;
    }
    // 来自外部入口（广场点击、扫卡、通知邮件）或者首次访问（没 cookie）或者设置了 Always，均展示邀请函
    from.is_some() || gate == GateMode::Always || !has_cookie
}

/// `Sec-Fetch-Dest: document` 是现代浏览器的准确信号，**它在场就以它为准**：
/// 一个 `fetch('/x.html')` 会带 `Sec-Fetch-Dest: empty`，有的库还顺手写
/// `Accept: text/html, */*;q=0.01`——两个信号打架时信 Accept 就等于给子资源发门禁页。
/// `Accept` 只兜不发 `Sec-Fetch-Dest` 的老客户端（Safari 16.4 之前）。
pub fn is_navigation(sec_fetch_dest: Option<&str>, accept: Option<&str>) -> bool {
    if let Some(dest) = sec_fetch_dest {
        // `document` 是顶层文档。`iframe` / `frame` 是嵌套导航，门禁页在别人的框里
        // 既拿不到用户手势也不是「这条链接」的入口，同样不出。
        return dest.eq_ignore_ascii_case("document");
    }
    accept.is_some_and(|a| a.contains("text/html"))
}

/// 请求路径本身就长得像一个资源。
///
/// 这一道只为一种情况存在：开了 `spa` 的作品，任何找不到的路径都回退到 `index.html`，
/// 于是一个 `/assets/app-4f2c.js` 在老客户端上有可能既被当成导航、又解析出 HTML，
/// 拿到 200 + 门禁页。反面教材是 pinggy——对每个没带 cookie 的路径都回 200 + 15 KB HTML。
///
/// 用白名单不用「有扩展名就算」：SPA 的路由里 `/user/v1.2` 这种段是存在的，
/// 认错了只是少出一次门禁页，认漏了才是把游戏弄坏。
fn looks_like_a_resource(path: &str) -> bool {
    let file = path.rsplit('/').next().unwrap_or(path);
    let stripped = file
        .strip_suffix(".br")
        .or_else(|| file.strip_suffix(".gz"))
        .unwrap_or(file);
    let Some((_, ext)) = stripped.rsplit_once('.') else {
        return false;
    };
    let ext = ext.to_ascii_lowercase();
    RESOURCE_EXTENSIONS.contains(&ext.as_str())
}

/// 引擎导出物里真实出现过的扩展名。少了谁就补谁，宁可长一点。
const RESOURCE_EXTENSIONS: &[&str] = &[
    "js",
    "mjs",
    "cjs",
    "wasm",
    "css",
    "json",
    "map",
    "txt",
    "xml",
    "csv", //
    "data",
    "pck",
    "unityweb",
    "bin",
    "mem",
    "symbols",
    "framework",
    "loader", //
    "png",
    "jpg",
    "jpeg",
    "gif",
    "webp",
    "avif",
    "svg",
    "ico",
    "bmp",
    "ktx2",
    "basis", //
    "mp3",
    "ogg",
    "wav",
    "m4a",
    "flac",
    "opus",
    "mp4",
    "webm",
    "mov", //
    "ttf",
    "otf",
    "woff",
    "woff2",
    "eot",
    "atlas",
    "glb",
    "gltf",
    "obj",
    "fbx",
    "tres",
];

/// 「开始」表单里那几个字段的名字。app.rs 收表单时认同一份。
pub mod field {
    /// 点了开始之后回到哪。
    pub const TO: &str = "to";
    /// 玩家自愿留的名字（DESIGN §3.3 第 5 条）。
    pub const NAME: &str = "name";
    /// 链接上的 `?from=`，只有 `card` / `notice`（[`playtest_common::FROM_PARAM`]）。
    pub const FROM: &str = "from";
    /// 门禁页收到的 Referer，原样带过去。**和 `FROM` 是两回事**：这个是「上一页是谁」，
    /// 那个是「这条链接是从哪张卡、哪封通知发出去的」，后者可信得多（DESIGN §3.5）。
    pub const REFERER: &str = "ref";
}

pub struct GatePage<'a> {
    pub manifest: &'a Manifest,
    /// 会变的那些：名额、群、公开反馈、头像、在不在广场上（DESIGN §4.5 的 `live.json`）。
    pub live: &'a SiteLive,
    /// 控制面现在能做什么。决定「有新版本时告诉我」那一行有没有、长什么样。
    pub caps: &'a Capabilities,
    /// 点了开始之后回到哪，同源相对路径。
    pub to: &'a str,
    /// 泛域名后缀，角标上显示的就是它（本机是 `localhost`）。
    pub host_suffix: &'a str,
    /// 角标链到的根域。
    pub root_url: &'a str,
    /// 这一页自己的地址，给 og:url。
    pub page_url: &'a str,
    /// 这个作品的源（`scheme://slug.suffix[:port]`），封面与横版卡的绝对地址接在它后面。
    pub origin: &'a str,
    pub wechat: bool,
    /// 版本那个位置显示什么。上传路径是 `None`，显示 `v7`；隧道路径传「在线」——
    /// 隧道没有版本这个概念（DESIGN §3.5），显示合成清单里那个 `v0` 会让玩家
    /// 以为自己拿到了一个坏链接。
    pub version_label: Option<&'a str>,
    /// 玩家是从哪个页面点到这条链接的（门禁页请求的 Referer）。会话从「开始」那一下才算起，
    /// 而那一下的 Referer 是门禁页自己，所以真正的来源要在表单里带过去（DESIGN §3.5「来自哪里」）。
    pub referer: &'a str,
    /// 链接上的 `?from=`：扫卡来的是 `card`，通知里点进来的是 `notice`，广场来的是 `plaza`。
    pub from: Option<&'a str>,
    /// 共享的设备身份凭证（pt_me），用于一键免登录秒关注。
    pub me_token: Option<&'a str>,
    /// 是否已经关注了该作品。
    pub already_followed: bool,
    /// 是否在根域（`playtest.run/p/<slug>`）上渲染（DESIGN §3.1、§3.3）。
    pub is_root: bool,
    /// 根域上的 CSP nonce。
    pub nonce: Option<&'a str>,
    /// 文章在控制面提交时生成的安全 HTML。只有 `Article` 使用；原始 Markdown 不进根域。
    pub article_html: Option<&'a str>,
    /// 如果是从合集跳过来的，在邀请函顶上显示返回合集与下一件作品（DESIGN §3.1）。
    pub collection_context: Option<&'a str>,
}

impl GatePage<'_> {
    pub fn render(&self) -> String {
        let m = self.manifest;
        let title = esc(&m.title);
        let developer = esc(&m.developer);
        let version = match self.version_label {
            Some(label) => esc(label),
            None => format!("v{}", m.version),
        };
        let invite = invite_verb(m.kind, m.is_game());
        let verb = action_verb(m.kind, m.is_game());
        let summary = m
            .summary
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(esc);

        // 服务端直出分享元数据（DESIGN §3.3）：Discord、iMessage、Telegram 会来抓。
        // 微信对未备案域名的抓取没有任何承诺，这里是尽力而为，不是「支持微信卡片」（§5）。
        let description = match &summary {
            Some(summary) => format!("{summary} · {developer} {invite}"),
            None => format!("{developer} {invite}《{title}》· {version}"),
        };
        let mut head = format!(
            "<meta name=\"description\" content=\"{description}\">\n\
<meta property=\"og:title\" content=\"《{title}》· {version}\">\n\
<meta property=\"og:description\" content=\"{og_description}\">\n\
<meta property=\"og:type\" content=\"website\">\n\
<meta property=\"og:url\" content=\"{url}\">\n",
            og_description = summary
                .clone()
                .unwrap_or_else(|| format!("{developer} {invite}")),
            url = esc(self.page_url),
        );
        // 有封面就用封面；没有封面就用横版邀请卡——一张写着作品名和开发者名的卡不是
        // 假截图，它和玩家点开后看到的是同一个物件（DESIGN §3.3、§3.4）。
        let hue = crate::html::hue(&m.slug);
        head.push_str(&format!("<style>:root{{--h:{hue}}}</style>\n"));
        if self.collection_context.is_some() {
            head.push_str("<style>.collection-context{display:flex;justify-content:space-between;align-items:center;width:100%;max-width:400px;margin:0 0 10px;font-size:13px}.collection-context a{color:var(--soft);text-decoration:none;padding:6px 12px;border-radius:8px;background:#ffffff0a;border:1px solid #ffffff12;transition:all .15s}.collection-context a:hover{color:var(--fg);background:#ffffff14;border-color:#ffffff24}</style>\n");
        }
        if m.kind != WorkKind::Web {
            head.push_str(&format!("<style>{PRESENTATION_CSS}</style>\n"));
        }

        // 有封面就用封面；没有封面就用算法生成的专属几何星轨图，
        // 它和作品 slug 色相呼应，具有高辨识度与收藏级数字票证质感。
        let hero = match &m.cover {
            Some(cover) => {
                head.push_str(&format!(
                    "<meta property=\"og:image\" content=\"{origin}{COVER_PATH}\">\n\
<meta property=\"og:image:type\" content=\"{mime}\">\n\
<meta name=\"twitter:card\" content=\"summary_large_image\">\n",
                    origin = esc(self.origin),
                    mime = esc(&cover.mime),
                ));
                let src = if self.is_root {
                    format!("{}{COVER_PATH}", esc(self.origin))
                } else {
                    COVER_PATH.to_string()
                };
                format!("<img class=\"hero\" src=\"{src}\" alt=\"\">\n")
            }
            None => {
                head.push_str(&format!(
                    "<meta property=\"og:image\" content=\"{origin}{CARD_WIDE_PATH}\">\n\
<meta property=\"og:image:type\" content=\"image/png\">\n\
<meta property=\"og:image:width\" content=\"{CARD_WIDE_WIDTH}\">\n\
<meta property=\"og:image:height\" content=\"{CARD_WIDE_HEIGHT}\">\n\
<meta name=\"twitter:card\" content=\"summary_large_image\">\n",
                    origin = esc(self.origin),
                ));
                self.generative_art()
            }
        };
        let summary_html = match &summary {
            Some(summary) => format!("<p class=\"summary\">{summary}</p>\n"),
            None => String::new(),
        };

        // 版本与到期并排；较长的更新说明放在下一行，不把期限挤到卡片底部。
        let mut stamp = version.clone();
        if self.version_label.is_none() {
            if let Some(day) = when::day(&m.created_at) {
                stamp.push_str(&format!(" · {}", esc(&day)));
            }
        }
        let note = m
            .note
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(|note| format!("<p class=\"version-note\">{}</p>\n", esc(note)))
            .unwrap_or_default();

        // 微信 UA 只决定要不要多说一句「右上角 → 在浏览器中打开」——那是微信独有的操作，
        // 别的浏览器里说了没意义。**能不能玩不由 UA 判断**：X5 / XWeb 的版本和能力没有
        // 官方对照表，黑名单一定会误伤（DESIGN §5）。判断交给下面那段能力检测。
        let tips = match (self.wechat, m.kind) {
            (true, WorkKind::Web) => {
                "<section class=\"tip\">\n\
<p>在微信里可能玩不了：点右上角「···」，选「在浏览器中打开」。</p>\n\
</section>\n"
            }
            (true, WorkKind::Video) => {
                "<section class=\"tip\">\n\
<p>如果在微信里不能播放，点右上角「···」，选「在浏览器中打开」。</p>\n\
</section>\n"
            }
            _ => "",
        };

        let nonce_attr = match self.nonce {
            Some(n) => format!(" nonce=\"{n}\""),
            None => String::new(),
        };

        // 那段脚本只为把到期时间换成访客本地时区的写法，没有到期时间就不发。
        let expires = match m.expires_at.as_deref().and_then(when::deadline) {
            Some((machine, human)) => format!(
                "<p class=\"expires\">{clock}<span><time datetime=\"{}\">{}</time> 到期</span></p>\n<script{nonce_attr}>\
for(const t of document.querySelectorAll('.expires time[datetime]')){{\
const d=new Date(t.getAttribute('datetime'));\
if(!isNaN(d))t.textContent=d.toLocaleString(undefined,{{month:'numeric',day:'numeric',hour:'2-digit',minute:'2-digit'}});}}\
</script>\n",
                esc(&machine),
                esc(&human),
                clock = CLOCK_ICON,
            ),
            None => String::new(),
        };

        let badge = if m.badge && !self.is_root {
            format!(
                "<p class=\"provider\"><a href=\"{}\">由 {} 提供</a></p>",
                esc(self.root_url),
                esc(self.host_suffix)
            )
        } else {
            String::new()
        };

        let from = match self.from {
            Some(from) => format!(
                "<input type=\"hidden\" name=\"{}\" value=\"{}\">\n",
                field::FROM,
                esc(from)
            ),
            None => String::new(),
        };

        let (start_action, target_attr, btn_text) = if self.is_root {
            (
                esc(self.to),
                " target=\"_blank\" rel=\"noopener\"",
                format!("开始{verb}"),
            )
        } else {
            (
                format!("{RESERVED_PATH_PREFIX}start"),
                "",
                "开始".to_string(),
            )
        };

        let prefix = RESERVED_PATH_PREFIX;
        let report_href = if self.is_root {
            format!("{}{prefix}report", esc(self.origin))
        } else {
            format!("{prefix}report")
        };
        let mobile_script = if self.is_root {
            format!("<script{nonce_attr}>if(/Mobi|Android|iPhone/i.test(navigator.userAgent)){{document.querySelector('form.start')?.removeAttribute('target');}}</script>\n")
        } else {
            String::new()
        };

        let presentation = match m.kind {
            WorkKind::Web => format!(
                "<div class=\"stub\">\n\
<form class=\"start\" method=\"post\" action=\"{start_action}\"{target_attr}>\n\
<input type=\"hidden\" name=\"{to_field}\" value=\"{to}\">\n\
<input type=\"hidden\" name=\"{ref_field}\" value=\"{referer}\">\n{from}\
<label class=\"holder\"><span>你的名字 <small>· 可不填</small></span>\
<input type=\"text\" name=\"{name_field}\" maxlength=\"{max_name}\" \
placeholder=\"怎么称呼你？\" autocomplete=\"nickname\">\
</label>\n\
<button type=\"submit\">{btn_text}</button>\n\
</form>\n\
</div>\n",
                to_field = field::TO,
                to = esc(self.to),
                ref_field = field::REFERER,
                referer = esc(self.referer),
                name_field = field::NAME,
                max_name = MAX_PLAYER_NAME_CHARS,
            ),
            WorkKind::Article => format!(
                "<article class=\"article-body\">{}</article>\n",
                self.article_html
                    .unwrap_or("<p class=\"tip\">文章正文暂时取不到，请稍后再试。</p>")
            ),
            WorkKind::Video => {
                let src = m
                    .entry
                    .as_deref()
                    .map(|path| public_file_url(self.origin, path))
                    .unwrap_or_default();
                format!(
                    "<div class=\"video-body\"><video controls preload=\"metadata\" playsinline src=\"{}\">你的浏览器不能播放这个视频。</video></div>\n",
                    esc(&src)
                )
            }
        };

        let body = format!(
            "<div class=\"ticket-head\">\n\
<p class=\"by\">{avatar}{developer} {invite}</p>\n\
</div>\n\
<h1>{title}</h1>\n\
{summary_html}<div class=\"edition\"><p class=\"stamp\">{stamp}</p>{expires}</div>\n{note}\
{seats}{tips}{presentation}\
{capability}{voices}\
<footer>{tools}<a href=\"{report_href}\" class=\"report\">举报</a></footer>{badge}\n{mobile_script}",
            avatar = self.avatar(),
            seats = self.seats(),
            tools = self.tools(),
            capability = self.capability_note(),
            voices = self.voices(),
        );

        let hero = hero.replacen(
            '>',
            &format!(
                " style=\"view-transition-name:{}\">",
                crate::html::transition_name(&m.slug)
            ),
            1,
        );
        let rendered = shell_hero(
            &format!("{} {invite}《{}》", m.developer, m.title),
            &head,
            &hero,
            &body,
        );
        let mark = crate::plaza::icon("mark").replacen("<svg ", "<svg width=\"20\" height=\"20\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.5\" ", 1).replace("<circle ", "<circle fill=\"currentColor\" stroke=\"none\" ");
        let navigation = format!("<nav class=\"invitation-nav\" aria-label=\"返回广场\"><a href=\"{}\" aria-label=\"playtest.run · 返回广场\">{mark}{wordmark}</a></nav>", esc(self.root_url), wordmark = crate::html::WORDMARK);
        let card_class = if m.kind == WorkKind::Web {
            "card"
        } else {
            "card media-card"
        };
        let context_bar = self.collection_context.unwrap_or_default();
        let rendered = rendered.replacen(
            "<main class=\"card\">",
            &format!("{context_bar}{navigation}<main class=\"{card_class}\">"),
            1,
        );
        let dialog = self.follow_dialog();
        let script = if dialog.is_empty() {
            String::new()
        } else {
            format!(
                "<script{nonce_attr}>{}</script>",
                include_str!("../../ui/dialog.js")
            )
        };
        rendered.replacen("</body>", &format!("{dialog}{script}</body>"), 1)
    }

    /// 无封面时按作品 slug 色相算法生成的几何星轨艺术图案，作为邀请函头图（DESIGN §3.3）。
    fn generative_art(&self) -> String {
        let m = self.manifest;
        let initial = m
            .title
            .chars()
            .next()
            .map(|c| esc(&c.to_string()))
            .unwrap_or_else(|| "P".into());
        let slug = esc(&m.slug);
        let version_tag = match self.version_label {
            Some(label) => esc(label),
            None => format!("v{}", m.version),
        };
        format!(
            "<div class=\"hero word\"><svg viewBox=\"0 0 400 250\" fill=\"none\" aria-hidden=\"true\">\
<defs><radialGradient id=\"p-sky\" cx=\"50%\" cy=\"45%\" r=\"65%\">\
<stop offset=\"0%\" stop-color=\"#131e1a\"/>\
<stop offset=\"100%\" stop-color=\"#08090b\"/>\
</defs>\
<rect width=\"400\" height=\"250\" fill=\"url(#p-sky)\"/>\
<path d=\"M0 83h400M0 166h400M133 0v250M267 0v250\" stroke=\"#ffffff08\"/>\
<circle cx=\"200\" cy=\"118\" r=\"64\" stroke=\"#75cdb518\" stroke-dasharray=\"2 6\"/>\
<circle cx=\"200\" cy=\"118\" r=\"38\" stroke=\"#75cdb530\" stroke-dasharray=\"4 4\"/>\
<circle cx=\"200\" cy=\"118\" r=\"16\" stroke=\"#75cdb555\"/>\
<circle cx=\"200\" cy=\"118\" r=\"5\" fill=\"#71e8bf\"/>\
<circle cx=\"200\" cy=\"118\" r=\"2\" fill=\"#fff\"/>\
<text x=\"24\" y=\"32\" font-size=\"9\" font-weight=\"700\" letter-spacing=\"1.5\" fill=\"#d4d4d8\" font-family=\"sans-serif\">PLAYTEST PASS</text>\
<text x=\"376\" y=\"32\" text-anchor=\"end\" font-size=\"9.5\" font-family=\"monospace\" font-weight=\"600\" fill=\"#75cdb5\">{version_tag}</text>\
<text x=\"24\" y=\"228\" font-size=\"52\" font-weight=\"800\" fill=\"#ffffff1c\" font-family=\"sans-serif\">{initial}</text>\
<text x=\"376\" y=\"228\" text-anchor=\"end\" font-size=\"9.5\" font-family=\"monospace\" fill=\"#a1a1aa\">{slug}</text>\
</svg></div>\n"
        )
    }

    /// 关注与分享在同一工具行；首次填写邮箱只在自愿展开后出现。
    fn follow_section(&self) -> String {
        if !self.caps.email {
            return String::new();
        }
        let slug = esc(&self.manifest.slug);
        let action = if self.is_root {
            playtest_common::follow::root_paths::FOLLOW
        } else {
            "/_playtest/follow"
        };
        let to = esc(self.to);
        if self.already_followed {
            format!(
                "<form class=\"follow-box followed\" method=\"post\" action=\"{action}\">\n\
<input type=\"hidden\" name=\"action\" value=\"unfollow\">\n\
<input type=\"hidden\" name=\"target\" value=\"site:{slug}\">\n\
<input type=\"hidden\" name=\"to\" value=\"{to}\">\n\
<button type=\"submit\" class=\"btn-follow\" title=\"取消关注\" aria-label=\"已关注更新，点击取消关注\">\
{bell}<span>已关注</span></button>\n\
</form>\n",
                bell = BELL_ICON,
            )
        } else if self.me_token.is_some() {
            format!(
                "<form class=\"follow-box\" method=\"post\" action=\"{action}\">\n\
<input type=\"hidden\" name=\"target\" value=\"site:{slug}\">\n\
<input type=\"hidden\" name=\"from\" value=\"gate\">\n\
<input type=\"hidden\" name=\"to\" value=\"{to}\">\n\
<button type=\"submit\" class=\"btn-follow\">{bell}<span>关注更新</span></button>\n\
</form>\n",
                bell = BELL_ICON,
            )
        } else {
            format!("<a class=\"btn-follow\" href=\"#notification-settings\" data-dialog=\"notification-settings\">{BELL_ICON}<span>关注更新</span></a>")
        }
    }

    fn follow_dialog(&self) -> String {
        if !self.caps.email || self.me_token.is_some() || self.already_followed {
            return String::new();
        }
        let action = if self.is_root {
            playtest_common::follow::root_paths::FOLLOW
        } else {
            "/_playtest/follow"
        };
        let hidden = format!("<input type=\"hidden\" name=\"target\" value=\"site:{}\"><input type=\"hidden\" name=\"from\" value=\"gate\"><input type=\"hidden\" name=\"to\" value=\"{}\">", esc(&self.manifest.slug), esc(self.to));
        let content = crate::follow::email_form(
            action,
            &hidden,
            "关注更新",
            "点击确认信后接收新版本通知，随时可退订。开发者看不到你的邮箱。",
        );
        crate::follow::notification_dialog("notification-settings", "关注更新", &content)
    }

    /// 开发者的头像（DESIGN §3.3、§3.9：一张脸比一个 ID 更像真人）。只认 https；
    /// 没有配置或匿名发布时，由 ODD FOLK 确定性算法生成专属角色矢量头像，不留白。
    fn avatar(&self) -> String {
        match self
            .live
            .avatar_url
            .as_deref()
            .filter(|u| u.starts_with("https://"))
        {
            Some(url) => format!(
                "<img src=\"{}\" alt=\"\" width=\"32\" height=\"32\" \
referrerpolicy=\"no-referrer\" loading=\"lazy\">",
                esc(url)
            ),
            None => playtest_common::avatar::svg_for_creator(
                &self.manifest.developer,
                &self.manifest.slug,
                32,
            ),
        }
    }

    /// 名额那一行（DESIGN §3.3 第 4 条）。**加入 = 留了名字的人**，不是所有打开的人；
    /// 到齐之后不拦人，只如实说。
    fn seats(&self) -> String {
        let Some(seats) = self.live.seats.filter(|n| *n > 0) else {
            return String::new();
        };
        let developer = esc(&self.manifest.developer);
        if self.live.seats_full() {
            let verb = match (self.manifest.kind, self.manifest.is_game()) {
                (WorkKind::Web, true) => "玩",
                (WorkKind::Web, false) => "体验",
                (WorkKind::Article, _) => "阅读",
                (WorkKind::Video, _) => "观看",
            };
            return format!("<p class=\"seats full\">{seats} 位已到齐 · 你仍然可以{verb}</p>\n");
        }
        let joined = match self.live.joined {
            0 => String::new(),
            n => format!(" · 已有 {n} 位加入"),
        };
        let audience = audience_noun(self.manifest.kind, self.manifest.is_game());
        format!("<p class=\"seats\">{developer}在找 {seats} 位{audience}{joined}</p>\n")
    }

    /// 同一工具行的辅助动作；没有可用动作时不留空行。
    fn tools(&self) -> String {
        let follow = self.follow_section();
        let mut rows = Vec::new();
        if let Some(url) = self
            .live
            .community_url
            .as_deref()
            .filter(|u| u.starts_with("https://") || u.starts_with("http://"))
        {
            rows.push(format!(
                "<a class=\"community\" href=\"{}\" rel=\"noopener nofollow\">开发者的群</a>",
                esc(url)
            ));
        }
        if self.live.listed {
            let share_href = if self.is_root {
                format!("{}{SHARE_PATH}", esc(self.origin))
            } else {
                SHARE_PATH.to_string()
            };
            rows.push(format!("<a class=\"share\" href=\"{share_href}\" aria-label=\"分享作品\" title=\"分享作品\">{SHARE_ICON}</a>"));
        }
        if rows.is_empty() && follow.is_empty() {
            return String::new();
        }
        let more = if rows.is_empty() {
            String::new()
        } else {
            format!("<div class=\"more\">{}</div>", rows.join("\n"))
        };
        format!("<div class=\"invitation-tools\">{follow}{more}</div>\n")
    }

    /// 体验者的话（DESIGN §3.5）：社会证明，不是讨论区——没有回复、没有点赞、没有楼层。
    fn voices(&self) -> String {
        if !self.live.feedback_public {
            return String::new();
        }
        let mut items = String::new();
        for item in self
            .live
            .public_feedback
            .iter()
            .take(PUBLIC_FEEDBACK_ON_GATE)
        {
            let text = item.text.trim();
            if text.is_empty() {
                continue;
            }
            let who = item
                .name
                .as_deref()
                .map(str::trim)
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| {
                    match audience_noun(self.manifest.kind, self.manifest.is_game()) {
                        "读者" => "一位读者",
                        "观众" => "一位观众",
                        "试玩者" => "一位试玩者",
                        _ => "一位体验者",
                    }
                });
            items.push_str(&format!(
                "<p class=\"voice\">「{}」<cite>{} · v{}</cite></p>\n",
                esc(text),
                esc(who),
                item.version
            ));
        }
        if items.is_empty() {
            return String::new();
        }
        let audience = audience_noun(self.manifest.kind, self.manifest.is_game());
        format!("<section class=\"voices\">\n<h2>{audience}的话</h2>\n{items}</section>\n")
    }

    /// 跨源隔离的作品在能力不足的浏览器里跑不起来（Godot 4 的线程导出没有
    /// SharedArrayBuffer 就直接报错）。这里内联一小段检测，**不引任何外部脚本**。
    ///
    /// 三条约束：
    ///
    /// 1. 用 `crossOriginIsolated` 与 `SharedArrayBuffer` 这两个真实能力判断，不用 UA 黑名单
    ///    ——微信 Android 的 XWeb 版本与 Chromium 的对应关系没有官方资料（DESIGN §5）。
    /// 2. 这一段默认是藏起来的，**检测通过就一个字都不显示**；关掉 JS 也不显示——
    ///    与其吓唬一个本来能玩的人，不如让他直接点。
    /// 3. **「开始」按钮始终在、始终可点。** 微信《外部链接内容管理规范》§3.2.3 把
    ///    「微信内展示效果和其他浏览器实质性不一致」列为对抗行为；只给一句
    ///    「去浏览器打开」而不给可玩的路径，正好撞在上面（DESIGN §5）。
    ///    所以这里只是按钮下面多一行说明加一个复制链接，不拦路。
    fn capability_note(&self) -> String {
        if !self.manifest.isolated {
            return String::new();
        }
        let nonce_attr = match self.nonce {
            Some(n) => format!(" nonce=\"{n}\""),
            None => String::new(),
        };
        format!(
            "<section class=\"tip\" id=\"pt-cap\" hidden>\n\
<p>这个作品需要系统浏览器才跑得起来（它要用到当前浏览器没开放的能力）。\
上面的「开始」照样可以点；打不开的话，复制链接到 Safari、Chrome 里粘贴打开。</p>\n\
{row}</section>\n<script{nonce_attr}>{COPY_JS}{CAPABILITY_SCRIPT}</script>\n",
            row = copy_row(&esc(self.page_url), None),
        )
    }
}

/// 玩家自愿留的名字（DESIGN §3.3 第 5 条）。服务端这一道做三件事：去掉控制字符
/// （它们会把点名册那一行拆断）、trim、按字符数截断。**脏词表在控制面**（§4.8），
/// 边缘不判断内容——它不认识作品的语境，也不该替开发者做这个决定。
pub fn clean_name(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_PLAYER_NAME_CHARS)
        .collect();
    let trimmed = cleaned.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// 链接上的 `?from=`。只认我们自己放上去的那两个值（DESIGN §3.5）：
/// 别的一律当没有——它是任何人都能在 URL 上改的字段。
pub fn known_source(raw: Option<&str>) -> Option<&'static str> {
    match raw.map(str::trim) {
        Some(playtest_common::FROM_CARD) => Some(playtest_common::FROM_CARD),
        Some(playtest_common::FROM_NOTICE) => Some(playtest_common::FROM_NOTICE),
        Some(playtest_common::FROM_PLAZA) => Some(playtest_common::FROM_PLAZA),
        Some(playtest_common::FROM_COLLECTION) => Some(playtest_common::FROM_COLLECTION),
        _ => None,
    }
}

/// 能力检测。写成 ES5、不用可选链，因为要跑的正是那些老 WebView。
/// 检测通过（或这段没跑）时那一节始终是 `hidden`，玩家什么都不会看到。
const CAPABILITY_SCRIPT: &str = "\
(function(){var n=document.getElementById('pt-cap');\
if(self.crossOriginIsolated&&typeof SharedArrayBuffer==='function')return;\
n.hidden=false;\
ptCopy(document.getElementById('pt-url'),document.getElementById('pt-copy'))})();";

#[cfg(test)]
mod tests {
    use super::*;
    use playtest_common::manifest::SCHEMA;

    fn manifest() -> Manifest {
        Manifest {
            schema: SCHEMA,
            slug: "brisk-otter-41".into(),
            version: 7,
            title: "小球大冒险".into(),
            developer: "某某".into(),
            note: None,
            summary: None,
            cover: None,
            created_at: "2026-09-07T00:00:00Z".into(),
            expires_at: None,
            badge: true,
            gate: GateMode::Once,
            isolated: false,
            spa: false,
            engine: Some("godot".into()),
            kind: playtest_common::manifest::WorkKind::Web,
            entry: None,
            article: None,
            files: vec![],
        }
    }

    /// 这几个测试里绝大多数只关心清单，`live` 与 `caps` 用空的：没有名额、没有群、
    /// 不公开反馈、控制面什么都做不了——门禁页上对应的那几行一行都不出现。
    fn page<'a>(m: &'a Manifest, wechat: bool) -> GatePage<'a> {
        GatePage {
            manifest: m,
            live: &EMPTY_LIVE,
            caps: &NO_CAPS,
            to: "/",
            host_suffix: "localhost",
            root_url: "http://localhost:8443/",
            page_url: "http://brisk-otter-41.localhost:8443/",
            origin: "http://brisk-otter-41.localhost:8443",
            wechat,
            version_label: None,
            referer: "",
            from: None,
            me_token: None,
            already_followed: false,
            is_root: false,
            nonce: None,
            article_html: None,
            collection_context: None,
        }
    }

    static EMPTY_LIVE: std::sync::LazyLock<SiteLive> =
        std::sync::LazyLock::new(|| SiteLive::empty("brisk-otter-41"));
    static NO_CAPS: std::sync::LazyLock<Capabilities> =
        std::sync::LazyLock::new(Capabilities::default);
    static EMAIL_CAPS: std::sync::LazyLock<Capabilities> =
        std::sync::LazyLock::new(|| Capabilities {
            email: true,
            ..Capabilities::default()
        });

    #[test]
    fn a_cover_becomes_the_hero_and_the_share_image() {
        let mut m = manifest();
        m.cover = Some(playtest_common::manifest::Cover {
            hash: "a".repeat(64),
            size: 1000,
            mime: "image/png".into(),
        });
        m.summary = Some("一个关于小球的冒险，三关，五分钟。".into());
        let html = page(&m, false).render();
        // 封面顶在卡片最上面，从作品自己的域上取（DESIGN §3.3）。
        assert!(html.contains("<img class=\"hero\" src=\"/_playtest/cover\""));
        assert!(html.find("class=\"hero\"") < html.find("<h1>"));
        // 分享卡片拿到绝对地址的图。
        assert!(html.contains(
            "<meta property=\"og:image\" content=\"http://brisk-otter-41.localhost:8443/_playtest/cover\">"
        ));
        assert!(html.contains("<meta property=\"og:image:type\" content=\"image/png\">"));
        // 一句话介绍进正文，也顶替掉分享描述里那句「某某 邀请你试玩」。
        assert!(html.contains("<p class=\"summary\">一个关于小球的冒险，三关，五分钟。</p>"));
        assert!(html.contains(
            "<meta property=\"og:description\" content=\"一个关于小球的冒险，三关，五分钟。\">"
        ));
        // 有封面就不拿卡去顶替它：玩家看到的第一眼应该是这个作品。
        assert!(!html.contains(CARD_WIDE_PATH));
        assert!(html.len() < 16 * 1024, "门禁页 {} 字节", html.len());
    }

    #[test]
    fn without_a_cover_the_share_image_is_the_wide_card() {
        // DESIGN §3.3：一张写着作品名和开发者名的卡不是假截图，它和玩家点开后
        // 看到的是同一个物件。
        let m = manifest();
        let html = page(&m, false).render();
        assert!(html.contains(
            "<meta property=\"og:image\" content=\"http://brisk-otter-41.localhost:8443/_playtest/card-wide.png\">"
        ));
        assert!(html.contains("<meta property=\"og:image:type\" content=\"image/png\">"));
        assert!(html.contains("<meta property=\"og:image:width\" content=\"1200\">"));
        assert!(html.contains("<meta property=\"og:image:height\" content=\"630\">"));
        // 页面上那一块是字卡，不是把 PNG 拉下来当封面——那要多一次请求。
        assert!(html.contains("<div class=\"hero word\""));
        assert!(!html.contains("<img class=\"hero\""));
    }

    #[test]
    fn shows_only_for_html_navigation_without_cookie() {
        assert!(should_show(GateMode::Once, "/", true, true, false, None));
        assert!(should_show(GateMode::Always, "/", true, true, false, None));
        // 有 cookie 就直接出文件。
        assert!(!should_show(GateMode::Once, "/", true, true, true, None));
        // 但如果带了 from（如广场进入），即使有 cookie 也出邀请函
        assert!(should_show(
            GateMode::Once,
            "/",
            true,
            true,
            true,
            Some("plaza")
        ));
        // 资源请求永远不拦。
        assert!(!should_show(GateMode::Once, "/", false, true, false, None));
        assert!(!should_show(GateMode::Once, "/", true, false, false, None));
        // 开发者选了不出就不出。
        assert!(!should_show(GateMode::Never, "/", true, true, false, None));
        assert!(!should_show(
            GateMode::Never,
            "/",
            true,
            true,
            false,
            Some("plaza")
        ));
    }

    #[test]
    fn a_resource_path_never_gets_the_gate_page() {
        // SPA 回退能把任何路径解析成 index.html（`is_html` 为真）。这些路径上
        // 就算客户端把自己说成导航，也不能出门禁页。
        for path in [
            "/assets/app-4f2c.js",
            "/Build/game.wasm",
            "/Build/game.wasm.br",
            "/Build/game.data.gz",
            "/Build/x.wasm.unityweb",
            "/game.pck",
            "/sprites/hero.png",
            "/style.css",
            "/manifest.json",
            "/audio/bgm.mp3",
            "/fonts/x.woff2",
            "/DEEP/PATH/A.PNG",
        ] {
            assert!(
                !should_show(GateMode::Once, path, true, true, false, None),
                "{path}"
            );
            assert!(looks_like_a_resource(path), "{path}");
        }

        // 真的会被人点开的那些反过来要出。
        for path in [
            "/",
            "/sub/",
            "/index.html",
            "/pages/about.htm",
            "/level/3",
            "/user/v1.2",
        ] {
            assert!(!looks_like_a_resource(path), "{path}");
            assert!(
                should_show(GateMode::Once, path, true, true, false, None),
                "{path}"
            );
        }
    }

    #[test]
    fn navigation_detection() {
        assert!(is_navigation(Some("document"), Some("*/*")));
        assert!(is_navigation(None, Some("text/html,application/xhtml+xml")));
        assert!(!is_navigation(Some("empty"), Some("*/*")));
        assert!(!is_navigation(None, None));
        assert!(!is_navigation(None, Some("application/wasm")));

        // `Sec-Fetch-Dest` 在场就以它为准：`fetch()` 带 `empty`，有的库还顺手写
        // `Accept: text/html`，信 Accept 就等于给子资源发门禁页。
        assert!(!is_navigation(Some("empty"), Some("text/html")));
        assert!(!is_navigation(Some("script"), Some("text/html,*/*")));
        assert!(!is_navigation(Some("iframe"), Some("text/html")));
        assert!(!is_navigation(Some("websocket"), Some("text/html")));
        // 大小写不敏感。
        assert!(is_navigation(Some("DOCUMENT"), None));
    }

    #[test]
    fn renders_required_pieces() {
        let m = manifest();
        let html = page(&m, false).render();
        assert!(html.starts_with("<!doctype html>\n<html lang=\"zh-CN\">"));
        assert!(html.contains("<meta name=\"viewport\""));
        assert!(html.contains("某某 邀请你试玩"));
        assert!(html.contains("《小球大冒险》"));
        assert!(html.contains("· v7"));
        assert!(html.contains("<button type=\"submit\">开始</button>"));
        assert!(html.contains("action=\"/_playtest/start\""));
        assert!(html.contains("method=\"post\""));
        assert!(html.contains("href=\"/_playtest/report\""));
        assert!(html.contains("由 localhost 提供"));
        // 分享出去时靠这几条：Discord / iMessage / Telegram 会抓，微信尽力而为。
        assert!(html.contains("<title>某某 邀请你试玩《小球大冒险》</title>"));
        assert!(html
            .contains("<meta name=\"description\" content=\"某某 邀请你试玩《小球大冒险》· v7\">"));
        assert!(html.contains("<meta property=\"og:title\" content=\"《小球大冒险》· v7\">"));
        assert!(html.contains("<meta property=\"og:description\" content=\"某某 邀请你试玩\">"));
        assert!(html.contains("<meta property=\"og:type\" content=\"website\">"));
        assert!(html.contains("og:url\" content=\"http://brisk-otter-41.localhost:8443/\""));
        // 版本、日期、这版改了什么，一行等宽小字（DESIGN §3.3 第 3 条）。
        assert!(html.contains("<p class=\"stamp\">v7 · 9 月 7 日</p>"));
        // 留名是可选的，不是必填。
        assert!(html.contains("<label class=\"holder\"><span>你的名字"));
        assert!(html.contains("· 可不填"));
        assert!(html.contains("maxlength=\"24\""));
        assert!(!html.contains("required"));
        // 玩家页面上不出现品牌域名。
        assert!(!html.contains(playtest_common::DEVELOPER_HOST));
        // 整页要小（DESIGN §3.3：内联矢量头像后保持在十几 KB，秒出）。
        assert!(html.len() < 16 * 1024, "门禁页 {} 字节", html.len());
    }

    #[test]
    fn the_seats_line_says_how_many_and_never_turns_anyone_away() {
        let m = manifest();
        let mut live = SiteLive::empty("brisk-otter-41");
        let mut p = page(&m, false);

        // 没设名额就整行不出现。
        assert!(!p.render().contains("class=\"seats\""));

        live.seats = Some(10);
        p.live = &live;
        assert!(p
            .render()
            .contains("<p class=\"seats\">某某在找 10 位试玩者</p>"));

        let mut live = live.clone();
        live.joined = 6;
        p.live = &live;
        assert!(p
            .render()
            .contains("<p class=\"seats\">某某在找 10 位试玩者 · 已有 6 位加入</p>"));

        // 到齐之后不拦人，只如实说（DESIGN §3.3 第 4 条）。
        let mut live = live.clone();
        live.joined = 10;
        p.live = &live;
        let html = p.render();
        assert!(html.contains("10 位已到齐 · 你仍然可以玩"));
        assert!(html.contains(">开始</button>"));
    }

    #[test]
    fn auxiliary_actions_appear_only_when_available() {
        let m = manifest();
        let mut live = SiteLive::empty("brisk-otter-41");
        let caps = Capabilities {
            email: true,
            ..Default::default()
        };
        let mut p = page(&m, false);

        // 控制面发不了信、没有群、没公开：一行都没有。
        assert!(!p.render().contains("class=\"invitation-tools\""));

        p.caps = &caps;
        let html = p.render();
        assert!(html.contains("<span>关注更新</span></a>"));
        assert!(html.contains("<dialog id=\"notification-settings\""));
        assert!(html.find("</main>").unwrap() < html.find("<dialog").unwrap());
        let footer = html
            .split("<footer>")
            .nth(1)
            .unwrap()
            .split("</footer>")
            .next()
            .unwrap();
        assert!(footer.contains("关注更新"));
        assert!(footer.contains("举报"));
        assert!(!html.contains("follow-details"));
        assert!(html.contains("action=\"/_playtest/follow\""));
        assert!(html.contains("value=\"site:brisk-otter-41\""));
        // 子域上不提供浏览器通知（作品可能有自己的 Service Worker，见 follow.rs）。
        assert!(!html.contains("用浏览器通知"));
        assert!(!html.contains("serviceWorker"));

        live.community_url = Some("https://qq.example/group/12345".into());
        p.live = &live;
        let html = p.render();
        assert!(html.contains("rel=\"noopener nofollow\">开发者的群</a>"));
        // 群链接旁边不写「加群领…」这类话，去哪是开发者的事。
        for word in ["领取", "福利", "内测码"] {
            assert!(!html.contains(word));
        }

        // 「分享」只给公开的作品：私测的邀请不该被转发（DESIGN §3.4）。
        assert!(!html.contains(SHARE_PATH));
        let mut live = live.clone();
        live.listed = true;
        p.live = &live;
        assert!(p
            .render()
            .contains("aria-label=\"分享作品\" title=\"分享作品\""));
    }

    #[test]
    fn other_players_words_are_shown_but_never_discussed() {
        use playtest_common::live::PublicFeedbackItem;
        let m = manifest();
        let mut live = SiteLive::empty("brisk-otter-41");
        let say = |name: Option<&str>, text: &str| PublicFeedbackItem {
            name: name.map(str::to_string),
            text: text.into(),
            version: 7,
            at: "2026-09-08T00:00:00Z".into(),
        };
        live.public_feedback = vec![
            say(Some("小雨"), "不知道要按哪个键"),
            say(None, "第三关卡住了"),
            say(Some("阿吉"), "手感很好"),
            say(Some("多出来的"), "这条不该出现"),
        ];
        let mut p = page(&m, false);

        // 开关没开就一条都不显示，哪怕文件里有。
        p.live = &live;
        assert!(!p.render().contains("不知道要按哪个键"));

        let mut live = live.clone();
        live.feedback_public = true;
        p.live = &live;
        let html = p.render();
        assert!(html.contains("「不知道要按哪个键」<cite>小雨 · v7</cite>"));
        // 没留名字的显示「一位试玩者」。
        assert!(html.contains("「第三关卡住了」<cite>一位试玩者 · v7</cite>"));
        assert_eq!(html.matches("class=\"voice\"").count(), 3, "最多三条");
        assert!(!html.contains("这条不该出现"));
        // 不是讨论区：没有回复、点赞、楼层（DESIGN §3.5）。
        for word in ["回复", "点赞", "评论", "楼"] {
            assert!(!html.contains(word), "「{word}」不该出现");
        }
    }

    #[test]
    fn the_avatar_is_optional_and_leaks_nothing() {
        let m = manifest();
        let mut live = SiteLive::empty("brisk-otter-41");
        live.avatar_url = Some("https://avatars.githubusercontent.com/u/1?v=4".into());
        let mut p = page(&m, false);
        p.live = &live;
        let html = p.render();
        assert!(html.contains(
            "<p class=\"by\"><img src=\"https://avatars.githubusercontent.com/u/1?v=4\""
        ));
        assert!(html.contains("referrerpolicy=\"no-referrer\""));
        assert!(html.contains("loading=\"lazy\""));

        // http 的头像不要：那一跳会在 https 页面上变成混合内容。
        let mut live = live.clone();
        live.avatar_url = Some("http://example.com/a.png".into());
        p.live = &live;
        assert!(!p.render().contains("<img src=\"http://example.com"));
    }

    #[test]
    fn the_source_on_the_link_rides_along_in_the_form() {
        // 扫卡进来的人要在点名册里显示「来自邀请卡」（DESIGN §3.5）。这一下 POST 的
        // Referer 是门禁页自己，所以来源必须由门禁页放进表单带过去。
        let m = manifest();
        let mut p = page(&m, false);
        assert!(!p.render().contains("name=\"from\""));

        p.from = Some("card");
        p.referer = "https://mp.weixin.qq.com/s/abc";
        let html = p.render();
        assert!(html.contains("<input type=\"hidden\" name=\"from\" value=\"card\">"));
        // Referer 走另一个字段：两者是两回事，`from` 可信得多。
        assert!(html.contains("name=\"ref\" value=\"https://mp.weixin.qq.com/s/abc\""));

        assert_eq!(known_source(Some("card")), Some("card"));
        assert_eq!(known_source(Some("notice")), Some("notice"));
        assert_eq!(known_source(Some("javascript:alert(1)")), None);
        assert_eq!(known_source(None), None);
    }

    #[test]
    fn a_name_is_trimmed_not_judged() {
        assert_eq!(clean_name("  小雨 "), Some("小雨".to_string()));
        assert_eq!(clean_name("小\u{0}雨\n"), Some("小雨".to_string()));
        assert_eq!(clean_name("   "), None);
        assert_eq!(clean_name(""), None);
        let long = "名".repeat(MAX_PLAYER_NAME_CHARS + 10);
        assert_eq!(
            clean_name(&long).unwrap().chars().count(),
            MAX_PLAYER_NAME_CHARS
        );
    }

    #[test]
    fn a_tunnel_says_online_where_a_version_would_be() {
        // 隧道模式下清单是从令牌合成的，version 是 0。玩家看到的必须是「在线」，
        // 分享卡片上也一样——一个「v0」会让人以为链接坏了（DESIGN §3.5）。
        let m = manifest();
        let mut p = page(&m, false);
        p.version_label = Some("在线");
        let html = p.render();
        assert!(html.contains("· 在线"));
        assert!(!html.contains("v0"));
        assert!(!html.contains("· v7"));
        assert!(html.contains("<meta property=\"og:title\" content=\"《小球大冒险》· 在线\">"));
    }

    #[test]
    fn the_verb_follows_what_the_thing_is() {
        let mut m = manifest();
        for engine in ["godot", "unity", "phaser", "cocos"] {
            m.engine = Some(engine.into());
            let html = page(&m, false).render();
            assert!(html.contains("邀请你试玩"), "{engine}");
            assert!(!html.contains("邀请你体验"), "{engine}");
        }

        // 用 AI 写小东西的人做的多数不是游戏，不认「试玩」（DESIGN §3.3）。
        for engine in [None, Some("vite"), Some("以后才有的东西")] {
            m.engine = engine.map(str::to_string);
            let html = page(&m, false).render();
            assert!(html.contains("某某 邀请你体验"), "{engine:?}");
            assert!(!html.contains("试玩"), "{engine:?}");
            // 标题栏和分享卡片跟着一起换，不能一处「试玩」一处「体验」。
            assert!(html.contains("<title>某某 邀请你体验《小球大冒险》</title>"));
            assert!(html.contains("content=\"某某 邀请你体验\">"));
        }
    }

    #[test]
    fn an_article_is_read_on_the_invitation_without_a_start_gate() {
        let mut m = manifest();
        m.kind = WorkKind::Article;
        m.engine = None;
        m.entry = Some("文章.md".into());
        let mut p = page(&m, false);
        p.article_html = Some("<h2>第一节</h2><p>正文 <strong>在这里</strong>。</p>");
        let html = p.render();

        assert!(html.contains("某某 邀请你阅读"));
        assert!(html.contains("class=\"card media-card\""));
        assert!(html.contains("<article class=\"article-body\"><h2>第一节</h2>"));
        assert!(!html.contains("class=\"start\""));
        assert!(!html.contains("开始阅读"));
    }

    #[test]
    fn a_video_is_embedded_with_native_controls_and_never_autoplays() {
        let mut m = manifest();
        m.kind = WorkKind::Video;
        m.engine = None;
        m.entry = Some("演示 video.mp4".into());
        let html = page(&m, false).render();

        assert!(html.contains("某某 邀请你观看"));
        assert!(html.contains("class=\"card media-card\""));
        assert!(html.contains("<video controls preload=\"metadata\" playsinline"));
        assert!(html.contains("/%E6%BC%94%E7%A4%BA%20video.mp4"));
        assert!(!html.contains("autoplay"));
        assert!(!html.contains("class=\"start\""));
    }

    #[test]
    fn escapes_everything_from_the_manifest() {
        let mut m = manifest();
        m.title = "<img src=x onerror=alert(1)>".into();
        m.developer = "\"><script>alert(2)</script>".into();
        m.note = Some("</p><script>alert(3)</script>".into());
        let html = page(&m, false).render();
        assert!(!html.contains("<script>alert"));
        assert!(!html.contains("<img src=x"));
        assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
    }

    #[test]
    fn note_and_badge_are_optional() {
        let mut m = manifest();
        m.badge = false;
        m.note = Some("  ".into());
        let html = page(&m, false).render();
        assert!(html.contains("<p class=\"stamp\">v7 · 9 月 7 日</p>"));
        assert!(!html.contains("提供"));

        // 版本说明紧邻版本日期，长说明不挤占到期事实的位置。
        m.note = Some("修了跳跃手感".into());
        let html = page(&m, false).render();
        assert!(html.contains("<p class=\"version-note\">修了跳跃手感</p>"));
    }

    #[test]
    fn wechat_tip_is_the_only_thing_the_user_agent_decides() {
        let mut m = manifest();
        let html = page(&m, true).render();
        // 「右上角 →」是微信独有的操作，别处说了没意义，所以这一句看 UA。
        assert!(html.contains("在浏览器中打开"));
        // 但「能不能玩」不看 UA：X5 / XWeb 的能力没有官方对照表，黑名单一定误伤。
        assert!(!html.contains("需要系统浏览器"));

        m.isolated = true;
        let html = page(&m, true).render();
        assert!(html.contains("在浏览器中打开"));

        // 不是微信就一个字都不提。
        let html = page(&m, false).render();
        assert!(!html.contains("在浏览器中打开"));
    }

    #[test]
    fn capability_detection_never_takes_the_button_away() {
        let mut m = manifest();
        // 不隔离的作品没什么可检测的，一个字节都不加。
        let plain = page(&m, false).render();
        assert!(!plain.contains("pt-cap"));
        assert!(!plain.contains("需要系统浏览器"));

        m.isolated = true;
        let html = page(&m, false).render();
        // 检测的是真实能力，不是 UA。
        assert!(html.contains("crossOriginIsolated"));
        assert!(html.contains("SharedArrayBuffer"));
        assert!(!html.contains("MicroMessenger"));
        // 服务端直出、默认藏着：检测通过、或者根本没有 JS，玩家什么都看不到。
        assert!(html.contains("id=\"pt-cap\" hidden"));
        assert!(html.contains("需要系统浏览器"));
        // 复制链接的办法就在旁边。
        assert!(html.contains("复制链接"));
        assert!(html.contains("value=\"http://brisk-otter-41.localhost:8443/\""));
        // 硬约束：「开始」始终在、始终可点，不能只给「去浏览器打开」
        // （微信《外部链接内容管理规范》§3.2.3，见 DESIGN §5）。
        assert!(html.contains("<button type=\"submit\">开始</button>"));
        assert!(!html.contains("disabled"));
        // 说明在按钮之后，不挡路。
        assert!(html.find(">开始</button>") < html.find("pt-cap"));
    }

    #[test]
    fn expiry_line_is_human_readable() {
        let mut m = manifest();
        m.expires_at = Some("2026-09-08T04:30:00Z".into());
        let html = page(&m, false).render();
        assert!(html.contains("<time datetime=\"2026-09-08T04:30:00Z\">"));
        assert!(html.contains("9 月 8 日 12:30（UTC+8）"));
        assert!(html.contains("</time> 到期"));
        assert!(html.find("class=\"expires\"") < html.find("class=\"start\""));

        // 解析不了就整行不出，不显示一串机器码给玩家看。
        m.expires_at = Some("下周".into());
        assert!(!page(&m, false).render().contains("class=\"expires\""));
    }

    #[test]
    fn no_script_when_there_is_nothing_for_it_to_do() {
        let m = manifest();
        assert!(!page(&m, false).render().contains("<script"));
    }

    #[test]
    fn nothing_is_loaded_from_anywhere_else() {
        // 整页内联，一秒内出现（DESIGN §3.3）。任何外链都是一次白屏的机会，
        // 在微信里还多一次证书与 DNS。
        let mut m = manifest();
        m.isolated = true;
        m.expires_at = Some("2026-09-08T04:30:00Z".into());
        m.note = Some("修了跳跃手感".into());
        let html = page(&m, true).render();
        for forbidden in [
            "<script src",
            "<link rel=\"stylesheet",
            "@import",
            "//fonts.",
        ] {
            assert!(!html.contains(forbidden), "{forbidden}");
        }
        // 最胖的一页也要小（内联头像后保持在十几 KB）。
        assert!(html.len() < 18 * 1024, "门禁页 {} 字节", html.len());
    }

    #[test]
    fn root_invitation_card_links_and_actions() {
        let mut m = manifest();
        m.cover = Some(playtest_common::manifest::Cover {
            hash: "a".repeat(64),
            size: 1000,
            mime: "image/png".into(),
        });
        let mut p = page(&m, false);
        p.is_root = true;
        p.caps = &EMAIL_CAPS;
        p.to = "/p/brisk-otter-41";
        p.nonce = Some("fake-nonce-1234");
        let html = p.render();
        // 开始表单提交到 /p/brisk-otter-41 并开新标签
        assert!(html.contains("<form class=\"start\" method=\"post\" action=\"/p/brisk-otter-41\" target=\"_blank\" rel=\"noopener\">"));
        assert!(html.contains("<button type=\"submit\">开始试玩</button>"));
        // 封面使用子域绝对地址
        assert!(html.contains(
            "<img class=\"hero\" src=\"http://brisk-otter-41.localhost:8443/_playtest/cover\""
        ));
        // 移动端脚本带 nonce
        assert!(html.contains("<script nonce=\"fake-nonce-1234\">if(/Mobi|Android|iPhone/i.test"));
        // 关注提交到根域 /follow
        assert!(html.contains("action=\"/follow\""));
        // 举报链接到子域
        assert!(html.contains("href=\"http://brisk-otter-41.localhost:8443/_playtest/report\""));
        // 根域不带由 localhost 提供的 badge
        assert!(!html.contains("由 localhost 提供"));
    }
}
