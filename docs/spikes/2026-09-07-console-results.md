# 2026-09-07 · 控制台四页在真机上跑通（作品列表 → 时间线 → 点名册 → 反馈流）

**结论**：控制台四页对着真的控制面跑通了，用的是本机 Chrome 152，375px 宽（iPhone SE / 主流安卓的窄边）。
DESIGN §3.4 举例的那段话，在真数据上一字不差地出来了：
「8 个人打开，6 个人进到游戏（2 个在加载时走了）。」
点名册默认按停留最短在前，反馈状态 `PATCH` 从 curl 和界面两条路都真的落库了。

**那段话不是控制面生成的**，规则全在 `console/src/words.ts` 的 `sentences()` / `versus()`。
控制面只给数（`common/src/results.rs`：只有计数和中位数，没有比例也没有平均值）。
这是有意的：同一组数字在手机上、在将来的 `--json`、在 `playtest mcp` 里要说成不同的话，
措辞钉在服务端的话每一处都得先把句子拆回数字。改文案只改 `words.ts` 一个文件。

## 机器与版本

```
macOS darwin 25.2.0 · rustc 1.96.0 (ac68faa20 2026-05-25)
node v22.22.0 · pnpm 11.25.0 · sqlite3 3.51.0
Google Chrome 152.0.7977.76（系统里已装的那个，playwright 1.63.0 用 channel: "chrome" 直接驱动）
```

没有下载任何 playwright 自带的浏览器（`~/Library/Caches/ms-playwright` 是空的，这次也没让它变非空），
也没有新装任何依赖——这台机器磁盘只剩不到 1 GB。

## 怎么起的（照着这段能复现）

```
$ cargo build -p playtest-api
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.83s

$ PLAYTEST_API_LISTEN=127.0.0.1:48787 PLAYTEST_DATA_DIR=.data/console ./target/debug/playtest-api
2026-09-07T08:38:50Z  INFO 数据库迁移已应用 version=1
2026-09-07T08:38:50Z  INFO 数据库迁移已应用 version=2
2026-09-07T08:38:50Z  INFO playtest 控制面已启动：http://127.0.0.1:48787
2026-09-07T08:38:50Z  INFO 数据目录：.data/console

$ sqlite3 .data/console/api.sqlite < console/dev/seed.sql
$ sqlite3 .data/console/api.sqlite "select count(*) from sessions; select count(*) from session_events; select count(*) from feedback;"
19
53
3

$ PLAYTEST_API=http://127.0.0.1:48787 pnpm --dir console dev --port 5273 --strictPort
  VITE v7.3.6  ready in 199 ms
  ➜  Local:   http://localhost:5273/
```

`pnpm build` 也过了（截图用的是 dev server，因为 `vite preview` 不带 `/v1` 代理，而 `vite.config.ts` 只配了 `server.proxy`）：

```
$ pnpm --dir console build
$ tsc --noEmit && vite build
✓ 18 modules transformed.
dist/index.html                  0.52 kB │ gzip:  0.33 kB
dist/assets/index-DY9sRpMV.css   4.10 kB │ gzip:  1.41 kB
dist/assets/index-BU6ap5Ok.js   29.21 kB │ gzip: 11.57 kB
✓ built in 126ms
```

整个控制台 gzip 后 13 KB，`tsc --noEmit` 零报错。

### 令牌哈希对得上

`console/dev/seed.sql` 往 `tokens` 里插的是 `console-dev-token` 的 sha256，和 `api/src/auth.rs` 的存法一致——
那边是 `hash::hash_bytes(token.as_bytes())`，也就是 `common/src/hash.rs` 的 SHA-256 小写十六进制，没有加盐、没有 HMAC：

```
$ printf 'console-dev-token' | shasum -a 256
14052dec726b93e56404b6fb06e67f38be021ed0f111fa90068a0eecc73825ef  -

$ sqlite3 .data/console/api.sqlite "select token_hash from tokens;"
14052dec726b93e56404b6fb06e67f38be021ed0f111fa90068a0eecc73825ef
```

不带令牌是 401，带上就能读——控制台的令牌页把这个串粘进去就通了，截图那一轮走的就是真表单（`input[type=password]` → 提交），不是预置 localStorage。

## 1. 每版那段话的实际输出（浏览器里 `.says` 的原文）

从 `#/s/brisk-otter-41` 的 DOM 里取出来的，不是我照着数字重写的：

**v7 · 9 月 5 日 22:20 · 「改了新手引导」**

```
8 个人打开，6 个人进到游戏（2 个在加载时走了）。
3 个人玩了 5 分钟以上，2 个人回来过，停留中位数 45 秒。
1 个错误撞了 3 次（TypeError: Cannot read 'x' of undefined @ main.js:412）。
1 条反馈。
比 v6 多 3 个人打开，加载失败从 2 降到 0。
```

第一句和 DESIGN §3.4 的示例一字不差。

