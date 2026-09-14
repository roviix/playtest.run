# 2026-09-14 · SEO / GEO 基础设施、广场 Hero 横幅与全站英文国际化上线实测

**结论**：生产环境（`playtest-hk`，`18.163.174.245`）已完成最新版本构建与无缝热重载（Tag: `20260914-032221` / `20260914-032605`）。SEO/GEO 协议（robots.txt、sitemap.xml、Schema.org JSON-LD、Canonical、OpenGraph / Twitter Cards）、广场 Hero 标语区以及控制台/边缘公共面的全量英文国际化均已上线并完成真实网络验证。

---

## 一、上线内容与技术架构

1. **SEO 与 GEO 结构化协议**：
   - **原生无依赖协议端点**：在 Edge 层实现纯 Rust 原生路由 `/robots.txt` 与动态 `/sitemap.xml`，自动收录根域、合集页与所有有效作品，携带真实 `<lastmod>`、`<changefreq>` 与 `<priority>`。
   - **AI 与搜索引擎爬虫策略**：`robots.txt` 明确声明针对通用搜索引擎及 AI 搜索模型（`GPTBot`、`PerplexityBot`、`ClaudeBot`）的开放路径，允许抓取公开作品门禁与合集，屏蔽私有状态接口与凭据路径。
   - **Schema.org JSON-LD 微数据**：根据作品形态（`game`、`app`、`article`、`video`）在门禁页动态注入 `WebApplication`、`VideoGame`、`Article`、`VideoObject` 结构化数据，具备严格的 XSS 实体转义保护。
   - **OpenGraph 与 Twitter Cards**：全站配置 `canonical` 绝对路径，规范化社交媒体分享卡片标题、描述与类型。

2. **产品定位与广场 Hero 标语区**：
   - **双语 Slogan**：
     - 中文：*拿作品说话，带观众走向下一版*
     - 英文：*Build in public. Show the work, not the hype.*
   - **黑曜石冷萃绿艺术级横幅**：在广场首屏顶部引入高质感视觉区域，包含 `pulse-dot` 呼吸律动圆点、放射状翡翠绿径向光晕、高对比度白底行动按钮与可点击复制的终端命令框（`playtest ./dist --public`）。

3. **海外/出海定位与全站英文国际化**：
   - **边缘玩家公共面（Edge）**：广场壁纸与过滤器、作品门禁页（Door/Gate）、合集展区（Collections）、关注抽屉与邮件确认（Follow/Me）、离线隧道提示页全面统一为地道英文。
   - **创作者控制台（Console）**：控制台外壳、作品列表、身份栏、点名册（Roster）、结果看板（Results）、反馈流（Feedback）、设置（Settings）、邀请卡（Card）、账号与开发者设置（Token）、以及六大完整章节文档（`#/docs/*`）全部完成英文化。
   - **严格单二进制与跨平台测试**：更新测试断言，保证 `cargo test --workspace`（199 单元测试 + 全部集成测试）以及 `node scripts/check-console-docs.mjs` 100% 绿灯。

---

## 二、生产环境（playtest.run）真实验证

### 1. `/robots.txt` 协议端点验证
```bash
curl -sS -i https://playtest.run/robots.txt
```
**实测结果**：
- 返回 `HTTP/2 200`，`content-type: text/plain; charset=utf-8`，`cache-control: public, max-age=3600`。
- 正确配置了通用爬虫规则以及针对 `GPTBot`、`PerplexityBot`、`ClaudeBot` 的抓取策略，指向 `Sitemap: https://playtest.run/sitemap.xml`。

### 2. `/sitemap.xml` 动态站点地图验证
```bash
curl -sS -i https://playtest.run/sitemap.xml
```
**实测结果**：
- 返回 `HTTP/2 200`，`content-type: application/xml; charset=utf-8`，`cache-control: public, max-age=1800`。
- 动态列出根域、合集（`/collections`、`/c/autumn-showcase-2026`、`/c/microgame-72h`）以及有效作品（如 `/p/paper-plane`、`/p/tiny-orbit` 等），均带有正确的 ISO-8601 `<lastmod>` 与权重。

### 3. 广场 Hero 标语与 HTML 验证
```bash
curl -sS https://playtest.run/ | grep -C 4 'hero-title'
```
**实测结果**：
- 渲染包含 `<section class="plaza-hero">`。
- 标语区输出：
  - 徽章：`Build in Public`
  - 标题：`Show the work, not the hype.`
  - 阐释：`One command to put your web app, game, tool, article, or video in front of real people. Zero friction, versioned feedback, and followers for what's next.`
  - 交互终端命令：`playtest ./dist --public`
- 页面 `<head>` 包含 `<link rel="canonical" href="https://playtest.run/">` 与 OpenGraph 元数据。

### 4. 作品门禁页 Schema.org JSON-LD 与社交标签验证
```bash
curl -sS https://playtest.run/p/tiny-orbit | grep -C 10 'application/ld+json'
```
**实测结果**：
- 正确输出 `<meta property="og:title" content="Tiny Orbit · v1">`、`<meta name="twitter:title" content="Tiny Orbit · v1">`、`<link rel="canonical" href="https://playtest.run/p/tiny-orbit">`。
- 正确注入 `<script type="application/ld+json">`：
  ```json
  {
    "@context": "https://schema.org",
    "@type": "VideoGame",
    "name": "Tiny Orbit",
    "url": "https://playtest.run/p/tiny-orbit",
    "version": "v1",
    "operatingSystem": "Web Browser",
    "applicationCategory": "Game"
  }
  ```

### 5. 创作者控制台（Console）英文部署验证
```bash
curl -sS -i https://playtest.run/console/
```
**实测结果**：
- 返回 `HTTP/2 200`，HTML 声明 `<html lang="en">`，页面标题 `<title>playtest Console</title>`。
- 静态资源正确引用最新产物打包哈希，各页面路由及文档章节正常运行。
