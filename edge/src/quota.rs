use std::io;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use axum::body::{Body, Bytes};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use hyper::body::{Body as HttpBody, Frame, SizeHint};
use playtest_common::plan::TrafficWindow;
use playtest_common::quota::Policy;
use playtest_common::store::Store;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pub struct Ledger {
    path: PathBuf,
    connection: Mutex<Option<Connection>>,
    store: Store,
}

#[derive(Debug)]
pub enum Denied {
    Full,
    Expired,
    Unavailable,
}

impl Denied {
    pub fn response(self) -> Response {
        let (status, title, message) = match self {
            Self::Full => (StatusCode::TOO_MANY_REQUESTS, "This project is out of traffic", "Ask the author, or try again later. Free accounts get a new monthly allowance next month; an anonymous link needs the author to publish again. Nothing is billed."),
            Self::Expired => (StatusCode::GONE, "This link has expired", "Ask the author for a new link."),
            Self::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "Can't check the traffic allowance", "We can't keep serving this project right now. Try again shortly; your device is fine."),
        };
        let mut response = (
            status,
            crate::html::shell(
                title,
                "",
                &format!("<h1>{title}</h1><p class=\"lead\">{message}</p>"),
            ),
        )
            .into_response();
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        );
        response
    }
}

impl Ledger {
    pub fn new(data_dir: PathBuf) -> Arc<Self> {
        let store = Store::new(data_dir.join("store"));
        Self::with_store(data_dir, store)
    }

    pub fn with_store(data_dir: PathBuf, store: Store) -> Arc<Self> {
        Arc::new(Self {
            store,
            path: data_dir.join("edge-usage.sqlite"),
            connection: Mutex::new(None),
        })
    }

    pub async fn meter(self: &Arc<Self>, slug: &str) -> Result<Arc<Meter>, Denied> {
        let policy = self
            .store
            .get_policy(slug)
            .await
            .map_err(|error| {
                tracing::error!(%error, "读取作品额度策略失败");
                Denied::Unavailable
            })?
            .ok_or(Denied::Unavailable)?;
        if policy.owner.is_empty() {
            return Err(Denied::Unavailable);
        }
        let meter = Arc::new(Meter {
            ledger: self.clone(),
            slug: slug.to_string(),
            policy,
        });
        meter.reserve(0)?;
        Ok(meter)
    }

    fn reserve_at(
        &self,
        slug: &str,
        policy: &Policy,
        bytes: u64,
        now: OffsetDateTime,
    ) -> Result<(), Denied> {
        if let Some(expiration) = &policy.expires_at {
            let expiration =
                OffsetDateTime::parse(expiration, &Rfc3339).map_err(|_| Denied::Unavailable)?;
            if expiration <= now {
                return Err(Denied::Expired);
            }
        }
        let limits = policy.plan.limits();
        let account = format!("owner:{}", policy.owner);
        let period = match limits.traffic_window {
            TrafficWindow::LinkLifetime => "lifetime".to_string(),
            TrafficWindow::Monthly => format!("month:{:04}-{:02}", now.year(), now.month() as u8),
        };
        let project = format!("project:{slug}");
        let bucket = now.unix_timestamp().div_euclid(300);
        let current = format!("hour:{bucket:012}");
        let oldest = format!("hour:{:012}", bucket - 11);
        let mut guard = self.connection.lock().map_err(|_| Denied::Unavailable)?;
        let operation = (|| -> anyhow::Result<bool> {
            if guard.is_none() {
                if let Some(parent) = self.path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let connection = Connection::open(&self.path)?;
                connection.busy_timeout(Duration::from_secs(2))?;
                connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS usage (scope TEXT NOT NULL, period TEXT NOT NULL, bytes INTEGER NOT NULL CHECK(bytes >= 0), PRIMARY KEY(scope, period));")?;
                *guard = Some(connection);
            }
            let transaction = guard
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("额度账本未打开"))?
                .transaction_with_behavior(TransactionBehavior::Immediate)?;
            let used = u64::try_from(
                transaction
                    .query_row(
                        "SELECT bytes FROM usage WHERE scope=?1 AND period=?2",
                        params![account, period],
                        |row| row.get::<_, i64>(0),
                    )
                    .optional()?
                    .unwrap_or(0),
            )?;
            let hourly = u64::try_from(transaction.query_row("SELECT COALESCE(SUM(bytes),0) FROM usage WHERE scope=?1 AND period BETWEEN ?2 AND ?3", params![project, oldest, current], |row| row.get::<_, i64>(0))?)?;
            if used >= limits.traffic_bytes
                || hourly >= limits.hourly_bytes
                || bytes > limits.traffic_bytes.saturating_sub(used)
                || bytes > limits.hourly_bytes.saturating_sub(hourly)
            {
                return Ok(false);
            }
            if bytes > 0 {
                for (scope, window) in [(&account, &period), (&project, &current)] {
                    transaction.execute("INSERT INTO usage(scope,period,bytes) VALUES(?1,?2,?3) ON CONFLICT(scope,period) DO UPDATE SET bytes=bytes+excluded.bytes", params![scope, window, i64::try_from(bytes)?])?;
                }
                transaction.execute(
                    "DELETE FROM usage WHERE scope=?1 AND period < ?2",
                    params![project, oldest],
                )?;
            }
            transaction.commit()?;
            Ok(true)
        })();
        match operation {
            Ok(true) => Ok(()),
            Ok(false) => Err(Denied::Full),
            Err(error) => {
                tracing::error!(%error, "流量额度无法落盘，停止分发");
                Err(Denied::Unavailable)
            }
        }
    }
}

