//! 把一条 WebSocket 当字节流用。

use std::collections::VecDeque;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{ready, Context, Poll};
use std::time::{Duration, Instant};

use bytes::Bytes;
use futures::task::AtomicWaker;
use futures::{Sink, Stream};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::time::{sleep, Instant as TokioInstant, Sleep};
use tokio_tungstenite::tungstenite::error::ProtocolError;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::{Error as WsError, Message, Utf8Bytes};
use tokio_tungstenite::WebSocketStream;

/// 一个 Binary 帧最多装这么多字节，再多就拆帧。
///
/// 帧不是越大越好：tungstenite 要把整帧攒齐才交出去，一个几 MB 的帧会让排在它后面的 Pong 和
/// Close 等很久，中间的反向代理对超大帧也不一定友好。yamux 默认按 16 KiB 切数据帧，
/// 所以实际写进来的多数本来就小于这个上限，64 KiB 只是兜底。
pub const MAX_FRAME_BYTES: usize = 64 * 1024;

/// 「还没有 Close」的哨兵。真的状态码是 u16，占不到这个值。
const NO_CLOSE_CODE: u32 = u32::MAX;

/// 「上一次收到帧是什么时候」。
///
/// 边缘要按 [`crate::tunnel::IDLE_TIMEOUT_SECS`] 判断隧道是不是还活着，但那时候
/// [`WsByteStream`] 已经被交给 [`super::Mux`] 拿不回来了，所以做成可以先克隆一份留在手上的句柄。
#[derive(Clone, Debug)]
pub struct ActivityClock(Arc<ActivityInner>);

#[derive(Debug)]
struct ActivityInner {
    /// `Instant` 塞不进原子量，这里存「相对 base 过了多少毫秒」，读的时候加回去。
    base: Instant,
    millis: AtomicU64,
}

impl ActivityClock {
    pub fn new() -> Self {
        Self(Arc::new(ActivityInner {
            base: Instant::now(),
            millis: AtomicU64::new(0),
        }))
    }

    fn touch(&self) {
        let millis = self.0.base.elapsed().as_millis() as u64;
        // fetch_max 而不是 store：多个克隆同时更新时不会把时间往回拨。
        self.0.millis.fetch_max(millis, Ordering::Relaxed);
    }

    pub fn last_activity(&self) -> Instant {
        self.0.base + Duration::from_millis(self.0.millis.load(Ordering::Relaxed))
    }

    pub fn idle_for(&self) -> Duration {
        self.last_activity().elapsed()
    }
}

impl Default for ActivityClock {
    fn default() -> Self {
        Self::new()
    }
}

/// 交出流之前先克隆一份留在手上的遥控器。
///
/// [`WsByteStream`] 一旦交给 [`super::Mux`] 就拿不回来了，而「主动带状态码关掉它」和
/// 「对端是用什么状态码关的」这两件事都发生在那之后：边缘挤掉旧隧道时要
/// [`WsControl::close`]，CLI 要靠 [`WsControl::close_code`] 区分自己是被
/// [`crate::tunnel::close::REPLACED`] 挤掉还是普通断线。
#[derive(Clone)]
pub struct WsControl(Arc<WsShared>);

struct WsShared {
    activity: ActivityClock,
    /// 记住最近一次 poll 这条流的任务。别的任务往 outbox 里塞了帧之后靠它把驾驭任务叫醒，
    /// 否则帧会一直躺着等下一次「刚好有人来读」。
    waker: AtomicWaker,
    /// 等着搭下一趟车发出去的控制帧。
    outbox: Mutex<VecDeque<Message>>,
    close_code: AtomicU32,
    close_queued: AtomicBool,
}

impl WsShared {
    fn new() -> Self {
        Self {
            activity: ActivityClock::new(),
            waker: AtomicWaker::new(),
            outbox: Mutex::new(VecDeque::new()),
            close_code: AtomicU32::new(NO_CLOSE_CODE),
            close_queued: AtomicBool::new(false),
        }
    }

