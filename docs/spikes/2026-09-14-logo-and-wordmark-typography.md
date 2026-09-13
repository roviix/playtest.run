# 2026-09-14 准星 Logo 配色统一与 Wordmark 字标抗锯齿排版重构

## 背景与问题

用户就全站品牌视觉提出了两个关键问题：
1. **准星 Logo 配色是否统一**：准星是外框钛白、中间焦点为冷萃绿吗？（之前在不同页面各处颜色不一，广场侧全绿、控制台侧全白、邀请函侧全绿，缺乏统一的层级感）。
2. **Wordmark 字体粗糙毛糙**：为什么 `playtest.run` 字标看起来有很多锯齿、缺乏科技产品的精品质感？

## 根因定位：为什么之前的字体有严重锯齿？

审查 `ui/wordmark.svg` 的历史实现（commit `09f98f3`）：
- 上一版代码尝试不用外部字体文件，手工用一条极长的单一 SVG `<path>` 拼凑出小写字母的描边轮廓（`M3 28V10m0 6a6 6 0 1 1 6 6H3...`）；
- 更关键的是，为了让字形显得窄紧，外层套了 `transform="scale(.8 1)"`；
- **致命缺陷**：
  1. `scale(.8 1)` 将垂直方向的笔画宽度压缩为水平方向的 80%，横竖笔画不等宽；
  2. 描边线头（round cap）被水平挤压成椭圆扁头；
  3. 手写圆弧线段在贝塞尔拟合不精准的情况下，在现代高 DPI / Retina 屏幕的光栅化引擎（Skia / CoreGraphics）中无法应用标准的亚像素字形抗锯齿（Subpixel Anti-Aliasing），产生强烈的像素台阶与毛刺（即用户感受到的「很多锯齿、没有质感」）。

## 解决方案

### 1. 准星 Logo（Mark）配色统一与解耦

- **结构语义**：准星由「四个外框包角」与「中心瞄准焦点」构成。
- **色彩分工**：
  - 外框包角采用**钛白**（`Titanium White #f4f4f5` / `currentColor`），构成稳定的仪器结构感；
  - 中心圆点采用**冷萃绿**（`Cold Brew Green #75cdb5` / `var(--accent)`），作为瞄准锁定的点睛焦点；
- **全端同步**：
  - `ui/mark.svg`：内部 `<circle class="dot" cx="12" cy="12" r="2.2" fill="var(--accent,#75cdb5)" stroke="none"/>`；
  - 广场（`ui/workspace.css`）、邀请函（`ui/invitation.css`）、控制台（`console/src/app.tsx`）、门禁页（`edge/src/gate.rs`）全面对齐，统一钛白外框 + 冷绿准心。

### 2. Wordmark 矢量系统排版重构

- 严格遵守 `DESIGN.md` §1.2 准入判据（零多余网络请求、严格 CSP 无外部 `font-src`、无多倍率位图下载负担）：
  ```xml
  <svg class="wordmark" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 134 26" width="134" height="26" aria-hidden="true">
    <text x="0" y="19" font-family="-apple-system, BlinkMacSystemFont, 'SF Pro Display', 'Segoe UI', Roboto, sans-serif" font-size="18" font-weight="700" letter-spacing="-0.04em" fill="currentColor">
      playtest<tspan fill="#75cdb5" font-weight="600">.run</tspan>
    </text>
  </svg>
  ```
- **视觉韵律（Visual Rhyme）**：
  - `[ 钛白准星框架 + 冷萃绿焦点 ]` 与 `[ 钛白 playtest + 冷萃绿 .run ]` 形成绝佳的品牌色彩呼应。
- **质感跃迁**：
  - 直接调用操作系统原生最高级别的抗锯齿排版（macOS 上的 SF Pro Display、Windows 上的 Segoe UI）；
  - 字偶间距 `-0.04em` 紧凑而富有张力，彻底消除原有描边失真，笔画浑厚沉稳、边缘如手术刀般锐利。

## 真机验证

在 Chrome 152（`--headless=new`，真实 2x Retina 渲染）上进行真机抓取与多场景像素级比对：

- **广场导航栏**：`docs/spikes/img/2026-09-14-new-plaza-logo.png`
- **邀请函顶栏**：`docs/spikes/img/2026-09-14-new-invitation-logo.png`
- **前后质感与排版对比看板**：`docs/spikes/img/2026-09-14-logo-typography-comparison.png`

## 自动化测试

- `cargo test -p playtest-edge` 测试全数通过。
