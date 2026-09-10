//! 广场（DESIGN §3.9）：根域那一页。
//!
//! 内容来自控制面写进对象存储的 `plaza.json`（DESIGN §4.5），这里只读、只渲染。
//! 控制面挂了照常能翻，只是人数旧几分钟。
//!
//! 整页服务端直出，**一行脚本都没有**：左边一条栏（品牌、广场、我的、发布作品），
//! 右边一面网格。广场是当前项。没有序号、没有口号、没有关注入口——关注只在「我的」
//! 里办。没有分段标题、没有筛选、没有排序、没有先讲这是什么的介绍段。每张卡是一个
//! 链接，点哪都进门禁页；关注、求测的话、举报都在门禁页上，卡是让人点进去的，不是
//! 让人在墙上办事的。
//!
//! 这一页是我们自己的，没有用户脚本，所以能带严格的 CSP（`app.rs`）；封面来自各作品自己的子域，
//! 头像来自 GitHub——CSP 的 `img-src` 里只多这一个来源。

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use playtest_common::follow::root_paths;
use playtest_common::plaza::{Plaza, PlazaItem, PLAYERS_WINDOW_DAYS};
use playtest_common::store::FsStore;
use playtest_common::DEVELOPER_API_URL;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::html::{esc, hue, page};

/// `plaza.json` 的缓存寿命。控制面最快也是每次变动重写一份，30 秒内的旧数据没人分得出来。
pub const TTL: Duration = Duration::from_secs(30);

pub struct PlazaCache {
    store: FsStore,
    cached: Mutex<Option<(Instant, Arc<Plaza>)>>,
    ttl: Duration,
}

impl PlazaCache {
    pub fn new(store: FsStore) -> Self {
        Self::with_ttl(store, TTL)
    }

    pub fn with_ttl(store: FsStore, ttl: Duration) -> Self {
        Self {
            store,
            cached: Mutex::new(None),
            ttl,
        }
    }

    /// 读不到（控制面还没写过、或文件坏了）就当作空的：广场页照常出，只是没有卡片。
    pub async fn get(&self) -> Arc<Plaza> {
        if let Some(hit) = self.cached.lock().ok().and_then(|c| {
            c.as_ref()
                .filter(|(at, _)| at.elapsed() < self.ttl)
                .map(|(_, p)| p.clone())
        }) {
            return hit;
        }
        let plaza = match self.store.get_plaza().await {
            Ok(Some(p)) => p,
            Ok(None) => empty(),
            Err(err) => {
                tracing::warn!(%err, "读 plaza.json 失败，广场按空的出");
                empty()
            }
        };
        let plaza = Arc::new(plaza);
        if let Ok(mut cache) = self.cached.lock() {
            *cache = Some((Instant::now(), plaza.clone()));
        }
        plaza
    }
}

fn empty() -> Plaza {
    Plaza {
        schema: playtest_common::plaza::SCHEMA,
        generated_at: String::new(),
        club_followers: 0,
        items: Vec::new(),
    }
}

/// 渲染时要知道的几样。
pub struct View<'a> {
    pub plaza: &'a Plaza,
    pub now: OffsetDateTime,
}

pub fn render(view: &View<'_>) -> String {
    let head =
        "<meta name=\"description\" content=\"正在找人试玩的作品。点开就能玩，不用注册。\">\n\
<meta property=\"og:title\" content=\"playtest · 正在找人试玩的作品\">\n\
<meta property=\"og:description\" content=\"开发者把手上能玩的版本放到这里，路过的人点开就玩。\">\n\
<meta property=\"og:type\" content=\"website\">\n";

    wrap(
        "playtest · 正在找人试玩的作品",
        head,
        Here::Plaza,
        &format!("{wall}{foot}", wall = wall(view), foot = foot()),
    )
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
        PLAZA_CSS,
        &format!(
            "{rail}<main class=\"wall\">\n{main}</main>\n",
            rail = rail(here)
        ),
    )
}