    fn queue(&self, msg: Message) {
        if let Ok(mut outbox) = self.outbox.lock() {
            outbox.push_back(msg);
        }
        self.waker.wake();
    }

    fn has_queued(&self) -> bool {
        self.outbox.lock().map(|o| !o.is_empty()).unwrap_or(false)
    }

    fn take_queued(&self) -> Option<Message> {
        self.outbox.lock().ok().and_then(|mut o| o.pop_front())
    }

    fn drop_queued(&self) {
        if let Ok(mut outbox) = self.outbox.lock() {
            outbox.clear();
        }
    }

    /// 第一个 Close 才算数：我们发 4001 出去，对端多半原样回一个，别让它把原因盖掉。
    fn record_close(&self, code: u16) {
        let _ = self.close_code.compare_exchange(
            NO_CLOSE_CODE,
            u32::from(code),
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
    }

    fn close_code(&self) -> Option<u16> {
        match self.close_code.load(Ordering::SeqCst) {
            NO_CLOSE_CODE => None,
            code => Some(code as u16),
        }
    }
}

impl WsControl {
    /// 排一个带状态码的 Close 帧，流在下一次被 poll 时把它发出去并冲刷。
    ///
    /// 状态码用 [`crate::tunnel::close`] 里的那几个。之后这条流读侧是 EOF、写侧是
    /// `BrokenPipe`。重复调用只有第一次算数。
    pub fn close(&self, code: u16, reason: &str) {
        if self.0.close_queued.swap(true, Ordering::SeqCst) {
            return;
        }
        self.0.record_close(code);
        self.0.queue(Message::Close(Some(CloseFrame {
            code: CloseCode::from(code),
            reason: Utf8Bytes::from(reason),
        })));
    }

    /// 手动发一个 Ping。开了 [`WsByteStream::with_keepalive`] 就不用自己管这个。
    pub fn ping(&self) {
        self.0.queue(Message::Ping(Bytes::new()));
    }

    /// 这条 WebSocket 是用什么状态码关的。自己关的和对端关的都记在这里；
    /// 对端关的时候没带状态码（比如普通的 `shutdown`）就是 `None`。
    pub fn close_code(&self) -> Option<u16> {
        self.0.close_code()
    }

    pub fn last_activity(&self) -> Instant {
        self.0.activity.last_activity()
    }

    pub fn idle_for(&self) -> Duration {
        self.0.activity.idle_for()
    }

    pub fn activity(&self) -> ActivityClock {
        self.0.activity.clone()
    }
}

impl std::fmt::Debug for WsControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WsControl")
            .field("close_code", &self.close_code())
            .finish()
    }
}

struct Keepalive {
    interval: Duration,
    /// `Sleep` 是 `!Unpin`，装箱之后 `WsByteStream` 才能继续保持 `Unpin`。
    timer: Pin<Box<Sleep>>,
}

/// 把一条 WebSocket 当成 tokio 的字节流：每个 Binary 帧是流里的一段。
///
/// `S` 是泛型，因为两端拿到的底层类型不一样——边缘是 axum 升级出来的连接，CLI 是
/// `connect_async` 得到的 TLS 或明文 TCP。
///
/// 只支持被**单个任务**驱动。tungstenite 内部给读和写各留了一个 waker 槽位，
/// 用 `tokio::io::split` 把读写分到两个任务会互相覆盖。隧道上驾驭 yamux 的就是单个任务，够用。
pub struct WsByteStream<S> {
    ws: WebSocketStream<S>,
    /// 当前 Binary 帧里还没交给 `poll_read` 的部分。
    pending: Bytes,
    /// 收到 Close 帧或底层结束，之后 `poll_read` 一律给 EOF。
    read_eof: bool,
    /// 对端发过 Close。WebSocket 没有 TCP 那样的半关闭，收到 Close 之后再发数据就是协议错误，
    /// 所以写侧直接给 `BrokenPipe`，而不是等 tungstenite 把错误抛回来。
    peer_closed: bool,
    write_closed: bool,
    /// 有东西排在 tungstenite 的写缓冲里等冲刷（自动 Pong、Close 回复、控制帧）。
    needs_flush: bool,
    /// 自己排的 Close 已经交给 tungstenite 了，等冲刷完再把读侧也收掉——
    /// 提前收会让调用方一读到 EOF 就把流丢了，Close 帧还躺在缓冲里没出门。
    eof_after_flush: bool,
    keepalive: Option<Keepalive>,
    shared: Arc<WsShared>,
}