pub struct Meter {
    ledger: Arc<Ledger>,
    slug: String,
    policy: Policy,
}

impl Meter {
    pub fn reserve(&self, bytes: u64) -> Result<(), Denied> {
        self.ledger
            .reserve_at(&self.slug, &self.policy, bytes, OffsetDateTime::now_utc())
    }

    pub fn response(self: &Arc<Self>, response: Response, head_only: bool) -> Response {
        if head_only || !response.status().is_success() {
            return response;
        }
        let length = response
            .headers()
            .get(header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .or_else(|| response.body().size_hint().exact());
        if let Some(length) = length {
            if let Err(denied) = self.reserve(length) {
                return denied.response();
            }
            return response;
        }
        let (parts, body) = response.into_parts();
        Response::from_parts(
            parts,
            Body::new(MeteredBody {
                inner: body,
                meter: self.clone(),
                finished: false,
            }),
        )
    }

    pub fn io<Stream>(self: &Arc<Self>, inner: Stream) -> MeteredIo<Stream> {
        MeteredIo {
            inner,
            meter: self.clone(),
            credit: 0,
        }
    }
}

struct MeteredBody {
    inner: Body,
    meter: Arc<Meter>,
    finished: bool,
}

impl HttpBody for MeteredBody {
    type Data = Bytes;
    type Error = io::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Self::Error>>> {
        if self.finished {
            return Poll::Ready(None);
        }
        match Pin::new(&mut self.inner).poll_frame(context) {
            Poll::Ready(Some(Ok(frame))) => {
                if let Some(data) = frame.data_ref() {
                    if self.meter.reserve(data.len() as u64).is_err() {
                        self.finished = true;
                        return Poll::Ready(Some(Err(io::Error::other(
                            "流量额度不足或无法检查，传输已停止",
                        ))));
                    }
                }
                Poll::Ready(Some(Ok(frame)))
            }
            Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(io::Error::other(error)))),
            Poll::Ready(None) => {
                self.finished = true;
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn is_end_stream(&self) -> bool {
        self.finished || self.inner.is_end_stream()
    }
    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

pub struct MeteredIo<Stream> {
    inner: Stream,
    meter: Arc<Meter>,
    credit: usize,
}

impl<Stream: AsyncRead + Unpin> AsyncRead for MeteredIo<Stream> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(context, buffer)
    }
}

impl<Stream: AsyncWrite + Unpin> AsyncWrite for MeteredIo<Stream> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<io::Result<usize>> {
        if buffer.is_empty() {
            return Poll::Ready(Ok(0));
        }
        if self.credit == 0 {
            let amount = buffer.len().min(64 * 1024);
            if self.meter.reserve(amount as u64).is_err() {
                return Poll::Ready(Err(io::Error::other("流量额度不足或无法检查，连接已关闭")));
            }
            self.credit = amount;
        }
        let limit = self.credit.min(buffer.len());
        match Pin::new(&mut self.inner).poll_write(context, &buffer[..limit]) {
            Poll::Ready(Ok(written)) => {
                self.credit -= written;
                Poll::Ready(Ok(written))
            }
            result => result,
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(context)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    use playtest_common::limits::GIB;
    use playtest_common::plan::Plan;
    use time::macros::datetime;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn policy(plan: Plan) -> Policy {
        Policy {
            owner: "same-owner".into(),
            plan,
            expires_at: None,
        }
    }

    #[test]
    fn monthly_budget_is_shared_persistent_and_resets_only_next_month() {
        let directory = tempfile::tempdir().unwrap();
        let ledger = Ledger::new(directory.path().to_path_buf());
        let policy = policy(Plan::Free);
        let now = datetime!(2026-09-12 10:00 UTC);
        for slug in ["first", "second", "third"] {
            ledger.reserve_at(slug, &policy, 3 * GIB, now).unwrap();
        }
        ledger
            .reserve_at("first", &policy, GIB, now + time::Duration::hours(2))
            .unwrap();
        assert!(matches!(
            ledger.reserve_at("second", &policy, 1, now + time::Duration::hours(3)),
            Err(Denied::Full)
        ));
        drop(ledger);
        let reopened = Ledger::new(directory.path().to_path_buf());
        assert!(matches!(
            reopened.reserve_at("third", &policy, 0, datetime!(2026-09-30 22:00 UTC)),
            Err(Denied::Full)
        ));
        reopened
            .reserve_at("first", &policy, 1, datetime!(2026-10-01 00:00 UTC))
            .unwrap();
    }

    #[test]
    fn anonymous_budget_does_not_reset_with_the_calendar() {
        let directory = tempfile::tempdir().unwrap();
        let ledger = Ledger::new(directory.path().to_path_buf());
        let policy = policy(Plan::Anon);
        let now = datetime!(2026-09-30 20:00 UTC);
        ledger.reserve_at("first", &policy, GIB / 2, now).unwrap();
        ledger
            .reserve_at("first", &policy, GIB / 2, now + time::Duration::hours(2))
            .unwrap();
        assert!(matches!(
            ledger.reserve_at("first", &policy, 1, datetime!(2026-10-01 00:00 UTC)),
            Err(Denied::Full)
        ));
    }

    #[test]
    fn concurrent_reservations_never_spend_the_same_last_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let ledger = Ledger::new(directory.path().to_path_buf());
        let policy = policy(Plan::Anon);
        let now = datetime!(2026-09-12 10:00 UTC);
        ledger
            .reserve_at("first", &policy, GIB / 2 - 3, now)
            .unwrap();
        let successes = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..12)
                .map(|_| scope.spawn(|| ledger.reserve_at("first", &policy, 1, now).is_ok()))
                .collect();
            handles
                .into_iter()
                .map(|handle| usize::from(handle.join().unwrap()))
                .sum::<usize>()
        });
        assert_eq!(successes, 3);
    }

    #[test]
    fn storage_failure_and_expiration_never_disable_the_limit() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("edge-usage.sqlite")).unwrap();
        let ledger = Ledger::new(directory.path().to_path_buf());
        let mut policy = policy(Plan::Anon);
        let now = datetime!(2026-09-12 10:00 UTC);
        assert!(matches!(
            ledger.reserve_at("first", &policy, 0, now),
            Err(Denied::Unavailable)
        ));
        policy.expires_at = Some("2026-09-12T09:00:00Z".into());
        assert!(matches!(
            ledger.reserve_at("first", &policy, 0, now),
            Err(Denied::Expired)
        ));
        policy.expires_at = Some("not-a-date".into());
        assert!(matches!(
            ledger.reserve_at("first", &policy, 0, now),
            Err(Denied::Unavailable)
        ));
    }

    async fn small_meter(remaining: u64) -> (tempfile::TempDir, Arc<Meter>) {
        let directory = tempfile::tempdir().unwrap();
        let ledger = Ledger::new(directory.path().to_path_buf());
        let policy = policy(Plan::Anon);
        let slug = "brisk-otter-41";
        ledger.store.put_policy(slug, &policy).await.unwrap();
        ledger
            .reserve_at(
                slug,
                &policy,
                GIB / 2 - remaining,
                OffsetDateTime::now_utc(),
            )
            .unwrap();
        let meter = ledger.meter(slug).await.unwrap();
        (directory, meter)
    }

    #[tokio::test]
    async fn known_length_is_rejected_before_any_content_is_sent() {
        let (_directory, meter) = small_meter(3).await;
        let response = meter.response(Response::new(Body::from("four")), false);
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        meter.reserve(3).unwrap();
    }

    #[tokio::test]
    async fn unknown_length_stream_stops_at_the_budget() {
        let (_directory, meter) = small_meter(3).await;
        let stream = futures::stream::iter([
            Ok::<_, io::Error>(Bytes::from_static(b"ab")),
            Ok(Bytes::from_static(b"cd")),
        ]);
        let response = meter.response(Response::new(Body::from_stream(stream)), false);
        let mut body = response.into_body();
        assert_eq!(
            body.frame().await.unwrap().unwrap().into_data().unwrap(),
            "ab"
        );
        assert!(body.frame().await.unwrap().is_err());
        assert!(body.frame().await.is_none());
    }

    #[tokio::test]
    async fn websocket_backpressure_does_not_double_charge_reserved_bytes() {
        let (_directory, meter) = small_meter(5).await;
        let (writer, mut reader) = tokio::io::duplex(2);
        let mut writer = meter.io(writer);
        let mut received = [0; 5];
        let (sent, read) =
            tokio::join!(writer.write_all(b"hello"), reader.read_exact(&mut received));
        sent.unwrap();
        read.unwrap();
        assert_eq!(&received, b"hello");
        assert!(writer.write_all(b"x").await.is_err());
    }

    #[tokio::test]
    async fn absent_policy_is_not_an_unlimited_account() {
        let directory = tempfile::tempdir().unwrap();
        let ledger = Ledger::new(directory.path().to_path_buf());
        assert!(matches!(
            ledger.meter("brisk-otter-41").await,
            Err(Denied::Unavailable)
        ));
    }
}