/// 左边那条栏：品牌、两间房、一扇门。手机上收成顶上一条（样式里做）。
///
/// 广场和我的是这一域上的两间房，当前那一间标出来。没有口号、没有序号。
/// 「发布作品」去开发者域——这是玩家路径上唯一那个例外链接（AGENTS 第 7 条），
/// 所以页脚不再放第二个。关注广场只在「我的」里办（DESIGN §3.9）。
fn rail(here: Here) -> String {
    let plaza = match here {
        Here::Plaza => "<a class=\"nav-item on\" href=\"/\" aria-current=\"page\">广场</a>",
        Here::Mine => "<a class=\"nav-item\" href=\"/\">广场</a>",
    };
    let mine = match here {
        Here::Plaza => format!("<a class=\"nav-item\" href=\"{}\">我的</a>", root_paths::ME),
        Here::Mine => format!(
            "<a class=\"nav-item on\" href=\"{}\" aria-current=\"page\">我的</a>",
            root_paths::ME
        ),
    };
    format!(
        "<aside class=\"rail\">\n\
<div class=\"rail-top\"><a class=\"brand\" href=\"/\">playtest<span>.run</span></a></div>\n\
<nav class=\"nav\" aria-label=\"页面\">{plaza}{mine}\
<a class=\"nav-item\" href=\"{dev}\" rel=\"noopener\">发布作品<u>\u{2197}</u></a></nav>\n\
</aside>\n",
        dev = DEVELOPER_API_URL,
    )
}

/// 右边那面墙：一面网格。顺序是推广位（最多几张、永远带标）、正在找人测的、其余按时间。
/// 一个作品只出现一次；推广位有限，多出来的按普通作品排——广场不因为付了钱就变长。
fn wall(view: &View<'_>) -> String {
    if view.plaza.items.is_empty() {
        return "<section class=\"empty-plaza\"><p>广场上还没有作品。</p>\
<p class=\"lead\"><code>playtest ./dist --public</code> 会把作品放到这里。</p></section>\n"
            .to_string();
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

    let mut out = String::from("<div class=\"grid\">\n");
    // 「推广」标只给真占着推广位的那几张；排到位子外面的按普通作品渲染。
    for item in &boosted {
        out.push_str(&tile(item, true, view.now));
    }
    for item in seeking.chain(rest) {
        out.push_str(&tile(item, false, view.now));
    }
    out.push_str("</div>\n");
    out
}

/// 一张卡，整张是一个链接。上面只有：插画窗、最多一个标、一个动词、作品名、一句话、
/// 开发者、一件事实（DESIGN §3.9）。`on_slot` 是它此刻是否真占着一个推广位。
///
/// 作品名永远在卡面上，只出现一次。插画窗里要么是封面图，要么是一块色田——
/// 色田里不写字，避免和下面的标题印两遍。
fn tile(item: &PlazaItem, on_slot: bool, now: OffsetDateTime) -> String {
    // 链接是控制面按模板拼的，不是用户输入；这一道只是不让一份写坏的 plaza.json 变成 `javascript:`。
    let url = if item.url.starts_with("https://") || item.url.starts_with("http://") {
        esc(&item.url)
    } else {
        "#".to_string()
    };
    let title = esc(&item.title);

    let cover = match &item.cover_url {
        Some(cover) => format!(
            "<img class=\"cover\" src=\"{}\" alt=\"\" loading=\"lazy\" decoding=\"async\">",
            esc(cover)
        ),
        // 没封面：插画窗是一块色田。色相由 slug 决定；字写在下面的卡面上。
        None => format!(
            "<div class=\"cover word\" style=\"--h:{}\"></div>",
            hue(&item.slug)
        ),
    };
    // 封面上最多一个标：「推广」是一句不得不说的实话，「正在找人测」是行动的邀请。
    // 同一种纸签；求测用字标那一系绿，推广只用灰——付了钱的位置不能长得像这个地方在邀请你。
    let tag = if on_slot {
        "<span class=\"tag ad\">推广</span>"
    } else if item.seeking {
        "<span class=\"tag\">正在找人测</span>"
    } else {
        ""
    };
    // 右下角那个动词：引擎导出物是「试玩」，其余是「体验」——和门禁页那句「邀请你试玩」同一套词。
    // 它顺便说了这是什么，所以卡上不再有「游戏 / 体验」的品类标。能悬停的设备上它是悬停时
    // 才浮出来的那一下（整张卡本来就是链接，不用一直举着牌子）；触屏上一直在。
    let verb = if item.is_game { "试玩" } else { "体验" };

    // 一行简介。没有简介时退到求测的那句话；两个都没有也留着这一行，一排卡的高度才齐。
    let line = item
        .summary
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            item.seek_note
                .as_deref()
                .map(str::trim)
                .filter(|s| item.seeking && !s.is_empty())
        })
        .map(esc)
        .unwrap_or_default();

    let avatar = match item
        .avatar_url
        .as_deref()
        .filter(|u| u.starts_with("https://"))
    {
        // 一张脸比一个 ID 更像真人（DESIGN §3.9）。外站图片：不带 referrer、懒加载；
        // 加载不出来就只剩名字，卡不塌。
        Some(url) => format!(
            "<img class=\"face\" src=\"{}\" alt=\"\" width=\"18\" height=\"18\" \
loading=\"lazy\" decoding=\"async\" referrerpolicy=\"no-referrer\">",
            esc(url)
        ),
        None => String::new(),
    };

    format!(
        "<a class=\"tile\" href=\"{url}\" data-slug=\"{slug}\">\n\
<div class=\"shot\">{cover}{tag}<span class=\"verb\">{verb}<u>\u{2197}</u></span></div>\n\
<div class=\"tile-body\"><h2>{title}</h2><p class=\"summary\">{line}</p>\
<p class=\"meta\"><span class=\"who\">{avatar}{developer}</span>{fact}</p></div>\n\
</a>\n",
        slug = esc(&item.slug),
        developer = esc(&item.developer),
        fact = fact(item, now),
    )
}

