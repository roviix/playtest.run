//! 把 yamux 的 `Connection` 包成一个能在两个任务之间传来传去的句柄。

use std::collections::VecDeque;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::{mpsc, oneshot, watch};
use tokio_util::compat::{Compat, FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};

/// 这一端在隧道上干什么。
///
/// 隧道是单向开流的：玩家连上来的是边缘，所以只有边缘开流，CLI 只管收。
/// 两端角色写死，省掉「谁能开流」的协商，也让流号的奇偶分配不会撞车。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// 边缘：每个玩家连接开一条流。
    Opener,
    /// CLI：收到流就去连 `127.0.0.1:<port>`。
    Acceptor,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MuxError {
    #[error("the tunnel is already closed")]
    Closed,
    #[error("yamux error: {0}")]
    Yamux(String),
    #[error("wrong role: only an Opener can open streams and only an Acceptor can accept them")]
    WrongRole,
}

impl From<yamux::ConnectionError> for MuxError {
    fn from(e: yamux::ConnectionError) -> Self {
        match e {
            yamux::ConnectionError::Closed => MuxError::Closed,
            other => MuxError::Yamux(other.to_string()),
        }
    }
}

/// 一条隧道上最多同时开这么多流。
///
/// 浏览器对同一个域一般并发 6 条连接，512 够几十个玩家同时在场；这个数也是背压：
/// 排队等 `accept` 的流占着名额，占满了 yamux 就会拒绝对端再开。
const MAX_NUM_STREAMS: usize = 512;

/// 连接上所有流加起来的接收窗口上限。
///
/// yamux 0.14 的单流窗口从 256 KiB（`yamux::DEFAULT_CREDIT`）起步，按带宽时延积自己往上长，
/// **没有**设定单流窗口初值的接口。窗口小对一个几十 MB 的 wasm 很要命：窗口没长起来之前，
/// 发送方每发满一个窗口就要停下来等窗口更新，一个来回一停。所以这里只能把连接级的天花板抬高，
/// 让它长得开。256 MiB 是上限不是预分配，同时也把「对端骗我们分配内存」的最坏情况钉死
/// （边缘那台机器只有 2 GB 内存）。yamux 要求它 >= 256 KiB × 最大流数，128 MiB，满足。
const MAX_CONNECTION_RECEIVE_WINDOW: usize = 256 * 1024 * 1024;

fn config() -> yamux::Config {
    let mut cfg = yamux::Config::default();
    cfg.set_max_connection_receive_window(Some(MAX_CONNECTION_RECEIVE_WINDOW));
    cfg.set_max_num_streams(MAX_NUM_STREAMS);
    // split_send_size 保持默认的 16 KiB。调大能省帧头，但一个大下载会把发送队列占得更久，
    // 联机消息只能排在后面——单条 TCP 上的队头阻塞正是 DESIGN §4.3 选 WSS 时认下的代价，
    // 不该再自己加重。
    cfg
}

enum Command {
    Open(oneshot::Sender<Result<yamux::Stream, MuxError>>),
    Close,
}

/// 一条隧道的多路复用句柄。
///
/// 真正干活的是 [`Mux::spawn`] 起的驾驭任务：yamux 的 `Connection` 是「不 poll 就不动」的，
/// 连已经开出去的流上的读写、窗口更新都要靠它推进，所以整条隧道的一生都得有人在那个循环里。
///
/// **把 `Mux` 丢掉就等于关闭隧道。** 会话结束时不想留着连接，所以驾驭任务看到句柄没了就收摊；
/// 想让流活得比句柄久，就把 `Mux` 一起留着。
pub struct Mux {
    role: Role,
    cmd: mpsc::UnboundedSender<Command>,
    inbound: mpsc::UnboundedReceiver<MuxStream>,
    done: watch::Receiver<Option<Result<(), MuxError>>>,
}

impl std::fmt::Debug for Mux {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mux").field("role", &self.role).finish()
    }
}

