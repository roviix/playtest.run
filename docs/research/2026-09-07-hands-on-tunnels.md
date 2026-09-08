# 真机试用竞品隧道 CLI（2026-09-07）

在这台 Mac 上真的跑了一遍「分享 localhost」这一批工具，用 `fixtures/` 里的真实导出物当源站。目的不是评测，是回答一件事：**别人把「一条命令拿到链接」做到了什么程度，玩家那一侧看到的是什么，以及在「游戏」这个特殊负载上他们在哪里露怯。**

## 0. 这份记录能信到什么程度（先读这一节）

试验跑在 12:54–13:03 之间，会话在 13:03 因平台用量上限被中断，**原始终端输出随会话记录轮转丢失了**。这份报告因此分三档标注来源，不混着写：

| 标注 | 含义 |
| --- | --- |
| **【本轮硬证据】** | 这一轮（13:09 之后）在这台机器上重新取到的、可复现的东西：npm 调试日志里的 argv 与时间戳、终端文件元数据、fixture 文件体积、npm registry 元数据、官方文档原文 |
| **【上轮实测·输出已丢】** | 12:54–13:03 确实在这台机器上跑出来的结论，但逐字终端输出没能恢复。结论可信，**数字的精度不可信到小数点** |
| **【只读了文档】** | 没跑，只读了 README / 官网 / registry |

一个补充说明：npm 的 `_logs` 只保留最近若干份，本轮为取 registry 元数据跑的 `npm view` 把大部分旧日志挤掉了。下面引的日志行是在被挤掉之前抄出来的，`2026-09-07T05_01_05_132Z-debug-0.log`（LocalhostVibe 那份）是唯一还在盘上的原件。

清理与环境记录（本轮）：`http.server` / pinggy / uplink-cli / kshare / localtunnel / cloudflared / untun 全部已停，`lsof -i :8101 -i :8102` 为空，`~/.npm/_npx` 已删。磁盘 `/System/Volumes/Data` 228Gi 总量、可用 18–20Gi（91%）。用户自己那条 `ssh ... ws2-hhd` 不在 pkill 的任何一个模式里，没有被碰。

## 1. 试验台

两个本地静态站点，都用 `python3 -m http.server` 当源站【本轮硬证据·终端文件元数据】：

**`:8101` = `fixtures/phaser-jump/export`** —— 真实体量的可玩游戏。

```
index.html                     843 B
assets/index-vqYE1ClE.js   1,200,411 B   ← 真正的东西在这里
```

入口 HTML 只有 843 字节，1.15 MiB 全在那个 JS 里。这个形状很重要：**任何在子资源上出岔子的中间层，都会让一个「首页能打开」的游戏白屏。**

**`:8102` = `fixtures/headers-lab/export`** —— 响应头考题。

```
index.html            9,110 B
data.bin          1,048,576 B    ← Range / 206 用
mod.wasm                 41 B    ← application/wasm 用
mod.wasm.br              42 B
mod.wasm.gz              61 B
Build/hello.wasm.br      42 B    ← 模拟 Unity 目录布局
sub/index.html          170 B    ← /sub → /sub/ 重定向用
```

**这里有一个必须说清的局限**：源站是 `python3 -m http.server`，它自己就不发 `application/wasm`、不认 `.br`、不加 COOP/COEP。所以这一轮测的是「隧道会不会**改**响应头」，**不是**「隧道会不会**补**响应头」。后者的答案不用测也知道：一个都不会。这一条在 §5(d) 展开，它比看上去重要。

机器环境【本轮硬证据·npm 日志尾部】：`Darwin 25.2.0` / `node v22.22.0` / `npm v10.9.4`。

## 2. 对比表

