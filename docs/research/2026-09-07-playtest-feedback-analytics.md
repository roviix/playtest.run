# 一次 playtest 到底想知道什么：产品、社区智慧与虚荣指标

> 调研日期 2026-09-07。本线只回答一个问题：**独立/业余开发者做一次 playtest 时真正想知道什么、会用什么工具、哪些是虚荣指标。**
> 服务于 DESIGN §3.4（知道结果：最小集）、§4.6（SDK）、§8（T6 每版反馈数 ≥ 1）与创始人判断 3（是否提供数据分析）。
> 所有价格、限额均标注抓取日期。来源分三级：**〔一手〕**官网/文档/定价页；**〔原话〕**论坛/评论/访谈里的用户或从业者原文（附链接）；**〔二手〕**评测站、综述、厂商自述的第三方数字，可信度较低。

---

## 0. 先说结论

十条，其余各节是出处。

1. **「流量」在 5–50 人的 playtest 里不成立。** 独立开发者的分析指南直接写：「如果你的游戏有 200 个玩家，统计噪声会把你带向错误结论」，A/B 测试每个变体需要 200–300 人〔原话，StraySpark〕。创始人问的「哪里流量好」在这个量级上没有可算的东西。**能成立的是「谁玩到哪一步、卡在哪、在哪退出」——这是计数问题，不是统计问题。**
2. **但「在哪退出」这件事，在网页游戏上有一半发生在游戏开始之前。** 一家网页游戏门户的实测：开始加载的用户里 **44%±5% 从没走到「加载完成」**，各浏览器差异不大；其中 5% 是浏览器 IndexedDB 故障、根本不可能成功〔一手，GameArter〕。这不需要 SDK，边缘就看得见。**DESIGN 第一层数据的价值被严重低估了——它不是「聊胜于无的元数据」，它覆盖的是最大的一段流失。**
3. **Playset 的录像不是刚需，是一个昂贵的替代品。** 它免费档 15 分钟/月、$19 档 10 小时/月、$49 档 50 小时/月，按实际游玩分钟计费〔一手〕。免费档只够看一个人玩一刻钟。同时它的对手不是别的工具，是「Discord + Google Forms + 随手录屏 + 表格」这套 DIY 栈——它自己网页上就是这么写的。**我没有查到任何一条独立用户说「看了录像才发现 ___」。**
4. **AI 总结不是产品，是模型账单的包装。** PlaytestCloud 把 AI 分析**免费包含在所有档位**、且不额外扣 video token〔一手〕；Playloop 免费档给你全部 AI 分析面，但要你自带 OpenAI/Anthropic key，付费档 $49/$99 卖的是「托管 AI」〔一手〕。**能白送的东西不构成护城河。** 而且 AI 总结的价值和会话量正相关——Side 做 AI 分析是为了处理 5 万到 10 万人的 playtest〔一手〕，5–50 人根本没有量。
5. **反馈按钮 T6 ≥ 1 的目标现实，但只在「按钮就在游戏里」时现实。** Refiner 2025 年 1,382 个 in-app survey、5000 万次曝光的数据：平均响应率 27.52%〔二手，厂商报告〕；对比 Survicate 的 3,095 个网站挂件 survey 中位数 7.64%〔二手〕。**取保守的 7.6%，30 个玩家能出 2.3 条反馈，T6 达标。取乐观的 27%，8 条。** 但这个数字对「多加一个必填字段」极度敏感（见 §4.6）。
6. **对照组：itch.io 上 jam 作品的平均反馈数比 1 还低。** itch 管理员公开的数据：他们办过的最大的一场 jam（16 万条评分）中位数是每个作品 **16 条评分**；而一场 185 个作品的小 jam，218 条评分只覆盖了 104 个作品（56.2%），**平均 1.2 条、中位 2 条——44% 的作品一条评分都没有**〔一手，itch.io〕。**T6 ≥ 1 不是低标准，它就是行业中位数。**
7. **业界共识极其一致：闭嘴看着，别问喜不喜欢。** Valve 的 Ken Birdwell、Mike Ambinder，GMTK 的 Mark Brown，Soren Johnson，Tynan Sylvester，Adriaan de Jongh，说的是同一件事——玩家说的和玩家做的经常是两回事。但**这条共识对我们是双刃的**：它同时也说明「一句话文字反馈」的信息价值本来就有限（§3.2）。
8. **有一条把「行为」和「自述」对上的硬证据，来自 Zachtronics 的 Zach Barth：「玩家 bounce 掉（开始但没做完）的那些谜题，恰好就是通关的人自述『太难』的那些谜题」**〔原话〕。这是我找到的最有力的一条「行为数据能替代问卷」的公开证据——**而 bounce（开始了没做完）恰好是不需要复杂埋点就能测的东西。**
9. **网页游戏的最大差异化不在分析，在「设备与浏览器」。** 一位做了一年 Unity WebGL 的开发者：「每个平台上的每个浏览器本质上都是一个独立平台。Chrome 里流畅 60fps？Firefox 里可能 20fps 卡顿。」〔原话〕连 PostHog 的 Unity SDK 都明确写着 **WebGL 不支持 Session Replay**〔一手〕。**「哪个设备/浏览器上坏了」是网页 playtest 独有的、第一层就能拿到的、别人拿不到的东西。**
10. **市场信号：build 分发这一层，浏览器游戏开发者已经不认为是问题了。** itch 论坛上一个正在做 playtest 工具的人问「$9/月你会付吗」，一位做浏览器多人游戏的开发者回：「构建分发对我来说早就解决了。它就是一个 URL。一个核心卖点是『管理构建、发给测试者』的工具对我完全没有东西可卖。」〔原话〕**这条直接打在 DESIGN §0.4「把版本送过去」的价值主张上。**

---

## 1. 产品扫描：谁给什么数据、多少钱、上手门槛

### 1.1 总览表

| 产品 | 要不要改游戏 | 给什么数据 | 价格（2026-09-07 抓取） | 谁在用 |
|---|---|---|---|---|
| **Playset** | 不要（浏览器内） | 全程录像、玩家截图、时间戳反馈、按分钟计量 | $0（15 分钟/月，1 游戏 1GB）/ $19（10 小时）/ $49（50 小时，3 游戏） | 未知，无公开用量 |
| **Playloop** | 要 SDK（或拖录屏） | 事件、session/tester/build 三层 AI 摘要、游戏内反馈表单、A/B digest、replay | beta 期免费（BYO AI key）/ Indie $49 / Studio $99；credit pack $25 起 | beta，母公司 Turtlevania 自用 |
| **Antidote** | 不要 | 录像 + think-aloud + 转写 + 情感分析 + AI Insights + 问卷 | 用他们玩家池 $25/人起；自己社区 Starter $115/月、Pro $475/月；indie 免费档（<10 人、无发行商）1 次/月 | 有匿名客户 testimonial |
| **PlaytestCloud** | 不要（自动注入录屏） | 录屏 + 语音 + 触摸 + 问卷 + 转写 + AI「First Findings」 | 按人按分钟买；免费试用 = 2 个玩家 30 分钟 | 企业/手游为主 |
| **Userplay** | 要（桌面游戏） | Session replay、语音转写、Discord 招募、Player CRM、问卷 | 未公开 | 面向工作室 |
| **Steam Playtest** | — | **什么都不给**，只有分发和准入 | 免费 | 大量 Steam 开发者 |
| **itch.io** | 不要 | views / downloads / **browser plays** / referrers / ratings / collections / followers / revenue，可调时间段 | 免费 | 海量独立开发者 |
| **The Gaming Nest** | 不要 | 配额、版本状态表（Ready/Superseded/Failed）、active 版本大小与 play URL | 免费 300MB / 账号，10 个版本 | 新，2025–26 |
| **Poki（Game Events）** | 要（Poki SDK） | 自定义事件、漏斗、版本对比 | 平台内免费 | 门户签约开发者 |
| **GameAnalytics** | 要 SDK | DAU/MAU、留存、漏斗、进度、ARPU/LTV、cohort、错误 | 免费（据称 100k MAU 内） | 100,000+ 月活跃游戏、2 亿 DAU |
| **Unity UGS Analytics** | 要 SDK | 事件、留存、funnel、A/B | 50,000 MAU 免费，之后 $0.00360/MAU；保留 13 个月 | Unity 独立开发者默认 |
| **Sentry** | 要 SDK | JS 错误 + 堆栈 + breadcrumbs + release 关联；Session Replay 是付费加项 | 免费 5,000 events/月；Team $29/月/user | 前端工程通用 |
| **PostHog** | 要 SDK | 事件、replay（含 canvas）、feature flag、error tracking | 免费 5,000 web sessions + 2,500 mobile sessions/月 | 创业公司通用 |
| **Umami / Plausible / GoatCounter** | 一行脚本 | pageview、referrer、设备、自定义事件 | Umami 自托管免费（MIT，~200MB RAM）；Plausible $9/月起；GoatCounter ~3.5KB | 个人站点/侧项目 |

### 1.2 Playset：和我们最像的那一家，看清它的边界

〔一手，https://playset.app/ ，2026-09-07 抓取〕

