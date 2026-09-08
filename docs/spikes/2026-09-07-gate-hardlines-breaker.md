# 2026-09-07 · 门禁页三条硬线与每小时熔断（本机真机）

把 `playtest-edge` 起在 18443，用 `.data/gate` 里那批现成清单，逐条看玩家真正收到的状态码、
响应头原文和字节数。下面每一条都是实际执行过的命令和实际看到的输出。

要钉死的是 DESIGN §3.3 那三句：

> 门禁页只在顶层文档导航上渲染；子资源、API、WebSocket upgrade **永不返回「200 + HTML」**，
> 要么给资源本身要么给 403 / 404；**不存在绕过它的请求头**。

加上 DESIGN §4.8 的每 slug 每小时熔断——SIMMER.io 2025-04 是有 billing alert 也没来得及，
所以额度要有第二重，在边缘本地判、不回源。

## 起进程

```
$ PLAYTEST_EDGE_LISTEN=127.0.0.1:18443 PLAYTEST_DATA_DIR=.data/gate \
  PLAYTEST_HOST_SUFFIX=localhost PLAYTEST_PUBLIC_SCHEME=http cargo run -p playtest-edge
2026-09-07T08:40:22.917704Z  INFO 边缘在 http://127.0.0.1:18443 上（明文，没有 TLS）
2026-09-07T08:40:22.917788Z  INFO 作品从 .data/gate/store 读，事件写到 .data/gate/edge-events.jsonl
2026-09-07T08:40:22.917793Z  INFO 一个作品就是一个 http://<slug>.localhost
```

用到的清单都是现成的，这次一个字节都没改：`gate-*` 五个是早先为这组硬线手写进对象存储的
（`manifests/7.json`，下面用到其中四个），`lilac-stork-61` 是更早用真的 CLI 传上去的那一份
（`manifests/1.json`，见 `2026-09-07-e2e-upload-localhost.md`）。

| slug | 用来看什么 | 文件 |
|---|---|---|
| `gate-game-1` | `engine: godot`、`isolated: true`、有 `note` | `index.html`、`assets/app.js`、`game.wasm`、`Build/game.wasm.unityweb` |
| `gate-tool-2` | `engine: vite`（不是游戏） | `index.html`、`assets/app.js` |
| `gate-spa-4` | `spa: true`，**没有 `engine` 字段** | `index.html` |
| `gate-anon-5` | 有 `expires_at`（匿名链接，额度更紧） | `index.html`、`data.bin`（1 MiB） |
| `lilac-stork-61` | 同一个 wasm 的未压缩 / `.br` / `.gz` 三份 | `mod.wasm`、`mod.wasm.br`、`mod.wasm.gz`、`Build/hello.wasm.br` … |

## 1. 硬线 a · 子资源永不「200 + HTML」

```
$ curl -s -H 'Host: gate-game-1.localhost:18443' -H "sec-fetch-dest: $d" -H 'accept: */*' \
    -o /tmp/pt-spike/body.bin -w '%{http_code} / %{content_type} / %{size_download}B' "$E$p"
```

| 路径 | Sec-Fetch-Dest | 结果 | 正文里有门禁页记号 |
|---|---|---|---|
| `/assets/app.js` | `script` | 200 / `text/javascript; charset=utf-8` / 45B | 否 |
| `/game.wasm` | `empty` | 200 / `application/wasm` / 40B | 否 |
| `/Build/game.wasm.unityweb` | `empty` | 200 / `application/octet-stream` / 41B | 否 |
| `/assets/missing.js` | `script` | 404 / `text/html; charset=utf-8` / 3418B | 否 |
| `/Build/absent.wasm` | `empty` | 404 / `text/html; charset=utf-8` / 3418B | 否 |
| `/textures/x.png` | `image` | 404 / `text/html; charset=utf-8` / 3418B | 否 |
| `/assets/missing.js` | `worker` | 404 / `text/html; charset=utf-8` / 3418B | 否 |
| `/assets/missing.js` | `audio` | 404 / `text/html; charset=utf-8` / 3418B | 否 |

