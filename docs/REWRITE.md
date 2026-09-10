# playtest · 重写蓝图（面向商业化发布）

> 2026-09-11 起草。这是一份施工蓝图：把产品、体验、架构、计费、运维一次想透，拆成可以直接照着做的方向与里程碑。
> 与其它文档的关系：`docs/DESIGN.md` 仍是唯一产品定义——本文 §2–§4 里所有产品判断在 M0 回写进 DESIGN，本文之后只保留 §5–§9（施工部分）；`docs/KICKOFF.md` 由本文 §6 取代。`docs/spikes/` 只增不改，每个里程碑收尾时记一条真机记录——**记录不是闸门**：本文不设「验证后再决定」的等待点，所有决定在本文里已经做完。
> 读法：§0 是一页结论；§2、§3 是要做成什么样；§5 是怎么做；§6 是按什么顺序；§9 是每个决定为什么不是另一个。

## 0. 一页结论

1. **重写的是结构，不是字节。** 现有 45,000 行 Rust 里，上传、隧道、混合、门禁硬线、熔断、上传时检查、版本回滚是在香港真机上成立过的，测试与实现约 1:1。这些保留。重写的是五处结构：CLI 的输出层（人类 / JSON / MCP 三套平行实现 → 一个 Report 两个渲染器）、边缘的渲染层（20 页分散在 6 个文件、两套 CSS → 一套设计系统一个模板层）、控制面的数据层（三套「作品展示态」、6 个后台循环、单 SQLite 互斥锁 → 一个 `Project` 事实表 + 投影、一个调度器、Postgres）、契约层（Rust 手写 + TS 手抄 → 单一来源生成 TS 与 OpenAPI）、以及所有对 agent 可读的面（现在是零）。
2. **一个原语：作品（Project）。** 版本、访问控制、上架、名额、封面、邀请卡、关注数、计划档位全部是作品身上的属性或投影，不再是并列的名词。玩家侧只有三个东西：会话、反馈、关注。名词表见 §2.4，之后新增名词要过 §2.5 的准入。
3. **三个环不变，顺序不变**：邀请环（卡）→ 回访环（关注与通知）→ 注意力环（广场、推广、周报）。施工顺序按环走（§6），不按 crate 分层并行。
4. **超越 here.now 的定义是四条**，不是功能表比它长：(a) 发布这一个动作的每个边角都到底（匿名 → 认领、增量与续传、版本与回滚、自选名字、口令与邀请名单、TTL、配额、撤销、读回）；(b) 每一个面都对 agent 可读（`llms.txt`、`skill.md`、OpenAPI、MCP 一一对应）；(c) 它没有而我们有的四件事在同一条命令里——隧道与混合后端、对游戏有感的边缘与门禁页、点名册式的结果、玩家会回来（卡、关注、广场）；(d) 中英双语，从 CLI 到邮件。对照表在 §1.3。
5. **商业化从第一天就是完整的**：档位、限制、执行点、Stripe Checkout、Webhook、发票、退款、赠送全部实现；只有密钥与主体是外部物件（§8）。主体未落地时控制台如实写「尚未开放」——但那是一个开关，不是一段没写的代码。
6. **两个域名不变**（`*.playtest.run` 玩家，`playtest.roviix.com` 开发者）。两个域一套设计系统一套颜色，开发者是唯一同时进两间房的人，不该觉得是两个产品。
7. **v1.0 的定义**在 §1.2：一个从没用过的开发者在任何一台电脑或任何一个 AI 编程工具里 60 秒内拿到链接、二维码、邀请卡；一个玩家在微信里扫卡、留名、关注；第二版发出去他收到信；开发者在控制台看到「来自通知 1」；开发者付了 $15 拿到自己的名字和口令。这五件事全部由本仓库的代码完成，没有一处是人工代办。

## 1. 目标、判据与对照

### 1.1 商业化发布的定义

- **能收钱**：Pro 订阅与推广 SKU 走 Stripe（信用卡 + 支付宝），发票自动，退款有路径，赠送有管理接口。
- **能兜底**：免费档硬上限、每作品每小时熔断、月配额到顶硬停——任何情况下不产生我们付不起的账单，也不产生用户付不起的账单（不做后付费）。
- **能运营**：管理接口覆盖举报处置、推广审核、赠送、封禁、配额调整、通知队列；有备份、有监控、有告警、有状态页；隐私声明、服务条款、内容政策、举报与申诉流程在文档站上。
- **能被找到**：开发者从搜索、AI 工具、引擎社区任一入口进来都能自己学会用；英文 README 与文档站与中文同步。
- **不虚报**：CLI、界面、文档里没有一句「已支持」是代码做不到的；等外部物件的功能写「尚未开放」并且是一个开关。

### 1.2 质量判据（v1.0 全部成立，每条对应一条 spike 记录）

| 判据 | 数字 |
| --- | --- |
| 安装到第一个链接 | 中位 < 60 s，含下载单文件二进制 |
| 命令到二维码 | P50 < 5 s（上传 ≤ 50 MB）；隧道 P50 < 2 s |
| 门禁页 | 首字节 < 300 ms（香港边缘、大陆三网）、整页 < 12 KB、零第三方脚本、无 JS 可点开始与留名 |
| 邀请卡 | 首次渲染 < 300 ms，命中 < 5 ms；中文不出方块 |
| 边缘可用性 | 内容路径 99.9%/月；控制面停机时作品、广场、卡、门禁页全部照常 |
| 控制面可用性 | 99.5%/月 |
| 结果延迟 | 玩家动作到控制台可见 < 90 s |
| 通知 | 新版本到邮件出队 < 5 min（受 24 小时合并规则）；Gmail / QQ / 163 不进垃圾箱 |
| 双语 | CLI、门禁页、卡、广场、控制台、邮件、文档全部中英；无一句机翻感文案 |
| 可访问性 | 玩家域与控制台全部可键盘操作、`:focus-visible` 可见、对比度 ≥ 4.5:1、`prefers-reduced-motion` 生效 |
| 测试 | `cargo test --workspace` 绿、clippy `-D warnings`、两个 TS 包类型检查、三进程端到端脚本绿、无头 Chrome 走完玩家路径 |

