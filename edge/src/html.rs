//! 玩家看到的每一页都在这里拼出来。
//!
//! 不引模板引擎：这几页加起来不到两百行，模板引擎换来的是一层编译期魔法和一个新依赖。
//! 代价是**每一处插值都必须自己调 [`esc`]**——清单里的作品名和开发者名是用户输入，
//! 这里是玩家域，漏一个就是 XSS。
//!
//! 长相（DESIGN §3.3、§3.9）：暖炭底、象牙字、唯一一支琥珀只给动作与焦点。封面优先。
//! 门禁页、分享页、「我的」和错误页共用下面这一份记号，广场在它之上再加自己的一段（`plaza.rs`）。
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

/// 全站共用的一份样式，内联进每一页。不引外部脚本与字体：玩家可能在微信里、
/// 在很慢的网上，多一个请求就多一次白屏。
///
/// 里面**不写 CSS 注释**：这段字节会随每一页发给玩家，注释既是白花的流量，
/// 也会把内部说法漏到页面源码里（有测试盯着页面上不该出现的词）。
/// 要解释的都在这里：
/// - `main.card` 用 auto 外边距居中，比 `align-items:center` 稳：内容比屏幕高时不会被截掉。
/// - `.hero.word` / `.cover.word` 是没有封面时的字卡，色相来自 [`hue`]，
///   门禁页、广场、邀请卡三处同一个作品是同一种颜色。
/// - `.by` 是「谁邀请你」那一行：一张脸加一个真人名字（DESIGN §3.9）。
/// - `.more` 里那几行是弱化的：一页只有一个主按钮（DESIGN §3.3 第 6 条）。
/// - `.voices` 是别人说过的话，只读：没有回帖、没有赞、没有楼层（DESIGN §3.5）。
/// - `.mine` 是「我的」那个抽屉：一行一项，右边一个「取消」，不分页。
pub const CSS: &str = "\
:root{color-scheme:dark;--bg:#0b0b0d;--rail:#0f0f12;--card:#151518;--card2:#1c1c20;--fg:#f1ede6;--soft:#cbc6be;--dim:#8a867f;\
--line:#ffffff14;--line2:#ffffff24;\
--accent:#ffb224;--accent-ink:#17120a;--accent-soft:#ffb22429;--warn:#f0dcac;--warn-bg:#1f1a10;--warn-line:#443a1e;\
--mono:ui-monospace,SFMono-Regular,Menlo,monospace}\
*{box-sizing:border-box}\
html,body{margin:0;padding:0}\
body{min-height:100vh;display:flex;flex-direction:column;padding:24px 16px;background:var(--bg) \
radial-gradient(60% 32% at 50% 0%,#17140f 0%,#17140f00 70%) no-repeat;color:var(--fg);\
-webkit-font-smoothing:antialiased;-webkit-text-size-adjust:100%;\
font:16px/1.65 -apple-system,BlinkMacSystemFont,\"Segoe UI\",\"PingFang SC\",\"Hiragino Sans GB\",\"Microsoft YaHei\",system-ui,sans-serif}\
:focus{outline:none}\
:focus-visible{outline:2px solid var(--accent);outline-offset:3px}\
input:focus-visible,select:focus-visible,textarea:focus-visible{outline-offset:0;border-color:var(--accent)}\
main.card{width:100%;max-width:30rem;margin:auto}\
.card{background:var(--card);border-radius:18px;overflow:hidden;\
box-shadow:inset 0 0 0 1px var(--line),inset 0 1px 0 #ffffff0d,0 30px 80px -40px #000000e6}\
.hero{display:block;width:100%;aspect-ratio:16/9;object-fit:cover;background:#101012;border-bottom:1px solid var(--line)}\
.hero.word{display:flex;align-items:flex-end;padding:20px 22px;\
background:radial-gradient(120% 90% at 12% 0%,hsl(var(--h) 36% 30%),transparent 60%),\
linear-gradient(150deg,hsl(var(--h) 26% 18%),#0b0b0c 82%)}\
.hero.word span{font-size:1.5rem;font-weight:700;letter-spacing:-.01em;line-height:1.3;color:#ffffffeb;\
overflow-wrap:anywhere;display:-webkit-box;-webkit-line-clamp:3;-webkit-box-orient:vertical;overflow:hidden}\
.body{padding:26px 24px 22px}\
h1{margin:0;font-size:1.55rem;line-height:1.35;font-weight:650;letter-spacing:-.015em;overflow-wrap:anywhere}\
h1 .ver{margin-left:.25em;font-size:.9rem;font-weight:500;color:var(--dim);white-space:nowrap}\
.by{display:flex;align-items:center;gap:8px;margin:0 0 10px;font-size:.9rem;font-weight:500;letter-spacing:.02em;color:var(--dim)}\
.by img{width:26px;height:26px;flex:0 0 auto;border-radius:50%;background:var(--card2);object-fit:cover}\
p{margin:14px 0 0}\
.lead{color:var(--dim)}\
.summary{margin:12px 0 0;color:var(--soft);overflow-wrap:anywhere}\
.stamp{margin:12px 0 0;font-family:var(--mono);font-size:.78rem;line-height:1.7;color:var(--dim);\
letter-spacing:.02em;overflow-wrap:anywhere}\
.stamp b{font-weight:400;color:var(--soft)}\
.seats{margin:14px 0 0;font-size:.92rem;color:var(--accent)}\
.seats.full{color:var(--dim)}\
.tip{margin:20px 0 0;padding:14px 16px;border:1px solid var(--warn-line);background:var(--warn-bg);color:var(--warn);border-radius:12px}\
.tip p{margin:0;font-size:.92rem}\
.tip p+p{margin-top:10px}\
[hidden]{display:none}\
.row{display:flex;gap:8px}\
.row input{flex:1;min-width:0}\
.row button{width:auto;margin:0;padding:9px 14px;font-size:.9rem;white-space:nowrap}\
input{width:100%;min-height:44px;padding:10px 12px;border:1px solid var(--line);border-radius:10px;\
background:var(--bg);color:var(--fg);font:inherit;font-size:1rem}\
input::placeholder{color:#5c5955}\
button{-webkit-appearance:none;appearance:none;display:block;width:100%;min-height:44px;margin:24px 0 0;padding:15px;\
border:0;border-radius:12px;background:var(--accent);color:var(--accent-ink);font:inherit;font-size:1.05rem;font-weight:700;\
letter-spacing:.03em;cursor:pointer;transition:transform .08s,filter .15s}\
button:hover{filter:brightness(1.06)}\
button:active{transform:translateY(1px) scale(.995)}\
.more{display:grid;gap:11px;margin:18px 0 0;font-size:.88rem}\
.more>*{margin:0}\
.more a{text-decoration:none}\
.more a:hover{text-decoration:underline;text-underline-offset:3px}\
.more summary{cursor:pointer;color:var(--dim);list-style:none;-webkit-tap-highlight-color:transparent}\
.more summary::-webkit-details-marker{display:none}\
.more summary::before{content:\"+ \";color:var(--accent)}\
.more details[open] summary::before{content:\"\\2212 \"}\
.more details[open] summary{color:var(--fg)}\
.more form{margin:10px 0 0}\
.more input{font-size:.95rem;padding:9px 11px}\
.more .said{color:var(--dim)}\
.voices{margin:22px 0 0;padding-top:16px;border-top:1px solid var(--line)}\
.voices h2{margin:0;font-size:.72rem;font-weight:600;letter-spacing:.1em;text-transform:uppercase;color:var(--dim)}\
.voice{margin:12px 0 0;font-size:.92rem;line-height:1.6;color:var(--soft);overflow-wrap:anywhere}\
.voice cite{display:block;margin-top:2px;font-style:normal;font-family:var(--mono);font-size:.72rem;color:var(--dim)}\
.mine{margin:16px 0 0;padding:0;border-top:1px solid var(--line)}\
.mine li{display:flex;align-items:center;justify-content:space-between;gap:14px;padding:10px 2px;\
border-bottom:1px solid var(--line);list-style:none}\
.mine li>a{color:var(--fg);text-decoration:none}\
.mine form,.pushed{display:flex;align-items:center;gap:10px;margin:0}\
.mine button,.pushed button,.ghost{width:auto;min-height:32px;margin:0;padding:4px 12px;border:1px solid var(--line2);border-radius:999px;\
background:none;color:var(--dim);font-size:.8rem;font-weight:500;letter-spacing:.02em}\
.mine button:hover,.pushed button:hover,.ghost:hover{color:var(--fg);border-color:#ffffff47;filter:none}\
.meta{margin:14px 0 0;text-align:center;font-size:.85rem;color:var(--dim)}\
label{display:block;margin:18px 0 0;font-size:.9rem;color:var(--dim)}\
label input,select,textarea{display:block;width:100%;margin-top:6px;padding:10px 12px;border:1px solid var(--line);\
border-radius:10px;background:var(--bg);color:var(--fg);font:inherit;font-size:1rem}\
textarea{resize:vertical;min-height:6.5em}\
code{padding:2px 7px;border-radius:6px;background:#080809;border:1px solid var(--line);font-size:.95em}\
pre{margin:16px 0 0;padding:14px 16px;border-radius:12px;background:#080809;border:1px solid var(--line);overflow-x:auto}\
pre code{padding:0;border:0;background:none}\
footer{display:flex;justify-content:space-between;gap:14px;margin:22px 0 0;padding-top:16px;\
border-top:1px solid var(--line);font-size:.8rem;color:var(--dim)}\
a{color:var(--dim);text-underline-offset:3px}a:hover{color:var(--fg)}\
@media(prefers-reduced-motion:reduce){*,*::before,*::after{transition:none!important;animation:none!important}}";

