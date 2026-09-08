# 2026-09-07 · 门禁页点「开始」之后，游戏那一侧还有没有用户手势（本机真机）

边缘现在的门禁页是「表单 POST `/_playtest/start` → 303 → 游戏页」。这一跳之后，游戏页还算不算
「用户点过」？如果不算，玩家点了「开始」却听不到声音，门禁页那一下就白点了。

测试台在 `fixtures/gate-audio/`：一个门禁页承载 8 个变体，一份探针在游戏那一侧做同样的四件事。
下面每一格都是实际跑出来的，`document.title` 原文附在后面。

## 环境

| | |
|---|---|
| 机器 | macOS 26.2（25C56），arm64 |
| node | v22.22.0 |
| playwright | 1.63.0（只用它开浏览器和点鼠标，不用它读页面，原因见下文） |
| 浏览器 | 真 Chrome 152.0.7977.76（`channel: 'chrome'`，headed，全新 profile） |
| 起服务 | `node solo.mjs --browsers <b> --variants <v...>`，9301 是门禁页那一侧，9302 只用来做跨源 iframe |
| 跑的日期 | 2026-09-07 |

Chrome 跑三档自动播放策略，因为这一栏最容易想当然：

| 档位 | 参数 |
|---|---|
| `chrome` | 不加参数，带界面的真 Chrome 默认行为。最接近一个真玩家第一次点开陌生链接 |
| `chrome-ugr` | `--autoplay-policy=user-gesture-required` |
| `chrome-strict` | `--autoplay-policy=document-user-activation-required` |

## 方法上的一个坑：不能用 `page.evaluate()` 读结果

`results/results.jsonl` 里前两轮（第 1–16 行）是**废的**，留着是为了说明这个坑。那两轮里 V0
（直接打开游戏页，全程没点过任何东西）居然也出了声：

```
第 1 轮 v0：ac=running→running osc=响(+0.304s) audio=ok act=false/false→true/true
第 2 轮 v0：ac=suspended→suspended osc=响(+0.304s) audio=ok/响 act=false/false→true/true
```

`act` 那一栏从 `false/false` 变成 `true/true` 就是证据：测量做到一半，用户激活凭空出现了。
是跑批脚本自己给的——Playwright 的 `page.evaluate()` / `page.title()` / `waitForFunction()` 走
CDP 的 `Runtime.callFunctionOn(userGesture: true)`，在 Chrome 里等于替你点了一下。

改法是探针把结果 `fetch('/report')` 送回服务器，跑批那一侧只 `goto` 和 `click`，别的什么都不做，
等测量结束了再读 title。第 3 轮起 V0 才正常地全灭。

第 3 轮又踩了另一个小坑：`AudioContext` 第一次用声卡有冷启动，只采样一次会把「刚开始渲染」误判成
「被挡住」（V1-direct 那一格 `+0.005s`，判成哑的）。探针改成最多等 1.5 秒等时钟走起来。

**所以 Chrome 默认档只取最后一轮**，也就是 `results.jsonl` 第 25–32 行；前 24 行不要引用。
`chrome-ugr` / `chrome-strict`（第 33–48 行）是今天补跑的，用的已经是改好的探针，一次到位，
没有废轮次。`results/<browser>.json` 由 `node collect.mjs` 从 jsonl 重建，每个 browser+variant
取最后一条——直接看这三个文件就不会取错轮次。

## 四项分别在测什么

| | 检查 | 判据 |
|---|---|---|
| (a) | `new AudioContext()` 的 state，`resume()` 前后各读一次 | 被挡住的 `resume()` **不 reject，就那么一直挂着**，所以有 2 秒上限 |
| (b) | `OscillatorNode` 真出 0.5 秒声音 | 不看 state 那个字符串，看 `currentTime` 走不走。挂起的上下文时钟是冻住的，比 state 难骗 |
| (c) | `<audio>.play()` | resolve 不等于真在放，200ms 后还要看 `paused` 和 `currentTime` |
| (d) | `navigator.userActivation` | `isActive`（瞬时）/ `hasBeenActive`（粘性）。Chromium 有，WebKit 到今天还没有 |

## 结果表

✓ = 四项都过（有声）；✗ = 挡住了。`act` 是探针开始时的 `isActive/hasBeenActive`。

