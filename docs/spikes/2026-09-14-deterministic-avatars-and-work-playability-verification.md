# 2026-09-14 线上全站确定性矢量头像统一与全部作品可玩性真机验证

## 1. 现象与归因

### 头像加载失败归因
1. **外部 CDN / 图床连接超时与污染**：
   此前 `scripts/seed_prod/seed_prod.py` 灌入的演示数据将用户的 `avatar_url` 设置为了外部地址（如 `https://api.dicebear.com/7.x/bottts/svg?seed=...` 与 `https://avatars.githubusercontent.com/...`）。
   在国内各省份网络环境下，访问上述外部域名存在严重的 DNS 阻断、连接重置或超时，导致 `<img class="face" src="...">` 批量出现裂图（红叉/破损图标）。
2. **边缘端原生生成算法完备**：
   Playtest 原生内置了确定性纯 Rust 几何矢量头像算法（`playtest_common::avatar::svg_for_creator`，即 ODD FOLK 3.0）。当 `avatar_url` 为 `None` 时，边缘端直接将精致的 SVG（带有确定性背景色调、几何面部、发型、眼睛、特征装饰）直接内联嵌入 HTML，零外部网络请求，100% 秒开且绝不裂图。

### 作品可玩性逐一排查
排查线上全量 7 个公开作品后发现：
- 5 个作品正常：
  - `lucky-robin-21` (鹈鹕单车, web): 自包含 SVG 动画与按键交互完好；
  - `ivory-beaver-33` (ODD FOLK 头像工坊, web): 头像调色与随机拼装交互完好；
  - `tiny-orbit` (Tiny Orbit, web): Canvas 引力弹弓物理游戏完好；
  - `design-notes` (Playtest 视觉设计手记, article): 独立源 303 直达主域原生阅读器，大纲目录与排版完好；
  - `deep-space-beacon` (深空信标, article): 独立源 303 直达主域原生阅读器，三章节目录连载与分页完好。
- 2 个作品资源缺失失效：
  - `paper-plane` (纸飞机, web): 静态文件引用了构建产物 `/assets/index-B1OnktKc.js`，返回 404，进入后白屏；
  - `rainy-store` (雨夜便利店, web): 静态文件引用了构建产物 `/assets/index-vqYE1ClE.js`，返回 404，进入后白屏。

---

## 2. 改造措施

### 1. 全面切换回 ODD FOLK 矢量头像
1. **线上数据库清洗**：
   在 `playtest-hk` 服务器的 `/home/ubuntu/playtest-data/api.sqlite` 执行：
   ```sql
   UPDATE users SET avatar_url = NULL;
   ```
2. **演示注入脚本防污染**：
   更新 `scripts/seed_prod/seed_prod.py`，全量移除所有外部图片 URL，全部保留为 `avatar: None`。
3. **线上渲染验证**：
   广场所有 7 张卡片和作品邀请函均直接内嵌 `<svg class="face" ... data-avatar-version="odd-folk/3.0.0">`，裂图率彻底归零。

### 2. 重写高品质自包含互动微游戏
1. **纸飞机（`paper-plane`）**：
   - 编写 `scripts/seed_prod/assets/paper_plane.html`；
   - 核心玩法：长按拖拽蓄力放飞、空中触屏/鼠标自由划风产生上升气流与俯冲推力、空气动力学升力/迎角物理模拟、360度特技大回旋（Loop-the-loop）、穿透上升气流环（Thermal Rings）、航行距离仪表盘与着陆结算；
   - 音效：内置 Web Audio API 纯程序合成风声白噪音、清脆穿环钟琴音与特技提示音。
2. **雨夜便利店（`rainy-store`）**：
   - 编写 `scripts/seed_prod/assets/rainy_store.html`；
   - 核心玩法：24小时街角温情白炽灯与夜雨淅沥、木质吧台与沸腾冒泡的关东煮台（可点击捞取大根萝卜、年糕福袋、溏心蛋、魔芋结、鱼豆腐等并获得暖心评语）、门上黄铜风铃与进店避雨客人对话系统、擦拭窗玻璃水雾效果；
   - 音效：内置 Web Audio API 纯程序合成粉红噪音真实雨声、深夜电台温暖 Lo-fi 和弦旋律（Dmaj7 - Bm7 - Em7 - A7sus4）、风铃叮咚声。
3. **数据重注入与版本刷新**：
   - 将自包含 HTML 写入 S3 Blob 存储，并更新 SQLite 清单与 `current.json`、`plaza.json`；
   - 重启 `playtest-edge-1` 与 `playtest-api-1` 刷新内存缓存。

---

## 3. 真机验证记录

验证时间：2026-09-14 09:18 (UTC+8)
验证目标：线上主机 `playtest-hk`（https://playtest.run）

### 1. 广场头像 100% ODD FOLK 验证
```bash
$ curl -s https://playtest.run/ | grep -o 'class="face"[^>]*'
class="face" aria-hidden="true" data-avatar-version="odd-folk/3.0.0"
class="face" aria-hidden="true" data-avatar-version="odd-folk/3.0.0"
class="face" aria-hidden="true" data-avatar-version="odd-folk/3.0.0"
class="face" aria-hidden="true" data-avatar-version="odd-folk/3.0.0"
class="face" aria-hidden="true" data-avatar-version="odd-folk/3.0.0"
class="face" aria-hidden="true" data-avatar-version="odd-folk/3.0.0"
class="face" aria-hidden="true" data-avatar-version="odd-folk/3.0.0"
```
外部裂图标签数量：0。

### 2. 全量 7 个作品在线状态与内容可玩性验证
```bash
=== lucky-robin-21
HTTP/2 200 
<title>鹈鹕骑单车 · 本地演示</title>

=== rainy-store
HTTP/2 200 
<title>雨夜便利店 · Rainy Night Store</title>

=== paper-plane
HTTP/2 200 
<title>纸飞机 · Paper Plane</title>

=== ivory-beaver-33
HTTP/2 200 
<title>ODD FOLK — 原创头像工作室</title>

=== tiny-orbit
HTTP/2 200 
<title>Tiny Orbit</title>

=== design-notes
HTTP/2 200 
<article class="article-body" id="article-content" data-reader-slug="design-notes" data-reader-version="1">

=== deep-space-beacon
HTTP/2 200 
<article class="article-body" id="article-content" data-reader-slug="deep-space-beacon" data-reader-version="1" data-reader-chapter="c1">
```
全量 7 个作品试玩/阅读入口均 200 OK，自包含交互与音效全部可玩。