它的卖点原文：**「Frictionless playtesting. Auto-recorded video. Timestamped feedback.」**
- 上传：拖一个 web build 的 `.zip`，拿到托管链接，「send anywhere — Discord, email, X」。
- 数据：每次 playtest 都有完整录像；玩家可以在浏览器里直接截图；反馈钉在 session 的精确时间点上；有实时模式（边看边和玩家聊）。
- **「No SDK, no install」**——它明确把「不用埋点」作为卖点，和我们 §3.4 第一层的思路一致。
- 计费：按实际游玩分钟数，跨游戏跨测试者共享额度。

价格（2026-09-07）：

| 档 | 价 | 额度 |
|---|---|---|
| FREE | $0 | 15 分钟 playtest/月，1 个游戏 ≤1GB，测试者与 session 数不限 |
| STARTER | $19/月（年付省 $48） | 10 小时/月，1 个游戏 ≤2GB |
| STUDIO | $49/月（年付省 $120） | 50 小时/月，3 个游戏各 ≤3GB |
| CUSTOM | 询价 | 更多游戏、更长录像保留 |

**它自己定义的对手是「The DIY Stack」**，网页原文：「Stop juggling Discord, Google Forms, random screen recordings, and spreadsheets just to understand what's going on in your game.」它列的 DIY 栈痛点是：新版本要重发链接给每个测试者、要靠测试者自己录像和记笔记、没有「他们在哪卡住/无聊」的时间线、只能手工找规律。

**几点判断：**
- 免费档 15 分钟/月的意思是：**一次 30 人的测试，免费档一个人都测不完。** 这是把免费档当 demo 用，不是当入口用。我们的免费档（DESIGN §6：每月 20GB、3 个 slug、并发 50）在「够不够跑完一次真实测试」这个维度上是完全不同的量级。
- 「按分钟计费」在 playtest 场景是反激励的：开发者希望玩家玩得久，付费模型却惩罚这件事。
- **我没有查到任何 Playset 的独立用户评价。** 没有 Reddit 讨论、没有 HN 帖、没有 itch 论坛提及、没有 Product Hunt 评论。「Playset 已经在卖一键 playtest」这件事是真的，「有人在买」我没有证据。这一点应该修正 DESIGN §2.2 里隐含的「它已经成功了」的语气。

### 1.3 Playloop：AI 总结这条路的实际形态

〔一手，https://playloop.gg/ 、/pricing、/docs，2026-09-07 抓取〕

- 接入：一次 `init`，一次 `client.telemetry.track('level_completed', { level: 3, deaths: 0 })`。SDK 覆盖 Unity / Unreal / Godot / Python / TypeScript。也支持「没有 SDK runtime 就拖一段录屏进来」。
- 产出：**三层叙事摘要——session → tester → build**。每条观察被打类型（stuck-point / confusion / bug / pacing）并按证据打严重度分。还有 build 对 build 的对比（play time、engagement rate、session count、top friction events）。
- **游戏内反馈表单免费给所有工作室**，服务端限流：每 (session, form) 一次提交、每 session 最多 3 次——**这说明他们预期反馈按钮会被滥用/刷屏，这是个有用的实现细节。**
- 定价：beta 期免费档覆盖「每一个分析面」，条件是自带 OpenAI/Anthropic/Gemini key；Indie $49/月、Studio $99/月加的是**托管 AI**、团队协作、更长保留。Indie 档 15,000 credits ≈ 5,000 次 Standard 分析。14 天试用给 3,000 credit 预算。Credit pack $25 起，一个月 Showcase pass $59。
- 注意：首页写免费档「1 GB 存储 / 30 天保留」，pricing 页写「5 GB · 30 天 · 2 seats」，两处不一致，说明还在改。

**这家产品结构最有信息量的一点：AI 分析本身不收钱，收的是模型账单。** 官方文档原话：「BYO-key analyses spend no credits, your own provider bills those directly」。也就是说 Playloop 的差异化不是「AI 能总结」，是「我们的 prompt 和三层结构」——这是个很薄的护城河。

### 1.4 Antidote / PlaytestCloud / Userplay：招募玩家池那条路

这三家和我们不在一个赛道，但它们的定价划出了「有人愿意为 playtest 结果付多少钱」的上限：

- **Antidote**〔一手〕：用他们的玩家池 **$25/人起**；用自己社区订阅 **Starter $115/月**（FAQ 页写 $114，pricing 页写 $115）、**Pro $475/月**。有 indie 免费档（团队 <10 人、无发行商无投资人），每月 1 次 playtest，条件是「Game and Studio featured quarterly in our Indie Section」——**用曝光换免费额度，这是创始人判断 2（曝光/孵化生态）的一个现成样本。** AI Insights 只在 $475 档。
- **PlaytestCloud**〔一手〕：「No modifications to the game are necessary. Screen recording and security features are added to the game automatically by PlaytestCloud.」浏览器游戏只要粘一个链接。免费试用 = 看 2 个玩家玩 30 分钟。**AI 分析免费包含在所有档、不额外扣 video token。**
- **Userplay**〔一手，userplay.hashnode.dev〕：他们对「问卷 vs 录像」的表述值得记：「A survey says: 'The tutorial felt confusing.' Useful. But not nearly as useful as the moment where they got confused. Did they miss the objective? Did they press the wrong button? Did they misunderstand an instruction? Did they wander around for five minutes before giving up? Those are very different problems, hidden behind one sentence in a survey.」

### 1.5 Steam Playtest：一个反例

〔一手，Steamworks 文档〕Steam 自己的措辞是「gives developers a free, low-risk way to get playtesting data」。但 PlaytestCloud 的这段拆解是对的〔一手，start.playtestcloud.com/blog/steam-playtest-vs-...〕：

> 「Although this explanation means Steam Playtest is intended to gather 'playtest data,' the feature itself doesn't actually provide any playtest data. Instead, Steam Playtest is a way to distribute your game to beta testers through Steam's infrastructure... there is no data or feedback collection built in.」

**Steam Playtest 是「只做分发不做结果」的极端样本，而它被广泛使用。** 这既支持 DESIGN §0.4（「知道结果」是差异化）——也提醒我们：**分发做得足够好，就算完全不给数据，人也照用。**

### 1.6 itch.io 的 analytics 面板到底给什么

〔一手 + 原话，https://itch.io/t/5281744/adding-actual-analytics-on-itch-for-our-own-releases ，2025-09 开帖，1.1 万浏览〕

用户 Hoiby 逐条列出的默认指标（不接 GA4）：

> 「views, downloads, browser plays, referrers, total counts per project of views, downloads, revenue, payments, ratings, and collections... per game: views, browser plays, payments, and referrers... in all of these you can adjust the time period for the graph.」

**它有「browser plays」，但没有「谁玩了」，也没有任何游戏内的东西。** Hoiby 的解释很到位：

> 「Itch can't provide those because they are essentially a file sharing service. All they do is provide the customer with the file the creator has. After that, they don't know what happens. They don't know playtime or anything that comes after the game download.」

想要更多只能挂 GA4，而挂 GA4 意味着开发者自己承担 GDPR/PIPL/CCPA 责任——楼主 Tenkarider 的原始诉求就是这个：

> 「those mandatory and different laws that are under the development's responsibility... are so strict, messy and difficult to implement... indies definitely do [have trouble], probably in order to stay compliant with those laws they will just won't use them in many cases」

**这是一个我们能占的位置：第一层数据由我们（平台）收，隐私责任在我们的隐私声明里，开发者不用碰 GDPR/PIPL。** DESIGN §3.4「不收集玩家身份、不跨站追踪、90 天保留」已经是这个方向，但没有把「替开发者承担合规」这一点讲出来。

另外，itch 用户抱怨过 referrers 数据不准〔原话，itch.io/t/6007036〕：「It claims that I have had a total of 20 visits over the past 30 days, which is a small fraction of what the bar chart claims for 'Factory Worker' alone.」——**referrer 这个字段本身就脏。** DESIGN §3.4 的「来自哪里（微信/浏览器/Discord）」如果只靠 `Referer` 头会不准，微信内置浏览器还经常不发 referrer。这一条需要在实现上想清楚（UA 特征 + 链接参数 + referrer 三路兜底）。

### 1.7 通用游戏分析：GameAnalytics 与 Unity 的口碑

**GameAnalytics** 的规模是真的：**100,000+ 月活跃游戏、2 亿 DAU、每天 27 亿事件**〔一手，gamesindustry.biz 2025 / mobidictum〕。2021 年华为稿里的数字是「近 100,000 developers、63,000+ studios」〔二手〕。

但它的指标清单说明了它服务谁：DAU / MAU / 留存 D1–D90 / ARPU / ARPPU / cohort / 收入 / 广告曝光〔一手，docs.gameanalytics.com〕。**这是运营中的手游的仪表盘，不是一次 playtest 的仪表盘。** 免费档据称覆盖到 100k MAU、无限自定义事件〔二手，aitoolsquare——厂商聚合站，我没有在 GameAnalytics 官方定价页确认到这个数字〕。