### 1.3 与 here.now 的对照（我们要做到的位置）

| 能力 | here.now | playtest v1.0 | 备注 |
| --- | --- | --- | --- |
| 匿名发布、24 小时 | ✓ | ✓ | 匿名 1 GB / 24 h |
| 认领匿名作品进账号 | ✓ | ✓ | `playtest login` 后自动归入（已做） |
| 增量上传 / 断点续传 | 部分（refresh URL） | ✓ | 按内容哈希增量；断了重跑同一条命令就是续传 |
| 版本历史、预览旧版、回滚 | ✓（付费） | ✓（免费 5 版，Pro 50 版） | 回滚是指针翻转 |
| 自选名字（slug） | ✓（API key） | ✓（Pro） | 两个词加数字免费 |
| 口令 / 邀请名单 | ✓ | ✓（Pro） | 在门禁页完成，玩家不注册 |
| TTL | ✓ | ✓ | `--ttl 7d`，登录用户 |
| SPA 回退 | ✓ | ✓ | 已做 |
| 读回文件清单 | ✓ | ✓ | `GET …/versions/{v}/files` |
| 自定义域名 | ✓ | **有意不做** | §9.7 |
| 团队 / Workspace | ✓ | Pro 3 位只读 reviewer | §9.8 |
| Drive / Site Data / Proxy routes | ✓ | **有意不做** | 不运行用户代码、不做通用存储（§9.9） |
| iframe 嵌入 | ✓ | **不做** | 门禁页是信任凭证，不可被嵌 |
| 对 agent 可读（llms.txt / skill / OpenAPI / MCP） | ✓ | ✓ | MCP 工具与 CLI 一一对应 |
| 分析 | 付费 | ✓ 点名册（免费） | 计数与点名，不是仪表盘 |
| 隧道、混合后端 | ✗ | ✓ | 同一条命令 |
| 对游戏有感的响应头、上传时检查 | ✗ | ✓ | wasm / 预压缩 / COOP-COEP / Godot / Unity |
| 门禁页、邀请卡 | ✗ | ✓ | 微信里唯一被看见的面 |
| 玩家留名、关注、通知、周报 | ✗ | ✓ | 回访环 |
| 广场、推广位 | ✗ | ✓ | 注意力环 |
| 双语 | 英 | 中英 | |

### 1.4 五条物理约束（不随本文改变）

从不运行用户代码；每作品每小时熔断；首次游玩零门槛；不往作品画面注入任何东西；玩家域与开发者域分离。

## 2. 产品模型：一个原语

### 2.1 作品（Project）

一个 slug 就是一个作品，永远指向它最新的版本。作品身上的属性分六组，每组在 CLI、API、控制台、`live.json` 里用同一组名字：

| 组 | 属性 | 谁改 |
| --- | --- | --- |
| **身份** | `slug`、`owner`、`created_at`、`expires_at`（匿名或 TTL）、`plan`（free / pro，随 owner） | 系统、`rename`、`--ttl` |
| **交付** | `mode`（upload / tunnel / hybrid）、`current_version`、`isolated`、`spa`、`gate`（once / always / off）、`tunnel_online`、`last_seen` | 发布时、边缘（在线状态） |
| **呈现** | `title`、`summary`、`cover`、`engine`、`community_url`、`badge`（free 有、pro 无） | `--name --summary --cover --community` |
| **访问** | `access`（link / password / invite）、`password_hash`、`invite_list` | Pro，`--password`、控制台 |
| **上架** | `public`、`seats`、`joined`、`listed_at`、`hidden`（举报 / 人工）、`boost`（当前推广窗） | `--public --seats`、`unlist`、管理接口 |
| **社会** | `followers`、`feedback_public`、`public_feedback[3]` | 玩家、控制台开关 |

**投影**（都从同一张事实表算，不再各自拼）：`Project`（API 完整响应，含全部六组）→ `ProjectCard`（广场卡与控制台作品墙：封面、标题、开发者、一件事实）→ `ProjectLive`（`live.json`，门禁页与卡要的那几样会变的）→ `ProjectPublic`（`plaza.json` 里的一项）。四个投影是一个 Rust 函数族 `Project::card() / live() / public()`，控制台 TS 类型由它生成。

### 2.2 版本（Version）

不可变。`v1, v2, …`；`note`（这版改了什么，也是通知正文）、`file_count`、`total_bytes`、`manifest`。隧道模式没有版本，只有在线 / 离线。

### 2.3 玩家侧的三个东西

- **会话（Session）**：设备上的一个 cookie；有 `name`（自愿）、`from`（card / notice / plaza / wechat / browser / discord…）、`is_return`。
- **反馈（Feedback）**：一句话 + 截图（可选）+ 设备 + 版本 + 进入多久；`status`（new / seen / done）、`hidden`。
- **关注（Follow）**：玩家（邮箱或推送订阅，双重确认）→ 作品或广场；`pt_me` 是根域上的钥匙，不是账号。

### 2.4 名词表（中英，一处定义，全仓引用）

| 中 | 英（CLI / API / 文档） | 不用的说法 |
| --- | --- | --- |
| 作品 | project | site、work、build、app |
| 版本 | version | build、release、deploy |
| 试玩者 / 玩家 | player | tester、user、visitor |
| 开发者 | developer | creator、author |
| 门禁页 | gate | landing、splash、interstitial |
| 邀请卡 | card | poster、ticket、share image |
| 广场 | plaza | gallery、feed、discover |
| 我的 | me | profile、account（玩家侧没有账号） |
| 点名册 | roster | sessions list、analytics |
| 结果 | results | analytics、dashboard、stats |
| 关注 | follow | subscribe、watch、wishlist |
| 名额 | seats | slots、quota |
| 推广 | boost | promote、ad、sponsored（界面上标「推广 / Boosted」） |
| 控制台 | console | dashboard |
| 隧道 / 上传 / 混合 | tunnel / upload / hybrid | share、deploy、host |

