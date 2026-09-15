//! 上传那一刻的检查。
//!
//! 这时候我们手上有全部文件的字节，能查的就都查了：这个构建要不要跨源隔离、`index.html`
//! 在不在最外层、它引用的文件缺不缺、Unity 那两条互斥的压缩路撞没撞、是不是把整个工程
//! 当成导出物传了（DESIGN §4.2）。
//!
//! 这里只看出来，不打印，也不自己决定加不加参数——那两件事在 `upload.rs`。产出是一串
//! [`Finding`]，人类模式一条一行走 stderr，`--json` 模式进 `findings` 数组，两边同一份。
//!
//! 「这一版没有封面」不在这里：那件事只有服务器知道（上一版传过的封面还在），所以它在
//! `upload.rs` 里问完再说。我们也不从目录里猜一张图当封面、更不去跑用户的代码截图
//! （DESIGN §3.12）——封面是开发者选的，猜错了比没有更糟。
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
    /// 混合模式（`--backend`）：目录里没有的路径会交给开发者的后端，所以「引用了目录里没有的
    /// 文件」不再是加载失败的预告，只是告诉他哪些请求会到后端那边。
    pub has_backend: bool,
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
            Self::SharedMemory(path) => format!("{path} asks for memory shared across threads"),
            Self::ScriptMentions(path) => {
                format!("{path} mentions SharedArrayBuffer, but the wasm bytes do not show it, so this is a guess")
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
    /// `index.html` 的 `<title>`，去掉了引擎模板的默认值。没给 `--name` 时它比目录名更像作品名。
    pub page_title: Option<String>,
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
        findings.push(Finding::note(format!(
            "Looks like a {} build",
            engine.label()
        )));
    }
    findings.extend(index_placement(&here, &paths));
    if let Some(index) = index.as_deref() {
        findings.extend(missing_references(index, &here, input.has_backend));
    }
    findings.extend(missing_engine_parts(&paths, index.as_deref()));
    findings.extend(compression(&paths, index.as_deref(), engine));
    findings.extend(wrong_directory(&paths, input.has_git_dir));

    Report {
        engine,
        threads,
        page_title: index
            .as_deref()
            .and_then(html::title)
            .filter(|t| !is_template_title(t)),
        findings,
    }
}

/// 引擎模板默认的标题，没有作品名的信息量，不拿来当作品名。
fn is_template_title(title: &str) -> bool {
    let t = title.trim().to_ascii_lowercase();
    t.is_empty()
        || t == "vite + ts"
        || t == "vite app"
        || t == "vite + vue + ts"
        || t == "vite + react + ts"
        || t.starts_with("unity webgl player")
        || t == "godot"
        || t == "phaser game"
        || t == "document"
        || t == "index"
        || t == "untitled"
        || t.len() > 80
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
            "This version is {} in total, and anonymous uploads are capped at {} per version. Run playtest login to publish something larger.",
            megabytes(*total),
            megabytes(*max)
        )),
        ManifestError::FileTooLarge { path, size, max } => Some(format!(
            "{path} is {}, and a single file can be at most {}. Split it during export, or host it elsewhere and fetch it from the game.",
            megabytes(*size),
            megabytes(*max)
        )),
        ManifestError::TooManyFiles { count, max } if looks_like_source_tree(paths) => Some(format!(
            "{count} files, and the limit is {max}. This looks like a source directory rather than an export: build first (npm run build, for example), then upload dist/."
        )),
        ManifestError::TooManyFiles { count, max } => Some(format!(
            "{count} files, and the limit is {max}. Upload only the directory your engine exported, not the whole project."
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
            Finding::blocker("No index.html at the top level, so the link will 404 for players")
                .hint("Engine exports normally have one. Check that this is the export directory"),
        );
    };
    let dir = nested.strip_suffix("/index.html")?;
    Some(
        Finding::blocker(format!(
            "No index.html at the top level. It is inside {nested}, so the link will 404 for players"
        ))
        .hint(format!(
            "Upload {dir} itself: playtest <that directory>/{dir}"
        )),
    )
}

/// `index.html` 里引用了、但目录里没有的文件。
///
/// 没有后端时这是加载失败的预告；有后端（混合模式）时这些路径会交给后端，
/// 只说一声让他知道哪些请求会走到自己机器上，不当成错。
fn missing_references(index: &str, here: &HashSet<&str>, has_backend: bool) -> Option<Finding> {
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
        .join(", ");
    let rest = missing.len().saturating_sub(MAX_MISSING_SHOWN);
    let tail = if rest > 0 {
        format!(" and {rest} more")
    } else {
        String::new()
    };
    if has_backend {
        return Some(Finding::note(format!(
            "index.html references {} paths that are not in the directory; they will go to your backend: {shown}{tail}",
            missing.len()
        )));
    }
    Some(
        Finding::warn(format!(
            "index.html needs {} files that are not in the directory, so they will fail to load for players: {shown}{tail}",
            missing.len()
        ))
        .hint("Most likely the export missed them, or only one level was uploaded. Check that the directory your engine wrote is complete"),
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
            Finding::warn("This is a Godot web export, but there is no .pck in the directory. The game content lives in that file, so players will sit on the loading bar")
                .hint("Upload the exported files together: .html, .js, .wasm, .pck, and possibly .audio.worklet.js"),
        );
    }
    // `.loader.js` 是 Unity 网页导出一定有的那个文件。
    if bare(".loader.js") && !bare(".data") {
        return Some(
            Finding::warn("This is a Unity web export, but there is no .data in the directory. The game assets live in that file, so players will sit on the loading bar")
                .hint("Upload the whole Build directory: .loader.js, .framework.js, .wasm and .data are all required"),
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
            "The directory has precompressed .br / .gz files; they are served as-is and the browser decompresses them",
        ));
    }
    if fallback {
        let seen = if found == Some(Engine::Unity) {
            "Unity has Decompression Fallback on"
        } else {
            "The directory has .unityweb files, which Unity only writes with Decompression Fallback on"
        };
        out.push(
            Finding::warn(format!(
                "{seen}, so the page decompresses them in JS at load time, which is slower than letting the browser do it"
            ))
            .hint(
                "Turning it off in Player Settings -> Publishing Settings and re-exporting is faster; we set the Brotli headers for you",
            ),
        );
    }
    if precompressed && fallback {
        let pointing = index
            .and_then(|html| html::config_value(html, "dataUrl"))
            .map(|url| {
                let winner = if url.ends_with(".unityweb") {
                    "the .unityweb set"
                } else {
                    "the precompressed .br / .gz set"
                };
                format!(
                    "dataUrl in index.html points at {url}, so {winner} is what actually gets used"
                )
            })
            .unwrap_or_else(|| {
                "No dataUrl found in index.html, so it is unclear which set gets used".to_string()
            });
        out.push(
            Finding::warn(format!(
                "Both compressed sets are here: .br / .gz and .unityweb. {pointing}"
            ))
            .hint("One set is uploaded for nothing. Deleting it saves a lot of bytes"),
        );
    }
    out
}