**Unity UGS Analytics**〔一手，unity.com/products/gaming-services/pricing〕：50,000 MAU 免费，之后 $0.00360/MAU，13 个月保留。Legacy Unity Analytics 已在 2023-06-27 关 dashboard、2024-01-31 停止收数据。一个值得注意的坑〔二手，Metaplay 引 Unity 说法〕：**超出任一服务免费额度而没加支付方式，会禁用所有 UGS 服务。**

**关键洞察**：itch 论坛那位反对者 redonihunter 的话，正好说出了这类工具和独立开发者的错位：

> 「I am curious, what exactly could data tell you to improve the games. You sure you are creating a game that will benefit from the type of data analysis that Unity promises? They are optimised for games with thousands of players and advertisements or micro transactions. It is about making more money.」
> 「I believe finding honest play testers for development and marketing the game to the target audience to be more helpful than looking for obscure data points that will only be usefull if you already have enough players to have statistics.」

### 1.8 Poki 的 Game Events：网页游戏最接近的一手案例

〔一手，https://poki.com/blog/game-events-new-tool-for-understanding-your-players 〕

Poki（网页游戏门户）自建了事件分析，**理由本身就值得记**：

> 「Third-party analytics tools aren't designed around how games run, load, restart, and generate sessions on the web. The way data is collected and interpreted can differ from native apps, and that can make seemingly straightforward metrics harder to reason about.」

三个开发者案例：

1. **Smash Room / Jim**：三个可能的开局物件（手机、玻璃杯、蛋糕），随机分配，测「3 分钟后还在玩」的比例——**玻璃杯 33.7%、蛋糕 27.4%、手机 23.8%**。他的直觉（手机）是最差的那个。
2. **Satisbox Mini Games / Erçin**：平均 session 4:50。埋了进度事件后立刻看到 Level 1 和 2 各流失 15%+，Level 5 完成率最高。**他没重做任何内容，只是把强的小游戏挪到前面**——4:50 → 5:43 →（再排一次）→ **6:23**。他的原话：「The best part? I didn't have to guess. Game Events showed exactly where players were leaving, making it easy to validate every improvement」。
3. **Stickman Fury / Jim**：少数玩家报「卡在地里」，报告本身不足以定位。事件数据显示 **17 个 stage 有掉率尖峰，全在 stage 56 之后**。「Stage 56 was where the game runs out of authored levels and starts replaying them mirrored - which was the key finding to help fix the bug.」

Poki 给的建议和我们的立场一致：**「start with a specific question」**，而不是把所有动作都埋上。

### 1.9 Sentry / PostHog：错误与录像在浏览器游戏上的真实边界

**Sentry**：免费 5,000 events/月、1 user、1 project；Team $29/月/user 含 50,000 events 和 session replay〔二手，pikvue 评测 2026〕。

官方文档里两条对我们直接相关的技术事实〔一手，docs.sentry.io/platforms/javascript/session-replay/troubleshooting/〕：
- 「The integration needs to enable `preserveDrawingBuffer` to export images from 3D and WebGL canvases. This can negatively affect canvas performance. If your canvas application is impacted... you'll need to enable manual snapshotting and call a `snapshot()` method inside of your re-paint loop.」
- Replay 会给 bundle 增加 **约 50 KB（gzip 后）**。

**PostHog**：免费 5,000 web sessions + 2,500 mobile sessions/月。Canvas recording 2D 和 WebGL 都支持，**默认 4 fps、且不自动开启**（因为无法遮蔽 canvas 里的 PII）〔一手，posthog.com/docs/session-replay/canvas-recording〕。

**但最关键的一条**〔一手，posthog.com/docs/libraries/unity〕：

| 平台 | Session Replay 支持 |
|---|---|
| Windows/Mac/Linux | Full |
| iOS / Android | Full |
| **WebGL** | **Not supported** |

> 「Session Replay requires async GPU readback support. On WebGL, Session Replay is automatically disabled.」

**连 PostHog 都在 Unity WebGL 上录不了。** Playset 能录是因为它录的是**浏览器标签页**（`captureStream` 层面），不是引擎层面——这也是它必须自己托管、按分钟收费的原因。

### 1.10 隐私友好的轻量分析：另一条参照线

- **Umami**：MIT 许可，单容器 + PostgreSQL，闲置 ~200MB RAM〔二手，多篇对比〕。
- **Plausible**：$9/月起（1 万 pageview），AGPL-3.0，自托管要 ClickHouse，至少 2GB RAM。
- **GoatCounter**：脚本 ~3.5 KB，有无 JS 的像素方案，也能从日志导入〔一手，GitHub README〕。

HN 上一条评论把这类工具的定位说得很清楚〔原话，news.ycombinator.com/item?id=23560823〕：

> 「I turned off Google Analytics, because I realized that it doesn't actually report any useful or actionable data, just vanity metrics, and many of them of dubious quality.」

**这条对我们是警告**：如果 playtest.run 的控制台只给「打开数、独立人数、设备分布」，它长得就像一个 Umami——用户会觉得「这是虚荣指标」。**第一层数据必须被组织成「一次 playtest 的叙事」，而不是「一个网站的流量面板」。**（见 §5.4 的具体建议。）

### 1.11 反馈收集工具链

- **Discord**：主流做法是独立的 `#bug-reports` 频道 + 置顶模板 + **表单 bot 强制结构**（discord.js modal，最多 5 个文本输入）〔一手，discordjs.guide + bugnet.io 教程〕。一句总结：「Templates are suggestions until a bot makes them mandatory.」
- **Google Forms**：Feature Upvote 的这条建议是我在整轮调研里看到的**最直接的填写率经验**〔原话，featureupvote.com/blog/how-to-get-player-feedback/〕：

  > 「Notice what we're not asking for: we're not asking for 'platform', we're not asking for 'build number', we're not asking for 'screen resolution', we're not asking for 'localization'. Although knowing this stuff will help you, requiring it will be a barrier in the way of people giving you feedback. **Every time you add one of these required fields, the amount of feedback you get will drop.**」

- **游戏内反馈按钮的实现原则**〔一手，bugnet.io/blog/how-to-collect-player-feedback-in-game〕：

  > 「never ask the player for information you can capture programmatically. Players are bad at describing technical details.」
  > 「Screenshot... is the single most valuable piece of context... Capture this the instant the feedback form opens, **before the overlay appears on screen**.」

- **一个开源 Godot 插件的问题陈述**〔原话，github.com/stoneforgelabs/forge-logger-godot〕，几乎逐字命中 DESIGN §3.3 的「版本告示牌」：

  > 「Testers paste screenshots into Discord with 'it crashed lol', you can't tell which build they were on, and triage becomes archaeology.」
  > 「Context comes for free. Each play session is stamped with the scene, game version, build hash, and environment... **no 'which version were you on?' round-trips**.」

- **Canny / 反馈板**：Nielsen 的 90-9-1 规则（90% 潜水、9% 偶尔、1% 贡献绝大部分）在反馈板上完全适用〔一手，nngroup.com〕。Canny 自己的一条数据：「Responding to just the top 5% of user feedback requests addresses 50% of votes.」〔一手，canny.io/blog〕

---

## 2. 社区智慧：playtest 该看什么

### 2.1 「闭嘴看着」是最强共识

Valve（GMTK 的 Mark Brown 整理，gmtk.substack.com/p/valves-secret-weapon）〔原话〕：

- Mike Ambinder（Valve 前驻场心理学家）：「We see our game designs as hypotheses and our playtests as experiments to validate these hypotheses.」
- Ken Birdwell：「**Nothing is quite so humbling as being forced to watch in silence as some poor playtester stumbles around your level for 20 minutes, unable to figure out the 'obvious' answer that you now realise is completely arbitrary and impossible to figure out.**」
- Kim Swift：「they may tell you later that they like the game but you'll really tell by their body language whether or not they actually enjoyed themselves」
- 节奏：Portal 几乎**每周**测一次——周五测、周一讨论、剩下几天改、下周五再测。Half-Life 2 每章约 100 个 playtester。
- Mark Brown 的结语：「playtesting feedback is just data, and it's up to the designer how that data is interpreted, filtered, and applied.」

**唯一一处「遥测直接驱动改动」的例子也在这篇里**：Steam 数据显示大量玩家卡在 Episode One 的一场攻城战，Valve 发补丁降了难度。**注意这是百万级玩家的数据，不是 20 个人的。**

Mark Brown 自己做《Mind Over Magnet》之后的反省〔原话，Rock Paper Shotgun 访谈〕：

> 「I didn't anticipate just how crucial playtesting would be. Having people play the game from an early stage, giving feedback, and observing their assumptions and mistakes has been eye-opening. It's amazing how much this process reveals about both the game and the designer's own blind spots.」

### 2.2 「玩家说的 ≠ 玩家做的」——以及它的反面

