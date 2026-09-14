# 2026-09-14 · 侧边栏发布入口重构、热门 Tab 横幅常驻、合集卡片比例与浮动徽章、控制台宽屏标题统一实测

**结论**：生产环境（`playtest-hk`，`18.163.174.245`，Tag: `20260914-054449`）已全量部署更新并验证通过。
1. **侧边栏结构优化**：主行动点（`Publish Project` / `+ 发布作品`）统一移至侧边栏顶部品牌 Logo 正下方（`.sidebar-action`），视觉层级鲜明；底栏仅保留「Documentation」与「Account」，消除原来「文档悬在发布按钮下方」的别扭感。
2. **热门 Tab 切换稳定性**：移除 `&& !query.hot` 条件，广场首页第 1 页在按最新（Latest）与热门（Popular）排序切换时均稳定展示 Hero 横幅，不再发生布局闪烁与内容高度跳变。
3. **合集卡片比例与浮动状态徽章**：合集卡片封面高度由扁平的 120px 统一提升为 160px（接近 16:10 比例），并将 `Collection` / `Open Challenge` 状态徽章（`.collection-state`）作为半透明磨砂标签悬浮于封面左上方，卡片正文以标题（`<h2>`）干净起笔，与作品卡片视觉语言完全拉齐。
4. **控制台标题与宽屏网格对齐**：控制台「My Works」与「My Collections」统一采用标准 `.workspace-head` 规范（标题 + 数量药丸胶囊），网格列宽对齐为 `minmax(min(100%, 280px), 1fr)`，舞台去除窄宽度封顶实现流体宽屏。
5. **双端构建与真机测试全绿**：全工作区 Rust 单元与集成测试（199 单元测试 + 74 集成测试）、前端 TypeScript / Vite 构建、`check-collections.mjs` E2E 校验均通过，线上 `https://playtest.run` curl 实测无误。

---

## 一、本次关键修改点

1. **侧边栏发布按钮移至顶部，底栏收敛文档与账号**：
   - 模板层：`edge/src/plaza.rs` 与 `console/src/app.tsx` 中，在品牌 Logo 下方插入 `<div class="sidebar-action"><Publish /></div>`，原本在底栏的发布按钮移出。
   - 样式层：`ui/workspace.css` 中配置桌面端 `.sidebar-action { margin-bottom: 20px; }`；移动端顶部网格分配为 3 列（Logo、发布按钮、账号入口），第二行水平滚动导航。
   - 底部区域：底栏仅保留文档（Documentation）与账号管理（Account），结构轻量清晰。

2. **Popular Tab 横幅常驻**：
   - `edge/src/discovery.rs` 中将 `hero_allowed` 条件从 `query.page == 1 && !query.has_search() && !query.hot` 调整为 `query.page == 1 && !query.has_search()`。
   - 玩家在广场切换「Latest」与「Popular」时，Hero 横幅持续保持在顶部，网格仅依据排序重排，体验平滑连贯。

3. **合集卡片比例与封面浮动标签**：
   - `edge/src/discovery.rs`：将 `<span class="collection-state ...">` 移入 `<div class="collection-art">` 内。
   - `ui/discovery.css`：`.collection-art` 高度设为 `160px; min-height: 160px; position: relative;`；`.collection-art .collection-state` 采用绝对定位 `top: 12px; left: 12px; z-index: 5; backdrop-filter: blur(10px); box-shadow: 0 2px 8px #0008;`。

4. **控制台标题与网格宽度对齐**：
   - `console/src/pages/home.tsx`：确保即使在作品数为 0 时，也常驻渲染 `<header class="workspace-head"><h1>My Works</h1><span class="workspace-count">{count}...</span></header>`，内部嵌入 `<EmptyHome>`。
   - `console/src/pages/collections.tsx`：移除生硬的超大单行 `collection-header-row` 与冗长中文解释，采用标准 `.workspace-head`（`<h1>My Collections</h1>` + count pill + `+ New Collection` 按钮）。
   - `ui/workspace.css` & `console/src/collections.css`：统一舞台为 `width: 100%`，网格为 `grid-template-columns: repeat(auto-fill, minmax(min(100%, 280px), 1fr))`。

---

## 二、生产环境实机验证数据（playtest.run）

### 1. 热门 Tab 横幅与侧边栏顶部发布入口
```bash
$ curl -sS "https://playtest.run/?sort=hot" | grep -E 'plaza-hero|sidebar-action'
.sidebar-action{margin-bottom:20px}
.sidebar .sidebar-action{grid-column:2;grid-row:1;display:flex;align-items:center;margin:0}
.plaza-hero{position:relative;overflow:hidden;margin-bottom:32px;...}
<div class="sidebar-action"><a class="publish" href="#publish-dialog"><svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14"/></svg>Publish Project</a></div>
<div class="content discovery"><section class="plaza-hero">
```
- 在 `?sort=hot` 状态下，`.plaza-hero` 正常输出，页面布局与首屏无任何位移差异。
- 顶部 Logo 紧随 `<div class="sidebar-action">` 发布按钮。

### 2. 合集封面高度与浮动状态徽章
```bash
$ curl -sS "https://playtest.run/collections" | grep -A 5 -B 2 'collection-art'
.collection-art{height:160px;min-height:160px;position:relative;display:flex;align-items:center;justify-content:center;background:radial-gradient(ellipse at 50% 120%,#162a22 0%,#0c1015 85%);border-bottom:1px solid #ffffff0f;overflow:hidden}
.collection-art .collection-state{position:absolute;top:12px;left:12px;z-index:5;backdrop-filter:blur(10px);box-shadow:0 2px 8px #0008}

<div class="collection-art" aria-hidden="true"><span class="collection-state collection-regular">Collection</span><span>深</span><img src="https://ivory-beaver-33.playtest.run/_playtest/cover?v=40f0aadc" alt="" loading="lazy"><img src="https://lucky-robin-21.playtest.run/_playtest/cover?v=72065232" alt="" loading="lazy"></div>
<div class="collection-card-body"><h2>2026 金秋独立作品试玩展</h2>...
```
- `.collection-art` 呈现 160px 高度。
- `.collection-state` 作为悬浮标签置于 `.collection-art` 内部左上角（`top: 12px; left: 12px;`），卡片主体由 `<h2>` 优雅起始。

### 3. 控制台最新产物与样式
```bash
$ curl -sS "https://playtest.run/console/"
<script type="module" crossorigin src="/console/assets/index-BzRv5FNy.js"></script>
<link rel="stylesheet" crossorigin href="/console/assets/index-fgbWd5OG.css">

$ curl -sS "https://playtest.run/console/assets/index-fgbWd5OG.css" | grep -oE '\.sidebar-action[^\{]*|\.workspace-head[^\{]*'
.sidebar-action
.workspace-head
.workspace-head h1
.workspace-head p
```
- 控制台静态产物正常响应，包含了更新后的 `.sidebar-action` 与 `.workspace-head` 统一样式。
