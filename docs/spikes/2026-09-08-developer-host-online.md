# 2026-09-08 · 开发者这一侧上线：`playtest.roviix.com` 的介绍页、控制台、控制面，以及「来自哪里」终于是真的

**结论**：开发者路径全部在公网上了。`playtest ./dist` 不带任何参数就能发布（CLI 默认控制面是
`https://playtest.roviix.com`，不再需要 ssh 隧道）；`https://playtest.roviix.com/console/` 用发布时的匿名令牌进去，
在 390px 宽的手机视口里走完作品列表 → 时间线 → 点名册，数据是刚才真实浏览器产生的；
`/_playtest/me` 现在告诉 SDK 事件发到 `https://playtest.roviix.com`，跨域 CORS 通过，`event` / `error` 落库。
顺手修了一个让「来自哪里」全部变成「其它」的 bug，Discord 来的玩家现在在点名册里写着「来自 Discord」。

域名决定：不买 `playtest.sh`，用 `roviix.com` 的子域（DESIGN §4.1）。DNS 由创始人在 Cloudflare 加 `A playtest → 18.163.174.245`（DNS-only）。

## 一、切换

```
服务器 deploy/.env：API_HOST=playtest.roviix.com  PLAYTEST_API_PUBLIC_URL=https://playtest.roviix.com
$ TARGET=server deploy/push.sh          # CLI 默认地址、根域介绍页链接、me.rs 都指向新域名
Caddy：certificate obtained successfully identifier=playtest.roviix.com   （约 40 秒）

$ curl -sS -o /dev/null -w '%{http_code}\n' https://playtest.roviix.com/healthz      200
$ curl -sS -o /dev/null -w '%{http_code}\n' https://playtest.roviix.com/console/     200
$ curl -sS -o /dev/null -w '%{http_code}\n' https://playtest.roviix.com/v1/sites     401（无令牌，正确）
$ curl -sS https://playtest.roviix.com/ | grep -o '<title>[^<]*'                     playtest · 一条命令，把这个版本放到别人面前
```

介绍页是 `deploy/site/index.html` 一个文件，Caddy 直接托管，4.8 KB，无外部资源。

## 二、CLI 不带参数发布

```
$ HOME=/tmp/pt-fresh ./target/debug/playtest ./fixtures/phaser-jump/export --no-qr -n "Phaser 跳一跳"
看起来是 Phaser 做的
已发布 v1
https://rapid-puffin-32.playtest.run
本次 1.0 秒
$ HOME=/tmp/pt-fresh ./target/debug/playtest /tmp/pt-sdk-site --no-qr -n "SDK 链路自检"
https://quick-toad-65.playtest.run
```

## 三、SDK 事件跨域到公网控制面

```
$ curl -sS -b <门禁 cookie> https://quick-toad-65.playtest.run/_playtest/me
{"slug":"quick-toad-65","version":1,"api":"https://playtest.roviix.com"}

无头 Chrome：门禁 → 开始 → 点「level_done」→ 点「抛错误」，Console 只有那个故意抛的错误
库里 quick-toad-65：load 3 · input 2 · start 2 · error 1（Uncaught Error: 自检用的错误 @ …:14）· event level_done 1 · html_view 1
```

## 四、控制台四页（无头 Chrome，390×844，真实按键输入令牌）

```
1 初始页：粘贴令牌 | 令牌只存在这台设备的浏览器里，不会发给第三方。 | 还没有令牌？在作品目录里运行 playtest…
2 作品列表：SDK 链路自检 | quick-toad-65 · 最新 v1 · 建于 9 月 8 日 11:44 | 看结果 | 反馈 | 玩家看到的链接 | 这个链接 9 月 9 日 11:44 到期 | Phaser 跳一跳 | …
3 时间线：v1 · 9 月 8 日 11:44 | 2 个人打开，1 个人进到游戏（1 个在加载时走了）。 | 停留中位数 0 秒。 | 1 个错误撞了 1 次（Uncaught Error: 自检用的错误 @ …）。 | 看这 2 个人
4 点名册：停留最短在前 | 0 秒 · 电脑 · 其它 · 其它 · 点了开始，没等到首帧 · 来自直接打开 | 13 秒 · 电脑 · Chrome · macOS · 进到游戏 · 玩到「level_done」· 1 个错误 · 最后一次动手在进入后 11 秒
Console 错误：无（favicon 404 除外）
```

截图：`docs/spikes/img/2026-09-08-console-timeline.png`、`2026-09-08-console-roster.png`。
「0 秒 · 其它 · 其它 · 点了开始，没等到首帧」那一行是 curl 造的会话，说的都对。

## 五、修掉的：「来自哪里」永远是「其它」

点名册第一次跑出来时，真实浏览器的那一行写着「来自其它」。查 `edge-events.jsonl`：

```
gate_view | sid=(空)     | referer='https://discord.com/'                 ← 真正的来源，但这时还没有会话
start     | sid=f98566f4 | referer='https://quick-toad-65.playtest.run/'  ← 会话从这一下算起，Referer 是门禁页自己
```

会话由「开始」那一下建立，而那一下的 Referer 永远是门禁页自己——于是每个人都「来自其它」。两处改：

1. 门禁页把自己收到的 Referer 放进表单隐藏字段 `from`，`start` 事件记的是它（只接受 `http(s)://` 形态，`edge/tests/serving.rs::the_real_referrer_survives_the_gate`）。
2. 控制面把「来源是作品自己」当作不知道而不是「其它」（`common::ingest::is_self_referral`），留给真的外部来源填。

验证：新浏览器带 `Referer: https://discord.com/channels/123/456` 打开门禁页 → 点开始 → 等 60 秒上报：

```
sid       referrer_kind  browser  first_seen_at
a8c6115a  discord        chrome   2026-09-08T04:29:36Z
```

## 六、顺手发现的一个时间炸弹

`api/tests/events_feedback.rs` 把事件时间写死成 2026-09-07 04:00，控制面只采信 24 小时内的客户端时间戳（`stamp`），
今天这条测试就静悄悄地失效了（时间戳被换成「现在」，断言首帧时间失败）。改成以「现在 − 10 分钟」为锚的偏移量。
`results.rs` 直接写库、不经过时间窗口，不受影响。

## 七、没做的

- 手机真机仍然没人点过；微信没试。
- 登录没有，控制台靠粘令牌进；令牌过期（24 小时）后控制台会看不到作品。
- 介绍页「安装」一节写的是「克隆后 cargo build」——没有预编译包，公开前要补 GitHub Release 与三平台二进制。
- 点名册里 curl 会话显示「电脑 · 其它 · 其它」，UA 识别不认 curl 是对的，但真实世界里 UA 五花八门，分类的准确率没测过。