**v6 · 9 月 4 日 00:40 · 「加了音效」**

```
5 个人打开，3 个人进到游戏（2 个在加载时走了）。
1 个人玩了 5 分钟以上，停留中位数 25 秒。
1 个错误撞了 3 次（TypeError: Cannot read 'x' of undefined @ main.js:412）。
2 次资源没加载出来。
1 条反馈。
比 v5 多 1 个人打开，加载失败从 0 升到 2，错误从 0 次升到 3 次，反馈从 0 条升到 1 条。
```

**v5 · 9 月 1 日 17:12 · 「第一版，能跑了」**（这一版故意没接 SDK）

```
4 个人打开，4 个人点了开始。
这一版没接 playtest.js，加载时走掉几个人看不出来。
1 个人玩了 5 分钟以上，停留中位数 40 秒。
```

v5 没有「和上一版比」那一行——它是最早的一版，没有可比的对象。

### 生成规则在哪、长什么样

`console/src/words.ts`：

- `sentences(version)` → `string[]`，一句一行。**数为 0 的句子整句不说**（「0 条反馈」占着地方却什么也没告诉人）。
  所以 v5 没有错误句、没有反馈句，v7 没有「资源没加载出来」那句。
- 控制面给的 `dropped_before_first_frame` 是 `null` 时（这一版没有任何会话报过首帧，多半是没接 SDK），
  不说「0 个人在加载时走了」，改说「点了开始」+ 明说看不出来。v5 走的就是这条路。
- 「进到游戏」的人数是 `entered - dropped`，在 `words.ts` 里算，控制面不算——
  控制面只报 `opened` / `entered` / `dropped_before_first_frame` 三个原始计数。
- `versus(version, previous)` → 和上一版比的那一行，没有一处变化就返回 `null`，不硬凑一句。
- `seconds()` 把秒说成「45 秒」「11 分 20 秒」；`moment()` 把 RFC 3339 按**看的人自己的时区**渲染。
  截图这一轮浏览器时区是 `Asia/Shanghai`，所以 UTC 的 `14:20` 显示成 `22:20`。

## 2. 点名册默认排序：停留最短在前

不带 `?sort=`，控制面回的 `sort` 就是 `dwell`（`common/src/results.rs` 里 `RosterSort` 的 `#[default]`）：

```
$ curl -sS -H "Authorization: Bearer console-dev-token" \
    "http://127.0.0.1:48787/v1/sites/brisk-otter-41/versions/7/sessions"
sort = dwell
  v7h  dwell=   7s  at=2026-09-05T14:35:00Z  errors=0  reached=None
  v7g  dwell=  11s  at=2026-09-05T14:28:00Z  errors=0  reached=None
  v7f  dwell=  30s  at=2026-09-05T14:25:00Z  errors=1  reached=None
  v7a  dwell=  45s  at=2026-09-05T14:00:00Z  errors=0  reached=过了引导
  v7e  dwell= 100s  at=2026-09-05T14:22:30Z  errors=2  reached=None
  v7d  dwell= 660s  at=2026-09-05T14:20:00Z  errors=0  reached=第一关过了
  v7c  dwell= 680s  at=2026-09-05T14:05:00Z  errors=0  reached=第二关过了
  v7b  dwell= 720s  at=2026-09-05T14:02:10Z  errors=0  reached=第一关过了
```

界面上这一列渲染成 `["7 秒","11 秒","30 秒","45 秒","1 分 40 秒","11 分","11 分 20 秒","12 分"]`，
顶上的切换是「停留最短在前 [选中] / 最近打开在前」。

这就是 DESIGN §3.4 那句「排在最前面的人就是你要看的人」：最前面两行是 7 秒和 11 秒进来就走的人，
两行都带「点了开始，没等到首帧」的红标。按时间排的话这两个人会被埋在第 1 和第 2 位之外。

同样的数据按 `?sort=time`（最近打开在前）：

```
v7h(7s) v7g(11s) v7f(30s) v7e(100s) v7d(660s) v7c(680s) v7b(720s) v7a(45s)
```

玩了 45 秒就没影的 v7a 掉到了最后一行。

排序参数不认识的时候是 400，不是悄悄退回默认：

```
$ curl ... "…/sessions?sort=按停留"
{"code":"invalid","message":"点名册只能按 dwell（停留最短在前）或 time（最近在前）排，不认识「按停留」"}
HTTP 400
```

## 3. PATCH 反馈状态

curl 原文（`-d` 里的 JSON 就是 `UpdateFeedbackRequest`，只有一个 `status`）：

```
$ curl -sS -X PATCH \
    -H "Authorization: Bearer console-dev-token" \
    -H 'Content-Type: application/json' \
    -d '{"status":"seen"}' \
    http://127.0.0.1:48787/v1/sites/brisk-otter-41/feedback/1
{"id":1,"session_id":"v7b","version":7,"ts":"2026-09-05T14:02:57Z","text":"不知道要按哪个键","seconds_in":47,"device":"phone","browser":"safari","status":"seen"}

$ curl … -d '{"status":"done"}' …/feedback/1
{"id":1,…,"status":"done"}
```

