# 2026-09-13 邀请函左上角品牌导航、移除下一件、左侧卡片与右侧沉浸式试玩交流面板

## 动机与改进范围

依照 DESIGN §3.15 与用户提出的体验优化要求：
1. **品牌标识左上角定位**：将 `playtest.run` 品牌名与图标固定在页面左上角，若从合集进入，则在右侧紧跟 `‹ 返回 [合集名]` 单一路径，路径清晰克制。
2. **剔除冗余按钮**：完全移除「下一件」轮播跳转按钮，玩家专注于当前作品体验或返回所属合集。
3. **打破呆板居中与加宽交流空间**：
   - 彻底破除原先 784px 狭窄居中束缚；
   - 采用 1040px 舒展舞台，左侧独立卡片（420px 黄金宽度、16:10 头图防拉伸），右侧原声交流聊天室（560px 宽敞气泡与即时上屏）；
   - 交流面板包含快速贴纸（`🎮 手感绝了`、`🎨 美术惊艳`、`🎵 配乐神作`、`💡 脑洞大开`、`🐛 抓个Bug`、`☕ 治愈满分`），支持一键点击发送贴纸；
   - 移动端采用全屏微信式聊天面板设计，带返回顶部条与自适应吸底输入框。

---

## 本机浏览器端到端全链路验收

执行 `node scripts/check-collections.mjs` 启动真实无头 Chrome 与后端进程，验收通过：

验收截图（归档于 `docs/spikes/img/`）：
- [邀请函左上角品牌及返回合集导航、左卡右聊舒展布局](img/2026-09-13-collections-115059-invitation-desktop.png)
- [独立邀请函桌面发送体验感受与精选贴纸即时上屏](img/2026-09-13-collections-115059-invitation-chat-submitted-desktop.png)
- [独立邀请函移动端（390px）自适应票根卡片](img/2026-09-13-collections-115059-invitation-standalone-mobile.png)
- [移动端全屏微信式试玩交流面板与贴纸栏](img/2026-09-13-collections-115059-invitation-chat-mobile.png)

---

## 自动化测试与预算核验

- `cargo test -p playtest-edge --lib gate::tests`：24/24 全部通过；
- `cargo test -p playtest-edge --lib html::tests`：6/6 全部通过（严格满足字节预算与无外部依赖约束）；
- `cargo test -p playtest-edge --lib share::tests`：3/3 全部通过；
- `cargo build -p playtest-edge`：编译无告警无错误；
- `node scripts/check-collections.mjs`：8 项核心验收全部通过（异常列表为空）。