| 工具 | 怎么起 | 账号 | 本机要装什么 | 访客侧中间页 | 这一轮的结果 | 证据档次 |
| --- | --- | --- | --- | --- | --- | --- |
| **localhost.run** | `ssh -R 80:localhost:8101 localhost.run` | 不要 | 无（系统 ssh） | 官方文档未提 | 通了 | 上轮实测·输出已丢 |
| **pinggy** | `npx pinggy@0.5.6 -l 8102`（也支持 ssh 形态） | 不要（免费档） | 1.03 MB 包 | **有，15 KB，每个没带确认 cookie 的路径都回 200** | 通了，吞吐 ~740 KB/s | 上轮实测 + 本轮文档 |
| **cloudflared 快速隧道**（经 untun） | `npx untun@0.2.2 tunnel http://localhost:8101` | 不要 | untun 20 KB + 本机已有 cloudflared | **没有** | **untun 首跑失败（包内 bin 缺 shebang）**，绕过后通了 | 上轮实测·输出已丢 |
| **localtunnel** | `npx localtunnel --port 8101` | 不要 | 24.5 KB 包 | 未复原 | 通了 | 上轮实测·输出已丢 |
| **uplink** | `npx uplink-cli tunnel create --port 8102 --json` | 访客隧道不要 | 595 KB 包 | 未验证 | **起来了，访客侧未完成验证**（被中断） | 本轮日志 + 上轮部分 |
| **bore** | `npx bore-cli ...` | 不要 | **10.06 MB 包** | 未到这一步 | **两次都失败** | 本轮日志（argv 逐字） |
| **serveo** | `ssh -R 80:localhost:8101 serveo.net` | 不要 | 无 | 未复原 | **结论随输出一起丢了** | — |
| **kshare** | `npx @sifxprime/kshare --port 8101` | 不要 | 42 KB 包 | README 未提，有 `--password` | 起来了，**输出未拿到**（被中断） | 本轮日志 + README |
| **LocalhostVibe** | `npx localhostvibe share 8101` | 不要 | **实际拉了 130 个包** | 无（但会插一层本地代理面板） | `npm exec` 退出码 0 | 本轮日志（硬） |
| **tinyfi.sh** | `ssh -R 80:localhost:PORT tinyfi.sh` | 不要 | 无 | 官网未提 | 没跑 | 只读了文档 |
| **tunr** | `brew install` 或 `npx tunr@latest`，单静态 Go 二进制 | 免费档不要 | 单二进制 | 无（但有 `--inject-widget` 主动注入） | 没跑 | 只读了文档 |

npm 包元数据都是本轮现取的【本轮硬证据】：

| 包 | 版本 | 最后发布 | 解包体积 |
| --- | --- | --- | --- |
| `bore-cli` | 0.1.11 | 2026-02-04 | 10,062,829 B |
| `pinggy` | 0.5.6 | 2026-08-24 | 1,028,941 B |
| `uplink-cli` | 0.2.13 | **2026-09-02（五天前）** | 595,307 B |
| `localhostvibe` | 1.0.0 | 2026-06-26 | 46,107 B |
| `@sifxprime/kshare` | 1.0.3 | 2026-05-18 | 42,270 B |
| `localtunnel` | 2.0.2 | **2023-11-07（近三年没动）** | 24,531 B |
| `untun` | 0.2.2 | 2026-07-20 | 20,256 B |

## 3. 一个一个说

### 3.1 localhost.run —— 零安装这一条太强

```bash
ssh -R 80:localhost:8101 localhost.run
```

官方文档【本轮硬证据】把卖点说得很清楚：「All major operating systems already have SSH installed, and localhost.run uses SSH as a client, so no download is necessary to use the service and no account setup is needed for free domains.」

上一轮通了【上轮实测·输出已丢】。中间页行为没能复原，官方文档也没提这件事。

对我们的意义：**「不用装东西」是这一批里唯一能压过「一条命令」的东西。** DESIGN §4.3 已经淘汰了 `ssh -R`（依赖系统 ssh、每用户占端口、按 Host 复用做不了、计量做不了），这个取舍我认为仍然对——但代价是我们必须让「装一个二进制」这件事快到让人感觉不到，见 §5(e)。

### 3.2 pinggy —— 这一批里最像「警告页」的一个

本轮从 npm 日志恢复的两次调用【本轮硬证据】：

```
7 verbose title npm exec pinggy@0.5.6 8102
8 verbose argv "exec" "--yes" "--" "pinggy@0.5.6" "8102"          ← 12:54:52

7 verbose title npm exec pinggy@0.5.6 -l 8102
8 verbose argv "exec" "--yes" "--" "pinggy@0.5.6" "-l" "8102"     ← 12:55:41
```

两次调用隔了 49 秒，第二次加了 `-l`。合理推断（不是硬证据）是第一种写法不对，翻了帮助才改对的。

**中间页**是这个工具最值得记的地方【上轮实测】：一个 15 KB 的页面，**每一个路径都返回 200**，不管你请求的是 `/` 还是 `/assets/index-vqYE1ClE.js`。绕过办法【本轮硬证据·官方文档】：

```bash
curl -H "X-Pinggy-No-Screen: 1" https://xxx.run.pinggy-free.link/
curl -A "my-app/1.0" https://xxx.run.pinggy-free.link/      # 任意非标准 UA 也行
```

