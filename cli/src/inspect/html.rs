//! 从 `index.html` 里读两样东西：它指到了哪些本地文件，以及引擎写在里面的配置。
//!
//! 不引 HTML 解析器：我们只要属性值，读错一两个的代价是少说或多说一句提醒，不改上传行为。
//! 宁可漏掉也不要误报——说不准的（带 `%` 转义、跳出上传目录）一律不算。

/// 一个页面最多往前找多远去取配置里的那个值。
const CONFIG_WINDOW_BYTES: usize = 200;

/// `src=` / `href=` 指到的本地文件，按出现顺序去重。
///
/// 返回的是清单里的样子：相对上传目录、正斜杠、不带开头的 `/`。外链、`data:`、锚点、
/// 目录、看不准的路径都不返回。
pub fn local_references(html: &str) -> Vec<String> {
    // 只把 ASCII 字母变小写，字节位置不变，所以可以拿它的下标去切原文。
    let lower = html.to_ascii_lowercase();
    let mut found: Vec<String> = Vec::new();
    let mut at = 0usize;
    while let Some(value_at) = next_attribute(&lower, at) {
        let (value, after) = read_value(&html[value_at..]);
        at = value_at + after;
        if let Some(path) = as_local_path(value) {
            if !found.contains(&path) {
                found.push(path);
            }
        }
    }
    found
}

/// `<title>` 里的文字，去掉首尾空白、把 HTML 实体里最常见的几个还原。
/// 目录叫 `dist` / `export` 的时候，这是唯一能拿到的作品名；引擎模板默认的那几个（`Unity WebGL Player`、
/// `Vite + TS`）没什么信息量，调用方自己判断要不要用。
pub fn title(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let open = lower.find("<title")?;
    let start = open + lower[open..].find('>')? + 1;
    let end = start + lower[start..].find("</title")?;
    let raw = html[start..end].trim();
    if raw.is_empty() {
        return None;
    }
    let text = raw
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    Some(text)
}

/// 引擎写在 HTML 里的配置项，取 `key` 后面第一个带引号的串。
///
/// Unity 的 `index.html` 长这样：`dataUrl: buildUrl + "/Build/game.data.br"`——
/// 这一串的后缀能说清它到底要哪一套文件。
pub fn config_value(html: &str, key: &str) -> Option<String> {
    let at = html.find(key)? + key.len();
    let rest = &html[at..];
    let quote_at = rest.find(['"', '\''])?;
    if quote_at > CONFIG_WINDOW_BYTES {
        return None;
    }
    let quote = rest.as_bytes()[quote_at] as char;
    let body = &rest[quote_at + 1..];
    let end = body.find(quote)?;
    Some(body[..end].to_string())
}

/// 下一个 `src=` / `href=` 的值从哪个字节开始。
fn next_attribute(lower: &str, from: usize) -> Option<usize> {
    let mut name_at = usize::MAX;
    let mut value_at = None;
    for name in ["src=", "href="] {
        let mut at = from;
        // 用 get 不用切片：`from` 可能已经走到头了，越界要当作「没有了」，不能崩。
        while let Some(offset) = lower.get(at..).and_then(|rest| rest.find(name)) {
            let found = at + offset;
            at = found + name.len();
            // 前面得是空白或引号，否则只是别的词的尾巴（`data-src=`、`xlink:href=`）。
            let standalone = found == 0
                || matches!(lower.as_bytes()[found - 1], b if b.is_ascii_whitespace() || b == b'"' || b == b'\'');
            if standalone {
                if found < name_at {
                    name_at = found;
                    value_at = Some(found + name.len());
                }
                break;
            }
        }
    }
    value_at
}

/// 读一个属性值，返回值本身和从等号后面往前走了多少字节。走的字节数至少是 1，不会原地打转。
fn read_value(rest: &str) -> (&str, usize) {
    let trimmed = rest.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let lead = rest.len() - trimmed.len();
    if let Some(quote) = trimmed.chars().next().filter(|c| *c == '"' || *c == '\'') {
        let body = &trimmed[1..];
        return match body.find(quote) {
            Some(end) => (&body[..end], lead + end + 2),
            None => (body, lead + 1 + body.len()),
        };
    }
    let end = trimmed
        .find(|c: char| c.is_ascii_whitespace() || c == '>')
        .unwrap_or(trimmed.len());
    (&trimmed[..end], (lead + end).max(1))
}

