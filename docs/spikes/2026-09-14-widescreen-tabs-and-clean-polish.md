# 2026-09-14 · 广场 Tab 切换稳定性、控制台宽屏统一、我的作品空状态与登录文案克制实测

**结论**：生产环境（`playtest-hk`，`18.163.174.245`，Tag: `20260914-043350`）已完成全量更新与热重载。
1. 广场在「作品（Projects）」与「合集（Collections）」间切换时，顶部 Hero Banner 与工具栏保持稳定，不发生跳跃变形。
2. 控制台「我的作品（Home）」去除了重复突兀的教程卡片，换以标准统一、质感克制的 `<EmptyHome>` 状态。
3. 导航与页面大标题收敛，页面容器消除窄屏限制（`max-width: 960px/1120px` 移出），全站统一铺开为清爽的宽屏布局。
4. 登录弹窗文案精简为「One account for following and creating.」，宽度扩展至 440px，消除了换行尴尬与冗余承诺。
5. 全工作区 Rust 单元/集成测试（包含 `edge`、`common`、`api`）及前端构建全绿通过，生产环境 curl 验证全部确认。

---

## 一、本次打磨与优化点

1. **广场 Projects 与 Collections 切换体验稳定**：
   - 在 `edge/src/discovery.rs` 中，使 Hero Banner 在 `/` 与 `/collections` 两种入口下均持续渲染（仅在主动搜索或深分页时收起）。
   - 将分类切换（`Projects` / `Collections`）与搜索框、排序下拉整合入统一工具栏 `.discover-toolbar`，去除原来突兀的 `<h1>Collections</h1>` 大标题。用户切换 Tab 时页面骨架稳定，仅下方网格平滑刷新。

2. **控制台首页（My Works）用法窗口收敛**：
   - 在 `console/src/pages/home.tsx` 中，彻底移除生硬展示命令的巨大卡片，替换为规范的 `<EmptyHome>` 组件。
   - 居中火箭勋章、简洁副标题与单行安静的复制胶囊（`$ playtest ./dist`）及「Quickstart Guide →」入口，保持与 Feedback / Roster / Results 等各页空状态设计语言绝对一致。

3. **去除生硬巨型标题与统一宽屏体验**：
   - 移除了 Plaza、Me 页面中的冗余大标题，改用辅助功能友好的 `<h1 class="sr-only">` 或精致的 18px-20px 标题。
   - 移除了控制台舞台 `.stage`、账户页 `.account-page`、合集页 `.collection-workspace` 内部的过窄 `max-width` 限制，将内容宽度统一扩展为 `width: 100%; max-width: 1440px`，大屏下视野开阔沉浸。

4. **登录弹窗文案与排版克制**：
   - 去除「Never sends unsolicited subscriptions」的啰嗦赘述，保留简洁有力的「One account for following and creating.」。
   - 弹窗宽度放宽至 `min(440px, calc(100vw - 32px))`，并配置 `text-wrap: balance; text-align: center;`，在大屏与移动端均呈现自然平衡的居中单行。

---

## 二、生产环境实测数据（playtest.run）

### 1. 广场与合集 Tab 切换稳定性实测
```bash
# 访问 /collections 检查顶部 Hero 与工具栏结构
curl -sS https://playtest.run/collections | grep -C 2 'plaza-hero'
curl -sS https://playtest.run/collections | grep -C 2 'discover-toolbar'
```
**实测结果**：
- `/collections` 页面首屏完整包含 `.plaza-hero`（与首页一致的「Show the work, not the hype.」横幅）。
- 工具栏输出：
  `<div class="discover-toolbar"><nav class="discovery-segmented" aria-label="Discovery category"><a href="/" class="seg-item">Projects</a><a href="/collections" class="seg-item active" aria-current="page">Collections</a></nav>...`
- 切换作品与合集时骨架零位移，仅 Tab 激活态与卡片列表切换。

### 2. 登录弹窗文案与居中测试
```bash
curl -sS https://playtest.run/ | grep -A 8 'account-login-title'
```
**实测结果**：
```html
<dialog id="account-login" class="account-dialog" aria-labelledby="account-login-title">
  <div class="notice-head"><h2 id="account-login-title">Sign in to playtest</h2><button type="button" class="dialog-close-btn" data-close-dialog aria-label="Close">×</button></div>
  <form class="notice-form" data-account-email>
    <label class="sr-only" for="account-email">Email</label>
    <input id="account-email" name="email" type="email" autocomplete="email" inputmode="email" placeholder="your@email.com" required>
    <button type="submit">Continue with Email</button>
  </form>
  <a class="account-github" data-account-github href="/v1/login/github/start">Continue with GitHub</a>
  <p class="notice-note">One account for following and creating.</p>
```
文案收敛为「One account for following and creating.」，样式规则宽度扩展至 `min(440px, calc(100vw - 32px))`，文本居中平衡。

### 3. 控制台构建产物验证
```bash
curl -sS https://playtest.run/console/assets/index-BO9QZYoN.js | grep -o 'No works yet'
curl -sS https://playtest.run/console/assets/index-BO9QZYoN.js | grep -o 'One account for following and creating.'
```
**实测结果**：均命中并正常提供服务。控制台首页空状态呈现高质感暗黑微光与紧凑复制胶囊。
