//! 版本清单：一个不可变的 `vN`。
//!
//! 边缘按它服务：请求路径 → 找到 [`FileEntry`] → 按哈希取对象。门禁页要显示的
//! 东西（作品名、开发者、版本、这版改了什么、是否到期）也在这里，边缘因此不必问控制面。

use serde::{Deserialize, Serialize};

use crate::limits;

/// 当前清单格式的版本号。不兼容的改动才加一。
pub const SCHEMA: u32 = 1;

/// 门禁页出现的策略（DESIGN §3.3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
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
                "门禁页策略只能是 once、always 或 never，不认识「{other}」"
            )),
        }
    }
}

/// 清单里的一个文件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    /// 相对于上传目录的路径。正斜杠分隔，不以斜杠开头，没有 `.` / `..` / 空段。
    /// 例：`index.html`、`Build/game.wasm.br`。
    pub path: String,
    /// 内容哈希，见 [`crate::hash`]。
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// 按 `path` 排序，路径唯一。
    pub files: Vec<FileEntry>,
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
        self.files.iter().map(|f| f.size).sum()
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
    #[error("路径为空")]
    Empty,
    #[error("路径不能以「/」开头：{0}")]
    LeadingSlash(String),
    #[error("路径不能用反斜杠，请用「/」：{0}")]
    Backslash(String),
    #[error("路径里有空段、「.」或「..」：{0}")]
    BadSegment(String),
    #[error("路径含控制字符：{0}")]
    ControlChar(String),
    #[error("路径太长（超过 {max} 字节）：{path}")]
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
    #[error("路径重复：{0}")]
    DuplicatePath(String),
    #[error("哈希格式不对：{0}")]
    BadHash(String),
    #[error("文件太多：{count} 个，上限 {max} 个")]
    TooManyFiles { count: usize, max: usize },
    #[error("单个文件太大：{path} 有 {size} 字节，上限 {max} 字节")]
    FileTooLarge { path: String, size: u64, max: u64 },
    #[error("这个版本总共 {total} 字节，上限 {max} 字节")]
    VersionTooLarge { total: u64, max: u64 },
    #[error("清单格式版本 {0} 不认识")]
    UnknownSchema(u32),
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
    validate_files(&m.files, max_total)
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
            created_at: "2026-09-07T00:00:00Z".into(),
            expires_at: None,
            badge: true,
            gate: GateMode::Once,
            isolated: false,
            spa: false,
            engine: None,
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
        // 没有引擎就不写这个字段，旧边缘读新清单也不会多出东西。
        assert!(!serde_json::to_string(&m).unwrap().contains("engine"));
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
}
