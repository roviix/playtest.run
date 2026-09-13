# 开工清单

> 2026-09-07 起草，2026-09-09 按方向 B 重写。新会话从这里开始：先读 `AGENTS.md`、`docs/DESIGN.md`，再读本文。
> 本文是活的：做完一条划掉一条，判断改了直接改。

## 0. 现状（2026-09-09）

- **管子这一半在真机上成立。** 上传、隧道、混合（`--backend`）三条路都在香港边缘上线（`deploy/`）；门禁页三条硬线、每 slug 每小时熔断、上传前检查、`--json` 与 `playtest mcp`、SDK 与结果第一层、控制台点名册、广场（告示板形态）与封面、版本与回滚、GitHub 登录（设备码 + 网页授权码）都已落地，各有 spike。`cargo test --workspace` 400+ 条全绿（09-09 早）。
- **方向 B 从今天开始施工。** DESIGN 重写为「一个地方，三个环」：邀请环（邀请卡）、回访环（关注与通知）、注意力环（广场目的地、推广位、周报）。共享契约已种进 `common/`（`live` / `capabilities` / `follow` / `boost` 四个新模块，`plaza` / `api` / `results` / `ingest` / `limits` / `store` / `lib` 的加法字段与常量），`cargo test -p playtest-common` 全绿；`api` / `edge` / `cli` 在接缝处等各自的实现跟上。
- 域名：`playtest.run`（内容，Cloudflare DNS-only → 香港）与 `playtest.roviix.com`（开发者侧）都在线。GitHub OAuth App 凭据是否已放到服务器由创始人确认（compose 空着 = 不提供登录）。
- 工作区里有一个会话留下的未提交改动（控制台网页登录、deploy 环境变量、广场页脚一句），本轮所有人都在它们之上继续，不回退。

## 1. 只有创始人能给的（缺一条，对应那一环只能在本机绕着走）

| # | 事 | 影响 |
|---|---|---|
| 1 | **境外主体与 Stripe 账号**（含支付宝） | 推广与 Pro 开卖；此前控制台如实写「尚未开放」，运营者只能用 `scripts/admin.sh grant` 赠送推广 |
| 2 | **邮件服务商账号**（Resend 或 SMTP）与发信域 `notice@playtest.run` 的 SPF / DKIM / DMARC（Cloudflare） | 关注通知与周报在线上能发；本机用 `PLAYTEST_EMAIL_PROVIDER=log` 验全链路 |
| 3 | **GitHub OAuth App 凭据放到服务器**（`deploy/.env` 的 `PLAYTEST_GITHUB_CLIENT_ID/SECRET`） | 开发者头像、长期作品、推广购买资格 |
| 4 | **香港哪家云**（三网晚高峰压测后定） | 正式边缘 |
| 5 | **推广对大陆用户的法律意见** | 推广开卖的前置 |
| 6 | **一个人的时间**转到供给与社区 | 每天有新东西上广场、举报有人看、周报有人读一遍再发 |
| 7 | 服务端镜像重建并推送（`Dockerfile` 加了 `fonts-noto-cjk`，邀请卡的中文靠它） | 线上邀请卡不是方块 |

## 2. v1 的施工（2026-09-09 起）

原则不变：最薄的一条线先穿过所有层，再加厚。这一轮按 **crate 分层并行**，四个会话各占一层、互不碰文件；共享契约先种好、本轮冻结。

| 层 | 范围 | 要交付的 |
|---|---|---|
| **共享契约** `common/`（已完成） | `live.rs`（`sites/<slug>/live.json`）、`capabilities.rs`、`follow.rs`（玩家表单字段、边缘 → 控制面路由、`MeView`）、`boost.rs`（推广模型与管理接口）、`plaza.rs` 新字段、`api.rs` 设置字段、`results.rs` 名字 / 来源 / 关注数 / 公开反馈、`ingest.rs` `from` / `name` / `source_kind`、`lib.rs` 卡片路径与来源常量 | 全绿；缺口由各层用 `TODO(contract)` 标出，整合时统一补 |
| **边缘** `edge/` + `Dockerfile` | `live` / `capabilities` 缓存；门禁页重做（留名、名额、群、分享、公开反馈、头像、`?from=`）；邀请卡渲染（resvg + fontdb + qrcode，竖版与横版）；分享页；`/_playtest/follow`；根域 `/follow`、`/me`、`/me/confirm/*`、`/me/unsubscribe/*`、`/me/action`、`/_playtest/sw.js`；广场（左栏 + 一面网格、卡上无关注）、稀疏态 | `cargo test -p playtest-edge`；spike `2026-09-09-edge-card-gate-plaza.md` |
| **控制面** `api/` + `scripts/admin.sh` + deploy 环境变量 | 迁移 `005_club.sql`；设置 PATCH；留名与来源；结果新字段；`live.json` / `plaza.json` / `capabilities.json` 写入；关注全流程（双重确认、`pt_me` 令牌、退订）；通知队列（log / resend / smtp、Web Push、24 小时合并、周报）；推广模型、状态推进、管理接口 | `cargo test -p playtest-api`；spike `2026-09-09-api-follow-notify-boost.md` |
| **CLI** `cli/` | `--seats` / `--community` / `--card-out` / `--no-card`；`playtest card` / `followers`；发布成功输出（卡路径、控制台地址、无封面提醒）；`--json` 新字段；MCP 五个工具含邀请卡图片 | `cargo test -p playtest`；spike `2026-09-09-cli-card-seats-community.md` |
| **控制台与 SDK** `console/` + `sdk/` | 设置区（邀请卡预览、名额、群、公开反馈、关注数、推广一节）；时间线来源与关注数；点名册名字；反馈流公开 / 隐藏；SDK 玩后落点（谢谢、有新版本时告诉我、看看别的作品） | 两个包构建与类型检查；`sdk/dist` 重建；spike `2026-09-09-console-sdk-club.md` |

