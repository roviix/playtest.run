//! 版本清单：一个不可变的 `vN`。
//!
//! 边缘按它服务：请求路径 → 找到 [`FileEntry`] → 按哈希取对象。门禁页要显示的
//! 东西（作品名、开发者、版本、这版改了什么、是否到期）也在这里，边缘因此不必问控制面。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::limits;

/// 当前清单格式的版本号。不兼容的改动才加一。
pub const SCHEMA: u32 = 1;

/// 作品怎样被体验。旧清单没有这一项，必须继续按网页读取。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkKind {
    #[default]
    Web,
    Article,
    Video,
}

impl WorkKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::Article => "article",
            Self::Video => "video",
        }
    }
}

impl std::str::FromStr for WorkKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "web" => Ok(Self::Web),
            "article" => Ok(Self::Article),
            "video" => Ok(Self::Video),
            other => Err(format!("unknown project kind: {other}")),
        }
    }
}

/// 门禁页出现的策略（DESIGN §3.3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum GateMode {
    /// 一次点开在 24 小时内不再重复出现。默认。
    #[default]
    Once,
    /// 每次都出，正式测试要认人。
    Always,
    /// 不出。
    Never,
}

impl std::str::FromStr for GateMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "once" => Ok(Self::Once),
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            other => Err(format!(
                "The invitation page policy has to be once, always or never, not {other}"
            )),
        }
    }
}

/// 清单里的一个文件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FileEntry {
    /// 相对于上传目录的路径。正斜杠分隔，不以斜杠开头，没有 `.` / `..` / 空段。
    /// 例：`index.html`、`Build/game.wasm.br`。
    pub path: String,
    /// 内容哈希，见 [`crate::hash`]。
    pub hash: String,
    pub size: u64,
}

/// 封面（DESIGN §3.3）。**不在 `files` 里**：它不是开发者目录的一部分，是清单单独的一条引用，
/// 边缘在 `/_playtest/cover` 提供。这样开发者目录的字节一个不多一个不少（§3.7 最后一条）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Cover {
    pub hash: String,
    pub size: u64,
    /// `image/png`、`image/jpeg` 或 `image/webp`，见 [`COVER_MIMES`]。
    pub mime: String,
}

/// 文章在控制面提交时生成的安全 HTML。它不是作者上传目录的一部分，也没有可猜的公开路径；
/// 根域邀请函按当前清单读这一个内容哈希。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ArticleArtifact {
    pub hash: String,
    pub size: u64,
}

/// 连载小说的有序章节（DESIGN §3.17）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ChapterEntry {
    /// 稳定章节标识（如 `c1`、`c2`），在作品生命周期内不可变。
    pub id: String,
    /// 章节标题（如「第一章：沉睡的三百年」）。
    pub title: String,
    /// 原稿文件在 `files` 里的路径（如 `01-wake.md`）。
    pub path: String,
    /// 章节生成的安全展示产物哈希。
    pub hash: String,
    pub size: u64,
}

/// 封面认这三种。SVG 不收：它能带脚本，而封面会被贴到根域那一页上。
pub const COVER_MIMES: &[&str] = &["image/png", "image/jpeg", "image/webp"];

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CoverError {
    #[error("the cover hash is not in a valid format")]
    BadHash,
    #[error("a cover has to be PNG, JPEG or WebP, and this one is {0}")]
    BadMime(String),
    #[error("a cover can be at most {max} MB, and this one is {size} bytes")]
    TooLarge { size: u64, max: u64 },
    #[error("the cover is empty")]
    Empty,
}

