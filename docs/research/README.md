# 市场调研综合 · 2026-09-07

> 13 条并行调研线的汇总。每条线一份独立报告（见文末索引），本文做四件事：回答创始人的三个判断、回答「找差异还是做得更好」、列出必须改和建议改 `DESIGN.md` 的地方、给出综合后的产品判断。**本文不改 DESIGN.md**，改不改由创始人定。
>
> 证据口径沿用各报告：一手（官网 / 文档 / 定价页 / 规范原文）、用户原话（带链接）、二手 / 推断。除真机试用线外，所有「几秒 / 多久」都是从文档推的，不是实测。运行记录：13 条线中 4 条（Playset、英文原话、真机试用、微信）在 12:25–13:06 间被平台用量上限中断，随后续写恢复并完整交付；真机线的原始终端输出未能恢复（报告 §0 分「硬证据 / 跑过但输出丢了 / 只读了文档」三档标注），Playset 线第一轮的抓取材料未落盘、第二轮重建。

## 0. 一句话结论

**方向对，承诺写得太满，入口和护城河的位置写反了。** 「一条命令把版本放到具体的人面前，然后知道他们玩成了什么样」被 13 条线反复支持；但 DESIGN 里有四句话现在写得像事实、实际上是待验证假设，其中「微信里点开就能玩」撞上了微信 2025-10-23 生效的外链规范原文。英文原话线的频次表说明了一件 §0 没说清的事：**托管坑是频率最高的痛（17 条、跨十年），反馈黑箱是最深的痛（22 条、集中在近 15 个月）——入口是托管，留住人的是结果。** 竞品格局比 DESIGN §2 写的更拥挤（上传侧、结果侧）也更空（大陆、微信、命令行入口）。「找差异」的习惯在文档里有四处痕迹，调研给出的替代答案是几件具体的、可验收的「做得更好」。

## 1. 三个判断

### 判断 1 · 「方便简洁、使用体验好是第一位的」—— 成立，但「体验」要重新定义

13 条线没有一条反对。分歧在「体验」指什么：

- **不是 CLI 参数最少。** `surge ./dir` 一条命令、免登录、不可变版本、`surge rollback`、`surge stats`，已存在多年（上传线）。免登录出链接是行业常态：Netlify Drop、Surge、tiiny.host、Final Parsec（Unity 插件一个按钮、事后 claim）、ArcadeLab、rendrd 全都做了。DESIGN §3.2 把「免登录 24 小时匿名链接」当亮点写，它是入场券——但同时它是 GitHub OAuth 在大陆间歇失败时唯一的兜底（中文原话线、微信线），**必要性远高于亮点性**。
- **是端到端第一次成功率**：`安装/匿名 → 拿到链接 → 在微信/Discord 里发出 → 玩家信任并点开始 → 加载成功 → 开发者看到结果`（全球隧道线）。ngrok 的 CLI 很强，但访客侧警告页、必须登录、价格阶梯让临时演示用户流失；localtunnel 访客要输入开发者公网 IP，前面省的事全被抵消。
- **「几秒拿到链接」只在单静态二进制的前提下成立**（真机线）。npx 一族的时间大头不是建隧道是下包：`bore-cli` 解包 10 MB；`localhostvibe` 包只有 46 KB 但一条命令实际拉了 130 个 npm 包；kshare 把「Thirty seconds later」当卖点写在 README 第一屏。ssh 一族（localhost.run / tinyfi.sh）是唯一能进「一两秒」量级的，因为不装任何东西。§4.7 选 Rust 单二进制方向对，但**第一次运行到出二维码的耗时应该是写进验收标准、并在 CLI 里打出来的数字**——所有人都在比这一条，没有一家给你看数字。
- **真正的对手常常不是另一家托管，是「算了不发了」和「录个视频发 B 站」**（中文原话线：Cocos 论坛求职作品帖里被附议的答案就是录视频）。如果不能显著短于录屏 + 网盘，产品没有成立空间。
- **上传这一侧「做得更好」有非常具体的落点**（上传线 17 项打分表里空得最彻底的格子）：
  1. **上传那一刻告诉你为什么跑不起来**——全行业最差的一格。Unity Play 给「Network Error: Unknown Error」和裸 500；itch 官方回复是「open the browser console」；服务端此刻明明已经拿到全部文件，却没有一家检查 Godot 导出物是不是线程版、`index.html` 在不在根、`.br` 与 Decompression Fallback 有没有冲突。英文原话里有人为搞定 Brotli 头「I can pay you by the hour」，有人「I can't believe I spent 4 hours on this」。
  2. **改一行三秒上线**——只有 itch 做了真增量，代价是后端最长 30 分钟 processing；其余全部整包重传。
  3. **终端二维码**——上传类产品一整行全空；HN 上 Gaming Couch 的用户反馈原话「Telling people to "scan the QR code" is great for getting it going」。
  4. **音频用户手势由托管方接手**——Unity 和 Godot 官方文档都在教开发者自己做 splash，没有一家托管方替他们做；一位做了 11 个浏览器小游戏的作者：「two of my games had shipped silent on every iPhone, because touchstart grants no user activation … No player ever reported it.」
  5. **COOP/COEP 一个开关就对、且不打坏别的**——只有 itch 做了，四年长尾 bug（YouTube 嵌入、Safari 弹窗、Firefox token 过期、口令 404）。
