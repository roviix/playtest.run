# 2026-09-13 合集与创作挑战系统重构打磨

## 动机与改进范围

依照 DESIGN §3.15、§3.3 与 §4.1，针对「合集与创作挑战」模块原有的视觉粗糙与结构混乱进行系统化重构与质感打磨：

1. **玩家端大厅 (`/collections`)**：
   - 彻底废弃原本强制 `rotate(±5deg)` 的生硬方块拼贴；采用层叠景深卡券画框（Stacked Deck），多作品与单作品均具备沉稳空间感，支持冷翡翠微缩几何背景。
   - 明确分类与状态心智：区分「开放投稿的挑战」（带翡翠脉冲微光圆点与截止提示）与「作者自选作品集」，作者与收录事实清晰展示。
2. **挑战与合集详情页 (`/c/<slug>`)**：
   - **信息架构重塑**：从上至下依次为「导航返回 → 标题与组织者 Hero 舞台 → 题目展台深色工作舱 → 统一行动控制栏 → 精选投稿作品墙」。
   - **题目展台舱（Prompt Canvas）**：将以往暴力占满首屏的原始 `<pre>` 改造为黑曜石工作舱。首屏仅呈现题目精华与单行一键复制（含平滑状态反馈），规则与参与指南以平行折叠抽屉呈现，让玩家一眼就能看到下方的精选投稿作品。
   - **作品网格与创作自述解耦**：移除破坏网格高度的原生 `<details>` 撑大行为，统一使用工整的 `[模型]` 与 `投稿 vN` 标签胶囊；展开自述后在抽屉内平齐呈现，不破坏卡片对齐。
   - **邀请函无缝切换**：作品邀请函 (`/p/<slug>?collection=<slug>`) 顶部优雅呈现 `.collection-context` 导航条，支持「← 返回合集」与「下一件 →」无缝轮转，保持门禁极简专注。
3. **控制台端 (`console/src/pages/collections.tsx` & `collections.css`)**：
   - 采用黑曜石控制台风格，引入分段控制器（Segmented Control）、高级悬停微光、作品关联提交面板与危险操作区分层。
4. **门禁与根域身份 Cookie 隔离修复**：
   - 修复了根域路由末尾多余的父域 Cookie 清理导致开发环境 Cookie 误删的隐患；将旧域清理精准限定在邮箱激活的 `confirm` 流程，彻底理顺了关注与回访闭环。

---

## 本机浏览器与全链路端到端验收

2026-09-13，通过 `node scripts/check-collections.mjs` 在 macOS 上启动真实 Chrome (Headless/DevTools Protocol) 与真实 API (`playtest-api`)、Edge (`playtest-edge`) 进程，执行 8 项端到端核验：

1. **真实数据库迁移与物料上传**：迁移 7 个版本，通过 API 提交并发布 6 件带有原创 SVG 动画的演示作品；
2. **合集聚合与基础搜索**：聚合挑战与普通作品集，检查标题、作者与作品计数；
3. **桌面与 390px 移动端布局**：视口在 1440px 桌面与 390px iPhone 规格下均无水平溢出（`scrollWidth <= innerWidth + 1`）；
4. **关闭 JavaScript 后搜索降级**：通过 CDP 禁用脚本，提交 `?q=晚风` 仍能准确过滤并返回 1 件作品，纯 HTML 表单降级完全成立；
5. **复制题目与状态反馈**：点击题目展台的「复制题目」按键，提示状态实时转为「已复制题目」；
6. **邀请函合集上下文与下一件切换**：从挑战卡片点入邀请函，正确显示「← 鹈鹕骑单车」与「下一件 →」上下文条；
7. **控制台真实投稿**：通过控制台挂载演示令牌，为挑战提交第 6 件作品，数据库即时持久化且作品列表增至 6 件；
8. **邮箱确认与根域关注状态管理闭环**：在合集页提交邮箱订阅，真实 SQLite 数据库生成激活通知；访问激活链接后返回 `/me`，合集卡片状态立即转为「✓ 已关注新投稿」。

验收截图（存储于 `docs/spikes/img/`）：

- [合集广场桌面](img/2026-09-13-collections-091843-plaza-desktop.png)
- [挑战详情页桌面](img/2026-09-13-collections-091843-challenge-desktop.png)
- [挑战详情页 390px 移动端](img/2026-09-13-collections-091843-challenge-mobile.png)
- [控制台投稿面板](img/2026-09-13-collections-091843-console-submit.png)
- [控制台 390px 移动端](img/2026-09-13-collections-091843-console-mobile.png)
- [邮箱确认后已关注状态](img/2026-09-13-collections-091843-subscribed-mobile.png)

---

## 自动化测试与红线指标

- **Rust 后端**：
  - `cargo test -p playtest-edge`：全量通过（190 项 lib 单元测试 + 82 项集成测试 = 272 项 100% 通过）。
  - `cargo test -p playtest-api`：全量通过（111 项测试 100% 通过）。
  - **CSS 字节预算（硬约束）**：
    - 卡页（`BASE + CARD`）：约 6.1 KB（红线上限 6.5 KB / 6656 字节）；
    - 整页（`BASE + PAGE`）：约 20.4 KB（红线上限 23 KB / 23552 字节）；
    - 所有内联 CSS 保持无注释、无冗余层级。
