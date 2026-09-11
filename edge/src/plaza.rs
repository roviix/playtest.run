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

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use playtest_common::follow::root_paths;
use playtest_common::plaza::{Plaza, PlazaItem, PLAYERS_WINDOW_DAYS};
use playtest_common::project::{Blurb, Fact};
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
        PLAZA_CSS,
        &format!(
            "{rail}<main class=\"main\">\n{main}</main>\n{publish}",
            rail = rail(here),
            publish = publish_sheet(),
        ),
    )
}

/// 左边那条栏：字标、两间房、栏底几行字。手机上收成顶上一条（样式里做）。
///
/// 栏上只有字，没有图标——图标是后台的记号，这里是一面墙。
/// 发布只走栏底那几行。说明里那条控制台链接，才是去开发者域的那一扇门。
fn rail(here: Here) -> String {
    let plaza = match here {
        Here::Plaza => {
            "<a class=\"nav-item active\" href=\"/\" aria-current=\"page\">广场<span class=\"nav-dot\"></span></a>"
        }
        Here::Mine => "<a class=\"nav-item\" href=\"/\">广场</a>",
    };
    let mine = match here {
        Here::Plaza => format!("<a class=\"nav-item\" href=\"{}\">我的</a>", root_paths::ME),
        Here::Mine => format!(
            "<a class=\"nav-item active\" href=\"{}\" aria-current=\"page\">我的<span class=\"nav-dot\"></span></a>",
            root_paths::ME
        ),
    };
    format!(
        "<aside class=\"sidebar\">\n\
<a class=\"brand\" href=\"/\" aria-label=\"playtest.run 首页\">\
<span class=\"wordmark\">playtest<span>.run</span></span></a>\n\
<nav class=\"nav\" aria-label=\"页面\">{plaza}{mine}</nav>\n\
<div class=\"sidebar-bottom\">\n\
<div class=\"publish-note\"><h3>不必等到「做完」。</h3>\
<p>把手上能玩的版本发出来，让真实的玩家试试看。</p>\
<a href=\"#publish-dialog\">从一条命令开始<u>\u{2197}</u></a></div>\n\
</div>\n\
</aside>\n"
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
            Ok(at) => remaining(at, now),
            Err(_) => format!("v{}", item.version),
        },
        Fact::Players { count } => format!("{count} 人玩过"),
        Fact::Updated { at } => match OffsetDateTime::parse(&at, &Rfc3339) {
            Ok(updated) => ago(updated, now),
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
<div class=\"dialog-head\"><h2 id=\"publish-title\">把这个版本，放到别人面前。</h2>\
<a class=\"close\" href=\"#\" aria-label=\"关闭\">{close}</a></div>\n\
<p class=\"dialog-desc\">安装 CLI 后，在终端运行一条命令。加上你想测的内容，让路过的人知道怎么帮你。</p>\n\
<div class=\"pub-box\">\n\
<div class=\"tabs\" role=\"tablist\" aria-label=\"发布方式\">\n\
<input type=\"radio\" name=\"pub-mode\" id=\"tab-static\" checked>\n\
<label for=\"tab-static\">导出目录</label>\n\
<input type=\"radio\" name=\"pub-mode\" id=\"tab-local\">\n\
<label for=\"tab-local\">本地服务</label>\n\
<input type=\"radio\" name=\"pub-mode\" id=\"tab-backend\">\n\
<label for=\"tab-backend\">带后端的应用</label>\n\
</div>\n\
<div class=\"codebox\" id=\"panel-static\"><b>$</b><code>playtest ./dist --public --seek \"想请你试试，第一次玩顺不顺畅\"</code></div>\n\
<div class=\"codebox\" id=\"panel-local\"><b>$</b><code>playtest 5173 --public --seek \"帮我看看，本地这个版本跑得怎么样\"</code></div>\n\
<div class=\"codebox\" id=\"panel-backend\"><b>$</b><code>playtest ./dist --backend 3000 --public --seek \"想测一下联机是否顺畅\"</code></div>\n\
</div>\n\
<p class=\"copy-hint\">公开展示由你决定，也可以随时 unlist。选中命令即可复制。</p>\n\
<p class=\"dialog-hint\">发布后，前往 <a href=\"{dev}/console/\" target=\"_blank\" rel=\"noopener\">开发者控制台</a> 查看加载、报错与玩家反馈。</p>\n\
</div>\n\
</div>\n",
        close = icon("close"),
        dev = DEVELOPER_API_URL,
    )
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

fn ago(updated: OffsetDateTime, now: OffsetDateTime) -> String {
    let span = now - updated;
    let minutes = span.whole_minutes();
    if minutes < 1 {
        "刚刚发布".to_string()
    } else if minutes < 60 {
        format!("{minutes} 分钟前")
    } else if span.whole_hours() < 24 {
        format!("{} 小时前", span.whole_hours())
    } else {
        format!("{} 天前", span.whole_days().max(1))
    }
}

/// 发布说明右上角那个 ×。画在页面里，不靠外部字体、也不靠 `<use href>`
/// （CSP `default-src 'none'` 会把同页 fragment 的引用挡掉）。整页只有这一个图标。
fn icon(name: &str) -> &'static str {
    match name {
        "close" => {
            r#"<svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="m6 6 12 12M6 18 18 6"/></svg>"#
        }
        _ => "",
    }
}

