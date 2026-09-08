//! 上传那一刻的检查。
//!
//! 这时候我们手上有全部文件的字节，能查的就都查了：这个构建要不要跨源隔离、`index.html`
//! 在不在最外层、它引用的文件缺不缺、Unity 那两条互斥的压缩路撞没撞、是不是把整个工程
//! 当成导出物传了（DESIGN §4.2）。
//!
//! 这里只看出来，不打印，也不自己决定加不加参数——那两件事在 `upload.rs`。产出是一串
//! [`Finding`]，人类模式一条一行走 stderr，`--json` 模式进 `findings` 数组，两边同一份。
//!
//! 一条纪律：**会改上传行为的判断要有硬证据**。要不要 `--isolated` 以读 `.wasm` 的字节为准
//! （[`wasm::memory_kind`]），读不出来才退回「脚本里提到 SharedArrayBuffer」这种猜测，
//! 并且在话里说清楚那是猜的。认引擎不改行为，认错了也只是少说一句。

mod engine;
mod html;
mod wasm;

use std::collections::HashSet;

use playtest_common::limits::MIB;
use playtest_common::manifest::{FileEntry, ManifestError};
use serde::Serialize;

use crate::output::Finding;

pub use engine::Engine;

/// 为了认引擎，一个脚本最多读开头多少字节。引擎的名字都印在很前面（Phaser 3.60 的包在
/// 75 KB 处），2 MB 已经很宽。
const SCRIPT_PREFIX_BYTES: usize = 2 * MIB as usize;

/// 最多翻几个脚本。导出物的引导脚本就那么几个；几百个 `.js` 的多半是源码目录，翻也没用。
const MAX_SCRIPTS: usize = 12;

/// 读 `.wasm` 的开头多少字节。要看的段（import、memory）都在文件最前面，没必要读全——
/// Godot 和 Unity 的 wasm 动辄几十 MB。
const WASM_PREFIX_BYTES: usize = MIB as usize;

/// 最多读几个 `.wasm`。
const MAX_WASM: usize = 4;

/// 缺的文件最多列几个，多了刷屏。
const MAX_MISSING_SHOWN: usize = 5;

/// 按清单路径读一个文件开头的若干字节。读不到（不在、读不了）返回 `None`。
pub type ReadPrefix<'a> = &'a mut dyn FnMut(&str, usize) -> Option<Vec<u8>>;

/// 要检查的东西。
pub struct Input<'a> {
    /// 会真的传上去的文件。
    pub files: &'a [FileEntry],
    /// 目录里有 `.git/`。以「.」开头的目录不上传，所以清单里看不到它，得调用方单独说一声。
    pub has_git_dir: bool,
}

/// 这个构建要多线程，是怎么看出来的。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "from", content = "path")]
pub enum Threads {
    /// 直接从 wasm 的字节里读出了多个线程共享的内存。这条是准的。
    SharedMemory(String),
    /// 只是在脚本里看到 `SharedArrayBuffer` 这个词——wasm 压缩过、不在、或者读下来没这回事，
    /// 只能这么猜。
    ScriptMentions(String),
}

impl Threads {
    /// 「为什么这么说」，跟在提示后面的括号里。
    pub fn because(&self) -> String {
        match self {
            Self::SharedMemory(path) => format!("{path} 里申请了多个线程共享的内存"),
            Self::ScriptMentions(path) => {
                format!("{path} 里提到 SharedArrayBuffer，但 wasm 的字节里没看出来，这是猜的")
            }
        }
    }

    /// 是硬证据还是猜的。
    pub fn is_certain(&self) -> bool {
        matches!(self, Self::SharedMemory(_))
    }
}

/// 检查的结论。
#[derive(Debug, Default, Serialize)]
pub struct Report {
    /// 认出来的引擎。
    pub engine: Option<Engine>,
    /// 要跨源隔离（也就是 `--isolated`）才跑得起来；`None` 表示没看出这个需要。
    pub threads: Option<Threads>,
    pub findings: Vec<Finding>,
}