- **AI 编程用户这一类要额外注意两点**（AI 编程线）：抽样 50 个 vibe coding 作品，纯游戏只占 14%，微型 SaaS / 工具 / 生成器占 78%——他们不认「playtest / 试玩」这两个词；他们手上是 `npm run dev`，很多人不会构建，**对这类人隧道是第一路径**。这和 DESIGN「上传占多数」相反，见 §5 分歧 1。
- **真机线的一个小坑值得记**：npm 上的 `bore-cli` 和 GitHub 上那个 Rust bore 同名不同物，照 README 抄命令两次都失败——命名冲突会直接吃掉用户的前两分钟。

### 判断 2 · 「建立反馈、曝光、孵化机制或生态」—— 反馈做，曝光不做，孵化不做，中间有一条窄路

**反馈：做，证据充分，而且形态已经清楚。** Kongregate 关闭六年后 2026-04 重开投稿，卖点原话是「the analytics and community tools you've been missing」；Playloop 免费档就给游戏内反馈表单；tunr 注入反馈组件；国内开发者的做法仍是 QQ 群 + 问卷 + 手工发码。英文原话线最痛的一条：「Every playtest goes the same way: I send the build, they play it, and a few days later I get "pretty fun, found one bug." What I actually want to know is whether they understood the mechanic, where they hesitated, what they clicked that wasn't a button. **I never find out.**」

**公开曝光 / 榜单 / 大厅：不做，反例密集且都是 2025 年的。**

| 案例 | 时间 | 发生了什么 |
|---|---|---|
| Glitch | 2025-07-08 关停托管 | CEO 原话：成本随平台老化和 bad actors 滥用而大增，且「不再提供独特价值」 |
| SIMMER.io | 2025-04 停服 | 与 playtest.run 形态几乎一样（上传 Unity WebGL → 链接），做了 8 年，DDoS 打出「astronomical cloud bills」，创始人删掉自己的存储桶止血 |
| Replit | 2024-08 关 Teams for Edu；2025-09 关 Bounties | 巨额亏损 + 欺诈滥用，全面转向高客单 AI 订阅 |
| itch.io | 2025-07 | 支付通道施压，被迫全站 deindex 成人内容，创作者 payout 冻结 |
| Newgrounds | 2025-08 | 25 年品牌、约 9,000 付费 Supporter、约 $27k/月，**仍在亏损**；2026-04 首次涨价 |

一旦做公开大厅，内容审核、排序公平（itch jam 的「中位数惩罚」公式被反复控诉）、反作弊、支付商压力全部落到一个两人团队头上；而且 playtest.run 的内容域是境外、不备案、匿名可发的——公开浏览页会把整域推到微信和监管的正面。

**孵化：没有任何一条线找到支持证据。**

**中间那条窄路（生态线提出，其余线部分支持）：把「结果」从开发者的私人看板，变成可以交给具体第三方的「一页可信作品简历」。** 证据是发行侧已经把数据化 playtest 当硬前置：Poki（1 亿 MAU）2024 年推出 Poki Playtesting；CrazyGames（5,000 万 MAU）强制 2 周 Basic Launch 关广告跑留存，达标才给推荐；Kepler Interactive 表单原话「Demo > Video > Concept Art > Words … generally unable to assess projects without a build」；TapTap 聚光计划提交材料明确要「以往玩家的试玩/测试反馈」。这条路不与 §3.7 冲突——受众仍是「具体的人」，不是大众。但要克制：商业线把 jam 主办方和学校排在买家优先级第 5、第 4（没预算、采购慢），它们是获客渠道不是首个付费对象。

**一个所有线都没预料到的相邻痛点：「没人玩」比「发不出去」更常见**（中文原话线：「前前后后花了一个月做的项目，上了抖音、微信和快手，根本没人玩」；英文原话线：「it's pretty common for people to say things like "no one I know cares about what I'm making"」）。Playset 线从供给侧看到同一件事：「结果」这一类有一半产品的价值在「帮你找到人」，而 DESIGN §1.2 默认开发者已经有 5–50 个具体的人。这不改变「不做曝光」的结论，但意味着 14 天私测里要直接问：**「你把链接发给了谁？你从哪找到这些人的？」**——如果多数人答不上来，§1.2 五个时刻需要重写。

### 判断 3 · 「提供数据分析，让开发者知道哪里流量好、哪里有问题」—— 做，但「流量」和「谁」两个词都要换掉

**「流量」在 5–50 人的 playtest 里在统计上不成立。** 独立开发者分析指南原话：200 个玩家仍充满统计噪声，A/B 每变体需 200–300 人。对 20 个具体玩家，正确句式不是「37% 在第三关流失」，而是「这 3 个人停在第三关」——**点名，不是比例**（数据线）。

**「谁」是我们想给的，不是开发者要的**（英文原话线）。22 条反馈类原话里没有一条要求知道玩家是谁；有两条明确的隐私反弹（「I personally would not install a keylogger just to give feedback」，作者解释完「不装东西、只在标签页内、事先告知」后对方仍回「Trust me bro lol」）。他们主动要的全是行为：「I log a few events (first click, time to first real action, where ppl drop)」「stick to analytics more than feedback forms」「800 people signed up … 270 actually loaded it up … 10 minutes median play time」。**§3.4 第一句「谁打开了」应改成「多少人打开、进到游戏、玩到哪、在哪断」——他们要的是一面镜子，不是一份名单。** 顺带一条实现细节：那位 iPhone 上静音的作者说的是「touchstart grants no user activation」——门禁页的「开始」必须绑 click / pointerup，不能绑 touchstart，否则 iOS 上音频照样不响。

