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
            "{base}?q={}&sort={}&page={page}",
            encode(&self.search),
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

fn search(query: &Query, base: &str) -> String {
    let clear = if !query.search.is_empty() {
        let clear_target = if query.hot {
            format!("{base}?sort=hot")
        } else {
            base.to_string()
        };
        format!(
            "<a class=\"search-clear\" href=\"{}\" aria-label=\"Clear search\" title=\"Clear search\">×</a>",
            esc(&clear_target)
        )
    } else {
        String::new()
    };
    format!(
        "<form class=\"discover-search\" method=\"get\" action=\"{}\" role=\"search\">\
         <div class=\"search-box\">\
         <span class=\"search-icon\" aria-hidden=\"true\">\
         <svg class=\"icon\" viewBox=\"0 0 24 24\"><circle cx=\"10.5\" cy=\"10.5\" r=\"6.5\"/><path d=\"m16 16 4 4\"/></svg>\
         </span>\
         <label class=\"search-field\">\
         <span class=\"sr-only\">Search projects and collections</span>\
         <input type=\"search\" name=\"q\" maxlength=\"280\" placeholder=\"Search projects, creators…\" value=\"{}\" autocomplete=\"off\" spellcheck=\"false\">\
         </label>\
         {clear}\
         <kbd class=\"search-kbd\" aria-hidden=\"true\" title=\"Press / to search\">/</kbd>\
         <button type=\"submit\" class=\"search-submit\" aria-label=\"Search\" title=\"Search\">\
         <svg class=\"icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><circle cx=\"10.5\" cy=\"10.5\" r=\"6.5\"/><path d=\"m16 16 4 4\"/></svg>\
         </button>\
         </div>\
         <input type=\"hidden\" name=\"sort\" value=\"{}\">\
         </form>",
        esc(base),
        esc(&query.search),
        if query.hot { "hot" } else { "latest" }
    )
}

fn sort_bar(query: &Query, base: &str) -> String {
    let explanation = if query.hot {
        "<details class=\"sort-help\"><summary aria-label=\"Sorting explanation\" title=\"Sorting explanation\"><svg class=\"icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><circle cx=\"12\" cy=\"12\" r=\"8\"/><path d=\"M12 11v5m0-9v.5\"/></svg></summary><p>Sorted by unique sessions started in the last 7 days; falls back to chronological order. Sessions do not equal unique people or project quality.</p></details>"
    } else {
        ""
    };
    format!(
        "<div class=\"discover-sort\" role=\"group\" aria-label=\"Sort projects\">\
         <span class=\"sort-icon\" aria-hidden=\"true\" title=\"Sort\"><svg class=\"icon\" viewBox=\"0 0 24 24\" width=\"13\" height=\"13\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M3 6h18M7 12h10M10 18h4\"/></svg></span>\
         <nav class=\"sort-options\" aria-label=\"Sort options\">\
         <a href=\"{}\" class=\"sort-item{}\" {}>Latest</a>\
         <span class=\"sort-sep\" aria-hidden=\"true\">/</span>\
         <a href=\"{}\" class=\"sort-item{}\" {} title=\"Sort by 7-day playtest activity\">Popular</a>\
         </nav>{explanation}</div>",
        esc(&query.link(base, false, 1)),
        if !query.hot { " active" } else { "" },
        if !query.hot { "aria-current=\"true\"" } else { "" },
        esc(&query.link(base, true, 1)),
        if query.hot { " active" } else { "" },
        if query.hot { "aria-current=\"true\"" } else { "" },
    )
}

fn toolbar(query: &Query, base: &str, index: bool) -> String {
    let segmented = format!(
        "<nav class=\"discovery-segmented\" aria-label=\"Discovery category\">\
         <a href=\"/\" class=\"seg-item{}\"{}>Projects</a>\
         <a href=\"/collections\" class=\"seg-item{}\"{}>Collections</a>\
         </nav>",
        if !index { " active" } else { "" },
        if !index { " aria-current=\"page\"" } else { "" },
        if index { " active" } else { "" },
        if index { " aria-current=\"page\"" } else { "" },
    );
    let search_form = search(query, base);
    let sort = if !index {
        sort_bar(query, base)
    } else {
        String::new()
    };
    format!("<div class=\"discover-toolbar\">{segmented}<div class=\"discover-tools\">{search_form}{sort}</div></div>")
}

