# 邀请卡、门禁页、广场：边缘这一轮在本机跑通（2026-09-09）

**结论**：DESIGN §3.3 / §3.4 / §3.9 / §3.10 的边缘部分在本机成立。门禁页收成了一张卡，
`/_playtest/card.png` 真的出图（1080×1350 PNG，release 下热渲染 53 毫秒），分享页、
作品域上的关注登记、根域的「我的」与广场三段都在真浏览器里看过了。
**控制面这一轮没起**——另一位正在改 `api/`，编不过——所以 `live.json`、`capabilities.json`、
`plaza.json` 是照 `common/` 的契约手写的 JSON，控制面那一端一个字节都没联调过。

机器：macOS darwin 25.6（Apple Silicon），rustc 1.96.0，Node v22.22.0，
Chrome 152.0.7977.83（`--headless=new`，用仓库里的 `scripts/headless-check.mjs` 走 CDP
设视口——直接 `--window-size=420` 在 macOS 上拿不到 420 的视口，窗口有最小宽度，
页面会按 ~500 排版，第一次截出来的图是错的）。

边缘是 **release 构建**，`127.0.0.1:8453`，`PLAYTEST_HOST_SUFFIX=localhost`，
数据目录 `.data/edge-agent/`。两个夹具作品：

| slug | 长什么样 |
|---|---|
| `brisk-otter-41`「小球大冒险」 | 有封面；`seats` 12 / `joined` 6；群链接；两条公开反馈（一条署名一条匿名）；GitHub 头像；`listed` + `seeking` |
| `wise-mink-28`「写给自己的记账小工具」 | 没封面；匿名链接（有 `expires_at`）；不在广场上；不是游戏 |

## 一、门禁页

截图 `img/2026-09-09-gate.png`（420×1180）。8523 字节，除封面外一个外部请求都没有。

从上到下八条都在：封面出血到圆角、头像 + 「周老板 邀请你试玩」、《小球大冒险》、
一句话介绍、`v7 · 9 月 9 日 · 「这版把跳跃手感调轻了一点」`（等宽小字）、
「周老板在找 12 位试玩者 · 已有 6 位加入」、留名输入（`maxlength="24"`）、一个「开始」、
下面三行弱化的（有新版本时告诉我 / 开发者的群 / 分享）、两条「试玩者的话」。

```
$ curl -H 'Accept: text/html' -H 'Sec-Fetch-Mode: navigate' '…/?from=card'
name="from" value="card"          ← 扫卡进来的人，start 事件带 card
name="from" value="gate"          ← 关注表单自己的来源字段
rel="noopener nofollow">开发者的群
href="/_playtest/share">分享
maxlength="24"
referrerpolicy="no-referrer"      ← 头像
在找 12 位试玩者 · 已有 6 位加入
```

没封面的作品，`og:image` 换成横版卡的绝对地址：

```
<meta property="og:image" content="http://wise-mink-28.localhost:8453/_playtest/card-wide.png">
<meta property="og:image:type" content="image/png">
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="630">
```

## 二、邀请卡

| 路径 | 尺寸 | 字节 | 头 |
|---|---|---|---|
| `/_playtest/card.png`（有封面） | 1080×1350 | 91330 | `image/png`、`cache-control: public, max-age=300`、`etag: "b3e79a68e09d99236083"` |
| `/_playtest/card.png`（没封面） | 1080×1350 | 100897 | 同上 |
| `/_playtest/card-wide.png` | 1200×630 | 57402 | 同上 |

截图：`img/2026-09-09-card.png`、`img/2026-09-09-card-anon.png`、`img/2026-09-09-card-wide.png`
（就是接口吐出来的 PNG 原件，不是屏幕截图）。

- ETag 跟着内容走：把 `live.json` 的 `seats` 从 10 改成 12，同一路径的 ETag
  从 `"a887693f493ccc8dd29e"` 变成 `"b3e79a68e09d99236083"`；带 `If-None-Match` 再请求回 304；
  `HEAD` 回 200 且 0 字节。
- 没封面那张走字卡：一块按 slug 定色相的底 + 作品名头一个字当记号（**不是**把作品名再排一遍——
  作品名在下面已经是最大的字），底下「小林 邀请你体验《写给自己的记账小工具》」，
  票根上是「这张邀请到 9 月 10 日 20:59」（`expires_at` 按 UTC+8 显示）。
- 卡上没有人数、没有「限时 / 领取 / 立即」、没有品类标签；`manifest.badge` 为假时角落那行
  `localhost`（线上是 `playtest.run`）不出现，但票根上的链接还在。

### 渲染耗时（release，这台机器）

| | 耗时 |
|---|---|
| 进程起来后的第一张（含一次性 `load_system_fonts`） | 157 ms |
| 竖版 1080×1350，有封面 | **53 ms** |
| 竖版 1080×1350，没封面 | 29 ms |
| 横版 1200×630，有封面 | 37 ms |
| 命中内存缓存 | 0.4 ms |

达到了「单次 < 100 ms」。同一台机器上 debug 构建是 0.6–2.1 秒，慢 20–40 倍；
本机开发时看到的是那个数，不是线上的数。

字体：进程里 `fontdb` 只加载一次（`OnceLock`，第一次渲染时才扫，不在启动路径上阻塞）。
本机用的是系统里的 PingFang SC；容器里靠 `Dockerfile` 新加的 `fonts-noto-cjk`，
也可以用 `PLAYTEST_FONT_DIRS`（冒号分隔）另指目录。**容器里没试过**，见「没验的」。
一个中文字体都没有时不崩：照样出图，中文是方块，启动日志里 `warn` 一次。