官方文档 <https://pinggy.io/docs/http_tunnels/screening/> 的说法是：「A visitor sees it only once per browser」「The screening page is served only to browsers. Requests from curl, wget, HTTP client libraries... are never screened」。

**我们量到的和官方说的不矛盾，但含义完全不同。** 官方描述的是「真浏览器带着确认 cookie 的稳态」；我们量到的是「没有 cookie 时的每一次请求」。差别在于它靠 **User-Agent 猜你是不是浏览器 + cookie 记住你确认过**。任何不带 cookie 又长得像浏览器的请求——Service Worker 里的 fetch、`credentials: 'omit'`、跨源子资源、预加载——都会拿到那 15 KB HTML，而且状态码是 200。§5(d) 展开为什么 200 比 404 更坏。

**吞吐 ~740 KB/s**【上轮实测】。这个数字换算成 5.9 Mbps，远低于家宽上行的常见值，说明瓶颈在 pinggy 免费档那边，不在这台机器的上行。

### 3.3 cloudflared 快速隧道（经 untun）—— 隧道最干净，壳最糙

`untun` 是 unjs 出的一个 20 KB 薄包装，registry 描述【本轮硬证据】写得很直白：「Tunnel your local HTTP(s) server to the world! Powered by Cloudflare Quick Tunnels.」真正干活的是 cloudflared，本机装在 `/opt/homebrew/bin/cloudflared`（本轮确认文件在，版本号没记到，见 §4）。

**首跑失败：untun 包里的可执行文件缺 shebang**【上轮实测·输出已丢】。绕过之后隧道本身正常。

这件事本身是个信号：**「给一个好隧道套一个更好用的壳」这个位置有人占了，但占得很糙。** 我们要做的正是这一层，这里没有护城河，只有做得好不好。

`trycloudflare.com` 的快速隧道**没有中间页**，也不改响应内容。它是这一批里访客侧最干净的一个。代价写在 DESIGN §0.5 和 §2.1 里了：骑 Cloudflare 的东西在大陆都不好用。

### 3.4 localtunnel —— 还能跑，但已经三年没人管

`localtunnel@2.0.2`，最后发布 **2023-11-07**【本轮硬证据】。它是这一批里唯一一个「老工具」，解包体积也最小（24.5 KB）。上一轮跑通了【上轮实测·输出已丢】，访客侧行为没能复原。

值得记的是它的角色变了：LocalhostVibe 把它当成两个后端之一（另一个是 Cloudflare）。**localtunnel 现在更像基础设施而不是产品。**

### 3.5 uplink —— 五天前还在发版，包描述直接写给 AI agent 看

本轮日志【本轮硬证据】：

```
verbose title npm exec uplink-cli tunnel create --port 8102 --json    ← 12:58:30
```

registry 描述【本轮硬证据】值得整句抄下来：

> Agent based web management — share localhost, host apps, and attach domains from the terminal. **JSON-first CLI for Cursor, Claude, Codex, and Windsurf.**

`uplink-cli@0.2.13` 发布于 **2026-09-02**，是这一批里最活跃的。它连命令形态（`tunnel create --json`）都是给程序调的，不是给人敲的。

**访客侧没验证完**——被中断了，没拿到 URL 和 curl 结果。

但有一条侧面证据【本轮硬证据·终端文件 `831538.txt`】。8102 那个 `http.server` 在被杀之前的访问日志是：

```
::ffff:127.0.0.1 - - [07/Sep/2026 13:06:43] "HEAD / HTTP/1.1" 200 -
::ffff:127.0.0.1 - - [07/Sep/2026 13:07:13] "HEAD / HTTP/1.1" 200 -
::ffff:127.0.0.1 - - [07/Sep/2026 13:07:43] "HEAD / HTTP/1.1" 200 -
```

整齐的 30 秒一次 `HEAD /`，是隧道客户端的保活探测。8102 上当时唯一还活着的隧道客户端是 uplink（kshare 在 8101），所以几乎可以肯定是它——但我没有把这个进程和这些请求直接绑起来的证据，标为推断。

对我们的意义：**保活探测要发到开发者的 dev server 上，这件事要想清楚。** 每 30 秒一个 `HEAD /` 会污染开发者自己的日志，也会在 Vite 这类工具里触发不必要的处理。我们的隧道保活应该在 WSS 控制通道上做，不要打到 `127.0.0.1:<port>`。

