# playtest

> 一条命令，把你手上这个能玩的版本放到别人面前，然后知道他们玩成了什么样。
> One command to put the build you have right now in front of real people — and see what happened.

**状态：v0.1 私测前——上传、隧道、结果三条线都在线上。** `playtest <目录>`、`playtest <端口>`、`playtest <目录> --backend <端口>` 都能拿到一个真的 `https://<slug>.playtest.run` 链接，控制面与控制台在 `https://playtest.roviix.com`：上传的导出物、穿隧道的 socket.io 联机房间、静态上传加后端走隧道的小应用都在真实 Chrome 里验过；门禁页、第一层数据、SDK、结果端点与控制台接成了一条线。GitHub 登录（`playtest login`，设备码流程；登录前发的匿名作品归入账号）刚接上，正在真机验证。还没做的：自己挑链接名字、令牌撤销、用量配额；方向 B 那条主线（邀请卡 → 扫码 → 关注 → 第二版收到通知）代码在了，真机还没走通一次。做不到的地方命令会明说。重写的蓝图在 [`docs/REWRITE.md`](docs/REWRITE.md)。真机记录在 [`docs/spikes/`](docs/spikes/)。产品定义在 [`docs/DESIGN.md`](docs/DESIGN.md)，每个结论都带推导过程；方向变了直接改它。施工顺序在 [`docs/KICKOFF.md`](docs/KICKOFF.md)；一张图看架构在 [`docs/architecture.html`](docs/architecture.html)（浏览器直接打开）；要 review 设计用 [`docs/overview.html`](docs/overview.html)——五张可点的图（故事、三条路、组件与域名、结果与身份、边界与判据），每个元素标着它在 DESIGN 里的出处。

## 它会是什么

