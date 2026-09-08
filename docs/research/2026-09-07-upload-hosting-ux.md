# 调研：「上传 Web 构建拿链接」的实际使用体验与商业模式

> 2026-09-07。调研线：上传类竞品（The Gaming Nest、itch.io、Unity Play、SIMMER.io、PlayFeed、Newgrounds、GameJolt、Final Parsec，以及被游戏开发者当托管用的通用静态托管）。
> 本文只做记录与判断，不改 `docs/DESIGN.md`；末尾第 6 节给出对 DESIGN 的修改建议。

## 0. 方法、可信度与本文的读法

- 标记规则：**〔一手〕**＝官网 / 官方文档 / 定价页 / 源码；**〔原话〕**＝论坛、issue、评论里的用户或平台方发言，带链接；**〔二手〕**＝媒体报道或第三方汇总；**〔推断〕**＝我从上述材料推出来的，没有直接来源。
- 所有数字都带日期。查不到就写「没查到」。
- 共完成 27 次搜索 / 抓取。调研末段搜索工具连续故障约 20 分钟，第 5 节表格里几个格子因此停在「未知」，第 7 节逐条列出了没查到的东西。**没有任何一条是我实测的**——没有注册账号、没有真机上传，所有「几步 / 多久」都是从官方文档的步骤数推出来的，不是计时结果。

一句话总结：**「上传 → 链接」这件事本身已经被做烂了，而且免登录出链接是行业常态、不是卖点；真正没人做好的是「上传时告诉你为什么跑不起来」、「一条命令走完到可玩链接」、「改一行三秒上线」和「大陆能打开」这四格。**

---

## 1. 逐个产品：事实卡

### 1.1 The Gaming Nest（`tgn deploy`）

**这不是一个开发者工具公司，是一个阿拉伯语游戏社区。** 这一点改变了它在竞品格局里的位置。