拿到的确实是作品自己的字节：

```
$ curl -s -H 'Host: gate-game-1.localhost:18443' -H 'sec-fetch-dest: script' -H 'accept: */*' \
    http://127.0.0.1:18443/assets/app.js
console.log('this is the game, not the gate')
```

**说清楚一件事**：找不到的路径回的是 404 加一页 3418 字节的 HTML。硬线管的是
「200 + HTML」这一种组合——加载器看见 404 会照常报错，看见 200 才会把一页 HTML
当成 JS 去解析。这里没有假装 404 是空的。

### 1.1 SPA 回退不把门禁页发给资源路径

`gate-spa-4` 开了 `spa: true`，任何路径都能解析成 `index.html`——这是最容易漏的一格。

```
$ curl -s -H 'Host: gate-spa-4.localhost:18443' -H 'accept: text/html,*/*' \
    -o /tmp/pt-spike/b2 -w '%{http_code}/%{content_type}' http://127.0.0.1:18443/assets/app-4f2c.js
200/text/html; charset=utf-8
```

| 路径 | 子资源（`Sec-Fetch-Dest: script`） | 老客户端（只有 `Accept: text/html`） | 门禁页记号 |
|---|---|---|---|
| `/assets/app-4f2c.js` | 404 / `text/html` | 200 / `text/html` | 否 |
| `/Build/game.wasm` | 404 / `text/html` | 200 / `text/html` | 否 |
| `/audio/bgm.mp3` | 404 / `text/html` | 200 / `text/html` | 否 |

老客户端那一列确实拿到 200 + HTML，但那是**作品自己的 `index.html`**（SPA 回退本来就该这样），
不是门禁页。同一个作品上真的导航照常出门禁页，上面那道不是把门禁关掉了：

```
$ curl -s -H 'Host: gate-spa-4.localhost:18443' -H 'accept: text/html' \
    -H 'sec-fetch-dest: document' -o /tmp/pt-spike/b3 -w '%{http_code} %{size_download}B' \
    http://127.0.0.1:18443/level/3
200 3997B          # 门禁页记号=1
```

### 1.2 WebSocket upgrade 不是导航

```
$ curl -s -H 'Host: gate-game-1.localhost:18443' -H 'connection: Upgrade' -H 'upgrade: websocket' \
    -H 'sec-fetch-dest: websocket' -H 'sec-websocket-version: 13' -H 'accept: text/html' \
    -o /tmp/pt-spike/b4 -w '%{http_code} %{content_type} %{size_download}B' http://127.0.0.1:18443/socket.io/
404 text/html; charset=utf-8 3418B          # 门禁页记号=0
```

隧道路径接上之后这里会是真的 WS；现在它至少不会拿到一页 HTML 冒充握手结果。

## 2. 硬线 b · `Sec-Fetch-Dest` 在场就以它为准

`fetch('/')` 带的是 `Sec-Fetch-Dest: empty`，有的库还顺手写 `Accept: text/html`。
两个信号打架时信 `Accept`，就等于把门禁页发给一个 XHR。

```
$ curl -s -H 'Host: gate-game-1.localhost:18443' -H 'accept: text/html,application/xhtml+xml' \
    -H "sec-fetch-dest: $d" -o /tmp/pt-spike/b -w '%{http_code}' http://127.0.0.1:18443/
```

17 个 `Sec-Fetch-Dest` 值全部走到作品自己的 `index.html`（`200`，正文以
`<!doctype html><meta charset=utf-8><title...` 开头，门禁页记号 0）：

```
empty script iframe frame image style font worker sharedworker serviceworker
manifest object embed audio video websocket report
```

只有 `document` 是导航，以及不发 `Sec-Fetch-Dest` 的老客户端（Safari 16.4 之前）靠 `Accept` 兜住：

