//! 广场（DESIGN §3.9）：根域那一页。
//!
//! 内容来自控制面写进对象存储的 `plaza.json`（DESIGN §4.5），这里只读、只渲染。
//! 控制面挂了照常能翻，只是人数旧几分钟。
//!
//! 整页服务端直出，**一行脚本都没有**：左边一条栏，右边一面网格。
//! 发布是本页 `:target` 说明，不是扔去控制台。
//! 栏上「关注」仍是试玩者的抽屉。每张卡是一个链接；正在找人测时先说这次想测。
//!
//! 这一页是我们自己的，没有用户脚本，所以能带严格的 CSP（`app.rs`）；封面来自各作品自己的子域，
//! 头像来自 GitHub——CSP 的 `img-src` 里只多这一个来源。

use std::sync::Arc;
use std::time::Duration;

use playtest_common::follow::root_paths;
use playtest_common::plaza::{Plaza, PlazaItem, PLAYERS_WINDOW_DAYS};
use playtest_common::project::{Blurb, Fact};
use playtest_common::store::FsStore;

use crate::cache::Cached;
use playtest_common::DEVELOPER_API_URL;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::html::{esc, hue, page};
use crate::when;

/// `plaza.json` 的缓存寿命。控制面最快也是每次变动重写一份，30 秒内的旧数据没人分得出来。
pub const TTL: Duration = Duration::from_secs(30);

pub struct PlazaCache {
    store: FsStore,
    cached: Cached<Plaza>,
}

impl PlazaCache {
    pub fn new(store: FsStore) -> Self {
        Self::with_ttl(store, TTL)
    }

    pub fn with_ttl(store: FsStore, ttl: Duration) -> Self {
        Self {
            store,
            cached: Cached::new(ttl),
        }
    }

    /// 读不到（控制面还没写过、或文件坏了）就当作空的：广场页照常出，只是没有卡片。
    pub async fn get(&self) -> Arc<Plaza> {
        self.cached
            .get(|| async {
                match self.store.get_plaza().await {
                    Ok(plaza) => plaza,
                    Err(err) => {
                        tracing::warn!(%err, "读 plaza.json 失败，广场按空的出");
                        None
                    }
                }
            })
            .await
    }
}

/// 渲染时要知道的几样。
pub struct View<'a> {
    pub plaza: &'a Plaza,
    pub now: OffsetDateTime,
}

pub fn render(view: &View<'_>) -> String {
    let head = format!(
        "<meta name=\"description\" content=\"Build in public. Show the work, not the hype. Instant delivery for web apps, tools, articles, and videos. Test instantly without signing up.\">\n\
<link rel=\"canonical\" href=\"https://{host}/\">\n\
<meta property=\"og:title\" content=\"playtest · Show the work, not the hype\">\n\
<meta property=\"og:description\" content=\"Creators push work in progress with one command. Testers play, read, or watch with zero friction and leave versioned feedback.\">\n\
<meta property=\"og:type\" content=\"website\">\n\
<meta property=\"og:url\" content=\"https://{host}/\">\n\
<meta name=\"twitter:card\" content=\"summary_large_image\">\n\
<meta name=\"twitter:title\" content=\"playtest · Show the work, not the hype\">\n\
<meta name=\"twitter:description\" content=\"Build in public with zero friction. Push your web app, tool, article, or video to real people and collect versioned feedback.\">\n",
        host = playtest_common::DEVELOPER_HOST,
    );

    wrap(
        "playtest · Show the work, not the hype",
        &head,
        Here::Plaza,
        &wall(view),
    )
}

/// 人在这一域上的哪一间房。栏上那一项标成当前。
#[derive(Clone, Copy)]
pub enum Here {
    Plaza,
    Mine,
    Collections,
}

/// 广场和关注页共用的外壳：顶上 54px 磨砂通栏，下部是调用方的内容（100% 全宽作品墙）。
pub fn wrap(title: &str, head: &str, here: Here, main: &str) -> String {
    page(
        title,
        head,
        &format!(
            "{rail}<main class=\"main\">\n{main}</main>\n{publish}{account}",
            rail = rail(here),
            account = include_str!("../../ui/account.html"),
            publish = if matches!(here, Here::Collections) {
                String::new()
            } else {
                publish_sheet()
            },
        ),
    )
}

