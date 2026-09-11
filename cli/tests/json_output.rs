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

/// 让 `POST /v1/projects/{slug}/uploads` 回一个指定的错，用来验各条退出码。
#[derive(Clone, Copy)]
struct Refusal {
    status: u16,
    code: &'static str,
}

#[derive(Default)]
struct Fake {
    refuse_prepare: Mutex<Option<Refusal>>,
    login_polls: Mutex<u32>,
    /// 作品链接。默认长得像玩家域上的一条链接（那台服务器不存在，取邀请卡会连不上，
    /// 正好也是一种要验的情形）；[`start_fake_as_edge`] 把它改成这个假服务器自己的地址，
    /// CLI 就会真的来这里取那张卡。
    site_url: Mutex<Option<String>>,
    /// PATCH 收到的那个请求体，用来验 CLI 到底把什么传了上去。
    patched: Mutex<Option<Value>>,
}

/// 一张「PNG」。只有头八个字节是真的——CLI 认的就是这八个字节加上响应头里的类型，
/// 塞一张真图进测试只会让这个文件多几十行看不懂的字节。
const CARD_BYTES: &[u8] = b"\x89PNG\r\n\x1a\nnot-really-a-png";

fn error_body(refusal: Refusal) -> Response {
    (
        StatusCode::from_u16(refusal.status).unwrap(),
        Json(json!({ "code": refusal.code, "message": "服务器说的那句话" })),
    )
        .into_response()
}

/// 这个假控制面没配 GitHub：登录要像线上没配时一样明说，而不是装作成功。
async fn login_unavailable() -> Response {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({ "code": "login_unavailable", "message": "这个控制面没有开 GitHub 登录。匿名链接照常能用，只是 24 小时后失效。" })),
    )
        .into_response()
}

/// 设备码流程：第一次轮询「还没输完」，第二次成功，并报告归入了 2 个匿名作品。
async fn device_start() -> Json<Value> {
    Json(json!({
        "device_code": "dev-1", "user_code": "WDJB-MJHT",
        "verification_uri": "https://github.com/login/device", "expires_in": 900, "interval": 0
    }))
}

async fn device_poll(State(fake): State<Arc<Fake>>) -> Json<Value> {
    let mut polls = fake.login_polls.lock().unwrap();
    *polls += 1;
    Json(if *polls == 1 {
        json!({ "status": "pending", "interval": 0 })
    } else {
        json!({ "status": "ok", "token": "long-lived", "login": "octo", "display_name": "Octo Cat", "migrated_sites": 2 })
    })
}

async fn anon_sessions() -> Json<Value> {
    Json(json!({ "token": "tok-1", "expires_at": "2099-01-01T00:00:00Z" }))
}

/// 作品链接：默认那条谁也连不上的，或者这个假服务器自己的地址。
fn site_url(fake: &Fake) -> String {
    fake.site_url
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(|| format!("http://{SLUG}.localhost:8443"))
}

async fn create_site(State(fake): State<Arc<Fake>>) -> Json<Value> {
    Json(json!({
        "slug": SLUG,
        "url": site_url(&fake),
        "title": "export",
        "created_at": "2026-09-07T00:00:00Z",
        "expires_at": "2026-09-08T03:30:00Z",
    }))
}

async fn list_sites(State(fake): State<Arc<Fake>>) -> Json<Value> {
    Json(json!([{
        "slug": SLUG,
        "url": site_url(&fake),
        "title": "小球试玩",
        "current_version": 3,
        "created_at": "2026-09-07T00:00:00Z",
        "expires_at": "2026-09-08T03:30:00Z",
        "listing": { "followers": 12 },
    }]))
}

/// 一个作品此刻的样子。没有封面——`--cover` 那一句提醒要有东西可依据。
async fn get_site(State(fake): State<Arc<Fake>>, UrlPath(slug): UrlPath<String>) -> Json<Value> {
    Json(json!({
        "slug": slug,
        "url": site_url(&fake),
        "title": "小球试玩",
        "current_version": 7,
        "created_at": "2026-09-07T00:00:00Z",
        "listing": { "has_cover": false, "followers": 12 },
    }))
}