```
document       200  门禁页=1
只有 Accept     200  门禁页=1
```

## 3. 硬线 c · 没有一个请求头能绕过门禁

这一条一半在运行时，一半在**代码的缺席里**。

### 3.1 运行时：30 个头挨个试

```
$ curl -s -H 'Host: gate-game-1.localhost:18443' -H 'accept: text/html' \
    -H 'sec-fetch-dest: document' -H "$h" -o /tmp/pt-spike/b -w '%{http_code}' http://127.0.0.1:18443/
```

30 个 `$h` 全部 `200 门禁页`，一个都没让路：

```
X-Playtest-No-Gate: 1                  X-Playtest-Skip-Gate: true
X-Playtest-Internal: 1                 X-Playtest-Token: whatever
X-Playtest-Gate: off                   X-No-Gate: 1
X-Skip-Gate: 1                         X-Bypass: 1
X-Forwarded-For: 127.0.0.1             X-Real-IP: 1.2.3.4
CF-Connecting-IP: 1.2.3.4              X-Forwarded-Host: gate-game-1.localhost
X-Forwarded-Proto: https               X-Requested-With: XMLHttpRequest
Purpose: prefetch                      Sec-Purpose: prefetch;prerender
X-Moz: prefetch                        Authorization: Bearer 000
Origin: http://gate-game-1.localhost:18443
Referer: http://gate-game-1.localhost:18443/
User-Agent: Mozilla/5.0 (compatible; Googlebot/2.1)
User-Agent: curl/8.7.1                 Sec-Fetch-Mode: cors
Sec-Fetch-Site: same-origin            Sec-Fetch-User: ?1
Cache-Control: no-cache                Cookie: pt_gate2=1
Cookie: xpt_gate=1                     Cookie: pt_gate_=1
Cookie: pt_sid=0123456789abcdef0123456789abcdef
```

唯一走得通的是门禁自己种的 cookie——它就是「这个浏览器点过开始了」的意思：

```
$ curl -s -H 'Host: gate-game-1.localhost:18443' -H 'accept: text/html' \
    -H 'sec-fetch-dest: document' -H 'Cookie: pt_gate=1' http://127.0.0.1:18443/ | head -c 60
<!doctype html><meta charset=utf-8><title>作品自己的页
```

这条 cookie 没有签名，谁都能自己带一个。门禁页不是访问控制（那是 DESIGN §3.6 的口令与
邀请名单，不在 v0.1），它是信任凭证、用户手势和会话起点。

### 3.2 代码里根本不存在这样一个头

试 30 个只能证明这 30 个不行。真正的判据是边缘一共只读九个请求头：

```
$ rg -n 'headers\.get\(|header_str\(|cookie_value\(' edge/src/app.rs edge/src/host.rs
edge/src/app.rs:143:  header_str(headers, "user-agent")
edge/src/app.rs:144:  header_str(headers, "referer")
edge/src/app.rs:145:  cookie_value(headers, SESSION_COOKIE)      # pt_sid
edge/src/app.rs:162:  cookie_value(headers, GATE_COOKIE)         # pt_gate
edge/src/app.rs:164:  header_str(headers, "sec-fetch-dest")
edge/src/app.rs:165:  header_str(headers, "accept")
edge/src/app.rs:167:  header_str(headers, "accept-encoding")
edge/src/app.rs:421:  parts.headers.get("if-none-match")
edge/src/app.rs:425:  header_str(&parts.headers, "range")
edge/src/app.rs:582:  header_str(&parts.headers, "host")
```

```
$ rg -in 'no.gate|skip.gate|bypass|x-playtest|x-internal|admin' edge/src/ common/src/
edge/src/host.rs:110:   assert_eq!(classify("admin.localhost", "localhost"), HostKind::Unknown);
common/src/slug.rs:10:  "www", "api", "app", "admin", "login", …          # 保留 slug 名单
```

