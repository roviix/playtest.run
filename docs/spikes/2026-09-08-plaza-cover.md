# 广场与封面：本机全链路（2026-09-08）

DESIGN §3.8（广场）与 §3.3（封面）落地后的真机记录。三个进程都是本机 debug 构建：
api 在 `127.0.0.1:8791`、edge 在 `127.0.0.1:8447`、数据目录 `.data/plaza`。

## 一条命令上广场

```
$ HOME=/tmp/pt-plaza-home ./target/debug/playtest fixtures/phaser-jump/export \
    --api http://127.0.0.1:8791 --no-qr -n "跳一跳" \
    --summary "一个 Phaser 平台跳跃小游戏，三关，五分钟。" \
    --cover /tmp/pt-cover.png \
    --seek "新手引导看得懂吗？第二关是不是太难？"
正在整理 fixtures/phaser-jump/export：2 个文件，1.1 MB
看起来是 Phaser 做的
需要上传 3 个文件（1.2 MB），其余 0 个服务器上已有        ← 封面和目录文件走同一条 blob 路
已发布 v1

http://glad-marten-32.localhost:8447

已放到广场上并标了「正在找人测」：http://localhost:8447/ 。来的人玩成什么样，控制台里看得见。
这是匿名链接，2026-09-09 20:59 后失效。…
本次 0.2 秒（哈希 0.1 · 上传 0.1 · 提交 0.0）
```

第二个作品（vite-vanilla，`--public` 不求测）同样上了广场；`playtest unlist sturdy-puffin-11`
之后广场只剩一张卡，链接照常能开。

## 验过的事

- **根域 = 广场**：`GET http://localhost:8447/` 服务端直出整页；两张卡的
  `data-seeking` / `data-game` 与实际一致（Phaser 认成游戏、Vite 认成体验）。
  截图确认：封面、「正在找人测」标、想让你看、匿名到期倒计时、页脚三句如实说明
  （不排名次 / 登录没做好所以只有 24 小时作品 / 想玩只记在本机）全部在。
- **CSP 锁死**：根域响应带 `default-src 'none'; img-src http://*.localhost:8447;
  style-src 'unsafe-inline'; script-src 'nonce-…'`；作品页面上依旧一个 CSP 都没有
  （`no_csp_header_anywhere` 测试守着）。
- **封面**：`GET /_playtest/cover` → `200 image/png 41862B`，带 `etag` 与
  `cache-control: public, max-age=86400`；没封面的作品是裸 404（无 HTML）。
  门禁页把它顶在卡片最上面（`class="hero"`），`og:image` 是绝对地址、
  `og:image:type` 跟清单；`--summary` 进了正文和 `og:description`。
- **plaza.json 的重写时机**：公开 / 撤下 / unlist / 提交新版本 / 控制面启动 / 每 5 分钟。
  边缘缓存 30 秒——unlist 后 30 秒内旧卡可能还在，第 31 秒起消失（实测）。
- **`cargo test --workspace` 414 条全绿**，其中新增：common 的封面校验与魔数嗅探、
  store 的封面引用与 plaza 读写、api 的 6 条广场集成（谁能上 / 立即重写 / 无版本不能公开 /
  3 个不同会话举报自动撤下且复核后旧举报不算 / 玩家计数与「来自广场」标记 / 封面 blob 校验）、
  edge 的广场渲染与转义、封面分发。

## 撞到的坑

- CLI 把封面算进上传清单后，「其余 N 个服务器上已有」的减法用的还是目录文件数，
  首跑 `attempt to subtract with overflow` panic。总数改成含封面后修复。
- `plaza_candidates` JOIN users 后按列名下标取 `display_name`，第一版下标写死 15，
  加 `reviewed_at` 列后错位（`Invalid column type Null`）。改成 `SITE_COLUMN_NAMES.len()`。
- 无头 Chrome `--headless=new` 截图在这台机器上不退出（截图文件已写出、进程挂着），
  用 `& sleep && kill` 兜底；跟产品无关。

## 没验、留给下一步的

- 「来自广场」标记只在 api 集成测试里验过（Referer=根域 → `referrer_kind=plaza`），
  真浏览器从广场点进门禁页再点开始的一整条还没在本机 Chrome 里走——它依赖边缘事件
  ship 到 api，形态和既有 `gate_view` 一样，风险低。
- 举报自动撤下在真机上没演习（测试里验了 3 个不同会话的阈值与复核语义）。
- 香港服务器还没部署这一版；`deploy/push.sh` 下次发布时迁移 003 会自动跑。