/// 广场自己的那段样式，接在共用样式后面。
///
/// 版式（DESIGN §3.9）：桌面上左边一条栏贴着屏幕不动，右边一面网格。
/// 三级面：墙、栏、卡，各差一两档，靠发丝线和上沿一道高光分层。卡是一件物件：
/// 一块面、一圈发丝线、嵌在里面的 4:3 插画窗、窗下的字、底下一条发丝线分出「谁 · 一件事实」。
/// 颜色只有共用样式里那一套：暖炭、象牙、一支琥珀。
///
/// 和共用样式一样不写 CSS 注释（会随每一页发出去）。要解释的几处：
/// - `.sidebar` 桌面 `position:fixed`：栏不随墙滚动。
/// - `body.page a.tile` 写全选择器：共用的 `a{color:var(--dim)}` 会把卡上的字压暗。
/// - `.cover.word` 四层背景：两层细网格、一团光、一层底色，色相都来自 `--h`；
///   `b` 是花押、`i` 是角上的 slug。
/// - `.tag::before` 是「正在找人测」前面那个发光的点；推广标没有点。
/// - `.fact i` 是名额那一件事实旁边 28px 的进度线，`--p` 是百分比。
/// - `.verb` 能悬停才出来；触屏（`hover:none`）上一直在；键盘焦点落到卡上也出来。
/// - 发布说明桌面居中、手机从底部升起，都是 `:target`，没有脚本；`.codebox b` 是提示符。
const PLAZA_CSS: &str = r#"
body.page{display:block;min-height:100vh;padding:0;margin:0;background:var(--bg);color:var(--fg)}
body.page a{color:inherit;text-decoration:none}
body.page a:hover{color:inherit}
body.page button,body.page input{font:inherit}
body.page button{display:inline-flex;width:auto;min-height:0;margin:0;padding:0;border:0;border-radius:0;background:none;color:inherit;font-size:inherit;font-weight:inherit;letter-spacing:inherit;filter:none;transform:none;cursor:pointer}
body.page p{margin:0}
body.page footer{display:block;margin:0;padding:0;border:0;font-size:inherit;color:inherit}
.icon{width:14px;height:14px;fill:none;stroke:currentColor;stroke-width:1.8;stroke-linecap:round;stroke-linejoin:round;flex-shrink:0}
.sidebar{position:fixed;inset:0 auto 0 0;width:15.5rem;display:flex;flex-direction:column;padding:26px 18px 20px;border-right:1px solid var(--line);background:var(--rail);z-index:10}
.brand{display:inline-block;width:fit-content;margin:0 6px;padding:6px 0;border-radius:6px}
.wordmark{font-size:21px;font-weight:700;letter-spacing:-.035em;line-height:1;color:var(--fg)}
.wordmark span{color:var(--dim);font-weight:400}
.nav{display:flex;flex-direction:column;gap:3px;margin:38px 0 0}
.nav-item{display:flex;align-items:center;min-height:42px;padding:0 12px;border-radius:10px;color:var(--dim);font-size:14px;font-weight:500;transition:color .15s,background .15s,box-shadow .15s}
.nav-item:hover{color:var(--fg);background:#ffffff08}
.nav-item.active{color:var(--fg);background:#ffffff0a;box-shadow:inset 0 0 0 1px var(--line),inset 0 1px 0 #ffffff0a}
.nav-dot{width:6px;height:6px;margin-left:auto;border-radius:50%;background:var(--accent);box-shadow:0 0 0 3px var(--accent-soft),0 0 10px var(--accent)}
.sidebar-bottom{margin-top:auto}
.publish-note{padding:18px 18px 16px;border-radius:14px;background:linear-gradient(165deg,#18181c,#101013);box-shadow:inset 0 0 0 1px var(--line),inset 0 1px 0 #ffffff0d}
.publish-note h3{margin:0;font-size:13.5px;font-weight:600;color:var(--fg)}
.publish-note p{margin:6px 0 0;font-size:12.5px;line-height:1.65;color:var(--dim)}
body.page .publish-note a{display:inline-flex;align-items:center;gap:7px;min-height:36px;margin-top:16px;padding:0 14px;border-radius:999px;background:var(--fg);color:var(--bg);font-size:12.5px;font-weight:600;transition:transform .15s,filter .15s}
body.page .publish-note a:hover{filter:brightness(1.05);transform:translateY(-1px)}
.publish-note a u{text-decoration:none;opacity:.55}
.main{margin-left:15.5rem;min-height:100vh;background:radial-gradient(80% 36% at 50% -8%,#18140c 0%,transparent 70%)}
.content{max-width:112rem;padding:28px 32px 96px}
.grid{display:grid;gap:20px;grid-template-columns:repeat(2,minmax(0,1fr))}
@media(min-width:48rem){.grid{grid-template-columns:repeat(3,minmax(0,1fr))}}
@media(min-width:80rem){.grid{grid-template-columns:repeat(4,minmax(0,1fr))}}
@media(min-width:112rem){.grid{grid-template-columns:repeat(5,minmax(0,1fr))}}
body.page a.tile{display:flex;flex-direction:column;min-width:0;padding:6px;border-radius:16px;background:var(--card);color:var(--fg);box-shadow:inset 0 0 0 1px var(--line),inset 0 1px 0 #ffffff0c;outline-offset:4px;transition:transform .3s cubic-bezier(.2,.7,.2,1),background .3s,box-shadow .3s}
body.page a.tile:hover{transform:translateY(-3px);background:var(--card2);box-shadow:inset 0 0 0 1px var(--line2),inset 0 1px 0 #ffffff14,0 24px 48px -24px #000,0 8px 18px -10px #000c}
.shot{position:relative;overflow:hidden;aspect-ratio:4/3;border-radius:11px;background:#0e0e10;box-shadow:inset 0 0 0 1px #ffffff0a}
.cover{position:absolute;inset:0;display:block;width:100%;height:100%;object-fit:cover;transition:transform .7s cubic-bezier(.2,.7,.2,1)}
body.page a.tile:hover img.cover{transform:scale(1.04)}
.cover.word{background:repeating-linear-gradient(0deg,hsl(var(--h) 30% 65% / .055) 0 1px,transparent 1px 22px),repeating-linear-gradient(90deg,hsl(var(--h) 30% 65% / .055) 0 1px,transparent 1px 22px),radial-gradient(90% 85% at 12% 0%,hsl(var(--h) 42% 36%),transparent 66%),linear-gradient(160deg,hsl(var(--h) 26% 19%),hsl(var(--h) 22% 9%))}
.cover.word b{position:absolute;left:14px;bottom:6px;font-size:58px;line-height:1.1;font-weight:700;letter-spacing:-.04em;color:hsl(var(--h) 45% 90% / .9);text-shadow:0 2px 24px hsl(var(--h) 50% 15% / .7)}
.cover.word i{position:absolute;top:13px;right:12px;font:10px/1 var(--mono);font-style:normal;letter-spacing:.06em;color:hsl(var(--h) 30% 85% / .42)}
.tag,.verb{position:absolute;z-index:1;display:inline-flex;align-items:center;height:24px;padding:0 9px;border-radius:999px;font-family:var(--mono);font-size:11px;letter-spacing:.03em;color:var(--fg);background:#0b0b0dbf;box-shadow:inset 0 0 0 1px #ffffff1f;-webkit-backdrop-filter:blur(10px) saturate(1.2);backdrop-filter:blur(10px) saturate(1.2)}
.tag{top:10px;left:10px;color:var(--accent)}
.tag::before{content:"";width:6px;height:6px;margin-right:6px;border-radius:50%;background:currentColor;box-shadow:0 0 8px currentColor}
.tag.ad{color:var(--soft)}
.tag.ad::before{display:none}
.verb{right:10px;bottom:10px;opacity:0;transform:translateY(4px);transition:opacity .2s,transform .2s}
.verb u{text-decoration:none;margin-left:5px;color:var(--accent)}
body.page a.tile:hover .verb,body.page a.tile:focus-visible .verb{opacity:1;transform:none}
@media(hover:none){.verb{opacity:1;transform:none}}
.tile-body{display:flex;flex-direction:column;gap:4px;padding:12px 8px 8px}
.tile-body h2{margin:0;font-size:15px;line-height:1.4;font-weight:600;letter-spacing:-.01em;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden}
.tile .summary{margin:0;min-height:1.5em;font-size:13px;line-height:1.5;color:var(--dim);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.seek-k{color:var(--soft);font-weight:500;margin-right:2px}
.tile .meta{display:flex;justify-content:space-between;align-items:center;gap:10px;margin:10px 0 0;padding-top:10px;border-top:1px solid var(--line);font-size:12px;color:var(--dim);text-align:left}
.who{display:inline-flex;align-items:center;gap:7px;min-width:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;color:var(--soft)}
.face{width:18px;height:18px;flex:0 0 auto;border-radius:50%;background:var(--card2);object-fit:cover;box-shadow:0 0 0 1px var(--line)}
.fact{display:inline-flex;align-items:center;gap:8px;flex:0 0 auto;font-family:var(--mono);font-size:11.5px;font-variant-numeric:tabular-nums;color:var(--dim)}
.fact i{position:relative;width:28px;height:3px;border-radius:2px;background:#ffffff14;overflow:hidden}
.fact i::after{content:"";position:absolute;inset:0;width:var(--p);border-radius:2px;background:var(--accent)}
.empty-plaza{max-width:32rem;padding:12px 0}
.empty-plaza .lead,.drawer .lead{margin-top:10px;color:var(--dim)}
.main>.drawer{max-width:34rem;margin:28px 32px 80px;padding:28px;border-radius:16px;background:var(--card);box-shadow:inset 0 0 0 1px var(--line),inset 0 1px 0 #ffffff0c}
.drawer h1{margin:0;font-size:22px;font-weight:650;letter-spacing:-.015em}
.drawer .summary{margin:12px 0 0;color:var(--soft)}
.drawer form.row{display:flex;gap:8px;margin:18px 0 0}
.drawer form.row input{flex:1;min-width:0;min-height:44px}
.drawer form.row button,.drawer>form button{display:inline-flex;align-items:center;justify-content:center;width:auto;min-height:44px;margin:0;padding:0 16px;border-radius:10px;background:var(--accent);color:var(--accent-ink);font-size:.95rem;font-weight:650}
body.page .mine button,body.page .pushed button,body.page .ghost{display:inline-flex;align-items:center;width:auto;min-height:32px;margin:0;padding:0 12px;border:1px solid var(--line2);border-radius:999px;background:none;color:var(--dim);font-size:.8rem;font-weight:500}
body.page .mine button:hover,body.page .pushed button:hover,body.page .ghost:hover{color:var(--fg);border-color:#ffffff40}
.drawer footer{margin-top:28px;font-size:.85rem;color:var(--dim)}
body.page .more>.ghost{justify-self:start}
.overlay{display:none;position:fixed;inset:0;z-index:40;align-items:center;justify-content:center;padding:24px}
.overlay:target{display:flex}
.overlay-back{position:absolute;inset:0;background:#000000b3;-webkit-backdrop-filter:blur(8px);backdrop-filter:blur(8px)}
.sheet{position:relative;z-index:1;width:min(31rem,100%);max-height:calc(100dvh - 48px);overflow:auto;padding:28px;border-radius:18px;background:var(--card);color:var(--fg);box-shadow:inset 0 0 0 1px var(--line2),inset 0 1px 0 #ffffff12,0 40px 100px -30px #000}
.dialog-head{display:flex;align-items:flex-start;justify-content:space-between;gap:16px}
.dialog-head h2{margin:0;font-size:20px;line-height:1.35;font-weight:600;letter-spacing:-.015em}
.close{flex:0 0 auto;display:grid;place-items:center;width:32px;height:32px;border-radius:9px;color:var(--dim);box-shadow:inset 0 0 0 1px var(--line);transition:color .15s,background .15s}
.close:hover{color:var(--fg);background:#ffffff0a}
.dialog-desc{margin:14px 0 22px;font-size:14px;line-height:1.7;color:var(--dim)}
.tabs{position:relative;display:inline-flex;gap:2px;margin-bottom:12px;padding:3px;border-radius:10px;background:var(--bg);box-shadow:inset 0 0 0 1px var(--line)}
.tabs input{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0)}
.tabs label{padding:6px 12px;border-radius:7px;font-size:12.5px;font-weight:500;color:var(--dim);cursor:pointer;transition:color .15s,background .15s}
.tabs label:hover{color:var(--fg)}
.pub-box:has(#tab-static:checked) label[for=tab-static],.pub-box:has(#tab-local:checked) label[for=tab-local],.pub-box:has(#tab-backend:checked) label[for=tab-backend]{color:var(--fg);background:var(--card2);box-shadow:inset 0 0 0 1px var(--line),0 1px 2px #0008}
.tabs input:focus-visible+label{outline:2px solid var(--accent);outline-offset:1px}
.codebox{display:none;gap:10px;padding:14px 16px;border-radius:12px;background:var(--bg);box-shadow:inset 0 0 0 1px var(--line)}
.pub-box:has(#tab-static:checked) #panel-static,.pub-box:has(#tab-local:checked) #panel-local,.pub-box:has(#tab-backend:checked) #panel-backend{display:flex}
.codebox b{flex:0 0 auto;font:12.5px/1.8 var(--mono);font-weight:400;color:var(--accent)}
.codebox code{display:block;padding:0;border:0;background:none;font:12.5px/1.8 var(--mono);color:var(--soft);white-space:pre-wrap;overflow-wrap:anywhere}
.copy-hint{margin:12px 0 0;font-size:12px;color:#5f5d59}
.dialog-hint{margin:22px 0 0;padding-top:18px;border-top:1px solid var(--line);font-size:13px;line-height:1.7;color:var(--dim)}
body.page .dialog-hint a{color:var(--fg);text-decoration:underline;text-decoration-color:#ffffff3a;text-underline-offset:3px}
@media(min-width:48rem) and (max-width:64rem){.sidebar{width:13rem;padding-left:14px;padding-right:14px}.main{margin-left:13rem}.content{padding-left:24px;padding-right:24px}.main>.drawer{margin-left:24px;margin-right:24px}}
@media(max-width:47.99rem){.sidebar{position:static;width:auto;flex-direction:row;flex-wrap:wrap;align-items:center;gap:10px 14px;padding:12px 14px;border-right:0;border-bottom:1px solid var(--line)}.brand{margin:0}.wordmark{font-size:19px}.nav{flex-direction:row;gap:4px;margin:0 0 0 auto}.nav-item{min-height:36px;padding:0 12px;border-radius:999px;font-size:13.5px}.nav-item.active{background:#ffffff0d;box-shadow:inset 0 0 0 1px var(--line)}.nav-dot{display:none}.sidebar-bottom{flex-basis:100%;margin:0}.publish-note{display:flex;align-items:center;justify-content:space-between;gap:12px;padding:8px 8px 8px 14px;border-radius:12px}.publish-note p{display:none}.publish-note h3{font-size:12.5px;font-weight:500;color:var(--dim)}body.page .publish-note a{min-height:30px;margin:0;padding:0 12px;font-size:12px}.main{margin:0;background:none}.content{padding:14px 14px 72px}.grid{gap:12px}body.page a.tile{padding:5px;border-radius:14px}.shot{border-radius:10px}.tile-body{padding:10px 6px 6px}.tile-body h2{font-size:14px}.tile .summary{font-size:12.5px}.tile .meta{margin-top:8px;padding-top:8px}.cover.word b{font-size:46px}.cover.word i{display:none}.main>.drawer{margin:14px;padding:22px 18px 24px}.overlay{align-items:flex-end;padding:0}.sheet{width:100%;max-height:88dvh;padding:24px 20px calc(24px + env(safe-area-inset-bottom));border-radius:18px 18px 0 0}}
"#;

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
        let plaza = empty();
        let html = render(&view(&plaza));
        assert!(html.contains("广场上还没有作品"));
        assert!(!html.contains("class=\"grid\""));
        assert!(html.contains("--public"));
        assert!(!html.contains("来玩点，还没定稿的"));
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

    #[test]
    fn ago_reads_like_a_person() {
        let now = datetime!(2026-09-08 04:00:00 UTC);
        assert_eq!(ago(datetime!(2026-09-08 03:59:30 UTC), now), "刚刚发布");
        assert_eq!(ago(datetime!(2026-09-08 03:20:00 UTC), now), "40 分钟前");
        assert_eq!(ago(datetime!(2026-09-08 03:00:00 UTC), now), "1 小时前");
        assert_eq!(ago(datetime!(2026-09-06 04:00:00 UTC), now), "2 天前");
    }

    #[tokio::test]
    async fn cache_serves_empty_when_nothing_was_written() {
        let dir = tempfile::tempdir().unwrap();
        let cache = PlazaCache::new(FsStore::new(dir.path()));
        assert!(cache.get().await.items.is_empty());
    }
}
