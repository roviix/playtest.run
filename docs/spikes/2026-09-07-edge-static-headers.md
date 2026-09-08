# 2026-09-07 · 边缘的响应头、门禁页与第一层数据（本机真机）

在本机把 `playtest-edge` 跑起来，用 `fixtures/headers-lab/export` 当作品，逐条看玩家真正收到的响应头。
下面每一条都是实际执行过的命令和实际看到的输出。

`api` 与 `cli` 还没通，所以对象存储是手写的：一个一次性 node 脚本（放 `/tmp`，没进仓库）遍历
`fixtures/headers-lab/export`，按 `common/src/store.rs` 的布局写 `blobs/<hh>/<hash>`、
`sites/<slug>/manifests/7.json`、`sites/<slug>/current.json`，造了四个作品：

| slug | 用来看什么 |
|---|---|
| `brisk-otter-41` | 普通作品，`gate: once`、`badge: true`、有 `note` |
| `keen-gecko-9` | `isolated: true` |
| `swift-heron-3` | `expires_at` 已过（`2026-09-06T00:00:00Z`） |
| `jolly-puffin-8` | `expires_at` 未过（`2026-09-08T04:30:00Z`） |

七个文件：`Build/hello.wasm.br`、`data.bin`（1 MiB）、`index.html`、`mod.wasm`、`mod.wasm.br`、
`mod.wasm.gz`、`sub/index.html`。同一个 wasm 有未压缩、`.br`、`.gz` 三份，正好把两条预压缩规则都试到。

## 起进程

用默认端口起一次，看它自己说了什么：

```
$ PLAYTEST_DATA_DIR=/tmp/pt-edge-spike RUST_LOG=info cargo run -p playtest-edge
2026-09-07T04:05:59.374742Z  INFO 边缘在 http://127.0.0.1:8443 上（明文，没有 TLS）
2026-09-07T04:05:59.374882Z  INFO 作品从 /tmp/pt-edge-spike/store 读，事件写到 /tmp/pt-edge-spike/edge-events.jsonl
2026-09-07T04:05:59.374889Z  INFO 一个作品就是一个 http://<slug>.localhost
```

对象存储还不存在时会先说出来，而不是等玩家点开才 404：

```
$ PLAYTEST_DATA_DIR=/tmp/pt-edge-spike3 PLAYTEST_EDGE_LISTEN=127.0.0.1:8445 RUST_LOG=warn cargo run -q -p playtest-edge
2026-09-07T04:16:32.030901Z  WARN /tmp/pt-edge-spike3/store 还不存在——api 往里写过东西之后作品才打得开
```

**下面第 1–8 节的响应头全部来自听 8444 的那个实例**（8443 上还留着一个早先编译的进程，换个端口是为了让每一条
都对得上同一个二进制）。

## 1. 健康检查

```
$ curl -sS -i -H 'Host: 127.0.0.1:8444' http://127.0.0.1:8444/_playtest/healthz
HTTP/1.1 200 OK
content-type: text/plain; charset=utf-8
x-content-type-options: nosniff
content-length: 3
date: Mon, 07 Sep 2026 04:10:13 GMT

ok
```

Host 是 IP 也能答——探针不用假装自己是某个作品。

## 2. 门禁页：导航请求出，带 cookie 就不出

```
$ curl -sD- -o/dev/null -H 'Host: brisk-otter-41.localhost:8444' \
    -H 'Accept: text/html' -H 'Sec-Fetch-Dest: document' http://127.0.0.1:8444/
HTTP/1.1 200 OK
content-type: text/html; charset=utf-8
x-content-type-options: nosniff
cache-control: no-store
content-length: 3562
```

