# Playset 与「playtest 结果」这一类产品：深挖

调研日期：2026-09-07。本文只写这一条线，不改仓库其它文件。

---

## 0. 这份报告怎么读

**来源分级**（每条事实后面都会标）：

- 〔一手〕官网、定价页、文档、changelog、GitHub README、公司公告，附 URL。
- 〔原话〕论坛 / 评论 / 博客里的用户或从业者原文，附 URL。
- 〔二手〕第三方报道、聚合站、公司资料站、检索引擎抽取的页面正文（原页面需 JavaScript，正文未逐字核对）。
- 〔推断〕我从上面这些推出来的，不是任何人说过的。

**本轮的两个硬限制，先说清楚：**

1. **上一轮抓取的原始材料没有落盘。** 12:25 左右那一轮被平台用量上限中断，中间结果没有写进文件。本轮用 8 次针对性检索 + 2 轮 HN Algolia API 查询重建，并复用了同日其它调研线**已带来源**的一手抓取（引用时会标明「另一条调研线一手」，指 `docs/research/2026-09-07-playtest-feedback-analytics.md`、`-business-models.md`、`-same-thing-better.md`）。凡是我本轮没有独立复核的数字，都标了「需复核」。
2. **playset.app 与多数同类站点是需要 JavaScript 的动态页面。** 我拿到的是检索引擎抽取的页面正文，字面可信、但**结构可能被截断**（比如定价表只抽到了免费档）。凡属这种情况，我标〔一手，检索抽取〕并注明截断位置。

---

## 1. Playset：一手能看到的全部

官网：<https://playset.app/>

### 1.1 它自己怎么说

〔一手，检索抽取，2026-09-07〕标题是 **「Playset — Playtesting, in one click」**，首屏三句话：

> **Frictionless playtesting. Auto-recorded video. Timestamped feedback.**
> Everything you need to run real playtests in minutes — without stitching together a dozen tools.

四个卖点，原文：

| 卖点 | 原文 |
| --- | --- |
| One-click uploads | 「Drop in a web build (.zip), get an instant hosted link you can send anywhere — Discord, email, X, you name it.」 |
| Complete Video & Screenshots | 「Every playtest includes a full video recording of the session, and players can capture screenshots directly from the browser.」 |
| Timestamped feedback | 「Testers log bugs, ideas, and react to moments while they play with frictionless controls — all pinned to the exact moment in the session.」 |
| Live & async | 「See playtest events as they come in and chat with players in real time, or review the full session timeline with video later.」 |
| No SDK, no install | 「Everything runs in the browser. No tracking SDKs, no extra build targets — just upload, share, and test.」 |

还有两条对我们有用的结构性描述〔一手，检索抽取〕：

- **「One link per game, with builds and sessions in one place.」**——链接是**按游戏**发的，版本（builds）挂在这个链接下面。这和 DESIGN §3.2 的「一个 slug 一个作品、版本在里面滚动」是同一种建模。
- **「Native builds coming later — starting with web keeps things frictionless for you and your testers.」**——路线图上是往原生走，不是往命令行走。

引擎支持〔一手，检索抽取〕：Godot 导出 HTML5 打 zip；Unity WebGL「work out of the box」；自定义引擎 / 裸 JS 只要能打成 zip 都行。

### 1.2 开发者流程与玩家流程：我能确认的和不能确认的

| 环节 | 结论 | 来源 |
| --- | --- | --- |
| 上传形态 | 拖一个 `.zip`，得到托管链接 | 〔一手，检索抽取〕 |
| 链接形态（子域 / 路径） | **没查到**。官网文案只说「an instant hosted link」，没给示例 URL | — |
| 版本概念 | 有，「builds and sessions in one place」 | 〔一手，检索抽取〕 |
| 是否有 CLI | **一手材料里完全没有 CLI、API、命令行的任何字样**；反复强调的是「Drop in」「in the browser」「No SDK, no install」 | 〔一手，检索抽取〕 |
| 是否有隧道 | 无任何迹象 | 〔一手，检索抽取〕 |
| 文件大小上限 / 支持的最大 zip | **没查到** | — |
| COOP/COEP、Brotli 响应头怎么处理 | **没查到**。官网只承诺「Unity WebGL works out of the box」，没有一句关于响应头、SharedArrayBuffer、预压缩的说明；也没找到文档站 | — |
| 移动端能不能玩 | **没查到**，官网未提 | — |
| 玩家要不要登录 | **没查到**。文案说链接可以「send anywhere — Discord, email, X」，暗示不需要，但没有明确写「no account required」 | — |
| 录像的授权提示怎么写、隐私声明 | **没查到**。这是本轮最想看但没看到的一页——一个「全程录像」的产品，它怎么向玩家开口，正是我们判断「录像会不会吓跑玩家」的关键证据 | — |
| 有没有中间页 / 门禁页 | **没查到** | — |

### 1.3 定价与计量

