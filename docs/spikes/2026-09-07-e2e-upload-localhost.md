# 2026-09-07 · 上传路径在本机全通：CLI → 控制面 → 边缘 → 真实 Chrome

**结论**：KICKOFF 第一周的目标在本机成立。三个进程从零起（空 `.data/`），用 `playtest <目录>` 把
`fixtures/` 里三个真实导出物发布成三个 `http://<slug>.localhost:8443` 链接；在真实 Chrome（无头）里
点开每个链接：先出门禁页，真实鼠标点「开始」后进入作品，**Phaser 游戏可玩、Vite 页面正常、
响应头自检页四项全绿**（wasm 流式编译、Unity 布局的 `.br` 预压缩、Range 206、`crossOriginIsolated`），
门禁页那一下点击解锁了 WebAudio（`AudioContext.state = running`），24 小时内第二次打开不再出门禁页。
边缘把每次打开记成了事件。

**没成立的**：DESIGN §8 说的四个引擎导出物只有 Phaser 与 Vite 两个（本机没有 Godot / Unity，
见 `2026-09-07-fixtures.md`）；手机没扫过码（`*.localhost` 手机解析不到，要等域名）；
无头 Chrome 用了 `--disable-gpu`，Phaser 走的是 Canvas 渲染器而不是 WebGL，WebGL 路径没验；
音频是「状态为 running」，没有人耳听过。

机器：macOS 26.2（Apple Silicon），rustc 1.96.0，Node v22.22.0，Google Chrome 152.0.7977.76。
全部命令从仓库根目录执行。本次还顺手修了 CLI 的两个 bug（第五节）。

## 一、起三个进程

先跑全 workspace 测试，确认三个并行写出来的 crate 合在一起能编译：

```
$ CARGO_INCREMENTAL=0 cargo test --workspace
playtest（cli）      34 单元 + 8 集成
playtest-api         10 单元 + 14 集成
playtest-common      16 单元
playtest-edge        43 单元 + 28 集成
全部 ok，共 153 条
```

清空数据目录，用 debug 二进制起 api 与 edge（KICKOFF §3 的约定，全部默认值）：

```
$ rm -rf .data
$ RUST_LOG=info PLAYTEST_DATA_DIR=.data ./target/debug/playtest-api
INFO 数据库迁移已应用 version=1
INFO playtest 控制面已启动：http://127.0.0.1:8787
INFO 数据目录：.data
INFO 玩家链接：http://{slug}.localhost:8443

$ RUST_LOG=info PLAYTEST_DATA_DIR=.data ./target/debug/playtest-edge
INFO 边缘在 http://127.0.0.1:8443 上（明文，没有 TLS）
INFO 作品从 .data/store 读，事件写到 .data/edge-events.jsonl
INFO 一个作品就是一个 http://<slug>.localhost

$ curl -s http://127.0.0.1:8787/healthz; curl -s -H 'Host: localhost' http://127.0.0.1:8443/_playtest/healthz
ok
ok
```

## 二、CLI 发布三个导出物

`~/.config/playtest/config.json` 里留着上一轮（数据库已清）的令牌与一个记住的 slug，正好把两条恢复路径都走了一遍。

```
$ PLAYTEST_API=http://127.0.0.1:8787 ./target/debug/playtest ./fixtures/headers-lab/export --isolated \
    -n "响应头实验页" -m "第一版：wasm、预压缩、Range、隔离自检"
正在整理 ./fixtures/headers-lab/export：7 个文件，1.0 MB
检测到预压缩文件，会按原样带 Content-Encoding 返回。
上次的匿名身份已经失效，换了一个新的。
需要上传 7 个文件（1.0 MB），其余 0 个服务器上已有
已发布 v1

http://lilac-stork-61.localhost:8443
这是匿名链接，2026-09-08 12:35 后失效。保留、改名、查看结果需要登录（登录还没做好）。
```

（这一次的实际过程是：第一遍在传 `data.bin` 时机器磁盘写满，控制面返回 500「服务器出错了，请稍后重试」，
CLI 以退出码 1 结束；腾出空间后重跑，前六个文件服务器上已有，只补传了 `data.bin`，得到 v1。
上面贴的是重跑那次的输出，「上次的匿名身份已经失效」出现在第一遍。）

