//! `--json` 这条路：stdout 上只有一个 JSON 对象，退出码按 [`playtest::output::Code`] 分层。
//!
//! 跑的是真的二进制，对面是一个假控制面。假控制面的**路由**用
//! `playtest_common::api::routes` 的常量注册（写错了这里就挂），**响应体**直接写字面 JSON
//! ——这样别人往那些结构体上加字段时，这个文件不用跟着改。

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use axum::extract::{Path as UrlPath, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use playtest_common::api::routes;
use serde_json::{json, Value};

const SLUG: &str = "brisk-otter-41";

// ---------------------------------------------------------------- 假控制面

/// 让 `POST /v1/sites/{slug}/uploads` 回一个指定的错，用来验各条退出码。
#[derive(Clone, Copy)]
struct Refusal {
    status: u16,
    code: &'static str,
}

#[derive(Default)]
struct Fake {
    refuse_prepare: Mutex<Option<Refusal>>,
}

fn error_body(refusal: Refusal) -> Response {
    (
        StatusCode::from_u16(refusal.status).unwrap(),
        Json(json!({ "code": refusal.code, "message": "服务器说的那句话" })),
    )
        .into_response()
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

async fn list_sites() -> Json<Value> {
    Json(json!([{
        "slug": SLUG,
        "url": format!("http://{SLUG}.localhost:8443"),
        "title": "小球试玩",
        "current_version": 3,
        "created_at": "2026-09-07T00:00:00Z",
        "expires_at": "2026-09-08T03:30:00Z",
    }]))
}

async fn prepare_upload(
    State(fake): State<Arc<Fake>>,
    UrlPath(_slug): UrlPath<String>,
    Json(request): Json<Value>,
) -> Response {
    if let Some(refusal) = *fake.refuse_prepare.lock().unwrap() {
        return error_body(refusal);
    }
    // 一个都不缺：这些测试要看的是输出的形状，不是传字节。
    let count = request["files"].as_array().map_or(0, Vec::len);
    Json(json!({ "upload_id": "upload-1", "missing": [], "missing_bytes": 0, "files": count }))
        .into_response()
}

async fn put_blob(UrlPath(_hash): UrlPath<String>, _body: axum::body::Bytes) -> Response {
    StatusCode::CREATED.into_response()
}

async fn commit_upload(UrlPath((slug, _upload)): UrlPath<(String, String)>) -> Json<Value> {
    Json(json!({
        "slug": slug,
        "version": 7,
        "url": format!("http://{slug}.localhost:8443"),
        "expires_at": "2026-09-08T03:30:00Z",
    }))
}

/// 起一个假控制面，返回它的地址。线程随进程一起结束。
fn start_fake(refuse_prepare: Option<Refusal>) -> String {
    let fake = Arc::new(Fake {
        refuse_prepare: Mutex::new(refuse_prepare),
    });
    let (tx, rx) = std::sync::mpsc::channel::<SocketAddr>();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            let app = Router::new()
                .route(routes::ANON_SESSIONS, post(anon_sessions))
                .route(routes::SITES, post(create_site).get(list_sites))
                .route(routes::SITE_UPLOADS, post(prepare_upload))
                .route(routes::BLOB, put(put_blob))
                .route(routes::SITE_UPLOAD_COMMIT, post(commit_upload))
                .route(routes::HEALTH, get(|| async { "ok" }))
                .with_state(fake);
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            tx.send(listener.local_addr().unwrap()).unwrap();
            axum::serve(listener, app).await.unwrap();
        });
    });
    format!("http://{}", rx.recv().expect("假控制面没起来"))
}

// ---------------------------------------------------------------- 跑二进制

