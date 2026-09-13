//! 把 `edge-events.jsonl` 里的行按 60 秒一批送到控制面（DESIGN §3.4、§4.5「用量在边缘按会话累计，每 60 秒批量上报」）。
//!
//! 为什么是「先落盘再送」而不是直接发：边缘对玩家的响应不能等控制面；控制面挂了、重启了、
//! 网络断了，事件都先在本地盘上，恢复后从上次送到的位置继续。送到哪一行记在旁边的 `.offset` 文件里，
//! 所以边缘自己重启也不重发、不漏发（最坏是最后一批重复一次，控制面按会话 id 合并，重复无害）。
//!
//! 只用 hyper 的 HTTP/1.1 客户端走明文：v0.1 控制面和边缘在同一台机器上（`compose.yaml`），
//! 上线到多边缘时控制面才有公网 HTTPS 入口，那时再加 TLS 或改走内网。
//! 控制面只接受带部署共享凭据的批量上报；凭据不写入事件日志和请求 URL。

use std::path::{Path, PathBuf};
use std::time::Duration;

use playtest_common::ingest::{self, EdgeBatch, EdgeEvent};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

/// 每隔这么久送一批。
pub const INTERVAL: Duration = Duration::from_secs(60);
/// 一次读多少字节的日志再切行。
const READ_CHUNK: usize = 256 * 1024;

const POST_TIMEOUT: Duration = Duration::from_secs(15);

pub struct Shipper {
    log_path: PathBuf,
    offset_path: PathBuf,
    /// 控制面地址，例如 `http://api:8787`（compose 内网）或 `http://127.0.0.1:8787`。
    api_base: String,
    ingest_token: String,
}

impl Shipper {
    pub fn new(
        log_path: impl Into<PathBuf>,
        api_base: impl Into<String>,
        ingest_token: impl Into<String>,
    ) -> Self {
        let log_path = log_path.into();
        let offset_path = log_path.with_extension("jsonl.offset");
        Self {
            log_path,
            offset_path,
            api_base: api_base.into().trim_end_matches('/').to_string(),
            ingest_token: ingest_token.into(),
        }
    }

    /// 一直跑到进程结束。第一轮不等 60 秒——边缘重启后积压的事件尽快送掉。
    pub async fn run(self) {
        loop {
            match self.ship_once().await {
                Ok(0) => {}
                Ok(n) => tracing::info!(sent = n, "边缘事件已送到控制面"),
                Err(err) => tracing::warn!(%err, "边缘事件这一轮没送出去，下一轮再试"),
            }
            tokio::time::sleep(INTERVAL).await;
        }
    }

    /// 从上次的位置读到文件末尾，按批发送，成功一批推进一次位置。返回送出的条数。
    pub async fn ship_once(&self) -> anyhow::Result<usize> {
        let mut offset = read_offset(&self.offset_path).await;
        let Ok(mut file) = tokio::fs::File::open(&self.log_path).await else {
            return Ok(0);
        };
        let len = file.metadata().await?.len();
        if offset > len {
            // 文件被轮转或清空了：从头开始。
            offset = 0;
        }
        if offset == len {
            return Ok(0);
        }
        file.seek(std::io::SeekFrom::Start(offset)).await?;

        let mut sent = 0usize;
        let mut buf = Vec::with_capacity(READ_CHUNK);
        let mut pending: Vec<EdgeEvent> = Vec::new();
        let mut pending_end = offset;
        let mut line_start = offset;
        let mut carry: Vec<u8> = Vec::new();

        loop {
            buf.clear();
            let n = (&mut file)
                .take(READ_CHUNK as u64)
                .read_to_end(&mut buf)
                .await?;
            if n == 0 {
                break;
            }
            carry.extend_from_slice(&buf);
            // 只处理完整的行；半行留到下一轮（写的一方是整行原子追加的，半行只会出现在读到文件尾时）。
            while let Some(nl) = carry.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = carry.drain(..=nl).collect();
                line_start += line.len() as u64;
                if let Ok(event) = serde_json::from_slice::<EdgeEvent>(&line) {
                    pending.push(event);
                } else {
                    tracing::debug!("跳过一行解析不了的边缘事件");
                }
                if pending.len() >= ingest::MAX_EVENTS_PER_BATCH {
                    self.post(&pending).await?;
                    sent += pending.len();
                    pending.clear();
                    pending_end = line_start;
                    write_offset(&self.offset_path, pending_end).await?;
                }
            }
        }
        if !pending.is_empty() {
            self.post(&pending).await?;
            sent += pending.len();
            pending_end = line_start;
        }
        if pending_end != offset {
            write_offset(&self.offset_path, pending_end).await?;
        }
        Ok(sent)
    }

    /// 一批的整个往返。后台任务，没人在等，所以给得比 `follow` 那条宽。
    async fn post(&self, events: &[EdgeEvent]) -> anyhow::Result<()> {
        let answer = crate::upstream::post_json_bearer(
            &self.api_base,
            ingest::routes::EDGE,
            &EdgeBatch {
                events: events.to_vec(),
            },
            POST_TIMEOUT,
            &self.ingest_token,
        )
        .await?;
        if !answer.ok() {
            // 没送到就不推进 offset，下一轮从同一行再来。
            anyhow::bail!("控制面拒收（{}）：{}", answer.status, answer.snippet());
        }
        Ok(())
    }
}

