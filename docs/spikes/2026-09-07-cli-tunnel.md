# 2026-09-07 · `playtest <端口>` 的 CLI 这一半

**结论**：隧道模式的 CLI 侧写完了，能跑的部分在本机跑过：端口探测、令牌与作品、要授权、
握手、退避重连、Ctrl-C 干净退出、`--json` 事件流，都有下面的实际输出。**没有真边缘**
——`edge/src/tunnel/` 是另一位同事在做，控制面的 `POST /v1/sites/{slug}/tunnel` 这台机器上
还是 404。所以「拿到链接、手机扫码、玩家的请求打到 dev server」这几件事，**只在
`cli/tests/tunnel_flow.rs` 的假边缘上验过，没有真机记录**。

机器：macOS 26.2，rustc 1.96.0 (ac68faa20 2026-05-25)。命令都从仓库根目录执行。

## 一、端口上没东西在监听

这是最常见的第一次出错：先跑 playtest 再跑 dev server。

```
$ cargo run -q -p playtest -- 39998 --note "写点什么" --spa
--note、--spa 只在上传目录时有用，这次是隧道模式，先忽略了。
端口 39998 上没有东西在监听。先把你的开发服务器跑起来，再运行 playtest 39998。
退出码 1
```

用不上的参数只说一句就过去，不报错。同一条路的 `--json`：

```
$ cargo run -q -p playtest -- 39998 --json
{"ok":false,"code":"error","message":"端口 39998 上没有东西在监听。先把你的开发服务器跑起来，再运行 playtest 39998。","elapsed_ms":5}
退出码 1
```

## 二、控制面还没有这个接口

这台机器上跑着别人起的 `playtest-api`（`127.0.0.1:38787`）。匿名会话和建作品都通，
隧道授权那条路由还没有：

```
$ curl -s -X POST http://127.0.0.1:38787/v1/anon/sessions -d '{}'
{"token":"…","expires_at":"2026-09-08T10:20:35Z"}

$ curl -s -X POST http://127.0.0.1:38787/v1/sites -H "authorization: Bearer …" -d '{"title":"隧道手测"}'
{"slug":"scarlet-tiger-35","url":"http://scarlet-tiger-35.localhost:38443",…}

$ curl -s -i -X POST http://127.0.0.1:38787/v1/sites/scarlet-tiger-35/tunnel -H "authorization: Bearer …" -d '{}'
HTTP/1.1 404 Not Found
{"code":"not_found","message":"没有这个地址。请确认 CLI 和控制面的版本对得上。"}
```

CLI 遇到它的样子（本地 5173 上跑着一个 `python3 -m http.server`）：

```
$ cargo run -q -p playtest -- 5173 --api http://127.0.0.1:38787
服务器说：没有这个地址。请确认 CLI 和控制面的版本对得上。
退出码 6
```

这句话是控制面自己写的，CLI 原样转出来，没有编一个更好听的说法。退出码 6 来自
`client.rs` 已有的分类（`not_found` → 「给的东西有问题」）——对「控制面比 CLI 旧」这件事
不算贴切，但那张表是上传路径定的，这次没动它。

## 三、连不上边缘时的退避重连

真边缘没有，所以用一个临时的假控制面（`/tmp/playtest-manual/fake_control.py`，不在仓库里）
发一张 `connect_url` 指向死端口 `ws://127.0.0.1:39999/_playtest/tunnel` 的授权，
看 CLI 连不上时的行为。等待时间是「上一次的两倍」和「一半到全额之间的抖动」两条规则叠起来的：

```
$ cargo run -q -p playtest -- 5173 --api http://127.0.0.1:39001
连不上边缘：那个地址上没有人接，0.8 秒后重连（第 1 次）
连不上边缘：那个地址上没有人接，1.6 秒后重连（第 2 次）
连不上边缘：那个地址上没有人接，3.7 秒后重连（第 3 次）
连不上边缘：那个地址上没有人接，6.0 秒后重连（第 4 次）
连不上边缘：那个地址上没有人接，15.5 秒后重连（第 5 次）
已停止，链接现在显示离线。
退出码 0
```

同一条路跑久一点，看它在 30 秒封顶（这一轮的实测：0.9 / 1.1 / 2.0 / 6.3 / 12.8 / 26.3 /
23.8 / 17.6 秒——封顶之后每次都在 15–30 秒之间抖）。最后一行是 Ctrl-C，退避等待中途按也立刻响应。

`--json` 模式下同一件事：