/// 改状态。把收到的东西原样回出去，CLI 报的就该是服务器认下来的那份。
async fn patch_site(
    State(fake): State<Arc<Fake>>,
    UrlPath(slug): UrlPath<String>,
    Json(request): Json<Value>,
) -> Json<Value> {
    *fake.patched.lock().unwrap() = Some(request.clone());
    Json(json!({
        "slug": slug,
        "url": site_url(&fake),
        "title": "小球试玩",
        "current_version": 7,
        "created_at": "2026-09-07T00:00:00Z",
        "listing": {
            "public": request["public"].as_bool().unwrap_or(false),
            "seeking": request["seeking"].as_bool().unwrap_or(false),
            "seek_note": request["seek_note"],
            "seats": request["seats"],
            "community_url": request["community_url"],
            "has_cover": false,
            "followers": 12,
        },
    }))
}

async fn list_versions(UrlPath(slug): UrlPath<String>) -> Json<Value> {
    Json(json!({
        "slug": slug,
        "current_version": 7,
        "versions": [
            { "version": 7, "created_at": "2026-09-09T00:00:00Z", "note": "改了新手引导",
              "file_count": 53, "total_bytes": 2_200_000, "current": true },
            { "version": 3, "created_at": "2026-09-08T00:00:00Z", "note": null,
              "file_count": 50, "total_bytes": 2_000_000, "current": false },
        ],
    }))
}

async fn activate_version(
    State(fake): State<Arc<Fake>>,
    UrlPath((slug, version)): UrlPath<(String, u32)>,
) -> Json<Value> {
    Json(json!({
        "slug": slug,
        "url": site_url(&fake),
        "title": "小球试玩",
        "current_version": version,
        "created_at": "2026-09-07T00:00:00Z",
        "listing": { "has_cover": false, "followers": 12 },
    }))
}

/// 边缘那一侧：门禁页上那张邀请卡。
async fn card_png() -> Response {
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "image/png")],
        CARD_BYTES,
    )
        .into_response()
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

async fn commit_upload(
    State(fake): State<Arc<Fake>>,
    UrlPath((slug, _upload)): UrlPath<(String, String)>,
) -> Json<Value> {
    Json(json!({
        "slug": slug,
        "version": 7,
        "url": site_url(&fake),
        "expires_at": "2026-09-08T03:30:00Z",
    }))
}

/// 起一个假控制面，返回它的地址。线程随进程一起结束。
fn start_fake(refuse_prepare: Option<Refusal>) -> String {
    start_with(
        Arc::new(Fake {
            refuse_prepare: Mutex::new(refuse_prepare),
            ..Fake::default()
        }),
        false,
    )
}

/// 同一个假服务器，但它同时假装自己是边缘：作品链接指回它自己，
/// `/_playtest/card.png` 上真有一张卡。要验邀请卡的测试用这个。
fn start_fake_as_edge() -> Arc<Fake> {
    let fake = Arc::new(Fake::default());
    start_with(Arc::clone(&fake), true);
    fake
}

/// 假服务器的地址。起的时候就记在 `site_url` 里了（当边缘用的时候）。
fn api_of(fake: &Fake) -> String {
    fake.site_url.lock().unwrap().clone().unwrap()
}

fn start_with(fake: Arc<Fake>, as_edge: bool) -> String {
    let (tx, rx) = std::sync::mpsc::channel::<SocketAddr>();
    let state = Arc::clone(&fake);
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            let app = Router::new()
                .route(routes::ANON_SESSIONS, post(anon_sessions))
                .route(routes::LOGIN_DEVICE_START, post(device_start))
                .route(routes::LOGIN_DEVICE_POLL, post(device_poll))
                .route(routes::LOGIN_WEB_START, get(login_unavailable))
                .route(routes::SITES, post(create_site).get(list_sites))
                .route(routes::SITE, get(get_site).patch(patch_site))
                .route(routes::SITE_VERSIONS, get(list_versions))
                .route(routes::SITE_VERSION_ACTIVATE, post(activate_version))
                .route(routes::SITE_UPLOADS, post(prepare_upload))
                .route(routes::BLOB, put(put_blob))
                .route(routes::SITE_UPLOAD_COMMIT, post(commit_upload))
                .route(playtest_common::CARD_PATH, get(card_png))
                .route(routes::HEALTH, get(|| async { "ok" }))
                .with_state(state);
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            tx.send(listener.local_addr().unwrap()).unwrap();
            axum::serve(listener, app).await.unwrap();
        });
    });
    let api = format!("http://{}", rx.recv().expect("假控制面没起来"));
    // 端口是绑完才知道的，所以「作品链接指回自己」只能在这一刻填。
    if as_edge {
        *fake.site_url.lock().unwrap() = Some(api.clone());
    }
    api
}

