//! ODD FOLK 原创确定性算法头像系统（DESIGN §3.3、§3.9）。
//!
//! 根据公开开发者名或 slug 作为种子（seed），通过确定性哈希与加权采样，
//! 稳定生成怪趣角色矢量头像；不保证不同种子绝不撞脸。
//!
//! 纯 Rust 实现，不新增依赖；纯矢量 SVG 直出，不经外部图床。

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetLayer {
    #[serde(rename = "name")]
    label: String,
    svg: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreatureAsset {
    name: String,
    body: String,
    eyes: [String; 3],
    mouths: [String; 4],
    foreground: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetCatalog {
    hat: [AssetLayer; 4],
    outfit: [AssetLayer; 3],
    eyes: [AssetLayer; 3],
    accessory: [AssetLayer; 3],
    background: [AssetLayer; 2],
    finish: String,
    creatures: [CreatureAsset; 4],
}

fn assets() -> &'static AssetCatalog {
    static ASSETS: OnceLock<AssetCatalog> = OnceLock::new();
    ASSETS.get_or_init(|| {
        serde_json::from_str(include_str!("avatar-assets.json"))
            .expect("bundled avatar catalog must pass release validation")
    })
}

fn asset_svg(layer: &AssetLayer, palette: &Palette) -> String {
    debug_assert!(!layer.label.is_empty());
    colorize(&layer.svg, palette)
}

fn colorize(template: &str, palette: &Palette) -> String {
    template
        .replace("{ink}", INK)
        .replace("{skin}", palette.skin)
        .replace("{light}", palette.light)
        .replace("{shade}", palette.shade)
        .replace("{coat}", palette.coat)
        .replace("{trim}", palette.trim)
        .replace("{accent}", palette.accent)
}

fn compatible_traits(mut traits: Traits) -> Traits {
    if traits.species >= HEADS.len() {
        traits.hat = 0;
        traits.outfit = 0;
        traits.accessory = 0;
        traits.eyes %= 3;
    }
    traits
}

fn draw_creature(traits: &Traits, palette: &Palette, id: &str) -> String {
    let creature = &assets().creatures[traits.species - HEADS.len()];
    debug_assert_eq!(creature.name, SPECIES_NAMES[traits.species]);
    let mut drawing = String::from("<g data-creature=\"");
    drawing.push_str(SPECIES_NAMES[traits.species]);
    drawing.push_str("\">");
    for layer in [
        &creature.body,
        &creature.eyes[traits.eyes],
        &creature.mouths[traits.mood],
        &creature.foreground,
    ] {
        drawing.push_str(layer);
    }
    drawing.push_str("</g>");
    colorize(&drawing, palette).replace("{id}", id)
}

fn normalized_seed(seed: &str) -> &str {
    let trimmed = seed.trim();
    if trimmed.is_empty() {
        "odd-folk-default"
    } else {
        trimmed
    }
}

pub const VERSION: &str = "odd-folk/3.0.0";
pub const INK: &str = "#292a29";
pub const OUTLINE: &str = "stroke=\"#292a29\" stroke-width=\"7\"";

pub const AXES_SPECIES_WEIGHTS: [u32; 10] = [12, 11, 9, 9, 8, 7, 12, 11, 10, 11];
pub const AXES_PALETTE_WEIGHTS: [u32; 12] = [12, 10, 10, 10, 9, 9, 9, 7, 7, 6, 6, 5];
pub const AXES_HAT_WEIGHTS: [u32; 10] = [20, 13, 12, 10, 4, 6, 11, 8, 9, 7];
pub const AXES_OUTFIT_WEIGHTS: [u32; 8] = [18, 17, 13, 12, 7, 12, 11, 10];
pub const AXES_EYES_WEIGHTS: [u32; 8] = [23, 20, 14, 13, 7, 10, 8, 5];
pub const AXES_MOOD_WEIGHTS: [u32; 4] = [35, 24, 28, 13];
pub const AXES_ACCESSORY_WEIGHTS: [u32; 8] = [30, 13, 11, 10, 10, 9, 10, 7];
pub const AXES_BACKGROUND_WEIGHTS: [u32; 6] = [34, 22, 12, 8, 14, 10];

pub const SPECIES_NAMES: [&str; 10] = [
    "街头猿",
    "尖耳猫",
    "长耳兔",
    "棕熊",
    "树蛙",
    "铁皮机器人",
    "蘑菇居民",
    "玻璃幽灵",
    "陶土精灵",
    "毛团野兽",
];
pub const PALETTE_NAMES: [&str; 12] = [
    "杏橙海盐",
    "薄荷紫罗兰",
    "樱桃汽水",
    "黄油抹茶",
    "靛蓝珊瑚",
    "薰衣草奶油",
    "桃子乌龙",
    "冰川银灰",
    "午夜黄铜",
    "珊瑚玉石",
    "蓝莓牛奶",
    "沙漠红陶",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Palette {
    pub bg: &'static str,
    pub skin: &'static str,
    pub light: &'static str,
    pub shade: &'static str,
    pub coat: &'static str,
    pub trim: &'static str,
    pub accent: &'static str,
}

pub const PALETTES: [Palette; 12] = [
    Palette {
        bg: "#b9d4e1",
        skin: "#c38b56",
        light: "#edc498",
        shade: "#8a593e",
        coat: "#e87948",
        trim: "#ffdf91",
        accent: "#668f86",
    },
    Palette {
        bg: "#bacbb3",
        skin: "#ae9bc6",
        light: "#d5c6e3",
        shade: "#796b94",
        coat: "#e9ad5f",
        trim: "#f8df9c",
        accent: "#728d75",
    },
    Palette {
        bg: "#e6b2b0",
        skin: "#ddd0a9",
        light: "#f5e4bd",
        shade: "#a59978",
        coat: "#517480",
        trim: "#e2c979",
        accent: "#b95b54",
    },
    Palette {
        bg: "#ddd19a",
        skin: "#849d6d",
        light: "#c1d09d",
        shade: "#536c4c",
        coat: "#697a9a",
        trim: "#eee3b9",
        accent: "#ce785a",
    },
    Palette {
        bg: "#7f9bb6",
        skin: "#dd9b85",
        light: "#f4c5a7",
        shade: "#b06e65",
        coat: "#9da96c",
        trim: "#e6d7a8",
        accent: "#bd665c",
    },
    Palette {
        bg: "#c3b5d3",
        skin: "#e6c876",
        light: "#fae5ad",
        shade: "#b09252",
        coat: "#728d80",
        trim: "#dbdfad",
        accent: "#b887a4",
    },
    Palette {
        bg: "#edc2a1",
        skin: "#8d9fac",
        light: "#c3d1d1",
        shade: "#5e7485",
        coat: "#ad6c61",
        trim: "#e2c7a0",
        accent: "#728c7a",
    },
    Palette {
        bg: "#b6ccca",
        skin: "#bac5c8",
        light: "#e7e8d9",
        shade: "#819197",
        coat: "#666a8c",
        trim: "#d8dbb2",
        accent: "#d18d70",
    },
    Palette {
        bg: "#343e50",
        skin: "#adbaa2",
        light: "#e0e5c4",
        shade: "#637967",
        coat: "#725565",
        trim: "#e9ba6f",
        accent: "#c68263",
    },
    Palette {
        bg: "#df9c86",
        skin: "#719b8b",
        light: "#b7d4b0",
        shade: "#416861",
        coat: "#485d7b",
        trim: "#f1d293",
        accent: "#d2775e",
    },
    Palette {
        bg: "#babfdf",
        skin: "#8575a2",
        light: "#cdbadb",
        shade: "#554d78",
        coat: "#ece0bf",
        trim: "#a25763",
        accent: "#d79d65",
    },
    Palette {
        bg: "#dfc7a1",
        skin: "#bb7356",
        light: "#eeb391",
        shade: "#854d42",
        coat: "#657e78",
        trim: "#edcf8c",
        accent: "#b85e47",
    },
];

pub const HEADS: [&str; 6] = [
    "M210 166C231 132 303 134 335 150C389 125 453 163 457 221L472 329C484 407 432 450 351 454C272 458 203 429 194 361L188 246Q184 201 210 166Z",
    "M192 233L176 100Q179 81 194 95L278 164Q338 143 383 165L457 94Q470 85 470 111L468 249L490 296L475 313L487 338L466 345C456 415 405 446 329 444C253 447 212 414 192 368L172 356L185 333L167 314L187 298Z",
    "M219 193C189 132 171 49 206 37C237 21 269 118 279 169L345 164C350 96 366 34 398 39C437 42 405 153 398 184C446 207 464 255 464 316C469 400 417 448 339 450C257 453 201 411 198 335C193 277 194 228 219 193Z",
    "M207 191C167 190 149 166 159 137C170 103 211 99 232 139C286 118 367 124 402 145C428 110 470 124 474 158C479 191 455 207 443 207C470 247 478 307 468 360C457 422 407 453 329 452C244 450 197 411 190 348C181 287 186 232 207 191Z",
    "M183 245C171 205 182 167 215 155C245 140 281 155 296 192Q332 179 361 191C378 148 422 141 450 159C481 180 483 219 466 249C492 286 490 342 471 383C449 430 397 451 326 450C252 447 197 429 179 382C163 338 166 284 183 245Z",
    "M204 165L436 153Q463 153 467 186L478 379Q479 418 441 427L247 444Q211 447 206 412L185 205Q181 171 204 165Z",
];

/// 角色 8 维 DNA 特征配置
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Traits {
    pub species: usize,
    pub palette: usize,
    pub hat: usize,
    pub outfit: usize,
    pub eyes: usize,
    pub mood: usize,
    pub accessory: usize,
    pub background: usize,
}

/// FNV-1a 变体散列函数（与前端 JS 版完全一致）
pub fn hash32(text: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for b in text.bytes() {
        h = (h ^ (b as u32)).wrapping_mul(16777619);
    }
    h ^= h >> 16;
    h = h.wrapping_mul(0x85ebca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;
    h
}

/// 加权随机采样
pub fn weighted_pick(seed: &str, key: &str, weights: &[u32]) -> usize {
    let total: u32 = weights.iter().sum();
    let json_text = serde_json::to_string(&[VERSION, seed, key])
        .unwrap_or_else(|_| format!("[\"{VERSION}\",\"{seed}\",\"{key}\"]"));
    let mut v = (hash32(&json_text) as f64 / 4294967296.0) * (total as f64);
    for (i, &w) in weights.iter().enumerate() {
        v -= w as f64;
        if v < 0.0 {
            return i;
        }
    }
    weights.len().saturating_sub(1)
}

/// 从任意公开种子确定性推导出角色 8 维特质
pub fn derive_traits(seed: &str) -> Traits {
    let seed = normalized_seed(seed);
    compatible_traits(Traits {
        species: weighted_pick(seed, "species", &AXES_SPECIES_WEIGHTS),
        palette: weighted_pick(seed, "palette", &AXES_PALETTE_WEIGHTS),
        hat: weighted_pick(seed, "hat", &AXES_HAT_WEIGHTS),
        outfit: weighted_pick(seed, "outfit", &AXES_OUTFIT_WEIGHTS),
        eyes: weighted_pick(seed, "eyes", &AXES_EYES_WEIGHTS),
        mood: weighted_pick(seed, "mood", &AXES_MOOD_WEIGHTS),
        accessory: weighted_pick(seed, "accessory", &AXES_ACCESSORY_WEIGHTS),
        background: weighted_pick(seed, "background", &AXES_BACKGROUND_WEIGHTS),
    })
}

/// 生成 8 位大写十六进制 Visual DNA 签名
pub fn fingerprint(traits: &Traits) -> String {
    let json_text = serde_json::to_string(&compatible_traits(*traits)).unwrap_or_default();
    let h = hash32(&format!("{VERSION}|{json_text}"));
    format!("{h:08X}")
}

#[inline]
fn path(d: &str, fill: &str, extra: &str) -> String {
    if extra.is_empty() {
        format!("<path d=\"{d}\" fill=\"{fill}\"/>")
    } else {
        format!("<path d=\"{d}\" fill=\"{fill}\" {extra}/>")
    }
}

#[inline]
fn line(d: &str, color: &str, width: &str, extra: &str) -> String {
    if extra.is_empty() {
        format!("<path d=\"{d}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"{width}\"/>")
    } else {
        format!(
            "<path d=\"{d}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"{width}\" {extra}/>"
        )
    }
}

#[inline]
fn ellipse(x: i32, y: i32, rx: i32, ry: i32, fill: &str, extra: &str) -> String {
    if extra.is_empty() {
        format!("<ellipse cx=\"{x}\" cy=\"{y}\" rx=\"{rx}\" ry=\"{ry}\" fill=\"{fill}\"/>")
    } else {
        format!("<ellipse cx=\"{x}\" cy=\"{y}\" rx=\"{rx}\" ry=\"{ry}\" fill=\"{fill}\" {extra}/>")
    }
}

#[inline]
fn circle(x: i32, y: i32, r: i32, fill: &str, extra: &str) -> String {
    if extra.is_empty() {
        format!("<circle cx=\"{x}\" cy=\"{y}\" r=\"{r}\" fill=\"{fill}\"/>")
    } else {
        format!("<circle cx=\"{x}\" cy=\"{y}\" r=\"{r}\" fill=\"{fill}\" {extra}/>")
    }
}

#[inline]
fn rect(x: i32, y: i32, w: i32, h: i32, r: i32, fill: &str, extra: &str) -> String {
    if extra.is_empty() {
        format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"{r}\" fill=\"{fill}\"/>"
        )
    } else {
        format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"{r}\" fill=\"{fill}\" {extra}/>"
        )
    }
}

fn draw_background(t: &Traits, p: &Palette, id: &str) -> String {
    let mut s = rect(0, 0, 600, 600, 0, p.bg, "");
    if t.background >= 4 {
        s.push_str(&asset_svg(&assets().background[t.background - 4], p));
        return s;
    }
    if t.background == 1 {
        s.push_str(&circle(313, 260, 227, p.light, "opacity=\".53\""));
        s.push_str(&rect(0, 412, 600, 188, 0, p.shade, "opacity=\".08\""));
    } else if t.background == 2 {
        s.push_str(&rect(0, 0, 600, 600, 0, &format!("url(#{id}-checks)"), ""));
    } else if t.background == 3 {
        for i in 0..14 {
            let a = (i as f64) * std::f64::consts::PI / 7.0;
            let b = a + 0.105;
            let x = 300.0 + 850.0 * a.cos();
            let y = 310.0 + 850.0 * a.sin();
            let xx = 300.0 + 850.0 * b.cos();
            let yy = 310.0 + 850.0 * b.sin();
            s.push_str(&path(
                &format!("M300 310L{x:.2} {y:.2}L{xx:.2} {yy:.2}Z"),
                p.light,
                "opacity=\".23\"",
            ));
        }
    }

    s
}

fn draw_outfit(t: &Traits, p: &Palette) -> String {
    if t.outfit >= 5 {
        return asset_svg(&assets().outfit[t.outfit - 5], p);
    }
    let mut s = path(
        "M270 399L269 453L223 477L361 542L431 468L399 433L393 395Z",
        p.skin,
        OUTLINE,
    );
    s.push_str(&path(
        "M279 419Q351 451 395 411L398 444Q353 473 276 446Z",
        p.shade,
        "opacity=\".4\"",
    ));

    let body = [
        "M240 446L165 468Q114 479 96 553L87 623H551L542 552Q531 485 464 468L403 444L333 492Z",
        "M226 438Q202 445 184 478Q126 489 111 550L97 623H554L537 538Q516 483 464 480Q452 444 405 433L336 475Z",
        "M253 452L174 478Q131 494 114 548L100 623H553L538 545Q522 488 460 472L402 451L333 485Z",
        "M257 447L165 480Q128 496 117 547L99 623H555L538 543Q520 493 461 472L397 444L331 483Z",
        "M255 439L200 463L179 482Q116 491 105 554L91 623H557L547 550Q536 490 470 480L448 458L398 437L332 461Z",
    ];
    s.push_str(&path(body[t.outfit], p.coat, OUTLINE));
    s.push_str(&path(
        "M481 491Q524 517 527 617H473L466 522Z",
        p.shade,
        "opacity=\".22\"",
    ));

    if t.outfit == 0 {
        s.push_str(&path(
            "M169 477Q146 517 143 623H99L109 550Q121 501 169 477Z",
            p.trim,
            OUTLINE,
        ));
        s.push_str(&path(
            "M465 473Q505 483 526 522L548 623H495L480 535Z",
            p.trim,
            OUTLINE,
        ));
        s.push_str(&path(
            "M245 445L332 485L409 444L420 467L335 518L235 470Z",
            p.trim,
            OUTLINE,
        ));
        s.push_str(&line("M251 462L334 503L408 461", p.shade, "3", ""));
        s.push_str(&line("M335 514L335 622", INK, "4", ""));
        for y in (541..620).step_by(28) {
            s.push_str(&circle(
                346,
                y,
                3,
                p.trim,
                &format!("stroke=\"{INK}\" stroke-width=\"2\""),
            ));
        }
        s.push_str(&path(
            "M208 517L246 523L240 552L202 546Z",
            p.trim,
            &format!("stroke=\"{INK}\" stroke-width=\"3\""),
        ));
        s.push_str(&line("M213 530l8 10 11-14", p.coat, "4", ""));
    } else if t.outfit == 1 {
        s.push_str(&path(
            "M228 444Q223 476 245 498L332 516L413 497Q435 472 407 444L336 476Z",
            p.trim,
            OUTLINE,
        ));
        s.push_str(&path(
            "M245 458L333 490L407 455L385 481L333 504L259 481Z",
            p.shade,
            &format!("stroke=\"{INK}\" stroke-width=\"3\""),
        ));
        s.push_str(&line("M277 499L276 552M385 499L392 548", p.light, "5", ""));
        s.push_str(&rect(
            271,
            548,
            9,
            16,
            3,
            p.trim,
            &format!("stroke=\"{INK}\" stroke-width=\"2\""),
        ));
        s.push_str(&rect(
            388,
            544,
            9,
            16,
            3,
            p.trim,
            &format!("stroke=\"{INK}\" stroke-width=\"2\""),
        ));
        s.push_str(&path(
            "M261 566L414 566L432 608L243 608Z",
            p.coat,
            &format!("stroke=\"{INK}\" stroke-width=\"4\""),
        ));
        s.push_str(&line("M267 578l-13 22M406 578l12 22", p.shade, "3", ""));
    } else if t.outfit == 2 {
        s.push_str(&path(
            "M184 479L247 530L332 554L418 531L464 476L401 453L332 479L257 452Z",
            p.trim,
            OUTLINE,
        ));
        s.push_str(&line(
            "M207 481L256 517L331 540L407 518L442 481",
            p.coat,
            "5",
            "",
        ));
        s.push_str(&path(
            "M329 538L349 538L367 585L341 572L320 589Z",
            p.accent,
            &format!("stroke=\"{INK}\" stroke-width=\"4\""),
        ));
        s.push_str(&line(
            "M143 561H241M443 561H528M129 584H270M407 584H536M126 607H313M382 607H540",
            p.trim,
            "9",
            "",
        ));
    } else if t.outfit == 3 {
        s.push_str(&path(
            "M267 452L334 482L395 449L408 548L333 616L259 547Z",
            p.light,
            &format!("stroke=\"{INK}\" stroke-width=\"4\""),
        ));
        s.push_str(&path(
            "M324 484L344 484L352 502L344 513L359 578L335 601L317 577L328 513L317 500Z",
            p.accent,
            &format!("stroke=\"{INK}\" stroke-width=\"4\""),
        ));
        s.push_str(&path(
            "M251 446L333 611L249 552L267 536L228 515Z",
            p.coat,
            OUTLINE,
        ));
        s.push_str(&path(
            "M403 447L335 611L429 548L409 532L444 512Z",
            p.coat,
            OUTLINE,
        ));
        s.push_str(&line("M436 556l45-7", p.trim, "5", ""));
        s.push_str(&circle(
            340,
            617,
            5,
            p.trim,
            &format!("stroke=\"{INK}\" stroke-width=\"2\""),
        ));
    } else {
        s.push_str(&path(
            "M247 440Q330 474 404 438L427 477Q335 526 229 479Z",
            p.trim,
            OUTLINE,
        ));
        s.push_str(&line("M252 462Q332 499 403 459", p.shade, "8", ""));
        s.push_str(&rect(
            253,
            532,
            162,
            79,
            13,
            p.trim,
            &format!("stroke=\"{INK}\" stroke-width=\"5\""),
        ));
        s.push_str(&rect(
            267,
            544,
            77,
            30,
            5,
            p.shade,
            &format!("stroke=\"{INK}\" stroke-width=\"3\""),
        ));
        s.push_str(&line("M276 560h8l6-8 9 15 8-8h22", p.light, "2.5", ""));
        s.push_str(&circle(
            368,
            554,
            7,
            p.accent,
            &format!("stroke=\"{INK}\" stroke-width=\"3\""),
        ));
        s.push_str(&circle(
            393,
            554,
            7,
            p.coat,
            &format!("stroke=\"{INK}\" stroke-width=\"3\""),
        ));
        s.push_str(&line("M270 590h58M359 590h31", p.shade, "5", ""));
        s.push_str(&format!(
            "<path d=\"M153 528L171 589M495 522L511 584\" fill=\"none\" stroke=\"{}\" stroke-width=\"20\"/>",
            p.trim
        ));
    }

    s.push_str(&line(
        "M188 572L177 621M478 570L489 621",
        INK,
        "4",
        "opacity=\".65\"",
    ));
    s
}

fn draw_head(t: &Traits, p: &Palette, id: &str) -> String {
    let mut s = String::new();
    if t.species == 0 {
        s.push_str(&ellipse(192, 301, 39, 53, p.skin, OUTLINE));
        s.push_str(&ellipse(462, 296, 31, 45, p.skin, OUTLINE));
        s.push_str(&line(
            "M184 324C156 282 194 271 205 291M465 274Q484 288 467 314",
            p.shade,
            "6",
            "",
        ));
    } else if t.species == 5 {
        s.push_str(&rect(167, 249, 30, 77, 10, p.shade, OUTLINE));
        s.push_str(&rect(466, 237, 30, 70, 9, p.shade, OUTLINE));
        s.push_str(&line("M320 155L315 110", INK, "7", ""));
        s.push_str(&circle(314, 101, 12, p.accent, OUTLINE));
    }

    s.push_str(&path(HEADS[t.species], p.skin, OUTLINE));
    s.push_str(&format!("<g clip-path=\"url(#{id}-head)\">"));
    s.push_str(&path(
        "M407 117Q450 309 418 391Q393 447 313 462L526 478L522 98Z",
        p.shade,
        "opacity=\".3\"",
    ));

    if t.species == 0 {
        s.push_str(&path(
            "M215 263C210 207 258 189 293 219Q325 238 344 215C398 168 447 214 443 267L428 344L243 363Z",
            p.light,
            "",
        ));
        s.push_str(&path(
            "M267 314C284 290 339 294 359 308C398 299 441 320 447 355C455 395 416 424 349 427C281 430 245 407 246 366Q245 336 267 314Z",
            p.light,
            &format!("stroke=\"{INK}\" stroke-width=\"5\""),
        ));
        s.push_str(&line(
            "M219 320l13-5M218 339l10-5M428 393l-9 9",
            p.shade,
            "3",
            "",
        ));
    } else if t.species == 1 {
        s.push_str(&path(
            "M189 118L252 172L204 211Z",
            p.accent,
            &format!("stroke=\"{INK}\" stroke-width=\"4\""),
        ));
        s.push_str(&path(
            "M447 120L445 213L405 174Z",
            p.accent,
            &format!("stroke=\"{INK}\" stroke-width=\"4\""),
        ));
        s.push_str(&path(
            "M285 332Q309 314 339 337Q367 310 394 332C423 371 392 406 338 403C276 407 258 365 285 332Z",
            p.light,
            "",
        ));
        s.push_str(&line(
            "M295 170L303 204M321 165L325 191M349 166L349 201",
            p.shade,
            "8",
            "",
        ));
        s.push_str(&line(
            "M202 318L237 330M202 340L235 341M423 323L455 309M426 341L460 335",
            p.shade,
            "4",
            "",
        ));
    } else if t.species == 2 {
        s.push_str(&path(
            "M213 66Q238 114 243 172L230 189Q206 143 202 97Q201 68 213 66Z",
            p.accent,
            &format!("stroke=\"{INK}\" stroke-width=\"3\""),
        ));
        s.push_str(&path(
            "M386 66Q393 58 393 83L375 174L361 172Q370 82 386 66Z",
            p.accent,
            &format!("stroke=\"{INK}\" stroke-width=\"3\""),
        ));
        s.push_str(&ellipse(334, 360, 77, 58, p.light, ""));
        s.push_str(&line(
            "M219 345L243 350M219 361L243 360M420 346L446 337M420 359L445 357",
            p.shade,
            "3",
            "",
        ));
    } else if t.species == 3 {
        s.push_str(&ellipse(193, 157, 18, 21, p.shade, ""));
        s.push_str(&ellipse(441, 163, 17, 20, p.shade, ""));
        s.push_str(&ellipse(341, 352, 86, 65, p.light, ""));
        s.push_str(&line(
            "M211 334l13 5M210 351l12 2M438 330l13-5",
            p.shade,
            "3",
            "",
        ));
    } else if t.species == 4 {
        s.push_str(&path(
            "M188 327Q311 301 472 322L478 421L342 468L196 417Z",
            p.light,
            "",
        ));
        s.push_str(&ellipse(225, 209, 21, 31, p.light, "opacity=\".4\""));
        s.push_str(&ellipse(422, 202, 22, 29, p.light, "opacity=\".4\""));
        for &(x, y, r) in &[
            (208, 303, 8),
            (221, 324, 5),
            (451, 292, 7),
            (447, 317, 4),
            (297, 212, 5),
            (333, 215, 4),
        ] {
            s.push_str(&circle(x, y, r, p.shade, "opacity=\".55\""));
        }
    } else {
        s.push_str(&rect(
            214,
            219,
            229,
            101,
            25,
            p.shade,
            &format!("stroke=\"{INK}\" stroke-width=\"5\" transform=\"rotate(-3 329 269)\""),
        ));
        s.push_str(&path(
            "M211 183L424 172L431 189L216 200Z",
            p.light,
            "opacity=\".75\"",
        ));
        s.push_str(&rect(
            255,
            338,
            170,
            65,
            15,
            p.light,
            &format!("stroke=\"{INK}\" stroke-width=\"4\""),
        ));
        s.push_str(&line("M220 356v33M232 355v32M445 340v33", p.shade, "4", ""));
        for &(x, y) in &[(208, 208), (445, 191), (232, 413), (451, 398)] {
            s.push_str(&circle(
                x,
                y,
                5,
                p.light,
                &format!("stroke=\"{INK}\" stroke-width=\"2\""),
            ));
            s.push_str(&line(&format!("M{} {y}h4", x - 2), INK, "1.5", ""));
        }
    }
    if t.species != 5 {
        s.push_str(&colorize(&assets().finish, p));
    }
    s.push_str("</g>");

    if t.species != 5 {
        let (nose_path, nose_fill, nose_width) = if t.species == 4 {
            ("M313 319q5-7 10 0M346 317q5-7 10 0", "none", "4")
        } else if t.species == 0 {
            ("M309 328Q328 308 348 324L340 342L320 345Z", p.shade, "3")
        } else {
            (
                "M317 322Q340 313 359 324Q349 344 336 344Q324 342 317 322Z",
                p.shade,
                "3",
            )
        };
        s.push_str(&path(
            nose_path,
            nose_fill,
            &format!("stroke=\"{INK}\" stroke-width=\"{nose_width}\""),
        ));
        if t.species != 4 {
            s.push_str(&line("M337 344v12", INK, "3", ""));
        }
    }
    s
}

fn draw_eyes(t: &Traits, p: &Palette) -> String {
    if t.eyes >= 5 {
        return asset_svg(&assets().eyes[t.eyes - 5], p);
    }
    let (left, right, yl, yr) = (278, 390, 271, 264);
    let mut s = String::new();

    let normal = |x: i32, y: i32, is_robot: bool| -> String {
        let fill = if is_robot { "#dbefb5" } else { "#f9f0d7" };
        let mut q = ellipse(
            x,
            y,
            29,
            26,
            fill,
            &format!("stroke=\"{INK}\" stroke-width=\"5\""),
        );
        q.push_str(&ellipse(x + 7, y + 3, 9, 13, INK, ""));
        q.push_str(&circle(x + 10, y - 2, 2, "#fff8e7", ""));
        q
    };

    if t.eyes == 0 {
        for &(x, y) in &[(left, yl), (right, yr)] {
            s.push_str(&ellipse(
                x,
                y + 5,
                29,
                19,
                "#f7edd2",
                &format!("stroke=\"{INK}\" stroke-width=\"5\""),
            ));
            s.push_str(&ellipse(x + 7, y + 9, 7, 9, INK, ""));
            s.push_str(&path(
                &format!(
                    "M{} {}Q{} {} {} {}Q{} {} {} {}Z",
                    x - 31,
                    y + 1,
                    x - 26,
                    y - 26,
                    x + 4,
                    y - 23,
                    x + 30,
                    y - 23,
                    x + 31,
                    y + 1
                ),
                p.skin,
                &format!("stroke=\"{INK}\" stroke-width=\"5\""),
            ));
        }
        s.push_str(&line(
            "M249 235q24-10 48-4M365 229q25-11 48-4",
            p.shade,
            "4",
            "",
        ));
    } else if t.eyes == 1 {
        let is_robot = t.species == 5;
        s.push_str(&normal(left, yl, is_robot));
        s.push_str(&normal(right, yr, is_robot));
        s.push_str(&line(
            "M252 231q21-12 45-5M367 223q22-11 43-3",
            INK,
            "5",
            "",
        ));
    } else if t.eyes == 2 {
        let is_robot = t.species == 5;
        s.push_str(&normal(left, yl, is_robot));
        s.push_str(&line("M364 264Q391 245 415 260M367 270l12-4", INK, "6", ""));
        s.push_str(&line("M252 228q23-12 46-3M370 230q21-8 42 1", INK, "4", ""));
    } else if t.eyes == 3 {
        s.push_str(&line("M209 251L247 256M419 245L456 234", INK, "9", ""));
        s.push_str(&path(
            "M244 248L308 246L303 285Q275 306 250 288Z",
            "#303f3a",
            &format!("stroke=\"{INK}\" stroke-width=\"7\""),
        ));
        s.push_str(&path(
            "M354 242L424 238L417 281Q387 299 360 282Z",
            "#303f3a",
            &format!("stroke=\"{INK}\" stroke-width=\"7\""),
        ));
        s.push_str(&line("M306 258Q330 246 355 254", INK, "7", ""));
        s.push_str(&line(
            "M259 253L277 289M273 251L290 285M369 247L390 285M384 246L403 281",
            p.light,
            "6",
            "opacity=\".45\"",
        ));
    } else {
        s.push_str(&path(
            "M233 244Q334 219 446 231L439 284Q333 313 240 294Z",
            p.shade,
            &format!("stroke=\"{INK}\" stroke-width=\"7\""),
        ));
        s.push_str(&path(
            "M244 252Q333 232 435 241L431 274Q333 295 248 284Z",
            p.accent,
            &format!("stroke=\"{INK}\" stroke-width=\"3\""),
        ));
        s.push_str(&path(
            "M266 247L289 244L312 287L289 289ZM365 237L379 237L403 279L387 281Z",
            p.light,
            "opacity=\".62\"",
        ));
        s.push_str(&line("M411 250h13M411 259h10M410 268h7", p.light, "2", ""));
    }
    s
}

fn draw_mouth(t: &Traits, p: &Palette) -> String {
    let mut s = String::new();
    if t.mood == 0 {
        let m = if t.species == 4 {
            "M253 370Q343 405 426 358"
        } else {
            "M293 375Q342 386 398 370"
        };
        s.push_str(&line(m, INK, "5", ""));
        s.push_str(&line("M301 396q39 11 67 0", p.shade, "3", ""));
    } else if t.mood == 1 {
        s.push_str(&path(
            "M282 359Q341 377 404 353L397 391Q343 421 290 395Z",
            INK,
            &format!("stroke=\"{INK}\" stroke-width=\"4\""),
        ));
        s.push_str(&path(
            "M287 363Q341 382 399 357L396 374Q342 396 291 381Z",
            "#fff1ce",
            &format!("stroke=\"{INK}\" stroke-width=\"3\""),
        ));
        s.push_str(&line(
            "M312 372v17M337 376v16M363 372v17M384 366v16",
            INK,
            "2",
            "",
        ));
        s.push_str(&path("M318 404Q340 387 368 405Z", p.accent, ""));
    } else if t.mood == 2 {
        s.push_str(&line(
            "M298 380Q343 391 398 364M391 356q15 4 14 17",
            INK,
            "5",
            "",
        ));
        s.push_str(&line("M367 391l20-7", p.shade, "3", ""));
        if t.species == 2 {
            s.push_str(&path(
                "M338 385L338 401L351 401L352 383Z",
                "#fff3d6",
                &format!("stroke=\"{INK}\" stroke-width=\"2\""),
            ));
        }
    } else {
        s.push_str(&ellipse(355, 373, 12, 9, INK, ""));
        s.push_str(&circle(
            403,
            374,
            47,
            "#e4a2b7",
            &format!("stroke=\"{INK}\" stroke-width=\"5\""),
        ));
        s.push_str(&path(
            "M374 352Q384 334 406 339",
            "none",
            "stroke=\"#fbd7de\" stroke-width=\"6\"",
        ));
        s.push_str(&circle(419, 394, 6, "#f7c7d5", "opacity=\".6\""));
    }
    s
}

fn draw_hat(t: &Traits, p: &Palette) -> String {
    if t.hat >= 6 {
        return asset_svg(&assets().hat[t.hat - 6], p);
    }
    if t.hat == 0 {
        if t.species == 0 || t.species == 3 {
            return path(
                "M275 150L291 130L298 146L322 129L319 149L342 139",
                p.skin,
                &format!("stroke=\"{INK}\" stroke-width=\"5\""),
            );
        }
        return String::new();
    }

    let mut s = String::new();
    if t.hat == 1 {
        s.push_str(&path(
            "M200 189Q179 117 251 99Q327 77 397 106Q444 125 450 184Z",
            p.coat,
            OUTLINE,
        ));
        s.push_str(&path(
            "M204 152Q320 112 442 150L456 204Q325 172 200 211Z",
            p.trim,
            OUTLINE,
        ));
        s.push_str(&line(
            "M229 155l-3 43M251 148l-2 44M274 143v43M299 140l2 41M325 138l2 43M351 140l3 42M377 143l4 42M403 150l4 40M429 157l5 36",
            p.shade,
            "3",
            "",
        ));
        s.push_str(&rect(
            306,
            145,
            40,
            28,
            4,
            p.coat,
            &format!("stroke=\"{INK}\" stroke-width=\"3\" transform=\"rotate(-2 326 159)\""),
        ));
        s.push_str(&line("M316 155l9 8 11-12", p.trim, "3", ""));
    } else if t.hat == 2 {
        s.push_str(&path(
            "M206 184Q210 92 306 93Q422 85 448 182Z",
            p.coat,
            OUTLINE,
        ));
        s.push_str(&path(
            "M211 173Q319 145 445 176L449 207Q330 176 203 211Z",
            p.trim,
            OUTLINE,
        ));
        s.push_str(&path(
            "M207 175L130 184Q113 184 125 198L209 219Z",
            p.coat,
            OUTLINE,
        ));
        s.push_str(&line("M313 101Q340 126 341 162", p.shade, "4", ""));
        s.push_str(&rect(
            296,
            177,
            56,
            20,
            5,
            p.shade,
            &format!("stroke=\"{INK}\" stroke-width=\"3\""),
        ));
        s.push_str(&line("M306 186h36", p.light, "3", ""));
        s.push_str(&circle(
            307,
            97,
            7,
            p.trim,
            &format!("stroke=\"{INK}\" stroke-width=\"3\""),
        ));
    } else if t.hat == 3 {
        s.push_str(&path(
            "M232 102Q322 84 406 112L441 184L203 193Z",
            p.coat,
            OUTLINE,
        ));
        s.push_str(&path(
            "M225 141Q324 129 422 147L432 170Q326 150 215 169Z",
            p.trim,
            &format!("stroke=\"{INK}\" stroke-width=\"3\""),
        ));
        s.push_str(&path(
            "M204 176Q326 149 440 178L480 224Q337 194 168 230Z",
            p.coat,
            OUTLINE,
        ));
        s.push_str(&line("M189 214Q332 181 459 208", p.trim, "3", ""));
        s.push_str(&circle(
            364,
            123,
            4,
            p.trim,
            &format!("stroke=\"{INK}\" stroke-width=\"2\""),
        ));
    } else if t.hat == 4 {
        s.push_str("<g transform=\"rotate(10 331 152)\">");
        s.push_str(&path(
            "M243 160L226 76L285 114L324 63L361 111L418 70L400 165Z",
            "#dfb958",
            OUTLINE,
        ));
        s.push_str(&path(
            "M241 148Q322 164 402 150L399 174Q319 186 245 172Z",
            "#edce7c",
            OUTLINE,
        ));
        s.push_str(&path(
            "M324 105L334 124L324 141L312 125Z",
            p.accent,
            &format!("stroke=\"{INK}\" stroke-width=\"3\""),
        ));
        for &(x, y) in &[(226, 76), (324, 63), (418, 70)] {
            s.push_str(&circle(
                x,
                y,
                7,
                p.trim,
                &format!("stroke=\"{INK}\" stroke-width=\"3\""),
            ));
        }
        s.push_str("</g>");
    } else {
        s.push_str(&line("M324 162Q327 114 345 87", INK, "8", ""));
        s.push_str(&path(
            "M333 128Q277 136 274 86Q326 76 333 128Z",
            "#8eab66",
            &format!("stroke=\"{INK}\" stroke-width=\"5\""),
        ));
        s.push_str(&path(
            "M337 112Q345 63 386 66Q396 109 337 112Z",
            "#bed48d",
            &format!("stroke=\"{INK}\" stroke-width=\"5\""),
        ));
        s.push_str(&line("M287 98l37 25M347 101l25-22", "#66834f", "3", ""));
    }
    s
}

fn draw_accessory(t: &Traits, p: &Palette) -> String {
    if t.accessory >= 5 {
        return asset_svg(&assets().accessory[t.accessory - 5], p);
    }
    if t.accessory == 1 {
        let x = if t.species == 0 {
            175
        } else if t.species == 5 {
            188
        } else {
            207
        };
        let mut s = ellipse(
            x,
            348,
            13,
            19,
            "none",
            &format!("stroke=\"{INK}\" stroke-width=\"9\""),
        );
        s.push_str(&ellipse(
            x,
            348,
            13,
            19,
            "none",
            "stroke=\"#e5bd63\" stroke-width=\"5\"",
        ));
        s.push_str(&circle(x - 5, 337, 2, "#fff2ba", ""));
        return s;
    }

    if t.accessory == 2 {
        return format!(
            "<g transform=\"rotate(-24 427 321)\">{}{}{}{}</g>",
            rect(
                400,
                307,
                55,
                23,
                7,
                p.trim,
                &format!("stroke=\"{INK}\" stroke-width=\"3\"")
            ),
            rect(
                419,
                309,
                17,
                19,
                2,
                p.light,
                "stroke=\"#80735b\" stroke-width=\"1\""
            ),
            circle(408, 318, 1, p.shade, ""),
            circle(448, 317, 1, p.shade, "")
        );
    }

    if t.accessory == 3 {
        let mut s = line("M250 455Q327 545 419 453", INK, "13", "");
        s.push_str(&line("M250 455Q327 545 419 453", "#ddb969", "8", ""));
        for i in 0..9 {
            let x = 261 + i * 17;
            let y = 470.0 + 29.0 * ((i as f64) * std::f64::consts::PI / 8.0).sin();
            let rot = 30 - i * 7;
            s.push_str(&ellipse(
                x,
                y.round() as i32,
                9,
                6,
                "none",
                &format!(
                    "stroke=\"{INK}\" stroke-width=\"2\" transform=\"rotate({rot} {x} {y:.0})\""
                ),
            ));
        }
        s.push_str(&path(
            "M327 504L339 524L329 543L316 525Z",
            p.trim,
            &format!("stroke=\"{INK}\" stroke-width=\"4\""),
        ));
        return s;
    }

    if t.accessory == 4 {
        let mut s = path(
            "M257 437Q336 475 413 432L423 458Q335 500 245 466Z",
            p.accent,
            OUTLINE,
        );
        s.push_str(&path(
            "M391 461L438 515L408 511L399 534L374 470Z",
            p.accent,
            &format!("stroke=\"{INK}\" stroke-width=\"5\""),
        ));
        s.push_str(&line("M261 455Q334 490 405 450", p.light, "3", ""));
        s.push_str(&line("M391 482l22 24", p.light, "3", ""));
        return s;
    }

    String::new()
}

pub fn svg_for_creator(developer: &str, slug: &str, size: u32) -> String {
    let developer = developer.trim();
    let seed = if developer.is_empty() || developer == "匿名开发者" {
        slug
    } else {
        developer
    };
    let traits = derive_traits(seed);
    let id = format!(
        "of-creator-{size}-{}-{}",
        hex::encode(slug.as_bytes()),
        fingerprint(&traits)
    );
    render_svg(&traits, Some(size), Some("face"), &id, true)
}

/// 根据给定的公开种子生成独立 SVG；产品署名使用 `svg_for_creator` 隔离局部引用。
///
/// * `seed`：开发者昵称或作品 slug，不传令牌等秘密。
/// * `size`：可选像素尺寸（产品中使用 24 或 32）。
/// * `class_name`：可选附加在 `<svg>` 标签上的 class 类名（如 `"face"`）。
pub fn svg_for_seed(seed: &str, size: Option<u32>, class_name: Option<&str>) -> String {
    let traits = derive_traits(seed);
    let id = format!("of-{}", fingerprint(&traits));
    render_svg(&traits, size, class_name, &id, false)
}

fn render_svg(
    traits: &Traits,
    size: Option<u32>,
    class_name: Option<&str>,
    id: &str,
    decorative: bool,
) -> String {
    let palette = PALETTES[traits.palette];
    let mut attrs = String::from(
        "xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 600 600\" focusable=\"false\"",
    );
    if let Some(size) = size {
        attrs.push_str(&format!(" width=\"{size}\" height=\"{size}\""));
    }
    if let Some(class_name) = class_name {
        let escaped = class_name
            .replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('\'', "&apos;");
        attrs.push_str(&format!(" class=\"{escaped}\""));
    }

    let description = if decorative {
        attrs.push_str(" aria-hidden=\"true\"");
        String::new()
    } else {
        let title = format!(
            "{} · {}",
            SPECIES_NAMES[traits.species], PALETTE_NAMES[traits.palette]
        );
        attrs.push_str(&format!(" role=\"img\" aria-label=\"{title}\""));
        format!("<title>{title}</title>")
    };
    let bg_svg = draw_background(traits, &palette, id);
    let background_defs = if traits.background == 2 {
        format!(
            "<pattern id=\"{id}-checks\" width=\"100\" height=\"100\" patternUnits=\"userSpaceOnUse\" patternTransform=\"rotate(-10 300 300)\">\
<rect width=\"50\" height=\"50\" fill=\"{}\" opacity=\".22\"/>\
<rect x=\"50\" y=\"50\" width=\"50\" height=\"50\" fill=\"{}\" opacity=\".22\"/>\
</pattern>",
            palette.light, palette.light,
        )
    } else {
        String::new()
    };
    if traits.species >= HEADS.len() {
        let creature = draw_creature(traits, &palette, id);
        return format!(
            "<svg {attrs} data-avatar-version=\"{VERSION}\">\
{description}<defs>{background_defs}</defs>\
<g stroke-linecap=\"round\" stroke-linejoin=\"round\">{bg_svg}{creature}</g></svg>"
        );
    }
    let outfit_svg = draw_outfit(traits, &palette);
    let head_svg = draw_head(traits, &palette, id);
    let eyes_svg = draw_eyes(traits, &palette);
    let mouth_svg = draw_mouth(traits, &palette);
    let hat_svg = draw_hat(traits, &palette);
    let acc_svg = draw_accessory(traits, &palette);
    let (face_acc, body_acc) = if matches!(traits.accessory, 1 | 2) {
        (acc_svg.as_str(), "")
    } else {
        ("", acc_svg.as_str())
    };
    format!(
        "<svg {attrs} data-avatar-version=\"{VERSION}\">\
{description}\
<defs>\
<clipPath id=\"{id}-head\"><path d=\"{}\"/></clipPath>\
{background_defs}\
</defs>\
<g stroke-linecap=\"round\" stroke-linejoin=\"round\">\
{bg_svg}\
{outfit_svg}\
<g transform=\"rotate(-4 335 370)\">\
{head_svg}\
{eyes_svg}\
{mouth_svg}\
{hat_svg}\
{face_acc}\
</g>\
{body_acc}\
</g>\
</svg>",
        HEADS[traits.species],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_schema_rejects_missing_unknown_and_incomplete_assets() {
        let catalog: serde_json::Value =
            serde_json::from_str(include_str!("avatar-assets.json")).unwrap();
        assert!(serde_json::from_value::<AssetCatalog>(catalog.clone()).is_ok());
        let mut missing = catalog.clone();
        missing.as_object_mut().unwrap().remove("finish");
        assert!(serde_json::from_value::<AssetCatalog>(missing).is_err());
        let mut unknown = catalog.clone();
        unknown["creatures"][0]["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<AssetCatalog>(unknown).is_err());
        for axis in [
            "hat",
            "outfit",
            "eyes",
            "accessory",
            "background",
            "creatures",
        ] {
            let mut incomplete = catalog.clone();
            incomplete[axis].as_array_mut().unwrap().pop();
            assert!(
                serde_json::from_value::<AssetCatalog>(incomplete).is_err(),
                "{axis}"
            );
        }
        for axis in ["eyes", "mouths"] {
            let mut incomplete = catalog.clone();
            incomplete["creatures"][0][axis]
                .as_array_mut()
                .unwrap()
                .pop();
            assert!(
                serde_json::from_value::<AssetCatalog>(incomplete).is_err(),
                "{axis}"
            );
        }
    }

    #[test]
    fn native_species_are_complete_independent_characters() {
        assert_eq!(SPECIES_NAMES.len(), HEADS.len() + assets().creatures.len());
        for (native_index, creature) in assets().creatures.iter().enumerate() {
            let species = HEADS.len() + native_index;
            let species_name = SPECIES_NAMES[species];
            assert_eq!(creature.name, species_name);
            assert_eq!(creature.eyes.len(), 3);
            assert_eq!(creature.mouths.len(), 4);
            for palette in &PALETTES {
                let mut distinct = std::collections::HashSet::new();
                for eyes in 0..3 {
                    for mood in 0..4 {
                        let traits = compatible_traits(Traits {
                            species,
                            eyes,
                            mood,
                            ..derive_traits("native")
                        });
                        let svg = draw_creature(&traits, palette, "audit");
                        assert!(svg.contains(species_name));
                        assert!(!svg.contains('{'));
                        assert!(!svg.contains("<image"));
                        assert!(!svg.contains("<filter"));
                        distinct.insert(svg);
                    }
                }
                assert_eq!(distinct.len(), 12);
            }
        }
    }

    #[test]
    fn native_traits_do_not_count_unused_clothing_as_variation() {
        for species in HEADS.len()..SPECIES_NAMES.len() {
            let base = compatible_traits(Traits {
                species,
                eyes: 1,
                ..derive_traits("native")
            });
            let dressed = Traits {
                hat: 9,
                outfit: 7,
                accessory: 7,
                eyes: 7,
                ..base
            };
            assert_eq!(compatible_traits(dressed), base);
            assert_eq!(fingerprint(&dressed), fingerprint(&base));
        }
    }

    #[test]
    fn native_layers_match_preview_golden_artwork() {
        let vectors: serde_json::Value =
            serde_json::from_str(include_str!("../tests/avatar-vectors.json")).unwrap();
        let mut checked = std::collections::HashSet::new();
        for vector in vectors.as_array().unwrap() {
            if let Some(expected) = vector["creature_hash"].as_u64() {
                let traits = derive_traits(vector["seed"].as_str().unwrap());
                assert_eq!(
                    hash32(&draw_creature(&traits, &PALETTES[traits.palette], "audit")),
                    expected as u32
                );
                checked.insert(traits.species);
            }
        }
        assert_eq!(checked.len(), 4);
    }

    #[test]
    fn every_material_renders_on_every_species_and_palette() {
        for species in 0..HEADS.len() {
            for palette in &PALETTES {
                let mut traits = derive_traits("material-audit");
                traits.species = species;
                for hat in 0..AXES_HAT_WEIGHTS.len() {
                    traits.hat = hat;
                    assert!(!draw_hat(&traits, palette).contains('{'));
                }
                for outfit in 0..AXES_OUTFIT_WEIGHTS.len() {
                    traits.outfit = outfit;
                    assert!(!draw_outfit(&traits, palette).is_empty());
                }
                for eyes in 0..AXES_EYES_WEIGHTS.len() {
                    traits.eyes = eyes;
                    assert!(!draw_eyes(&traits, palette).is_empty());
                }
                for accessory in 0..AXES_ACCESSORY_WEIGHTS.len() {
                    traits.accessory = accessory;
                    assert!(!draw_accessory(&traits, palette).contains('{'));
                }
                for background in 0..AXES_BACKGROUND_WEIGHTS.len() {
                    traits.background = background;
                    assert!(!draw_background(&traits, palette, "audit").is_empty());
                }
            }
        }
    }

    #[test]
    fn deterministic_generation() {
        let s1 = svg_for_seed("odd-folk-studio-001", Some(22), Some("face"));
        let s2 = svg_for_seed("odd-folk-studio-001", Some(22), Some("face"));
        assert_eq!(s1, s2);
        assert!(s1.contains("viewBox=\"0 0 600 600\""));
        assert!(s1.contains("class=\"face\""));
        assert!(s1.contains("width=\"22\""));
    }

    #[test]
    fn traits_diversity() {
        let t1 = derive_traits("alice");
        let t2 = derive_traits("bob");
        let t3 = derive_traits("charlie");
        assert_ne!(fingerprint(&t1), fingerprint(&t2));
        assert_ne!(fingerprint(&t2), fingerprint(&t3));
    }

    #[test]
    fn empty_seed_fallback() {
        let s = svg_for_seed("", None, None);
        assert!(s.contains("<svg"));
        assert!(s.ends_with("</svg>"));
    }
}
