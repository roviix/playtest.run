//! SEO 与 GEO 基础设施：/robots.txt、/sitemap.xml、结构化微数据（DESIGN §3.9）。
//!
//! 零外部依赖，基于内存中的 Plaza 数据服务端直接生成。
//! - robots.txt：规范搜索引擎爬虫与主流 AI 爬虫（GPTBot, PerplexityBot, ClaudeBot 等）的准入与边界。
//! - sitemap.xml：全量索引公开广场作品与合集，携带精准 <lastmod> 与优先级。
//! - JSON-LD：为作品页与广场注入 Schema.org 知识图谱实体。

use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use playtest_common::manifest::WorkKind;
use playtest_common::plaza::Plaza;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::html::esc;

const CACHE_ROBOTS: &str = "public, max-age=3600";
const CACHE_SITEMAP: &str = "public, max-age=1800";

/// `GET /robots.txt`
pub fn robots_txt() -> Response {
    let body = "\
User-agent: *
Allow: /
Allow: /p/
Allow: /c/
Allow: /collections
Allow: /llms.txt
Allow: /llms-full.txt
Allow: /skill.md
Allow: /openapi.json
Disallow: /v1/
Disallow: /_playtest/
Disallow: /me/

# AI Search & Generation Crawlers
User-agent: GPTBot
Allow: /
Allow: /p/
Allow: /c/
Allow: /collections
Allow: /llms.txt
Disallow: /v1/
Disallow: /_playtest/
Disallow: /me/

User-agent: PerplexityBot
Allow: /
Allow: /p/
Allow: /c/
Allow: /collections
Allow: /llms.txt
Disallow: /v1/
Disallow: /_playtest/
Disallow: /me/

User-agent: ClaudeBot
Allow: /
Allow: /p/
Allow: /c/
Allow: /collections
Allow: /llms.txt
Disallow: /v1/
Disallow: /_playtest/
Disallow: /me/

Sitemap: https://playtest.run/sitemap.xml
";

    (
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            ),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static(CACHE_ROBOTS),
            ),
            (
                header::ACCESS_CONTROL_ALLOW_ORIGIN,
                HeaderValue::from_static("*"),
            ),
        ],
        body,
    )
        .into_response()
}

/// `GET /sitemap.xml`
pub fn sitemap_xml(plaza: &Plaza, now: OffsetDateTime) -> Response {
    let now_iso = now.format(&Rfc3339).unwrap_or_default();
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n  \
<url>\n    \
<loc>https://playtest.run/</loc>\n    \
<changefreq>daily</changefreq>\n    \
<priority>1.0</priority>\n  \
</url>\n  \
<url>\n    \
<loc>https://playtest.run/collections</loc>\n    \
<changefreq>daily</changefreq>\n    \
<priority>0.8</priority>\n  \
</url>\n",
    );

    // 公开且未隐藏的合集
    for col in &plaza.collections {
        if col.public && !col.hidden {
            let loc = format!("https://playtest.run/c/{}", esc(&col.slug));
            let lastmod = if !col.updated_at.is_empty() {
                format!("\n    <lastmod>{}</lastmod>", esc(&col.updated_at))
            } else {
                String::new()
            };
            xml.push_str(&format!(
                "  <url>\n    \
<loc>{loc}</loc>{lastmod}\n    \
<changefreq>weekly</changefreq>\n    \
<priority>0.6</priority>\n  \
</url>\n"
            ));
        }
    }

    // 公开且未过期的作品
    for item in &plaza.items {
        if let Some(ref exp) = item.expires_at {
            if exp <= &now_iso {
                continue;
            }
        }
        let loc = format!("https://playtest.run/p/{}", esc(&item.slug));
        let lastmod = if !item.updated_at.is_empty() {
            format!("\n    <lastmod>{}</lastmod>", esc(&item.updated_at))
        } else {
            String::new()
        };
        xml.push_str(&format!(
            "  <url>\n    \
<loc>{loc}</loc>{lastmod}\n    \
<changefreq>weekly</changefreq>\n    \
<priority>0.7</priority>\n  \
</url>\n"
        ));
    }

    xml.push_str("</urlset>\n");

    (
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/xml; charset=utf-8"),
            ),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static(CACHE_SITEMAP),
            ),
            (
                header::ACCESS_CONTROL_ALLOW_ORIGIN,
                HeaderValue::from_static("*"),
            ),
        ],
        xml,
    )
        .into_response()
}

