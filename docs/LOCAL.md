# 本机把 playtest 跑起来，并走完一遍

给要在这台电脑上验收效果的人。产品定义仍是 [`DESIGN.md`](DESIGN.md)；本文只写「怎么开、点哪里、本机和线上差在哪」。

用 **Chrome 或 Firefox**。Safari 不认 `*.localhost`，作品链接会打不开。手机真机这一轮走不通——要等真域名。

## 0. 本机和线上差在哪

| | 本机 | 线上 |
|---|---|---|
| 玩家打开的地方 | `http://localhost:8443`（广场）、`http://<slug>.localhost:8443`（作品） | `https://playtest.run`、`https://<slug>.playtest.run` |
| 开发者控制台 | `http://127.0.0.1:5273` | `https://playtest.roviix.com/console/` |
| 控制面 | `http://127.0.0.1:8787` | `https://playtest.roviix.com` |
| 信 | 不真的寄。确认链接写在 `.data/api.sqlite` 的 `notifications` 表里 | 寄到邮箱 |
| GitHub 登录 | 本机控制面通常没配 OAuth，点了会失败 | `playtest login` 或控制台「用 GitHub 登录」 |
| TLS | 没有。地址是 `http://` | `https://` |

两套身份不要混：

- **开发者**：`~/.config/playtest/config.json` 里的令牌。发作品、看控制台。
- **玩家**：根域 cookie `pt_me`。只开「关注」那一页。没有密码，和 GitHub 没关系。

## 1. 先决

- 仓库在本机，Rust 工具链能编（`cargo`）。
- 控制台还要 Node：仓库根目录有 `console/`，用 `pnpm`。
- 装了 `sqlite3`（验收关注时用来取出确认链接）。没有的话用任何能读 SQLite 的工具也行。
- 端口空着：`8787`（控制面）、`8443`（边缘）、`5273`（控制台）。被别的程序占了先停掉，或看下面「端口被占」。

线上那份 CLI 配置已经备份在 `~/.config/playtest/config.roviix.json`。当前 `config.json` 应指向本机。看一眼（不要把令牌贴到任何地方）：

```bash
python3 -c "import json; print(json.load(open('$HOME/.config/playtest/config.json')).get('api'))"
```

应是 `http://127.0.0.1:8787`。若是 `https://playtest.roviix.com`，先备份再切：

```bash
cp ~/.config/playtest/config.json ~/.config/playtest/config.roviix.json
# 对 127.0.0.1:8787 跑一次下面「发出一个作品」，CLI 会把本机地址写进 config.json
```

## 2. 启动

三个终端，都在仓库根目录。先编一次：

```bash
cargo build -p playtest-api -p playtest-edge -p playtest
```

### 终端 A · 控制面

```bash
PLAYTEST_DATA_DIR=.data \
PLAYTEST_EMAIL_PROVIDER=log \
PLAYTEST_ADMIN_TOKEN=dev \
PLAYTEST_EDGE_INGEST_TOKEN=dev-edge-ingest-token-0000000000 \
PLAYTEST_SITE_URL_TEMPLATE='http://{slug}.localhost:8443' \
PLAYTEST_PUBLIC_ROOT_URL=http://localhost:8443 \
./target/debug/playtest-api
```

看到 `playtest 控制面已启动：http://127.0.0.1:8787` 再开下一个。

### 终端 B · 边缘

```bash
PLAYTEST_DATA_DIR=.data \
PLAYTEST_HOST_SUFFIX=localhost \
PLAYTEST_EDGE_LISTEN=127.0.0.1:8443 \
PLAYTEST_API_INTERNAL_URL=http://127.0.0.1:8787 \
PLAYTEST_EDGE_INGEST_TOKEN=dev-edge-ingest-token-0000000000 \
PLAYTEST_API_PUBLIC_URL=http://127.0.0.1:8787 \
./target/debug/playtest-edge
```

看到 `边缘在 http://127.0.0.1:8443 上`。

### 终端 C · 开发者控制台

```bash
cd console && pnpm install && pnpm dev
```

默认 `http://127.0.0.1:5273`，会把 `/v1` 转到本机控制面。

### 地址怎么写（容易踩）