fn run_cli(home: &Path, api: &str, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_playtest"));
    command
        .args(args)
        .env("HOME", home)
        .env("PLAYTEST_API", api)
        .env("NO_PROXY", "*")
        .env_remove("APPDATA")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("http_proxy")
        .env_remove("https_proxy")
        .env_remove("ALL_PROXY");
    command.output().expect("跑不起来 playtest")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// stdout 必须整个就是一个 JSON 对象——多一行别的，脚本就得先做文本清洗。
fn only_object(output: &Output) -> Value {
    let text = stdout_of(output);
    let value: Value = serde_json::from_str(text.trim())
        .unwrap_or_else(|e| panic!("stdout 不是一个 JSON 对象（{e}）：{text:?}"));
    assert!(value.is_object(), "stdout 上应该是一个对象：{text:?}");
    assert_eq!(
        text.trim().lines().count(),
        1,
        "stdout 只该有一行：{text:?}"
    );
    value
}

fn expect_failure(output: &Output, exit: i32, code: &str) -> Value {
    let value = only_object(output);
    assert_eq!(value["ok"], false, "{value}");
    assert_eq!(value["code"], code, "{value}");
    assert_eq!(
        output.status.code(),
        Some(exit),
        "退出码不对（code={code}）：{}",
        stderr_of(output)
    );
    value
}

fn write(path: &Path, contents: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn make_export(root: &Path) -> PathBuf {
    let dist = root.join("dist");
    write(&dist.join("index.html"), b"<html>hi</html>");
    write(&dist.join("game.js"), b"console.log(1)");
    dist
}

// ---------------------------------------------------------------- 做成了

#[test]
fn an_upload_answers_with_one_object_and_keeps_the_talking_on_stderr() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());
    let api = start_fake(None);

    let output = run_cli(home.path(), &api, &["--json", dist.to_str().unwrap()]);
    assert!(output.status.success(), "{}", stderr_of(&output));

    let value = only_object(&output);
    assert_eq!(value["ok"], true);
    assert_eq!(value["action"], "upload");
    assert_eq!(value["slug"], SLUG);
    assert_eq!(value["url"], format!("http://{SLUG}.localhost:8443"));
    assert_eq!(value["version"], 7);
    assert_eq!(value["expires_at"], "2026-09-08T03:30:00Z");
    assert!(value["elapsed_ms"].is_u64(), "{value}");
    for segment in ["hash_ms", "upload_ms", "commit_ms"] {
        assert!(value["timings"][segment].is_u64(), "缺 {segment}：{value}");
    }
    assert!(value["findings"].is_array(), "{value}");
    // 二维码不画，但原文给出去，agent 可以贴回对话。
    let qr = value["qr_text"].as_str().expect("该有 qr_text");
    assert!(qr.contains('█'), "qr_text 该是画好的二维码：{qr:?}");

    // 说给人听的那些话一句都没跑到 stdout 上。
    let stderr = stderr_of(&output);
    assert!(stderr.contains("正在整理"), "{stderr}");
    assert!(!stderr.contains('█'), "JSON 模式不该画二维码：{stderr}");
}

#[test]
fn no_qr_means_the_field_is_simply_absent() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());
    let api = start_fake(None);

    let output = run_cli(
        home.path(),
        &api,
        &["--json", "--no-qr", dist.to_str().unwrap()],
    );
    assert!(output.status.success(), "{}", stderr_of(&output));
    let value = only_object(&output);
    assert!(value.get("qr_text").is_none(), "{value}");
}

#[test]
fn listing_nothing_is_an_empty_array_not_a_sentence() {
    let home = tempfile::tempdir().unwrap();
    let output = run_cli(home.path(), "http://127.0.0.1:1", &["ls", "--json"]);
    assert!(output.status.success(), "{}", stderr_of(&output));
    let value = only_object(&output);
    assert_eq!(value["ok"], true);
    assert_eq!(value["action"], "list");
    assert_eq!(value["sites"], json!([]));
}

#[test]
fn listing_after_an_upload_names_the_work() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());
    let api = start_fake(None);
    // 先发一次，把令牌存下来；没有令牌 ls 只会回空表。
    let first = run_cli(home.path(), &api, &["--json", dist.to_str().unwrap()]);
    assert!(first.status.success(), "{}", stderr_of(&first));

    let output = run_cli(home.path(), &api, &["ls", "--json"]);
    let value = only_object(&output);
    assert_eq!(value["sites"][0]["slug"], SLUG);
    assert_eq!(value["sites"][0]["title"], "小球试玩");
    assert_eq!(value["sites"][0]["version"], 3);
}

#[test]
fn asking_for_help_in_machine_mode_still_yields_one_object() {
    let home = tempfile::tempdir().unwrap();
    let output = run_cli(home.path(), "http://127.0.0.1:1", &["--json", "--help"]);
    assert!(output.status.success());
    let value = only_object(&output);
    assert_eq!(value["action"], "help");
    let text = value["text"].as_str().unwrap();
    assert!(text.contains("退出码"), "帮助里该有退出码那一段：{text}");
}

