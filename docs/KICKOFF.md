# 开工清单

> 2026-09-07。`DESIGN.md` 已定，代码一行没写。新会话从这里开始：先读 `AGENTS.md`、`docs/DESIGN.md`，再读本文。
> 本文是活的：做完一条划掉一条，判断改了直接改。

## 0. 现状

- 仓库已 `git init`，尚无提交。代码：Rust workspace `common/`、`api/`、`edge/`、`cli/` 都已落地，`cargo test --workspace` 153 条全绿（2026-09-07）。
- **第一周目标在本机成立**：`playtest <目录>` → 控制面 → 边缘 → 真实 Chrome 点开门禁页进游戏，见 `docs/spikes/2026-09-07-e2e-upload-localhost.md`。四个引擎导出物只有 Phaser 与 Vite，缺 Godot、Unity（本机没引擎）。
- **第二周主干同日完成**：`playtest <端口>` 隧道在本机与香港边缘都通，socket.io 联机穿过去了，见 `docs/spikes/2026-09-07-tunnel-e2e.md`。差两台手机对局、开发者侧断网恢复、真 Vite。
- **第三周「结果」那一半提前完成**（2026-09-08）：门禁页硬线与熔断、上传前检查、`--json` 与 MCP、SDK 与写入端点、结果端点与控制台，以及边缘事件每 60 秒上报——在本机接成一条线（`docs/spikes/2026-09-08-sdk-ingest-results.md`），并已部署到香港。
- **开发者这一侧上线**（2026-09-08）：`playtest.roviix.com` 承载介绍页、控制台、控制面；CLI 默认指向它，不再需要 ssh 隧道（`docs/spikes/2026-09-08-developer-host-online.md`）。
- **CI 与发布**：每次推送跑全量测试、clippy `-D warnings`、`cargo fmt --check`、控制台与 SDK 构建；`v*` 标签出五个目标的 CLI 单文件到 GitHub Releases。`v0.1.0` 已打。仓库已有三次提交，`cargo test --workspace` 388 条全绿。
- 域名 `playtest.run`（内容）**已购买、解析已在 Cloudflare（DNS-only）指向香港机器**；开发者侧用 `playtest.roviix.com`（2026-09-08 定，不买 `playtest.sh`），已解析到同一台机器，控制面与控制台有了公网入口。
- **香港边缘已上线**：Caddy + api + edge 跑在 `playtest-hk` 上，`*.playtest.run` 泛域名证书已签，三个真链接在线（`docs/spikes/2026-09-07-hk-online-first-links.md`，部署方式见 `deploy/README.md`）。没有 OAuth 应用；对象存储是机器磁盘。
- 竞品图见 DESIGN §2，2026-09-07 的快照；调研原文在 `docs/research/`（13 条线，综合在 `docs/research/README.md`）。**DESIGN 已于同日下午按调研修订**：微信可玩降为待验证、香港不作卖点、上传与隧道一等公民、`--json` 与 `playtest mcp` 进 v0.1、免费档 10 GB + 匿名 1 GB/24h + 每 slug 每小时熔断、结果层改点名册句式、时刻表加 M2b「让对方敢点开」、留门与指标更新（DESIGN §9 有清单）。

## 1. 开工前只有创始人能给的（缺一条，代码就只能在本机绕着走）

| # | 事 | 影响 |
|---|---|---|
| 1 | ~~**语言**：Go 还是 Rust（DESIGN §4.7）~~ **已定 Rust**（2026-09-07），一个 Cargo workspace：`cli/`、`edge/`、`api/`、`common/` | 骨架已按此起 |
| 2 | ~~**两个域名买下**，DNS 放在有 API 的服务商（泛域名证书要 DNS-01）~~ `playtest.run` **已买、DNS 已迁 Cloudflare、证书已签**；开发者侧 **`playtest.roviix.com`**（不买 `playtest.sh`） | 都已上线 |
| 2b | **GitHub 归属**：放 `roviix` 组织，仓库名 `playtest.run`（`github.com/playtest` 是 2015 年起的闲置个人号，拿不到）。这意味着 roviix 是「制作者」品牌、daemon 是旗舰产品——同名公司 + 多个产品是常见结构（`astral-sh` 的 ruff 与 uv，`denoland` 的 deno 与 fresh）。配套三件事：两个仓库都公开后组织 profile 写明「roviix 做两件东西」；本仓库自带 TRADEMARK / SECURITY / 举报邮箱，托管内容的举报与安全报告不流向 daemon；选了 Rust，代码里没有模块路径写归属的问题。备选：先放个人账号，公开前再转（GitHub 保留跳转）。顺手占 npm 包名 `playtest.run`、crates.io `playtest`（2026-09-07 查过均未被占） | 不再阻塞代码；公开前落实 |
| 3 | **GitHub OAuth App**（设备授权流用） | 没有就先做匿名 24 小时链接那条路 |
| 4 | ~~**香港云账号**一台机器~~ **已开**：AWS `ap-east-1` 的 `t4g.small`，弹性 IP `18.163.174.245`（`docs/spikes/2026-09-07-aws-hk-instance.md`）。哪家云做正式边缘仍按 DESIGN §4.4 晚高峰压测后定。对象存储私测阶段用这台机器的磁盘，S3 等第二个边缘 | **已部署**（`deploy/`：Docker、Caddy 自动签证书、服务器上构建、每小时备份），三个真链接在线 |
| 5 | ~~匿名链接进不进 v0.1、免费档角标~~ **已定：都要**（2026-09-07，DESIGN §9）。~~对外文案先说哪边~~ **已定：不选，文案不说地理**（DESIGN §4.4） | 门禁页文案与 CLI 默认行为已按此做 |

