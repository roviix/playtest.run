# 2026-09-09 · GitHub 登录接上：设备码到 GitHub 那一步在香港真机走通，人点确认那一步待创始人

DESIGN §3.2「登录的形态」。这条 spike 记的是**机器能替人验的部分**；「一个真人在 GitHub 上输码、点确认、看到匿名作品归入账号」还没有发生，下面单列。

## 做了什么

- 控制面 `api/src/routes/login.rs`：`POST /v1/login/github/device`（要设备码）、`POST …/device/poll`（轮询）、`GET /v1/login/github/start`（303 到 GitHub 授权页）、`POST …/exchange`（授权码换令牌）、`GET /v1/me`。请求里带着匿名令牌的，成功后 `db::adopt_sites` 把那个身份下的作品换主人、去掉到期时间，并重写每一版清单里的 `expires_at` / `developer`，再重发广场。
- 配置只从环境变量来：`PLAYTEST_GITHUB_CLIENT_ID` / `PLAYTEST_GITHUB_CLIENT_SECRET` / `PLAYTEST_CONSOLE_URL`。三样都不在仓库里；服务器 `deploy/.env` 里有，`chmod 600`。
- CLI `playtest login`（`cli/src/login.rs`）；控制台 token 页多一个「用 GitHub 登录」，回来自动换令牌，顶栏显示 `@login`。

## 验了什么

**对着假 GitHub 的集成测试**（`api/tests/login.rs`，5 条）：设备码第一次轮询 pending、第二次成功；带匿名令牌来的作品被归入（库里 `expires_at` 变空、清单里 `developer` 变真名、匿名令牌再看列表是空的）；网页流程 state 对不上拒、用过一次再用拒；GitHub 回 `bad_verification_code` 是 400 `login_failed` 不是 500；没配 GitHub 时 501 `login_unavailable` 且匿名照常。CLI 侧 `cli/tests/json_output.rs` 对着假控制面走完设备码轮询，令牌落到配置里、没有到期时间。

**香港线上（标签 `20260909-134304`，真 GitHub）**：

```
$ curl -X POST https://playtest.roviix.com/v1/login/github/device -d '{}'
{"user_code":"F83F-01EF","verification_uri":"https://github.com/login/device","expires_in":899,"interval":5,…}

$ curl -o /dev/null -w '%{http_code} %{redirect_url}' https://playtest.roviix.com/v1/login/github/start
303 https://github.com/login/oauth/authorize?client_id=Ov23…&redirect_uri=https%3A%2F%2Fplaytest.roviix.com%2Fconsole%2F&state=…&scope=
```

GitHub 真的签出了设备码，说明 client_id 对、Device Flow 在 OAuth App 里勾了、香港到 GitHub 通。本机 `~/.cargo/bin/playtest login`（v0.2.0）打出了「在浏览器里打开 https://github.com/login/device / 输入这个码：882C-4977 / 等你在 GitHub 上确认……」并拉起了浏览器，然后被我 Ctrl-C 掉——我没有创始人的 GitHub 会话。

控制台登录页真机截图（无头 Chrome 1380 宽）：一个黑色「用 GitHub 登录」，下面「或者，把 CLI 给你的令牌粘在这里」。

## 没验、等一个真人

1. `playtest login` → 在 GitHub 输码、点 Authorize → 终端打出「已登录：@xxx」；`~/.config/playtest/config.json` 里 `login` 有值、`token_expires_at` 没了。
2. 登录前先 `playtest ./dist` 发一个匿名作品，登录后它的门禁页上「匿名开发者」变成真名、「这个链接在 … 后失效」那行消失；`playtest ls` 不再打「… 后失效」。
3. 控制台点「用 GitHub 登录」→ GitHub → 回到 `/console/?code=…&state=…` → 自动进作品列表，顶栏是 `@xxx`。GitHub OAuth App 里登记的回调地址必须是 `https://playtest.roviix.com/console/` 或它的父路径，否则 GitHub 会在授权页直接报 redirect_uri 不匹配。
4. 大陆网络下 github.com 打不开时的体验：终端那句话还在，人换网络或稍后再试即可；匿名链接不受影响——这是设计，但没在被墙的网络里亲眼看过。

这四条谁验了，请再加一条带日期的 spike，不改这条。

## 撞到的坑

- axum 的兜底中间层 `as_error_body` 把所有非 2xx 都改写成 JSON 错误体，303 跳转被它吞了 Location；放过 3xx 才行。
- reqwest 0.13 关掉默认特性后没有 `.form()`；GitHub 的 OAuth 端点接受 JSON 请求体，就用 JSON，不为表单编码多拖一个依赖。
- `#[tool_handler(version = "…")]` 只收字面量，CLI 升 0.2.0 时它没跟上，握手测试抓到了（这正是那条测试的用途）。
