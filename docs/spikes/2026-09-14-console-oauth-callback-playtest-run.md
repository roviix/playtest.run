# 2026-09-14 线上 GitHub 回调改到 playtest.run/console/

单主域之后，`https://playtest.run/v1/login/github/start` 仍把 `redirect_uri` 指到旧地址 `https://playtest.roviix.com/console/`。GitHub 授权页报 *The redirect_uri is not associated with this application.*

在 `playtest-hk` 的 `deploy/.env` 把 `PLAYTEST_CONSOLE_URL` 改成 `https://playtest.run/console/`（不入库）。`TARGET=none deploy/push.sh` 只重建 api。容器环境与公网 303 都是：

`redirect_uri=https://playtest.run/console/`

`deploy/.env.example` 同步成同一条。OAuth App 的 Authorization callback URL 须与此逐字相同。