**第一层数据被 DESIGN 严重低估。** 一家网页游戏门户实测：从「开始加载」到「加载完成」之间流失 44%±5%，各浏览器一致（样本小、约 2018 年，数量级可信、精确值不可引用）。最大的一批人根本没进游戏，而这不需要 SDK，边缘就看得见。数据线建议新增 **L7：门禁页点「开始」到首帧之间掉了几个人**——第一层里唯一一条能独立撑起「知道结果」承诺的数据。

**录像是卖点不是刚需，而且正在跌价到零。** 13 条线没有一条「看了录像才发现 ___」的用户原话；所有卖录像的产品文案自己都在说「你不想看四小时录像」；PostHog 的 Unity SDK 在 WebGL 上直接禁用 replay；Playset 按分钟计费是反向激励。Slaytester 一行 HTML 就能录 canvas + 音频 + 麦克风且开源自托管、PGPT 免费公开 alpha、LoopKit beta 免费——**「不做录像」不是护城河，是不进泥潭**，理由要从「重、贵、隐私敏感」换成「跌价到零、产出物要二次加工、网页游戏里技术上尤其贵」。**AI 总结不值钱**：PlaytestCloud 全档免费送，Playloop 免费档自带 API key 即可全开。

**有一条把行为和自述对上的硬证据**（Zach Barth，Infinifactory Early Access）：「玩家 bounce 掉（开始但没做完）的那些谜题，恰好就是通关者自述『太难』的那些谜题」。英文原话里有独立印证：「the most egregious example was one player saying that it took them 15 min to finish the playtest when they were clearly playing for 45 minutes」。「开始了但没做完」是一个 5 个人身上就能数出来、且能代理一份问卷的指标。

**商业线对什么数据能收钱的判断一致**：有付费证据的是版本比较、加载与错误、结果留存与导出、私密邀请、只读 reviewer；PV、漏斗、热图已有大量免费替代。**隐私是卖点不只是合规**：itch 用户的原始诉求是「接 GA4 就要自己扛 GDPR/PIPL，小团队扛不起」。

**一个 DESIGN 完全没覆盖的场景**（itch 论坛做浏览器多人游戏的开发者原话）：「Build distribution is already solved for me. It is a URL. … What is not solved is **concurrency**. The bottleneck in multiplayer playtesting is never getting the build out — it is getting four people into the same lobby at the same time. A tester who turns up alone doesn't give you a weak signal, they give you a misleading one.」§4.3 花了大篇幅论证 WSS vs QUIC，没回答这个。

## 2. 「找差异」还是「做得更好」——调研的答案

创始人的自我提醒被三条线独立印证，而且它们指出了「同一件事」具体是什么。

**所有人都在做「上传 → 链接 → 看结果」三步，区别在第三步的重量**（数据线）：itch 极轻（views / browser plays，用户评价 trivial）；TGN / SIMMER 没有；Playset 极重（录像按分钟）；Playloop 重（SDK + 三层 AI）；Antidote / PlaytestCloud 极重（$115–2,825/月）。**中间是空的：一段话 + 一张点名册 + 一个反馈按钮 + 错误上报。**

**「同一件事做得更好」线的评分卡**（18 个时刻 × 9 个竞品）里没有任何人做好的格子有五个：大陆与全球边缘兼顾、微信内可用性、移动端视口 + 音频手势、COOP/COEP 一键正确、免录像的轻反馈闭环。它给出的「只允许在三件事上比所有人好」：**速度与零摩擦**、**对游戏有感的边缘**、**一目了然的结果闭环**。

**「找差异」在 DESIGN 里的具体表现**（同一线点名，其余线部分同意）：
- §0.5 / §4.4 / §9.2 把香港拔高为核心战略并纠结「文案先说大陆还是全球」——用户只在乎「朋友在微信里点开卡不卡」，香港是工程手段不是卖点；文案永远只说「一条命令，几秒就能玩」。
- §2.3 / §2.2 把上传写成「不是差异化，是入场券」「不会比 TGN 好多少」——**低估了把上传做到极致的杀伤力**。TGN 实际是约旦两人团队的阿拉伯语游戏社区，托管是获客功能、靠卖课变现，五步流程、整包重传、明说不做 COOP/COEP，HN / PH / Reddit 上零发布帖。
- §4.5 / §7 为假想的自托管用户约束控制面（SQL 子集、单机 compose）——目标用户 99% 希望官方托管好一切；CLI 开源是信任之本，服务端自托管是极客背书。
- §8「留门」对录像的过度防备——为了和 Playset 划清界限，把「反馈附截图」也推到 v0.2；而截图是这一类的标配不是升级项（Playset 玩家可直接截图、Bugnet 组件带截图），数据线还发现上传模式下可以在注入 SDK 时 monkey-patch `getContext` 强制 `preserveDrawingBuffer`，截图能「默认成功」。

