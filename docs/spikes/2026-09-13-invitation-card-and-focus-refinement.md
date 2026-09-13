# 2026-09-13 邀请函卡片比例重塑、搜索框双高亮消除、广场间距与登录弹窗引导打磨

## 动机与改进范围

依照 DESIGN §3.3、§3.9、§3.10、§3.15 与 AGENTS 规约，针对用户提出的 4 项 UI 细节问题进行了专项精雕与验证：

1. **邮箱登录弹窗：「使用关注时的邮箱」体验重塑 (`edge/src/follow.rs`, `docs/DESIGN.md`)**：
   - 移除按钮下方突兀的免责小字短语 `<p class="notice-note">使用关注时的邮箱。</p>`；
   - 遵照 DESIGN §3.10 精神，将引导自然融入输入框占位符：`placeholder="输入曾用于关注的邮箱"`；
   - 弹窗结构收敛为标题、输入框、纯白发送按钮，阅读动线自然流畅，消除生硬补丁感。

2. **搜索框同心双高亮与抢戏绿图标消除 (`ui/discovery.css`)**：
   - 彻底清除内层 `<input>` 在 `:focus` 和 `:focus-visible` 时的默认 outline 与 box-shadow；
   - 移除 `.search-box:focus-within button{color:#71e8bf}` 强行将放大镜变绿的干扰样式，打字时放大镜保持安静克制的 `#71717a`；
   - 搜索框外壳保持单一而精致的黑曜微晶冷萃绿发光边缘（`border-color: #71e8bf; box-shadow: 0 0 0 1px #71e8bf33, 0 2px 10px -2px #57ad9520;`）。

3. **广场「最新 / 近 7 天」Tab 与卡片间距重叠修复 (`ui/discovery.css`)**：
   - 为 `.discover-toolbar` 补充 `margin-bottom: 22px;` 呼吸间距；
   - 彻底解决绿色激活指示线下划线与下方第一排作品卡片顶边 0 像素贴合、卡片 hover 浮动时产生碰撞重叠的视觉拥挤问题。

4. **「开始体验卡片」（邀请函门禁卡）长宽比例重塑与票根质感精雕 (`ui/invitation.css`, `edge/src/gate.rs`, `ui/dialog.css`)**：
   - **长宽黄金比例**：卡片宽度舒展至 `max-width: 420px;`，头图从高耸的 `4/3`（高 300px）统一调整为精炼稳重的 `16/10`（高 262px），高度骤降，与广场作品卡片头图比例严格一致，view-transition 转场无形变拉伸；
   - **字卡星轨重绘**：同步将 `edge/src/gate.rs` 中生成星轨艺术 SVG 调整为 `viewBox="0 0 400 250"`（16:10），保持准星居中与微粒对齐；
   - **票根咬口与撕线**：`.body` 声明 `position: relative`，使两侧票根半圆冲孔咬痕（`::before`, `::after`）精准咬合在头图与正文交界处的虚线撕口（`border-bottom: 1.5px dashed var(--line2)`）上，呈现出物理票根凭证的质感；
   - **行动存根区（Stub）精雕**：姓名输入框采用紧凑黑曜石卡槽设计，主行动按钮采用纯钛冷白高对比质感，并跨组件复用样式节约体积；
   - **严守字节上限**：优化后卡页（`BASE + CARD`）与整页（`BASE + PAGE`）均严格控制在未压缩硬限制内（6656 字节与 23552 字节）。

---

## 本机浏览器端到端全链路验收

通过 `node scripts/check-collections.mjs` 启动真实无头 Chrome 与后端进程，验收通过：

验收截图（归档于 `docs/spikes/img/`）：
- [广场桌面 22px 间距与无重叠卡片流](img/2026-09-13-collections-104935-plaza-desktop.png)
- [搜索框聚焦黑曜单发光与安静放大镜图标](img/2026-09-13-collections-104935-search-focus-desktop.png)
- [邮箱登录弹窗紧凑占位引导无多余小字](img/2026-09-13-collections-104935-follow-login-dialog.png)
- [独立邀请函卡片 16:10 黄金比例与精致票根撕线咬口（桌面）](img/2026-09-13-collections-104935-invitation-standalone-desktop.png)
- [独立邀请函卡片移动端 390px 自适应](img/2026-09-13-collections-104935-invitation-standalone-mobile.png)

---

## 自动化测试核验

- `cargo test -p playtest-edge --lib html::tests`：全量通过（CSS 预算断言通过）；
- `cargo test -p playtest-edge --test social`：全量通过（20 项测试全部通过）；
- `cargo test -p playtest-edge --test avatar`：全量通过（3 项测试全部通过）；
- `cargo test -p playtest-edge --lib follow::tests`：全量通过（13 项测试全部通过）。
