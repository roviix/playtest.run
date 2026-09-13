# 2026-09-13 编辑级阅读与视听体验落地与免责声明清理

## 背景与诉求

根据玩家侧体验反馈：
1. 移除每页显式展示的噪音文本「阅读位置仅保存在此浏览器，不会同步或作为阅读统计。」，将「回到上次阅读位置」改为安静胶囊式交互（有保存记录时才静默出现，用户操作即隐去）。
2. 将文章、连载小说、视频作品的实际渲染体验与此前 ChatGPT 设计稿（`docs/spikes/2026-09-13-media-reading-watching-design.md`）对齐，抹去游戏大厅的强聊天室/票根残留，还原清爽、克制的编辑部旁注风格。

## 改进内容

- **免责声明清理**：彻底移除 `阅读位置仅保存在此浏览器，不会同步或作为阅读统计。` 冗余文案；仅在检测到本地有效阅读位置时显示安静的 `‹ 回到上次阅读位置` 浮层与清除按钮。
- **元信息单行化与格式化**：
  - 连载小说：`连载小说 · 共 N 章` 标签，主标题 + 章标题，单行 Byline（`{avatar} {developer} 连载中 · 共 N 章 · v{version} · {date}`）。
  - 文章：`文章 · 深度阅读` 标签，主标题，单行 Byline（`{avatar} {developer} 邀请你阅读 · v{version} · {date}`）。
  - 视频：`视频 · 实机演示` 标签，主标题，单行 Byline（`{avatar} {developer} 邀请你观看 · v{version} · {date}`）。
- **边栏反馈重塑（Editorial Feedback Margin）**：
  - 区分 Web 游戏联机大厅（`.chat-panel`）与文章/小说/视频的作者反馈栏（`.media-feedback-panel`）。
  - 置顶「作者想听」提示卡片（`.media-prompt-box`），明确向读者/观众征询具体意见。
  - 读者原声流采用克制的书卷引用块（`.editorial-quote-item`，`{name} · 读者/观众`，`「...」`），剥离游戏圆头像、对话气泡角与预设表情药丸。
  - 多行文本输入区（Textarea）配合高对比钛白「发送」按键。

## 本机检查

- 环境：macOS Chrome Headless (1280x900, 375x812)。
- 检查对象：
  - `/p/deep-space-beacon`（连载小说《深空信标》第一章）
  - `/p/design-notes`（文章《黑曜石与冷萃绿：Playtest 视觉设计手记》）
  - `/p/deep-sea-echo`（视频《深海回响：实机演示片段》）
- 截图记录：
  - [连载小说桌面端](img/2026-09-13-refined-novel-c1.png)
  - [连载小说移动端](img/2026-09-13-refined-novel-c1-mobile.png)
  - [单篇文章桌面端](img/2026-09-13-refined-article.png)
  - [实机视频桌面端](img/2026-09-13-refined-video.png)
- 单元测试与集成测试：`cargo test -p playtest-edge` 全部 195 个单元测试与 76 个集成测试通过（0 失败）。