/// 校验一条封面引用的形态。CLI 选文件时调用，api 收到清单时再调用一次。
pub fn validate_cover(cover: &Cover) -> Result<(), CoverError> {
    if !crate::hash::is_valid_hex(&cover.hash) {
        return Err(CoverError::BadHash);
    }
    if !COVER_MIMES.contains(&cover.mime.as_str()) {
        return Err(CoverError::BadMime(cover.mime.clone()));
    }
    if cover.size == 0 {
        return Err(CoverError::Empty);
    }
    if cover.size > limits::MAX_COVER_BYTES {
        return Err(CoverError::TooLarge {
            size: cover.size,
            max: limits::MAX_COVER_BYTES / limits::MIB,
        });
    }
    Ok(())
}

/// 从文件开头几个字节认出图片类型。扩展名会骗人，魔数不会。
pub fn sniff_image_mime(head: &[u8]) -> Option<&'static str> {
    if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if head.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else if head.len() >= 12 && &head[0..4] == b"RIFF" && &head[8..12] == b"WEBP" {
        Some("image/webp")
    } else if head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a") {
        Some("image/gif")
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Manifest {
    pub schema: u32,
    pub slug: String,
    /// 从 1 开始，每次上传加一。
    pub version: u32,
    /// 作品名，门禁页标题用。没给就用目录名。
    pub title: String,
    /// 开发者显示名。匿名链接是「匿名开发者」。
    pub developer: String,
    /// 这版改了什么，可选，门禁页显示。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// 一句话介绍这个作品是什么（DESIGN §3.8），门禁页、分享卡片、广场卡片都用。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// 封面，见 [`Cover`]。没有就没有 `og:image`（DESIGN §3.3）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<Cover>,
    /// RFC 3339。
    pub created_at: String,
    /// 匿名链接到期时间，RFC 3339；登录用户的作品没有。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// 门禁页底部是否带「由 playtest.run 提供」角标（免费档与匿名为 true）。
    #[serde(default)]
    pub badge: bool,
    #[serde(default)]
    pub gate: GateMode,
    /// 边缘加 COOP / COEP 响应头（Godot 4 线程导出需要 SharedArrayBuffer）。
    #[serde(default)]
    pub isolated: bool,
    /// 找不到的路径回退到 `index.html`。
    #[serde(default)]
    pub spa: bool,
    /// 上传时认出来的引擎或框架，小写标识符，认不出来就没有这个字段。
    /// 门禁页拿它决定说「试玩」还是「体验」（DESIGN §3.3）。
    /// 已知值见 [`GAME_ENGINES`] 与 `vite`。旧清单没有这个字段，按 `None` 解析。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    /// 网页 / 文章 / 视频。旧清单没有，按网页处理。
    #[serde(default, skip_serializing_if = "is_web")]
    pub kind: WorkKind,
    /// 文章原稿或视频文件在 `files` 里的路径。网页没有固定入口，仍按 `index.html`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    /// 只有文章有：控制面从 Markdown 重新生成的安全展示产物。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub article: Option<ArticleArtifact>,
    /// 连载小说的有序章节列表（DESIGN §3.17）。为空表示普通单篇文章。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chapters: Vec<ChapterEntry>,
    /// 按 `path` 排序，路径唯一。
    pub files: Vec<FileEntry>,
}

fn is_web(kind: &WorkKind) -> bool {
    *kind == WorkKind::Web
}

/// 认出这些就把作品当游戏说（DESIGN §3.3）。
///
/// 判据是「这个工具链是拿来做能玩的东西的」，不是「它有多像引擎」。落在名单外的一律
/// 说「体验」：`vite` 只说明作品是打包出来的，用 AI 写小东西的人手上多数是工具、
/// 生成器、微型 SaaS（DESIGN §1.1），管它们叫「试玩」是把话说错了。
/// 不认识的值同样按「体验」——CLI 将来加了新引擎而边缘还没升级时，
/// 宁可少说一句，也不要对着一个数据看板说「邀请你试玩」。
pub const GAME_ENGINES: &[&str] = &[
    "godot",
    "unity",
    "phaser",
    "cocos",
    "construct",
    "gdevelop",
    "pico8",
    "twine",
    "bitsy",
    "three",
    "p5",
    "pixi",
];