- `playtest ./dist` —— 上传引擎导出的目录，几秒拿到 `https://<slug>.playtest.run` 和一个二维码。
- `playtest 5173` —— 把正在跑的本地开发服务器或联机后端接出去，链接长得一样。
- `playtest ./dist --backend 3000` —— 带后端的小应用：目录里有的文件上传，目录里没有的路径（`/api/…`、WebSocket）走隧道到你电脑上的 3000 端口。
- `playtest ./dist --public --seats 10` —— 顺手放到广场（`playtest.run` 首页）上，标「正在找 10 位试玩者」，路过的人点开就玩；`playtest unlist` 拿下来。
- `playtest files <slug>` —— 线上这一版到底是哪些文件：路径、大小、内容哈希。换台电脑也知道现在发出去的是什么。
- 给 AI 助手：`playtest mcp` 起一个 MCP server；没装的助手可以读 [`/llms.txt`](https://playtest.roviix.com/llms.txt)、[`/skill.md`](https://playtest.roviix.com/skill.md)、[`/openapi.json`](https://playtest.roviix.com/openapi.json)。
- 玩家点开就玩：不登录、不装东西、微信里能开。
- 开发者知道结果：谁开了、什么设备、加载成没成、报了什么错、说了什么、有几个人是从广场来的。

## 域名

- `*.playtest.run` 只放玩家看的东西（作品、门禁页、隧道转发），没有登录。根域是广场：开发者主动公开的、正在找人测的作品，按时间排，没有点赞和评论。
- `playtest.roviix.com` 开发者控制台、API、文档。

## 怎么活

开发者对免费工具最怕的是它某天消失——SIMMER.io 是被一笔 DDoS 账单打死的，Glitch 停了托管。所以先把账算给你看（推导在 [`docs/DESIGN.md`](docs/DESIGN.md) §6）：

- 固定成本是一台香港边缘、控制面、对象存储和两个域名，每月百美元量级；变动成本是带宽，约 $0.1 / GB。
- 免费档有硬上限：登录账号每月 10 GB、3 个活跃作品；匿名链接 1 GB / 24 小时。到上限硬停，玩家看到一页说明，**不会产生账单**。我们不做后付费——链接发出去之后你控制不了有多少人点开，任何后付费都会把「分享」变成财务风险。
- 收钱的对象是「认真做测试的人」：固定可改的 slug、去角标、口令与邀请名单、更长的结果留存、版本比较。更多流量只卖预付包。
- v0.1 不收钱，先验证 §8 的假设；有人主动问价再接支付。
- CLI 与 SDK Apache-2.0，边缘与控制面 AGPL-3.0，`deploy/` 里一台机器能自己跑通。哪天我们不做了，代码和部署脚本都在。

## 仓库布局

Rust 一个 Cargo workspace（`cargo test --workspace`），控制台与 SDK 是 TypeScript。

| 目录 | 内容 | 许可 | 状态 |
|---|---|---|---|
| `cli/` | `playtest` 命令行：上传、隧道、`ls / rm / open / versions / rollback / files / whoami`、`--json`、`mcp`、上传前检查 | Apache-2.0 | 可用 |
| `common/` | 三方共享的契约：清单、对象存储布局、API 类型、隧道令牌与 WebSocket↔yamux 字节流 | Apache-2.0 | 可用 |
| `edge/` | 边缘：泛域名入口、门禁页、按清单分发、隧道接入、熔断、第一层事件与上报、同源 SDK | AGPL-3.0 | 可用；TLS 由前置 Caddy 做 |
| `api/` | 控制面：匿名令牌与 GitHub 登录、slug、版本、隧道令牌、事件与反馈写入、结果读取、给助手读的 `llms.txt` / `skill.md` / `openapi.json` | AGPL-3.0 | 可用；令牌撤销、配额未做 |
| `sdk/` | `playtest.js`：JS 错误、加载用时、自定义事件、反馈按钮；由边缘在 `/_playtest/sdk.js` 提供 | Apache-2.0 | 可用；上传时自动注入未做 |
| `console/` | 控制台（Preact）：作品列表、时间线、点名册、反馈流，手机可看 | AGPL-3.0 | 在线：`playtest.roviix.com/console/` |
| `deploy/` | 一台机器跑通：compose、Caddy（DNS-01 泛域名证书）、服务器上构建与发布脚本 | — | 香港在用 |
| `fixtures/` | 测试用的真实导出物（Vite、Phaser、响应头自检页、socket.io 联机房间、门禁页音频测试台） | — | 缺 Godot、Unity |
| `scripts/` | 开发用脚本（无头 Chrome 走一遍玩家路径） | — | — |
| `docs/` | `DESIGN.md` 唯一产品定义；`spikes/` 真机记录（只增）；`research/` 市场与用户调研 | — | — |

根目录 `LICENSE` 是 Apache-2.0；`api/`、`edge/` 各自带 AGPL-3.0。

## 安装与第一次使用

预编译的单文件在 [Releases](https://github.com/roviix/playtest.run/releases)（macOS arm64 / x86_64、Linux x86_64 / arm64 musl、Windows x86_64）。解开后把 `playtest` 放进 PATH：

```
cd 你的导出目录        # Godot / Unity / Phaser / Vite 导出的那个，里面有 index.html
playtest .            # 几秒后：链接 + 二维码；第一次运行自动拿一个 24 小时的匿名链接
playtest 5173         # 或者把正在跑的本地开发服务器接出去
```

结果在 [`playtest.roviix.com/console/`](https://playtest.roviix.com/console/)，令牌在 `~/.config/playtest/config.json` 里。

## 本机跑一遍

```
cargo build --workspace
PLAYTEST_DATA_DIR=.data ./target/debug/playtest-api     # 127.0.0.1:8787
PLAYTEST_DATA_DIR=.data PLAYTEST_API_INTERNAL_URL=http://127.0.0.1:8787 ./target/debug/playtest-edge   # 127.0.0.1:8443，明文
PLAYTEST_API=http://127.0.0.1:8787 ./target/debug/playtest ./fixtures/phaser-jump/export   # 上传
PLAYTEST_API=http://127.0.0.1:8787 ./target/debug/playtest 5174                             # 隧道（先在 fixtures/ws-rooms 里 node server.mjs 5174）
```

拿到的 `http://<slug>.localhost:8443` 用 Chrome 或 Firefox 打开（Safari 不认 `*.localhost`）。
控制台：`cd console && pnpm install && pnpm dev`，令牌用 `~/.config/playtest/config.json` 里的那个。
上线部署见 [`deploy/README.md`](deploy/README.md)。

## 参与

私有开发到 v0.1 私测通过后公开。建造纪律见 [`AGENTS.md`](AGENTS.md)。