### 3.6 bore —— 两次都失败，而且 npm 上的 `bore-cli` 不是那个 bore

本轮恢复的两次 argv【本轮硬证据】：

```
verbose title npm exec bore-cli local 8101 --to bore.pub                ← 12:54:42
verbose title npm exec bore-cli --help                                  ← 12:55:03
verbose title npm exec bore-cli -u http://localhost:8101 -inspect=false ← 12:55:41
```

中间那次 `--help` 说明第一次没成，去翻了帮助。

第一次用的是 `bore local <port> --to bore.pub`，这是 Rust 那个 `ekzhang/bore` 的语法。第二次变成 `-u ... -inspect=false`，是另一种风格。npm 上 `bore-cli` 的 homepage 字段【本轮硬证据】是 `https://trybore.com`——**跟 Rust 那个 bore 不是一个东西，同名不同物。** 照 GitHub 上 bore 的 README 抄命令，装到的是另一个包，然后失败。

而且它解包 **10.06 MB**，是这次试的所有 npm 工具里最大的，比第二名 pinggy 大十倍。

两条给我们的教训：**命名冲突会直接吃掉用户的前两分钟**；**npm 分发的隧道 CLI 体积可以离谱到 10 MB**。

### 3.7 serveo —— 结论丢了

上一轮跑过，结论随原始输出一起丢了。不编。需要重跑。

### 3.8 kshare —— 「30 秒」写在 README 第一屏，以及一个会坏游戏的设计

本轮日志【本轮硬证据】：`npm exec @sifxprime/kshare --port 8101`（12:58:30）。输出没拿到，被中断了。

README <https://github.com/sifxprime/kshare>【本轮重读】有三处值得记：

**第一，它自己承认 30 秒。**

> Run one command: `npx @sifxprime/kshare --port 3000`
> **Thirty seconds later**, anyone on earth can open your app

把 30 秒当卖点写在第一屏，说明这一档产品的心理预期就是几十秒，不是几秒。

**第二，它有我们打算做的两个东西。** 终端二维码（`--qr`）、口令保护（`--password`）、24 小时过期、`kshare status` / `kshare stop`、本地 4040 的请求面板。README 展示的输出长这样：

```
  KShare  by KODELYTH

  Connected

  Public URL   https://ab12x.kodelyth.net
  Dashboard    http://localhost:4040
  Expires      23h 59m
  Local        localhost:3000
```

**第三，也是最要命的一条**：

> it **rewrites URLs in your app's HTML automatically**
> URL rewriting happens on **raw text** — no DOM manipulation, no scripts

对 HTML / CSS / JS 做原始文本 URL 替换。这对 CRUD 网页没问题，对游戏是灾难：Phaser 那个 1.2 MB 的 bundle、Unity 的 `.wasm.br`、Godot 的 `.pck`，只要被当文本处理一次就坏。§5(d) 展开。

规模上它是一个人的 VPS：README 自己写自托管「a $4/month Hetzner or DigitalOcean box works」，服务端默认 `MAX_TUNNELS_PER_IP=5`、`RATE_LIMIT_MAX_REQUESTS=200`。免费档跑在 `kodelyth.net` 上。

### 3.9 LocalhostVibe —— 一句「一条命令」背后是 130 个 npm 包

这是本轮唯一还在盘上的完整 npm 日志（`2026-09-07T05_01_05_132Z-debug-0.log`，107 KB）【本轮硬证据】：

```
7 verbose title npm exec localhostvibe share 8101
8 verbose argv "exec" "--yes" "--" "localhostvibe" "share" "8101"
...
silly placeDep ROOT http-errors@2.0.1 OK for: express@4.22.2 want: ~2.0.0
...
silly audit report  title: 'Axios: Nested axios option objects can consume polluted prototype values',
silly audit report  severity: 'moderate',
silly audit report  url: 'https://github.com/advisories/GHSA-7q8q-rj6j-mhjq',
...
verbose node v22.22.0
verbose npm  v10.9.4
verbose exit 0
```

`silly placeDep ROOT` 出现 **130 次**——一条 `npx localhostvibe share 8101` 往临时目录里装了 130 个包，其中有完整的 `express@4.22.2` 和 `axios`（npm audit 顺手报了一条 moderate 的 axios 原型污染公告）。

包本身解包只有 46 KB【本轮硬证据·registry】。**「包很小」和「装得快」是两回事**，这一条对我们选分发形态是直接的证据。