两条都是别的意思，源码里没有任何绕过分支。`gate::should_show` 的入参只有
`(gate, path, is_html, navigation, has_cookie)`，其中只有 `sec-fetch-dest`、`accept`、
`pt_gate` 三个来自请求头——**要是需要一个 `X-Playtest-No-Gate`，说明门禁站错了位置**。

## 4. `.unityweb` 不带 `Content-Encoding`

Unity 的两种导出要求相反的服务器配置。Decompression Fallback 的 `.unityweb` 里字节已经是
brotli，但解压是加载器在 JS 里做的；给它加 `Content-Encoding`，浏览器先解一遍、加载器再解一遍，
直接崩。

```
$ curl -s -D - -o /dev/null -H 'Host: gate-game-1.localhost:18443' \
    -H 'accept-encoding: gzip, deflate, br, zstd' -H 'sec-fetch-dest: empty' \
    http://127.0.0.1:18443/Build/game.wasm.unityweb
HTTP/1.1 200 OK
x-content-type-options: nosniff
content-type: application/octet-stream
accept-ranges: bytes
cross-origin-opener-policy: same-origin
cross-origin-embedder-policy: require-corp
cross-origin-resource-policy: same-origin
etag: "a9ebc49f13e07258b86bb742b16594f16e3b7a48de76d15882a6f011cfe25c03"
cache-control: public, max-age=0, must-revalidate
content-length: 41
date: Mon, 07 Sep 2026 08:46:25 GMT
```

没有 `content-encoding`。反过来，真的预压缩产物必须带（`lilac-stork-61`）：

```
$ curl -s -D - -o /dev/null -H 'Host: lilac-stork-61.localhost:18443' \
    -H 'accept-encoding: br, gzip' -H 'sec-fetch-dest: empty' http://127.0.0.1:18443/mod.wasm
HTTP/1.1 200 OK
content-type: application/wasm
content-encoding: br
vary: Accept-Encoding
accept-ranges: bytes
content-length: 42

$ …同上，改 -H 'accept-encoding: identity'
HTTP/1.1 200 OK
content-type: application/wasm
vary: Accept-Encoding
content-length: 41

$ …同上，改 /Build/hello.wasm.br
HTTP/1.1 200 OK
content-type: application/wasm
content-encoding: br
accept-ranges: bytes
content-length: 42
```

三件事一起对上了：`.br` 出去带 `Content-Encoding: br`；`Content-Type` 是**里层**的
`application/wasm`（不是 `.br` 猜出来的），否则没有 `WebAssembly.instantiateStreaming`；
`Vary: Accept-Encoding` 只在目录里同时有压缩和未压缩两份时才发（`/mod.wasm` 有，
`/Build/hello.wasm.br` 没有）。

## 5. 门禁页

### 5.1 标题跟着 `engine` 走

```
$ curl -s -H "Host: $s.localhost:18443" -H 'accept: text/html' -H 'sec-fetch-dest: document' \
    -o /tmp/pt-spike/$s.html -w '%{size_download}' http://127.0.0.1:18443/
```

| slug | `engine` | 字节 | `<title>` |
|---|---|---|---|
| `gate-game-1` | `godot` | 5017 | `某某 邀请你试玩《小球大冒险》` |
| `gate-tool-2` | `vite` | 3972 | `某某 邀请你体验《报销单生成器》` |
| `gate-spa-4` | 字段缺席 | 3983 | `某某 邀请你体验《前端路由的作品》` |
| `gate-anon-5` | 字段缺席 | 4281 | `某某 邀请你体验《匿名链接》` |
| `lilac-stork-61` | 字段缺席 | 5437 | （最胖的一页：`isolated` + `note` + `expires_at`） |

`vite` 落在 `GAME_ENGINES` 名单外，和「没有 `engine`」是同一个结果——`vite` 只说明作品是
打包出来的，用 AI 写小东西的人手上多数是工具、生成器、微型 SaaS，管它们叫「试玩」是把话说错了。
旧清单（`gate-spa-4`、`gate-anon-5` 的 JSON 里真的没有这个字段）按 `None` 解析，不报错。