impl Mux {
    /// 在 `io` 上跑 yamux，返回句柄。`io` 一般是 [`super::WsByteStream`]。
    pub fn spawn<T>(io: T, role: Role) -> Self
    where
        T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        // yamux 按流号的奇偶决定谁能开流，两端必须一个 Client 一个 Server。
        let mode = match role {
            Role::Opener => yamux::Mode::Client,
            Role::Acceptor => yamux::Mode::Server,
        };
        let conn = yamux::Connection::new(TokioAsyncReadCompatExt::compat(io), config(), mode);

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        let (done_tx, done_rx) = watch::channel(None);

        tokio::spawn(drive(conn, role, cmd_rx, inbound_tx, done_tx));

        Self {
            role,
            cmd: cmd_tx,
            inbound: inbound_rx,
            done: done_rx,
        }
    }

    pub fn role(&self) -> Role {
        self.role
    }

    /// 开一条新流。只有 [`Role::Opener`] 能用。
    pub async fn open(&self) -> Result<MuxStream, MuxError> {
        if self.role != Role::Opener {
            return Err(MuxError::WrongRole);
        }
        let (tx, rx) = oneshot::channel();
        self.cmd
            .send(Command::Open(tx))
            .map_err(|_| MuxError::Closed)?;
        rx.await.map_err(|_| MuxError::Closed)?.map(MuxStream::new)
    }

    /// 等对端开一条流过来。连接结束返回 `None`；
    /// [`Role::Opener`] 调用也返回 `None`——那一端不该收流。
    pub async fn accept(&mut self) -> Option<MuxStream> {
        if self.role != Role::Acceptor {
            return None;
        }
        self.inbound.recv().await
    }

    /// 请求优雅关闭。不阻塞，关完了 [`Mux::closed`] 会完成。
    pub fn close(&self) {
        let _ = self.cmd.send(Command::Close);
    }

    /// 连接结束时完成。可以在句柄还在的时候先拿走，之后 `Mux` 丢了也照样能等。
    pub fn closed(&self) -> impl Future<Output = Result<(), MuxError>> + Send + 'static {
        let mut rx = self.done.clone();
        async move {
            loop {
                let settled = rx.borrow_and_update().clone();
                if let Some(result) = settled {
                    return result;
                }
                if rx.changed().await.is_err() {
                    // 驾驭任务没留下结果就没了（比如运行时正在收摊）。
                    return Err(MuxError::Closed);
                }
            }
        }
    }
}

async fn drive<T>(
    mut conn: yamux::Connection<Compat<T>>,
    role: Role,
    mut cmd: mpsc::UnboundedReceiver<Command>,
    inbound: mpsc::UnboundedSender<MuxStream>,
    done: watch::Sender<Option<Result<(), MuxError>>>,
) where
    T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let mut waiting: VecDeque<oneshot::Sender<Result<yamux::Stream, MuxError>>> = VecDeque::new();
    let mut closing = false;

    let result = std::future::poll_fn(|cx| {
        loop {
            if !closing {
                match cmd.poll_recv(cx) {
                    Poll::Ready(Some(Command::Open(tx))) => {
                        waiting.push_back(tx);
                        continue;
                    }
                    Poll::Ready(Some(Command::Close)) => {
                        closing = true;
                        continue;
                    }
                    // 句柄全没了：会话结束，收摊。
                    Poll::Ready(None) => {
                        closing = true;
                        continue;
                    }
                    Poll::Pending => {}
                }
            }

            if closing {
                return conn.poll_close(cx).map(|r| r.map_err(MuxError::from));
            }

            if !waiting.is_empty() {
                match conn.poll_new_outbound(cx) {
                    Poll::Ready(Ok(stream)) => {
                        if let Some(tx) = waiting.pop_front() {
                            let _ = tx.send(Ok(stream));
                        }
                        continue;
                    }
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e.into())),
                    Poll::Pending => {}
                }
            }

            // 这一句不只是「收流」：连接上所有的读写都靠它推进，所以每一轮都必须走到。
            match conn.poll_next_inbound(cx) {
                Poll::Ready(Some(Ok(stream))) => {
                    if role == Role::Acceptor {
                        // 送不进去说明没人在 accept 了，丢掉即可——drop 会给对端回一个 RST。
                        let _ = inbound.send(MuxStream::new(stream));
                    }
                    continue;
                }
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Err(e.into())),
                Poll::Ready(None) => return Poll::Ready(Ok(())),
                Poll::Pending => return Poll::Pending,
            }
        }
    })
    .await;

    // 连接没了，等在「开一条流」上的人要马上知道，不然会一直挂着。
    let reason = result.clone().err().unwrap_or(MuxError::Closed);
    for tx in waiting.drain(..) {
        let _ = tx.send(Err(reason.clone()));
    }
    cmd.close();
    while let Ok(pending) = cmd.try_recv() {
        if let Command::Open(tx) = pending {
            let _ = tx.send(Err(reason.clone()));
        }
    }

    let _ = done.send(Some(result));
    // inbound 在这里 drop，Acceptor 那边的 accept() 随之返回 None。
}