README <https://github.com/teionarr/LocalhostVibe>【本轮重读】自称 "Software-as-a-Meme"、"a focused 9-minute build"，也老实交代了底层：

> Fine. Between us, off the record, no slides: it's a tunnel. A real one. Traffic leaves through **Cloudflare (default) or localtunnel**

它自己不做隧道，是套壳 + 一层本地代理插进去的实时访客面板（`--no-proxy` 会关掉面板，反证面板靠代理实现）。给的东西是：剪贴板里的链接、终端二维码、一个 "control room" 实时访客列表、一个 "vibe score"。

**这是 DESIGN §3.4「知道结果」的玩具版，已经有人做了，而且是 9 分钟做的。** 这不削弱我们的判断，反而说明「访客可见性」这个需求直觉上足够强，强到有人拿它当段子做——但没人认真做。

### 3.10 tinyfi.sh —— 最聪明的分发动作

【只读了文档】`ssh -o StrictHostKeyChecking=accept-new -R 80:localhost:PORT tinyfi.sh`。零安装、零账号、随机子域、支持自定义子域、关笔记本就断。

真正值得记的不是隧道，是**它把自己做成了 AI agent 的 skill**：clawhub 上的 `tunneling` skill（作者 simantak-dabhade，2.1k downloads，updated 2026-05-01），skill 描述里直接教 agent「在后台跑 SSH 命令，把公开 URL 报给用户」。

**它的分发渠道不是开发者，是开发者的 agent。**

### 3.11 tunr —— 功能最全，而且已经在卖「知道结果」

【只读了文档】单个静态 Go 二进制（`brew install ahmetvural79/tap/tunr` 或 `npx tunr@latest`），v0.4.1 发布于 2026-06-11。官网自称「exposes your local development server to the internet in **< 3 seconds**」。

功能表里对我们最扎眼的几个：

| 参数 | 干什么 |
| --- | --- |
| `--inject-widget` | **往 HTML 里注入反馈小组件** |
| `--freeze` | cache-on-crash，你的 dev server 崩了访客还能看 |
| `--demo` | 只读模式，POST/PUT/DELETE 全挡 |
| `--qr` | 终端二维码 |
| `--json` | 给脚本/CI 用 |
| `--password` / `--auth-token` / `--allow-ip` | 三档访问控制 |
| `--ttl 1h` | 到点自动关 |

外加 MCP server、官方 Node 与 Python SDK、UDP / TLS 隧道、区域选择。官网首页直接把这三个打包叫 **"Full vibecoder demo package"**：

```
$ tunr share -p 3000 --freeze --inject-widget --demo
```

DESIGN §2.2 写「tunr 的反馈小组件说明『知道结果』这个方向别人也看到了」——**实际比这更进一步：他们已经把它打包成一条命令在卖了，而且给它起了名字。**

## 4. 没能验证的，和为什么

1. **访客侧真浏览器行为**——全程只用了 curl，没开真浏览器。所以 pinggy 中间页对**真实游戏加载**的影响（子资源到底会不会拿到 HTML）是推断，不是实测。这是这份报告最需要补的一个洞。
2. **uplink 的访客 URL 与 curl 结果**——12:58:30 起来了，13:03 被中断，没拿到。
3. **kshare 的实际输出**——同上。
4. **serveo 的结论**——跑过，输出随会话记录轮转丢失。
5. **cloudflared 版本号**——本机 `/opt/homebrew/bin/cloudflared` 存在，但三次 `--version` 调用都被工具层拒绝（`Failed to find tool call context`），没记到。
6. **各家「敲回车到出链接」的精确秒数**——秒表数据在轮转里丢了。§5(a) 用能站住的方式回答。
7. **手机 UA / 微信内置浏览器**——没测。
8. **大文件与真实引擎导出物**——`fixtures/` 里最大的只有 1.15 MiB。Godot 与 Unity 的导出物还没有（`fixtures/README.md` 里已经写明这台机器没装引擎，「没做，也没有伪造」）。30–50 MB 这一档完全没碰。
9. **大陆可达性**——没有大陆出口可测。

按仓库规约，上面第 1、2、3、6 条要在真机上补齐后，写一条带日期的 `docs/spikes/` 记录才算「已验证」。这份文件是调研记录，不是 spike。

## 5. 对 playtest.run 的含义

### (a) 各家实际用了几秒拿到链接

精确秒数丢了，不编。但**时间花在哪**这件事有硬证据，而且比秒数有用：

