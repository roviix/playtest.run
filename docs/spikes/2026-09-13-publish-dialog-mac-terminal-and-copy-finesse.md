# 2026-09-13 发布弹窗 macOS 终端交通灯、复制滚动条根除与邮箱表单极简打磨

## 动机与改进范围

依照 DESIGN §3.9、§3.10、§3.15 与 AGENTS 规约，针对用户提出的 4 项细节反馈进行了专项精细打磨与验证：

1. **邮箱表单极简重塑 (`edge/src/follow.rs`, `ui/dialog.css`)**：
   - 移除了输入框上方多余的可见文本小标题「邮箱」；
   - 为输入框赋予标准的无障碍属性 `aria-label="邮箱"`，配合占位符引导（如 `placeholder="输入曾用于关注的邮箱"`）；
   - 剔除 `ui/dialog.css` 中冗余的 `.notice-form label` 规则，弹窗结构收敛为「标题 → 单输入框 → 纯白主按键」，极致纯粹。

2. **发布弹窗点击复制滚动条根除 (`ui/workspace.css`)**：
   - 为 `.publish-sheet` 设置 `overflow: hidden;`，完全杜绝因内容或状态微扩展导致的垂直滚动条；
   - 将 `.pub-status` 改造为屏幕阅读器严格脱离文档流规范（`position: absolute; width: 1px; height: 1px; top: 0; left: 0; overflow: hidden; clip: rect(0,0,0,0); margin: -1px;`），彻底消除对文档流与高度的任何干扰。在 `check-collections.mjs` 中增加 `scrollHeight <= clientHeight` 严格自动化断言。

3. **精致克制复制胶囊与高透翡翠绿 (`ui/workspace.css`, `ui/player.js`, `console/src/publish.tsx`)**：
   - 尺寸收缩为 **26px 紧凑胶囊**（`height: 26px; min-width: 62px; padding: 0 8px; font-size: 12px;`），消除多余边距与粗糙感；
   - 移除不成熟的感叹号，文案规范统一为沉着克制的「已复制」；
   - 已复制激活状态升级为**高透纯净翡翠绿**（`color: #34d399; background: #10b98118; border-color: #10b9814d;`），清透纯正且高对比。

4. **bash 终端 macOS 经典三色交通灯设计 (`ui/workspace.css`)**：
   - 引入正统 macOS Terminal 经典红黄绿交通灯微控制点（关闭红 `#ff5f56`、最小化黄 `#ffbd2e`、全屏绿 `#27c93f`），取代原本单调乏味的单色灰点；
   - 配合深黑曜石终端窗口底色（`#090a0f`）与冷萃绿命令高亮，呈现地道精美的高品质 macOS 终端窗口质感。

5. **底部链接位置自然下沉 (`ui/workspace.css`)**：
   - 将 `.pub-foot` 的 `margin-top` 从 16px 扩展到 26px，`padding-top` 设为 16px；
   - 「下载 CLI · 使用方法 · 前往控制台」与上方终端核心代码区拉开清晰的呼吸距离，自然沉底舒展。

---

## 本机浏览器端到端全链路验收

通过 `node scripts/check-collections.mjs` 在真实 Chrome 环境中自动化执行：
- 成功打开发布弹窗并截取初始状态；
- 点击「复制」命令，验证 `scrollHeight <= clientHeight`（无滚动条产生）；
- 验证复制成功文案变为「已复制」，按钮呈现翡翠高透绿；
- 8 项核心链路检查全部通过，0 异常。

验收截图（归档于 `docs/spikes/img/`）：
- [发布弹窗 macOS 经典交通灯与舒展底部链接（桌面）](img/2026-09-13-collections-110022-publish-dialog-desktop.png)
- [发布弹窗点击复制后翡翠高透质感与无滚动条（桌面）](img/2026-09-13-collections-110022-publish-dialog-copied-desktop.png)
- [邮箱登录弹窗无多余「邮箱」小标题极简形态](img/2026-09-13-collections-110022-follow-login-dialog.png)

---

## 自动化测试核验

- `cargo test -p playtest-edge --lib html::tests`：全量通过（CSS 预算断言通过）；
- `cargo test -p playtest-edge --lib plaza::tests`：全量通过（14 项通过）；
- `cargo test -p playtest-edge --lib follow::tests`：全量通过（13 项通过）。
