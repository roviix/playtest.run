use playtest_common::avatar::{
    derive_traits, fingerprint, svg_for_creator, svg_for_seed, Traits, PALETTES, SPECIES_NAMES,
    VERSION,
};

fn canonical_artwork(svg: &str) -> String {
    let mut artwork = svg[svg.find("<defs>").unwrap()..].to_string();
    for (index, suffix) in svg.split(" id=\"").skip(1).enumerate() {
        let identifier = suffix.split('"').next().unwrap();
        artwork = artwork.replace(identifier, &format!("layer-{index}"));
    }
    artwork
}

#[test]
fn avatar_generation_is_strictly_deterministic() {
    let seed = "playtest-seed-42";
    let svg1 = svg_for_seed(seed, Some(22), Some("face"));
    let svg2 = svg_for_seed(seed, Some(22), Some("face"));
    assert_eq!(svg1, svg2);

    let traits1 = derive_traits(seed);
    let traits2 = derive_traits(seed);
    assert_eq!(traits1, traits2);
    assert_eq!(fingerprint(&traits1), fingerprint(&traits2));
}

#[test]
fn anonymous_creator_faces_follow_the_work_not_the_placeholder_name() {
    assert_eq!(
        canonical_artwork(&svg_for_creator("匿名开发者", "quiet-otter", 24)),
        canonical_artwork(&svg_for_seed("quiet-otter", Some(24), Some("face")))
    );
    assert_eq!(
        svg_for_creator(" \n", "quiet-otter", 24),
        svg_for_creator("匿名开发者", "quiet-otter", 24)
    );
    assert_ne!(
        svg_for_creator("匿名开发者", "quiet-otter", 24),
        svg_for_creator("匿名开发者", "brisk-cat", 24)
    );
    assert_eq!(
        canonical_artwork(&svg_for_creator(" 小林🎨 ", "quiet-otter", 32)),
        canonical_artwork(&svg_for_creator("小林🎨", "brisk-cat", 32))
    );
}