/// 页面外壳。`head` 放 OG 之类的额外元信息，`body` 放卡片里面的内容。
pub fn shell(title: &str, head: &str, body: &str) -> String {
    shell_hero(title, head, "", body)
}

/// 带一张封面的外壳（门禁页有封面时）：封面顶到卡片边，正文在它下面。
/// `hero` 是已经拼好的 HTML（调用方负责转义）；空字符串就是没有封面。
pub fn shell_hero(title: &str, head: &str, hero: &str, body: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"zh-CN\">\n<head>\n\
<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1,viewport-fit=cover\">\n\
<title>{title}</title>\n{head}<style>{CSS}</style>\n\
</head>\n<body>\n<main class=\"card\">\n{hero}<div class=\"body\">\n{body}</div>\n</main>\n</body>\n</html>\n",
        title = esc(title),
    )
}

/// 不套卡片的整页外壳，广场与「我的」用：自己的版式、自己那段追加样式。
pub fn page(title: &str, head: &str, extra_css: &str, body: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"zh-CN\">\n<head>\n\
<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1,viewport-fit=cover\">\n\
<title>{title}</title>\n{head}<style>{CSS}{extra_css}</style>\n\
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
    fn shared_css_stays_small() {
        // 每一页都内联这一份；它长一点，每个玩家的第一屏就慢一点（DESIGN §3.3「整页不超过几 KB」）。
        // 6 KB 是这一版的上限：门禁页整页（含这份样式）仍在 11 KB 以内，见 gate.rs 的尺寸测试。
        assert!(CSS.len() < 6 * 1024, "共用样式 {} 字节", CSS.len());
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
}
