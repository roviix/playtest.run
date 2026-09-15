//! 首批文章契约：一份 Markdown 原稿，加上它明确引用的本地位图。
//!
//! 这里同时供 CLI 与 API 使用。CLI 用它决定只打包哪些图片；API 在提交时重新检查并生成
//! 安全 HTML，不能相信客户端自称已经过滤过。原生 HTML、远程图片、相对文件链接与危险协议
//! 都在进入清单前失败；普通 http(s) / mailto 链接和页内锚点保留。

use std::collections::BTreeSet;

use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use pulldown_cmark::{html, CowStr, Event, HeadingLevel, Options, Parser, Tag};

use crate::manifest::validate_path;

const PATH_ENCODE: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}');

pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif"];

pub fn image_mime(path: &str) -> Option<&'static str> {
    match path.rsplit_once('.')?.1.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Article {
    pub title: Option<String>,
    /// 已解析成相对于文章根目录的清单路径，排序且去重。
    pub images: Vec<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ArticleError {
    #[error("The article is empty. Write something before publishing it.")]
    Empty,
    #[error(
        "Articles can't contain raw HTML. Use Markdown instead — the reader does not execute tags, scripts or embeds."
    )]
    RawHtml,
    #[error("Images must be local PNG, JPEG, WebP or GIF files next to the article. Remote images aren't accepted: {0}")]
    RemoteImage(String),
    #[error("This image path points outside the article's own directory: {0}")]
    EscapingImage(String),
    #[error("This image format isn't supported yet: {0} (PNG, JPEG, WebP and GIF are)")]
    UnsupportedImage(String),
    #[error("Links to local files aren't supported yet: {0}. Use image syntax for figures, or a full http(s) link to send people elsewhere.")]
    LocalLink(String),
    #[error("This link uses a scheme that is unsafe or not supported yet: {0}")]
    UnsafeLink(String),
    #[error("This article path isn't valid: {0}")]
    BadPath(String),
}

/// 检查文章并找出标题、显式引用的本地图片。
pub fn inspect(markdown: &str, source_path: &str) -> Result<Article, ArticleError> {
    if markdown.trim().is_empty() {
        return Err(ArticleError::Empty);
    }
    validate_path(source_path).map_err(|error| ArticleError::BadPath(error.to_string()))?;
    let source_dir = source_path.rsplit_once('/').map(|(dir, _)| dir);
    let mut images = BTreeSet::new();
    let mut title = None;
    let mut in_h1 = false;
    let mut h1_text = String::new();

    for event in parser(markdown) {
        match event {
            Event::Html(_) | Event::InlineHtml(_) => return Err(ArticleError::RawHtml),
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }) if title.is_none() => {
                in_h1 = true;
                h1_text.clear();
            }
            Event::End(pulldown_cmark::TagEnd::Heading(HeadingLevel::H1)) if in_h1 => {
                let cleaned = h1_text.split_whitespace().collect::<Vec<_>>().join(" ");
                if !cleaned.is_empty() {
                    title = Some(cleaned);
                }
                in_h1 = false;
            }
            Event::Text(text) | Event::Code(text) if in_h1 => h1_text.push_str(&text),
            Event::Start(Tag::Image { dest_url, .. }) => {
                images.insert(resolve_image(source_dir, &dest_url)?);
            }
            Event::Start(Tag::Link { dest_url, .. }) => validate_link(&dest_url)?,
            _ => {}
        }
    }

    Ok(Article {
        title,
        images: images.into_iter().collect(),
    })
}