| 变体 | 怎么进游戏 | chrome（默认） | chrome-ugr | chrome-strict | act（三档一致处） |
|---|---|---|---|---|---|
| V0 | 直接打开游戏页，不点 | ✗ 全灭 | (a)(b) 过、**(c) ✗** | ✗ 全灭 | `false/false` |
| V1-direct | 点击回调里就地跑那四件事 | ✓ | ✓ | ✓ | `true/true` |
| V1-fetch | 点击回调里 `await fetch` + 换 body + 重跑 script | ✓ | ✓ | ✓ | `true/true` |
| V2 | `location.href` 导航到同源游戏页 | ✓ | **✗ (c) NotAllowedError** | ✓ | `false/true` |
| V2b | 表单 POST → 303 → 游戏页（**边缘现在的形态**） | ✓ | **✗ (c) NotAllowedError** | ✓ | `false/true` |
| V3 | 同源 iframe（点击回调里插入） | ✓ | ✓ | ✓ | `true/true` |
| V4 | 跨源 iframe（另一个端口） | ✓ | **✗ 全灭**（连 (a)(b) 都没了） | ✓ | ugr 下 `false/false`，另两档 `true/true` |
| V5 | 导航之后游戏页干等 3 秒再碰音频 | ✓ | **✗ (c) NotAllowedError** | ✓ | `false/true` |

三件事值得单独拎出来：

1. **`act` 那一列才是主线。** 导航（V2 / V2b / V5）之后 `isActive` 一律是 `false`，只剩
   `hasBeenActive=true`。导航把文档换掉了，**瞬时激活不跨文档，粘性激活跨**。留在同一个文档里
   （V1-direct / V1-fetch）或者在点击回调里开 iframe（V3），瞬时激活都还在。
2. **`AudioContext` 只要粘性激活就够，`<audio>.play()` 要瞬时的。** 所以导航之后 (a)(b) 全过、
   (c) 单独挂——V2 / V2b / V5 在 `chrome-ugr` 下就是这个形状。
3. **`chrome-strict` 比 `chrome-ugr` 松。** 名字最吓人的
   `document-user-activation-required` 认的正是粘性激活，导航之后还在，所以 V2/V2b 反而全过；
   `user-gesture-required` 认瞬时的，导航之后就没了。V0 在 ugr 下 `AudioContext` 甚至不用手势就
   直接 `running`——这一档对 `AudioContext` 基本只管跨源 iframe（V4 那一格全灭就是它管的）。
   **别照名字猜策略。**

V5 和 V2 完全同形：`actBefore` 在游戏页一开始就已经是 `false/true` 了，干等 3 秒不会让它更糟。
**慢不是问题，导航才是。**

### 每个变体的 `document.title` 原文

`chrome`（默认策略，最接近真玩家）：

```
[v0] ac=suspended→suspended(resume 一直挂着(>2000ms 没 resolve 也没 reject)) osc=哑(suspended,+0s) audio=NotAllowedError/哑 act=false/false→false/false frame=top ref=(空)
[v1-direct] ac=running→running(resume resolved) osc=响(running,+0.096s) audio=ok/响 act=true/true→true/true frame=top ref=(空)
[v1-fetch] ac=running→running(resume resolved) osc=响(running,+0.069s) audio=ok/响 act=true/true→true/true frame=top ref=(空)
[v2] ac=running→running(resume resolved) osc=响(running,+0.096s) audio=ok/响 act=false/true→false/true frame=top ref=http://localhost:9301/gate.html?v=v2&xo=http%3A%2F%2Flocalhost%3A9302
[v2b] ac=running→running(resume resolved) osc=响(running,+0.101s) audio=ok/响 act=false/true→false/true frame=top ref=http://localhost:9301/gate.html?v=v2b&xo=http%3A%2F%2Flocalhost%3A9302
[v3] ac=running→running(resume resolved) osc=响(running,+0.091s) audio=ok/响 act=true/true→true/true frame=iframe同源 ref=http://localhost:9301/gate.html?v=v3&xo=http%3A%2F%2Flocalhost%3A9302
[v4] ac=running→running(resume resolved) osc=响(running,+0.101s) audio=ok/响 act=true/true→true/true frame=iframe跨源 ref=http://localhost:9301/
[v5] ac=running→running(resume resolved) osc=响(running,+0.08s) audio=ok/响 act=false/true→false/true frame=top ref=http://localhost:9301/gate.html?v=v5&xo=http%3A%2F%2Flocalhost%3A9302&delay=3000
```

`chrome-ugr`（`--autoplay-policy=user-gesture-required`）：

