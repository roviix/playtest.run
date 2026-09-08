# 2026-09-08 · 「知道结果」这条线在本机全通：SDK → 写入端点 → 库 → 结果端点

**结论**：DESIGN §3.4 的两层数据在本机接成了一条线。一个只加了一行
`<script src="/_playtest/sdk.js"></script>` 的页面，从门禁页进去之后：边缘记下 `start` / `html_view`，
SDK 报了加载用时、输入、一个自定义事件 `level_done`、一个 JS 错误；这些全部进了控制面的
SQLite，`GET /v1/sites/{slug}/results` 读出来是「1 个人打开，1 个人进到游戏，停留中位 12 秒，
1 种错误」——边缘那半和 SDK 那半合在**同一个会话**里，因为两边用的是同一个 `pt_sid`。

这次接上的两段线（此前是断的）：

1. **边缘事件不再只留在本地文件里**：`edge/src/ship.rs` 每 60 秒把 `edge-events.jsonl` 里新增的行
   批量 `POST /v1/ingest/edge`，送到哪一行记在旁边的 `.offset` 文件里，边缘或控制面重启都不重发不漏发。
   由 `PLAYTEST_API_INTERNAL_URL` 开启（compose 里是 `http://api:8787`）；没设就只落盘。
2. **SDK 由边缘同源提供**：`GET /_playtest/sdk.js`，`sdk/dist/playtest.js` 在构建期编进边缘二进制
   （服务器上不装 Node），ETag 是内容哈希。上传模式的「自动注入」没做（要改开发者的 HTML），
   v0.1 让开发者自己加那一行。

**没成立的**：反馈按钮的文字提交没在这次点过（`feedback` 表为空；写入端点有集成测试）；
`/_playtest/me` 的 `api` 字段上线时必须显式配 `PLAYTEST_API_PUBLIC_URL`——`playtest.sh` 没买，
现在线上是空，SDK 会安静地不发（第一层数据不受影响，边缘走内网）；只在无头 Chrome 里跑过。

机器：macOS，rustc 1.96.0，Chrome 152（无头，`--disable-gpu`）。

## 一、起进程

```
$ PLAYTEST_API_LISTEN=127.0.0.1:8790 PLAYTEST_DATA_DIR=.data/final PLAYTEST_SITE_URL_TEMPLATE='http://{slug}.localhost:8446' ./target/debug/playtest-api
$ PLAYTEST_EDGE_LISTEN=127.0.0.1:8446 PLAYTEST_DATA_DIR=.data/final \
    PLAYTEST_API_PUBLIC_URL=http://127.0.0.1:8790 PLAYTEST_API_INTERNAL_URL=http://127.0.0.1:8790 ./target/debug/playtest-edge
INFO 事件每 60 秒送一批到 http://127.0.0.1:8790
```

## 二、发一个只加了一行脚本的页面

`/tmp/pt-sdk-site/index.html`：一个标题、两个按钮（打 `level_done`、故意抛错）、
`<script src="/_playtest/sdk.js"></script>`。

```
$ HOME=/tmp/pt-sdk-home ./target/debug/playtest /tmp/pt-sdk-site --api http://127.0.0.1:8790 --no-qr -n "SDK 链路自检"
http://tidy-moose-74.localhost:8446
本次 0.1 秒

$ curl -s -D - -o /tmp/sdk.js -H 'Host: wise-mink-28.localhost' http://127.0.0.1:8446/_playtest/sdk.js
HTTP/1.1 200 OK
content-type: text/javascript; charset=utf-8
cache-control: public, max-age=86400
etag: "a66c15b26ff54168"
content-length: 5243
```

## 三、无头 Chrome 走一遍

```
$ node scripts/headless-check.mjs "http://tidy-moose-74.localhost:8446/" --click 'form[action="/_playtest/start"] button' --wait 2500
gate：匿名开发者 邀请你体验《SDK 链路自检》
page：playtest 对象存在：event, feedback, session | … | 反馈      ← 右下角反馈按钮由 SDK 挂上
console：[]

$ node scripts/headless-check.mjs "http://tidy-moose-74.localhost:8446/" --settle 800 --click '#fire' --wait 2500
page：… | 已发 level_done
$ node scripts/headless-check.mjs "http://tidy-moose-74.localhost:8446/" --settle 800 --click '#boom' --wait 2500
page：… | 已抛错误
console：[exception] Uncaught Error: 自检用的错误 at http://tidy-moose-74.localhost:8446/:14:78
```

## 四、库里与结果端点

```
$ sqlite3 .data/final/api.sqlite "select kind, name, count(*) from session_events where slug='tidy-moose-74' group by 1,2"
load|      |3
input|     |2
error|Uncaught Error: 自检用的错误 @ http://tidy-moose-74.localhost:8446/:14|1
event|level_done|1
html_view| |1          ← 边缘经 /v1/ingest/edge 送来的
start|     |1          ← 同上，和 SDK 的事件在同一个 session

$ curl -s -H "Authorization: Bearer <发布时的匿名令牌>" http://127.0.0.1:8790/v1/sites/tidy-moose-74/results
{
 "slug": "tidy-moose-74", "title": "SDK 链路自检", "current_version": 1,
 "versions": [{
   "version": 1, "opened": 1, "entered": 1, "dropped_before_first_frame": 0, "returned": 0,
   "dwell_median_s": 12, "played_5min_plus": 0,
   "errors": { "distinct": 1, "total": 1, "top": [{ "fingerprint": "Uncaught Error: 自检用的错误 @ …:14", "count": 1 }] },
   "load_failures": 0, "feedback_count": 0
 }]
}
```

三次「打开」在 `opened` 里算 1 个人：同一个浏览器、同一个 `pt_sid`——点名册数的是人不是页面刷新，
这正是 DESIGN §3.4 想要的。

## 五、`ship.rs` 的三条单测验证了什么

新行只送一次、送到哪记住了、半行不算送过；控制面连不上时 offset 不动（下一轮重送）；日志文件不存在不算错。
上一轮进程被磁盘满带走前，`.offset` 里已经是文件总长（1288），控制面里也有对应的 `start` / `html_view`——
说明进程死掉那一刻之前的事件已经送达。