/// 属性值 → 上传目录里的路径。不是本地文件、或者说不准的，返回 `None`。
fn as_local_path(raw: &str) -> Option<String> {
    let value = raw.trim();
    let lower = value.to_ascii_lowercase();
    if lower.is_empty()
        || lower.starts_with('#')
        || lower.starts_with("//")
        || lower.contains("://")
        || lower.starts_with("data:")
        || lower.starts_with("blob:")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
        || lower.starts_with("javascript:")
    {
        return None;
    }
    // 带百分号转义的名字要解码才能和磁盘上的文件对上，解错了就会冤枉一个存在的文件，不如不说。
    if value.contains('%') {
        return None;
    }
    let path = value.split(['?', '#']).next().unwrap_or_default();
    // 门禁页和边缘都把 `/x` 当作上传目录里的 `x`：index.html 就在根上，两种写法落到同一个文件。
    let path = path.trim_start_matches('/');
    let path = path.strip_prefix("./").unwrap_or(path);
    // `href="."` 是「回到这一层」，指的是目录不是文件；`..` 跳出上传目录，说不准指到哪。
    if path.is_empty()
        || path.ends_with('/')
        || path.split('/').any(|part| part == ".." || part == ".")
    {
        return None;
    }
    Some(path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_title_and_normalises_it() {
        assert_eq!(
            title("<html><head><title>\n  跳一跳 &amp; 跑酷  </title></head>"),
            Some("跳一跳 & 跑酷".to_string())
        );
        assert_eq!(
            title("<TITLE lang=\"en\">Deep   Space</TITLE>"),
            Some("Deep Space".to_string())
        );
        assert_eq!(title("<title></title>"), None);
        assert_eq!(title("<h1>没有 title</h1>"), None);
    }

    #[test]
    fn picks_up_scripts_styles_and_images() {
        let html = r#"<!doctype html>
<link rel="icon" href="/favicon.svg">
<link rel="stylesheet" crossorigin href='assets/index-CsUDhMuy.css'>
<script type="module" crossorigin src="/assets/index-B1OnktKc.js"></script>
<img src=hero.png>
"#;
        assert_eq!(
            local_references(html),
            [
                "favicon.svg",
                "assets/index-CsUDhMuy.css",
                "assets/index-B1OnktKc.js",
                "hero.png",
            ]
        );
    }

    #[test]
    fn leaves_out_everything_that_is_not_a_file_here() {
        // 用 r##"…"## 是因为里面有 `"#`（锚点），r#"…"# 会在那里断掉。
        let html = r##"
<a href="https://playtest.run">站</a>
<a href="//cdn.example.com/x.js">协议相对</a>
<a href="#start">锚点</a>
<a href="mailto:a@b.c">信</a>
<img src="data:image/png;base64,AAAA">
<a href="/">根</a>
<a href=".">这一层</a>
<a href="./">还是这一层</a>
<a href="assets/">一个目录</a>
<a href="../外面.png">上一层</a>
<img src="我的%20图.png">
"##;
        assert!(local_references(html).is_empty());
    }

    #[test]
    fn query_strings_and_anchors_come_off() {
        let html = r#"<script src="main.js?v=3"></script><link href="a.css#top">"#;
        assert_eq!(local_references(html), ["main.js", "a.css"]);
    }

    #[test]
    fn the_same_file_twice_is_listed_once() {
        let html = r#"<script src="a.js"></script><script src="./a.js"></script>"#;
        assert_eq!(local_references(html), ["a.js"]);
    }

    /// `data-src` 不是 `src`，别把它当成引用。
    #[test]
    fn attributes_that_only_end_in_src_are_not_src() {
        let html = r#"<div data-src="lazy.png"></div>"#;
        assert!(local_references(html).is_empty());
    }

    #[test]
    fn an_unclosed_quote_does_not_hang_or_panic() {
        assert_eq!(local_references(r#"<script src="a.js"#), ["a.js"]);
        assert!(local_references("src=").is_empty());
        assert!(local_references("").is_empty());
    }

    #[test]
    fn reads_the_unity_config_urls() {
        let html = r#"
      var buildUrl = "Build";
      var config = {
        dataUrl: buildUrl + "/webgl.data.br",
        frameworkUrl: buildUrl + "/webgl.framework.js.br",
        codeUrl: buildUrl + "/webgl.wasm.br",
      };"#;
        assert_eq!(config_value(html, "dataUrl:").unwrap(), "/webgl.data.br");
        assert_eq!(config_value(html, "codeUrl:").unwrap(), "/webgl.wasm.br");
        assert_eq!(config_value(html, "没有这个键"), None);
    }

    /// 键在页面最后、后面再没有引号时不能读出个不相干的值。
    #[test]
    fn a_config_key_with_nothing_after_it_reads_as_nothing() {
        assert_eq!(config_value("dataUrl:", ""), None);
        assert_eq!(config_value("dataUrl: 空", "dataUrl:"), None);
    }
}
