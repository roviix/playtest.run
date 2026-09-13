use std::collections::HashSet;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use playtest_common::avatar::{
    derive_traits, svg_for_creator, svg_for_seed, PALETTES, SPECIES_NAMES,
};
use playtest_common::hash::hash_bytes;
use playtest_common::live::SiteLive;
use playtest_common::manifest::{FileEntry, GateMode, Manifest};
use playtest_common::plaza::{Plaza, PlazaItem};
use playtest_common::store::{Current, FsStore};
use playtest_edge::{router, App, Config};
use tower::ServiceExt;

struct Fixture {
    _directory: tempfile::TempDir,
    store: FsStore,
    config: Config,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let config = Config {
            listen: "127.0.0.1:0".parse().unwrap(),
            data_dir: directory.path().to_path_buf(),
            host_suffix: "localhost".into(),
            public_scheme: "http".into(),
            api_internal_url: None,
            edge_ingest_token: None,
        };
        Self {
            store: FsStore::new(directory.path().join("store")),
            _directory: directory,
            config,
        }
    }

    async fn work(&self, slug: &str, developer: &str, avatar_url: Option<&str>) -> PlazaItem {
        let content = b"<!doctype html><title>avatar release fixture</title>";
        let hash = hash_bytes(content);
        self.store.put_blob(&hash, content).await.unwrap();
        let manifest = Manifest {
            schema: playtest_common::manifest::SCHEMA,
            slug: slug.into(),
            version: 1,
            title: "头像验收作品".into(),
            developer: developer.into(),
            note: None,
            summary: None,
            cover: None,
            created_at: "2026-09-12T00:00:00Z".into(),
            expires_at: None,
            badge: true,
            gate: GateMode::Once,
            isolated: false,
            spa: false,
            engine: None,
            kind: Default::default(),
            entry: None,
            article: None,
            files: vec![FileEntry {
                path: "index.html".into(),
                hash,
                size: content.len() as u64,
            }],
        };
        self.store.put_manifest(&manifest).await.unwrap();
        self.store
            .set_current(
                slug,
                &Current {
                    version: 1,
                    updated_at: "2026-09-12T00:00:00Z".into(),
                },
            )
            .await
            .unwrap();
        let mut live = SiteLive::empty(slug);
        live.avatar_url = avatar_url.map(str::to_owned);
        self.store.put_live(&live).await.unwrap();
        PlazaItem {
            slug: slug.into(),
            url: format!("http://{slug}.localhost:8443"),
            title: manifest.title,
            developer: developer.into(),
            summary: None,
            engine: None,
            is_game: true,
            kind: Default::default(),
            version: 1,
            updated_at: manifest.created_at,
            expires_at: None,
            cover_hash: None,
            players: 0,
            seeking: false,
            note: None,
            seats: None,
            joined: 0,
            followers: 0,
            avatar_url: avatar_url.map(str::to_owned),
            boosted: false,
        }
    }

    async fn app(&self, items: Vec<PlazaItem>) -> axum::Router {
        self.store
            .put_plaza(&Plaza {
                schema: playtest_common::plaza::SCHEMA,
                generated_at: "2026-09-12T00:00:00Z".into(),
                club_followers: 0,
                collections: Vec::new(),
                items,
            })
            .await
            .unwrap();
        router(Arc::new(App::new(self.config.clone())))
    }
}

async fn page(app: &axum::Router, path: &str) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .header("host", "localhost:8443")
                .header("accept", "text/html")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK, "{path}");
    String::from_utf8(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap()
}

fn identifiers(svg: &str) -> HashSet<String> {
    let document = usvg::roxmltree::Document::parse(svg).expect("well-formed SVG XML");
    let mut identifiers = HashSet::new();
    for node in document.descendants().filter(|node| node.is_element()) {
        assert!(matches!(
            node.tag_name().name(),
            "svg"
                | "title"
                | "defs"
                | "g"
                | "path"
                | "ellipse"
                | "circle"
                | "rect"
                | "clipPath"
                | "pattern"
                | "linearGradient"
                | "stop"
        ));
        for attribute in node.attributes() {
            assert!(!attribute.name().starts_with("on"));
            assert!(!matches!(attribute.name(), "href" | "style"));
        }
        if let Some(identifier) = node.attribute("id") {
            assert!(
                identifiers.insert(identifier.to_string()),
                "duplicate {identifier}"
            );
        }
    }
    for attribute in document.descendants().flat_map(|node| node.attributes()) {
        let value = attribute.value();
        if let Some(reference) = value.strip_prefix("url(") {
            let local = reference
                .strip_prefix('#')
                .and_then(|local| local.strip_suffix(')'))
                .expect("only local paint and clip references");
            assert!(identifiers.contains(local), "unresolved {local}");
        }
        assert!(!value.contains('{'));
    }
    identifiers
}

