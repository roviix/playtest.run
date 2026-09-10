# 2026-09-10 广场对照参考稿：橄榄夜 + 三列表面卡

对着用户贴来的那一页参考稿，把广场的壳和卡对过去：墙 `#101110`，栏 `#151614`，卡 `#1a1c19`，品牌绿 `#c5f277`。卡是带细边的表面，横窗约 3:2，玻璃标，认得出的引擎名，卡底一行看起来像「即刻试玩」。整页仍一行脚本都没有；「我的」仍是试玩者抽屉；控制台链接只出现在本页 `:target` 发布说明里。

参考稿里要脚本的预览开关和试玩弹窗没有做。

## 看过的

Chrome 152 `--headless=new`，`scripts/headless-check.mjs`。稠密态对着 `/tmp/pt-plaza-club/store/plaza.json`（6 张卡，1 张推广）。边缘起在 `127.0.0.1:8466`，`Host: localhost:8466`。控制台无报错。

- 桌面 `img/2026-09-10-plaza-club-desktop.png`（1440×1100）
- 手机 `img/2026-09-10-plaza-club-phone.png`（420×860）

## 测试

`cargo test -p playtest-edge --lib plaza::` 与 `--test social --test serving` 在表面卡那一版全绿。
