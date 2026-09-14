# 2026-09-14 广场副标单行舒展与文档标题消除绿框实机验证

## 背景

1. **广场副标折行问题**：
   - 副标文案为 `Deploy games and web builds in seconds. Real playtesters, zero friction, and versioned feedback for your next release.`（共 116 字符）。
   - 之前 `.hero-sub` 容器宽度设为 `max-width: 52rem`（832px），在桌面端标准视口下比 116 字符实际排版宽度（约 870px）略窄，导致最后一个单词 `release.` 被生硬挤到下一行。
2. **进入文档默认标题带绿框问题**：
   - 当点击导航中的「使用文档 / Documentation」或发布弹窗底部的「Documentation」链接进入 `/console/#/docs/start` 时，`locate("start")` 会自动执行 `document.getElementById("docs-title")?.focus({ preventScroll: true })`。
   - 全局 `:focus-visible` 样式此前定义为 `outline: 2px solid var(--accent);`（`#75cdb5` 薄荷绿）。
   - Chromium / WebKit 浏览器对于通过脚本聚焦的 `tabIndex={-1}` 标题元素触发了 `:focus-visible`，从而在顶头标题 `从做完可玩，到真的有人玩。` 外圈绘制出一个显眼的绿色聚焦方框，破坏了文档页面的质感。

## 方案与改动

1. **广场副标桌面单行舒展**：
   - 在 `ui/workspace.css` 中将 `.hero-sub` 的 `max-width` 由 `52rem` 扩展至 `64rem`（1024px），并补充 `text-wrap: pretty`。
   - 在桌面视口下，116 字符的副标自然平铺为单行，末尾单词 `release.` 不再出现悬垂折行；在小屏或手机窄屏下，`text-wrap: pretty` 确保排版平衡美观，避免单字孤行。
2. **彻底消除非交互标题外框与恢复纯钛白冷光环（DESIGN §3.10）**：
   - 遵循 `DESIGN.md §3.10` 硬约束（*「落到哪儿哪儿有一圈纯钛白冷光环（`:focus-visible`），鼠标点不出这一圈」*），将全局及发布弹窗中的 `:focus-visible` 由薄荷绿 `var(--accent)` 统一修正为冷钛纯白 `rgba(255, 255, 255, 0.45)`。
   - 在 `console/src/style.css`、`console/src/docs.css` 与 `edge/src/html.rs` 中为 `[tabindex="-1"]`、`h1`、`h2`、`h3` 以及 `#docs-title`、`.doc-section-head h2` 显式声明 `outline: none !important; box-shadow: none !important;`。
   - 保证无论是点击章节链接、默认锚点进入还是脚本定位，任何标题均无任何外框干扰。

## 验证

1. **前端构建与边缘测试**：
   - `pnpm --dir console build`：编译无报错，产物体积正常。
   - `cargo test -p playtest-edge`：全部 291 个边缘及集成测试均 100% 通过。
2. **真机与线上验证**：
   - 执行 `deploy/push.sh` 推送至生产环境并校验。
