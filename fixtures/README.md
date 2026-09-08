# fixtures

给 `api/`、`edge/`、`cli/` 用的真实导出物。每个目录里**源码进仓库、`export/` 是构建产物、`node_modules` 构建完就删**。

`export/` 而不是 `dist/`，因为根 `.gitignore` 忽略 `dist/`、`build/`、`node_modules/`。

三个 fixture 的 `export/` 都是**站点根目录**：`index.html` 在根，内部资源用绝对路径（`/assets/...`）引用。`<slug>.playtest.run` 正好是这个形状，边缘不需要改写 base path。

| fixture | `export/` | 文件数 |
|---|---|---|
| `vite-vanilla` | 45.1 KiB | 7 |
| `phaser-jump` | 1173.1 KiB | 2 |
| `headers-lab` | 1033.2 KiB | 7 |

## vite-vanilla

`create-vite` 的 vanilla-ts 模板，只改一处：加一个按钮，点击用 `AudioContext` + `OscillatorNode` 响一声并显示 `AudioContext.state`（`src/audio.ts`）。这是门禁页「开始」那一下能否解锁音频的最小验证手段。

重新生成：`pnpm install && pnpm build`，再把 `dist/` 复制成 `export/`，然后删掉 `node_modules`。

验证边缘的：最普通的一份静态站长什么样——带哈希的文件名（可长缓存、不可变）、`index.html` 不能缓存、`.js` / `.css` / `.png` / `.svg` 的 MIME。

## phaser-jump

Vite + Phaser 3 的最小可玩游戏：方块站在地面上，点屏幕或按空格跳一下，跳时用 WebAudio 合成一个短音。纹理用 `Graphics.generateTexture` 画出来，**没有任何外部图片或音频文件**——`export/` 里只有 `index.html` 和一个 JS。手机上用 `Phaser.Scale.FIT` + `CENTER_BOTH` 等比铺满，`pointerdown` 同时接鼠标和触屏。

重新生成：`pnpm install && pnpm build`，`dist/` → `export/`，删 `node_modules`。

注意 `pnpm add phaser` 现在装的是 **Phaser 4.2.1**；这里按 Phaser 3 的要求锁在 `phaser@^3`（实际 3.90.0），因为社区现存的 jam 作品绝大多数还是 3.x。

验证边缘的：1.2 MB 的单个 JS——真实体量下的压缩、缓存、首屏时间。这是四个引擎导出物里最轻的一个，Godot 和 Unity 会大一个数量级。

## headers-lab

不用构建工具，`node gen.mjs` 一条命令产出整个 `export/`。专门用来检验 DESIGN §4.2 里边缘的响应头逻辑。

- `mod.wasm` — 手写的 41 字节合法 wasm 模块，导出 `add(i32, i32) -> i32`。`gen.mjs` 在写盘前先 `WebAssembly.instantiate` 跑一次，不合法就不产出。
- `mod.wasm.br` / `mod.wasm.gz` — 同一份 wasm 的 brotli 与 gzip。**都比原文件大**（42 / 61 vs 41 字节），41 字节没有可压的东西，这不是 bug。
- `Build/hello.wasm.br` — 同一份 brotli，放在 `Build/` 下模拟 Unity 的目录布局。
- `data.bin` — 1 MiB 确定性字节，第 i 个字节是 `i % 251`，取任意区间都能对答案。
- `index.html` — 无外部依赖的检测页，逐项显示 ✅/❌ 与原因：wasm 的 MIME、`.br` 的 `Content-Encoding`、Range 的 206 与 `Content-Range`、`crossOriginIsolated` 与 `SharedArrayBuffer`、以及一个点了响一声的按钮。
- `sub/index.html` — 测目录索引与 `/sub` → `/sub/` 重定向。

验证边缘的四条：`.wasm` 给 `application/wasm`；`.br` / `.gz` 按 `Accept-Encoding` 直接给并加 `Content-Encoding`；`--isolated` 时加 COOP / COEP；Range 回 206。

