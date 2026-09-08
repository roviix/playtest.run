//! 各个处理函数共用的东西。

use std::sync::Arc;

use playtest_common::store::FsStore;
use playtest_common::tunnel::SigningKey;
use tokio::sync::{Mutex, MutexGuard};

use crate::config::{Config, SLUG_PLACEHOLDER};
use crate::db::Db;
use crate::tunnel_keys;

#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,
}

struct Inner {
    db: Db,
    store: FsStore,
    site_url_template: String,
    tunnel_key: SigningKey,
    commit_lock: Mutex<()>,
}

impl AppState {
    pub fn new(db: Db, store: FsStore, site_url_template: String, tunnel_key: SigningKey) -> Self {
        Self {
            inner: Arc::new(Inner {
                db,
                store,
                site_url_template,
                tunnel_key,
                commit_lock: Mutex::new(()),
            }),
        }
    }

    /// 按配置建数据目录、开库、备好对象存储。目录不存在就建。
    pub async fn from_config(config: &Config) -> anyhow::Result<Self> {
        let store_root = config.store_root();
        tokio::fs::create_dir_all(&store_root).await?;
        let db = Db::open(&config.sqlite_path())?;
        // 顺带把隧道公钥发布到对象存储：边缘要能在控制面之后、之外单独起来。
        let tunnel_key = tunnel_keys::load_or_create(
            &config.data_dir,
            &store_root,
            tunnel_keys::key_from_env().as_deref(),
        )?;
        Ok(Self::new(
            db,
            FsStore::new(store_root),
            config.site_url_template.clone(),
            tunnel_key,
        ))
    }

    pub fn db(&self) -> &Db {
        &self.inner.db
    }

    pub fn store(&self) -> &FsStore {
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

    /// 提交一个版本要「读当前版本号 → 写清单 → 挪指针 → 回写库」，中间有文件 I/O，
    /// 不能全塞进一个数据库事务里。用一把锁把提交串起来，两次并发提交就不会抢到同一个版本号。
    /// v0.1 是单进程，一把全局锁够了；多进程时这把锁要挪进数据库（DESIGN §4.5）。
    pub async fn lock_commit(&self) -> MutexGuard<'_, ()> {
        self.inner.commit_lock.lock().await
    }
}