〔一手，检索抽取〕免费档字面：**「15 mins of playtesting each month」**，定位是 **「Ideal for early prototypes, jam games, or a small beta group」**，条目里还有「Full recordings & feedback timeline」和一条以「Any number ...」开头、被抽取截断的条目（**所指没查到**）。定价页整体强调 **「Built for indie budgets. No four-figure enterprise contracts just to watch someone play your game.」**

〔二手，需复核〕$19 / $49 两个付费档（约 10 小时 / 50 小时试玩分钟，$49 档 3 个游戏），免费档 1 个游戏、1 GB——这组数字来自另一条调研线从站点检索索引里取到的，该报告自己也标注了「需人工再核」。**我本轮没有独立复核到这两档的正文**，因此 DESIGN §2.1 里写的「$0 / 19 / 49」目前是一个**未经一手确认**的数字。

**计量单位是「试玩分钟」这件事本身是确认的**〔一手〕：免费档卡的是分钟，不是上传次数、不是流量、不是测试者人数。这一点后面 §4(a) 要用。

### 1.4 团队、阶段、活跃度

- **「纽约两人团队，2025」**（DESIGN §2.1 的写法）：**本轮没查到任何来源。** 没有 About 页、没有 LinkedIn 公司页、没有创始人访谈、没有融资记录进入我的检索结果。这句话在 DESIGN 里现在是没有出处的。
- 更新频率 / changelog：**没查到**，没有找到 changelog 或 blog。
- 公开用量信号（PH 票数、Discord 人数、X 粉丝、用户案例）：**全部没查到**，见下节。

### 1.5 用户声音：一次彻底的空手而归

这是本节最重要的发现，所以把方法写清楚，便于以后复核：

**Hacker News（Algolia 全库检索 API，2026-09-07）**〔一手〕：