/// 左侧 208px 现代精致侧栏：测试准星、单行小写字标、两间房与发布说明（DESIGN §3.9）。
/// 手机上自适应为顶部紧凑条。
fn rail(here: Here) -> String {
    let plaza = match here {
        Here::Plaza | Here::Collections => format!(
            "<a class=\"nav-item active\" href=\"/\" aria-current=\"page\">{icon}Plaza<span class=\"nav-dot\"></span></a>",
            icon = icon("grid")
        ),
        Here::Mine => format!(
            "<a class=\"nav-item\" href=\"/\">{icon}Plaza</a>",
            icon = icon("grid")
        ),
    };
    let mine = match here {
        Here::Plaza | Here::Collections => format!(
            "<a class=\"nav-item\" href=\"{}\">{icon}Following</a>",
            root_paths::ME,
            icon = icon("bell")
        ),
        Here::Mine => format!(
            "<a class=\"nav-item active\" href=\"{}\" aria-current=\"page\">{icon}Following<span class=\"nav-dot\"></span></a>",
            root_paths::ME,
            icon = icon("bell")
        ),
    };
    let my_projects = format!(
        "<a class=\"nav-item\" href=\"/console/#/\" data-manage>{}My Projects</a>",
        icon("grid")
    );
    let my_collections = format!(
        "<a class=\"nav-item\" href=\"/console/#/collections\" data-manage>{}My Collections</a>",
        icon("folder")
    );
    let publish_link = if matches!(here, Here::Collections) {
        "/#publish-dialog"
    } else {
        "#publish-dialog"
    };
    format!(
        "<aside class=\"sidebar\">\n\
<div>\
<a class=\"brand\" href=\"/\" aria-label=\"playtest.run Home\">\
<span class=\"mark\" aria-hidden=\"true\">{mark}</span>\
{wordmark}</a>\n\
<div class=\"sidebar-action\"><a class=\"publish\" href=\"{publish_link}\">{plus}Publish Project</a></div>\n\
<nav class=\"nav\" aria-label=\"Navigation\">{plaza}{mine}{my_projects}{my_collections}</nav>\
</div>\n\
<div class=\"sidebar-footer\">\
<a class=\"nav-item\" href=\"/console/#/docs/start\">{book}Documentation</a>\
<a class=\"nav-item account\" href=\"/console/?login=1\" data-account-login><span class=\"account-name\">Sign in</span></a>\
</div>\n\
</aside>\n",
        mark = crate::html::MARK,
        wordmark = crate::html::WORDMARK,
        plus = icon("plus"),
        book = icon("book"),
    )
}

pub(crate) fn hero() -> String {
    String::from(
        "<section class=\"plaza-hero\">\n\
<div class=\"hero-badge\"><span class=\"hero-dot\" aria-hidden=\"true\"></span>Build in Public</div>\n\
<h1 class=\"hero-title\">Show the work, not the hype.</h1>\n\
<p class=\"hero-sub\">Deploy games and web builds in seconds. Real playtesters, zero friction, and versioned feedback for your next release.</p>\n\
<div class=\"hero-actions\">\n\
<a class=\"hero-btn primary\" href=\"#publish-dialog\">Publish Project</a>\n\
<div class=\"hero-command\" title=\"Click to copy\">\
<b aria-hidden=\"true\">$</b>\
<code>playtest ./dist --public</code>\
<button class=\"copy-command\" type=\"button\" aria-label=\"Copy command\" title=\"Copy command\" data-hero-copy data-copy-text=\"playtest ./dist --public\">\
<svg class=\"icon copy-glyph\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><rect x=\"8\" y=\"8\" width=\"12\" height=\"12\" rx=\"2\"/><path d=\"M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3\"/></svg>\
<svg class=\"icon copied-glyph\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"m5 12 4 4L19 6\"/></svg>\
<span class=\"hero-copy-text\">Copy</span>\
</button>\
</div>\n\
</div>\n\
</section>\n"
    )
}

/// 下部作品网格：100% 全宽响应式排布。
/// 顺序是推广位（最多几张、永远带标）、正在找人测的、其余按时间。
/// 一个作品只出现一次；推广位有限，多出来的按普通作品排——广场不因为付了钱就变长。
fn wall(view: &View<'_>) -> String {
    let mut out = String::from("<div class=\"content\">\n");
    out.push_str(&hero());
    out.push_str("<h1 class=\"sr-only\">Plaza</h1>\n");
    if view.plaza.items.is_empty() {
        out.push_str(&empty_body());
        out.push_str("</div>\n");
        return out;
    }

    let items = &view.plaza.items;
    let boosted: Vec<&PlazaItem> = items
        .iter()
        .filter(|i| i.boosted)
        .take(playtest_common::boost::MAX_SLOTS)
        .collect();
    let promoted: Vec<&str> = boosted.iter().map(|i| i.slug.as_str()).collect();
    let seeking = items
        .iter()
        .filter(|i| i.seeking && !promoted.contains(&i.slug.as_str()));
    let rest = items
        .iter()
        .filter(|i| !i.seeking && !promoted.contains(&i.slug.as_str()));

    out.push_str("<section class=\"grid\" aria-label=\"Public playtest works\">\n");
    for item in &boosted {
        out.push_str(&tile(item, true, view.now));
    }
    for item in seeking.chain(rest) {
        out.push_str(&tile(item, false, view.now));
    }
    out.push_str("</section>\n");
    out.push_str("</div>\n");
    out
}

fn empty_body() -> String {
    "<section class=\"empty-plaza\"><p>No projects on the plaza yet.</p>\
<p class=\"lead\"><code>playtest ./dist --public</code> will put your project here.</p></section>\n"
        .to_string()
}