/// 重新检查并渲染安全 HTML。图片地址改成作品子域的绝对地址，这样正文嵌在根域邀请函里
/// 仍只会从对应作品读取，继续经过作品权限、Range 与流量上限。
pub fn render(markdown: &str, source_path: &str, origin: &str) -> Result<String, ArticleError> {
    inspect(markdown, source_path)?;
    let source_dir = source_path.rsplit_once('/').map(|(dir, _)| dir);
    let origin = origin.trim_end_matches('/').to_string();
    let events = parser(markdown).map(move |event| match event {
        Event::Start(Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        }) => {
            let path =
                resolve_image(source_dir, &dest_url).expect("inspect 已经检查过同一条图片路径");
            let encoded = path
                .split('/')
                .map(|part| utf8_percent_encode(part, PATH_ENCODE).to_string())
                .collect::<Vec<_>>()
                .join("/");
            Event::Start(Tag::Image {
                link_type,
                dest_url: CowStr::Boxed(format!("{origin}/{encoded}").into_boxed_str()),
                title,
                id,
            })
        }
        other => other,
    });
    let mut output = String::with_capacity(markdown.len().saturating_mul(2));
    html::push_html(&mut output, events);
    Ok(output)
}

fn parser(markdown: &str) -> Parser<'_> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    Parser::new_ext(markdown, options)
}

fn resolve_image(source_dir: Option<&str>, raw: &str) -> Result<String, ArticleError> {
    let target = raw.trim();
    if target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("//")
        || target.contains("://")
        || target.starts_with("data:")
    {
        return Err(ArticleError::RemoteImage(target.to_string()));
    }
    if target.is_empty()
        || target.starts_with('/')
        || target.contains('\\')
        || target.contains('?')
        || target.contains('#')
    {
        return Err(ArticleError::EscapingImage(target.to_string()));
    }
    let mut parts = source_dir
        .into_iter()
        .flat_map(|dir| dir.split('/'))
        .map(str::to_string)
        .collect::<Vec<_>>();
    for part in target.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(ArticleError::EscapingImage(target.to_string()));
                }
            }
            normal => parts.push(normal.to_string()),
        }
    }
    let path = parts.join("/");
    validate_path(&path).map_err(|_| ArticleError::EscapingImage(target.to_string()))?;
    if image_mime(&path).is_none() {
        return Err(ArticleError::UnsupportedImage(path));
    }
    Ok(path)
}

fn validate_link(raw: &str) -> Result<(), ArticleError> {
    let target = raw.trim();
    if target.starts_with('#')
        || target.starts_with("https://")
        || target.starts_with("http://")
        || target.starts_with("mailto:")
    {
        return Ok(());
    }
    if target.contains(':') || target.starts_with("//") {
        return Err(ArticleError::UnsafeLink(target.to_string()));
    }
    Err(ArticleError::LocalLink(target.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_title_and_only_explicit_local_images() {
        let found = inspect(
            "# 一篇文章\n\n![图](<images/a b.png>)\n\n[站外](https://example.com)",
            "notes/post.md",
        )
        .unwrap();
        assert_eq!(found.title.as_deref(), Some("一篇文章"));
        assert_eq!(found.images, ["notes/images/a b.png"]);
    }

    #[test]
    fn rendered_images_stay_on_the_work_origin() {
        let html = render(
            "![说明](<images/一 张.png>)",
            "post.md",
            "https://quiet-fox.playtest.run/",
        )
        .unwrap();
        assert!(
            html.contains(
                "src=\"https://quiet-fox.playtest.run/images/%E4%B8%80%20%E5%BC%A0.png\""
            ),
            "{html}"
        );
        assert!(html.contains("alt=\"说明\""), "{html}");
    }

    #[test]
    fn raw_html_remote_images_and_dangerous_links_are_refused() {
        assert_eq!(
            inspect("<script>alert(1)</script>", "a.md"),
            Err(ArticleError::RawHtml)
        );
        assert!(matches!(
            inspect("![](https://example.com/a.png)", "a.md"),
            Err(ArticleError::RemoteImage(_))
        ));
        assert!(matches!(
            inspect("[点我](javascript:alert(1))", "a.md"),
            Err(ArticleError::UnsafeLink(_))
        ));
        assert!(matches!(
            inspect("![](../outside.png)", "a.md"),
            Err(ArticleError::EscapingImage(_))
        ));
    }
}
