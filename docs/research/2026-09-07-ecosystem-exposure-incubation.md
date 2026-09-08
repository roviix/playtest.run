# 调研报告：曝光、反馈与孵化生态——检验与推演 playtest.run 的生态位

> 日期：2026-09-07  
> 调研对象：Web 游戏发行商计划、独立游戏孵化/基金投递标准、工具平台长出曝光生态的成败得失、Game Jam 与教学评审痛点。  
> 核心任务：检验创始人判断（「是否要建立用户反馈、曝光、甚至后续孵化机制或生态」），推演 DESIGN.md §3.7「不做发现、榜单、商店、开发者变现」这一断言的合理性、边界与破局点。

---

## 一、调研背景与核心问题

在 `docs/DESIGN.md` 中，playtest.run 被定义为：
> **一条命令，把手上这个能玩的版本放到别人面前，然后知道他们玩成了什么样。**
> 核心观众是 5–50 个具体的人；明确不做发现、榜单、商店、变现，不做玩家社区与账号体系（§3.7）。

创始人当前的三个判断中，重点待检验的是：
**「是否要建立用户反馈、曝光、甚至后续孵化之类的机制或生态？」**

如果完全「不做发现与生态」，产品是否会沦为纯粹的管道工具（有被 Cloudflare / 开源隧道平替的风险）？但如果做「发现与曝光」，是否会瞬间卷入内容审核、排榜不公、作弊治理的泥潭？

本报告通过深挖 **Web 游戏平台**、**全球及本土游戏孵化/基金评审流程**、**典型开发者平台生态演进与夭折案例**，结合 **Jam 组织者**与**高校教学**两条真实 B 端痛点，尝试给出一个既不违背「工具极简」又能形成生态抓手的解法。

---

## 二、Web 游戏发行商的开发者计划与投递标准

Web 游戏平台（以 Poki、CrazyGames、Y8、GameDistribution 等为代表）是目前全球 WebGL / HTML5 游戏商业化最成熟的阵地。通过对官方文档、开发者协议及 2025–2026 年最新政策的调查，梳理出其核心机制：

