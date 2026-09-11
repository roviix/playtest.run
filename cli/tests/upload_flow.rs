//! 对着一个假控制面跑真的 `playtest` 二进制，看整条上传路径。
//!
//! 假服务器直接用 `playtest_common::api::routes` 里的常量注册路由——axum 0.8 的
//! `{参数}` 写法和那些常量正好一样，所以路径写错了这里就会挂。

use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use axum::extract::{Path as UrlPath, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{post, put};
use axum::{Json, Router};
use playtest_common::api::{
    routes, AnonSessionResponse, CommitUploadResponse, CreateSiteRequest, ErrorBody, ErrorCode,
    PrepareUploadRequest, PrepareUploadResponse, Site,
};
use playtest_common::hash::hash_bytes;
use playtest_common::manifest::GateMode;

const NEW_SLUG: &str = "brisk-otter-41";
/// 配置里记着、但服务器已经不认的那个作品。
const GONE_SLUG: &str = "gone-site-9";

// ---------------------------------------------------------------- 假控制面

#[derive(Default, Clone, Copy)]
struct Behaviour {
    /// 对 `GONE_SLUG` 的 prepare 回 401 token_expired。
    remembered_slug_expired: bool,
    /// 所有 PUT 都回 hash_mismatch。
    always_hash_mismatch: bool,
}

#[derive(Default)]
struct Recorded {
    calls: Vec<String>,
    prepared: Vec<PrepareUploadRequest>,
    blobs: Vec<(String, Vec<u8>)>,
    sessions: u32,
}

struct Fake {
    log: Mutex<Recorded>,
    already_have: HashSet<String>,
    behaviour: Behaviour,
}

impl Fake {
    fn record(&self, call: String) {
        self.log.lock().unwrap().calls.push(call);
    }

    fn calls(&self) -> Vec<String> {
        self.log.lock().unwrap().calls.clone()
    }
}

fn refuse(status: StatusCode, code: ErrorCode, message: &str) -> Response {
    (
        status,
        Json(ErrorBody {
            code,
            message: message.to_string(),
        }),
    )
        .into_response()
}

async fn anon_sessions(State(fake): State<Arc<Fake>>) -> Json<AnonSessionResponse> {
    let mut log = fake.log.lock().unwrap();
    log.calls.push("POST /v1/anon/sessions".into());
    log.sessions += 1;
    Json(AnonSessionResponse {
        token: format!("tok-{}", log.sessions),
        expires_at: "2099-01-01T00:00:00Z".into(),
    })
}

async fn create_site(
    State(fake): State<Arc<Fake>>,
    Json(request): Json<CreateSiteRequest>,
) -> Json<Site> {
    fake.record("POST /v1/projects".into());
    Json(Site {
        slug: NEW_SLUG.into(),
        url: format!("http://{NEW_SLUG}.localhost:8443"),
        title: request.title.unwrap_or_default(),
        current_version: None,
        created_at: "2026-09-07T00:00:00Z".into(),
        expires_at: Some("2026-09-08T03:30:00Z".into()),
        listing: Default::default(),
    })
}

async fn prepare_upload(
    State(fake): State<Arc<Fake>>,
    UrlPath(slug): UrlPath<String>,
    Json(request): Json<PrepareUploadRequest>,
) -> Response {
    fake.record(format!("POST /v1/projects/{slug}/uploads"));
    if fake.behaviour.remembered_slug_expired && slug == GONE_SLUG {
        return refuse(
            StatusCode::UNAUTHORIZED,
            ErrorCode::TokenExpired,
            "这个匿名链接已经过期了",
        );
    }

    let mut missing: Vec<String> = Vec::new();
    let mut missing_bytes = 0u64;
    for file in &request.files {
        if !fake.already_have.contains(&file.hash) && !missing.contains(&file.hash) {
            missing.push(file.hash.clone());
            missing_bytes += file.size;
        }
    }
    fake.log.lock().unwrap().prepared.push(request);
    Json(PrepareUploadResponse {
        upload_id: "upload-1".into(),
        missing,
        missing_bytes,
    })
    .into_response()
}

async fn put_blob(
    State(fake): State<Arc<Fake>>,
    UrlPath(hash): UrlPath<String>,
    body: axum::body::Bytes,
) -> Response {
    {
        let mut log = fake.log.lock().unwrap();
        log.calls.push(format!("PUT /v1/blobs/{hash}"));
        log.blobs.push((hash.clone(), body.to_vec()));
    }
    if fake.behaviour.always_hash_mismatch || hash_bytes(&body) != hash {
        return refuse(
            StatusCode::BAD_REQUEST,
            ErrorCode::HashMismatch,
            "收到的内容和这个哈希对不上",
        );
    }
    StatusCode::CREATED.into_response()
}

async fn commit_upload(
    State(fake): State<Arc<Fake>>,
    UrlPath((slug, upload_id)): UrlPath<(String, String)>,
) -> Json<CommitUploadResponse> {
    fake.record(format!("POST /v1/projects/{slug}/uploads/{upload_id}/commit"));
    Json(CommitUploadResponse {
        slug: slug.clone(),
        version: 1,
        url: format!("http://{slug}.localhost:8443"),
        expires_at: Some("2026-09-08T03:30:00Z".into()),
    })
}

/// 起一个假控制面，返回它的地址和记录本。线程随进程一起结束。
fn start_fake(behaviour: Behaviour, already_have: HashSet<String>) -> (String, Arc<Fake>) {
    let fake = Arc::new(Fake {
        log: Mutex::new(Recorded::default()),
        already_have,
        behaviour,
    });
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

fn write(path: &Path, contents: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

/// 一个像样的导出目录：`assets/old.txt` 假装服务器上已经有了。
fn make_export(root: &Path) -> PathBuf {
    let dist = root.join("dist");
    write(&dist.join("index.html"), b"<html>GODOT</html>");
    write(&dist.join("game.js"), b"console.log(1)");
    write(&dist.join("assets/old.txt"), b"old");
    // 这两个不该出现在清单里。
    write(&dist.join(".DS_Store"), b"junk");
    write(&dist.join(".git/HEAD"), b"ref: refs/heads/main");
    dist
}

fn config_path(home: &Path) -> PathBuf {
    home.join(".config").join("playtest").join("config.json")
}

fn read_config(home: &Path) -> serde_json::Value {
    let bytes = std::fs::read(config_path(home)).expect("配置文件没写出来");
    serde_json::from_slice(&bytes).expect("配置文件不是 JSON")
}

fn canonical(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// PUT 是并发的，顺序不定；其余几步的先后顺序才是要断言的东西。
fn steps_without_puts(calls: &[String]) -> Vec<String> {
    calls
        .iter()
        .filter(|c| !c.starts_with("PUT"))
        .cloned()
        .collect()
}

// ---------------------------------------------------------------- 测试

#[test]
fn uploads_only_the_blobs_the_server_is_missing() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());

    let already_have = HashSet::from([hash_bytes(b"old")]);
    let (api, fake) = start_fake(Behaviour::default(), already_have);

    let output = run_cli(home.path(), &api, &[dist.to_str().unwrap(), "--no-qr"]);
    assert!(output.status.success(), "{}", stderr_of(&output));

    // stdout 上只有链接，方便 `| pbcopy`。
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("http://{NEW_SLUG}.localhost:8443\n")
    );

    let calls = fake.calls();
    assert_eq!(
        steps_without_puts(&calls),
        [
            "POST /v1/anon/sessions".to_string(),
            "POST /v1/projects".to_string(),
            format!("POST /v1/projects/{NEW_SLUG}/uploads"),
            format!("POST /v1/projects/{NEW_SLUG}/uploads/upload-1/commit"),
        ]
    );

    // 每个 PUT 都夹在 prepare 和 commit 中间。
    let prepare_at = calls.iter().position(|c| c.ends_with("/uploads")).unwrap();
    let commit_at = calls.iter().position(|c| c.ends_with("/commit")).unwrap();
    for (i, call) in calls.iter().enumerate() {
        if call.starts_with("PUT") {
            assert!(i > prepare_at && i < commit_at, "PUT 的位置不对：{calls:?}");
        }
    }

    // 只传了缺的那两个，内容也对得上。
    let log = fake.log.lock().unwrap();
    let mut sent: Vec<String> = log.blobs.iter().map(|(h, _)| h.clone()).collect();
    sent.sort();
    let mut expected = vec![
        hash_bytes(b"<html>GODOT</html>"),
        hash_bytes(b"console.log(1)"),
    ];
    expected.sort();
    assert_eq!(sent, expected);
    for (hash, body) in &log.blobs {
        assert_eq!(&hash_bytes(body), hash, "传上去的字节和哈希对不上");
    }

    // 清单：按路径排序、不含以「.」开头的东西、哈希和大小都对。
    let prepared = log.prepared.first().expect("没收到 prepare");
    let paths: Vec<&str> = prepared.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["assets/old.txt", "game.js", "index.html"]);
    let index = &prepared.files[2];
    assert_eq!(index.hash, hash_bytes(b"<html>GODOT</html>"));
    assert_eq!(index.size, 18);
    assert_eq!(prepared.title.as_deref(), Some("dist"));
    assert_eq!(prepared.gate, GateMode::Once);
    assert!(!prepared.isolated);
    assert!(!prepared.spa);

    // 这个目录记住了它的 slug，令牌也存下来了。
    let config = read_config(home.path());
    assert_eq!(config["sites"][canonical(&dist)], NEW_SLUG);
    assert_eq!(config["token"], "tok-1");
    assert_eq!(config["api"], api);
}