**真正成立的差异有三处。**
1. **微信是全球竞品的绝对真空**（AI 编程线、全球隧道线、上传线、真机线四条独立得出）——Lovable、bolt、v0、Replit、Claude Artifacts、tunr、ngrok、Cloudflare 全部架在境外边缘，没有一家把微信内置浏览器当验收目标；只是这块真空同时是最大的规则风险（§3 第 1 条）。
2. **「结果」这一类里没有任何一家以命令行为入口**（Playset 线把这一类从 DESIGN 的 4 家扩到 11 家）——有 SDK 和 API 的几家（Playloop、Bugnet、FirstLook）入口仍是网页 dashboard；隧道则是全类目零。「CLI 为中心」是这个类目唯一空着的格子，DESIGN 应把它写得更重。
3. **「永不改写你的字节」是一个可验证的承诺**（真机线）——kshare 对 HTML/CSS/JS 做原始文本 URL 改写（对 1.2 MB 的 Phaser bundle 和 `.wasm.br` 是灾难），LocalhostVibe 靠代理注入面板，tunr 的 `--inject-widget` 可选；没有人把「响应体一个字节不动」当承诺讲，而它能用哈希对拍验证。

## 3. 必须改的：写成了事实的假设

按严重程度排序。

**1. §0.3 / §3.3 / §5「微信里点开就能玩」——撞上了规范原文。** 微信《外部链接内容管理规范》（2025-10-23 生效，本轮已核对原文）§2.5「H5 游戏、测试类内容：以游戏、测试等方式，吸引用户参与互动的，具体形式包括但不限于比手速、好友问答、性格测试，测试签、网页小游戏」——**没有「并诱导分享」的限定**；§2.18.4 把「未按照法律法规规定履行备案手续或难以追溯网站运营者真实身份的网站链接」单列为风险；§3.2.2 把多域名规避列为对抗行为（域名轮换、短链套娃不能当抗封架构）；§3.2.3 把「微信内展示效果和其他浏览器实质性不一致」列为对抗（门禁页在微信里只显示「去浏览器打开」有风险）。实际执法针对的多半是病毒营销 H5，但规则文字给了腾讯全部裁量权。**建议**：把「微信里点开就能玩」从 §0 承诺降为待验证假设；v0.1 完成标准前加发布阻断条件——真实微信账号、Android ≥ 6 台 / iOS ≥ 4 个系统版本、私聊 / 群聊 / 朋友圈各测门禁页与实机游戏，并向腾讯客服书面确认 §2.5 对非营销、私人 playtest 链接的适用口径。另外两个被低估的未备案代价：朋友圈分享未备案一级域有频次限制、达到次数后仅自己可见；微信 JS-SDK 安全域名必须备案且不接受短链——**分享卡片只能服务端直出 OG 元数据「尽力而为」**。整域处理的官方条件是「同域名下大量链接违规、处理后仍未有效整改」，不是「一次举报自动连坐根域」——DESIGN §5 这一句过于悲观，但 PSL 不会让微信按 slug 隔离执法，治理不能放松。

**2. §4.4「大陆到香港 30–60 ms、接近国内 CDN」——只对优质回程成立。** 2026 年 ITDOG 公开样本：CUG/CMI 优化线全国三网平均 34–56 ms；同样标称香港的普通国际回程（TATA/Arelion）全国平均 194–319 ms，路由绕美欧；V2EX 用户对阿里云香港轻量的实测「就是个展示网页 ping 300~400ms」。**没有找到**任何主流香港云 × 三运营商 × 20:00–23:00 × 连续一周 × 含丢包的 2025–2026 公开数据。**建议**：30–60 ms 写成「优化回程的验收目标」；压测矩阵至少覆盖三网 × 两城市 × 家宽 / 4G × 白天 / 晚高峰 × 微信 WebView / 系统浏览器，记录 p50/p95/p99、丢包、30/100 MB 持续下载、traceroute。

**3. §5「GitHub 在大陆基本可用且开发者都有」——过于乐观。** GreatFire 2026-09-06：`github.com` 最近 55 次有效测试 67% 受干扰；OONI 2026-07-01 至 09-07 中国探针 76 条中 41 anomaly、7 failure、28 clear；TRAE 社区有「绑定 GitHub 一直显示正在认证」的求助。「都有」没有证据。**建议**：匿名 24 小时链接必须在任何登录之前完整覆盖第一次体验；GitHub OAuth 做成可失败、可重试的增强路径。

**4. §2.1 对大陆可达性的描述要分开写、带日期。** itch.io 不是「慢且不稳」，是 GFW 整域封锁：GreatFire 自 2025-09-25 起每次测试 100% 屏蔽，26/26 样本，最近 2026-08-30（三条线独立引用；第三方测量，需真机复核）。`vercel.app` 131 个样本 127 个被屏蔽；**`netlify.app` 79 个样本 72 个可访问**；Cloudflare Pages 波动；GitHub Pages 间歇。

**5. §2.1 / §2.3 关于 Playset 的三处写法都缺出处。** 两条线独立在 HN（Algolia 全库）、Product Hunt、Reddit、itch 论坛复核，Playset 零条独立用户评价、零讨论、零用量数据——「也证明这条路上已经有人在收钱」建议改为「已经有人在这条路上标了价」；「纽约两人团队，2025」**没有找到任何来源**；「$0 / 19 / 49」里只有免费档「15 分钟/月」是一手确认的。另一个意外：`playtesting tool` 在 Hacker News 全部历史里只有 38 条命中——**这个品类在开发者社交平台上是隐形的，Show HN 大概率不是冷启动渠道**，用户在 Discord、itch 和各引擎论坛里。