impl<S> WsByteStream<S> {
    pub fn new(ws: WebSocketStream<S>) -> Self {
        Self {
            ws,
            pending: Bytes::new(),
            read_eof: false,
            peer_closed: false,
            write_closed: false,
            needs_flush: false,
            eof_after_flush: false,
            keepalive: None,
            shared: Arc::new(WsShared::new()),
        }
    }

    /// 开内建保活：距离上一次**发出或收到**任何帧超过 `interval` 就发一个 Ping。
    ///
    /// 不另起任务——驾驭 yamux 的那个任务本来就一直在 poll 这条流。但计时用的是流里面自己的
    /// 一个 `Sleep`，到点会主动唤醒那个任务，所以对端完全静默也照发不误，不靠「刚好有人在读」。
    ///
    /// 要在 tokio 运行时里调用（建定时器需要）。CLI 用 [`crate::tunnel::KEEPALIVE_INTERVAL_SECS`]。
    pub fn with_keepalive(mut self, interval: Duration) -> Self {
        self.keepalive = Some(Keepalive {
            interval,
            timer: Box::pin(sleep(interval)),
        });
        self
    }

    /// 复制一份遥控器，好在这个流被 [`super::Mux`] 拿走之后还能关它、看它。
    pub fn control(&self) -> WsControl {
        WsControl(self.shared.clone())
    }

    /// 这条 WebSocket 是用什么状态码关的，还没关就是 `None`。
    pub fn close_code(&self) -> Option<u16> {
        self.shared.close_code()
    }

    /// 上一次收到任何帧（含 Ping / Pong）的时刻。
    pub fn last_activity(&self) -> Instant {
        self.shared.activity.last_activity()
    }

    /// 复制一份活动时钟，好在这个流被 [`super::Mux`] 拿走之后继续看它。
    pub fn activity(&self) -> ActivityClock {
        self.shared.activity.clone()
    }

    pub fn get_ref(&self) -> &WebSocketStream<S> {
        &self.ws
    }

    /// 收到帧：空闲判定和保活计时一起往后推。
    fn saw_frame(&mut self) {
        self.shared.activity.touch();
        self.push_keepalive();
    }

    /// 发出帧：只推保活计时。空闲判定看的是对端还在不在，自己说话不算数。
    fn push_keepalive(&mut self) {
        if let Some(ka) = self.keepalive.as_mut() {
            let next = TokioInstant::now() + ka.interval;
            ka.timer.as_mut().reset(next);
        }
    }

    fn finish_close(&mut self) {
        if self.eof_after_flush {
            self.eof_after_flush = false;
            self.read_eof = true;
        }
    }
}