根 `.gitignore` 的 `build/` 在 macOS 的大小写不敏感文件系统上会把 `Build/` 也匹配掉，所以根 `.gitignore` 给 `fixtures/**/export/**` 开了例外；`git status` 里能看到 `export/Build/hello.wasm.br` 才算正常。

## ws-rooms

前面三个是**上传路径**的导出物，这一个是**隧道路径**（DESIGN §4.3）的靶子，所以它没有 `export/`——它不是一份静态产物，是一个要一直跑着的进程。

一个房间的联机小游戏：连上就进默认房间，自动分到昵称和颜色，手指或鼠标移动广播归一化坐标，点一下全房间看到一圈涟漪，断开就从名单里移出。服务器每秒广播一次 `tick`，前端收到就打一个带 8 字节 `Buffer` 的 ack 回去，用来量真实往返并顺带过一趟二进制帧。无构建步骤，`socket.io` 是唯一依赖，前端一页 `public/index.html`（内联 CSS/JS，`/socket.io/socket.io.js` 由服务器自己提供，没有外部字体和第三方脚本）。

```
pnpm install
node server.mjs 5174          # 默认就是 5174，只监听 127.0.0.1
node check.mjs http://127.0.0.1:5174
rm -rf node_modules           # 验完就删
```

`check.mjs` 起两个 `socket.io-client` 把「两台手机能对局」拆成 14 条断言（互相看得见、`move` 坐标原样到达、`tap` 广播到全房间、升级到 websocket、往返算得出来、断线自己回来、离开就移出），打印 PASS/FAIL，非零退出表示失败。**同一条命令把地址换成 `https://<slug>.playtest.run` 就是隧道路径的验收**，不用改代码。要让同一 Wi-Fi 下的手机不经隧道直连，用 `HOST=0.0.0.0 node server.mjs`。

页面把状态挂在 `window.__pt = { connected, transport, rtt, players }`，无头 Chrome 直接读；顶部状态条同样的信息给人看：`已连接 · websocket · 往返 38 ms · 2 人在线`，断开变红并显示重连次数。

它替边缘验四条，每一条都是隧道最容易坏的地方：

- **polling → websocket 升级。** 故意保留 socket.io 的默认行为，不写 `transports: ['websocket']`。第一个 HTTP 长轮询请求和随后的 101 走的是同一条路径，边缘只要在升级这一步上少一个头就卡住。`check.mjs` 断言起手是 `polling`、之后 `socket.io.engine.transport.name` 变成 `websocket`。
- **`Host` 被改写之后还能握手。** 边缘默认把 `Host` 换成 `localhost:<port>`、原值放 `X-Forwarded-Host`。服务器把每个请求和每次进房的这两个头都打进日志，隧道那头对不对一眼能看出来。
- **二进制帧与文本帧都要原样过。** socket.io 的事件是文本帧，`Buffer` 附件是独立的二进制帧；ack 里那 8 字节写的是服务器时间戳，客户端解回来对得上才算没被改写过。这一条是对 DESIGN §4.3「响应体一个字节不动」的检查。
- **断线重连。** `check.mjs` 直接 `engine.close()` 掐掉传输，断言客户端自己带着新 id 回来、房间名单跟着更新。隧道重连（DESIGN §4.3 的 1 s → 30 s 退避）和客户端重连是两层，这里只测得到靠玩家侧的那一层。

## 还缺 Godot 与 Unity

DESIGN §8 的 v0.1 完成定义要四个引擎各一个真实导出物。**这台机器上没装 Godot，也没装 Unity，所以这两个没做，也没有伪造。**

需要在装了 Godot 4 / Unity 的机器上导出，放进 `fixtures/godot-*/export/` 和 `fixtures/unity-*/export/`。这两个才是响应头逻辑真正的考题：Godot 4 默认线程导出要 `SharedArrayBuffer`，没有 COOP/COEP 直接打不开；Unity 默认导出就是 `Build/*.wasm.br`，服务器不发 `Content-Encoding: br` 就白屏。`headers-lab` 是这两条规则的替身，不是替代品。