`lilac-stork-61` 的 5437 字节是这次见到的上限，比进程内测试钉的 8 KiB 还有富余。

### 5.2 OG 元数据

```
$ grep -E '<title>|<meta ' /tmp/pt-spike/gate-game-1.html
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">
<title>某某 邀请你试玩《小球大冒险》</title>
<meta name="description" content="某某 邀请你试玩《小球大冒险》· v7">
<meta property="og:title" content="《小球大冒险》· v7">
<meta property="og:description" content="某某 邀请你试玩">
<meta property="og:type" content="website">
<meta property="og:url" content="http://gate-game-1.localhost:18443/">
```

`engine` 缺席时四处一起换，不会一处「试玩」一处「体验」：

```
$ grep -E '<title>|og:|name="description"' /tmp/pt-spike/gate-spa-4.html
<title>某某 邀请你体验《前端路由的作品》</title>
<meta name="description" content="某某 邀请你体验《前端路由的作品》· v7">
<meta property="og:title" content="《前端路由的作品》· v7">
<meta property="og:description" content="某某 邀请你体验">
<meta property="og:type" content="website">
<meta property="og:url" content="http://gate-spa-4.localhost:18443/">
```

`og:image` 一个都没有（`grep -c og:image` = 0）：现在没有封面，编一张假图比没有图糟——
玩家看到的第一眼应该是这个作品，不是我们的占位符。

四页都没有外链（`grep -c -E '<script src|<link rel="stylesheet|@import|//fonts\.|playtest\.sh'` 全是 0）。
玩家路径上不出现 `playtest.sh`（AGENTS 第 7 条）。

### 5.3 门禁页自己也带隔离头

```
$ curl -s -D - -o /dev/null -H 'Host: gate-game-1.localhost:18443' \
    -H 'accept: text/html' -H 'sec-fetch-dest: document' http://127.0.0.1:18443/
HTTP/1.1 200 OK
content-type: text/html; charset=utf-8
x-content-type-options: nosniff
cache-control: no-store
cross-origin-opener-policy: same-origin
cross-origin-embedder-policy: require-corp
content-length: 5017
```

点了开始之后才隔离等于没隔离，所以门禁页这一跳就得带上。顶层文档不给 CORP，子资源才给
（见第 4 节 `.unityweb` 那份的 `cross-origin-resource-policy: same-origin`）。

### 5.4 能力检测：不看 UA，也不拿走按钮

```
$ grep -o -E 'crossOriginIsolated|SharedArrayBuffer|id="pt-cap" hidden|MicroMessenger|disabled|<button type="submit">开始</button>' \
    /tmp/pt-spike/gate-game-1.html | sort | uniq -c
   1 <button type="submit">开始</button>
   1 crossOriginIsolated
   1 id="pt-cap" hidden
   1 SharedArrayBuffer
```

`MicroMessenger`、`disabled` 都是 0。判据是 `crossOriginIsolated` 与 `SharedArrayBuffer`
两个真实能力，不是 UA 黑名单——微信 Android 的 XWeb 版本与 Chromium 的对应关系没有官方资料，
黑名单一定误伤。那一节默认 `hidden`，检测通过或者关掉 JS 都不显示。
`gate-tool-2`（`isolated: false`）里 `pt-cap` 与 `crossOriginIsolated` 计数都是 0，一个字节都不加。

### 5.5 到期时间

```
$ grep -o '这个链接在.\{0,80\}' /tmp/pt-spike/gate-anon-5.html
这个链接在 <time datetime="2099-01-01T00:00:00Z">1月1日 08:00（UTC+8）</time> 后失效</p>
```

没有 JS 时看到的是 UTC+8 的写法，括号里把时区写出来；有 JS 那一小段把它换成访客本地时区。

## 6. 熔断：触发前后

上限来自 `common/src/limits.rs`：