async fn read_offset(path: &Path) -> u64 {
    match tokio::fs::read_to_string(path).await {
        Ok(s) => s.trim().parse().unwrap_or(0),
        Err(_) => 0,
    }
}

async fn write_offset(path: &Path, offset: u64) -> std::io::Result<()> {
    let tmp = path.with_extension("offset.tmp");
    tokio::fs::write(&tmp, offset.to_string()).await?;
    tokio::fs::rename(&tmp, path).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{header, HeaderMap};
    use axum::{routing::post, Json, Router};
    use std::sync::{Arc, Mutex};

    const EDGE_TOKEN: &str = "test-edge-ingest-token";

    fn line(kind: &str, sid: &str) -> String {
        format!(
            r#"{{"ts":"2026-09-07T10:00:00Z","type":"{kind}","slug":"brisk-otter-41","version":1,"sid":"{sid}","ua":"x","referer":"","wechat":false}}"#
        )
    }

    async fn fake_api(received: Arc<Mutex<Vec<EdgeBatch>>>) -> String {
        let app = Router::new().route(
            ingest::routes::EDGE,
            post(move |headers: HeaderMap, Json(batch): Json<EdgeBatch>| {
                let received = received.clone();
                async move {
                    assert_eq!(
                        headers.get(header::AUTHORIZATION).unwrap(),
                        &format!("Bearer {EDGE_TOKEN}")
                    );
                    received.lock().unwrap().push(batch);
                    Json(ingest::Accepted { accepted: 1 })
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn ships_new_lines_only_and_remembers_where_it_was() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("edge-events.jsonl");
        std::fs::write(
            &log,
            format!(
                "{}\n{}\n",
                line("gate_view", "a".repeat(32).as_str()),
                line("start", "a".repeat(32).as_str())
            ),
        )
        .unwrap();
        let received = Arc::new(Mutex::new(Vec::new()));
        let api = fake_api(received.clone()).await;
        let shipper = Shipper::new(&log, &api, EDGE_TOKEN);

        assert_eq!(shipper.ship_once().await.unwrap(), 2);
        assert_eq!(shipper.ship_once().await.unwrap(), 0, "没有新行就不发");

        // 再追加一行加半行：只有完整的那行被送。
        let mut f = std::fs::OpenOptions::new().append(true).open(&log).unwrap();
        use std::io::Write;
        write!(
            f,
            "{}\n{{\"ts\":\"半",
            line("html_view", "b".repeat(32).as_str())
        )
        .unwrap();
        drop(f);
        assert_eq!(shipper.ship_once().await.unwrap(), 1);

        let batches = received.lock().unwrap();
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].events.len(), 2);
        assert_eq!(batches[0].events[1].kind, "start");
        assert_eq!(batches[1].events[0].kind, "html_view");

        let offset: u64 = std::fs::read_to_string(log.with_extension("jsonl.offset"))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let full_len = std::fs::metadata(&log).unwrap().len();
        assert!(offset < full_len, "半行不算送过");
    }

    #[tokio::test]
    async fn a_dead_api_leaves_the_offset_alone() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("edge-events.jsonl");
        std::fs::write(
            &log,
            format!("{}\n", line("gate_view", "c".repeat(32).as_str())),
        )
        .unwrap();
        // 一个没人听的端口。
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let shipper = Shipper::new(&log, format!("http://{addr}"), EDGE_TOKEN);
        assert!(shipper.ship_once().await.is_err());
        assert!(!log.with_extension("jsonl.offset").exists());
    }

    #[tokio::test]
    async fn missing_log_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let shipper = Shipper::new(
            dir.path().join("nope.jsonl"),
            "http://127.0.0.1:1",
            EDGE_TOKEN,
        );
        assert_eq!(shipper.ship_once().await.unwrap(), 0);
    }
}