## 三、分享页

`/_playtest/share`，截图 `img/2026-09-09-share.png`。竖版卡 + 「把这张卡发出去」+
只读链接框 + 「复制链接」+ 「分享」（`navigator.share` 有才显示）+ 「保存图片」（`<a download>`）。
`wise-mink-28`（`listed` 为假）上这个路径回 404，不是「你没有权限」。

## 四、关注登记与「我的」

控制面没起，所以这一节验的是**没有控制面时的失败形态**（有控制面的三种回答由
`edge/tests/social.rs` 里的假控制面覆盖：`ConfirmSent` / `Subscribed` / `AlreadyFollowing`）。

```
POST /_playtest/follow (作品域)   → 503，页面上一句「现在登记不了，稍后再试。」
GET  /me                          → 200，只读本机版（截图 img/2026-09-09-me.png）
GET  /me/confirm/nope             → 200，「这条链接现在不管用了。」+ 再发一条
GET  /me/unsubscribe/nope         → 503，「现在退订不了，稍后再试。」
GET  /_playtest/sw.js             → 200 application/javascript，572 字节
```

`/me` 上没有「登录 / 注册 / 账号」这些词：一句「这台设备上想玩的作品记在浏览器里；
留个邮箱，换一台设备也能找到它们」，加一个邮箱输入和一个「用浏览器通知」。

## 五、广场

- 稠密态 `img/2026-09-09-plaza.png`（1280×1620，5 个作品）：三段依次是**推广**（2 张，
  左上角冷灰蓝的「推广」标）、**正在找人测**（1 张，琥珀色标）、**最近更新**（2 张）；
  卡上有头像、「N 人玩过」、「N 人关注」、「13 / 20 位」、想玩 / 关注 / 举报；
  页头「128 人关注着这里」；页脚四句如实说明。
- 稀疏态 `img/2026-09-09-plaza-sparse.png`（1 个作品）：介绍段在卡前面，卡居中，没有推广段。
- 根域 CSP（实测响应头）：

```
default-src 'none'; img-src http://*.localhost:8453 https://avatars.githubusercontent.com;
style-src 'unsafe-inline'; script-src 'nonce-…'; connect-src 'self'; worker-src 'self';
base-uri 'none'; form-action 'self'; frame-ancestors 'none'
```

`avatars.githubusercontent.com` 是这一轮唯一新增的外部来源，只为开发者头像。

## 六、三条硬线还在

- 作品域上一个 `Content-Security-Policy` 都没有（`no_csp_header_anywhere` 守着，本机 curl 复核为 0 条）。
- 只拦顶层导航：不带 `Accept: text/html` / `Sec-Fetch-Mode: navigate` 的同一个 `/` 请求
  直接拿到作品自己的 `index.html`，不是门禁页。
- 子资源永远不会拿到 `200 + HTML`（`gate_hardlines.rs` 六条继续通过；缺文件是 404，不是门禁页）。

`cargo test -p playtest-edge --release`：222 条全过（lib 155 · breaker 7 · gate_hardlines 6 ·
serving 30 · social 16 · tunnel 8）。clippy `--all-targets -D warnings` 干净，fmt 干净。

## 七、真机上才看见的三个问题（都已修）

1. **没封面的卡把作品名排了两遍**——上半张字卡一遍，下面标题一遍。改成上半张只放头一个字当记号。
2. **稀疏态的居中规则误伤了稠密页**：`.grid[data-count="1"]` 是按「一段里有几张卡」给的，
   于是稠密页里只有一张卡的那一段自己跑到屏幕中间去了。改成只在整页稀疏（`< 3`）时才收窄居中，
   靠 `<main class="bands sparse">` 这一层限定；筛选之后只剩一两张时脚本再把这个类加回来。
3. **分享页把卡拉长了 2.5 倍**：`<img width="1080" height="1350">` 上的 `height` 是有效的样式声明，
   光有 `aspect-ratio` 压不住它，得显式写 `height:auto`。这个只有在浏览器里看才会发现。

## 没验的

- **控制面一端全没联调**：`/v1/follow`、`/v1/me/*` 只对着 `edge/tests/social.rs` 里的假控制面跑过；
  真的确认信、真的 `me_token`，以及 `live.json` / `capabilities.json` / `plaza.json` 由谁在什么时候写，
  都还没有一次真实的往返。
- **浏览器通知没有真订阅过**：`/_playtest/sw.js` 只验了它能被取到；`pushManager.subscribe`
  要 HTTPS 和真的 VAPID 公钥，本机 http 上做不了。推送负载的字段名（`title`/`body`/`url`/`tag`）
  是边缘自己定的，契约里还没有（代码里标了 `TODO(contract)`）。
- **真手机、真微信都没开过**：420 宽的无头 Chrome 不能替代微信内置浏览器；卡的长按存图、
  `navigator.share` 带图片文件这两条尤其只在推理层面成立。
- **容器里的字体没试过**：`Dockerfile` 加了 `fonts-noto-cjk`，但这一轮没有构建镜像。
  本机用的是 macOS 自带的 PingFang SC，Linux 上出来的字形一定和截图不一样。
- **二维码没有被真手机扫过**（无头 Chrome 不会扫码），只核对了它编的内容是 `…/?from=card`。
- 这台机器磁盘一直是满的（几个 agent 并行构建），中途清过三次；要复核哪个数字，
  重跑之前先看一眼 `df`。
