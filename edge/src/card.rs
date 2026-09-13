//! 邀请卡（DESIGN §3.4、§4.9）：一张 PNG，是门禁页在微信里的实体。
//!
//! 竖版 `/_playtest/card.png`（1080×1350）发群里，横版 `/_playtest/card-wide.png`
//! （1200×630）做没有封面时的链接预览图。两张都由边缘按当前版本渲染：
//! 拼一段 SVG → `usvg` 解析 → `resvg` 光栅化 → `tiny-skia` 出 PNG。
//!
//! 为什么在边缘渲染而不是 CLI 或浏览器：只有边缘同时手里有清单、封面 blob 和 `live.json`
//! 里的名额（DESIGN §4.9）。渲染结果按 `ETag` 在内存里缓存，一次渲染多次命中。
//!
//! **口吻是「某某邀请你」，不是「你获得了」**（DESIGN §3.4 的硬线）。所以卡上没有玩过人数、
//! 没有「限时」「领取」「立即」、没有品类标签，也没有我们的任何一句营销话——
//! 邀请函是私人递出的东西，兑换券是商家发的东西，我们只能是前者。
//!
//! 中文字体从系统读，不编进二进制（十几 MB，且任何标题都可能用到子集之外的字）。
//! 一个都没读到时仍然出图，中文会是方块——启动日志里说清楚怎么装（AGENTS 第 4 条）。

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use base64::Engine as _;
use bytes::Bytes;
use playtest_common::live::SiteLive;
use playtest_common::manifest::Manifest;
use playtest_common::wording::{audience_noun, invite_verb};

use crate::when;
use playtest_common::{card_qr_url, CARD_HEIGHT, CARD_WIDE_HEIGHT, CARD_WIDE_WIDTH, CARD_WIDTH};
use qrcode::{Color as QrColor, QrCode};

use crate::html::{esc, hue};

/// 卡在浏览器与中间层里缓存多久。名额变了 `ETag` 就变，5 分钟是「转发出去的图不会太旧」
/// 和「一屏广场不要每张都回源」之间的中点。
pub const MAX_AGE: u64 = 300;

/// 内存里最多存几张渲染好的 PNG。一张竖版约 200–600 KB，256 张是百兆量级的上限；
/// 满了整个清掉而不是逐个淘汰——淘汰算法要维护顺序，而这里重渲一次只是几十毫秒。
const CACHE_CAP: usize = 256;

// 一套记号和门禁页、广场共用（`html.rs` 的 CSS）：同一张封面、同一句话、同一个版本号。
const BG: &str = "#0a0b0e";
const VOID: &str = "#05060a";
const FG: &str = "#f2f3f5";
const DIM: &str = "#9aa2b1";
const FAINT: &str = "#6b7280";
const BODY: &str = "#b6bdc9";
const ACCENT: &str = "#ffffff";
const RULE: &str = "#2b3039";

/// 候选字体一串写在 SVG 里，谁在就用谁。装了 `fonts-noto-cjk` 的服务器命中第一个，
/// macOS 命中 PingFang，Windows 命中雅黑。
const FAMILY: &str =
    "&quot;Noto Sans CJK SC&quot;, &quot;Noto Sans SC&quot;, &quot;PingFang SC&quot;, \
&quot;Hiragino Sans GB&quot;, &quot;Microsoft YaHei&quot;, &quot;Source Han Sans SC&quot;, \
&quot;WenQuanYi Micro Hei&quot;, sans-serif";

/// 版本、日期这类小字用等宽——和门禁页、广场上的元数据是同一种排法。
const MONO: &str =
    "ui-monospace, &quot;SF Mono&quot;, Menlo, Consolas, &quot;DejaVu Sans Mono&quot;, monospace";

/// 系统里认得出的中文字体家族。探到一个就把它设成 usvg 的兜底家族，
/// 免得候选全落空时连一个能用的字形都找不到。
const CJK_HINTS: &[&str] = &[
    "Noto Sans CJK SC",
    "Noto Sans SC",
    "PingFang SC",
    "Hiragino Sans GB",
    "Microsoft YaHei",
    "Source Han Sans SC",
    "WenQuanYi Micro Hei",
    "Noto Serif CJK SC",
    "Heiti SC",
    "Songti SC",
    "SimSun",
    "SimHei",
];

/// 另加的字体目录，冒号分隔（本机想试某一款字体时用）。
pub const FONT_DIRS_ENV: &str = "PLAYTEST_FONT_DIRS";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// 1080×1350，发群里的那张。
    Portrait,
    /// 1200×630，链接预览用的那张。
    Wide,
}

impl Shape {
    pub fn size(self) -> (u32, u32) {
        match self {
            Shape::Portrait => (CARD_WIDTH, CARD_HEIGHT),
            Shape::Wide => (CARD_WIDE_WIDTH, CARD_WIDE_HEIGHT),
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Shape::Portrait => "p",
            Shape::Wide => "w",
        }
    }
}

/// 封面的字节与类型。没有封面时排一张字卡，不编一张假截图。
pub struct Cover<'a> {
    pub mime: &'a str,
    pub bytes: &'a [u8],
}