| 常量 | 值 | 为什么是这个数 |
|---|---|---|
| `SLUG_HOURLY_BYTES` | `3 * GIB` = 3221225472 | DESIGN 没给数字，两头夹出来的：往下，一个 30 MB 的构建一小时能被完整打开约 100 次，一次 jam 或一轮 30 人的测试都够用（DESIGN §8 的 T3 目标是每版打开人数中位数 ≥ 5，差两个数量级）；往上，按 §6 的 $0.1/GB，一个 slug 被刷满一小时的账单封顶在 $0.3 量级。保守值，私测里量到真实利用率再调 |
| `ANON_SLUG_HOURLY_BYTES` | `ANON_TOTAL_BYTES / 2` = 536870912（512 MiB） | 匿名链接 24 小时总共才 1 GiB（DESIGN §6），一小时就烧掉一半已经说明这不是「发给几个朋友」 |

窗口一小时切 12 格，每格 5 分钟（`edge/src/breaker.rs`）。格子越细内存越多，越粗窗口边缘抖得越厉害。

`gate-anon-5` 有 `expires_at`，走的是 512 MiB 那一档。`data.bin` 正好 1 MiB，
所以 512 次完整 GET 就是不多不少一个上限。

```
$ E=http://127.0.0.1:18443; A='Host: gate-anon-5.localhost:18443'
$ curl -s -D - -o /dev/null -H "$A" -H 'sec-fetch-dest: empty' "$E/data.bin"
HTTP/1.1 200 OK
content-length: 1048576

$ urls=(); for i in $(seq 1 510); do urls+=("$E/data.bin"); done
$ time curl -s -H "$A" -H 'sec-fetch-dest: empty' "${urls[@]}" > /dev/null
0.19s user 0.62s system 45% cpu 1.800 total
```

511 MiB 之后**还没跳闸**，第 512 次照常出体、正好把额度填满：

```
$ curl -s -o /dev/null -H "$A" -H 'accept: text/html' -H 'sec-fetch-dest: document' -w '%{http_code}' "$E/"
200
$ curl -s -o /dev/null -H "$A" -H 'sec-fetch-dest: empty' -w '%{http_code} %{size_download}B' "$E/data.bin"
200 1048576B
```

到了上限就是超了，不是超过才算。下一次请求：

```
$ curl -s -D - -H "$A" -H 'accept: text/html' -H 'sec-fetch-dest: document' -o /tmp/pt-spike/over.html "$E/"
HTTP/1.1 429 Too Many Requests
content-type: text/html; charset=utf-8
cache-control: no-store
retry-after: 225
content-length: 3576
```

正文 3576 字节，`<h1>这个作品这一小时的流量用完了</h1>`，里面不出现 `playtest.sh`。
玩家拿到的是别人发给他的一条链接，浏览器默认的错误页只会让他以为是自己的网络坏了。

**导航和子资源分开答**，理由同门禁页的硬线——给一个 `.wasm` 回一页 HTML，加载器会崩在
一个和真实原因无关的地方：

```
$ for p in /data.bin /index.html /nope.png; do curl -s -D … -H 'sec-fetch-dest: script' "$E$p"; done
/data.bin     HTTP/1.1 429 Too Many Requests  正文=0字节  retry-after: 225
/index.html   HTTP/1.1 429 Too Many Requests  正文=0字节  retry-after: 225
/nope.png     HTTP/1.1 429 Too Many Requests  正文=0字节  retry-after: 225
```

`retry-after: 225` 是这一格（5 分钟 = 300 秒）剩下的时间。在最老那一格掉出窗口之前，
总量不可能变小，所以承诺的是「最早可能好转」的秒数，永远 ≥ 1、≤ 300。

用 429 不用 503：503 是「这个服务挂了」，会让探针、CDN 和搜索引擎当成故障，而这里坏的只是
一个 slug 的一小时。429 带 `Retry-After` 正好说清「你等一会儿再来」，也是唯一一个客户端
普遍认得的「稍后再试」。