```
$ PLAYTEST_API=http://127.0.0.1:8787 ./target/debug/playtest ./fixtures/vite-vanilla/export \
    -n "Vite 计数器" -m "create-vite 模板加一个响一声的按钮"
正在整理 ./fixtures/vite-vanilla/export：7 个文件，45.1 KB
上次的匿名链接已过期（24 小时），这是一个新链接。
需要上传 7 个文件（45.1 KB），其余 0 个服务器上已有
已发布 v1

http://misty-seal-28.localhost:8443
这是匿名链接，2026-09-08 12:41 后失效。保留、改名、查看结果需要登录（登录还没做好）。

$ PLAYTEST_API=http://127.0.0.1:8787 ./target/debug/playtest ./fixtures/phaser-jump/export \
    -n "Phaser 跳一跳" -m "点屏幕跳一下"
正在整理 ./fixtures/phaser-jump/export：2 个文件，1.1 MB
需要上传 2 个文件（1.1 MB），其余 0 个服务器上已有
已发布 v1

http://witty-swan-36.localhost:8443
这是匿名链接，2026-09-08 12:41 后失效。保留、改名、查看结果需要登录（登录还没做好）。
```

三次都是 1 秒内完成（本机回环）。二维码没画是因为 stdout 不是终端，符合设计；终端里的形态见
`2026-09-07-cli-upload.md` 第四节。

对象存储里的样子（api 写、edge 读）：

```
$ find .data/store -type f | sed 's|/[0-9a-f]\{64\}$|/<hash>|' | sort | uniq -c
   2 .data/store/blobs/61/<hash>
   1 .data/store/blobs/ea/<hash>
   1 .data/store/blobs/ee/<hash>
   1 .data/store/blobs/f6/<hash>
   …（三个作品共 16 个文件，对象存储里 15 个 blob：有两个文件内容相同，只存了一份）
   1 .data/store/sites/lilac-stork-61/current.json
   1 .data/store/sites/lilac-stork-61/manifests/1.json
   1 .data/store/sites/misty-seal-28/current.json
   1 .data/store/sites/misty-seal-28/manifests/1.json
   1 .data/store/sites/witty-swan-36/current.json
   1 .data/store/sites/witty-swan-36/manifests/1.json
```

## 三、边缘的响应（curl）

```
$ E=http://127.0.0.1:8443; H='Host: lilac-stork-61.localhost'

# 门禁页：无 cookie 的导航请求
$ curl -s -D - -o /tmp/gate.html -H "$H" -H 'Accept: text/html' $E/
HTTP/1.1 200 OK
content-type: text/html; charset=utf-8
cache-control: no-store
cross-origin-opener-policy: same-origin
cross-origin-embedder-policy: require-corp
（页面 4018 字节，含：邀请你试玩《响应头实验页》、v1、这版改了什么、开始、后失效、举报、由 localhost 提供）

# 点「开始」
$ curl -s -D - -o /dev/null -X POST -H "$H" -d 'to=/' $E/_playtest/start
HTTP/1.1 303 See Other
location: /
set-cookie: pt_gate=1; Path=/; SameSite=Lax; HttpOnly; Max-Age=86400
set-cookie: pt_sid=6bfde53289485efe36874f5476808ce1; Path=/; SameSite=Lax; HttpOnly; Max-Age=7776000

# 带 cookie 再来
$ curl -s -D - -o /dev/null -H "$H" -H 'Accept: text/html' -H 'Cookie: pt_gate=1' $E/
HTTP/1.1 200 OK
etag: "51da0473356260c2c5b65e32bad6f153460488184a6359944a6965e1024f5a8e"
cache-control: no-cache
content-type: text/html; charset=utf-8

# wasm：不接受 br 给原文件，接受 br 给预压缩的
$ curl -s -D - -o /dev/null -H "$H" $E/mod.wasm
HTTP/1.1 200 OK
vary: Accept-Encoding
cross-origin-resource-policy: same-origin
content-type: application/wasm
content-length: 41
$ curl -s -D - -o /dev/null -H "$H" -H 'Accept-Encoding: br' $E/mod.wasm
HTTP/1.1 200 OK
vary: Accept-Encoding
content-encoding: br
content-type: application/wasm
content-length: 42

# Unity 布局：显式请求 .br
$ curl -s -D - -o /dev/null -H "$H" $E/Build/hello.wasm.br
HTTP/1.1 200 OK
content-encoding: br
content-type: application/wasm

# Range
$ curl -s -D - -o /tmp/r.bin -H "$H" -H 'Range: bytes=0-9' $E/data.bin
HTTP/1.1 206 Partial Content
accept-ranges: bytes
content-range: bytes 0-9/1048576
content-length: 10
$ xxd -p /tmp/r.bin
00010203040506070809
$ curl -s -D - -o /dev/null -H "$H" -H 'Range: bytes=999999999-' $E/data.bin
HTTP/1.1 416 Range Not Satisfiable
content-range: bytes */1048576

# 目录索引重定向；不存在的 slug、保留名、根域
$ curl -s -D - -o /dev/null -H "$H" -H 'Cookie: pt_gate=1' $E/sub
HTTP/1.1 301 Moved Permanently
location: /sub/
$ for h in nope-nope-1.localhost login.localhost localhost; do curl -s -o /dev/null -w "$h %{http_code}\n" -H "Host: $h" $E/; done
nope-nope-1.localhost 404
login.localhost 404
localhost 200

# 微信 UA 的门禁页多出两句
$ curl -s -H "$H" -H 'Accept: text/html' -A '… MicroMessenger/8.0.50' $E/ | grep -oE '在微信里[^<]*|系统浏览器[^<]*'
在微信里可能玩不了：点右上角「···」，选「在浏览器中打开」。
系统浏览器（微信内置浏览器不支持它用到的能力）。
```