```
$ curl -sD- -o/dev/null -H 'Host: brisk-otter-41.localhost:8444' \
    -H 'Accept: text/html' -H 'Sec-Fetch-Dest: document' -H 'Cookie: pt_gate=1' http://127.0.0.1:8444/
HTTP/1.1 200 OK
x-content-type-options: nosniff
accept-ranges: bytes
etag: "51da0473356260c2c5b65e32bad6f153460488184a6359944a6965e1024f5a8e"
cache-control: no-cache
content-type: text/html; charset=utf-8
content-length: 9110
```

9110 就是 `fixtures/headers-lab/export/index.html` 的大小，出的是作品自己的 HTML。
门禁页 3562 字节，没有到期时间时整页一个 `<script` 都没有：

```
$ curl -s -H 'Host: brisk-otter-41.localhost:8444' -H 'Accept: text/html' http://127.0.0.1:8444/ | grep -c '<script'
0
```

有到期时间时多一行和一段内联脚本（脚本只把绝对时间换成访客本地时区，关掉 JS 仍显示 UTC+8）：

```
$ curl -s -H 'Host: jolly-puffin-8.localhost:8444' -H 'Accept: text/html' http://127.0.0.1:8444/ | grep 后失效
<p class="meta">这个链接在 <time datetime="2026-09-08T04:30:00Z">9月8日 12:30（UTC+8）</time> 后失效</p>
```

## 3. `.wasm` 的 MIME 与两条预压缩规则

`mod.wasm`、`mod.wasm.br`、`mod.wasm.gz` 三份都在清单里。

不接受任何压缩 → 出未压缩那份，但因为有 `.br` / `.gz` 兄弟，仍然要 `Vary`：

```
$ curl -sD- -o/dev/null -H 'Host: brisk-otter-41.localhost:8444' -H 'Accept-Encoding:' http://127.0.0.1:8444/mod.wasm
HTTP/1.1 200 OK
x-content-type-options: nosniff
accept-ranges: bytes
etag: "f61fd62f57c41269c3c23f360eeaf1090b1db9c38651106674d48bc65dba88ba"
cache-control: public, max-age=0, must-revalidate
vary: Accept-Encoding
content-type: application/wasm
content-length: 41
```

接受 br → 换成 `.br` 那份，`Content-Type` 仍是里层的 `application/wasm`，ETag 跟着换：

```
$ curl -sD- -o/dev/null -H 'Host: brisk-otter-41.localhost:8444' -H 'Accept-Encoding: br' http://127.0.0.1:8444/mod.wasm
HTTP/1.1 200 OK
etag: "877014e3675bbbdcbb03951a0c09d4f8954329c9ddec2540dc3a6f2414088715"
cache-control: public, max-age=0, must-revalidate
vary: Accept-Encoding
content-encoding: br
content-type: application/wasm
content-length: 42
```

只接受 gzip → 出 `.gz`：

```
$ curl -sD- -o/dev/null -H 'Host: brisk-otter-41.localhost:8444' -H 'Accept-Encoding: gzip' http://127.0.0.1:8444/mod.wasm
HTTP/1.1 200 OK
etag: "ea8c2491efd050b47261f5272107a8eebfceac19824438a775bfde774efa0cb2"
vary: Accept-Encoding
content-encoding: gzip
content-type: application/wasm
content-length: 61
```

Unity 的加载器直接请求 `Build/x.wasm.br` 这种路径。显式请求就照出，`Content-Type` 按去掉压缩后缀的
里层扩展名定，这条路径不会随 `Accept-Encoding` 变，所以没有 `Vary`：

```
$ curl -sD- -o/dev/null -H 'Host: brisk-otter-41.localhost:8444' -H 'Accept-Encoding:' \
    http://127.0.0.1:8444/Build/hello.wasm.br
HTTP/1.1 200 OK
etag: "877014e3675bbbdcbb03951a0c09d4f8954329c9ddec2540dc3a6f2414088715"
cache-control: public, max-age=0, must-revalidate
content-encoding: br
content-type: application/wasm
content-length: 42
```

## 4. Range