/// 一张卡要放的全部东西。
pub struct Card<'a> {
    pub manifest: &'a Manifest,
    pub live: &'a SiteLive,
    /// 作品的源 `scheme://<slug>.<后缀>[:端口]`，二维码里就是它加 `?from=card`。
    pub origin: &'a str,
    /// 角落那个名字（本机是 `localhost`）。`manifest.badge` 为假（Pro）时不放。
    pub host_suffix: &'a str,
    pub cover: Option<Cover<'a>>,
    /// 版本那个位置显示什么。隧道没有版本这个概念，传「在线」（和门禁页同一个选择）。
    pub version_label: Option<&'a str>,
    pub shape: Shape,
}

impl Card<'_> {
    /// 内容一样就命中：卡上会变的东西只有版本（连带标题、介绍、封面）与名额。
    pub fn etag(&self) -> String {
        let seed = format!(
            "{}|{}|{}|{:?}|{}|{}|{}",
            self.manifest.slug,
            self.manifest.version,
            self.version_label.unwrap_or(""),
            self.live.seats,
            self.live.joined,
            self.cover.is_some(),
            self.shape.tag(),
        );
        format!(
            "\"{}\"",
            &playtest_common::hash::hash_bytes(seed.as_bytes())[..20]
        )
    }

    pub fn svg(&self) -> String {
        match self.shape {
            Shape::Portrait => self.portrait(),
            Shape::Wide => self.wide(),
        }
    }

    // ------------------------------------------------------------ 竖版 1080×1350

    fn portrait(&self) -> String {
        let (w, h) = (CARD_WIDTH as f32, CARD_HEIGHT as f32);
        let pad = 80.0;
        let text_w = w - pad * 2.0;
        let cover_h = 675.0; // 16:10
        let tear_y = 1105.0;

        let title_lines = wrap(&self.title(), cells(text_w, 66.0), 2);
        let summary_lines = self
            .summary()
            .map(|text| wrap(&text, cells(text_w, 30.0), 3))
            .unwrap_or_default();

        let mut s = String::with_capacity(4096);
        s.push_str(&head(w, h));
        s.push_str(&self.art(0.0, 0.0, w, cover_h, 340.0, pad));
        s.push_str(&format!("<g font-family='{FAMILY}'>\n"));

        // 一句话、作品名、介绍是一整块，行数随内容变。把整块摆在封面和撕票线正中间：
        // 只有一行介绍时下面不会空一片，三行时也不会顶到线上。
        let mut block = 84.0 + 82.0 * (title_lines.len() as f32 - 1.0);
        if !summary_lines.is_empty() {
            block += 58.0 + 46.0 * (summary_lines.len() as f32 - 1.0);
        }
        let mut y = cover_h + (tear_y - cover_h - block - 26.0) / 2.0 + 26.0;

        // 「某某 邀请你试玩」——卡上唯一的一句话，和门禁页上那一句一字不差。
        s.push_str(&line(
            pad,
            y,
            &format!("{} {}", esc(&self.manifest.developer), self.verb()),
            32.0,
            DIM,
            None,
        ));

        y += 84.0;
        for text in &title_lines {
            s.push_str(&line(pad, y, &esc(text), 66.0, FG, Some(700)));
            y += 82.0;
        }
        y -= 82.0;

        if !summary_lines.is_empty() {
            y += 58.0;
            for text in &summary_lines {
                s.push_str(&line(pad, y, &esc(text), 30.0, BODY, None));
                y += 46.0;
            }
        }

        s.push_str(&tear_across(0.0, w, tear_y));

        // 票根：二维码在左，小字在右。名额、版本日期这些「会变的」都在这里，
        // 上半张是不变的那部分——撕开之后两半各自成立。
        let qr = 170.0;
        let qr_y = 1136.0;
        s.push_str(&self.qr(pad, qr_y, qr));
        let info_x = pad + qr + 34.0;
        let mut info_y = qr_y + 50.0;
        s.push_str(&mono_line(info_x, info_y, &esc(&self.stamp()), 28.0, DIM));
        if let Some(seats) = self.seats() {
            info_y += 46.0;
            s.push_str(&line(info_x, info_y, &esc(&seats), 29.0, ACCENT, Some(600)));
        }
        info_y += 46.0;
        s.push_str(&mono_line(info_x, info_y, &esc(&self.host()), 25.0, FAINT));

        if self.manifest.badge {
            s.push_str(&end_line(
                w - pad,
                h - 44.0,
                &esc(self.host_suffix),
                24.0,
                FAINT,
            ));
        }
        s.push_str("</g>\n</svg>\n");
        s
    }

    // -------------------------------------------------------------- 横版 1200×630

    fn wide(&self) -> String {
        let (w, h) = (CARD_WIDE_WIDTH as f32, CARD_WIDE_HEIGHT as f32);
        let art_w = 560.0;
        let x = 612.0;
        let text_w = 372.0;

        let title_lines = wrap(&self.title(), cells(text_w, 44.0), 2);
        let summary_lines = self
            .summary()
            .map(|text| wrap(&text, cells(text_w, 24.0), 2))
            .unwrap_or_default();
        let seats = self.seats();

        let mut s = String::with_capacity(4096);
        s.push_str(&head(w, h));
        s.push_str(&self.art(0.0, 0.0, art_w, h, 300.0, 52.0));
        s.push_str(&tear_down(art_w, 0.0, h));
        s.push_str(&format!("<g font-family='{FAMILY}'>\n"));

        // 和竖版同一个道理：右边这一列摆在正中间，行数多少都对齐二维码。
        let mut block = 60.0 + 56.0 * (title_lines.len() as f32 - 1.0);
        if !summary_lines.is_empty() {
            block += 46.0 + 38.0 * (summary_lines.len() as f32 - 1.0);
        }
        block += 48.0;
        if seats.is_some() {
            block += 42.0;
        }
        let mut y = (h - block - 20.0) / 2.0 + 20.0;

        s.push_str(&line(
            x,
            y,
            &format!("{} {}", esc(&self.manifest.developer), self.verb()),
            24.0,
            DIM,
            None,
        ));

        y += 60.0;
        for text in &title_lines {
            s.push_str(&line(x, y, &esc(text), 44.0, FG, Some(700)));
            y += 56.0;
        }
        y -= 56.0;

        if !summary_lines.is_empty() {
            y += 46.0;
            for text in &summary_lines {
                s.push_str(&line(x, y, &esc(text), 24.0, BODY, None));
                y += 38.0;
            }
            y -= 38.0;
        }

        y += 48.0;
        s.push_str(&mono_line(x, y, &esc(&self.stamp()), 22.0, DIM));
        if let Some(seats) = self.seats() {
            y += 42.0;
            s.push_str(&line(x, y, &esc(&seats), 24.0, ACCENT, Some(600)));
        }

        s.push_str(&self.qr(1016.0, 241.0, 148.0));
        if self.manifest.badge {
            s.push_str(&end_line(
                w - 48.0,
                h - 36.0,
                &esc(self.host_suffix),
                22.0,
                FAINT,
            ));
        }
        s.push_str("</g>\n</svg>\n");
        s
    }

    // ------------------------------------------------------------------ 各块内容

    /// 封面，或者一张字卡。字卡的色相和广场上那张同一个算法，同一个作品每次都是同一种颜色。
    fn art(&self, x: f32, y: f32, w: f32, h: f32, mark_size: f32, inset: f32) -> String {
        match &self.cover {
            Some(cover) => format!(
                "<image x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" \
preserveAspectRatio=\"xMidYMid slice\" href=\"data:{mime};base64,{data}\"/>\n\
<rect x=\"{x}\" y=\"{fade_y}\" width=\"{w}\" height=\"{fade_h}\" fill=\"url(#fade)\"/>\n",
                mime = esc(cover.mime),
                data = base64::engine::general_purpose::STANDARD.encode(cover.bytes),
                fade_y = y + h - 130.0,
                fade_h = 130.0,
            ),
            None => {
                // 作品名在下面已经是整张卡最大的字，这里再排一遍就是同一句话说两次。
                // 放作品名的头一个字当记号：还是设计好的版式，不是假装的截图。
                let h_deg = hue(&self.manifest.slug);
                format!(
                    "<defs><linearGradient id=\"word\" x1=\"0\" y1=\"0\" x2=\"0.6\" y2=\"1\">\
<stop offset=\"0\" stop-color=\"{top}\"/><stop offset=\"0.85\" stop-color=\"#0b0c10\"/>\
</linearGradient></defs>\n\
<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"url(#word)\"/>\n\
<text x=\"{mx}\" y=\"{my}\" font-family='{FAMILY}' font-size=\"{mark_size}\" \
font-weight=\"700\" fill=\"#ffffff\" fill-opacity=\"0.13\">{mark}</text>\n",
                    top = hsl_hex(h_deg, 0.40, 0.22),
                    mx = x + inset,
                    my = y + h - mark_size * 0.14,
                    mark = esc(&self.mark()),
                )
            }
        }
    }

    fn qr(&self, x: f32, y: f32, size: f32) -> String {
        let door = playtest_common::door_url(self.origin, &self.manifest.slug);
        qr_svg(&card_qr_url(&door), x, y, size)
    }

    fn verb(&self) -> &'static str {
        invite_verb(self.manifest.kind, self.manifest.is_game())
    }

    fn title(&self) -> String {
        format!("《{}》", self.manifest.title)
    }

    /// 没封面时那个大字：作品名的头一个字。跳过引号书名号一类的符号，
    /// 拉丁字母大写——一个作品每次都是同一个字，和色相一样认得出来。
    fn mark(&self) -> String {
        self.manifest
            .title
            .chars()
            .chain(self.manifest.slug.chars())
            .find(|ch| ch.is_alphanumeric())
            .map(|ch| ch.to_uppercase().to_string())
            .unwrap_or_default()
    }

    fn summary(&self) -> Option<String> {
        self.manifest
            .summary
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    }

    /// `v7 · 9 月 9 日`；匿名链接换成到期时间——那才是拿到这张卡的人需要知道的事。
    fn stamp(&self) -> String {
        if let Some(until) = self.manifest.expires_at.as_deref().and_then(when::day_time) {
            return format!("这张邀请到 {until}");
        }
        let version = match self.version_label {
            Some(label) => label.to_string(),
            None => format!("v{}", self.manifest.version),
        };
        match when::day(&self.manifest.created_at) {
            Some(day) => format!("{version} · {day}"),
            None => version,
        }
    }

    /// 名额（设了才有）。**卡上不放已加入人数**：0 人不说是 DESIGN §3.5 的规矩，
    /// 卡上没有可数的东西，就一个字不放。
    fn seats(&self) -> Option<String> {
        match self.live.seats {
            Some(n) if n > 0 => Some(format!(
                "在找 {n} 位{}",
                audience_noun(self.manifest.kind, self.manifest.is_game())
            )),
            _ => None,
        }
    }

    fn host(&self) -> String {
        self.origin
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(self.origin)
            .to_string()
    }
}

