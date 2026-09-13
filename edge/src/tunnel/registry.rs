//! 谁现在连着，谁刚刚离开，谁已经被挤掉。
//!
//! 一个 slug 同时只绑一条隧道（DESIGN §4.3「新的挤掉旧的」）。这里管三张表：
//!
//! - `live`：`slug → 会话`。玩家的请求按它找去处。
//! - `last_seen`：`slug → 上次在线的时间和当时的作品信息`。同时落一份到
//!   `<数据目录>/tunnels/<slug>.json`——这是边缘自己的观察，不是 api 写的东西，
//!   所以不进对象存储；落盘是为了边缘重启之后离线页仍然说得出「上次在线」。
//! - `evicted`：被挤掉的令牌 `jti`。拿它再握手一律 409，CLI 据此退出而不是无限重连。
//!
//! 会话在两种情况下注销：yamux 那条连接自己结束，或者空闲看门狗发现
//! [`IDLE_TIMEOUT_SECS`] 秒没收到任何帧。两条路都汇到 [`Tunnels::unregister`]，
//! 而它只在「当前会话确实是这个 jti」时才动手——旧会话退出的通知常常晚于新会话接上来，
//! 不校验就会把刚连上的人踢掉。

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::{Duration, Instant};

use playtest_common::manifest::{GateMode, Manifest, SCHEMA};
use playtest_common::slug;
use playtest_common::store::Store;
use playtest_common::tunnel::io::{ActivityClock, Mux, WsControl};
use playtest_common::tunnel::{close, Claims, IDLE_TIMEOUT_SECS};
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::config::Config;
use crate::tunnel::keys::Keys;

/// 空闲看门狗多久扫一遍。比 [`IDLE_TIMEOUT_SECS`] 小一个数量级就够：
/// 判定慢十秒对玩家没有区别，扫得太勤只是白占 CPU。
const SWEEP: Duration = Duration::from_secs(10);

/// 被挤掉的 `jti` 在名单里留多久。令牌本身只活一小时，留 24 小时是为了让
/// 「关掉笔记本、第二天早上那个进程醒过来重连」也能收到 409 而不是又挤一次。
const EVICTED_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// 「查过磁盘、确实没有上次在线记录」的 slug 最多记这么多个。一台边缘上有流量的作品远不到这个数，
/// 到了只可能是有人在扫子域名。
const NEVER_SEEN_CAP: usize = 4096;

/// 一条连着的隧道。
///
/// 拿住 [`Mux`] 就是拿住这条隧道：句柄全丢掉，驾驭任务就收摊（见 `common` 的 `Mux` 文档）。
/// 所以正在转发的请求会把 `Arc<Session>` 一起拿着，玩家的响应流完之前隧道不会被顺手关掉。
pub struct Session {
    pub claims: Claims,
    /// 开发者机器上的端口，`Host` 改写成 `localhost:<它>`。
    pub local_port: u16,
    pub mux: Mux,
    pub activity: ActivityClock,
    /// 发 WebSocket 控制帧的句柄；`Mux` 拿走流之后只剩它能给 CLI 送关闭码。测试里直接用裸流时没有。
    control: Option<WsControl>,
    pub connected_at: OffsetDateTime,
    /// 算在线时长用。`OffsetDateTime` 会被系统时钟的调整带跑。
    since: Instant,
    manifest: Arc<Manifest>,
    /// 正在转发的请求数，[`Claims::max_players`] 是它的上限。
    pub open_streams: AtomicU32,
    /// 玩家 → 开发者的字节数。
    pub bytes_in: AtomicU64,
    /// 开发者 → 玩家的字节数。
    pub bytes_out: AtomicU64,
    pub requests: AtomicU64,
}

impl Session {
    pub fn new(claims: Claims, local_port: u16, mux: Mux, activity: ActivityClock) -> Self {
        let connected_at = OffsetDateTime::now_utc();
        let manifest = Arc::new(synthetic_manifest(&claims, connected_at));
        Self {
            claims,
            local_port,
            mux,
            activity,
            control: None,
            connected_at,
            since: Instant::now(),
            manifest,
            open_streams: AtomicU32::new(0),
            bytes_in: AtomicU64::new(0),
            bytes_out: AtomicU64::new(0),
            requests: AtomicU64::new(0),
        }
    }