- **Soren Johnson**（Civ IV 首席设计师，designer-notes.com GD Column 19）〔原话〕：「what players say and what they actually do are often two [different things]」——但他紧接着写了限度：「**metrics have their limitations as no set of numbers is going to help the designer understand why people have stopped playing the game.**」以及「to learn whether a game is actually fun, the designer's only option is to find out what players are feeling by listening closely to what they are saying.」
- **Mike Ambinder**（Casey Weeks 访谈引用）〔原话〕：「People are not great at explaining why they do what they do... If they're like, 'No! That was easy!' But they died like 35 times, well, maybe it wasn't so easy.」
- **Adriaan de Jongh**（同上）〔原话〕：「The clearest feedback you can get is to see how someone plays your game. Everything else is very easily a distraction.」
- **Tynan Sylvester**（RimWorld，ludeon.com 论坛）〔原话〕：「theorycrafting is really dangerous... Even professional game designers with 15 years' experience can't theorize accurately at how a game design will play. I can't! So we use tons of coping mechanisms (constant playtests, short iteration cycles, unstable builds for feedback) to escape from our own mental incapacity.」

**对我们的意义是双向的**：这条共识支持「行为数据有独立价值」，但同时**削弱「一句话文字反馈」的价值**——DESIGN §8 的 T6 把「每版反馈数 ≥ 1」当作「知道结果有没有被用」的判据，而业界共识说文字反馈本来就是次要信号。**T6 测的其实是「玩家愿不愿意给我们留东西」，不是「结果层有没有用」。这两件事应该分开测。**

### 2.3 唯一一条把行为和自述对上的公开数据

**Zach Barth（Zachtronics）在 Reddit AMA 里**〔原话，bestofama.com/amas/78wv2h〕：

> 「We had a survey in Infinifactory during Early Access that taught us a lot about how the data we collect correlates with how players self-report about difficulty and enjoyment.」
> 「**Most importantly, puzzles that people 'bounce' off (start but never finish) are the same puzzles that people who solve self-report as having been too difficult.**」

这条极其重要，因为：
1. 它是我找到的唯一一条**实证**「行为指标能代理自述指标」的公开证据。
2. 它用的指标是 **bounce（开始了但没做完）**，不是留存、不是漏斗、不是 DAU——**这是一个可以在 5–50 人量级上数出来的东西。**
3. Zachtronics 还把这套数据做成了游戏机制（每关完成后给你和其他玩家对比的直方图）——但 Barth 说「I'm much more pleased with them as a mechanic than random magic-circle-breaking achievements」，说明他把它当设计元素而不是仪表盘。

### 2.4 朋友与家人的偏差

〔二手，socratopia.app 的独立开发者手册章节，逻辑清晰但无一手数据〕三重偏差：**politeness**（负面反馈被软化）、**context**（游玩环境不代表真实）、**prior exposure**（朋友已经吸收了陌生人不会有的信息）。结论：「Friends can answer functional questions (build runs, controls work, obvious bugs)... they cannot answer the questions that determine commercial outcome (first-impression, onboarding, difficulty calibration for context-free players, retention).」

**这条对 DESIGN §1.1「观众是 5–50 个具体的人（朋友、测试者、评委、学生）」是一个提醒**：我们的目标用户的测试对象里，「朋友」这一类正好是最不可信的一类。**这反而抬高了行为数据的价值——朋友的嘴不可信，朋友的行为可信。**

一个两人团队的真实抱怨〔原话，beplayful.gg/blog/playtesting-teeto，开发者 Mitch〕：

> 「You find QA bugs, but otherwise most of the feedback is 'this is great' or 'this sucks.' **There's only so much you can do with that.**」

### 2.5 小样本：够用还是不够用，取决于问什么

- **Nielsen 的「5 个用户找出 85% 的问题」**〔一手，nngroup.com〕经常被误引。MeasuringU 的修正〔一手，measuringu.com/five-user-85/〕：「after testing five users you have only found 85% of problems **that affect 31% or more of your users** given those tasks」。5 个人能抓到 15% 发生率的问题的约 50%、25% 发生率的约 75%。
- **对「发现问题」，小样本够**：一家 playtest 公司的表述〔原话，emhance.ai〕：「If six independent participants show the same attention collapse at the same moment in your FTUE, you do not need sixty participants to confirm the pattern is real.」
- **对「测量差异」，小样本完全不够**〔原话，strayspark.studio 的独立开发者分析指南〕：
  > 「Analytics can be misleading at small sample sizes. If your game has 200 players, statistical noise can lead you to wrong conclusions.」
  > 「Minimum viable sample: You need at least 200-300 players per variant to detect meaningful differences. For a two-variant test, that means 400-600 total players.」
  > 「If your game has been out for two days and has 50 players, the data is statistically meaningless. Wait until you have at least a few hundred players before drawing conclusions.」

**这直接回答了创始人的第三个判断：在 5–50 人的量级上，「分析」这个词应该被禁用。能做的只有「计数」和「点名」。**

### 2.6 一个来自 itch 论坛的市场信号

〔原话，itch.io/t/6804701 ，2026-08 左右开帖〕有人在做和我们方向重叠的 playtest 工具（上传构建 → 私密链接 → 玩家点「报告 Bug」→ 自动收集 FPS/console error/系统信息/截图/前几秒录像 → 开发者仪表盘），问「$9/月你会付吗」。

两条回复：

**hechelion**：
> 「The question isn't whether someone would be willing to pay $9 for your solution, but rather **why they should choose your solution over the dozens of others already on the market**.」

**HKM Industry**（做浏览器多人游戏）：
> 「Honest answer: no for build distribution, yes for the thing nobody does.」
> 「**Build distribution is already solved for me. It is a URL.** A tool whose core pitch is 'manage builds and get them to your testers' has nothing at all to sell me, and browser games are not a small slice of itch.」
> 「What is not solved, as far as I can tell, is **concurrency**. The bottleneck in multiplayer playtesting is never getting the build out or capturing the bug — it is getting four people into the same lobby at the same time. And a tester who turns up alone does not give you a weak signal, **they give you a misleading one**: they report 'nobody was online, seems dead', which tells you nothing about your game and burns a tester you cannot easily replace.」
> 「So the thing I would actually pay for is a **scheduled session**. Pick a 20-minute window, N testers commit to it, everyone gets the same build at the same moment, and the session comes back as ONE artifact with every participant's view time-aligned.」

**这是本轮调研里最有价值的单条用户原话**，三个理由：
1. 它证明「把构建送过去」在浏览器游戏这一侧**不被认为是痛点**——对 DESIGN §0.2「交付两条路都做」是个警告（上传路径的价值主要在大陆可达，不在便利）。
2. 它指出了一个 DESIGN 完全没覆盖的场景：**联机 playtest 的瓶颈是并发，不是分发。** DESIGN §4.3 花了大篇幅论证隧道技术选型，但没有回答「怎么让 4 个人同时在线」。
3. 「一个人独自来测给的不是弱信号，是误导信号」——这条对我们的 T3（每版打开人数中位数 ≥ 5）有含义：**打开人数分散在几天里和集中在 20 分钟里，是完全不同的产品。**

---

## 3. 逐条回答

### (a) 不改游戏代码就能拿到的第一层数据，有人证明有用吗？

**有，而且比 DESIGN 里假设的更有用。**

**支持的证据：**

1. **加载失败是最大的一段流失，而它完全在第一层可见。**〔一手，gamearter.com〕一家网页游戏门户对 4 个游戏（8 / 14 / 51 / 100 MB）只统计新用户、4 天数据：
   - 「There are not significant differencies for various browsers. **Losses in all browsers are in a ratio of 44±5%.**」（「开始下载」到「加载完成」之间流失 44%±5%）
   - 「There was detected problem with browser's IndexedDB in **5% of cases**. This 5% of 'download started' had no chance to be completed. This problem occured in a case of all major browsers.」
   - 老用户（认识开发者的）流失 40%，比新用户低 7 个百分点。
   - **警告**：作者自己承认「made on basis of low amount of audience」，且这是门户站的休闲游戏、2018 年前后的网速环境。数量级可信，精确值不可引用。

2. **另一家门户的实测：玩家在游戏开始之前就走了。**〔一手，slowden.com/blog/why-players-leave-before-your-game-starts/〕手机上游戏框只有 330×205 px；宿主页面 0.3 秒画完，游戏区是好几秒的纯黑矩形；一个游戏的「Let's play」按钮落在笔记本视口的底边以下。结论原话：「**nobody who left has played it yet. Whatever they rejected, they rejected before the game started.**」

3. **设备/浏览器碎片化在网页游戏上是一等问题。**
   - 〔原话，ernesernesto.github.io/writes/oneyearwebgl/〕「every browser on every platform is essentially a unique platform. That smooth 60fps experience in Chrome? It might stutter at 20fps in Firefox. Works great in Firefox on Windows? Completely different story on Firefox Linux.」
   - 〔一手，Unity 手册〕Web 可用内存取决于设备、OS、浏览器、32/64 位、JS 引擎解析你代码需要多少内存、浏览器是否每个标签页一个进程。
   - 〔二手，Godot Web 导出 checklist〕白屏的常见原因：`.wasm`/`.pck` MIME 类型错、缺 COOP/COEP、路径大小写、缓存冲突——**这些全都是边缘的 404 / 响应头 / 中断下载能看见的东西。**

4. **「版本对不上」是 playtest 的经典损耗，第一层就能消灭。**〔原话，forge-logger-godot〕「you can't tell which build they were on, and triage becomes archaeology」。DESIGN §3.3 的门禁页「版本告示牌」正是对这个问题的解。