#[test]
fn repeated_creators_have_isolated_local_svg_references() {
    let mut identifiers = std::collections::HashSet::new();
    for slug in ["quiet-otter", "brisk-cat", "a-b", "a_2db", "\"><script>&猫"] {
        for size in [24, 32] {
            let svg = svg_for_creator("native-7-3335", slug, size);
            let local: std::collections::HashSet<_> = svg
                .split(" id=\"")
                .skip(1)
                .map(|suffix| suffix.split('"').next().unwrap())
                .collect();
            assert!(!local.is_empty());
            for identifier in &local {
                assert!(identifier
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'));
                assert!(
                    identifiers.insert(identifier.to_string()),
                    "duplicate {identifier}"
                );
            }
            for suffix in svg.split("url(#").skip(1) {
                let reference = suffix.split(')').next().unwrap();
                assert!(local.contains(reference), "missing {reference}");
            }
            assert!(!svg.contains("<script>"));
        }
    }
}

#[test]
fn byline_avatars_do_not_interrupt_the_accessible_creator_name() {
    let byline = svg_for_creator("小林🎨", "quiet-otter", 24);
    assert!(byline.contains("aria-hidden=\"true\""));
    assert!(byline.contains("focusable=\"false\""));
    assert!(!byline.contains("aria-label="));
    assert!(!byline.contains("<title>"));
    let standalone = svg_for_seed("小林🎨", None, None);
    assert!(standalone.contains("role=\"img\""));
    assert!(standalone.contains("aria-label="));
    assert!(standalone.contains("<title>"));
}

#[test]
fn avatar_svg_contains_valid_vector_elements() {
    let svg = svg_for_seed("developer-neo", Some(16), Some("face"));
    assert!(svg.starts_with("<svg"));
    assert!(svg.ends_with("</svg>"));
    assert!(svg.contains("viewBox=\"0 0 600 600\""));
    assert!(svg.contains("width=\"16\""));
    assert!(svg.contains("height=\"16\""));
    assert!(svg.contains("class=\"face\""));
    if derive_traits("developer-neo").species < 6 {
        assert!(svg.contains("<clipPath"));
    } else {
        assert!(svg.contains("data-creature="));
    }
    assert_eq!(
        svg.contains("<pattern"),
        derive_traits("developer-neo").background == 2
    );
}

#[test]
fn diversity_across_different_seeds() {
    let seeds = [
        "iron-monk-7a",
        "brisk-otter-41",
        "quiet-badger-42",
        "crimson-hawk-99",
        "solar-robot-01",
        "cyber-frog-88",
    ];

    let mut fingerprints = std::collections::HashSet::new();
    for s in seeds {
        let t = derive_traits(s);
        let fp = fingerprint(&t);
        fingerprints.insert(fp);
        assert!(t.species < SPECIES_NAMES.len());
        assert!(t.palette < PALETTES.len());
    }
    // 6 个不同种子应当产生丰富多样的不同角色
    assert_eq!(fingerprints.len(), seeds.len());
}

#[test]
fn utf8_seeds_match_the_preview_golden_vectors() {
    #[derive(serde::Deserialize)]
    struct Vector {
        seed: String,
        traits: Traits,
        fingerprint: String,
    }
    let vectors: Vec<Vector> = serde_json::from_str(include_str!("avatar-vectors.json")).unwrap();
    for vector in vectors {
        let traits = derive_traits(&vector.seed);
        assert_eq!(traits, vector.traits, "seed {:?}", vector.seed);
        assert_eq!(fingerprint(&traits), vector.fingerprint);
    }
}

#[test]
fn svg_attributes_are_escaped_and_seeds_are_never_embedded() {
    let svg = svg_for_seed(
        "<private-seed>&",
        Some(34),
        Some("face\" onload=\"alert(1)<&"),
    );
    assert!(svg.contains("class=\"face&quot; onload=&quot;alert(1)&lt;&amp;\""));
    assert!(!svg.contains(" onload=\""));
    assert!(!svg.contains("private-seed"));
    assert!(svg.contains(&format!("data-avatar-version=\"{VERSION}\"")));
    assert!(svg.contains("aria-label="));
    assert!(!svg.contains("<text"));
}

#[test]
fn seed_normalization_is_identical_for_traits_and_rendering() {
    for (left, right) in [("", "odd-folk-default"), (" \n", ""), (" 猫🎨 \n", "猫🎨")] {
        assert_eq!(derive_traits(left), derive_traits(right));
        assert_eq!(
            svg_for_seed(left, None, None),
            svg_for_seed(right, None, None)
        );
    }
}

#[test]
fn production_and_preview_ship_identical_supplemental_artwork() {
    let html = include_str!("../../docs/avatar-concept.html");
    let embedded = html
        .split("id=\"avatar-assets\">")
        .nth(1)
        .unwrap()
        .split("</script>")
        .next()
        .unwrap();
    let production: serde_json::Value =
        serde_json::from_str(include_str!("../src/avatar-assets.json")).unwrap();
    let preview: serde_json::Value = serde_json::from_str(embedded).unwrap();
    assert_eq!(production, preview);
}

#[test]
fn seeded_avatars_stay_within_the_svg_payload_budget() {
    let start = std::time::Instant::now();
    let mut largest = 0;
    let mut total = 0;
    let mut species = std::collections::HashSet::new();
    for index in 0..4096 {
        let seed = format!("release-audit-{index}");
        let svg = svg_for_creator(&seed, "quiet-otter", 24);
        species.insert(derive_traits(&seed).species);
        largest = largest.max(svg.len());
        total += svg.len();
        assert!(svg.len() <= 16 * 1024, "{seed}: {} bytes", svg.len());
        assert!(!svg.contains('{'));
        assert!(!svg.contains("<image"));
        assert!(!svg.contains("<filter"));
    }
    assert_eq!(species.len(), SPECIES_NAMES.len());
    eprintln!(
        "4096 avatars: max {largest} B, mean {} B; elapsed {:?} (informational)",
        total / 4096,
        start.elapsed()
    );
}