```
$ curl -sD- -o/dev/null -H 'Host: brisk-otter-41.localhost:8444' -H 'Range: bytes=0-9' http://127.0.0.1:8444/data.bin
HTTP/1.1 206 Partial Content
accept-ranges: bytes
etag: "631b84027d6b9e52b539c4e8373622d23032dfadc64d60af87339c9037e4f769"
cache-control: public, max-age=0, must-revalidate
content-range: bytes 0-9/1048576
content-type: application/octet-stream
content-length: 10
```

```
$ curl -sD- -o/dev/null -H 'Host: brisk-otter-41.localhost:8444' -H 'Range: bytes=999999999-' http://127.0.0.1:8444/data.bin
HTTP/1.1 416 Range Not Satisfiable
content-type: text/plain; charset=utf-8
accept-ranges: bytes
content-range: bytes */1048576
content-length: 37
```

`HEAD` 只报长度不给体：

```
$ curl -sI -H 'Host: brisk-otter-41.localhost:8444' http://127.0.0.1:8444/data.bin
HTTP/1.1 200 OK
accept-ranges: bytes
etag: "631b84027d6b9e52b539c4e8373622d23032dfadc64d60af87339c9037e4f769"
cache-control: public, max-age=0, must-revalidate
content-type: application/octet-stream
content-length: 1048576
```

1 MiB 整包流下来字节没变（边缘用 `tokio::fs::File` + `ReaderStream`，不整读进内存）：

```
$ curl -s -H 'Host: brisk-otter-41.localhost:8444' -H 'Accept-Encoding:' http://127.0.0.1:8444/data.bin | shasum -a 256
631b84027d6b9e52b539c4e8373622d23032dfadc64d60af87339c9037e4f769  -
$ shasum -a 256 fixtures/headers-lab/export/data.bin
631b84027d6b9e52b539c4e8373622d23032dfadc64d60af87339c9037e4f769  fixtures/headers-lab/export/data.bin
```

## 5. 条件请求

```
$ curl -sD- -o/dev/null -H 'Host: brisk-otter-41.localhost:8444' -H 'Accept-Encoding:' \
    -H 'If-None-Match: "f61fd62f57c41269c3c23f360eeaf1090b1db9c38651106674d48bc65dba88ba"' \
    http://127.0.0.1:8444/mod.wasm
HTTP/1.1 304 Not Modified
accept-ranges: bytes
etag: "f61fd62f57c41269c3c23f360eeaf1090b1db9c38651106674d48bc65dba88ba"
cache-control: public, max-age=0, must-revalidate
vary: Accept-Encoding
```

## 6. 跨源隔离

`keen-gecko-9` 的清单 `isolated: true`：

```
$ curl -sD- -o/dev/null -H 'Host: keen-gecko-9.localhost:8444' -H 'Accept-Encoding:' http://127.0.0.1:8444/mod.wasm
HTTP/1.1 200 OK
cross-origin-opener-policy: same-origin
cross-origin-embedder-policy: require-corp
cross-origin-resource-policy: same-origin
content-type: application/wasm
content-length: 41
```

## 7. 路径与作品状态

```
$ curl -sD- -o/dev/null -H 'Host: brisk-otter-41.localhost:8444' -H 'Accept: text/html' http://127.0.0.1:8444/sub
HTTP/1.1 301 Moved Permanently
content-type: text/html; charset=utf-8
location: /sub/
cache-control: no-store
content-length: 87
```

```
$ for h in swift-heron-3 never-used-42 admin a.b localhost; do ... done
swift-heron-3 → 410      # expires_at 已过
never-used-42 → 404      # 没有这个 slug
admin.localhost:8444 → 404   # 保留名
a.b.localhost:8444 → 404     # 二级标签
localhost:8444 → 200         # 根域介绍页
```

（实际执行的是四条分开的 `curl -so/dev/null -w '%{http_code}' -H 'Host: …'`，上面并成一张表。）