#[test]
fn a_second_run_reuses_the_remembered_site_and_uploads_nothing() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());
    let (api, fake) = start_fake(Behaviour::default(), HashSet::new());

    let first = run_cli(home.path(), &api, &[dist.to_str().unwrap(), "--no-qr"]);
    assert!(first.status.success(), "{}", stderr_of(&first));

    // 第一次传上去的三个，第二次服务器就都有了。
    let uploaded: HashSet<String> = fake
        .log
        .lock()
        .unwrap()
        .blobs
        .iter()
        .map(|(h, _)| h.clone())
        .collect();
    assert_eq!(uploaded.len(), 3);

    let (api2, fake2) = start_fake(Behaviour::default(), uploaded);
    // 第二次换了控制面地址，令牌就不能复用了，所以仍会先要一个匿名会话；
    // 要验的是「作品沿用记住的那个」和「一个字节都不用传」。
    let second = run_cli(home.path(), &api2, &[dist.to_str().unwrap(), "--no-qr"]);
    assert!(second.status.success(), "{}", stderr_of(&second));
    assert!(
        fake2.log.lock().unwrap().blobs.is_empty(),
        "不该再传任何文件"
    );
    assert!(
        stderr_of(&second).contains("都已经有了"),
        "{}",
        stderr_of(&second)
    );
    let calls = fake2.calls();
    assert!(
        calls
            .iter()
            .any(|c| c == &format!("POST /v1/projects/{NEW_SLUG}/uploads")),
        "应该沿用记住的作品：{calls:?}"
    );
    assert!(
        !calls.iter().any(|c| c == "POST /v1/projects"),
        "记住了就不该再新建作品：{calls:?}"
    );
}