**ssh 一族（localhost.run / tinyfi.sh / serveo，以及 pinggy 的 ssh 形态）** 是唯一能进入「一两秒」量级的，因为它们不装任何东西——时间只有一次 SSH 握手加服务端分配子域。

**npx 一族的时间大头不是建隧道，是下包。** 按本轮实测的解包体积排：

```
bore-cli      10.06 MB
pinggy         1.03 MB
uplink-cli      595 KB
localhostvibe    46 KB   ← 但实际拉了 130 个包
kshare           42 KB
localtunnel      24.5 KB
```

LocalhostVibe 那条日志里 130 次 `placeDep` 是关键反例：**声明体积小不代表装得快。**

各家自称的数字：tunr「< 3 seconds」，kshare README「Thirty seconds later」。kshare 把 30 秒当卖点写在第一屏，说明这一档的心理预期就是几十秒。

**结论**：DESIGN §3.2 承诺「几秒拿到链接和二维码」，这个承诺只有在 CLI 是**单个静态二进制、不走 npm、不依赖系统 ssh** 的前提下才成立。§4.7 已经定了 Rust，方向对；但这里要加一条约束——**第一次运行的耗时应该是一个写进验收标准的数字，不是随缘的结果**，见 (e)。

### (b) 访客侧中间页是常态还是例外

**是例外，不是常态。** 这一批里明确有中间页的只有 pinggy 一家；cloudflared 快速隧道没有；tunr 有的是主动注入的 `--inject-widget`，那是叠在游戏上的组件，不是拦在前面的页。

**最像警告的：pinggy。** 判据不是它长什么样，而是**它提供了绕过办法**。一个东西如果需要 `X-Pinggy-No-Screen` 这样的逃生门，说明它自己也知道它是拦路的。而且绕过条件是「任意非标准 User-Agent」——它靠猜 UA 判断访客是不是人，这个判断在微信内置浏览器里成不成立，没人验证过。

**最像产品的：tunr 的 `--inject-widget` 和 kshare 的 `--password`。** 它们是开发者主动打开的功能，不是服务商强加的关卡。区别在于**谁决定它出现**。

**对我们**：DESIGN §3.3 说门禁页「不是 ngrok 那种警告页，它是产品的一部分」。这一轮支持这个判断，但 pinggy 给出了三条必须守住的硬线，否则我们会做出第二个警告页：

1. **门禁页只在导航请求上渲染**（`Sec-Fetch-Dest: document`，或 `Accept: text/html` 且非 XHR）。子资源永远不渲染。
2. **子资源请求在没有会话时，绝不返回「HTTP 200 + HTML」**。要么给资源本身，要么给 403。
3. **不存在绕过 header。** 如果我们需要一个 `X-Playtest-No-Gate`，说明门禁页站错了位置——它应该站在会话层，不是流量层。

判断依据必须是请求语义，**不能是 User-Agent 字符串**。pinggy 在这一点上做错了，而我们的第一批用户在微信里。

### (c) 哪个整体体验最好

**cloudflared 快速隧道。** 三条理由：不要账号、访客侧没有中间页、不改你的响应内容——而且它背后有一家真公司在兜带宽。代价 DESIGN §0.5 和 §2.1 已经写了：大陆连接差。

第二是 **ssh 一族**，因为「不用装东西」这条太强，强到能压过功能差距。tinyfi.sh 把自己发成 agent skill 是这一批里最聪明的一步棋。

**npx 一族体验最差**，不是隧道差，是三件事叠在一起：第一次要等下载；你不知道自己刚往机器上装了什么（LocalhostVibe 的 130 个包、bore-cli 的 10 MB、kshare 会改写你的响应体）；命令行不统一，`bore` 这种同名不同物直接吃掉两分钟。

**tunr 功能最全，但功能全不等于体验好。** 它是这一批里唯一一个既是单静态二进制、又有 SDK 和 MCP、还想到了 `--freeze` 这种细节的。它是我们最该认真对待的对手。

### (d) 在「游戏」上哪里露怯

五个地方，按危害排序：

**1. 子资源拿到中间页，而且状态码是 200。**

Phaser 那个 fixture 的形状说明了一切：入口 HTML 843 字节，真东西是 1.2 MB 的 JS。如果那条 JS 请求拿回来的是 15 KB 的 HTML，浏览器**不会报网络错误**——状态码是 200，Content-Type 大概率也是 `text/html`——最好的情况是控制台里一个看不懂的语法错，最坏是白屏加沉默。**在游戏场景里，200 比 404 坏得多。** 真浏览器带着确认 cookie 一般踩不到，但 Service Worker、`credentials: 'omit'` 的 fetch、跨源子资源、预加载都可能踩到。这条必须在真浏览器里复测。