// ---------------------------------------------------------------- 各条退出码

#[test]
fn a_directory_that_is_not_there_is_our_users_problem_not_the_networks() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let missing = work.path().join("nope");
    let output = run_cli(
        home.path(),
        "http://127.0.0.1:1",
        &["--json", missing.to_str().unwrap()],
    );
    let value = expect_failure(&output, 6, "bad_input");
    assert!(
        value["message"].as_str().unwrap().contains("找不到"),
        "{value}"
    );
}

#[test]
fn nobody_listening_is_a_network_problem() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());
    let output = run_cli(
        home.path(),
        "http://127.0.0.1:1",
        &["--json", dist.to_str().unwrap()],
    );
    let value = expect_failure(&output, 4, "network");
    assert!(
        value["message"].as_str().unwrap().contains("连不上服务器"),
        "{value}"
    );
    assert!(value["hint"].as_str().unwrap().contains("--api"), "{value}");
}

#[test]
fn a_full_quota_has_its_own_exit_code() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());
    let api = start_fake(Some(Refusal {
        status: 413,
        code: "quota_exceeded",
    }));
    let output = run_cli(home.path(), &api, &["--json", dist.to_str().unwrap()]);
    let value = expect_failure(&output, 7, "quota_exceeded");
    assert!(
        value["message"]
            .as_str()
            .unwrap()
            .contains("服务器说的那句话"),
        "服务器的原话要带回来：{value}"
    );
}

#[test]
fn an_identity_the_server_no_longer_knows_asks_for_a_login() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());
    let api = start_fake(Some(Refusal {
        status: 401,
        code: "token_expired",
    }));
    let output = run_cli(home.path(), &api, &["--json", dist.to_str().unwrap()]);
    expect_failure(&output, 3, "needs_login");
}

#[test]
fn a_broken_server_is_not_blamed_on_the_directory() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());
    let api = start_fake(Some(Refusal {
        status: 500,
        code: "internal",
    }));
    let output = run_cli(home.path(), &api, &["--json", dist.to_str().unwrap()]);
    expect_failure(&output, 5, "server_error");
}

#[test]
fn a_flag_that_does_not_exist_is_a_usage_error() {
    let home = tempfile::tempdir().unwrap();
    let output = run_cli(home.path(), "http://127.0.0.1:1", &["--json", "--nope"]);
    expect_failure(&output, 2, "usage");
    assert!(
        stderr_of(&output).contains("--nope"),
        "clap 的原话要留在 stderr 上：{}",
        stderr_of(&output)
    );
}

#[test]
fn what_is_not_built_yet_says_so_instead_of_pretending() {
    let home = tempfile::tempdir().unwrap();
    let output = run_cli(home.path(), "http://127.0.0.1:1", &["--json", "login"]);
    let value = expect_failure(&output, 2, "not_implemented");
    assert!(
        value["message"].as_str().unwrap().contains("还没做好"),
        "{value}"
    );
}

#[test]
fn removing_without_confirmation_is_refused_not_guessed() {
    let home = tempfile::tempdir().unwrap();
    let output = run_cli(home.path(), "http://127.0.0.1:1", &["rm", SLUG, "--json"]);
    let value = expect_failure(&output, 2, "usage");
    assert!(value["message"].as_str().unwrap().contains("-y"), "{value}");
}

// ---------------------------------------------------------------- 人类模式没被弄坏

#[test]
fn without_the_flag_stdout_is_still_just_the_link() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());
    let api = start_fake(None);

    let output = run_cli(home.path(), &api, &[dist.to_str().unwrap(), "--no-qr"]);
    assert!(output.status.success(), "{}", stderr_of(&output));
    assert_eq!(
        stdout_of(&output),
        format!("http://{SLUG}.localhost:8443\n"),
        "`playtest ./dist | pbcopy` 拿到的必须就是链接"
    );

    // 「几秒」是数字：结尾那行给总数和分段。
    let stderr = stderr_of(&output);
    assert!(stderr.contains("本次 "), "{stderr}");
    assert!(stderr.contains("（哈希 "), "{stderr}");
    assert!(stderr.contains("· 上传 "), "{stderr}");
    assert!(stderr.contains("· 提交 "), "{stderr}");
}