英文动词：`publish`（发布）、`share`（隧道分享端口）。API 路径 `/v1/projects`（§5.2）。

### 2.5 名词准入

新增名词、页面、参数、依赖必须满足其一：让 §1.2 的某一条数字更好；让 DESIGN §1.2 / §1.3 的某一个时刻更短或更可信；落在三个环之一。并且要在 §2.1 的六组属性里找到位置——找不到位置的名词就是第二个原语，不加。

## 3. 体验设计

### 3.1 CLI

**命令表（最终形态）。** 一个二进制 `playtest`，10 个子命令，位置参数决定交付模式：

```
playtest <目录|端口> [选项]           发布：目录走上传、端口走隧道；--backend 是混合
  呈现     -n --name  --summary  --cover <png>  --community <url>
  这一版   -m --note "这版改了什么 / 想让人看什么"
  上架     --public  --seats <N>（隐含 --public）
  访问     --password <口令>  --invite <邮箱,…>（Pro）
  交付     --backend <端口>  --isolated[=auto|on|off]  --spa  --gate <once|always|off>  --watch
  身份     --to <slug>（发到指定作品；默认按目录记住上次的）  --ttl <7d|24h>（登录用户）
  输出     --json  --no-qr  --card <路径>（默认 ./<作品名>-邀请卡.png，写 - 不落盘）
playtest login | logout | whoami        GitHub 设备码；whoami 显示档位与本月用量
playtest ls                             本机发过的作品，一行一个：slug · 名字 · 状态 · 一件事实
playtest open <slug|目录>
playtest versions <slug|目录>           每一版一行，标出玩家现在看的是哪版
playtest rollback <slug|目录> <v>
playtest rename <slug> <新名字>         Pro
playtest unlist <slug>                  从广场撤下，链接照常
playtest rm <slug> [-y]
playtest card <slug|目录> [--out]       重新拿邀请卡
playtest mcp [--setup]                  作为 MCP server
全局：--json  --api <url>（隐藏）  --lang zh|en（默认跟 LANG）  -q（只出链接）
```

去掉的：`--seek`（并入 `--note`）、`--no-isolated`（`--isolated=off`）、`--no-card`（写 `--card -`）、`--card-out`（就是 `--card`）、`--site` / `--new`（`--to` 一个；不给就按目录记住的，`--to new` 强制新建）、`--force`（变成交互确认 + `-y`）、`followers`（`ls` 与 `whoami` 里就有）。`--api` 11 处声明收成一处全局。

**发布成功时打出的东西**（顺序固定，双语；`--json` 里是同名字段）：

```
已发布 v7 · 《摸一只鱼》· 3.4 秒（哈希 0.1 · 上传 2.9 · 提交 0.2 · 卡 0.2）
https://sage-salmon-91.playtest.run
[二维码]
邀请卡  ./摸一只鱼-邀请卡.png          → 发到群里，别人长按识别就能玩
广场    在广场上 · 正在找 10 位试玩者    （或：没放到广场。加 --public 放上去）
到期    这是匿名链接，9 月 12 日 20:59 失效。playtest login 之后不再到期
结果    12 人关注着它 · 来的人玩成什么样在 https://playtest.roviix.com/console/#/p/sage-salmon-91
提醒    没有封面：广场上和卡上会用作品名排一张字卡。加 --cover cover.png 换成你的图
```

**输出契约。** stdout 只放用户要拿走的东西（链接；`--json` 下一个 JSON 对象；隧道 `--json` 下一行一个事件），一切叙述走 stderr。`-q` 只出链接。退出码：0 成功、1 一般错误、2 参数错误、3 上传前检查阻断、4 连不上控制面、5 需要登录 / 档位不够、6 配额。**一个 `Report` 结构、三个渲染器（人类 / JSON / MCP），任何命令只实现一次。**

**错误规范。** 每条错误三句：出了什么事、为什么、下一步敲什么。例：「这个构建需要 `--isolated`：它是 Godot 的线程导出（game.wasm 里有共享内存）。加 `--isolated` 重发，或者在 Godot 里关掉线程重新导出。」不出现内部词（yamux、blob、manifest、清单）。

**上传前检查**（已有 9 项）保留，新增：没有 `--summary` 提醒一句（卡和广场靠它）；文件里出现 `localhost:` 字面量提醒一句（常见的 API 地址写死）。

**隧道模式的话**：连上时打「已连上，这条链接现在能玩了」；断线时打「断了，正在重连（第 2 次）…」，链接不变；退出时打「已离线。玩家现在会看到『开发者的电脑暂时不在线』」。对大目录（> 8 MB）建议改上传并给数字。

### 3.2 Agent 面

**MCP** `playtest mcp`：工具与子命令一一对应、同名同参：`playtest_publish`（目录）、`playtest_share`（端口，长驻）、`playtest_list`、`playtest_get`、`playtest_versions`、`playtest_rollback`、`playtest_unlist`、`playtest_card`（回 PNG 内容块）。`--setup` 打出 Cursor / Claude Code / Codex 的配置片段。版本号读 `CARGO_PKG_VERSION`。

**HTTP API**（`playtest.roviix.com/v1`，OpenAPI 3.1 在 `/openapi.json`）：三步发布 `POST /v1/projects` → `POST /v1/projects/{slug}/uploads`（回缺的哈希）→ `PUT /v1/blobs/{hash}` → `POST …/uploads/{id}/commit`；`GET /v1/projects/{slug}/versions/{v}/files` 读回；其余与 CLI 用的同一组。匿名：不带令牌 `POST /v1/anon` 拿 24 小时令牌。文档里给 curl 三步与 Python / Node 十行示例。

