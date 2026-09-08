//! 上传那一刻的检查：六种真的会遇到的导出目录，看 `playtest` 到底说了什么。
//!
//! 跑的是真的二进制，样本在临时目录里用字节现造（连线程版 Godot 的 `.wasm` 也是手写的
//! ——本机没装引擎，见 `docs/KICKOFF.md` §0）。检查发生在联网之前，但 `--json` 模式下
//! 只有「做成了」那个对象里才有 `findings`，所以这里仍然起一个假控制面把整条路走完，
//! 顺便看清单里的 `engine` 和 `isolated` 是不是真的跟着请求过去了。
//!
//! 每个样本都跑两遍：一遍人类模式看 stderr 上的话，一遍 `--json` 看数组里的条目。
//! 两遍说的必须是同一件事——一边有另一边没有，就是接口漏了。

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use axum::extract::{Path as UrlPath, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{post, put};
use axum::{Json, Router};
use playtest_common::api::routes;
use serde_json::{json, Value};

const SLUG: &str = "brisk-otter-41";

// ---------------------------------------------------------------- 假控制面

/// 只记一件事：CLI 提交上来的那份 prepare 请求。清单里的 `engine`、`isolated` 从这里看。
#[derive(Default)]
struct Fake {
    prepared: Mutex<Vec<Value>>,
}

async fn anon_sessions() -> Json<Value> {
    Json(json!({ "token": "tok-1", "expires_at": "2099-01-01T00:00:00Z" }))
}

async fn create_site() -> Json<Value> {
    Json(json!({
        "slug": SLUG,
        "url": format!("http://{SLUG}.localhost:8443"),
        "title": "export",
        "created_at": "2026-09-07T00:00:00Z",
        "expires_at": "2026-09-08T03:30:00Z",
    }))
}

async fn prepare_upload(
    State(fake): State<Arc<Fake>>,
    UrlPath(_slug): UrlPath<String>,
    Json(request): Json<Value>,
) -> Response {
    fake.prepared.lock().unwrap().push(request);
    // 一个都不缺：这里要看的是检查说了什么，不是传字节。
    Json(json!({ "upload_id": "upload-1", "missing": [], "missing_bytes": 0 })).into_response()
}

async fn put_blob(UrlPath(_hash): UrlPath<String>, _body: axum::body::Bytes) -> Response {
    StatusCode::CREATED.into_response()
}

async fn commit_upload(UrlPath((slug, _upload)): UrlPath<(String, String)>) -> Json<Value> {
    Json(json!({
        "slug": slug,
        "version": 1,
        "url": format!("http://{slug}.localhost:8443"),
        "expires_at": "2026-09-08T03:30:00Z",
    }))
}

/// 起一个假控制面，返回它的地址和记录本。线程随进程一起结束。
fn start_fake() -> (String, Arc<Fake>) {
    let fake = Arc::new(Fake::default());
    let served = Arc::clone(&fake);
    let (tx, rx) = std::sync::mpsc::channel::<SocketAddr>();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            let app = Router::new()
                .route(routes::ANON_SESSIONS, post(anon_sessions))
                .route(routes::SITES, post(create_site))
                .route(routes::SITE_UPLOADS, post(prepare_upload))
                .route(routes::BLOB, put(put_blob))
                .route(routes::SITE_UPLOAD_COMMIT, post(commit_upload))
                .with_state(served);
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            tx.send(listener.local_addr().unwrap()).unwrap();
            axum::serve(listener, app).await.unwrap();
        });
    });
    let addr = rx.recv().expect("假控制面没起来");
    (format!("http://{addr}"), fake)
}

// ---------------------------------------------------------------- 跑二进制