// ---------------------------------------------------------------- 跑二进制

fn run_cli(home: &Path, api: &str, args: &[&str]) -> Output {
    run_cli_in(home, home, api, args)
}

/// 同上，但指定在哪个目录里跑——邀请卡默认存到「当前目录」，所以那几个测试要管这件事。
fn run_cli_in(cwd: &Path, home: &Path, api: &str, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_playtest"));
    command
        .args(args)
        .current_dir(cwd)
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
    for segment in ["hash_ms", "prepare_ms", "upload_ms", "commit_ms"] {
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
fn an_upload_saves_the_invite_card_next_to_you_and_says_so_in_the_object() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());
    let fake = start_fake_as_edge();
    let api = api_of(&fake);

    let output = run_cli_in(
        work.path(),
        home.path(),
        &api,
        &["--json", "--no-qr", dist.to_str().unwrap()],
    );
    assert!(output.status.success(), "{}", stderr_of(&output));
    let value = only_object(&output);

    // 卡的地址一直有效，所以它总在；存下来了才有 card_path。
    assert_eq!(value["card_url"], format!("{api}/_playtest/card.png"));
    assert_eq!(value["card_path"], "./dist-邀请卡.png", "文件名是作品名");
    let saved = work.path().join("dist-邀请卡.png");
    assert_eq!(
        std::fs::read(&saved).unwrap(),
        CARD_BYTES,
        "存下来的就是那张卡"
    );

    // 「去哪儿看结果」是开发者那一侧的地址，不是玩家域。
    let console = value["console_url"].as_str().unwrap();
    assert!(
        console.ends_with(&format!("/console/#/s/{SLUG}")),
        "{console}"
    );
    assert!(
        !console.contains("playtest.run"),
        "控制台在开发者域上：{console}"
    );

    // 这一版没有封面，提醒一句，但不拦。
    assert!(
        value["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["message"].as_str().unwrap_or("").contains("没有封面")),
        "{value}"
    );
}

#[test]
fn no_card_means_no_file_and_no_path() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());
    let fake = start_fake_as_edge();

    let output = run_cli_in(
        work.path(),
        home.path(),
        &api_of(&fake),
        &["--json", "--no-qr", "--card", "-", dist.to_str().unwrap()],
    );
    assert!(output.status.success(), "{}", stderr_of(&output));
    let value = only_object(&output);
    assert!(value.get("card_path").is_none(), "{value}");
    assert!(value["card_url"].is_string(), "地址还是要给的：{value}");
    assert!(
        !work.path().join("dist-邀请卡.png").exists(),
        "说了不要就一个文件都不该留下"
    );
}

#[test]
fn seats_and_a_group_link_go_up_with_the_version() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());
    let fake = start_fake_as_edge();

    let output = run_cli_in(
        work.path(),
        home.path(),
        &api_of(&fake),
        &[
            "--json",
            "--no-qr",
            "--card",
            "-",
            dist.to_str().unwrap(),
            "--seats",
            "10",
            "--community",
            "https://t.me/playtest",
        ],
    );
    assert!(output.status.success(), "{}", stderr_of(&output));

    let patched = fake.patched.lock().unwrap().clone().expect("该改一次状态");
    assert_eq!(patched["seats"], 10);
    assert_eq!(patched["community_url"], "https://t.me/playtest");
    assert_eq!(patched["public"], true, "想找人测就得在广场上：{patched}");
    assert_eq!(patched["seeking"], true, "{patched}");
    assert!(
        patched.get("feedback_public").is_none(),
        "CLI 这一轮不碰这一项：{patched}"
    );

    let value = only_object(&output);
    assert_eq!(value["seats"], 10);
    assert!(
        value["plaza_url"].as_str().unwrap().starts_with("http"),
        "{value}"
    );
}