// ------------------------------------------------------------------ SVG 零件

fn head(w: f32, h: f32) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" \
viewBox=\"0 0 {w} {h}\">\n\
<defs><linearGradient id=\"fade\" x1=\"0\" y1=\"0\" x2=\"0\" y2=\"1\">\
<stop offset=\"0\" stop-color=\"{BG}\" stop-opacity=\"0\"/>\
<stop offset=\"1\" stop-color=\"{BG}\" stop-opacity=\"1\"/></linearGradient></defs>\n\
<rect width=\"{w}\" height=\"{h}\" fill=\"{BG}\"/>\n"
    )
}

fn line(x: f32, y: f32, text: &str, size: f32, fill: &str, weight: Option<u32>) -> String {
    let weight = match weight {
        Some(w) => format!(" font-weight=\"{w}\""),
        None => String::new(),
    };
    format!(
        "<text x=\"{x}\" y=\"{y}\" font-size=\"{size}\" fill=\"{fill}\"{weight}>{text}</text>\n"
    )
}

fn mono_line(x: f32, y: f32, text: &str, size: f32, fill: &str) -> String {
    format!(
        "<text x=\"{x}\" y=\"{y}\" font-family='{MONO}' font-size=\"{size}\" fill=\"{fill}\" \
letter-spacing=\"0.4\">{text}</text>\n"
    )
}