impl Report {
    /// 写进清单的引擎名字。门禁页拿它决定说「试玩」还是「体验」（DESIGN §3.3）。
    pub fn manifest_engine(&self) -> Option<String> {
        self.engine.map(|e| e.manifest_name().to_string())
    }
}

/// 把一个待上传的目录从头到尾看一遍。
pub fn inspect(input: Input<'_>, read: ReadPrefix<'_>) -> Report {
    let paths: Vec<&str> = input.files.iter().map(|f| f.path.as_str()).collect();
    let here: HashSet<&str> = paths.iter().copied().collect();

    let index = read("index.html", SCRIPT_PREFIX_BYTES).map(|bytes| text_of(&bytes));
    let scripts = read_scripts(&paths, &mut *read);
    let engine = engine::identify(&paths, index.as_deref(), &scripts);
    let threads = find_threads(&paths, index.as_deref(), &scripts, &mut *read);

    let mut findings = Vec::new();
    if let Some(engine) = engine {
        findings.push(Finding::note(format!("看起来是 {} 做的", engine.label())));
    }
    findings.extend(index_placement(&here, &paths));
    if let Some(index) = index.as_deref() {
        findings.extend(missing_references(index, &here));
    }
    findings.extend(missing_engine_parts(&paths, index.as_deref()));
    findings.extend(compression(&paths, index.as_deref(), engine));
    findings.extend(wrong_directory(&paths, input.has_git_dir));

    Report {
        engine,
        threads,
        findings,
    }
}

/// 目录里像是源码而不是导出物吗。文件多得离谱时用它换一句更有用的提示。
pub fn looks_like_source_tree(paths: &[String]) -> bool {
    paths
        .iter()
        .any(|p| p == "package.json" || p.starts_with("node_modules/") || p.starts_with("src/"))
}

/// 配额把上传拦下来时多说的一句：是哪个文件、多大、上限多少。
pub fn explain_limit(error: &ManifestError, paths: &[String]) -> Option<String> {
    match error {
        ManifestError::VersionTooLarge { total, max } => Some(format!(
            "这一版一共 {}，匿名上传一个版本最多 {}。要传更大的得先登录，登录还没做好。",
            megabytes(*total),
            megabytes(*max)
        )),
        ManifestError::FileTooLarge { path, size, max } => Some(format!(
            "{path} 有 {}，单个文件最多 {}。把它拆开导出，或者放到别处再从游戏里去取。",
            megabytes(*size),
            megabytes(*max)
        )),
        ManifestError::TooManyFiles { count, max } if looks_like_source_tree(paths) => Some(format!(
            "{count} 个文件，上限 {max} 个。这看起来是源码目录，不是导出物；先构建（例如 npm run build），再上传 dist/。"
        )),
        ManifestError::TooManyFiles { count, max } => Some(format!(
            "{count} 个文件，上限 {max} 个。只传引擎导出的那个目录，别把整个工程带上。"
        )),
        _ => None,
    }
}

/// 读进来的一个脚本：清单里的路径，加上开头那一段文本。
pub struct Script {
    pub path: String,
    pub text: String,
}

fn read_scripts(paths: &[&str], read: ReadPrefix<'_>) -> Vec<Script> {
    paths
        .iter()
        .filter(|p| p.ends_with(".js") || p.ends_with(".mjs"))
        .take(MAX_SCRIPTS)
        .filter_map(|path| {
            let bytes = read(path, SCRIPT_PREFIX_BYTES)?;
            Some(Script {
                path: (*path).to_string(),
                text: text_of(&bytes),
            })
        })
        .collect()
}