    pub fn with_control(mut self, control: WsControl) -> Self {
        self.control = Some(control);
        self
    }

    pub fn slug(&self) -> &str {
        &self.claims.slug
    }

    /// 门禁页、`/_playtest/me`、响应头都按清单说话，隧道也得有一份。
    pub fn manifest(&self) -> Arc<Manifest> {
        self.manifest.clone()
    }

    pub fn uptime(&self) -> Duration {
        self.since.elapsed()
    }

    /// 关掉这条隧道。先排一个带 `code` 的 WebSocket Close 帧，CLI 才分得清「被挤掉了，退出」
    /// 和「边缘要重启，退避重连」（`common` 的 `tunnel::close`）；yamux 关闭时会先把排好的控制帧冲出去。
    fn close(&self, code: u16) {
        tracing::debug!(slug = %self.claims.slug, code, "关掉这条隧道");
        if let Some(control) = &self.control {
            let reason = match code {
                close::REPLACED => "replaced",
                close::REVOKED => "revoked",
                close::TOKEN_EXPIRED => "token expired",
                _ => "going away",
            };
            control.close(code, reason);
        }
        self.mux.close();
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("slug", &self.claims.slug)
            .field("jti", &self.claims.jti)
            .field("local_port", &self.local_port)
            .finish()
    }
}

/// 隧道没有版本这个概念（DESIGN §3.5：只有「在线 / 离线」和会话），但门禁页、
/// `/_playtest/me`、响应头这些都是照着清单写的。用令牌里的声明拼一份出来，
/// `version` 留 0——门禁页那个位置显示的是「在线」，玩家看不到这个 0。
/// `files` 为空是准确的：隧道模式下边缘手上一个文件都没有，字节全在开发者那边。
pub fn synthetic_manifest(claims: &Claims, connected_at: OffsetDateTime) -> Manifest {
    Manifest {
        schema: SCHEMA,
        slug: claims.slug.clone(),
        version: 0,
        title: claims.title.clone(),
        developer: claims.developer.clone(),
        note: None,
        summary: None,
        cover: None,
        created_at: connected_at.format(&Rfc3339).unwrap_or_default(),
        expires_at: None,
        badge: claims.badge,
        gate: claims.gate,
        isolated: claims.isolated,
        spa: false,
        engine: None,
        kind: playtest_common::manifest::WorkKind::Web,
        entry: None,
        article: None,
        chapters: Vec::new(),
        files: Vec::new(),
    }
}

/// 离线页要说的那几句话。存成 JSON 是为了边缘重启之后还说得出来。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastSeen {
    /// RFC 3339。
    pub at: String,
    pub title: String,
    pub developer: String,
    #[serde(default)]
    pub badge: bool,
    #[serde(default)]
    pub gate: GateMode,
    #[serde(default)]
    pub isolated: bool,
    /// 上次那条隧道是混合模式的后端（只接清单里没有的路径）。离线时这些路径要回
    /// 「后端不在线」，而不是让上传的清单回一页 404。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hybrid: bool,
}

#[derive(Default)]
struct Inner {
    live: HashMap<String, Arc<Session>>,
    last_seen: HashMap<String, LastSeen>,
    /// 去磁盘上找过、确实没有记录的 slug。上传路径每遇到一个清单里没有的路径都会来问一次
    /// 「有没有掉线的后端」，不记下来的话没开过隧道的作品每个 404 都要碰一次磁盘。
    never_seen: HashSet<String>,
    evicted: HashMap<String, Instant>,
}

pub struct Tunnels {
    inner: Mutex<Inner>,
    /// `<数据目录>/tunnels`，边缘自己的，不是对象存储。
    dir: PathBuf,
    keys: Keys,
    watchdog: AtomicBool,
}

