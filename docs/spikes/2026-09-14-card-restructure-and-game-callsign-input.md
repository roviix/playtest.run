# 门禁卡下半部结构重组与游戏化代号下划线设计记录

**日期**：2026-09-14  
**目标**：解决邀请卡下半部信息杂乱、表单感重、虚线撕裂以及底部按钮形态不统一的问题；遵从用户要求将留名输入框改为类似精致主机/独立游戏设定角色名（Callsign）的极致简洁下划线。

---

## 1. 现状问题剖析

1. **信息碎片化**：原版本号小标签（`v2 · 9 月 10 日`）与版本说明（`重点测试第 2 关...`）上下分离为两个独立方块，视觉上各说各话。
2. **中间断裂感**：`.stub` 带有横向虚线边框（`border-top: 1px dashed var(--line)`），将整张精致卡片硬生生切成上下两截。
3. **招募名额孤立**：名额行（`Zhongshang Wu在找 10 位试玩者...`）为一段悬浮的青绿文字，缺少状态归属感。
4. **留名表单感过重**：原留名框为标准的 Web 表单灰色深坑方框（带有四周边框与暗黑底色），给试玩玩家一种“需要填表注册”的心理负担。
5. **底部动作栏杂乱**：
   - 铃铛关注按钮原因 `.btn-follow span { display: none }` 变成了孤立的 34px 正方形，新用户看不懂；
   - 开发者的群为矩形文本按钮（padding 10px）；
   - 原声为带图标与气泡的矩形；
   - 分享为单独的图标正方形；
   - 举报为右侧裸文本。五种不同形态堆在底部，节奏零碎。

---

## 2. 改进方案

1. **版本聚焦微卡（Focus Microcard）**：
   - 保留精密单行版本戳记 `v2 · 9 月 10 日`。
   - 将版本说明重塑为带有冷萃绿晶体左棱线（`border-left: 3px solid var(--accent)`）与微渐变暗夜绿底色（`linear-gradient(90deg, #101916, #0d1214)`）的要点微卡，前缀统一为 `本版要点：`，使 build in public 的测试重心一目了然。
2. **招募状态仪表条（Quota Instrument Bar）**：
   - 封装为微型仪表盘容器，搭配冷萃绿呼吸状态指示灯（`box-shadow: 0 0 8px var(--accent)`），名额满额时自动转为沉静灰色。
3. **消除横截虚线**：
   - 移除 `.stub` 上的切断虚线，让卡片信息流自上而下顺畅滑入行动区。
4. **游戏化代号下划线（Game Callsign Underline）**：
   - 彻底摒弃封闭矩形框与底色，采用纯透明背景、零圆角、仅底部 1.5px 极细基准线。
   - 留名标签 `你的名字 · 可不填` 采用克制的 11.5px 浅灰微字，聚焦时平滑高亮。
   - 输入框获得焦点时底部线瞬间点亮为签名冷萃绿（`var(--accent)`），光标采用冷萃绿（`caret-color: var(--accent)`），极具游戏 HUD/终端代号输入的触感。
   - 针对 Chrome 自动填充做 `-webkit-autofill` 修复，保持纯黑透光。
5. **主行动焦点（High-Contrast CTA）**：
   - 纯钛白底墨黑字（18:1 高对比），附带轻微右箭头引导微动效 `→`，稳坐卡片第一视觉重心。
6. **标准化 32px 工具栏（Unified Action Chips）**：
   - 解除关注文本隐藏，呈现清晰的 `[🔔 关注更新]` 动作芯片。
   - 所有次级动作（关注、群、原声、分享）统一为 32px 高度、6px 圆角、细微边框与统一交互状态。
   - 左侧收拢互动群组，右侧对齐克制的 `举报` 链接，手机端自动按 11.5px 微调，单行完整容纳。

---

## 3. 真机验证（Headless Chrome via CDP）

通过真实 Headless Chrome（Chromium 128+）在两个标准视口下进行像素级验证：

- **移动端（390×844 iPhone 视口）**：
  - 截图：`docs/spikes/img/2026-09-14-card-restructure-mobile.png`
  - 验证点：卡片下半部结构舒展，四个工具芯片与右端“举报”在单行完美对齐无折行，版本微卡与招募指示条清晰分明。
- **代号输入聚焦态（390×844，输入 `Rookie 77`）**：
  - 截图：`docs/spikes/img/2026-09-14-card-input-focused.png`
  - 验证点：输入下划线与光标完整呈现冷萃绿（`rgb(117, 205, 181)`），文字干净悬浮于线上，完全没有传统网页表单的笨重感。
- **宽屏桌面端（1280×860 视口）**：
  - 截图：`docs/spikes/img/2026-09-14-card-restructure-desktop.png`
  - 验证点：卡片与右侧实时原声舱（Chat Panel）比例和谐，主操作与次级操作节奏统一。

---

## 4. 自动化测试结果

```
$ cargo test -p playtest-edge gate::tests
running 25 tests ... ok (25 passed; 0 failed)

$ cargo test -p playtest-edge html::tests
running 6 tests ... ok (6 passed; 0 failed)

$ cargo test -p playtest-edge --test gate_hardlines
running 6 tests ... ok (6 passed; 0 failed)
```
门禁页 HTML 总体积维持在 ~15 KB，卡页样式严格控制在预算范围内（< 14 KB）。