### 1. Poki for Developers (Poki Playground)
- **流量与规模**：官方 2025 年终回顾显示，平台月活玩家达 1 亿（100M MAU），全年游戏游玩次数超 111 亿次，超过 1,018 款游戏达到 100 万+ 游玩量；全职团队规模在 2025 年从 50 人扩充到 65 人（[来源: Poki 2025 Year in Review, Medium](https://medium.com/poki/2025-at-poki-a-year-in-review-6e2315f41cb5)）。
- **收入分成模式（一手文档）**：
  - **自有流量 100% 留存**：如果玩家通过开发者自己的书签、搜索、社交媒体或社区链接进入，开发者保留 **100%** 的广告收益。
  - **平台流量 50/50 分成**：若玩家来自 Poki.com 平台推荐或 Poki 的买量营销，收益双方按 **50/50** 分成（[来源: Poki Documentation](https://sdk.poki.com/)）。
  - 部分头部游戏享有定制化独占授权费（Licensing Deals），由商务个别谈判（[来源: IndieGameBusiness 2025](https://indiegamebusiness.com/web-gaming-for-indie-developers/)）。
- **Poki Playtesting（核心机制）**：
  - Poki 在 GDC 2024 正式推出其内建的 **Poki Playtesting** 工具，并扩展至 **Player Fit Test**（[来源: Medium - Poki at GDC 2024](https://medium.com/poki/poki-at-gdc-2024-introducing-poki-playtesting-1c55436ea224)）。
  - 运作机制：开发者将 Web 构建提交至后台，Poki 会在主站向部分真实玩家推送一个「神秘地块」（Mystery Tile）。玩家点开即可试玩。
  - 产出反馈：系统自动录制玩家完整的游玩视频（10 到 50 个 session）、记录按键输入流、控制台 console 输出、游玩时长（是否超过数分钟）、国家及设备类型。更高级别的测试可扩展至单日 500 个真实玩家样本（[来源: sdk.poki.com/playtesting](https://sdk.poki.com/playtesting)）。
- **开发者准入门槛**：强人工筛选（Hand-curated）。提交时需要提供完整可玩 Web 链接，只有通过人工初筛的游戏才会被纳入平台池。

### 2. CrazyGames Developer Portal
- **流量与规模**：月活跃玩家超 5,000 万，主力集中在欧美等 Tier 1 高 CPM 地区（[来源: CrazyGames Documentation FAQ 2026](https://docs.crazygames.com/faq/)）。
- **分成与提现（2025–2026 现状）**：
  - 广告分成：主文档未硬编码全量比例，但在其 2026 年 GameMaker Web Jam 条款中公开标注开发者享有 **60% 广告收益分成**，头部邀请制游戏内购（通过 Xsolla）享 **70% 内购分成**（[来源: Cinevva 2026 CrazyGames Developer Guide](https://app.cinevva.com/guides/publish-game-crazygames)）。
  - 最低提现门槛 €100，支持 PayPal 和国际电汇，按月结算。
- **两阶段发布机制（Basic Launch vs Full Launch）**：
  - **Basic Launch（软启动/公测验证期）**：提交后经 QA 人工快速检查（1-2 个工作日，重点查 PEGI-12、无恶性 Bug、英文、首屏体积 ≤50MB）。通过后进入为期 **2 周的公测**。此阶段**强制关闭所有广告变现**，纯粹向小规模真实玩家投放，跑基准数据（留存率、平均游玩时长、启动到进入核心玩法转化率）（[来源: docs.crazygames.com/requirements/intro/](https://docs.crazygames.com/requirements/intro/)）。
  - **Full Launch（正式全球推荐与变现）**：只有在 Basic Launch 阶段核心留存与时长达到同品类基准线的作品，才会被邀请接入完整 SDK（插屏广告、激励视频、Xsolla 内购、云存档、好友联机），获得全站流量倾斜。
- **开发者工具**：内建 Quality Assurance Tool，开发者可在上传后实时预览并检测 SDK 事件与加载耗时。

### 3. GameDistribution 与 Y8
- **GameDistribution**：按其 2025 年 6 月 19 日更新的最新开发者协议（Developer Game License Agreement），净收益分成固定为 **33%**，主要做全球数千个小型网页站的联盟分发（Syndication），最低提现 €100（[来源: static.gamedistribution.com/terms/developer.html](https://static.gamedistribution.com/terms/developer.html)）。
- **Y8.com**：月活约 3,000 万，常规托管合作广告分成 **50%**；符合资质的团队可接入 Google AdSense for Platforms (AFP) 直接收汇，门槛为可玩的 HTML5 稳定版本（[来源: developer.y8.com](https://developer.y8.com/)）。

### 4. 平台兴衰与启示：Kongregate、Newgrounds 与 itch.io
- **Kongregate 的衰落与 2026 尝试**：
  - 曾经的 Web 游戏霸主因 Flash 死亡、母公司收购、社区转型迟缓，于 2020 年关闭新游戏提交，一度沦为僵尸站。
  - **2026 年 4 月最新动作**：推出了「2026 Welcome Back Revenue Share Program」，以 70% 的分成率试图重新吸引开发者回归，但流量与社区热度已严重断层（[来源: blog.kongregate.com](https://blog.kongregate.com/hc/en-us/articles/44770578988429-2026-Welcome-Back-Revenue-Share-Program)）。
- **Newgrounds**：
  - 依然保持自 1995 年以来的社区驱动。放弃了早年的广告分成计划，转为依靠 Patreon 风格的 Supporter 订阅和赞助金。其核心价值在于「Underdog 独立艺术文化」，拥有强大的 UGC 评审通道（The Portal Judgment Phase），但缺乏现代商业化体系。
- **结论与共性规律**：
  - **数据成为了 Web 发行商的「准生证」**：无论 Poki 的「Playtest 地块」还是 CrazyGames 的「2周 Basic Launch」，大平台都不相信纯文字 Pitch，只相信「先给 50–500 个真实玩家跑一跑，拿数据说话（留存、时长、加载失败率）」。
  - 发行商都在将「Playtesting」前置为自身的核心竞争壁垒。

---

## 三、独立游戏孵化、基金与发行商的评审标准

通过对 2025–2026 年全球 400+ 游戏基金目录（Game Funding Directory）及官方 Pitch 渠道的调研，提炼出资方与发行商到底想看什么：

### 1. 欧美基金与独立发行商
- **Indie Fund**：
  - 经典独立基金（曾资助《Dear Esther》《Q.U.B.E.》等）。
  - **硬性提交要求（一手来源）**：1-2 句核心特色介绍；1-4 张截图；**1-2 分钟纯实机视频（不要炫酷预告片，只看真实玩法）**；**必须提供可下载/可试玩的 Demo（Windows/Mac 或 TestFlight 链接）**；预算表。明确写道：*“We will not fund games without a playable prototype.”*（[来源: indie-fund.com/apply](https://indie-fund.com/apply)）。
- **Kepler Interactive / Kowloon Nights**：
  - 投资过《Sifu》《Sea of Stars》《Spiritfarer》，总流水超 1.5 亿美元。
  - **核心评审准则（一手来源）**：其官方 Inbound 表单开宗明义写着：*“Demo > Video > Concept Art > Words: we prioritize projects that have something playable or a proof of concept to show; and are generally unable to assess projects without a build.”*（Demo > 视频 > 概念图 > 文字；没有 build 基本无法评估）（[来源: kepler-interactive.com/form](https://kepler-interactive.com/form)）。
  - 业务负责人 Matt Handrahan 2025 年末公开发言明确警示：如今是绝对的买方市场，单纯发一个 PPT 和灰模原型（gray box prototype）拿到签约的几率几乎为零。开发者必须展现从第一天开始就有自我验证玩法与吸引受众的证据（[来源: PreMortem Games, 2025-09](https://premortem.games/2025/09/16/matt-handrahan-of-kepler-interactive-playing-it-safe-doesnt-have-any-value/)）。
- **WINGS Interactive**：
  - 针对女性及多元背景主创团队的独立游戏基金（单项目最高资助 $500K，50/50 抽成直至收回成本后转为 15%）。
  - **必须材料**：Pitch Deck、预算明细、**Playable Build / Demo**、团队角色分工（[来源: wingsfund.me/faq](https://www.wingsfund.me/faq/)）。
- **Outersloth（Innersloth《Among Us》设立的独立基金）**：
  - 资金池 2,500 万美元，单项目 $50K–$2M。截至 GDC 2026 已投资 24 个项目共计 1,916 万美元。录取率极低（约 1.4%），同样必须提交可玩版本（[来源: GameFundingDirectory 2026](https://github.com/GameDevGrzesiek/GameFunds)）。
- **行业震荡**：
  - Humble Games 内部重组并解散全部原团队（资产移交 Balor Games 管理）；Private Division 被 Take-Two 剥离出售；FireStoke、Versus Evil 等关停。2025–2026 年资方极度风险厌恶，**「纸面提案（Paper Pitches）已死」**。

### 2. 国内平台计划与赛事
- **TapTap 聚光灯计划（2025–2026 升级版）**：
  - 千万资金扶持 + 亿级流量，联动核聚变、ChinaJoy。
  - **提交材料清单（一手来源）**：
    1. 游戏页面（Steam 或 TapTap 链接）；
    2. 实机演示视频（拒绝纯剪辑，要求看实际操作与 UI）；
    3. **游戏试玩 Demo（试玩包或网页试玩地址，明确要求附带以往玩家的试玩/测试反馈）**；
    4. 团队研发故事与背景（[来源: TapTap 开发者服务动态](https://www.taptap.cn/moment/659083796930366040)）。
- **腾讯 GWB 独立游戏大奖赛 / 腾讯游戏创作大赛（2026）**：
  - 设置 400 万+ 奖金池，打通「赛事-孵化-上线」闭环，要求提供实机可玩演示及核心规则说明（[来源: 品玩 2026 腾讯游戏创作大赛](https://www.pingwest.com/a/313047)）。
- **微信小游戏创意鼓励计划**：
  - 针对玩法创新的小游戏提供「创意认证」，给予 70% 现金分成（普通小游戏为 60%）以及单月最高数十万至数百万的买量广告金配赠。但要求采用微信小游戏技术栈并首发上线（[来源: 游戏陀螺、钛媒体 2026](https://youxituoluo.com/532712.html)）。
- **CiGA indiePlay / 机核 BOOOM**：
  - 核心准入条件：必须有在评委电脑/手机上能跑起来的构建（试玩包或 Web 链接），每年上千份提交，线上评委打分。

### 3. 资方与发行商的核心痛点
- 评委与 Scout（选品人）每天要面对几十个 Pitch。**最大的阻力不是没钱，而是「打不开」和「不知道真不真实」**：
  - 很多开发者给的 Google Drive / 百度网盘 zip 包含 2GB 的 exe，解压后报缺少动态链接库，或者 macOS 下报未签名开发者拦截。
  - Web 版链接常常骑在海外慢速服务器上，国内评委打开白屏或卡在 99%。
  - 开发者宣称「我的玩法很多人喜欢」，但没有任何客观数据佐证（比如到底多少人玩过了新手教程、崩溃率如何）。

---

## 四、Game Jam 的提交与曝光机制：深陷泥潭的评分生态

Game Jam 是独立开发者的摇篮，也是 playtest.run 的核心目标人群聚集地。调研 Ludum Dare、GMTK Game Jam、js13kGames 以及 itch.io 的评审系统，暴露出当前生态极度深重的痛点：

### 1. itch.io 每年承载的 Jam 规模与中位数惩罚争议
- **规模**：截至 2025–2026 年，itch.io 累计承载了超 **388,600 场 Game Jam**，提交了超 55 万款作品（[来源: ACM FDG 2025 / itch.io](https://doi.org/10.1145/3723498.3723739)）。仅 GMTK Game Jam 单场就有近万人报名、数千个作品提交（2024 年 7,520 个提交，15.8 万条评分；2026 年超 10,500 个提交）。
- **中位数惩罚公式（Calculated Ratings）引发的众怒**：
  - itch.io 的官方排名公式为：
    $$\text{final\_score} = \text{average\_score} \times \sqrt{\min\left(1, \frac{\text{num\_ratings}}{\text{median\_num\_ratings}}\right)}$$
  - **问题**：如果某款游戏的评分总数低于整场 Jam 的中位数（比如中位数是 20 票，某游戏只有 19 票），它的得分就会被狠狠砍下一大截！
  - **社区真实抱怨**：在 GMTK 及各类 Jam 社区中，开发者反复控诉：
    > *“《31st March, Midnight》原始得分 4.429，本来稳进前 5 名，因为少了几票被中位数惩罚直降 0.7 分，掉到了第 30 名！”*（[来源: Interactive Fiction Community Forum 2025](https://intfiction.org/t/short-games-showcase-feedback-thread/73448)）
    > *“只要评分数低于中位数，分数就会贬值。这变成了拉票大战，谁社交网络朋友多谁就能保住分数！”*（[来源: itch.io Topic 8970561](https://itch.io/post/8970561)）

### 2. Rating Queue（随机评审队列）的失效与疲劳
- 为防止抱团刷分，itch.io 创始人 `leafo` 推出了 **Rating Queue（随机队列模式）**：要求参与者必须先随机评价并反馈系统分发的 5–25 款游戏，才能解锁自由评分权（[来源: itch.io 文档与功能讨论帖](https://itch.io/t/874902/rating-queue-feedback-issues)）。
- **产生的新痛苦**：
  1. **下载安装门槛**：队列里随机分到 Windows 独占或 Linux 独占的可执行文件，Mac 评委根本跑不起来；即便增加了「仅限浏览器」过滤，依然经常遇到卡死、404、报错的坏构建。
  2. **垃圾反馈与刷任务**：为了赶快解锁 25 个名额去给自己的朋友投票，大量玩家在随机分到的游戏下随便打 1 星或留无意义字符，严重破坏反馈质量。
  3. **评委精力枯竭**：GMTK 主办方 Mark Brown 每年赛后即便只玩排名前 100 的游戏做视频盘点，也需要耗费数周时间；普通参赛者更没有精力认真玩 20 款以上的长游戏。

### 3. js13kGames 的克制方案
- 限制包体在 13KB 以内，全部基于 Web 浏览器运行。
- 在提交时要求双通道：一个压缩包在线试玩，一个 GitHub 源代码仓库。
- **与 Poki 合作引入 Playtesting**：在正式评审前，由 Poki 提供自动化玩家测试渠道，帮助开发者提前暴露 Bug（[来源: js13kGames Medium 2024](https://medium.com/js13kgames/playtest-your-js13kgames-entries-with-poki-0e7a9d496917)）。

---

## 五、案例：工具起步长出社区/曝光的得与失

很多开发工具在拥有用户后，都曾尝试涉足「社区、展示、曝光或交易市场」，其轨迹极具警示意义：

| 产品 | 初始定位 | 做了什么生态/曝光扩张 | 结局与教训 |
| :--- | :--- | :--- | :--- |
| **Glitch** (Fog Creek / Fastly) | 极简 Node.js 网页原型与在线协作，`glitch.me` | 打造 Remix 文化、公开 Showcase 社区、个人主页、类似 GitHub 的探索流 | **2025年7月彻底关停应用托管与个人主页**。CEO Anil Dash 坦承：遗留架构面对现代云平台毫无优势，更关键的是**平台长期沦为恶意软件、钓鱼和自动化机器人的温床（Bad actors），小团队维护滥用与审核的成本高到不可承受**（[来源: The Register 2025-05](https://www.theregister.com/off-prem/2025/05/23/glitch-hits-kill-switch-on-app-web-hosting/1463632)）。 |
| **Replit** | 浏览器轻量级 IDE，零门槛多语言执行 | 推出 Teams for Education（免费教育版）、Bounties 悬赏接单市场、公开 Repls 社区发现页 | **全线砍掉边缘生态**：<br>1. **Teams for Education 于 2024年8月1日彻底下线并删除所有学生数据**，CEO Amjad 明确表示该业务巨额亏损、极其消耗算力，且遭遇大量欺诈滥用（[来源: The Register 2023-11](https://www.theregister.com/software/2023/11/21/compsci-teachers-panic-as-replit-kills-off-educational-ide/1303835)）；<br>2. **Bounties 悬赏计划于 2025年9月6日全面停止运营**（[来源: Hacker News 2025-08](https://news.ycombinator.com/item?id=44643875)）；公司全面转型高客单价 AI Agent 订阅（Replit Core / Agent 3），放弃低端社区和免费生态。 |
| **Lovable** | AI 全栈 Web 应用生成器（GPT Engineer） | 2025年1月推出 **Lovable Launched**（类似 Product Hunt 的每日打榜页），配合 X 官方转发扶持社区优质作品 | 作为高估值（$6.6B）AI 明星，Launched 主要作为营销杠杆（Growth Loop）带动 AI 算力订阅，展示页仅做展示与社交投票，不承担内容托管审核与支付闭环（[来源: Lovable Blog 2025-01](https://lovable.dev/blog/2025-01-30-how-to-launch-and-get-traffic-to-an-app-built-with-lovable)）。 |
| **itch.io** | 独立游戏无门槛售卖工具（Pay-what-you-want） | 长出万级 Game Jam 生态与全球最大的独立/小众游戏长尾发现页 | **深陷审核与支付断流危机**：2025 年 7 月，因支付巨头（Visa、Mastercard）在反成人内容倡议压力下的威胁，itch.io 被迫紧急从搜索和推荐中下架（deindex）数以千计的边缘游戏。创始人直言：非标内容的欺诈、退款、争议成本使得运营费用激增，独立平台极其脆弱（[来源: The Guardian / GamesIndustry.biz 2025-07](https://www.theguardian.com/world/2025/jul/29/mastercard-visa-backlash-adult-games-removed-online-stores-steam-itchio-ntwnfb)）。 |
| **Game Jolt** | 独立游戏免费托管与社交论坛 | 试图转型为面向 13-16 岁青少年的社交平台，启动严格的公开展区内容清查 | 2022 年因粗暴封禁成人及跨性别议题独立游戏遭到开发者强烈抵制，其公关嘲讽更加剧了开发者出逃；事实证明**做公共广场的审查必将两头不讨好**（[来源: PC Gamer 2022-01](https://www.pcgamer.com/indie-store-game-jolts-porn-ban-has-hit-games-with-no-sexual-content/)）。 |

### 核心警示
**做「公开曝光 / 发现大厅」是典型的重资产、高法律与合规风险业务。**
1. **内容审核与合规成本**：一旦有公开大厅，任何黄赌毒、钓鱼、侵权（盗版 Mario 或宝可梦同人）都会直接暴露在公众和监管视线中。正如 Glitch、itch.io 和微信生态的惨痛经验：平台会被连坐。
2. **公平性算法的无底洞**：只要有公共排序（排行榜），就必然存在刷票、刷时长、互投水军。为了反作弊就得做类似 itch.io Rating Queue 的复杂算法，最后让所有参赛者都不爽。
3. **商业化错配**：免费做公共社区会吸引大量毫无付费能力的羊毛党和滥用者（如 Replit 遇到的算力吸血和挖矿），最终拖垮产品财务模型。

---

## 六、新角度深度推演：四个特定场景与生态位

基于前述调研，回到创始人关心的问题：**既然公开的大众发现（B2C）是一条布满地雷的死胡同，那么在 B2B / 专有投递通道（B2B2C）上是否存在巨大的、未被满足的生态需求？**

### (a) 「一页可信的作品简历」：成为项目投递的事实标准
- **现实场景**：
  - 开发者给发行商（Poki、CrazyGames、Raw Fury、Kepler）或基金（Indie Fund、TapTap 聚光灯）投递时，表单里通常要求填：Demo 下载链接、演示视频、已有测试反馈。
  - 传统做法：发网盘链接（评委要下载、解压、承担安全风险）或发 itch.io 秘密链接（慢、没有详细漏斗数据）。
- **playtest.run 的机会**：
  - **不改当前架构**：`https://<slug>.playtest.run` 是游戏本体，而 `playtest.sh/p/<slug>` 可以成为一个公开的**「投递证书/作品简历页（Playtest Dossier）」**。
  - **简历页上有什么（完全基于现有设计提取）**：
    - 当前版本号（不可变哈希，如 `v7`）；
    - 开发者自述的「本次改动」；
    - 验证数据（匿名脱敏）：在过去 7 天内被 42 位真实玩家打开过、中位游玩时长 4 分 15 秒、主流运行设备覆盖（如 iOS Safari 30%、Android Chrome 50%、Desktop 20%）、JS 崩溃率 0.5%；
    - 玩家真实原话反馈摘要（3-5 条）。
  - **这与 DESIGN §3.7 冲突吗？**
    - **完全不冲突！** DESIGN §3.7 反对的是「做面向大众的发现大厅、榜单、让世界找到它」。而投递简历是**「把链接定向给具体的一个人（发行商 Scout、投资经理、老师）」**，依然属于 1.1 的「给具体的人看」，但让这个版本**极具可信度**。
  - **受众愿不愿意收？**
    - 发行商 Scout 和基金经理极度欢迎：他们不需要在自己的电脑上运行可疑的 exe，在手机或浏览器里点开即玩，且一眼看到真实玩家的基础游玩数据，极大提升其选品效率。

### (b) Game Jam 主办方作为 B 端：解决数百个提交的评审地狱
- **现状与痛点**：
  - 一个中型 Game Jam（如机核 BOOOM、Global Game Jam 各分站、大学社团 Jam）会有 50–300 个提交。
  - 主办方评委（通常是几个特邀嘉宾或内部策划）需要在 3–7 天内评完所有游戏。
  - 现在的做法：评委在 itch.io 上一个个点开，或者下载解压包。常常遇到安装失败、版本不对、不知道别人玩得怎么样。
- **playtest.run 的 Jam 主办方解决方案**：
  - 主办方发起一个「评测试玩集」（例如 `playtest.run/jam/booom-2026` 或仅通过命令行标签 `--jam booom-2026`）。
  - 参赛者在提交时只需一条命令：`playtest ./dist --jam booom-2026`。
  - **给评委的体验**：一个评委专用的免登录轻量列表页，评委用 iPad 或手机躺在沙发上就能逐个点开试玩，门禁页自动记录评委已玩过该版本；
  - **给主办方的看板**：主办方后台能一眼看到哪些作品评委还没玩过、哪些作品加载报错、平均通关率，彻底告别 itch.io 的中位数惩罚噩梦。

### (c) 课堂与教学场景：老师收作业的极简通道
- **前车之鉴**：
  - **Replit Teams for Education** 的教训：做重型 IDE、提供在线虚拟机执行环境，导致算力成本失控与滥用激增，最终只能将几十万师生无情抛弃。
  - **p5.js Web Editor** 的痛点：官方编辑器公开 sketch 导致学生作业相互抄袭，老师无法设置私密提交；滑铁卢大学 CS105 课程大纲中甚至明文要求学生「不要在 p5.js web editor 上做作业，因为所有人都能通过 URL 看到答案」（[来源: Waterloo CS105 Outline](https://student.cs.uwaterloo.ca/~cs105/W20_content/1201_CS105_Course_Outline.pdf)）。
- **playtest.run 的切入可能**：
  - 互动媒体、游戏设计、创意编码（Processing / p5.js / Three.js / Phaser）课程，老师每学期要收几百份作业。
  - playtest.run **不运行服务端代码**（只托管静态前端文件），算力成本几乎为零；支持口令访问与匿名短链（DESIGN §3.6）；
  - 老师只要给出一个 `--tag class-nyu-itp-2026`，学生终端直接敲 `playtest ./my-sketch`，老师后台形成一个干净的列表。

### (d) 反向风险警示：为什么绝对不能做「中心化曝光广场」
- 调研结果给出了压倒性的反面证据：
  1. **极度脆弱的合规生命线**：playtest.run 采用香港边缘节点、无大陆备案、免登录匿名链接（DESIGN §5）。一旦在根域开放公开浏览大厅（Showcase / Hall of Fame），微信、监管机构、爬虫会在第一时间将整个根域名直接封禁（域名污染或微信拦截），所有开发者的正常测试链接将全军覆没。
  2. **反作弊成本碾压团队规模**：一旦有公开榜单，就会有流量争夺，进而出现刷点击、刷留存、恶意差评。目前的双人或小团队根本无法承担审核员与反作弊工程师的成本。
  3. **定位漂移**：一旦做曝光，产品心智就会从「专注开发循环的高效 CLI」漂移成「二流的 itch.io 仿品」。

---

## 七、对 playtest.run 的含义：支持、冲突与修改建议

### 1. 哪些发现强烈支持 DESIGN.md 的现有结论？
- **支持「不做大众发现与社区」（§3.7）**：Glitch 的关停、Replit 的收缩、itch.io 的支付危机表明，小团队做公开社区和大众曝光是自寻死路。
- **支持「玩家零门槛、免登录即开」（§1.2, §3.3）**：Web 游戏平台（Poki、CrazyGames）的核心护城河就是零安装摩擦。Jam 评委和资方 Scout 极其排斥需要注册或装专用客户端的提交。
- **支持「从不执行用户代码，只做静态分发与流量转发」（§3.7, §4.8）**：正是这一条原则，让 playtest.run 避免了 Replit 和 Glitch 因算力吸血和恶意滥用而倒闭的宿命。
- **支持「把对游戏有感的细节做到极致」（§4.2）**：COOP/COEP（SharedArrayBuffer）、预压缩 `.br/.gz`、wasm MIME 是 Web 游戏最大的技术阻碍。The Gaming Nest 和通用的隧道工具都做不好，而这正是 playtest.run 的核心护城河。

### 2. 哪些发现与 DESIGN.md 产生了冲突或暴露了盲点？
- **冲突/盲点 1：把「结果」仅仅当成开发者的私人仪表盘，忽视了「结果作为信用凭据（Proof of Play）」的对外价值。**
  - 当前 DESIGN §3.4 仅设想开发者自己在控制台看数据。但在现实商业链条中，开发者不仅想自己看，更需要把数据拿给别人看（给投资人证明玩法成立、给 Jam 评委证明经过验证、给发行商做软启动初筛）。
- **冲突/盲点 2：忽略了 Game Jam 主办方与高校教师作为核心「B 端催化剂」的潜力。**
  - DESIGN 目前把 Jam 参与者视为单体开发者（§1.1）。但实际上，Game Jam 的**主办方/评委**才是最大的效率受害者。如果能给主办方提供一个「评测集」工具，就能一次性直接撬动该场 Jam 的几百名开发者使用 playtest.run。

### 3. 对 DESIGN.md 的具体修改建议（建议清单，不直接修改文件）
1. **新增「作品简历页 / 投递报告（Dossier）」功能（建议排期在 v0.2）**：
   - 开发者可一键生成一个只读的分享页（如 `playtest.sh/dossier/<slug>?token=xxx`），展示：版本不可变凭证、真实游玩人数、平均游玩时长、主流设备表现、玩家精选反馈。不进入公开展区，但让其成为向 Steam/Poki/TapTap 投递时的标准附录。
2. **在 v0.3 探索「Jam / 评审专用收集标签（Event Buckets）」**：
   - 允许开发者在 CLI 中指定 `--collection <id>`，为主办方/评委提供一个轻量、无需排榜的聚合测试列表，解决 itch.io 评审体验极差的痛点。
3. **坚定封死任何建立「公开首页榜单、游戏大厅、玩家公共评论区」的念头**：
   - 在 DESIGN §3.7 中强化这一条的论据，明确引用 Glitch 与 itch.io 的教训，守住轻量与安全底线。

---

## 八、最有想象力但仍可验证的 3 个生态方向

以下 3 个方向均不破坏 DESIGN 核心原则，但能让产品在没有「公开大厅」的前提下长出强大的生态网络效应：

### 方向一：发行商/基金投递用的「Playtest 事实简历」（The Verified Pitch Link）
- **核心逻辑**：
  - 开发者给发行商/孵化器投递时，附带一个由 playtest.run 官方背书的 `playtest.sh/dossier/<token>`。
  - 页面上清晰展示：该作品真实被 35 个人玩过、加载耗时 1.8 秒、中位游玩时长 6.5 分钟、零崩溃率。
- **工作量**：
  - **极小（约 2-3 天）**。数据在控制面已有（SQLite），只需在 `playtest.sh` 渲染一个简洁专业的只读总结卡片，支持导出为网页或 PDF。
- **与 DESIGN 的冲突**：
  - **无冲突**。不仅不与 §3.7 冲突，反而深化了 §3.4「知道结果」的产品价值，让结果从「自嗨」变成「信用资产」。
- **验证方法（20 个私测开发者）**：
  - 挑选 5 个准备参加 2026 下半年 TapTap 聚光灯、GWB 或准备向 Poki/CrazyGames 投稿的开发者；
  - 协助他们将此链接放入投递邮件或表单的 Demo 栏；
  - 观察接收方（Scout/投资经理）是否点开、停留时长，以及开发者是否反馈「对方觉得很专业」。

### 方向二：面向小型 Jam / 线下活动主办方的「评委快筛通道」（Jam Host Pass）
- **核心逻辑**：
  - Jam 主办方生成一个专属 token，参赛者使用 `playtest ./dist --event <token>` 提交；
  - 主办方获得一个极简的「评委专用试玩看板」：所有作品支持一键全屏秒开试玩，自动标记该评委是否已玩过，评委可单向打分或留一句话评语。
  - 解决 itch.io 队列冗长、下载麻烦、中位数惩罚扭曲排名的痛点。
- **工作量**：
  - **中等（约 1 周）**。CLI 增加一个可选参数，API 增加一个事件关联表，控制台增加一个专为评委优化的全屏快试 UI。
- **与 DESIGN 的冲突**：
  - 略微扩展了 §3.6（谁能玩）与控制台的角色，但观众依然是「具体的人（主办方评委，通常 3-10 人）」，严格杜绝了大众化榜单。
- **验证方法（20 个私测开发者）**：
  - 联合一个身边的小型高校 Game Jam 或社群微型 Jam（20-30 个参赛作品）；
  - 约定所有参赛者统一使用 playtest.run 交付，给 3 位评委发放评测链接；
  - 检验指标：评委是否能在 2 小时内顺畅试玩完全部作品，是否有坏包白屏，评委的主观满意度。

### 方向三：教学课堂的「免翻墙互动作业收集器」（Classroom Collector）
- **核心逻辑**：
  - 针对国内高校互动媒体、数字媒体艺术专业（讲授 p5.js、Three.js、Phaser）教师收作业难、学生作业互相抄袭的痛点；
  - 老师在后台创建一个班级作业集，学生在本地终端一条命令提交；老师在教师端统一批量查看并测试学生代码与运行表现。
- **工作量**：
  - **中等（约 1 周）**。主要基于已有上传和权限机制，增加一个简单的批量管理看板。
  - 相比 Replit 和 p5.js web editor，playtest.run 不跑服务端代码、不卖算力，香港节点大陆直连，极具成本与访问优势。
- **与 DESIGN 的冲突**：
  - 观众完全契合 §1.1「课堂演示与作业」，是给具体的人看；没有破坏任何核心约束。
- **验证方法（20 个私测开发者/师生）**：
  - 找 1 位讲授互动媒体/游戏开发的大学老师及其班级（20 位学生）；
  - 用一次小作业（例如提交一个 Phaser 小关卡）进行全流程实测；
  - 检验指标：学生从写完到生成链接的成功率是否达到 95% 以上，老师批改 20 份作业的耗时是否降低 50%。

---

## 结论摘要

1. **大平台的 Playtesting 正在成为发行的硬前置**：Poki（Poki Playtesting/100M MAU）与 CrazyGames（两阶段 Basic Launch/50M MAU）均用自动化试玩与两周真实数据作为准入门槛，证明「先给几百人测，用数据说话」已是行业共识。
2. **公开曝光大厅是不可触碰的高压线**：Glitch（2025年7月关停托管）、Replit（砍停 Teams for Edu 与 Bounties）、itch.io（深陷支付封杀危机与中位数评分作弊）的教训表明，做大众发现和大厅会招致沉重的内容审核、作弊治理与合规成本，小团队必被拖垮。
3. **破局点在于「定向信用（B2B/B2B2C）」**：playtest.run 应坚定贯彻「不做大众发现」，但应把结果转化为面向发行商、Jam 评委和老师的「一页可信作品简历」与「专有评审通道」。