/// 引擎标识符最长多少字节。写清单的是我们自己的 CLI，但请求是外面来的，进清单之前先掐一道。
const MAX_ENGINE_BYTES: usize = 32;

/// 把请求里带来的引擎名字收拾干净再写进清单：小写字母、数字、`-`，短。
///
/// 形状不对就当没说（`None`，也就是「体验」），不报错——认引擎是锦上添花，
/// 不该因为它把一次上传挡回去。
pub fn clean_engine(raw: Option<&str>) -> Option<String> {
    let name = raw?.trim();
    let shaped = !name.is_empty()
        && name.len() <= MAX_ENGINE_BYTES
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    shaped.then(|| name.to_string())
}

impl Manifest {
    pub fn total_bytes(&self) -> u64 {
        self.files
            .iter()
            .map(|f| f.size)
            .sum::<u64>()
            .saturating_add(self.article.as_ref().map(|a| a.size).unwrap_or_default())
            .saturating_add(self.chapters.iter().map(|c| c.size).sum::<u64>())
    }

    /// 是否为连载多章作品。
    pub fn is_serial(&self) -> bool {
        self.kind == WorkKind::Article && !self.chapters.is_empty()
    }

    /// 按章节 id 查找章节及其在目录中的顺序。
    pub fn find_chapter(&self, chapter_id: &str) -> Option<(usize, &ChapterEntry)> {
        self.chapters
            .iter()
            .enumerate()
            .find(|(_, c)| c.id == chapter_id)
    }

    /// 这份作品该不该用「试玩」这个词。见 [`GAME_ENGINES`]。
    pub fn is_game(&self) -> bool {
        self.engine
            .as_deref()
            .is_some_and(|e| GAME_ENGINES.contains(&e))
    }

    /// 按精确路径找文件。
    pub fn find(&self, path: &str) -> Option<&FileEntry> {
        self.files.iter().find(|f| f.path == path)
    }