**反面证据 / 限度：**

- itch.io 的第一层数据（views / browser plays / referrers）被它自己的用户评价为「trivial」「mostly few Gross metrics... this is not very helpful」〔原话，Tenkarider〕。**说明第一层数据如果只是数字，会被当作虚荣指标。**
- referrer 字段本身不准〔原话，itch.io/t/6007036〕。
- HN 上的那句「it doesn't actually report any useful or actionable data, just vanity metrics」是对通用网页分析的典型评价。

**「开发者会看几次」：没查到任何公开数据。** 没有任何来源报告过 playtest 工具的 dashboard 回访频率。这是一个应该在私测里自己测的数（建议加成 T7，见 §5.5）。

### (b) 加 SDK 才有的——哪些是真需求，哪些没人接？

按「有多少证据说它被真的用起来」排序：

| 能力 | 证据强度 | 判断 |
|---|---|---|
| **JS 错误 + 堆栈** | 强。Sentry 是前端事实标准，免费档 5,000 events 够小项目跑 3–6 个月〔二手〕。Godot/Unity WebGL 崩溃「你自己复现不出来，只能让失败自己找上门」是共识〔二手，bugnet.io〕 | **真需求，第一优先** |
| **加载完成用时 / 加载阶段事件** | 强。§(a) 的 44% 流失就发生在这里；Poki 和 Cinevva 的教程都把 load 分阶段埋点当基础动作 | **真需求，而且能和第一层的「资源加载失败」拼成完整故事** |
| **自定义事件（进度/关卡）** | 强，但**证据来自量大的场景**。Poki 三个案例都成立（4:50 → 6:23），但 Poki 的游戏有门户流量 | **真需求，但在 5–50 人量级只能当「点名」用，不能当「比例」用** |
| **反馈按钮（一句话 + 截图）** | 中。基准填写率 7.6%–27.5%〔二手〕；实现原则清晰；但**没有任何 playtest 场景的公开填写率** | **真需求，但目标要按 §4.6 的数字定** |
| **画面录像** | 弱。见 (c) | **不做，留门** |
| **AI 总结** | 弱。见 (d) | **不做** |

**「没人接入」的证据：**

- **GameAnalytics 的 100,000 个游戏几乎全是手游。** AppGoblin 的静态分析显示它在 Android 上 SDK 渗透率 2.95%、iOS <0.1%，且份额在下降（-0.46% / -2.46%）〔二手，appgoblin.info——第三方反编译统计，样本口径不明〕。**我没有找到任何关于 GameAnalytics 在网页游戏 / playtest 场景接入率的数据。**
- **Unity Analytics 的口碑负面点集中在「被砍过一次」**：Legacy 版 2023 年关 dashboard、2024 年停收数据，论坛里有人抱怨「I never got any email about disabling remote settings (or unity analytics). Now 7 days before the end date I'm getting a 'reminder'」〔原话，Unity Discussions〕。**这对我们是个正面信号：开发者被平台方砍过分析服务，会更愿意用一个「不接 SDK 也有数据」的东西。**
- **接入门槛的真实形状**：Firebase 在 Unity 里要用 `#if UNITY_ANDROID` 把代码整段排除，否则 WebGL build 直接崩〔原话，medium.com 一位开发者〕：「If you just initialize Firebase blindly, your WebGL build will throw massive errors because the Google Mobile Ads/Firebase SDKs do not work on web browsers.」**这说明「网页游戏能用的分析 SDK」这个位置本身是空的。**

### (c) 录像是刚需还是卖点？

**是卖点，不是刚需。四条理由：**

1. **我一条「看了录像才发现 ___」的用户原话都没找到。** 搜了多轮，能找到的最接近的是一位 playtester（不是开发者）的博客〔原话，waldorodriguez.com〕：「That was pretty easy to see on my body language for anyone that watched the recording」——但这是在描述大厂 playtest 实验室，不是在描述工具。
2. **所有卖录像的产品，自己的文案都在说「你不想看录像」。** Playloop 首页：「Built for indie studios who'd rather ship than **scrub through a four-hour recording**」；PlaytestCloud：「If you don't have time to watch all of your recordings, use these tools to see which moments to zoom in on」；Turtlevania（Playloop 母公司）：「We couldn't watch four-hour playtest recordings forever, so we built Playloop」。**录像的整个下游生态都在解决「录像太长了」这个问题——这是一个产品在给自己造麻烦。**
3. **技术上在网页游戏里特别贵。** PostHog 的 Unity SDK 在 WebGL 上直接禁用 replay；PostHog 的 canvas recording 默认 4fps 且不自动开；Sentry 的 replay 加 50KB 且要 `preserveDrawingBuffer`〔均一手〕。**Playset 能做到是因为它托管整个页面、录的是标签页——这是一条我们不走的路（DESIGN §3.7「从不运行用户的代码」的精神一致）。**
4. **它的经济模型和 playtest 冲突。** Playset 按分钟收费、免费档 15 分钟/月。一次 30 人 × 10 分钟的测试 = 300 分钟 = 5 小时，落在 $19 档的一半。**开发者每多一个玩家就多一笔钱，这和「多找几个人来玩」是相反的激励。**

**但有一个场景录像是不可替代的**，来自 §2.6 那位多人游戏开发者：**多人游戏的 bug 无法从单个玩家的视角诊断**。「'He walked straight through my bomb' is only answerable if you can scrub to that tick and see what the other client believed was happening.」——这不是「录像」能解，是「时间对齐的多视角回放」才能解，比录像难得多。**留门条款应该改写成这个形状**（见 §6）。

### (d) AI 总结有人买吗？

**没有直接证据说有人买；有间接证据说它不值钱。**

- **PlaytestCloud 把 AI 分析免费包含在所有档位、不额外扣 video token**〔一手〕。这是最强的信号：**一家专业公司把它当作留客功能白送。**
- **Playloop 免费档给你「every analytics surface」包括「Full AI summary ladder」，条件只是自带 API key**〔一手〕。付费档 $49/$99 卖的是「Managed AI」——**卖的是 OpenAI 账单的代付和加成，不是智能。**
- **Antidote 把 AI Insights 放在 $475/月的 Pro 档**〔一手〕，说明它认为这是企业向的增值项，不是 indie 会买的东西。
- **AI 总结的价值和会话量成正比。** Side 明确说他们做 AI 是因为「Traditional playtesting can break down when moving from 50 to 50,000 players」〔一手〕。**50 个人以下，你自己读完所有反馈需要 10 分钟。**
- **付费意愿**：没查到任何独立开发者说「我为 AI 总结付了钱」。Playloop 本身还在 beta（「Free during beta. Paid tiers coming soon.」）。

**结论：DESIGN §8「AI 总结留门但不推进」这个判断是对的，而且可以更强——建议改成「不做，除非会话量到了人读不完的规模」。**

### (e)「知道哪里流量好」这个概念对 5–50 人的 playtest 成立吗？

**不成立。应该整个换掉。**

**「流量」这个词在这里错了三层：**

1. **统计上不成立。**〔原话，StraySpark〕200 个玩家就会有噪声误导；A/B 每变体要 200–300 人。20 个开发者 × 每版 5–10 个玩家的私测（DESIGN §8 的 T3 目标），任何比例都不可解释。
2. **概念上错位。** 「流量好」隐含「有一个可优化的转化漏斗」。playtest 阶段的目标不是转化，是发现问题。Nielsen 的框架说得清楚：这是 **formative testing**（发现问题，小样本，迭代），不是 **summative testing**（测量性能，大样本，统计）〔一手〕。
3. **它会让开发者做错事。** Poki 的建议是「start with a specific question」；Relish Games 的分析指南直接写〔二手〕：「do not change a design that is already working just because the numbers are interesting. Interesting is not the same as broken.」

**该换成什么：把「哪里流量好」翻译成三个可以数出来的问题。**

| 错的问法 | 对的问法 | 在什么量级成立 | 需要哪一层 |
|---|---|---|---|
| 哪里流量好 | **有几个人根本没进到游戏里** | 1 个人起 | 第一层（门禁页点击 vs 首帧） |
| 留存率多少 | **哪几个人回来了第二次**（点名，不是比例） | 1 个人起 | 第一层（cookie + 版本） |
| 转化漏斗 | **每个人玩到了第几步、在哪一步停下** | 3–5 个人起（Nielsen） | SDK（自定义事件） |
| 平均停留时长 | **停留时长的那一列数字**（中位数，不是平均——分布是偏的） | 5 个人起 | 第一层 |
| 错误率 | **这一版有几个人撞上了同一个错误** | 2 个人起 | SDK（错误）+ 第一层（404） |

**最有力的支撑是 Zach Barth 的那条**（§2.3）：他实证了「bounce 掉的谜题 = 通关者自述太难的谜题」。**「开始了但没做完」是一个可以在 5 个人身上数出来的指标，而且它能代理一个原本要问卷才能拿到的判断。**

