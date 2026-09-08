//! 玩家看到的每一页都在这里拼出来。
//!
//! 不引模板引擎：这几页加起来不到两百行，模板引擎换来的是一层编译期魔法和一个新依赖。
//! 代价是**每一处插值都必须自己调 [`esc`]**——清单里的作品名和开发者名是用户输入，
//! 这里是玩家域，漏一个就是 XSS。
//!
//! 长相（DESIGN §3.8「长相」）：深色、安静、封面优先，像一排游戏机的柜面，不像商店首页。
//! 一套记号（颜色、圆角、字号）门禁页、广场、错误页共用，玩家从广场点进门禁页时不觉得换了地方。

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

/// 全站共用的一份样式，内联进每一页。不引外部脚本与字体：玩家可能在微信里、
/// 在很慢的网上，多一个请求就多一次白屏。广场页在这之上再加自己的一段（`plaza.rs`）。
pub const CSS: &str = "\
:root{color-scheme:dark;--bg:#0a0b0e;--card:#14161b;--card2:#1b1e25;--fg:#f2f3f5;--dim:#9aa2b1;--line:#262a33;\
--accent:#ffb224;--accent-ink:#17120a;--warn:#f0dcac;--warn-bg:#241d10;--warn-line:#4a3b1c}\
*{box-sizing:border-box}\
html,body{margin:0;padding:0}\
body{min-height:100vh;display:flex;flex-direction:column;padding:24px 16px;background:var(--bg) \
radial-gradient(70% 36% at 50% 0%,#171b26 0%,rgba(23,27,38,0) 70%) no-repeat;color:var(--fg);\
-webkit-font-smoothing:antialiased;-webkit-text-size-adjust:100%;\
font:16px/1.65 -apple-system,BlinkMacSystemFont,\"Segoe UI\",\"PingFang SC\",\"Hiragino Sans GB\",\"Microsoft YaHei\",system-ui,sans-serif}\
/* auto 外边距居中，比 align-items:center 稳：内容比屏幕高时不会把顶部截掉、滚不上去。 */\
main.card{width:100%;max-width:30rem;margin:auto}\
.card{background:var(--card);border:1px solid var(--line);border-radius:20px;overflow:hidden;\
box-shadow:0 24px 60px -30px rgba(0,0,0,.8)}\
.hero{display:block;width:100%;aspect-ratio:16/9;object-fit:cover;background:#0e1014;border-bottom:1px solid var(--line)}\
.body{padding:26px 24px 22px}\
h1{margin:0;font-size:1.55rem;line-height:1.35;font-weight:650;letter-spacing:-.01em;overflow-wrap:anywhere}\
h1 .invite{display:block;margin-bottom:8px;font-size:.9rem;font-weight:500;letter-spacing:.02em;color:var(--dim)}\
h1 .ver{margin-left:.25em;font-size:.9rem;font-weight:500;color:var(--dim);white-space:nowrap}\
p{margin:14px 0 0}\
.lead{color:var(--dim)}\
.summary{margin:12px 0 0;color:#c9cfda;overflow-wrap:anywhere}\
.note{margin:20px 0 0;padding:14px 16px;background:var(--card2);border-radius:14px}\
.note h2{margin:0 0 4px;font-size:.72rem;font-weight:600;letter-spacing:.1em;text-transform:uppercase;color:var(--dim)}\
.note p{margin:0;white-space:pre-wrap;overflow-wrap:anywhere}\
.tip{margin:20px 0 0;padding:14px 16px;border:1px solid var(--warn-line);background:var(--warn-bg);color:var(--warn);border-radius:14px}\
.tip p{margin:0;font-size:.92rem}\
.tip p+p{margin-top:10px}\
[hidden]{display:none}\
/* 复制链接那一行：输入框占满、按钮跟在后面，不用上面那条整宽按钮的样式。 */\
.row{display:flex;gap:8px}\
.row input{flex:1;min-width:0;padding:9px 10px;border:1px solid var(--line);border-radius:10px;\
background:var(--bg);color:var(--fg);font:inherit;font-size:.82rem}\
.row button{width:auto;margin:0;padding:9px 14px;font-size:.9rem;white-space:nowrap}\
button{-webkit-appearance:none;appearance:none;display:block;width:100%;margin:24px 0 0;padding:16px;\
border:0;border-radius:14px;background:var(--accent);color:var(--accent-ink);font:inherit;font-size:1.1rem;font-weight:700;\
letter-spacing:.04em;cursor:pointer;transition:transform .08s,filter .15s}\
button:hover{filter:brightness(1.06)}\
button:active{transform:translateY(1px) scale(.995)}\
.meta{margin:14px 0 0;text-align:center;font-size:.85rem;color:var(--dim)}\
label{display:block;margin:18px 0 0;font-size:.9rem;color:var(--dim)}\
select,textarea{display:block;width:100%;margin-top:6px;padding:11px 12px;border:1px solid var(--line);\
border-radius:12px;background:var(--bg);color:var(--fg);font:inherit;font-size:1rem}\
textarea{resize:vertical;min-height:6.5em}\
code{padding:2px 7px;border-radius:6px;background:#090a0d;border:1px solid var(--line);font-size:.95em}\
pre{margin:16px 0 0;padding:14px 16px;border-radius:14px;background:#090a0d;border:1px solid var(--line);overflow-x:auto}\
pre code{padding:0;border:0;background:none}\
footer{display:flex;justify-content:space-between;gap:14px;margin:22px 0 0;padding-top:16px;\
border-top:1px solid var(--line);font-size:.8rem;color:var(--dim)}\
a{color:var(--dim)}a:hover{color:var(--fg)}";

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

/// 不套卡片的整页外壳，广场用：自己的版式、自己那段追加样式。
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
        assert!(CSS.len() < 3800, "共用样式 {} 字节", CSS.len());
    }

    #[test]
    fn hero_sits_above_the_body() {
        let html = shell_hero("t", "", "<img class=\"hero\" src=\"/x\">", "<h1>x</h1>");
        let hero = html.find("class=\"hero\"").unwrap();
        let body = html.find("class=\"body\"").unwrap();
        assert!(hero < body);
        assert!(!shell("t", "", "<h1>x</h1>").contains("class=\"hero\""));
    }
}