**发现文件**：`playtest.roviix.com/llms.txt`、`/llms-full.txt`、`/skill.md`（可 `npx skills add`）、`/openapi.json`、`/.well-known/agent.json`；根域 `playtest.run/llms.txt` 一行指向开发者域（玩家域上唯一的第二个例外，与介绍页那个链接同性质：给 agent，不给玩家）。README 顶部一段「给 AI 助手的话」。

### 3.3 玩家域

**门禁页**（同一 URL、不跳转，只在顶层导航渲染，三条硬线不变）从上到下：封面或字卡；「某某 邀请你试玩《作品名》」（识别到引擎或 `--game` 用「试玩」，否则「体验」）；一句话；`v7 · 9 月 9 日 · 「改了新手引导」`；名额一行（设了才有）；**开始** + 「你的名字（可不填）」；弱化三行：有新版本时告诉我（邮箱 / 浏览器通知按能力显示）、开发者的群、分享（公开才有）；公开反馈最多三条；举报；角标。**访问控制态**：口令作品「开始」上面多一个口令框；邀请名单作品「输入邀请码或用你收到的链接」。**离线态**：「开发者的电脑暂时不在线，上次在线 12:40」+ 关注。**熔断态**：「这个作品今天太受欢迎了，一小时后再来」+ 关注。**到期态**（410）：「这条链接已经到期。开发者用 `playtest login` 之后的链接不会到期」。**举报后**：「收到，我们会看」。所有态一套模板、一套 CSS、无脚本可用。

**邀请卡**：竖版 1080×1350、横版 1200×630；封面或字卡、「某某邀请你试玩《作品名》」、一句话、`v7 · 9 月 9 日`（匿名写到期）、名额（设了才有，不写已加入）、二维码 `?from=card`、撕票线、角落 `playtest.run`（Pro 可去）。卡上永不出现人数、限时、领取。渲染在边缘，ETag 跟版本与名额走。

**分享页** `/_playtest/share`：只放卡、保存图片、复制链接、系统分享。公开作品才有。

**广场** `playtest.run/`：左栏（字标、广场、我的、栏底「从一条命令开始」打开命令说明）+ 一面墙（推广 ≤ 2 在前并标「推广」→ 正在找人测 → 按时间）。卡：4:3 窗（封面 / 色田）、左上最多一个标、窗下作品名、一行简介或求测语、底行开发者头像名字 + 一件事实。无脚本。稀疏态从左排。空态「广场上还没有作品」+ 命令说明。

**我的** `playtest.run/me`：没钥匙——一句话 + 邮箱（留下就是关注广场）+ 有能力时「用浏览器通知」；有钥匙——关注列表逐个取消、邮箱打码、通知开关（有路由）、换一台设备。没有头像、昵称、历史。

**邮件**（纯文本优先 + 一张封面）：确认信、新版本（每作品每 24 小时最多一封，正文是 `--note`）、周报（周一 09:00 UTC+8，本周新出的 N 个 + 其中 M 个在找人测 + 推广 ≤ 2 标「推广」）、换设备链接。每封底部一键退订。语言跟关注时的页面语言。

### 3.4 开发者域

**登录**：GitHub（设备码 / 网页授权码）；`playtest login` 后匿名作品自动归入。控制台没有令牌粘贴页——开发者在网页上点 GitHub 登录即可；CLI 令牌的撤销在账号页。

**控制台** `playtest.roviix.com/console/`，同一套设计系统：

- **作品墙**（首页）：一张卡一个作品，与广场卡同一份规则；只有一个作品直接进它；没有作品是设计好的起点（三条命令 + 发出去之后会出现什么）。
- **作品页**：身份栏（封面、名字、slug、玩家链接 + 复制 + 二维码、状态签：`v7 · 玩家看到的`、`在广场上 · 找 10 位`、`12 人关注`、`隧道 · 在线`、匿名到期）；「现在」一行（近一小时几人、最新一条反馈、这版错误数——控制面提供字段）；五个签：**结果**（每版一块：四个大数字、§3.5 那段话、来源、与上版比、「看这 8 个人」「让玩家看这一版」）、**点名册**（每人一行，名字主角，停留最短在前，展开事件与错误；可隐藏名字）、**反馈**（一句话主体、状态、公开开关下逐条隐藏、有截图时「设为封面」）、**邀请卡**（两张放大、保存 / 复制 / 分享、去哪用）、**设置**（呈现 / 上架与名额 / 访问控制（Pro）/ 群 / 公开反馈 / 推广（可买或「尚未开放」）/ 版本清单与回滚 / TTL / 危险区）。
- **账号页**：档位与本月用量（流量、活跃作品、版本数）、升级 / 管理订阅（Stripe 客户门户）、发票、推广订单、已登录的设备与 CLI 令牌（撤销）、reviewer 邀请（Pro）、删除账号。
- **运营页**（`/console/admin/`，运营者账号才有）：举报队列、推广审核、赠送、封禁、配额调整、通知队列与死信、发信信誉指标。

**文档站** `playtest.roviix.com/docs/`：快速开始（三种入口：终端、AI 工具、直接 API）、命令参考、API 参考（由 OpenAPI 生成）、引擎指南（Godot / Unity / Phaser / Cocos / Construct / p5 / Vite）、门禁页与硬承诺、结果与隐私、计费、自托管、服务条款、隐私声明、内容政策与举报申诉。中英切换。

### 3.5 SDK

