//! 各个处理函数共用的东西。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use playtest_common::store::Store;
use playtest_common::tunnel::SigningKey;
use tokio::sync::{Mutex, MutexGuard};

use crate::config::{Config, GitHubApp, SLUG_PLACEHOLDER};
use crate::db::Db;
use crate::notify;
use crate::tunnel_keys;

/// 网页登录的 `state` 从签发到 GitHub 回来之间最多等这么久。
const LOGIN_STATE_TTL: Duration = Duration::from_secs(10 * 60);
/// 同时挂着的 `state` 上限：超过就先清过期的，还超就拒——这是防被人刷满内存，不是配额。
const LOGIN_STATE_CAP: usize = 4096;

#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,
}

struct Inner {
    db: Db,
    store: Store,
    site_url_template: String,
    tunnel_key: SigningKey,
    commit_lock: Mutex<()>,
    github: Option<GitHubApp>,
    http: reqwest::Client,
    notify: Arc<notify::Runtime>,
    admin_token: Option<String>,
    edge_ingest_token: Option<String>,
    data_dir: std::path::PathBuf,
    /// 后台任务的现状（`scheduler::Board`）。
    jobs: crate::scheduler::Board,
    /// 网页登录流程里发出去、还没回来的 `state`。进程内存里就够：控制面是单进程（DESIGN §4.5）。
    login_states: std::sync::Mutex<HashMap<String, Instant>>,
}

impl AppState {
    pub fn new(
        db: Db,
        store: Store,
        tunnel_key: SigningKey,
        config: &Config,
    ) -> anyhow::Result<Self> {
        // reqwest 的 rustls-no-provider 没有内置加密后端，建 Client 前必须先装一个。
        let _ = rustls::crypto::ring::default_provider().install_default();
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(20))
            .user_agent(concat!("playtest-api/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("reqwest 客户端用的都是固定配置，建不出来说明构建本身坏了");
        let vapid = notify::load_or_create_vapid(&config.data_dir)?;
        let notify = notify::Runtime::new(config.notify.clone(), vapid, http.clone())?;
        Ok(Self {
            inner: Arc::new(Inner {
                db,
                store,
                site_url_template: config.site_url_template.clone(),
                tunnel_key,
                commit_lock: Mutex::new(()),
                github: config.github.clone(),
                http,
                notify,
                admin_token: config.admin_token.clone(),
                edge_ingest_token: config.edge_ingest_token.clone(),
                data_dir: config.data_dir.clone(),
                jobs: crate::scheduler::Board::default(),
                login_states: std::sync::Mutex::new(HashMap::new()),
            }),
        })
    }

    /// 按配置建数据目录、开库、备好对象存储。目录不存在就建。
    pub async fn from_config(config: &Config) -> anyhow::Result<Self> {
        tokio::fs::create_dir_all(&config.data_dir).await?;
        let db = Db::open(&config.sqlite_path())?;
        let store = Store::from_config(&config.store_config()?)?;
        // 后端写不进、读不回或删不掉时直接拒绝启动；尤其不能在 S3 坏时落回本地盘。
        store.probe_write().await?;
        // 顺带把隧道公钥发布到对象存储：边缘要能在控制面之后、之外单独起来。
        let tunnel_key = tunnel_keys::load_or_create(
            &config.data_dir,
            &store,
            tunnel_keys::key_from_env().as_deref(),
        )
        .await?;
        let state = Self::new(db, store, tunnel_key, config)?;
        crate::quota::reconcile(&state).await?;
        Ok(state)
    }

    pub fn db(&self) -> &Db {
        &self.inner.db
    }

    pub fn store(&self) -> &Store {
        &self.inner.store
    }

    /// 隧道令牌的签名私钥（[`crate::tunnel_keys`]）。
    pub fn tunnel_key(&self) -> &SigningKey {
        &self.inner.tunnel_key
    }

    /// 玩家点开的链接。
    pub fn site_url(&self, slug: &str) -> String {
        self.inner.site_url_template.replace(SLUG_PLACEHOLDER, slug)
    }

    /// GitHub 登录的配置；`None` 就是这个控制面不提供登录。
    pub fn github(&self) -> Option<&GitHubApp> {
        self.inner.github.as_ref()
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.inner.http
    }

    /// 发信与推送要用的东西（[`crate::notify`]）。
    pub fn notify(&self) -> &notify::Runtime {
        &self.inner.notify
    }

    /// 管理接口的令牌。`None` 就是这台机器不开 `/admin/*`。
    pub fn admin_token(&self) -> Option<&str> {
        self.inner.admin_token.as_deref()
    }

    /// 边缘写入批量事件时携带的共享凭据。没配置就拒绝所有写入，而不是退回公开入口。
    pub fn edge_ingest_token(&self) -> Option<&str> {
        self.inner.edge_ingest_token.as_deref()
    }

    /// 数据目录。周报的时间戳这类「跟迁移无关的一行状态」放在这里。
    /// 后台任务板：每个任务上次什么时候跑、结果如何。
    pub fn jobs(&self) -> &crate::scheduler::Board {
        &self.inner.jobs
    }

    /// 测试用：临时目录里一套完整的状态。目录随返回值一起活着（泄漏给测试进程）。
    #[cfg(test)]
    pub async fn for_tests() -> Self {
        let dir = tempfile::tempdir().expect("建不了临时目录");
        let config = Config {
            listen: "127.0.0.1:0".parse().expect("字面量"),
            data_dir: dir.keep(),
            site_url_template: "http://{slug}.localhost:8443".to_string(),
            github: None,
            ..Config::default()
        };
        Self::from_config(&config).await.expect("测试状态起不来")
    }

    pub fn data_dir(&self) -> &std::path::Path {
        &self.inner.data_dir
    }

    /// 记下一个刚发出去的网页登录 `state`。满了返回 `false`。
    pub fn remember_login_state(&self, state: String) -> bool {
        let mut states = self.inner.login_states.lock().unwrap();
        let now = Instant::now();
        if states.len() >= LOGIN_STATE_CAP {
            states.retain(|_, issued| now.duration_since(*issued) < LOGIN_STATE_TTL);
        }
        if states.len() >= LOGIN_STATE_CAP {
            return false;
        }
        states.insert(state, now);
        true
    }

    /// GitHub 带着 `state` 回来了：认识且没过期就消费掉它。
    pub fn take_login_state(&self, state: &str) -> bool {
        let mut states = self.inner.login_states.lock().unwrap();
        match states.remove(state) {
            Some(issued) => issued.elapsed() < LOGIN_STATE_TTL,
            None => false,
        }
    }

    /// 提交一个版本要「读当前版本号 → 写清单 → 挪指针 → 回写库」，中间有文件 I/O，
    /// 不能全塞进一个数据库事务里。用一把锁把提交串起来，两次并发提交就不会抢到同一个版本号。
    /// v0.1 是单进程，一把全局锁够了；多进程时这把锁要挪进数据库（DESIGN §4.5）。
    pub async fn lock_commit(&self) -> MutexGuard<'_, ()> {
        self.inner.commit_lock.lock().await
    }
}