/// 一张卡，整张是一个链接。上面只有：横窗、最多一个标、悬停时的动词、
/// 作品名、这次想测或简介、开发者、一件事实（DESIGN §3.9）。
///
/// 没封面时的字卡：色田上一个花押（作品名的第一个字）和角上的 slug。
/// 两样都是作品本来就有的事实，不是编出来的图；花押只有一个字，作品名在卡上仍只出现一次。
pub(crate) fn tile(item: &PlazaItem, on_slot: bool, now: OffsetDateTime) -> String {
    let url = esc(&playtest_common::follow::root_paths::project_path(
        &item.slug,
    ));
    let title = esc(&item.title);
    let verb = playtest_common::wording::action_verb(item.kind, item.is_game);

    let art = match &item.cover_url() {
        Some(cover) => format!(
            "<img class=\"cover\" src=\"{}\" alt=\"\" loading=\"lazy\" decoding=\"async\">",
            esc(cover)
        ),
        None => format!(
            "<div class=\"cover word\" style=\"--h:{}\"><b>{}</b><i>{}</i></div>",
            hue(&item.slug),
            esc(&monogram(item)),
            esc(&item.slug)
        ),
    };
    let tag = if on_slot {
        "<span class=\"tag ad\">Featured</span>"
    } else if item.seeking {
        "<span class=\"tag\">Seeking testers</span>"
    } else {
        ""
    };

    format!(
        "<a class=\"tile\" href=\"{url}\" data-slug=\"{slug}\">\n\
<div class=\"shot\" style=\"view-transition-name:{transition}\">{art}{tag}<span class=\"verb\">{verb}<u>\u{2197}</u></span></div>\n\
<div class=\"tile-body\"><h2>{title}</h2>{blurb}\
<p class=\"meta\"><span class=\"who\">{who}</span>{fact}</p></div>\n\
</a>\n",
        slug = esc(&item.slug),
        transition = crate::html::transition_name(&item.slug),
        who = who(item),
        blurb = blurb(item),
        fact = fact_html(item, now),
    )
}

/// 字卡上的花押：作品名的第一个字；作品名是空的就用 slug 的第一个字母。
fn monogram(item: &PlazaItem) -> String {
    item.title
        .trim()
        .chars()
        .next()
        .or_else(|| item.slug.chars().next())
        .map(|c| c.to_string())
        .unwrap_or_default()
}

/// 卡底右边那一件事实的 HTML。名额那一件多一条 28px 的细线表示进度——
/// 它是同一件事实的另一种写法，不是第二个数字，也不是图表。
fn fact_html(item: &PlazaItem, now: OffsetDateTime) -> String {
    let text = esc(&fact(item, now));
    if item.seeking {
        if let Some(seats) = item.seats.filter(|s| *s > 0) {
            let pct = (item.joined.min(seats) as f64 / seats as f64 * 100.0).round() as u32;
            return format!("<span class=\"fact seats\" style=\"--p:{pct}%\"><i></i>{text}</span>");
        }
    }
    format!("<span class=\"fact\"{}>{text}</span>", fact_hint(item))
}

fn who(item: &PlazaItem) -> String {
    let face = match item
        .avatar_url
        .as_deref()
        .filter(|u| u.starts_with("https://"))
    {
        Some(url) => format!(
            "<img class=\"face\" src=\"{}\" alt=\"\" width=\"24\" height=\"24\" \
loading=\"lazy\" decoding=\"async\" referrerpolicy=\"no-referrer\">",
            esc(url)
        ),
        None => playtest_common::avatar::svg_for_creator(&item.developer, &item.slug, 24),
    };
    format!("{face}{}", esc(&item.developer))
}

/// 作品名下面那一行。说哪一句由 [`PlazaItem::blurb`] 决定（控制台那一侧读的是同一个判断），
/// 这里只负责把它排成 HTML。
fn blurb(item: &PlazaItem) -> String {
    let line = match item.blurb() {
        Some(Blurb::Seeking(note)) => {
            format!("<span class=\"seek-k\">Testing</span> {}", esc(note.trim()))
        }
        Some(Blurb::Summary(summary)) => esc(summary.trim()),
        None => String::new(),
    };
    format!("<p class=\"summary\">{line}</p>")
}

/// 卡底右边那一件事实。**是哪一件**由 [`PlazaItem::fact`] 一处判定（控制台读同一个判断），
/// 这里只把它说成中文；时间那两种要拿「现在」去算，所以留在渲染这一侧。
///
/// 时间解析不出来就退到版本号——那也是一句真话，好过在卡上留一块空白。
fn fact(item: &PlazaItem, now: OffsetDateTime) -> String {
    match item.fact() {
        Fact::Seats { joined, seats } => format!("{joined} / {seats} joined"),
        Fact::Expires { at } => match OffsetDateTime::parse(&at, &Rfc3339) {
            Ok(at) => when::remaining(at, now),
            Err(_) => format!("v{}", item.version),
        },
        Fact::Players { count } => format!("{count} played"),
        Fact::Updated { at } => match OffsetDateTime::parse(&at, &Rfc3339) {
            Ok(updated) => when::ago(updated, now),
            Err(_) => format!("v{}", item.version),
        },
    }
}

