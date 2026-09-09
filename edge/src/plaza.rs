//! 广场（DESIGN §3.8）：根域那一页。
//!
//! 内容来自控制面写进对象存储的 `plaza.json`（DESIGN §4.5），这里只读、只渲染。
//! 控制面挂了照常能翻，只是人数旧几分钟。整页服务端直出：卡片、筛选、排序都在 HTML 里，
//! 一小段内联脚本只做三件事——筛选、排序、把「想玩」记在这台设备的 localStorage 里。
//! 没有 JS 也能看、能点进去玩；筛选栏那时候藏着。
//!
//! 这一页是我们自己的，没有用户脚本，所以能带严格的 CSP（`app.rs`）；封面来自各作品自己的子域。

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use playtest_common::plaza::{Plaza, PlazaItem, PLAYERS_WINDOW_DAYS};
use playtest_common::store::FsStore;
use playtest_common::{DEVELOPER_API_URL, RESERVED_PATH_PREFIX};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::html::{esc, page};

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
        items: Vec::new(),
    }
}

/// 渲染时要知道的几样。
pub struct View<'a> {
    pub plaza: &'a Plaza,
    /// 泛域名后缀，页头和角标上的名字（本机是 `localhost`）。
    pub host_suffix: &'a str,
    /// 每个响应一个随机值，CSP 只放行带它的那段脚本。
    pub nonce: &'a str,
    pub now: OffsetDateTime,
}

pub fn render(view: &View<'_>) -> String {
    let suffix = esc(view.host_suffix);
    let head =
        "<meta name=\"description\" content=\"正在找人试玩的作品。点开就能玩，不用注册。\">\n\
<meta property=\"og:title\" content=\"playtest · 正在找人试玩的作品\">\n\
<meta property=\"og:description\" content=\"开发者把手上能玩的版本放到这里，路过的人点开就玩。\">\n\
<meta property=\"og:type\" content=\"website\">\n";

    let mut tiles = String::new();
    for item in &view.plaza.items {
        tiles.push_str(&tile(item, view.now));
    }
    let grid = if view.plaza.items.is_empty() {
        "<section class=\"empty-plaza\"><p>广场上还没有作品。</p>\
<p class=\"lead\"><code>playtest ./dist --public</code> 会把作品放到这里。</p></section>\n"
            .to_string()
    } else {
        format!("<main class=\"grid\" id=\"pt-grid\">\n{tiles}</main>\n")
    };

    // 页脚一行说完，但该如实说的都在（DESIGN §3.8）：只放主动公开的、不排名次、
    // 「想玩」只在本机、登录未上线所以只有 24 小时作品。多余的解释一个字不留。
    let body = format!(
        "<header class=\"top\">\n\
<a class=\"brand\" href=\"/\">playtest<span>.run</span></a>\n\
<p class=\"tagline\">正在找人试玩的作品，点开就玩。</p>\n\
</header>\n\
<nav class=\"bar\" id=\"pt-bar\" hidden>\n\
<div class=\"filters\" role=\"group\" aria-label=\"筛选\">\
<button type=\"button\" class=\"chip on\" data-f=\"all\">全部</button>\
<button type=\"button\" class=\"chip\" data-f=\"seeking\">正在找人测</button>\
<button type=\"button\" class=\"chip\" data-f=\"game\">游戏</button>\
<button type=\"button\" class=\"chip\" data-f=\"other\">体验</button>\
<button type=\"button\" class=\"chip\" data-f=\"want\">想玩的</button></div>\n\
<div class=\"sorts\" role=\"group\" aria-label=\"排序\">\
<button type=\"button\" class=\"chip on\" data-s=\"updated\">最近更新</button>\
<button type=\"button\" class=\"chip\" data-s=\"players\">最多人玩</button></div>\n\
</nav>\n\
{grid}\
<footer class=\"page-foot\">\n\
<p>只放开发者主动公开的作品，按时间排 · 「想玩」只记在这台设备 · 登录未上线，作品最多停留 24 小时</p>\n\
<p><code>xxx.{suffix}</code> · <a href=\"{dev}\">开发者从这里开始</a></p>\n\
</footer>\n\
<script nonce=\"{nonce}\">{SCRIPT}</script>\n",
        dev = DEVELOPER_API_URL,
        nonce = esc(view.nonce),
    );

    page("playtest · 正在找人试玩的作品", head, PLAZA_CSS, &body)
}