**整合**（分层做完之后，一个会话做）：`cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all --check`、两个 TS 包构建；把各层 `TODO(contract)` 收进 `common/`；本机三进程全链路：发布 → 邀请卡 → 扫码进门禁页留名 → 关注（`log` 发信看到确认链接）→ 点确认落到 `/me` → 发第二版 → 队列里出现通知 → 广场一面网格与稀疏态 → `admin.sh grant` 赠送推广出现「推广」标；写 `docs/spikes/2026-09-09-club-e2e-localhost.md`；然后 `deploy/push.sh` 到香港（镜像含字体），真手机扫卡一次。

**共用文件的规矩**（沿用 09-07 的并行经验）：本轮按 crate 分层所以几乎没有共用文件；`deploy/compose.yaml` 与 `.env.example` 只追加环境变量行；`Dockerfile` 只加字体包；每个层用自己的 `CARGO_TARGET_DIR=target/<层>-agent` 避免抢锁；本机各自起进程用自己的端口与 `.data/<层>/`；谁都不跑 `git stash / checkout / reset / commit`。

## 3. 本机怎么验泛域名

按步骤走完广场、门禁、关注、控制台：[`LOCAL.md`](LOCAL.md)。

不买域名也能把两条路走通：Chrome 与 Firefox 把 `*.localhost` 一律解析到 `127.0.0.1`，边缘本机监听 `:8443`（明文），`brisk-otter-41.localhost:8443` 就是一个 slug，根域就是 `localhost:8443`。Safari 不认 `*.localhost`，本机测试用 Chrome；手机真机要等域名。

本机三个进程的约定（都从仓库根目录起，数据都在 `.data/`，已 gitignore）：

| 进程 | 监听 | 环境变量 |
|---|---|---|
| `api` | `127.0.0.1:8787` | `PLAYTEST_DATA_DIR=.data`、`PLAYTEST_SITE_URL_TEMPLATE=http://{slug}.localhost:8443`、`PLAYTEST_PUBLIC_ROOT_URL=http://localhost:8443`、`PLAYTEST_EMAIL_PROVIDER=log`、`PLAYTEST_ADMIN_TOKEN=dev`、`PLAYTEST_EDGE_INGEST_TOKEN=dev-edge-ingest-token-0000000000` |
| `edge` | `127.0.0.1:8443` 明文 | `PLAYTEST_DATA_DIR=.data`（只读 `store/`）、`PLAYTEST_HOST_SUFFIX=localhost`、`PLAYTEST_API_INTERNAL_URL=http://127.0.0.1:8787`、`PLAYTEST_EDGE_INGEST_TOKEN=dev-edge-ingest-token-0000000000`、`PLAYTEST_API_PUBLIC_URL=http://127.0.0.1:8787`、可选 `PLAYTEST_FONT_DIRS` |
| `playtest` CLI | — | `PLAYTEST_API=http://127.0.0.1:8787`（或 `--api`） |

对象存储的目录布局与文件格式定义在 `common/`：api 写、edge 读，两边不直接通话；边缘对控制面只有写方向的调用（事件、关注、确认、退订）。

## 4. 每次合入都要过的

- DESIGN 里对应的那句「完成」能指到一条 spike。
- CLI 输出、门禁页、邮件里没有内部比喻，第一次用的人看得懂。
- 玩家路径上没有出现 `playtest.roviix.com`（根域介绍那一个链接除外）；登录路径上没有出现 `*.playtest.run`；`pt_me` 只种在根域、host-only。
- 门禁页三条硬线的测试全过；作品域上没有我们的 Service Worker。
- 邀请卡上没有人数、没有「限时 / 领取」；推广位永远标「推广」。
- 没有密钥进仓库；等外部依赖的功能写「尚未开放」，不放不能点的按钮。

## 5. 给新会话的第一句话（建议）

> 读 `AGENTS.md`、`docs/DESIGN.md`（方向 B），再按日期读 `docs/spikes/` 看做到哪。语言是 Rust，契约在 `common/`（`live` / `capabilities` / `follow` / `boost` 是 09-09 新的）。接下来看 §2 的整合那一段：四层是否都交付、`TODO(contract)` 收进 `common/`、本机全链路、然后推香港。每做完一步在 `docs/spikes/` 里记一条。