fn fact_hint(item: &PlazaItem) -> String {
    if matches!(item.fact(), Fact::Players { .. }) {
        format!(" title=\"Unique sessions clicking start in the past {PLAYERS_WINDOW_DAYS} days\"")
    } else {
        String::new()
    }
}
fn publish_sheet() -> String {
    format!(
        "<dialog id=\"publish-dialog\" class=\"overlay\" aria-labelledby=\"publish-title\">\n\
<a class=\"overlay-back\" href=\"#\" tabindex=\"-1\" aria-label=\"Close\"></a>\n\
<div class=\"sheet publish-sheet\">\n\
<div class=\"dialog-head\">\
<div class=\"dialog-title-wrap\"><span class=\"dialog-mark\" aria-hidden=\"true\">{mark}</span><h2 id=\"publish-title\">Publish Project</h2></div>\
<a class=\"close\" href=\"#\" autofocus aria-label=\"Close\">{close}</a></div>\n\
<div class=\"pub\">\n\
<div class=\"publish-tabs\" role=\"group\" aria-label=\"Publish mode\">\n\
<input type=\"radio\" name=\"pub-mode\" id=\"tab-ai\" checked>\n\
<label for=\"tab-ai\" class=\"tab-ai\"><svg class=\"icon tab-spark\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><defs><linearGradient id=\"ai-grad-p\" x1=\"0%\" y1=\"0%\" x2=\"100%\" y2=\"100%\"><stop offset=\"0%\" stop-color=\"#c084fc\"/><stop offset=\"50%\" stop-color=\"#38bdf8\"/><stop offset=\"100%\" stop-color=\"#75cdb5\"/></linearGradient></defs><path fill=\"url(#ai-grad-p)\" d=\"m12 3 1.9 4.9a3.5 3.5 0 0 0 2.2 2.2L21 12l-4.9 1.9a3.5 3.5 0 0 0-2.2 2.2L12 21l-1.9-4.9a3.5 3.5 0 0 0-2.2-2.2L3 12l4.9-1.9a3.5 3.5 0 0 0 2.2-2.2z\"/></svg>AI Prompt</label>\n\
<input type=\"radio\" name=\"pub-mode\" id=\"tab-static\">\n\
<label for=\"tab-static\">Export Directory</label>\n\
<input type=\"radio\" name=\"pub-mode\" id=\"tab-local\">\n\
<label for=\"tab-local\">Local Port</label>\n\
<input type=\"radio\" name=\"pub-mode\" id=\"tab-backend\">\n\
<label for=\"tab-backend\">With Backend</label>\n\
<input type=\"radio\" name=\"pub-mode\" id=\"tab-install\">\n\
<label for=\"tab-install\">Install CLI</label>\n\
</div>\n\
<div class=\"cli\">\n\
<div class=\"cli-bar\"><div class=\"cli-info\"><span class=\"cli-dots\" aria-hidden=\"true\"></span></div>\
<button class=\"copy-command\" type=\"button\" aria-label=\"Copy command\" title=\"Copy command\" data-copy-command hidden>\
<svg class=\"icon copy-glyph\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><rect x=\"8\" y=\"8\" width=\"12\" height=\"12\" rx=\"2\"/><path d=\"M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3\"/></svg>\
<svg class=\"icon copied-glyph\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"m5 12 4 4L19 6\"/></svg><span class=\"copy-text\">Copy</span></button></div>\n\
<div class=\"codebox ai-box\" id=\"panel-ai\" tabindex=\"0\" role=\"region\" aria-label=\"AI Prompt\" data-copy-text=\"{ai_prompt_plain}\"><code>{ai_prompt_plain}</code></div>\n\
<div class=\"codebox\" id=\"panel-static\" tabindex=\"0\" role=\"region\" aria-label=\"Publish command\"><b aria-hidden=\"true\">$</b><code><span class=\"k\">playtest</span> <span class=\"a\">./dist</span></code></div>\n\
<div class=\"codebox\" id=\"panel-local\" tabindex=\"0\" role=\"region\" aria-label=\"Publish command\"><b aria-hidden=\"true\">$</b><code><span class=\"k\">playtest</span> <span class=\"a\">3000</span></code></div>\n\
<div class=\"codebox\" id=\"panel-backend\" tabindex=\"0\" role=\"region\" aria-label=\"Publish command\"><b aria-hidden=\"true\">$</b><code><span class=\"k\">playtest</span> <span class=\"a\">./dist</span> <span class=\"f\">--backend</span> <span class=\"a\">8000</span></code></div>\n\
<div class=\"codebox\" id=\"panel-install\" tabindex=\"0\" role=\"region\" aria-label=\"Install command\"><b aria-hidden=\"true\">$</b><code><span class=\"k\">curl</span> <span class=\"f\">-fsSL</span> <span class=\"a\">https://playtest.run/install.sh</span> <span class=\"f\">|</span> <span class=\"k\">bash</span></code></div>\n\
<p class=\"leg\" id=\"leg-static\">{info}<span>Build your project and replace <code>./dist</code> with your export folder.</span></p>\n\
<p class=\"leg\" id=\"leg-local\">{info}<span>Start your local dev server and keep the terminal session open.</span></p>\n\
<p class=\"leg\" id=\"leg-backend\">{info}<span>Proxies unmatched routes to your local backend.</span></p>\n\
<p class=\"leg\" id=\"leg-install\">{info}<span>macOS &amp; Linux · Installs to ~/.local/bin</span></p>\n\
</div>\n\
<p class=\"pub-status\" role=\"status\"></p>\n\
</div>\n\
<div class=\"pub-foot\">\
<label for=\"tab-install\" class=\"foot-install-btn\">{dl}Install CLI (curl)</label>\
<a href=\"{usage}\" target=\"_blank\" rel=\"noopener\">{book}Documentation</a>\
<a href=\"{dev}/console/\" target=\"_blank\" rel=\"noopener\">Developer Console{out}</a>\n\
</div>\n\
</div>\n\
</dialog>\n",
        close = icon("close"),
        mark = crate::html::MARK,
        info = icon("info"),
        dl = icon("download"),
        book = icon("book"),
        out = icon("out"),
        usage = USAGE_URL,
        dev = DEVELOPER_API_URL,
        ai_prompt_plain = crate::html::esc(
            "Build and publish this project to playtest.run:\n\
1. Ensure CLI is installed: curl -fsSL https://playtest.run/install.sh | bash\n\
2. Publish: run playtest <dir> for static build with index.html (add --spa for SPA), or playtest <port> for local dev server. Optional: -n \"<name>\", -m \"<note>\".\n\
3. Hand back the playable link and QR code."
        ),
    )
}