#[test]
fn seats_out_of_range_is_caught_here_not_after_the_upload() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());
    // 连不上任何服务器也该在本地就拦下来——这是输入的问题，不是网络的问题。
    let output = run_cli(
        home.path(),
        "http://127.0.0.1:1",
        &["--json", dist.to_str().unwrap(), "--seats", "99999"],
    );
    let value = expect_failure(&output, 6, "bad_input");
    let message = value["message"].as_str().unwrap();
    assert!(message.contains("99999"), "要把他写的那个数说出来：{value}");
    assert!(
        message.contains(&playtest_common::limits::MAX_SEATS.to_string()),
        "也要把上限说出来：{value}"
    );
}

#[test]
fn a_group_link_that_a_browser_cannot_open_is_refused_before_anything_is_sent() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());
    let output = run_cli(
        home.path(),
        "http://127.0.0.1:1",
        &[
            "--json",
            dist.to_str().unwrap(),
            "--community",
            "qq://12345",
        ],
    );
    let value = expect_failure(&output, 6, "bad_input");
    assert!(
        value["message"].as_str().unwrap().contains("http"),
        "{value}"
    );
}

#[test]
fn the_card_command_fetches_one_more_copy() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());
    let fake = start_fake_as_edge();
    let api = api_of(&fake);
    // 先发一次，把令牌和这个目录记下来。
    let first = run_cli_in(
        work.path(),
        home.path(),
        &api,
        &["--json", "--no-qr", "--card", "-", dist.to_str().unwrap()],
    );
    assert!(first.status.success(), "{}", stderr_of(&first));

    let out = work.path().join("卡片").join("邀请.png");
    let output = run_cli_in(
        work.path(),
        home.path(),
        &api,
        &["--json", "card", SLUG, "--out", out.to_str().unwrap()],
    );
    assert!(output.status.success(), "{}", stderr_of(&output));
    let value = only_object(&output);
    assert_eq!(value["action"], "card");
    assert_eq!(value["slug"], SLUG);
    assert_eq!(std::fs::read(&out).unwrap(), CARD_BYTES);
    assert!(
        value["card_path"].as_str().unwrap().contains("邀请.png"),
        "{value}"
    );

    // 目录也行：这个目录上次发到哪个作品，我们记着。
    let output = run_cli_in(
        work.path(),
        home.path(),
        &api,
        &["--json", "card", dist.to_str().unwrap()],
    );
    assert!(output.status.success(), "{}", stderr_of(&output));
    assert!(work.path().join("小球试玩-邀请卡.png").exists());
}

#[test]
fn how_many_are_waiting_shows_up_in_the_list() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());
    let fake = start_fake_as_edge();
    let api = api_of(&fake);
    let first = run_cli_in(
        work.path(),
        home.path(),
        &api,
        &["--json", "--no-qr", "--card", "-", dist.to_str().unwrap()],
    );
    assert!(first.status.success(), "{}", stderr_of(&first));

    // 关注数在 ls 里，不再单开一条命令（REWRITE §3.1）。
    let listed = run_cli(home.path(), &api, &["ls", "--json"]);
    assert!(listed.status.success(), "{}", stderr_of(&listed));
    let one = &only_object(&listed)["sites"][0];
    assert_eq!(one["followers"], 12);
    assert_eq!(one["title"], "小球试玩");

    // 那条命令真的没了，而不是悄悄留着。
    let gone = run_cli(home.path(), &api, &["--json", "followers", SLUG]);
    assert!(!gone.status.success(), "followers 已经并进 ls");
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

/// `versions / rollback / unlist` 以前只有人话那一半：`rollback --json` 往 stdout 写
/// 一条裸链接，`versions --json` 什么都不写。三条现在都产出同一种对象（REWRITE §3.1）。
#[test]
fn every_command_answers_in_one_object_in_machine_mode() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());
    let api = start_fake(None);
    let first = run_cli(home.path(), &api, &["--json", dist.to_str().unwrap()]);
    assert!(first.status.success(), "{}", stderr_of(&first));

    let output = run_cli(home.path(), &api, &["versions", SLUG, "--json"]);
    assert!(output.status.success(), "{}", stderr_of(&output));
    let value = only_object(&output);
    assert_eq!(value["action"], "versions");
    assert_eq!(value["current_version"], 7);
    assert_eq!(value["versions"][0]["version"], 7);
    assert_eq!(value["versions"][0]["note"], "改了新手引导");
    assert!(value["versions"][1]["note"].is_null(), "没写就是 null");

    let output = run_cli(home.path(), &api, &["rollback", SLUG, "v3", "--json"]);
    assert!(output.status.success(), "{}", stderr_of(&output));
    let value = only_object(&output);
    assert_eq!(value["action"], "rollback");
    assert_eq!(value["version"], 3);
    assert_eq!(value["slug"], SLUG);

    let output = run_cli(home.path(), &api, &["unlist", SLUG, "--json"]);
    assert!(output.status.success(), "{}", stderr_of(&output));
    let value = only_object(&output);
    assert_eq!(value["action"], "unlist");
    assert_eq!(value["listed"], false);
}