## 2. v0.1 的施工顺序

原则：**最薄的一条线先穿过所有层**，先证明形状对，再加厚。每一步的「做完」都对应 DESIGN §8 v0.1 完成定义里的一句。

**第一周 · 上传路径全通（本机）** —— 2026-09-07 大部分完成
~~`playtest ./dist` → CLI 算哈希、问缺哪些、上传、提交清单 → 控制面存清单 → 边缘按清单服务（wasm MIME、预压缩、`--isolated`、Range）→ 门禁页（标题 / 版本 / 开始 / 举报）~~ → 同一 Wi‑Fi 手机扫终端二维码打开（`*.localhost` 手机解析不到，等域名）。
做完 = ~~Phaser、Vite~~ **Godot、Unity** 四个真实导出物在本机边缘上零配置能玩，记进 `docs/spikes/`。还差：在装了引擎的机器上导出 Godot 4（线程与非线程各一个）和 Unity（Brotli）放进 `fixtures/`；WebGL 渲染路径（无头验证用了 `--disable-gpu`）；有人在带界面的 Chrome 和微信里真点一次。

**第二周 · 隧道路径全通（本机）** —— 2026-09-07 主干完成，本机与香港都通
~~`playtest 5173` → 出站 WSS + yamux → 边缘按 Host 找会话、开流、HTTP/1.1 写入 → `Host` 改写 → 101 后双向拷贝 → 离线页 → 退避重连~~（`docs/spikes/2026-09-07-tunnel-e2e.md`：socket.io 联机的 14 条自检在 `http://<slug>.localhost:8446` 与 `https://<slug>.playtest.run` 上都全过；接管、边缘重启后重连、Ctrl-C 离线页都验了）。~~游戏响应头对隧道回来的响应同样修正~~（`game_headers::apply`，只有集成测试）；~~响应体一个字节不动~~（逐字节比对过）；~~保活走 WSS 控制通道~~；~~按体积算「一个玩家约等多少秒」~~（代码与单测有，真机没触发过 >10 MB 的页面）。
做完 = 一个 socket.io 或 Colyseus 的联机小游戏**两台手机能对局**（链接是真的了，`https://<slug>.playtest.run`，还没人用两台手机试）；拔网线 30 秒内自动恢复（只验了边缘侧重启，开发者侧断网没验）；Vite 开发服务器不 403（`Host` 改写已做，没接过真 Vite）；Godot 线程导出物经隧道也能开（缺 Godot 导出物）。

**第三周 · 身份、版本、结果** —— 2026-09-07/08 结果这一半先做完了
GitHub 设备授权、~~匿名 24 小时链接~~、slug 改名与黑名单（黑名单已在 `common::slug`，改名要登录）、版本列表与回滚、~~短期签名令牌~~（隧道令牌已做）与撤销、~~用量按 60 秒上报~~（边缘事件每 60 秒批量送控制面，`edge/src/ship.rs`；带宽用量还没算）；~~结果第一层（打开、进到游戏、设备、来源、停留、资源失败）~~；~~错误上报与反馈按钮（只收文字）~~（`sdk/`，边缘在 `/_playtest/sdk.js` 同源提供）；~~控制台（作品时间线 · 每版一段话、会话点名册、反馈流），手机可看~~（`console/`，`playtest.roviix.com/console/`）；~~`--json` 与 `playtest mcp`~~；~~上传时的导出物检查与人话报错~~（`cli/src/inspect/`，「一定打不开」的会拦下，`--force` 放行）；~~每 slug 每小时熔断~~。
spike：`2026-09-07-gate-hardlines-breaker`、`2026-09-07-cli-json-mcp`、`2026-09-07-console-results`、`2026-09-07-gate-user-activation`、`2026-09-08-sdk-ingest-results`、`2026-09-08-cli-inspect`。
还差：GitHub 登录（需要 OAuth App）、版本回滚接口、令牌撤销推给边缘、带宽配额计量、上传时自动注入 SDK。

