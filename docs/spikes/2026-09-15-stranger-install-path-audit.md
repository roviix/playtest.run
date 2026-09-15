# 2026-09-15 · 一个陌生人今天能不能装上并发出第一条链接

日期：2026-09-15 12:40–13:05 CST。环境：macOS arm64 本机，HEAD `bc12db9`（v0.2.0 之后 60 个提交），生产 `playtest.run`。没有用本机已保存的令牌：以空 `HOME` 运行 CLI，走的是匿名首次路径。

## 结论

- **今天公开可下载的唯一二进制 v0.2.0 已经连不上生产。** 它编进的控制面是 `playtest.roviix.com`（`common/src/lib.rs` v0.2.0: `DEVELOPER_HOST`），路径是 `/v1/sites`；现在 `playtest.roviix.com` 不解析（curl 返回 000），生产只在 `playtest.run/v1/projects` 应答。任何人按 README、`skill.md` 或 `install.sh` 装完，第一条命令就失败。
- **HEAD 构建的 CLI（0.3.0）以陌生人身份走完整条路。** `playtest . --json` 上传 Phaser 夹具（2 个文件 1.1 MB）：`elapsed_ms` 5733（prepare 4344 · upload 1168 · commit 215），拿到 `calm-mink-74`。玩家侧未登录 curl：`/p/calm-mink-74` 200、`calm-mink-74.playtest.run/` 200 且 `<title>` 是作品名、`card.png` 69 KB、`card-wide.png` 54 KB 均 200。`playtest rm calm-mink-74 -y` 删除成功。
- **仓库私有导致所有 Releases 链接对外 404**（未登录 `github.com/roviix/playtest.run/releases` → 404）。`agent.json`、`skill.md`、README 的安装入口都指向它。
- **release.yml 的 R2 上传步骤从未真正执行过。** 步骤 `if: env.CLOUDFLARE_API_TOKEN != ''` 引用的是它自己的 step-level env（在 `if` 里不可见），且 release job 没有 checkout，`deploy/upload-release-to-r2.sh` 不存在。R2 上的 v0.2.0 与 `VERSION` 是手工传的。仓库当前没有配置任何 Actions secrets。
- git 历史扫描：没有 `.env`、密钥或 IP 入库；仅测试里的 `gho_fake` 假令牌与 ssh 别名 `playtest-hk`。可以翻公开。

## 本轮改动

- `api/src/routes/agents.rs`：`skill.md` / `llms.txt` / `agent.json` 安装入口改为 `curl -fsSL https://playtest.run/install.sh | bash`，Windows 指向 `releases/latest`。
- `README.md`：同上；去掉「Example for macOS Apple Silicon」误导。
- `deploy/site/install.sh`：兜底版本 v0.2.0 → v0.3.0；下载失败文案不再说「向邀请你的人确认权限」。
- `.github/workflows/release.yml`：release job 加 checkout；R2 步骤条件改为 job 层 `HAS_R2`；产物里生成 `VERSION` 让 `install.sh` 不问 GitHub 就知道最新版。
- `edge/src/plaza.rs`：AI Prompt 文案 "Report back" → "Hand back"，修掉 `edge/tests/social.rs` 里「广场卡上不出现 Report」的守卫误报（`cargo test --workspace --locked` 现在全绿）。
- 新增 `SECURITY.md`。

## 没验的

- 没有从 `install.sh` 实际装 v0.3.0——tag 还没打，R2 `VERSION` 仍是 v0.2.0。打 tag 前若不配 `CLOUDFLARE_API_TOKEN` / `CLOUDFLARE_ACCOUNT_ID` secrets，`install.sh` 会继续从 R2 装到坏的 v0.2.0。
- 没有 Windows、Linux 真机；没有手机扫码；没有第二个人。
- `prepare_ms` 4.3 秒里多少是匿名令牌签发、多少是网络，没拆。