fn tile(item: &PlazaItem, now: OffsetDateTime) -> String {
    // 链接是控制面按模板拼的，不是用户输入；这一道只是不让一份写坏的 plaza.json 变成 `javascript:`。
    let url = if item.url.starts_with("https://") || item.url.starts_with("http://") {
        esc(&item.url)
    } else {
        "#".to_string()
    };
    let title = esc(&item.title);
    let developer = esc(&item.developer);

    let cover = match &item.cover_url {
        Some(cover) => format!(
            "<img class=\"cover\" src=\"{}\" alt=\"\" loading=\"lazy\" decoding=\"async\">",
            esc(cover)
        ),
        // 没封面就用作品名排一张字卡：设计好的版式，不是假装的截图（DESIGN §3.3）。
        None => format!(
            "<div class=\"cover word\" style=\"--h:{}\"><span>{title}</span></div>",
            hue(&item.slug)
        ),
    };
    // 卡片上只有「正在找人测」一个标——它是行动的邀请。「游戏 / 体验」只做筛选的依据
    // （data-game），不占卡面：邀请函上不写品类。
    let seek_chip = if item.seeking {
        "<span class=\"badge\">正在找人测</span>"
    } else {
        ""
    };
    let summary = match item
        .summary
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(s) => format!("<p class=\"summary\">{}</p>\n", esc(s)),
        None => String::new(),
    };
    let seek_note = match item
        .seek_note
        .as_deref()
        .map(str::trim)
        .filter(|s| item.seeking && !s.is_empty())
    {
        Some(s) => format!("<p class=\"seek-note\">{}</p>\n", esc(s)),
        None => String::new(),
    };
    // 0 就不说：「还没人玩过」是一句替作品道歉的话（DESIGN §3.4 的规矩在这里同样成立）。
    let players = match item.players {
        0 => String::new(),
        n => format!(
            "<span title=\"最近 {PLAYERS_WINDOW_DAYS} 天里点了「开始」的人数\">{n} 人玩过</span>"
        ),
    };
    let expires = item
        .expires_at
        .as_deref()
        .and_then(|raw| OffsetDateTime::parse(raw, &Rfc3339).ok())
        .map(|at| format!("<span>{}</span>", esc(&remaining(at, now))))
        .unwrap_or_default();

    format!(
        "<article class=\"tile\" data-slug=\"{slug}\" data-seeking=\"{seeking}\" data-game=\"{game}\" \
data-players=\"{players_n}\" data-updated=\"{updated}\">\n\
<a class=\"cover-link\" href=\"{url}\">{cover}{seek_chip}</a>\n\
<div class=\"tile-body\">\n\
<h2><a href=\"{url}\">{title}</a><span class=\"v\">v{version}</span></h2>\n\
{summary}{seek_note}\
<p class=\"meta\"><span>{developer}</span>{players}{expires}</p>\n\
<div class=\"actions\"><button type=\"button\" class=\"want\" data-slug=\"{slug}\" aria-pressed=\"false\">想玩</button>\
<a class=\"report\" href=\"{url}{prefix}report\">举报</a></div>\n\
</div>\n</article>\n",
        slug = esc(&item.slug),
        seeking = item.seeking as u8,
        game = item.is_game as u8,
        players_n = item.players,
        updated = esc(&item.updated_at),
        version = item.version,
        prefix = RESERVED_PATH_PREFIX,
    )
}