- **Console 前端**：
  - `pnpm --prefix console build`：TypeScript 0 报错，Vite 编译生产打包成功。

---

## 限制与说明

- 脚本中使用的「鹈鹕骑单车」等作品为验证架构与流程的原创本地演示素材，明确标注为演示数据，未虚构模型能力或评测名次；
- 邮箱通知在本机使用日志与 SQLite 存储方式验证，未向公网真实邮箱投递；
- 不破坏现有任何既有契约，确保纯 HTML 无脚本降级、零门槛浏览。

---

## 补充记录：挑战详情页首屏左右分屏一体化与多余顶栏精简

依照用户反馈针对 `/c/<slug>` 详情页「原垂直堆叠结构突兀割裂、右侧大面积荒漠空白、顶部多余返回条抢占首屏空间」进行深度打磨：

1. **移除冗余返回条**：侧边栏与移动端顶栏已高亮常驻「合集」入口，移除 `<nav class="collection-nav">`，首屏直入主题；
2. **左右分屏一体化 Hero 舞台 (`.collection-hero.split`)**：
   - 左侧主舱（`.hero-primary`）：状态胶囊、标题、说明、主行动按钮（`+ 我也来做一个 ↗`）及克制化次级微胶囊（`分享`、`关注新投稿`）；
   - 右侧工作舱（`.hero-stage`）：深色代码工作舱呈现场景题目与一键复制，下方内嵌折叠的规则说明抽屉，填平右侧原本 600px+ 荒漠空白；
   - 响应式断点（<=1000px）自适应自然折叠为上下单列。
3. **真实验收截图更新**：
   - [挑战详情页分屏桌面](img/2026-09-13-collections-100136-challenge-desktop.png)
   - [挑战详情页 390px 移动端](img/2026-09-13-collections-100136-challenge-mobile.png)
   - 验证通过：`check-collections.mjs` 8 项端到端全绿，0 异常；`cargo test -p playtest-edge` 272 项全部通过，CSS 体积保持在预算红线内。

---

## 补充记录：去除左右双仓，彻底合并为单一自洽 Hero 卡片

依照用户反馈「不要分左右两个仓，直接合并上下两部分」，进一步将 Hero 区域收敛为单一自洽的挑战总卡片：

1. **结构合并（Single Unified Hero Card）**：
   - 彻底废除 `.collection-hero.split`、`.hero-primary` 与 `.hero-stage` 的双仓分立，整合为单一 `<header class="collection-hero">`；
   - 顶部行 (`.collection-hero-head`)：左侧承载状态微标、标题与说明，右侧整齐排布主要行动（`+ 我也来做一个 ↗`）及克制微胶囊（`分享`、`关注新投稿`），左右视觉自然拉齐且无需分立两盒；
   - 嵌套题目工作舱 (`.challenge-brief`)：题目、复制按钮与折叠规则无缝嵌入卡片内部，作为挑战定义主体；
   - 底部反馈：就地呈现「题目已复制」等状态提示。
2. **真实验收截图**：
   - [挑战详情页合并单卡桌面](img/2026-09-13-collections-100759-challenge-desktop.png)
   - [挑战详情页合并单卡 390px 移动端](img/2026-09-13-collections-100759-challenge-mobile.png)
   - 验证通过：`cargo test -p playtest-edge` 272 项全部通过；`check-collections.mjs` 8 项端到端全绿。

---

## 补充记录：解除宽度枷锁，拉宽合集与关注页面舞台

依照用户反馈「为什么宽度不拉宽，还有关注菜单里也是？」，排查并彻底移除了历史遗留的僵硬宽度限制：

1. **合集与挑战页全宽自适应 (`ui/discovery.css`)**：
   - 移除了 `.discovery { max-width: 1480px; }` 与 `.collection-summary { max-width: 760px; }`，改为 `width: 100%` 全宽流式布局，与广场大厅一致自然铺满视口；
   - 搜索框自适应延伸至 `max-width: 720px`，合集网格采用 `repeat(auto-fill, minmax(min(100%, 320px), 1fr))` 充分利用宽屏横向空间。
2. **关注菜单/关注页解封 (`ui/follow.css`)**：
   - 彻底废除 `.main > .drawer { max-width: 760px; box-sizing: content-box; }` 的历史抽屉窄条设定，改为与主站一致的 `width: 100%; padding: 32px clamp(24px, 3vw, 48px) 64px;`；
   - 关注项目行（`.mine li`）与顶栏（`.follow-head`）自然铺满舞台，右侧「取消关注」与「通知设置」平齐呼应，消除右侧大面积死区。
3. **真实验收截图**：
   - [关注页宽屏桌面](img/2026-09-13-collections-101738-follow-desktop.png)
   - [挑战详情页宽屏桌面](img/2026-09-13-collections-101738-challenge-desktop.png)
   - [合集广场宽屏桌面](img/2026-09-13-collections-101738-plaza-desktop.png)
   - 验证通过：`cargo test -p playtest-edge` 272 项全部通过；`check-collections.mjs` 8 项端到端全绿。



