# 2026-09-13 主题色彩打磨：黑曜钛合金与冷萃极光绿 (Obsidian Titanium & Cold Emerald)

## 背景与问题

既有主题选用冷蓝暗底搭配长春花紫（`#b1adff`），在实际全屏视觉与移动端上色块偏平、缺乏质感深度与通透科技感，被判定廉价（「紫色色块太廉价」）。同时，发布按钮、字卡与卡片标签的质感需要进一步精细打磨，以体现单文件分发系统的专业与克制。

## 设计决策与落地

依据用户指定的黑曜钛合金与冷萃绿晶体棱镜质感规范，调整全栈视觉主题：

1. **底色与表面体系**：
   - 根底色由 `#0e111b` 转向深邃黑曜钛底 `--bg: #09090b`
   - 侧边导轨采用黑曜钛金栏 `--rail: #0c0c0e`
   - 卡片与面板采用黑曜石卡片面 `--card: #121215`，浮层面 `--card2: #18181c`
   - 边框采用微透白钛晶线 `--line: #ffffff14` (`rgba(255,255,255,0.08)`)，强化边 `--line2: #ffffff29` (`rgba(255,255,255,0.16)`)

2. **核心质感主按钮（`.publish` / `button`）**：
   - 钛合金垂直渐变：`linear-gradient(180deg, #18181c 0%, #101014 100%)`
   - 顶部冷萃绿晶体棱边：`border-top: 1px solid rgba(52, 211, 153, 0.65)` (`#34d399a6`)
   - 绿意微光投影：`box-shadow: 0 0 0 1px #10b98126, 0 8px 20px -4px #000c, 0 0 18px -2px #10b98138`
   - 状态微标与脉冲指示：左侧脉冲绿点（`#34d399`）、右侧等宽 `CLI` 胶囊徽标、翠绿加号图标（`#6ee7b7`）
   - 悬浮时微光弥散：`box-shadow: 0 0 0 1px #34d39973, 0 0 24px #10b98173`

3. **字卡（`.cover.word` / `.hero.word`）重塑**：
   - 采用 16px 极微弱点阵网格与冷萃极光绿弥散光融合：
     `background: radial-gradient(circle at 75% 20%, #10b98138 0, #05966914 45%, transparent 75%), radial-gradient(#ffffff1f 1px, transparent 1px) 0 0/16px 16px, #09090c`
   - 首字母 Monogram：冷白等宽超细体（`font: 200 clamp(44px,4vw,58px)/1 var(--mono)`），叠加翠绿冰透微光（`filter: drop-shadow(0 0 16px #10b9813d)`）
   - 角标 Slug 水印：等宽灰色字符（`#71717a`）
   - 状态标签（`.tile .tag`）：翠绿冷光毛玻璃胶囊（`background: #022c22a6; border: 1px solid #10b9814d; color: #34d399`），带微光小圆点

4. **终端说明舱（`.pub .cli`）**：
   - 全面抛弃浅紫浅底（`#eef0ff` / `#5644bb`），改为深色黑曜钛金终端（`#101014`）
   - 翠绿高亮命令动词与参数，保持专业开发者质感

5. **硬性指标验证（Byte Budget）**：
   - 严格满足 `edge/src/html.rs` 约束：
     - 卡页样式：`BASE + CARD = 2462 + 3649 = 6111 字节`（< 6144 字节，余 33 字节）
     - 整页样式：`BASE + PAGE = 2462 + 17922 = 20384 字节`（< 20480 字节，余 96 字节）
   - 全量测试通过：266 项自动化测试（含 avatar、discovery、social、gate、serving、tunnel 全部 pass）
   - 控制台前端编译通过：TypeScript / Vite production bundle 构建通过（0 警告）