impl<S> WsByteStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    /// 这条流每次被 poll 都先走一遍：登记 waker、看保活到点没、把排队的控制帧发出去、冲刷。
    ///
    /// 自己不返回 `Pending`——控制帧一时写不出去不该把读堵住，下一次 poll 接着来。
    fn poll_control(&mut self, cx: &mut Context<'_>) {
        // 先登记再看队列。反过来的话，别的任务「塞帧 + wake」正好挤在两步中间就会漏掉唤醒。
        self.shared.waker.register(cx.waker());

        let due = self.keepalive.as_mut().is_some_and(|ka| {
            if ka.timer.as_mut().poll(cx).is_pending() {
                return false;
            }
            let next = TokioInstant::now() + ka.interval;
            ka.timer.as_mut().reset(next);
            // 重新 poll 一次把新的到点时间注册进去，否则下一个 Ping 没人来叫。
            let _ = ka.timer.as_mut().poll(cx);
            true
        });
        if due {
            self.shared.queue(Message::Ping(Bytes::new()));
        }

        while self.shared.has_queued() {
            match Pin::new(&mut self.ws).poll_ready(cx) {
                Poll::Ready(Ok(())) => {}
                // 连接已经不行了，排着的控制帧再也发不出去，别攒着。
                Poll::Ready(Err(_)) => {
                    self.shared.drop_queued();
                    break;
                }
                Poll::Pending => break,
            }
            let Some(msg) = self.shared.take_queued() else {
                break;
            };
            let is_close = matches!(msg, Message::Close(_));
            if Pin::new(&mut self.ws).start_send(msg).is_err() {
                self.shared.drop_queued();
                break;
            }
            self.needs_flush = true;
            self.push_keepalive();
            if is_close {
                self.write_closed = true;
                self.eof_after_flush = true;
            }
        }

        if self.needs_flush && Pin::new(&mut self.ws).poll_flush(cx).is_ready() {
            self.needs_flush = false;
            self.finish_close();
        }
    }
}

impl<S> std::fmt::Debug for WsByteStream<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WsByteStream")
            .field("pending", &self.pending.len())
            .field("read_eof", &self.read_eof)
            .field("peer_closed", &self.peer_closed)
            .field("write_closed", &self.write_closed)
            .field("close_code", &self.close_code())
            .field("keepalive", &self.keepalive.as_ref().map(|k| k.interval))
            .finish()
    }
}

impl<S> AsyncRead for WsByteStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            if !this.pending.is_empty() {
                let n = this.pending.len().min(buf.remaining());
                buf.put_slice(&this.pending[..n]);
                let _ = this.pending.split_to(n);
                return Poll::Ready(Ok(()));
            }

            // 保活、遥控器排的 Close、tungstenite 自动排的 Pong 都在这里出门。
            // 驾驭 yamux 的任务一直在读，所以不必为它们单开一个任务。
            this.poll_control(cx);

            if this.read_eof {
                return Poll::Ready(Ok(()));
            }

            match Pin::new(&mut this.ws).poll_next(cx) {
                Poll::Ready(Some(Ok(msg))) => {
                    this.saw_frame();
                    match msg {
                        Message::Binary(data) => this.pending = data,
                        // tungstenite 读到 Ping 会自动排一个 Pong，我们只负责让它出门。
                        Message::Ping(_) => this.needs_flush = true,
                        Message::Close(frame) => {
                            if let Some(frame) = &frame {
                                this.shared.record_close(u16::from(frame.code));
                            }
                            this.read_eof = true;
                            this.peer_closed = true;
                            this.needs_flush = true;
                        }
                        // Text 在这条隧道上没有含义（字节流只走 Binary），Pong 只用来更新活动时间。
                        Message::Pong(_) | Message::Text(_) | Message::Frame(_) => {}
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    this.read_eof = true;
                    return if is_eof(&e) {
                        Poll::Ready(Ok(()))
                    } else {
                        Poll::Ready(Err(to_io(e)))
                    };
                }
                Poll::Ready(None) => {
                    this.read_eof = true;
                    return Poll::Ready(Ok(()));
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl<S> AsyncWrite for WsByteStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        this.poll_control(cx);
        if this.write_closed || this.peer_closed {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "WebSocket 已经关了，写不进去",
            )));
        }
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        // poll_ready 在写缓冲满的时候会先冲刷，背压就落在这里。
        ready!(Pin::new(&mut this.ws).poll_ready(cx)).map_err(write_err)?;

        // 一次只收下一帧的量，剩下的让调用方再来一次——AsyncWrite 本来就允许短写。
        let n = buf.len().min(MAX_FRAME_BYTES);
        Pin::new(&mut this.ws)
            .start_send(Message::Binary(Bytes::copy_from_slice(&buf[..n])))
            .map_err(write_err)?;
        this.push_keepalive();
        Poll::Ready(Ok(n))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        this.poll_control(cx);
        let r = ready!(Pin::new(&mut this.ws).poll_flush(cx));
        this.needs_flush = false;
        this.finish_close();
        Poll::Ready(r.map_err(write_err))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        this.poll_control(cx);
        if this.write_closed {
            // 遥控器已经排过 Close 了，确认它冲出去就算关完，不要再发第二个。
            let r = ready!(Pin::new(&mut this.ws).poll_flush(cx));
            this.needs_flush = false;
            this.finish_close();
            return Poll::Ready(r.map_err(write_err));
        }
        // tungstenite 的 poll_close 就是「发 Close 帧再冲刷」。
        let r = ready!(Pin::new(&mut this.ws).poll_close(cx));
        this.write_closed = true;
        this.needs_flush = false;
        Poll::Ready(r.map_err(write_err))
    }
}