impl Tunnels {
    pub fn new(config: &Config, store: Store) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(Inner::default()),
            dir: config.tunnels_dir(),
            keys: Keys::new(store),
            watchdog: AtomicBool::new(false),
        })
    }

    pub fn keys(&self) -> &Keys {
        &self.keys
    }

    pub fn get(&self, slug: &str) -> Option<Arc<Session>> {
        self.lock().live.get(slug).cloned()
    }

    /// 这个令牌是不是被挤掉过。是的话握手回 409，CLI 退出而不是重试。
    pub fn is_evicted(&self, jti: &str) -> bool {
        self.lock()
            .evicted
            .get(jti)
            .is_some_and(|at| at.elapsed() < EVICTED_TTL)
    }

    /// 接上一条新隧道。同一个 slug 已经有连接时，旧的进驱逐名单并被关掉。
    pub fn register(self: &Arc<Self>, session: Arc<Session>) {
        let slug = session.claims.slug.clone();
        let jti = session.claims.jti.clone();

        let replaced = {
            let mut inner = self.lock();
            let old = inner.live.insert(slug.clone(), session.clone());
            if let Some(old) = &old {
                inner.evicted.insert(old.claims.jti.clone(), Instant::now());
            }
            old
        };
        if let Some(old) = replaced {
            tracing::info!(
                slug = %slug,
                jti = %old.claims.jti,
                "同一个作品又来了一条隧道，把旧的挤掉"
            );
            old.close(close::REPLACED);
        }

        // events.rs 现在只认固定那几种事件类型，加一种要改别人的文件，所以隧道的上下线
        // 先只落 tracing 日志（第三周把用量上报接上时一起进事件流）。
        tracing::info!(
            event = "tunnel_online",
            slug = %slug,
            jti = %jti,
            local_port = session.local_port,
            title = %session.claims.title,
            "隧道接上了"
        );

        self.start_watchdog();

        // yamux 那条连接结束就注销：CLI 退出、电脑休眠、网络断掉都走这里。
        let weak = Arc::downgrade(self);
        let closed = session.mux.closed();
        tokio::spawn(async move {
            let _ = closed.await;
            if let Some(tunnels) = weak.upgrade() {
                tunnels.unregister(&slug, &jti).await;
            }
        });
    }

    /// 注销。**只在当前会话确实是这个 `jti` 时才动手**：旧会话的结束通知常常晚于
    /// 新会话接上来，不校验就会把刚连上的人踢下去。
    pub async fn unregister(&self, slug: &str, jti: &str) {
        let session = {
            let mut inner = self.lock();
            match inner.live.get(slug) {
                Some(current) if current.claims.jti == jti => inner.live.remove(slug),
                _ => None,
            }
        };
        let Some(session) = session else { return };
        session.close(close::GOING_AWAY);

        let seen = LastSeen {
            at: OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .unwrap_or_default(),
            title: session.claims.title.clone(),
            developer: session.claims.developer.clone(),
            badge: session.claims.badge,
            gate: session.claims.gate,
            isolated: session.claims.isolated,
            hybrid: session.claims.hybrid,
        };
        tracing::info!(
            event = "tunnel_offline",
            slug = %slug,
            jti = %jti,
            uptime_secs = session.uptime().as_secs(),
            requests = session.requests.load(Ordering::Relaxed),
            bytes_in = session.bytes_in.load(Ordering::Relaxed),
            bytes_out = session.bytes_out.load(Ordering::Relaxed),
            "隧道断开了"
        );

        if let Ok(mut inner) = self.inner.lock() {
            inner.never_seen.remove(slug);
            inner.last_seen.insert(slug.to_string(), seen.clone());
        }
        self.persist(slug, &seen).await;
    }

    /// 上次在线。内存里没有就去磁盘上找一次（边缘刚重启的情形）；找过没有的记下来，不再找。
    pub async fn last_seen(&self, slug: &str) -> Option<LastSeen> {
        {
            let inner = self.lock();
            if let Some(hit) = inner.last_seen.get(slug).cloned() {
                return Some(hit);
            }
            if inner.never_seen.contains(slug) {
                return None;
            }
        }
        let path = self.file_of(slug)?;
        let seen = match tokio::fs::read(&path).await {
            Ok(bytes) => match serde_json::from_slice::<LastSeen>(&bytes) {
                Ok(seen) => Some(seen),
                Err(err) => {
                    tracing::warn!(path = %path.display(), %err, "上次在线的记录读不懂，当没有");
                    None
                }
            },
            Err(_) => None,
        };
        if let Ok(mut inner) = self.inner.lock() {
            match &seen {
                Some(seen) => {
                    inner.last_seen.insert(slug.to_string(), seen.clone());
                }
                None => {
                    // 随机打子域名的扫描器每个 slug 都会来问一次；满了就整个清掉，
                    // 代价只是下一次多碰一回磁盘，比无界长大好。
                    if inner.never_seen.len() >= NEVER_SEEN_CAP {
                        inner.never_seen.clear();
                    }
                    inner.never_seen.insert(slug.to_string());
                }
            }
        }
        seen
    }

    async fn persist(&self, slug: &str, seen: &LastSeen) {
        let Some(path) = self.file_of(slug) else {
            return;
        };
        let Ok(bytes) = serde_json::to_vec(seen) else {
            return;
        };
        let write = async {
            tokio::fs::create_dir_all(&self.dir).await?;
            tokio::fs::write(&path, &bytes).await
        };
        if let Err(err) = write.await {
            tracing::warn!(path = %path.display(), %err, "上次在线的记录写不进去");
        }
    }

    /// slug 在这里会被拼进文件路径，所以进磁盘之前再校验一次形态——
    /// 它一路上已经被 `host::classify` 验过，但这一层不该依赖上一层没写错。
    fn file_of(&self, slug: &str) -> Option<PathBuf> {
        slug::validate(slug).ok()?;
        Some(self.dir.join(format!("{slug}.json")))
    }

    /// 第一次有隧道接上来才起看门狗：没有隧道的边缘（现在绝大多数）不该多一个空转的任务。
    fn start_watchdog(self: &Arc<Self>) {
        if self.watchdog.swap(true, Ordering::SeqCst) {
            return;
        }
        let weak = Arc::downgrade(self);
        tokio::spawn(watchdog(weak));
    }

    async fn sweep(&self) {
        let idle_limit = Duration::from_secs(IDLE_TIMEOUT_SECS);
        let stale: Vec<(String, String)> = {
            let mut inner = self.lock();
            inner.evicted.retain(|_, at| at.elapsed() < EVICTED_TTL);
            inner
                .live
                .iter()
                .filter(|(_, session)| session.activity.idle_for() > idle_limit)
                .map(|(slug, session)| (slug.clone(), session.claims.jti.clone()))
                .collect()
        };
        for (slug, jti) in stale {
            tracing::info!(slug = %slug, "隧道 {IDLE_TIMEOUT_SECS} 秒没有任何动静，当离线");
            self.unregister(&slug, &jti).await;
        }
    }

    /// 锁中毒只会发生在别的线程持锁时 panic，那时候表里的东西已经不可信；
    /// 但把整个边缘拖垮更糟，所以取回里面的数据继续用。
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl std::fmt::Debug for Tunnels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tunnels")
            .field("live", &self.lock().live.len())
            .field("dir", &self.dir)
            .finish()
    }
}