`/_playtest/sdk.js` 同源提供，一行 `<script>`。收 JS 错误、加载分段、`playtest.event(name, data)`、最后一次输入；反馈按钮（一句话 + 截图 + 设备 + 版本 + 进入多久）；提交后落点：谢谢 / 有新版本时告诉我 / 看看别的作品。只往开发者域发，`text/plain` 不预检。新增：`playtest.ready()`（首帧标记，不叫的话用 `load`）与 `playtest.identify(name)`（把游戏里的名字送进点名册，玩家没在门禁页留名时用）。

### 3.6 设计系统（两个域共用一套）

- **颜色**：墙 `#0b0b0c`、字 `#ece8e1`、弱字 `#9a958d`、唯一一支色琥珀 `#ffb224`（动作、当前项、求测标、焦点圈）、警示暗橙 `#d97a3a`、推广标只用灰。字标不上色。**不再有第二支色。** 控制台与玩家域同一套，控制台只多一个「等宽数字」的用法。
- **字体**：系统字体栈（中文优先 PingFang / Noto Sans CJK / 微软雅黑），数字等宽 `tabular-nums`。不下载字体。
- **组件**（一处实现，Rust 模板层与 Preact 各一份但同名同 CSS）：壳（栏 + 面）、卡（4:3 窗 + 标 + 名 + 一行 + 底行）、按钮（主 / 幽灵）、字段、表单行、状态签、空态、错误态、说明页（`:target` 弹层）。
- **原则**：玩家域零脚本（门禁页只有能力检测与复制链接那两小段，无脚本可用）；键盘可达；触屏可点 ≥ 40 px；`prefers-reduced-motion`；卡不是盒子（无边无底无阴影）。
- **文案**：第一次用的人看得懂；不用内部比喻；数字先句子后；0 的句子不说；能不说的字不说。中英各一份文案表（`common/i18n`），页面与邮件按 `Accept-Language` / 关注时语言，CLI 按 `--lang` / `LANG`。

### 3.7 失败形态总表（设计出来的，不是遇到再说）

| 情况 | 玩家看到 | 开发者看到 |
| --- | --- | --- |
| 控制面停机 | 一切照常；关注得到「现在登记不了，稍后再试」 | CLI：「控制面暂时连不上，已发布的链接不受影响」退出码 4 |
| 隧道离线 | 离线页 + 上次在线 + 关注 | CLI 重连计数 |
| 熔断 | 熔断页 + 关注 | 控制台状态签「今天流量用完，明天恢复」+ 邮件一封 |
| 月配额到顶 | 同熔断页 | CLI 退出码 6 + 升级链接；控制台账号页 |
| 匿名到期 | 410 页 | CLI 剩 3 小时时提醒；`ls` 里显示 |
| 邮件服务商不可用 | 「确认信稍后送达」 | 运营页死信 |
| 上传中断 | — | 重跑同一条命令，只传缺的 |
| GitHub 登录失败 | — | 匿名照常；「GitHub 现在连不上，先用匿名链接，稍后再 login」 |
| 支付未开通 | — | 控制台「尚未开放」，一个按钮都没有 |
| 作品被举报撤下 | 链接照常 | 邮件 + 控制台状态签 + 申诉入口 |

## 4. 商业模型与计费

### 4.1 档位

| | 匿名 | Free（GitHub 登录） | Pro $15/月 · $150/年 |
| --- | --- | --- | --- |
| 链接寿命 | 24 h | 长期 | 长期 + `--ttl` |
| 流量 | 1 GB / 24 h | 10 GB / 月 | 50 GB / 月，预付包 50 GB / $10 |
| 活跃作品 | 1 | 3 | 20 |
| 保留版本 | 1 | 5 | 50 |
| 并发玩家 / 作品 | 20 | 50 | 200 |
| 结果留存 | 24 h | 30 天 | 180 天 + 导出 + 版本比较 |
| 自选名字 | ✗ | ✗ | ✓ |
| 角标 | 有 | 有 | 可去 |
| 口令 / 邀请名单 | ✗ | ✗ | ✓ |
| reviewer（只读） | ✗ | ✗ | 3 |
| 上广场、关注、邀请卡、点名册 | ✓ | ✓ | ✓ |

**推广**（要有账号）：3 天 $9、7 天 $19、进本周周报 $9；支付宝按当日汇率。规则：永远标「推广」、每屏 ≤ 2、免费流不动、先人工看后上、拒了全额退。

### 4.2 执行点

- **月配额与并发**：边缘从 `current.json` 读 `plan` 与 `limits`（控制面写），本地计数，按 60 秒批量报回；月配额由控制面汇总后写回 `live.json` 的 `quota_state`（ok / warning / exhausted），边缘据此切页。到顶硬停，不后付。
- **每作品每小时熔断**：边缘本地判定（已有）。
- **版本数**：提交时控制面裁旧版（保留当前指向的）。
- **活跃作品数**：创建时判。
- **档位变化**：Webhook 到达即改 `plan`，重写该 owner 全部作品的 `current.json`。

### 4.3 Stripe 流程

Checkout Session（订阅 / 一次性推广）→ Webhook（`checkout.session.completed`、`customer.subscription.updated/deleted`、`invoice.paid/payment_failed`、`charge.refunded`）→ 落 `orders` / `subscriptions` 表 → 改 `plan` / 建 `boost(pending)`。客户门户管理订阅与发票。退款：运营页一键（调 Stripe），推广拒审自动退。`PLAYTEST_STRIPE_*` 未配置时：账号页与设置页显示「尚未开放：付款通道要等主体落地」，无按钮；运营者仍可赠送。

## 5. 架构与实现方案

### 5.1 目标仓库布局

