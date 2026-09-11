//! 广场（DESIGN §3.9）：根域那一页。
//!
//! 内容来自控制面写进对象存储的 `plaza.json`（DESIGN §4.5），这里只读、只渲染。
//! 控制面挂了照常能翻，只是人数旧几分钟。
//!
//! 整页服务端直出，**一行脚本都没有**：左边一条栏，右边一面网格。
//! 发布是本页 `:target` 说明，不是扔去控制台。
//! 栏上「我的」仍是试玩者的抽屉。每张卡是一个链接；正在找人测时先说这次想测。
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
    let head =
        "<meta name=\"description\" content=\"正在找人试玩的作品。点开玩一会儿，不用注册。\">\n\
<meta property=\"og:title\" content=\"playtest.run\">\n\
<meta property=\"og:description\" content=\"开发者把手上能玩的版本放到这里，路过的人点开就玩。\">\n\
<meta property=\"og:type\" content=\"website\">\n";

    wrap("playtest.run", head, Here::Plaza, &wall(view))
}

/// 人在这一域上的哪一间房。栏上那一项标成当前。
#[derive(Clone, Copy)]
pub enum Here {
    Plaza,
    Mine,
}

/// 广场和「我的」共用的外壳：左边那条栏，右边是调用方的内容。
pub fn wrap(title: &str, head: &str, here: Here, main: &str) -> String {
    page(
        title,
        head,
        &format!(
            "{rail}<main class=\"main\">\n{main}</main>\n{publish}",
            rail = rail(here),
            publish = publish_sheet(),
        ),
    )
}

/// 左边那条窄栏：标记、两间房、栏底一个「发布」（DESIGN §3.9）。手机上收成顶上一条（样式里做）。
///
/// 每一项是一个图标加一个两字标签。图标是画在页面里的几笔线（`icon`），不引图标字体；
/// 「发布」打开的是本页的命令说明，不是一间房——上传在终端里，登录在另一张域上。
fn rail(here: Here) -> String {
    let plaza = match here {
        Here::Plaza => format!(
            "<a class=\"nav-item active\" href=\"/\" aria-current=\"page\">{}广场<span class=\"nav-dot\"></span></a>",
            icon("grid")
        ),
        Here::Mine => format!("<a class=\"nav-item\" href=\"/\">{}广场</a>", icon("grid")),
    };
    let mine = match here {
        Here::Plaza => format!(
            "<a class=\"nav-item\" href=\"{}\">{}我的</a>",
            root_paths::ME,
            icon("me")
        ),
        Here::Mine => format!(
            "<a class=\"nav-item active\" href=\"{}\" aria-current=\"page\">{}我的<span class=\"nav-dot\"></span></a>",
            root_paths::ME,
            icon("me")
        ),
    };
    format!(
        "<aside class=\"sidebar\">\n\
<a class=\"brand\" href=\"/\" aria-label=\"playtest.run 首页\">\
<span class=\"mark\" aria-hidden=\"true\">{mark}</span>\
<span class=\"wordmark\">playtest<span>.run</span></span></a>\n\
<nav class=\"nav\" aria-label=\"页面\">{plaza}{mine}</nav>\n\
<div class=\"sidebar-bottom\">\n\
<a class=\"nav-item publish\" href=\"#publish-dialog\">{plus}发布</a>\n\
</div>\n\
</aside>\n",
        mark = icon("mark"),
        plus = icon("plus"),
    )
}

/// 右边那面墙：直接一面网格。
/// 顺序是推广位（最多几张、永远带标）、正在找人测的、其余按时间。
/// 一个作品只出现一次；推广位有限，多出来的按普通作品排——广场不因为付了钱就变长。
fn wall(view: &View<'_>) -> String {
    let mut out = String::from("<div class=\"content\">\n");
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

    out.push_str("<section class=\"grid\" aria-label=\"公开试玩作品\">\n");
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
    "<section class=\"empty-plaza\"><p>广场上还没有作品。</p>\
<p class=\"lead\"><code>playtest ./dist --public</code> 会把作品放到这里。</p></section>\n"
        .to_string()
}