/// 卡底右边那一件事实。按这个顺序取第一个成立的，一张卡上不并排放两个数字：
/// 名额进度（在找人且设了名额）→ 还剩多久（匿名作品）→ 7 天内玩过的人数（0 不说）→ 版本号。
///
/// 名额说的是进度不是稀缺：「6 / 10 位」，不写「仅剩 4 席」。0 人玩过一个字不说——
/// 「还没人玩过」是一句替作品道歉的话（DESIGN §3.5 的规矩在这里同样成立）。
fn fact(item: &PlazaItem, now: OffsetDateTime) -> String {
    if item.seeking {
        if let Some(seats) = item.seats.filter(|s| *s > 0) {
            return format!(
                "<span class=\"fact seats\">{} / {seats} 位</span>",
                item.joined
            );
        }
    }
    if let Some(at) = item
        .expires_at
        .as_deref()
        .and_then(|raw| OffsetDateTime::parse(raw, &Rfc3339).ok())
    {
        return format!("<span class=\"fact\">{}</span>", esc(&remaining(at, now)));
    }
    if item.players > 0 {
        return format!(
            "<span class=\"fact\" title=\"最近 {PLAYERS_WINDOW_DAYS} 天里点了「开始」的人数\">{} 人玩过</span>",
            item.players
        );
    }
    format!("<span class=\"fact\">v{}</span>", item.version)
}

/// 页脚一行说完，但该如实说的都在（DESIGN §3.9）：只放主动公开的、不排名次、
/// 推广位永远标出来、匿名作品 24 小时后自己下来。开发者的入口在栏上，这里不重复。
fn foot() -> String {
    "<footer class=\"page-foot\"><p>只放开发者主动公开的作品，按时间排 · 推广位永远标出来 · \
没登录发的作品 24 小时后自己下来</p></footer>\n"
        .to_string()
}

/// 「还剩 5 小时」。到期时间是匿名作品才有的，广场页缓存 30 秒，误差一分钟以内。
fn remaining(expires: OffsetDateTime, now: OffsetDateTime) -> String {
    let left = expires - now;
    let minutes = left.whole_minutes();
    if minutes <= 0 {
        "即将下线".to_string()
    } else if minutes < 60 {
        format!("还剩 {minutes} 分钟")
    } else {
        format!("还剩 {} 小时", left.whole_hours())
    }
}