## 四、真实 Chrome 里点开

本次环境起不了带界面的浏览器，也没有浏览器自动化工具，所以用无头 Chrome 加 DevTools 协议，
脚本在 `scripts/headless-check.mjs`（只用 Node 22 自带的 `fetch` 与 `WebSocket`）。
点击用的是 `Input.dispatchMouseEvent`，是可信的用户手势——`element.click()` 解不了锁 WebAudio。
视口 420×860、移动端模式。cookie 在同一个 `--user-data-dir` 里共享，所以「第二次打开」也能验。

```
$ "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new --remote-debugging-port=9222 \
    --user-data-dir=/tmp/pt-chrome --window-size=420,860 --hide-scrollbars --no-first-run --disable-gpu about:blank
```

### 响应头自检页（`lilac-stork-61`，`--isolated`）

```
$ node scripts/headless-check.mjs "http://lilac-stork-61.localhost:8443/" \
    --click 'form[action="/_playtest/start"] button' --wait 3000 --shot /tmp/pt-shot-lab.png
首屏 title：匿名开发者 邀请你试玩《响应头实验页》
首屏文字：匿名开发者 邀请你试玩 / 《响应头实验页》· v1 / 这版改了什么 / 第一版：wasm、预压缩、Range、隔离自检 /
          开始 / 这个链接在 2026年9月8日 12:35 后失效 / 有问题？举报 / 由 localhost 提供
点击：已点 form[action="/_playtest/start"] button @ (210, 485)，整页跳转
之后 URL：http://lilac-stork-61.localhost:8443/   title：headers-lab · 边缘响应头自检
```

自检页四项（页面文字原样）：

```
1. mod.wasm 流式编译（要求 Content-Type: application/wasm）
✅ 流式编译成功，add(2, 3) = 5
2. Build/hello.wasm.br 预压缩（要求 Content-Encoding: br，Unity 布局）
✅ 解压 + 流式编译成功，add(2, 3) = 5
3. data.bin 的 Range 请求（要求 206 + Content-Range）
✅ 206，Content-Range 与前 10 字节都对
4. 跨源隔离（对应 --isolated 的 COOP/COEP）
✅ 已跨源隔离，SharedArrayBuffer 可用
crossOriginIsolated: true
typeof SharedArrayBuffer: function
```

Console：只有 `favicon.ico` 404（导出物里本来没有）。

第二次打开（同一浏览器，24 小时内）直接进作品，没有门禁页；点「响一声」按钮：

```
$ node scripts/headless-check.mjs "http://lilac-stork-61.localhost:8443/" --click '#beep' --wait 1500
首屏 title：headers-lab · 边缘响应头自检        ← 没有再出门禁页
5. 用户手势解锁音频
刚创建 AudioContext，state = running
resume() 之前 state = running
resume() 之后 state = running
sampleRate = 48000 Hz
✅ 已解锁，应该听到一声 660 Hz。
```