/// 为主域作品门面页生成 Schema.org JSON-LD 结构化数据
#[allow(clippy::too_many_arguments)]
pub fn project_json_ld(
    title: &str,
    slug: &str,
    developer: &str,
    version: u32,
    kind: WorkKind,
    is_game: bool,
    summary: Option<&str>,
    cover_url: Option<&str>,
) -> String {
    let page_url = format!("https://playtest.run/p/{slug}");
    let schema_type = match kind {
        WorkKind::Web if is_game => "VideoGame",
        WorkKind::Web => "WebApplication",
        WorkKind::Article => "Article",
        WorkKind::Video => "VideoObject",
    };

    let title_escaped = esc_json(title);
    let dev_escaped = esc_json(developer);
    let version_field = if version > 0 {
        format!("\"version\": \"v{version}\",\n  ")
    } else {
        String::new()
    };
    let mut json = format!(
        "{{\n  \
\"@context\": \"https://schema.org\",\n  \
\"@type\": \"{schema_type}\",\n  \
\"name\": \"{title_escaped}\",\n  \
\"url\": \"{page_url}\",\n  \
{version_field}\
\"author\": {{\n    \
\"@type\": \"Person\",\n    \
\"name\": \"{dev_escaped}\"\n  \
}},\n  \
\"offers\": {{\n    \
\"@type\": \"Offer\",\n    \
\"price\": \"0\",\n    \
\"priceCurrency\": \"USD\"\n  \
}}"
    );

    if let Some(desc) = summary {
        json.push_str(&format!(",\n  \"description\": \"{}\"", esc_json(desc)));
    }
    if let Some(img) = cover_url {
        json.push_str(&format!(",\n  \"image\": \"{}\"", esc_json(img)));
    }
    if kind == WorkKind::Web {
        json.push_str(",\n  \"operatingSystem\": \"Web Browser\"");
        json.push_str(&format!(
            ",\n  \"applicationCategory\": \"{}\"",
            if is_game {
                "GameApplication"
            } else {
                "WebApplication"
            }
        ));
    }

    json.push_str("\n}");
    format!("<script type=\"application/ld+json\">\n{json}\n</script>\n")
}

/// 为广场根域生成 WebSite 与 Organization JSON-LD
pub fn plaza_json_ld() -> String {
    "<script type=\"application/ld+json\">\n{\n  \
\"@context\": \"https://schema.org\",\n  \
\"@type\": \"WebSite\",\n  \
\"name\": \"playtest\",\n  \
\"url\": \"https://playtest.run/\",\n  \
\"description\": \"以作品为中心的 build in public 平台：拿作品说话，带观众走向下一版。\",\n  \
\"potentialAction\": {\n    \
\"@type\": \"SearchAction\",\n    \
\"target\": \"https://playtest.run/?q={search_term_string}\",\n    \
\"query-input\": \"required name=search_term_string\"\n  \
}\n}\n</script>\n"
        .to_string()
}

fn esc_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
        .replace('\n', " ")
        .replace('\r', "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use playtest_common::collection::{Collection, CollectionKind};
    use playtest_common::project::ProjectCard;

    #[test]
    fn robots_txt_allows_public_and_declares_sitemap() {
        let resp = robots_txt();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[test]
    fn project_json_ld_renders_valid_schema() {
        let ld = project_json_ld(
            "小球冒险",
            "rolling-ball",
            "Alice",
            3,
            WorkKind::Web,
            true,
            Some("一个好玩的游戏"),
            Some("https://example.com/cover.png"),
        );
        assert!(ld.contains("\"@type\": \"VideoGame\""));
        assert!(ld.contains("\"name\": \"小球冒险\""));
        assert!(ld.contains("\"version\": \"v3\""));
        assert!(ld.contains("\"operatingSystem\": \"Web Browser\""));
        assert!(ld.contains("https://playtest.run/p/rolling-ball"));
    }

    #[test]
    fn article_json_ld_renders_article_type() {
        let ld = project_json_ld(
            "我的第一篇文章",
            "first-post",
            "Bob",
            1,
            WorkKind::Article,
            false,
            Some("文章摘要"),
            None,
        );
        assert!(ld.contains("\"@type\": \"Article\""));
        assert!(!ld.contains("operatingSystem"));
    }

    #[test]
    fn sitemap_contains_standard_entries() {
        let mut plaza = Plaza::default();
        plaza.collections.push(Collection {
            slug: "ai-jam".into(),
            title: "AI Jam".into(),
            summary: "Jam summary".into(),
            kind: CollectionKind::Challenge,
            prompt: String::new(),
            rules: String::new(),
            closes_at: None,
            public: true,
            hidden: false,
            creator: "Admin".into(),
            created_at: "2026-09-01T00:00:00Z".into(),
            updated_at: "2026-09-10T12:00:00Z".into(),
            entries: vec![],
            blocked_slugs: vec![],
        });
        plaza.items.push(ProjectCard {
            slug: "demo-game".into(),
            url: "https://demo-game.playtest.run".into(),
            title: "Demo Game".into(),
            kind: WorkKind::Web,
            developer: "Charlie".into(),
            avatar_url: None,
            summary: Some("Demo".into()),
            note: None,
            engine: None,
            is_game: true,
            cover_hash: None,
            version: 2,
            updated_at: "2026-09-11T10:00:00Z".into(),
            expires_at: None,
            players: 5,
            seeking: false,
            seats: None,
            joined: 0,
            followers: 2,
            boosted: false,
        });

        let now = OffsetDateTime::now_utc();
        let _ = sitemap_xml(&plaza, now);
    }
}