/// 一张卡，整张是一个链接。上面只有：横窗、最多一个标、悬停时的动词、
/// 作品名、这次想测或简介、开发者、一件事实（DESIGN §3.9）。
///
/// 没封面时的字卡：色田上一个花押（作品名的第一个字）和角上的 slug。
/// 两样都是作品本来就有的事实，不是编出来的图；花押只有一个字，作品名在卡上仍只出现一次。
fn tile(item: &PlazaItem, on_slot: bool, now: OffsetDateTime) -> String {
    let url = if item.url.starts_with("https://") || item.url.starts_with("http://") {
        esc(&item.url)
    } else {
        "#".to_string()
    };
    let title = esc(&item.title);
    let verb = if item.is_game { "试玩" } else { "体验" };

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
        "<span class=\"tag ad\">推广</span>"
    } else if item.seeking {
        "<span class=\"tag\">正在找人测</span>"
    } else {
        ""
    };

    format!(
        "<a class=\"tile\" href=\"{url}\" data-slug=\"{slug}\">\n\
<div class=\"shot\">{art}{tag}<span class=\"verb\">{verb}<u>\u{2197}</u></span></div>\n\
<div class=\"tile-body\"><h2>{title}</h2>{blurb}\
<p class=\"meta\"><span class=\"who\">{who}</span>{fact}</p></div>\n\
</a>\n",
        slug = esc(&item.slug),
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
            "<img class=\"face\" src=\"{}\" alt=\"\" width=\"16\" height=\"16\" \
loading=\"lazy\" decoding=\"async\" referrerpolicy=\"no-referrer\">",
            esc(url)
        ),
        None => String::new(),
    };
    format!("{face}{}", esc(&item.developer))
}

/// 作品名下面那一行。说哪一句由 [`PlazaItem::blurb`] 决定（控制台那一侧读的是同一个判断），
/// 这里只负责把它排成 HTML。
fn blurb(item: &PlazaItem) -> String {
    let line = match item.blurb() {
        Some(Blurb::Seeking(note)) => format!(
            "<span class=\"seek-k\">这次想测</span> {}",
            esc(note.trim())
        ),
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
        Fact::Seats { joined, seats } => format!("{joined} / {seats} 位"),
        Fact::Expires { at } => match OffsetDateTime::parse(&at, &Rfc3339) {
            Ok(at) => when::remaining(at, now),
            Err(_) => format!("v{}", item.version),
        },
        Fact::Players { count } => format!("{count} 人玩过"),
        Fact::Updated { at } => match OffsetDateTime::parse(&at, &Rfc3339) {
            Ok(updated) => when::ago(updated, now),
            Err(_) => format!("v{}", item.version),
        },
    }
}

fn fact_hint(item: &PlazaItem) -> String {
    if matches!(item.fact(), Fact::Players { .. }) {
        format!(" title=\"近 {PLAYERS_WINDOW_DAYS} 天\"")
    } else {
        String::new()
    }
}

