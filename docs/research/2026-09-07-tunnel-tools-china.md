# 大陆内网穿透工具与国内 Web 游戏「给人试玩」现状调研

**调研日期：2026-09-07**

## 0. 结论先行

1. **「把网页作品发给朋友玩」是可观察到的真实需求，但不是一个已有统一入口的成熟品类。** 最直接的证据不是搜索量，而是开发者反复用临时二维码、网盘、GitHub Pages、Cloudflare Pages、微信体验版和活动自建试玩站拼出同一条路径。腾讯 EdgeOne Makers 甚至直接把「让 AI 做一个贪吃蛇，生成链接发给朋友玩」写成官方场景，说明国内云厂商也在验证这个需求。[一手：EdgeOne Makers AI 部署说明](https://pages.edgeone.ai/document/ai-dialogue-deployment-deploy-project-with-one-sentence-using-skill) [用户原话：两天用 Codex 做游戏并部署到 Cloudflare Pages](https://www.v2ex.com/t/1227978)
2. **方便、简洁和首次成功率确实应排第一。** 国内穿透产品的首页普遍卖「一分钟」「无需公网 IP」「微信调试」，而真正给普通玩家的微信小游戏、TapTap 测试、网盘 Demo 都额外要求白名单、审核、安装或下载。需求缺口不是又一个隧道协议，而是开发者发出一个链接后，普通玩家能直接打开并成功开始。[一手：NATAPP](https://natapp.cn/) [一手：cpolar](https://www.cpolar.com/) [一手：微信体验成员规则](https://kf.qq.com/faq/170302zeQryI170302beuEVn.html)
3. **「国内穿透工具免费档都是 1 Mbps」不成立。** 截至 2026-09-07，cpolar 免费档明确为 1 Mbps，飞鸽为 0.5 Mbps；SakuraFrp 则是 10 Mibps、2 条隧道、5 GiB/月。NATAPP、网云穿等当前公开页没有完整披露免费带宽，网上的「1 Mbps」多是旧教程，不能当成当前一手事实。[一手：cpolar 定价](https://www.cpolar.com/pricing) [一手：SakuraFrp 首页](https://www.natfrp.com/) [一手：飞鸽定价](https://www.fgnwct.com/price.html)
4. **境外免费托管并非同样不可用，但最常见的默认域名风险很高。** GreatFire 最近测量中，`itch.io` 域下 26/26 个样本被屏蔽，`vercel.app` 131 个样本中 127 个被屏蔽；`netlify.app` 则有 79 个样本中的 72 个可访问。应分别描述，不宜统称为「都慢」。[二手测量：itch.io](https://zh.greatfire.org/domain/itch.io) [二手测量：vercel.app](https://zh.greatfire.org/domain/vercel.app) [二手测量：netlify.app](https://zh.greatfire.org/domain/netlify.app)
5. **香港免备案是可行的上线手段，不是稳定性的保证。** NATAPP、飞鸽都在卖「香港线路、无需备案」，证明这条供给真实存在；但香港普通公网回大陆晚高峰会拥塞，同一台阿里云香港轻量服务器有用户测到 300–400 ms，厂商工单回复也明确峰值带宽不作业务承诺。要把「香港首节点」写成待真机验证的技术假设，而不是性能结论。[一手：NATAPP 香港线路](https://natapp.cn/) [一手：飞鸽定价](https://www.fgnwct.com/price.html) [用户原话：阿里云香港轻量 300–400 ms](https://www.v2ex.com/t/1178018)
6. **轻量反馈和加载诊断比「曝光/孵化生态」更接近当前缺口。** 有开发者明确想让几十个陌生人验证「会不会点进来、能不能秒懂、第一局能不能打完、还想不想再来」，并追问加载、UI、难度；BOOOM、GmHub、TapTap 已在做作品发现、活动和招募。v0.1 更应把门禁点击、加载成功/失败、设备、来源、JS 错误和一句话反馈做准，而不是再造作品广场。[用户原话：陌生人试玩验证问题](https://www.v2ex.com/t/1224053) [一手：TapTap 篝火测试](https://developer.taptap.cn/docs/store/operation/test/test-skills/gameplaytest/) [一手：BOOOM 2025](https://site.gcores.com/booom2025/)

## 1. 方法、口径与限制

- **[一手]** 指产品官网、官方文档、官方定价页、平台规则或公司官网；只说明页面在 2026-09-07 可见的内容，不代表服务商会永久维持该政策。
- **[用户原话]** 指能定位到具体帖子或评论的知乎、V2EX、B 站、贴吧、CSDN、掘金等内容。软文、SEO 聚合页和服务商自测不算用户原话。
- **[二手/推断]** 指媒体转述、第三方网络测量、工商数据库、教程及基于已知带宽的计算。GreatFire 是外部测量，不是中国运营商或平台官方结论。
- **价格口径**：除非另行注明，均为 2026-09-07 页面所示价格；促销价、签到赠送、流量计费和长期预付不能直接横向比较。
- **本轮没有注册账号、购买套餐、上传构建，也没有在大陆电信/联通/移动及微信真机上做访问测试。** 因而访客中间页、真实吞吐、TLS 兼容、微信风控和晚高峰表现只能写为文档事实、用户报告或待验证项。
- 搜索对小红书、即刻和部分 B 站评论的索引能力有限；能搜到大量教程和营销稿，但没有把不可归因的内容包装成用户原话。

## 2. 大陆内网穿透工具

### 2.1 免费档、域名、实名和付费档

| 工具 | 免费档（2026-09-07） | 实名 | 域名/备案 | 付费档举例 | 微信调试与访客页 |
|---|---|---|---|---|---|
| **cpolar** | 1 Mbps、4 条在线隧道、随机公网地址；免费地址会变化。[一手](https://www.cpolar.com/pricing) | 2025 版服务条款要求账户信息真实并配合实名认证。[一手](https://www.cpolar.com/tos) | 免费为平台随机子域；更高档可保留/自定义域名。公开定价页没有把自有域名备案条件讲完整。 | 页面显示基础版约 ¥99/年、2 Mbps；专业版约 ¥149/年、3 Mbps，价格可能为活动价。[一手](https://www.cpolar.com/pricing) | 官网把微信公众号/小程序本地调试列为场景。[一手](https://www.cpolar.com/) 未查到国内访客必经广告中间页的规则，未真机验证。 |
| **NATAPP** | 免费隧道、随机域名、地址不定期强制变化；当前公开首页**没有明确写免费带宽和隧道数**，旧教程常写 1 Mbps，不能视为当前一手事实。[一手](https://natapp.cn/) | 注册页要求实名，页面说明含身份证与人脸核验。[一手](https://natapp.cn/register) | 免费为平台随机域名；VIP 可用平台二级域名或自有域名。香港线路明确写「无需备案」；国内自有域名的当前完整条件没有在公开首页找到。[一手](https://natapp.cn/) | VIP 约 ¥9/月；100 Mbps 按量线路约 ¥10/月另加 ¥1.1/GB；香港 100 Mbps 按量约 ¥15/月另加 ¥2/GB。[一手](https://natapp.cn/) | 首页首要案例是微信开发、本地 Web 调试。[一手](https://natapp.cn/) 未查到强制访客中间页。 |
| **花生壳（贝锐）** | 当前公开页确认有免费体验/免费映射，但没有在无需登录的定价页稳定披露完整免费带宽、流量和映射数；旧资料中的 1 Mbps 不足以确认当前政策。[一手](https://hsk.oray.com/price/) | 贝锐产品体系要求账户实名；本轮没有走到具体花生壳套餐的实名页面，故不写验证方式。[一手：贝锐用户协议](https://service.oray.com/question/1820.html) | 平台提供映射域名；专属域名、HTTPS 和更多映射随套餐升级。自有域名及备案约束未在公开价格页完整查到。 | 面向个人的 Plus/长期活动价和面向企业的年费套餐并存；公开活动价格变化频繁，当前页面未给出一个可稳定复核的统一个人月价。[一手](https://hsk.oray.com/price/) | 官方场景覆盖远程访问、摄像头、NAS、开发调试，不只面向微信。[一手](https://hsk.oray.com/) 未查到访客中间页规则。 |
| **SakuraFrp（樱花 FRP）** | 10 Mibps、2 条隧道、5 GiB/月；免费节点不承诺 SLA。[一手](https://www.natfrp.com/) | 自 2024 年起使用隧道前需完成实名认证；官方文档说明验证可能收取小额费用。[一手](https://doc.natfrp.com/faq/realname.html) | 提供平台域名；中国大陆节点提供 HTTP(S) 服务时要求已备案域名，海外节点无需大陆备案，但实名认证仍保留。[一手](https://doc.natfrp.com/faq/network.html) | 银卡约 ¥20/月，页面列出 36 Mibps、10 条隧道和更高月流量；还有按节点/流量升级的档位。[一手](https://www.natfrp.com/) | 不以微信为唯一主场景，覆盖游戏联机、远程桌面、Web。未查到访客中间页。 |
| **飞鸽内网穿透** | 0.5 Mbps、2 个 TCP/UDP 端口、平台域名，需每日签到维持/获得权益。[一手](https://www.fgnwct.com/price.html) | 当前价格页没有写清实名流程；第三方教程提到实名，但本轮未登录验证，记为**没查到**。 | 提供免费二级域名和自定义域名档；香港线路明确写免备案。[一手](https://www.fgnwct.com/price.html) | 约 ¥6.9/月起，页面另列 5–40 Mbps 的月付档及香港线路。[一手](https://www.fgnwct.com/price.html) | 场景包含 Web、远程和游戏。未查到访客中间页。官网自称团队规模小，无法提供专业客服，应视为运营风险披露。[一手](https://www.fgnwct.com/about.html) |
| **网云穿** | 当前官网确认免费用户可建 1 个端口，性能低于付费；**没有公开写准确免费带宽和流量**。旧教程常见「1 Mbps、1 GB/月」，只能标为过期资料。[一手](https://www.wangyunchuan.com/) | 公开首页没有写当前实名方式，**没查到**。 | 平台分配访问地址；自定义域名/备案的当前细则需登录，公开页没查到。 | 官网公开页强调免费、VIP 和企业服务，但精确个人价格在登录后，**没查到可复核的当前价目表**。[一手](https://www.wangyunchuan.com/) | 官网场景含网站、游戏、远程办公。[一手](https://www.wangyunchuan.com/) 未查到访客中间页。 |
| **ngrok.cc（Sunny-Ngrok）** | 官网仍提供免费服务器/免费隧道入口；当前公开页没有稳定披露免费带宽。2024 年教程常写美国免费节点约 128 KB/s，属于二手旧值。[一手](https://www.ngrok.cc/) | 第三方教程提到实名和小额验证费，官网公开页没查到当前规则，故记为**未验证**。 | 常见平台子域为 `*.ngrok.cc`；自定义域名和国内节点备案条件以控制台为准，本轮没登录。 | 官网列有不同节点和带宽的付费服务器，但动态价格页无法稳定抓取，**没查到可复核的完整当前档位**。[一手](https://www.ngrok.cc/) | 历史上以微信开发、本地 Web 调试著称。[一手](https://www.ngrok.cc/) 未查到国内服务的强制中间页。注意：国际 `ngrok` 免费 HTML 流量有官方警告中间页，这是另一家公司，不能移植到 ngrok.cc。[一手：ngrok 免费限制](https://ngrok.com/docs/pricing-limits/free-plan-limits) |
| **神卓互联** | 官网宣传硬件配套可获免费映射；可检索到的旧官方文章曾写 1 Mbps、2 条映射、3 天试用，不足以证明 2026 年长期免费档。[一手](https://www.shenzhuohl.com/) | 首页要求账户/设备绑定，具体实名流程未在公开页找到，**未验证**。 | 提供平台域名及端口映射；自有域名、备案条件公开页没查到。 | 2026 活动页出现约 ¥398/3 年、5 Mbps/2 条及 ¥598 长期、10 Mbps/3 条等促销组合，需注意它与硬件绑定及活动期限。[一手](https://www.shenzhuohl.com/) | 主打 NAS、远程设备、监控和游戏联机，不是专门的微信调试产品。未查到访客中间页。 |
| **自建 frp + 云服务器** | frp 软件开源免费；限制取决于 VPS 带宽、流量、线路和配置，不存在统一免费档。[一手：frp](https://github.com/fatedier/frp) | 阿里云/腾讯云等大陆账户和服务器购买通常要求实名；具体取决于云商和区域。 | 大陆节点提供网站需要域名 ICP 备案；香港/海外源站不要求大陆备案，但一旦使用大陆 CDN 节点仍需备案。[一手：腾讯云备案说明](https://cloud.tencent.com/document/product/243/18905) | 腾讯云/阿里云轻量促销可低至几十元/年，但常见大陆轻量套餐只有 3 Mbps；香港套餐带宽看似更高，却不保证回大陆线路质量。[用户原话](https://www.v2ex.com/t/1178018) | 无平台强制中间页；代价是购买、部署、证书、域名、安全、升级和监控都由开发者承担。 |

### 2.2 免费档到底有多难用

**不能用一句「都是 1 Mbps」概括。** 能从当前一手页面确认的至少有三种约束模型：

- cpolar 是 **1 Mbps + 4 条隧道 + 随机地址**。[一手](https://www.cpolar.com/pricing)
- 飞鸽是 **0.5 Mbps + 端口数 + 签到**。[一手](https://www.fgnwct.com/price.html)
- SakuraFrp 是 **10 Mibps + 2 条隧道 + 5 GiB/月 + 节点不保证**；官方 FAQ 直接说明免费节点可能因滥用或用户过多而长期超载。[一手](https://www.natfrp.com/) [一手](https://doc.natfrp.com/faq/network.html)

若把 30 MB 当作一个试玩首包：

- 1 Mbps 的纯理论下限为 \(30 \times 8 / 1 = 240\) 秒，即 **4 分钟**；
- 0.5 Mbps 的纯理论下限为 **8 分钟**；
- 10 Mibps 的纯理论下限约 **24 秒**。

这是单位换算，不是实测。TCP 慢启动、TLS、丢包、共享节点拥塞、浏览器并发和开发机上行都会继续拉长时间。Unity 官方只给出「移动 Web 必须尽量减小构建、慢网络会导致跳出」的方向性结论，没有给 30 MB 在国内隧道上的统一成绩。[一手：Unity Web 移动端优化](https://docs.unity3d.com/6000.7/Documentation/Manual/web-optimization-mobile.html)

**没查到**可信的「同一 30 MB 文件、同一时段、三大运营商、各免费隧道横向下载」实测帖。因此，DESIGN 可以用 4 分钟作 1 Mbps 的容量推演，但不能写成用户实测。

免费档的另一组痛点比标称带宽更重要：

- **地址会变**：免费随机域名让已经发到群里的链接失效，直接破坏「发出去等反馈」。[一手：NATAPP](https://natapp.cn/) [一手：cpolar](https://www.cpolar.com/pricing)
- **共享节点无保证**：标称 10 Mibps 不等于晚高峰或热门节点能持续跑满。[一手：SakuraFrp 网络 FAQ](https://doc.natfrp.com/faq/network.html)
- **实名和备案分开存在**：用海外节点免去的是大陆 ICP 备案，不等于可以匿名使用隧道服务。[一手：SakuraFrp 实名](https://doc.natfrp.com/faq/realname.html) [一手：SakuraFrp 网络 FAQ](https://doc.natfrp.com/faq/network.html)
- **玩家可能直接连开发机**：开发机休眠、切网、端口重启或上行被占用都会让链接失效；这也是上传静态构建通常比隧道更可信的工程原因，但本轮没有量化两类需求比例。

### 2.3 用户吐槽与真实故障

逐个小厂工具搜索时，大量结果是「永久免费」「白嫖」「最强」教程或推广软文，缺少可归因、可复现的用户抱怨。对 cpolar、NATAPP、花生壳、飞鸽、网云穿、ngrok.cc、神卓，**没有为每一家找到足够可信且在 2025/2026 发布的用户原话**；不应用旧 CSDN 教程代替口碑。

能核实的跨工具问题包括：

- 一位阿里云香港轻量用户称「就是个展示网页 ping 300~400ms」，工单答复称套餐公网带宽是峰值上限、不作业务承诺，高峰资源争抢导致延迟和丢包属于可能情况；评论还有「我的 30M……做正常网站，还被限速到 5M 了」。[用户原话，2025-11](https://www.v2ex.com/t/1178018)
- 一位自建 Tailscale DERP 用户写道「服务器只有 3Mbps 的带宽，看视频并不流畅」，说明自建组网也会回到 VPS 带宽问题。[用户原话，2025](https://www.v2ex.com/t/1113865)
- SakuraFrp 自己承认免费节点可能超载，这比无来源的用户吐槽更可靠，也说明「免费 10 Mibps」不能按专线理解。[一手](https://doc.natfrp.com/faq/network.html)

### 2.4 ZeroTier / Tailscale 为什么不是玩家分享替代品

ZeroTier 与 Tailscale 的核心是把设备加入虚拟网络。玩家通常要安装客户端、登录或被邀请加入网络；这违反 playtest.run「玩家不安装、不登录」的硬约束。[一手：ZeroTier 加入网络](https://docs.zerotier.com/start/) [一手：Tailscale 分享设备](https://tailscale.com/kb/1084/sharing)

它们适合开发者远程访问自己的机器或小团队设备互联，不适合把一个普通 HTTPS 链接丢进微信群。Tailscale 官网域名当前并非整体屏蔽，但大陆直连海外 DERP 的性能因线路而异，国内用户常自建 DERP；「能访问官网」不能推导为「玩家路径可用」。[二手测量：tailscale.com](https://en.greatfire.org/domain/tailscale.com) [用户原话](https://www.v2ex.com/t/1113865)

### 2.5 公司与商业模式

- **贝锐科技/花生壳**：官网自称拥有 1.2 亿注册用户、150 万企业客户、26 亿连接设备；这些是公司自报口径，不是审计数据。未查到公开上市信息或可复核营收，宜描述为成熟、非上市的远程连接 SaaS/硬件公司，不应写具体营收。[一手：贝锐关于我们](https://www.oray.com/about/)
- **cpolar**：网站主体为北京嘉迅科技有限公司；公开页面没有披露用户规模、融资或营收。本轮只可确认其以免费获客、带宽/固定域名/更多隧道订阅升级为商业模式。[一手：cpolar](https://www.cpolar.com/) [一手：定价](https://www.cpolar.com/pricing)
- **SakuraFrp**：页面显示运营名为 iDea Leaper，并以捐赠/会员、流量和节点能力售卖服务；本轮没有在其官网找到清晰法人公司、融资或营收披露，不能确认是个人项目还是公司化团队。[一手](https://www.natfrp.com/)
- **飞鸽**：官网页脚/关于页指向深圳市猿人网络科技有限公司，并主动说明规模较小、难以提供专业客服；商业模式是低价带宽套餐和香港线路。[一手](https://www.fgnwct.com/about.html) [一手](https://www.fgnwct.com/price.html)
- **网云穿**：官网主体显示河南图易网络科技有限公司，官网自报用户数不能视为第三方规模证明；以个人 VIP、企业服务和带宽升级变现。[一手](https://www.wangyunchuan.com/)
- **神卓互联**：官网主体为江苏神卓网络科技有限公司，商业模式明显包含硬件、长期映射套餐和企业远程连接。[一手](https://www.shenzhuohl.com/)

共同模式是：免费档获取搜索和开发者流量，再卖固定域名、更多隧道、更高带宽、HTTPS、专属节点、流量包、硬件或企业支持。它们解决的是「让内网服务被访问」，不是「玩家是否加载成功、玩到了哪一步、如何反馈」。

## 3. 国内 Web 游戏/小游戏开发者如何给人试玩

### 3.1 微信小游戏

微信体验版可以生成二维码，但不是任意人扫码即玩：

- 体验成员需要由管理员添加微信号；官方帮助页按主体和账号状态给出 **15/30/60/90 人**不同上限。[一手](https://kf.qq.com/faq/170302zeQryI170302beuEVn.html)
- 微信小游戏可视化制作工具的预览文档另写有最多 50 人预览，这是特定工具口径，不能与普通体验成员额度混为一谈。[一手](https://developers.weixin.qq.com/minigame/introduction/gamemaker/minigame/release)
- 正式发布前必须完成版本审核和小程序备案；企业主体必须先微信认证才能提交备案。官方给出的发布总时效估算是：不开虚拟支付的 IAA 游戏约 **13–37 个工作日**，开虚拟支付的 IAP 游戏约 **9–22 个工作日**。[一手](https://developers.weixin.qq.com/minigame/introduction/guide)
- 企业通过微信认证验证主体身份的标准费用为 **¥300/次**；已有认证公众号/服务号复用资质可免重复支付。个人主体的当前认证政策与类目能力不同，不应笼统写成「所有小游戏都必须付 ¥300」。[一手：小程序介绍](https://developers.weixin.qq.com/miniprogram/introduction/) [一手：复用认证资质](https://kf.qq.com/faq/170427jqmmUB170427UBVJjQ.html)

它适合已经决定进入微信生态、愿意做平台适配和运营的作品，不适合在做完一个 Web 构建后立刻发给几十个不认识的人。一个 2026 年 V2EX 作者称代码「一周」完成，但认证、审核、备案「折腾了一个多月」，并总结「开发可能只占整个项目难度的 20%，真正困难的是审核、运营和获客」；这是个人经历，不是全行业时长统计，但与官方 13–37 工作日流程相互印证。[用户原话](https://www.v2ex.com/t/1221245)

### 3.2 抖音小游戏

抖音开放平台支持个人和企业主体创建小游戏；测试版可生成二维码扫码体验，后台最多保留 25 个测试版本。正式首发前有质量保障审核，官方说明通常 1–3 个工作日，之后还有内容与版本审核。[一手](https://developer.open-douyin.com/docs/resource/zh-CN/mini-game/guide/minigame/examineguide)

这仍然是平台内小游戏流程，不是通用 Web 构建托管。本轮**没有在当前官方文档中查到测试成员人数上限**，不能类比微信的 15/30/60/90。

### 3.3 Cocos Creator

Cocos Creator 的浏览器预览适合开发机自测；官方手机预览要求手机和电脑处于同一局域网/网段，通过编辑器显示的局域网地址或二维码访问。[一手](https://docs.cocos.com/creator/3.8/manual/zh/editor/preview/)

给远端朋友试玩仍需构建并发布到 Web 服务器、小游戏平台或原生平台。也就是说，编辑器解决 M1「自己跑起来」，没有自动解决 M2「别人点开」。

### 3.4 4399、TapTap、好游快爆与 indienova

- **4399 开放平台**提供开发者注册、提交、审核和上线流程，目标是进入 4399 渠道而非私密临时链接；当前公开文档对不同 H5/小游戏品类和资质的说明分散，本轮没有查到一个可复核的「任意 Web 构建即时试玩」入口。[一手](https://open.4399.cn/)
- **TapTap 篝火测试**是招募式封闭测试：开发者创建测试、设置问卷和资格，玩家需要 TapTap 账号并获得资格，主要面向 Android/iOS 安装包，不是零门槛网页分享。[一手](https://developer.taptap.cn/docs/store/operation/test/test-skills/gameplaytest/)
- **好游快爆开发者平台**提供预约、测试与分发能力，玩家侧依赖好游快爆平台与应用安装；它解决测试招募和游戏社区，不是通用浏览器托管。[一手](https://dev.3839.com/)
- **indienova**提供作品页、开发日志、活动与社区曝光，下载/试玩通常由作者填写外部链接或参与具体活动；本轮没查到面向所有作者的通用即时 Web 托管承诺。[一手](https://indienova.com/)

这些渠道说明「找测试者、收反馈、曝光」有需求，但也说明做完整发现生态会直接进入已有平台的主战场。

### 3.5 BOOOM、CiGA、腾讯高校赛

- **机核 BOOOM**使用自有活动站完成组队、提交、评审和作品展示；活动本身提供作品试玩/下载入口，而不是统一要求作者去 itch.io。[一手：BOOOM 2025](https://site.gcores.com/booom2025/)
- **CiGA Game Jam 2026**使用 GmHub 作为报名和提交平台。indienova 的官方合作报道回顾 2025 年有 2,000+ 名开发者完成 450+ 个原型，这是活动方/媒体口径，但足以说明国内 Game Jam 原型供给不是个别现象。[一手/合作报道](https://indienova.com/indie-game-news/2026-ciga-game-jam-sign-up/)
- **腾讯高校游戏创意制作大赛 2026**要求提交可供至少 15 分钟试玩的 Demo、PPT、图片和文字材料，走大赛自己的报名提交系统，不是 itch.io 链接征集。[一手](https://gameinstitute.qq.com/awards2026/culture)

因此，「国内 Jam 都用 itch.io」与事实不符。大型活动倾向自建站或专用平台，以便处理账号、资格、内容审核、评审和作品长期展示。

### 3.6 B 站 UP 主与网盘 Demo

B 站是发现和传播渠道，不是游戏包托管。可检索案例中，独立开发者先在视频简介/评论放百度网盘 Demo，收集粉丝反馈后迭代；后来因为包体变大、网盘会员体验等原因转向 Steam。[二手案例，含作者经历转述](https://www.gameres.com/889086.html)

2025/2026 的视频简介还能看到百度网盘、夸克网盘、itch.io、Steam 和群文件等混合方式，但本轮没有做足量随机抽样，**不能给出各种方式的占比**。能确认的是：视频能带来测试者，下载、解压、系统兼容和版本更新仍是另一条摩擦很高的路径。

## 4. itch.io 与境外静态托管在大陆的真实可用性

### 4.1 2026 年测量快照

| 服务/域名 | 可观察结果 | 证据边界 |
|---|---|---|
| `itch.io` | GreatFire 显示域下 26/26 个已测样本被屏蔽，最近测试为 2026-08-30。[二手测量](https://zh.greatfire.org/domain/itch.io) | 这比「慢且不稳」更严重，但仍是第三方样本，不等于每条线路每一刻都绝对打不开。 |
| `vercel.app` | 131 个样本中 127 个被屏蔽。[二手测量](https://zh.greatfire.org/domain/vercel.app) | Vercel 官方同时说明其没有中国大陆节点，无法保证大陆性能或可用性。[一手](https://www.vercel.com/kb/guide/accessing-vercel-hosted-sites-from-mainland-china) |
| `netlify.app` | 79 个样本中 72 个可访问、3 个被屏蔽，其余为间歇受扰或无结论。[二手测量](https://zh.greatfire.org/domain/netlify.app) | 反驳「所有境外 Pages 默认域都已被墙」，但少量成功也不能替代玩家真机测试。 |
| `github.com` | 960 个已测 URL 中 38 个被屏蔽、538 个间歇受扰、350 个可访问、34 个无近期结论，最近测试为 2026-09-04。[二手测量](https://zh.greatfire.org/domain/github.com) | 这是 GitHub 主域混合样本，不是 OAuth 登录端点或 `github.io` 的专项 SLA。 |
| `notion.so` | 对 `https://notion.so` 最近 8 次有结论测试中 88% 受干扰，最近一次为 2026-08-16。[二手测量](https://zh.greatfire.org/https/notion.so) | 说明「境外成熟 SaaS + CDN」也会发生主站、图片、同步端点表现不一致。 |

对应的用户原话：

- 中文 itch.io 社区有用户在 2025 年称「浏览器能登录，但是 itch 程序登录不上去」，说明同一服务的网页和客户端端点可能表现不同。[用户原话](https://itch.io/topic/5661344)
- V2EX 用户概括为「vercel.com 国内访问很快，部署完的 vercel.app 被稳稳地墙了」，域名控制台可达不代表作品默认域可达。[用户原话](https://www.v2ex.com/t/1091687)
- 2026-09-02 有用户报告 `github.io`「突然无法访问」，评论中不同地区和运营商结果不一致，恰好说明不能用开发者自己的电脑打开一次就宣布可用。[用户原话](https://www.v2ex.com/t/1239025)

### 4.2 Cloudflare Pages 与微信

Cloudflare Pages/Workers 的大陆问题更像 IP 段、域名和运营商组合的波动，未找到像 `itch.io` 那样可直接引用的「全部样本被屏蔽」结论。一位 2026 年开发者仍能把 Codex 做的 Web 游戏部署到 Cloudflare Pages + D1 并公开征集反馈，证明它不是任何时候都不可用；另一面，掘金作者称把 `*.workers.dev` 链接发给朋友后「手机一点——打不开」，移动网络尤其不稳。[用户原话：可用案例](https://www.v2ex.com/t/1227978) [用户经历：手机打不开](https://juejin.cn/post/7653686112276496418)

微信还有独立于 GFW 的链接治理：

- 《微信外部链接内容管理规范》允许限制具体链接、域名或 IP 在微信内直接打开，严重或反复违规时可永久处理。[一手](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)
- 腾讯客服明确说，未备案一级域名分享达到一定次数后，再分享可能仅自己可见；未备案并非每次首次分享都必然拦截，但规模化传播存在政策摩擦。[一手](https://kf.qq.com/faq/170118UnqeUZ170118mUb6fu.html)
- 一个 2025 年 V2EX 案例中，正常宣传页因上传接口风险触发域名封禁，投放页面和存量小程序内嵌网页一起失效；作者称申诉多日没有解决，后续腾讯账号回复确认解除。这说明即使内容初衷正常，共用主域下的 UGC/文件能力也会扩大整域风险。[用户原话及腾讯回复](https://www.v2ex.com/t/1158792)

因此，`*.playtest.run` 的 slug 隔离能减少作品 URL 冲突，却不能假设微信只封单个 slug 而不会处理主域、IP 或分享接口。必须准备主动内容治理、恶意文件隔离、域名/IP 监测、下线和申诉流程。

### 4.3 EdgeOne Makers：最值得关注的国内相邻产品

EdgeOne Makers/Pages 支持 Git 仓库、HTML/ZIP 和 AI 对话生成部署；官方内容直接用「做一个贪吃蛇并分享给朋友」说明场景。这同时验证了需求，也构成一键部署层面的直接竞争。[一手](https://pages.edgeone.ai/resources/edgeone-pages-mcp-turns-ai-code-into-live-websites) [一手](https://pages.edgeone.ai/document/ai-dialogue-deployment-deploy-project-with-one-sentence-using-skill)

但其大陆规则留下了 playtest.run 的空间：

- 项目选择中国大陆节点或全球含大陆节点时，自定义域名必须完成 ICP 备案。[一手](https://pages.edgeone.ai/zh/document/domain-overview)
- 平台项目域名在中国大陆网络环境需使用控制台生成的预览链接，链接只有 **3 小时**有效；非大陆网络可直接访问。稳定分享需要绑定自定义域名。[一手](https://pages.edgeone.ai/zh/document/domain-overview)
- 选择「全球可用区（不含中国大陆）」可免备案，但也失去大陆境内节点。[一手](https://pages.edgeone.ai/zh/document/domain-overview)

用户讨论也非常直接：「国内没有备案的可能不能访问，特别是微信」；另一位作者部署 `*.edgeone.cool` 后发现电脑能开而手机打不开，最后选择自有域名备案。两者都是个体经历，不应推导成所有 EdgeOne 默认域都不可达，但说明真机和备案是绕不过去的验证点。[用户原话](https://cn.v2ex.com/t/1123748) [用户经历](https://juejin.cn/post/7653686112276496418)

### 4.4 Gitee Pages、对象存储和自定义域名

Gitee Pages 已无法作为当前通用答案；2025 年用户实际查找时称服务菜单已经下线，本轮没找到 Gitee 对公众免费 Pages 恢复服务的官方公告。[用户经历](https://juejin.cn/post/7653686112276496418)

阿里云 OSS、腾讯云 COS 可以做静态网站，但国内用户讨论指出默认对象存储域访问 HTML 时可能强制下载，稳定网站仍需自有域名、备案和 CDN/静态网站配置。[用户原话](https://cn.v2ex.com/t/1123748) 本轮没有逐一购买验证各云商 2026 控制台，故不把这条用户说法升级为一手平台规则。

自定义域名可以绕开已被集中屏蔽的平台默认域，但不能绕开底层 IP、跨境线路、微信整域风控或大陆 CDN 的备案要求。它是降低共享域连带风险的方法，不是万能解法。

## 5. AI 做小游戏的人怎么分享

目前能找到的高信号案例是：

- 2026 年一位 V2EX 用户称两天用 Codex 做出游戏，使用 Cloudflare Pages + D1 发布；帖子回复直接反馈玩法、触屏和屏幕溢出问题。[用户原话](https://www.v2ex.com/t/1227978)
- 一位中文 Claude Code 用户在一小时内生成 Web 游戏，使用 GitHub Pages 公开分享。[用户原话](https://www.cocoloop.cn/t/topic/7250)
- 一位微信小游戏作者用 AI 辅助快速完成代码，但认证、备案、审核和获客耗时远超开发。[用户原话](https://www.v2ex.com/t/1221245)
- EdgeOne Makers 把 AI 生成 HTML/ZIP 后直接发布公网链接产品化，并提供面向 AI 编码工具的 Skill/MCP 接入。[一手](https://pages.edgeone.ai/resources/edgeone-pages-mcp-turns-ai-code-into-live-websites) [一手](https://pages.edgeone.ai/document/ai-dialogue-deployment-deploy-project-with-one-sentence-using-skill)

这些案例共同显示：AI 把 M0「写出来」和 M1「本机跑起来」压缩到小时或天，M2「普通朋友稳定打开」、M3「知道哪里坏了」没有随代码生成自动消失。

**没查到/不足以验证的部分：**

- 没有找到足量、可归因的 2025/2026 原帖来量化 Cursor、Trae、Claude Code、豆包、DeepSeek 用户各自选择 Vercel、GitHub Pages、EdgeOne、OSS/COS 的比例。
- 豆包/DeepSeek 搜索结果以教程、营销稿和平台集成为主，缺少作者明确说明「用它做了游戏—发给朋友—朋友打不开」的完整链路。
- 小红书、即刻和部分 B 站评论不易被公开搜索稳定索引；不能因为没搜到就判断这些社区没有需求。
- 不能从几篇爆款帖子推算市场规模或付费意愿。

## 6. 四个重点问题

### 6.1 （a）是否存在真实需求，规模感受如何

**存在，证据强度为「多场景反复出现」，尚不是可量化 TAM。**

1. 腾讯 EdgeOne 已把「AI 生成小游戏—一键部署—发给朋友」写进产品场景，属于供给侧验证。[一手](https://pages.edgeone.ai/document/ai-dialogue-deployment-deploy-project-with-one-sentence-using-skill)
2. 开发者会公开寻找几十名陌生人，验证「会不会点、能否秒懂、第一局能否完成、是否想再来」，并需要加载、UI、难度反馈；这与 DESIGN 的打开、加载、错误和一句话反馈高度重合。[用户原话](https://www.v2ex.com/t/1224053)
3. CiGA 2025 的 2,000+ 开发者、450+ 原型以及 BOOOM 的周期性活动说明短周期原型供给持续存在。[合作报道](https://indienova.com/indie-game-news/2026-ciga-game-jam-sign-up/) [一手](https://site.gcores.com/booom2025/)
4. 微信、抖音、TapTap、好游快爆都提供体验/测试流程，说明「上线前给一小群人试」是平台标准能力；它们的白名单、审核、账号和安装门槛，正是通用 Web 链接的机会窗口。[一手：微信](https://kf.qq.com/faq/170302zeQryI170302beuEVn.html) [一手：抖音](https://developer.open-douyin.com/docs/resource/zh-CN/mini-game/guide/minigame/examineguide) [一手：TapTap](https://developer.taptap.cn/docs/store/operation/test/test-skills/gameplaytest/)

规模上只能说「足以形成早期用户池」，不能从本轮资料推到数十万付费用户。最合理的首批切入仍是 AI Web 作品、Game Jam Web 构建和独立开发者私测，而不是整个小游戏发行市场。

### 6.2 （b）香港边缘 + 不备案有没有先例，会遭遇什么

**有先例，但先例证明的是「可以上线」，不是「天然稳定」。**

- NATAPP 和飞鸽都公开售卖香港免备案线路，说明开发工具用香港节点服务大陆用户是现成商业方案。[一手：NATAPP](https://natapp.cn/) [一手：飞鸽](https://www.fgnwct.com/price.html)
- EdgeOne 的「全球可用区（不含中国大陆）」也允许未备案自定义域名，只是明确不分配大陆节点。[一手](https://pages.edgeone.ai/zh/document/domain-overview)
- Notion、Vercel、Cloudflare Pages 等境外服务在大陆出现不同程度的域名、资源和线路不稳定，说明「国际大厂 + 全球 CDN」仍不能替代大陆实测。[二手测量：Notion](https://zh.greatfire.org/https/notion.so) [一手：Vercel 说明](https://www.vercel.com/kb/guide/accessing-vercel-hosted-sites-from-mainland-china)
- 香港普通公网回大陆的用户体验可能很差。阿里云香港轻量用户报告 300–400 ms；云厂商回复称峰值带宽不保证。这与机房物理距离无关，关键是三网回程、拥塞和带宽是否独享。[用户原话](https://www.v2ex.com/t/1178018)
- 服务商自测常宣称 CN2 GIA 晚高峰可维持 30–50 ms、低于 1% 丢包，但这类数据来自售卖线路的服务商，不能替代 playtest.run 自己从大陆三网、不同城市、微信 WebView 做的长期拨测。[二手/利益相关自测](https://hengxun.cn/news/content/10788.html)
- 微信可独立限制未备案域名的传播频率，并可按链接、域名或 IP 处置；香港节点不能规避这一层。[一手：腾讯客服](https://kf.qq.com/faq/170118UnqeUZ170118mUb6fu.html) [一手：外链规范](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)

所以香港首节点应配套：三网持续拨测、晚高峰告警、静态资源分层、连接超时可见化、域名/IP 风控监测、快速切换节点，以及明确告诉开发者「大陆可达是目标，不是 SLA」。在真机 spike 之前，DESIGN 中「30–60 ms、接近国内 CDN」只能写作目标假设。

### 6.3 （c）免费穿透到底多难用

结论不是「一律不能用」，而是**它们适合短时开发调试，不适合把几十 MB Web 游戏稳定交付给未知玩家**：

- 最慢的已确认免费档 0.5 Mbps，30 MB 理论至少 8 分钟；cpolar 的 1 Mbps 理论至少 4 分钟。[一手：飞鸽](https://www.fgnwct.com/price.html) [一手：cpolar](https://www.cpolar.com/pricing)
- SakuraFrp 免费档标称 10 Mibps，但有 5 GiB/月和共享节点超载风险。[一手](https://www.natfrp.com/) [一手](https://doc.natfrp.com/faq/network.html)
- 免费随机域名、签到、隧道数、节点维护和开发机在线状态会让链接寿命比带宽更早成为问题。
- **没有找到 30 MB 横向实测**；4/8 分钟必须标成理论值。

这支持「静态目录上传为主、隧道为辅」，但不支持凭印象写「所有工具免费档都是 1 Mbps」。

### 6.4 （d）境外服务 + GitHub 登录的接受度

**核心开发者群体对 GitHub 有认知和账号的概率高，但 GitHub OAuth 不能成为首次成功的单点依赖。**

- 《2025 中国开源年度报告》的媒体转述称，中国在 GitHub 的活跃开发者约 210 万、全球第三；开放原子基金会另一份 2025 报告称中国活跃开源开发者约 263 万。两者统计口径不同，但都说明存在很大的 GitHub 熟练人群。[二手：开源社报告转述](https://www.ithome.com/0/968/251.htm) [一手机构发布：开放原子基金会](https://www.openatom.org/journalism/detail/bdPGZ0UY6bn5)
- 同一时间，GreatFire 对 `github.com` 的样本中有 538/960 间歇受扰；GitHub Pages 也有 2026 年用户报告突发不可达。[二手测量](https://zh.greatfire.org/domain/github.com) [用户原话](https://www.v2ex.com/t/1239025)
- GitHub 活跃开发者数量不能证明 AI 新手、Game Jam 学生、Cocos/Construct 作者都有 GitHub 账号，也不能证明 OAuth 回调在微信或校园网稳定。

因此，DESIGN 的「首次匿名 24 小时链接」非常关键。GitHub 登录适合在用户已经看到链接成功后，用于认领、延长和管理；不宜在第一条命令前强制。是否增加邮箱、微信或国内代码平台登录应由真实转化漏斗决定，而不是本轮调研先扩范围。

## 7. 未查到、未验证清单

- 未注册任何穿透账户，故小厂的 2026 实名方式、免费节点实际数量、控制台价格和域名后缀可能比公开页更细。
- 未找到 cpolar、NATAPP、花生壳、飞鸽、网云穿、ngrok.cc、神卓每家都对应的近期高质量用户吐槽；找不到就是找不到。
- 未做 30 MB 文件在免费隧道上的真实下载测试，也未测 WebAssembly 编译、Range 请求、缓存命中和微信 WebView 首屏。
- 未从大陆电信、联通、移动、广电、校园网分别测试香港节点；未验证晚高峰、IPv4/IPv6、DNS 和不同省份。
- 未验证 `playtest.run`/候选子域在微信、QQ、各国产浏览器和腾讯网址安全中心的初始信誉。
- 未获得 GitHub 登录在目标人群中的转化率或失败率；210 万活跃开发者不是登录接受度调查。
- 未找到国内开发者用 Cursor、Trae、Claude Code、豆包、DeepSeek 分享游戏时各托管方案的可靠市场份额。
- 未系统抽样 B 站 Demo 链接，不能回答网盘、Steam、itch.io、QQ群文件的占比。
- 未找到 CiGA 因 itch.io 大陆可用性而「迁移平台/做镜像」的官方声明；只能确认 2026 使用 GmHub，不能把因果补出来。
- 本报告不是法律意见。香港托管、ICP 备案、游戏版号、数据跨境、内容审核和未成年人规则需要另行做合规审查。

## 8. 对 playtest.run 的含义

### 8.1 支持 DESIGN 现有判断的发现

- **体验第一得到强支持。** 现有替代品各自多一步：穿透要实名/保活，微信要加体验成员，TapTap 要登录获资格，网盘要下载，Pages 默认域可能打不开。`playtest ./dist` 后直接得到可发微信的链接和二维码，仍是有价值的组合。
- **玩家零登录、零安装是正确硬约束。** ZeroTier/Tailscale、平台体验版和原生测试都不能替代这条路径。
- **上传优先、隧道辅助在工程上合理。** 上传避免开发机休眠、上行不足、随机域名和共享节点问题；但本轮没有需求占比数据，不应把「上传占绝大多数」写成已验证事实。
- **香港作为首个不备案节点有现实先例。** NATAPP、飞鸽和 EdgeOne 海外区都证明可以这样上线；关键差异应是大陆三网真机质量和失败诚实度，而不只是地理位置。
- **「知道结果」有明确用户语言。** 用户想知道是否点进来、是否秒懂、第一局是否完成、哪里加载/UI/难度有问题；打开、设备、来源、加载、JS 错误和一句话反馈正好覆盖最小集合。[用户原话](https://www.v2ex.com/t/1224053)

### 8.2 与 DESIGN 冲突或需要收窄的地方

1. **把「国内穿透免费档约 1 Mbps」改为分布描述。** 可写「0.5–10 Mibps，常叠加随机域名、流量、签到、节点和隧道数限制；cpolar 为 1 Mbps，SakuraFrp 是明显反例」。
2. **把「itch.io 在大陆慢且不稳」更新为更强且带日期的风险。** 截至 2026-08-30，GreatFire 的 26/26 样本为屏蔽；应同时保留「第三方测量、需真机复核」的限定。
3. **不要把所有境外静态托管写成一个结论。** `vercel.app` 风险极高，`netlify.app` 大多数样本仍可达，Cloudflare Pages 波动，GitHub Pages 间歇故障；差异本身说明 playtest.run 需要持续拨测而不是静态竞品表。
4. **把「香港 30–60 ms、接近国内 CDN」降级为待验证目标。** 只有精品回程可能做到；普通香港轻量云有 300–400 ms 用户报告。第一条 `docs/spikes/` 应覆盖大陆三网、多城市、晚高峰和微信 WebView。
5. **收窄「开发者基本都有 GitHub、GitHub 基本可用」。** 中国 GitHub 活跃用户很多，但访问有大面积间歇干扰，目标用户也包含非传统开发者。保持匿名首次发布，登录放在成功之后。
6. **不要暗示国内 Jam 仍主要依赖 itch.io。** BOOOM 用自有站，CiGA 2026 用 GmHub，腾讯大赛用自己的提交系统。itch.io 的阻断是机会证据，但不是所有国内活动当前工作流的描述。
7. **「微信是主分发渠道」方向可信但未量化。** 本轮有微信体验版、外链规则和用户案例支撑重要性，没有渠道占比调查；应保留为首批用户假设。

### 8.3 产品建议

- **v0.1 不做发现、榜单、孵化。** BOOOM、GmHub、TapTap、好游快爆、indienova 已拥有活动、社区和测试招募关系；playtest.run 应先做它们也需要的基础设施。未来更轻的路径是导入外部测试者、为活动提供私密链接/数据摘要、导出结果或成为合作方，而不是抢内容社区。
- **把分析做成「一次试玩是否成功」而不是通用 BI。** 默认漏斗建议只覆盖：门禁页打开 → 点击开始 → HTML/关键资源加载 → 首次可交互或开发者上报的首局完成 → 一句话反馈。按来源、设备、浏览器和网络错误切片，优先回答「哪里没打开」而非堆图表。
- **把 EdgeOne Makers 当成必须持续跟踪的相邻竞品。** 它已经做到 AI/ZIP → 链接；playtest.run 要胜出，必须在玩家门禁、微信内成功率、游戏构建兼容、错误采集、版本语义、二维码和反馈闭环上明显更好。
- **为微信整域风险设计隔离与响应。** 上传内容域与控制台域保持彻底分离；限制主动下载/任意跳转/危险 MIME；做恶意文件扫描、投诉和一键下线；监控 slug、主域和 IP 在微信内的状态；预备申诉证据。不能承诺「子域隔离就不会连坐」。
- **匿名首发是竞争优势，不只是增长技巧。** `playtest ./dist` 应先成功，再提示用 GitHub 认领；OAuth 不可用时不能让已上传作品消失。
- **把香港路径的验证设为发布准入。** 至少记录电信/联通/移动 × 华南/华东/华北 × 白天/晚高峰 × 微信 WebView/系统浏览器的 DNS、TCP/TLS、TTFB、首包吞吐、完整 30 MB 下载、WASM 启动和丢包；验证前界面应明确「大陆网络可能因地区和运营商失败」。
- **最重要的差异不是多功能，而是同一件事做得更可靠。** 国内已经有很多「生成公网 URL」的工具；机会在于把 URL 之后的玩家成功率、错误可见性和反馈闭环做得比它们好。