/// CLI 的说明处。
const _RELEASES_URL: &str = "https://github.com/roviix/playtest.run/releases";
const USAGE_URL: &str = "https://github.com/roviix/playtest.run#readme";

/// 页面上的几个图标，画在页面里：不靠外部字体、也不靠 `<use href>`
/// （CSP `default-src 'none'` 会把同页 fragment 的引用挡掉）。都是 24 格里的几笔线。
pub(crate) fn icon(name: &str) -> &'static str {
    match name {
        "close" => {
            r#"<svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="m6 6 12 12M6 18 18 6"/></svg>"#
        }
        "out" => {
            r#"<svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M7 17 16.2 7.8M17 17V8H8"/></svg>"#
        }
        "grid" => {
            r#"<svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><rect x="3.5" y="3.5" width="7" height="7" rx="1.8"/><rect x="13.5" y="3.5" width="7" height="7" rx="1.8"/><rect x="3.5" y="13.5" width="7" height="7" rx="1.8"/><rect x="13.5" y="13.5" width="7" height="7" rx="1.8"/></svg>"#
        }
        "folder" => {
            r#"<svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M3 7V5a1 1 0 0 1 1-1h5l2 3h9v13H3Z"/></svg>"#
        }
        "bell" => {
            r#"<svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4.2a5.3 5.3 0 0 0-5.3 5.3c0 4-1.7 5.4-1.7 5.4h14s-1.7-1.4-1.7-5.4A5.3 5.3 0 0 0 12 4.2Z"/><path d="M10.3 18.2a2 2 0 0 0 3.4 0"/></svg>"#
        }
        "plus" => {
            r#"<svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14"/></svg>"#
        }
        "info" => {
            r#"<svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4m0-4h.01"/></svg>"#
        }
        "download" => {
            r#"<svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v12m0 0-4-4m4 4 4-4M4 20h16"/></svg>"#
        }
        "book" => {
            r#"<svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20M4 4.5A2.5 2.5 0 0 1 6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15Z"/></svg>"#
        }
        "mark" => crate::html::MARK,
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn item(slug: &str) -> PlazaItem {
        PlazaItem {
            slug: slug.into(),
            url: format!("http://{slug}.localhost:8443"),
            title: "小球大冒险".into(),
            kind: playtest_common::manifest::WorkKind::Web,
            developer: "某某".into(),
            summary: Some("三关，五分钟，手机上也能玩。".into()),
            engine: Some("godot".into()),
            is_game: true,
            version: 7,
            updated_at: "2026-09-08T03:00:00Z".into(),
            expires_at: Some("2026-09-08T20:30:00Z".into()),
            cover_hash: None,
            players: 12,
            seeking: true,
            note: Some("新手引导看得懂吗".into()),
            seats: Some(10),
            joined: 6,
            followers: 24,
            avatar_url: None,
            boosted: false,
        }
    }

    fn view(plaza: &Plaza) -> View<'_> {
        View {
            plaza,
            now: datetime!(2026-09-08 04:00:00 UTC),
        }
    }

    fn wall_of(items: Vec<PlazaItem>) -> Plaza {
        Plaza {
            schema: 1,
            generated_at: "2026-09-08T03:57:00Z".into(),
            club_followers: 0,
            collections: vec![],
            items,
        }
    }

    #[test]
    fn a_card_is_one_link_with_only_these_things_on_it() {
        let plaza = wall_of(vec![item("brisk-otter-41")]);
        let html = render(&view(&plaza));
        assert!(html.starts_with("<!doctype html>\n<html lang=\"en\">"));
        assert!(html.contains(
            "<a class=\"tile\" href=\"/p/brisk-otter-41\" data-slug=\"brisk-otter-41\">"
        ));
        assert!(html.contains("新手引导看得懂吗"));
        assert!(html.contains("Testing"));
        assert!(!html.contains("三关，五分钟，手机上也能玩。"));
        assert!(html.contains("某某"));
        assert!(html.contains("<span class=\"tag\">Seeking testers</span>"));
        assert!(html.contains("<span class=\"verb\">Play"));
        assert!(!html.contains("即刻试玩"));
        assert!(html.contains("6 / 10 joined"));
        assert!(!html.contains("人玩过"));
        assert!(!html.contains("还剩"));
        assert!(html.contains("<div class=\"cover word\" style=\"--h:"));
        // 字卡：花押是作品名的第一个字，角上是 slug；作品名本身仍只出现一次。
        assert!(html.contains("><b>小</b><i>brisk-otter-41</i></div>"));
        // 名额那一件事实带一条进度线：6 / 10 就是 60%。
        assert!(html
            .contains("<span class=\"fact seats\" style=\"--p:60%\"><i></i>6 / 10 joined</span>"));
        assert!(html.contains("<h2>小球大冒险</h2>"));
        assert!(!html.contains("class=\"type\""));
        assert_eq!(html.matches("小球大冒险").count(), 1, "作品名只出现一次");
        assert!(!html.contains("<img class=\"cover\""));
        for word in [
            "想玩",
            "举报",
            "人关注</span>",
            "value=\"site:",
            ">游戏<",
            "<span class=\"v\">",
        ] {
            assert!(!html.contains(word), "卡上不该有「{word}」");
        }
        assert!(!html.contains("没登录发的作品 24 小时后自己下来"));
        assert!(!html.contains("推广位永远标出来"));
        assert!(!html.contains("只放开发者主动公开的作品"));
        assert!(!html.contains("无需登录"));
        assert!(!html.contains("不用安装"));
    }

    /// 这一页一行脚本都没有：服务端直出，CSP 锁死，没有 JS 的浏览器看到的就是全部。
    #[test]
    fn the_wall_has_no_script_at_all() {
        let plaza = wall_of(vec![item("a"), item("b"), item("c")]);
        let html = render(&view(&plaza));
        assert!(!html.contains("<script"));
        assert!(!html.contains("localStorage"));
        assert!(html.contains("aria-current=\"page\""));
        assert!(html.contains("Plaza<span class=\"nav-dot\"></span>"));
        assert!(html.contains(&format!("href=\"{}\"", root_paths::ME)));
        assert!(html.contains("Following</a>"));
        assert!(!html.contains("<i>01</i>"));
        assert!(!html.contains("class=\"hero\""));
        assert!(!html.contains("来玩点，还没定稿的"));
        assert!(!html.contains("THE PLAYTEST CORNER"));
        assert!(!html.contains("SMALL BUILDS"));
        assert!(!html.contains("发现 / DISCOVER"));
        assert!(!html.contains("关于 playtest"));
        assert!(!html.contains(">v0.1<"));
        assert!(!html.contains("LESS LAUNCH"));
        assert!(!html.contains("brand-icon"));
        assert!(!html.contains("发布我的作品"));
        assert!(html.contains(">Publish Project</a>"));
        assert!(!html.contains("class=\"toolbar\""));
        assert!(!html.contains("class=\"topbar\""));
        assert!(!html.contains("class=\"breadcrumb\""));
        assert!(!html.contains("点开就玩，不用注册"));
        assert!(html.contains("href=\"#publish-dialog\""));
        assert!(html.contains("id=\"publish-title\">Publish Project</h2>"));
        assert!(!html.contains("把作品递给第一位玩家"));
        assert!(!html.contains("链接可以直接分享，玩家不用注册。"));
        assert!(html.contains("class=\"mark-svg\""));
        assert!(html.contains("class=\"sidebar\""));
        assert!(html.contains(crate::html::WORDMARK));
        assert!(html.contains("aria-label=\"playtest.run Home\""));
        assert!(!html.contains("<span class=\"mark\" aria-hidden=\"true\">p"));
        assert!(!html.contains("class=\"steps\""));
        assert!(!html.contains("--seek"));
        assert!(!html.contains("class=\"dialog-foot\""));
        assert!(!html.contains("class=\"pub-details\""));
        assert!(!html.contains("CLI 模式"));
        assert!(html.contains(crate::html::MARK));
        assert!(html.contains("class=\"cli-bar\""));
        assert!(
            html.find("class=\"publish-tabs\"").unwrap() < html.find("class=\"cli\"").unwrap(),
            "Tab 在终端舱上方"
        );
        assert!(html.contains("class=\"pub-foot\""));
        assert!(html.contains(">Install CLI (curl)</label>"));
        assert!(html.contains("id=\"panel-install\""));
        assert!(html.contains(">Documentation</a>"));
        assert!(html.contains("for=\"tab-static\">Export Directory</label>"));
        assert!(html.contains("for=\"tab-install\">Install CLI</label>"));
        assert!(html
            .contains("<span class=\"k\">playtest</span> <span class=\"a\">./dist</span></code>"));
        assert!(html.contains("<span class=\"k\">playtest</span> <span class=\"a\">3000</span>"));
        assert!(html.contains("<span class=\"f\">--backend</span> <span class=\"a\">8000</span>"));
        assert!(html.contains(
            "aria-label=\"Copy command\" title=\"Copy command\" data-copy-command hidden>"
        ));
        assert!(
            html.find("data-copy-command").unwrap() < html.find("id=\"panel-static\"").unwrap()
        );
        assert!(!html.contains("brew install"));
        assert!(!html.contains("href=\"#about-dialog\""));
        assert!(html.contains(&format!(
            "href=\"{DEVELOPER_API_URL}/console/\" target=\"_blank\" rel=\"noopener\">"
        )));
        assert_eq!(
            html.matches(&format!("{DEVELOPER_API_URL}/console/"))
                .count(),
            1
        );
        for word in [
            "最多人玩",
            "筛选",
            "data-band",
            "class=\"chip\"",
            "最近更新</h2>",
            "preview-panel",
            "play-dialog",
        ] {
            assert!(!html.contains(word), "「{word}」不该出现");
        }
    }

    /// 顺序：推广位永远在最前面而且带标，正在找人测其次，其余最后。一个作品只出现一次。
    #[test]
    fn order_is_paid_then_seeking_then_the_rest_and_nothing_twice() {
        let mut ad = item("paid-one");
        ad.boosted = true;
        let mut plain = item("quiet-one");
        plain.seeking = false;
        plain.note = None;
        let plaza = wall_of(vec![plain, ad, item("seeking-one")]);
        let html = render(&view(&plaza));

        let paid = html.find("data-slug=\"paid-one\"").expect("推广");
        let seeking = html.find("data-slug=\"seeking-one\"").expect("找人测");
        let quiet = html.find("data-slug=\"quiet-one\"").expect("其余");
        assert!(paid < seeking && seeking < quiet);
        assert_eq!(html.matches("data-slug=\"paid-one\"").count(), 1);
        assert!(html.contains("<span class=\"tag ad\">Featured</span>"));
        let paid_card = &html[paid..seeking];
        assert!(!paid_card.contains("Seeking testers"));
        let quiet_card = &html[quiet..];
        assert!(!quiet_card.contains("class=\"tag\""));
    }

    /// 推广位最多 `MAX_SLOTS` 张，多出来的按普通作品排、不带「推广」标。
    #[test]
    fn boost_slots_are_capped() {
        let items: Vec<PlazaItem> = (0..playtest_common::boost::MAX_SLOTS + 2)
            .map(|i| {
                let mut it = item(&format!("ad-{i}"));
                it.boosted = true;
                it
            })
            .collect();
        let plaza = wall_of(items);
        let html = render(&view(&plaza));
        assert_eq!(
            html.matches("<span class=\"tag ad\">").count(),
            playtest_common::boost::MAX_SLOTS
        );
        assert_eq!(
            html.matches("class=\"tile\"").count(),
            playtest_common::boost::MAX_SLOTS + 2
        );
    }

    /// 稀疏态也只是墙：不口号、不介绍，卡自己说话。
    #[test]
    fn a_sparse_wall_is_just_the_wall() {
        let plaza = wall_of(vec![item("only-one")]);
        let html = render(&view(&plaza));
        assert!(html.contains("class=\"tile\""));
        assert!(!html.contains("class=\"hero\""));
        assert!(!html.contains("来玩点，还没定稿的"));
        assert!(!html.contains("class=\"toolbar\""));
        assert!(!html.contains("class=\"intro\""));
        assert!(!html.contains("这里是开发者手上正在做的东西"));
        assert!(!html.contains("作品页上有地方留话"));
    }

    /// 一件事实的顺序：名额 → 还剩多久 → 人玩过 → 多久以前。
    #[test]
    fn one_fact_per_card_in_this_order() {
        let now = datetime!(2026-09-08 04:00:00 UTC);
        let mut it = item("x");
        assert_eq!(fact(&it, now), "6 / 10 joined");

        it.seeking = false;
        assert_eq!(fact(&it, now), "16h left");

        it.expires_at = None;
        assert_eq!(fact(&it, now), "12 played");
        assert!(fact_hint(&it).contains(&PLAYERS_WINDOW_DAYS.to_string()));

        it.players = 0;
        assert_eq!(fact(&it, now), "1h ago");
        assert!(fact_hint(&it).is_empty());

        it.updated_at = "not-a-date".into();
        assert_eq!(fact(&it, now), "v7");

        it.seeking = true;
        it.seats = None;
        it.updated_at = "2026-09-08T03:00:00Z".into();
        it.expires_at = None;
        it.players = 0;
        assert_eq!(fact(&it, now), "1h ago");
    }

    /// 关注广场只在关注页里办。栏上、卡上都不放第二扇门。
    #[test]
    fn following_the_plaza_is_not_on_the_wall() {
        let plaza = wall_of(vec![item("brisk-otter-41")]);
        let html = render(&view(&plaza));
        assert!(!html.contains("有新东西时告诉我"));
        assert!(!html.contains("value=\"plaza\""));
        assert!(!html.contains(&format!("action=\"{}\"", root_paths::FOLLOW)));
        assert!(!html.contains("value=\"site:brisk-otter-41\""));
        assert!(html.contains(&format!("href=\"{}\"", root_paths::ME)));
        assert!(html.contains("Following</a>"));
    }

    #[test]
    fn the_club_size_is_not_on_the_rail() {
        let mut plaza = wall_of(vec![item("a")]);
        assert!(!render(&view(&plaza)).contains("关注着这里"));
        plaza.club_followers = 128;
        assert!(!render(&view(&plaza)).contains("关注着这里"));
        assert!(!render(&view(&plaza)).contains("人关注"));
    }

    /// 头像是外站图片：不带 referrer、懒加载，而且只认 https。
    #[test]
    fn an_avatar_is_a_face_not_a_tracker() {
        let mut it = item("brisk-otter-41");
        it.avatar_url = Some("https://avatars.githubusercontent.com/u/1?v=4".into());
        let plaza = wall_of(vec![it]);
        let html = render(&view(&plaza));
        assert!(html.contains("class=\"face\""));
        assert!(html.contains("referrerpolicy=\"no-referrer\""));
        assert!(html.contains("loading=\"lazy\""));

        let mut it = item("brisk-otter-41");
        it.avatar_url = Some("http://evil.example/a.png".into());
        let plaza = wall_of(vec![it]);
        let html = render(&view(&plaza));
        // 不认非 https 的外部地址，退到确定性算法头像，绝不把流量引去非法的图床
        assert!(!html.contains("evil.example"));
        assert!(html.contains("<svg"));
        assert!(html.contains("class=\"face\""));
    }

    #[test]
    fn a_cover_is_an_image_and_a_non_game_says_experience() {
        let mut it = item("wise-mink-28");
        it.cover_hash = Some("abcd1234ef".into());
        it.is_game = false;
        it.seeking = false;
        it.players = 0;
        it.expires_at = None;
        it.followers = 0;
        it.summary = None;
        it.engine = None;
        let plaza = wall_of(vec![it]);
        let html = render(&view(&plaza));
        assert!(html.contains(
            "<img class=\"cover\" src=\"http://wise-mink-28.localhost:8443/_playtest/cover?v=abcd1234\""
        ));
        assert!(html.contains("<h2>小球大冒险</h2>"));
        assert_eq!(html.matches("小球大冒险").count(), 1);
        assert!(!html.contains("cover word"));
        assert!(html.contains("<span class=\"verb\">Test"));
        assert!(!html.contains("即刻体验"));
        assert!(!html.contains("class=\"tag\""));
        assert!(!html.contains("这次想测"));
        assert!(!html.contains("class=\"type\""));
        assert!(html.contains("1h ago"));
        assert!(!html.contains("人玩过"));
        // 不是名额那一件事实，就没有进度线。
        assert!(!html.contains("class=\"fact seats\""));
    }

    /// 花押取作品名的第一个字；作品名空了就退到 slug 的第一个字母。名额进度不超过 100%。
    #[test]
    fn a_monogram_is_the_first_glyph_and_seats_never_overflow() {
        let mut it = item("wise-mink-28");
        assert_eq!(monogram(&it), "小");
        it.title = "  vite-vanilla".into();
        assert_eq!(monogram(&it), "v");
        it.title = "   ".into();
        assert_eq!(monogram(&it), "w");

        let now = datetime!(2026-09-08 04:00:00 UTC);
        it.joined = 14;
        assert!(fact_html(&it, now).contains("style=\"--p:100%\""));
        it.seats = Some(0);
        assert!(!fact_html(&it, now).contains("fact seats"));
    }

    #[test]
    fn everything_from_the_feed_is_escaped() {
        let mut it = item("x");
        it.title = "<img src=x onerror=alert(1)>".into();
        it.developer = "\"><script>alert(2)</script>".into();
        it.summary = Some("</p><script>alert(3)</script>".into());
        it.note = Some("</p><script>alert(5)</script>".into());
        it.url = "javascript:alert(4)\" onmouseover=\"".into();
        it.avatar_url = Some("https://avatars.githubusercontent.com/u/1?a=\"><b>".into());
        let plaza = wall_of(vec![it]);
        let html = render(&view(&plaza));
        assert!(!html.contains("<script"));
        assert!(!html.contains("<img src=x"));
        assert!(!html.contains("onmouseover=\""));
        assert!(!html.contains("javascript:"));
        assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
        assert!(html.contains("href=\"#\""));
    }

    #[test]
    fn empty_plaza_says_so() {
        let plaza = Plaza::default();
        let html = render(&view(&plaza));
        assert!(html.contains("No projects on the plaza yet"));
        assert!(!html.contains("class=\"grid\""));
        assert!(html.contains("--public"));
        assert!(!html.contains("来玩点，还没定稿的"));
        assert!(html.contains(DEVELOPER_API_URL));
    }

    #[tokio::test]
    async fn cache_serves_empty_when_nothing_was_written() {
        let dir = tempfile::tempdir().unwrap();
        let cache = PlazaCache::new(FsStore::new(dir.path()));
        assert!(cache.get().await.items.is_empty());
    }
}