fn end_line(x: f32, y: f32, text: &str, size: f32, fill: &str) -> String {
    format!(
        "<text x=\"{x}\" y=\"{y}\" text-anchor=\"end\" font-size=\"{size}\" fill=\"{fill}\">{text}</text>\n"
    )
}

/// 一条撕票线：虚线加两端的半圆缺口。它是这张卡最像「票」的地方，也是 DESIGN §3.4
/// 借 ZCODE 那类凭证卡的克制的那一笔——深底、一张图、一个署名、一个日期、一条撕票线，别的没有。
fn tear_across(x0: f32, x1: f32, y: f32) -> String {
    format!(
        "<line x1=\"{a}\" y1=\"{y}\" x2=\"{b}\" y2=\"{y}\" stroke=\"{RULE}\" stroke-width=\"3\" \
stroke-dasharray=\"14 12\"/>\n\
<circle cx=\"{x0}\" cy=\"{y}\" r=\"22\" fill=\"{VOID}\"/>\n\
<circle cx=\"{x1}\" cy=\"{y}\" r=\"22\" fill=\"{VOID}\"/>\n",
        a = x0 + 26.0,
        b = x1 - 26.0,
    )
}

fn tear_down(x: f32, y0: f32, y1: f32) -> String {
    format!(
        "<line x1=\"{x}\" y1=\"{a}\" x2=\"{x}\" y2=\"{b}\" stroke=\"{RULE}\" stroke-width=\"3\" \
stroke-dasharray=\"14 12\"/>\n\
<circle cx=\"{x}\" cy=\"{y0}\" r=\"20\" fill=\"{VOID}\"/>\n\
<circle cx=\"{x}\" cy=\"{y1}\" r=\"20\" fill=\"{VOID}\"/>\n",
        a = y0 + 24.0,
        b = y1 - 24.0,
    )
}

/// 二维码：只要 `qrcode` 给的模块矩阵，形状我们自己拼——一行里连着的暗块合成一个矩形，
/// path 短一半，`usvg` 少解析一半。白底是为了扫得动：深色卡面上直接印暗色模块识别率很差。
fn qr_svg(url: &str, x: f32, y: f32, size: f32) -> String {
    let Ok(code) = QrCode::new(url.as_bytes()) else {
        tracing::warn!(url, "二维码编不出来，这张卡上就没有码");
        return String::new();
    };
    let n = code.width();
    // 规范要求四个模块的静区，少了很多识别器不认。
    let quiet = 4.0;
    let module = size / (n as f32 + quiet * 2.0);
    let origin_x = x + quiet * module;
    let origin_y = y + quiet * module;
    let colors = code.to_colors();

    let mut d = String::new();
    for row in 0..n {
        let mut col = 0;
        while col < n {
            if colors[row * n + col] != QrColor::Dark {
                col += 1;
                continue;
            }
            let start = col;
            while col < n && colors[row * n + col] == QrColor::Dark {
                col += 1;
            }
            let run = (col - start) as f32;
            d.push_str(&format!(
                "M{:.2} {:.2}h{:.2}v{:.2}h-{:.2}z",
                origin_x + start as f32 * module,
                origin_y + row as f32 * module,
                run * module,
                module,
                run * module,
            ));
        }
    }
    format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{size}\" height=\"{size}\" rx=\"10\" fill=\"#ffffff\"/>\n\
<path d=\"{d}\" fill=\"#0b0d12\"/>\n"
    )
}

/// HSL → `#rrggbb`。SVG 里直接写 `hsl()` 也行，但换算在这里，字卡的颜色就能被测。
fn hsl_hex(h: u32, s: f32, l: f32) -> String {
    let h = (h % 360) as f32 / 60.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let byte = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", byte(r), byte(g), byte(b))
}