fn raster(svg: &str, size: u32) -> Vec<u8> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).expect("renderable SVG");
    assert_eq!(tree.size().width(), size as f32);
    assert_eq!(tree.size().height(), size as f32);
    let mut pixmap = tiny_skia::Pixmap::new(size, size).unwrap();
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    assert!(pixmap.data().chunks_exact(4).all(|pixel| pixel[3] == 255));
    let colors: HashSet<_> = pixmap
        .data()
        .chunks_exact(4)
        .map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect();
    assert!(
        colors.len() > 20,
        "avatar must contain more than an empty backdrop"
    );
    pixmap.take()
}

#[test]
fn svg_parses_and_rasterizes_at_product_sizes_without_changing_the_artwork() {
    let mut combinations = HashSet::new();
    for index in 0..8192 {
        let seed = format!("release-raster-{index}");
        let traits = derive_traits(&seed);
        if !combinations.insert((traits.species, traits.palette)) {
            continue;
        }
        for size in [24, 32, 96] {
            let standalone = svg_for_seed(&seed, Some(size), None);
            let first = svg_for_creator(&seed, "first-work", size);
            let second = svg_for_creator(&seed, "second-work", size);
            let first_ids = identifiers(&first);
            let second_ids = identifiers(&second);
            assert!(first_ids.is_disjoint(&second_ids));
            let expected = raster(&standalone, size);
            assert_eq!(raster(&first, size), expected, "{seed} at {size}px");
            assert_eq!(raster(&second, size), expected, "{seed} at {size}px");
        }
        if combinations.len() == SPECIES_NAMES.len() * PALETTES.len() {
            break;
        }
    }
    assert_eq!(combinations.len(), SPECIES_NAMES.len() * PALETTES.len());
}

#[tokio::test]
async fn creator_avatars_match_real_wall_and_invitation_routes_for_every_species() {
    let fixture = Fixture::new();
    let mut developers = vec!["匿名开发者".to_string(), "小林🎨".into(), String::new()];
    for species in 0..SPECIES_NAMES.len() {
        developers.push(
            (0..1000)
                .map(|index| format!("头像作者-{species}-{index}"))
                .find(|seed| derive_traits(seed).species == species)
                .unwrap(),
        );
    }
    developers.extend(["native-7-3335".into(), "native-7-3335".into()]);
    let mut items = Vec::new();
    for (index, developer) in developers.iter().enumerate() {
        items.push(
            fixture
                .work(&format!("avatar-work-{index}"), developer, None)
                .await,
        );
    }
    let app = fixture.app(items.clone()).await;
    let wall = page(&app, "/").await;
    let mut all_ids = HashSet::new();
    for item in &items {
        for (path, size) in [("/".into(), 24), (format!("/p/{}", item.slug), 32)] {
            let html = if path == "/" {
                wall.clone()
            } else {
                page(&app, &path).await
            };
            let expected = svg_for_creator(&item.developer, &item.slug, size);
            assert!(html.contains(&expected), "{path}: {}", item.developer);
            assert_eq!(html.matches(&expected).count(), 1);
            assert!(expected.contains("aria-hidden=\"true\""));
            assert!(!expected.contains("<title>"));
            let styles = if size == 24 {
                playtest_edge::html::PAGE
            } else {
                playtest_edge::html::CARD
            };
            let rule = styles
                .split(".face{")
                .nth(1)
                .unwrap()
                .split('}')
                .next()
                .unwrap();
            assert!(rule.contains(&format!("width:{size}px;height:{size}px")));
            assert!(rule.contains("border-radius:50%"));
            if size == 24 {
                for identifier in identifiers(&expected) {
                    assert!(
                        all_ids.insert(identifier.clone()),
                        "wall duplicate {identifier}"
                    );
                }
            }
        }
    }
}

#[tokio::test]
async fn github_avatars_remain_primary_and_unsafe_schemes_use_generated_faces() {
    let fixture = Fixture::new();
    let github = "https://avatars.githubusercontent.com/u/1?v=4&size=64";
    let safe = fixture
        .work("github-work", "GitHub 作者", Some(github))
        .await;
    let unsafe_item = fixture
        .work(
            "invalid-avatar",
            "<script>作者</script>",
            Some("javascript:alert(1)"),
        )
        .await;
    let app = fixture.app(vec![safe, unsafe_item]).await;
    for path in ["/", "/p/github-work"] {
        let html = page(&app, path).await;
        let size = if path == "/" { 24 } else { 32 };
        assert!(html.contains("src=\"https://avatars.githubusercontent.com/u/1?v=4&amp;size=64\""));
        assert!(html.contains(&format!("alt=\"\" width=\"{size}\" height=\"{size}\"")));
        assert!(html.contains("referrerpolicy=\"no-referrer\""));
        assert!(!html.contains(&svg_for_creator("GitHub 作者", "github-work", size)));
    }
    for path in ["/", "/p/invalid-avatar"] {
        let html = page(&app, path).await;
        let size = if path == "/" { 24 } else { 32 };
        assert!(html.contains(&svg_for_creator(
            "<script>作者</script>",
            "invalid-avatar",
            size
        )));
        assert!(!html.contains("javascript:alert(1)"));
        assert!(!html.contains("<script>作者</script>"));
    }
}
