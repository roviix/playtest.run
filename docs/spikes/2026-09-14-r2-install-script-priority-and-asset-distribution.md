# 2026-09-14 install.sh 优先走 R2 CDN 分发与 CLI 安装包同步实机验证

## 背景

1. **现象**：
   用户在终端运行 `curl -fsSL https://playtest.run/install.sh | bash` 时报错：
   ```
   release not found
   错误: 下载失败。私测版本需要仓库访问权限；请向邀请你的人确认版本和权限。
   ```
2. **根因剖析**：
   - **分支优先级倒置**：`install.sh` 此前将 `if command -v gh && gh auth status` 放在最前。只要开发者本机安装且登录了 GitHub CLI（如 `gh auth login`），脚本就会绕过 `dl.roviix.com`，强制走 GitHub 私有 Release 下载；
   - **版本探测兜底错配**：GitHub 仓库 `roviix/playtest.run` 为私有仓库，匿名 curl `releases/latest` 返回 404；此前代码在未能识别最新 tag 时将版本写死为了未发布的 `v0.3.0`（GitHub 上当时最新 tag 为 `v0.2.0`），导致 `gh release download "v0.3.0"` 抛出 `release not found`；
   - **R2 产物尚未初次同步**：此前接入 `dl.roviix.com` 工具时仅编写了上传脚本，未将现有的 `v0.2.0` 全平台预编译包及校验和写入 Cloudflare R2 `nexus-desktop` 存储桶的 `files/` 路径下。

## 方案与改动

1. **R2 极速 CDN 分发设为全局最高优先级**：
   - 修改 `deploy/site/install.sh`，将 Cloudflare R2 / `dl.roviix.com/files` 作为第一首选下载源，免鉴权、免 GitHub 登录、免代理，国内与全球直连；
   - 增加从 `https://dl.roviix.com/files/VERSION` 自动获取当前最新版本能力，兜底设置为已发布的稳定版本 `v0.2.0`；
   - 仅当 R2 镜像未命中且开发者配置了 `gh` 鉴权时，才优雅尝试 `gh release download` 作为备用通道。
2. **全平台 CLI 产物上传至 R2 并在 CDN 验证可用**：
   - 将 `playtest` v0.2.0 的 5 个目标平台压缩包（`aarch64-apple-darwin`、`x86_64-apple-darwin`、`aarch64-unknown-linux-musl`、`x86_64-unknown-linux-musl`、`x86_64-pc-windows-msvc`）以及 `SHA256SUMS` 和 `VERSION` 全部同步至 R2；
   - 增强 `deploy/upload-release-to-r2.sh`，支持上传 `VERSION` 文件，并在 CDN 回读核验时加上时间戳参数避免 Cloudflare 边缘负向 404 缓存干扰。

## 验证

1. **CDN 回读与安装实测**：
   - `PLAYTEST_INSTALL_DIR=/tmp/playtest-test-bin bash deploy/site/install.sh`：直接命中 `https://dl.roviix.com/files/`，4 秒内完成下载、SHA-256 校验与解压；
   - `/tmp/playtest-test-bin/playtest --version` 输出 `playtest 0.2.0`；
2. **边缘测试**：
   - `cargo test -p playtest-edge` 291 项全部通过；
3. **线上部署**：
   - 执行 `deploy/push.sh` 部署最新的 `install.sh`，公网直测 `curl -fsSL https://playtest.run/install.sh | bash`。