fn pages(query: &Query, base: &str, count: usize) -> (usize, String) {
    let last = count.div_ceil(PAGE_SIZE).max(1);
    let current = query.page.max(1).min(last);
    if last == 1 {
        return (0, String::new());
    }
    let mut output = String::from("<nav class=\"discovery-pages\" aria-label=\"Pagination\">");
    if current > 1 {
        output.push_str(&format!(
            "<a href=\"{}\">Previous</a>",
            esc(&query.link(base, query.hot, current - 1))
        ));
    }
    output.push_str(&format!("<span>Page {current} of {last}</span>"));
    if current < last {
        output.push_str(&format!(
            "<a href=\"{}\">Next</a>",
            esc(&query.link(base, query.hot, current + 1))
        ));
    }
    output.push_str("</nav>");
    ((current - 1) * PAGE_SIZE, output)
}

fn status(collection: &Collection, now: &str) -> String {
    if collection.kind == CollectionKind::Collection {
        return "Collection".to_string();
    }
    if collection.closed(now) {
        return "Ended · Still playable".to_string();
    }
    match collection
        .closes_at
        .as_deref()
        .and_then(|value| time::OffsetDateTime::parse(value, &Rfc3339).ok())
    {
        Some(close) => format!(
            "Open · Submissions close {} {} (UTC)",
            crate::when::month_short(close.month()),
            close.day()
        ),
        None => "Open challenge · Join anytime".to_string(),
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
    let h = crate::html::hue(&collection.slug);
    if art.is_empty() {
        let glyph = collection.title.chars().next().unwrap_or('★');
        art = format!(
            "<div class=\"collection-empty-medallion\"><span class=\"medallion-glyph\">{}</span></div>",
            esc(&glyph.to_string())
        );
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
    format!("<a class=\"collection-card\" href=\"{}\"><div class=\"collection-art\" style=\"--h:{h}\" aria-hidden=\"true\"><span class=\"collection-state {}\">{}</span>{art}</div><div class=\"collection-card-body\"><h2>{}</h2><p>{}</p><div class=\"collection-meta\"><span>{}</span><span class=\"count-badge\">{}</span></div></div></a>",
        esc(&collection.path()), state_class, esc(&status(collection, &now)), esc(&collection.title), esc(&collection.summary), esc(&collection.creator), if count == 0 { "Be the first to submit".to_string() } else if count == 1 { "1 project ↗".to_string() } else { format!("{count} projects ↗") })
}

pub fn home(view: &View<'_>, query: &Query, index: bool) -> String {
    let base = if index { "/collections" } else { "/" };
    let now = view.now.format(&Rfc3339).unwrap_or_default();
    let searching = !query.search.is_empty();
    let hero_banner = if !searching && query.page <= 1 {
        crate::plaza::hero()
    } else {
        String::new()
    };
    let title = if index { "Collections" } else { "Plaza" };
    let mut body = format!(
        "<div class=\"content discovery\">{hero_banner}<h1 class=\"sr-only\">{title}</h1>"
    );
    body.push_str(&toolbar(query, base, index));
    let collections: Vec<_> = view
        .plaza
        .collections
        .iter()
        .filter(|collection| {
            collection.public && !collection.hidden && collection_matches(collection, &query.search)
        })
        .filter(|_| {
            index
        })
        .collect();
    if !collections.is_empty() {
        if !index {
            body.push_str(&format!("<div class=\"discovery-section\"><h2>{}</h2><a href=\"/collections\">All collections →</a></div>", if searching { "Collections" } else { "Challenges" }));
        }
        let (offset, navigation) = if index {
            pages(query, base, collections.len())
        } else {
            (0, String::new())
        };
        body.push_str("<section class=\"collection-grid\" aria-label=\"Collections\">");
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
        body.push_str(if searching { "<section class=\"discovery-empty\"><h2>No collections found</h2><p>Try different keywords, or <a href=\"/collections\">view all collections</a>.</p></section>" } else { "<section class=\"discovery-empty\"><h2>Group projects under a theme</h2><p>Organize your work, or invite creators to tackle the same prompt together.</p><a href=\"/console/#/collections\" data-manage>Create your first collection →</a></section>" });
    }
    if !index {
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
            body.push_str(if searching { "<section class=\"discovery-empty\"><h2>No projects found</h2><p>Try a project name or creator, or <a href=\"/\">clear search</a>.</p></section>" } else { "<section class=\"empty-plaza\"><p>No projects on the plaza yet.</p><p class=\"lead\"><code>playtest ./dist --public</code> will put your project here.</p></section>" });
        } else {
            body.push_str("<section class=\"grid\" aria-label=\"Public playtests\">");
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
    let canonical = if index {
        format!("<link rel=\"canonical\" href=\"https://{}/collections\">\n", playtest_common::DEVELOPER_HOST)
    } else {
        format!("<link rel=\"canonical\" href=\"https://{}/\">\n", playtest_common::DEVELOPER_HOST)
    };
    let mut html = wrap(
        if index {
            "Collections & Challenges"
        } else {
            "playtest · Show the work, not the hype"
        },
        &format!("{canonical}<meta name=\"description\" content=\"Discover browser-based projects and build together around great prompts.\">"),
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
            "Follow updates",
            "utility-btn collection-subscribe",
        )
    } else {
        let form = crate::follow::email_details(
            caps,
            "Follow updates",
            "/follow",
            &target,
            &base,
            "collection",
            "Confirm follow",
        );
        form.replacen(
            "</form>",
            "<p class=\"tell-hint\">Weekly digest of new submissions, unsubscribe anytime</p></form>",
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
        "<a class=\"discovery-button primary action-participate\" href=\"#participate\"><svg class=\"icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M12 5v14M5 12h14\"/></svg>Submit a Project ↗</a>"
    } else {
        ""
    };

    let brief = if has_prompt || has_rules {
        let mut guide_sections = String::new();
        if has_rules {
            guide_sections.push_str(&format!(
                "<div class=\"guide-section\"><h3>Submission Rules</h3><p class=\"preserve-lines\">{}</p></div>",
                esc(&collection.rules)
            ));
        }
        if is_open_challenge {
            guide_sections.push_str(&format!(
                "<div class=\"guide-section\"><h3>How to Participate</h3>\
<p>Build a browser-playable project around the theme. Existing projects can also be submitted without re-uploading.</p>\
<div class=\"command-pill\"><code>playtest ./dist --public</code></div>\
<p class=\"guide-subtext\">Select your public project to submit. No project yet? Publish first, then come back.</p>\
<a class=\"guide-cta\" href=\"/console/#/collections/{}\" data-manage>Submit a Project →</a>\
</div>",
                esc(&collection.slug)
            ));
        }
        let guide = if guide_sections.is_empty() {
            String::new()
        } else {
            format!(
                "<details id=\"participate\" class=\"challenge-guide\">\
<summary><div class=\"guide-summary-label\"><svg class=\"icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><circle cx=\"12\" cy=\"12\" r=\"10\"/><path d=\"M12 16v-4m0-4h.01\"/></svg><span>Rules & Guidelines</span></div><span class=\"guide-toggle-hint\">Details ▾</span></summary>\
<div class=\"guide-body\">{guide_sections}</div></details>"
            )
        };
        let prompt_part = if has_prompt {
            format!(
                "<div class=\"brief-bar\"><div class=\"brief-label\"><svg class=\"icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z\"/><path d=\"M14 2v6h6M16 13H8M16 17H8M10 9H8\"/></svg><h2>Prompt</h2></div>\
<button type=\"button\" class=\"utility-btn copy-prompt-btn\" data-copy-prompt hidden><svg class=\"icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><rect x=\"8\" y=\"8\" width=\"12\" height=\"12\" rx=\"2\"/><path d=\"M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3\"/></svg><span>Copy prompt</span></button></div>\
<div class=\"prompt-canvas\"><pre id=\"challenge-prompt\">{}</pre></div>",
                esc(&collection.prompt)
            )
        } else {
            String::new()
        };
        format!(
            "<section class=\"challenge-brief\" aria-label=\"Challenge prompt\">\
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
<div class=\"collection-badges\"><span class=\"collection-state {state_class}\">{}</span><span class=\"collection-creator\">by {}</span></div>\
<h1>{}</h1>\
<p class=\"collection-summary\">{}</p>\
</div>\
<div class=\"collection-actions\">{participate_btn}\
<div class=\"collection-utilities\"><button type=\"button\" class=\"utility-btn action-share\" data-share-collection hidden><svg class=\"icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><circle cx=\"18\" cy=\"5\" r=\"3\"/><circle cx=\"6\" cy=\"12\" r=\"3\"/><circle cx=\"18\" cy=\"19\" r=\"3\"/><path d=\"m8.59 13.51 6.83 3.98m-.01-10.98-6.82 3.98\"/></svg><span>Share</span></button>{subscribe}</div>\
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
    body.push_str(&search(query, &base));
    body.push_str(&sort_bar(query, &base));
    body.push_str("</div>");
    let mut entries: Vec<_> = collection
        .entries
        .iter()
        .filter_map(|entry| {
            let item = view.plaza.items.iter().find(|item| {
                item.slug == entry.slug && visible(item, &now) && matches(item, &query.search)
            })?;
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
            if query.search.is_empty() {
                "No projects yet"
            } else {
                "No matching projects"
            },
            if query.search.is_empty() {
                "Submitted projects will appear here.".to_string()
            } else {
                format!("<a href=\"{}\">Clear filters</a> to view other projects.", esc(&base))
            }
        ));
    } else {
        body.push_str("<section class=\"grid collection-entries\" aria-label=\"Submitted projects\">");
        for (entry, item) in entries.iter().skip(offset).take(PAGE_SIZE) {
            let tile = plaza::tile(item, false, view.now).replacen(
                &format!("href=\"/p/{}\"", item.slug),
                &format!(
                    "href=\"/p/{}?collection={}&amp;from=collection\"",
                    item.slug, collection.slug
                ),
                1,
            );
            let version_diff = if entry.submitted_version != item.version {
                format!(" · Current v{} (updated since submission)", item.version)
            } else {
                String::new()
            };
            let note_tag = if !entry.note.is_empty() {
                "<span class=\"note-tag\">Note</span>"
            } else {
                ""
            };
            let note_box = if entry.note.is_empty() {
                String::new()
            } else {
                format!("<div class=\"prompt-box\"><span class=\"box-label\">Creator's note</span><pre>{}</pre></div>", esc(&entry.note))
            };
            body.push_str(&format!(
                "<article class=\"collection-entry\">{tile}\
<details class=\"creation-note\">\
<summary class=\"creation-summary\">{note_tag}<span class=\"version-tag\">Submitted v{}{}</span></summary>\
<div class=\"creation-body\">\
<p class=\"creation-time\">Submitted at · {}</p>\
{}\
</div></details></article>",
                entry.submitted_version,
                version_diff,
                esc(&entry.submitted_at),
                note_box
            ));
        }
        body.push_str("</section>");
        body.push_str(&navigation);
    }
    body.push_str("</div>");
    let canonical = format!(
        "<link rel=\"canonical\" href=\"https://{}/c/{}\">\n",
        playtest_common::DEVELOPER_HOST,
        esc(&collection.slug)
    );
    let head = format!(
        "{canonical}<meta property=\"og:type\" content=\"website\"><meta property=\"og:title\" content=\"{}\"><meta property=\"og:description\" content=\"{}\"><meta name=\"description\" content=\"{}\">",
        esc(&collection.title),
        esc(&collection.summary),
        esc(&collection.summary)
    );
    wrap(&collection.title, &head, Here::Collections, &body)
}

pub fn context(plaza: &Plaza, slug: &str, collection_slug: Option<&str>) -> String {
    let now = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default();
    let Some(collection) = plaza.collections.iter().find(|collection| {
        collection.public
            && !collection.hidden
            && collection_slug.is_none_or(|wanted| wanted == collection.slug)
            && collection.entries.iter().any(|entry| {
                entry.slug == slug
                    && plaza
                        .items
                        .iter()
                        .any(|item| item.slug == entry.slug && visible(item, &now))
            })
    }) else {
        return String::new();
    };
    format!("<a class=\"invitation-back collection-context\" href=\"{}\" title=\"Back to {}\"><svg class=\"icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"m15 18-6-6 6-6\"/></svg><span>返回 {}</span></a>", esc(&collection.path()), esc(&collection.title), esc(&collection.title))
}
