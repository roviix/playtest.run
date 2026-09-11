//! 控制面的后台活，一个调度器一张表。
//!
//! 原来是六个各自 `tokio::spawn` 的循环（清过期作品、回收 blob、周报、发通知、重写广场、
//! 重写 `live.json`），各有各的 `interval`，谁上次什么时候跑过、跑成没跑成，只能翻日志。
//! 现在一个任务就是 [`Job`] 的一个实现，注册进 [`JOBS`]；一个滴答（[`TICK`]）扫一遍，
//! 到点的就跑，跑完把「什么时候、结果如何」记在 [`Board`] 上——运营接口能看，测试能数。
//!
//! 任务串行跑：它们本来就都在争同一把写锁，并发只是换个地方排队；串行还让「广场重算」
//! 一定发生在「过期作品清掉」之后，不用再在清理里手工触发一次。
//!
//! 每个任务的**周期**写在它自己身上，这里不复述；任务失败只记下来、下一轮再来，不 panic。

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use time::OffsetDateTime;

use crate::clock;
use crate::state::AppState;

/// 调度器多久醒一次。最勤的任务（发通知）是 30 秒，滴答不用更细。
pub const TICK: Duration = Duration::from_secs(30);

pub type JobFuture<'a> = Pin<Box<dyn Future<Output = anyhow::Result<usize>> + Send + 'a>>;

/// 一件周期性的活。返回值是「这一轮做了几件」，只用于记账和日志。
pub trait Job: Send + Sync {
    /// 稳定的英文标识：运营接口和日志里用。
    fn name(&self) -> &'static str;
    /// 多久跑一次。
    fn every(&self) -> Duration;
    /// 进程刚起来那一个滴答要不要跑。多数要（重启前的那份可能已经旧了）；
    /// 周报那种「到点才发」的不要。
    fn on_start(&self) -> bool {
        true
    }
    fn run<'a>(&'a self, state: &'a AppState) -> JobFuture<'a>;
}

pub use playtest_common::boost::JobStatus;

/// 所有任务的现状。`AppState` 持有一份，运营接口读它。
#[derive(Clone, Default)]
pub struct Board {
    rows: Arc<Mutex<Vec<JobStatus>>>,
}

impl Board {
    pub fn snapshot(&self) -> Vec<JobStatus> {
        self.rows.lock().expect("任务板的锁不该中毒").clone()
    }

    fn init(&self, jobs: &[Box<dyn Job>]) {
        let mut rows = self.rows.lock().expect("任务板的锁不该中毒");
        rows.clear();
        rows.extend(jobs.iter().map(|job| JobStatus {
            name: job.name().to_string(),
            every_secs: job.every().as_secs(),
            last_run_at: None,
            last_ok: None,
            last_count: None,
            last_error: None,
            runs: 0,
            failures: 0,
        }));
    }

    fn record(&self, index: usize, at: OffsetDateTime, outcome: &anyhow::Result<usize>) {
        let mut rows = self.rows.lock().expect("任务板的锁不该中毒");
        let Some(row) = rows.get_mut(index) else {
            return;
        };
        row.last_run_at = Some(clock::format(at));
        row.runs += 1;
        match outcome {
            Ok(count) => {
                row.last_ok = Some(true);
                row.last_count = Some(*count);
                row.last_error = None;
            }
            Err(err) => {
                row.last_ok = Some(false);
                row.failures += 1;
                row.last_error = Some(format!("{err:#}"));
            }
        }
    }
}

/// 调度器本体。`run_due` 是它唯一会做的事，`spawn` 只是每个滴答调一次。
pub struct Scheduler {
    jobs: Vec<Box<dyn Job>>,
    /// 每个任务上次跑的时刻（单调时钟，不受系统时间调整影响）。
    last: Vec<Option<tokio::time::Instant>>,
    board: Board,
    started: bool,
}

impl Scheduler {
    pub fn new(jobs: Vec<Box<dyn Job>>, board: Board) -> Self {
        board.init(&jobs);
        let last = vec![None; jobs.len()];
        Self {
            jobs,
            last,
            board,
            started: false,
        }
    }

    /// 跑一遍所有到点的任务。返回这一轮跑了几个。
    pub async fn run_due(&mut self, state: &AppState, now: tokio::time::Instant) -> usize {
        let first = !self.started;
        self.started = true;
        let mut ran = 0;
        for (index, job) in self.jobs.iter().enumerate() {
            let due = match self.last[index] {
                None => {
                    // 「到点才发」的任务第一轮不跑，但把这一轮记成起点，否则下一轮又是「没跑过」。
                    if first && !job.on_start() {
                        self.last[index] = Some(now);
                        continue;
                    }
                    true
                }
                Some(then) => now.duration_since(then) >= job.every(),
            };
            if !due {
                continue;
            }
            self.last[index] = Some(now);
            let outcome = job.run(state).await;
            match &outcome {
                Ok(count) if *count > 0 => tracing::info!(job = job.name(), count, "做完一轮"),
                Ok(_) => tracing::debug!(job = job.name(), "这一轮没有要做的"),
                Err(err) => tracing::error!(
                    job = job.name(),
                    error = format!("{err:#}"),
                    "没做完，下一轮再试"
                ),
            }
            self.board.record(index, clock::now(), &outcome);
            ran += 1;
        }
        ran
    }
}