**2. 响应体被改写。**

kshare README 明说对 HTML / CSS / JS 做原始文本 URL 替换。这类工具是给 CRUD 网页设计的，作者大概没想过有人会用它传 1.2 MB 的 bundle 或者 `.wasm.br`。**只要被当文本处理一次，二进制产物就坏了。**

**3. 没有一个隧道会替你补响应头——而这恰恰是游戏最需要的。**

这是 §1 那个「测的是改不是补」的局限反过来变成了结论：`.wasm` 的 MIME、`.br` 的 `Content-Encoding`、Godot 4 线程导出要的 COOP/COEP，**这些头在隧道路径上跟上传路径一样缺**。开发者的 dev server 未必发（`python3 -m http.server` 一个都不发，Vite 也不发 COOP/COEP），隧道更不会补。

**这暴露了 DESIGN 的一个洞**：§4.2（上传路径）把这些头写得很细，§4.3（隧道路径）**一个字都没提**。但 Godot 4 的开发者敲 `playtest 5173` 时一样打不开。见 §6。

**4. 大文件走免费隧道，瓶颈比想象的早。**

实测 pinggy ~740 KB/s【上轮实测】。按这个速率推：

| 负载 | 一个玩家等多久 |
| --- | --- |
| 我们的 Phaser fixture（1.15 MiB） | ~1.6 秒 |
| 典型 Godot Web 导出（30 MB） | ~41 秒 |
| 典型 Unity WebGL 导出（50 MB） | ~69 秒 |

740 KB/s 约等于 5.9 Mbps。DESIGN §4.3 把这个瓶颈归给「家宽上行（普遍 30–50 Mbps）」，**但实测说明瓶颈在隧道服务商的免费档限速，比家宽上行早得多**。这对我们是双面的：好消息是我们不限速就能明显更快；坏消息是带宽成本（§6）要按真实吞吐重算，不能按家宽上行封顶。

**5. 手机与微信没人验证。**

pinggy 靠 UA 猜浏览器，tinyfi.sh / serveo 靠 ssh 会话活着，kshare 的链接 24 小时过期。这些机制在微信内置浏览器里的表现，这一批工具里没有一家提过——因为他们的用户不在那儿。**这是我们唯一一块别人根本没在看的地。**

### (e) 我们要「比所有人好」的具体落点

不是五个方向，是五个可以被验收的数字或承诺：

**1. 把「第一次运行到出二维码」变成一个写进验收的数字。**

单静态二进制，不走 npm，不依赖 ssh。目标对齐 tunr 自称的 < 3 秒，并且**在 CLI 里把这个耗时打出来**。所有人都在比这一条，但没有一家给你看数字——kshare 甚至把 30 秒写成卖点。这是最便宜也最显眼的一格。

**2. 门禁页三条硬线，写进文档当承诺。**

只在导航请求上渲染；子资源永不返回「200 + HTML」；不存在绕过 header。判据用 `Sec-Fetch-*`，不用 UA。这是 pinggy 露的怯，也是 DESIGN §3.3 唯一可能被做歪的地方。

**3. 「永不改写你的字节」——这是可验证的承诺，所以值钱。**

隧道只改 `Host` 头（§4.3 已定），响应体一个字节不动。SDK 是开发者主动加的，不是我们注入的。这一条能用「对拍哈希」验证，所以可以写成一句硬承诺放在 README 里。kshare 做不到，LocalhostVibe 做不到（它靠代理注入面板），tunr 的 `--inject-widget` 是可选的所以也算做到——但**没有人把它当承诺讲**。

**4. 响应头在隧道路径上也补，不只在上传路径。**

`--isolated`、`.wasm` MIME、`.br` 的 `Content-Encoding` 对两条路都生效。这一格**一个竞品都没做**，而且它正好卡在 Godot / Unity 开发者身上——这两个引擎的用户是最容易被「白屏、不知道为什么」劝退的。

**5. 把限速和体积说成人话，在开发者按回车之前。**

没有一家把免费档的真实吞吐写在首页。我们可以在 CLI 里直接算给开发者看：

```
你这个目录 32 MB。隧道模式下一个玩家大约要等 45 秒，十个人同时进来会更久。
建议改用 playtest ./dist —— 上传一次，玩家从香港边缘直接拿。
```