```
common/      契约：模型、投影、路由常量、限制、文案表；生成 TS 与 OpenAPI
api/         控制面（axum + Postgres）：auth、projects、uploads、versions、players、follow、notify、billing、plaza、admin、scheduler
edge/        边缘（axum）：router、render（模板层）、cache、store 读、tunnel、breaker、card、ship
cli/         playtest：commands、report、render（human / json / mcp）、inspect、upload、tunnel、client、i18n
console/     Preact SPA：由 common 生成的类型；pages/{home,project,account,admin}
sdk/         playtest.js
docs/        DESIGN.md、REWRITE.md、spikes/、research/、site/（文档站源，中英）
deploy/      compose、Caddy、Postgres、备份、监控、push
scripts/     admin.sh、e2e.sh（三进程全链路）、headless-check.mjs
fixtures/    真实导出物（补 Godot 线程 / 无线程、Unity 预压缩）
```

现有目录名不变，变的是内部分层。

### 5.2 契约层 `common`

- **模型**：`Project`（六组属性）、`Version`、`Session`、`Feedback`、`Follow`、`Player`、`Boost`、`Order`、`Plan/Limits`。投影 `ProjectCard / ProjectLive / ProjectPublic` 是 `Project` 上的方法，唯一实现。删掉 `api::Listing`、`plaza::PlazaItem`、`live::SiteLive` 三套并存；`follow::PushSubscription` 与 `notify::push::Subscription` 合一；`ingest::Me` 改名 `SessionIdentity`；`boost::NotificationQueue` 与 `db::QueueCounts` 合一。
- **路由常量**：`/v1/projects…`（从 `/v1/sites` 更名，§9.4），边缘转发与 ingest 不变。
- **生成**：`schemars`（已在依赖树）出 JSON Schema → `console/src/generated/*.ts`（构建脚本，CI 校验无 diff）与 `api` 的 OpenAPI 3.1（`utoipa` 或从 schema 手拼，选后者以少一个依赖）。控制台里「那边改了这里跟着改」的注释消失。
- **文案表** `common/i18n`：`Text` 枚举 + `zh` / `en` 两份表，CLI、边缘、邮件共用；控制台文案在 `console/src/words.ts` 由同一份表生成。
- **对象存储布局**（api 写、edge 读，唯一的读路径）：`projects/<slug>/v<N>.json`、`current.json`（含 `plan`、`limits`、`access` 的哈希）、`live.json`、`plaza.json`、`capabilities.json`、`blobs/<hash>`。`Store` 成为 trait：`FsStore`（本机 / 单机）与 `S3Store`（香港 S3 兼容），边缘本地盘做热缓存。

### 5.3 控制面 `api`

- **数据库：Postgres**（§9.5）。`sqlx` 编译期校验查询；迁移 `001_rewrite.sql` 一次建全（现有 SQLite 数据一次性导入脚本，只导用户、作品、版本、blob 索引，会话与反馈不导）。`db.rs` 按聚合切成 `db/{users,projects,versions,uploads,sessions,feedback,players,follows,notifications,boosts,orders}.rs`。
- **调度器**：一个 `scheduler` 模块，一个 tick（30 秒），按到期时间派发任务：过期清理（10 min）、blob GC（24 h）、周报（周一 09:00）、通知投递（30 s）、`plaza.json` 与 `live.json` 刷新（5 min，或事件触发立即）、推广上下位、配额月度重置。任务表 `jobs` 记上次运行与结果；运营页可见。
- **模块**：`auth`（匿名、GitHub 设备码 / 授权码、令牌撤销、reviewer）、`projects`（六组属性的 PATCH、rename、TTL、access）、`uploads`（prepare / blob / commit、裁版本、检查配额）、`versions`（list / activate / files）、`ingest`（边缘批量、SDK 事件与反馈、限速）、`results`（每版聚合、「现在」字段、点名册、导出）、`players`（关注、确认、退订、`pt_me`、推送订阅、关通知）、`notify`（入队与 24 h 合并、mailer log / resend / smtp、Web Push、周报）、`billing`（Checkout、Webhook、门户、退款、赠送）、`plaza`（`plaza.json`、举报阈值、隐藏）、`admin`（运营页 API）、`store`（写文件）。
- **删掉**：`cover/from-feedback` 整链（截图落地后再加）、`boosts.rs` 手写的 enum↔TEXT 映射（serde 已有）。
- **可观测**：`/metrics`（Prometheus 文本）、结构化日志（`tracing-json`）、每个后台任务的成功 / 失败计数。

### 5.4 边缘 `edge`

- **路由表**：`router.rs` 一张显式表（根域 8 条、子域保留路径 11 条、兜底清单分发），不再是 1,500 行的 `handle`。
- **渲染层** `render/`：一个模板函数族（壳、卡、按钮、字段、说明页）+ 一份 CSS（≤ 8 KB，测试守着）；门禁、分享、离线、熔断、到期、举报、广场、我的、确认、退订全部走它。删掉 `plaza.rs` 那 88 行覆盖、三份时间格式化、两份复制链接脚本、`pages::not_implemented`。`invite_verb` 只在 `common` 里一处。
- **缓存层** `cache/`：`Cached<T>`（单槽 + TTL）与 `CachedMap<K, V>`（上限 + TTL + 读不到给默认值），`current / manifest / live / plaza / capabilities` 五处用它；`manifest` 加上限。卡渲染缓存、隧道 `last_seen`、熔断计数保持各自语义。
- **写回**：`upstream.rs` 一个「连一次、发一个 JSON、拿状态码与字节」的函数，`follow` 与 `ship` 共用。
- **访问控制**：口令与邀请码在门禁页表单提交到 `/_playtest/start`，边缘从 `current.json` 里的 `access` 校验（口令哈希、邀请码 HMAC），通过才种门禁 cookie；`--isolated` 下同样生效（COOP/COEP 与口令不冲突：门禁页与作品同源）。
- **配额**：读 `limits`，本地计并发与字节，超限切页，60 秒报回。
- **卡**：保留 `card.rs` 的 SVG → resvg 路径；CJK 断行那 185 行换成 `usvg` 的文本度量（先 layout 一次拿宽度再换行），删掉自估字宽。
- **多边缘预留**：隧道注册表加 `edge_id`；`current.json` 里 `tunnel_edge` 字段，第二个区域上线时 DNS 按区域解析、控制面记录作品绑在哪个边缘。v1.0 单边缘。

