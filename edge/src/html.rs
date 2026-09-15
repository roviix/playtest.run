//! 玩家看到的每一页都在这里拼出来。
//!
//! 不引模板引擎：这几页加起来不到两百行，模板引擎换来的是一层编译期魔法和一个新依赖。
//! 代价是**每一处插值都必须自己调 [`esc`]**——清单里的作品名和开发者名是用户输入，
//! 这里是玩家域，漏一个就是 XSS。
//!
//! 长相（DESIGN §3.3、§3.9）：黑曜暗底、浅色文字、冷萃绿只给动作与焦点。封面优先。
//! 门禁页、分享页、关注页和错误页共用下面这一份记号，广场在它之上再加自己的一段（`plaza.rs`）。
//! 玩家域只有这一套颜色：群里那张邀请卡、点开的门禁页、回到的广场，不换色。门禁页和邀请卡
//! （`card.rs`）是**同一个物件的两种媒介**：同一张封面、同一句话、同一个版本号，
//! 所以卡片版式（封面出血、真人名字与作品名最大、版本日期用等宽小字、一个主按钮）
//! 在这里定，两边照着做。

/// HTML 文本与属性值通用的转义。单引号也转，属性用单引号包时才安全。
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// 字卡的色相：同一个作品每次都是同一种颜色，不同作品大概率不同。
/// 门禁页、广场卡片、邀请卡三处共用它，一个作品在三个地方是同一种蓝或同一种绿。
pub fn hue(slug: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for b in slug.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    h % 360
}

/// 玩家页面只有这一套设计系统，分三层写，每一页只带自己用得上的两层：
///
/// - [`BASE`]：颜色记号、字体、焦点环、输入框、几个两边都用的类（`.lead` `.summary` `.row` `.more`）。
/// - [`CARD`]：一张居中的卡（门禁、分享、离线、到期、举报、确认、退订）。
/// - [`PAGE`]：整页版式（广场、关注页）：左边一条栏、右边一面墙。
///
/// 卡页发 `BASE + CARD`，整页发 `BASE + PAGE`。两层**互不覆盖**——原来广场那一段是叠在
/// 卡的样式之上再一条条撤销（`body.page button{...}` 之类 88 行），撤不干净的就漏成怪版式
/// （广场卡右下那件事实曾经带着卡页 `.seats` 的 14px 上边距）。现在没有可撤销的东西。
///
/// 不引外部脚本与字体：玩家可能在微信里、在很慢的网上，多一个请求就多一次白屏。
/// 里面**不写 CSS 注释**：这段字节随每一页发给玩家，注释既是白花的流量，也会把内部说法
/// 漏到页面源码里（有测试盯着页面上不该出现的词）。要解释的都写在这里：
/// - `main.card` 用 auto 外边距居中，比 `align-items:center` 稳：内容比屏幕高时不会被截掉。
/// - `.hero.word` / `.cover.word` 是没有封面时的字卡，色相来自 [`hue`]，
///   门禁页、广场、邀请卡三处同一个作品是同一种颜色。
/// - `.by` 是「谁邀请你」那一行：一张脸加一个真人名字（DESIGN §3.9）。
/// - `.more` 里那几行是弱化的：一页只有一个主按钮（DESIGN §3.3 第 6 条）。
/// - `.voices` 是别人说过的话，只读：没有回帖、没有赞、没有楼层（DESIGN §3.5）。
/// - `.mine` 是关注页那一列：一行一项，右边一个「取消」，不分页。
/// - `.sidebar` 桌面 208px 现代精致侧栏固定在左，右侧为协调舒适的主舞台；手机上收成顶上一条。
/// - `.cover.word` 四层背景：两层细网格、一团光、一层底色；`b` 是花押、`i` 是角上的 slug。
/// - `.tag::before` 是「正在找人测」前面那个发光的点；推广标没有点。
/// - `.fact i` 是名额那一件事实旁边 28px 的进度线，`--p` 是百分比。
/// - `.mark` 是顶栏与侧栏的测试准星线稿，中心悬浮纯钛冷白聚焦核。
/// - `.wordmark` 是单行不折行的纯小写品牌字标，`playtest` 钛白加粗，`.run` 浅钛银灰。
/// - `.nav-dot` 是当前那间房旁边的一颗微核指示点。
/// - 发布说明桌面居中 42rem、手机贴近底部；原生 dialog 增强焦点与关闭，无脚本时保留 `:target`。
pub const BASE: &str = concat!(":root{color-scheme:dark;--bg:#09090b;--card:#121216;--card2:#17181d;--fg:#f4f4f5;--soft:#d4d4d8;--dim:#a1a1aa;--line:#ffffff14;--line2:#ffffff20;--accent:#75cdb5;--accent-ink:#101e19;--warn:#f0dcac;--warn-bg:#1f1a10;--warn-line:#443a1e;--mono:ui-monospace,Menlo,monospace}*{box-sizing:border-box}body{margin:0;min-height:100vh;background:var(--bg);color:var(--fg);font:15px/1.6 system-ui}:focus{outline:none}:focus-visible{outline:2px solid rgba(255,255,255,.45);outline-offset:2px}[tabindex=\"-1\"]:focus,[tabindex=\"-1\"]:focus-visible{outline:none!important}[hidden]{display:none!important}::selection{background:#ffffff33;color:#ffffff}.lead{color:var(--dim);font-size:14px}.row{display:flex;gap:8px}.row input{flex:1;min-width:0}.row button{width:auto;margin:0;padding:9px 14px;white-space:nowrap}:is(input,select,textarea){width:100%;min-height:44px;padding:10px 12px;border:1px solid var(--line);border-radius:8px;background:var(--bg);color:var(--fg);font:inherit}label{display:block}textarea{resize:vertical}.more{display:flex;justify-content:center;gap:12px;margin:10px 0 0;font-size:12px}@media(prefers-reduced-motion:reduce){@view-transition{navigation:none}*,*::before,*::after{animation:none!important;transition:none!important}}", include_str!("../../ui/dialog.css"));

