# 2026-09-14 · 导航架构彻底对齐、概念统一（My Projects）与文档文案收敛实测

**结论**：生产环境（`playtest-hk`，`18.163.174.245`，Tag: `20260914-060342`）已全量部署更新并通过线上真实验证。
1. **文档底部冗余文案清理**：彻底移除了文档页底部的 `<span>playtest · Creator Guide</span>`，底部仅保留单项右对齐的「Back to top ↑」，文档目录底部返回链接规范为「Back to my projects ↗」。
2. **广场与控制台侧边栏 100% 镜像一致**：
   - 移除原广场侧边栏多余的公共「Collections」项（公共合集统一在广场内分段栏 `[ Projects | Collections ]` 切换），当访问 `/` 或 `/collections` 时，侧边栏一律稳定激活 `Plaza`。
   - 全站侧边栏（无论在广场、关注、控制台、文档、还是账户设置）严格统一步调与顺序：
     1. **Plaza**（发现：公开作品与公开合集）
     2. **Following**（关注：玩家关注流）
     3. **My Projects**（工作台：我的作品/项目列表）
     4. **My Collections**（工作台：我的合集/发起挑战管理）
   - 彻底解决了此前从广场跳控制台时菜单名称变异、位置错位、合集与我的合集概念打架的割裂感。
3. **概念用词统一（My Projects）**：
   - 将原控制台内部残余的「My Works」全面对齐为行业通用、与顶部「Publish Project」及广场 Tab「Projects」一致的「My Projects」。
   - 空状态文案同步升级为「No projects yet」，统一全站英文措辞。
4. **全套自动化测试全绿**：
   - 全工作区 Rust 单元测试（202 项）全部 PASS。
   - 集成测试 `edge/tests/social.rs` 22 项测试全部 PASS。
   - 前端 TypeScript / Vite 构建与 CLI 帮助文档校验全部 PASS。
   - `scripts/check-collections.mjs` 8 项全流程端到端自动化测试全部 PASS。
   - 生产线上 `https://playtest.run` 各路由实机 curl 校验无误。

---

## 一、本次关键修改与技术决策

1. **为什么侧边栏不该单独放公共 Collections？**
   - 广场页面内部已经有顶级的 `[ Projects | Collections ]` 发现切换栏。在侧边栏单独放一个 Collections，会导致一个页面存在两个平级的 Collections 入口；更糟的是，进入控制台后，侧边栏突然变成了「My Collections」，用户在空间心智上无法区分公共合集与自己的合集。
   - 将侧边栏的公共发现收敛为唯一的 `Plaza`，在访问 `/` 和 `/collections` 时均保持 `Plaza` 高亮，页面内分段栏平滑切换内容，符合现代 Web 产品的单一职责原则。

2. **为什么统一为 My Projects 而非 My Works？**
   - 按钮是 `Publish Project`；
   - 广场分类是 `Projects`；
   - 英文现代产品（GitHub, Vercel, itch.io）标准用词是 `Projects`，英文 `Works` 作为复数名词往往偏向古典著作或工厂，作为软件/交互作品时显得生涩；
   - 全量收敛为 `My Projects` 让用户无论在侧边栏、标题、按钮、空状态还是登录提示中，面对的都是完全一致的名词。

---

## 二、生产环境实机验证数据（playtest.run）

### 1. 广场与合集页面侧边栏一致性（无 Collections 赘余，激活态一致）
```bash
$ curl -sS https://playtest.run/ | grep -A 8 '<nav class="nav"'
<nav class="nav" aria-label="Navigation">
  <a class="nav-item active" href="/" aria-current="page">...Plaza<span class="nav-dot"></span></a>
  <a class="nav-item" href="/me">...Following</a>
  <a class="nav-item" href="/console/#/" data-manage>...My Projects</a>
  <a class="nav-item" href="/console/#/collections" data-manage>...My Collections</a>
</nav>

$ curl -sS https://playtest.run/collections | grep -A 8 '<nav class="nav"'
<nav class="nav" aria-label="Navigation">
  <a class="nav-item active" href="/" aria-current="page">...Plaza<span class="nav-dot"></span></a>
  <a class="nav-item" href="/me">...Following</a>
  <a class="nav-item" href="/console/#/" data-manage>...My Projects</a>
  <a class="nav-item" href="/console/#/collections" data-manage>...My Collections</a>
</nav>
```
- `/` 与 `/collections` 的侧边栏结构 100% 相同，Plaza 均保持激活，侧边栏不再变形。

### 2. 文档页文案与控制台最新产物
```bash
# 验证 Creator Guide 已被完全清除
$ curl -sS https://playtest.run/console/assets/index-7Ovjg82z.js | grep -o 'Creator Guide'
(无输出，已清除)

# 验证 My Projects 与 No projects yet
$ curl -sS https://playtest.run/console/assets/index-7Ovjg82z.js | grep -o 'My Projects' | head -3
My Projects
My Projects
My Projects

$ curl -sS https://playtest.run/console/assets/index-7Ovjg82z.js | grep -o 'No projects yet'
No projects yet
```
- 验证生产环境前端产物已完整生效。
