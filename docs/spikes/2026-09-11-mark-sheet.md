# 2026-09-11 标记改竖卡、发布说明收成一块

接着 `2026-09-11-narrow-rail-console.md`。用户看了上一稿的弹窗说还是乱；另外问字标要不要生成图片、栏上那个缩写 p 很丑。

## 字标：不做图片，不引字体

CSP `default-src 'none'`，没有 `font-src`；`img-src` 只给封面和 GitHub 头像。一张 PNG 字标要在 `playtest.run` 和 `playtest.roviix.com` 两张域、1x/2x/3x 各备一份，糊了还占请求，也不让 §1.2 任何一格更短。系统字体把字距收到 `-0.04em`，就是这一页的字标。桌面窄栏上只留给读屏，手机顶条和控制台栏上才显示。

## 标记：竖卡线稿，不是字母 p

38px 方块里塞一个系统字体的 p，再贴一个点，看起来像没做完的 favicon。改成邀请卡的缩影：圆角竖卡、两行字、右下角琥珀点。广场栏和控制台栏同一份 SVG（`plaza.rs` 的 `icon("mark")`，控制台 `app.tsx` 里照着画），不引图标字体。

## 发布说明：去掉 1 2 3

上一稿把安装、发布、之后排成三步，步骤 2 里再塞标签和命令——三种排版叠在一张面上，标签和命令各有各的左边。这一页的全部就是那一条命令。现在一张面里只有三样：三种情形的等宽三列标签、一条命令、脚一行（Releases、控制台）。标签和命令收在同一块近黑的面里，左右边齐。

## 看过的

Chrome 152 `--headless=new`。广场对着 `/tmp/pt-plaza-shot`（本机边缘 `:8455`）。控制台对着 mock `:8799` + vite `:5199`。控制台无报错。

- 桌面 `img/2026-09-11-mark-sheet-desktop.png`（1440×1000）：栏顶是竖卡，没有 p。
- 手机 `img/2026-09-11-mark-sheet-phone.png`（420×860）：标记加字标。
- 发布说明桌面 `img/2026-09-11-mark-sheet-publish.png`、手机 `img/2026-09-11-mark-sheet-publish-phone.png`。
- 控制台 `img/2026-09-11-mark-sheet-console.png`：同一枚标记。

## 测试

`cargo test -p playtest-edge --lib plaza::` 16 条。断言加上：页面里有 `mark-svg`、没有 `>p` 那个标记、没有 `class="steps"`、没有 `--seek`、脚在 `dialog-foot`。`pnpm exec tsc --noEmit` 通过。

## 没验的

- 竖卡在 32px 上两根线是否还能认出来；真机没看。
- 字标在 Windows / 安卓默认 UI 字体下的字距。
- 邀请卡 PNG 上的字标仍是栅格字，没动。
