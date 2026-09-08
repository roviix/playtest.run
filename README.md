# playtest

> 一条命令，把你手上这个能玩的版本放到别人面前，然后知道他们玩成了什么样。
> One command to put the build you have right now in front of real people — and see what happened.

**状态：v0.1 私测前——两条路都在香港边缘上线了，还没有登录和公网控制面。** `playtest <目录>` 与 `playtest <端口>` 都能拿到一个真的 `https://<slug>.playtest.run` 链接：上传的导出物和穿隧道的 socket.io 联机房间都在真实 Chrome 里验过；门禁页、第一层数据、SDK、结果端点与控制台在本机接成了一条线。还没做的：GitHub 登录（现在全是 24 小时匿名链接）、`playtest.sh` 域名（控制面只能经 ssh 隧道访问，控制台因此还没上线）、令牌撤销、用量配额。做不到的地方命令会明说。真机记录在 [`docs/spikes/`](docs/spikes/)。产品定义在 [`docs/DESIGN.md`](docs/DESIGN.md)，每个结论都带推导过程；方向变了直接改它。施工顺序在 [`docs/KICKOFF.md`](docs/KICKOFF.md)。

## 它会是什么

- `playtest ./dist` —— 上传引擎导出的目录，几秒拿到 `https://<slug>.playtest.run` 和一个二维码。
- `playtest 5173` —— 把正在跑的本地开发服务器或联机后端接出去，链接长得一样。
- 玩家点开就玩：不登录、不装东西、微信里能开。
- 开发者知道结果：谁开了、什么设备、加载成没成、报了什么错、说了什么。

## 域名

- `*.playtest.run` 只放玩家看的东西（作品、门禁页、隧道转发），没有登录。
- `playtest.sh` 品牌站、开发者控制台、API、文档。

## 仓库布局

Rust 一个 Cargo workspace（`cargo test --workspace`），控制台与 SDK 是 TypeScript。

| 目录 | 内容 | 许可 | 状态 |
|---|---|---|---|
| `cli/` | `playtest` 命令行：上传、隧道、`ls / rm / open`、`--json`、`mcp`、上传前检查 | Apache-2.0 | 可用；`login` 未做 |
| `common/` | 三方共享的契约：清单、对象存储布局、API 类型、隧道令牌与 WebSocket↔yamux 字节流 | Apache-2.0 | 可用 |
| `edge/` | 边缘：泛域名入口、门禁页、按清单分发、隧道接入、熔断、第一层事件与上报、同源 SDK | AGPL-3.0 | 可用；TLS 由前置 Caddy 做 |
| `api/` | 控制面：匿名令牌、slug、版本、隧道令牌、事件与反馈写入、结果读取 | AGPL-3.0 | 可用；登录、撤销、配额未做 |
| `sdk/` | `playtest.js`：JS 错误、加载用时、自定义事件、反馈按钮；由边缘在 `/_playtest/sdk.js` 提供 | Apache-2.0 | 可用；上传时自动注入未做 |
| `console/` | 控制台（Preact）：作品列表、时间线、点名册、反馈流，手机可看 | AGPL-3.0 | 本机可用；上线等 `playtest.sh` |
| `deploy/` | 一台机器跑通：compose、Caddy（DNS-01 泛域名证书）、服务器上构建与发布脚本 | — | 香港在用 |
| `fixtures/` | 测试用的真实导出物（Vite、Phaser、响应头自检页、socket.io 联机房间、门禁页音频测试台） | — | 缺 Godot、Unity |
| `scripts/` | 开发用脚本（无头 Chrome 走一遍玩家路径） | — | — |
| `docs/` | `DESIGN.md` 唯一产品定义；`spikes/` 真机记录（只增）；`research/` 市场与用户调研 | — | — |

根目录 `LICENSE` 是 Apache-2.0；`api/`、`edge/` 各自带 AGPL-3.0。

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
