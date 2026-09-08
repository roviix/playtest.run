# playtest.run 商业模式、免费档与死亡案例研究

研究日期：2026-09-07

## 0. 结论先行

1. **“方便简洁、使用体验好”不仅是第一位，而且是这个品类最可靠的获客机制。** Railway 取消免费档后增长骤降，后来恢复一个很小但能完成首次体验的免费额度；Cloudflare、Expo、Unity 则把免费工具当作更大生态的入口。它们共同说明：免费档的首要任务不是长期补贴，而是让用户完整经历一次“发布—分享—得到结果”。[[一手：Railway]](https://blog.railway.com/p/free-plan) [[一手：Cloudflare]](https://blog.cloudflare.com/cloudflares-commitment-to-free/) [[一手：Expo]](https://expo.dev/pricing) [[一手：Unity Play]](https://play.unity.com/en/faq)
2. **曝光、榜单、孵化不是现在应补的“生态”。** itch.io、Unity Play 已经免费提供发布与发现；Antidote 甚至把曝光作为 Indie 计划的附赠权益，但真正能收高价的是招募、录像、研究设计、安全与能改变产品决策的结果。playtest.run 应继续服务“把指定的人带到指定版本”，而不是争夺流量市场。[[一手：itch.io Jam]](https://itch.io/docs/creators/game-jams) [[一手：Antidote 定价]](https://antidote.gg/pricing/) [[一手：PlaytestCloud 定价]](https://www.playtestcloud.com/pricing)
3. **数据值得做，但应是“测试结果工作流”，不是通用网站分析。** 有付费证据的是版本比较、加载与错误、录像/反馈归档、目标玩家筛选、团队审阅和导出；普通 PV、漏斗、热图已有大量免费或低价替代。PlaytestCloud 的 `€1,025–2,825/月` 与 Antidote 的 `$115–475/游戏/月` 买的是研究能力，不是多一张访问量图。[[一手：PlaytestCloud]](https://www.playtestcloud.com/pricing) [[一手：Antidote]](https://antidote.gg/pricing/)
4. **DESIGN §6 的“30 MB × 1,000 次 ≈ 30 GB ≈ $3”在 `$0.10/GB` 假设下算术正确。** 但协议开销、重传、源站回源、请求和存储可能把它推高；使用亚洲 CDN 又可能降到约 `$0.90–2.43`。价格不是最大未知数，**免费用户的实际额度利用率、恶意流量和中国大陆实测可达性**才是。
5. **20 GB/月适合小规模邀请制验证，不适合作为未经观测的公开承诺。** 1,000 个免费账号满额会产生约 20 TB；按 `$0.10/GB` 即 `$2,000/月`。建议公开免费档先定为 **10 GB/月硬上限**，匿名链接 **1 GB/24 小时**；没有信用卡、没有自动超额费、达到上限即透明硬停。
6. **第一付费档应卖“认真、私密、对外专业的测试”，而不是单卖流量。** 建议 `$15/月` 或 `$150/年`：固定/可改 slug 与去角标、口令/邀请名单、180 天结果留存与版本比较/导出，含 50 GB。额外流量只卖预付包，例如 50 GB / `$10`，不默认后付费。
7. **CLI/SDK Apache-2.0、边缘与控制面 AGPL-3.0 的方向可保留。** AGPL 能要求网络服务公开其修改，不能阻止别人不修改代码而合规托管；真正护城河仍是香港网络、域名信誉、滥用处置和整合体验。必须明确引擎插件不受 AGPL 影响，并避免后续抽走社区版能力。

## 1. 口径、证据等级与局限

- **[一手]**：官网、官方文档、官方定价页、公司博客、财报或创始人访谈。
- **[用户原话]**：论坛、评论和账单事件中的用户陈述；只能证明“有人如此经历”，不能代表总体。
- **[二手]**：媒体、数据库或他人整理；不与一手来源等同。
- **[计算]**：基于明确假设的算术。
- **[推断]**：本报告从证据推出的产品判断，不伪装成已验证转化率。

本次进行了超过 30 次搜索/抓取，优先使用 2025–2026 页面；历史价格页、动态渲染页和私营公司财务经常不可完整访问。**没有公开数字的地方写“没查到”，没有把定价页上的功能分层当成真实付费转化率。** 所有价格均为页面在 2026-09-07 可见或搜索索引可核验的标价，不含税；美元、欧元和英镑不换算，以免混入汇率假设。

## 2. 托管、隧道与开发者工具案例

### 2.1 横向表

| 产品 | 定价与免费档演变 | 主要转化杠杆 | 公开规模/资金 | 反弹与教训 |
|---|---|---|---|---|
| **ngrok** | [一手] 当前免费档 1 GB/月、20,000 HTTP 请求、最多 3 个 endpoint，并显示 interstitial；Hobbyist `$10/月` 起，付费去 interstitial，并增加自定义域、团队和更长日志留存。[[定价]](https://ngrok.com/pricing) 2024 年起，免费 TCP endpoint 因恶意软件滥用要求绑卡验证。[[官方说明]](https://ngrok.com/blog/tcp-endpoints-require-verification) **2023–2024 每个历史套餐的完整价格阶梯没查到可靠存档，不能声称完整还原。** | 去门禁、自定义/固定域名、更多流量、团队、日志留存与安全策略。 | [一手] 2022 年融资 `$50M`；当时称 500 万开发者、3 万付费客户、每天新增约 4,000 名开发者、收入同比翻倍。未披露可信美元收入。[[融资稿]](https://ngrok.com/press-releases/ngrok-raises-50m-for-ingress-as-a-service) 2024 官方称已有 700 万开发者。[[官方博客]](https://ngrok.com/blog/tcp-endpoints-require-verification) | [用户原话] 2022 年有用户称所需能力从 `$9` 涨到 `$24`；另一讨论称约 `$25` 的起价促使其转用免费 Cloudflare Tunnel。[[HN 讨论一]](https://news.ycombinator.com/item?id=31773570) [[HN 讨论二]](https://news.ycombinator.com/item?id=33968967) 教训是免费入口可收紧，但价格跳级会直接把轻度用户交给免费替代品。 |
| **Cloudflare Tunnel / Pages** | [一手] Tunnel 免费；Pages、Workers 等也以大额免费量作为开发者入口。Cloudflare 解释免费服务能利用网络闲置容量，并换来更多网络覆盖、威胁情报、产品测试和企业内部扩散。[[Tunnel for everyone]](https://blog.cloudflare.com/tunnel-for-everyone/) [[免费策略]](https://blog.cloudflare.com/cloudflares-commitment-to-free/) | 不是靠 Tunnel 本身去角标收费，而是把用户带入 Zero Trust、Workers、R2、企业安全与付费支持。 | Cloudflare 是上市公司，但**没有把 Tunnel/Pages 单独收入拆出**；公司总收入不能当作免费 Tunnel 可独立成立的证据。 | [推断] 它的全球网络、对等互联和企业交叉销售是 playtest.run 不具备的补贴条件。不能据此推出“免费无限流量可持续”。 |
| **Vercel** | [一手] 2024-04 将原 `$0.40/GB` 带宽拆为 Fast Data Transfer `$0.15/GB` 和区域性源站传输，官方示例称总体降低；Hobby 仍有免费额度，超额暂停。[[2024 调价]](https://vercel.com/blog/improved-infrastructure-pricing) 2025-09 Pro 改为 `$20` 的灵活信用额、免费 Viewer、默认成本控制；官方称约 7% 团队会涨价，10 万多个团队价格不变或下降。[[2025 Pro 调整]](https://vercel.com/blog/new-pro-pricing-plan) | 部署体验、团队协作、构建/函数/带宽用量、观测、安全和企业能力。 | [一手] 2024 年 Series E 融资 `$250M`、估值 `$3.25B`。[[官方融资公告]](https://vercel.com/blog/vercel-raises-series-e) 未找到经审计的独立收入数字。 | [用户原话/二手事件] 社区多次出现爬虫、函数失控或构建导致数百美元账单的投诉。即使个案最终退款，核心伤害是“不知道一次分享会花多少钱”。本报告不把未能稳定复核链接的个案金额用于模型。 |
| **Netlify** | [一手] 2025-09-04 起，新账户改为信用点：Free 300 credits，带宽 20 credits/GB，若只使用带宽约等于 15 GB；达到额度硬停，不收超额费。Personal `$9/1,000 credits`，Pro `$20/3,000 credits`。[[官方文档]](https://docs.netlify.com/manage/accounts-and-billing/billing/billing-for-credit-based-plans/credit-based-pricing-plans) | 构建、带宽、函数、团队、企业安全；新模型强调统一预算与硬上限。 | [一手] 2021 年 Series D 融资 `$105M`。[[公司公告]](https://www.netlify.com/press/netlify-raises-105-million-to-transform-development-for-the-modern-web/) 未找到 2026 经审计收入。 | [用户原话] 2024 年静态站异常流量出现 `$104,500` 账单，CEO 后来表示不会收取。[[HN 原帖与跟进]](https://news.ycombinator.com/item?id=39521986) 2025 的硬停政策可视为对“无限账单风险”的产品修正。 |
| **Heroku** | [一手] 2022-11-28 终止免费 dyno、免费 Postgres 和 Redis；官方理由包括欺诈滥用，并称要集中资源服务关键客户。[[官方公告]](https://www.heroku.com/blog/next-chapter/) | 从免费开发/教学入口转向付费生产工作负载和 Salesforce 企业客户。 | Salesforce 不单独披露 Heroku 当前收入；**没查到可归因的 2022–2026 独立数字。** | [用户原话] 社区认为这破坏了学生、教程和小项目的 onboarding；有自称每月为 Heroku 支付 `$10k–20k` 的客户也表示会重新评估。[[HN 讨论]](https://news.ycombinator.com/item?id=32594533) 免费用户本身不付费，但会影响工具在组织内的默认选择。 |
| **Glitch** | [一手] 2025-07-08 停止项目托管。官方明确提到运行数百万应用的成本、恶意使用，以及旧架构已不再有独特价值。[[官方公告]](https://blog.glitch.com/post/changes-are-coming-to-glitch) | 曾依赖免费即开即用、社区 remix 和协作；后期无法把这些优势转成可持续托管。 | 没查到关闭前可核验的托管收入或免费转付费比例。 | 这是最接近 playtest.run 的双重警告：**免费托管成本会杀死服务；即使成本可控，若“一个链接”已被通用平台商品化，也会失去复用理由。** |
| **Railway** | [一手] 官方复盘称早期最坏时每获得 `$1` 收入会亏 `$16`；取消免费档后增长“fell off a cliff”。2025 恢复为首月 `$5`，此后每月 `$1`、不滚存，目标是完成首次体验而非补贴长期生产。[[官方复盘]](https://blog.railway.com/p/free-plan) | 极短部署路径、可预测用量和从试用到生产的连续性。 | 该复盘给了单位经济比例，但未给可核验收入；本次未找到公开免费转付费率。 | 免费档不必慷慨，**但必须足够完成首次价值闭环**；完全取消入口的损失可能大于节省的资源。 |
| **Replit** | [一手] 2025 的 effort-based pricing 按 Agent 工作量收费。官方复盘承认发布质量未达标、约 6% 付费用户被错误计费并退款，另向受影响用户补 `$10`。[[官方复盘]](https://replit.com/blog/effort-based-pricing-recap) | AI 生成价值、算力、协作和部署；但“工作量”对用户不透明。 | 本次没有找到可审计的 2026 收入；媒体估算不采用。 | 用户不是绝对反对用量计费，而是反对**不可预测、不可验证、出错后才解释**。playtest.run 不应默认按玩家流量后付费。 |
| **Fly.io** | [一手] 2024 后的新用户不再有永久免费档，只有短试用；老账户保留旧的免费 allowance，例如若干小 VM 和区域流量。[[官方定价文档]](https://fly.io/docs/about/pricing/) | 全球部署、低延迟、资源用量；从免费爱好者入口转向付费工作负载。 | 未找到按免费/付费拆分的公开转化率。 | [推断] “grandfather” 老用户比突然清零更能保护信任，但会形成长期计费复杂度。 |
| **Surge** | [一手] 免费档提供无限发布、自定义域和基础 SSL；Professional `$30/月` 主要卖口令、重定向、CORS、自定义证书等。[[定价]](https://surge.sh/pricing) | 私密性、规则控制和高级证书，而非自定义域。 | 没查到可信收入、融资或用户数。 | 它直接反驳“自定义域天然能单独收费”：若替代品免费给，自定义域只能是组合价值的一部分。 |

### 2.2 从这些案例提炼出的免费档规律

1. **免费档要买到完整的第一体验，不要买到长期生产。** Railway 的 `$1/月` 和 ngrok 的 1 GB 都很小，但足以让人验证产品；playtest.run 的免费额度应围绕“一轮真实测试”而不是“永久免费托管”设计。
2. **硬上限优于意外账单。** Netlify 新信用点模式、Vercel 默认 spend controls 都朝这个方向移动。开发者可以接受测试停止，难以接受分享给朋友后收到未知账单。
3. **滥用是产品约束，不是之后再处理的运营问题。** ngrok 免费 TCP 绑卡、Heroku 关闭免费资源、Glitch 关闭托管都把滥用列为重要原因。匿名链接必须有单链接总流量、速率、文件类型和时间限制。
4. **大厂免费不代表小产品也应免费。** Cloudflare 用全球网络闲置容量和企业交叉销售补贴；Unity 用免费发布维持引擎生态；itch.io 从创作者销售分成。playtest.run 当前没有相同补贴面。
5. **改变承诺比价格本身更伤信任。** 因此一开始就应写“额度可能按实测调整，但已生成链接在 24 小时内按创建时额度执行”，并尽量对老付费用户保留原条款。

## 3. Playtest、游戏托管和“分享一个东西”

### 3.1 直接与邻近产品

| 产品 | 2026-09-07 可核验定价/免费档 | 真正收费点 | 对 playtest.run 的含义 |
|---|---|---|---|
| **Playset** | [一手搜索索引；页面需 JavaScript] 免费约 15 分钟试玩/月、1 个游戏、1 GB；Starter `$19/月` 约 10 小时，Studio `$49/月` 约 50 小时/3 个游戏。[[官网]](https://playset.app/) 动态页面正文未能抓取，因此这些数字应在采购或引用前再次人工核验。 | 按实际试玩分钟、游戏数和更高容量收费。 | “玩家真的玩了多久”比原始下载 GB 更贴近价值，但计量和争议也更复杂；v0.1 不宜复制分钟计费。 |
| **The Gaming Nest** | [一手] 每账户 300 MB 免费存储、每项目保留 10 个版本、500 MB 上传包上限、全球 CDN，并明确设置硬字节上限以避免免费托管消失。页面称有一次性付费扩容，但未列稳定套餐价格。[[官方页面，更新于 2026-07-12]](https://thegamingnest.com/en/webgl-hosting) | 扩大存储配额；同时有项目页、发现和 jam。 | 它与 playtest.run 的上传链路高度接近，并免费给版本管理。仅“上传并得到链接”不够形成付费差异；中国可达性、门禁、轻结果和 CLI 体验必须更好。 |
| **PlaytestCloud** | [一手] Mobile/PC Professional Basic `€1,025/月`、Advanced `€2,240/月`；全平台 Basic `€1,130/月`、Advanced `€2,825/月`，均按年开票；Enterprise 询价。免费试用含 2 个 Video Tokens。[[定价]](https://www.playtestcloud.com/pricing) | 招募、目标人群、试玩录像、问卷、AI 分析、儿童/多人测试、研究服务、SSO 与合规。 | B 端预算确实存在，但买的是“可靠研究结果”。Kwalee 自报一次测试避免约 `£300k` 开发成本，是 ROI 叙事，不是一般客户平均值。[[客户案例]](https://start.playtestcloud.com/case-studies/kwalee) |
| **Antidote** | [一手] BYOP 免费试用 14 天；Starter `$115/游戏/月`，Master `$475/游戏/月`；Indie `$0`，有发行商/融资的 Indie Plus `$99.95/月`。[[定价]](https://antidote.gg/pricing/) | 无限版本/邀请、录像与 think-aloud、转录、情感/AI 分析、流式运行、团队、定向招募和研究服务。 | 它把“有发行商/投资”直接当作付费能力分界，支持先瞄准**认真测试的小团队**，而非所有爱好者。 |
| **Unity Play** | [一手] 公开、Unlisted 和私有发布免费，Unlisted 可加口令。[[FAQ]](https://play.unity.com/en/faq) | 不以托管直接收费，而是服务 Unity 创作者生态。 | 口令和托管本身已有免费替代；playtest.run 的私密付费点应是邀请名单、审计、版本和结果工作流，而不只是一个 password 字段。 |
| **itch.io** | [一手] 创作者可把平台分成设为 0–100%，默认 10%；创建 game jam 免费，当前 jam 文档页显示已有 555,169 个 jam 游戏。[[支付]](https://itch.io/docs/creators/payments) [[Jam]](https://itch.io/docs/creators/game-jams) | 游戏销售分成、捐赠和成熟的发现/社区，而非按托管订阅。 | 没查到可信、经审计的平台收入。2025 年支付商压力造成成人内容紧急下架，说明一旦做发现/内容市场，就会继承支付与治理风险。[[官方更新]](https://itch.io/updates/update-on-nsfw-content) |
| **tiiny.host** | [一手] 免费档是 1 个小项目并带品牌角标；付费逐级提供去角标、独占子域、口令、自定义域、分析、团队和更大容量。[[官网]](https://www.tiiny.host/) 动态页面未稳定返回完整 2026 价格，因此本报告不引用未复核的具体美元档位。 | “分享后看起来专业”、隐私、域名、分析和团队。 | 这是 playtest.run 最接近的付费包装参照；但其 ARR/用户数只见于公司新闻稿和二手报道，本次没找到可独立核验财务，故不采用。 |

### 3.2 录屏、网页与移动构建分享

| 产品 | 免费入口与付费分层 | 可观察的转化杠杆 |
|---|---|---|
| **Carrd** | [一手] Pro Lite `$9/年`：3 个站点、去 `Made with Carrd`；Pro Standard `$19/年`：10 个站点、自定义域、表单和分析；Pro Plus `$49/年`：25 个站点、口令、下载源码等。[[官方套餐]](https://carrd.co/docs/pro/plans) | [一手访谈] 创始人称 `Made with Carrd` 带来增长，Black Friday 降价时转化会 “shoots through the roof”；访谈时约 `$100k MRR`、260 万用户、近 400 万站点。[[SaaS Club 访谈]](https://saasclub.io/podcast/carrd-aj-306/) 这是本次少数接近“真实转化杠杆”的公开证据，但仍没有各功能的独立转化率。 |
| **Loom** | [一手] 免费 25 个视频、每段 5 分钟；Business `$18/人/月`，提供无限视频/时长、去品牌、口令、互动分析和团队空间。[[定价]](https://www.loom.com/pricing) | 免费分享页传播品牌；付费把“偶尔发一段”升级为“团队的正式沟通资产”。2023 年 Atlassian 以 `$975M` 收购，报道当时约 2,500 万用户、每月约 500 万段视频；该规模不等于付费质量，媒体也质疑免费用户占比过大。[[TechCrunch]](https://techcrunch.com/2023/10/12/atlassian-to-acquire-former-unicorn-loom-for-975m/) |
| **Screen Studio** | 官网定价页在本次抓取中未稳定返回；**2026 当前价格、云分享限制和公开收入均没查到可复核一手数字**。[[官网定价]](https://screen.studio/pricing) | 可确定的产品价值主要是高质量录屏、自动镜头与编辑/导出，而非低成本托管；不能拿它证明“去角标一定转化”。 |
| **CleanShot X / Cloud** | 官方页面显示桌面工具与 Cloud 分享组合，但动态价格未稳定返回；第三方常见的 `$29` 买断、Cloud Pro 约 `$8/月` 等数字未获本次一手复核，因此不作为结论依据。[[官网定价]](https://cleanshot.com/pricing) | 可核验的包装方向是云空间、链接控制、口令、自毁、自定义域、白标和团队；它支持“分享资产的隐私与专业呈现可收费”，但没有公开功能级转化率。 |
| **Expo EAS** | [一手] Free：每月 15 次 Android + 15 次 iOS build、1,000 MAU、100 GiB edge bandwidth；Starter `$19/月`，Production `$199/月`。Starter/Production 的 Update 超额带宽均为 `$0.10/GiB`；付费增加高优先队列、并发、观测、SSO 与支持。[[定价]](https://expo.dev/pricing) | 免费 SDK/工具扩大生态，云端构建速度、并发、发布规模和团队可靠性收费。它的 100 GiB 免费量有生态交叉销售支撑，不宜直接照搬。 |
| **TestFlight** | [一手] Apple 提供最多 10,000 名外部测试者，开发者通常需加入 `$99/年` 的 Apple Developer Program；安装仍受 Apple 平台、审核和原生包流程约束。[[TestFlight]](https://developer.apple.com/testflight/) [[Developer Program]](https://developer.apple.com/programs/whats-included/) | 它不是浏览器零门槛体验；替代品卖的是更快分发、过期控制、设备信息和安装统计。 |
| **Diawi** | [一手] 免费账户 50 MB、3 天、每包 10 次安装；Starter `€2.99/月`，Premium `€29.99/月`，Enterprise `€299.99/月`。口令免费；付费卖更大包、更多安装、更长过期、通知/统计、历史、自定义页面/域名和团队。自定义域附加项 `€4.29/月`。[[官方定价]](https://www.diawi.com/features-services) | “过期时间、安装次数、统计与品牌页面”比口令更稳定；这支持把保留期和结果工作流纳入付费包。 |
| **InstallOnAir** | [一手] FAQ 当前称服务免费，登录用户链接 8 天、访客 2 天，且允许无限构建；站内其他页面出现“60 天”描述，互相矛盾，**未验证哪条为准**。[[FAQ]](https://www.installonair.com/faq) | 说明原生测试包分发也存在免费替代；其商业模式和收入没查到。 |

## 4. 开源核心与托管服务如何共存

### 4.1 项目对照

| 项目 | 许可证与托管方式 | 已知商业结果 | 贡献/转化证据 |
|---|---|---|---|
| **frp** | Apache-2.0，自托管反向代理，无官方同名托管主业务。[[GitHub]](https://github.com/fatedier/frp) | 约 109k GitHub stars（2026-09-07 页面量级），但没有对应托管收入。 | GitHub contributor 图未披露贡献者是自托管用户、雇员还是其他使用者；**巨大采用不等于自然产生云收入**。没查到自托管用户的付费转化率。[[贡献者图]](https://github.com/fatedier/frp/graphs/contributors) |
| **ngrok** | 托管网络和商业控制面是产品核心，免费客户端接入其网络；不是“下载完整 ngrok 云自行运营”的模式。 | 见 §2：500 万开发者、3 万付费客户（2022），粗略账户付费率约 0.6%，但企业账户口径、活跃度和席位不明，不能当 SaaS 转化基准。 | 这是网络、域名信誉和运营能力构成护城河的强例。 |
| **cloudflared** | 客户端 Apache-2.0，Cloudflare Tunnel 托管服务免费。[[GitHub]](https://github.com/cloudflare/cloudflared) | 通过 Zero Trust、网络、安全和企业合同变现，不靠卖客户端。 | 贡献主要围绕客户端兼容、协议和部署；没有 Tunnel 免费用户转付费比例。 |
| **zrok / OpenZiti** | zrok Apache-2.0，可无限自托管；托管免费 5 GB/日、25 environments、50 shares，绑卡可去 interstitial；生产 SLA、专用设施和自定义限额询价。[[定价]](https://zrok.io/pricing/) | 没查到托管收入或付费客户数。 | 免费托管额比 ngrok 大，说明“角标/验证 + 企业 SLA”也是一种分层；但没有可证明其可持续性的公开财务。 |
| **rathole** | Apache-2.0 的轻量隧道，自托管，无官方云。[[GitHub]](https://github.com/rapiz1/rathole) | 没有可见托管业务。 | 适合作为“CLI/协议开源会扩大采用”的例子，不是开源自然转收入的例子。 |
| **Plausible** | Community Edition 为 AGPL，可自托管；Cloud 托管订阅是公司唯一资金来源。云版更新更快，并提供高级 bot filtering、漏斗、旅程、电商、SSO、API 和支持。[[官方对照]](https://plausible.io/self-hosted-web-analytics) | 官方称 bootstrapped、无外部投资，并由订阅者完全资助；另一官方文章称约 19,000 个付费订阅者。[[开源商业说明]](https://plausible.io/blog/building-open-source) | 官方明确说开源是原则，不是因为它“本身好做生意”。没公开自托管→云的比例。 |
| **PostHog** | 当前保留无保证的 MIT Docker Compose hobby deployment；2023 停止付费 Kubernetes 自托管支持，推荐迁到 Cloud。[[官方说明]](https://posthog.com/blog/sunsetting-helm-support-posthog) | [一手] 当时 23,000 家公司注册、上年收入增长 6 倍；Kubernetes 用户仅约 3.5%，却占用过多基础设施团队时间。 | [二手创始人访谈] 曾称收入结构约 90% Cloud / 10% self-hosted，而支持问题约 30% / 70%。[[访谈整理]](https://plg.beehiiv.com/p/posthog-unconventional-growth) 这是少数公开比例，但并非经审计 cohort 转化率。 |
| **Umami** | MIT 自托管 + 官方 Cloud 套餐。[[官网]](https://umami.is/) [[Cloud 定价]](https://umami.is/pricing) | 没查到公开收入、云客户数或自托管转化率。 | 主要卖免运维、托管可靠性与团队便利，不证明许可证本身带来转化。 |
| **Coolify** | 核心 Apache-2.0，相同代码可自托管；Cloud 从约 `$5/月` 起，用户仍自带服务器。[[官网]](https://coolify.io/) [[Cloud 文档]](https://coolify.io/docs/get-started/cloud) | [一手页面口径] 官网显示 3,641+ Cloud customers；OpenCollective 页面曾披露约 `$15k+` 托管 MRR 与约 `$4.5k/月` sponsors，均是项目自报而非审计。[[OpenCollective]](https://opencollective.com/coollabsio) | 便利和维护可在“功能不阉割”的情况下收费；没公开自托管用户分母，无法算转化率。 |
| **Dokploy** | 2026 核心转 Apache-2.0；自托管免费，Cloud 按服务器收费，企业能力另有 source-available 边界。[[官网]](https://dokploy.com/) [[定价]](https://dokploy.com/pricing) | 没查到收入、付费客户或转化率。 | 与 Coolify 竞争说明开放核心本身不是壁垒，体验、升级、支持和渠道才是。 |
| **Cal.com** | [一手] 2026-04 官方宣布生产版转闭源，同时推出 MIT 的 `Cal.diy` 社区分支，理由包括 AI 时代的安全和产品边界。[[官方说明]](https://cal.com/blog/cal-com-goes-closed-source-why) | 没有在该公告中给出“闭源提高收入”的证据。 | 即使有理由，回撤长期“开源”承诺也会增加采用者对未来抽功能的担忧。playtest.run 应先把商业边界写清，而不是日后重解释。 |

### 4.2 AGPL 到底帮了什么

**能做的：**

- [一手许可证文本] AGPLv3 要求修改后的程序通过网络向用户提供服务时，向这些用户提供相应源代码。[[AGPLv3]](https://www.gnu.org/licenses/agpl-3.0.html)
- 对边缘与控制面的直接修改更难被闭源拿走；Plausible 证明 AGPL 社区版与托管订阅可以长期并存。
- 它向自托管者提供可审计、可退出的保证，有助于安全敏感用户先采用。

**不能做的：**

- 不能阻止竞争者原样部署、围绕它做运维和支持，或独立实现兼容服务。
- 不能自动带来贡献者。frp、rathole 的采用很大，商业回流仍不明显。[推断] 这类仓库最自然的外部贡献面是协议兼容、平台打包、文档和 bug，但本次没查到按来源归因的公开统计。
- 不能消除企业法务顾虑。网络 copyleft 会让部分公司要求额外审查；这不是“AGPL 一定吓退所有人”，但会抬高企业嵌入成本。

**“云厂商抄走”与改许可证案例：**

- [一手] Elastic 2021 因 AWS 提供托管 Elasticsearch 而改用 SSPL/Elastic License；AWS 随即 fork OpenSearch。[[Elastic 解释]](https://www.elastic.co/blog/why-license-change-aws) [[AWS OpenSearch 公告]](https://aws.amazon.com/blogs/opensource/stepping-up-for-a-truly-open-source-elasticsearch/)
- [一手] Redis 2024 改 SSPL 后，AWS/Google 等维护 Valkey；Redis 2025 又为 Redis 8 加入 OSI 批准的 AGPLv3，并承认此前变化伤害了社区关系。[[Redis 官方复盘]](https://redis.io/blog/agplv3/)
- [一手] MinIO 2021 从 Apache-2.0 转 AGPLv3，明确把网络分发纳入回馈义务。[[MinIO 公告]](https://www.min.io/blog/from-open-source-to-free-and-open-source-minio-is-now-fully-licensed-under-gnu-agplv3) 后续社区版 UI 变化引发的反弹缺少本次可稳定复核的一手解释，因此不把它写成已证实的许可证因果。

**结论：** AGPL 是“修改须回馈”的护栏，不是“禁止竞争托管”的商业许可证。若 playtest.run 真正担心云厂商原样托管，应该靠运营网络、信誉与服务品质取胜；若改成 source-available，则会与 DESIGN §7 的开源承诺直接冲突，并可能重演 Elastic/Redis 的 fork 与信任成本。

### 4.3 建议的开源边界

1. **保留 CLI、SDK、引擎插件 Apache-2.0。** 插件只调用公开协议，不链接 AGPL 服务端库；在仓库和 FAQ 中用示意图写清边界。这样 Unity/Godot/创作工具作者可以无顾虑集成。
2. **边缘、控制面和 `compose.yaml` 保留 AGPL-3.0。** 托管版与自托管版保持同一核心能力，不把“能否发布、门禁、基础结果”后来抽走。
3. **托管版卖运营而非秘密代码：** 香港路径、域名与证书、滥用/申诉、自动升级、备份、SLA、团队治理和支持。
4. **自托管支持分层：** 文档与社区支持免费；生产迁移、SLA 和企业支持收费。不要承诺调试每个用户的网络环境，PostHog 已证明复杂自托管支持会吞噬工程时间。
5. **明确无“插件传染”。** 许可证最终解释应由法律文本和律师给出，但工程上应避免把 AGPL 包直接依赖进 Apache 插件，并保留清晰进程/API 边界。

### 4.4 贡献者究竟从哪里来

- [一手仓库页面] frp、cloudflared、Plausible 都公开 contributor 图，但 GitHub 只展示 commit 身份与数量，不说明对方是雇员、托管客户、自托管用户还是一次性修复者。[[frp]](https://github.com/fatedier/frp/graphs/contributors) [[cloudflared]](https://github.com/cloudflare/cloudflared/graphs/contributors) [[Plausible]](https://github.com/plausible/analytics/graphs/contributors)
- Plausible、PostHog、Coolify 的公开商业说明能证明托管收入支持核心团队，却没有给出“多少代码来自社区”或“贡献者后来转付费”的漏斗。[[Plausible]](https://plausible.io/self-hosted-web-analytics) [[PostHog]](https://posthog.com/blog/sunsetting-helm-support-posthog) [[Coolify]](https://opencollective.com/coollabsio)
- [推断] 对 playtest.run，最可能的外部贡献不是控制面商业功能，而是 Unity/Godot 插件、OS/CPU 打包、代理兼容、自托管文档和可复现 bug。应分别统计首次贡献来源与后续托管采用，不能把 GitHub stars 当销售线索。
- **结论：本次没查到任何上述项目公开“自托管用户→贡献者→托管付费”的完整比例。** 开源更可靠的价值是信任、集成面和退出权；获客与收入仍要由托管体验验证。

## 5. 转化杠杆：什么有证据，什么只是常见包装

### 5.1 功能级证据表

| 杠杆 | 外部证据 | 证据强度 | 对 playtest.run 的建议 |
|---|---|---|---|
| **去角标/去门禁** | Carrd `$9/年` 即去品牌，且创始人称免费角标驱动增长；Loom `$18/人/月` 同时卖去品牌；ngrok `$10/月` 去 interstitial；tiiny.host 也把去角标放入付费。[[Carrd]](https://carrd.co/docs/pro/plans) [[Carrd 访谈]](https://saasclub.io/podcast/carrd-aj-306/) [[Loom]](https://www.loom.com/pricing) [[ngrok]](https://ngrok.com/pricing) | **中等。** 有 Carrd 创始人增长陈述和多家套餐布局，但没有“仅去角标”的独立转化率。 | 免费门禁页保留一行克制角标；付费可去掉。不要插全屏广告或多一次跳转。角标的任务是带来下一位开发者，不是惩罚玩家。 |
| **固定/可改子域** | ngrok、tiiny.host 付费提供稳定/独占名称；但 Surge 免费给自定义域。[[Surge]](https://surge.sh/pricing) | **中低。** 常见套餐证据，且有免费反例。 | 与去角标捆绑为“专业链接”，不要单独定高价。免费随机 slug 保持可分享，付费才固定和改名。 |
| **自定义域** | Carrd `$19/年`、Diawi `€4.29/月` add-on、Expo Starter 才有 Hosting custom domain；Surge 又免费提供。[[Carrd]](https://carrd.co/docs/pro/plans) [[Diawi]](https://www.diawi.com/features-services) [[Expo]](https://expo.dev/pricing) | **中低。** 明确有人包装收费，但没有功能级转化率。 | 首档可暂缓；证书、DNS、滥用和客服成本高。先验证固定 slug 是否已满足“对外专业”。 |
| **口令/邀请名单** | Carrd 把口令放最高 `$49/年`；Loom Business 有口令；Surge Pro 卖口令。但 Unity Play、Diawi 免费给口令。[[Carrd]](https://carrd.co/docs/pro/plans) [[Loom]](https://www.loom.com/pricing) [[Unity]](https://play.unity.com/en/faq) [[Diawi]](https://www.diawi.com/features-services) | **口令单独：低；受控邀请：中。** 免费反例很多，企业产品则为细粒度安全付费。 | 免费不必给口令；付费卖口令 + 邮箱名单 + 单次邀请 + 访问记录的完整“未公开测试”能力。 |
| **团队** | Loom 按席位；PlaytestCloud 套餐给 unlimited users/seats；Antidote 按团队人数与游戏；Expo 把 SSO、支持和并发放高档。[[Loom]](https://www.loom.com/pricing) [[PlaytestCloud]](https://www.playtestcloud.com/pricing) [[Antidote]](https://antidote.gg/pricing/) [[Expo]](https://expo.dev/pricing) | **高于个人美化。** B 端定价普遍围绕协作、权限和责任。 | 首档先给 3 个只读 reviewer 或共享结果链接；真正多成员 workspace、角色和 SSO 后置为 Studio。 |
| **带宽/并发/分钟** | ngrok 以 GB/请求限额；Netlify 用统一 credits；Expo 以构建、MAU、带宽和并发；Playset 以试玩分钟。[[ngrok]](https://ngrok.com/pricing) [[Netlify]](https://docs.netlify.com/manage/accounts-and-billing/billing/billing-for-credit-based-plans/credit-based-pricing-plans) [[Expo]](https://expo.dev/pricing) [[Playset]](https://playset.app/) | **高，但易伤信任。** 这是成本控制杠杆，不必然是购买动机。 | 免费硬限额；付费含可理解流量包。只做预付扩容，不默认超额后付。仪表盘同时显示“还能支持约多少次 30 MB 打开”。 |
| **保留期/版本** | Diawi 直接卖链接延长，`+1 月 €0.49`、`+1 年 €4.99`；Expo 免费日志 7 天、付费 3 个月，preview deployment 免费 30 天、付费 90 天；ngrok 付费延长日志。[[Diawi]](https://www.diawi.com/features-services) [[Expo]](https://expo.dev/pricing) [[ngrok]](https://ngrok.com/pricing) | **中高。** 与真实工作流直接相连，且成本可控。 | 免费 30 天结果、5 个版本；付费 180 天结果、20 个版本和导出。它比泛分析更容易被认真测试者理解。 |
| **更细结果/分析** | Loom 付费给 engagement/export；Expo 付费给导航、更新、错误和 session timeline；PlaytestCloud/Antidote 为录像、招募和分析收高价。[[Loom]](https://www.loom.com/pricing) [[Expo]](https://expo.dev/pricing) [[PlaytestCloud]](https://www.playtestcloud.com/pricing) [[Antidote]](https://antidote.gg/pricing/) | **高，但前提是能改变决策。** | 做版本对比、来源、加载成功率、错误聚合、反馈状态和 CSV；不做通用营销漏斗、热图或用户画像平台。 |
| **品牌传播** | Carrd 创始人明确说 `Made with Carrd` 带来增长，Loom 已核验把去品牌放在 `$18/人/月` Business。Notion 官方帮助只确认付费计划增加自定义 slug、外观/SEO，并可购买自定义域 add-on，没有在该页明确写“remove branding”。[[Carrd 访谈]](https://saasclub.io/podcast/carrd-aj-306/) [[Loom]](https://www.loom.com/pricing) [[Notion Sites 帮助]](https://www.notion.com/help/public-pages-and-web-publishing) Typeform 帮助页返回 403，Linktree 帮助链接重定向到通用说明；两者 2026 的准确去品牌档位本次未核验。[[Typeform 定价]](https://www.typeform.com/pricing/) [[Linktree 定价]](https://linktr.ee/s/pricing) | **增长证据中等，付费证据弱。** Carrd 有创始人陈述；其余主要是套餐包装，没有功能级转化率。 | 角标应可点击但不抢玩家注意力；先测“角标带来的开发者注册/CLI 安装”，再判断去角标是否是强付费点。 |

### 5.2 关键判断

- **最可能触发付费的不是某一个小功能，而是一个身份变化：** 从“随手给朋友看”变成“给发行商、客户、封闭玩家群或同事做正式测试”。
- 去角标、固定链接、邀请名单和更长结果分别解决**专业、稳定、私密、可追溯**，组合后才形成清晰购买理由。
- 更细数据有价值，但“PV 更多”不是。高价 playtest 产品证明的是“降低错误决策成本”；playtest.run 应展示“这个版本比上个版本少了什么错误、哪个来源的人成功打开、反馈如何归档”。
- 公开资料中，除 Carrd 创始人对品牌传播/折扣的陈述外，**没查到这些产品逐功能的真实转化率或 A/B 测试结果**。因此前三个付费点仍是假设，必须用“有人主动问价”和 checkout intent 验证。

## 6. 单位经济

### 6.1 2025–2026 带宽基准

| 供应商/路径 | 2026-09-07 公开价格 | 可靠性与限制 |
|---|---|---|
| **阿里云香港 ECS 直出** | [一手] `$0.153/GB`；CDT 普通 BGP 亚太前 10 TB `$0.10/GB`、10–50 TB `$0.07/GB`。自 2025-06-01 起，境外区域每账户每月前 200 GB 免费。[[CDT 文档]](https://www.alibabacloud.com/help/en/cdt/internet-data-transfers/) | 适合香港源站基线。Premium BGP 回大陆为 `$0.452/GB` 起，不能把普通 BGP 价格与优质大陆线路混为一谈。 |
| **阿里云 CDN AP1** | [一手] 0–50 TB `$0.081/GB`。[[CDN 计费]](https://www.alibabacloud.com/help/en/cdn/product-overview/billing-rules-of-basic-services) | 计费量包含日志未显示的 TCP/IP 头与重传，实际略高于应用层字节。 |
| **腾讯云香港 CVM 直出** | [一手] 公开区域表约 `$0.12/GB`；官方提醒协议头和重传可使计费比应用日志高约 5–15%。[[CVM 网络计费]](https://www.tencentcloud.com/document/product/213/39743) | 页面动态渲染，最终应以控制台订单为准。 |
| **腾讯云 CDN 亚太** | [一手] 0–2 TB `$0.0665/GB`、2–10 TB `$0.0592/GB`、10–50 TB `$0.0533/GB`。[[CDN 定价]](https://www.tencentcloud.com/document/product/228/2949) | 需要实测大陆三网命中率、回源与缓存规则，不能只看标价。 |
| **AWS 香港** | [一手] AWS 各服务/区域合计每月前 100 GB 互联网出站免费。[[EC2 定价]](https://aws.amazon.com/ec2/pricing/on-demand/) 香港具体费率由动态区域表/计算器给出，本次观察约 `$0.12/GB`，但未获得稳定静态表，故不作为主模型。 | 需另计实例、请求、存储；价格不优于香港本地/CDN基线。 |
| **Cloudflare R2** | [一手] Standard storage `$0.015/GB-month`，Class B `$0.36/百万次`，互联网 egress 免费；每月含 10 GB-month、100 万 Class A、1,000 万 Class B。[[R2 定价，2026-08-07 更新]](https://developers.cloudflare.com/r2/pricing/) | 免费出网很有吸引力，但 Cloudflare China Network 是 Enterprise 合作网络，不能据此假设 R2 从大陆稳定可达。[[China Network 产品范围]](https://developers.cloudflare.com/china-network/reference/available-products/) 必须真机验证。 |
| **Bunny CDN 亚洲** | [一手] 亚洲区域公开价约 `$0.03/GB`。[[定价]](https://bunny.net/pricing/cdn/) | 理论最低，但大陆回程、HTTPS 握手、丢包和大文件持续下载尚未真机验证，不能先写入 SLA。 |
| **Vultr / Hetzner** | [一手] Vultr 当前公开区域列表未见香港；Hetzner Cloud 只有欧洲、美国和新加坡等，无香港。[[Vultr 区域]](https://www.vultr.com/campaign/compute/) [[Hetzner locations]](https://docs.hetzner.com/cloud/general/locations/) | 不能拿它们的低价流量作为“香港边缘”成本。 |
| **DMIT 香港** | 低价、大流量信息主要来自第三方商家/论坛，不同线路差异极大。 | **没有查到足够稳定的一手按 GB 价格与 SLA；在真机压测和正式报价前，不纳入可靠预算。** |

### 6.2 “30 MB 被玩 1,000 次 ≈ $3”复核

[计算] 按十进制近似：

\[
30\ \text{MB} \times 1{,}000 = 30{,}000\ \text{MB} \approx 30\ \text{GB}
\]

| 路径 | 30 GB 理论出网费 | 若计费字节同样增加 5–15% |
|---|---:|---:|
| Bunny 亚洲 `$0.03/GB` | `$0.90` | `$0.95–1.04` |
| 腾讯 CDN 首档 `$0.0665/GB` | `$2.00` | `$2.10–2.29` |
| 阿里 CDN AP1 `$0.081/GB` | `$2.43` | `$2.55–2.79` |
| `$0.10/GB` 基线 | **`$3.00`** | **`$3.15–3.45`** |
| 腾讯香港直出 `$0.12/GB` | `$3.60` | `$3.78–4.14` |
| 阿里香港 ECS `$0.153/GB` | `$4.59` | `$4.82–5.28` |

所以 DESIGN 的 `$3` **不是错，而是一个合理中位基线**。但表中没有计算对象存储、请求、回源 miss、日志、错误采集、DDoS/爬虫、固定服务器和支付成本；也假设每次完整下载 30 MB、没有浏览器缓存和压缩差异。

### 6.3 20 GB 免费额度的风险

[计算] 单账户满额的纯出网成本：

| 路径 | 20 GB / 账号 / 月 |
|---|---:|
| Bunny 亚洲 | `$0.60` |
| 腾讯 CDN 首档 | `$1.33` |
| 阿里 CDN AP1 | `$1.62` |
| `$0.10/GB` 基线 | `$2.00` |
| 腾讯香港直出 | `$2.40`，另有协议开销 |
| 阿里香港 ECS | `$3.06` |

1,000 个免费开发者、每人额度 20 GB，按 `$0.10/GB` 基线：

| 平均额度利用率 | 总流量 | 月出网成本 |
|---:|---:|---:|
| 10% | 2 TB | `$200` |
| 25% | 5 TB | `$500` |
| 50% | 10 TB | `$1,000` |
| 100% | 20 TB | `$2,000` |

若全部满额，不同路径约为：Bunny `$600`；腾讯 CDN 按 2/8/10 TB 分段约 `$1,140`；阿里 CDN `$1,620`；阿里 CDT 按 10 TB `$0.10` + 10 TB `$0.07` 约 `$1,700`（再减账户级 200 GB 免费量）；腾讯直出约 `$2,400+`；阿里 ECS 约 `$3,060+`。

### 6.4 要多少付费转化才能平衡

先用一个对现有 DESIGN 友好的假设：付费档 `$15/月`，贡献毛利 80%，即每位付费用户贡献 `$12/月`；不计固定成本和付费用户自己的流量。

| 20 GB 免费档平均利用率 | 1,000 免费用户成本 | 仅覆盖免费流量所需付费率 |
|---:|---:|---:|
| 10% | `$200` | 1.7% |
| 25% | `$500` | 4.2% |
| 50% | `$1,000` | 8.3% |
| 100% | `$2,000` | 16.7% |

若再加约 `$150/月` 固定成本，所需比例变为约 2.9%、5.4%、9.6%、17.9%。这仍过于乐观，因为付费用户也会消耗带宽。

对建议的 **10 GB 免费档**做更保守模型：付费档 `$15` 含 50 GB；若付费用户真的用满，按 `$0.10/GB` 其带宽成本 `$5`，只剩 `$10` 覆盖免费用户与固定成本。加 `$150/月` 固定成本：

| 10 GB 免费档平均利用率 | 免费流量成本 | 所需付费率 |
|---:|---:|---:|
| 10% | `$100` | 2.5% |
| 25% | `$250` | 4.0% |
| 50% | `$500` | 6.5% |
| 100% | `$1,000` | 11.5% |

**判断：**

- 20 GB 对 20–100 人内测很安全，最坏不过几十到两百美元；对无门槛公开增长则危险。
- 10 GB 仍比 ngrok 1 GB 慷慨，也接近 Netlify 新免费档若只消费带宽的约 15 GB；Cloudflare/Expo 的更高免费量有大型生态补贴。
- 上线前最需要的不是继续比较云价，而是观测每账户 GB 的 p50/p90/p99、匿名链接被爬取率、缓存命中率、30 MB 作品的实际重复下载比例。
- R2 或廉价 CDN 可降低成本，但**不能在大陆三网真机记录前，把“免费出网/低价”写成产品能力**。

## 7. 谁有 B 端预算

| 买方 | 预算与先例 | 与当前产品的匹配 | 优先级 |
|---|---|---|---:|
| **有发行商/融资的小工作室** | Antidote 直接把“有 publisher/investment、团队 <30”放入 `$99.95/月` Indie Plus；PlaytestCloud Basic 从 `€1,025/月` 起。[[Antidote]](https://antidote.gg/pricing/) [[PlaytestCloud]](https://www.playtestcloud.com/pricing) | 需要私密链接、固定版本、错误/反馈、可向发行商转发的结果。无需一开始提供玩家招募。 | **1** |
| **发行商 / 内部用户研究团队** | PlaytestCloud Enterprise 卖 SSO、预算管理、合规和研究运营；客户引文/案例包括 PlayStation、Mojang、Homa、Kwalee。Antidote 案例包括 Remedy、Ubisoft、Bandai Namco、Ankama。[[PlaytestCloud]](https://www.playtestcloud.com/pricing) [[Antidote 案例]](https://antidote.gg/case-study/) | 预算最高，但会要求 NDA、招募、录像、角色权限、审计、合规和服务承诺；远超 v0.1。 | **2（后续）** |
| **游戏出海团队** | 香港路径、微信/大陆打开成功率和海外分享都有现实价值，但本次**没查到专门为“WebGL 测试链接跨境可达”付费的公开采购案例**。 | 网络优势高度匹配，但必须先有真实三网/设备数据，不能只卖地理标签。 | **3（验证型）** |
| **学校 / 课程** | Loom 提供教育折扣；本次没查到学校为 WebGL 作业链接按席位付费的强先例。[[Loom 定价 FAQ]](https://www.loom.com/pricing) | 班级名单、截止时间、批量归档有价值，但预算低、采购慢、隐私要求高。适合教育免费/团体额度获客。 | **4** |
| **Game Jam 主办方** | itch.io 创建 jam 免费，且已有大规模提交生态；Devpost 等比赛平台走企业询价，卖的是报名、评审、运营和品牌，不只是文件托管。[[itch.io Jam]](https://itch.io/docs/creators/game-jams) [[Devpost for Teams]](https://info.devpost.com/product/devpost-for-teams) | 流量突发、提交多、持续时间短，成本风险高；若不做评审/运营，很难向主办方收费。可做赞助额度或获客合作，不是首个 ICP。 | **5** |
| **外包 QA** | PlaytestCloud/Antidote 证明 QA/研究预算存在，但它们提供录像、设备、招募、研究服务和原生包分发。[[PlaytestCloud]](https://www.playtestcloud.com/pricing) [[Antidote]](https://antidote.gg/pricing/) | 当前仅浏览器作品，缺设备矩阵、bug 管理、录像与 SLA；容易被需求拖成另一产品。 | **6** |

**B 端结论：** 第一位买家不是大型发行商采购，而是“已经需要向发行商、客户或封闭玩家群交付一个看起来正式且可追踪链接的小团队”。企业研究市场可以证明结果有价值，却不能证明 v0.1 应立即做 enterprise。

## 8. 给 playtest.run 的具体方案

### 8.1 v0.1 免费档

**匿名首次使用：**

- 1 个活跃链接，24 小时有效；
- **总出网 1 GB 硬上限**，约等于 30 MB 构建完整打开 33 次；
- 单个压缩上传包建议 500 MB 上限；
- 随机 slug、门禁页角标、50 并发；
- 80% 提醒，100% 时停止作品流量，展示明确的“此测试链接本期流量已用完”，不要求玩家登录；
- 不绑卡、不产生账单、不自动续期；开发者登录后可转入账户额度。

**登录后的免费账户：**

- **10 GB/月出网硬上限**；
- 3 个活跃 slug；
- 每 slug 保留 5 个版本；
- 每版本压缩包上限 500 MB；
- 每 slug 50 并发；
- 结果保留 30 天；
- 随机 slug，门禁页底部保留“由 playtest.run 提供”；
- 达到额度默认硬停，不自动收费；可删除旧版本但不能用删除绕过当月出网计量。

邀请制 v0.1 可以暂时给创始用户 20 GB，以验证真实利用率；但公开文档不要先承诺 20 GB。若连续两个月 p95 使用低于 5 GB、滥用率可控，再上调，而不是反过来突然削减。

### 8.2 第一付费档

建议名称先用直白的 **Pro**，价格 **`$15/月` 或 `$150/年`**：

- 50 GB/月；
- 10 个活跃 slug；
- 每 slug 20 个版本；
- 200 并发；
- 结果保留 180 天；
- 80%/100% 用量通知；
- 可购买 **50 GB / `$10` 的预付流量包**，不用则不扣费，不自动续包。

最可能被买的三个东西：

1. **专业链接：** 固定且可改的 slug、去门禁页角标；自定义域等有人主动提出后再做。
2. **私密测试：** 口令、邮箱邀请名单、可撤销/单次邀请和基础访问记录；不是只卖一个口令框。
3. **结果工作流：** 版本间打开/加载/错误比较，180 天留存，反馈状态、CSV 导出，以及最多 3 位只读 reviewer。

这三项对应同一个购买时刻：“我要把这次测试认真地交给别人，并在之后拿结果做决定。” 若拆成多个微型 add-on，会让首次定价像云账单一样难懂。

### 8.3 数据分析边界

**应做：**

- 每版本唯一打开人数与成功加载率；
- 设备/浏览器/来源；
- 加载时间分位数与 JS/WASM 错误聚合；
- 一句话反馈、已读/已处理状态；
- 与上一个版本对比；
- CSV/JSON 导出和可分享结果摘要。

**不应做：**

- 通用事件埋点平台；
- 营销漏斗、用户画像、广告归因；
- 全量 session replay/录像；
- 跨站追踪或玩家账户体系；
- 为“数据更丰富”增加玩家同意步骤。

这既保持 DESIGN §3.7 的边界，也把“知道结果”从看一张 PV 图升级为能回答“这个版本是否更可玩”。

### 8.4 曝光、反馈与孵化

- **反馈：做。** 但保持一句话、零登录，并围绕版本归档；这是核心闭环，不是社区。
- **曝光/榜单：不做。** itch.io、Unity Play、The Gaming Nest 已有发现层；加入它会带来审核、版权、成人内容、支付商和域名信誉风险。
- **孵化：不做产品机制。** 可以人工关注高复用团队、访谈和提供额度；在有“发布后反复回来、结果会被使用”的数据前，不建设申请、评审、基金或流量分发系统。
- **未来触发条件：** 只有当大量开发者明确说“我没有测试者”而不是“分享太麻烦”，并且人工撮合能持续提高复用率，才重新评估 opt-in tester pool；这将是一次 DESIGN 方向变更。

## 9. 三种最可能的死法

### 死法一：免费流量、恶意使用和域名信誉先于 PMF 放大

- **先例：** Railway 曾每收入 `$1` 亏 `$16`；Heroku 因欺诈滥用关闭免费资源；Glitch 因数百万应用成本和恶意使用关闭托管；ngrok 因 malware 要求免费 TCP 绑卡。[[Railway]](https://blog.railway.com/p/free-plan) [[Heroku]](https://www.heroku.com/blog/next-chapter/) [[Glitch]](https://blog.glitch.com/post/changes-are-coming-to-glitch) [[ngrok]](https://ngrok.com/blog/tcp-endpoints-require-verification)
- **playtest.run 风险：** 大文件、自动爬虫、热链、钓鱼、加密货币脚本或一次 viral 分享都可能消耗成本并伤害 `*.playtest.run` 信誉。
- **预防：** 匿名 1 GB/24h、账户硬上限、静态文件白名单、压缩包/解压后文件数限制、速率与并发双限制、举报页、自动隔离和清晰失败页。免费额度根据真实 cohort 扩大。

### 死法二：只是又一个静态托管/隧道，没有重复使用理由

- **先例：** Glitch 官方承认旧架构不再有独特价值；Cloudflare Tunnel、Surge、Unity Play、itch.io 和 The Gaming Nest 都能免费给链接。[[Glitch]](https://blog.glitch.com/post/changes-are-coming-to-glitch) [[Cloudflare]](https://blog.cloudflare.com/tunnel-for-everyone/) [[Surge]](https://surge.sh/pricing) [[Unity Play]](https://play.unity.com/en/faq) [[itch.io]](https://itch.io/docs/creators/game-jams) [[The Gaming Nest]](https://thegamingnest.com/en/webgl-hosting)
- **playtest.run 风险：** 用户第一次觉得 CLI 很酷，第二次直接用已有平台；“香港”若未经真机证明也只是一句营销话。
- **预防：** 优先打磨从命令到玩家打开的成功率、二维码/门禁、版本替换不换链接，以及轻量结果；用“30 天内再次部署、查看结果、发起第二次测试”衡量 PMF，不用注册数或上传数自我安慰。

### 死法三：不可预测计费或突然收紧承诺，失去开发者信任

- **先例：** Netlify `$104,500` 异常流量账单事件、Replit effort-based 错误计费、Heroku 突然终止免费档、ngrok 大幅价格跳级的社区反弹。[[Netlify 用户原帖]](https://news.ycombinator.com/item?id=39521986) [[Replit 官方复盘]](https://replit.com/blog/effort-based-pricing-recap) [[Heroku]](https://www.heroku.com/blog/next-chapter/) [[ngrok 用户讨论]](https://news.ycombinator.com/item?id=31773570)
- **playtest.run 风险：** 开发者把链接发出去后无法控制访问；任何默认后付费都让“分享”变成财务风险。未来抽走开源版功能也会产生相同信任问题。
- **预防：** 默认硬停、预付扩容、预算告警、公开计量口径、清晰 grandfather 政策；开源核心持续同源发布，不在用户采用后回撤或重新解释核心边界。

## 10. 对 playtest.run 的含义

### 10.1 支持 DESIGN.md 现有结论

- **支持 §0/§2：** 最大优势应是同一件事做得更快、更可信，而不是发明另一种生态。Railway、Carrd、Loom 都支持极短首次体验与分享传播可以形成增长。[[Railway]](https://blog.railway.com/p/free-plan) [[Carrd 访谈]](https://saasclub.io/podcast/carrd-aj-306/) [[Loom]](https://www.loom.com/pricing)
- **支持 §3.3：** 玩家零门槛是正确硬约束。所有高价 playtest 工具都在减少参与摩擦；要求玩家登录会直接损害样本。
- **支持 §3.7：** “打开、设备、来源、错误、一句话反馈”是合理核心。建议只补版本比较、加载成功率、归档/导出，不扩成通用 analytics。
- **支持 §6 的 v0.1 不收费：** 当前没有真实利用率和主动问价，过早结算系统会分散对首次体验和复用的验证。
- **支持 §7：** Apache 客户端 + AGPL 服务端可与托管业务共存；Plausible、Coolify 提供了先例。托管网络与运营才是生意。[[Plausible]](https://plausible.io/self-hosted-web-analytics) [[Coolify]](https://coolify.io/)
- **支持 §8：** 不做发现/榜单/商店。itch.io 已占据免费发现与 jam，进入该层会引入支付和治理负担。[[itch.io Jam]](https://itch.io/docs/creators/game-jams) [[itch.io 内容治理事件]](https://itch.io/updates/update-on-nsfw-content)

### 10.2 与 DESIGN.md 冲突或需要收紧之处

1. **免费档 20 GB 的公开承诺偏危险。** 在 `$0.10/GB` 下，1,000 人满额就是 `$2,000/月`，超过“固定成本百美元”叙事。建议公开 10 GB；20 GB 仅用于邀请制试验。
2. **“30 MB × 1,000 ≈ $3”正确，但不是完整成本。** 文档应注明协议/重传、回源、存储、请求、滥用与大陆线路品质；不能把 R2 免费出网或 Bunny `$0.03` 当作未经真机验证的答案。
3. **付费功能优先级应调整。** 自定义域成本和支持较高，且 Surge 等免费提供；首档先做固定 slug + 去角标、私密邀请、结果留存/比较。自定义域等真实客户提出。
4. **“更多流量与并发”应是成本边界，不是首要营销语。** 第一购买理由应是正式、私密、可追溯；流量包保持简单预付。
5. **更细结果值得做，但要写成测试决策能力。** 不建议把“数据分析”扩成通用行为分析；高价 B 端工具的价值来自招募、录像和研究结论，不能由 PV 模仿。
6. **AGPL 边界需要更明确的开发者说明。** 尤其要写清 Apache 引擎插件不链接 AGPL 服务端、普通自托管无需商业许可、修改后网络提供服务时的源码义务；否则会无谓吓退插件作者和企业评估。

### 10.3 下一步应验证的数字

在修改长期套餐前，用 v0.1 记录：

- 匿名链接 1 GB 是否足够完成首轮测试；
- 每账户月出网 p50/p90/p99；
- 作品大小、首次加载与重复加载的真实字节；
- 大陆移动/联通/电信 + 微信内置浏览器的成功率和 p95 加载时间；
- 30 天内第二次部署率；
- 查看结果、处理反馈和导出的比例；
- 点击角标后注册/安装 CLI 的比例；
- 主动提出固定 slug、去角标、口令/名单、长留存、团队的用户数；
- 滥用、热链、爬虫和投诉占比。

只有这些数据能回答 10 GB 是否该升到 20 GB、`$15` 是否合理，以及“更多结果”是否真的比“更方便分享”更重要。

## 11. 没查到或尚未验证

- ngrok 2023–2024 每个历史套餐、限额和迁移日期的完整可引用存档；只核验了当前价格、2024 TCP 验证与部分用户反弹。
- ngrok、Vercel、Netlify、Railway、Fly.io、Glitch、Surge、itch.io、tiiny.host 等私营业务的经审计收入，以及绝大多数产品的免费→付费 cohort 转化率。
- 去角标、自定义域、固定子域、口令、团队和分析各自的独立增量转化率；公开定价页只能证明产品把它们当作杠杆。
- Typeform、Linktree 2026 去品牌功能的准确套餐和价格；Typeform 帮助页返回 403，Linktree 帮助链接重定向到通用产品说明。Notion 本次只核验到付费站点定制、slug 和自定义域 add-on，未核验独立的 remove-branding 条款。
- Plausible、Umami、Coolify、Dokploy、frp、zrok 等自托管用户转云的比例；PostHog 只有特定时期的收入结构/部署比例，不是标准转化漏斗。
- frp、cloudflared、Plausible 等贡献者按“雇员、云客户、自托管用户、其他使用者”拆分的来源，以及贡献者转托管付费的比例。
- Playset 动态页面的完整 2026 套餐正文；报告中的 15 分钟、`$19/$49` 来自官网搜索索引，需人工再核。
- The Gaming Nest 一次性扩容的实际价格、收入和长期资金来源。
- Screen Studio、CleanShot X 2026 动态价格与公开收入；第三方价格未获一手复核，未用于建议。
- InstallOnAir “8 天”与站内“60 天”描述的矛盾。
- AWS 香港静态可引用的当前出网阶梯、DMIT 香港可靠一手单价与 SLA。
- R2、Bunny、阿里/腾讯普通香港线路在中国大陆三网、微信内置浏览器和 30–500 MB WebGL 大文件上的真机表现。
- Game Jam 主办方、学校课程和外包 QA 为“浏览器作品一键分享 + 轻结果”付费的直接案例；现有证据更支持把它们当获客/团体额度，而非首个付费 ICP。
