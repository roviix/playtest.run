use std::path::PathBuf;

use base64::Engine;
use playtest_common::capabilities::Capabilities;
use playtest_common::follow::{FollowTarget, FollowView, MeView};
use playtest_common::live::SiteLive;
use playtest_common::manifest::{GateMode, Manifest};
use playtest_common::plaza::{Plaza, PlazaItem};
use playtest_edge::discovery::{self, Query};
use playtest_edge::follow::{me_page, MePage};
use playtest_edge::gate::GatePage;
use playtest_edge::html::enhance;
use playtest_edge::plaza::View;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

const NONCE: &str = "design-preview";

// 原创几何封面仅用于本机模拟数据，以 PNG data URI 内嵌，不读取或请求外部素材。
fn cover_data_uri() -> std::io::Result<String> {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="800" height="500" viewBox="0 0 800 500">
<defs><linearGradient id="sky" x2="0" y2="1"><stop stop-color="#193b43"/><stop offset="1" stop-color="#63988b"/></linearGradient></defs>
<rect width="800" height="500" fill="url(#sky)"/>
<circle cx="579" cy="144" r="54" fill="#e3d7a7"/>
<path d="M0 300 141 239 303 304 459 249 611 288 800 217V500H0Z" fill="#193c3e"/>
<path d="M0 363 178 326 381 377 569 316 800 366V500H0Z" fill="#102a2d"/>
<path d="m351 500 84-171h37l-55 171" fill="#78978a"/>
<rect x="453" y="242" width="8" height="115" rx="4" fill="#d6d2b1"/>
<path d="M432 243v-17q0-18 18-18h35q18 0 18 18v17Z" fill="#d69971"/>
<path d="M438 229h46" stroke="#5f493d" stroke-width="5" stroke-linecap="round"/>
<circle cx="184" cy="129" r="2" fill="#c6d9cb"/><circle cx="319" cy="81" r="2" fill="#c6d9cb"/>
</svg>"##;
    let tree =
        usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(std::io::Error::other)?;
    let mut pixmap = tiny_skia::Pixmap::new(800, 500)
        .ok_or_else(|| std::io::Error::other("could not allocate the mock cover"))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    let png = pixmap.encode_png().map_err(std::io::Error::other)?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    ))
}

fn gallery(now: OffsetDateTime) -> Plaza {
    let samples = [
        (
            "moon-post",
            "Moon Courier",
            "Shu",
            "Carry the last letter of the night along the moonlight.",
            true,
        ),
        (
            "rainy-store",
            "Rainy Night Store",
            "Yao",
            "A corner shop that only opens when it rains.",
            true,
        ),
        (
            "quiet-map",
            "Drawing every walking route near home into one quiet map",
            "Someone making something slowly",
            "Start at a familiar crossing, note the shade, the evening wind and the small shops around the corner, and see how a long summary ends inside a card.",
            false,
        ),
        ("paper-plane", "Paper Plane", "Ji", "", true),
        (
            "tiny-orbit",
            "Tiny Orbit",
            "Nori",
            "Keep a satellite in orbit with one finger.",
            true,
        ),
        (
            "colour-pocket",
            "Pocket Colors",
            "Bai",
            "Put today's colors in your pocket.",
            false,
        ),
        (
            "cloud-room",
            "Cloud Room",
            "Man",
            "Leave a window open and listen to ten minutes of rain.",
            false,
        ),
        (
            "after-school",
            "After School",
            "Kawa",
            "Find the way home before the sun goes down.",
            true,
        ),
    ];
    let items = samples
        .into_iter()
        .enumerate()
        .map(|(index, (slug, title, developer, summary, is_game))| {
            PlazaItem {
                slug: slug.into(),
                // 模拟作品只指向本机相对路径；不会连接生产作品。
                url: format!("/preview-content/{slug}"),
                title: title.into(),
                developer: developer.into(),
                avatar_url: None,
                summary: (!summary.is_empty()).then(|| summary.into()),
                note: (index == 0 || index == 2).then(|| {
                    "Curious whether the next step is obvious the first time you open it.".into()
                }),
                engine: is_game.then(|| "godot".into()),
                is_game,
                kind: Default::default(),
                cover_hash: (index == 0).then(|| "a".repeat(64)),
                version: index as u32 + 1,
                updated_at: (now - Duration::hours(index as i64 + 1))
                    .format(&Rfc3339)
                    .unwrap(),
                expires_at: (index == 4)
                    .then(|| (now + Duration::hours(5)).format(&Rfc3339).unwrap()),
                players: [28, 12, 6, 0, 4, 9, 0, 3][index],
                seeking: index == 0 || index == 2,
                seats: (index == 0).then_some(10),
                joined: if index == 0 { 6 } else { 0 },
                followers: 0,
                boosted: false,
            }
        })
        .collect();
    Plaza {
        generated_at: now.format(&Rfc3339).unwrap(),
        items,
        ..Plaza::default()
    }
}