**建议 DESIGN §3.4 的控制台文案彻底避开「分析」「流量」「留存率」这些词**，改用「这一版：8 个人打开、6 个人进到游戏、2 个人回来过、3 个人在第 2 关之后没有再动作、1 条反馈」这种句子。**它读起来应该像一份点名册，不像一个仪表盘。**

### (f) 反馈按钮的实际填写率：T6 现实吗？

**现实，但边界比想象窄。**

**能找到的基准（全部是通用 SaaS，不是游戏）：**

| 渠道 | 数字 | 样本 | 来源 |
|---|---|---|---|
| In-app survey（平均） | **27.52%** 响应 / 24.84% 完成 | 1,382 个 survey，5000 万+ 曝光，2025 | Refiner〔二手，厂商自研报告〕 |
| In-app survey（web app） | 26.48% | 同上 | Refiner〔二手〕 |
| **网站挂件 survey（中位）** | **7.64%** | Survicate 3,095 个 widget survey，2025 | 经 TruRating 引用〔二手〕 |
| 所有数字调研（中位） | 9.98% | Survicate 数据集 | 同上〔二手〕 |
| Email survey | ~15% | Delighted 自家基准 | 经 Userpilot 引用〔二手〕 |
| 电商 email | 3.24% | Retently，2500 万+ 邀请 | 经 TruRating 引用〔二手〕 |
| In-game survey（游戏） | 声称 30–40% vs 外链 5–10% | **无样本量、无方法** | FirstLook〔厂商自述，不可信〕 |

**取哪个数：** 我们的反馈按钮更像「网站挂件」（常驻、被动、不打断），不像「in-app survey」（主动弹出、有触发时机）。所以**保守取 7.6%，乐观取 27%**。

| 每版打开人数 | 7.6% | 27% |
|---|---|---|
| 5（T3 目标下限） | 0.4 条 | 1.4 条 |
| 10 | 0.8 条 | 2.7 条 |
| 30（一轮真实测试） | 2.3 条 | 8.1 条 |

**结论：T6（每版反馈数 ≥ 1）在「每版 10 个人以上」时安全达标，在 T3 刚好卡在 5 的时候是 coin flip。**

**对照组（这个比工具基准更重要）**——itch.io 上 jam 作品拿到反馈的真实情况〔一手〕：

- itch 管理员 leafo：「the largest jam we've hosted collecting over 160k total ratings, with many entries collecting hundreds and even over a thousand ratings, **has a median number of ratings of 16**.」
- 一场普通的小 jam（itch.io/jam/2025-game-jam/results）：**185 个作品，218 条评分给了 104 个作品（56.2%）；平均 1.2 条、中位 2 条。44% 的作品一条都没有。**
- 一个具体案例〔原话，Skele-Tom 的 post-jam devlog〕：作者主动去评了 128 个游戏（占参赛总数的 44.6%），换回 70 个人给他评分（24.4%）、66 个人留言。「More than half my ratings (38) came [in the] first two days of rating... 'I rated your game and I hope you rate yours'」——**在 jam 里拿到反馈的主要手段是互评，不是作品本身。**

**这组数字的含义：在开发者最需要反馈的场景（jam），一半的作品拿到零条反馈。我们的反馈按钮如果能稳定做到「每版 ≥ 1 条」，就已经超过了 itch 一半的作品。**

**能提高填写率的四个已知杠杆**：
1. **不要必填任何能自动采集的字段。**「Every time you add one of these required fields, the amount of feedback you get will drop.」〔原话，Feature Upvote〕
2. **单问题。**「A single well-timed question... can hit 30%+ response rates. Turn it into a five-question survey and watch that number collapse to under 10%.」〔二手，TinyAsk〕
3. **常驻按钮 > 主动弹窗。** Userpilot 的一条反直觉数据〔原话〕：他们在产品里放的常驻反馈按钮，「these passive surveys get about **four to five times better response rates** than the other targeted feedback surveys we run」。
4. **限流。** Playloop 的实现是「每 (session, form) 一次提交、每 session 最多 3 次」〔一手〕——**这不是为了少收，是为了防刷。**

---

## 4. 建议的最小集与优先级

### 4.1 第一层（不改一行代码，边缘 + 门禁页）

| # | 数据 | DESIGN §3.4 现状 | 建议 | 理由 |
|---|---|---|---|---|
| L1 | 每次打开的时间 | 有 | **保留** | 会话的骨架 |
| L2 | 设备与浏览器 | 有 | **保留并提升为一等公民** | §(a).3 网页游戏碎片化是一等问题；这是别人拿不到的东西 |
| L3 | 来自哪里（微信/浏览器/Discord） | 有 | **保留，但降级为「尽力而为」** | referrer 本身脏（itch 用户实证）；微信内置浏览器常不发 referrer。实现上要 UA + 链接参数 + referrer 三路兜底，并在界面上标「可能不准」 |
| L4 | 停留多久 | 有 | **保留，但只显示中位数和逐条数值，不显示平均值** | 分布是偏的〔StraySpark〕；5–50 人时逐条列出比任何汇总都有用 |
| L5 | 资源加载失败（404 / 中断下载） | 有 | **保留并提升为一等公民** | §(a).1 的 44% 流失；这是第一层里唯一直接对应「玩不了」的信号 |
| L6 | 多少人开过 / 多少人回头 | 有 | **保留，但改成点名不是比例** | 5–50 人时「3 个人回来过」比「回头率 27%」有信息量 |
| **L7** | **「点了开始」到「游戏首帧/首次交互」之间掉了几个人** | **没有** | **新增，第一优先** | §(a).1 + §(a).2：最大的一段流失就在这里，而且完全不需要 SDK——门禁页点击是我们埋的，首帧可以由边缘观测「最后一个关键资源被完整下载」来近似。**这是第一层里唯一一条能独立支撑「知道结果」这个承诺的数据。** |
| **L8** | **这一版每个玩家一行的「会话卡」（时间 / 设备 / 来源 / 停留 / 有没有加载失败 / 是不是回头）** | 部分（§3.4 说「点进去是会话列表」） | **明确为一等界面，而不是二级页面** | §(e)：控制台应该读起来像点名册。DESIGN 现在的「每个版本一行（打开数、独立人数、错误数、反馈数）」是仪表盘句式，会被当虚荣指标 |

**建议删掉的：无。** 第一层的每一项都有支撑。

### 4.2 SDK 层（`playtest.js`）

| # | 能力 | DESIGN §3.4/§4.6 现状 | 建议 | 理由 |
|---|---|---|---|---|
| S1 | JS 错误与堆栈 | 有 | **保留，第一优先** | Sentry 是事实标准说明这是刚需；WebGL 崩溃只能靠上报 |
| S2 | 加载完成用时 | 有 | **保留，改成「加载分阶段」而不是只有一个总时长** | 和 L5/L7 拼成完整的加载故事：请求发出 → 关键资源下完 → 引擎初始化 → 首帧 → 首次输入 |
| S3 | 自定义事件 `playtest.event()` | 有 | **保留，但文档只教一种用法：标记进度里程碑** | Poki 的建议「start with a specific question」；不要教漏斗、不要教属性维度分析 |
| S4 | 反馈按钮（一句话 + 截图 + 设备 + 版本 + 进入多久） | 有 | **保留，但把「尽力而为的截图」改成「默认能截到」** | 见下 |
| **S5** | **「玩到了哪里就没动作了」——最后一个事件 + 最后一次输入的时间戳** | 没有 | **新增，优先级仅次于 S1** | Zach Barth 的 bounce 证据（§2.3）；这是 5 个人量级上唯一能代理「太难/无聊」的行为指标，而且实现成本几乎为零（S3 的副产品） |

**关于 S4 的截图（一个可以修正 DESIGN 的技术细节）：**

DESIGN §3.4 写「附当前画面截图（尽力而为，WebGL 画布需要 `preserveDrawingBuffer` 才截得到）」。这个限制**在「边缘自动注入 SDK」的上传模式下可以绕过**：在游戏脚本执行之前 monkey-patch `HTMLCanvasElement.prototype.getContext`，对 `webgl` / `webgl2` / `experimental-webgl` 强制注入 `preserveDrawingBuffer: true`〔一手，StackOverflow 54047609 gman 的回答〕：

> 「Likely you either need to read the pixels in the same event as they are rendered, or you need to force the canvas to use `preserveDrawingBuffer: true` so you can read the canvas at any time.」

代价是 canvas 性能会受影响（Sentry 官方也这么说）。**所以正确的产品形态是：上传模式下由边缘注入并默认开启，CLI 用 `--no-screenshot` 可关；隧道模式下开发者自己加 SDK，就退回「尽力而为」。** 这比 DESIGN 现在的一刀切「尽力而为」更准确，也更能兑现「知道结果」。

**建议不做的（明确写进 §3.7）：**

- **热图 / 漏斗 / 分群 / A/B**——§(e) 的统计理由。
- **DAU / MAU / 留存曲线 / ARPU**——这是运营中的游戏的语言，不是一次 playtest 的语言。GameAnalytics 的整个指标体系就是这个（§1.7），我们做了只会变成一个更差的 GameAnalytics。
- **AI 总结**——§(d)。
- **画面录像**——§(c)。

### 4.3 与 DESIGN §3.4 的逐条对照

