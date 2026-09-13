# 2026-09-13 邀请卡 4:3 凭证画框比例、物理撕票线与纯钛白主行动按钮打磨

## 本轮依据

针对用户体验对比与视觉反馈（早期概念原型与当前卡片比率、行动按键质感）：
1. **上下比例失衡问题**：此前的 20:9 字卡与 16:9 封面让整个卡片头身比偏向扁平条带，字卡被压成窄长横条，挤占了正文与行动区的纵向节奏，缺乏实体卡片的张力。
2. **凭证（Playtest Pass）物理隐喻弱化**：扁平圆角卡片退化为普通的 Web Modal，缺少了概念原型中令人眼前一亮的入场券、收藏票根（Pass）仪式感。
3. **主行动按键对比度不足（Dark on Dark 疲劳）**：此前的黑曜渐变暗色按钮（`#191b20`）叠加青绿细微渐变，在暗色卡片（`#121215`）背景上对比度过弱，容易被误认为不可点击的置灰禁用态，削弱了「进入作品」的第一意图。

产品定义先行更新至 `docs/DESIGN.md` §3.3、§3.10。

## 改动与实现

- **恢复 4:3 凭证画框比例与物理撕票打孔 (`edge/src/gate.rs`, `ui/invitation.css`)**：
  - `.hero` 统一采用 `aspect-ratio: 4/3`（400x300），头图不再被截断为压抑的窄条，画面与艺术图案具备舒展呼吸感。
  - 画框底边采用 `1px dashed var(--line2)` 撕票虚线，副券正文区两侧伪元素叠加 `16px` 半圆形内凹打孔缺口（`::before` 与 `::after` 融合背景底色），形成逼真可信的垂直撕票卡券（Playtest Pass）物理隐喻。
  - 无封面时生成同心星轨坐标与首字花押的轻量 SVG 艺术画框，冷萃绿/青色仅作为星轨聚焦点（`#67e8f9`）微量点睛。
- **重构主行动按钮为纯钛白高对比底面 (`ui/invitation.css`, `ui/dialog.css`)**：
  - `.start button` 与 `.notice-form button` 改用纯钛白底面（`#ffffff`）、墨黑正文字（`#09090b`，字重 600、15px），形成 18:1 的极致明暗对比，赋予按键最强的确定性与实体按压触感（Tactile Touch）。
  - 按压微动效：`transform: scale(.985) translateY(1px)`，具备轻微内阴影与自然光晕。
  - 冷萃绿（`#75cdb5`）严格退回「稀缺焦点色」（Accent by Scarcity），仅在准星焦点、雷达中心点、已关注状态等微型徽标上点睛，不再铺满大面积按钮。
- **CSS 尺寸预算与单元测试硬线守住 (`edge/src/html.rs`, `ui/invitation.css`)**：
  - 剔除无用规则与重复声明（如 `.start` 隐式单列网格，精简 SVG 属性），使卡页样式层 `BASE + CARD` 严格控制在 6656 字节预算之内（实测 6586 字节）。
  - 区分子域门禁 (`slug.playtest.run` 保留 `<button type="submit">开始</button>`) 与根域公开卡片 (`playtest.run/p/slug` 渲染 `开始{verb}`)，确保单元测试与规范完全吻合。

## 验证与测试

- **样式与体积硬线测试**：
  - `html::tests::each_page_carries_only_its_two_layers_and_they_stay_small`：通过（`card < 13 * 512`，`page < 23 * 1024`）。
  - `gate::tests::renders_required_pieces`：通过（整页渲染 HTML 保持在 16KB 以内）。
- **全工作区测试**：
  - `cargo test --workspace`：全库通过（190 个 `playtest-edge` 单元测试，3 个 `avatar` 测试，32 个 `serving` 测试，20 个 `social` 测试，8 个 `tunnel` 测试，4 个 `tunnel_flow` 测试，23 个 `upload_flow` 测试，5 个 `login` 测试，全部 Exit 0）。
- **DOM 与视觉结构校验**：
  - 邀请卡 4:3 视窗、撕票打孔槽、纯钛白主行动按钮与折叠关注原生弹窗，在桌面端与移动端模拟下渲染结构完整，无溢出与死链接。