```
$ cargo run -q -p playtest -- 5173 --api http://127.0.0.1:39001 --json --no-qr
{"event":"reconnecting","attempt":1,"wait_ms":831,"reason":"连不上边缘：那个地址上没有人接"}
{"event":"reconnecting","attempt":2,"wait_ms":1508,"reason":"连不上边缘：那个地址上没有人接"}
{"event":"reconnecting","attempt":3,"wait_ms":2537,"reason":"连不上边缘：那个地址上没有人接"}
{"event":"stopped","connections":0,"bytes":0}
```

**这一条改了一次文案**。第一次跑出来的是
`和服务器断开了（IO error: Connection refused (os error 61)），0.9 秒后重连（第 1 次）`
——两处不对：还没连上过，谈不上「断开」；tungstenite 的错误是英文的，第一次用的人看不懂
（AGENTS.md 第 8 条）。改成了上面的样子，常见的三种（对方拒绝、超时、网络不通）说人话，
实在归不了类的才带上原文。

## 四、TLS 用的是 rustls + ring

`tokio-tungstenite` 只开了 `__rustls-tls`，crypto provider 复用 `client.rs` 里
`install_crypto_provider` 装的 ring，根证书用 `webpki-roots`：

```
$ cargo tree -p playtest -i aws-lc-rs
error: package ID specification `aws-lc-rs` did not match any packages

$ cargo tree -p playtest -e normal | rg "tokio-tungstenite|rustls |ring "
│   ├── tokio-tungstenite v0.30.0
│   │   ├── rustls v0.23.43
│   │   │   ├── ring v0.17.14
│   │   ├── tokio-rustls v0.26.5
```

`openssl-sys` 和 `native-tls` 同样查过，都是「did not match any packages」。
真的握过 WSS 手没有——假边缘走的是 `ws://`，所以 **TLS 这条路只有依赖图，没有实际连接记录**。

## 五、自动化测试

```
$ cargo test -p playtest
test result: ok. 103 passed  （单元）
test result: ok. 10 passed   （inspect.rs）
test result: ok. 14 passed   （json_output.rs）
test result: ok. 4 passed    （mcp.rs）
test result: ok. 4 passed    （tunnel_flow.rs）
test result: ok. 7 passed    （upload_flow.rs）
```

`cli/tests/tunnel_flow.rs` 起三个东西：假控制面（axum，路由用
`playtest_common::api::routes` 的常量注册）、假边缘（裸 `TcpListener` +
`tokio_tungstenite::accept_hdr_async`，因为要校验握手头、还要按 409 拒绝握手，axum 的
`WebSocket` 拿不回 `WebSocketStream`）、假 dev server（手写 HTTP/1.1）。跑的是真二进制。
四条测试分别验：

1. 玩家的请求真的穿过 yamux 流到了 dev server 并原样回来；边缘关掉连接后 CLI 出
   `reconnecting` 并再次握手成功，且**没有再要一张授权**（令牌还有一小时，链接才不会变）。
2. 假边缘对第二次握手回 409 `tunnel_replaced` → CLI 退出码 1，stderr 里有「接管」。
3. 本地端口没监听 → 退出码 1，且没向控制面要过任何东西。
4. 人类模式下 stdout 只有链接，二维码和其余说明都在 stderr。

## 没验证的

- **整条路**。没有边缘，所以没拿到过一条真链接，没手机扫过码，没测过 Host 改写、
  WebSocket 穿隧道（玩家侧的 WS）、多个玩家同时连。边缘就绪后要一起补。
- **WSS**。见第四节。
- **驱逐**。409 `tunnel_replaced` 只在假边缘上验过；真边缘按令牌记驱逐名单这件事没验过。
- **令牌续期**。`expires_at` 前 5 分钟换新的判断有单元测试，但真机上跑满一小时没做过。
- **「这个页面要下载约 N MB」那条提示**。资源清点、`HEAD` 求和、10 MB 门槛、按 30 Mbps
  折算秒数，全部有单元测试；真机上没跑出过一次——要跑出来得先有边缘（提示是连上之后才说的）。
- **Vite 的 HMR 提示**同理：识别 `/@vite/client` 有单元测试和集成测试，
  但真机上没在一条真链接旁边看到过它。
- **Windows 与 Linux**。只在 macOS 上跑过。Ctrl-C 走的是 `tokio::signal::ctrl_c`。

## 已知的不够好

- **「端口上没东西在监听」在 `--json` 里是 `code: "error"`**，和「没预料到的错误」一个筐。
  这是给的东西有问题，更像 `output.rs` 里的退出码 6 那一类。本任务被要求用退出码 1，
  就先这样，但这两处（人类模式退出码 1、JSON 里 code 不具体）值得一起重新定。
- **磁盘**。这台机器上 `/System/Volumes/Data` 只剩 300–500 MB 且在波动，
  `target/` 已经 2.9 GB。这次有一段时间跌到 317 MB，按约定停下等了几分钟才继续编。
