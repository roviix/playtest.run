# 个人中心质感重构（对标 mira）与接入 dl.roviix.com（对标 nexus R2）

日期：2026-09-14
分支：main
范围：`console`（个人中心页面与样式）、`deploy`（一键安装脚本与 R2 上传工具）、`.github`（发布工作流）

---

## 背景与改进动机

1. **个人中心质感生硬**：原有个人中心以大面积表单和沉重输入框为主，缺乏视觉层级与呼吸感；用户明确提出参考 `../mira` 极简、克制、编辑式高级质感的架构；
2. **下载分发国内提速**：原有 `install.sh` 仅依赖 GitHub Releases，在部分网络环境下易发生超时或断流；用户提出参考 `../nexus`，接入公司已在用的 `dl.roviix.com`（Cloudflare R2 CDN 边缘缓存分发）。

---

## 落地与技术细节

### 1. 个人中心质感重构（全面对标 mira 设计哲学）

- **身份展台 (Identity Surface)**：
  - 呈现 56px 细腻头像与翡翠微光外圈（当缺少头像时以单字母渐变微标优雅兜底）；
  - 展示名采用编辑体（`font-weight: 600`, `letter-spacing: -0.015em`），紧跟绿色实心状态点 `Creator` 胶囊微标；
  - 提供极简弱化的「公开主页 ↗」与「退出」快速操作。
- **行内就地编辑 (Inline Editing)**：
  - 彻底废除原有全宽表单，展示名静止态表现为纯净文本，悬停微现铅笔图标；
  - 点击后行内平滑展开微型输入框与 `0/40` 字数计数器，提供极简的「保存」与「取消」按钮，支持 Enter 提交与 Escape 放弃。
- **统一设置原语体系 (SettingsCard & SettingsRow)**：
  - `SettingsCard`：采用黑曜石深色背景、柔性弥散阴影（`0 1px 3px rgba(0,0,0,0.3), 0 6px 18px rgba(0,0,0,0.2)`）与极细半透边框；
  - `Card Header`：14px 紧致标题，支持长说明折叠进 `?` 按钮展开沉降提示框；
  - `SettingsRow`：左侧「13.5px 标题 + 12px 浅灰说明」，右侧自然对齐「控件/状态胶囊」；
  - `SettingsChip`：带 6px 实心状态点的胶囊徽章（`tone="pos"` 翠绿已连接、`tone="accent"` 平台身份）。
- **开发者令牌安全交互**：
  - 卡片右上角集成「+ 生成新令牌」操作按钮；
  - 生成新令牌时以深墨绿安全高亮呼出框展示，支持一键复制与复制成功反馈；
  - 活跃令牌列表展现前缀等宽字、创建日期与活跃徽章，撤销操作引入行内二次防误触确认。

### 2. 下载分发接入 `dl.roviix.com`（Cloudflare R2）与平滑回退

- **`deploy/site/install.sh`**：
  - 默认优先通过 `https://dl.roviix.com/files/` 下载二进制包与 `SHA256SUMS`（国内直连免代理、Cloudflare 边缘缓存命中后极速下载）；
  - 若 CDN 线路未命中（例如尚未上传或 404），脚本自动打印提示并平滑回退至 `https://github.com/$REPOSITORY/releases/download/$VERSION`；
  - 补充 `fail()` 错误退出处理，杜绝之前缺少退出函数的问题。
- **`deploy/upload-release-to-r2.sh`**：
  - 参考 `nexus`，编写专门的上传校验脚本，支持利用 wrangler 自动化将构建产物上传至 R2 `files/` 前缀，并自动执行 CDN 回读及 SHA-256 核验；
- **`.github/workflows/release.yml`**：
  - 在 Release job 中追加 R2 上传步骤，环境支持时自动发布至 CDN。

---

## 验证与测试记录

1. **前端构建与类型检查**：
   - `pnpm --prefix console build`：TypeScript 检查 0 错误，打包产物 `dist/assets/index-*.js` / `css` 成功生成；
2. **后端测试回归**：
   - `cargo test -p playtest-edge`：通过全量 202 项测试；
   - `cargo test -p playtest-api --test accounts`：通过全部 8 项账号与令牌测试；
3. **安装脚本真机回退实测**：
   - 在未上传 R2 阶段测试 `install.sh`，确认自动优雅回退至 GitHub Releases，提示清晰，退出码与错误处理正常。