**第四周 · 上线与真机** —— 2026-09-07 提前做了前两项
~~香港边缘部署~~（AWS 香港这一台已上线，`deploy/`；云在三网晚高峰压测一周后定）、~~泛域名证书~~（Caddy + Cloudflare DNS-01 已签）、对象存储接上（私测用机器磁盘）、~~控制台上线~~（`playtest.roviix.com/console/`）；**手机流量扫码点开一次**（链接已是真的，`docs/spikes/2026-09-07-hk-online-first-links.md`，还没有人用手机试过）；**发布阻断**：微信真机（Android ≥ 6 台、iOS ≥ 4 个系统版本，私聊 / 群聊 / 朋友圈）+ 腾讯对外链规范 §2.5 的书面口径、香港压测记录；写完所有 spike；招 20 个私测开发者（DESIGN §8 八个指标与一个访谈问题从这里开始计；渠道是 Discord、itch 社区、引擎论坛、国内 Jam 社群、AI 编程社群，不是 Show HN）。

### 2b. 并行分工（2026-09-07 下午起）

两个会话同时在改仓库。为了不互相覆盖，按文件归属分：

| 归属 | 范围 | 谁 |
|---|---|---|
| 隧道路径 | `edge/src/tunnel*`、`cli/src/tunnel*`、`common/src/tunnel*`、隧道令牌的 api 路由；`scripts/`、云与域名、部署 | 实现会话（第二周） |
| 门禁页与边缘防线 | `edge/src/gate.rs`、`html.rs`、`pages.rs`，新 `edge/src/breaker.rs`、`edge/src/game_headers.rs`（把 `app.rs` 里的游戏响应头逻辑抽成可复用函数，隧道路径调它）；`common/src/manifest.rs` 加 `engine` 字段；`common/src/limits.rs` 加带宽配额 | 调研会话 · 子任务 A |
| 上传时检查 | `cli/src/detect.rs` 及其扩展、`cli/src/upload.rs` 里的调用点 | 调研会话 · 子任务 B |
| CLI 机器模式与 MCP | 新 `cli/src/output.rs`、`cli/src/mcp.rs`；`cli/src/args.rs` / `main.rs` / `ui.rs` 里的最小接线 | 调研会话 · 子任务 C |
| SDK 与写入端点 | 新 `sdk/`；新 `api/src/routes/events.rs`、`feedback.rs`、迁移 `002_sessions_events_feedback.sql`；新 `common/src/ingest.rs` | 调研会话 · 子任务 D |
| 结果读端点与控制台 | 新 `api/src/routes/results.rs`、新 `console/`、新 `common/src/results.rs` | 调研会话 · 子任务 E |
| 门禁页音频手势 spike | 新 `fixtures/gate-audio/`、`docs/spikes/2026-09-07-gate-user-activation.md` | 调研会话 · 子任务 F |

共用文件（`edge/src/app.rs`、`edge/src/lib.rs`、`cli/src/main.rs`、`cli/src/args.rs`、`api/src/routes/mod.rs`、`common/src/lib.rs`、各 `Cargo.toml`）只做最小插入，改前重读，不整文件重排，不跑 `cargo fmt --all`。本机 8787 / 8443 上跑着的 api / edge 是别人的，自己起用别的端口和 `.data/<名字>/`（`.data/` 已 gitignore）。

## 3. 本机怎么验泛域名

不买域名也能把两条路走通：Chrome 与 Firefox 把 `*.localhost` 一律解析到 `127.0.0.1`，边缘本机监听 `:8443`（自签或明文），`brisk-otter-41.localhost:8443` 就是一个 slug。Safari 不认 `*.localhost`，本机测试用 Chrome；手机真机要等域名。

本机三个进程的约定（都从仓库根目录起，数据都在 `.data/`，已 gitignore）：

| 进程 | 监听 | 环境变量 |
|---|---|---|
| `api` | `127.0.0.1:8787` | `PLAYTEST_DATA_DIR=.data`（`store/` 对象存储、`api.sqlite`）、`PLAYTEST_SITE_URL_TEMPLATE=http://{slug}.localhost:8443` |
| `edge` | `127.0.0.1:8443` 明文 | `PLAYTEST_DATA_DIR=.data`（只读 `store/`）、`PLAYTEST_HOST_SUFFIX=localhost` |
| `playtest` CLI | — | `PLAYTEST_API=http://127.0.0.1:8787`（或 `--api`） |

对象存储的目录布局与清单格式定义在 `common/`，api 写、edge 读，两边不直接通话。

## 4. 每次合入都要过的

- DESIGN 里对应的那句「完成」能指到一条 spike。
- CLI 输出里没有内部比喻，第一次用的人看得懂。
- 玩家路径上没有出现 `playtest.roviix.com`（根域介绍页那一个链接除外）；登录路径上没有出现 `*.playtest.run`。
- 没有密钥进仓库。

## 5. 给新会话的第一句话（建议）

> 读 `AGENTS.md`、`docs/DESIGN.md`，再按日期读 `docs/spikes/` 看做到哪（上传、隧道、结果三条线都已在香港边缘上线，`deploy/README.md`）。语言是 Rust，契约在 `common/`。接下来是第三周剩下的「身份」那一半：GitHub 设备授权（要 OAuth App，回调地址在 `playtest.roviix.com`）；版本回滚接口；令牌撤销推给边缘；带宽配额计量。再往后是第四周的真机：手机扫码、微信、Godot / Unity 导出物、晚高峰回程压测。每做完一步在 `docs/spikes/` 里记一条。
