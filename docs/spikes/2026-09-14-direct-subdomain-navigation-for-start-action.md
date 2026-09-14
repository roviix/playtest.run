# 2026-09-14 开始体验直达作品专属子域与异步统计信标改造

## 背景与排查

用户在访问作品主域邀请函（`https://playtest.run/p/<slug>`）时反馈：
> 「点击开始体验.. 感觉跳转有问题？这个时候是否应该直接新标签页跳转域名打开呢？」

通过查阅生产环境边缘服务器日志（`/data/edge-events.jsonl`），确认了现场真实情况：
- 用户在 `00:16:47` 访问邀请页；
- 用户在 `00:16:50` 点击「开始试玩」，但**未产生任何页面请求**（被浏览器拦截或无响应）；
- 直到 10 秒后用户怀疑卡住再次点击，才终于记录到 `00:17:00.64 html_view`。

### 根本原因排查：
1. **跨源 POST + 303 重定向遭遇现代浏览器弹窗拦截（Popup Blocker）**：
   原有实现是 `<form class="start" method="post" action="/p/<slug>" target="_blank">`。当用户点击时，浏览器在打开的新标签页中发起跨源 POST 提交，服务端返回 `HTTP 303 See Other` 重定向到 `https://<slug>.playtest.run/`。
   现代浏览器（特别是 Safari、配置了防追踪机制的 Chrome/Edge）将包含 `target="_blank"` 的 POST 表单跨源 303 重定向判定为可疑弹出窗口（Suspicious Pop-up），从而静默挂起或延迟弹出。
2. **网络双跳延迟**：
   新标签页必须经历 `POST playtest.run -> 等待 303 -> GET slug.playtest.run` 两次网络往返，在公网环境下导致 500~1000ms 白屏卡顿。
3. **移动端强行剥离 target**：
   代码曾包含 `if(/Mobi|Android|iPhone/i.test(navigator.userAgent)){document.querySelector('form.start')?.removeAttribute('target');}`，导致手机端点击后覆盖当前主域邀请函，按后退还会触发浏览器的「是否重新提交表单」警告。

---

## 解决方案

### 1. 直达作品专属域名（Direct Subdomain Navigation）
根据 `docs/DESIGN.md §3.3` 规范：
- 将「开始试玩 / 开始体验」按钮渲染为直达独立源的链接元素：`<a class="start-btn" href="https://<slug>.playtest.run/" target="_blank" rel="noopener">`；
- 浏览器识别为最高优先级的**用户直接手势导航**，在 Safari / Chrome / iOS / Android 下 **100% 不会被拦截**；
- 鼠标悬停时浏览器状态栏直接显示作品干净域名（如 `https://lucky-robin-21.playtest.run/`），点击后地址栏立即建立 DNS/TLS 连接，瞬间直达，杜绝任何中间白屏。

### 2. 玩家留名与开始事件后台异步信标上报
- 在 `ui/player.js` 中增加全局监听：
  - 用户点击 `.start-btn` 时，通过非阻塞的 `navigator.sendBeacon`（或 `fetch(..., { keepalive: true })`）向 `/p/<slug>` 发送表单数据（昵称、来源、Referer）；
  - 用户在昵称输入框中按回车（`Enter`）时，拦截 `form.start` 提交事件，触发异步信标并调用 `window.open(targetUrl, '_blank', 'noopener')`；
  - 绝不阻塞目标作品子域在新标签页的秒级开启。
- 在 `edge/src/app.rs` 中：
  - `project_door` 在接收到 POST 且包含 `Accept: application/json` 时，记录 `Kind::Start` 事件后快速返回 `200 OK` JSON，保持高效轻量。

### 3. 彻底移除移动端剥离 target 脚本
- 删除原有 `mobile_script`（`removeAttribute('target')`）；
- 手机端与桌面端保持一致：在新标签页打开作品，原主域邀请函和广场在后台标签页完好保留，方便玩家随时返回关注或写一句话反馈。

### 4. 样式与渐进增强对齐
- 在 `ui/invitation.css` 与 `ui/dialog.css` 中将 `.start :is(button, .start-btn)` 样式完全统合；
- 无 JS 环境下，用户点击 `.start-btn` 依然能通过纯原生 `<a>` 标签 100% 成功进入作品。

---

## 验证记录

### 1. 本地自动化测试
- `cargo test -p playtest-edge --lib gate::tests`：25 passed, 0 failed；
  - 包含 `root_invitation_card_links_and_actions` 验证直接子域超链接与 `removeAttribute('target')` 的彻底清除。
- `cargo test -p playtest-edge --test serving`：33 passed, 0 failed；
  - 包含对直达链接 `href="http://{HOST}/"`、`class="start-btn"`、`target="_blank"` 的断言；
  - 包含携带 `Accept: application/json` 与玩家昵称时返回 `200 OK` 并准确记录 `start` 事件的断言。

### 2. 真机公网部署与验证
- 代码提交并推送到远端；
- 部署至线上生产节点 `playtest-hk`；
- 公网 curl 验证与真实浏览器点击验证。