**6. §4.1「把 playtest.run 提交进 Public Suffix List」——大概率会被拒。** pico.sh 运营者在 HN 原话：「The public suffix has some fuzzy limits on usage size before they will add domains (e.g. on the scale of thousands of active users) … we were also rejected.」两域名分离那一半我们自己能做到，「子域之间彻底隔离」这一半在有几千活跃用户之前不成立。**建议**：改成「申请，但不依赖」。

**7. §2.1「国内穿透免费档 1 Mbps、多数要实名」——是分布不是常数。** cpolar 1 Mbps、飞鸽 0.5 Mbps、SakuraFrp 10 Mibps + 5 GiB/月。另：微信小游戏个人主体**可以**上部分类目；国内 Jam 不用 itch——BOOOM 用机核自有库、CiGA 2026 用 GmHub。

**8. §4.3 把大文件的瓶颈归给「家宽上行（普遍 30–50 Mbps）」——实测不支持。** 真机线实测 pinggy 免费档吞吐约 740 KB/s（5.9 Mbps），瓶颈在服务商限速而不在家宽；30 MB 的 Godot 导出物一个玩家要等约 41 秒。这是双面的：我们不限速就明显更快；但 §6 的带宽成本要按真实吞吐重算。

**9. §6 免费额度只有月度维度，挡不住 SIMMER 的死法。** SIMMER 是被分钟级 DDoS 烧死的，有 billing alert 也没来得及。**建议**：加「每 slug 每小时流量上限 + 边缘本地判定的自动熔断」，不依赖控制面回源。额度上：商业线核算 1,000 个免费账号满额 20 GB 在 $0.10/GB 下是 $2,000/月；建议公开免费档 10 GB/月硬上限、匿名链接 1 GB/24 小时，邀请制私测可给 20 GB 以观测真实利用率。「30 MB × 1,000 ≈ $3」算术正确，是合理中位基线。

## 4. 建议改的：定位、表述与优先级

**§1.2 五个时刻漏了一格：「让对方敢点开」。** 英文原话线的证据强度超出预期，而且是唯一一个在 2026 年还在恶化的：r/gamedev 的钓鱼 PSA 教人「让对方给你一个浏览器里能玩的 web build」；一个做了 30 年游戏的业余开发者被 Windows SmartScreen 拦到无法把 exe 给朋友；「asking someone to download and run an exe from a stranger on social media is a big ask」；学校和公司网络封 itch.io。**门禁页最值钱的一条价值 DESIGN 没写——它是这条链接的信任凭证**：一个写着真人名字和作品名的页面，正是 exe 和裸隧道链接给不了的东西。

**§2 竞品格局的修正**
- 删掉「上传这一路我们不会比它更好多少」；TGN 从「最近的三家」降为「同形态小团队案例」。
- 补 **Surge**（形状最像 §3.2 + §3.5，必须能正面回答「我们和 surge 差在哪」——游戏响应头、门禁页、结果、香港、同一条命令走隧道）。
- 补 **AI 单文件 HTML 托管群**：Clawcade（2026-06，「publish from Claude through MCP / get an instant playable link」，英文原话线认为是离我们最近的一个）、AIGameShare、rendrd（两家已有 MCP server）、ArcadeLab、localgames.fun、PlayFeed——打的正是 §1.1 排第一的人群。
- 补 **腾讯 EdgeOne Pages / Makers**：官方场景就是「让 AI 做一个贪吃蛇，生成链接发给朋友玩」；但大陆节点要备案、平台域名在大陆只给 3 小时预览链接——这正是 playtest.run 的空间，也是必须持续跟踪的相邻对手。
- 补两类真实替代品：**录视频**（行为替代）和 **AI 编辑器内置分享**（TRAE 已有产物链接；GitHub Spark 2026-08 关停的教训是强制登录杀死传播）。
- 「Playtest 结果」那一格：「都没有隧道」成立且更强（扩到十几家仍为零）；「没有 CLI 工作流」改为「没有一家以命令行为入口」；不再罗列名字，改按三种商业模式写——按录像分钟卖（Playset、Playcocola）、按 AI credit / 席位 / 容量卖（Playloop、Bugnet、LoopKit、PGPT）、按「帮你找到人」卖（The Playtest、FirstLook、Antidote、PlaytestCloud）。
- 「结果层比 Playset 轻」这个位置已被 Playloop / Bugnet / LoopKit 免费档占了；角度改成「**在同一条命令里 + 不改一行代码就有第一层**」——他们全都不做交付，他们的第一层全以接 SDK 为前提。
- 隧道竞品分两层写：认真的对手只有 tunr（单静态 Go 二进制 + SDK + MCP + `--freeze --inject-widget --demo` 已打包成「Full vibecoder demo package」在首页卖）和 uplink（五天前还在发版，npm 描述直接写「JSON-first CLI for Cursor, Claude, Codex, and Windsurf」）；kshare 是一个人的 $4/月 VPS，LocalhostVibe 自称「Software-as-a-Meme」。「tunr 的反馈小组件说明方向被验证」→「被另一个 builder 看见」（18 stars）。「新品绝大多数骑 Cloudflare」→ 只确认 tunr 与 LocalhostVibe。ngrok 免费账号现在是固定 dev domain，不是每次随机。

