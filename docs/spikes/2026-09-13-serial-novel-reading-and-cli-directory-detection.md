# 2026-09-13 小说与连载作品阅读体验、章间导航与目录上传落地验证

## 1. 设计意图与判据回答（为什么不是 WorkKind::Dir）

在架构讨论中，我们澄清了核心问题：**「文件夹/目录（Directory）是物理存储载体，不是玩家体验形态。」**
- 网页游戏（Godot / Unity 导出物）也是一个包含多个文件的目录，但在玩家侧是一个交互应用；
- 连载小说也是一个包含多个 `.md` 章节文件的目录，但在玩家侧是一部单作品、多章节的文学连载体验。
- 引入 `WorkKind::Dir` 会模糊交付边界与消费形态，违反 DESIGN §1.2 准入判据。
- 因此，小说连载在契约模型中优雅收敛为：`WorkKind::Article` 附带 `chapters: Vec<ChapterEntry>` 清单。

## 2. 交付范围与落地实现

1. **Manifest 与 API 契约扩展**（`playtest-common`）：
   - 在 `Manifest` 与 `PrepareUploadRequest` 中新增 `chapters: Vec<ChapterEntry>`；
   - 增加 `is_serial()` 与 `find_chapter(id)` 工具方法；
   - 更新并同步生成控制台 TypeScript 契约（`console/src/generated/api.ts`）。
2. **CLI 零负担连载目录探测**（`playtest` CLI）：
   - `playtest run <novel-dir>`：当目录内包含 Markdown 章节且无 HTML 游戏入口与工程源码文件时，自然按文件名排序解析章节树（提取 Markdown `# 标题`）；
   - 保留对普通网页导出的严格防呆检查（`index.html` 在子目录时的 404 Blocker 正确报错拦截）。
3. **边缘端连载阅读与导航渲染**（`playtest-edge`）：
   - 支持 `?chapter=<id>` 动态查阅任意章节，默认加载第一章（或本地记录的上次阅读章节）；
   - 书头标题展示：书名眉题（`《书名》`）+ 章节主标题 + 连载章数标识；
   - 折叠抽屉式章节目录（`<details class="chapter-toc">`），高亮当前阅读章节，点击任意章节快速跳转；
   - 章末翻页导航（`<nav class="chapter-pagination">`）：`‹ 上一章`、`目录`、`下一章 ›`；
   - 读者反馈带章节上下文归属：反馈流与输入框自动关联 `[{chapter_id}]`，作者一目了然读者是在评哪一章。
4. **演示数据丰富**（`scripts/seed_demo_data.py`）：
   - 注入 3 章完整硬科幻小说演示《深空信标》（`deep-space-beacon`），并植入金秋独立展集。

## 3. 验证结果与实机截图

### 3.1 自动化测试
- `cargo test --workspace`：全工作区（common, api, edge, cli）**全部通过，0 失败**；
- CLI 上传流程测试：新增 `uploads_a_directory_of_markdown_chapters_as_a_serialized_article`，校验全套自然上传解析；
- CLI 检查回归测试：10/10 全部通过，包括子目录 index 拦截。

### 3.2 真实无头 Chrome 渲染截图

- **桌面端第一章首屏**：
  ![深空信标 第一章桌面端](img/novel-desktop-c1.png)
- **章节目录抽屉展开**：
  ![深空信标 章节目录展开](img/novel-desktop-toc-open.png)
- **第二章底部章间翻页导航**：
  ![深空信标 章末翻页](img/novel-desktop-c2-bottom.png)
- **移动端排版自适应**：
  ![深空信标 移动端排版](img/novel-mobile-c1.png)