| DESIGN §3.4 原文 | 调研判断 |
|---|---|
| 「每次打开的时间」 | ✅ 保留 |
| 「设备与浏览器」 | ✅ 保留，**证据比预期强，应提为主打** |
| 「来自哪里（微信 / 浏览器 / Discord）」 | ⚠️ 保留，**但数据脏，界面要如实说** |
| 「停留多久」 | ⚠️ 保留，**只给中位数和明细，禁用平均值** |
| 「资源有没有加载失败」 | ✅ 保留，**证据最强的一条，应提为主打** |
| 「多少人开过、多少人回头」 | ⚠️ 保留，**改点名不改比例** |
| —— | ➕ **新增 L7：门禁页到首帧的掉队人数** |
| 「JS 错误与堆栈」 | ✅ 保留，SDK 第一优先 |
| 「加载完成用时」 | ✅ 保留，**改成分阶段** |
| 「自定义事件」 | ✅ 保留，**文档只教里程碑一种用法** |
| 「反馈按钮 + 截图（尽力而为）」 | ✅ 保留，**上传模式下截图可以做到默认成功，不必写「尽力而为」** |
| —— | ➕ **新增 S5：最后一个事件 / 最后一次输入** |
| 「控制台每个版本一行（打开数、独立人数、错误数、反馈数）」 | ❌ **句式错了。会被读成流量面板。建议改成一版一段话 + 一张会话点名册** |
| 「不收集玩家身份、跨站追踪、精确位置；90 天保留」 | ✅ 保留，**并且应该把「合规责任在我们」写成卖点**（itch 用户的 GDPR/PIPL 痛点） |

---

## 5. 对 playtest.run 的含义

### 5.1 支持 DESIGN 现有结论的发现

1. **§0.4「名字里的 playtest 兑现在知道结果」——成立，而且第一层比想的更硬。** 加载流失 44%、设备浏览器碎片化、白屏原因（MIME / COOP-COEP / 404）——这些全在边缘可见，而且是网页游戏独有的失败形态。itch.io 明确做不到（「they are essentially a file sharing service」），Steam Playtest 明确不做，通用隧道工具不管。
2. **§2.2「结果层要做得比 Playset 轻而不是更全」——成立。** Playset 的录像在网页游戏上技术贵、经济模型反激励、下游生态都在解决「录像太长」，而且我找不到一条用户说它救了他。
3. **§3.3 门禁页作为「版本告示牌」——被独立印证。** 开源 Godot 插件的问题陈述几乎逐字命中：「you can't tell which build they were on, and triage becomes archaeology」。
4. **§3.7「不做通用分析平台、热图、漏斗」——被统计学支持。** 5–50 人量级上这些概念不成立。
5. **§8「AI 总结留门不推进」——成立，可以更强。** 三家专业公司里两家白送 AI，第三家放在 $475 档。
6. **§3.4 的隐私立场——是差异化，不只是合规。** itch 用户的原始诉求就是「接 GA4 就要自己扛 GDPR/PIPL/CCPA，小团队扛不起」。**「我们收数据、我们担责任、开发者不用碰」应该从合规条款升格为产品卖点。**
7. **T6 ≥ 1 的目标现实。** 基准填写率 7.6%–27.5%；对照组 itch jam 有 44% 的作品拿到零条反馈。

### 5.2 与 DESIGN 冲突或需要修正的发现

**冲突 1：§0.2 / §4.2「交付两条路都做」的价值主张，在浏览器游戏社区里可能已经过时。**

itch 论坛上做浏览器多人游戏的开发者原话：「**Build distribution is already solved for me. It is a URL.** A tool whose core pitch is 'manage builds and get them to your testers' has nothing at all to sell me, and browser games are not a small slice of itch.」

DESIGN §2.3 已经说了「上传路径不是差异化，是入场券」，但整篇文档在 §4.2 上花的篇幅远超它的战略权重。**建议：不改架构，改文案和优先级——上传路径在对外表述里的唯一卖点应该是「大陆可达」和「对游戏有感的响应头」，不是「方便」。**

**冲突 2：§3.4 控制台的呈现形式（「每个版本一行：打开数、独立人数、错误数、反馈数」）会被读成虚荣指标。**

HN 上对通用分析的经典评价：「it doesn't actually report any useful or actionable data, just vanity metrics」。itch 用户对 itch 面板的评价：「trivial, mostly few Gross metrics... this is not very helpful」。**四个数字排一行，长得就像一个 Umami。**

建议改成：**每版一段自然语言 + 一张会话点名册**。例如：

```
v7 · 上传于 9 月 5 日 14:20 · 「改了新手引导」
  8 个人打开，6 个人真的进到游戏里（2 个人在加载时走了 —— 其中 1 个是
  微信 Android，卡在 game.wasm 下到 61% 中断）
  3 个人玩了 5 分钟以上，2 个人回来过第二次
  1 个 JS 错误撞了 3 次（TypeError: Cannot read 'x' of undefined @ main.js:412）
  1 条反馈：「不知道要按哪个键」—— 附截图 · iPhone 15 Safari · 进入 47 秒
```

**冲突 3：T6「每版反馈数 ≥ 1」测的不是它想测的东西。**

DESIGN 说 T6 是「『知道结果』有没有被用」的判据，而且「T6 长期为零，砍掉 SDK 只留第一层」。但：
- 业界共识说文字反馈本来就是次要信号（§2.2）；
- 反馈数主要由**打开人数**和**按钮设计**决定，不由「结果层有没有用」决定；
- 如果 T3（每版打开人数）刚好在 5，T6 达标是 coin flip（§(f) 的表）。

**建议：把 T6 拆成两个数。**
- **T6a（反馈数 / 打开人数）≥ 8%**——测反馈按钮本身。低于这个说明按钮的设计有问题（字段太多、位置不对），不是说明玩家不想说话。
- **T6b（发过第二个版本的开发者里，有多少人在发新版之前看过上一版的结果页）≥ 60%**——**这才是「知道结果」有没有被用。** 而且它只需要第一层，不需要 SDK。

**建议：把「T6 长期为零就砍 SDK」的条件改成「T6a 长期低于 3% 就砍反馈按钮，但错误上报（S1）无论如何保留」**——因为错误上报的价值不依赖玩家的配合。

**冲突 4：§4.3 隧道路径缺了联机 playtest 的核心问题。**

DESIGN §4.3 用了很大篇幅论证 WSS + yamux vs QUIC vs WebRTC，但那位多人游戏开发者说的瓶颈完全不在这里：

> 「What is not solved... is concurrency. The bottleneck in multiplayer playtesting is never getting the build out or capturing the bug — it is getting four people into the same lobby at the same time. And a tester who turns up alone does not give you a weak signal, **they give you a misleading one**.」

他愿意付钱的东西是「**scheduled session**：选一个 20 分钟的窗口，N 个测试者承诺到场，大家同时拿到同一个 build，session 回来是一个把每个参与者视角时间对齐的整体」。

**建议：不要在 v0.1 加这个功能（它是一个完整的产品），但要在 DESIGN §8 的「留门」里替换掉现在的「画面录像」条目**，改成：

> **联机 playtest 的并发问题**（约定时间窗口、N 人同时在线、多视角时间对齐的回放）：这是隧道路径上唯一被用户明确说出「我今天就买」的需求，但它是一个完整的产品，不是一个功能。只有在 T4 显示隧道占比超过 30% 且出现两个以上联机作品时再评估。

**冲突 5：§2.2 对 Playset 的描述隐含「它已经在收钱了」，但我没有任何证据。**

DESIGN §2.1 写「Playset ... 纽约两人团队，2025」，§2.3 写「Playset 的存在既证明有人要『一键 playtest』，也证明这条路上已经有人在收钱」。**「有人在收钱」这句话我查不到支撑**——没有独立用户评价、没有社区讨论、没有用量数据。**建议改成「有人在这条路上定了价」**，并把「缺口不等于需求」的警告保持在原位。

### 5.3 一条创始人自我提醒的印证

创始人说「过去太喜欢找差异，但很多时候赢是因为在同一件事上做得比别人好」。本轮调研支持这个反省，而且指出了「同一件事」是什么：

**所有人都在做「上传 → 链接 → 看结果」这三步。区别在第三步的重量。**

| | 第三步的重量 | 对 5–50 人的适配 |
|---|---|---|
| itch.io | 极轻（views / browser plays） | 太轻，用户抱怨 trivial |
| The Gaming Nest / SIMMER | 无 | 无 |
| Playset | 极重（录像，按分钟计费） | 太重，免费档一个人测不完 |
| Playloop | 重（SDK + 三层 AI） | 太重，5–50 人没有量给 AI 嚼 |
| Antidote / PlaytestCloud | 极重（招募 + 录像 + 转写 + AI，$115–475/月） | 完全不是这个市场 |
| **中间是空的** | **一页话 + 一张点名册 + 一个反馈按钮 + 错误上报** | **正好** |

**我们不需要一个新概念，我们需要把「一次 playtest 之后你想知道的那五句话」做得比所有人都清楚。** 这正好是创始人反省的那种赢法。

### 5.4 建议的控制台信息架构（一条具体建议）