```
[v0] ac=running→running(resume resolved) osc=响(running,+0.064s) audio=NotAllowedError/哑 act=false/false→false/false frame=top ref=(空)
[v1-direct] ac=running→running(resume resolved) osc=响(running,+0.101s) audio=ok/响 act=true/true→true/true frame=top ref=(空)
[v1-fetch] ac=running→running(resume resolved) osc=响(running,+0.053s) audio=ok/响 act=true/true→true/true frame=top ref=(空)
[v2] ac=running→running(resume resolved) osc=响(running,+0.091s) audio=NotAllowedError/哑 act=false/true→false/true frame=top ref=http://localhost:9301/gate.html?v=v2&xo=http%3A%2F%2Flocalhost%3A9302
[v2b] ac=running→running(resume resolved) osc=响(running,+0.101s) audio=NotAllowedError/哑 act=false/true→false/true frame=top ref=http://localhost:9301/gate.html?v=v2b&xo=http%3A%2F%2Flocalhost%3A9302
[v3] ac=running→running(resume resolved) osc=响(running,+0.053s) audio=ok/响 act=true/true→true/true frame=iframe同源 ref=http://localhost:9301/gate.html?v=v3&xo=http%3A%2F%2Flocalhost%3A9302
[v4] ac=suspended→suspended(resume 一直挂着(>2000ms 没 resolve 也没 reject)) osc=哑(suspended,+0s) audio=NotAllowedError/哑 act=false/false→false/false frame=iframe跨源 ref=http://localhost:9301/
[v5] ac=running→running(resume resolved) osc=响(running,+0.096s) audio=NotAllowedError/哑 act=false/true→false/true frame=top ref=http://localhost:9301/gate.html?v=v5&xo=http%3A%2F%2Flocalhost%3A9302&delay=3000
```

`chrome-strict`（`--autoplay-policy=document-user-activation-required`）：

```
[v0] ac=suspended→suspended(resume 一直挂着(>2000ms 没 resolve 也没 reject)) osc=哑(suspended,+0s) audio=NotAllowedError/哑 act=false/false→false/false frame=top ref=(空)
[v1-direct] ac=running→running(resume resolved) osc=响(running,+0.091s) audio=ok/响 act=true/true→true/true frame=top ref=(空)
[v1-fetch] ac=running→running(resume resolved) osc=响(running,+0.096s) audio=ok/响 act=true/true→true/true frame=top ref=(空)
[v2] ac=running→running(resume resolved) osc=响(running,+0.053s) audio=ok/响 act=false/true→false/true frame=top ref=http://localhost:9301/gate.html?v=v2&xo=http%3A%2F%2Flocalhost%3A9302
[v2b] ac=running→running(resume resolved) osc=响(running,+0.053s) audio=ok/响 act=false/true→false/true frame=top ref=http://localhost:9301/gate.html?v=v2b&xo=http%3A%2F%2Flocalhost%3A9302
[v3] ac=running→running(resume resolved) osc=响(running,+0.091s) audio=ok/响 act=true/true→true/true frame=iframe同源 ref=http://localhost:9301/gate.html?v=v3&xo=http%3A%2F%2Flocalhost%3A9302
[v4] ac=running→running(resume resolved) osc=响(running,+0.096s) audio=ok/响 act=true/true→true/true frame=iframe跨源 ref=http://localhost:9301/
[v5] ac=running→running(resume resolved) osc=响(running,+0.096s) audio=ok/响 act=false/true→false/true frame=top ref=http://localhost:9301/gate.html?v=v5&xo=http%3A%2F%2Flocalhost%3A9302&delay=3000
```

## 没跑的：WebKit、Firefox、iOS Safari、微信

**这次只有 Chrome 一个引擎的证据。** 说清楚每一个缺口：

| 想测的 | 状态 | 为什么 |
|---|---|---|
| Playwright WebKit（近似 Safari） | **没跑** | `~/Library/Caches/ms-playwright/` 整个目录不存在了。昨天那次下载没留下东西——磁盘 100% 满（`/System/Volumes/Data` 只剩 791 MB），这个目录在 `~/Library/Caches` 下，被系统当可回收空间清掉了。重下 WebKit 解压后约 400 MB，装不下，也不许再下 |
| Playwright Firefox | **没跑** | 同上，同一个缓存目录。本机也没有系统 Firefox（Playwright 也驱动不了系统 Firefox，它要自己那份改过的构建） |
| 真 Safari（macOS） | **没跑** | `/System/Cryptexes/App/usr/bin/safaridriver` 在，Safari 也在，但 `safaridriver` 要先 `safaridriver --enable`（要管理员认证）才会接受会话。`fixtures/gate-audio/safari-check.sh` 是探这一步的脚本，本机跑不起来 |
| **iOS Safari** | **没跑** | 没有设备也没有模拟器。而且 Playwright 的 WebKit 就算跑起来也**只是近似**，不能替 iOS Safari 说话——iOS 上 `<audio>` 和 `AudioContext` 的门槛历来比桌面 WebKit 还严 |
| **微信内置浏览器** | **没跑** | 安卓上是 X5/XWeb，iOS 上是 WKWebView 再套一层微信自己的策略，两边都跟标准浏览器不一样，只能拿真机测 |
| 附带观察（全屏、横屏锁定） | **没跑** | `run.mjs --bonus` 这条路没执行过。iframe 里的 `requestFullscreen()` 还要父页给 `allow="fullscreen"`，这一条对下面的建议有影响，得单独测 |

