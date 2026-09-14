# 2026-09-14 全站浏览器标签页 Favicon 与品牌准星矢量图标落地

## 背景与问题

虽然网站内部（侧边栏、顶栏、邀请函）已统一使用「黑曜底座 + 钛白测试准星 + 中心冷萃绿聚焦核」的品牌 Mark，但**浏览器标签页（Browser Tab）中一直缺少 Favicon 图标**：
1. 页面模版（`edge/src/html.rs`）与控制台（`console/index.html`）的 `<head>` 均未声明 `<link rel="icon">`；
2. 浏览器自动发起的 `/favicon.ico` 与 `/favicon.svg` 请求在边缘路由器中命中 404；
3. 用户在 Chrome、Safari、Edge 等主流浏览器中打开 `playtest.run` 时，标签页仅显示默认的空页面 / 地球图标，缺乏品牌建立度与专业质感。

## 解决方案

### 1. 矢量 Favicon 设计（`ui/favicon.svg`）

- **构型**：32×32 viewBox 标准画板，基于 Playtest 官方 Mark 准星资产设计；
- **底座材质**：采用黑曜石圆角微徽章（Squircle `rx="7"`，底色 `#121316`，外覆 `0.5px` 细微浅高光边 `stroke="#ffffff" stroke-opacity="0.14"`）；
- **深浅标签栏通吃**：在浅灰/纯白标签栏（Light Mode）下黑曜底座对比度极高；在深黑标签栏（Dark Mode）下高光边与钛白准星确保边界极其清晰；
- **准星与冷萃绿焦点**：
  - 四边角钛白测试准星（`#f4f4f5`，`stroke-width="2"`，圆角线帽），微缩至 16×16 依然清晰可辨；
  - 中心悬浮冷萃绿聚焦核（`#75cdb5`，半径 `2.2`）。

### 2. 传统点阵 ICO 兜底（`ui/favicon.ico`）

- 使用系统级高精光栅化工具生成标准 Windows/Web 点阵 ICO 文件（`4.4 KB`），放置于 `ui/favicon.ico` 与 `console/public/favicon.ico`；
- 兼顾对 SVG Favicon 支持不全的旧版本爬虫、第三方书签及外部 RSS 抓取工具。

### 3. 全站 HTML 声明

在 `edge/src/html.rs`（覆盖整页广场、合集、作品邀请函、关注中心及错误页）与 `console/index.html`（控制台单页应用）的 `<head>` 中统一加入：
```html
<link rel="icon" type="image/svg+xml" href="/favicon.svg">
<link rel="alternate icon" href="/favicon.ico">
```
控制台由 Vite 在构建时自动补齐 `/console/` 基础路径前缀。

### 4. 边缘服务与根域路由挂载

- `playtest_common::follow::root_paths` 声明 `FAVICON_SVG = "/favicon.svg"` 与 `FAVICON_ICO = "/favicon.ico"`；
- `edge/src/router.rs` 根域路由表加入两个端点，允许 `GET` 与 `HEAD`；
- `edge/src/app.rs` 原生极速响应对应二进制 / 文本流，附带 `public, max-age=86400, immutable` 长效缓存；
- `DESIGN.md` §3.9 同步更新准入定义与路径规范。

## 自动化测试验证

- `edge/tests/serving.rs` 新增 `root_serves_favicon_svg_and_ico` 集成测试：
  - 验证 `GET /favicon.svg` 返回 200、`image/svg+xml`、正确冷萃绿特征码与缓存头；
  - 验证 `HEAD /favicon.svg` 返回 200、空响应体；
  - 验证 `GET /favicon.ico` 返回 200、`image/x-icon` 与非空点阵二进制；
  - 验证广场首页直出 HTML 包含上述两枚 link 标签。
- 执行 `cargo test -p playtest-edge --test serving`：33 passed，0 failed。
- 执行 `cargo test -p playtest-common`：95 passed，0 failed。