/// 一行放得下几「格」。一个汉字两格、一个拉丁字母一格，格宽按半个字号估——
/// 这是不做真正排版的前提下最省事又不会差太多的近似。
fn cells(width: f32, font_size: f32) -> usize {
    (width / (font_size * 0.5)).floor().max(1.0) as usize
}

// ------------------------------------------------------------------ 折行

/// 按显示宽度折行。中日韩字符哪里都能断，拉丁词只在空格处断（一个词自己超过一行才硬断）。
///
/// 单位是「格」：一个汉字两格、一个字母一格。超出 `max_lines` 时最后一行以 `…` 收尾——
/// 让卡自己决定放不下就不放，比让一行字溢出卡面好。
pub fn wrap(text: &str, cells_per_line: usize, max_lines: usize) -> Vec<String> {
    if cells_per_line == 0 || max_lines == 0 {
        return Vec::new();
    }
    let units = units_of(text);
    if units.is_empty() {
        return Vec::new();
    }

    let mut lines: Vec<(String, usize)> = Vec::new();
    let mut current = (String::new(), 0usize);
    let mut overflow = false;

    for unit in units {
        let space = if current.1 > 0 && unit.space_before {
            1
        } else {
            0
        };
        if current.1 + space + unit.cells > cells_per_line && current.1 > 0 {
            lines.push(std::mem::take(&mut current));
            if lines.len() == max_lines {
                overflow = true;
                break;
            }
        }
        if current.1 > 0 && unit.space_before {
            current.0.push(' ');
            current.1 += 1;
        }
        if unit.cells > cells_per_line {
            // 一个单元自己就比一行长（一串没有空格的链接之类）：只能硬断。
            for ch in unit.text.chars() {
                let cw = char_cells(ch);
                if current.1 + cw > cells_per_line && current.1 > 0 {
                    lines.push(std::mem::take(&mut current));
                    if lines.len() == max_lines {
                        overflow = true;
                        break;
                    }
                }
                current.0.push(ch);
                current.1 += cw;
            }
            if overflow {
                break;
            }
        } else {
            current.0.push_str(&unit.text);
            current.1 += unit.cells;
        }
    }
    if !overflow && !current.0.is_empty() {
        lines.push(current);
    }
    if overflow {
        if let Some(last) = lines.last_mut() {
            *last = with_ellipsis(&last.0, cells_per_line);
        }
    }
    lines.into_iter().map(|(text, _)| text).collect()
}

struct Unit {
    text: String,
    cells: usize,
    space_before: bool,
}

fn units_of(text: &str) -> Vec<Unit> {
    let mut out: Vec<Unit> = Vec::new();
    let mut word = String::new();
    let mut word_cells = 0usize;
    let mut pending_space = false;

    for ch in text.trim().chars() {
        if ch.is_whitespace() {
            flush(&mut word, &mut word_cells, &mut pending_space, &mut out);
            pending_space = true;
            continue;
        }
        let cells = char_cells(ch);
        if cells == 2 {
            flush(&mut word, &mut word_cells, &mut pending_space, &mut out);
            out.push(Unit {
                text: ch.to_string(),
                cells,
                space_before: std::mem::take(&mut pending_space),
            });
        } else {
            word.push(ch);
            word_cells += cells;
        }
    }
    flush(&mut word, &mut word_cells, &mut pending_space, &mut out);
    out
}

fn flush(word: &mut String, cells: &mut usize, pending_space: &mut bool, out: &mut Vec<Unit>) {
    if word.is_empty() {
        return;
    }
    out.push(Unit {
        text: std::mem::take(word),
        cells: std::mem::replace(cells, 0),
        space_before: std::mem::take(pending_space),
    });
}

fn with_ellipsis(text: &str, cells_per_line: usize) -> (String, usize) {
    let mut chars: Vec<char> = text.trim_end().chars().collect();
    let mut total: usize = chars.iter().copied().map(char_cells).sum();
    // 省略号自己也要占位置，按宽字符算——中文字体里它就是一个全角的点。
    while total + 2 > cells_per_line && !chars.is_empty() {
        if let Some(ch) = chars.pop() {
            total -= char_cells(ch);
        }
    }
    while chars.last().is_some_and(|c| c.is_whitespace()) {
        chars.pop();
        total -= 1;
    }
    let mut out: String = chars.into_iter().collect();
    out.push('…');
    (out, total + 2)
}

/// 一个字符占几格。East Asian Wide / Fullwidth 算两格，其余一格——够用的近似，
/// 不引 unicode-width 那一层依赖。
fn char_cells(ch: char) -> usize {
    let c = ch as u32;
    let wide = (0x1100..=0x115F).contains(&c)
        || (0x2E80..=0x303E).contains(&c)
        || (0x3041..=0x33FF).contains(&c)
        || (0x3400..=0x4DBF).contains(&c)
        || (0x4E00..=0x9FFF).contains(&c)
        || (0xA000..=0xA4CF).contains(&c)
        || (0xAC00..=0xD7A3).contains(&c)
        || (0xF900..=0xFAFF).contains(&c)
        || (0xFE10..=0xFE19).contains(&c)
        || (0xFE30..=0xFE6F).contains(&c)
        || (0xFF00..=0xFF60).contains(&c)
        || (0xFFE0..=0xFFE6).contains(&c)
        || (0x20000..=0x3FFFD).contains(&c);
    if wide {
        2
    } else {
        1
    }
}

// ------------------------------------------------------------------ 渲染