/// 起一个任务：每 [`TICK`] 扫一遍。
pub fn spawn(state: AppState, jobs: Vec<Box<dyn Job>>) -> tokio::task::JoinHandle<()> {
    let board = state.jobs().clone();
    tokio::spawn(async move {
        let mut scheduler = Scheduler::new(jobs, board);
        let mut ticker = tokio::time::interval(TICK);
        loop {
            let now = ticker.tick().await;
            scheduler.run_due(&state, now).await;
        }
    })
}

/// 控制面的全部后台活。顺序有意义：清理在前，广场与 `live.json` 的重算在后。
pub fn jobs(state: &AppState) -> Vec<Box<dyn Job>> {
    vec![
        Box::new(crate::sweeper::ExpireSites),
        Box::new(crate::sweeper::DailyCleanup),
        Box::new(crate::notify::digest::WeeklyDigest::new(state.data_dir())),
        Box::new(crate::notify::worker::Deliver),
        Box::new(crate::plaza::Refresh),
        Box::new(crate::live::Refresh),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Counter {
        name: &'static str,
        every: Duration,
        on_start: bool,
        hits: Arc<AtomicUsize>,
        fail: bool,
    }

    impl Job for Counter {
        fn name(&self) -> &'static str {
            self.name
        }
        fn every(&self) -> Duration {
            self.every
        }
        fn on_start(&self) -> bool {
            self.on_start
        }
        fn run<'a>(&'a self, _: &'a AppState) -> JobFuture<'a> {
            Box::pin(async move {
                self.hits.fetch_add(1, Ordering::SeqCst);
                if self.fail {
                    anyhow::bail!("故意坏掉")
                }
                Ok(3)
            })
        }
    }

    #[tokio::test]
    async fn jobs_run_on_their_own_period_and_the_board_keeps_score() {
        let state = crate::state::AppState::for_tests().await;
        let fast = Arc::new(AtomicUsize::new(0));
        let slow = Arc::new(AtomicUsize::new(0));
        let broken = Arc::new(AtomicUsize::new(0));
        let jobs: Vec<Box<dyn Job>> = vec![
            Box::new(Counter {
                name: "fast",
                every: Duration::from_secs(30),
                on_start: true,
                hits: fast.clone(),
                fail: false,
            }),
            Box::new(Counter {
                name: "slow",
                every: Duration::from_secs(90),
                on_start: false,
                hits: slow.clone(),
                fail: false,
            }),
            Box::new(Counter {
                name: "broken",
                every: Duration::from_secs(30),
                on_start: true,
                hits: broken.clone(),
                fail: true,
            }),
        ];
        let board = Board::default();
        let mut scheduler = Scheduler::new(jobs, board.clone());
        let t0 = tokio::time::Instant::now();

        // 第一个滴答：要求起步就跑的跑了，「到点才发」的没跑。
        scheduler.run_due(&state, t0).await;
        assert_eq!(fast.load(Ordering::SeqCst), 1);
        assert_eq!(slow.load(Ordering::SeqCst), 0);
        assert_eq!(broken.load(Ordering::SeqCst), 1);

        scheduler
            .run_due(&state, t0 + Duration::from_secs(30))
            .await;
        scheduler
            .run_due(&state, t0 + Duration::from_secs(60))
            .await;
        assert_eq!(fast.load(Ordering::SeqCst), 3);
        assert_eq!(slow.load(Ordering::SeqCst), 0, "90 秒还没到");
        scheduler
            .run_due(&state, t0 + Duration::from_secs(90))
            .await;
        assert_eq!(slow.load(Ordering::SeqCst), 1);

        let rows = board.snapshot();
        assert_eq!(rows.len(), 3);
        let fast_row = &rows[0];
        assert_eq!(fast_row.runs, 4);
        assert_eq!(fast_row.last_ok, Some(true));
        assert_eq!(fast_row.last_count, Some(3));
        let broken_row = &rows[2];
        assert_eq!(broken_row.failures, 4);
        assert_eq!(broken_row.last_ok, Some(false));
        assert!(broken_row
            .last_error
            .as_deref()
            .unwrap()
            .contains("故意坏掉"));
        assert!(rows[1].last_run_at.is_some());
    }

    #[tokio::test]
    async fn the_real_job_list_has_six_entries_in_cleanup_first_order() {
        let state = crate::state::AppState::for_tests().await;
        let names: Vec<&str> = jobs(&state).iter().map(|j| j.name()).collect();
        assert_eq!(
            names,
            [
                "expire_sites",
                "daily_cleanup",
                "weekly_digest",
                "deliver_notifications",
                "plaza",
                "live"
            ]
        );
    }
}
