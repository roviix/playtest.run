# 2026-09-13 广场合集解耦、搜索框黑曜微晶质感与登录主行动钛白按键打磨

## 动机与改进范围

依照 DESIGN §3.9、§3.10、§3.15 与 AGENTS 规约，针对用户提出的广场与合集架构关系、搜索框样式与移动端布局、以及关注页按钮对齐和弹窗主行动按键质感进行专项系统性打磨：

1. **广场与合集权责解耦（Plaza & Collections Decoupling）**：
   - 彻底废除原广场页（`/`）顶部硬塞 3 张巨大合集卡（`collection-grid`）导致作品流被严重压至第二屏的问题；
   - 拒绝多层 Tab 结构（作品本身已具备「最新 / 近 7 天」Tab，顶部再加 Tab 将造成混乱的双层 Tab 灾难）；
   - 确立清晰职责：**左侧侧栏保留「合集」一级入口**；**广场专注作品流**，首屏直达公开试玩作品；合集与创作挑战在大厅（`/collections`）完整呈现；仅在用户主动输入关键词搜索且命中合集时，才在结果中智能列出相关合集。
2. **搜索框黑曜微晶质感与移动端自适应布局 (`ui/discovery.css`, `edge/src/discovery.rs`)**：
   - **布局约束**：桌面端右侧固定 280px 宽度（`flex: 0 0 280px`），不做通栏大搜索，与左侧标题自然拉齐；移动端（`<=640px`）标题与搜索框自适应转为纵向垂直流，搜索框占满全宽（`width: 100%`），高度升至 40px 便于触控；
   - **材质质感**：采用黑曜微晶底 `#0c0d12` 与发丝微边 `#ffffff14`；悬停提亮；激活聚焦（`:focus-within`）采用精细冷萃绿发丝微晶圈（`border-color: #71e8bf; box-shadow: 0 0 0 1px #71e8bf33, 0 2px 10px -2px #57ad9520;`），拒绝粗糙发散的大面积绿光晕；
   - **快捷交互**：搜索有内容时，右侧呈现极简圆形一键清空按键（`.search-clear`），点击直达清空状态。
3. **关注页与登录弹窗细节修复 (`ui/follow.css`, `ui/dialog.css`)**：
   - **关注页登录按钮居中**：给 `.follow-empty .restore-follow` 增加 `display: inline-flex; align-items: center; justify-content: center; text-align: center;`，彻底修复因缺少弹性居中声明导致的 160px 宽度内文字偏左问题；
   - **登录弹窗主按键改纯钛白**：遵照 DESIGN §3.3 规范，将 `.notice-form button` 从原青绿色改为**纯钛白高对比底面**（`#ffffff` 白底墨黑字 `#09090b`，字重 600、14px，`box-shadow: 0 4px 14px #0008, inset 0 1px #fff`），具备 `scale(.985)` 实体按压触感，与邀请函主按键保持一致。
4. **CSS 字节预算硬约束守住 (`edge/src/html.rs`)**：
   - 剔除 `workspace.css` 与 `dialog.css` 中的重复声明与冗余选择器，卡页样式（`BASE + CARD`）控制在 6640 字节（< 6656 字节），整页样式（`BASE + PAGE`）控制在 23430 字节（< 23552 字节），100% 遵守未压缩硬上限。

---

## 本机浏览器端到端全链路验收

2026-09-13，通过 `node scripts/check-collections.mjs` 在 macOS 上启动真实无头 Chrome (DevTools Protocol) 以及 API、Edge 后端进程，执行 8 项端到端真机验收：

1. **真实数据库迁移与上传**：全量迁移 7 个版本，生成演示作品物料；
2. **合集聚合与基础搜索**：聚合挑战与普通作品集，检查标题、作者与作品计数；
3. **桌面与 390px 布局**：1440px 桌面与 390px 手机视口下均无水平溢出；
4. **关闭 JavaScript 后搜索降级**：禁用脚本通过 GET 表单正确过滤作品；
5. **复制题目与状态反馈**：题目展台点击一键复制且即时显示「已复制题目」；
6. **邀请函合集上下文与下一件轮转**：从挑战点入邀请函，上下文条与轮转正常；
7. **控制台真实投稿**：通过控制台提交新作品并即时持久化；
8. **邮箱确认与关注闭环**：通过 SQLite 确认邮件激活关注，关注列表即时显示合集。

验收截图（归档于 `docs/spikes/img/`）：
- [广场桌面 1440px 直达作品与 280px 微晶搜索框](img/2026-09-13-collections-103426-plaza-desktop.png)
- [广场移动端 390px 垂直自适应](img/2026-09-13-collections-103426-plaza-mobile.png)
- [关注页空态 160px 邮箱登录按钮完美文字居中](img/2026-09-13-collections-103426-follow-empty-desktop.png)
- [邮箱登录弹窗纯钛白高对比主行动按键](img/2026-09-13-collections-103426-follow-login-dialog.png)
- [合集详情页桌面](img/2026-09-13-collections-103426-challenge-desktop.png)
- [已关注合集后移动端状态](img/2026-09-13-collections-103426-subscribed-mobile.png)

---

## 自动化测试核验

- `cargo test -p playtest-edge --lib html::tests`：全量通过（CSS 预算断言严格通过）；
- `cargo test -p playtest-edge --test discovery`：全量通过（6 项测试全部 Exit 0）；
- `cargo test -p playtest-edge --lib follow::tests`：全量通过（13 项测试全部 Exit 0）；
- `cargo test -p playtest-edge --lib plaza::tests`：全量通过（14 项测试全部 Exit 0）。