回的是改完之后的整条，界面拿它直接替换本地那一条，不用再拉一次列表。

别人作品下的反馈（`id=3` 属于 `sage-lynx-30`）从 `brisk-otter-41` 这条路径改不动，
说的话和「这条不存在」一模一样，不告诉外面这个 id 存不存在：

```
$ curl … -d '{"status":"seen"}' …/sites/brisk-otter-41/feedback/3
{"code":"not_found","message":"没有这条反馈，或者它不是你的作品的。"}
HTTP 404
```

不认识的状态是 400：

```
$ curl … -d '{"status":"nope"}' …/feedback/1
{"code":"invalid","message":"请求内容不是我们认识的格式：可能不是合法的 JSON，也可能少了必填字段。请确认 CLI 和控制面的版本对得上。"}
HTTP 400
```

界面那条路也真的走了一遍：在 375px 的 Chrome 里点第一条的「标为看过了」，标签从「还没看」变成「看过了」，
退出浏览器后库里也是：

```
$ sqlite3 .data/console/api.sqlite "select id, status from feedback order by id;"
1|seen
2|seen
3|new
```

反馈流只列这个作品自己的两条（第三条是 `sage-lynx-30` 的），按时间倒序。

## 4. 375px 截图

`docs/spikes/img/`，Chrome 152，视口 375×812，`deviceScaleFactor: 2`，整页截：

| 图 | 是什么 |
|---|---|
| `console-375-0-token.png` | 令牌页，粘贴前 |
| `console-375-1-sites.png` | 作品列表，两个作品 |
| `console-375-2-timeline.png` | 时间线，v7 / v6 / v5 三张卡 |
| `console-375-3-roster.png` | v7 点名册，第三行展开着事件流 |
| `console-375-4-feedback.png` | 反馈流，第一条「还没看」 |
| `console-375-5-feedback-marked.png` | 点过「标为看过了」之后 |
| `console-375-6-roster-v5-no-sdk.png` | v5 点名册，顶上是「这一版没接 playtest.js」那条提示 |

窄屏上四页都没有横向滚动，卡片、标签、事件流都在 375px 里排得下。
点名册展开一行看到的是这个样子（真实 DOM 文本）：

```
30 秒  平板 · Safari · iOS  9 月 5 日 22:25 打开  收起
进到游戏 / 1 个错误 / 来自直接打开 / 最后一次动手在进入后 22 秒
22:25:04  点了开始
22:25:13  首帧  {"ms":8800}
22:25:26  错误  TypeError: Cannot read 'x' of undefined @ main.js:412
                {"stack":"main.js:412 → update() → tick()"}
会话 v7f
```

没接 SDK 的 v5，点名册顶上明说看不到什么，每一行只有「点了开始」，不冒充「没等到首帧」：

```
这一版没接 playtest.js：首帧、玩到哪、最后一次输入都看不到。
```

## 没验到的 / 已知缺口

- **只在 Chrome 上看过。** Safari（尤其是 iOS 上的微信内置浏览器）没试。控制台是开发者侧、只在 `playtest.sh` 上，
  优先级低于玩家侧，但发布前至少要在真手机的 Safari 上翻一遍。
- **只在本机跑过，没上真机。** 域名、HTTPS、和 edge 一起部署的样子都没验；控制台的静态产物怎么托管也还没定。
- **`vite preview` 不能直接用。** `vite.config.ts` 只配了 `server.proxy`，没配 `preview.proxy`，
  所以 `pnpm preview` 出来的站点打不到 `/v1`。构建产物的真实形态是同源部署（`api.ts` 里 `API_BASE` 默认空串），
  本机想验构建产物得另配一个反代。这次截图走的是 dev server。
- **`/favicon.ico` 404。** `console/index.html` 没声明图标，Chrome 每次都多要一次。只是噪音，但真机上会进日志。
- **`moment()` 不显示年份。** 作品列表上 2027-01-01 到期显示成「1 月 1 日 08:00」，看不出是明年。
  匿名链接是 24 小时，正常不会跨年，但自托管和将来的登录用户会。
- **假数据不是真流量。** 会话、事件、反馈全是 `seed.sql` 摆出来的，时间戳都很规整。
  真人打开时 UA 解析、referrer 归类、`is_return` 判定会不会歪，要等第一次真私测。
- **反馈截图还没有。** `FeedbackItem.screenshot_hash` 一直是 `null`，DESIGN §3.4 说的「附截图」是 v0.2 的事，
  界面上现在不显示这一块。
- **一版超过 100 条事件的截断没构造过。** `more_events` 那句提示语的代码路径没在浏览器里见过。