/// 传的是不是整个工程，而不是导出物。
fn wrong_directory(paths: &[&str], has_git_dir: bool) -> Vec<Finding> {
    let mut out = Vec::new();
    if paths.iter().any(|p| p.starts_with("node_modules/")) {
        out.push(
            Finding::warn("The directory has node_modules/, so this looks like source rather than a build output")
                .hint("Build first (npm run build, for example), then upload the dist/ level"),
        );
    }
    if has_git_dir {
        out.push(
            Finding::note("The directory has .git/, so it looks like a whole repository rather than an export")
                .hint("Anything starting with a dot is skipped. Check that you are publishing the build output level"),
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
            self.inspect_with(false, false)
        }

        fn inspect_with_git(&self, has_git_dir: bool) -> Report {
            self.inspect_with(has_git_dir, false)
        }

        fn inspect_with_backend(&self) -> Report {
            self.inspect_with(false, true)
        }

        fn inspect_with(&self, has_git_dir: bool, has_backend: bool) -> Report {
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
                    has_backend,
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
        assert!(threads.because().contains("guess"), "{threads:?}");
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
            about(&report, "No index.html at the top level").level,
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
        let found = about(&report, "fail to load for players");
        assert_eq!(found.level, Level::Warn);
        assert!(found.message.contains("assets/b.js"), "{found:?}");
        assert!(found.message.contains("hero.png"), "{found:?}");
        assert!(!found.message.contains("a.css"), "{found:?}");
    }

    /// 有后端时，目录里没有的路径是后端的地界，不是加载失败的预告。
    #[test]
    fn with_a_backend_missing_files_are_a_note_about_where_requests_go() {
        let dir = Dir::new(&[
            (
                "index.html",
                br#"<a href="new">drop</a><script src="/app.js"></script>"#,
            ),
            ("app.js", b"1"),
        ]);
        let report = dir.inspect_with_backend();
        let found = about(&report, "go to your backend");
        assert_eq!(found.level, Level::Note);
        assert!(found.message.contains("new"), "{found:?}");
        assert!(found.hint.is_none(), "没有要他改的东西，就别给建议");
        assert!(!messages(&report)
            .iter()
            .any(|m| m.contains("fail to load for players")));
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
        let found = about(&found, "fail to load for players");
        assert!(found.message.contains("m4.js"), "{found:?}");
        assert!(!found.message.contains("m5.js"), "{found:?}");
        assert!(found.message.contains("and 4 more"), "{found:?}");
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
        let both = about(&report, "Both compressed sets");
        assert!(both.message.contains("webgl.data.unityweb"), "{both:?}");
        assert!(both.message.contains("the .unityweb set"), "{both:?}");
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
                "Looks like a Unity build",
                "The directory has precompressed .br / .gz files; they are served as-is and the browser decompresses them"
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
        let found = about(&report, "no .pck");
        assert_eq!(found.level, Level::Warn);
        assert!(found.hint.as_ref().unwrap().contains(".pck"), "{found:?}");

        // 补上就不说了。
        let whole = Dir::new(&[
            ("index.html", b"<script>const GODOT_CONFIG = {};</script>"),
            ("index.js", b"//"),
            ("index.wasm", b"\0asm\x01\x00\x00\x00"),
            ("index.pck", b"pack"),
        ]);
        assert_eq!(messages(&whole.inspect()), ["Looks like a Godot build"]);
    }

    /// 只是在页面里提了一句 Godot 的，不算 Godot 导出——不能因为一个词就说人家少传了文件。
    #[test]
    fn a_page_that_merely_mentions_godot_is_not_missing_anything() {
        let dir = Dir::new(&[
            ("index.html", b"<p>Godot 4 needs two response headers</p>"),
            ("mod.wasm", b"\0asm\x01\x00\x00\x00"),
        ]);
        assert_eq!(messages(&dir.inspect()), ["Looks like a Godot build"]);
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
        assert_eq!(about(&report, "no .data").level, Level::Warn);
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
        assert_eq!(messages(&report), ["Looks like a Vite build"]);
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
            .contains("source directory"));
        assert!(!explain_limit(&too_many, &[])
            .unwrap()
            .contains("source directory"));
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