/// 字体库与渲染好的 PNG。
///
/// 字体**第一次渲染时才加载**，而且整个进程只加载一次：`load_system_fonts` 要扫整个
/// 字体目录，几十到几百毫秒，那是一段纯阻塞的活。放在启动路径上会卡住 tokio 的线程，
/// 放在每张卡上会让「100 ms 以内」变成笑话（DESIGN §4.9）。调用方本来就在
/// `spawn_blocking` 里渲染，所以第一张卡付这笔账正合适。
pub struct Renderer {
    fonts: OnceLock<Fonts>,
    cache: Mutex<HashMap<String, Bytes>>,
}

struct Fonts {
    db: Arc<usvg::fontdb::Database>,
    /// 探到的中文字体，做 usvg 的兜底家族；一个都没探到就是 `sans-serif`。
    fallback_family: String,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

/// 整个进程共用的那一个：字体只加载一次，渲染好的卡大家一起用。
pub fn shared() -> Arc<Renderer> {
    static SHARED: OnceLock<Arc<Renderer>> = OnceLock::new();
    SHARED.get_or_init(|| Arc::new(Renderer::new())).clone()
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            fonts: OnceLock::new(),
            cache: Mutex::new(HashMap::new()),
        }
    }

    fn fonts(&self) -> &Fonts {
        self.fonts.get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            db.load_system_fonts();
            if let Ok(dirs) = std::env::var(FONT_DIRS_ENV) {
                for dir in dirs.split(':').filter(|d| !d.is_empty()) {
                    db.load_fonts_dir(dir);
                }
            }
            let fallback_family = cjk_family(&db);
            match &fallback_family {
                Some(family) => tracing::info!(%family, faces = db.len(), "邀请卡用这个中文字体"),
                None => tracing::warn!(
                    faces = db.len(),
                    "没有找到中文字体，邀请卡上的中文会是方块；装 fonts-noto-cjk 或设 {}",
                    FONT_DIRS_ENV
                ),
            }
            Fonts {
                db: Arc::new(db),
                fallback_family: fallback_family.unwrap_or_else(|| "sans-serif".to_string()),
            }
        })
    }

    /// 这个 `ETag` 的 PNG 之前渲染过没有。
    pub fn cached(&self, etag: &str) -> Option<Bytes> {
        self.cache.lock().ok()?.get(etag).cloned()
    }

    pub fn render(&self, card: &Card<'_>) -> anyhow::Result<Bytes> {
        let etag = card.etag();
        if let Some(hit) = self.cached(&etag) {
            return Ok(hit);
        }
        self.render_svg(etag, &card.svg(), card.shape)
    }

    /// 光栅化并按 `ETag` 记下来。拼 SVG 是廉价的，光栅化是几十毫秒的纯 CPU，
    /// 所以调用方在异步环境里应当把这一步丢给 `spawn_blocking`。
    pub fn render_svg(&self, etag: String, svg: &str, shape: Shape) -> anyhow::Result<Bytes> {
        if let Some(hit) = self.cached(&etag) {
            return Ok(hit);
        }
        let png = Bytes::from(self.rasterize(svg, shape)?);
        if let Ok(mut cache) = self.cache.lock() {
            // 满了整个清掉：淘汰算法要维护顺序，而重渲一次只是几十毫秒。
            if cache.len() >= CACHE_CAP {
                cache.clear();
            }
            cache.insert(etag, png.clone());
        }
        Ok(png)
    }

    /// 只把 SVG 变成 PNG。测试和 spike 里量耗时用它。
    pub fn rasterize(&self, svg: &str, shape: Shape) -> anyhow::Result<Vec<u8>> {
        let fonts = self.fonts();
        let mut options = usvg::Options {
            fontdb: fonts.db.clone(),
            ..Default::default()
        };
        options.font_family = fonts.fallback_family.clone();
        let tree = usvg::Tree::from_str(svg, &options)?;
        let (w, h) = shape.size();
        let mut pixmap = tiny_skia::Pixmap::new(w, h)
            .ok_or_else(|| anyhow::anyhow!("{w}×{h} 的画布建不出来"))?;
        resvg::render(
            &tree,
            tiny_skia::Transform::identity(),
            &mut pixmap.as_mut(),
        );
        Ok(pixmap.encode_png()?)
    }
}

