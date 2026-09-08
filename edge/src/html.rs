//! 玩家看到的每一页都在这里拼出来。
//!
//! 不引模板引擎：这几页加起来不到两百行，模板引擎换来的是一层编译期魔法和一个新依赖。
//! 代价是**每一处插值都必须自己调 [`esc`]**——清单里的作品名和开发者名是用户输入，
//! 这里是玩家域，漏一个就是 XSS。

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

/// 全站唯一一份样式，内联进每一页。不引外部脚本与字体：玩家可能在微信里、
/// 在很慢的网上，多一个请求就多一次白屏。
pub const CSS: &str = "\
:root{color-scheme:dark;--bg:#0e1013;--card:#171a1f;--fg:#e9ecf1;--dim:#98a1ae;--line:#272c34;--accent:#4f8cff}\
*{box-sizing:border-box}\
html,body{margin:0;padding:0}\
body{min-height:100vh;display:flex;padding:24px 18px;\
background:var(--bg);color:var(--fg);-webkit-font-smoothing:antialiased;-webkit-text-size-adjust:100%;\
font:16px/1.65 -apple-system,BlinkMacSystemFont,\"Segoe UI\",\"PingFang SC\",\"Hiragino Sans GB\",\"Microsoft YaHei\",system-ui,sans-serif}\
/* auto 外边距居中，比 align-items:center 稳：内容比屏幕高时不会把顶部截掉、滚不上去。 */\
main{width:100%;max-width:30rem;margin:auto}\
.card{background:var(--card);border:1px solid var(--line);border-radius:16px;padding:28px 24px}\
h1{margin:0;font-size:1.5rem;line-height:1.4;font-weight:600;overflow-wrap:anywhere}\
h1 .invite{display:block;margin-bottom:6px;font-size:.95rem;font-weight:400;color:var(--dim)}\
h1 .ver{margin-left:.2em;font-size:.95rem;font-weight:400;color:var(--dim);white-space:nowrap}\
p{margin:14px 0 0}\
.lead{color:var(--dim)}\
.note{margin:20px 0 0;padding:14px 16px;background:#1d2128;border-radius:12px}\
.note h2{margin:0 0 2px;font-size:.75rem;font-weight:600;letter-spacing:.08em;color:var(--dim)}\
.note p{margin:0;white-space:pre-wrap;overflow-wrap:anywhere}\
.tip{margin:20px 0 0;padding:14px 16px;border:1px solid #453a1f;background:#221d11;color:#f0dcac;border-radius:12px}\
.tip p{margin:0;font-size:.92rem}\
.tip p+p{margin-top:10px}\
[hidden]{display:none}\
/* 复制链接那一行：输入框占满、按钮跟在后面，不用上面那条整宽按钮的样式。 */\
.row{display:flex;gap:8px}\
.row input{flex:1;min-width:0;padding:9px 10px;border:1px solid var(--line);border-radius:8px;\
background:#0e1013;color:var(--fg);font:inherit;font-size:.82rem}\
.row button{width:auto;margin:0;padding:9px 14px;font-size:.9rem;white-space:nowrap}\
button{-webkit-appearance:none;appearance:none;display:block;width:100%;margin:24px 0 0;padding:16px;\
border:0;border-radius:12px;background:var(--accent);color:#fff;font:inherit;font-size:1.1rem;font-weight:600;cursor:pointer}\
button:hover{filter:brightness(1.08)}\
button:active{transform:translateY(1px)}\
.meta{margin:14px 0 0;text-align:center;font-size:.85rem;color:var(--dim)}\
label{display:block;margin:18px 0 0;font-size:.9rem;color:var(--dim)}\
select,textarea{display:block;width:100%;margin-top:6px;padding:11px 12px;border:1px solid var(--line);\
border-radius:10px;background:#0e1013;color:var(--fg);font:inherit;font-size:1rem}\
textarea{resize:vertical;min-height:6.5em}\
code{padding:2px 7px;border-radius:6px;background:#0a0c0f;border:1px solid var(--line);font-size:.95em}\
pre{margin:16px 0 0;padding:14px 16px;border-radius:12px;background:#0a0c0f;border:1px solid var(--line);overflow-x:auto}\
pre code{padding:0;border:0;background:none}\
footer{display:flex;justify-content:space-between;gap:14px;margin:22px 0 0;padding-top:16px;\
border-top:1px solid var(--line);font-size:.8rem;color:var(--dim)}\
a{color:var(--dim)}a:hover{color:var(--fg)}";

/// 页面外壳。`head` 放 OG 之类的额外元信息，`body` 放 `<main>` 里面的内容。
pub fn shell(title: &str, head: &str, body: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"zh-CN\">\n<head>\n\
<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1,viewport-fit=cover\">\n\
<title>{title}</title>\n{head}<style>{CSS}</style>\n\
</head>\n<body>\n<main class=\"card\">\n{body}</main>\n</body>\n</html>\n",
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
}