fn publish_sheet() -> String {
    format!(
        "<div id=\"publish-dialog\" class=\"overlay\">\n\
<a class=\"overlay-back\" href=\"#\" aria-label=\"关闭\"></a>\n\
<div class=\"sheet\" role=\"dialog\" aria-labelledby=\"publish-title\">\n\
<div class=\"dialog-head\"><h2 id=\"publish-title\">从一条命令开始</h2>\
<a class=\"close\" href=\"#\" aria-label=\"关闭\">{close}</a></div>\n\
<div class=\"pub-box\">\n\
<div class=\"tabs\" role=\"tablist\" aria-label=\"发布方式\">\n\
<input type=\"radio\" name=\"pub-mode\" id=\"tab-static\" checked>\n\
<label for=\"tab-static\">导出目录</label>\n\
<input type=\"radio\" name=\"pub-mode\" id=\"tab-local\">\n\
<label for=\"tab-local\">本地端口</label>\n\
<input type=\"radio\" name=\"pub-mode\" id=\"tab-backend\">\n\
<label for=\"tab-backend\">带后端</label>\n\
</div>\n\
<div class=\"codebox\" id=\"panel-static\"><b>$</b><code>playtest ./dist --public -m \"想让人看什么\"</code></div>\n\
<div class=\"codebox\" id=\"panel-local\"><b>$</b><code>playtest 5173</code></div>\n\
<div class=\"codebox\" id=\"panel-backend\"><b>$</b><code>playtest ./dist --backend 3000 --public -m \"想让人看什么\"</code></div>\n\
</div>\n\
<details class=\"tip\"><summary aria-label=\"还没装，或者发出去之后呢\">{help}</summary>\n\
<div class=\"tip-body\"><p>还没装：从 <a href=\"{releases}\" target=\"_blank\" rel=\"noopener\">Releases</a> 下载一个文件，放进 PATH。</p>\
<p>发出去之后，谁来玩过在 <a href=\"{dev}/console/\" target=\"_blank\" rel=\"noopener\">开发者控制台</a> 看。</p></div>\n\
</details>\n\
</div>\n\
</div>\n",
        close = icon("close"),
        help = icon("help"),
        releases = RELEASES_URL,
        dev = DEVELOPER_API_URL,
    )
}

/// CLI 的下载处。它不是开发者域（AGENTS 第 7 条管的是登录、令牌、控制台），是公开的发布页。
const RELEASES_URL: &str = "https://github.com/roviix/playtest.run/releases";

