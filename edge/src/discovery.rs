use std::collections::BTreeSet;

use playtest_common::capabilities::Capabilities;
use playtest_common::collection::{Collection, CollectionKind};
use playtest_common::follow::{FollowTarget, MeView};
use playtest_common::plaza::{Plaza, PlazaItem};
use time::format_description::well_known::Rfc3339;

use crate::html::esc;
use crate::plaza::{self, Here, View};

const PAGE_SIZE: usize = 24;
pub const STYLE: &str = include_str!("../../ui/discovery.css");

fn wrap(title: &str, head: &str, here: Here, body: &str) -> String {
    plaza::wrap(title, &format!("{head}<style>{STYLE}</style>"), here, body)
}

#[derive(Default, Clone)]
pub struct Query {
    pub search: String,
    pub model: String,
    pub hot: bool,
    pub page: usize,
    pub join: String,
}

impl Query {
    pub fn parse(raw: &str) -> Self {
        let mut query = Self {
            page: 1,
            ..Self::default()
        };
        for pair in raw.split('&') {
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            let decoded = percent_encoding::percent_decode_str(&value.replace('+', " "))
                .decode_utf8_lossy()
                .chars()
                .take(280)
                .collect::<String>();
            match key {
                "q" => query.search = decoded.trim().to_string(),
                "model" => query.model = decoded,
                "sort" => query.hot = decoded == "hot",
                "page" => query.page = decoded.parse::<usize>().unwrap_or(1).clamp(1, 10000),
                "join" if playtest_common::slug::validate(&decoded).is_ok() => {
                    query.join = decoded;
                }
                _ => {}
            }
        }
        query
    }

    fn link(&self, base: &str, hot: bool, page: usize) -> String {
        format!(
            "{base}?q={}&model={}&sort={}&page={page}",
            encode(&self.search),
            encode(&self.model),
            if hot { "hot" } else { "latest" }
        )
    }
}

fn encode(text: &str) -> String {
    percent_encoding::utf8_percent_encode(text, percent_encoding::NON_ALPHANUMERIC).to_string()
}

fn visible(item: &PlazaItem, now: &str) -> bool {
    item.expires_at.as_deref().is_none_or(|expiry| expiry > now)
}

fn matches(item: &PlazaItem, query: &str) -> bool {
    let text = format!(
        "{} {} {}",
        item.title,
        item.summary.as_deref().unwrap_or_default(),
        item.developer
    )
    .to_lowercase();
    query
        .to_lowercase()
        .split_whitespace()
        .all(|word| text.contains(word))
}

fn collection_matches(collection: &Collection, query: &str) -> bool {
    let text = format!(
        "{} {} {}",
        collection.title, collection.summary, collection.creator
    )
    .to_lowercase();
    query
        .to_lowercase()
        .split_whitespace()
        .all(|word| text.contains(word))
}

fn search(query: &Query, base: &str, collection: Option<&Collection>) -> String {
    let mut models = String::new();
    if let Some(collection) = collection {
        let names: BTreeSet<&str> = collection
            .entries
            .iter()
            .map(|entry| entry.model.as_str())
            .filter(|name| !name.is_empty())
            .collect();
        if !names.is_empty() {
            models.push_str("<label class=\"model-filter\">模型<select name=\"model\"><option value=\"\">所有模型</option>");
            for name in names {
                models.push_str(&format!(
                    "<option value=\"{}\"{}>{}</option>",
                    esc(name),
                    if query.model == name { " selected" } else { "" },
                    esc(name)
                ));
            }
            models.push_str("</select></label>");
        }
    }
    let clear = if !query.search.is_empty() {
        let clear_target = if query.hot { format!("{base}?sort=hot") } else { base.to_string() };
        format!("<a class=\"search-clear\" href=\"{}\" aria-label=\"清空搜索\" title=\"清空搜索\">×</a>", esc(&clear_target))
    } else {
        String::new()
    };
    format!("<form class=\"discover-search\" method=\"get\" action=\"{}\" role=\"search\"><div class=\"search-box\"><label class=\"search-field\"><span class=\"sr-only\">搜索作品与合集</span><input type=\"search\" name=\"q\" maxlength=\"280\" placeholder=\"搜索作品、作者…\" value=\"{}\"></label>{clear}<button type=\"submit\" aria-label=\"搜索\" title=\"搜索\"><svg class=\"icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><circle cx=\"10.5\" cy=\"10.5\" r=\"6.5\"/><path d=\"m16 16 4 4\"/></svg></button></div>{models}<input type=\"hidden\" name=\"sort\" value=\"{}\"></form>", esc(base), esc(&query.search), if query.hot { "hot" } else { "latest" })
}