/// 这个错误对字节流来说只是「读到头了」，不是故障。
fn is_eof(e: &WsError) -> bool {
    match e {
        WsError::ConnectionClosed | WsError::AlreadyClosed => true,
        // 对端没走关闭握手就断了 TCP。对上层的拷贝循环来说和读到 EOF 没区别。
        WsError::Protocol(ProtocolError::ResetWithoutClosingHandshake) => true,
        WsError::Io(e) => matches!(
            e.kind(),
            io::ErrorKind::UnexpectedEof
                | io::ErrorKind::ConnectionReset
                | io::ErrorKind::ConnectionAborted
                | io::ErrorKind::BrokenPipe
        ),
        _ => false,
    }
}

fn to_io(e: WsError) -> io::Error {
    match e {
        WsError::Io(e) => e,
        other => io::Error::other(other),
    }
}

fn write_err(e: WsError) -> io::Error {
    if is_eof(&e) || matches!(e, WsError::Protocol(ProtocolError::SendAfterClosing)) {
        io::Error::new(io::ErrorKind::BrokenPipe, e.to_string())
    } else {
        to_io(e)
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{raw_pair, ws_pair};
    use super::*;
    use crate::tunnel;
    use futures::{SinkExt, StreamExt};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    const LIMIT: Duration = Duration::from_secs(10);

    #[tokio::test]
    async fn round_trips_a_chunk() {
        tokio::time::timeout(LIMIT, async {
            let (mut a, mut b) = ws_pair().await;
            a.write_all(b"hello playtest").await.unwrap();
            a.flush().await.unwrap();

            let mut got = [0u8; 14];
            b.read_exact(&mut got).await.unwrap();
            assert_eq!(&got, b"hello playtest");
        })
        .await
        .expect("超时：一段小数据的往返不该卡住");
    }

    #[tokio::test]
    async fn large_write_is_split_into_several_frames() {
        tokio::time::timeout(LIMIT, async {
            const N: usize = 200 * 1024;
            // 收侧用没包装过的 WebSocketStream，好数清楚到底发了几帧。
            let (client, server) = raw_pair().await;
            let mut tx = WsByteStream::new(client);
            let payload: Vec<u8> = (0..N).map(|i| (i % 251) as u8).collect();

            let reader = tokio::spawn(async move {
                let mut rx = server;
                let mut frames = 0usize;
                let mut got: Vec<u8> = Vec::with_capacity(N);
                while got.len() < N {
                    match rx.next().await {
                        Some(Ok(Message::Binary(b))) => {
                            frames += 1;
                            got.extend_from_slice(&b);
                        }
                        Some(Ok(_)) => {}
                        other => panic!("还没收完就结束了：{other:?}"),
                    }
                }
                (frames, got)
            });

            tx.write_all(&payload).await.unwrap();
            tx.flush().await.unwrap();

            let (frames, got) = reader.await.unwrap();
            assert_eq!(got, payload, "拆帧之后重组出来的字节要和写进去的一样");
            assert!(frames > 1, "200 KiB 应该拆成多帧，实际只有 {frames} 帧");
        })
        .await
        .expect("超时：大块写入不该卡住");
    }

    #[tokio::test]
    async fn shutdown_gives_the_peer_eof_and_breaks_its_write() {
        tokio::time::timeout(LIMIT, async {
            let (mut a, mut b) = ws_pair().await;
            a.write_all(b"tail").await.unwrap();
            a.shutdown().await.unwrap();

            let mut got = Vec::new();
            b.read_to_end(&mut got).await.unwrap();
            assert_eq!(got, b"tail", "关闭之前写的数据不能丢");

            let err = b.write_all(b"x").await.unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
        })
        .await
        .expect("超时：一端关闭之后另一端该立刻读到 EOF");
    }

    #[tokio::test]
    async fn ping_is_answered_with_pong_on_the_wire() {
        tokio::time::timeout(LIMIT, async {
            let (client, server) = raw_pair().await;
            let mut peer = client;
            let mut ours = WsByteStream::new(server);

            peer.send(Message::Ping(Bytes::from_static(b"pt"))).await.unwrap();
            peer.send(Message::Binary(Bytes::from_static(b"after-ping"))).await.unwrap();

            let mut got = [0u8; 10];
            ours.read_exact(&mut got).await.unwrap();
            assert_eq!(&got, b"after-ping", "Ping 不能把后面的数据挡住");

            // Pong 必须真的写回线上：只排队不冲刷的话，对端的保活会把活着的隧道判成离线。
            loop {
                match peer.next().await {
                    Some(Ok(Message::Pong(p))) => {
                        assert_eq!(&p[..], b"pt");
                        break;
                    }
                    Some(Ok(_)) => continue,
                    other => panic!("没等到 Pong：{other:?}"),
                }
            }
            assert!(ours.last_activity().elapsed() < Duration::from_secs(5));
        })
        .await
        .expect("超时：Ping 应该被自动 Pong 掉");
    }

    #[tokio::test]
    async fn text_and_pong_frames_are_skipped() {
        tokio::time::timeout(LIMIT, async {
            let (client, server) = raw_pair().await;
            let mut peer = client;
            let mut ours = WsByteStream::new(server);

            peer.send(Message::text("这条不该进字节流")).await.unwrap();
            peer.send(Message::Pong(Bytes::from_static(b"unsolicited"))).await.unwrap();
            peer.send(Message::Binary(Bytes::from_static(b"real"))).await.unwrap();

            let mut got = [0u8; 4];
            ours.read_exact(&mut got).await.unwrap();
            assert_eq!(&got, b"real");
        })
        .await
        .expect("超时：Text / Pong 应该被跳过而不是卡住");
    }

    #[tokio::test]
    async fn activity_clock_survives_being_cloned_out() {
        tokio::time::timeout(LIMIT, async {
            let (mut a, b) = ws_pair().await;
            let clock = b.activity();
            let before = clock.last_activity();

            // 时钟的分辨率是毫秒，先让时间走过一格再说。
            tokio::time::sleep(Duration::from_millis(5)).await;
            a.write_all(b"tick").await.unwrap();
            a.flush().await.unwrap();

            let mut b = b;
            let mut got = [0u8; 4];
            b.read_exact(&mut got).await.unwrap();

            assert!(clock.last_activity() > before, "收到帧之后活动时间要往前走");
            assert!(clock.idle_for() < Duration::from_secs(5));
        })
        .await
        .expect("超时");
    }

    #[tokio::test]
    async fn keepalive_pings_a_silent_peer() {
        tokio::time::timeout(LIMIT, async {
            let (client, server) = raw_pair().await;
            let mut peer = client;
            let ours = WsByteStream::new(server).with_keepalive(Duration::from_millis(300));

            // 模拟驾驭 yamux 的那个任务：它只是一直在读，从不写。对端全程静默，
            // 所以 Ping 只可能是流里面那个定时器自己叫醒这个任务发出来的。
            let driver = tokio::spawn(async move {
                let mut ours = ours;
                let mut buf = [0u8; 1];
                let _ = ours.read(&mut buf).await;
            });

            let ping = tokio::time::timeout(Duration::from_secs(1), async {
                loop {
                    match peer.next().await {
                        Some(Ok(Message::Ping(_))) => return true,
                        Some(Ok(_)) => continue,
                        _ => return false,
                    }
                }
            })
            .await
            .unwrap_or(false);

            driver.abort();
            assert!(ping, "300 ms 保活、对端静默，1 秒内该收到至少一个 Ping");
        })
        .await
        .expect("超时");
    }

    #[tokio::test]
    async fn control_close_carries_the_status_code_and_ends_the_stream() {
        tokio::time::timeout(LIMIT, async {
            let (client, server) = raw_pair().await;
            let mut peer = client;
            let mut ours = WsByteStream::new(server);
            // 边缘就是这么用的：先留一份遥控器，再把流交给 Mux。
            let control = ours.control();

            control.close(tunnel::close::REPLACED, "replaced");

            let mut buf = [0u8; 8];
            assert_eq!(
                ours.read(&mut buf).await.unwrap(),
                0,
                "排了 Close 之后读侧该是 EOF"
            );

            loop {
                match peer.next().await {
                    Some(Ok(Message::Close(Some(frame)))) => {
                        assert_eq!(u16::from(frame.code), tunnel::close::REPLACED);
                        assert_eq!(frame.reason.as_str(), "replaced");
                        break;
                    }
                    Some(Ok(_)) => continue,
                    other => panic!("对端没收到 Close：{other:?}"),
                }
            }

            assert_eq!(control.close_code(), Some(tunnel::close::REPLACED));
            assert_eq!(ours.close_code(), Some(tunnel::close::REPLACED));
            let err = ours.write(b"x").await.unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::BrokenPipe, "关了之后写侧该是坏管子");
        })
        .await
        .expect("超时：control().close() 之后两边都该收场");
    }

    #[tokio::test]
    async fn peer_close_code_reaches_the_cloned_handle() {
        tokio::time::timeout(LIMIT, async {
            let (client, server) = raw_pair().await;
            let mut peer = client;
            let mut ours = WsByteStream::new(server);
            let control = ours.control();
            assert_eq!(control.close_code(), None, "还没关的时候不该有状态码");

            peer.send(Message::Close(Some(CloseFrame {
                code: CloseCode::from(tunnel::close::TOKEN_EXPIRED),
                reason: Utf8Bytes::from("token expired"),
            })))
            .await
            .unwrap();

            let mut buf = [0u8; 8];
            assert_eq!(ours.read(&mut buf).await.unwrap(), 0);
            // CLI 靠这个区分「该换令牌重连」和「被挤掉了该退出」。
            assert_eq!(control.close_code(), Some(tunnel::close::TOKEN_EXPIRED));
        })
        .await
        .expect("超时：对端的关闭状态码该能传到句柄上");
    }

    #[tokio::test]
    async fn control_ping_reaches_the_peer() {
        tokio::time::timeout(LIMIT, async {
            let (client, server) = raw_pair().await;
            let mut peer = client;
            let ours = WsByteStream::new(server);
            let control = ours.control();

            let driver = tokio::spawn(async move {
                let mut ours = ours;
                let mut buf = [0u8; 1];
                let _ = ours.read(&mut buf).await;
            });

            // 此刻驾驭任务多半已经 Pending 在读上了，全靠 waker 把它叫回来发这个 Ping。
            tokio::time::sleep(Duration::from_millis(20)).await;
            control.ping();

            let ping = tokio::time::timeout(Duration::from_secs(2), async {
                loop {
                    match peer.next().await {
                        Some(Ok(Message::Ping(_))) => return true,
                        Some(Ok(_)) => continue,
                        _ => return false,
                    }
                }
            })
            .await
            .unwrap_or(false);

            driver.abort();
            assert!(ping, "遥控器发的 Ping 该把驾驭任务唤醒并真的写出去");
        })
        .await
        .expect("超时");
    }
}
