use std::collections::BTreeMap;

use crate::html::esc;

pub struct ArticleView {
    pub body: String,
    pub contents: String,
}

pub fn article_view(source: &str, title: &str) -> ArticleView {
    let mut body = String::with_capacity(source.len());
    let mut contents = String::new();
    let mut remaining = source;
    let mut counts = BTreeMap::new();
    let mut first_heading = true;
    while let Some((offset, level)) = (1..=6)
        .filter_map(|level| remaining.find(&format!("<h{level}>")).map(|offset| (offset, level)))
        .min()
    {
        let closing = format!("</h{level}>");
        let inner_start = offset + 4;
        let Some(length) = remaining[inner_start..].find(&closing) else {
            break;
        };
        let inner = &remaining[inner_start..inner_start + length];
        let plain = heading_text(inner);
        body.push_str(&remaining[..offset]);
        let norm_title = normalize(title);
        let duplicate = first_heading && (plain == norm_title || plain == esc(&norm_title));
        first_heading = false;
        if !duplicate {
            let digest = playtest_common::hash::hash_bytes(plain.as_bytes());
            let count = counts.entry(digest[..16].to_string()).or_insert(0);
            *count += 1;
            let anchor = format!("section-{}-{count}", &digest[..16]);
            body.push_str(&format!("<h{level} id=\"{anchor}\">{inner}</h{level}>"));
            if !plain.is_empty() && level <= 3 {
                contents.push_str(&format!(
                    "<li class=\"toc-level-{level}\"><a href=\"#{anchor}\">{plain}</a></li>"
                ));
            }
        }
        remaining = &remaining[inner_start + length + closing.len()..];
    }
    body.push_str(remaining);
    if !contents.is_empty() {
        contents = format!("<details class=\"article-toc\"><summary>本文目录</summary><nav aria-label=\"本文目录\"><ol>{contents}</ol></nav></details>");
    }
    ArticleView { body, contents }
}

fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn heading_text(inner: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    for character in inner.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(character),
            _ => {}
        }
    }
    normalize(&text)
}

use playtest_common::manifest::ChapterEntry;

/// 章节目录抽屉组件（DESIGN §3.17）。
/// 包含已发布章节列表，高亮标记当前章节。
pub fn chapter_toc(chapters: &[ChapterEntry], current_id: &str, slug: &str) -> String {
    if chapters.is_empty() {
        return String::new();
    }
    let mut items = String::new();
    for (idx, chapter) in chapters.iter().enumerate() {
        let is_current = chapter.id == current_id;
        let current_attr = if is_current {
            " aria-current=\"page\" class=\"is-current\""
        } else {
            ""
        };
        let num = idx + 1;
        items.push_str(&format!(
            "<li><a href=\"/p/{slug}?chapter={chapter_id}\"{current_attr}><span class=\"chapter-num\">{num}</span><span class=\"chapter-title\">{title}</span></a></li>\n",
            slug = esc(slug),
            chapter_id = esc(&chapter.id),
            num = num,
            title = esc(&chapter.title),
            current_attr = current_attr,
        ));
    }
    let total = chapters.len();
    format!(
        "<details class=\"chapter-toc\" id=\"chapter-toc\">\n\
<summary class=\"chapter-toc-trigger\"><span class=\"chapter-toc-label\">章节目录</span><span class=\"chapter-toc-badge\">共 {total} 章</span></summary>\n\
<nav class=\"chapter-toc-nav\" aria-label=\"小说章节目录\">\n\
<ol class=\"chapter-list\">\n{items}</ol>\n\
</nav>\n\
</details>\n"
    )
}