/// 版本号写错的时候说清楚，不去猜一个。
#[test]
fn rolling_back_to_something_that_is_not_a_version_says_so() {
    let home = tempfile::tempdir().unwrap();
    let api = start_fake(None);
    let output = run_cli(home.path(), &api, &["rollback", SLUG, "最新", "--json"]);
    let value = expect_failure(&output, 6, "bad_input");
    assert!(
        value["message"].as_str().unwrap().contains("v3"),
        "要告诉他该怎么写：{value}"
    );
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
fn login_polls_until_github_says_yes_and_keeps_a_long_lived_token() {
    let home = tempfile::tempdir().unwrap();
    let api = start_fake(None);
    let output = run_cli(home.path(), &api, &["--json", "login"]);
    assert!(output.status.success(), "{}", stderr_of(&output));
    let value = only_object(&output);
    assert_eq!(value["ok"], true);
    assert_eq!(value["action"], "login");
    assert_eq!(value["login"], "octo");
    assert_eq!(value["migrated_sites"], 2);
    // 机器模式下码和网址走 stderr，调用方能转给人。
    let stderr = stderr_of(&output);
    assert!(stderr.contains("WDJB-MJHT"), "{stderr}");

    // 令牌落在配置里，没有到期时间，记着是谁。
    let config_path = home
        .path()
        .join(".config")
        .join("playtest")
        .join("config.json");
    let config: Value =
        serde_json::from_str(&std::fs::read_to_string(config_path).unwrap()).unwrap();
    assert_eq!(config["token"], "long-lived");
    assert_eq!(config["login"], "octo");
    assert!(config.get("token_expires_at").is_none(), "{config}");
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

    // 「几秒」是数字：结尾那行给总数。对着本机假服务器一下就完，分段不打（那是慢的时候才有用的）。
    let stderr = stderr_of(&output);
    assert!(stderr.contains("本次 "), "{stderr}");
    assert!(stderr.contains(" 秒"), "{stderr}");
    assert!(!stderr.contains("（哈希 0.0"), "四个 0.0 是噪声：{stderr}");
}

/// 发完那一屏，从上到下就是「接下来做什么」的次序（DESIGN §3.2）。
///
/// 这里连着真的二进制跑，看的是它真打出来的那几行——单元测试验的是拼句子，这里验的是
/// 这些句子确实按那个顺序落到了终端上。
#[test]
fn what_a_first_timer_sees_after_publishing_comes_in_one_useful_order() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());
    let fake = start_fake_as_edge();

    let output = run_cli_in(
        work.path(),
        home.path(),
        &api_of(&fake),
        &[
            dist.to_str().unwrap(),
            "--no-qr",
            "--seats",
            "10",
            "--community",
            "https://t.me/playtest",
            "--summary",
            "三关，五分钟能玩完",
        ],
    );
    assert!(output.status.success(), "{}", stderr_of(&output));

    let stderr = stderr_of(&output);
    let said: Vec<&str> = stderr.lines().filter(|l| !l.trim().is_empty()).collect();
    let order = [
        "已发布",
        "邀请卡已存到",
        "已放到广场上",
        "想找 10 位试玩者",
        // 快到期时这一句会换成提醒，两种说法里都有「匿名链接」这四个字。
        "匿名链接",
        "来的人玩成什么样",
        "没有封面",
        "本次 ",
    ];
    let mut at = 0usize;
    for head in order {
        at = said[at..]
            .iter()
            .position(|line| line.contains(head))
            .map(|i| at + i + 1)
            .unwrap_or_else(|| panic!("「{head}」没按顺序出现：\n{stderr}"));
    }
    // 链接只在 stdout 上，方便 `| pbcopy`。
    assert_eq!(stdout_of(&output).trim(), api_of(&fake));
}
