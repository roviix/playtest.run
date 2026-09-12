# 2026-09-11 发布说明：脚注折进「?」，标签改分段，命令一点全选

接着 `2026-09-11-mark-sheet.md`。用户看了上一稿说不酷、不够简洁，命令底下那一行「从 Releases 下载，放进 PATH。发出去之后，谁来玩过在开发者控制台看」该收到别处或一个「?」里。

## 为什么折起来

那两句只有第一次的人需要，之后每次打开都在命令底下读一遍，是噪音。现在是标题右边一个「?」（`<details>`，不用 JS），点开在命令底下出一小块：还没装从哪下、发出去之后去哪看。不点不占地方。DESIGN §3.9 那句「一张面里只有三样」改成：分段标签、一条命令、一个「?」。

## 命令是唯一的正文

标签从下划线页签改成分段控件（一条浅槽，选中的那格亮起来），三格等宽，收在近黑的面里。命令字号 12.5 → 13.5，颜色 `--soft` → `--fg`，`$` 琥珀。`user-select:all`：点一下整条选中，不放「复制」按钮也不用 JS。面左上角一点琥珀的光（`radial-gradient`），是这一页唯一的暖色。

## 样式搬了家

这一轮另一个会话把广场的样式从 `plaza.rs` 收进 `html.rs` 的 `PAGE` 层（`BASE + PAGE` 上限 14 KB）。新规则写在那里，选中态用 `input:checked+label`，比 `:has(#id:checked) label[for=id]` 三段短 200 字节；「?」和关闭按钮共用一条规则。改完 `BASE + PAGE` ≈ 14.1 KB。

## 看过的

Chrome 153 `--headless=new`，`scripts/headless-check.mjs`。对着本机三进程（api `:8787`、edge `:8443`、两条本地作品）。控制台无报错。

- 桌面 `img/2026-09-11-publish-tip-desktop.png`（1440×1000）：标题、「?」、关闭；三格分段；一条命令。
- 点开「?」 `img/2026-09-11-publish-tip-open.png`：两句折在命令底下。
- 手机 `img/2026-09-11-publish-tip-phone.png`（420×860）：底部抽屉，「?」在同一位置，命令换行。

## 测试

`cargo test -p playtest-edge --lib` 167 条。断言：页面里没有 `dialog-foot`，有 `<details class="tip">`，「Releases」在里面，开发者域仍只出现一次。

## 没验的

- `<details>` 在微信内置浏览器里的默认三角是否真的被 `::-webkit-details-marker` 藏掉。
- `user-select:all` 在 iOS Safari 长按时的行为。