**§3.2 CLI 与入口**
- 机器模式进 v0.1：稳定 JSON 输出、日志走 stderr、分层退出码、`--ttl`。三家竞品（tinyfi.sh 做成 agent skill、uplink JSON-first、tunr MCP、Clawcade 从 Claude 里发布）都在把 **AI coding agent 当分发渠道而不是把开发者当分发渠道**；§1.1 第一类用户正是这群人，§3.2 的形态清单里却没有 MCP / agent skill 的位置——现在是没提，不是决定了不做。
- 检测到 Unity 导出物时若 `.br` 与 Decompression Fallback 冲突、检测到 Godot 线程版 wasm 时，在终端明说并给选项。
- 隧道模式检测到大静态目录时，用**实测吞吐**算给开发者看：「你这个目录 32 MB，隧道模式下一个玩家大约要等 45 秒，十个人同时进来会更久。建议改用 `playtest ./dist`」。
- 保活探测走 WSS 控制通道，不要每 30 秒往开发者的 dev server 打 `HEAD /`（真机线观察到 uplink 这样做，会污染开发者日志）。

**§3.3 门禁页——三条硬线 + 文案**
- **只在导航请求上渲染**（`Sec-Fetch-Dest: document`），**子资源永不返回「200 + HTML」**，**不存在绕过 header**——判据用请求语义，不用 User-Agent。pinggy 是完整的反面教材：靠 UA 猜访客是不是浏览器，每个没带 cookie 的路径都回 200 + 15 KB HTML；Phaser fixture 的形状（入口 843 字节、真东西是 1.2 MB 的 JS）说明在游戏里 200 比 404 坏得多——浏览器不报网络错，最好是一个看不懂的语法错，最坏是白屏加沉默。
- 文案按识别结果切换：探测到 Godot / Unity / Phaser / Cocos 用「邀请你试玩」，其余默认「邀请你体验 / 预览」。
- 「24 小时内不再出现」当实验参数（ngrok / zrok 是 7 天，itch 移动端是每次 launch），量 `gate_view → start → first_asset → game_ready` 各段。
- 极致轻量（< 3 KB、无外部脚本），像作品封面不像安全告警；提供开发者可关闭门禁页的选项；微信里不能只显示「去浏览器打开」（规范 §3.2.3）。

**§3.4 结果层**
- 首句「谁打开了」改为「多少人打开、进到游戏、玩到哪、在哪断」。
- 控制台句式从「每个版本一行：打开数、独立人数、错误数、反馈数」改成**一版一段自然语言 + 一张会话点名册**，可按「停留最短」排序；不画图表。
- 第一层新增 L7（门禁到首帧掉队人数）；设备 / 浏览器和资源加载失败提为主打；「来自哪里」降为尽力而为；停留只给中位数和明细；回头改点名不改比例。
- SDK 层新增 S5「最后一个事件 / 最后一次输入的时间戳」；加载用时改分阶段；自定义事件文档只教「标记进度里程碑」。
- 明确写进 §3.7 不做：热图、漏斗、分群、A/B、DAU/MAU/留存曲线、AI 总结、全程录像；把「截图」和「录像」分开写。

**§3.6 访问控制**：itch 的四档已包装成「Limited Playtests」；更重要的是**口令 + 跨源隔离会打架**（itch 上开了 SAB 的项目加口令在 Safari / Firefox 弹窗里 404，bug 活了两年）——`--isolated` 与门禁 cookie / 一次性令牌的语义要在设计阶段对齐。

**§4.2 / §4.3 响应头对两条路都生效**：§4.2 把 `.wasm` MIME、`.br` 的 `Content-Encoding`、COOP/COEP 写得很细，§4.3 一个字没提——但 Godot 4 开发者敲 `playtest 5173` 一样打不开，dev server 不发这些头，隧道也不会补（真机线：**没有一个竞品隧道会补响应头**）。这一格一个竞品都没做。

**§8 留门**：把「画面录像」替换为「联机 playtest 的并发问题（约定时间窗、N 人同时在线、多视角时间对齐）——隧道路径上唯一被用户明确说出『我今天就买』的需求，但它是一个完整产品；T4 隧道占比 > 30% 且出现两个以上联机作品时再评估」。加一条「作品简历页（只读分享 token + 版本 + 打开数 + 中位时长 + 设备 + 反馈摘要），v0.2 评估」。

**§8 私测指标**：T6 拆成 **T6a 反馈数 / 打开人数 ≥ 8%** 和 **T7 发新版前看过上版结果页的开发者 ≥ 60%**（60% 是拍的，无公开基准）；加 **T8 门禁到首帧掉队率**（只记录）；T4 提前到第一周看（英文世界「隧道 + 游戏」只找到 1 条真实抱怨，证据比 DESIGN 假设的薄）；加一个定性问题「你把链接发给了谁、从哪找到的」。「T6 长期为零就砍 SDK」改为「T6a 长期 < 3% 砍反馈按钮，错误上报无论如何保留」。