/// 这个构建要不要多线程。先读 wasm 的字节，读不出来才看脚本里的词。
fn find_threads(
    paths: &[&str],
    index: Option<&str>,
    scripts: &[Script],
    read: ReadPrefix<'_>,
) -> Option<Threads> {
    let mut looked = 0;
    for path in paths.iter().filter(|p| p.ends_with(".wasm")) {
        if looked >= MAX_WASM {
            break;
        }
        let Some(bytes) = read(path, WASM_PREFIX_BYTES) else {
            continue;
        };
        looked += 1;
        if wasm::memory_kind(&bytes) == wasm::Memory::Shared {
            return Some(Threads::SharedMemory((*path).to_string()));
        }
    }

    const WORD: &str = "SharedArrayBuffer";
    if index.is_some_and(|text| text.contains(WORD)) {
        return Some(Threads::ScriptMentions("index.html".to_string()));
    }
    scripts
        .iter()
        .find(|script| script.text.contains(WORD))
        .map(|script| Threads::ScriptMentions(script.path.clone()))
}

/// `index.html` 在不在最外层。不在，玩家点开链接就是 404。
fn index_placement(here: &HashSet<&str>, paths: &[&str]) -> Option<Finding> {
    if here.contains("index.html") {
        return None;
    }
    let Some(nested) = paths
        .iter()
        .filter(|p| p.ends_with("/index.html"))
        .min_by_key(|p| p.matches('/').count())
    else {
        return Some(
            Finding::blocker("最外层没有 index.html，玩家点开链接会是 404")
                .hint("引擎导出物一般都有；确认一下给的是不是导出目录"),
        );
    };
    let dir = nested.strip_suffix("/index.html")?;
    Some(
        Finding::blocker(format!(
            "最外层没有 index.html，它在 {nested} 里，玩家点开链接会是 404"
        ))
        .hint(format!(
            "把 {dir} 这一层直接传上来：playtest <刚才那个目录>/{dir}"
        )),
    )
}

/// `index.html` 里引用了、但目录里没有的文件。
fn missing_references(index: &str, here: &HashSet<&str>) -> Option<Finding> {
    let missing: Vec<String> = html::local_references(index)
        .into_iter()
        .filter(|path| !here.contains(path.as_str()))
        .collect();
    if missing.is_empty() {
        return None;
    }
    let shown = missing
        .iter()
        .take(MAX_MISSING_SHOWN)
        .cloned()
        .collect::<Vec<_>>()
        .join("、");
    let rest = missing.len().saturating_sub(MAX_MISSING_SHOWN);
    let tail = if rest > 0 {
        format!("，还有 {rest} 个")
    } else {
        String::new()
    };
    Some(
        Finding::warn(format!(
            "index.html 要用 {} 个目录里没有的文件，玩家那边会加载失败：{shown}{tail}",
            missing.len()
        ))
        .hint("多半是导出时漏了，或者只传了其中一层；对一下引擎输出的那个目录是不是完整的"),
    )
}

/// 引擎那件最大的产物漏了没有：Godot 的 `.pck`、Unity 的 `.data`。文件列表看着挺齐、
/// 一打开却永远卡在加载条上，多半就是这两样（DESIGN §4.2）。
///
/// 只在**确凿**认出这是哪种导出时才说。认引擎那一套宽松到「页面里出现过 Godot 这个词」
/// 就算，用它来判缺文件会冤枉一堆正常页面——比如一篇讲 Godot 的文章。
fn missing_engine_parts(paths: &[&str], index: Option<&str>) -> Option<Finding> {
    let bare = |suffix: &'static str| {
        paths
            .iter()
            .any(move |p| engine::without_compression(p).ends_with(suffix))
    };
    // `.audio.worklet.js` 这个名字只有 Godot 会写；`GODOT_CONFIG` 是它塞进页面的那段配置。
    let godot_export = paths.iter().any(|p| p.ends_with(".audio.worklet.js"))
        || index.is_some_and(|text| text.contains("GODOT_CONFIG"));
    if godot_export && bare(".wasm") && !bare(".pck") {
        return Some(
            Finding::warn("这是 Godot 的网页导出，但目录里没有 .pck——游戏的内容都在那个文件里，玩家会一直卡在加载条上")
                .hint("导出出来的那几个文件要一起传：.html、.js、.wasm、.pck，可能还有 .audio.worklet.js"),
        );
    }
    // `.loader.js` 是 Unity 网页导出一定有的那个文件。
    if bare(".loader.js") && !bare(".data") {
        return Some(
            Finding::warn("这是 Unity 的网页导出，但目录里没有 .data——游戏的资源都在那个文件里，玩家会一直卡在加载条上")
                .hint("把 Build 整个目录传上来：.loader.js、.framework.js、.wasm、.data 四件缺一不可"),
        );
    }
    None
}