### 5.5 CLI

- `commands/*.rs` 每个子命令一个文件，返回 `Report`（枚举：`Published / Online / Listed / Versions / …`）；`render/{human,json,mcp}.rs` 三个渲染器；`ls/rm/open/card/versions/rollback/unlist` 只实现一次。
- `args.rs` 按 §3.1 重排；`--api` 全局；`resolve_target`（slug 或目录）与 `saved_client` 各一处；`config` 记「目录 → slug」映射。
- `inspect/` 保留，`is_generic_dir_name` 与 `is_template_title` 合成一份「不像作品名」清单；新增两项检查。
- `mcp.rs` 工具表由 `commands` 派生，`playtest_share` 长驻（stdio 会话内保持隧道）。
- `i18n`：`--lang` / `PLAYTEST_LANG` / `LANG`；所有输出走文案表。
- `--watch`：目录变化 debounce 2 秒发新版本，`--note` 复用上次并加「(自动)」。

### 5.6 控制台

Preact 不变。`src/generated/` 来自 `common`；`pages/{home,project/{results,roster,feedback,card,settings},account,admin}`；去掉令牌粘贴页（改 GitHub 登录，CLI 令牌只在账号页管理）；`dev/mock-api.mjs` 删除——本机跑真控制面（`scripts/e2e.sh` 一键起三进程并灌种子）。

### 5.7 部署与运维

- `deploy/compose.yaml`：caddy、edge、api、postgres、console（静态）、docs（静态）、backup（每日 `pg_dump` + 对象存储同步到第二个区域）、node-exporter / 一个轻量 Prometheus + Alertmanager（或托管 Grafana Cloud 免费档，二选一在 M6 定，不影响代码）。
- 告警：控制面 5xx 率、边缘 5xx 率、通知死信 > 0、发信投诉率 > 0.1%、磁盘 > 80%、证书 < 14 天、备份失败。
- 状态页：`status.playtest.roviix.com`（静态，运营手动或托管 Uptime 服务）。
- 日志保留 14 天，不记 IP 到持久层。

### 5.8 安全与滥用

不运行用户代码；`*.playtest.run` 提交 PSL；根域 CSP `default-src 'none'`；作品域不加 CSP（不改开发者字节）；`pt_me` host-only；控制台 cookie `SameSite=Lax; Secure; HttpOnly`；API 令牌只存哈希；令牌撤销立即生效（边缘不认令牌，隧道令牌有驱逐名单）；ingest 与 follow 按来源令牌桶；留名与反馈过脏词表；举报 3 会话 / 24 h 自动撤下；推广先审后上；发信 SPF / DKIM / DMARC、一键退订、投诉率阈值自动停发；上传扫 `.exe/.dmg/.apk` 等可执行分发（拒，人话说明）；slug 黑名单；安全披露渠道 `security@`。

### 5.9 测试策略

- 单元与集成按 crate（现有 7,000 + 6,000 + 3,000 行保留并随重构迁移）。
- **契约测试**：`common` 的 JSON Schema 快照；TS 生成物 CI 无 diff；OpenAPI 与路由表一致性测试。
- **三进程端到端** `scripts/e2e.sh`：起 api（Postgres 容器）+ edge + 用 CLI 发布 → 卡 PNG 尺寸 → 无头 Chrome 走门禁留名 → 关注（log mailer 抓确认链接）→ 确认落 `/me` → 发 v2 → 队列出现通知 → 广场与稀疏态截图 → 赠送推广出「推广」标 → 口令作品拦截与放行 → 熔断切页 → 控制面停掉后门禁、卡、广场照常。CI 每次跑。
- **金图**：门禁、卡（竖横）、广场（稠密 / 稀疏 / 手机）、控制台五签，像素对比阈值 0.5%。
- **真机记录**：每个里程碑收尾记一条 spike（真手机、真微信、真邮箱），是记录不是闸门。

## 6. 施工顺序

原则：**按名词穿层，不按层并行。** 每个里程碑交付的是一条从 CLI 到控制台都通的线，里程碑内可以并行不同名词。每个里程碑结束：workspace 绿、e2e 绿、一条 spike、推香港。