## 8. 「开始」与第一层数据

```
$ curl -sD- -o/dev/null -X POST -H 'Host: brisk-otter-41.localhost:8444' -d 'to=/sub/' \
    http://127.0.0.1:8444/_playtest/start
HTTP/1.1 303 See Other
location: /sub/
cache-control: no-store
set-cookie: pt_gate=1; Path=/; SameSite=Lax; HttpOnly; Max-Age=86400
set-cookie: pt_sid=5d3e9dee68eb77b06d5cfc9ef749da9e; Path=/; SameSite=Lax; HttpOnly; Max-Age=7776000
content-length: 0
```

跑完上面几条之后的 `/tmp/pt-edge-spike/edge-events.jsonl`（原样，一行一条）：

```
{"ts":"2026-09-07T04:10:13.184608Z","type":"gate_view","slug":"brisk-otter-41","version":7,"sid":"","ua":"curl/8.7.1","referer":"","wechat":false}
{"ts":"2026-09-07T04:10:13.191702Z","type":"html_view","slug":"brisk-otter-41","version":7,"sid":"","ua":"curl/8.7.1","referer":"","wechat":false}
{"ts":"2026-09-07T04:11:30.388783Z","type":"start","slug":"brisk-otter-41","version":7,"sid":"5d3e9dee68eb77b06d5cfc9ef749da9e","ua":"curl/8.7.1","referer":"","wechat":false}
{"ts":"2026-09-07T04:11:36.710041Z","type":"report","slug":"brisk-otter-41","version":7,"sid":"","ua":"curl/8.7.1","referer":"","wechat":false,"reason":"phishing","detail":"这页假冒银行"}
{"ts":"2026-09-07T04:11:36.716716Z","type":"gate_view","slug":"brisk-otter-41","version":7,"sid":"","ua":"Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) MicroMessenger/8.0.49 NetType/WIFI","referer":"","wechat":true}
```

最后一条是带微信 UA 打的，`wechat` 判成了 `true`，那次的门禁页里有「在浏览器中打开」那段提示。
四种事件都有，没有 IP 字段。

写这一条时发现的一个真 bug：`tokio::fs::File` 自带一层缓冲，只 `write_all` 不 `flush`，字节可能还没交给内核
就随 `File` 一起 drop 了，集成测试因此偶发读到空文件。加上 `flush()` 之后连跑三轮全绿，换成修好的二进制
（听 8445）重打一遍，事件照常落盘：

```
{"ts":"2026-09-07T04:17:04.473288Z","type":"gate_view","slug":"brisk-otter-41","version":7,"sid":"","ua":"curl/8.7.1","referer":"","wechat":false}
{"ts":"2026-09-07T04:17:04.48043Z","type":"start","slug":"brisk-otter-41","version":7,"sid":"9c66e20862b72ffb751a523a1504e07b","ua":"curl/8.7.1","referer":"","wechat":false}
```

## 没验的

- **浏览器里没跑过。** `fixtures/headers-lab/export/index.html` 是一页自检（wasm 流式编译、
  `Build/hello.wasm.br` 解压后编译、Range、`crossOriginIsolated`、手势解锁 WebAudio），上面全部是
  curl 看响应头，**没有在 Chrome 里点开过这一页**，所以「Godot / Unity 导出物零配置能玩」这句话现在还不能说。
- **微信真机没试过。** 只用改过的 `User-Agent` 试了分支。
- **手机、二维码、局域网**都没试；`playtest.run` 域名与 TLS 不在这一步。
- **实时压缩没做**，只出目录里已有的 `.br` / `.gz`。清单里只有 `.br` 而客户端不接受 br 时是 404，不是
  发一份没法解压的字节。
- **隧道没做**（第二周）。
- **事件没送控制面**（第三周），只落本地 JSONL。
- **`og:image` 没有**，分享卡片现在只有标题和一句描述。