**§6 / §7 商业与开源**：第一付费档卖一个身份变化——「从随手给朋友看，变成给发行商 / 客户 / 封闭玩家群做正式测试」：固定可改 slug + 去角标、口令 + 邮箱名单 + 一次性邀请、180 天结果留存 + 版本比较 + 导出 + 3 个只读 reviewer；商业线建议 $15/月 或 $150/年 含 50 GB，流量只卖预付包不默认后付。自定义域名后置。AGPL 边界写清：Apache 引擎插件不链接 AGPL 服务端库、普通自托管无需商业许可。HN 上对付费的态度集中在「你会不会跑路」而不是「值不值」（「I'd rather pay a few dollars for a service that will be around 5 years from now, than pay nothing and have to deal with churn」）——这支持 v0.1 就把「怎么活」写清楚，哪怕暂不收钱。

## 5. 线与线之间的分歧，以及我的判断

1. **上传还是隧道是第一路径？** AI 编程线：对 AI 编程用户隧道第一。国内穿透线、中文原话线、英文原话线：公开原话更支持「产物拿链接」，英文世界 53 条相关原话几乎全在「静态构建 + 链接」的世界，BOOOM 要求 Win/Mac 包、WebGL 可选。全球隧道线：大资源经家宽上行的矛盾真实。**判断**：按人群分，不预判占比；两条路都一等公民，T4 第一周就看；隧道模式对大目录用实测吞吐算给开发者看再建议上传。
2. **门禁页是妙招还是流失点？** 「更好」线：多一步就是流失，需极轻、可关。全球隧道线 + 英文原话线：正证据充分（itch 移动端本来就 Click to Play、Chrome 音频策略、Discord 邀请页、packetriot 运营者「portal page … very effective at preventing abuse」、链接可信度），前提是像封面不像警告。真机线：三条硬线。微信线：不能只显示「去浏览器打开」。**判断**：保留并把「信任凭证」写进价值主张，但按可测变量处理。
3. **截图进不进 v0.1？** 「更好」线和 Playset 线：截图是标配，提前。数据线：上传模式技术上可默认成功。**判断**：v0.1 文字优先没错，但把「尽力而为」改成更准的承诺；是否提前由实现工期定。
4. **免费档 20 GB 慷慨还是危险？** 不矛盾——私测 20 GB 观测利用率，公开 10 GB，真正的防线是每小时熔断。
5. **香港到底几毫秒？** 三条线一致：取决于回程线路不是地理。
6. **MCP 是不是 v0.1？** AI 编程线、英文原话线、真机线三条都说这是入口位置的事、不该等社区；全球隧道线说不必为「有 MCP」先加依赖。**判断**：`--json` 机器模式进 v0.1 没有争议；`playtest mcp` 子命令的成本主要是一个 stdio 协议壳，建议进 v0.1 的「完成」标准之外、v0.2 第一项。
7. **远程 playtest 本身值不值？** 英文社区最权威的声音反复说「Playtests are best conducted privately and in person」，被第三人引用「I upvote this every time I see it because I know it is correct」；紧接着的反驳同样有力「it's generally untenable for a majority of smaller indie devs」。**判断**：定位不是替代线下观察，是「你线下看不了的那些人，至少别是黑箱」。

## 6. 13 条线都没查到 / 没验证的

- Playset、Playloop、tunr、uplink 等任何一家的独立用户评价、留存、付费数据。
- 游戏 playtest 场景下反馈按钮的真实填写率、开发者回看结果页的频率——全网无公开基准。
- 微信 Android XWeb 对 COOP/COEP、SharedArrayBuffer、WebGPU、Service Worker、30–100 MB 缓存的真机矩阵；「在浏览器中打开」遮罩的转化率；申诉成功率。
- 主流香港云 × 三网 × 晚高峰 × 含丢包的公开数据。
- GitHub OAuth 在大陆从授权到回调的成功率。
- Cocos Web 导出使用率、国内 Jam 的 Web 提交占比——**「国内做开放 Web 游戏的人很多」不能作为立项前提**。
- user activation 是否跨文档保留——直接决定 §3.3「门禁页顺手解决音频」能否成立，需要 `docs/spikes/` 记录。
- 真浏览器访客侧行为（真机线全程只用 curl，「子资源会不会拿到中间页」是推断）、各竞品精确秒数、30–50 MB 真实引擎导出物（fixtures 里最大 1.15 MiB）。
- 所有 Reddit 引文的点赞数（`.json` 端点返回反爬页）；Reddit 原话引用前建议手工核对原帖。

## 7. 综合后的产品判断

把 13 条线的结论折叠成一页，作为修订 DESIGN 的提纲。

**产品的一句话不变**：一条命令，把你手上这个能玩的版本放到具体的人面前，然后知道他们玩成了什么样。

**入口与留存分开写。** 入口是「链接能开、而且对方敢开」——托管坑是十年不愈的高频病，链接可信度是 2026 年还在恶化的新病；留住人的是「送出去之后不是黑箱」——近 15 个月爆发的最深的痛。§0 现在把「知道结果」写成分界与护城河是对的，但要补一句：**开发者因为托管来，因为结果留下。**

