# CLI 默认发布路径：减少等待与文件副作用

日期：2026-09-12，22:18 CST。设备：macOS / Darwin arm64。

## 本轮决定

先更新 DESIGN §3.2 / §3.4，再实现：普通上传和端口分享不默认下载邀请卡；不为了「没有封面」的建议追加请求；耗时覆盖返回结果前的实际工作；`--gate` 从帮助撤出，旧写法明确报错。保留现有命令体系、JSON 主体、退出码、招募和结果入口，不新增配置文件或详细输出开关。

邀请卡仍可通过 `playtest card --out invite.png` 或发布时 `--card invite.png` 下载；`--card -` 保留兼容。显式下载仍等待图片操作完成，并将等待计入 `elapsed_ms`。MCP 上传仍返回图片内容块，但不写本地 PNG。

## 本机真实二进制检查

- `target/debug/playtest --help`：不展示 `--gate`；`--card` 明确默认不下载；`--summary` 说明省略时沿用；`--public` 说明只控制广场展示；`--spa` 区分导航与资源请求。
- 使用隔离 HOME、API 指向本地未监听端口，运行 `playtest ./dist --gate once --json`：退出 2，stdout 一个 `usage` 对象，明确提示撤出原因及移除参数；不先检查目录或联网。
- 人类发布输出的进程回归保留单独 stdout 链接，可用于管道；保留匿名到期、广场操作结果、结果入口；没有默认邀请卡路径、缺封面建议或耗时拆解。

## 回归证据

`cargo test -p playtest --no-fail-fast`：205 项通过，0 失败（单元 143、导出检查 10、JSON/命令流程 33、MCP 8、隧道 4、上传 7）。关键新增用例：

1. 人类 / JSON 两种模式从导出目录连续发布两次：卡片 HTTP 请求计数均为 0、没有新增邀请卡文件、两次提交文件清单相同；首次发布无额外作品查询。
2. 显式下载卡片：假边缘延迟 350 ms，实际有图片文件，`elapsed_ms` 包含该等待，且不超过父进程观测时长。
3. 显式保存路径被普通文件阻挡：发布仍成功，`card_path` 不出现，`findings` 明说作品已发布但邀请卡未能保存；原文件内容不变。
4. `--gate once / always / never` 用于目录、端口或管理命令：均返回明确的迁移提示，而非悄悄忽略。
5. 真实 MCP 子进程调用上传工具：返回可解码的 PNG 图片块，没有 `card_path`，工作目录和导出目录均未写图片。

`cargo test -p playtest-api routes::agents`：6 项通过，机器使用指南同步默认行为。

`cargo clippy -p playtest -p playtest-api --all-targets -- -D warnings`、对应包的格式检查和 `git diff --check` 通过。

## 验收边界

流程回归使用真实 CLI / MCP 子进程、本机假 API / 边缘。350 ms 是测试注入的延迟，不是生产性能测量。没有部署、没有使用真实账号、没有手机扫码或 Windows / Linux 真机验收。旧版本清单和 API 的 gate 字段保留兼容；本轮没有修改线上玩家路由或历史 spikes。