/// 拿 `Weak` 不拿 `Arc`：边缘（或测试里的一个 `App`）没了，这个任务跟着结束。
async fn watchdog(weak: Weak<Tunnels>) {
    let mut tick = tokio::time::interval(SWEEP);
    loop {
        tick.tick().await;
        match weak.upgrade() {
            Some(tunnels) => tunnels.sweep().await,
            None => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use playtest_common::tunnel::io::Role;

    fn claims(slug: &str, jti: &str) -> Claims {
        Claims {
            v: 1,
            slug: slug.into(),
            sub: "user-1".into(),
            title: "小球大冒险".into(),
            developer: "某某".into(),
            badge: true,
            gate: GateMode::Once,
            isolated: false,
            max_players: 8,
            hybrid: false,
            iat: 1_800_000_000,
            exp: 1_800_003_600,
            jti: jti.into(),
        }
    }

    /// 一条谁都没接的 yamux：测登记表不需要真的隧道对端。
    ///
    /// **另一端要拿住**——丢掉它等于把网线拔了，`register` 起的那个任务会立刻
    /// 看到「这条连接结束了」并把会话注销，测的就不是想测的东西了。
    fn session(slug: &str, jti: &str) -> (Arc<Session>, tokio::io::DuplexStream) {
        let (ours, peer) = tokio::io::duplex(1024);
        let session = Session::new(
            claims(slug, jti),
            5173,
            Mux::spawn(ours, Role::Opener),
            ActivityClock::new(),
        );
        (Arc::new(session), peer)
    }

    fn tunnels(dir: &std::path::Path) -> Arc<Tunnels> {
        let config = Config {
            listen: "127.0.0.1:0".parse().unwrap(),
            data_dir: dir.to_path_buf(),
            host_suffix: "localhost".into(),
            public_scheme: "http".into(),
            api_internal_url: None,
            edge_ingest_token: None,
        };
        Tunnels::new(&config, Store::new(config.store_root()))
    }

    #[tokio::test]
    async fn a_new_tunnel_pushes_the_old_one_out() {
        let dir = tempfile::tempdir().unwrap();
        let tunnels = tunnels(dir.path());

        let (first, _peer1) = session("brisk-otter-41", "jti-1");
        tunnels.register(first);
        assert_eq!(tunnels.get("brisk-otter-41").unwrap().claims.jti, "jti-1");
        assert!(!tunnels.is_evicted("jti-1"));

        let (second, _peer2) = session("brisk-otter-41", "jti-2");
        tunnels.register(second);
        assert_eq!(tunnels.get("brisk-otter-41").unwrap().claims.jti, "jti-2");
        // 旧令牌再来一律 409，CLI 据此退出而不是和新进程互相挤。
        assert!(tunnels.is_evicted("jti-1"));
        assert!(!tunnels.is_evicted("jti-2"));
    }

    #[tokio::test]
    async fn a_departing_old_session_cannot_kick_the_new_one() {
        let dir = tempfile::tempdir().unwrap();
        let tunnels = tunnels(dir.path());
        let (first, _peer1) = session("brisk-otter-41", "jti-1");
        let (second, _peer2) = session("brisk-otter-41", "jti-2");
        tunnels.register(first);
        tunnels.register(second);

        // 旧会话这时候才发现自己断了。它不能把刚接上来的那条踢下去。
        tunnels.unregister("brisk-otter-41", "jti-1").await;
        assert_eq!(tunnels.get("brisk-otter-41").unwrap().claims.jti, "jti-2");

        tunnels.unregister("brisk-otter-41", "jti-2").await;
        assert!(tunnels.get("brisk-otter-41").is_none());
    }

    #[tokio::test]
    async fn going_offline_leaves_something_for_the_offline_page() {
        let dir = tempfile::tempdir().unwrap();
        let live = tunnels(dir.path());
        assert!(live.last_seen("brisk-otter-41").await.is_none());

        let (only, _peer) = session("brisk-otter-41", "jti-1");
        live.register(only);
        live.unregister("brisk-otter-41", "jti-1").await;

        let seen = live.last_seen("brisk-otter-41").await.unwrap();
        assert_eq!(seen.title, "小球大冒险");
        assert_eq!(seen.developer, "某某");
        assert!(seen.at.contains('T'), "at 要是 RFC 3339：{}", seen.at);

        // 边缘重启：内存里什么都没有，磁盘上那份还在。
        let restarted = tunnels(dir.path());
        assert_eq!(restarted.last_seen("brisk-otter-41").await, Some(seen));
    }

    #[test]
    fn the_synthetic_manifest_says_what_the_token_says() {
        let mut c = claims("brisk-otter-41", "jti-1");
        c.isolated = true;
        c.badge = false;
        c.gate = GateMode::Always;
        let at = OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap();
        let m = synthetic_manifest(&c, at);

        assert_eq!(m.schema, SCHEMA);
        assert_eq!(m.slug, "brisk-otter-41");
        assert_eq!(m.title, "小球大冒险");
        assert_eq!(m.developer, "某某");
        assert_eq!(m.version, 0);
        assert!(m.isolated);
        assert!(!m.badge);
        assert_eq!(m.gate, GateMode::Always);
        assert_eq!(m.created_at, "2027-01-15T08:00:00Z");
        // 边缘手上一个文件都没有，字节全在开发者那边。
        assert!(m.files.is_empty());
        assert!(m.expires_at.is_none());
        // 隧道认不出引擎（没有目录可看），门禁页因此说「体验」。
        assert!(!m.is_game());
    }

    #[tokio::test]
    async fn a_slug_that_is_not_a_slug_never_becomes_a_path() {
        let dir = tempfile::tempdir().unwrap();
        let tunnels = tunnels(dir.path());
        assert!(tunnels.file_of("../../etc/passwd").is_none());
        assert!(tunnels.last_seen("../../etc/passwd").await.is_none());
    }
}