**只在三件事上比所有人好，每件都可验收。**
1. **第一次运行到手机扫码，是一个打出来的数字。** 单静态二进制、不走 npm、不依赖 ssh；CLI 结束时显示本次耗时；验收 P50 < 5 s。
2. **对游戏有感，两条路都算。** `.wasm` MIME、预压缩、COOP/COEP、Range 对上传和隧道同样生效；上传那一刻做检查并用人话说出为什么跑不起来；门禁页接手音频手势；响应体一个字节不动。
3. **结果读起来像点名册。** 不改一行代码就有：多少人打开、多少人进到游戏（L7）、每个人玩了多久在哪断、什么设备、加载哪里失败；加一行脚本再有错误堆栈、里程碑、最后一次输入、一句话反馈 + 截图。不做流量、漏斗、录像、AI 总结。

**四条硬承诺写进 README 和门禁页。** 玩家不登录不安装；永不改写你的字节；门禁只拦顶层导航、子资源永不返回 200 + HTML、没有绕过 header；不运行你的代码。

**三个假设降级为待验证，且各配一条 spike 或发布阻断条件。** 微信内可玩（规范 §2.5 + 真机多机型 + 腾讯口径）；香港 30–60 ms（三网晚高峰一周压测）；GitHub 登录可用（匿名链接先成功，OAuth 可失败可重试）。

**入口形态加一项。** CLI 仍是中心，但 `--json` / 退出码 / `--ttl` 进 v0.1，`playtest mcp` 进 v0.2 第一项——因为 AI coding agent 正在成为这群用户的分发渠道，而不是我们希望社区做的「桌面托盘应用」。

**不做的边界更清楚，理由更硬。** 不做曝光 / 榜单 / 大厅（Glitch、SIMMER、Replit、itch 的 2025）；不做录像（跌价到零）；不做 AI 总结（免费档就有）；不做孵化（无证据）。留门两条：作品简历页（v0.2 评估）、联机并发（T4 > 30% 再评估）。

**冷启动渠道改写。** 不是 Show HN（38 条历史命中），是 Discord、itch 社区、Godot / Unity / Cocos 论坛、国内 Jam 社群（BOOOM、CiGA / GmHub）、以及 Cursor / Claude Code 用户的 agent。

**私测多问一句话。** 除 T1–T8，问每个开发者：「你把链接发给了谁？从哪找到这些人？」——答案决定 §1.2 是否成立。

## 8. 索引

| 报告 | 模型 | 一句话 |
|---|---|---|
| [same-thing-better](2026-09-07-same-thing-better.md) | Gemini 3.8 flash | 18 时刻 × 9 竞品评分卡；五个全行业空白格；点名 DESIGN 里「找差异」的四处 |
| [upload-hosting-ux](2026-09-07-upload-hosting-ux.md) | Opus 5 | 17 项 UX 打分表；TGN 真面目；SIMMER / Glitch 死因；itch SAB 开关四年长尾原话 |
| [playtest-feedback-analytics](2026-09-07-playtest-feedback-analytics.md) | Opus 5 | 「流量」不成立、点名成立；44% 加载流失；录像是卖点；T6 拆分建议；控制台点名册句式 |
| [playset-deep-dive](2026-09-07-playset-deep-dive.md) | Opus 5 | Playset 零用户声音；「结果」类扩到 11 家、三种商业模式；没有一家以 CLI 为入口；录像正在免费化 |
| [developer-voice-en](2026-09-07-developer-voice-en.md) | Opus 5 | 110 条英文原话按 9 主题归类；「让对方敢点开」是漏掉的时刻；「谁」是我们想给的；PSL 会被拒；Clawcade |
| [hands-on-tunnels](2026-09-07-hands-on-tunnels.md) | Opus 5 | 本轮唯一一手实测：npx 一族首跑成本；pinggy 每个路径回 200+HTML；门禁页三条硬线；没有隧道会补响应头 |
| [tunnel-tools-global](2026-09-07-tunnel-tools-global.md) | GPT 5.6 sol | 14 个隧道工具访客侧中间页对比；ngrok 定价史核实；门禁页正反证据；可度量验收矩阵 |
| [tunnel-tools-china](2026-09-07-tunnel-tools-china.md) | GPT 5.6 sol | 国内穿透免费档分布；小游戏体验版门槛；GreatFire 可达性快照；EdgeOne Makers |
| [developer-voice-cn](2026-09-07-developer-voice-cn.md) | GPT 5.6 sol | 80 条中文原话按 9 主题归类；「录视频」是最危险替代品；「没人玩」比「发不出去」靠前 |
| [business-models](2026-09-07-business-models.md) | GPT 5.6 sol | 免费档规律与死亡案例；单位经济复核；转化杠杆证据表；三种死法 |
| [wechat-mobile-reality](2026-09-07-wechat-mobile-reality.md) | GPT 5.6 sol | 微信规范 §2.5 原文；能力矩阵；封禁案例表；香港延迟证据表；GitHub OONI 数据 |
| [ecosystem-exposure-incubation](2026-09-07-ecosystem-exposure-incubation.md) | Gemini 3.8 flash | Poki / CrazyGames 把 playtest 数据当发行前置；公开大厅的反例；「作品简历页」窄路 |
| [ai-coder-sharing](2026-09-07-ai-coder-sharing.md) | Gemini 3.8 flash | 50 个作品抽样 14% 是游戏；内建发布能力矩阵；MCP 作为入口 |