| 里程碑 | 交付 | 验收 |
| --- | --- | --- |
| **M0 · 契约与骨架**（1 周） | 本文 §2–§4 回写 DESIGN；`common` 模型与投影、路由更名、文案表、TS / OpenAPI 生成与 CI 校验；Postgres 与 `sqlx`、迁移、导入脚本；调度器骨架；边缘路由表、渲染层、缓存层、`upstream`；CLI `Report` + 三渲染器骨架与新 `args`；`scripts/e2e.sh` 骨架；修绿现有红测试 | 全绿；现有真机能力（上传 / 隧道 / 混合 / 门禁硬线 / 熔断 / 回滚 / 登录）在新骨架上一条不掉 |
| **M1 · 发布原语到底** | `--to` / `rename` / `--ttl` / `--watch` / `--password` / `--invite`（Pro 判档位）；配额与并发执行点、月度重置、令牌撤销、`whoami` 用量；读回文件；上传前两项新检查；MCP 八工具一一对应、`share` 长驻；双语 CLI | e2e：匿名 → 登录归入 → 自选名字 → 口令拦放 → 配额切页；spike：Godot 线程 / 无线程、Unity 预压缩、Phaser、Vite 四个真实导出物两条路各过一遍 |
| **M2 · 邀请环与回访环** | 门禁页全部态、卡、分享、留名、关注（邮箱 + 推送）、`/me` 含关通知、确认 / 退订、通知合并、周报、mailer 三实现、`?from` 来源；SDK `ready / identify` 与落点 | e2e 全链路；spike：真微信群一张卡、真手机扫码留名关注、真邮箱收到 v2 通知并点进、点名册「来自通知」 |
| **M3 · 结果与控制台** | 「现在」字段、每版聚合、点名册名字与隐藏、反馈公开与隐藏、导出与版本比较（Pro）、五签、作品墙、账号页（用量、设备与令牌）、GitHub 网页登录取代令牌粘贴 | 金图；spike：两台真手机各玩一次，控制台那段话每个数字对得上 |
| **M4 · 注意力环与计费** | 广场（推广位、求测、时间序、稀疏、空态）、举报与撤下、推广数据模型与排期、Stripe Checkout / Webhook / 门户 / 退款 / 赠送、订单与发票、档位切换重写 `current.json`、控制台设置与账号页的购买入口（无密钥时「尚未开放」开关） | e2e：赠送出标、Stripe 测试模式走通订阅与推广、拒审退款；spike：广场稠密 / 稀疏 / 手机 |
| **M5 · Agent 面与文档** | `llms.txt` / `skill.md` / OpenAPI / `agent.json`、文档站中英（含法务页）、README 英文、`playtest mcp --setup` 三种工具配置、`npx skills add` 可装、根域 `llms.txt` | spike：Cursor、Claude Code、Codex 各一次「发给朋友玩」出链接 + 卡；一个只读文档的 agent 用 curl 三步发布成功 |
| **M6 · 运营与上线** | 运营页、metrics、告警、备份与恢复演练、状态页、PSL 提交、发信域 DNS、S3 存储切换、压测（三网晚高峰一周）、v1.0 tag 与三平台 Releases、flip public | §1.2 全部数字有记录；恢复演练一次成功 |

M0–M1 不新增任何玩家侧名词；M2 之前广场视觉冻结在当前版。

## 7. 施工纪律

- 一个名词一个工作流，从 CLI 到控制台穿到底；不按 crate 分层并行。
- `common` 改了先跑生成，TS 与 OpenAPI 的 diff 与 Rust 改动同一个提交。
- 每条 CLI 输出、每页文案先写进文案表再用，中英同时。
- 不放不能点的按钮；等外部物件的功能是一个开关，开关关着时如实写「尚未开放」。
- 每个里程碑收尾一条 spike，写清「验到的」与「没验的」。
- 提交信息一句话说为什么，中文。密钥不入库。

## 8. 外部物件（不阻塞施工，只阻塞开关）

| 物件 | 影响的开关 |
| --- | --- |
| 境外主体 + Stripe（含支付宝） | `PLAYTEST_STRIPE_*`：Pro 与推广开卖 |
| 邮件服务商 + `notice@playtest.run` 的 SPF / DKIM / DMARC | `PLAYTEST_EMAIL_PROVIDER=resend|smtp`：线上发信 |
| GitHub OAuth App 凭据 | 登录（本机可用测试 App） |
| 香港云与 S3 兼容存储 | `PLAYTEST_STORE=s3` |
| 推广对大陆用户的法律意见 | 推广对大陆账号的开关 |
| 运营者的时间 | 举报有人看、周报有人读 |

## 9. 决定与为什么不是另一个

1. **结构重写而非从零重写。** 边缘的隧道、硬线、熔断、Range、预压缩是几十条集成测试和多次真机换来的；从零重写会把这部分正确性再赢一遍。要换的是分层方式，不是字节。
2. **一个原语。** here.now 的完成度来自「一切是 Site 的属性」；我们 09-09 一天长出九个并列名词，每个到一半。把它们收成作品的六组属性，加字段只改一处，控制台类型跟着生成。
3. **按名词穿层。** 09-09 按 crate 并行留下 4 条红测试、三套展示态、两套 CSS、两个 HTTP 客户端。09-07 按线穿层那轮没有这些。
4. **`/v1/sites` 更名 `/v1/projects`。** API 要对 agent 可读并写进 OpenAPI，`sites` 是托管商的词，和门禁页上「作品」、CLI 里「作品」不一致。更名是机械改动，趁契约层重写一次做完。
5. **Postgres 而非 SQLite。** 今天已经是并发写入：6 个循环、边缘写回、以后的 Webhook 抢一把互斥锁。商业化之后再迁移是在有真实数据时迁，现在迁是零成本。运维多一个容器，用托管 Postgres 可消掉。
6. **`--seek` 并入 `--note`。** 「想让人看什么」和「这版改了什么」对开发者是同一句话，对玩家也是同一句话（门禁页上、通知里）；三个 140 字文本变两个。
7. **不做自定义域名。** §1.2 里没有一个时刻需要它：试玩链接活几天到几周，引擎导出物用绝对路径也不在意域名；它要 Caddy 按需签证书与域名验证，是一整块新面。here.now 做它因为它是托管商。Pro 卖的是自选名字，不是域名。
8. **不做团队 Workspace，做 3 位只读 reviewer。** 发行商、老师、评委看结果是真实场景；共同发布不是——发布是一条命令，谁敲都一样。
9. **不做 Drive / Site Data / Proxy。** Site Data 与 Proxy 是半个后端，触到「不运行用户代码」的边；有后端需求的人走 `--backend`，这正是我们比 here.now 多出来的那条路。
10. **双语进 v1.0。** 引擎社区、Game Jam、AI 工具的用户一半在英文世界；agent 读的文档必须英文；只中文等于放弃 `playtest mcp` 这条最大供给管道的一半。
11. **玩家域零脚本不变，控制台不追求零脚本。** 门禁页是信任凭证，快与干净是它的全部；控制台是登录后的工作台。
12. **验收是数字加记录，不是等待。** 每个里程碑收尾记一条 spike 是为了不虚报；没有任何一条设计决定挂在「验证之后再定」上。真机结果推翻某条决定时，改 DESIGN，不改本文的施工方式。
