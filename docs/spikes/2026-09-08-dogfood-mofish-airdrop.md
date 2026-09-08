# 2026-09-08 · 拿自己的两个作品当第一批用户：mofish 走上传，空投站走隧道

**结论**：两条路都一次通过，但**作为第一个真实用户，我在门禁页上看到了「邀请你体验《v2》」**——
作品名取自目录名，而真正做作品的人目录名多半是 `v2` / `dist` / `web`。这是这次 dogfood 最值钱的发现，
其它三处体验问题列在第四节。用的是 Releases 上的 `v0.1.0` 单文件，不是本机构建。

两个作品都是 `imagination-space/experiments/` 下早先的实验，不是为 playtest 做的靶子：
**mofish v2**（摸一只鱼，53 个文件 2.2 MB 纯静态，Canvas2D，可选后端探测 `api/now`，Service Worker，按需从 unpkg / NASA / 地图瓦片站取外部资源）；
**airdrop**（空投站，静态前端 + Node 后端，`/airdrop/api/*`，Cloudflare Turnstile 可选）。

## 一、mofish · 上传

```
$ cd imagination-space/experiments/mofish/v2
$ playtest . --no-qr
正在整理 .：53 个文件，2.2 MB
需要上传 53 个文件（2.2 MB），其余 0 个服务器上已有
已发布 v1
https://sage-salmon-91.playtest.run
本次 3.4 秒（哈希 0.0 · 上传 2.4 · 提交 0.1）
```

玩家侧（无头 Chrome 390×844，`--disable-gpu`）：门禁页「匿名开发者 邀请你体验《v2》· v1」→ 点开始 →
标题「摸一只鱼」，全屏 Canvas 420×860，「用我此刻的位置 / 先不定位」→ 点「先不定位」→ 「今天来的是这三条——选一条陪你」，
三尾水墨鱼画出来了（截图 `img/2026-09-08-mofish-pick-a-fish.png`）。

Console 只有一条：`GET /api/now → 404`。这是 mofish 自己的「同源探测后端、探不到就单机」逻辑，边缘按清单答 404 是对的，
它随即回单机模式——**没有任何一个我们的行为妨碍它**。资源头都对：`sw.js` `text/javascript`、`manifest.json` `application/json`、
`citylights.json` 411 KB `application/json`、`chart-data.js` 648 KB。Service Worker 在 HTTPS 下可注册（`file://` 下不行），
这正是它需要一个真链接的原因之一。

## 二、空投站 · 隧道

```
$ cd imagination-space/experiments/airdrop && PORT=8894 node server/dev.mjs &
$ playtest 8894 --no-qr -n "空投站"
已连上，这条链接现在能玩了。
https://brisk-owl-60.playtest.run
本次 1.5 秒（从敲命令到链接出来）
```

玩家侧：`/airdrop/` 门禁页「邀请你体验《空投站》」→ 开始 → 标题「空投驿站」，页面完整（OUYZ、投一箱、空舞台线框、
「此刻没有空投，也没有预告」），`/airdrop/api/now` 穿隧道回 `{"ok":true,"now":…,"community":{"drops":0}…}`。
Console 一条 `type.woff2` 404——它的字体文件在仓库另一处（`key/web`），dev 服务器本来就找不到，不是隧道的问题。
Ctrl-C 之后 `/airdrop/` → 503 离线页。

后端日志里 `Host` 是 `localhost:8894`——改写起效，后端不需要任何配置。

## 三、这次真的验到了什么

| DESIGN 的说法 | 这次 |
|---|---|
| §3.2「几秒」 | 上传 3.4 秒（53 文件 2.2 MB，经代理出口）；隧道 1.5 秒 |
| §4.2 `.json` / `.js` / SW 的 MIME | 对；SW 在真 HTTPS 下可注册 |
| §4.3 Host 改写、后端零配置 | 对；`/airdrop/api/now` 原样穿过 |
| §3.3 门禁页 → 用户手势 | mofish 的 Canvas 与音频都在门禁之后正常起来 |
| 一个作品既有静态又有后端 | 两个都能用，但各走一条路——mofish 的可选后端如果开着，上传模式下它探不到（404 后单机），这正是 v0.3「混合模式」要解的 |

## 四、作为用户我撞到的四处（按疼的程度）

1. **《v2》**。目录叫 `v2`，门禁页就写「邀请你体验《v2》」。CLI 应该在目录名像版本号 / 通用名时，先用 `index.html` 的 `<title>`（「摸一只鱼」），
   再不行才用目录名，并且在发布时**把作品名打出来**让人看见——这次我发完才在玩家那边看到。
   （`<title>` 优先的逻辑已在 `2f631e8` 做了，但只覆盖 `dist` / `export` 那几个词，`v2` 这种没覆盖；本 spike 后补。）
2. **「邀请你体验」还是「试玩」**。mofish 是游戏，但 CLI 没认出引擎（自绘 Canvas，没有 Phaser / Godot 签名），门禁页就说「体验」。
   判据是引擎名，而不是作品是不是游戏——自绘游戏永远拿不到「试玩」。可以让 `--name` 旁边有个 `--game` 或者从 `<canvas>` + 输入监听猜，
   但猜错的代价是对着看板说「试玩」。先记着，私测里看有多少人在意这个词。
3. **匿名链接 24 小时**。两个实验都是「发到群里让人玩几天」的形态，第二天链接就死了。登录是解法，在做；在此之前 CLI 应该在链接剩不到几小时时提醒。
4. **发完没有一行「怎么看结果」**。控制台在 `playtest.roviix.com/console/`，令牌在配置文件里——这两句话应该出现在发布成功的输出里，
   否则「知道结果」这个产品的一半没人知道在哪。

## 五、没验的

真手机没点（还是那句）；mofish 的定位与 Service Worker 离线路径没走；空投站的 Turnstile 没配、抽签流程没跑；两个作品都没有第二个人来玩，
所以点名册里只有我自己——「知道结果」的那一半这次没有新证据。