DESIGN §4.3 已经有这个提示的雏形，把它做成**带具体秒数、基于实测吞吐的**，就是没人做的那一格。

**外加一条关于分发渠道的观察，它不在上面五条里，但可能比它们都重要**：tinyfi.sh 把自己做成 agent skill，uplink 在 npm 包描述里直接写 "JSON-first CLI for Cursor, Claude, Codex, and Windsurf"，tunr 带 MCP server。**三家都在把 AI coding agent 当分发渠道，不是把开发者当分发渠道。** DESIGN §1.1 把「用 AI 写小东西的人」列为第一类用户，但 §3.2 的形态清单里只有 CLI，加上「希望社区做」的托盘应用和引擎插件，没有 MCP / agent skill 的位置。这是一个和 DESIGN 有张力的地方。

## 6. 与 DESIGN.md 的对照

### 支持现有结论的

- **§2.1「绝大多数骑在 Cloudflare 上，大陆连接差」**——坐实了。LocalhostVibe 默认后端就是 Cloudflare，untun 就是 cloudflared 的壳。这一批里访客体验最好的（cloudflared 快速隧道）恰恰是大陆最不可用的。
- **§2.2「tunr 的反馈小组件说明『知道结果』这个方向别人也看到了」**——比 DESIGN 写的还进一步。tunr 已经把 `--freeze --inject-widget --demo` 打包成 "Full vibecoder demo package" 在首页卖。
- **§3.3「门禁页不是 ngrok 那种警告页」**——pinggy 提供了完整的反面教材，也提供了三条可执行的硬线。
- **§4.3 最后一段「隧道不适合大体积静态资源」**——给了它一个具体数字（740 KB/s → 30 MB 要 41 秒）。
- **§4.7 选 Rust 单二进制**——npx 一族的首跑代价（130 个包 / 10 MB / 同名不同物）是这一轮最强的支持证据。
- **§0.4「知道结果是护城河」**——LocalhostVibe 用 9 分钟做了个玩具版访客面板，说明这个需求的直觉强度很高，但没人认真做。

### 冲突或需要补的

**1. §4.3 完全没写响应头，这是个洞。**

§4.2（上传）把 `.wasm` MIME、`.br` 的 `Content-Encoding`、`--isolated` 的 COOP/COEP 写得很细，§4.3（隧道）一个字没提。但 Godot 4 的开发者用 `playtest 5173` 时同样打不开——dev server 不发这些头，隧道也不会补。建议在 §4.3 补一段，并明确 `--isolated` 对两条路都生效。这同时是 §5(e) 第 4 条那个「没人做的格子」。

**2. §4.3 把大文件的瓶颈归给家宽上行，实测不支持。**

原文写「大体积静态资源经家宽上行（普遍 30–50 Mbps）」。实测 pinggy 只有 5.9 Mbps，瓶颈在服务商免费档限速。建议改成「免费隧道服务商的限速通常比你的家宽上行更早成为瓶颈」，并把「我们不限速」当成一个可量化的优势——同时在 §6 按真实吞吐重算带宽成本。

**3. §3.2 的形态清单里没有 MCP / agent skill。**

三家竞品（tinyfi.sh、uplink、tunr）都在走这条分发路，而 §1.1 的第一类用户正是 AI coding 用户。建议在 §3.2 或 §8「留门但不推进」里明确表态——做还是不做，理由是什么。现在是没提，不是决定了不做。

**4. §2.1 把 kshare / LocalhostVibe 和 tunr / uplink 并列，高估了这个市场的密度。**

实际差距很大：tunr 是单静态 Go 二进制 + SDK + MCP + 十几个参数；uplink 五天前还在发版、命令行是给 agent 设计的。另一头，kshare 是一个人的 VPS（README 自己说 $4/月 Hetzner 够用，`MAX_TUNNELS_PER_IP=5`），LocalhostVibe 自称 "Software-as-a-Meme"、"9-minute build"、底层套别人的隧道。放在同一行会让人以为对手很多，实际上**认真的对手只有 tunr 和 uplink 两家**。建议 §2.1 那一格分成两层写。

**5. §4.3 的生命周期段落没提保活探测打到哪。**

观察到 uplink（推断）每 30 秒往源站发一次 `HEAD /`。我们的保活应该在 WSS 控制通道上做，不要打到 `127.0.0.1:<port>`——那会污染开发者自己的日志。建议在 §4.3 的生命周期里加一句。