#[test]
fn an_expired_anonymous_link_is_replaced_with_a_new_one() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());

    let (api, fake) = start_fake(
        Behaviour {
            remembered_slug_expired: true,
            ..Behaviour::default()
        },
        HashSet::new(),
    );

    // 配置里有一个看起来还没过期的令牌，和一个服务器已经不认的作品。
    let config = serde_json::json!({
        "api": api,
        "token": "stale-token",
        "token_expires_at": "2099-01-01T00:00:00Z",
        "sites": { canonical(&dist): GONE_SLUG },
    });
    let path = config_path(home.path());
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();

    let output = run_cli(home.path(), &api, &[dist.to_str().unwrap(), "--no-qr"]);
    assert!(output.status.success(), "{}", stderr_of(&output));
    assert!(
        stderr_of(&output).contains("上次的匿名链接已过期"),
        "得告诉用户链接换了：{}",
        stderr_of(&output)
    );

    assert_eq!(
        steps_without_puts(&fake.calls()),
        [
            // 先拿着旧令牌去问记住的那个作品，被拒。
            format!("POST /v1/projects/{GONE_SLUG}/uploads"),
            // 于是换一个匿名会话、建一个新作品，再来一次。
            "POST /v1/anon/sessions".to_string(),
            "POST /v1/projects".to_string(),
            format!("POST /v1/projects/{NEW_SLUG}/uploads"),
            format!("POST /v1/projects/{NEW_SLUG}/uploads/upload-1/commit"),
        ]
    );

    let saved = read_config(home.path());
    assert_eq!(saved["sites"][canonical(&dist)], NEW_SLUG);
    assert_eq!(saved["token"], "tok-1");
}