| 查询 | 结果 |
| --- | --- |
| `playset` | 33.7 万条命中，但按相关度排序的头部全部是噪音：1951 年的 U-238 原子能玩具套装、NSA Playset、Cubetto 玩具。**没有一条指向 playset.app。** |
| `playloop` | 1.46 万条噪音（Common Lisp 的 CEPL 项目「Code, Eval, Play, Loop」）。**没有一条指向 playloop.gg。** |
| `playtesting tool` | **全库总共 38 条命中。** 其中和本品类沾边的只有 [Show HN: Vlly – Turn all your users into playtesters \[VR\]](https://www.usevlly.com/)（2023-08-25，**2 分**）。 |

`playtesting tool` 在 HN 十几年历史里总共 38 条命中，这个数字本身就是一条发现：**这一整个品类在 HN 上几乎不存在。** 顺带说明，我们如果指望 Show HN 作为冷启动渠道，要预期这个词在那里没有受众。

**Product Hunt**〔一手，检索〕：搜不到 playset.app 的 PH 发布页。返回的是官网本身，加上同名的桌游 App「Playsets」等无关产品。**没查到 Playset 的 PH 发布记录**（不能证明不存在，只能说搜不到）。

**Reddit / itch 论坛 / X / YouTube 评论**：另一条调研线做过一轮同样的检索，结论也是零〔另一条调研线一手〕：「我没有查到任何 Playset 的独立用户评价。没有 Reddit 讨论、没有 HN 帖、没有 itch 论坛提及、没有 Product Hunt 评论。」我本轮在 HN 和 PH 两处独立复核，结论一致。

**这条结论要说准**：找不到评价 ≠ 没有用户。但它意味着 **DESIGN §2.3 那句「Playset 的存在……也证明这条路上已经有人在收钱」目前没有任何证据支撑**。准确的说法是「已经有人在这条路上**标了价**」。

---

## 2. Playloop：团队、阶段、版本

官网 <https://playloop.gg/>，本轮抓到了它的 `/about`、`/status`、`/changelog`、`/pricing`、`/docs/credits` 五页，是本轮一手信息最完整的一家。

**团队与来历**〔一手，<https://playloop.gg/about>〕：

- **「Playloop is built and run by Turtlevania Games LLC, an independent game studio.」** 创始人一人身兼三摊：「Runs Playloop, Wander & Wilt Studios, and Turtlevania Games.」
- 起因原文：「In 2022 I started Turtlevania Games with a friend and went all in on a game called Tyro... it taught me the lesson every indie dev learns the hard way: it is really difficult to know what your players actually feel. **They quit silently. They get confused silently.**」
- 「The tool started as an internal thing for our own playtests, then **escaped**.」
- 团队规模：只说「The team is small. We don't maintain a manufactured "Team" page with stock photos.」**具体人数没查到。**

**阶段与版本**〔一手〕：

- `/about` 与 `/status` 都写 **「Waitlist, v0.5.0」**。五个引擎的 SDK（Unity / Unreal / Godot / Python / TypeScript）已上线，dashboard 已上线，**Indie 与 Studio 付费档「launch from the waitlist」——也就是还没开卖**。
- `/status` 最后自检时间 **2026-08-27 13:39:38 UTC**，全绿；并且写着「We're a small team, so we don't run an on-call rotation yet.」
- `/docs/credits` 页顶：「**Playloop is preparing for open beta.** The docs are here early; we're still building and polishing, so a few details may change.」
- **一处自相矛盾，值得记**：`/about` 说「We ship a couple times a week; **the changelog is the source of truth**」，但 `/changelog` 页面正文是 **「No releases yet — We're still in pre-v1 beta」**。〔一手，两页对照〕

**定价**〔一手，<https://playloop.gg/pricing>〕：Indie $49/月（年付 $490，省 17%）、Studio $99/月（年付 $990，折合 $82/月）。页面明确写 **「Paid plans aren't out yet, they're on the waitlist.」** 新账号有 14 天 Indie 试用 + 3,000 credits，不要信用卡，而且**试用倒计时从你跑第一次 playtest 才开始算**，不是注册日〔一手，`/docs/credits`〕。加购 credit pack 从 $25 起，「Showcase pass」从 $59 起（一个月，非订阅）。

**免费档给什么**〔一手，首页〕：自带 OpenAI / Anthropic / Gemini key、完整的三层 AI 摘要（session → tester → build）、live dashboard、event timeline、replay、游戏内 Player Feedback 表单、单文件最多导入 2,500 个 session（PlayFab / Mixpanel / GameAnalytics / Amplitude / Unity / CSV）、webhooks、**MCP server**、Management API、Slack + Discord ingest、无限 session / build / tester、1 GB 存储、30 天保留。

> 注意：另一条调研线在同一天抓到的是「5 GB · 30 天 · 2 seats」，我今天抓到的首页写「1 GB storage, 30-day retention」。**两个数字对不上，说明这家的免费档正在改。** 引用任何一个都要标日期。

**它对自己的定位**〔一手，首页原文〕：「Built for indie studios who'd rather ship than **scrub through a four-hour recording**.」——一家做 playtest 结果的公司，把自己的对手设定成「录像」。

**对 DESIGN 的直接影响**：§2.1 把 Playloop 归在「Playtest 结果」一行，缺的那一格写「都没有隧道、没有 CLI 工作流」。前半句成立，**后半句对 Playloop 不成立**：SDK 覆盖五个引擎、有 Management API、有 MCP server、有 webhooks，这已经是一套面向开发者工作流的接口了。准确的说法是「入口都是网页 dashboard，**没有一家以命令行为入口**」——见 §4(c)。

---

## 3. DESIGN §2 没有列的那一批

DESIGN §2.1「Playtest 结果」那一格现在只有四家：Playset、Playloop、Antidote、PlaytestCloud。2025–2026 这一片实际比这拥挤得多。下表是我这一轮确认到的、DESIGN 里没有的：

| 产品 | 是什么 | 核心机制 | 玩家侧门槛 | 定价（2026-09-07） | 阶段 / 规模信号 | 来源 |
| --- | --- | --- | --- | --- | --- | --- |
| **Playcocola** | 面向 indie 的 playtest 平台 + 测试者社区 | 浏览器内录屏 + 语音「thinking aloud」+ 文字反馈，开发者建一个 play session 发邀请链接 | 「Browser-based (Zero playtester friction)」，不装软件不注册 | Hobbyist **€0**（1 小时录像上限、自带测试者、视频存 1 周）；Indie **€2 / 20 分钟**；Premium **€7 / 20 分钟**（他们帮你招人 + 不满意退款） | 2 人，2022 年成立，德国 + 印度；月访问约 2,068〔二手，公司资料站〕 | 〔一手〕<https://playcocola.com/>；〔二手〕<https://linkedin.com/company/playcocola>、<https://gamesuserresearch.com/top-remote-playtest-platforms-for-unmoderated-testing/> |
| **Playruo** | 面向发行商 / 工作室的**云端串流** playtest | 把 PC 构建关在受控 Windows 虚拟机里串流给测试者，浏览器打开；每 session 取证水印、审计轨迹、NDA、口令、地理封锁、时间窗、随时吊销 | 「no app install, no account creation, and no local game download」 | **没查到**（白标 / 询价） | 有署名案例：Tara Gaming 用它给《The Age of Bhaarat》把外包 AAA playtest 成本降到 1/3、准备时间减半（2026-04-10） | 〔一手〕<https://playruo.com/playtests>、<https://playruo.com/lp/playtests> |
| **Pretty Good Playtesting (PGPT)** | 单人做的 indie playtest 工具箱 | **一键把 OBS 塞进你的构建**（自调的 OBS build），加问卷、管测试者、自动出报告；「NDA included or BYO、Anonymised Tester data、Encrypted Builds」 | 要装带 OBS 的构建，**Windows only** | 现在**免费公开 alpha**；计划 hobbyist **$15/月**、studios **$30/月**，年付 8 折 | 作者 Zac Lucarelli（同时做 Poly.Pizza）；桌面客户端仓库 [Chikanz/pgpt-client](https://github.com/Chikanz/pgpt-client) **1 star**、2023-08-17 建、GPL-2.0；官方 Discord **27 人**（2024-04-09 建）〔二手，disdex〕 | 〔一手〕<https://pgpt.dev/>、<https://duck.pizza/> |
| **Slaytester 2** | **自托管、开源**的网页游戏录像 | 你把它跑在自己服务器上，建一个 playtest，**把一行 HTML 粘进游戏 `index.html` 的 `<head>` 顶部**；经明确同意后录 canvas、游戏音频、玩家麦克风 | 玩家点一个按钮，同意后开录 | 免费（自托管，Deno + Fresh） | 个人项目，作者自述已经是第三或第四次重写 | 〔一手〕<https://github.com/cannonheart/slaytester2> |
| **LoopKit** | Unity 的 playtest + session replay | SDK 采集**快照帧**（不是视频）+ 玩家操作 + 系统事件（FPS、内存、设备/GPU、错误日志），拼成回放 | 要在游戏里做 opt-in 同意，支持按地区强制 opt-in、本地化同意文案、玩家可暂停/停止采集、导出/删除请求 | **beta 期免费**，付费档「later」 | 目前**只支持 Unity**，Unreal / Godot 在路线图上 | 〔一手〕<https://loopkit.dev/> |
| **Live Aware** | 面向工作室的「素材汇聚 + AI 洞察」 | 把 Discord / Twitch / YouTube / 内部 playtest 的录像和讨论汇到一处，自动转写、自动生成 insights dashboard；有桌面录制器，也有 Unreal / Unity SDK 带 telemetry（状态、事件、FPS、输入，与视频时间轴自动对齐）；对接 Jira / Notion / Monday | 玩家要用它的录制器或 OBS 推流 | **没查到**（走 demo / 询价） | 有具名案例：Treehouse Games 自 2024 年中起用于双周 playtest，此前要「20+ 小时把 400 名玩家的素材剪成 15 分钟合集」；Lunchbox Entertainment 用于《Sirocco》 | 〔一手〕<https://home.liveaware.io/>、[Treehouse 案例](https://home.liveaware.io/case-studies/treehouse-games)、[Lunchbox 案例](https://home.liveaware.io/case-studies/lunchbox-entertainment)、[SDK telemetry 公告](https://home.liveaware.io/blog/sdk-telemetry) |
| **Bugnet** | **给 indie 游戏的错误追踪**（对标 Sentry） | 游戏内报错组件 + 崩溃堆栈 + 设备信息 + **session replay（录的是鼠标移动、点击、按键，不是视频）**；自动在 GitHub/GitLab 建 issue 并按频次 × 严重度排序；同步 Steam 评论并可从后台直接回复 | 玩家在游戏里点组件填一句话 + 截图 | **没查到完整档位**（免费档抽取被截断，只看到「2 team members、in-game widget」等碎片） | 官方 SDK 覆盖 Unity / Unreal / Godot / GameMaker / Pygame / **web-HTML5**，另有 120+ endpoint 的 REST API；自述 340+ 工作室、1.2M+ bug 报告、73% 回归率、38% 降流失——**全部厂商自述，无第三方验证** | 〔一手〕<https://bugnet.io/>、<https://bugnet.io/about>、<https://bugnet.io/docs/sdks> |
| **FirstLook** | 「player relationship platform」，playtest 只是其中一块 | 跨 Steam / Epic / 主机 / 移动发 key、NDA 封测 + 候补名单 + 好友邀请、在 Discord / 游戏内 / 网页发问卷、聚合 Discord + Steam 评论做情感分析、创作者计划 | 玩家要有 FirstLook 账号（有 player 门户，可浏览并报名 playtest） | **对 indie 免费，上限 500 名玩家** | **1.0 于 2026-02-17 正式发布**（洛杉矶）；changelog 可见节奏：2025-07-08 问卷、2025-09-23 问卷答案筛选、2025-12-15 情感分析、2026-04-07 Unity SDK。Unity 包名是 `com.pragma.firstlook.*`，文档索引挂在 `pragma` 名下——〔推断〕它是 Pragma（游戏后端基础设施公司）的产品 | 〔一手〕<https://firstlook.gg/>、<https://docs.firstlook.gg/developers/sdk/unity/>；〔二手〕[Games Press 发布稿](https://www.gamespress.com/THE-GAMES-INDUSTRYS-FIRST-PLAYER-RELATIONSHIP-PLATFORM-ARRIVES-WITH-TH) |
| **The Playtest** | 双边的「有组织 playtest」平台 | 开发者发布项目、设定对测试者的要求；测试者按平台 / 类型 / 地区筛选并申请；结构化反馈表单；内置 AI 助手 **CHAP** 帮忙匹配测试者、生成表单、汇总洞察 | 测试者要注册、建档案、申请 | 「No upfront cost... **Free forever for small teams**. No credit card needed.」 | 页脚 © 2026，是这批里最新的一个。自我定位原话：「**Not a marketplace or social network.** A focused tool for structured playtesting.」 | 〔一手〕<https://theplaytest.com/> |
| **Insights.gg** | **不是 playtest 工具**，是电竞复盘 | Windows 录制器自动识别击杀 / 死亡等高光，网页端 VOD review：打时间戳、画图、评论、和教练一起看 | 面向玩家和战队，不面向开发者 | **没查到** | 列在这里是为了**防止以后有人再把它当成对手**——它和「开发者要知道自己这一版怎么样」没有关系 | 〔一手〕<https://insights.gg/> |

**顺带撞到的、同一片市场但更偏「招募」的三家**（不展开，仅留名字与一手链接，便于以后需要时接上）：[Userplay](https://userplay.io/)（「TestFlight for PC Games」，Discord 招募 + session replay + 语音转写）、[Go Testify](https://www.gotestify.com/)（自述 5,000+ 团队）、[Playtester Network](https://playtester.net/)（自述 75 万测试者，属于 Partnier 营销生态，用游戏 key 等实物奖励代替现金）。

### 3.1 这张表能看出三件事

**第一，这一类的商业模式已经分成了三种，而 DESIGN §2.1 只描述了其中一种。**

| 模式 | 卖什么 | 谁在用 |
| --- | --- | --- |
| **按录像分钟卖** | 视频存储与带宽的成本转嫁 | Playset（15 分钟/月免费）、Playcocola（€2 / 20 分钟） |
| **按 AI credit / 席位 / 容量卖** | 模型账单 + 保留期 + 协作 | Playloop（$49/$99 + credit pack）、Bugnet、LoopKit、PGPT（$15/$30 计划中） |
| **按「帮你找到人」卖** | 测试者招募与筛选 | Playcocola Premium、The Playtest、FirstLook、Playtester Network、Go Testify，以及 DESIGN 已列的 Antidote / PlaytestCloud |

**第二，「玩家零门槛」在这一类里已经是共识，不是差异化。** Playset「No SDK, no install」、Playcocola「Zero playtester friction」、Playruo「no app install, no account creation, no local game download」——三家不同价位段的产品用几乎相同的措辞在承诺同一件事。DESIGN §0.3 把「玩家零门槛」列为核心原则是对的，但它是**入场券**，对外文案不该当卖点讲。

**第三，「录像」正在被免费化和自托管化。** Slaytester 把「一行 HTML 就能录 canvas + 音频 + 麦克风」做成了开源自托管；PGPT 免费公开 alpha；LoopKit beta 期免费。**一个正在被开源工具白送的能力，不适合作为收费点，也不适合作为「我们不做所以我们不同」的差异化叙事**——见 §4(d)。

---

## 4. 重点回答

### (a) Playset 的用户是为「录像」付钱，还是为「一键把版本给人玩」付钱？

**需求侧无法回答：我找不到任何一个 Playset 用户。** 见 §1.5。所以下面全是从**供给侧的定价结构**做的〔推断〕，请按推断使用。

**它自己的答案是「录像」。** 判据是免费档卡在哪：Playset 免费档卡的是**试玩分钟**，不是上传次数、不是游戏数量、不是流量、不是测试者人数〔一手〕。**一个产品把什么设成稀缺资源，就是它认为什么值钱；把什么白送，就是它认为什么留不住人。** 按这个读法，Playset 认为「一键把版本给人玩」是获客免费品，「录像」才是商品。

三条旁证：

1. **横向看计价单位。** 只有两家按分钟计价，恰好就是两家以视频为中心的（Playset、Playcocola）。不录视频的几家（Playloop、Bugnet、LoopKit、PGPT）全都改按 credit / 席位 / 容量 / 月费。**按分钟计价不是价值定价，是视频存储和带宽的成本转嫁。**
2. **卖录像的公司自己在贬低录像。** Playloop 首页把对手写成「scrub through a four-hour recording」〔一手〕；Live Aware 的招牌案例是「以前要花 20+ 小时把 400 名玩家的素材剪成 15 分钟合集」〔一手〕；另一条调研线记下 PlaytestCloud 的原话是「If you don't have time to watch all of your recordings, use these tools to see which moments to zoom in on」〔另一条调研线一手〕。**整个下游生态都在卖「怎么少看点录像」——这是一个产品在给自己造麻烦。**
3. **「一键给人玩」在英文世界很难收钱。** 另一条调研线在 itch 论坛挖到的原话，来自一位做浏览器多人游戏的开发者〔原话，另一条调研线一手〕：「构建分发对我来说早就解决了。它就是一个 URL。一个核心卖点是『管理构建、发给测试者』的工具对我完全没有东西可卖。」

**给我们的结论**：不要指望「送过去」这一步能收钱，也不要以为「录像」是一个已经被验证的付费需求——**Playset 只是给它标了价，没有任何证据说明有人在买。**

### (b) 它的免费用户在抱怨什么？

**本轮无法回答，因为找不到免费用户。** 零 Reddit、零 HN、零 Product Hunt、零 itch 论坛。这条要在 DESIGN 里如实体现，不能把「按结构推出来的抱怨」写成「用户在抱怨」。

**能说的只有结构性判断**〔推断〕：

- 免费档 15 分钟/月〔一手〕，按 DESIGN §8 设想的一次 30 人测试，**免费档连第二个人都看不完**。这是把免费档当 demo，不是当入口。
- 按分钟计费在 playtest 场景是**反向激励**：开发者希望玩家玩得久、玩得深，付费模型却按这个惩罚他。
- 一个「全程录像」的产品必然要面对玩家侧的授权与隐私，而**我没有找到它的隐私声明或授权提示文案**（§1.2）——这既是我没查到，也说明它没有把这一页当成对外的卖点来展示。

### (c) 有没有隧道或 CLI 的迹象？

**Playset：两者都没有。**〔一手〕官网通篇是「Drop in a web build (.zip)」「Everything runs in the browser」「No SDK, no install」，没有一处提到命令行、API、CI 或本地端口。唯一的路线图信号是「Native builds coming later」——**它在往「支持更多构建形态」走，不是往「进入开发者的命令行」走。**

**但这一类整体上，「开发者接口」已经出现了**：

| 产品 | 开发者接口 | 入口是什么 |
| --- | --- | --- |
| Playloop | 五引擎 SDK + Management API + **MCP server** + webhooks + Slack/Discord ingest | 网页 dashboard（先建项目、拿 key、接 SDK） |
| Bugnet | 六引擎 SDK（含 web/HTML5）+ 120+ endpoint 的 REST API | 网页 dashboard |
| FirstLook | Unity / Unreal SDK + scoped API tokens | 网页 dashboard（SDK 要从 dashboard 里下载 tarball） |
| Slaytester | 一行 HTML | 自己的服务器 + 网页管理页 |
| Playset / Playcocola / The Playtest / Live Aware / Playruo | 无公开开发者接口（Live Aware 有 SDK 但要申请名额） | 网页 |

**两条结论：**

1. **隧道：全类目零。** 这一类里没有任何一家碰本地 dev server。DESIGN §2.1「都没有隧道」这半句完全成立，而且比 DESIGN 写得更强——连相邻的招募类、录像类、错误追踪类加起来十几家，一家都没有。
2. **CLI：DESIGN 的措辞需要改。** 「都没有 CLI 工作流」对 Playloop、Bugnet、FirstLook 已经不成立。准确的说法是：**有 SDK 和 API 的几家，入口仍然是网页 dashboard——先注册、先建项目、先拿 key，然后才轮到代码。没有任何一家的第一步是在终端里敲一条命令。** 这个格子还是空的，但空的理由和 DESIGN 现在写的不一样。

### (d) 这对我们「结果层做轻、不录像、CLI 为中心」意味着什么？

**「不录像」——决定对，理由要换。**

DESIGN §2.2 现在的理由是「Playset 以录像为核心，重、贵、隐私敏感」。这个理由在 2026 年已经不够硬了，因为**录像正在变成免费品**：Slaytester 一行 HTML 就能录 canvas + 音频 + 麦克风且完全自托管、PGPT 免费公开 alpha、LoopKit beta 免费。更硬的三条理由是：

1. **它正在跌价到零**，不是可防守的收费点，也不构成别人的护城河（所以「我们不做」也不构成我们的差异化）。
2. **卖它的人自己在解决「它太长了」**（§4(a) 第 2 条），说明它的产出物需要二次加工才能用。
3. **在网页游戏里技术上尤其贵**：另一条调研线查到 PostHog 的 Unity SDK 在 WebGL 上直接禁用 session replay，Sentry 的 replay 要 `preserveDrawingBuffer`〔另一条调研线一手〕。Playset 能录是因为它托管整个页面、录的是标签页——**那条路要求我们成为玩家那一侧的运行环境**，和 DESIGN 的边缘定位是两件事。

一句话：**不做录像不是护城河，是不进泥潭。** DESIGN §2.2 里把「不录像」当差异化讲的语气应当降级。

**「结果层做轻」——这个位置已经有人，而且不止一家。**

- Playloop 免费档给完整的 event timeline + replay + 三层 AI 摘要 + 游戏内反馈表单 + MCP，条件只是自带 AI key〔一手〕。
- Bugnet 的 session replay 录的是输入事件而不是视频，有 web/HTML5 SDK，直接面向 indie〔一手〕。
- LoopKit 的 replay 是快照帧 + 事件，beta 免费〔一手〕。

**所以「比 Playset 轻」不是一个空位。** 我们的角度必须从「更轻」换成两条别人给不了的：**(i) 在同一条命令里**——他们全都不做交付，开发者得先在别处把游戏放出去，再回到他们的产品上传第二次；**(ii) 不改一行代码就有第一层**——他们的第一层数据全部以「接 SDK」为前提，而 DESIGN §3.4 的第一层来自门禁页与边缘，是零接入的。

**「CLI 为中心」——这是这一整类里唯一真正空着的格子。** 见 §4(c)。十一个产品，没有一个的第一步是终端。这一条应当在 DESIGN 里被加强，而不只是当作「我们碰巧是命令行工具」。

**但要标出一个风险：这一类有一半的产品，一半价值在「帮你找到人」。** Playcocola Premium、The Playtest、FirstLook、Playtester Network、Go Testify、Antidote、PlaytestCloud——它们卖的是测试者，不是工具。DESIGN §1.2 的五个时刻**默认开发者已经有 5–50 个具体的人**（朋友、群友、jam 评委）。这个假设如果不成立，我们做的恰好是这个市场里最不值钱的那一段（送过去 + 轻结果），而值钱的那一段（找到人）被 §3.7 明确排除了。**这个假设值得在 §8 的验证里单列一条去测，而不是当作前提。**

---

## 5. 对 playtest.run 的含义

### 5.1 支持 DESIGN 现有结论的

1. **§0.4「名字里的 playtest 兑现在知道结果」——成立。** 这一类在 2025–2026 冒出十几家，全部围绕「这一版玩成了什么样」，说明这个需求是真的有人在做产品、在定价。
2. **§2.1「都没有隧道」——成立，且比 DESIGN 写得更强。** 扩到十几家之后依然零。
3. **§3.7 不做发现 / 榜单 / 商店——被侧面支持。** 做「找到人」的那几家（The Playtest、Playtester Network、FirstLook）都需要一个双边市场，那是另一门生意、另一套获客成本。
4. **不做全程录像——成立，但理由要按 §4(d) 换掉。**
5. **玩家零门槛——是入场券。** 三家不同价位段用相同措辞承诺，说明这是本品类的准入条件，不是我们的发明。

### 5.2 与 DESIGN 冲突、需要改的

按优先级排：

1. **§2.1「Playtest 结果」那一格的「缺的」里，「没有 CLI 工作流」这半句要改。** 建议改成：「都没有隧道；有 SDK 与 API 的几家（Playloop、Bugnet、FirstLook）入口仍是网页 dashboard——**没有一家以命令行为入口**。」
2. **§2.1 那一格只列了四家，实际至少还有十家（本文 §3 表）。** 建议不要继续罗列名字，改成按 §3.1 的三种商业模式来写——罗列会过期，模式不会。
3. **§2.1「Playset……纽约两人团队，2025」——没有出处。** 本轮没查到任何来源。建议删掉，或改成「团队规模与所在地未核实」。
4. **§2.3 第 3 条「也证明这条路上已经有人在收钱」——没有证据。** 建议改成「已经有人在这条路上**标了价**」，并保留紧随其后的「缺口不等于需求」那句。（这与另一条调研线的结论一致，我在 HN 与 PH 两处独立复核，仍为零。）
5. **§2.2 / §2.3 第 2 条「结果层要做得比 Playset 轻」——位置已被占。** 建议把角度改成「**在同一条命令里** + **不改一行代码就有第一层**」。
6. **§2.1 里 Playset 的「$0 / 19 / 49」缺一手确认。** 15 分钟/月是一手确认的，$19 / $49 不是。建议在核实前加一个「（未逐字核实）」，或只写免费档那个确认过的数字。
7. **§8「留门但不推进」里的「画面录像」，留错了对象。** 真正该留的门是「一句话反馈 + 单帧截图」——Playset 的玩家能在浏览器里直接截图、Bugnet 的游戏内组件带截图、PGPT 的路线图是「给视频打时间戳的事件 API」。**截图是这一类的标配，不是升级项。** DESIGN §3.4 已经把截图放在 SDK 层了，措辞上把它和「全程录像」分开写会更清楚：录像明确不做，截图是第一批要做对的东西。
8. **新增一条风险，建议进 §8 的验证清单：** 「开发者自带 5–50 个具体的人」这个前提没有被验证过，而这一类有一半的产品价值就在于它不成立（见 §4(d) 末段）。私测 14 天里应当直接问：**你把链接发给了谁？你是从哪找到这些人的？** 如果多数人答不上来，DESIGN §1.2 的五个时刻需要重写。

### 5.3 一条本轮意外的观察

`playtesting tool` 在 Hacker News 的全部历史里只有 38 条命中〔一手，Algolia〕，而 Playset、Playloop、PGPT、LoopKit、Bugnet、The Playtest 无一在 HN 或 Product Hunt 上留下可检索的发布记录。**这一整个品类在开发者社交平台上是隐形的。** 它的用户不在 HN，可能在 Discord、在 itch、在各自引擎的论坛里。这对 DESIGN §8 的冷启动渠道选择有直接影响——**Show HN 在这个品类里大概率无效**。

---

## 6. 没查到 / 没验证（不要当结论用）

**关于 Playset：**

1. 任何一条独立用户评价、用量数字、留存数据。HN、PH、Reddit、itch 论坛全零。
2. 团队规模、所在地、成立时间、融资——DESIGN 里的「纽约两人团队，2025」**完全没有找到出处**。
3. $19 / $49 两档的一手正文；免费档「1 游戏 / 1 GB」；免费档里那条以「Any number ...」开头被截断的条目。
4. 链接形态（子域还是路径）、文件大小上限、是否处理 COOP/COEP 与 Brotli、移动端表现、玩家要不要登录。
5. **录像的授权提示原文与隐私声明**——本轮最想看却没看到的一页。
6. changelog / blog / 更新频率——找不到，无法判断它最近一次更新是什么时候，也就无法回答「它还活着吗」。
7. 大陆可达性——没测。

**关于其它产品：**

8. Playruo、Live Aware、Insights.gg 的定价，Bugnet 的完整档位（免费档抽取被截断）。
9. Playloop 的团队具体人数；`/about` 说「每周发几次」与 `/changelog` 说「No releases yet」的矛盾我只记录，没有解释。
10. Playloop 免费档到底是 1 GB 还是 5 GB——同一天两个来源两个数。
11. FirstLook 属于 Pragma 是〔推断〕（依据是 Unity 包名 `com.pragma.firstlook.*` 与文档索引的挂靠位置），**没有官方声明确认**；Games Press 发布稿里被截断的那个「$25...」（疑似融资额）也没查到。
12. Bugnet 的「340+ 工作室、1.2M+ bug 报告、73% 回归率、38% 降流失」全部是厂商自述，**无任何第三方验证**。同理 Go Testify 的「5,000+ 团队」、Playtester Network 的「75 万测试者」。
13. FirstLook 声称的「in-game surveys 30–40% 回收率」〔另一条调研线记录〕没有样本量与方法，是厂商自述，不可引用。
14. PGPT、LoopKit、The Playtest、Playcocola 的用户评价——同样一条都没找到。这一类普遍没有可检索的用户声音。

**方法上的：**

15. 本轮所有官网正文都来自检索引擎抽取，**没有一页是我逐字读完的原始 HTML**。凡是引号里的英文都是抽取结果，字面可信但可能不完整。
16. 上一轮（12:25 前）已经做过的检索，其原始结果没有落盘，本轮是重建，**可能漏掉了上一轮见过而本轮没有再检索到的东西**。

---

## 7. 一手来源清单

- Playset — <https://playset.app/>
- Playloop — <https://playloop.gg/> · [/about](https://playloop.gg/about) · [/pricing](https://playloop.gg/pricing) · [/status](https://playloop.gg/status) · [/changelog](https://playloop.gg/changelog) · [/docs/credits](https://playloop.gg/docs/credits)
- Playcocola — <https://playcocola.com/>
- Playruo — <https://playruo.com/playtests> · <https://playruo.com/lp/playtests>
- Pretty Good Playtesting — <https://pgpt.dev/> · <https://duck.pizza/> · <https://github.com/Chikanz/pgpt-client>
- Slaytester 2 — <https://github.com/cannonheart/slaytester2>
- LoopKit — <https://loopkit.dev/>
- Live Aware — <https://home.liveaware.io/> · [Treehouse 案例](https://home.liveaware.io/case-studies/treehouse-games) · [Lunchbox 案例](https://home.liveaware.io/case-studies/lunchbox-entertainment) · [SDK telemetry](https://home.liveaware.io/blog/sdk-telemetry)
- Bugnet — <https://bugnet.io/> · <https://bugnet.io/about> · <https://bugnet.io/docs/sdks>
- FirstLook — <https://firstlook.gg/> · <https://docs.firstlook.gg/developers/sdk/unity/> · <https://docs.firstlook.gg/developers/sdk/unreal/>
- The Playtest — <https://theplaytest.com/>
- Insights.gg — <https://insights.gg/>
- Userplay — <https://userplay.io/> ；Go Testify — <https://www.gotestify.com/> ；Playtester Network — <https://playtester.net/>
- Hacker News 全库检索 — <https://hn.algolia.com/api/v1/search>（查询 `playset` / `playloop` / `playtesting tool`，2026-09-07）

**二手 / 第三方：**

- Games Press，FirstLook 1.0 发布稿（2026-02-17）— <https://www.gamespress.com/THE-GAMES-INDUSTRYS-FIRST-PLAYER-RELATIONSHIP-PLATFORM-ARRIVES-WITH-TH>
- Games User Research，远程 playtest 平台盘点（含 Playcocola）— <https://gamesuserresearch.com/top-remote-playtest-platforms-for-unmoderated-testing/>
- Playcocola 公司资料（2 人 / 2022 / 德国 + 印度）— <https://linkedin.com/company/playcocola>
- PGPT Discord 规模 — <https://disdex.io/server/1227107024001568808>

**同仓库同日其它调研线（引用其一手抓取时已在正文标注）：** `docs/research/2026-09-07-playtest-feedback-analytics.md`、`2026-09-07-business-models.md`、`2026-09-07-same-thing-better.md`、`2026-09-07-upload-hosting-ux.md`