fn tabs(query: &Query, base: &str) -> String {
    let explanation = if query.hot {
        "<details class=\"sort-help\"><summary aria-label=\"排序说明\" title=\"排序说明\"><svg class=\"icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><circle cx=\"12\" cy=\"12\" r=\"8\"/><path d=\"M12 11v5m0-9v.5\"/></svg></summary><p>按近 7 天点击开始的去重会话排序，无记录时按时间排列。会话数不等于真人数，也不代表作品质量。</p></details>"
    } else {
        ""
    };
    format!("<div class=\"discover-toolbar\"><nav class=\"discover-tabs\" aria-label=\"作品顺序\"><a href=\"{}\" {}>最新</a><a href=\"{}\" {}>近 7 天</a></nav>{explanation}</div>",
        esc(&query.link(base, false, 1)), if !query.hot { "aria-current=\"true\"" } else { "" },
        esc(&query.link(base, true, 1)), if query.hot { "aria-current=\"true\"" } else { "" })
}

fn pages(query: &Query, base: &str, count: usize) -> (usize, String) {
    let last = count.div_ceil(PAGE_SIZE).max(1);
    let current = query.page.max(1).min(last);
    if last == 1 {
        return (0, String::new());
    }
    let mut output = String::from("<nav class=\"discovery-pages\" aria-label=\"分页\">");
    if current > 1 {
        output.push_str(&format!(
            "<a href=\"{}\">上一页</a>",
            esc(&query.link(base, query.hot, current - 1))
        ));
    }
    output.push_str(&format!("<span>第 {current} / {last} 页</span>"));
    if current < last {
        output.push_str(&format!(
            "<a href=\"{}\">下一页</a>",
            esc(&query.link(base, query.hot, current + 1))
        ));
    }
    output.push_str("</nav>");
    ((current - 1) * PAGE_SIZE, output)
}

fn status(collection: &Collection, now: &str) -> String {
    if collection.kind == CollectionKind::Collection {
        return "作品集".to_string();
    }
    if collection.closed(now) {
        return "已结束 · 仍可观看".to_string();
    }
    match collection
        .closes_at
        .as_deref()
        .and_then(|value| time::OffsetDateTime::parse(value, &Rfc3339).ok())
    {
        Some(close) => format!(
            "征集中 · {} 月 {} 日截止（UTC）",
            u8::from(close.month()),
            close.day()
        ),
        None => "开放创作 · 随时加入".to_string(),
    }
}