/// 广场自己的那段样式，接在共用样式后面。
///
/// 版式（DESIGN §3.9）：桌面上左边一条 14.5rem 的栏贴着屏幕不动，右边一面网格滚动。
/// 卡是竖版、比墙亮一档的黑，插画窗在上、文字在下。手机两列，平板三列，桌面四到五列。
///
/// 借参考图的三样：统一物件贴在墙上、颜色只出现在插画窗、排密。
/// 不做法力、规则栏、宝石。卡有顶沿高光和很轻的落影，悬停抬起一档——它们是物件，不是面板。
/// `--run` 是已经张开的叶子，只给字标、求测标的墨、动词箭头。
/// 推广标不碰这支绿。
///
/// 和共用样式一样不写 CSS 注释（会随每一页发出去）。要解释的几处：
/// - `.rail` 在桌面上是 `position:sticky` 加 `height:100vh`：栏不随墙滚动。
/// - `.tile` 自己写字色：共用的 `a{color:var(--dim)}` 太暗，黑卡上要抬起来。
/// - `.tag` 仍是窗上的纸签，才能压住任意封面；求测用 `--run`，推广只用灰。
/// - `.brand span` 用 `--run`，不铺到卡上。
/// - `.verb`：能悬停的设备上默认透明、悬停浮现；`(hover:none)` 的触屏一直显示。
/// - `.fact.seats` 要把共用样式里 `.seats`（门禁页那行名额）的字号、边距和金色压回来。
const PLAZA_CSS: &str = "\
body.page{--run:#58d98a;display:grid;grid-template-columns:minmax(0,1fr);min-height:100vh;padding:0;background:#08070a}\
@media(min-width:64rem){body.page{grid-template-columns:14.5rem minmax(0,1fr)}}\
.rail{display:flex;flex-wrap:wrap;align-items:center;gap:10px 22px;padding:16px 20px;border-bottom:1px solid #1a1816}\
@media(min-width:64rem){.rail{position:sticky;top:0;align-self:start;height:100vh;flex-direction:column;flex-wrap:nowrap;\
align-items:stretch;gap:0;padding:30px 24px 26px;border-bottom:0;border-right:1px solid #1a1816}}\
.rail-top{margin-right:auto}\
.brand{display:inline-block;font-size:1.05rem;font-weight:700;letter-spacing:-.02em;line-height:1.2;color:var(--fg);text-decoration:none}\
.brand span{color:var(--run)}\
.brand:hover{color:var(--fg);text-decoration:none}\
.nav{display:flex;gap:2px 18px}\
@media(min-width:64rem){.nav{flex-direction:column;gap:0;margin:40px 0 0}}\
.nav-item{display:flex;align-items:baseline;gap:12px;padding:8px 0;color:var(--dim);text-decoration:none;\
font-size:.9rem;font-weight:500;letter-spacing:.01em;white-space:nowrap;transition:color .15s}\
.nav-item:hover{color:var(--fg);text-decoration:none}\
.nav-item.on{color:var(--fg)}\
.nav-item u{text-decoration:none;margin-left:3px;font-size:.7rem;color:#4f5664}\
.wall{min-width:0;width:100%;max-width:100rem;padding:20px 16px 40px}\
@media(min-width:40rem){.wall{padding:26px 22px 44px}}\
@media(min-width:64rem){.wall{padding:32px 32px 48px}}\
.grid{display:grid;gap:12px;grid-template-columns:repeat(2,minmax(0,1fr))}\
@media(min-width:48rem){.grid{grid-template-columns:repeat(3,minmax(0,1fr));gap:14px}}\
@media(min-width:76rem){.grid{grid-template-columns:repeat(4,minmax(0,1fr))}}\
@media(min-width:110rem){.grid{grid-template-columns:repeat(5,minmax(0,1fr))}}\
.tile{display:flex;flex-direction:column;min-width:0;padding:8px 8px 10px;border-radius:8px;color:#efeae2;text-decoration:none;\
background:linear-gradient(180deg,rgba(255,244,220,.05),transparent 28%),\
repeating-linear-gradient(-32deg,transparent,transparent 3px,rgba(255,255,255,.014) 3px,rgba(255,255,255,.014) 4px),#16141a;\
box-shadow:0 16px 36px -22px rgba(0,0,0,.95),inset 0 1px 0 rgba(255,244,220,.07),inset 0 0 0 1px rgba(255,255,255,.045);\
transition:transform .18s,box-shadow .18s}\
.tile:hover{color:#efeae2;text-decoration:none;transform:translateY(-3px);\
box-shadow:0 22px 40px -18px rgba(0,0,0,.96),inset 0 1px 0 rgba(255,244,220,.09),inset 0 0 0 1px rgba(255,255,255,.06)}\
.shot{position:relative;overflow:hidden;border-radius:5px;background:#1a1816}\
.cover{display:block;width:100%;aspect-ratio:1/1;object-fit:cover;transition:transform .5s ease}\
.tile:hover img.cover{transform:scale(1.03)}\
.cover.word{background:radial-gradient(90% 80% at 28% 18%,hsl(var(--h) 54% 64%),transparent 58%),\
linear-gradient(165deg,hsl(var(--h) 52% 50%),hsl(var(--h) 48% 32%))}\
.tag,.verb{position:absolute;padding:2px 8px;border-radius:999px;font-size:.66rem;line-height:1.45;letter-spacing:.03em}\
.tag{top:8px;left:8px;font-weight:650;color:var(--run);background:rgba(88,217,138,0.12);border:1px solid rgba(88,217,138,0.25);backdrop-filter:blur(4px);-webkit-backdrop-filter:blur(4px)}\
.tag.ad{font-weight:500;color:#8c8680;background:rgba(255,255,255,0.06);border:1px solid rgba(255,255,255,0.1)}\
.verb{right:8px;bottom:8px;font-family:var(--mono);background:#0c0b0a;color:#efeae2;opacity:0;transform:translateY(3px);\
transition:opacity .2s,transform .2s}\
.verb u{text-decoration:none;margin-left:4px;color:var(--run)}\
.tile:hover .verb{opacity:1;transform:none}\
@media(hover:none){.verb{opacity:1;transform:none}}\
.tile-body{display:flex;flex:1;flex-direction:column;gap:3px;padding:10px 4px 2px}\
.tile-body h2{margin:0;font-size:.95rem;line-height:1.3;font-weight:650;letter-spacing:-.01em;color:#efeae2;\
display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden}\
.tile .summary{margin:0;min-height:1.4em;font-size:.78rem;line-height:1.4;color:#8c8680;\
white-space:nowrap;overflow:hidden;text-overflow:ellipsis}\
.tile .meta{margin:8px 0 0;display:flex;justify-content:space-between;align-items:center;gap:8px;text-align:left;\
font-family:var(--mono);font-size:.66rem;color:#6e6a64;font-variant-numeric:tabular-nums}\
.tile-body .meta{margin-top:auto;padding-top:8px}\
.who{display:inline-flex;align-items:center;gap:6px;min-width:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;color:#8c8680}\
.face{width:16px;height:16px;flex:0 0 auto;border-radius:50%;background:#2a2724;object-fit:cover}\
.fact{flex:0 0 auto}\
.fact.seats{margin:0;font-size:inherit;color:#8c8680}\
.empty-plaza,.drawer{max-width:30rem;margin:48px 0 0}\
.empty-plaza p{margin:0}\
.empty-plaza .lead,.drawer .lead{margin-top:10px}\
.drawer h1{margin:0;font-size:1.35rem;font-weight:650;letter-spacing:-.02em}\
.drawer footer{margin-top:28px}
.page-foot{display:block;margin:36px 0 0;padding:14px 0 0;border-top:1px solid #1a1816;\
font-size:.74rem;line-height:1.8;color:#6b7280}\
.page-foot p{margin:0}";

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
            cover_url: None,
            players: 12,
            seeking: true,
            seek_note: Some("新手引导看得懂吗".into()),
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
        // 整张卡是一个链接，点哪都进门禁页。
        assert!(html.contains(
            "<a class=\"tile\" href=\"http://brisk-otter-41.localhost:8443\" data-slug=\"brisk-otter-41\">"
        ));
        assert!(html.contains("<p class=\"summary\">三关，五分钟，手机上也能玩。</p>"));
        assert!(html.contains("<span class=\"who\">某某</span>"));
        // 插画窗上最多一个标、一个动词。
        assert!(html.contains("<span class=\"tag\">正在找人测</span>"));
        assert!(html.contains("<span class=\"verb\">试玩<u>\u{2197}</u></span>"));
        // 一件事实：在找人且设了名额，所以是名额进度——人数和到期都不并排出现。
        assert!(html.contains("<span class=\"fact seats\">6 / 10 位</span>"));
        assert!(!html.contains("人玩过"));
        assert!(!html.contains("还剩"));
        // 没封面：插画窗是色田，作品名只在卡面上出现一次。
        assert!(html.contains("<div class=\"cover word\" style=\"--h:"));
        assert!(!html.contains("cover word\"><h2"));
        assert!(html.contains("<div class=\"tile-body\"><h2>小球大冒险</h2>"));
        assert_eq!(html.matches("小球大冒险").count(), 1, "作品名只出现一次");
        assert!(!html.contains("PLAYTEST"));
        assert!(!html.contains("<img class=\"cover\""));
        // 卡上没有的（DESIGN §3.9）：关注、想玩、举报、求测的那句话、关注数、版本日期组合、品类标。
        for word in [
            "想玩",
            "举报",
            "新手引导看得懂吗",
            "人关注</span>",
            "value=\"site:",
            ">游戏<",
            "<span class=\"v\">",
        ] {
            assert!(!html.contains(word), "卡上不该有「{word}」");
        }
        // 该如实说的在页脚，一行说完。
        assert!(html.contains("没登录发的作品 24 小时后自己下来"));
        assert!(html.contains("推广位永远标出来"));
        assert!(html.contains("只放开发者主动公开的作品"));
    }

    /// 这一页一行脚本都没有：服务端直出，CSP 锁死，没有 JS 的浏览器看到的就是全部。
    #[test]
    fn the_wall_has_no_script_at_all() {
        let plaza = wall_of(vec![item("a"), item("b"), item("c")]);
        let html = render(&view(&plaza));
        assert!(!html.contains("<script"));
        assert!(!html.contains("localStorage"));
        // 栏上两间房：广场是当前项，我的在旁边。没有序号、没有口号。
        // 开发者的那个是玩家路径上唯一的例外链接，页脚不再放第二个。
        assert!(html.contains("aria-current=\"page\">广场</a>"));
        assert!(html.contains(&format!("href=\"{}\">我的</a>", root_paths::ME)));
        assert!(!html.contains("<i>01</i>"));
        assert!(!html.contains("class=\"tagline\""));
        assert!(!html.contains("点开就玩，不用注册"));
        assert!(html.contains(&format!(
            "href=\"{DEVELOPER_API_URL}\" rel=\"noopener\">发布作品"
        )));
        assert_eq!(html.matches(DEVELOPER_API_URL).count(), 1);
        // 没有筛选、排序、分段标题这些后台列表的东西。
        for word in [
            "最多人玩",
            "筛选",
            "排序",
            "data-band",
            "class=\"chip\"",
            "最近更新</h2>",
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
        plain.seek_note = None;
        let plaza = wall_of(vec![plain, ad, item("seeking-one")]);
        let html = render(&view(&plaza));

        let paid = html.find("data-slug=\"paid-one\"").expect("推广");
        let seeking = html.find("data-slug=\"seeking-one\"").expect("找人测");
        let quiet = html.find("data-slug=\"quiet-one\"").expect("其余");
        assert!(paid < seeking && seeking < quiet);
        assert_eq!(html.matches("data-slug=\"paid-one\"").count(), 1);
        assert!(html.contains("<span class=\"tag ad\">推广</span>"));
        // 推广标不能长得像玩家的邀请：推广的那张卡上没有「正在找人测」。
        let paid_card = &html[paid..seeking];
        assert!(!paid_card.contains("正在找人测"));
        // 没标的卡就没有标。
        let quiet_card = &html[quiet..];
        assert!(!quiet_card.contains("<span class=\"tag"));
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
        assert!(!html.contains("class=\"intro\""));
        assert!(!html.contains("这里是开发者手上正在做的东西"));
        assert!(!html.contains("作品页上有地方留话"));
    }

    /// 一件事实的顺序：名额 → 还剩多久 → 人数 → 版本号。
    #[test]
    fn one_fact_per_card_in_this_order() {
        let now = datetime!(2026-09-08 04:00:00 UTC);
        let mut it = item("x");
        assert_eq!(
            fact(&it, now),
            "<span class=\"fact seats\">6 / 10 位</span>"
        );

        // 不在找人：名额哪怕设了也不说，退到到期时间。
        it.seeking = false;
        assert_eq!(fact(&it, now), "<span class=\"fact\">还剩 16 小时</span>");

        // 登录用户的作品没有到期：说人数。
        it.expires_at = None;
        assert!(fact(&it, now).contains(">12 人玩过</span>"));

        // 0 人玩过一个字不说，退到版本号。
        it.players = 0;
        assert_eq!(fact(&it, now), "<span class=\"fact\">v7</span>");

        // 在找人但没设名额：名额那一档不成立，往下走。
        it.seeking = true;
        it.seats = None;
        assert_eq!(fact(&it, now), "<span class=\"fact\">v7</span>");
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
        assert!(html.contains(&format!("href=\"{}\">我的</a>", root_paths::ME)));
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
        assert!(!render(&view(&plaza)).contains("class=\"face\""));
    }

    #[test]
    fn a_cover_is_an_image_and_a_non_game_says_experience() {
        let mut it = item("wise-mink-28");
        it.cover_url = Some("http://wise-mink-28.localhost:8443/_playtest/cover?v=abcd1234".into());
        it.is_game = false;
        it.seeking = false;
        it.players = 0;
        it.expires_at = None;
        it.followers = 0;
        it.summary = None;
        let plaza = wall_of(vec![it]);
        let html = render(&view(&plaza));
        assert!(html.contains(
            "<img class=\"cover\" src=\"http://wise-mink-28.localhost:8443/_playtest/cover?v=abcd1234\""
        ));
        // 有封面：作品名在卡面上，而且只这一处。
        assert!(html.contains("<div class=\"tile-body\"><h2>小球大冒险</h2>"));
        assert_eq!(html.matches("小球大冒险").count(), 1);
        assert!(!html.contains("cover word"));
        assert!(html.contains("<span class=\"verb\">体验<u>"));
        // 没在找人：没有标；求测的那句话也不拿来当简介，那一行空着但还在（一排卡高度才齐）。
        assert!(!html.contains("<span class=\"tag"));
        assert!(html.contains("<p class=\"summary\"></p>"));
        // 0 人玩过、没到期：事实那一格是版本号。
        assert!(html.contains("<span class=\"fact\">v7</span>"));
        assert!(!html.contains("人玩过"));
    }

    #[test]
    fn everything_from_the_feed_is_escaped() {
        let mut it = item("x");
        it.title = "<img src=x onerror=alert(1)>".into();
        it.developer = "\"><script>alert(2)</script>".into();
        it.summary = Some("</p><script>alert(3)</script>".into());
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
        let plaza = empty();
        let html = render(&view(&plaza));
        assert!(html.contains("广场上还没有作品"));
        assert!(!html.contains("class=\"grid\""));
        assert!(html.contains("--public"));
        // 栏照常在：开发者从这里去发布。
        assert!(html.contains(DEVELOPER_API_URL));
    }

    #[test]
    fn remaining_reads_like_a_person() {
        let now = datetime!(2026-09-08 04:00:00 UTC);
        assert_eq!(
            remaining(datetime!(2026-09-08 04:30:00 UTC), now),
            "还剩 30 分钟"
        );
        assert_eq!(
            remaining(datetime!(2026-09-08 09:59:00 UTC), now),
            "还剩 5 小时"
        );
        assert_eq!(
            remaining(datetime!(2026-09-08 03:00:00 UTC), now),
            "即将下线"
        );
    }

    #[tokio::test]
    async fn cache_serves_empty_when_nothing_was_written() {
        let dir = tempfile::tempdir().unwrap();
        let cache = PlazaCache::new(FsStore::new(dir.path()));
        assert!(cache.get().await.items.is_empty());
    }
}