玩家页**必须**用主机名 `localhost`，不要用 `127.0.0.1`：

- 对：`http://localhost:8443/`、`http://vite-vanilla.localhost:8443/`
- 错：`http://127.0.0.1:8443/` → 边缘不认，广场是 404

开发者控制台用 `127.0.0.1:5273` 即可。

## 3. 验收路线

按这个顺序走，刚好覆盖玩家看到的和开发者看到的。每一步括号里是「看起来对」的样子。

### 3.1 广场和发布说明

浏览器打开 [`http://localhost:8443/`](http://localhost:8443/)。

- 左栏：标记、**广场**（当前）、**关注**、栏底 **发布**。
- 点「发布」：标题「从一条命令开始」；面 32rem；一个沉下去的终端舱，顶栏胶囊槽三个词切下面命令和舱内词条；词条颜色跟命令同一套；底栏 Releases + 前往控制台。没有 1 2 3，没有「?」，没有「复制」按钮——点命令整条选中。
- 手机宽（DevTools 420px）：栏收成顶上一条，三个药丸还在。

### 3.2 发出一个作品

第三个终端（或新开一个），仍在仓库根：

```bash
./target/debug/playtest ./fixtures/vite-vanilla/export \
  --public \
  -n "验收用" \
  -m "想让人看什么" \
  --summary "本机走一遍用的最小页"
```

第一次对这个控制面跑，会自动拿一个 24 小时匿名令牌，写进 `~/.config/playtest/config.json`。终端里会打出：

- 玩家链接：`http://<slug>.localhost:8443/`
- 控制台地址
- 一张邀请卡 PNG（当前目录，不想要就加 `--card -`）

记下这个 slug。再看列表：

```bash
./target/debug/playtest ls
./target/debug/playtest whoami
```

回到广场刷新：墙上应有这张卡，标着公开或「正在找人测」（这次没写 `--seats`，所以只是普通公开）。

### 3.3 门禁页 → 玩

点卡，或直接打开终端里那条 `http://<slug>.localhost:8443/`。

- 先出门禁页：封面或字卡、作品名、那句 note、一颗琥珀「开始」。
- 点「开始」进作品。`vite-vanilla` 里有一个按钮会响一声——用来确认音频解锁。
- 再开同一个链接，或回广场再点这张卡：默认 `gate=once`，同一浏览器 24 小时内直接进游戏，门禁不再出现。

要订**这个作品**，在还看得见门禁的时候展开「有新版本时告诉我」。点过「开始」再想订：用无痕窗口打开同一条链接，或在开发者工具里删掉该子域的 `pt_gate`。`/me` 上那一下订的是广场，补不回这一步。

### 3.4 关注页（没钥匙）

打开 [`http://localhost:8443/me`](http://localhost:8443/me)。

- 栏上当前项是「关注」。
- 标题是「有新东西时告诉我」，不是「登录」。
- 一行说每周一会收到什么；一个邮箱框；按钮「告诉我」。
- 底下小字：先发确认信、开发者看不到邮箱。
- 能推送的浏览器会多一颗「用浏览器通知」。

填一个你自己的邮箱（比如 `you@example.com`），点「告诉我」。页面说确认信已发到打码地址。

**本机信没寄出去。** 取出链接：

```bash
sqlite3 .data/api.sqlite \
  "select body from notifications order by rowid desc limit 1;"
```

正文里有一条 `http://localhost:8443/me/confirm/...`。用**同一个浏览器**打开它。

- 应 跳到 `/me`，标题变成「关注」。
- 写着「这台设备连着 y***@…」。
- 一行「广场」+「取消」。底下没有作品——这一步订的是广场周报，不是某张卡。
- 一句「还没有关注作品」，链回广场。
- 底下「浏览器通知」「换一台设备」。没有反馈入口（反馈在游戏里，要作品接了 SDK）。

要订一个作品：打开它的门禁页（没点过「开始」的浏览器，或无痕窗口），展开「有新版本时告诉我」，填**同一个邮箱**，再点一封确认信。子域读不到根域的 `pt_me`，不是一键；已经确认过的邮箱也要再点一次。确认之后回到 `/me` 才多出这一项。已经点过「开始」的窗口再点卡会直接进游戏，看不到这一行。

### 3.5 开发者控制台

打开 [`http://127.0.0.1:5273/`](http://127.0.0.1:5273/)。

本机 GitHub 登录通常不可用。用粘贴令牌：

```bash
python3 -c "import json; print(json.load(open('$HOME/.config/playtest/config.json'))['token'])"
```

把打印出来的那一串贴进控制台（不要发到聊天、不要入库）。

- 作品墙应能看到刚才那个 slug。
- 点进去：时间线、点名册、反馈。先在作品页里玩一会儿、刷新几次，点名册里应出现会话。
- 控制台栏上的「广场」链到本机根域时，应是 `http://localhost:8443/`，不要跳到线上 `playtest.run`。

### 3.6 可选：找人测、邀请卡、第二版

```bash
# 标「正在找 N 位」，广场卡和门禁页会写出来
./target/debug/playtest ./fixtures/vite-vanilla/export --seats 3 -m "这一版请跳一下"

# 再拿一张邀请卡
./target/debug/playtest card <slug>

# 换一版之后，已关注的人会进通知队列（本机仍不寄信）
sqlite3 .data/api.sqlite "select kind, subject from notifications order by rowid desc limit 5;"
```

广场卡左上应出现「正在找人测」。门禁页应看得到名额。

### 3.7 可选：隧道（本地开发服务器）

另起一个静态或 Vite 服务，例如：

```bash
# 任意一个能在 5174 回 HTML 的都行
python3 -m http.server 5174 --directory fixtures/vite-vanilla/export
```

然后：

```bash
./target/debug/playtest 5174
```

这条命令会占着终端，直到 Ctrl-C。打开它打出的 `http://<slug>.localhost:8443/`，应能玩到 5174 上那个页。电脑休眠或这个命令停了，玩家看到的是离线页，不是空白。

带后端的那条（命令舱第三档）是 `playtest ./dist --backend <端口>`：目录里有的文件上传，没有的路径（`/api`、WebSocket）走隧道。fixture `fixtures/ws-rooms` 是给这条路用的。

## 4. 本机不要当成坏了的

- **`playtest login` / 控制台「用 GitHub 登录」**：没配 OAuth 就会失败。匿名 24 小时令牌足够验收上传、广场、门禁、关注、控制台读结果。
- **邮箱收不到信**：`PLAYTEST_EMAIL_PROVIDER=log`。链接在 `notifications` 表，不在收件箱。
- **`http://127.0.0.1:8443/` 是 404**：改用 `http://localhost:8443/`。
- **Safari 打不开 `*.localhost`**：换 Chrome。
- **匿名作品 24 小时后从广场下来**：正常。要长期作品得 GitHub 登录（线上做）。
- **微信里打开**：本机没有公网域名，不必验。
- **推广位**：本机可用 `ADMIN_TOKEN=dev scripts/admin.sh grant <slug> days3` 再 `review <id> approve` 送上；没做这一步，广场不应出现「推广」标。

## 5. 停掉、切回线上

三个终端 Ctrl-C。

CLI 改回打线上（令牌在备份里）：

```bash
cp ~/.config/playtest/config.roviix.json ~/.config/playtest/config.json
```

再切回本机：对 `127.0.0.1:8787` 再跑一次 `playtest ./fixtures/vite-vanilla/export`，或自己把 `config.json` 的 `api` 改回 `http://127.0.0.1:8787`（令牌必须是那台控制面签发的，两套不能混用）。

## 6. 端口被占

```bash
lsof -nP -iTCP:8787 -sTCP:LISTEN
lsof -nP -iTCP:8443 -sTCP:LISTEN
lsof -nP -iTCP:5273 -sTCP:LISTEN
```

`8787` 若是别的 App（本机出现过 Grok Bot 占 `[::1]:8787`），停掉它再起控制面。不要两个进程抢同一个口。

换口可以，但三个地方要一起改：控制面的 `PLAYTEST_API_LISTEN`、边缘的 `PLAYTEST_API_*`、控制台的 `PLAYTEST_API`、CLI 的 `--api` 或 `config.json` 里的 `api`。