/// 字卡的色相：同一个作品每次都是同一种颜色，不同作品大概率不同。
fn hue(slug: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for b in slug.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    h % 360
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

/// 广场自己的那段样式，接在共用样式后面。手机一列、平板两列、桌面三到四列。
///
/// 形态是「邀请墙」不是商店（DESIGN §3.8）：卡片几乎没有框，封面出血到边，
/// 元数据用等宽小字排出工具感，唯一的颜色留给「正在找人测」。质感来自排版层次、
/// 极淡的点阵底纹和 hover 时封面的呼吸，不来自更多的框和更多的字。
const PLAZA_CSS: &str = "\
body.page{display:block;padding:0 0 48px;\
background:radial-gradient(60% 32% at 50% 0%,#141824 0%,rgba(20,24,36,0) 100%),\
radial-gradient(rgba(255,255,255,.028) 1px,transparent 1.5px) 0 0/26px 26px,var(--bg)}\
.top{max-width:78rem;margin:0 auto;padding:44px 24px 0}\
.brand{display:inline-block;font-size:1.3rem;font-weight:750;letter-spacing:-.03em;color:var(--fg);text-decoration:none}\
.brand span{color:var(--accent)}\
.tagline{margin:2px 0 0;color:var(--dim);font-size:.95rem;letter-spacing:.01em}\
.bar{max-width:78rem;margin:22px auto 0;padding:10px 24px;display:flex;flex-wrap:wrap;gap:6px 20px;justify-content:space-between;\
border-top:1px solid var(--line);border-bottom:1px solid var(--line)}\
.filters,.sorts{display:flex;flex-wrap:wrap;gap:2px}\
.chip{display:inline-block;padding:3px 10px;border-radius:999px;border:1px solid transparent;background:none;\
color:var(--dim);font:inherit;font-size:.8rem;line-height:1.6;letter-spacing:.02em;width:auto;margin:0;cursor:pointer;white-space:nowrap}\
.bar .chip:hover{color:var(--fg);filter:none}\
.bar .chip.on{color:var(--fg);border-color:var(--line);background:var(--card);font-weight:600}\
.grid{max-width:78rem;margin:26px auto 0;padding:0 24px;display:grid;gap:26px 22px;grid-template-columns:1fr}\
@media(min-width:40rem){.grid{grid-template-columns:repeat(2,1fr)}}\
@media(min-width:64rem){.grid{grid-template-columns:repeat(3,1fr)}}\
@media(min-width:88rem){.grid{grid-template-columns:repeat(4,1fr)}}\
.tile{display:flex;flex-direction:column}\
.cover-link{position:relative;display:block;border-radius:14px;overflow:hidden;border:1px solid var(--line);\
background:#0e1014;text-decoration:none;transition:border-color .2s,transform .2s;transform:translateZ(0)}\
.cover-link:hover{text-decoration:none}\
.tile:hover .cover-link{border-color:#39404e;transform:translateY(-2px)}\
.cover{display:block;width:100%;aspect-ratio:16/10;object-fit:cover;transition:transform .35s ease}\
.tile:hover img.cover{transform:scale(1.025)}\
.cover.word{display:flex;align-items:flex-end;padding:18px;\
background:radial-gradient(120% 90% at 12% 0%,hsl(var(--h) 42% 26%),transparent 60%),\
linear-gradient(150deg,hsl(var(--h) 38% 17%),#0b0c10 82%)}\
.cover.word span{font-size:1.45rem;font-weight:700;letter-spacing:-.01em;line-height:1.3;color:rgba(255,255,255,.92);\
overflow-wrap:anywhere;display:-webkit-box;-webkit-line-clamp:3;-webkit-box-orient:vertical;overflow:hidden}\
.badge{position:absolute;top:10px;left:10px;padding:3px 9px;border-radius:999px;font-size:.75rem;font-weight:600;\
letter-spacing:.03em;color:var(--accent-ink);background:var(--accent)}\
.tile-body{display:flex;flex-direction:column;flex:1;padding:11px 2px 0}\
.tile h2{margin:0;display:flex;align-items:baseline;gap:8px;font-size:1.02rem;line-height:1.45;font-weight:650;\
letter-spacing:-.005em;overflow-wrap:anywhere}\
.tile h2 a{color:var(--fg);text-decoration:none}\
.tile h2 a:hover{color:var(--accent)}\
.tile .v{flex:0 0 auto;font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:.72rem;color:var(--dim)}\
.tile .summary{margin:3px 0 0;font-size:.88rem;line-height:1.6;color:#aab2bf;\
display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden}\
.seek-note{margin:7px 0 0;padding:0 0 0 10px;border-left:2px solid var(--accent);font-size:.85rem;line-height:1.6;color:#d8c9a0}\
.tile .meta{margin:8px 0 0;display:flex;flex-wrap:wrap;gap:2px 14px;text-align:left;\
font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:.72rem;color:var(--dim);font-variant-numeric:tabular-nums}\
.actions{display:flex;justify-content:space-between;align-items:baseline;gap:10px;margin-top:auto;padding-top:8px}\
.want{width:auto;margin:0;padding:2px 0;border:0;border-radius:0;background:none;\
color:var(--dim);font-size:.8rem;font-weight:600;letter-spacing:.02em;cursor:pointer}\
.want:hover{color:var(--fg);filter:none;transform:none}\
.want.on{color:var(--accent)}\
.report{font-size:.72rem;color:#5b6270}\
.report:hover{color:var(--dim)}\
.empty-plaza{max-width:30rem;margin:64px auto 0;padding:0 24px;text-align:center}\
.page-foot{max-width:78rem;margin:44px auto 0;padding:14px 24px 0;display:block;border-top:1px solid var(--line);\
font-size:.76rem;line-height:1.8;color:#6b7280}\
.page-foot p{margin:0}\
.page-foot code{font-size:.9em}";

/// 三件事：记「想玩」、筛选、排序。写成 ES5，没有 JS 的时候整页照常能看、能点。
const SCRIPT: &str = "(function(){\
var KEY='pt_want',grid=document.getElementById('pt-grid'),bar=document.getElementById('pt-bar');\
if(!grid){return}\
bar.hidden=false;\
function want(){try{return JSON.parse(localStorage.getItem(KEY)||'[]')}catch(e){return[]}}\
function save(a){try{localStorage.setItem(KEY,JSON.stringify(a))}catch(e){}}\
var tiles=[].slice.call(grid.querySelectorAll('.tile')),f='all',s='updated';\
function paint(){var w=want();\
tiles.forEach(function(t){var b=t.querySelector('.want'),on=w.indexOf(t.getAttribute('data-slug'))>=0;\
b.className='want'+(on?' on':'');b.textContent=on?'\\u2665 想玩':'想玩';b.setAttribute('aria-pressed',on?'true':'false');\
var show=f==='all'||(f==='seeking'&&t.getAttribute('data-seeking')==='1')||(f==='game'&&t.getAttribute('data-game')==='1')\
||(f==='other'&&t.getAttribute('data-game')==='0')||(f==='want'&&on);t.hidden=!show});\
var sorted=tiles.slice().sort(function(a,b){\
if(s==='players'){var d=(+b.getAttribute('data-players'))-(+a.getAttribute('data-players'));if(d)return d}\
var sa=a.getAttribute('data-seeking'),sb=b.getAttribute('data-seeking');if(sa!==sb&&s!=='players')return sb>sa?1:-1;\
var ua=a.getAttribute('data-updated'),ub=b.getAttribute('data-updated');return ua<ub?1:ua>ub?-1:0});\
sorted.forEach(function(t){grid.appendChild(t)})}\
grid.addEventListener('click',function(e){var b=e.target.closest&&e.target.closest('.want');if(!b)return;\
var slug=b.getAttribute('data-slug'),w=want(),i=w.indexOf(slug);if(i>=0)w.splice(i,1);else w.push(slug);save(w);paint()});\
bar.addEventListener('click',function(e){var b=e.target.closest&&e.target.closest('button');if(!b)return;\
var sib=b.parentNode.querySelectorAll('button');for(var i=0;i<sib.length;i++)sib[i].className='chip';b.className='chip on';\
if(b.hasAttribute('data-f'))f=b.getAttribute('data-f');else s=b.getAttribute('data-s');paint()});\
paint()})();";

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
        }
    }

    fn view<'a>(plaza: &'a Plaza) -> View<'a> {
        View {
            plaza,
            host_suffix: "localhost",
            nonce: "n0nce",
            now: datetime!(2026-09-08 04:00:00 UTC),
        }
    }

    #[test]
    fn renders_cards_with_everything_on_them() {
        let plaza = Plaza {
            schema: 1,
            generated_at: "2026-09-08T03:57:00Z".into(),
            items: vec![item("brisk-otter-41")],
        };
        let html = render(&view(&plaza));
        assert!(html.starts_with("<!doctype html>\n<html lang=\"zh-CN\">"));
        assert!(html.contains("<article class=\"tile\" data-slug=\"brisk-otter-41\" data-seeking=\"1\" data-game=\"1\""));
        assert!(html.contains("href=\"http://brisk-otter-41.localhost:8443\""));
        assert!(html.contains("小球大冒险"));
        assert!(html.contains("<span class=\"v\">v7</span>"));
        assert!(html.contains("三关，五分钟，手机上也能玩。"));
        // 求测的那句话直接说，不带「想让你看：」的标签腔。
        assert!(html.contains("<p class=\"seek-note\">新手引导看得懂吗</p>"));
        assert!(html.contains("<span>某某</span>"));
        assert!(html.contains(">12 人玩过</span><span>还剩 16 小时</span>"));
        assert!(html.contains("class=\"badge\">正在找人测"));
        // 卡片上不写品类：「游戏 / 体验」只是筛选的依据（data-game），邀请函上不标货架分类。
        assert!(!html.contains(">游戏</span>"));
        // 没封面：字卡，不是假截图。
        assert!(html.contains("class=\"cover word\""));
        assert!(!html.contains("<img class=\"cover\""));
        // 举报入口复用作品自己的。
        assert!(html.contains("href=\"http://brisk-otter-41.localhost:8443/_playtest/report\""));
        // 该如实说的仍在页脚，一行说完。
        assert!(html.contains("登录未上线，作品最多停留 24 小时"));
        assert!(html.contains("「想玩」只记在这台设备"));
        assert!(html.contains("只放开发者主动公开的作品"));
        // 脚本带 nonce，CSP 才放行。
        assert!(html.contains("<script nonce=\"n0nce\">"));
        // 这是玩家路径上唯一允许出现开发者域名的一页。
        assert!(html.contains(DEVELOPER_API_URL));
    }

    #[test]
    fn a_cover_is_an_image_and_a_non_game_says_experience() {
        let mut it = item("wise-mink-28");
        it.cover_url = Some("http://wise-mink-28.localhost:8443/_playtest/cover?v=abcd1234".into());
        it.is_game = false;
        it.seeking = false;
        it.players = 0;
        it.expires_at = None;
        let plaza = Plaza {
            schema: 1,
            generated_at: String::new(),
            items: vec![it],
        };
        let html = render(&view(&plaza));
        assert!(html.contains(
            "<img class=\"cover\" src=\"http://wise-mink-28.localhost:8443/_playtest/cover?v=abcd1234\""
        ));
        assert!(html.contains("data-game=\"0\""));
        // 卡片上没有「正在找人测」的标（筛选栏里那个按钮不算）。
        assert!(!html.contains("class=\"badge\""));
        // 没求测就不显示那句话，哪怕字段里有。
        assert!(!html.contains("class=\"seek-note\""));
        // 0 个人玩过就一个字不说，不替作品道歉。
        assert!(!html.contains("人玩过"));
        assert!(!html.contains("还剩"));
    }

    #[test]
    fn everything_from_the_feed_is_escaped() {
        let mut it = item("x");
        it.title = "<img src=x onerror=alert(1)>".into();
        it.developer = "\"><script>alert(2)</script>".into();
        it.summary = Some("</p><script>alert(3)</script>".into());
        it.seek_note = Some("<b>bold</b>".into());
        it.url = "javascript:alert(4)\" onmouseover=\"".into();
        let plaza = Plaza {
            schema: 1,
            generated_at: String::new(),
            items: vec![it],
        };
        let html = render(&view(&plaza));
        assert!(!html.contains("<script>alert"));
        assert!(!html.contains("<img src=x"));
        assert!(!html.contains("<b>bold</b>"));
        assert!(!html.contains("onmouseover=\""));
        assert!(!html.contains("javascript:"));
        assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
    }

    #[test]
    fn empty_plaza_says_so() {
        let plaza = empty();
        let html = render(&view(&plaza));
        assert!(html.contains("广场上还没有作品"));
        assert!(!html.contains("id=\"pt-grid\""));
        assert!(html.contains("--public"));
    }

    #[test]
    fn hue_is_stable_and_spread() {
        assert_eq!(hue("brisk-otter-41"), hue("brisk-otter-41"));
        assert!(hue("a") < 360);
        assert_ne!(hue("brisk-otter-41"), hue("wise-mink-28"));
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