〔一手〕来源：[about](https://thegamingnest.com/en/about)、[faq](https://thegamingnest.com/en/faq)、[webgl-hosting](https://thegamingnest.com/en/webgl-hosting)（页面自标 Last updated 2026-07-12）、[publish](https://thegamingnest.com/en/publish)、[webgl-hosting/unity](https://thegamingnest.com/en/webgl-hosting/unity)、[webgl-hosting/godot](https://thegamingnest.com/en/webgl-hosting/godot)、[CLI 指南](https://thegamingnest.com/en/articles/904fbf7d-b883-4372-a9e9-c2e452c6cf13)。

- **团队与时间线**：总部约旦安曼，2025 年成立。Beta 2025 年 11 月，正式版 2026 年 2 月。〔二手，LinkedIn 公司页 2026〕员工 2 人。自我定位「the largest Arab gaming community for developers and designers」，全站阿拉伯语 / 英语双语 + RTL。
- **WebGL 托管只是平台的一个功能**。平台还有：开发者档案、招聘、游戏 jam、Pitch Hub（5 项评审标准的策展）、培训课程、阿拉伯语漫画，2026-06-09 又上了积分 / 商城 / 邀请返利系统。
- **注册门槛**：要账号。CLI 要先在网页 `/profile/api-keys` 建 key（`tgn_k1_…` 格式，只显示一次），再 `tgn auth login`。
- **从零到链接**：官方文档写成 **5 步**（建 key → 登录 → 选 project → deploy → 在管理页上翻 Publish 开关）。浏览器路径也是 5 步（建 project → 开 WebGL 页 → 初始化并传 zip → 激活版本 → Publish）。**没有「一条命令从零到可玩链接」这条路**——项目必须先在网页里存在。
- **免费额度**：300 MB / 账号（所有作品共享）；单个压缩包 ≤ 500 MB；解压后 ≤ 1,000 个文件 / 1 GB；每个项目保留 10 个版本；超出的版本 24 小时内清理但保留在历史里。拒绝 `.exe .bat .cmd .sh .ps1 .dll .msi .com .scr .vbs`。签名上传票据有效期 15 分钟。静态资源边缘缓存 365 天 immutable。
- **付费**：〔一手〕FAQ 只说「one-off paid tiers raise the quota, or you can submit a custom-quota request」。**没有定价页，具体价格没查到。** 平台整体的收入来源写得很明确：「All core features are free... Payment is required only for some advanced training courses.」即托管不赚钱，课程赚钱。不抽成、不要独占。
- **对游戏的处理**：
  - 预压缩：`tgn deploy --precompress` 在客户端做 Brotli，「cuts the payload 3–5×」，CDN 直接把预压缩文件给浏览器。
  - **COOP/COEP：明确不做。** Godot 页原话：「We serve builds from a separate storage origin without those headers, so a thread-enabled Godot export will load and then fail. Export with thread support disabled and it runs.」publish 页 FAQ：「Don't enable thread support — a build that relies on SharedArrayBuffer will not run.」
  - ⚠️ **它自己的文档互相打架**：同一站点的通用发布文章《How to Publish Your Game Online as WebGL》（`thegamingnest.com/en/articles/f5903cc3-…`，UUID 未逐字核对）写「The platform hosts the files, serves the correct MIME types for `.wasm` and `.br`, **sets the isolation headers Godot threads need**, and hands you a shareable link」。以引擎专页和 FAQ 为准的话，这句营销文案是错的。**DESIGN.md §2.1 说 TGN「明说 SharedArrayBuffer 的构建跑不了」是对的。**
  - Range、SPA 回退：没查到。
- **增量上传：没有。** CLI 的五步是「zip 整个文件夹 → 初始化 → 要签名 URL → 整包传 → 服务端解压激活」。改一个文件要重传整包。
- **链接形态**：作品挂在公开 project page 上（路径式，`/projects/manage/{projectId}/webgl` 是管理页）。构建从**独立的 storage origin** 提供，可 iframe 嵌入。分享到 X 有 `twitter:player` 内联播放卡片。
- **版本 / 回滚**：10 版，一键激活或回滚，不用重传。
- **访问统计**：没查到。
- **移动端、大陆可达**：没查到。
- **社区反响**：**在 Hacker News、Product Hunt、Reddit 上都没找到发布帖或讨论。** 能找到的只有 LinkedIn 公司页发帖，互动个位数到 24 个 reaction。〔推断〕它的实际开发者用量很可能非常小，而且用户集中在 MENA。

**它自己写下的、最值得抄的一段话**（webgl-hosting 页，谈为什么要设硬额度）：

> In April 2025 the best-known free WebGL host in this category went offline, after repeated upload abuse and denial-of-service attacks turned uncapped storage and bandwidth into a cloud bill no free service could absorb. **Free hosting without a hard per-account ceiling is not a business model — it is an open invoice.**

它指的是 SIMMER.io（见 1.4）。

### 1.2 itch.io

〔一手〕来源：[butler 手册 · Pushing builds](https://itch.io/docs/butler/pushing)、[Limited Playtests & Releases](https://itch.io/docs/creators/limited-releases)、[Accepting Payments](https://itch.io/docs/creators/payments)、[Experimental SharedArrayBuffer Support](https://itch.io/t/2025776/experimental-sharedarraybuffer-support)、[Introducing open revenue sharing](https://itch.io/updates/introducing-open-revenue-sharing)、[Update on NSFW content](https://itch.io/updates/update-on-nsfw-content)。

- **注册门槛**：要账号。
- **从零到链接**：网页上传是「建 project → 填页面信息 → 传 zip → 把页面类型从 Downloadable 改成 HTML → 勾 This file will be played in the browser → 发布」。
- **butler CLI 的关键限制**：`butler push directory user/game:channel`。〔一手〕文档原话：「Tagging a channel as 'HTML5 / Playable in browser' needs to be done from the itch.io Edit game page, **once the first build is pushed**. The page also needs to be set to 'HTML' rather than the default 'Downloadable'.」**即 CLI 单独无法产出一个可玩链接**，第一次必须回网页点两个开关。
- **增量上传：做得最好的一家，但有代价。** 〔一手〕butler 分两段：本机用 rsync 式定长块 diff + 低质量 Brotli 生成 default patch；推上去之后后端用 bsdiff 重新逐字节 diff + 高质量 Brotli 生成 optimized patch。文档原话：「How long it takes depends on the size of your build, and **for a large game it can run for around 30 minutes**. This is the 'processing' step.」
- 〔二手，2026-05-18〕butler push 进了 itch 桌面 app 的 GUI（v26.12.0+），不用开终端。来源：[gamineai.com 汇总](https://gamineai.com/blog/how-to-use-itch-butler-in-the-app-first-gui-upload-evening-2026)（未在 itch 官方站点二次确认）。
- **HTML5 硬限制**（〔一手〕，转引自官方 Uploading HTML5 Games FAQ，在[论坛帖](https://forum.godotengine.org/t/posting-a-biggish-game-on-itch-io-200mb-pck-file-size-limit/141054)里被完整引用）：解压后 ≤ 1,000 个文件；路径含文件名 ≤ 240 字符；解压后总体积 ≤ 500 MB；**单个解压后文件 ≤ 200 MB**；文件名区分大小写、UTF-8。
- **整体上传上限**〔原话，社区整理〕：标准网页 1 GB / butler 2 GB / 申请后 4 GB / 特批 5 GB+。
- **COOP/COEP**：项目编辑页 Embed Options → Frame Options 里一个「SharedArrayBuffer support」勾选框。开了之后走第二套 CDN 配置（`html.itch.zone`），游戏文件加 `COOP: same-origin` + `COEP: require-corp` + `CORP: cross-origin`，承载页加 `COOP: same-origin` + `COEP: credentialless`。**副作用极长**，见第 2 节用户原话。
- **访问控制**（〔一手〕Limited Playtests 文档）：四档——限定名单 / 口令 / 秘密 URL / 公开。itch 明确把这套包装成「run limited playtests」，并且用 wharf 增量作为卖点：「You shouldn't be penalized to send a new build to your testers just because your game is big.」
- **链接形态**：`user.itch.io/game`，子域 + 路径。游戏本身在 iframe 里，跑在独立 CDN 域上。**官方明确不支持直接用 CDN URL**：「The only supported ways to play an HTML game hosted on our platform is either directly from the project's page or using the Game Embed functionality.」
- **大陆可达：完全打不开。** 〔一手，GreatFire 实测数据〕[itch.io 在中国大陆：全部被屏蔽](https://zh.greatfire.org/domain/itch.io)：「防火长城会污染其 DNS、静默丢弃数据包使请求超时，直接阻断连接。自 2016-02-24 以来，该域名下共进行了 374 次测试，最近一次为 2026-08-30」，26 个可判定 URL 全部被屏蔽；[域名页](https://zh.greatfire.org/itch.io)：「自 2025-09-25 起每一次有定论的测试均被屏蔽」，2026-06-19 的测试 6 个大陆节点全部显示干扰，屏蔽率 100%。
- **商业模式**：Open Revenue Sharing，2015-03-23 上线，卖家自己设 0%–100%，默认 10%。支付通道费（约 $0.30 + 2.9%）转嫁给卖家。〔一手〕FAQ 原话：「Because we are not guaranteed an amount from sellers through the sales of their goods, we pass any operating costs directly onto the seller.」**没有公开财务数据。**
- **2025 年 7 月的支付通道危机**（这条决定了「谁在赚钱、赚得稳不稳」）：〔一手〕[Update on NSFW content](https://itch.io/updates/update-on-nsfw-content)——因为一款叫 No Mercy 的游戏，澳大利亚组织 Collective Shout 向 itch 的支付通道施压，itch 在 2025-07-24 前后把**全部成人 NSFW 内容从浏览和搜索里 deindex**，暂停 Stripe 的 18+ 支付。leafo 原话：「If we lose our ability to accept payments from a partner like PayPal or Stripe, we impact the ability of all creators to do business.」〔二手〕[TechCrunch 2025-07-27](https://techcrunch.com/2025/07/27/itch-io-is-the-latest-marketplace-to-crack-down-on-adult-games/)、[Rascal News](https://www.rascal.news/itch-io-delists-bans-games-under-pressure-from-payment-processors-and-an-australian-anti-porn-group/)。〔原话〕受影响创作者报告收入归零、payout 无法发起。

### 1.3 Unity Play（编辑器「Publish to Play」）

〔一手〕来源：[Unity 手册 · Web build settings reference](https://docs.unity3d.com/6000.4/Documentation/Manual/web-build-settings.html)、[play.unity.com FAQ](https://play.unity.com/en/faq)。

- **从零到链接**：Build Profiles → Web → Publish to Play 按钮 → 上传完自动打开浏览器完成发布。用 Unity 引擎账号登录。
- **体积上限**：〔一手，手册原话〕「Web builds must be under 1 GB in size to upload to Unity Play.」
- **访问控制**：〔一手，FAQ〕Public / Unlisted（可选加口令）/ Private 三档，另有 mobile controls 开关。（注：〔原话〕2021 年 Unity 员工 MarcelPursche 曾说不支持限制访问；到 2025-07 社区已确认可以设 unlisted + 口令。功能是后加的。）
- **CLI / 版本 / 回滚 / 增量：都没有。** 只有编辑器按钮和网页拖拽，可以「replace an existing game」但没有版本列表。
- **可靠性是它最大的问题**，见第 2 节用户原话。社区自己写了一份「手动上传 workaround」指南在流传。
- **对游戏的处理**：Unity 自家平台，〔推断〕wasm MIME 和压缩应当正确；COOP/COEP 没查到。
- **商业模式**：免费，是引擎的引流 / 学习漏斗（FAQ 措辞是「a space to publish and play games made by creators like you」）。
- **大陆可达**：〔原话〕Unity 论坛资深用户 CodeSmile 回答上传失败时说：「If you are located in a sanctioned or 'firewalled' country (no, not Canada, more like China, Iran, ..) you may not be able to upload.」——从大陆上传本身就不被期待能成功。

### 1.4 SIMMER.io —— 这条线最重要的死亡案例

〔一手〕[simmer.io 首页](https://simmer.io/)（抓取时显示「UPLOAD.PLAY.REPEAT. The arcade for Unity WebGL games is powering back up... ▶ INSERT COIN — FALL 2026」）、[创始人公告全文](https://simmer.io/c/founders-club)（经搜索索引取得，直接抓取返回 403）。

创始人 Rocco 的公告原话（〔原话〕）：

> I have some very unfortunate news: SIMMER.io and ShareMyGame.com are now discontinued. At this time, there's a strong likelihood they will not return.
> Despite having billing alerts in place, **I was hit with astronomical cloud bills while the site was under attack from bad actor(s).** I'm currently working with the cloud providers to negotiate forgiveness for those charges.
> **I had to delete the main storage buckets during the attack to stop the financial bleed.** However, I backed up game data to another cloud provider... Most games were preserved. A small number (<0.1%) may have failed to copy during the backup process.
> I did what I could to keep it secure — like using Cloudflare CDN — but my resources were limited.

一个受影响的用户（〔原话〕，[salahkouhen.com, 2026-05-14](https://www.salahkouhen.com/simulations-are-down/)）：

> The simulations and games on this website have been hosted on simmer.io. Unfortunately, Simmer.io was hit with a DDoS attack, resulting in **an unexpected bill in the hundreds of thousands USD** for its owner, forcing them to shut down the site. Although I was initially hopeful the site could be restored, it has now been down for a year. The simulations will be migrated to itch.io as I find the time to do so.

〔二手，LinkedIn〕Simmer Industries，2017 年成立，旧金山，2 人。

要点：**这是一个和 playtest.run 形态几乎一样的产品**（上传 Unity WebGL 构建 → 拿链接 → 分享），做了 8 年，被一次 DDoS 打到必须删自己的存储桶止血。它的死因不是竞争，是**免费托管的成本上限没有设计**。TGN 的整段营销文案就是围着这件事写的。

### 1.5 PlayFeed

〔一手〕来源：[playfeed.app/create](https://playfeed.app/create)。

- 定位：「Upload an HTML game and share a browser-play link in minutes.」明确面向 AI 产物：「Built with ChatGPT, Claude, Gemini, Construct, Godot, GDevelop, or by hand?」
- 注册门槛：**Continue with Google**，要账号。
- 形态：单个 HTML 文件或粘贴代码。「PlayFeed is built for lightweight browser games.」
- 访问控制：private / unlisted / public 三档。
- 安全：「Play happens on an isolated play origin. Uploaded game HTML runs separately from the main app.」
- 它自己写的差异化（〔一手〕原话，值得注意，因为这正是「托管 vs 平台」的分界）：
  > Raw hosting can put a file online, but it does not automatically give you a game page, a browser-play destination, visibility controls, tags, or a creator identity on the platform.
- 游戏页可以填 blurb、tags、controls、device notes、**AI disclosure**。
- **没查到**：体积上限、免费额度、付费档、团队背景、是否有 CLI、发布帖（HN / PH / Reddit / X 上都没找到）。

### 1.6 一整类 DESIGN.md 没写的竞争者：AI 单文件 HTML 游戏托管

这是本次调研最大的意外。DESIGN.md §2.1 只列了 PlayFeed 一家，实际上 2026 年这一格挤满了人，而且**其中两家已经把「发布」做成了 MCP 工具，AI agent 可以直接发**——这打的正是 DESIGN.md §1.1 的第一类用户。

| 产品 | 一手事实 | 来源 |
| --- | --- | --- |
| **AIGameShare** | 单 HTML 或 zip 构建；公开游戏页含 plays / likes / comments / creator profile / leaderboard；**「Claude Code, Cursor, Codex, ChatGPT, Gemini, and other MCP-capable agents can publish or update games through the AIGameShare MCP server or REST API」** | [aigameshare.com](https://www.aigameshare.com/share-ai-game) |
| **rendrd** | 拖一个 `.html` 就出链接，**不用账号、不用 Git、不用构建**；「Connect rendrd.io once as an MCP server and your assistant does the rest: ask it to publish, and it hands you a live rendrd link inside the chat」；免费账号加口令保护和浏览量看板 | [rendrd.io](https://rendrd.io/) |
| **ArcadeLab**（原 KidHubb） | 粘贴单个 HTML 就发布，**无需注册**；≤ 500 KB；沙箱 iframe，`connect-src none`（fetch/XHR/WebSocket 全禁）；顶部要加 `ARCADELAB` 注释头声明标题和用到的库，平台自动注入 phaser/p5/three/gsap/tone/pixi/matter/d3/react；URL 形如 `arcadelab.ai/play/<title>-<creator>`；Creator Code 存在浏览器里，更新保持同一 URL | [arcadelab.ai](https://arcadelab.ai/)、[分享指南](https://arcadelab.ai/learn/share-interactive-thing-made-with-ai) |
| **localgames.fun** | 单个 `.html` ≤ 5 MB；要标注用了哪个 LLM / agent；沙箱 iframe `allow-scripts` only；**每个游戏保留两个版本**可回滚；公开主页 `/u/<name>`；免费无需信用卡 | [localgames.fun](https://localgames.fun/) |
| **bunpav** | 更上游——聊天生成游戏 + 实时预览 + public `/play` 链接 + 排行榜 | [bunpav.com](https://bunpav.com/features/browser-game-generator) |

〔一手〕还提到但没深入查证的：UltimatePlay、EveryGameMade、Summer Engine。

〔推断〕这一群的共同形态是：**零门槛、单文件、沙箱、平台化（有游戏页和创作者身份）**。它们不碰引擎导出物（几百 MB、上千个文件、要 wasm MIME 和预压缩），所以和 playtest.run 的重叠只在「AI 写小东西的人」这一类用户上——但那恰好是 DESIGN.md §1.1 排第一的人群。

### 1.7 Newgrounds

〔一手〕[supporter 页](https://www.newgrounds.com/supporter)；〔二手，社区 wiki 但引用了 Tom Fulp 原帖〕[Supporter Status](https://newgrounds.wiki.gg/wiki/Supporter_Status)、[Newgrounds Finances](https://newgrounds.wiki.gg/wiki/Newgrounds_Finances)。

- 独立所有（Tom Fulp），25 年以上历史，不公开财务。
- **靠什么活**：几乎完全靠 Supporter 订阅，已基本弃用广告。supporter 页原话：「Newgrounds is an independent website with over 25 years of history. We don't sell or share your personal data and we don't do any weird tracking for personalized ads. Our goal is to be 100% ad-free, ideally with a surplus that could plug into our rev-share system, previously developed for ads. **We can't do this without your help!**」
- **规模数字**：〔二手 wiki，引 Tom Fulp 2025-08〕近 9,000 名活跃 Supporter，按最低 $3/月算约 **$27,000/月**；「Despite receiving thousands of donations, **the site still operates at an undisclosed loss**」。
- **涨价**：2026-03-02 宣布、2026-04-06 生效，新订阅从 $2.99/月、$25/年涨到 **$5/月、$36/年**，老用户不涨。这是 Supporter 上线以来第一次调价。默认档位设成 $4.99「because it's much more sustainable for us after processing fees」。
- 历史对照：2007-10 首页单个横幅位可卖到 $17,000/月；2012 年前后广告收入崩塌，裁员、关店，才有了 Supporter。

**这条对 playtest.run 的意义**：一个有巨大存量用户、25 年品牌、极高忠诚度的免费网页游戏站，2025 年靠捐款月入约 $27k **仍然亏损**。「网页游戏托管」这个品类本身不产生现金流。

### 1.8 Kongregate —— 关了六年，2026 年 4 月带着「分析」重开

- 〔二手，2020-07-02〕[The Verge](https://www.theverge.com/2020/7/2/21311318/kongregate-stops-accepting-new-game-submissions-flash-discontinued-layoffs)、[Engadget](https://www.engadget.com/kongregate-stops-accepting-new-games-july-22nd-191508621.html)、[PC Games Insider](https://www.pcgamesinsider.biz/news/71325/kongregate-is-no-longer-accepting-new-games-makes-layoffs/)：停止接受新游戏投稿（当时 128,655 款），关闭多数聊天室和论坛，同时裁员。公司声明：「we're focused on our internal game development and acquisitions rather than our legacy flash gaming platform」。
- 〔原话，LinkedIn 2026-04-24〕：「Indie devs — Kongregate is open for new game submissions again! Earn up to 70% revenue share while reaching an established player community... **Our newly rebuilt Developer Portal gives you the analytics and community tools you've been missing on other platforms to grow your audience.**」
- 〔原话，Jon Dean，2026-06-26〕：「Kongregate has reopened to developer submissions... I'm genuinely excited about the mission to help developers build and grow their games, so that **uploading is not 'sit back, wait and see what happens' but 'let me look at the tools available to see the feedback I'm getting, and what the analytics tell me on how to improve my game'**. Perfection is not the goal: improvement is.」

**这条直接回答创始人的问题 #3**：一个 2006 年的老平台在 2026 年重新入场，它的卖点不是流量、不是分发，是**给开发者的反馈与分析**。市场上有人认为这一层值钱。

### 1.9 Final Parsec —— 已经做了「免登录出链接 + 引擎插件」

〔一手〕[hosting 页](https://www.finalparsec.com/tools/unity_game_hosting)、[插件安装](https://www.finalparsec.com/blog_posts/install-plugin)、[上传流程](https://www.finalparsec.com/blog_posts/final-parsec-upload)、[API 文档](https://www.finalparsec.com/docs)、[GitHub](https://github.com/Final-Parsec/official-plugin)。

- 原话：「We have an open source plugin that lets you upload your games straight from Unity with the click of a button. **You don't even need an account to get started publishing and sharing your games!** Add the script to your project, and you have one-click releases of your game straight to the internet.」
- 流程：把 `UploaderWindow.cs` 丢进 `Assets/Editor/` → Final Parsec → Upload Game → 填游戏名、勾场景 → 一个按钮**同时构建并上传** → 完成后给一个「play and **claim** your game」的按钮。
- 免费、不放广告、编辑器上传次数不限；免费账号有公开主页，最多展示 5 个作品。
- API：`POST /api/games`，bearer token，JSON body 里塞 base64 编码的 `.wasm` / `.pck` 等资产。〔推断〕base64 有约 33% 体积开销，对大构建不合适。
- 规模：GitHub 仓库 2 个 star、2 个贡献者、2022-12-19 创建。〔推断〕极小。

**为什么重要**：DESIGN.md §3.2 把「第一次运行不登录也能拿到匿名链接」和「引擎导出插件」都当成产品设计的亮点（后者放在 §8 留门给社区）。Final Parsec 用两个人的力量把这两件事都做了，而且是 2022 年就做了。**这两条都是入场券，不是差异化。**

### 1.10 GameJolt

〔一手〕[Add your game](https://gamejolt.com/help-docs/creators/add-game)；〔原话〕[如何上传 HTML build](https://gamejolt.com/f/how-do-i-upload-an-html-build-to-game-jolt/13867)。

- 流程（社区总结，官方帮助文档口径一致）：zip 里 `index.html` 在根 → 新建 package / release → 加 build → 选 Browser → 选 HTML → 传 zip → **手动填画布的 width 和 height** → Upload build → Publish build。
- 单个 build ≤ 2 GB（〔一手〕平台公告，11 年前从 1 GB 提到 2 GB）。
- 论坛已归档只读。没找到 CLI、版本回滚、增量上传。
- 〔推断〕流程停留在 2015 年前后的形态，手填画布尺寸这一步说明它不把响应式 / 移动端当默认。

### 1.11 通用静态托管（游戏开发者实际在用的）

| 产品 | 免登录出链接 | 免费额度（日期） | 对游戏最致命的一条 |
| --- | --- | --- | --- |
| **Netlify Drop** | ✅ 可以先部署后认领 | **自 2025-09-04 新账号改成积分制**：免费 300 credits/月，硬上限、不能加购。生产部署 15 credits/次，带宽 20 credits/GB，请求 2 credits/万次 | ① 300 credits ≈ **20 次部署**或 **15 GB 带宽**，二选一；② 「if one site/web project exceeds its limits, **all sites/projects on your team will be paused**」；③ 「Deploys under 50MB work best, and **individual files over 10MB may cause your deploy to get stuck**」——Unity 的 `.wasm` / `.data` 经常超过 10 MB |
| **Surge** | ✅「No account setup beforehand—the CLI walks you through creating one on first publish」 | 免费不限站点数 + 自定义域名 + SSL；付费 $30/月（口令保护等） | 完全不管游戏：没有 wasm MIME 保证、没有 COOP/COEP、没有预压缩协商的说明。**但它的 CLI 形态是所有竞品里离 playtest.run 最近的**：`surge ./dir`、immutable revisions、`surge rollback` / `rollfore` / `cutover`、`surge revs`、`surge stats`（traffic / audience / usage / load）、`surge --preview`。需要 Node 18+ |
| **tiiny.host** | ✅ 不用账号就能开始 | 免费：1 个项目、**每个项目 3 MB**、3 次上传/天、5,000 次访问/月、5 GB 带宽/月，带一条品牌角标；Tiny $5/月 25 MB；Solo $13/月 5 项目 × 75 MB + 口令 + 自定义域名；Pro $31/月；Pro Max 2 TB / 200 GB 带宽（限额页 2026-08-21 更新） | **3 MB 免费额度对任何引擎导出物都不够用**。免费链接只要每 3 个月登录一次就不下线 |
| **Neocities** | ❌ 要账号 | 免费 1 GB 存储 / 200 GB 带宽 / 无广告 / Anycast CDN；Supporter $5/月 → 50 GB / 3,000 GB | 免费档有**文件类型限制**、**严格 CSP**、**无 CORS**——这三条恰好是引擎导出物会踩的。这些限制要 $5/月才解除 |
| **GitHub Pages** | ❌ 要 Git | 无限带宽（fair use） | **无法设置任何自定义响应头** → 没有 COOP/COEP、没法强制 Content-Encoding。唯一出路是 [`coi-serviceworker`](https://github.com/gzuidhof/coi-serviceworker)，它「will reload the page on the user's first load」，且必须同源、单独文件、HTTPS。真实案例：〔一手〕[laride/img2lv 提交 14cd253, 2026-07-08](https://github.com/laride/img2lv/commit/14cd25316b33ab831c669956e1e31d4d1efcfa8c)「GitHub Pages cannot set custom response headers, so SharedArrayBuffer... was unavailable, causing Buffer errors at runtime」 |
| **Cloudflare Pages** | ❌ 要账号 | 免费 500 次构建/月、1 并发、20,000 文件/次部署、**单文件上限 25 MiB**、`_headers` ≤ 100 条规则、静态请求与带宽不限（[官方 Limits 页](https://developers.cloudflare.com/pages/platform/limits/)） | **25 MiB 单文件上限**会直接卡住中等以上的 Unity `.wasm` / `.data`。好处是 `_headers` 能写 COOP/COEP |
| **Vercel** | ❌ 要 Git（据 2026-04 第三方对比） | **没查到**（搜索工具故障，未取得一手定价页） | 没查到 |
| **Glitch** | —— | **已于 2025-07-08 关闭托管**，见第 4 节 | —— |

---

## 2. 用户原话（这一节全部带链接）

### 2.1 SharedArrayBuffer / COOP-COEP：一个开关引发的四年长尾

全部来自 itch.io 官方公告帖 [Experimental SharedArrayBuffer Support](https://itch.io/t/2025776/experimental-sharedarraybuffer-support)（leafo 发于 2022-04-02，累计 46,419 次浏览、50 条回复；下面的相对时间是抓取时页面上的显示）：

- **平台方自己承认这是妥协**（leafo，4 年前）：「Because of the side effects from enabling the headers, it's not something we could just turn on for everyone, as we don't want to risk breaking our site and the existing games we host.」
- **YouTube 嵌入四年没修好**：verysoftwares（4 年前）「it seems that embedded youtube videos refuse to work while this setting is active.」→ leafo「That's correct... We'll probably have to switch over to opening iframe content in new tabs」→ Sensei Xiongmao（77 天前）「sorry for necro..but it has been a couple years and youtube video still refuse to work :C」
- **Firefox 因为一个过期的 origin trial token 全线挂掉**：rancidbacon（2 年前）「I happened to base64 decode the transmitted token & realised the `expiry` field had a value of `1704063600` which equates to `2023-12-31T23+00:00`」。leafo 的修法是「all Firefox versions to fall back to launching HTML games in a popup」。
- **弹窗修法又打坏了口令保护**：DamienKusters（2 年前）「Because the game is in its testing stages I have the game currently on Restricted with a required password... Due to the pop-up change, users reported that the **pop-up gave a 404 page** when loading the browser game... So testers that use Firefox are unable to play the browser build currently.」
- **两年后同一个 bug 还在**：Subversion Studios（225 天前）「We have SharedArrayBuffer enabled, because we are using Thread Support. **It reduces the game load time from 30s to 5s, and has a lot less audio stuttering.** Our exported build above works perfectly on Chrome, but shows a 404-not-found error on Safari and Firefox. I've confirmed it's something to do with the password setting.」
- **jam 的「随机试玩」按钮被打坏**：SeaLiteral（2 年前）贴出错误原文「Error The following features required to run Godot projects on the Web are missing: Cross Origin Isolation - Check web server configuration (send correct headers) SharedArrayBuffer - Check web server configuration (send correct headers)」，「whereas the same game, if I control-click the submission to button and then go back to the game in the new tab it works fine.」benjatk 跟帖「Same thing here. Seems to be an universal problem. Really annoying.」
- **itch 桌面 app 会崩**：Barldon（2 年前）「attempting to open the game's page in the Itch Desktop App results in the program hanging and no web page being displayed at all. **The Itch app even crashes sometimes.**」并追问（2 年前）「Enabling a feature when you submit your game should not cause the entire app to crash on page load, even if the feature is experimental.」
- **最能说明「这个开关没人搞得懂」的一条**——一位高中老师（Fuzzy，132 天前）：
  > I am a high school teacher. My students publish their games on Itch. **None of the games will play in full screen mode even though it's enabled. We see the icon in the play screen, but clicking it does nothing.** They are not complex games. **If I check SharedArrayBuffer it launches to full screen.** That seems like the perfect fix. The games are made in Godot 4.5 or 4.6. Earlier comments said ONLY enable it if you need it. **How do I know if I need my students to do it?** I don't want them to do it if it will break other parts of their games.
- 最新进展（leafo，63 天前）：「We just deployed a change for Firefox today, as it seems the separate Window is no longer required. **It's still required for Safari**」。

还有 Godot 官方 issue 里的定性（〔原话〕[godotengine/godot#69020](https://github.com/godotengine/godot/issues/69020)）：

> It's only hosting platforms such as itch.io which do not provide a good way to configure Cross Origin Isolation, but that's **an ecosystem problem that hosting platforms will have to solve**.

同一 issue 整理的托管方状况：itch 有实验开关；Firebase Hosting 可配；**GitHub Pages「Does not seem to have intent to implement that」**；GitLab Pages 有个长期 open 的 issue。

### 2.2 「为什么我的构建在本地能跑，传上去就黑屏」

- 〔原话〕[Godot 论坛：Cross Origin Isolation and Shared Array Buffer missing](https://forum.godotengine.org/t/cross-origin-isolation-and-shared-array-buffer-missing/67518)（2024-06-18）：「This also happens when I run the index.html file in my browser (outside of Crazy Games). The game does run on the web when I run it from the remote debug option on Godot though. **However, it runs very poorly, it barely gets to 40fps and when running it normally it can get over 500fps.**」
- 〔原话〕[Stack Overflow, 2023-08-17](https://stackoverflow.com/questions/76924109/error-the-following-features-required-to-run-godot-projects-on-the-web-are-missi)（23,067 次浏览）：「I exported the godot project to HTML5 and after uploading to itch.io I got the error... I tried to export the project with different settings but still got 1 result.」被采纳的答案就一句：勾 itch 的 SharedArrayBuffer Support。
- 〔原话〕[Unity Discussions：Build loads into black screen on Itch.io or github pages, but works from 'Build and Run'](https://discussions.unity.com/t/build-loads-into-black-screen-on-itch-io-or-github-pages-but-works-from-build-and-run/851793/1)：「it loads without issue, but then displays a black screen and I can't figure out why... I have tried various publishing settings: with and without compression, with and without decompression fallback.」
- 〔原话〕[RPG Maker 论坛](https://forums.rpgmakerweb.com/threads/black-screen-html5-deployment-itch-io.81735/)，一位开发者自己踩完所有坑后写的清单：「**DO NOT ENCRYPT THE AUDIO.** Make sure your Zip file does not contain any empty directories or it will corrupt the file... Release it as Public, **THE JAVASCRIPT CODE IS NOT RUN unless it is released not as a draft.** Make sure you have set the resolution to something NOT AS ZERO.」
- **一整类「昨天还好好的」失败**（〔原话〕[itch.io 论坛, 2025-08-18](https://itch.io/t/5229127/games-have-suddenly-stopped-working-in-the-browser)，1,883 次浏览）：「All my games that work on browser have suddenly stopped working. **I have not updated the games at all either.** It shows a grey screen and nothing else.」原因是 pygbag 的 WASM 解释器是运行时从 GitHub 上取的、不在上传的文件里：「if the pygbag library updates and stops supporting an older version (for example, 0.8), **all games packaged with that version will stop working, even if you haven't touched anything.**」
- **平台方的标准回复**（〔原话〕leafo 关闭 [itchio/itch.io#1855](https://github.com/itchio/itch.io/issues/1855)，2026-02-04）：「Your game is crashing most likely, open the browser console to see if there are any error messages. Going to close this issue unless you can produce any information that suggests the issue is with itchio specifically.」

### 2.3 Unity Play 的上传可靠性

来自 [Cannot publish game - Network error](https://discussions.unity.com/t/cannot-publish-game-network-error-failed-to-transmit-date/1698924)（2025-12 起，1.3k 浏览 / 21 赞）和 [Publish to Unity Play Error in Tutorial](https://discussions.unity.com/t/publish-to-unity-play-error-in-tutorial/1702445)（2026-01 起，498 浏览）：

- 「i tried so many times. i restarted it, updated to 6.3, started the program as administor but it always show error. it says **'Curl error 55: Send failure: Connection was reset'** on the console」
- 「I am brand new to building a game and I am having issues where I can not get the game to upload to the web. I turned off VPN, Fire walls, and made sure everything was set to the right format and even made sure I had the latest version. **It keeps giving me a Network Error Unknown Error.**」
- SteveMoore92117（Jan 14）：「In California Unity Editor and Hub both whitelisted on Norton Firewall, gigE to the home, no VPN on, **the build plays locally, but cannot publish.**」
- inenai（Aug 21）：「Same, it's failing both by uploading from the editor and by uploading the zip file manually. I've been trying since yesterday with no luck. **Editor throws generic 500 server error**, and the web page uploads the file, you can see 'Success' for a moment and then an error prompt saying **'Invalid game build'**」
- 官方侧承认（Unity 论坛）：「There was a change to the networking library in the editor recently that caused the publish button to stop working. **We're working on it.**」

### 2.4 itch.io 的体积与文件数限制

- 〔原话〕[Web size limitation question](https://itch.io/t/2080897/web-size-limitation-question)：开发者「We have tried everything and last effort is to ask here... (We wrote to support mail 6x with only one reply)」；itch 方回复「If your game can not fit within those requirements then you'll have to provide a downloadable version to host it on itch.io.」
- 同帖里另一位用户为限制辩护：「Just had to force-restart my machine due to a web game that caused the browser to eat up all my RAM and start thrashing too quick for me to react. **Large games in a browser are a DDoS attack waiting to happen.** By the time your game bumps against the limits set by itch.io, it's already way too large to run in a browser.」
- 〔原话〕[1000 files limit on browsergame](https://itch.io/t/3964421/1000-files-limit-on-browsergame)：「I have combined graphics of the same size into sheets and reduced the number of files to about 500... **All in all, I think Itch is the [wrong] place for complex browser games.**」「The size limit is explained on the faq. **The file limit is not.**」

### 2.5 大陆用户

- 〔原话〕itch.io 官方论坛的中文求助帖（[topic/5661344](https://itch.io/topic/5661344)，2025-12-17，671 次浏览）。回帖的 NikoCat233：「由于中华人民共和国大陆政府单方面禁止了访问 itch.io，你需要一个代理服务访问 itch.io。……由于 itch 客户端强制不走 http 代理，你会看到 json-rpc 超时。您可以使用您代理的 tun 模式，这预期代理全部流量。」
- 〔二手〕B 站上「itch.io 打不开 / 进不去 / 网页打不开」已经形成一个视频门类，标题里反复出现「2026 保姆级」「解决进不去问题」「卡人机认证」（[示例](https://www.bilibili.com/video/BV1aKBHBGEgi/)）。**这说明需求真实存在且没有被满足**——有人愿意为了打开 itch 去看教程装加速器。

---

## 3. 引擎与浏览器的硬事实（决定「对游戏有感」到底指什么）

- **Godot 4 线程导出需要 SAB**，SAB 需要 `COOP: same-origin` + `COEP: require-corp` 且必须是安全上下文。〔一手〕[Godot 官方文档](https://docs.godotengine.org/en/stable/tutorials/export/exporting_for_web.html)：「the use of multiple threads for the Web platform has multiple drawbacks, including requiring specific server-side headers and complete cross-origin isolation (**meaning no ads, nor third-party integrations on the website hosting your game**)。」
- **Godot 4.3 起单线程导出回归**，官方明确推荐：「it doesn't require as much overhead... It is also **more compatible overall with stores like itch.io or Web publishers like Poki or CrazyGames**. The single-threaded export works very well on macOS and iOS too.」→ 〔推断〕`--isolated` 的实际需求量正在下降，但没有消失：Subversion Studios 的原话说明线程版能把加载从 30s 降到 5s 并消除音频卡顿；FopenP（297 天前）「I export with threads and use SharedArrayBuffer because I don't want glitches in my audio streams. I make 3D games」。
- **Godot 还有第三条路**：Web 导出的 PWA 选项「applies a service worker-based workaround that allows the project to run by simulating the presence of these response headers」。
- **Unity 的预压缩是个陷阱**。〔一手〕[Unity 手册](https://docs.unity3d.com/6000.7/Documentation/Manual/web-optimization-mobile.html)：「**Make sure your web server is configured to serve Brotli files with the correct encoding.**」并且「Chrome and Firefox only natively support Brotli compression over HTTPS」。→ 这正是 DESIGN.md §4.2 说的「服务器不配就白屏」。
- **Unity WebGL 在手机上不是一等公民**。〔原话，Unity 论坛资深用户 manuelgoellnitz〕「I see in that here in the forum many try to run their Unity-WebGL App on mobile browsers, **although this is not yet fully supported by Unity**.」〔二手〕iOS Safari 的 WebGL 堆上限约 300–500 MB，建议 Memory Size 设 256/384 MB。〔一手〕[Unity Issue Tracker](https://issuetracker.unity3d.com/issues/memory-usage-increased-in-newer-versions-when-using-safari)：新版本在 Safari 上内存从预期的 300–500 MB 涨到 700 MB–1.3 GB，在 iPhone 16 Pro / 13 mini / 12 Pro 上复现。
- **音频自动播放：两家引擎的官方文档都建议做一个「点一下开始」的页面**，这是对 DESIGN.md §3.3 最强的支持。
  - 〔一手〕[Unity 手册 · Audio in Web](https://docs.unity3d.com/6000.5/Documentation/Manual/webgl-audio.html)：「For security reasons, browsers don't allow audio playback until an end user interacts with your application webpage via a mouse click, touch event, or key press. **Use a loading screen to allow the end user to interact with your application and start audio playback before your main content begins.**」
  - 〔一手〕[Godot 官方文档](https://docs.godotengine.org/en/stable/tutorials/export/exporting_for_web.html)：「Some browsers restrict autoplay for audio on websites. **The easiest way around this limitation is to request the player to click, tap or press a key/button to enable audio, for instance when displaying a splash screen at the start of your game.**」
  - ⚠️ **但这里有一个 DESIGN.md 还没验证的技术前提**：浏览器的 user activation 是**按文档**记的。如果门禁页点击之后是导航到另一个文档、或者游戏跑在跨源 iframe 里，这次点击不一定会被当作游戏那一侧的用户手势。我在调研末段想核实 MDN 的 user activation 规范时搜索工具故障，**没验证**。这条必须进 `docs/spikes/`。

---

## 4. 商业模式：谁在赚钱，谁死了，怎么死的

### 4.1 收入结构一览

| 平台 | 钱从哪来 | 状态（日期） |
| --- | --- | --- |
| itch.io | Open Revenue Sharing，卖家自选 0–100%，默认 10%，支付通道费转嫁卖家 | 活着，但 2025-07 被支付通道逼着 deindex 全部成人内容，创作者收入受损、payout 被冻 |
| Newgrounds | 几乎全靠 Supporter 订阅，基本放弃广告 | 〔2025-08〕约 9,000 订阅、约 $27,000/月，**仍在亏损**；2026-04-06 首次涨价到 $5/月 |
| The Gaming Nest | 托管不收钱（只有 one-off 加额度档），**平台收入来自付费培训课程** | 活着，2 人，2026-02 正式版 |
| Unity Play | 免费，引擎的引流 / 教学漏斗 | 活着，但上传链路持续故障 |
| Kongregate | 最高 70% 分成 + 自营游戏与并购 | 2020 关闭投稿，**2026-04 带着「分析与社区工具」重新开放** |
| SIMMER.io | 免费 + SIMMERconnect 订阅 | **2025-04 死于 DDoS 引发的云账单**，宣称 2026 秋回归 |
| Glitch | Glitch Pro 订阅，2022 年被 Fastly 收购 | **2025-07-08 关闭托管** |
| Playset | 按录像分钟计费，$0 / $19 / $49 | 活着 |
| Playloop | SaaS，$0 / ~$49 / ~$99 | 活着 |
| tiiny.host / Surge / Neocities | 明确的订阅阶梯（$5–$31 / $30 / $5） | 活着——**唯一一组从第一天就按体积和流量收钱的** |

### 4.2 被关停 / 收缩的先例，以及原因

三个案例，死因高度一致：**免费托管用户内容 + 没有硬成本上限 + 滥用。**

1. **SIMMER.io（2025-04）**——DDoS 打出「hundreds of thousands USD」的云账单，创始人删掉自己的存储桶止血。2 人团队，做了 8 年。
2. **Glitch（2025-07-08）**——〔一手〕[官方公告](https://blog.glitch.com/post/changes-are-coming-to-glitch)；〔二手〕[The Verge](https://www.theverge.com/news/673457/glitch-coding-platform-shutting-down)、[The Register](https://www.theregister.com/off-prem/2025/05/23/glitch-hits-kill-switch-on-app-web-hosting/1463632)、[BleepingComputer](https://www.bleepingcomputer.com/news/security/glitch-to-end-app-hosting-and-user-profiles-on-july-8/)。CEO Anil Dash 原话：托管应用所需的时间和金钱「**has greatly increased as the platform has gotten older and bad actors try to misuse the platform**」。规模是「millions of users, running tens of millions of apps」，2022 年已被 Fastly 收购。官方博客另一句很重要：「Glitch's legacy architecture hasn't been providing something uniquely valuable to the developer ecosystem at this point」——**它不是被成本压死的，是被「不再独特」压死的**：公告里自己列出了 Fly.io、Deno、GitHub Pages、Val Town、Netlify、Digital Ocean 一串替代品。善后做得很体面：dashboard 和代码下载保留到 2025 年底，子域重定向要在 2025-12-31 前设好、承诺至少活到 2026 年底，新 Pro 订阅立即停售、已有订阅退款。
3. **Kongregate（2020-07）**——Flash 退场 + 战略转向自营，停投稿 + 裁员，128,655 款游戏留档。六年后带着开发者门户重开。

〔推断〕**给 playtest.run 的两条**：
- 免费额度必须有**双重上限**——月度配额挡长期滥用，**单位时间的流量熔断**挡 DDoS。SIMMER 是被分钟级烧钱打死的，月度配额根本来不及生效。
- Glitch 的死因提醒：一个托管产品的护城河不是「能托管」，是「有别人给不了的东西」。这恰好是 DESIGN.md §0 第 4 条（知道结果）和 §5（地理）的论证依据。

---

## 5. 核心产出：「上传 → 链接」这件事上「做得更好」到底是什么

### 5.1 打分表

评分：**✅** ＝做到且做得好；**◐** ＝做到但有明显代价或要额外步骤；**❌** ＝没做；**?** ＝一手信息不足 / 未知。
「playtest.run」一列是 DESIGN.md 的**设计目标**，不是已实现的事实，用「（设计）」标注以免混淆。

| # | 可度量的 UX 细节 | playtest.run（设计） | TGN | itch.io | Unity Play | Playset | Netlify Drop | Surge | tiiny.host | CF Pages | GitHub Pages | Final Parsec | ArcadeLab / rendrd 类 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | **免登录就能出第一个链接** | ✅ 24h 匿名链接 | ❌ 要账号 + 网页建 key | ❌ | ❌ | ? | ✅ 先部署后认领 | ✅ | ✅ | ❌ | ❌ | ✅ 插件免账号，事后 claim | ✅ |
| 2 | **一条命令从零走到可玩链接** | ✅ `playtest ./dist`（设计） | ❌ 官方文档 5 步，项目须先在网页存在 | ❌ butler 推完还要回网页改页面类型 + 勾「在浏览器里玩」 | ❌ 只有编辑器按钮 | ❌ 网页拖 zip | ◐ 拖拽即出链接，但要在网页 | ✅ `surge ./dir` | ◐ 网页 3 步 | ❌ 要 Git / wrangler + 配置 | ❌ | ◐ 编辑器一个按钮（仅 Unity） | ✅ 粘贴 / 拖一个文件 |
| 3 | **改一个文件重传：增量还是全量** | ✅ 按内容哈希只传变化（设计） | ❌ 每次 zip 整个目录 | ◐ 真 diff（rsync + bsdiff），但后端 processing **大构建约 30 分钟** | ❌ 整包 | ? | ❌ 整包 | ? | ❌ 整包 | ❌ 整包 | ◐ git 增量，但要 commit + Actions | ❌ base64 塞 JSON | ❌ 单文件 |
| 4 | **版本 + 一键回滚，URL 对玩家不变** | ✅（设计） | ✅ 10 版，一键激活/回滚 | ◐ channel 有 build 列表 | ❌ 只能 replace | ◐ builds 在一处 | ◐ 有部署历史 | ✅ immutable revisions + `surge rollback` | ❌ | ◐ 有部署历史 | ◐ git | ❌ | ◐ localgames 2 版 |
| 5 | **终端二维码，手机直接扫** | ✅（设计） | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 6 | **链接在微信 / Discord / X 里出卡片** | ✅ 门禁页带元信息（设计） | ◐ X 的 `twitter:player` 内联播放 | ✅ 项目页元信息完整 | ◐ | ? | ❌ 看你自己的 HTML | ❌ | ❌ | ❌ | ❌ | ? | ◐ 有游戏页 |
| 7 | **大陆能打开** | ✅ 香港边缘（设计，未验证） | ? | ❌ **GFW 整域封锁，自 2025-09-25 起连续 100%** | ◐ Unity 论坛明说「firewalled country... may not be able to upload」 | ? | ? | ? | ? | ? | ? | ? | ? |
| 8 | **`.wasm` 给 `application/wasm`** | ✅（设计） | ✅（宣称） | ✅ | ✅ | ✅ | ✅ | ? | ? | ✅ | ✅ | ? | n/a 单文件 |
| 9 | **`.br` / `.gz` 预压缩产物直接给** | ✅ 按 Accept-Encoding（设计） | ✅ `--precompress` + CDN 直给 | ✅ | ✅ | ? | ◐ | ? | ? | ◐ 要写 `_headers` | ❌ 只能用 Decompression Fallback | ? | n/a |
| 10 | **COOP/COEP 一个开关就对** | ✅ `--isolated`（设计） | ❌ **明说做不到** | ◐ 有勾选框，但副作用一长串（见 2.1） | ? | ? | ◐ 能写 `_headers` | ❌ | ❌ | ◐ 能写 `_headers` | ❌ **只能上 service worker hack** | ❌ | n/a |
| 11 | **玩家首屏：无广告、无跳转、同一 URL** | ✅ 门禁页同 URL（设计） | ◐ 游戏挂在 project page 上 | ◐ 项目页 + iframe，Safari 还要弹窗 | ◐ 平台页 | ✅ 直链 | ✅ 纯净 | ✅ 纯净 | ◐ 免费档带品牌角标 | ✅ | ✅ | ◐ 平台页 | ◐ 平台页 |
| 12 | **音频自动播放：托管方内建用户手势** | ✅ 门禁页顺手解决（设计，**待 spike**） | ❌ 让你自己加 splash | ❌ | ❌ | ? | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 13 | **移动端全屏 / 触屏可用** | ? DESIGN 未展开 | ? | ❌ 老师原话：不勾 SAB 就点不了全屏 | ◐ 有 mobile controls 开关 | ? | n/a | n/a | n/a | n/a | n/a | ? | ◐ 沙箱 iframe |
| 14 | **私密访问（口令 / 邀请名单）** | ✅ 三档（设计） | ? | ✅ 四档，且明确包装成 playtest | ✅ unlisted + 可选口令 | ? | ◐ 团队私有 | ❌ 免费档没有，$30/月才有 | ◐ $13/月起 | ❌ | ❌ | ❌ | ◐ rendrd 免费账号有口令 |
| 15 | **访问统计 / 知道结果** | ✅ 最小集（设计） | ? | ◐ 有基础统计 | ◐ | ✅ 全程录像 + 时间戳反馈 | ◐ 1 天保留 | ◐ `surge stats` | ◐ 付费档 | ◐ | ❌ | ❌ | ◐ plays / likes |
| 16 | **报错时告诉你为什么** | ? DESIGN 未展开 | ❌ 只有 FAQ 文章 | ❌ 「open the browser console」 | ❌ **「Network Error: Unknown Error」/ 裸 500 / 「Invalid game build」** | ? | ❌ 「may cause your deploy to get stuck」 | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 17 | **免费额度够不够一次 30 人的测试** | ✅ 20 GB/月（设计） | ◐ 存储 300 MB，流量未标明 | ✅ | ✅ 1 GB | ❌ **免费档只有 15 分钟/月** | ❌ **300 credits ≈ 20 次部署或 15 GB** | ✅ | ❌ **免费 3 MB** | ◐ 单文件 25 MiB 卡大构建 | ✅ | ◐ 5 个作品 | ◐ 500 KB–5 MB |

### 5.2 没有人做好的格子（这张表最重要的结论）

按「空得最彻底」排序：

1. **第 5 行 · 终端二维码——一整行全空。** 上传类产品没有一家做。这是 M1「手机上试一下」和微信分发的直接入口。（DESIGN.md §2.1 提到隧道类的 LocalhostVibe 做了，说明这个点子不新，但**上传这一侧确实是空的**。）
2. **第 16 行 · 报错时告诉你为什么——全行业最差的一格。** Unity Play 给「Network Error: Unknown Error」和裸 500；itch 的官方回复是「open the browser console」；TGN 写了一篇 FAQ 文章让你自己对照；Netlify 的文档只敢说「may cause your deploy to get stuck」。
   **具体到可以做的事**：上传的那一刻，服务端已经拿到了完整的文件清单和字节。**没有一家在这个时刻做过任何检查。** 能检查而没人检查的东西包括：Godot 导出物里 `.wasm` 是不是线程版（是的话直接告诉开发者「这个构建需要 `--isolated`，我帮你开还是你重新导出？」）；`index.html` 在不在根；`.pck` / `.data` 有没有漏；有 `.br` 但没有未压缩版本时提醒 HTTPS 的前提；单文件是否超过目标平台上限；Unity 的 Decompression Fallback 有没有关。**这一格如果做成，是「一条命令」之外最大的体验差距。**
3. **第 12 行 · 音频用户手势——托管方全空。** 两家引擎的官方文档都在教开发者自己写 splash screen。**没有一家托管方替开发者做掉。** DESIGN.md §3.3 的门禁页是唯一在设计里认领这件事的，但前提（user activation 跨文档是否保留）**还没验证**。
4. **第 10 行 · COOP/COEP——只有 itch 做了，而且做得很痛。** itch 的方案是全站 opt-in 开关 + 第二套 CDN + 四年长尾 bug（YouTube 嵌入、Safari 弹窗、Firefox token 过期、桌面 app 崩溃、口令 404）。TGN、GitHub Pages、Unity Play 都不做，CF Pages 要自己写 `_headers`。**「一个 flag 就对，而且不打坏别的东西」没人做到。** 注意 Godot 4.3+ 单线程导出正在削减这个需求——所以这一格的价值在下降，但对 3D / 有音频流的项目仍然成立（加载 30s → 5s）。
5. **第 3 行 · 改一行三秒上线——只有 itch 做了真增量，但代价是最长 30 分钟的后端 processing。** 其余全部整包重传。**「改一行代码，3 秒后手机上刷新就能看到」这个循环，市面上没有。** 这是 DESIGN.md §4.2 哈希增量最值钱的地方，比「上传快」这个说法更值得写成产品承诺。
6. **第 7 行 · 大陆可达——一整行是 ❌ 和 ?。** 唯一有硬数据的 itch.io 是 100% 封锁。这一格支持 DESIGN.md §0 第 5 条。

### 5.3 已经有人做好了、不要当卖点的格子

- **第 1 行 · 免登录出链接**：Netlify Drop、Surge、tiiny.host、Final Parsec、ArcadeLab、rendrd 全都做了。**这是行业常态。**
- **第 4 行 · 版本 + 回滚**：TGN 10 版一键回滚、Surge 的 immutable revisions + `surge rollback` 都很成熟。
- **第 14 行 · 私密访问**：itch 的四档（名单 / 口令 / 秘密 URL / 公开）比 DESIGN.md §3.6 的三档还全，而且已经明确包装成「Limited Playtests」。
- **第 2 行 · 一条命令**：Surge 早就是 `surge ./dir` 一条命令。playtest.run 在这一格的优势**只在于同一条命令还能走隧道、还带游戏响应头**，不在「一条命令」本身。

### 5.4 一个 itch 用血换来的教训，playtest.run 要提前设计

**访问控制和跨源隔离会打架。** itch 为了 Safari/Firefox 的 SAB 兼容改成弹窗打开游戏，结果口令保护的项目在弹窗里 404（Subversion Studios 原话，225 天前；DamienKusters 原话，2 年前，同一个 bug 隔了两年还在）。

playtest.run 的 `--isolated` + 口令 / 邀请名单是同一个组合。**门禁页的 cookie、一次性令牌，和 `COOP: same-origin` 的窗口隔离语义要在设计阶段就对齐**，不要等上线后发现「开了隔离的作品，加了口令就打不开」。

---

## 6. 对 playtest.run 的含义

### 6.1 支持 DESIGN.md 现有结论的发现

| DESIGN.md 的结论 | 支撑证据 |
| --- | --- |
| §2.3「上传路径不是差异化，是入场券」 | 完全成立，而且比 DESIGN 写的更红海——§1.6 那一整类 AI 单文件托管是 DESIGN 没数进去的 |
| §3.3 门禁页顺手解决音频用户手势 | Unity 和 Godot **官方文档都在教开发者做这件事**，而**没有一家托管方替他们做**。这是真痛点、真空位 |
| §4.2「服务器不配预压缩就白屏」 | Unity 手册明写要服务器正确配 Brotli encoding，且 Chrome/Firefox 只在 HTTPS 下原生支持 |
| §2.1「TGN 明说 SharedArrayBuffer 的构建跑不了」 | 确认。TGN 的 Godot 页原话：「We serve builds from a separate storage origin without those headers, so a thread-enabled Godot export will load and then fail.」（注意它另一篇文章的营销文案写反了） |
| §5「大陆是缺口」 | **比 DESIGN 说的更强。** itch.io 不是「慢且不稳」，是 GFW 整域封锁：DNS 污染 + 静默丢包，自 2025-09-25 起每次测试 100% 屏蔽（GreatFire，最近测试 2026-08-30）。B 站上「itch 打不开怎么办」已经是一个视频门类 |
| §6「免费档给得慷慨但有上限」 | SIMMER.io 和 Glitch 两个死亡案例直接证明。TGN 把这条写成了自己的营销文案：「Free hosting without a hard per-account ceiling is not a business model — it is an open invoice」 |
| §8 生死线要看留存而不是注册 | Newgrounds 的数字：25 年品牌、约 9,000 付费支持者、约 $27k/月，**仍然亏损**。这个品类没有轻松的现金流 |
| §4.8「我们从不运行用户代码」 | Glitch 死因之一就是「bad actors try to misuse the platform」，而它是**跑用户代码**的。不跑代码把滥用面砍掉一大半，这条判断是对的 |

### 6.2 与 DESIGN.md 冲突、建议修改的地方

**① §3.2 的「免登录 24 小时匿名链接」不该当卖点。**
现状：DESIGN 把它写成「免登录是 M1 的要求」的产品设计亮点。
事实：Netlify Drop（先部署后认领）、Surge（首次发布时才建账号）、tiiny.host、**Final Parsec（Unity 插件里一个按钮，事后 claim）**、ArcadeLab、rendrd 全都免登录出链接。
**建议**：保留这个功能（它确实必要），但在 §2.3 的「入场券 / 差异化」分类里把它挪到入场券一侧，对外文案不要拿它当亮点。

**② §2.2 对 The Gaming Nest 的定位需要重写，那句自我设限尤其要删。**
现状：「The Gaming Nest：上传路径几乎就是我们 §4.2 的样子，还先做了……上传这一路我们**不会比它更好多少**。」
事实：TGN 是约旦 2 人团队的**阿拉伯语游戏社区**，WebGL 托管是获客功能，平台货币化靠培训课程；HN / PH / Reddit 上找不到任何发布帖，LinkedIn 互动个位数。技术上它是**整包 zip 重传**、**要先在网页建 API key 和 project**、**deploy 完还要手动翻 Publish 开关**、**明确不做 COOP/COEP**。
**建议**：把 TGN 从「离我们最近的三家」里降级为「同形态的小团队案例」，并**删掉「不会比它更好多少」这句**——哈希增量 + 一条命令到可玩链接，对比它的 5 步流程，是可度量的、明显的更好。DESIGN.md §9 的自我提醒（「赢是因为在同一件事上做得比别人好」）在这里正好适用：**上传这一路值得认真做到最好，而不是当作已经输掉的入场券。**

**③ §2.1 漏了两类竞争者，必须补。**
- **AI 单文件 HTML 托管群**（§1.6）：AIGameShare 和 rendrd **都已经有 MCP server，AI agent 可以直接发布并把链接递回聊天里**。这打的正是 §1.1 排第一的「用 AI 写小东西的人」。DESIGN 只列了 PlayFeed 一家。
- **Surge**：`surge ./dist` 一条命令、免登录、immutable revisions、`surge rollback`、`surge stats`——几乎就是 §3.2 + §3.5 的形状，已经存在多年。§2.2 必须能正面回答「我们和 surge 差在哪」（答案是：游戏响应头、门禁页、结果、香港、以及同一条命令能走隧道）。

**④ §2.2 / §2.3 第 2 条对「轻量结果」的判断需要修正。**
现状：「『知道结果』这一层要做得比 Playset **轻**而不是更全：不录像、不加 SDK 就有第一层数据，是我们的角度。」
事实：Playloop 的**免费档**已经给了 event timeline、session replay、live dashboard、5 GB 存储、30 天历史、2 个席位、以及 **Ask Playloop + MCP（直接和自己的玩家数据对话）**，并且明确「Playloop ingests event streams, **not video**」。**「比 Playset 轻」这个位置已经有人占了，而且是免费占的。**
**建议**：把角度从「更轻」改成「**在同一条命令里**」——playtest.run 的结果层不需要开发者去另一个产品上传第二次、不需要装 SDK 就有第一层数据、并且和交付（上传 / 隧道）、版本、门禁页是同一件事。这是 Playloop 和 Playset 都给不了的，因为它们不做交付。

**⑤ §3.6 的「谁能玩」三档是入场券，且要提前解决与 `--isolated` 的冲突。**
itch 的四档访问控制更全，还明确包装成「Limited Playtests」。更重要的是 §5.4 那个教训：**口令 + 跨源隔离会互相打架**，itch 上这个 bug 活了两年。建议在 §3.6 或 §4.2 里加一句设计约束。

**⑥ §3.3 门禁页解决音频的承诺，需要一条 spike 才能写成事实。**
浏览器的 user activation 是按文档记的。门禁页点击之后如果是导航到新文档、或者游戏在跨源 iframe 里，这次手势不一定带得过去。按 AGENTS.md 第 3 条，DESIGN 里这句现在写得像已经成立。
**建议**：在 v0.1 第一周做一条 `docs/spikes/` 记录，在 Chrome / Safari / 微信 X5 三处各验一次（Godot 4 sample 播放模式和 Unity 的 AudioContext 行为可能不同），验证不过就改门禁页的实现方式（例如游戏与门禁页同文档、用 DOM 替换而非导航）。

**⑦ §6 的免费额度只有月度维度，挡不住 SIMMER 的死法。**
现状：「每账号每月 20 GB、3 个活跃 slug、保留最近 5 个版本、每个 slug 并发 50 人」。
事实：SIMMER.io 是被**分钟级**的 DDoS 烧死的，创始人有 billing alert 也没来得及——最后只能删存储桶止血。月度配额在这个时间尺度上完全无效。
**建议**：§6 加一条「**每 slug 每小时流量上限 + 自动熔断（超限直接把 slug 转成静态说明页，而不是继续出流量）**」，并且在 §4.5 里说明熔断是在边缘本地判定的，不依赖控制面回源——因为控制面挂了或者被打满时，正是最需要熔断的时候。另外对比一下别人的免费档，20 GB/月是有竞争力的：Netlify 新免费档约合 15 GB 且 20 次部署就用光，tiiny.host 免费 5 GB。

**⑧ §4.2 建议补一条 Unity 的现实检查。**
Unity 的预压缩有两条互斥的路（服务器配 Content-Encoding vs. 关掉服务器配置改用 Decompression Fallback）。CLI 在上传时能看出是哪一种。建议 §4.2 加一句：检测到 Unity 导出物时，若发现 `.br` 但 HTML 里启用了 Decompression Fallback（或反之），在终端里明确指出，而不是让开发者面对白屏。这属于 §5.2 第 2 格「没有人做好的格子」的具体兑现。

**⑨ 关于创始人的三个判断。**
- **判断 1（方便简洁、体验第一）——调研强烈支持。** 第 5 节整张表里，所有产品失分最多的都是体验细节，不是能力缺失。而且「体验好」在这个品类里有一个非常具体的定义：**上传时就告诉你为什么跑不起来**（第 16 行）、**改一行三秒上线**（第 3 行）、**手机上一秒能打开**（第 5 行）。这三件事都是可度量的，都没人做好。
- **判断 2（要不要做反馈 / 曝光 / 孵化生态）——调研倾向于「不要」，但反馈要做。** 曝光和孵化是 Newgrounds 和 Kongregate 的路，前者 25 年后仍在亏损，后者关了六年才回来。DESIGN.md §3.7「不做发现、榜单、商店」这条应当坚持。但**反馈（§3.4）要做**——Kongregate 2026 年重开的卖点就是它，Playloop 免费档也在做它。
- **判断 3（要不要做数据分析）——做，但要守住 §3.7 的边界。** 证据两面：一面是 Kongregate 拿它当重返市场的旗帜、Playloop 免费档就给 session replay 和 MCP；另一面是 Playset 靠录像收费、PlaytestCloud 卖企业价，说明「更深的分析」是另一门生意。**建议按 §3.4 的最小集做，但把「这一版有没有坏」而不是「哪里流量好」当作组织原则**——前者是 M3 的问题（错误、加载失败、卡在哪、一句话反馈），后者是 M4 的问题，DESIGN.md 已经明确不做 M4。「哪里流量好」需要漏斗和热图，那条路通向通用分析平台，是 §3.7 排除掉的。

---

## 7. 没查到 / 没验证的（不要当成结论用）

- **The Gaming Nest 的付费档价格**——只找到「one-off paid tiers raise the quota」的措辞，没有定价页。
- **PlayFeed 的免费额度、体积上限、付费档、团队背景、是否有 CLI**——官网 `/create` 页没写，也没找到定价页。
- **TGN 和 PlayFeed 的发布帖及评论**（HN / Product Hunt / Reddit / X）——都没找到。这本身是一个信号（说明传播极小），但不能证明不存在。
- **Vercel 的免费额度与对 `.wasm` / 预压缩的处理**——调研末段搜索工具连续故障约 20 分钟，未取得一手定价页。
- **各家的实测数字**：首个链接用时、玩家首屏时间、重传耗时——**全部没有实测**，表格里的判断是从官方文档的步骤数和限制推出来的。真要用这些数字对外说话，需要 `docs/spikes/` 里的真机记录。
- **大陆可达性**：只有 itch.io 有 GreatFire 的硬数据，Unity Play 只有一条社区提及。TGN、Netlify、Surge、tiiny.host、Cloudflare Pages、Playset 的大陆可达性**都没查**。
- **微信里这些链接的分享卡片和 X5 内核表现**——完全没有一手数据。
- **user activation 是否跨文档 / 跨源 iframe 保留**——想核实 MDN 规范时工具故障，**未验证**。这条直接影响 §3.3 的核心承诺。
- **SIMMER.io 云账单的具体金额**——「hundreds of thousands USD」来自一位受影响用户的转述（salahkouhen.com），创始人自己的公告只说「astronomical cloud bills」，没给数字。
- **itch.io 的财务状况**——不公开，只能从「开放分成 + 默认 10% + 费用转嫁卖家 + 2025 年被支付通道拿捏」推测其脆弱性，没有营收数据。
- **itch.io、GameJolt 的访问统计到底给开发者看什么**——只确认「有基础统计」，没看到具体字段。
- **Newgrounds 的「undisclosed loss」到底多大**——wiki 标注为需要引用，没有一手来源。