fn run_cli(home: &Path, api: &str, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_playtest"));
    command
        .args(args)
        .env("HOME", home)
        .env("PLAYTEST_API", api)
        // 走的是 127.0.0.1，任何代理设置都只会碍事。
        .env("NO_PROXY", "*")
        .env_remove("APPDATA")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("http_proxy")
        .env_remove("https_proxy")
        .env_remove("ALL_PROXY");
    command.output().expect("跑不起来 playtest")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// 一次样本的两遍结果。
struct Looked {
    /// 人类模式下 stderr 上的全部文字。
    said: String,
    /// `--json` 模式下 `findings` 数组里的条目。
    findings: Vec<Value>,
    /// `--json` 那一遍提交上去的 prepare 请求。
    request: Value,
}

impl Looked {
    /// 找到说了某件事的那一条 finding；两边都要说到，只有一边说了就是漏了。
    fn about(&self, needle: &str) -> &Value {
        assert!(
            self.said.contains(needle),
            "人类模式没说到「{needle}」，只说了：\n{}",
            self.said
        );
        self.findings
            .iter()
            .find(|f| f["message"].as_str().is_some_and(|m| m.contains(needle)))
            .unwrap_or_else(|| {
                panic!("--json 的 findings 里没有「{needle}」：{:#?}", self.findings)
            })
    }

    /// 确认这件事一个字都没提。
    fn silent_about(&self, needle: &str) {
        assert!(
            !self.said.contains(needle),
            "不该提「{needle}」，却说了：\n{}",
            self.said
        );
        assert!(
            !self
                .findings
                .iter()
                .any(|f| f["message"].as_str().is_some_and(|m| m.contains(needle))),
            "--json 里不该有「{needle}」：{:#?}",
            self.findings
        );
    }

    fn levels_of(&self, needle: &str) -> &str {
        self.about(needle)["level"].as_str().unwrap()
    }
}

/// 对一个目录跑两遍：不带 `--json` 一遍、带 `--json` 一遍。两遍都用干净的配置目录，
/// 免得第二遍沿用第一遍记住的作品。
fn look_at(dir: &Path, extra: &[&str]) -> Looked {
    let (api, fake) = start_fake();
    let path = dir.to_str().unwrap();

    let human_home = tempfile::tempdir().unwrap();
    let mut args: Vec<&str> = vec![path, "--no-qr"];
    args.extend_from_slice(extra);
    let human = run_cli(human_home.path(), &api, &args);
    assert!(human.status.success(), "人类模式挂了：{}", stderr_of(&human));

    let machine_home = tempfile::tempdir().unwrap();
    let mut args: Vec<&str> = vec!["--json", path, "--no-qr"];
    args.extend_from_slice(extra);
    let machine = run_cli(machine_home.path(), &api, &args);
    assert!(
        machine.status.success(),
        "--json 挂了：{}",
        stderr_of(&machine)
    );

    let text = String::from_utf8_lossy(&machine.stdout).into_owned();
    let value: Value = serde_json::from_str(text.trim())
        .unwrap_or_else(|e| panic!("stdout 不是一个 JSON 对象（{e}）：{text:?}"));
    let findings = value["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("成功的对象里必须有 findings 数组：{value}"))
        .clone();
    // 检查的话一句都不能跑到 stdout 上——那条流是给 `| pbcopy` 和脚本的。
    for finding in &findings {
        assert!(finding["message"].is_string(), "{finding}");
    }

    let request = fake
        .prepared
        .lock()
        .unwrap()
        .last()
        .cloned()
        .expect("假控制面没收到 prepare");

    Looked {
        said: stderr_of(&human),
        findings,
        request,
    }
}

fn write(path: &Path, contents: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

// ---------------------------------------------------------------- 六种样本

/// 一个带共享内存导入的最小 wasm 模块：8 字节的头，加一个 import 段，段里是
/// `env.memory`，limits 的标志位写着「有上限 + 共享」。Godot 4 的线程导出在字节层面
/// 就是这个形状——本机没装引擎，只能手写这一段，真导出物要等 `fixtures/` 补上。
fn threaded_wasm() -> Vec<u8> {
    let mut body = vec![0x01]; // 一条导入
    body.extend_from_slice(&[0x03, b'e', b'n', b'v']); // 从模块 env
    body.extend_from_slice(&[0x06, b'm', b'e', b'm', b'o', b'r', b'y']); // 导入名 memory
    body.push(0x02); // 导入的是一块内存
    body.push(0b011); // 写了上限、共享
    body.extend_from_slice(&[0x80, 0x02, 0x80, 0x04]); // 最少 256 页，最多 512 页
    let mut out = b"\0asm\x01\x00\x00\x00".to_vec();
    out.push(0x02); // import 段
    out.push(u8::try_from(body.len()).unwrap());
    out.extend_from_slice(&body);
    out
}

/// 同样的形状，但没有共享那一位：单线程导出。
fn plain_wasm() -> Vec<u8> {
    let mut out = b"\0asm\x01\x00\x00\x00".to_vec();
    out.extend_from_slice(&[0x05, 0x03, 0x01, 0x00, 0x01]); // 自己声明一块普通内存
    out
}

/// Godot 4 的线程导出：`.pck` 里是内容，`.wasm` 里申请了共享内存。
fn godot_threaded(root: &Path) -> PathBuf {
    let dir = root.join("godot-threads");
    write(
        &dir.join("index.html"),
        br#"<!doctype html><html><head><title>Bounce</title></head><body>
<canvas id="canvas"></canvas>
<script src="index.js"></script>
<script>const GODOT_CONFIG = {"executable":"index","fileSizes":{}};</script>
</body></html>"#,
    );
    write(&dir.join("index.js"), b"// Godot engine bootstrap\n");
    write(&dir.join("index.audio.worklet.js"), b"// audio worklet\n");
    write(&dir.join("index.wasm"), &threaded_wasm());
    write(&dir.join("index.pck"), b"godot pack\n");
    dir
}

/// Unity 开着 Decompression Fallback 导出：四件产物都叫 `.unityweb`，页面自己解压。
fn unity_unityweb(root: &Path) -> PathBuf {
    let dir = root.join("unity-unityweb");
    write(
        &dir.join("index.html"),
        br#"<!doctype html><html><body><canvas id="unity-canvas"></canvas>
<script>
  var buildUrl = "Build";
  var loaderUrl = buildUrl + "/webgl.loader.js";
  var config = {
    dataUrl: buildUrl + "/webgl.data.unityweb",
    frameworkUrl: buildUrl + "/webgl.framework.js.unityweb",
    codeUrl: buildUrl + "/webgl.wasm.unityweb",
  };
</script>
</body></html>"#,
    );
    write(&dir.join("Build/webgl.loader.js"), b"// unity loader\n");
    write(&dir.join("Build/webgl.data.unityweb"), b"packed data\n");
    write(&dir.join("Build/webgl.framework.js.unityweb"), b"packed js\n");
    write(&dir.join("Build/webgl.wasm.unityweb"), b"packed wasm\n");
    dir
}

/// Unity 关掉 Decompression Fallback 的导出：预压缩的 `.br`，服务器配好头就行。
fn unity_brotli(root: &Path) -> PathBuf {
    let dir = root.join("unity-brotli");
    write(
        &dir.join("index.html"),
        br#"<!doctype html><html><body><canvas id="unity-canvas"></canvas>
<script>
  var buildUrl = "Build";
  var config = {
    dataUrl: buildUrl + "/webgl.data.br",
    frameworkUrl: buildUrl + "/webgl.framework.js.br",
    codeUrl: buildUrl + "/webgl.wasm.br",
  };
</script>
</body></html>"#,
    );
    write(&dir.join("Build/webgl.loader.js"), b"// unity loader\n");
    write(&dir.join("Build/webgl.data.br"), b"\x1b\x28\x00data\n");
    write(&dir.join("Build/webgl.framework.js.br"), b"\x1b\x28\x00js\n");
    write(&dir.join("Build/webgl.wasm.br"), b"\x1b\x28\x00wasm\n");
    dir
}

/// 导出物在子目录里，传的是它上面那一层。
fn index_in_subdirectory(root: &Path) -> PathBuf {
    let dir = root.join("one-level-up");
    write(&dir.join("web/index.html"), b"<html><body>hi</body></html>");
    write(&dir.join("web/game.js"), b"console.log(1)\n");
    write(&dir.join("README.md"), b"# how to build\n");
    dir
}

/// 页面点名要的文件没跟着传上来。
fn missing_reference(root: &Path) -> PathBuf {
    let dir = root.join("half-copied");
    write(
        &dir.join("index.html"),
        br#"<!doctype html><html><head>
<link rel="stylesheet" href="/assets/style.css">
</head><body><script src="/assets/game.js"></script><img src="cover.png"></body></html>"#,
    );
    write(&dir.join("assets/style.css"), b"body{margin:0}\n");
    dir
}

/// 一个普通的 Vite 网页应用：什么毛病都没有，只该说一句它是什么做的。
fn vite_app(root: &Path) -> PathBuf {
    let dir = root.join("vite-app");
    write(
        &dir.join("index.html"),
        br#"<!doctype html><html><head>
<link rel="icon" href="/favicon.svg">
<script type="module" crossorigin src="/assets/index-B1OnktKc.js"></script>
<link rel="stylesheet" crossorigin href="/assets/index-CsUDhMuy.css">
</head><body><div id="app"></div></body></html>"#,
    );
    write(&dir.join("assets/index-B1OnktKc.js"), b"console.log(1)\n");
    write(&dir.join("assets/index-CsUDhMuy.css"), b":root{}\n");
    write(&dir.join("favicon.svg"), b"<svg/>\n");
    dir
}

// ---------------------------------------------------------------- 测试

/// Godot 线程版：认出引擎、从 wasm 的字节里拿到硬证据、没人能回答问题时自己开隔离。
#[test]
fn a_threaded_godot_export_turns_isolation_on_by_itself_and_says_so() {
    let work = tempfile::tempdir().unwrap();
    let looked = look_at(&godot_threaded(work.path()), &[]);

    looked.about("看起来是 Godot 做的");
    // 硬证据来自 wasm 的字节，所以话里说的是「用了」而不是「可能用了」。
    let threads = looked.about("多线程");
    let message = threads["message"].as_str().unwrap();
    assert!(message.contains("index.wasm"), "{message}");
    assert!(message.contains("共享的内存"), "{message}");
    assert!(!message.contains("可能"), "有硬证据就别说可能：{message}");
    looked.about("已经自动加上 --isolated");
    // 不是终端也不是 --json 都得自己拿主意，所以不能反过来问一句就卡在那里。
    looked.silent_about("帮你开吗");
    // 说了就要做到：清单里 isolated 是真的。
    assert_eq!(looked.request["isolated"], true, "{}", looked.request);
    assert_eq!(looked.request["engine"], "godot", "{}", looked.request);
    // 四件产物都在，不该念叨少了什么。
    looked.silent_about("没有 .pck");
}

/// `--no-isolated` 是明确拒绝：照办，但要说清楚玩家那边会怎样。
#[test]
fn saying_no_to_isolation_is_obeyed_and_the_consequence_is_spelled_out() {
    let work = tempfile::tempdir().unwrap();
    let looked = look_at(&godot_threaded(work.path()), &["--no-isolated"]);

    let refused = looked.about("--no-isolated");
    assert_eq!(refused["level"], "warn", "{refused}");
    assert!(
        refused["hint"]
            .as_str()
            .is_some_and(|h| h.contains("报错")),
        "得说清楚不开的后果：{refused}"
    );
    assert_eq!(looked.request["isolated"], false, "{}", looked.request);
}

/// 已经加了 `--isolated` 的：确认一句就够，别再问一遍。
#[test]
fn an_export_that_already_asked_for_isolation_is_just_acknowledged() {
    let work = tempfile::tempdir().unwrap();
    let looked = look_at(&godot_threaded(work.path()), &["--isolated"]);

    assert_eq!(looked.levels_of("--isolated 已经开着"), "note");
    assert_eq!(looked.request["isolated"], true, "{}", looked.request);
}

/// Unity 的 Decompression Fallback：能跑，但比浏览器解压慢，说清楚怎么关。
#[test]
fn a_unity_export_with_decompression_fallback_is_flagged_as_the_slow_path() {
    let work = tempfile::tempdir().unwrap();
    let looked = look_at(&unity_unityweb(work.path()), &[]);

    looked.about("看起来是 Unity 做的");
    let slow = looked.about("Decompression Fallback");
    assert_eq!(slow["level"], "warn", "{slow}");
    assert!(
        slow["hint"]
            .as_str()
            .is_some_and(|h| h.contains("Publishing Settings")),
        "得说在哪里关：{slow}"
    );
    // 只有 .unityweb 这一套，没有第二套可选。
    looked.silent_about("两套压缩产物");
    looked.silent_about("没有 .data");
    assert_eq!(looked.request["engine"], "unity", "{}", looked.request);
    // Unity 的导出不用跨源隔离，别顺手给人加上。
    assert_eq!(looked.request["isolated"], false, "{}", looked.request);
}

/// Unity 的 `.br`：这正是我们想要的形态，只平静地说一句「照原样给」。
#[test]
fn a_unity_export_with_brotli_only_gets_one_calm_line() {
    let work = tempfile::tempdir().unwrap();
    let looked = look_at(&unity_brotli(work.path()), &[]);

    assert_eq!(looked.levels_of("压好的 .br / .gz"), "note");
    looked.silent_about("Decompression Fallback");
    looked.silent_about("两套压缩产物");
    assert_eq!(looked.request["engine"], "unity", "{}", looked.request);
}

/// `index.html` 不在最外层：这是「传上去也一定打不开」，要指出该传哪一层。
#[test]
fn an_index_html_one_level_down_is_a_blocker_that_names_the_right_directory() {
    let work = tempfile::tempdir().unwrap();
    let dir = index_in_subdirectory(work.path());
    let path = dir.to_str().unwrap();

    // 「传上去一定打不开」的不传：匿名作品只有三个名额，一个 404 的链接会白占一个。
    // 退出码 6（给的东西有问题），话里要点名该传哪一层，并告诉人家 --force 能放行。
    let (api, fake) = start_fake();
    let home = tempfile::tempdir().unwrap();
    let blocked = run_cli(home.path(), &api, &["--json", path, "--no-qr"]);
    assert_eq!(blocked.status.code(), Some(6), "{}", stderr_of(&blocked));
    let value: Value = serde_json::from_slice(&blocked.stdout).unwrap();
    assert_eq!(value["ok"], false);
    assert_eq!(value["code"], "bad_input");
    let message = value["message"].as_str().unwrap();
    assert!(message.contains("最外层没有 index.html") && message.contains("web/index.html"), "{value}");
    let hint = value["hint"].as_str().unwrap();
    assert!(hint.contains("/web") && hint.contains("--force"), "得告诉人家该传哪一层、怎么放行：{value}");
    assert!(fake.prepared.lock().unwrap().is_empty(), "拦下就不该向控制面提交清单");

    // --force 照传，且那条发现仍然在 findings 里让人看见。
    let looked = look_at(&dir, &["--force"]);
    let found = looked.about("最外层没有 index.html");
    assert_eq!(found["level"], "blocker", "{found}");
    assert!(looked.request["files"].as_array().unwrap().len() == 3);
}

/// 页面要的文件没传上来：点名是哪几个。
#[test]
fn files_the_page_asks_for_but_nobody_uploaded_are_named() {
    let work = tempfile::tempdir().unwrap();
    let looked = look_at(&missing_reference(work.path()), &[]);

    let found = looked.about("玩家那边会加载失败");
    assert_eq!(found["level"], "warn", "{found}");
    let message = found["message"].as_str().unwrap();
    assert!(message.contains("assets/game.js"), "{message}");
    assert!(message.contains("cover.png"), "{message}");
    // 传上来的那个不能被冤枉。
    assert!(!message.contains("style.css"), "{message}");
}

/// 一个干净的网页应用：认出是 Vite 打的，此外一句废话都不说。
#[test]
fn a_clean_web_app_is_recognised_and_otherwise_left_alone() {
    let work = tempfile::tempdir().unwrap();
    let looked = look_at(&vite_app(work.path()), &[]);

    assert_eq!(looked.findings.len(), 1, "{:#?}", looked.findings);
    assert_eq!(looked.levels_of("看起来是 Vite 做的"), "note");
    // 清单里照实写 vite；「这算不算游戏」由门禁页那份名单判断，玩家会看到「体验」。
    assert_eq!(looked.request["engine"], "vite", "{}", looked.request);
    assert_eq!(looked.request["isolated"], false, "{}", looked.request);
}

/// 叫 `.wasm` 但不是 wasm 的东西：安静地当作看不出来，不能崩，也不能顺手加上隔离。
///
/// `fixtures/headers-lab/export/mod.wasm` 只有 41 字节，上传目录里什么都可能叫这个名字：
/// 压缩过的、传了一半的、随手改名的。
#[test]
fn a_wasm_that_is_not_a_wasm_is_shrugged_off_quietly() {
    let work = tempfile::tempdir().unwrap();
    let dir = work.path().join("odd-wasm");
    write(&dir.join("index.html"), b"<html><body>hi</body></html>");
    write(&dir.join("truncated.wasm"), b"\0asm\x01\x00\x00\x00\x02");
    write(&dir.join("brotli.wasm"), b"\x1b\x28\x00\x00\x04");
    write(&dir.join("empty.wasm"), b"");
    write(&dir.join("html.wasm"), b"<!doctype html>");
    write(&dir.join("single.wasm"), &plain_wasm());

    let looked = look_at(&dir, &[]);
    assert!(looked.findings.is_empty(), "{:#?}", looked.findings);
    assert_eq!(looked.request["isolated"], false, "{}", looked.request);
    assert!(
        looked.request.get("engine").is_none_or(Value::is_null),
        "看不出引擎就不该往清单里写：{}",
        looked.request
    );
}

/// 传的是整个工程而不是导出物：`node_modules/` 和 `.git/` 各说一句。
#[test]
fn a_whole_project_directory_is_called_out() {
    let work = tempfile::tempdir().unwrap();
    let dir = work.path().join("whole-repo");
    write(&dir.join("index.html"), b"<html><body>hi</body></html>");
    write(&dir.join("package.json"), b"{\"name\":\"x\"}\n");
    write(&dir.join("node_modules/left-pad/index.js"), b"module.exports=1\n");
    write(&dir.join(".git/HEAD"), b"ref: refs/heads/main\n");

    let looked = look_at(&dir, &[]);
    assert_eq!(looked.levels_of("node_modules/"), "warn");
    assert_eq!(looked.levels_of(".git/"), "note");
    // 以「.」开头的东西不上传，说了也不能真把它传上去。
    let paths: Vec<&str> = looked.request["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    assert!(!paths.iter().any(|p| p.starts_with(".git/")), "{paths:?}");
}