/// 章末翻页与状态导航（DESIGN §3.17）。
/// 提供上一章 / 目录 / 下一章；最新一章标示「已读到最新章节 · 连载中」。
pub fn chapter_pagination(chapters: &[ChapterEntry], current_index: usize, slug: &str) -> String {
    if chapters.is_empty() {
        return String::new();
    }
    let prev = if current_index > 0 {
        let p = &chapters[current_index - 1];
        format!(
            "<a class=\"chapter-step prev\" href=\"/p/{slug}?chapter={id}\" rel=\"prev\">‹ 上一章：{title}</a>",
            slug = esc(slug),
            id = esc(&p.id),
            title = esc(&p.title)
        )
    } else {
        "<span class=\"chapter-step disabled\">这是第一章</span>".to_string()
    };
    let toc = "<a class=\"chapter-step toc-link\" href=\"#chapter-toc\">目录</a>".to_string();
    let next = if current_index + 1 < chapters.len() {
        let n = &chapters[current_index + 1];
        format!(
            "<a class=\"chapter-step next\" href=\"/p/{slug}?chapter={id}\" rel=\"next\">下一章：{title} ›</a>",
            slug = esc(slug),
            id = esc(&n.id),
            title = esc(&n.title)
        )
    } else {
        "<span class=\"chapter-step latest-note\">已读到最新章节 · 连载中</span>".to_string()
    };
    format!(
        "<nav class=\"chapter-pagination\" aria-label=\"章节翻页\">\n\
<div class=\"chapter-nav-row\">{prev}{toc}{next}</div>\n\
</nav>\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_only_a_matching_first_title() {
        let view = article_view("<h1>雨<strong>夜</strong></h1><p>正文</p><h2>雨夜</h2>", "雨夜");
        assert!(!view.body.contains("<h1>"));
        assert!(view.body.contains("<p>正文</p>"));
        assert!(view.contents.contains("雨夜</a>"));
        assert!(article_view("<h1>另一个标题</h1>", "雨夜").body.contains("<h1 id="));
    }

    #[test]
    fn section_links_are_stable_unique_and_escaped() {
        let first = article_view("<h2>A &amp; B</h2><h2>A &amp; B</h2>", "文章");
        let second = article_view("<h2>新增小节</h2><h2>A &amp; B</h2>", "文章");
        let anchor = first.body.split("id=\"").nth(1).unwrap().split('"').next().unwrap();
        assert!(second.body.contains(anchor));
        assert_eq!(first.body.matches(anchor).count(), 1);
        assert!(first.contents.contains("A &amp; B</a>"));
        assert!(!first.contents.contains("<strong>"));
    }

    #[test]
    fn no_headings_means_no_empty_directory() {
        let view = article_view("<p>完整正文</p><pre>&lt;h2&gt;代码&lt;/h2&gt;</pre>", "文章");
        assert!(view.contents.is_empty());
        assert!(view.body.contains("完整正文"));
    }

    #[test]
    fn chapter_toc_and_pagination_render_cleanly() {
        let chapters = vec![
            ChapterEntry {
                id: "c1".into(),
                title: "沉睡的三百年".into(),
                path: "01.md".into(),
                hash: "hash1".into(),
                size: 100,
            },
            ChapterEntry {
                id: "c2".into(),
                title: "奥尔特云的谐波".into(),
                path: "02.md".into(),
                hash: "hash2".into(),
                size: 200,
            },
        ];

        let toc = chapter_toc(&chapters, "c1", "beacon");
        assert!(toc.contains("class=\"chapter-toc\""));
        assert!(toc.contains("共 2 章"));
        assert!(toc.contains("href=\"/p/beacon?chapter=c1\" aria-current=\"page\""));
        assert!(toc.contains("沉睡的三百年"));
        assert!(toc.contains("href=\"/p/beacon?chapter=c2\""));

        let p1 = chapter_pagination(&chapters, 0, "beacon");
        assert!(p1.contains("这是第一章"));
        assert!(p1.contains("下一章：奥尔特云的谐波 ›"));

        let p2 = chapter_pagination(&chapters, 1, "beacon");
        assert!(p2.contains("‹ 上一章：沉睡的三百年"));
        assert!(p2.contains("已读到最新章节 · 连载中"));
    }
}