fn main() -> std::io::Result<()> {
    let directory = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("playtest-ui-design"));
    std::fs::create_dir_all(&directory)?;
    std::fs::write(
        directory.join("report.html"),
        playtest_edge::pages::report_form(),
    )?;
    let capabilities = Capabilities {
        email: true,
        ..Default::default()
    };
    let now = OffsetDateTime::now_utc();
    let plaza = gallery(now);
    let cover = cover_data_uri()?;
    for (filename, hot) in [("gallery.html", false), ("gallery-hot.html", true)] {
        let html = discovery::home(
            &View { plaza: &plaza, now },
            &Query {
                hot,
                page: 1,
                ..Query::default()
            },
            false,
        );
        let html = html.replace(&plaza.items[0].cover_url().unwrap(), &cover);
        std::fs::write(directory.join(filename), enhance(html, NONCE))?;
    }
    let view = MeView {
        email_masked: Some("p***@example.com".into()),
        push: false,
        follows: vec![
            FollowView {
                target: FollowTarget::Site {
                    slug: "moon-post".into(),
                },
                title: Some("Moon Courier".into()),
                url: None,
                since: "2026-09-12T00:00:00Z".into(),
            },
            FollowView {
                target: FollowTarget::Site {
                    slug: "rainy-store".into(),
                },
                title: Some("Rainy Night Store".into()),
                url: None,
                since: "2026-09-12T00:00:00Z".into(),
            },
            FollowView {
                target: FollowTarget::Plaza,
                title: None,
                url: None,
                since: "2026-09-12T00:00:00Z".into(),
            },
        ],
    };
    for (filename, identity) in [
        ("follow-new.html", None),
        ("follow-subscribed.html", Some(&view)),
    ] {
        let html = me_page(&MePage {
            view: identity,
            caps: &capabilities,
            nonce: NONCE,
        });
        std::fs::write(directory.join(filename), enhance(html, NONCE))?;
    }

    let sample = &plaza.items[0];
    let manifest = Manifest {
        schema: playtest_common::manifest::SCHEMA,
        slug: sample.slug.clone(),
        version: sample.version,
        title: sample.title.clone(),
        developer: sample.developer.clone(),
        summary: sample.summary.clone(),
        note: sample.note.clone(),
        cover: None,
        created_at: sample.updated_at.clone(),
        expires_at: Some((now + Duration::hours(5)).format(&Rfc3339).unwrap()),
        badge: true,
        gate: GateMode::Once,
        isolated: false,
        spa: false,
        engine: sample.engine.clone(),
        kind: Default::default(),
        entry: None,
        article: None,
        chapters: vec![],
        files: vec![],
    };
    let mut live = SiteLive::empty(&sample.slug);
    live.listed = true;
    live.seeking = true;
    live.seats = Some(10);
    live.joined = 6;
    live.community_url = Some("http://localhost:5275/#preview-community".into());
    for (filename, already_followed) in [
        ("invitation-new.html", false),
        ("invitation-subscribed.html", true),
    ] {
        let html = GatePage {
            manifest: &manifest,
            live: &live,
            caps: &capabilities,
            to: "/preview-content/moon-post",
            host_suffix: "localhost",
            root_url: "/gallery.html",
            page_url: filename,
            origin: "/preview-content/moon-post",
            wechat: false,
            version_label: None,
            referer: "",
            from: None,
            me_token: already_followed.then_some("preview-only-no-identity"),
            already_followed,
            is_root: true,
            nonce: Some(NONCE),
            article_html: None,
            current_chapter: None,
            current_chapter_index: None,
            collection_context: None,
        }
        .render();
        std::fs::write(directory.join(filename), enhance(html, NONCE))?;
    }
    println!(
        "Mock-data pages written to {}; no account is used and no email is sent.",
        directory.display()
    );
    Ok(())
}