不要「作品 → 版本 → 会话」三层树。改成：

- **一屏：作品的时间线。** 每个版本一张卡，卡上是 §5.2 冲突 2 里那段自然语言。版本之间的差异用一行标出（「比 v6 多 3 个人打开，加载失败从 2 个降到 0」）。
- **点开一张卡：这一版的点名册。** 每个会话一行，横向是时间 / 设备 / 来源 / 停留 / 加载成没成 / 玩到哪 / 有没有留话。**可以按「停留最短」排序——这一列排在最前面的人就是你要看的人。**
- **反馈单独一条流**，每条带版本、设备、进入多久、截图。
- **不要图表。** 5–50 个数据点画柱状图只会显得空。

### 5.5 建议新增的私测指标

DESIGN §8 现在有 T1–T6。建议加两个，都不需要新功能：

- **T7 · 结果页回访率**：发过第二个版本的开发者里，有多少人在发新版之前打开过上一版的结果页。目标 ≥ 60%。**这是「知道结果」是不是产品的直接判据**，比 T6 准。（附记：**「开发者会看几次结果页」这个数我在全网没有查到任何公开基准**，所以这个 60% 是我拍的，只能用作内部相对比较。）
- **T8 · 门禁页到首帧的掉队率**：目标只是「记录」。如果这个数在真实作品上稳定在 20% 以上，**L7 单独就能撑起整个结果层的价值主张**，SDK 的优先级可以再降。

---

## 6. 我没查到 / 没验证的

诚实列出，不要当作「已验证」引用：

1. **Playset 的任何独立用户评价、用量、留存。** 搜了 Reddit、HN、itch 论坛、Product Hunt，零结果。我只有它的官网。「有人在买」没有证据。
2. **Playloop 的付费用户。** 它还在 beta，官网写「Free during beta. Paid tiers coming soon.」定价页写的是计划中的档位。
3. **游戏 playtest 场景下反馈按钮的真实填写率。** 所有基准（7.6% / 27.5%）都来自通用 SaaS 的 in-app survey，不是游戏、不是 playtest。FirstLook 声称的「in-game surveys 30–40%」没有样本量和方法，是厂商自述。
4. **开发者会看几次结果页。** 全网零公开数据。
5. **GameAnalytics 在网页游戏 / playtest 场景的接入率。** 只有全局数字（10 万游戏、2 亿 DAU），几乎全是手游。AppGoblin 的 SDK 渗透率数字（Android 2.95%）是第三方反编译统计，口径不明。
6. **GameAnalytics 免费档「100k MAU、无限自定义事件」的官方确认。** 这个数字来自一个聚合评测站，我没有在 GameAnalytics 官方定价页确认到。
7. **itch.io「browser plays」的准确定义。** 是「点了 Run game」还是「加载完成」？没找到官方说明。用户帖提到「People who play web games in the Itch app show up in stats as downloaders」，说明它的口径本身有洞。
8. **GameArter 那组 44% 加载流失数据的可复现性。** 作者自述样本量小、是门户站休闲游戏、时间偏早（内文引的网速统计是 2017–2018 的）。**数量级我信，精确值不能引用。** 建议在 `docs/spikes/` 里用我们自己的第一个真实作品复测一次。
9. **微信/大陆场景的任何 playtest 数据。** 微信小游戏、TapTap、4399 的开发者能看到什么数据，本轮完全没查（不在本线任务范围，但对 DESIGN §5 有直接影响，建议另开一线）。
10. **DESIGN §3.4「多少人是回头的」在只有本站 cookie、玩家不登录、24 小时门禁页免打扰的前提下能不能算准。** 我没有查证 Safari ITP / 微信内置浏览器对第一方 cookie 的存活期。**这是一个技术 spike，不是调研题。**
11. **Reddit 上的原始讨论。** Reddit 对我的抓取返回 403，本轮所有「reddit 原话」都是通过搜索引擎的摘要间接获得的（Zach Barth 的 AMA 是通过 bestofama.com 的镜像拿到的）。**引用前建议手工核对原帖。**

---

## 附：本轮主要来源清单

**一手（官网 / 文档 / 定价页）**
- Playset — https://playset.app/
- Playloop — https://playloop.gg/ · /pricing · /docs
- Antidote — https://antidote.gg/pricing/ · /faqconc/what-is-the-cost-of-the-platform/
- PlaytestCloud — https://www.playtestcloud.com/single-session-playtest · /ai-powered-analysis · https://help.playtestcloud.com/en/articles/1148763-setting-up-your-first-playtest
- Steam Playtest — https://partner.steamgames.com/doc/features/playtest
- Unity UGS 定价 — https://unity.com/products/gaming-services/pricing
- GameAnalytics 文档 — https://docs.gameanalytics.com/events-metrics-and-filtering/metrics
- Sentry Session Replay 排错 — https://docs.sentry.io/platforms/javascript/session-replay/troubleshooting/
- PostHog Unity SDK — https://posthog.com/docs/libraries/unity · Canvas recording — /docs/session-replay/canvas-recording
- Poki Game Events — https://poki.com/blog/game-events-new-tool-for-understanding-your-players
- GameArter 加载流失 — https://www.gamearter.com/blog/importance-of-game-loading-time
- SlowDen 门户实测 — https://slowden.com/blog/why-players-leave-before-your-game-starts/
- The Gaming Nest — https://thegamingnest.com/en/webgl-hosting
- NN/g 5 users — https://www.nngroup.com/articles/why-you-only-need-to-test-with-5-users/ · 90-9-1 — /articles/participation-inequality/
- Unity 手册 Web 内存 — https://docs.unity3d.com/6000.7/Documentation/Manual/webgl-memory.html
- Forge Logger（开源 Godot 插件）— https://github.com/stoneforgelabs/forge-logger-godot
- GoatCounter — https://github.com/arp242/goatcounter/

**用户原话（论坛 / 访谈 / 博客）**
- itch.io「Adding actual analytics on Itch」— https://itch.io/t/5281744/adding-actual-analytics-on-itch-for-our-own-releases
- itch.io「Would you pay $9/month」— https://itch.io/t/6804701/im-building-a-playtesting-tool-for-game-developers-would-you-pay-9month-for-it
- itch.io jam 评分算法与中位数 — https://itch.io/t/644401/calculated-ratings-for-jams-vs-raw-scores
- itch.io 小 jam 结果页 — https://itch.io/jam/2025-game-jam/results
- Skele-Tom post-jam devlog — https://dragonforge-development.itch.io/skele-tom/devlog/1096114/post-jam-wrap-up-skele-tom
- GMTK「Valve's Secret Weapon」— https://gmtk.substack.com/p/valves-secret-weapon
- Mark Brown / RPS 访谈 — https://www.rockpapershotgun.com/how-ten-years-of-game-makers-toolkits-design-analysis-informed-mind-over-magnet
- Zach Barth AMA — https://bestofama.com/amas/78wv2h
- Soren Johnson「Taking Feedback」— http://www.designer-notes.com/game-developer-column-19-taking-feedback/
- Tynan Sylvester on 平衡与 theorycrafting — https://ludeon.com/forums/index.php?topic=41839.0
- Casey Weeks（引 Mike Ambinder、Adriaan de Jongh）— https://caseyweeks.com/how-to-make-a-better-game-in-less-time/
- Playful / Teeto 案例（开发者 Mitch）— https://www.beplayful.gg/blog/playtesting-teeto
- Userplay 建设手记 — https://userplay.hashnode.dev/what-we-learned-building-playtesting-infrastructure-for-game-studios
- Feature Upvote「How to get player feedback」— https://featureupvote.com/blog/how-to-get-player-feedback/
- 一年 Unity WebGL — http://ernesernesto.github.io/writes/oneyearwebgl/
- HN「Lightweight Alternatives to Google Analytics」— https://news.ycombinator.com/item?id=23560823
- StackOverflow：强制 preserveDrawingBuffer — https://stackoverflow.com/questions/54047609/how-do-i-obtain-pixel-data-from-a-canvas-i-dont-own

**二手 / 推断（引用需谨慎）**
- Refiner 2025 in-app survey 报告 — https://refiner.io/blog/in-app-survey-response-rates/
- TruRating 2026 基准汇总（引 Survicate / Retently）— https://trurating.com/blog/response-rate-for-customer-satisfaction-surveys/
- MeasuringU 对「5 users 85%」的修正 — https://measuringu.com/five-user-85/
- StraySpark 独立开发者分析指南 — https://www.strayspark.studio/blog/game-analytics-indie-developers-player-behavior
- Bugnet 系列（游戏内反馈 / Discord bug 频道 / WebGL 崩溃）— https://bugnet.io/blog/
- GamesIndustry.biz GameAnalytics IQ Suite — https://www.gamesindustry.biz/from-data-to-decisions-heres-how-gameanalytics-iq-suite-will-power-the-next-era-of-game-growth
- Socratopia 独立开发者手册（朋友家人偏差）— https://www.socratopia.app/library/indie-game-playbook-en/chapter-8
- Sentry 评测 2026 — https://pikvue.com/sentry-review-2026-error-monitoring-that-indie-developers-actually-need/