/// 压缩相关的话：预压缩产物照原样给；Unity 的 Decompression Fallback 会拖慢加载。
fn compression(paths: &[&str], index: Option<&str>, found: Option<Engine>) -> Vec<Finding> {
    let precompressed = paths
        .iter()
        .any(|p| p.ends_with(".br") || p.ends_with(".gz"));
    let fallback = paths.iter().any(|p| p.ends_with(".unityweb"));
    let mut out = Vec::new();

    if precompressed {
        out.push(Finding::note(
            "目录里有压好的 .br / .gz，会按原样直接给玩家，由浏览器解压",
        ));
    }
    if fallback {
        let seen = if found == Some(Engine::Unity) {
            "Unity 开着 Decompression Fallback"
        } else {
            "目录里有 .unityweb，那是 Unity 开着 Decompression Fallback 才有的"
        };
        out.push(
            Finding::warn(format!(
                "{seen}，加载时由页面里的 JS 自己解压，比浏览器慢一截"
            ))
            .hint(
                "在 Player Settings → Publishing Settings 里关掉它重新导出会更快，Brotli 的响应头我们配好",
            ),
        );
    }
    if precompressed && fallback {
        let pointing = index
            .and_then(|html| html::config_value(html, "dataUrl"))
            .map(|url| {
                let winner = if url.ends_with(".unityweb") {
                    ".unityweb 那一套"
                } else {
                    "压好的 .br / .gz 那一套"
                };
                format!("index.html 里的 dataUrl 指着 {url}，真正会用的是{winner}")
            })
            .unwrap_or_else(|| "index.html 里没找到 dataUrl，看不出用的是哪一套".to_string());
        out.push(
            Finding::warn(format!(
                "两套压缩产物都在：.br / .gz 和 .unityweb。{pointing}"
            ))
            .hint("另一套是白传的，删掉能少传不少字节"),
        );
    }
    out
}

/// 传的是不是整个工程，而不是导出物。
fn wrong_directory(paths: &[&str], has_git_dir: bool) -> Vec<Finding> {
    let mut out = Vec::new();
    if paths.iter().any(|p| p.starts_with("node_modules/")) {
        out.push(
            Finding::warn("目录里有 node_modules/，这看着是源码目录，不是构建出来的导出物")
                .hint("先构建（例如 npm run build），再传 dist/ 那一层"),
        );
    }
    if has_git_dir {
        out.push(
            Finding::note("目录里有 .git/，看着像整个仓库而不是导出目录")
                .hint("以「.」开头的东西不会上传；确认一下要发的是不是构建输出的那一层"),
        );
    }
    out
}