fn collection_card(collection: &Collection, view: &View<'_>) -> String {
    let now = view.now.format(&Rfc3339).unwrap_or_default();
    let mut art = String::new();
    for entry in collection.entries.iter().take(3) {
        if let Some(item) = view
            .plaza
            .items
            .iter()
            .find(|item| item.slug == entry.slug && visible(item, &now))
        {
            match item.cover_url() {
                Some(url) => art.push_str(&format!(
                    "<img src=\"{}\" alt=\"\" loading=\"lazy\">",
                    esc(&url)
                )),
                None => art.push_str(&format!(
                    "<span>{}</span>",
                    esc(&item.title.chars().take(1).collect::<String>())
                )),
            }
        }
    }
    if art.is_empty() {
        art = "<span aria-hidden=\"true\">＋</span>".to_string();
    }
    let count = collection
        .entries
        .iter()
        .filter(|entry| {
            view.plaza
                .items
                .iter()
                .any(|item| item.slug == entry.slug && visible(item, &now))
        })
        .count();
    let state_class = if collection.kind == CollectionKind::Challenge && !collection.closed(&now) {
        "challenge-open"
    } else if collection.closed(&now) {
        "challenge-closed"
    } else {
        "collection-regular"
    };
    format!("<a class=\"collection-card\" href=\"{}\"><div class=\"collection-art\" aria-hidden=\"true\">{art}</div><div class=\"collection-card-body\"><span class=\"collection-state {}\">{}</span><h2>{}</h2><p>{}</p><div class=\"collection-meta\"><span>{}</span><span class=\"count-badge\">{}</span></div></div></a>",
        esc(&collection.path()), state_class, esc(&status(collection, &now)), esc(&collection.title), esc(&collection.summary), esc(&collection.creator), if count == 0 { "等你带来第一件作品".to_string() } else { format!("{count} 件作品 ↗") })
}

pub fn home(view: &View<'_>, query: &Query, index: bool) -> String {
    let base = if index { "/collections" } else { "/" };
    let now = view.now.format(&Rfc3339).unwrap_or_default();
    let searching = !query.search.is_empty();
    let mut body = format!(
        "<div class=\"content discovery\"><header class=\"workspace-head\"><h1>{}</h1>{}</header>",
        if index { "合集与挑战" } else { "广场" },
        search(query, base, None)
    );
    let collections: Vec<_> = view
        .plaza
        .collections
        .iter()
        .filter(|collection| {
            collection.public && !collection.hidden && collection_matches(collection, &query.search)
        })
        .filter(|_| {
            index || searching
        })
        .collect();
    if !collections.is_empty() {
        if !index {
            body.push_str(&format!("<div class=\"discovery-section\"><h2>{}</h2><a href=\"/collections\">所有合集 →</a></div>", if searching { "合集" } else { "创作挑战" }));
        }
        let (offset, navigation) = if index {
            pages(query, base, collections.len())
        } else {
            (0, String::new())
        };
        body.push_str("<section class=\"collection-grid\" aria-label=\"合集\">");
        for collection in collections
            .iter()
            .skip(offset)
            .take(if index { PAGE_SIZE } else { 3 })
        {
            body.push_str(&collection_card(collection, view));
        }
        body.push_str("</section>");
        body.push_str(&navigation);
    } else if index {
        body.push_str(if searching { "<section class=\"discovery-empty\"><h2>没有找到合集</h2><p>换个词，或者 <a href=\"/collections\">看看所有合集</a>。</p></section>" } else { "<section class=\"discovery-empty\"><h2>还没有公开合集</h2><p>发布作品后，可在控制台创建合集。</p><a href=\"/#publish-dialog\">发布作品 →</a></section>" });
    }
    if !index {
        body.push_str(&tabs(query, base));
        let mut items: Vec<_> = view
            .plaza
            .items
            .iter()
            .filter(|item| visible(item, &now) && matches(item, &query.search))
            .collect();
        items.sort_by(|left, right| {
            let priority = if query.hot {
                right.players.cmp(&left.players)
            } else if !searching {
                right.boosted.cmp(&left.boosted)
            } else {
                std::cmp::Ordering::Equal
            };
            priority
                .then_with(|| right.updated_at.cmp(&left.updated_at))
                .then_with(|| left.slug.cmp(&right.slug))
        });
        let (offset, navigation) = pages(query, base, items.len());
        if items.is_empty() {
            body.push_str(if searching { "<section class=\"discovery-empty\"><h2>没有找到作品</h2><p>试试作品名或作者名，或者 <a href=\"/\">清空搜索</a>。</p></section>" } else { "<section class=\"empty-plaza\"><p>广场上还没有作品。</p><p class=\"lead\"><code>playtest ./dist --public</code> 会把作品放到这里。</p></section>" });
        } else {
            body.push_str("<section class=\"grid\" aria-label=\"公开试玩作品\">");
            for item in items.iter().skip(offset).take(PAGE_SIZE) {
                body.push_str(&plaza::tile(
                    item,
                    item.boosted && !searching && !query.hot,
                    view.now,
                ));
            }
            body.push_str("</section>");
            body.push_str(&navigation);
        }
    }
    body.push_str("</div>");
    let mut html = wrap(
        if index {
            "合集与挑战"
        } else {
            "playtest.run"
        },
        "<meta name=\"description\" content=\"发现浏览器作品，围绕一个好题目一起创作。\">",
        if index {
            Here::Collections
        } else {
            Here::Plaza
        },
        &body,
    );
    if !index && !query.join.is_empty() {
        html = html.replace(
            &format!("{}/console/", playtest_common::DEVELOPER_API_URL),
            &format!(
                "{}/console/#/collections/{}",
                playtest_common::DEVELOPER_API_URL,
                query.join
            ),
        );
    }
    html
}