/// 页面上的几个图标，画在页面里：不靠外部字体、也不靠 `<use href>`
/// （CSP `default-src 'none'` 会把同页 fragment 的引用挡掉）。都是 24 格里的几笔线。
fn icon(name: &str) -> &'static str {
    match name {
        "close" => {
            r#"<svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="m6 6 12 12M6 18 18 6"/></svg>"#
        }
        "help" => {
            r#"<svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="8.5"/><path d="M9.7 9.9a2.4 2.4 0 1 1 3.4 2.2c-.7.4-1.1.9-1.1 1.7M12 16.7h.01"/></svg>"#
        }
        "grid" => {
            r#"<svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><rect x="3.5" y="3.5" width="7" height="7" rx="1.8"/><rect x="13.5" y="3.5" width="7" height="7" rx="1.8"/><rect x="3.5" y="13.5" width="7" height="7" rx="1.8"/><rect x="13.5" y="13.5" width="7" height="7" rx="1.8"/></svg>"#
        }
        "me" => {
            r#"<svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 4.5h12a1 1 0 0 1 1 1V20l-7-4-7 4V5.5a1 1 0 0 1 1-1Z"/></svg>"#
        }
        "plus" => {
            r#"<svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14"/></svg>"#
        }
        "mark" => {
            r#"<svg class="mark-svg" viewBox="0 0 24 24" aria-hidden="true"><rect x="6" y="3.5" width="12" height="17" rx="2.2"/><path d="M8.6 8h6.8M8.6 11.2h4.6"/><circle class="dot" cx="14.8" cy="16.4" r="1.55"/></svg>"#
        }
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
            items,
        }
    }

    #[test]
    fn a_card_is_one_link_with_only_these_things_on_it() {
        let plaza = wall_of(vec![item("brisk-otter-41")]);
        let html = render(&view(&plaza));
        assert!(html.starts_with("<!doctype html>\n<html lang=\"zh-CN\">"));
        assert!(html.contains(
            "<a class=\"tile\" href=\"http://brisk-otter-41.localhost:8443\" data-slug=\"brisk-otter-41\">"
        ));
        assert!(html.contains("新手引导看得懂吗"));
        assert!(html.contains("这次想测"));
        assert!(!html.contains("三关，五分钟，手机上也能玩。"));
        assert!(html.contains("某某"));
        assert!(html.contains("<span class=\"tag\">正在找人测</span>"));
        assert!(html.contains("<span class=\"verb\">试玩"));
        assert!(!html.contains("即刻试玩"));
        assert!(html.contains("6 / 10 位"));
        assert!(!html.contains("人玩过"));
        assert!(!html.contains("还剩"));
        assert!(html.contains("<div class=\"cover word\" style=\"--h:"));
        // 字卡：花押是作品名的第一个字，角上是 slug；作品名本身仍只出现一次。
        assert!(html.contains("><b>小</b><i>brisk-otter-41</i></div>"));
        // 名额那一件事实带一条进度线：6 / 10 就是 60%。
        assert!(
            html.contains("<span class=\"fact seats\" style=\"--p:60%\"><i></i>6 / 10 位</span>")
        );
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
        assert!(html.contains("广场<span class=\"nav-dot\"></span>"));
        assert!(html.contains(&format!("href=\"{}\"", root_paths::ME)));
        assert!(html.contains("我的</a>"));
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
        assert!(!html.contains(">发布作品<"));
        assert!(!html.contains("class=\"toolbar\""));
        assert!(!html.contains("class=\"topbar\""));
        assert!(!html.contains("class=\"breadcrumb\""));
        assert!(!html.contains("点开就玩，不用注册"));
        assert!(html.contains("href=\"#publish-dialog\""));
        assert!(html.contains("从一条命令开始"));
        assert!(html.contains("class=\"mark-svg\""));
        assert!(!html.contains("<span class=\"mark\" aria-hidden=\"true\">p"));
        assert!(!html.contains("class=\"steps\""));
        assert!(!html.contains("--seek"));
        assert!(!html.contains("class=\"dialog-foot\""));
        assert!(html.contains("<details class=\"tip\"><summary"));
        assert!(html.contains("Releases</a> 下载"));
        assert!(html.contains("for=\"tab-static\">导出目录</label>"));
        assert!(html.contains("playtest ./dist --public -m"));
        assert!(!html.contains("href=\"#about-dialog\""));
        assert!(html.contains(&format!(
            "href=\"{DEVELOPER_API_URL}/console/\" target=\"_blank\" rel=\"noopener\">"
        )));
        assert_eq!(html.matches(DEVELOPER_API_URL).count(), 1);
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
        assert!(html.contains("<span class=\"tag ad\">推广</span>"));
        let paid_card = &html[paid..seeking];
        assert!(!paid_card.contains("正在找人测"));
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
        assert_eq!(fact(&it, now), "6 / 10 位");

        it.seeking = false;
        assert_eq!(fact(&it, now), "还剩 16 小时");

        it.expires_at = None;
        assert_eq!(fact(&it, now), "12 人玩过");
        assert!(fact_hint(&it).contains(&PLAYERS_WINDOW_DAYS.to_string()));

        it.players = 0;
        assert_eq!(fact(&it, now), "1 小时前");
        assert!(fact_hint(&it).is_empty());

        it.updated_at = "not-a-date".into();
        assert_eq!(fact(&it, now), "v7");

        it.seeking = true;
        it.seats = None;
        it.updated_at = "2026-09-08T03:00:00Z".into();
        it.expires_at = None;
        it.players = 0;
        assert_eq!(fact(&it, now), "1 小时前");
    }

    /// 关注广场只在「我的」里办。栏上、卡上都不放第二扇门。
    #[test]
    fn following_the_plaza_is_not_on_the_wall() {
        let plaza = wall_of(vec![item("brisk-otter-41")]);
        let html = render(&view(&plaza));
        assert!(!html.contains("有新东西时告诉我"));
        assert!(!html.contains("value=\"plaza\""));
        assert!(!html.contains(&format!("action=\"{}\"", root_paths::FOLLOW)));
        assert!(!html.contains("value=\"site:brisk-otter-41\""));
        assert!(html.contains(&format!("href=\"{}\"", root_paths::ME)));
        assert!(html.contains("我的</a>"));
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
        assert!(!html.contains("class=\"face\""));
        assert!(!html.contains("evil.example"));
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
        assert!(html.contains("<span class=\"verb\">体验"));
        assert!(!html.contains("即刻体验"));
        assert!(!html.contains("class=\"tag\""));
        assert!(!html.contains("这次想测"));
        assert!(!html.contains("class=\"type\""));
        assert!(html.contains("1 小时前"));
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
        assert!(html.contains("广场上还没有作品"));
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