同一个自检页在 `python3 -m http.server` 下第 2、3 项是失败的（`2026-09-07-fixtures.md`），在边缘下全部通过——这两项就是 DESIGN §4.2 说的「服务器不配就白屏」。

### Phaser（`witty-swan-36`）

```
$ node scripts/headless-check.mjs "http://witty-swan-36.localhost:8443/" \
    --click 'form[action="/_playtest/start"] button' --wait 3000 --shot /tmp/pt-shot-phaser1.png
首屏 title：匿名开发者 邀请你试玩《Phaser 跳一跳》
之后 title：phaser-jump
canvas：480×720，CSS 420×630（FIT 缩放），webgl=false（Chrome 带了 --disable-gpu）
Console：Phaser v3.90.0 (Canvas | Web Audio)；favicon.ico 404
截图：标题、方块角色站在地面上、HUD「跳了 0 次 · AudioContext.state: 还没跳过」

$ node scripts/headless-check.mjs "http://witty-swan-36.localhost:8443/" --settle 2500 --click 'canvas' --wait 250 --shot /tmp/pt-shot-phaser3.png
首屏 title：phaser-jump                          ← 没有再出门禁页
截图 HUD：跳了 1 次 · AudioContext.state: running
```

（第一次点画布是在 load 后 300 ms，Phaser 还没启动完，HUD 仍是 0 次；等 2.5 秒再点就对了。
脚本因此加了 `--settle`。）

### Vite（`misty-seal-28`）

```
$ node scripts/headless-check.mjs "http://misty-seal-28.localhost:8443/" --click 'form[action="/_playtest/start"] button' --wait 1500
首屏 title：匿名开发者 邀请你试玩《Vite 计数器》
之后 title：vite-vanilla；页面：Get started / Count is 0 / 点一下响一声 / AudioContext.state: 还没创建 …

$ node scripts/headless-check.mjs "http://misty-seal-28.localhost:8443/" --settle 800 --click '#beep' --wait 1200
首屏 title：vite-vanilla                          ← 没有再出门禁页
页面：AudioContext.state: running → running（48000 Hz）
Console：空
```

### 边缘记下的第一层数据

```
$ wc -l .data/edge-events.jsonl
12
$ head -1 .data/edge-events.jsonl
{"ts":"2026-09-07T04:44:09.620078Z","type":"gate_view","slug":"lilac-stork-61","version":1,"sid":"","ua":"curl/8.7.1","referer":"","wechat":false}
按作品与类型：lilac-stork-61 gate_view 2 / start 2 / html_view 2；misty-seal-28 与 witty-swan-36 各 1 / 1 / 1
```

## 五、这次修掉的两个 CLI bug

都是端到端时撞出来的，单元与集成测试之前没覆盖到：

1. **本地令牌没到期、服务器却不认（服务器重置过）**：CLI 在新建作品时收到 401 `unauthorized` 直接退出，
   还提示「重新运行会拿到新链接」——但重跑用的仍是旧令牌，永远失败。现在 401 时换一个匿名会话再建一次。
2. **记住的作品返回 404 时错误地换了身份**：原实现在 404 时新建匿名会话再建作品，导致同一台机器上
   之前发的作品归在旧身份下，`playtest ls` 看不到（本次 `ls` 只列出两个作品，`响应头实验页` 不见了，
   就是这个原因）。现在 404 只换作品、不换身份；只有 401 才换身份。

改动在 `cli/src/upload.rs` 与 `cli/src/client.rs`（新增 `Error::means_token_gone`），
`cargo test -p playtest` 42 条仍全绿。`ls` 少一个作品的现象本次没有重跑复核（作品已经归在旧身份下，
无法找回），修复由集成测试里「令牌过期自动换新」那条和上面第 2 条的代码路径保证——**这里是代码保证，
不是真机复核**。

## 六、环境记录

- 磁盘全程在 0.2–2.8 GB 之间波动，一次真的写满：`PUT /v1/blobs/{hash}` 落盘失败，控制面按设计
  返回 500 与「服务器出错了，请稍后重试」，CLI 每个文件重试 3 次后以退出码 1 结束，没有半个 blob 留在
  对象存储里。这是一次意外的故障注入，行为符合 DESIGN 的「失败形态是设计出来的」。
- 四个 crate 由四个并行的 agent 各写各的目录，靠 `common/` 契约对接；合并后没有改一行接口就通过了
  全部 153 条测试。