pub fn collection(
    view: &View<'_>,
    collection: &Collection,
    query: &Query,
    caps: &Capabilities,
    viewer: Option<&MeView>,
) -> String {
    let now_text = view.now.format(&Rfc3339).unwrap_or_default();
    let mut safe = collection.clone();
    safe.entries.retain(|entry| {
        view.plaza
            .items
            .iter()
            .any(|item| item.slug == entry.slug && visible(item, &now_text))
    });
    let collection = &safe;
    let base = collection.path();
    let now = view.now.format(&Rfc3339).unwrap_or_default();
    let target = FollowTarget::Collection {
        slug: collection.slug.clone(),
    };
    let followed =
        viewer.is_some_and(|viewer| viewer.follows.iter().any(|follow| follow.target == target));
    let subscribe = if followed {
        "<a class=\"utility-btn subscribed\" href=\"/me\"><svg class=\"icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"m5 13 4 4L19 7\"/></svg>已关注</a>".to_string()
    } else if !caps.email {
        String::new()
    } else if viewer.is_some_and(|viewer| viewer.email_masked.is_some()) {
        crate::follow::one_click(
            "/follow",
            &target,
            &base,
            "collection",
            "关注新投稿",
            "utility-btn collection-subscribe",
        )
    } else {
        let form = crate::follow::email_details(
            caps,
            "关注新投稿",
            "/follow",
            &target,
            &base,
            "collection",
            "确认关注",
        );
        form.replacen(
            "</form>",
            "<p class=\"tell-hint\">每周有新投稿时汇总一封，可随时退订</p></form>",
            1,
        )
    };
    let state_class = if collection.kind == CollectionKind::Challenge && !collection.closed(&now) {
        "challenge-open"
    } else if collection.closed(&now) {
        "challenge-closed"
    } else {
        "collection-regular"
    };
    let has_prompt = !collection.prompt.is_empty();
    let has_rules = !collection.rules.is_empty();
    let is_open_challenge = collection.kind == CollectionKind::Challenge && !collection.closed(&now);
    let participate_btn = if is_open_challenge {
        "<a class=\"discovery-button primary action-participate\" href=\"#participate\"><svg class=\"icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M12 5v14M5 12h14\"/></svg>我也来做一个 ↗</a>"
    } else {
        ""
    };

    let brief = if has_prompt || has_rules {
        let mut guide_sections = String::new();
        if has_rules {
            guide_sections.push_str(&format!(
                "<div class=\"guide-section\"><h3>投稿规则</h3><p class=\"preserve-lines\">{}</p></div>",
                esc(&collection.rules)
            ));
        }
        if is_open_challenge {
            guide_sections.push_str(&format!(
                "<div class=\"guide-section\"><h3>如何参与</h3>\
<p>把题目交给你喜欢的模型，做一个浏览器里能打开的作品。已有作品也可以投稿，不用重复上传。</p>\
<div class=\"command-pill\"><code>playtest ./dist --public</code></div>\
<p class=\"guide-subtext\">发布后到控制台的「合集与挑战」，选择这个题目和自己的公开作品。临时隧道与到期链接不能留作投稿。</p>\
<a class=\"guide-cta\" href=\"/?join={}#publish-dialog\">发布说明与投稿入口 →</a>\
</div>",
                esc(&collection.slug)
            ));
        }
        let guide = if guide_sections.is_empty() {
            String::new()
        } else {
            format!(
                "<details id=\"participate\" class=\"challenge-guide\">\
<summary><div class=\"guide-summary-label\"><svg class=\"icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><circle cx=\"12\" cy=\"12\" r=\"10\"/><path d=\"M12 16v-4m0-4h.01\"/></svg><span>规则与参与说明</span></div><span class=\"guide-toggle-hint\">展开 ▾</span></summary>\
<div class=\"guide-body\">{guide_sections}</div></details>"
            )
        };
        let prompt_part = if has_prompt {
            format!(
                "<div class=\"brief-bar\"><div class=\"brief-label\"><svg class=\"icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z\"/><path d=\"M14 2v6h6M16 13H8M16 17H8M10 9H8\"/></svg><h2>创作题目</h2></div>\
<button type=\"button\" class=\"utility-btn copy-prompt-btn\" data-copy-prompt hidden><svg class=\"icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><rect x=\"8\" y=\"8\" width=\"12\" height=\"12\" rx=\"2\"/><path d=\"M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3\"/></svg><span>复制题目</span></button></div>\
<div class=\"prompt-canvas\"><pre id=\"challenge-prompt\">{}</pre></div>",
                esc(&collection.prompt)
            )
        } else {
            String::new()
        };
        format!(
            "<section class=\"challenge-brief\" aria-label=\"创作题目\">\
{prompt_part}\
{guide}\
</section>"
        )
    } else {
        String::new()
    };

    let mut body = format!(
        "<div class=\"content discovery\">\
<header class=\"collection-hero\">\
<div class=\"collection-hero-head\">\
<div class=\"collection-hero-main\">\
<div class=\"collection-badges\"><span class=\"collection-state {state_class}\">{}</span><span class=\"collection-creator\">{} 发起</span></div>\
<h1>{}</h1>\
<p class=\"collection-summary\">{}</p>\
</div>\
<div class=\"collection-actions\">{participate_btn}\
<div class=\"collection-utilities\"><button type=\"button\" class=\"utility-btn action-share\" data-share-collection hidden><svg class=\"icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><circle cx=\"18\" cy=\"5\" r=\"3\"/><circle cx=\"6\" cy=\"12\" r=\"3\"/><circle cx=\"18\" cy=\"19\" r=\"3\"/><path d=\"m8.59 13.51 6.83 3.98m-.01-10.98-6.82 3.98\"/></svg><span>分享</span></button>{subscribe}</div>\
</div>\
</div>\
{brief}\
<p role=\"status\" class=\"discovery-note\" data-collection-status></p>\
</header>",
        esc(&status(collection, &now)),
        esc(&collection.creator),
        esc(&collection.title),
        esc(&collection.summary),
    );
    body.push_str("<div class=\"collection-gallery-bar\">");
    body.push_str(&search(query, &base, Some(collection)));
    body.push_str(&tabs(query, &base));
    body.push_str("</div>");
    let mut entries: Vec<_> = collection
        .entries
        .iter()
        .filter_map(|entry| {
            let item = view.plaza.items.iter().find(|item| {
                item.slug == entry.slug && visible(item, &now) && matches(item, &query.search)
            })?;
            if !query.model.is_empty() && entry.model != query.model {
                return None;
            }
            Some((entry, item))
        })
        .collect();
    entries.sort_by(|(left_entry, left), (right_entry, right)| {
        (if query.hot {
            right.players.cmp(&left.players)
        } else {
            std::cmp::Ordering::Equal
        })
        .then_with(|| right_entry.submitted_at.cmp(&left_entry.submitted_at))
        .then_with(|| left.slug.cmp(&right.slug))
    });
    let (offset, navigation) = pages(query, &base, entries.len());
    if entries.is_empty() {
        body.push_str(&format!(
            "<section class=\"discovery-empty\"><h2>{}</h2><p>{}</p></section>",
            if query.search.is_empty() && query.model.is_empty() {
                "还没有作品"
            } else {
                "没有符合条件的作品"
            },
            if query.search.is_empty() && query.model.is_empty() {
                "作品投稿后会显示在这里。".to_string()
            } else {
                format!("<a href=\"{}\">清空条件</a>，看看其他答案。", esc(&base))
            }
        ));
    } else {
        body.push_str("<section class=\"grid collection-entries\" aria-label=\"投稿作品\">");
        for (entry, item) in entries.iter().skip(offset).take(PAGE_SIZE) {
            let tile = plaza::tile(item, false, view.now).replacen(
                &format!("href=\"/p/{}\"", item.slug),
                &format!(
                    "href=\"/p/{}?collection={}&amp;from=collection\"",
                    item.slug, collection.slug
                ),
                1,
            );
            let model_name = if entry.model.is_empty() {
                "模型未填写"
            } else {
                &entry.model
            };
            let version_diff = if entry.submitted_version != item.version {
                format!(" · 当前 v{}（作品已更新，打开的是当前版本）", item.version)
            } else {
                String::new()
            };
            let prompt_box = if entry.prompt.is_empty() {
                "<p class=\"empty-prompt\">作者没有公开提示词。</p>".to_string()
            } else {
                format!("<div class=\"prompt-box\"><span class=\"box-label\">公开提示词</span><pre>{}</pre></div>", esc(&entry.prompt))
            };
            body.push_str(&format!(
                "<article class=\"collection-entry\">{tile}\
<details class=\"creation-note\">\
<summary class=\"creation-summary\"><span class=\"model-tag\">{}</span><span class=\"method-tag\">{}</span><span class=\"version-tag\">投稿 v{}{}</span></summary>\
<div class=\"creation-body\">\
<p class=\"creation-disclaimer\">作者填写，未经平台认证。</p>\
<p class=\"creation-time\">投稿时间 · {}</p>\
{}\
</div></details></article>",
                esc(model_name),
                entry.method.label(),
                entry.submitted_version,
                version_diff,
                esc(&entry.submitted_at),
                prompt_box
            ));
        }
        body.push_str("</section>");
        body.push_str(&navigation);
    }
    body.push_str("</div>");
    let head = format!("<meta property=\"og:type\" content=\"website\"><meta property=\"og:title\" content=\"{}\"><meta property=\"og:description\" content=\"{}\"><meta name=\"description\" content=\"{}\">", esc(&collection.title), esc(&collection.summary), esc(&collection.summary));
    wrap(&collection.title, &head, Here::Collections, &body)
}

pub fn context(plaza: &Plaza, slug: &str, collection_slug: Option<&str>) -> String {
    let Some(collection) = plaza.collections.iter().find(|collection| {
        collection.public
            && !collection.hidden
            && collection_slug.is_none_or(|wanted| wanted == collection.slug)
            && collection.entries.iter().any(|entry| entry.slug == slug)
    }) else {
        return String::new();
    };
    let now = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default();
    let entries: Vec<_> = collection
        .entries
        .iter()
        .filter(|entry| {
            plaza
                .items
                .iter()
                .any(|item| item.slug == entry.slug && visible(item, &now))
        })
        .collect();
    let Some(position) = entries.iter().position(|entry| entry.slug == slug) else {
        return String::new();
    };
    let next = if entries.len() > 1 {
        format!(
            "<a href=\"/p/{}?collection={}&amp;from=collection\">下一件 →</a>",
            entries[(position + 1) % entries.len()].slug,
            collection.slug
        )
    } else {
        String::new()
    };
    format!("<nav class=\"collection-context\" aria-label=\"同合集作品\"><a href=\"{}\">← {}</a>{next}</nav>", esc(&collection.path()), esc(&collection.title))
}