    pub fn has_index(&self) -> bool {
        self.find("index.html").is_some()
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PathError {
    #[error("the path is empty")]
    Empty,
    #[error("a path can not start with a slash: {0}")]
    LeadingSlash(String),
    #[error("a path can not use backslashes; use forward slashes: {0}")]
    Backslash(String),
    #[error("this path has an empty segment, a dot or a double dot: {0}")]
    BadSegment(String),
    #[error("this path contains control characters: {0}")]
    ControlChar(String),
    #[error("this path is longer than {max} bytes: {path}")]
    TooLong { path: String, max: usize },
}

/// 校验清单里一条路径的形态。CLI 在打包时调用，api 在收到清单时再调用一次。
pub fn validate_path(path: &str) -> Result<(), PathError> {
    if path.is_empty() {
        return Err(PathError::Empty);
    }
    if path.len() > limits::MAX_PATH_BYTES {
        return Err(PathError::TooLong {
            path: path.to_string(),
            max: limits::MAX_PATH_BYTES,
        });
    }
    if path.starts_with('/') {
        return Err(PathError::LeadingSlash(path.to_string()));
    }
    if path.contains('\\') {
        return Err(PathError::Backslash(path.to_string()));
    }
    if path.chars().any(|c| c.is_control()) {
        return Err(PathError::ControlChar(path.to_string()));
    }
    if path
        .split('/')
        .any(|seg| seg.is_empty() || seg == "." || seg == "..")
    {
        return Err(PathError::BadSegment(path.to_string()));
    }
    Ok(())
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ManifestError {
    #[error(transparent)]
    Path(#[from] PathError),
    #[error("duplicate path: {0}")]
    DuplicatePath(String),
    #[error("this hash is not in a valid format: {0}")]
    BadHash(String),
    #[error("too many files: {count}, and the limit is {max}")]
    TooManyFiles { count: usize, max: usize },
    #[error("this file is too large: {path} is {size} bytes, and the limit is {max} bytes")]
    FileTooLarge { path: String, size: u64, max: u64 },
    #[error("this version is {total} bytes in total, and the limit is {max} bytes")]
    VersionTooLarge { total: u64, max: u64 },
    #[error("unknown manifest schema version: {0}")]
    UnknownSchema(u32),
    #[error("the project kind does not match its files: {0}")]
    BadPresentation(String),
}

/// 校验一组文件条目：路径形态、唯一性、哈希形态、数量与体积。
/// `max_total` 由调用方按档位传入（匿名与登录不同）。
pub fn validate_files(files: &[FileEntry], max_total: u64) -> Result<(), ManifestError> {
    if files.len() > limits::MAX_FILES_PER_VERSION {
        return Err(ManifestError::TooManyFiles {
            count: files.len(),
            max: limits::MAX_FILES_PER_VERSION,
        });
    }
    let mut seen = std::collections::HashSet::with_capacity(files.len());
    let mut total: u64 = 0;
    for f in files {
        validate_path(&f.path)?;
        if !seen.insert(f.path.as_str()) {
            return Err(ManifestError::DuplicatePath(f.path.clone()));
        }
        if !crate::hash::is_valid_hex(&f.hash) {
            return Err(ManifestError::BadHash(f.hash.clone()));
        }
        if f.size > limits::MAX_FILE_BYTES {
            return Err(ManifestError::FileTooLarge {
                path: f.path.clone(),
                size: f.size,
                max: limits::MAX_FILE_BYTES,
            });
        }
        total = total.saturating_add(f.size);
    }
    if total > max_total {
        return Err(ManifestError::VersionTooLarge {
            total,
            max: max_total,
        });
    }
    Ok(())
}

/// 完整校验一份清单（api 提交前、edge 读取后都可以调）。
pub fn validate_manifest(m: &Manifest, max_total: u64) -> Result<(), ManifestError> {
    if m.schema != SCHEMA {
        return Err(ManifestError::UnknownSchema(m.schema));
    }
    validate_files(&m.files, max_total)?;
    match m.kind {
        WorkKind::Web => {
            if m.entry.is_some() || m.article.is_some() {
                return Err(ManifestError::BadPresentation(
                    "a web project can not carry an article or video entry".into(),
                ));
            }
        }
        WorkKind::Article => {
            let entry = m.entry.as_deref().ok_or_else(|| {
                ManifestError::BadPresentation("the article has no Markdown source entry".into())
            })?;
            if !entry.to_ascii_lowercase().ends_with(".md") || m.find(entry).is_none() {
                return Err(ManifestError::BadPresentation(
                    "the article entry has to point at a .md file in this version".into(),
                ));
            }
            let article = m.article.as_ref().ok_or_else(|| {
                ManifestError::BadPresentation("the article has no safely rendered output".into())
            })?;
            if !crate::hash::is_valid_hex(&article.hash)
                || article.size == 0
                || article.size > limits::MAX_ARTICLE_HTML_BYTES
            {
                return Err(ManifestError::BadPresentation(
                    "the rendered article output has an invalid hash or size".into(),
                ));
            }
        }
        WorkKind::Video => {
            let entry = m.entry.as_deref().ok_or_else(|| {
                ManifestError::BadPresentation("the video has no MP4 entry".into())
            })?;
            if !entry.to_ascii_lowercase().ends_with(".mp4") || m.find(entry).is_none() {
                return Err(ManifestError::BadPresentation(
                    "the video entry has to point at an .mp4 file in this version".into(),
                ));
            }
            if m.files.len() != 1 || m.article.is_some() {
                return Err(ManifestError::BadPresentation(
                    "a video project currently takes exactly one MP4 file; the cover is uploaded separately".into(),
                ));
            }
        }
    }
    if m.total_bytes() > max_total {
        return Err(ManifestError::VersionTooLarge {
            total: m.total_bytes(),
            max: max_total,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, size: u64) -> FileEntry {
        FileEntry {
            path: path.to_string(),
            hash: crate::hash::hash_bytes(path.as_bytes()),
            size,
        }
    }

    #[test]
    fn accepts_normal_paths() {
        for p in [
            "index.html",
            "Build/game.wasm.br",
            "assets/a b.png",
            "深/中文.txt",
        ] {
            assert_eq!(validate_path(p), Ok(()), "{p}");
        }
    }

    #[test]
    fn rejects_dangerous_paths() {
        assert_eq!(validate_path(""), Err(PathError::Empty));
        assert!(matches!(
            validate_path("/index.html"),
            Err(PathError::LeadingSlash(_))
        ));
        assert!(matches!(
            validate_path("a\\b"),
            Err(PathError::Backslash(_))
        ));
        assert!(matches!(
            validate_path("a//b"),
            Err(PathError::BadSegment(_))
        ));
        assert!(matches!(
            validate_path("../etc/passwd"),
            Err(PathError::BadSegment(_))
        ));
        assert!(matches!(
            validate_path("a/./b"),
            Err(PathError::BadSegment(_))
        ));
        assert!(matches!(
            validate_path("a\nb"),
            Err(PathError::ControlChar(_))
        ));
    }

    #[test]
    fn rejects_duplicates_and_size_overflow() {
        let files = vec![entry("a", 1), entry("a", 1)];
        assert!(matches!(
            validate_files(&files, 1000),
            Err(ManifestError::DuplicatePath(_))
        ));

        let files = vec![entry("a", 600), entry("b", 600)];
        assert!(matches!(
            validate_files(&files, 1000),
            Err(ManifestError::VersionTooLarge { total: 1200, .. })
        ));
    }

    #[test]
    fn engine_decides_whether_we_say_playtest() {
        let mut m = Manifest {
            schema: SCHEMA,
            slug: "brisk-otter-41".into(),
            version: 1,
            title: "测试".into(),
            developer: "某某".into(),
            note: None,
            summary: None,
            cover: None,
            created_at: "2026-09-07T00:00:00Z".into(),
            expires_at: None,
            badge: true,
            gate: GateMode::Once,
            isolated: false,
            spa: false,
            engine: None,
            kind: WorkKind::Web,
            entry: None,
            article: None,
            chapters: vec![],
            files: vec![],
        };
        assert!(!m.is_game(), "认不出引擎时说「体验」");
        for engine in GAME_ENGINES {
            m.engine = Some((*engine).to_string());
            assert!(m.is_game(), "{engine}");
        }
        // 构建工具说明不了作品是什么；不认识的值也一样按「体验」。
        for other in ["vite", "webpack", "Godot", ""] {
            m.engine = Some(other.to_string());
            assert!(!m.is_game(), "{other}");
        }
    }

    #[test]
    fn old_manifests_without_engine_still_parse() {
        // 三个进程不会同时升级：边缘新、对象存储里还是旧清单，必须照常能开。
        let raw = r#"{"schema":1,"slug":"brisk-otter-41","version":7,"title":"小球大冒险",
            "developer":"某某","created_at":"2026-09-07T00:00:00Z","badge":true,
            "gate":"once","isolated":false,"spa":false,"files":[]}"#;
        let m: Manifest = serde_json::from_str(raw).unwrap();
        assert_eq!(m.engine, None);
        assert!(!m.is_game());
        assert_eq!(m.summary, None);
        assert_eq!(m.cover, None);
        // 没有引擎就不写这个字段，旧边缘读新清单也不会多出东西。
        let json = serde_json::to_string(&m).unwrap();
        assert!(!json.contains("engine"));
        assert!(!json.contains("cover"));
        assert!(!json.contains("summary"));
    }

    #[test]
    fn cover_must_be_a_small_raster_image() {
        let ok = Cover {
            hash: "a".repeat(64),
            size: 120_000,
            mime: "image/png".into(),
        };
        assert_eq!(validate_cover(&ok), Ok(()));
        for mime in ["image/jpeg", "image/webp"] {
            assert_eq!(
                validate_cover(&Cover {
                    mime: mime.into(),
                    ..ok.clone()
                }),
                Ok(()),
                "{mime}"
            );
        }
        // SVG 能带脚本，而封面会贴到根域那一页上。
        assert!(matches!(
            validate_cover(&Cover {
                mime: "image/svg+xml".into(),
                ..ok.clone()
            }),
            Err(CoverError::BadMime(_))
        ));
        assert!(matches!(
            validate_cover(&Cover {
                size: limits::MAX_COVER_BYTES + 1,
                ..ok.clone()
            }),
            Err(CoverError::TooLarge { .. })
        ));
        assert_eq!(
            validate_cover(&Cover {
                size: 0,
                ..ok.clone()
            }),
            Err(CoverError::Empty)
        );
        assert_eq!(
            validate_cover(&Cover {
                hash: "zz".into(),
                ..ok
            }),
            Err(CoverError::BadHash)
        );
    }

    #[test]
    fn image_type_comes_from_the_bytes_not_the_name() {
        assert_eq!(
            sniff_image_mime(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR"),
            Some("image/png")
        );
        assert_eq!(
            sniff_image_mime(b"\xff\xd8\xff\xe0\0\x10JFIF"),
            Some("image/jpeg")
        );
        assert_eq!(
            sniff_image_mime(b"RIFF\x24\x00\x00\x00WEBPVP8 "),
            Some("image/webp")
        );
        assert_eq!(sniff_image_mime(b"<svg xmlns"), None);
        assert_eq!(sniff_image_mime(b"GIF89a"), Some("image/gif"));
        assert_eq!(sniff_image_mime(b""), None);
    }

    #[test]
    fn gate_mode_round_trips_lowercase() {
        assert_eq!(
            serde_json::to_string(&GateMode::Always).unwrap(),
            "\"always\""
        );
        assert_eq!("never".parse::<GateMode>(), Ok(GateMode::Never));
        assert!("Sometimes".parse::<GateMode>().is_err());
    }

    #[test]
    fn chapters_serialize_and_detect_serial_works() {
        let mut m = Manifest {
            schema: SCHEMA,
            slug: "star-pilot".into(),
            version: 1,
            title: "星轨漫游".into(),
            developer: "探险家".into(),
            note: None,
            summary: None,
            cover: None,
            created_at: "2026-09-13T00:00:00Z".into(),
            expires_at: None,
            badge: false,
            gate: GateMode::Once,
            isolated: false,
            spa: false,
            engine: None,
            kind: WorkKind::Web,
            entry: None,
            article: None,
            chapters: vec![],
            files: vec![],
        };
        assert!(!m.is_serial());
        m.kind = WorkKind::Article;
        m.chapters = vec![
            ChapterEntry {
                id: "c1".into(),
                title: "第一章：起航".into(),
                path: "01.md".into(),
                hash: "0000000000000000000000000000000000000000000000000000000000000001".into(),
                size: 100,
            },
            ChapterEntry {
                id: "c2".into(),
                title: "第二章：迷途".into(),
                path: "02.md".into(),
                hash: "0000000000000000000000000000000000000000000000000000000000000002".into(),
                size: 200,
            },
        ];
        assert!(m.is_serial());
        assert_eq!(m.find_chapter("c2").unwrap().0, 1);
        assert_eq!(m.find_chapter("c2").unwrap().1.title, "第二章：迷途");
        assert!(m.find_chapter("c3").is_none());

        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"chapters\":["));
        let parsed: Manifest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.chapters.len(), 2);
        assert_eq!(parsed.chapters[0].title, "第一章：起航");
    }
}