/// 隧道上的一条流：边缘那头是一个玩家连接，CLI 那头是一条到 `127.0.0.1:<port>` 的 TCP。
pub struct MuxStream {
    inner: Compat<yamux::Stream>,
}

impl MuxStream {
    fn new(stream: yamux::Stream) -> Self {
        Self {
            inner: FuturesAsyncReadCompatExt::compat(stream),
        }
    }

    /// 流号。日志里把「玩家的这一次请求」两端串起来用。
    pub fn id(&self) -> yamux::StreamId {
        self.inner.get_ref().id()
    }
}

impl std::fmt::Debug for MuxStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MuxStream").field("id", &self.id()).finish()
    }
}

impl AsyncRead for MuxStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for MuxStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    /// 半关闭：发一个 FIN，对端读到 EOF 但还能写回来。
    /// HTTP 的「请求发完了，等响应体」就靠这个语义。
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::ws_pair;
    use super::*;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    const LIMIT: Duration = Duration::from_secs(30);

    async fn mux_pair() -> (Mux, Mux) {
        let (edge, cli) = ws_pair().await;
        (
            Mux::spawn(edge, Role::Opener),
            Mux::spawn(cli, Role::Acceptor),
        )
    }

    #[tokio::test]
    async fn three_streams_echo_concurrently() {
        tokio::time::timeout(LIMIT, async {
            let (opener, mut acceptor) = mux_pair().await;

            let server = tokio::spawn(async move {
                for _ in 0..3 {
                    let mut stream = acceptor.accept().await.expect("该收到三条流");
                    tokio::spawn(async move {
                        let mut buf = Vec::new();
                        stream.read_to_end(&mut buf).await.unwrap();
                        stream.write_all(&buf).await.unwrap();
                        stream.shutdown().await.unwrap();
                    });
                }
                // 把句柄还回去：在这里 drop 会顺手关掉整条隧道。
                acceptor
            });

            let mut clients = Vec::new();
            for i in 0..3u8 {
                let mut stream = opener.open().await.unwrap();
                clients.push(tokio::spawn(async move {
                    let payload = vec![i; 4096 + i as usize];
                    stream.write_all(&payload).await.unwrap();
                    stream.shutdown().await.unwrap();
                    let mut back = Vec::new();
                    stream.read_to_end(&mut back).await.unwrap();
                    assert_eq!(back, payload, "第 {i} 条流回显的内容不对");
                }));
            }
            for c in clients {
                c.await.unwrap();
            }
            let _acceptor = server.await.unwrap();
        })
        .await
        .expect("超时：三条流并发回显不该卡住");
    }

    #[tokio::test]
    async fn four_mib_on_one_stream_does_not_deadlock() {
        tokio::time::timeout(LIMIT, async {
            // 4 MiB 远大于单流 256 KiB 的初始接收窗口，发送方必须靠窗口更新才能继续；
            // 窗口更新和数据走同一条连接，驾驭循环卡住的话这里就会挂死。
            const N: usize = 4 * 1024 * 1024;
            let (opener, mut acceptor) = mux_pair().await;

            let server = tokio::spawn(async move {
                let mut stream = acceptor.accept().await.expect("该收到一条流");
                let mut total = 0usize;
                let mut buf = vec![0u8; 64 * 1024];
                loop {
                    let n = stream.read(&mut buf).await.unwrap();
                    if n == 0 {
                        break;
                    }
                    total += n;
                }
                stream
                    .write_all(&(total as u64).to_be_bytes())
                    .await
                    .unwrap();
                stream.shutdown().await.unwrap();
                acceptor
            });

            let mut stream = opener.open().await.unwrap();
            stream.write_all(&vec![7u8; N]).await.unwrap();
            stream.shutdown().await.unwrap();

            let mut got = [0u8; 8];
            stream.read_exact(&mut got).await.unwrap();
            assert_eq!(u64::from_be_bytes(got), N as u64, "对端收到的字节数对不上");

            let _acceptor = server.await.unwrap();
        })
        .await
        .expect("超时：4 MiB 单流传输死锁了，多半是流控没推进");
    }

    #[tokio::test]
    async fn shutdown_is_half_close() {
        tokio::time::timeout(LIMIT, async {
            let (opener, mut acceptor) = mux_pair().await;

            let server = tokio::spawn(async move {
                let mut stream = acceptor.accept().await.expect("该收到一条流");
                let mut got = Vec::new();
                // 对端只关了写的那一半，这里就该读到 EOF。
                stream.read_to_end(&mut got).await.unwrap();
                assert_eq!(got, "请求".as_bytes());
                // 半关闭之后反方向还能写。
                stream.write_all("响应".as_bytes()).await.unwrap();
                stream.shutdown().await.unwrap();
                acceptor
            });

            let mut stream = opener.open().await.unwrap();
            stream.write_all("请求".as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();

            let mut back = Vec::new();
            stream.read_to_end(&mut back).await.unwrap();
            assert_eq!(
                back,
                "响应".as_bytes(),
                "半关闭之后应该还能收到对端写回来的东西"
            );

            let _acceptor = server.await.unwrap();
        })
        .await
        .expect("超时：半关闭语义不对");
    }

    #[tokio::test]
    async fn close_ends_the_peer_and_completes_closed() {
        tokio::time::timeout(LIMIT, async {
            let (opener, mut acceptor) = mux_pair().await;
            // 先把等待句柄拿在手上，验证它不依赖 Mux 还活着。
            let peer_closed = acceptor.closed();

            opener.close();
            assert_eq!(opener.closed().await, Ok(()), "主动关闭的一端该干净收场");

            assert!(
                acceptor.accept().await.is_none(),
                "对端关了之后不该再有新流进来"
            );
            // 被动感知关闭的一端，收到的是干净的收尾还是半个帧要看时序，这里只要求它一定会结束。
            let _ = peer_closed.await;

            assert_eq!(
                opener.open().await.unwrap_err(),
                MuxError::Closed,
                "关了之后不该还能开流"
            );
        })
        .await
        .expect("超时：close() 之后两端都该结束");
    }

    #[tokio::test]
    async fn roles_are_enforced() {
        tokio::time::timeout(LIMIT, async {
            let (mut opener, acceptor) = mux_pair().await;
            assert_eq!(acceptor.open().await.unwrap_err(), MuxError::WrongRole);
            assert!(opener.accept().await.is_none());
            assert_eq!(opener.role(), Role::Opener);
            assert_eq!(acceptor.role(), Role::Acceptor);
        })
        .await
        .expect("超时");
    }
}