pub const MARK: &str = include_str!("../../ui/mark.svg");

pub const WORDMARK: &str = include_str!("../../ui/wordmark.svg");

pub const FAVICON_SVG: &str = include_str!("../../ui/favicon.svg");

pub const FAVICON_ICO: &[u8] = include_bytes!("../../ui/favicon.ico");

pub const CARD: &str = include_str!("../../ui/invitation.css");

pub const PAGE: &str = concat!(
    include_str!("../../ui/workspace.css"),
    include_str!("../../ui/follow.css")
);

pub fn transition_name(slug: &str) -> String {
    format!(
        "art-{}",
        slug.bytes()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

pub fn enhance(html: String, nonce: &str) -> String {
    let discovery = if html.contains("data-share-collection") {
        include_str!("../../ui/discovery.js")
    } else {
        ""
    };
    let script = format!(
        "<script nonce=\"{}\">{}\n{}\n{}\n{}</script>",
        esc(nonce),
        include_str!("../../ui/dialog.js"),
        include_str!("../../ui/player.js"),
        include_str!("../../ui/account.js"),
        discovery
    );
    html.replacen("</body>", &format!("{script}</body>"), 1)
}

/// 「复制链接」那一下。分享页和门禁页上「复制到系统浏览器里打开」都用它，
/// 原来各写了一遍、一字不差。写成 ES5、每一步都能失败：没有 JS 时那个只读输入框自己就是
/// 「复制链接」的办法。`i` 是只读输入框，`b` 是按钮；成功了按钮上的字换成「已复制」。
pub const COPY_JS: &str = "function ptCopy(i,b){b.onclick=function(){i.focus();i.select();i.setSelectionRange(0,i.value.length);\
var ok=function(){b.textContent='Copied'};\
var old=function(){try{document.execCommand('copy');ok()}catch(e){}};\
if(navigator.clipboard&&navigator.clipboard.writeText){navigator.clipboard.writeText(i.value).then(ok,old)}else{old()}}}";

/// 只读输入框加一个「复制链接」按钮。`url` 由调用方转义。
pub fn copy_row(url_escaped: &str, aria_label: Option<&str>) -> String {
    let label = aria_label
        .map(|l| format!(" aria-label=\"{}\"", esc(l)))
        .unwrap_or_default();
    format!(
        "<p class=\"row\"><input id=\"pt-url\" readonly value=\"{url_escaped}\"{label}>\
<button type=\"button\" id=\"pt-copy\">Copy link</button></p>\n"
    )
}

/// 页面外壳。`head` 放 OG 之类的额外元信息，`body` 放卡片里面的内容。
pub fn shell(title: &str, head: &str, body: &str) -> String {
    shell_hero(title, head, "", body)
}

/// 带一张封面的外壳（门禁页有封面时）：封面顶到卡片边，正文在它下面。
/// `hero` 是已经拼好的 HTML（调用方负责转义）；空字符串就是没有封面。
pub fn shell_hero(title: &str, head: &str, hero: &str, body: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n\
<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1,viewport-fit=cover\">\n\
<title>{title}</title>\n\
<link rel=\"icon\" type=\"image/svg+xml\" href=\"/favicon.svg\">\n\
<link rel=\"alternate icon\" href=\"/favicon.ico\">\n\
{head}<style>{BASE}{CARD}</style>\n\
</head>\n<body>\n<main class=\"card\">\n{hero}<div class=\"body\">\n{body}</div>\n</main>\n</body>\n</html>\n",
        title = esc(title),
    )
}

/// 不套卡片的整页外壳，广场与关注页用：左边一条栏，右边一面墙。
pub fn page(title: &str, head: &str, body: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n\
<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1,viewport-fit=cover\">\n\
<title>{title}</title>\n\
<link rel=\"icon\" type=\"image/svg+xml\" href=\"/favicon.svg\">\n\
<link rel=\"alternate icon\" href=\"/favicon.ico\">\n\
{head}<style>{BASE}{PAGE}</style>\n\
</head>\n<body class=\"page\">\n{body}</body>\n</html>\n",
        title = esc(title),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_every_dangerous_char() {
        assert_eq!(
            esc("<img src=x onerror=\"a&b\" title='c'>"),
            "&lt;img src=x onerror=&quot;a&amp;b&quot; title=&#39;c&#39;&gt;"
        );
        assert_eq!(esc("《正常的名字》"), "《正常的名字》");
    }

    #[test]
    fn each_page_carries_only_its_two_layers_and_they_stay_small() {
        // 卡页 ~15 KB：含整套门禁卡与右侧悬浮实时原声舱（含微信式全屏聊天、表情盘与试玩贴纸），无外链脚本，样式在 16 KB 以内。
        // 整页 ~31 KB：广场的墙、栏、卡、Slogan Hero、发布说明（终端舱）、通知设置都在里面，样式在 32 KB 以内。
        let card = BASE.len() + CARD.len();
        let page = BASE.len() + PAGE.len();
        assert!(card < 32 * 512, "卡页样式 {card} 字节");
        assert!(page < 32 * 1024, "整页样式 {page} 字节");
        assert!(
            PAGE.contains(".publish-sheet{width:min(34rem,100%)"),
            "发布说明独立使用 34rem 紧凑阅读宽度"
        );
        // 两层互不覆盖：整页那一层不该再出现「撤销卡页」的写法。
        assert!(!PAGE.contains("body.page"));
        assert!(PAGE.contains(".sidebar .brand"));
        assert!(!PAGE.contains("transform:none;cursor"));
        // 每一层里没有注释——它们会随页面发给玩家。
        for layer in [BASE, CARD, PAGE] {
            assert!(!layer.contains("/*"));
        }
    }

    #[test]
    fn hero_sits_above_the_body() {
        let html = shell_hero("t", "", "<img class=\"hero\" src=\"/x\">", "<h1>x</h1>");
        let hero = html.find("class=\"hero\"").unwrap();
        let body = html.find("class=\"body\"").unwrap();
        assert!(hero < body);
        assert!(!shell("t", "", "<h1>x</h1>").contains("class=\"hero\""));
    }

    #[test]
    fn hue_is_stable_and_spread() {
        assert_eq!(hue("brisk-otter-41"), hue("brisk-otter-41"));
        assert!(hue("a") < 360);
        assert_ne!(hue("brisk-otter-41"), hue("wise-mink-28"));
    }

    #[test]
    fn transition_names_are_stable_unique_and_css_safe() {
        assert_eq!(transition_name("one"), transition_name("one"));
        assert_ne!(transition_name("one"), transition_name("two"));
        assert!(transition_name("\"<你好>")
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-'));
    }

    #[test]
    fn interaction_enhancement_uses_the_response_nonce() {
        let html = enhance("<html><body>作品</body></html>".into(), "test-nonce");
        assert_eq!(html.matches("<script").count(), 1);
        assert!(html.contains("<script nonce=\"test-nonce\">"));
        assert!(html.contains("dialog.showModal()"));
        assert!(html.contains("dialog.addEventListener('cancel'"));
        assert!(!html.contains("<script src="));
    }
}