fn cjk_family(db: &usvg::fontdb::Database) -> Option<String> {
    let names: Vec<String> = db
        .faces()
        .flat_map(|face| face.families.iter().map(|(name, _)| name.clone()))
        .collect();
    CJK_HINTS
        .iter()
        .find(|hint| names.iter().any(|name| name.eq_ignore_ascii_case(hint)))
        .map(|hint| (*hint).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use playtest_common::manifest::{GateMode, WorkKind, SCHEMA};

    fn manifest() -> Manifest {
        Manifest {
            schema: SCHEMA,
            slug: "brisk-otter-41".into(),
            version: 7,
            title: "小球大冒险".into(),
            developer: "某某".into(),
            note: None,
            summary: Some("三关，五分钟，手机上也能玩。".into()),
            cover: None,
            created_at: "2026-09-09T04:00:00Z".into(),
            expires_at: None,
            badge: true,
            gate: GateMode::Once,
            isolated: false,
            spa: false,
            engine: Some("godot".into()),
            kind: playtest_common::manifest::WorkKind::Web,
            entry: None,
            article: None,
            chapters: vec![],
            files: vec![],
        }
    }

    fn card<'a>(m: &'a Manifest, live: &'a SiteLive, shape: Shape) -> Card<'a> {
        Card {
            manifest: m,
            live,
            origin: "http://brisk-otter-41.localhost:8443",
            host_suffix: "localhost",
            cover: None,
            version_label: None,
            shape,
        }
    }

    #[test]
    fn wraps_chinese_anywhere_and_english_at_spaces() {
        assert_eq!(
            wrap("一二三四五六", 4, 3),
            vec!["一二".to_string(), "三四".into(), "五六".into()]
        );
        assert_eq!(
            wrap("hello brave new world", 12, 3),
            vec!["hello brave".to_string(), "new world".into()]
        );
        // 中英混排：英文词整块搬，中文字随便断。
        assert_eq!(
            wrap("玩 Godot 的人", 8, 2),
            vec!["玩 Godot".to_string(), "的人".into()]
        );
    }

    #[test]
    fn overflow_ends_with_an_ellipsis() {
        let lines = wrap("一二三四五六七八九十", 4, 2);
        assert_eq!(lines, vec!["一二".to_string(), "三…".into()]);
        // 省略号自己也占位置，行宽不会被撑破。
        for line in &lines {
            let cells: usize = line.chars().map(char_cells).sum();
            assert!(cells <= 4, "{line}");
        }
    }

    #[test]
    fn wrapping_handles_the_silly_inputs() {
        assert!(wrap("", 10, 2).is_empty());
        assert!(wrap("   ", 10, 2).is_empty());
        assert!(wrap("字", 0, 2).is_empty());
        assert!(wrap("字", 10, 0).is_empty());
        // 一个词比一行还长：硬断，不让它溢出卡面。
        assert_eq!(
            wrap("abcdefgh", 3, 3),
            vec!["abc".to_string(), "def".into(), "gh".into()]
        );
    }

    #[test]
    fn the_card_says_who_invites_you_and_nothing_about_a_coupon() {
        let live = SiteLive::empty("brisk-otter-41");
        let svg = card(&manifest(), &live, Shape::Portrait).svg();
        assert!(svg.starts_with(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1080\" height=\"1350\""
        ));
        assert!(svg.contains("某某 邀请你试玩"));
        assert!(svg.contains("《小球大冒险》"));
        assert!(svg.contains("v7 · 9 月 9 日"));
        assert!(svg.contains("三关，五分钟，手机上也能玩。"));
        // 撕票线与二维码。
        assert!(svg.contains("stroke-dasharray"));
        assert!(svg.contains("<path d=\"M"));
        // DESIGN §3.4 的硬线：卡上没有人数、没有兑换券口吻、没有品类标签。
        for forbidden in [
            "人玩过",
            "人加入",
            "限时",
            "领取",
            "立即",
            "抢",
            "免费",
            "游戏",
            "体验版",
            "优惠",
        ] {
            assert!(!svg.contains(forbidden), "卡上不该有「{forbidden}」");
        }
    }

    #[test]
    fn seats_show_up_but_never_how_many_joined() {
        let mut live = SiteLive::empty("brisk-otter-41");
        live.seats = Some(10);
        live.joined = 6;
        let svg = card(&manifest(), &live, Shape::Portrait).svg();
        assert!(svg.contains("在找 10 位试玩者"));
        // 已加入几位只在门禁页上说。卡是邀请函，不是进度条——「还差 4 位」是催促的口吻。
        for forbidden in ["已有", "6 位", "6/10", "6 / 10"] {
            assert!(!svg.contains(forbidden), "卡上不该有「{forbidden}」");
        }
    }

    #[test]
    fn article_and_video_cards_name_the_real_action_and_audience() {
        let mut live = SiteLive::empty("brisk-otter-41");
        live.seats = Some(10);
        let mut m = manifest();
        m.engine = None;

        m.kind = WorkKind::Article;
        let article = card(&m, &live, Shape::Portrait).svg();
        assert!(article.contains("某某 邀请你阅读"));
        assert!(article.contains("在找 10 位读者"));

        m.kind = WorkKind::Video;
        let video = card(&m, &live, Shape::Portrait).svg();
        assert!(video.contains("某某 邀请你观看"));
        assert!(video.contains("在找 10 位观众"));
    }

    #[test]
    fn an_anonymous_link_says_when_the_invitation_runs_out() {
        let mut m = manifest();
        m.expires_at = Some("2026-09-10T12:59:00Z".into());
        let live = SiteLive::empty("brisk-otter-41");
        let svg = card(&m, &live, Shape::Portrait).svg();
        assert!(svg.contains("这张邀请到 9 月 10 日 20:59"));
        assert!(!svg.contains("v7"));
    }

    #[test]
    fn pro_takes_the_corner_off() {
        let mut m = manifest();
        let live = SiteLive::empty("brisk-otter-41");
        assert!(card(&m, &live, Shape::Portrait)
            .svg()
            .contains(">localhost</text>"));
        m.badge = false;
        let svg = card(&m, &live, Shape::Portrait).svg();
        assert!(!svg.contains(">localhost</text>"));
        // 但链接本身还在票根上——那不是角标，是这张卡要带的东西。
        assert!(svg.contains("brisk-otter-41.localhost:8443"));
    }

    #[test]
    fn no_cover_means_a_word_card_not_a_fake_screenshot() {
        let live = SiteLive::empty("brisk-otter-41");
        let svg = card(&manifest(), &live, Shape::Portrait).svg();
        assert!(!svg.contains("<image"));
        assert!(svg.contains("url(#word)"));
        // 字卡上是一个记号，不是把作品名再写一遍——整张卡上作品名只出现一次。
        assert_eq!(svg.matches("小球大冒险").count(), 1);
        assert!(svg.contains(">小</text>"));

        let png = [0x89u8, b'P', b'N', b'G'];
        let m = manifest();
        let mut with_cover = card(&m, &live, Shape::Portrait);
        with_cover.cover = Some(Cover {
            mime: "image/png",
            bytes: &png,
        });
        let svg = with_cover.svg();
        assert!(svg.contains("href=\"data:image/png;base64,iVBORw==\""));
        assert!(svg.contains("preserveAspectRatio=\"xMidYMid slice\""));
    }

    #[test]
    fn the_qr_points_at_the_site_with_the_source_on_it() {
        // 扫卡进来的人要在点名册里显示「来自邀请卡」（DESIGN §3.5）。
        let live = SiteLive::empty("brisk-otter-41");
        let m = manifest();
        let c = card(&m, &live, Shape::Portrait);
        let door = playtest_common::door_url(c.origin, &m.slug);
        assert_eq!(
            card_qr_url(&door),
            format!(
                "http://localhost:8443/p/brisk-otter-41?from={}",
                playtest_common::FROM_CARD
            )
        );
        assert!(c.svg().contains("fill=\"#0b0d12\""));
    }

    #[test]
    fn everything_from_the_manifest_is_escaped() {
        let mut m = manifest();
        m.title = "<script>alert(1)</script>".into();
        m.developer = "\"><script>".into();
        m.summary = Some("</text><script>".into());
        let live = SiteLive::empty("brisk-otter-41");
        let svg = card(&m, &live, Shape::Portrait).svg();
        assert!(!svg.contains("<script"));
        assert!(svg.contains("&lt;script&gt;"));
    }

    #[test]
    fn the_wide_one_is_the_same_card_lying_down() {
        let live = SiteLive::empty("brisk-otter-41");
        let svg = card(&manifest(), &live, Shape::Wide).svg();
        assert!(svg.contains("width=\"1200\" height=\"630\""));
        assert!(svg.contains("某某 邀请你试玩"));
        assert!(svg.contains("《小球大冒险》"));
        assert!(svg.contains("v7 · 9 月 9 日"));
        // 竖版的撕票线横着撕，横版的竖着撕，二维码在右侧。
        assert!(svg.contains("<line x1=\"560\""));
        assert!(svg.contains("<rect x=\"1016\""));
    }

    #[test]
    fn a_tunnel_says_online_where_a_version_would_be() {
        let mut m = manifest();
        m.version = 0;
        let live = SiteLive::empty("brisk-otter-41");
        let mut c = card(&m, &live, Shape::Portrait);
        c.version_label = Some("在线");
        let svg = c.svg();
        assert!(svg.contains("在线 · 9 月 9 日"));
        assert!(!svg.contains("v0"));
    }

    #[test]
    fn the_etag_only_moves_when_the_card_does() {
        let mut live = SiteLive::empty("brisk-otter-41");
        let m = manifest();
        let first = card(&m, &live, Shape::Portrait).etag();
        assert_eq!(first, card(&m, &live, Shape::Portrait).etag());
        assert_ne!(first, card(&m, &live, Shape::Wide).etag());

        live.joined = 1;
        assert_ne!(first, card(&m, &live, Shape::Portrait).etag());
        // 关注数不上卡，它变了卡不用重出。
        let mut quiet = SiteLive::empty("brisk-otter-41");
        quiet.followers = 99;
        assert_eq!(first, card(&m, &quiet, Shape::Portrait).etag());
    }

    #[test]
    fn hsl_matches_the_plaza_word_card_colours() {
        assert_eq!(hsl_hex(0, 1.0, 0.5), "#ff0000");
        assert_eq!(hsl_hex(120, 1.0, 0.5), "#00ff00");
        assert_eq!(hsl_hex(240, 1.0, 0.5), "#0000ff");
        assert_eq!(hsl_hex(0, 0.0, 0.0), "#000000");
    }

    #[test]
    fn renders_a_real_png_of_the_right_size() {
        let renderer = Renderer::new();
        let live = SiteLive::empty("brisk-otter-41");
        for shape in [Shape::Portrait, Shape::Wide] {
            let png = renderer.render(&card(&manifest(), &live, shape)).unwrap();
            assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "{shape:?} 不是 PNG");
            let (w, h) = shape.size();
            // IHDR：8 字节魔数 + 4 长度 + 4 类型，然后是宽高各四个字节的大端。
            assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), w);
            assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), h);
        }
    }

    #[test]
    fn the_same_card_is_only_rendered_once() {
        let renderer = Renderer::new();
        let live = SiteLive::empty("brisk-otter-41");
        let m = manifest();
        let first = renderer.render(&card(&m, &live, Shape::Portrait)).unwrap();
        assert!(renderer
            .cached(&card(&m, &live, Shape::Portrait).etag())
            .is_some());
        let second = renderer.render(&card(&m, &live, Shape::Portrait)).unwrap();
        // 同一片字节，不是又渲染了一遍。
        assert_eq!(first.as_ptr(), second.as_ptr());
    }
}