### 6.1 熔断关的是作品，不是举报入口

```
举报表单 GET   200 4096B   （正文里有 <select name="reason">）
举报提交 POST  200
健康检查       200
```

一个正在被刷的 slug 恰恰是最需要能举报的；健康检查也不能受影响，否则一个被刷的作品会把
整台机器从负载均衡里摘掉。

### 6.2 一个 slug 跳闸不碰别的

```
gate-game-1     200 5017B
gate-tool-2     200 3972B
gate-spa-4      200 3983B
lilac-stork-61  200 5437B
根域介绍页       200
```

### 6.3 被刷的时候日志不许变成放大器

上面撞在跳闸上的请求有四次（一次导航加三个子资源），事件只写了一条：

```
$ grep -c breaker_trip .data/gate/edge-events.jsonl
1
$ grep breaker_trip .data/gate/edge-events.jsonl
{"ts":"2026-09-07T08:51:38.473366Z","type":"breaker_trip","slug":"gate-anon-5","version":7,
 "sid":"","ua":"curl/8.7.1","referer":"","wechat":false,"bytes":536870912,"limit":536870912}
```

`bytes` 与 `limit` 都是 536870912，正好卡在阈值上，和上面数出来的 512 次对得上。
没有 `ip` 字段——熔断这条也一样（DESIGN §3.4）。

### 6.4 不出体的响应长什么样

换回没跳闸的 `gate-game-1`（`G='Host: gate-game-1.localhost:18443'`）：

```
$ curl -s -D - -o /dev/null -H "$G" -H "if-none-match: \"c1f74a…\"" "$E/game.wasm"
HTTP/1.1 304 Not Modified

$ curl -s -I -H "$G" "$E/game.wasm"
HTTP/1.1 200 OK
content-type: application/wasm
content-length: 40

$ curl -s -D - -o /dev/null -H "$G" -H 'range: bytes=0-7' "$E/game.wasm"
HTTP/1.1 206 Partial Content
content-range: bytes 0-7/40
content-length: 8
```

判定和计数是分开的：304、HEAD、416 要过判定但不该记账。**这一条这里只看到了响应形状——
计数器本身没有 HTTP 出口，「不记账」是靠 `edge/tests/breaker.rs::serving_a_file_counts_toward_the_hour`
在进程内断言的，不是这次 curl 出来的。**

## 这次没有验证的

- **重启后计数归零。** 这是设计上接受的代价（熔断是止血不是记账，真正的账在控制面按 60 秒上报
  的用量里），但这次没有真的重启进程再打一遍来看它确实归零。
- **多节点各算各的。** v0.1 只有一个节点，没有第二台可打。
- **真的把 3 GiB 打满。** 只跑了匿名档的 512 MiB；`SLUG_HOURLY_BYTES` 那一档只有
  `edge/tests/breaker.rs` 在进程内填计数器验证过。
- **HTTPS 与真实浏览器。** 全程明文 `http://*.localhost:18443` + curl。`crossOriginIsolated`
  在真浏览器里到底是不是 `true`、微信 X5 里那段能力检测会不会显示，都还没在真机上看过。
- **隧道路径。** `game_headers::Source::Upstream` 那半边只有单元测试，边缘还没接上游。

## 收尾

```
$ cargo test -p playtest-edge -p playtest-common
     Running unittests src/lib.rs (playtest_common)
test result: ok. 28 passed; 0 failed
     Running unittests src/lib.rs (playtest_edge)
test result: ok. 67 passed; 0 failed
     Running tests/breaker.rs
test result: ok. 7 passed; 0 failed
     Running tests/gate_hardlines.rs
test result: ok. 6 passed; 0 failed
     Running tests/serving.rs
test result: ok. 28 passed; 0 failed
```

18443 上那个进程已经 kill 掉。`.data/gate/edge-events.jsonl` 里留着这次跑出来的事件，
`.data` 不进仓库。