#[test]
fn a_file_that_changes_mid_upload_is_reported_not_retried_forever() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = work.path().join("dist");
    write(&dist.join("index.html"), b"<html>hi</html>");

    let (api, fake) = start_fake(
        Behaviour {
            always_hash_mismatch: true,
            ..Behaviour::default()
        },
        HashSet::new(),
    );

    let output = run_cli(home.path(), &api, &[dist.to_str().unwrap(), "--no-qr"]);
    // 退出码 6：本地输入的问题（文件在上传中途变了）。
    assert_eq!(output.status.code(), Some(6), "{}", stderr_of(&output));
    assert!(
        stderr_of(&output).contains("上传时文件变了"),
        "{}",
        stderr_of(&output)
    );

    let puts = fake.calls().iter().filter(|c| c.starts_with("PUT")).count();
    assert_eq!(puts, 3, "最多试三次");
    assert!(
        !fake.calls().iter().any(|c| c.ends_with("/commit")),
        "传不上去就不该提交"
    );
}

// 端口那条路现在走隧道，不再是「还没做好」，它的行为在 tunnel_flow.rs 里测。

// `playtest login` 的设备码流程在 json_output.rs 里对着假控制面测；这里只看连不上时它不假装成功。
#[test]
fn login_with_no_control_plane_fails_like_everything_else() {
    let home = tempfile::tempdir().unwrap();
    let output = run_cli(home.path(), "http://127.0.0.1:1", &["login"]);
    assert_eq!(output.status.code(), Some(4), "{}", stderr_of(&output));
    assert!(
        stderr_of(&output).contains("连不上服务器"),
        "{}",
        stderr_of(&output)
    );
}

#[test]
fn a_missing_directory_and_a_plain_file_each_get_their_own_message() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let missing = work.path().join("nope");
    let output = run_cli(
        home.path(),
        "http://127.0.0.1:1",
        &[missing.to_str().unwrap()],
    );
    // 退出码 6：给的东西有问题（见 `playtest --help` 尾部）。
    assert_eq!(output.status.code(), Some(6));
    assert!(
        stderr_of(&output).contains("找不到"),
        "{}",
        stderr_of(&output)
    );

    let file = work.path().join("index.html");
    write(&file, b"<html></html>");
    let output = run_cli(home.path(), "http://127.0.0.1:1", &[file.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(6));
    assert!(
        stderr_of(&output).contains("请给目录，不是文件"),
        "{}",
        stderr_of(&output)
    );

    let empty = work.path().join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let output = run_cli(
        home.path(),
        "http://127.0.0.1:1",
        &[empty.to_str().unwrap()],
    );
    assert_eq!(output.status.code(), Some(6));
    assert!(
        stderr_of(&output).contains("是空的"),
        "{}",
        stderr_of(&output)
    );
}

#[test]
fn the_server_cannot_be_reached() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dist = make_export(work.path());
    // 1 号端口上不会有人在听。
    let output = run_cli(home.path(), "http://127.0.0.1:1", &[dist.to_str().unwrap()]);
    // 退出码 4：网络不通。
    assert_eq!(output.status.code(), Some(4));
    let stderr = stderr_of(&output);
    assert!(stderr.contains("连不上服务器"), "{stderr}");
    assert!(stderr.contains("--api"), "{stderr}");
}