WebKit 恰恰是最该测的那个——它没有 `navigator.userActivation`（探针 (d) 那一栏会是「无此 API」），
音频门槛也最严。**下面的结论只在 Chrome 上验证过。**

## 结论

### 门禁页应该用哪种方式进游戏

按「瞬时激活还在不在」排，只有两类能进游戏之后立刻出声：

1. **留在同一个文档里**（V1-direct / V1-fetch）——瞬时激活完整。但要求把游戏塞进门禁页的文档，
   对「玩家上传的任意一份游戏」不现实。
2. **点击回调里插一个同源 iframe**（V3）——瞬时激活完整，游戏还留在自己的文档里，原样加载。
   **三档策略全过，是唯一两头都占的形状。**

**导航（V2 / V2b）是会丢东西的那一类**，`isActive` 必然变成 `false`。

### V2 / V2b 在哪个浏览器上失败

**在真 Chrome 152 + `--autoplay-policy=user-gesture-required` 下失败，失败的是第 (c) 项
`<audio>.play()`，报 `NotAllowedError`，`paused=true`、`currentTime=0`，是真的哑的。**
`AudioContext` 那两项（a）(b) 还是过的，所以**只用 Web Audio 的游戏察觉不到，用 `<audio>` 标签的
游戏一进去就是哑的**——这个差别很容易在自测时漏掉。

默认策略和 `document-user-activation-required` 两档下 V2/V2b 都过。也就是说：
**边缘现在的形态在大多数 Chrome 玩家身上是好的，但它靠的是粘性激活，安全边际只有一层。**
WebKit / iOS Safari 没测，而那正是这层边际最可能不够的地方。

### 跨源 iframe 不要用

V4 在 `chrome-ugr` 下**全灭**——`act=false/false`，连 `AudioContext` 都 `suspended`、`resume()`
一直挂着。跨源 iframe 在那一档下完全不继承激活。**游戏和门禁页必须同源。**

## 对子任务 A / 边缘实现的建议

1. **先不要动 `/_playtest/start` 的 303。** 它在三档 Chrome 里挂的只有 `<audio>.play()` 那一格，
   而且只在 ugr 一档挂。在 WebKit 上跑出结果之前，为了这一格重做门禁与 `pt_gate` cookie 的架构，
   证据不够。
2. **要改就改成「点击回调里插同源 iframe」（V3），别改成别的。** 它是唯一在三档里瞬时激活都还在
   的形状。代价是门禁页要一直留在顶层文档、`pt_gate` cookie 的时机跟着变、全屏要补
   `allow="fullscreen"`（**这一条没测**）。
3. **跨源 iframe 直接排除。** 见上。
4. **把「游戏可能是哑的」当成设计出来的失败形态。** 只要还走 303，游戏页第一次碰音频就有可能被
   挡。边缘能做的是在门禁页那一下**就地把音频解锁**——但 `AudioContext` 活不过导航，所以走 303
   的话这条路是堵的。要么接受「进去可能没声，玩家自己再点一下」，要么换 V3。
   两条路都行，**不能装作这个问题不存在**。
5. **补测再下结论。** 缺的是 WebKit / iOS Safari / 微信。测试台已经在 `fixtures/gate-audio/`，
   磁盘腾出约 1 GB 后 `npx playwright install webkit firefox`，再
   `./once.sh webkit` 和 `./once.sh firefox` 就能把表填满。**在那之前，任何「已支持音频」的说法
   都不能写进文档或 CLI 输出。**

## 复现

```
cd fixtures/gate-audio
pnpm install
./once.sh chrome-ugr                                      # 一个浏览器的 8 个变体，分 4 小批跑完
node solo.mjs --browsers chrome --variants v0,v1-direct   # 只想跑某几个变体
node collect.mjs                                          # 从 jsonl 重建 results/<browser>.json，缺哪个变体会报
```

`solo.mjs` 把服务和跑批放进同一个进程，跑完就退——不用自己管 9301/9302 上那个 node 的生命周期。
分批是因为一次跑满 8 个要一分多钟，长命令在半路被收走的话服务跟着没，剩下的变体会齐刷刷报
「25000ms 内没收到探针结果」，看着像浏览器的问题，其实是跑批自己断了。
`server.mjs` + `run.mjs` 分开起也行，那样服务得自己起自己关。