/// 按文本看这段字节。不是 UTF-8 的地方换成替换字符——只在里面找特征词，不回写。
fn text_of(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// `12.3 MB`。配额都是按 MB 说的，这里不跟着换小单位。
fn megabytes(n: u64) -> String {
    format!("{:.1} MB", n as f64 / MIB as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::Level;
    use playtest_common::hash::hash_bytes;
    use std::collections::HashMap;

    /// 一个假目录：路径 → 内容。
    struct Dir {
        files: Vec<FileEntry>,
        bytes: HashMap<String, Vec<u8>>,
    }

    impl Dir {
        fn new(entries: &[(&str, &[u8])]) -> Self {
            Self {
                files: entries
                    .iter()
                    .map(|(path, body)| FileEntry {
                        path: (*path).to_string(),
                        hash: hash_bytes(body),
                        size: body.len() as u64,
                    })
                    .collect(),
                bytes: entries
                    .iter()
                    .map(|(path, body)| ((*path).to_string(), body.to_vec()))
                    .collect(),
            }
        }

        fn inspect(&self) -> Report {
            self.inspect_with_git(false)
        }

        fn inspect_with_git(&self, has_git_dir: bool) -> Report {
            let bytes = self.bytes.clone();
            // 真实的读法也是只读开头：这里跟着截，才测得到「只读了一段」那条路径。
            let mut read = move |path: &str, max: usize| {
                let body = bytes.get(path)?;
                Some(body[..body.len().min(max)].to_vec())
            };
            super::inspect(
                Input {
                    files: &self.files,
                    has_git_dir,
                },
                &mut read,
            )
        }
    }

    fn messages(report: &Report) -> Vec<String> {
        report.findings.iter().map(|f| f.message.clone()).collect()
    }

    /// 找到说了某件事的那一条。
    fn about<'a>(report: &'a Report, needle: &str) -> &'a Finding {
        report
            .findings
            .iter()
            .find(|f| f.message.contains(needle))
            .unwrap_or_else(|| panic!("没有说到「{needle}」，只说了 {:?}", messages(report)))
    }

    /// 一个带共享内存导入的最小 wasm。
    fn threaded_wasm() -> Vec<u8> {
        let body: Vec<u8> = vec![
            0x01, 0x03, b'e', b'n', b'v', 0x06, b'm', b'e', b'm', b'o', b'r', b'y', 0x02, 0x03,
            0x80, 0x02, 0x80, 0x04,
        ];
        let mut out = b"\0asm\x01\x00\x00\x00".to_vec();
        out.push(0x02);
        out.push(body.len() as u8);
        out.extend_from_slice(&body);
        out
    }

    #[test]
    fn a_threaded_godot_export_needs_isolation_and_says_why() {
        let dir = Dir::new(&[
            ("index.html", b"<html>Godot</html>"),
            ("game.pck", b"pack"),
            ("game.wasm", &threaded_wasm()),
        ]);
        let report = dir.inspect();
        assert_eq!(report.engine, Some(Engine::Godot));
        assert_eq!(report.manifest_engine().as_deref(), Some("godot"));
        let threads = report.threads.expect("该看出要多线程");
        assert!(threads.is_certain());
        assert!(threads.because().contains("game.wasm"), "{threads:?}");
    }

    /// 单线程导出不该被误判——误判会白白给所有人加上跨源隔离。
    #[test]
    fn a_single_threaded_export_is_left_alone() {
        let mut plain = b"\0asm\x01\x00\x00\x00".to_vec();
        plain.extend_from_slice(&[0x05, 0x03, 0x01, 0x00, 0x01]); // 自己声明一块普通内存
        let dir = Dir::new(&[("index.html", b"<html>Godot</html>"), ("game.wasm", &plain)]);
        assert_eq!(dir.inspect().threads, None);
    }

    /// wasm 是压缩的，读不到里面，只能按脚本里的词猜——而且要说出这是猜的。
    #[test]
    fn a_compressed_wasm_falls_back_to_the_word_in_the_script() {
        let dir = Dir::new(&[
            ("index.html", b"<script src=\"index.js\"></script>"),
            ("index.wasm.br", b"\x1b\x28\x00"),
            (
                "index.js",
                b"if (typeof SharedArrayBuffer === 'undefined') throw 0;",
            ),
        ]);
        let threads = dir.inspect().threads.expect("该猜到要多线程");
        assert!(!threads.is_certain());
        assert!(threads.because().contains("index.js"), "{threads:?}");
        assert!(threads.because().contains("猜"), "{threads:?}");
    }

    /// 41 字节的合法小模块（`fixtures/headers-lab` 里那个）不能被当成线程版。
    #[test]
    fn a_tiny_valid_module_is_not_threaded() {
        let dir = Dir::new(&[
            ("index.html", b"<html>hi</html>"),
            ("mod.wasm", b"\0asm\x01\x00\x00\x00"),
        ]);
        assert_eq!(dir.inspect().threads, None);
    }

    #[test]
    fn an_index_html_in_a_subdirectory_is_called_out() {
        let dir = Dir::new(&[("web/index.html", b"<html></html>"), ("web/a.js", b"1")]);
        let report = dir.inspect();
        let found = about(&report, "web/index.html");
        assert_eq!(found.level, Level::Blocker);
        assert!(found.hint.as_ref().unwrap().contains("/web"), "{found:?}");
    }

    #[test]
    fn no_index_anywhere_is_still_worth_saying() {
        let dir = Dir::new(&[("main.js", b"console.log(1)")]);
        let report = dir.inspect();
        assert_eq!(
            about(&report, "最外层没有 index.html").level,
            Level::Blocker
        );
    }

    #[test]
    fn files_the_page_asks_for_but_nobody_uploaded() {
        let dir = Dir::new(&[
            (
                "index.html",
                br#"<link rel="stylesheet" href="/assets/a.css">
                    <script src="/assets/b.js"></script>
                    <img src="hero.png">"#,
            ),
            ("assets/a.css", b"body{}"),
        ]);
        let report = dir.inspect();
        let found = about(&report, "玩家那边会加载失败");
        assert_eq!(found.level, Level::Warn);
        assert!(found.message.contains("assets/b.js"), "{found:?}");
        assert!(found.message.contains("hero.png"), "{found:?}");
        assert!(!found.message.contains("a.css"), "{found:?}");
    }

    /// 缺得多的时候只列前几个。
    #[test]
    fn only_the_first_few_missing_files_are_listed() {
        let mut html = String::new();
        for i in 0..9 {
            html.push_str(&format!("<script src=\"m{i}.js\"></script>"));
        }
        let dir = Dir::new(&[("index.html", html.as_bytes())]);
        let found = dir.inspect();
        let found = about(&found, "玩家那边会加载失败");
        assert!(found.message.contains("m4.js"), "{found:?}");
        assert!(!found.message.contains("m5.js"), "{found:?}");
        assert!(found.message.contains("还有 4 个"), "{found:?}");
    }

    /// Unity 两套压缩产物都在：说清楚以哪一套为准。
    #[test]
    fn unity_with_both_compression_sets_says_which_one_wins() {
        let index = br#"var config = { dataUrl: buildUrl + "/webgl.data.unityweb" };"#;
        let dir = Dir::new(&[
            ("index.html", index),
            ("Build/webgl.loader.js", b"loader"),
            ("Build/webgl.data.br", b"br"),
            ("Build/webgl.data.unityweb", b"fallback"),
            ("Build/webgl.framework.js.unityweb", b"fallback"),
        ]);
        let report = dir.inspect();
        assert_eq!(report.engine, Some(Engine::Unity));
        assert_eq!(report.manifest_engine().as_deref(), Some("unity"));
        assert_eq!(about(&report, "Decompression Fallback").level, Level::Warn);
        let both = about(&report, "两套压缩产物");
        assert!(both.message.contains("webgl.data.unityweb"), "{both:?}");
        assert!(both.message.contains(".unityweb 那一套"), "{both:?}");
    }

    /// 只有 .br 的 Unity 导出正是我们想要的形态，只说一句「照原样给」。
    #[test]
    fn unity_with_only_brotli_gets_one_calm_line() {
        let dir = Dir::new(&[
            ("index.html", b"<html></html>"),
            ("Build/webgl.loader.js", b"loader"),
            ("Build/webgl.data.br", b"br"),
        ]);
        let report = dir.inspect();
        assert_eq!(
            messages(&report),
            [
                "看起来是 Unity 做的",
                "目录里有压好的 .br / .gz，会按原样直接给玩家，由浏览器解压"
            ]
        );
    }

    /// Godot 的 `.pck` 漏了：一打开就永远卡在加载条上，这一条要在上传前说。
    #[test]
    fn a_godot_export_without_its_pck_is_called_out() {
        let dir = Dir::new(&[
            ("index.html", b"<script>const GODOT_CONFIG = {};</script>"),
            ("index.js", b"//"),
            ("index.wasm", b"\0asm\x01\x00\x00\x00"),
            ("index.audio.worklet.js", b"//"),
        ]);
        let report = dir.inspect();
        let found = about(&report, "没有 .pck");
        assert_eq!(found.level, Level::Warn);
        assert!(found.hint.as_ref().unwrap().contains(".pck"), "{found:?}");

        // 补上就不说了。
        let whole = Dir::new(&[
            ("index.html", b"<script>const GODOT_CONFIG = {};</script>"),
            ("index.js", b"//"),
            ("index.wasm", b"\0asm\x01\x00\x00\x00"),
            ("index.pck", b"pack"),
        ]);
        assert_eq!(messages(&whole.inspect()), ["看起来是 Godot 做的"]);
    }

    /// 只是在页面里提了一句 Godot 的，不算 Godot 导出——不能因为一个词就说人家少传了文件。
    #[test]
    fn a_page_that_merely_mentions_godot_is_not_missing_anything() {
        let dir = Dir::new(&[
            ("index.html", b"<p>Godot 4 needs two response headers</p>"),
            ("mod.wasm", b"\0asm\x01\x00\x00\x00"),
        ]);
        assert_eq!(messages(&dir.inspect()), ["看起来是 Godot 做的"]);
    }

    /// Unity 的 Build 只拷了一半。
    #[test]
    fn a_unity_build_without_its_data_is_called_out() {
        let dir = Dir::new(&[
            ("index.html", b"<html></html>"),
            ("Build/webgl.loader.js", b"loader"),
            ("Build/webgl.framework.js", b"framework"),
            ("Build/webgl.wasm", b"\0asm\x01\x00\x00\x00"),
        ]);
        let report = dir.inspect();
        assert_eq!(about(&report, "没有 .data").level, Level::Warn);
    }

    #[test]
    fn a_whole_repository_is_called_out_rather_than_uploaded_quietly() {
        let dir = Dir::new(&[
            ("index.html", b"<html></html>"),
            ("node_modules/left-pad/index.js", b"module.exports=1"),
            ("src/main.ts", b"export {}"),
        ]);
        let report = dir.inspect_with_git(true);
        assert_eq!(about(&report, "node_modules/").level, Level::Warn);
        assert_eq!(about(&report, ".git/").level, Level::Note);
    }

    /// 一个干净的导出物不该被念叨。
    #[test]
    fn a_clean_export_says_nothing_beyond_the_engine() {
        let dir = Dir::new(&[
            (
                "index.html",
                br#"<script type="module" src="/assets/index-abc.js"></script>"#,
            ),
            ("assets/index-abc.js", b"console.log(1)"),
        ]);
        let report = dir.inspect();
        assert_eq!(report.engine, Some(Engine::Vite));
        // 清单里照实写 vite；「打包工具不等于游戏」由门禁页那份名单判断，玩家看到「体验」。
        assert_eq!(report.manifest_engine().as_deref(), Some("vite"));
        assert_eq!(messages(&report), ["看起来是 Vite 做的"]);
    }

    #[test]
    fn limit_messages_name_the_file_and_both_numbers() {
        let too_big = ManifestError::FileTooLarge {
            path: "Build/webgl.data".into(),
            size: 260 * MIB,
            max: 200 * MIB,
        };
        let text = explain_limit(&too_big, &[]).unwrap();
        assert!(text.contains("Build/webgl.data"), "{text}");
        assert!(text.contains("260.0 MB"), "{text}");
        assert!(text.contains("200.0 MB"), "{text}");

        let too_many = ManifestError::TooManyFiles {
            count: 9000,
            max: 5000,
        };
        let paths = vec!["node_modules/x/index.js".to_string()];
        assert!(explain_limit(&too_many, &paths)
            .unwrap()
            .contains("源码目录"));
        assert!(!explain_limit(&too_many, &[]).unwrap().contains("源码目录"));
    }

    #[test]
    fn source_trees_are_recognised() {
        let paths = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(looks_like_source_tree(&paths(&[
            "package.json",
            "index.html"
        ])));
        assert!(looks_like_source_tree(&paths(&["src/main.ts"])));
        assert!(looks_like_source_tree(&paths(&["node_modules/x/index.js"])));
        assert!(!looks_like_source_tree(&paths(&[
            "index.html",
            "game.wasm"
        ])));
    }
}
