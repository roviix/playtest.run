# 全球「分享 localhost」隧道工具调研

> 调研日期：2026-09-07  
> 范围：使用体验、访客侧体验、限制、定价、商业模式、AI 编程新品，以及这些结论对 playtest.run 的含义。  
> 约束：未注册账号、未安装客户端、未在微信真机打开临时隧道；因此本文不把网页文档检查写成真机验证。

## 0. 读法与证据等级

本文把证据明确分成三类：

- **一手**：厂商官网、官方文档、官方博客、代码仓库、GitHub API 中的仓库元数据。
- **用户原话**：HN、GitHub issue/discussion、产品论坛、社区问答中的原帖或评论。它能证明“有人遇到过”，不能自动代表总体比例。
- **二手/推断**：媒体、竞品比较、公司数据库，以及由多个事实推出的产品判断。无法交叉验证的数字会明确标出。

“没查到”表示在本轮检索范围内没有找到可引用的一手资料，不等于功能一定不存在。价格、限额和 star 数均以 2026-09-07 页面状态为准，未来可能变化。

## 1. 结论先行

1. **“一条命令得到链接”已高度商品化，但“第一次就成功”仍未被做透。** `ssh -R` 路线可以做到零安装、零账号；CLI 路线可以自动重连、显示二维码、检查请求。真正反复出问题的是账号/令牌、Host 校验、HMR/WSS、随机 URL、企业网络封锁、免费服务不稳定，以及访客先撞上平台警告页。[一手·localhost.run 基础用法](https://localhost.run/docs/) [一手·Pinggy 快速开始](https://pinggy.io/docs/) [用户原话·localtunnel 503](https://github.com/localtunnel/localtunnel/issues/726)
2. **中间页本身不是原罪；“没有作品语义、让访客替平台反滥用、甚至要求账号/IP”才是。** itch.io 在移动端强制 Click to Play，Chrome 也要求用户手势才能可靠解锁 Web Audio；这为有作品名、邀请者、版本和明确“开始”动作的门禁页提供正证据。反例是 ngrok 的泛化风险警告、localtunnel 的公网 IP 密码、Loom 手机端注册门槛和 Vercel 登录墙。[一手·itch.io HTML5](https://itch.io/docs/creators/html5) [一手·Chrome Web Audio](https://developer.chrome.com/blog/web-audio-autoplay) [用户原话·Loom 手机注册门槛](https://community.atlassian.com/forums/Loom-questions/Are-people-expected-to-sign-up-to-view-videos-with-public-links/qaq-p/3205171)
3. **“知道结果”不是无人看见，但仍远未成为隧道品类标配。** ngrok、Pinggy、LocalXpose 做的是请求检查与重放；LocalhostVibe 只做实时连接计数；tunr 已宣称代理注入可视反馈、收集浏览器信息和 JS 错误，并让 MCP 读取反馈，是离 playtest.run 最近的一个。但截至 2026-09-07，tunr 仅 18 stars，未找到独立用户采用数据，不能把产品自述当需求验证。[一手·tunr 文档](https://tunr.sh/docs.html) [一手·tunr 仓库元数据](https://api.github.com/repos/ahmetvural79/tunr)
4. **通用隧道在“游戏”上露怯的核心不是不支持字节转发，而是不理解游戏交付。** Unity Web 构建需要正确的 `Content-Encoding`、`application/wasm`，线程构建还涉及 COOP/COEP；HTTPS 页面下联机必须 WSS；大构建又迅速吃完 1 GB 免费流量。多数隧道只转发本地服务器给出的内容，不检查这些条件，也没有“大静态资源改走上传”的路径。[一手·Unity Web 部署](https://docs.unity3d.com/6000.3/Documentation/Manual/webgl-deploying.html) [一手·Unity Transport WebGL/WSS](https://docs.unity3d.com/Packages/com.unity.transport@6.6/manual/websockets.html) [一手·ngrok 免费限制](https://ngrok.com/docs/pricing-limits/free-plan-limits)
5. **开源自托管没有消灭托管生意。** frp 在 2026-09-07 有 109,244 stars，却要求用户自备公网服务器、域名、证书、监控与滥用处置；ngrok 卖的是托管网络、稳定入口、检查/重放、访问策略和企业支持。ngrok 在首次融资前已有 30,000+ 付费客户且收入同比翻倍，是“免费开源替代品很多仍可收费”的直接证据。[一手·frp GitHub API](https://api.github.com/repos/fatedier/frp) [一手·ngrok $50M 新闻稿](https://ngrok.com/press-releases/ngrok-raises-50m-for-ingress-as-a-service)
6. **Cloudflare 免费不是慈善，也不是 playtest.run 可复制的成本结构。** Cloudflare 明说免费流量利用企业流量低谷的闲置容量，同时带来产品改进、增长和付费升级；Tunnel 又把用户带入 DNS、CDN、WAF、Access、Smart Routing 等整套产品。独立香港边缘没有这种全球沉没成本，免费档必须有真实流量上限。[一手·Cloudflare 免费策略](https://blog.cloudflare.com/cloudflares-commitment-to-free/) [一手·Tunnel 免费化](https://blog.cloudflare.com/tunnel-for-everyone/)
7. **AI 隧道新品目前更多是包装与控制面变化，不是新网络原理。** agent 友好主要体现为 `--json`、稳定退出码、MCP、可委派短期凭据和“一句话让 agent 开隧道”；tinyfi.sh 甚至只是给 `ssh -R` 配一份 skill。除 tunr 外，本轮没有看到新品把会话、加载成功、JS 错误、版本和玩家反馈完整连起来。[一手·uplink](https://github.com/firstprinciplecode/uplink) [一手·tinyfi.sh](https://tinyfi.sh/) [一手·kshare](https://github.com/sifxprime/kshare)
8. **“曝光/生态/孵化”没有从隧道市场得到支持。** 隧道用户购买的是快速、可信地把一个具体版本给具体的人，不是公开分发；加公开发现会改变使用时刻、扩大滥用面，也会把产品推向 itch/Steam/Product Hunt。现阶段更有证据的是轻结果层，不是流量平台。

## 2. 成熟工具：从安装到访客打开

### 2.1 首链路径、URL 与访客中间页

| 工具 | 开发者第一次拿链接 | URL 形态与稳定性 | 访客侧 |
| --- | --- | --- | --- |
| **ngrok** | 下载或包管理器安装；自 2023-12 起所有使用都要账号，先配置 authtoken，再运行 `ngrok http 3000`。[一手](https://ngrok.com/blog/tcp-endpoints-require-verification) | 免费账号现在自动分配一个固定 dev domain，例如 `your-assigned-name.ngrok-free.app`；不是过去“每次随机”。付费才有可选 ngrok 域名或自有域名。[一手](https://ngrok.com/docs/pricing-limits/free-plan-limits) | 免费 HTML 浏览器流量先看到“由 ngrok 提供、只在信任发送者时继续”的警告，点 **Visit Site** 后以 cookie 对该域抑制 7 天；付费移除。普通浏览器无法由开发者在服务端替访客加 `ngrok-skip-browser-warning`。[一手](https://ngrok.com/docs/pricing-limits/free-plan-limits) |
| **Cloudflare Quick Tunnel / trycloudflare** | 安装 `cloudflared` 后一条 `cloudflared tunnel --url http://localhost:3000`；无需 Cloudflare 账号或域名。[一手](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/do-more-with-tunnels/trycloudflare/) | 每次生成随机 `*.trycloudflare.com`，无稳定性与 SLA 承诺。Named Tunnel 要账号、Cloudflare DNS 域名和额外配置。[一手](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/do-more-with-tunnels/trycloudflare/) | 官方 Quick Tunnel 文档没有写原生警告/中间页；本轮未真机打开，故只能记为“**没查到官方中间页说明**”，不能写成已验证无中间页。 |
| **localhost.run** | 系统已有 SSH 即可一条 `ssh -R 80:localhost:3000 localhost.run`；免费短隧道不用下载、账号或 token。[一手](https://localhost.run/docs/) | 免费域名会定期变化且限速；注册并添加 SSH key 可延长域名寿命。`$9/月`、按年付的 Custom Domain 提供自有域或稳定 `lhr.rocks` 域。[一手](https://localhost.run/docs/forever-free/) [一手](https://localhost.run/docs/custom-domains/) | 官网未披露警告页或额外点击，本轮未实开。访客直接使用 HTTPS URL 是其公开承诺。[一手](https://localhost.run/) |
| **Pinggy** | 无需下载和账号，一条 `ssh -p 443 -R0:localhost:8000 free.pinggy.io`；SSH 走 443 对只放行常见端口的网络更友好。[一手](https://pinggy.io/docs/) | 免费示例为随机 `*.a.free.pinggy.link`（区域标签可能变化），60 分钟后断开，重开换 URL；Pro 才有持久子域、自定义域和持久 TCP/UDP 端口。[一手](https://pinggy.io/) | 免费浏览器第一次打开显示一次安全确认页；非浏览器不显示。浏览器型客户端可发 `X-Pinggy-No-Screen` 绕过，Pro 不显示。[一手](https://pinggy.io/docs/http_tunnels/screening/) |
| **zrok** | 下载单一二进制后先 `zrok invite` 创建账号，再 `zrok enable`，最后 `zrok share`；比“无账号一条命令”多两个生命周期步骤。[一手](https://zrok.io/) | public share 为 `https://<token>.share.zrok.io`；token 可临时或 reserved，reserved 可自定义名称。[一手](https://zrok.io/pricing/) | 未验证信用卡的免费账号，其 public share 第一次显示反钓鱼页；按钮设置一周 cookie。验证信用卡即可在 `$0` 档移除；也可由客户端发 `skip_zrok_interstitial`。[一手·实际页面文本](https://auth.kamilmemory.share.zrok.io/) [一手·配置说明](https://github.com/openziti/zrok/commit/ace8bae54f47b37ce08ce9ee228c825c84be2b52) |
| **bore** | 下载/Cargo 安装后 `bore local 8000 --to bore.pub`；公共实例不要求账号，也可一条命令自托管 server。[一手](https://github.com/ekzhang/bore) | 输出 `bore.pub:<随机端口>`，是原始 TCP 地址，不是浏览器友好的 HTTPS 子域名。[一手](https://github.com/ekzhang/bore) | 无网页中间页，因为它只转发 TCP；TLS、域名、可信证书、作品封面都要上层自己解决。 |
| **localtunnel** | `npx localtunnel --port 8000` 或全局安装后 `lt --port 8000`；无需账号。[一手](https://raw.githubusercontent.com/localtunnel/localtunnel/master/README.md) | 默认随机 `*.loca.lt`/`*.localtunnel.me`；可请求命名子域，但不保证拿到；只在当前 session 有效。[一手](https://raw.githubusercontent.com/localtunnel/localtunnel/master/README.md) | 公共服务要求真人在 consent page 输入“隧道创建者公网 IP”作为密码；按访客公网 IP 与子域记住 7 天。非浏览器或 `Bypass-Tunnel-Reminder` 可绕过。[一手/维护者说明](https://github.com/localtunnel/localtunnel/issues/598) |
| **Tunnelmole** | npm、安装脚本或预编译二进制；运行 `tmole 8080`，基础托管不要求账号。[一手](https://raw.githubusercontent.com/robbie-cahill/tunnelmole-client/main/README.md) | 免费随机 `*.tunnelmole.net`；托管服务自定义子域付费，自托管可自定义。[一手](https://raw.githubusercontent.com/robbie-cahill/tunnelmole-client/main/README.md) | 官网没有披露警告/密码页；未实开，记为“没查到”。 |
| **Serveo** | 一条 `ssh -R 80:localhost:3000 serveo.net`，无需专用客户端或注册；也提供 WireGuard 与浏览器扩展路线。[一手](https://serveo.net/) | 分配 `*.serveo.net`，免费档含子域和 3 条 active tunnels；Pro 可永久声明更整洁的域。[一手](https://serveo.net/) | 匿名/免费 SSH 与 WireGuard 隧道有 interstitial，点 Continue；Pro 去除。[一手](https://serveo.net/docs/) |
| **Expose** | 需要本机 PHP；使用官方全球网要先创建免费账号、取得 token，再 `expose share`。[一手](https://expose.dev/) [一手](http://expose.dev/docs/getting-started/authentication) | 免费每次随机域且仅德国节点；Pro/Team 才有持久 URL、自定义/保留子域和全球节点；自托管可自行配置。[一手](https://expose.dev/) | 官网未写强制警告页；支持开发者自己设置访问控制。未注册实测。 |
| **Loophole** | `.cloud` 文档提供 macOS/Linux/Windows GUI 与 CLI，并用 Auth0 认证；只需告诉客户端本地端口。[一手](https://loophole.cloud/docs) | `.cloud` 文档宣称多个自定义 hostname 与免费 TLS；仅欧洲区域。[一手](https://loophole.cloud/docs/faq) | `.cloud` 文档未写访客中间页。本轮另发现 `loophole.run` 页面宣称 npm CLI、随机域和分档定价，但它与 `.cloud` 官方仓库链接和能力叙述冲突，**不能确认是同一代正式服务**。[一手·冲突页面](https://www.loophole.run/) |
| **Tailscale Funnel** | 必须先安装并登录 Tailscale、建立 tailnet，开启 MagicDNS/HTTPS；第一次 `tailscale funnel 3000` 还会弹 Web UI 要管理员批准，随后才输出链接。[一手](https://tailscale.com/docs/features/tailscale-funnel) | 固定为 `<node>.<tailnet>.ts.net`；不是自定义品牌域。公开 DNS 最长可能等 10 分钟传播。[一手](https://tailscale.com/docs/features/tailscale-funnel) | 访客无需安装 Tailscale、无需登录，直接浏览器访问；官方未写 interstitial。[一手](https://tailscale.com/docs/use-cases/application-testing/share-local-dev-server-with-internet) |
| **LocalXpose（补充竞品）** | 安装 CLI、创建免费账号并 `loclx account login`，再开 HTTP tunnel。[一手](https://localxpose.io/) | 免费 unique subdomain；Pro 有 custom subdomain/domain/wildcard。[一手](https://localxpose.io/pricing) | 免费档明确有 interstitial warning page；付费档没有列该限制。[一手](https://localxpose.io/pricing) |

### 2.2 限额、WebSocket、重连、价格、开源与网络

| 工具 | 免费档与并发/流量 | WebSocket 与断线 | 收费与开源 | 是否明确骑 Cloudflare |
| --- | --- | --- | --- | --- |
| **ngrok** | 1 GB/月出站、20,000 HTTP 请求/月、5,000 TCP 连接/月、HTTP 4,000/min、TCP 100/min、3 online endpoints、3 agents；**免费 endpoint 无时长限制**。[一手](https://ngrok.com/docs/pricing-limits/free-plan-limits) | HTTP endpoint 开箱支持 WebSocket；agent 心跳失败后自动重新 DNS、快速重试并逐渐退避，但既有 WS 会随底层连接中断而断，应用仍需自己重连。[一手](https://ngrok.com/docs/gateway/endpoints/http) [一手](https://ngrok.com/docs/agent) | Hobbyist `$10/月` 或 `$8/月` 年付，5 GB、100k 请求、无警告；PAYG `$20/月` 含等额额度，超出按 `$0.10/GB`、`$1/100k HTTP` 等计费。[一手](https://ngrok.com/pricing.md) 当前 agent/云服务不是可自托管开源版；只有停止维护的 v1 原版仍在 GitHub。[一手](https://ngrok.com/docs/faq) | **否**；官方描述为 ngrok 自有 cloud service/edge，而非 Cloudflare 转售。[一手](https://ngrok.com/docs/pricing-limits) |
| **Cloudflare Quick / Named** | Quick Tunnel 硬上限 200 个同时 in-flight 请求，超过返 429，且不支持 SSE；官方定位测试用途。Named Tunnel 的基础隧道免费，本轮未找到公开带宽硬上限。[一手](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/do-more-with-tunnels/trycloudflare/) | Cloudflare 代理支持 WS，但 idle WS 会被关闭，需 heartbeat；`cloudflared` 默认建 4 条到至少 2 个数据中心的连接。进程/replica 切换时现有 WS/TCP 会断，只接新连接。[一手](https://github.com/cloudflare/cloudflare-docs/blob/production/src/content/docs/network/websockets.mdx) [一手](https://developers.cloudflare.com/tunnel/configuration/) | Tunnel 免费；可向 Cloudflare Access、WAF、Load Balancer、Argo Smart Routing 和企业支持升级。`cloudflared` 客户端开源，托管网络不是。[一手](https://blog.cloudflare.com/tunnel-for-everyone/) | **是，本体。** |
| **localhost.run** | 免费明确“限速”，但**没公布 Mbps、请求数或并发数**；付费自定义域最多同时 5 条 tunnel。[一手](https://localhost.run/docs/forever-free/) [一手](https://localhost.run/docs/custom-domains/) | 官方未给浏览器 WebSocket 协议矩阵或自动重连 SLO；SSH 断线后的保活由 SSH/autossh 使用方式决定。TLS passthrough 仅 Custom Domain。[一手](https://localhost.run/docs/tls-passthru-tunnels/) | `$9/月`、按年付，稳定域与 priority bandwidth；托管服务端未找到官方开源仓库。 | **没查到明确使用 Cloudflare。** |
| **Pinggy** | 免费 60 分钟/session、随机域、无限数据传输；官网没有公开请求率与并发连接上限。[一手](https://pinggy.io/) | CLI 明确“robust auto reconnection”，新 CLI 默认自动重连；HTTP(S)、TCP、UDP、TLS 均列为支持。官网没有单列浏览器 WS 保证，故不把“CLI 管理使用 WebSocket”误写成“所有应用 WS 已验证”。[一手](https://pinggy.io/) [一手](https://pinggy.io/docs/cli/) | Pro 官方博客写 `$2.50/月` 年付或 `$3/月` 月付，持久域/端口、团队、远程设备；闭源托管，未找到自托管服务端。[一手·厂商博客](https://pinggy.io/blog/best_webhook_testing_tools_for_local_development/) | **否/没查到 CF**；官网列自有美国、欧洲、英国、新加坡、巴西、澳洲节点。[一手](https://pinggy.io/) |
| **zrok** | `$0` 档 5 GB/滚动 24 小时、25 environments、50 share backends、50 private frontends；public share 每 IP 2,000 请求/300 秒、每 share 7,500/300 秒。[一手](https://zrok.io/pricing/) [一手](https://netfoundry.io/docs/zrok/1.0/myzrok/limits/) | 本轮没找到 public share 对浏览器 WS 和客户端重连的明确保证；不要从“OpenZiti overlay”直接推断体验。 | 当前公开页只有 `$0` SaaS、销售接洽的生产/SLA/专用容量，以及无软件限额的自托管。zrok 与 OpenZiti 开源，Apache-2.0。[一手](https://zrok.io/pricing/) [一手·GitHub](https://api.github.com/repos/openziti/zrok) | **否**；明确构建于 OpenZiti、由 NetFoundry 托管。[一手](https://zrok.io/) |
| **bore** | `bore.pub` 没公布 SLA、流量、并发或保留承诺；自托管上限由 VPS 和 `min/max-port` 决定。[一手](https://github.com/ekzhang/bore) | 只做 raw TCP，因此能搬运承载 WS 的 TCP 字节，但不提供 HTTP Host、TLS、证书、WS 检查或应用重连；默认数据流本身不加密，secret 只验证握手。[一手](https://github.com/ekzhang/bore/blob/main/README.md) | 免费、MIT、自托管；没有商业托管计划。 | **否。** |
| **localtunnel** | 公共服务未公布正式带宽/请求/并发配额，也没有付费 SLA；仓库允许自托管。[一手](https://raw.githubusercontent.com/localtunnel/localtunnel/master/README.md) | 本地 app 重启后 client 会检测并恢复；公共 tunnel 自身断开没有可靠自动恢复承诺。2025–2026 issue 有“几分钟后 connection refused”“持续 503、需手工重开”。[一手](https://raw.githubusercontent.com/localtunnel/localtunnel/master/README.md) [用户原话](https://github.com/localtunnel/localtunnel/issues/715) | 公共服务免费；客户端/服务端 MIT 开源，没有正式付费档。 | **没查到。** |
| **Tunnelmole** | 免费随机 HTTPS 的限流/并发没公开。付费 Starter `$3.99/月` 或 `$2.99/月` 年付：2 custom subdomains、2 concurrent tunnels、30 tunnels/hour、无 timeout；更高档依次增加。[一手](https://dashboard.tunnelmole.com/) | 客户端到服务端使用持久 WebSocket，但这不等同于官方承诺任意上游 WebSocket 透传；自动重连细节没查到。[一手](https://github.com/robbie-cahill/tunnelmole-client/) | 客户端 MIT、服务端 AGPL；官方托管在开源服务上加闭源 billing verification。[一手](https://github.com/robbie-cahill/tunnelmole-service/) | **没查到。** |
| **Serveo** | 免费 3 active tunnels；未公布带宽与访客并发。Pro `$6/月`、`$60/年`，另有 `$6` 的 10-day pass。[一手](https://serveo.net/) | SSH 持久化官方建议 autossh；WireGuard 路线强调跨 Wi‑Fi/热点 roaming。没有找到正式 WebSocket 协议承诺。[一手](https://serveo.net/docs/) [一手](https://serveo.net/articles/zero-config-wireguard-tunnels/) | 当前托管实现未找到官方开源仓库。 | **没查到。** |
| **Expose** | 免费档明确有连接时长限制但**没公布具体分钟数**，单一 EU 节点、随机域；流量/并发没公布。[一手](https://expose.dev/) | 自托管反向代理文档明确配置 WebSocket Upgrade；连接超过 server 的最大时长会被主动关，客户端自动重连保证没查到。[一手](http://expose.dev/docs/server/server/ssl) [一手](http://expose.dev/docs/server/server/server-configuration) | Pro `$79/人/年`；Team `$229/团队/年`、最多 10 人；核心 MIT、可自托管。[一手](https://expose.dev/) [一手·GitHub](https://api.github.com/repos/beyondcode/expose) | **没查到。** |
| **Loophole** | `.cloud` FAQ 声称不限制并发、请求和带宽，但保留封恶意用户权利；只在欧洲。[一手](https://loophole.cloud/docs/faq) | `.cloud` 明说当前只支持 HTTP/HTTPS、TCP 以后再做；未明确 WS。`.run` 却声称付费有 TCP & WebSocket，资料冲突，不能采信为同一版本。[一手](https://loophole.cloud/docs/faq) [一手·冲突页](https://www.loophole.run/) | `.cloud` 声称所有功能免费，停止收捐款；CLI MIT，2026-04 尚有 push、210 stars。[一手](https://loophole.cloud/docs/faq) [一手·GitHub API](https://api.github.com/repos/loophole/cli) | **没查到。** |
| **Tailscale Funnel** | 所有计划可用；只允许公网端口 443/8443/10000、只用 tailnet 域、仅 TLS；有“不可配置的带宽限制”，具体数值没公开。[一手](https://tailscale.com/docs/features/tailscale-funnel) | 使用 Tailscale TCP proxy/relay；适合 HTTP/TLS 字节流，但既有连接遇到节点/relay 中断仍要应用重连。Funnel 仍是 beta。[一手](https://tailscale.com/docs/features/tailscale-funnel) | Funnel 对包含免费 Personal 在内的所有计划开放，未见单独加价；动态价格页本轮无法稳定抓取，故不引用未复核的账号套餐单价。客户端有开源变体，控制面与 relay 为托管。[一手](https://tailscale.com/docs/features/tailscale-funnel) | **否**；Tailscale 自营 ingress/Funnel relay。[一手](https://tailscale.com/blog/tailscale-funnel-beta) |
| **LocalXpose** | 免费 2 条 active HTTP/HTTPS、time limits、warning page；官网定价页没有给具体免费时长。Pro 10 tunnels、无限带宽（受 AUP 约束）。[一手](https://localxpose.io/pricing) | HTTP/TCP/TLS/UDP 在 Pro 明列；本轮没找到 WS 与断线恢复 SLO。[一手](https://localxpose.io/pricing) | Pro `$8/月`，按 `$96/年`；未找到官方服务端开源说明。[一手](https://localxpose.io/pricing) | **没查到。** |

### 2.3 手机与微信里的中间页：能确认什么，不能确认什么

**能从一手页面确认的结构：**

- ngrok 是通用风险告知：显示目标 host、说明“此网站通过 ngrok 免费提供，只在信任发送者时访问”，主动作是 **Visit Site**；同一域 7 天一次。[一手·错误页文案](https://ngrok.com/docs/errors/err_ngrok_6024)
- zrok 的实际 interstitial 更强烈：明确提醒不要披露密码、电话、信用卡，动作是 **Visit Share**。[一手·实际页面](https://auth.kamilmemory.share.zrok.io/)
- Pinggy 是一次 screening，说明流量经 Pinggy 代理并让访客确认；每浏览器一次。[一手](https://pinggy.io/docs/http_tunnels/screening/)
- localtunnel 不只是“继续”：访客要知道并输入开发者出口公网 IP。换 Wi‑Fi、VPN 或国家后因公网 IP 变化会重新要求密码。[一手/维护者说明](https://github.com/localtunnel/localtunnel/issues/655)
- Serveo 的匿名 tunnel 有 Continue interstitial，但官方文档未展示完整手机布局。[一手](https://serveo.net/docs/)

**手机/WebView 已有负证据：**

- localtunnel 用户原话：“the ‘Friendly Reminder’ page does not work if you use the app inside a web view on a mobile device. If I click on ‘Continue’, nothing happens… effectively it's causing more ‘harm’ than helping anybody.”[用户原话](https://github.com/localtunnel/localtunnel/issues/366)
- localtunnel 另一用户为手机测试输入了 `ipconfig`/Google 查询的不同 IP 仍不工作，并明确说移动端无法靠浏览器扩展加 bypass header。[用户原话](https://github.com/localtunnel/localtunnel/issues/598)
- ngrok 有“Visit Site 按钮只刷新，无法越过 splash”的 issue；虽非微信专属，证明“多一个按钮”必须有端到端成功率指标，不能只看页面渲染。[用户原话](https://github.com/inconshreveable/ngrok/issues/860)
- Loom 2026-03 的用户称公开链接突然在手机要求注册：“it's tough adding an extra step”；回答称该 gate 只在 mobile 出现。[用户原话](https://community.atlassian.com/forums/Loom-questions/Are-people-expected-to-sign-up-to-view-videos-with-public-links/qaq-p/3205171)

**微信真机结论：没查到可引用的 2026 截图或稳定行为，也未实测。** 搜到的中文文章多在讨论微信回调被免费 ngrok warning 干扰，但属于二手教程，不能证明普通玩家在微信内置浏览器里的视觉与跳转结果。[二手](http://www.framerc.cn/news/2168053/) 因而本报告不声称任何一个海外 tunnel 的中间页“已在微信可用”。

## 3. 商业模式

### 3.1 ngrok：从开发工具到生产 ingress

**可确认的时间线：**

- 2013 年 Alan Shreve 发布首版，最初目标是帮助开发者处理 webhooks；公司先靠口碑和客户收入增长。[一手·创始人自述](https://ngrok.com/blog/ngrok-raises-50m-for-ingress-as-a-service)
- 2022-04 ngrok 3.0 发布时，HN 用户发现价格近乎翻倍：“Wait -- isn't this like 2x the price it was yesterday? … that is NOT cool.” 另一位用户说自己的使用量不值 `$20/月`，最终自己搭 Cloudflare Tunnel，但同时承认 ngrok “way easier to get started with and use”。[用户原话](https://news.ycombinator.com/item?id=31014525)
- 2022-12 首轮融资即 Series A `$50M`，Lightspeed 领投、Coatue 参与。官方当时称 5M+ developers、30,000+ paying customers、每天新增 4,000+ developers、收入同比翻倍；这些是公司口径，不是审计报告。[一手](https://ngrok.com/press-releases/ngrok-raises-50m-for-ingress-as-a-service)
- 2023-03，ngrok 反而把 OAuth 与 webhook validation 的部分能力加入免费档；说明“免费档变化”不是单向删减。[一手](https://ngrok.com/blog/free-security)
- 2023-12，取消无账号使用。[一手](https://ngrok.com/blog/tcp-endpoints-require-verification)
- 2024-06 左右，免费 TCP endpoint 要求信用卡验证但不扣费。官方解释是 2024-03 起恶意软件使用激增，agent 被杀毒软件当作入侵指标，影响正常用户；同页称开发者已超过 7M。[一手](https://ngrok.com/blog/tcp-endpoints-require-verification)
- 之后推出面向生产的 PAYG：按 active endpoint、流量、请求、连接等使用量收费，目标从临时开发预览扩大到客户网络、IoT 设备、机器人和生产 API ingress。[一手](https://ngrok.com/blog/introducing-pay-as-you-go-pricing-for-ngrok)
- 2026-09 当前免费档是固定 dev domain、1 GB/月、20k HTTP/月、3 endpoints；Hobbyist `$8–10/月`，PAYG `$20/月` 起。[一手](https://ngrok.com/pricing.md)

**关于“2023–2024 改价、1 GB 是何时收紧”的谨慎结论：**

- 找到了 2022 年明显的涨价争议，也找到了 2023-12 强制账号与 2024 免费 TCP 强制信用卡。
- **没查到官方资料能证明 1 GB 恰好在 2023–2024 才引入。** 2026 DDEV issue 中 ngrok 员工反而列出“old free plan”和“new free plan”都已有 1 GB/20k，并说 request limit 没变。[用户/厂商回复](https://github.com/ddev/ddev/issues/8101) 因此不能把“1 GB”硬写成某次 2024 改价新加的限制。
- DDEV 的实际破坏来自新固定 dev domain 切换后调用必须显式 `--url`，令既有 `ddev share` 流程失效；这说明即使账面额度没变，**默认行为改变也会破坏一条命令体验**。[用户原话](https://github.com/ddev/ddev/issues/8101)

**收入与团队：**

- 当前收入没有官方公开数字，**没查到**。
- 官方可引用的是 2022 年融资前 30,000+ 付费客户与收入同比翻倍。[一手](https://ngrok.com/press-releases/ngrok-raises-50m-for-ingress-as-a-service)
- 团队规模的二手数字冲突：LinkedIn 搜索页抽取为约 57 人，PitchBook 为 83 人；均非公司审计披露，只能写“约数且不可靠”，不能选择一个当事实。[二手·LinkedIn](https://linkedin.com/company/ngrok) [二手·PitchBook](https://pitchbook.com/profiles/company/343185-22)

### 3.2 Cloudflare 为什么免费送 Tunnel

Cloudflare 自己给出的理由比“用安全产品获客”更具体：

1. 企业流量呈日周期峰谷，免费客户可以使用网络的 off-cycle headroom；因此大规模网络能以较低边际成本提供 unmetered free bandwidth，但免费流量不一定获得企业流量同等的就近与性能优先级。[一手](https://blog.cloudflare.com/cloudflares-commitment-to-free/)
2. 免费用户扩大网络样本、帮助改进产品、推动增长，并形成向高级安全、性能与企业功能升级的入口。Cloudflare 称超过 30M Internet properties、约 20% web 在其网络后；这是公司口径。[一手](https://blog.cloudflare.com/cloudflares-commitment-to-free/)
3. Tunnel 本来属于按带宽收费的 Argo Smart Routing；Cloudflare 把“安全 outbound-only 连接”免费，把 Smart Routing 加速与 Access/Zero Trust 等高级能力留作增购。[一手](https://blog.cloudflare.com/tunnel-for-everyone/)

所以 Cloudflare 的免费是**全球网络闲置容量 + 多产品交叉销售 + 数据/分发飞轮**。playtest.run 只有香港边缘与单一产品，不应拿它的“无计量”作为免费档基准。

### 3.3 localhost.run 与 Pinggy：小团队如何活

**localhost.run：**

- 创始人 Tom 说它最初为自己写 webhook 而做，“accidentally got a bunch of users”，长期是 side project；2024 年仍公开说需要把它变成可全职投入的生意。[一手/创始人原话](https://www.indiehackers.com/post/hello-world-again-3ebaa7e95b)
- 免费入口刻意限速、域名轮换以减少钓鱼；收费点是 `$9/月` 年付的稳定域与优先带宽。[一手](https://localhost.run/docs/forever-free/) [一手](https://localhost.run/docs/custom-domains/)
- 没查到可验证用户数、MRR、融资或团队人数。它证明的是“一人/小团队可以用免费入口获客并卖稳定性”，不证明这已经是一门大生意。

**Pinggy：**

- 变现点是 60 分钟免费 session 之上的持久域/端口、自定义域、团队、远程设备和 API；入门 Pro 仅 `$2.50/月` 年付或 `$3/月` 月付。[一手](https://pinggy.io/) [一手·厂商博客](https://pinggy.io/blog/best_webhook_testing_tools_for_local_development/)
- 官方首页公开多个自营区域与“unlimited data transfer”，说明它用极低价格换持久性和团队功能，而非按 GB 收费。[一手](https://pinggy.io/)
- 二手公司资料称团队约 7–8 人、bootstrapped，2025 获印度 MeitY GENESIS Grant；未找到公司官网财务披露。[二手·LinkedIn](https://linkedin.com/company/pinggy) [二手·Inc42](https://inc42.com/company/pinggy/) [一手性较弱·创始人发帖](https://www.linkedin.com/posts/ghoshbishakh_thrilled-to-announce-that-pinggy-has-been-activity-7309854964191834114-TSYZ)
- Latka 声称 2023 年收入 `$4K`，但没有交叉来源，本文不把它当可靠营收。[二手](https://getlatka.com/companies/pinggy.io)

### 3.4 zrok/OpenZiti：开源 + 免费托管 + 企业容量

zrok 的组合很清楚：

- 客户端、服务端和底层 OpenZiti 均可审计、自托管；zrok Apache-2.0，自托管不受 zrok.io 套餐的软件限额。[一手](https://zrok.io/pricing/) [一手](https://api.github.com/repos/openziti/zrok)
- 公共 SaaS 给 5 GB/天、reserved shares 与 private shares，以足够慷慨的 `$0` 档建立使用面。[一手](https://zrok.io/pricing/)
- 验证信用卡即可去除反钓鱼页，进一步把身份成本放在开发者而不是每位访客身上。[一手](https://zrok.io/pricing/)
- 生产工作负载、SLA、专用设施和自定义上限走销售。[一手](https://zrok.io/pricing/)

这比“开源 core + 免费公共实例 + 一个便宜域名档”更完整：开源建立信任与自托管分发，免费 SaaS 降低试用成本，企业为确定性和隔离容量付费。

### 3.5 为什么 frp 十万星没有妨碍 ngrok 收钱

截至 2026-09-07，frp 有 109,244 stars、15,210 forks，仍活跃更新；它支持用户在自己的公网机器上构建 HTTP/TCP/UDP 等反向代理。[一手·GitHub API](https://api.github.com/repos/fatedier/frp)

frp 和 ngrok 实际卖的不是同一件事：

- frp 免许可证费，但用户先要买/管 VPS，配置 server/client、开放端口、域名、TLS、监控、升级、容量与滥用响应。
- ngrok 把这些折叠为账号、agent、域名和托管 edge，还提供请求检查/重放、访问控制、流量策略、团队与支持。[一手](https://ngrok.com/docs/pricing-limits)
- HN 用户的表述很准确：自己搭 Cloudflare/其它替代方案可以更便宜，但 ngrok “way easier to get started with and use”。[用户原话](https://news.ycombinator.com/item?id=31014525)

因此，开源会压低“软件许可”价格，却不会消灭**托管网络、信誉、可达性、反滥用、默认正确与支持**的价值。

## 4. 用户反馈：真正被骂和真正被需要的

### 4.1 ngrok：价格、账号、警告页、被企业封

- “For hobby/side projects, I don't mind paying `$5–10/mo` … but ngrok's floor of `$25/mo` was too steep for me. Instead I now use Cloudflare Tunnel.”[用户原话·HN](https://news.ycombinator.com/item?id=33968967)
- “It used to be free, with no signup, and it was great. Now all I want is something that is exactly what ngrok was in 2015.”[用户原话·HN](https://news.ycombinator.com/item?id=33968967)
- 一位服务医院客户的用户说：“When we want to let them try our services behind ngrok domains, they are often blocked.”[用户原话·HN](https://news.ycombinator.com/item?id=33968967)
- 免费警告页 issue 中，用户点 **Visit Site** 只得到刷新，完全进不去站点；建议的 header 对一个收到链接的普通访客也不可操作。[用户原话](https://github.com/inconshreveable/ngrok/issues/860)
- 一个开源 forward proxy 专门为绕过 warning 而存在，并指出 EventSource 等客户端无法方便地控制 header；这本身是摩擦强到有人另造一层基础设施的信号。[一手·项目自述](https://github.com/igops/ngrok-skip-browser-warning)
- 另一方面，用户也持续肯定它：HN 评论称它是自己想到“临时暴露本地服务”时第一个工具；另一个嵌入式设备团队说在不稳定网络上 “worked flawlessly”。[用户原话](https://news.ycombinator.com/item?id=31014525)

结论不是“ngrok 体验差”，而是**开发者侧依旧很强，免费访客侧和价格阶梯让一部分临时/客户演示用户流失**。

### 4.2 localtunnel：把反滥用成本直接交给访客

- 维护者解释，简单 Continue 没能遏制钓鱼，自己收到大量全球 abuse notice，甚至被告知要承担 IP 黑名单移除成本，所以临时把创建者公网 IP 当 password。[维护者原话](https://github.com/localtunnel/localtunnel/issues/598)
- 用户反驳：“I'd rather not have to give out my server IP … this is illogical.”[用户原话](https://github.com/localtunnel/localtunnel/issues/702)
- 2026 issue 直接总结密码步骤：“Adds friction … Inconvenient for friends/users when accessing”，并有人说向朋友发 IP “unsafe”。[用户原话](https://github.com/localtunnel/localtunnel/issues/719)
- 公共实例可靠性也是问题：用户报告 tunnel 只能活约一分钟、持续 503；另有人“stuck for an hour … thought the issue is on my side”。[用户原话](https://github.com/localtunnel/localtunnel/issues/726)

这是 playtest.run 最该避免的模式：反滥用必须有，但不应要求玩家理解 tunnel、索要公网 IP、装 header 扩展或排查平台故障。

### 4.3 Pinggy/zrok：一次确认较温和，但仍会污染应用协议

- Pinggy 用户从浏览器 `fetch` 调本地 Flask API，实际拿到 caution page HTML，又因 CORS 看不到真正原因；直到把 `X-Pinggy-No-Screen` 放进 fetch 才恢复。[用户原话](https://forum.pinggy.io/t/unable-to-successfully-connect-to-flask-by-javascript-fetch/57)
- FlutterFlow 用户用 Postman 正常、在 app 里却拿到 200 + Pinggy Caution Page，而不是 JSON。[用户原话](https://community.flutterflow.io/ask-the-community/post/api-calls-to-a-flask-server-with-the-local-port-shared-with-pinggy-io-Gnfw5orIpLfbN47)
- zrok 官方承认 2024 稳定增长后不得不处理 abuse/phishing，最终给免费 public share 加每周一次 interstitial；私有 share、自托管和付费不受影响。[一手](https://blog.openziti.io/zrok-is-growing-up)

因此中间页必须只拦**顶层 HTML navigation**，不能把 API、资源请求、WS upgrade、机器回调误判成“浏览器”后返回 200 HTML。

### 4.4 Cloudflare：免费、无账号，但信誉和协议故障外溢

- 安全研究报告称 TryCloudflare 的临时、可轮换子域被用于隐藏钓鱼源站，2025 的相关 credential phishing/malware incidents 明显上升；这是安全厂商观测，不是 Cloudflare 自己的统计。[二手·Cofense](https://cofense.com/blog/how-cloudflare-services-are-abused-for-credential-theft-and-malware-distribution)
- 因域名轮换快，封单一子域追不上攻击者；组织可能直接封 `trycloudflare.com`。这和 HN 中医院网络经常封 ngrok 域是同一类“共享域信誉”外部性。[二手](https://www.csoonline.com/article/4009636/phishing-campaign-abuses-cloudflare-tunnels-to-sneak-malware-past-firewalls.html) [用户原话](https://news.ycombinator.com/item?id=33968967)
- Cloudflare 官方说标准全球网络跨中国网络边界有显著 latency/reliability 问题；真正的大陆 China Network 是 Enterprise 另购、JD Cloud 运营、要求 ICP 和内容审核。[一手](https://developers.cloudflare.com/china-network/) 这意味着免费 trycloudflare 不能被当成“大​​陆可达”的替代。
- WS 不是“支持”二字就结束：cloudflared issue 有用户看到 502，另一个 issue 报告 Upgrade header 在 QUIC/HTTP2 路径丢失、浏览器只看到 1006。[用户原话](https://github.com/cloudflare/cloudflared/issues/1083) [用户原话](https://github.com/cloudflare/cloudflared/issues/1652)
- 上行不足时也会失控：一个 issue 报告请求率超过上传带宽后 `cloudflared` 内存从数百 MB 涨到数 GB，最终 host 无响应。[用户原话](https://github.com/cloudflare/cloudflared/issues/1205)

### 4.5 新品的用户评价：大部分还不存在

本轮没有找到 tunr、uplink、kshare、LocalhostVibe 的独立 HN/Reddit/PH 讨论能证明真实复用或付费。GitHub 采用量也很小：

| 项目 | 创建 / 最后 push | stars（2026-09-07） | 判断 |
| --- | --- | ---: | --- |
| tunr | 2026-03-10 / 2026-08-12 | 18 | 功能方向最接近，但只是早期信号。[一手](https://api.github.com/repos/ahmetvural79/tunr) |
| uplink | 2025-12-11 / 2026-09-02 | 10 | 仍在更新；agent CLI 设计值得学，采用未验证。[一手](https://api.github.com/repos/firstprinciplecode/uplink) |
| kshare | 2026-05-17 / 2026-05-18 | 2 | 几乎是一次性发布状态。[一手](https://api.github.com/repos/sifxprime/kshare) |
| LocalhostVibe | 2026-06-26 / 2026-06-28 | 0 | README 自称 “software-as-a-meme”；不能当市场规模证据。[一手](https://api.github.com/repos/teionarr/LocalhostVibe) |

OtterKit 的 Product Hunt 聚合页有 5 upvotes、9 comments、当日 #159；评论肯定 native macOS + traffic inspection，但样本极小，而且页面是第三方聚合。[二手](https://hunted.space/product/otterkit)

## 5. 2025–2026 AI 编程隧道新品

### 5.1 指定对象

| 工具 | 它真正新增了什么 | 账号/URL/限制 | “知道结果” | 开源、活跃度、Cloudflare |
| --- | --- | --- | --- | --- |
| **tunr** | `tunr share` 外加 read-only demo、freeze cache、自动登录 cookie、TTL、路径路由、QR、HTTP inspector、MCP；还宣传 TCP/UDP/TLS。[一手](https://github.com/ahmetvural79/tunr) | core/free；custom subdomain/team dashboard 要云账号。托管价格、带宽、并发没找到可靠公开表。[一手](https://tunr.sh/) | `--inject-widget` 代理注入类似 Marker.io 的落点评论；采集 JS exception、timestamp、browser info，在 Dashboard 与 MCP `get_feedback` 查看。**这是产品自述，没找到用户使用率。**[一手](https://tunr.sh/docs.html) | Apache-2.0；18 stars，2026-08 仍更新；官网明确 relay 由 Cloudflare global edge 支撑。[一手](https://api.github.com/repos/ahmetvural79/tunr) [一手](https://tunr.sh/) |
| **uplink** | agent-first 的 `--json`、稳定 exit codes、从 stdin 读 token 防 argv 泄露、无浏览器 CLI signup；这比 MCP 更接近可靠自动化基本功。[一手](https://github.com/firstprinciplecode/uplink) | CLI signup 会创建 user+token，所以是“无浏览器”，不是“无账号”；随机 `abc123.x.uplink.spot`，permanent alias 需账号侧开 premium。价格/限额没查到。[一手](https://github.com/firstprinciplecode/uplink) | 有 tunnel stats，但没查到访客会话、反馈、JS error、录屏。 | CLI MIT；backend 不在该开源范围；10 stars、2026-09-02 有 push；是否走 CF 没查到。[一手](https://api.github.com/repos/firstprinciplecode/uplink) |
| **tinyfi.sh** | 没有新协议：一条 `ssh -R`，再发布 `llms.txt`、Claude/Cursor/Copilot skill，让 agent 学会后台启动并返回 URL。[一手](https://tinyfi.sh/) | 不登录、不安装；随机 `abc123.tinyfi.sh`，也可请求未占用 custom subdomain；SSH 活着才在线。[一手](https://tinyfi.sh/) | 无面板、反馈、错误或会话结果。 | 服务端开源/价格/基础设施归属均没查到；是否走 CF 没查到。 |
| **kshare** | `npx @sifxprime/kshare --port 3000`；outbound WebSocket、HTML/CSS/JS URL rewriting、Wi‑Fi 断开自动重连、24 小时自动过期、可密码保护和自托管。[一手](https://github.com/sifxprime/kshare) | 不要账号；URL 24h。README 的 `MAX_TUNNELS=5`、`MAX_CONNECTIONS=200` 是 self-host server 配置默认值，不能无证据当公共服务套餐。[一手](https://github.com/sifxprime/kshare/blob/main/README.md) | 明说不注入代码；没有访客反馈与结果。 | MIT；2 stars，发布次日后未 push；是否走 CF 没查到。[一手](https://api.github.com/repos/sifxprime/kshare) |
| **LocalhostVibe** | 本质是 Cloudflare Quick Tunnel/localtunnel wrapper，加终端 QR 和本地 `/_vibe` control room；可切 `--no-proxy` 关闭访客 feed。[一手](https://github.com/teionarr/LocalhostVibe) | `npx localhostvibe share 3000`；稳定性继承 provider；自定义子域走 localtunnel。[一手](https://github.com/teionarr/LocalhostVibe) | 只统计连接与实时 visitor feed，所谓 vibe score 是玩笑；无反馈、错误、版本或加载完成。[一手](https://github.com/teionarr/LocalhostVibe) | 未标 license，0 stars、两天后停止 push；默认明确骑 Cloudflare，也可 localtunnel。[一手](https://api.github.com/repos/teionarr/LocalhostVibe) |

### 5.2 本轮另外发现的新品

- **stunl**：主张“一条稳定 hostname 路由整个 stack”，把 `/`、`/api`、`/admin` 和 TCP/UDP 服务写入 `stunl.yaml`，另做 scoped agent keys 与 MCP registry；Pro 从 `$10/月`。没找到公开仓库采用量、访客反馈或游戏专用交付能力。[一手·官网](https://stunl.com/)
- **LivePort**：定位“AI agents 的安全 localhost tunnel”，提供 CLI、MCP、SDK和免费固定 URL；仓库截至检索时 0 stars，采用未验证。[一手·GitHub](https://github.com/dundas/liveport/)
- **OtterKit**：native macOS 应用 + agent 可调用 CLI；按在线小时 `$0.01`/credit，每 tunnel 每日封顶 `$0.10`，提供 webhook capture/traffic inspection。它优化开发调试，不做玩家反馈。[二手·PH 聚合原始产品文案](https://hunted.space/product/otterkit)
- **Wormhole**：面向 agents/worktrees，支持自托管 relay、Tailscale、Cloudflare，多 provider 稳定 URL、inspection/replay 与 webhook buffering；5 stars，仍属极早期。[一手](https://github.com/nikuscs/wormhole)
- **Pugloo**：把本地域名、TLS、path routing、daemon 和 public share 合并，适合多 worktree；public share 仍依赖 hosted service 登录。[一手](https://github.com/Ani-HQ/pugloo)
- **OpenAI Secure MCP Tunnel / mcppipe**：这是“让远端模型调用私有 MCP server”，不是“给人类访客打开网页”。OpenAI tunnel 不把 server 暴露给公网；它解决 agent-to-tool 的身份与权限，不能算 playtest 分享链接直接竞品。[一手](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels) [一手·开源 client](https://github.com/openai/tunnel-client)

### 5.3 AI-native 到底改变了什么

可复用的变化有四个：

1. **机器可判定输出**：URL 应有 JSON 模式，日志走 stderr，退出码区分 usage/auth/network/server；uplink 在这点最清楚。[一手](https://github.com/firstprinciplecode/uplink)
2. **凭据委派**：agent 不应拿永久全权 token，应拿短期、限定 tunnel/TTL/次数的 child key。21tunnel 对这一模式有系统性论述，但它是厂商观点，属二手产品建议。[二手](https://21tunnel.com/blog/giving-ai-agents-public-urls/)
3. **agent 控制面**：MCP 可以开/关 tunnel、取 URL、读 request logs；这减少人工复制，不会自动创造访客价值。
4. **演示状态**：tunr 的 freeze/read-only/feedback 比传统 tunnel 更理解“把未完成作品给别人看”，但 freeze 缓存最后一个 HTML response 对大型 SPA、动态 API、WebSocket 游戏是否正确没有证据。

## 6. 重点问题回答

### 6.1 （a）“做得更好”的空间在哪里：用可度量项说话

不能只写“更快、更简单”。建议以如下公开 SLO/验收矩阵竞争：

| 面 | 指标 | 建议的 v0.1 验收方式 |
| --- | --- | --- |
| **安装到首链** | 冷启动步骤数；P50/P95 时间；需浏览器/账号次数；安装包大小；三平台成功率 | 新机器、无缓存：匿名 tunnel 从敲命令到二维码/HTTPS URL，P50 < 10s、P95 < 30s；完整安装到手机打开仍守 DESIGN 的 <60s |
| **第一次成功** | Vite/Next/webpack Host 403 比例；HMR 成功率；错误提示可行动性 | 自动 Host rewrite；识别 HMR 端口不匹配；每种框架真 fixture，不能只测 hello world |
| **访客门禁** | gate TTFB/LCP；gate→Start；Start→game loaded；重复出现率；WebView 点击成功率 | 香港/大陆三网与境外测 P50/P95；微信 Android/iOS 各测；每个阶段独立埋点 |
| **静态大资源** | 30/100/300 MB 构建首载；并发 1/5/20/50；Range；中断续传 | Unity/Godot 实包，不用合成小 HTML；同时观察开发者上行、edge 排队与失败形态 |
| **WebSocket** | 101 成功率；30/120 分钟存活；消息 P50/P95/P99；断网/换网恢复时间 | socket.io/Colyseus 两台真手机；空闲 heartbeat；CLI 网络切换后 1–30s 退避是否符合承诺 |
| **游戏头部** | `.wasm` MIME、`.br/.gz` encoding、COOP/COEP、CORS、CSP、Range、SPA fallback | Unity、Godot threaded/non-threaded、Phaser、Vite 固定 fixtures 全过 |
| **离线与故障** | 隧道离线被识别耗时；错误页出现时间；误报；恢复后自动返回 | 合盖、kill dev server、断 Wi‑Fi、edge 重启；不允许玩家等到浏览器 timeout 才知道 |
| **结果层** | unique opens 准确度；load success 覆盖率；JS error 去重；反馈提交率；版本归因 | 对同一玩家刷新、换版本、换入口、资源 404、JS crash 做可重复测试 |
| **信誉与滥用** | abuse/1k links；误杀；举报到下架 P50/P95；根域/子域被拦比例 | 每周透明内部报表；匿名链接独立阈值；域名信誉是核心运营指标 |
| **成本** | GB/会话、GB/版本、并发玩家边际成本、香港出网成本 | 按“一个 30 MB 构建被 5/30/100 人打开”展示真实单位经济 |

现有工具暴露的可超越基线：

- ngrok 的开发者 CLI 很成熟，但免费 1 GB：一个 30 MB 构建忽略缓存与其它资源也只够约 33 次完整下载。这是由额度直接计算的上限，不是实测吞吐。[一手·额度](https://ngrok.com/docs/pricing-limits/free-plan-limits)
- Quick Tunnel 有 200 in-flight 上限、随机 URL、无 SLA；适合临时网页，不是 50 人同时加载多资源游戏的明确承诺。[一手](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/do-more-with-tunnels/trycloudflare/)
- localhost.run 免费限速但不公布速度；Tailscale Funnel 也只写“不可配置 bandwidth limits”。**把具体吞吐和失败形态说清楚，本身就是差异。**[一手](https://localhost.run/docs/forever-free/) [一手](https://tailscale.com/docs/features/tailscale-funnel)
- localtunnel、ngrok、Cloudflare 都有“名义支持、实际 WebView/WS/可靠性出错”的例子；playtest.run 的真机 fixture 与公开 spike 比 feature checklist 更可信。

### 6.2 （b）产品化门禁页，用户会怎么看

#### 正证据

1. itch.io 默认要求 HTML5 项目 click to play；移动端无论桌面设置如何都先点击再以 fullscreen/maximized 运行，理由是避免意外消耗资源、避免游戏和页面 UI 互相干扰。[一手](https://itch.io/docs/creators/html5)
2. itch.io 明说有些 engine（尤其 Unity）加载时会让浏览器卡顿，Run 按钮让用户主动决定何时付出资源成本。[一手](https://itch.io/updates/click-to-play-for-html-embedded-games)
3. Chrome 明确建议 Web Audio 等待用户 interaction；过早创建的 `AudioContext` 会 suspended，点击后 `resume()`。对网页游戏，“开始”是功能性手势，不是装饰。[一手](https://developer.chrome.com/blog/autoplay)
4. Discord invite splash 先显示 server name/icon/description/member count，再给 Join/Accept；它证明“带目标上下文的一次确认”是用户熟悉的邀请模型。[一手·Discord 加入流程](https://support.discord.com/hc/en-us/articles/360034842871-How-do-I-join-a-Server) [二手·邀请页字段整理](https://discord.dog/blog/discord-invite-preview)
5. Figma 官方做远程 prototype test 时，特意建议隐藏 Figma UI/sidebar/hotspot hints，且用无痕窗口验证“所有人能打开”；说明外部测试的目标是减少工具本身的存在感。[一手](https://help.figma.com/hc/en-us/articles/19790203466263-Test-your-prototypes-with-UserTesting)

#### 反证据

1. Vercel Authentication 会把访客重定向到 Vercel 登录；外部分享和自动化要 shareable link/bypass secret，CORS preflight 甚至会在应用代码之前被 401。[一手](https://vercel.com/docs/deployment-protection/methods-to-protect-deployments/vercel-authentication) [用户原话](https://community.vercel.com/t/cors-preflight-request-fails-with-401-in-preview-deployment/15130)
2. Figma 用户反复报告“Anyone with the link”仍要求登录，尤其 mobile；官方/社区解释往往是分享了 editor link 而非 prototype viewer 内生成的 link。对发送者只是链接类型差异，对访客却是完全失败。[用户原话](https://forum.figma.com/ask-the-community-7/sharing-a-prototype-without-having-user-required-to-sign-in-to-figma-4274)
3. Discord 在 TikTok 等 in-app webview 无法唤起 app 时会退化为登录、CAPTCHA、2FA 再 Accept 的长链路；该页面是商业站分析，属二手，但机制与用户摩擦合理。[二手](https://link.boo/fix/discord-invite-from-tiktok)
4. ngrok/zrok 的警告文案强调钓鱼与金融信息，会在客户/朋友看到作品前先制造不信任；这是安全必要性，不是作品价值。

#### 对 playtest.run 的具体判断

门禁页**有机会被看成游戏封面/邀请卡，而不是 tunnel warning**，但必须同时满足：

- 首屏只讲“谁邀请你、玩什么、哪个版本、这版要看什么”，平台角标退居底部；不要在正常页先讲“危险、隧道、localhost”。
- 一个 **开始** 即进，不登录、不装 app、不填邮箱/IP、不二次跳转。风险提示只在异常信号或举报场景升级。
- 门禁只拦顶层 navigation，不向 API、静态资源、WS upgrade 返回 200 HTML。
- 点击后同 URL 承载游戏；24 小时内不重复只是起点，应测“按浏览器/作品/版本”的最佳频率。ngrok/zrok 是 7 天一次，itch mobile 是每次 launch；没有证据证明 24 小时天然最优。
- 离线态、版本说明、设备不兼容提示确实给玩家信息，而不是品牌广告。
- 必须单独量 `gate_view → start → first_asset → game_ready`。如果 Start 转化低，不能用“用户手势有用”替额外流失辩护。
- 免费角标可以存在，但去角标不应成为去掉一次多余点击的赎金；付费卖品牌控制可以，玩家基本可达性不能付费解锁。

### 6.3 （c）AI 隧道工具做到“知道结果”了吗

| 层次 | 工具 | 做到什么 | 没做到什么 |
| --- | --- | --- | --- |
| **网络请求** | ngrok | 本地 Web Inspection Agent、request/response、replay；免费也有。[一手](https://ngrok.com/docs/pricing-limits) | 不知道一次玩家会话、版本、游戏是否加载完成或玩家意见。 |
| **网络请求** | Pinggy | live headers、bandwidth stats、inspect/replay/modify；免费可用。[一手](https://pinggy.io/) | 同上；是 webhook/API debugger。 |
| **网络请求** | LocalXpose | HTTP/TCP/UDP request logging、inspect/modify/replay。[一手](https://localxpose.io/) | 同上。 |
| **连接计数** | LocalhostVibe | 本地 control room 实时列连接。[一手](https://github.com/teionarr/LocalhostVibe) | 没有独立访客、load success、错误去重、反馈、版本。 |
| **产品反馈** | tunr | 代理注入 pin comment；收 JS exception、browser info、timestamp；Dashboard 聚合，MCP 可取反馈。[一手](https://tunr.sh/docs.html) | 没有公开使用量；未说明 WebGL canvas 截图、版本归档、隐私/保留、跨页面会话准确性。 |
| **预览评论** | Vercel（非 tunnel-only） | Shareable Link 可让外部人看 preview，登录后使用 Comments。[一手](https://vercel.com/docs/deployment-protection/methods-to-bypass-deployment-protection/sharable-links) | 评论依赖 Vercel toolbar/account，不符合玩家零门槛。 |
| **流量检查** | OtterKit | webhook capture、traffic inspection。[二手·PH 聚合](https://hunted.space/product/otterkit) | 没有玩家反馈/游戏结果证据。 |

**录屏：**本轮目标隧道工具中没查到内建整段玩家录屏。tunr 做落点评论与错误，不是 session recording；LocalhostVibe 是连接 feed，不是画面。不能把 Playset 这类 playtest 平台的录像能力归到 tunnel 新品。

**用户用不用：没查到。** tunr 只有 18 stars，且未发现独立评价、公开反馈提交量或客户案例；因此它只证明“另一个 builder 也认为反馈值得做”，不能证明市场已验证。

### 6.4 （d）它们在“游戏”上哪里露怯

1. **把“游戏服务器”与“浏览器游戏试玩”混为一谈。** tunr/Pinggy 把 UDP、Minecraft/game server 当游戏能力；但 WebGL 玩家需要的是 HTTPS 静态交付 + WSS、正确资源头、移动浏览器兼容、加载结果和版本反馈。浏览器本身不能直接开普通 UDP socket。[一手·Unity Transport](https://docs.unity3d.com/Packages/com.unity.transport@6.6/manual/websockets.html)
2. **不理解构建产物。** Unity 的 `.wasm/.wasm.br/.wasm.gz` 需要正确 MIME/encoding；错误时是黑屏或卡 loading bar。通用 tunnel 原样转本地 server，server 配错就原样失败。[一手](https://docs.unity3d.com/6000.3/Documentation/Manual/webgl-deploying.html) [二手·常见故障整理](https://thegamingnest.com/en/articles/5d6a833e-57f5-444a-9777-57248293ae76)
3. **大文件走家宽上行。** Pinggy 自己在 Minecraft 文章承认本机在线才可用、home upload 就是 server bandwidth，十人 modpack 会压垮 laptop。[一手·厂商博客](https://pinggy.io/blog/best_minecraft_server_hosting/) Tailscale Funnel 摄像头流用户也遇到只出一帧且找不到具体带宽上限。[用户原话/无回答](https://wiki.hoelee.com/content/stackoverflow.com_en_all_2023-11/questions/76091568/tailscale-funnel-bandwidth-limit)
4. **HMR/Host/WS 是隐藏配置。** Vite 6.0.9/5.4.12 等版本加强 Host allowlist；反向代理域名需 allow 或改写。HTTPS tunnel 的 HMR 通常还需 `clientPort=443`/`wss`。[一手/维护讨论](https://github.com/vitejs/vite/discussions/19426) [用户讨论](https://github.com/vitejs/vite/discussions/5399)
5. **没有静态上传逃生门。** tunnel 最适合带后端/联机或正在开发的服务；纯导出物经 tunnel 只是把 CDN 问题变成开发者电脑上行问题。目标工具几乎都没有检测“大静态目录可上传”的产品动作。
6. **没有玩家生命周期。** tunnel 知道 request，不知道邀请、开始、加载完成、卡在哪、版本、反馈；电脑离线通常是 502/503/timeout，而不是“开发者电脑暂时不在线”。
7. **没有微信/X5 兼容责任。** 海外工具以通用浏览器可达为目标；本轮没找到任何一家承诺微信内置浏览器、X5、SharedArrayBuffer/WebGPU 的实际矩阵。

## 7. 大陆可达与“是否骑 Cloudflare”

### 7.1 可以确认的

- Cloudflare 官方承认全球网跨中国网络边界面临 significant latency and reliability issues；大陆 China Network 是 Enterprise 独立订阅，要求 ICP、JD Cloud 内容审核，且并非所有产品可用。[一手](https://developers.cloudflare.com/china-network/) Quick Tunnel 不属于可假定在大陆优化的产品。
- ngrok 早期维护者收到过多次 GFW 阻碍报告；这是老 issue，不足以描述 2026 每个 ISP，但足以否定“默认稳定”。[用户/维护者原话](https://github.com/inconshreveable/ngrok/issues/214)
- 2025 V2EX 用户报告部分 Cloudflare 免费 IP 在大陆 timeout/route stop；这是社区样本，不是全国测量。[用户原话](https://www.v2ex.com/t/1143620)

### 7.2 本轮不能确认的

- 没有从三大运营商、晚高峰、香港/华南/华北做统一测速。
- 没有在微信 Android X5、微信 iOS WKWebView 中实开上述每家链接。
- 除 tunr、LocalhostVibe 明写 Cloudflare 外，没有证据支持“AI 新品几乎全部骑 Cloudflare”：uplink、tinyfi.sh、kshare 的官网/仓库没有披露；应写“没查到”，而不是按 DNS 猜。

这直接修正了一个容易形成的叙事：**新品普遍依赖海外托管网络是真的；新品普遍依赖 Cloudflare，本轮证据不足。**

## 8. 对创始人三个判断的回答

### 8.1 “方便简洁、使用体验好，是第一位的”

**支持，但要扩大“体验”的定义。** 开发者一条命令只是前半程；完整体验是：

`安装/匿名 → 得到链接 → 在目标聊天工具里发出 → 玩家信任并开始 → 游戏成功加载/联机 → 开发者看到结果`

ngrok 证明 polished CLI 有商业价值；localhost.run/Pinggy 证明零安装零账号有传播力；localtunnel 则证明只要访客侧多一个莫名密码，前面的省事会被全部抵消。优先级应是“端到端第一次成功率”，不是 CLI 参数最少。

### 8.2 “是否建立反馈、曝光、孵化机制或生态”

- **反馈：有正信号。** tunr、Vercel Comments、Figma/UserTesting 都说明“给具体人看后收意见”是自然相邻能力；但 tunnel 市场还没有采用数据证明哪种反馈最常用。
- **曝光：没有支持证据。** 目标工具都围绕具体链接、客户 demo、webhook、真机测试，不围绕公开 feed。曝光会从 DESIGN M2/M3 移到 M4，并显著扩大钓鱼、版权和内容审核成本。
- **孵化：更没有证据。** 它需要作品筛选、流量分配、创作者关系和资金/发行能力，不是 tunnel/结果层自然长出的功能。
- **建议：**保留“一句话反馈”和以后可选招募测试者的接口，不做公开发现/榜单/评论区。只有当真实开发者反复说“链接有了但找不到 5 个测试者”，再把“招募”作为独立假设验证，而不是先建生态。

### 8.3 “是否提供数据分析”

- ngrok/Pinggy/LocalXpose 的成熟需求是请求可观测性；tunr 把它推进到 JS error + 人类反馈。
- 没有证据支持通用漏斗、热图、复杂 BI 是 tunnel 用户愿意购买的核心。
- **建议：**坚持 DESIGN 的最小集：打开、独立人数、来源、设备、load success/resource failure、JS error、停留、版本、一句话反馈。把“分析”改称“这一版发生了什么”，避免建设平台型报表。
- 验证重点不是功能完整度，而是 `T6 每版反馈数`、错误是否帮助修复、开发者是否因结果回来发第二版。

## 9. 对 playtest.run 的含义

### 9.1 支持 DESIGN.md 的结论

1. **上传 + 隧道双路径正确。** 大资源、Unity/Godot 构建与家宽上行的矛盾非常真实；纯 tunnel 无法给 M2/M3 最佳体验。
2. **玩家零登录正确。** Loom/Figma/Vercel/Discord 的反例都显示，外部具体受邀者遇到注册、CAPTCHA、工具账号时会流失或直接抱怨。
3. **门禁页可以成立。** itch.io 移动 Click to Play 与 Chrome 音频政策给出强正证据；关键是门禁页要像作品封面，不像通用安全告警。
4. **轻结果层是合理楔子。** 传统工具停在 request inspector；tunr 的方向说明有人也看见 JS error/feedback，但市场仍空，且录像不是 tunnel 标配。
5. **开源 + 托管服务成立。** frp、zrok、Tunnelmole、Expose 都证明客户端/核心开源与托管收费可共存；护城河确实更偏网络、信誉、默认正确和运营。
6. **香港而非 Cloudflare 是有意义的赌注。** Cloudflare 官方自己承认标准全球网跨大陆边界的可靠性问题；但这仍需真实三网 spike 才能从“合理”变成“已验证”。
7. **不做发现/商店正确。** 本轮没有证据推翻 §3.7；公开流量还会放大 §4.8 的滥用风险。

### 9.2 与 DESIGN.md 冲突或需要降调的地方

1. **“tunr 的反馈小组件说明方向已被验证”应降为“方向被另一个 builder 看见”。** 18 stars、无独立采用/付费/反馈量，不能当需求证据。[一手](https://api.github.com/repos/ahmetvural79/tunr)
2. **“新品绝大多数骑 Cloudflare”证据不足。** 本轮只明确确认 tunr 与 LocalhostVibe；uplink、tinyfi.sh、kshare 没查到。建议 DESIGN 改为“多数依赖境外托管网络；其中部分明确使用 Cloudflare”。
3. **若 DESIGN 或对外比较仍把 ngrok 免费 URL 写成每次随机，需要更新。** 当前免费账号有一个自动分配、绑定账号的固定 dev domain；随机/自选域反而是不同付费能力。[一手](https://ngrok.com/docs/pricing-limits/free-plan-limits)
4. **“ngrok 2023–2024 因改价才加 1 GB”不能写。** 找到强制账号与 TCP 信用卡的时间，但没找到 1 GB 首次引入日期；DDEV 讨论称旧新计划都有 1 GB。[用户/厂商回复](https://github.com/ddev/ddev/issues/8101)
5. **24 小时内不重复门禁缺乏比较证据。** ngrok/zrok 是一周，itch mobile 是每次 launch；建议把频率当实验参数，按作品/版本测 gate drop，不先写成最优。
6. **“WSS 走 443 就凡是 HTTPS 能通的地方都能通”需要降为工程目标而非事实。** 企业安全产品会按 proxy avoidance/共享域信誉封 ngrok；WebSocket upgrade 也可能在中间层失败。[用户原话](https://github.com/ngrok/ngrok/issues/9) [用户原话](https://github.com/cloudflare/cloudflared/issues/1652)
7. **免费 20 GB 的市场位置要说清。** 它比 ngrok 1 GB 慷慨、低于 zrok 理论 150 GB/月（5 GB/天）、又无法复制 Cloudflare unmetered；对 30 MB 构建约 666 次完整下载，足够小规模测试但不是“无限”。这与 DESIGN 成本模型基本一致，建议把示例换成该可感知单位。

### 9.3 建议改 DESIGN 的方向（这里只提建议，不直接修改）

1. 把门禁页定义补一条：**仅拦顶层文档导航，永不污染 API、资源、WebSocket、webhook。**
2. 把门禁 KPI 写进 §8：`gate LCP`、`gate→start`、`start→game_ready`、微信 WebView 点击成功率；“页面存在”不是完成。
3. 在匿名链接策略中明确：24h 生命周期、流量/并发、随机 slug、角标和门禁共同承担反滥用；以后若要求信用卡/手机号，必须重新评估 M1。
4. CLI 增加机器模式设计：稳定 JSON schema、stderr 日志、分层 exit code、`--ttl`，方便 Cursor/Claude Code 调用；不急着为“有 MCP”新增依赖。
5. “知道结果”先做无 SDK 的 load success/resource failure，再做文字反馈和 JS error；不要因为 tunr 有 widget 就提前做 pin comment、截图或录像。
6. 将游戏验收从“HTTP/WS 支持”改成固定真机矩阵：Unity compressed、Godot threaded/non-threaded、Phaser/Vite HMR、Colyseus/socket.io、30–300 MB、微信 Android/iOS。
7. 把离线恢复明确到 SLO：CLI 断网后重连 P95、edge 判离线时间、现有 WS 如何通知/重建；参考竞品时区分“控制连接恢复”和“玩家连接无感”。
8. 在竞品表中分开“公共 HTTP 分享”“原始 TCP/UDP game server”“私有 MCP tunnel”，避免被“支持 UDP/AI”带偏。
9. 不新增曝光/孵化路线；只在 14 天验证中补问一个定性问题：“链接拿到后，你是否找得到 5 个愿意点开的人？”用事实决定是否另开招募假设。

## 10. 未查到与未验证

- 未注册/登录任何产品，未测付费 checkout、控制台真实流程。
- 未安装客户端，未做统一带宽、延迟、并发与断线 benchmark。
- 未在微信 Android/iOS 真机查看 ngrok、Pinggy、zrok、localtunnel、Serveo 或 trycloudflare 中间页。
- 未查到 trycloudflare 官方原生警告页说明；也未实开确认“无警告”。
- 未查到 Pinggy、localhost.run 的可靠活跃用户数、MRR/ARR；ngrok 当前收入也未公开。
- 未查到 tunr/uplink/kshare/LocalhostVibe 的独立活跃用户、留存、付费或反馈组件使用量。
- 未查到 zrok、Tunnelmole、Serveo、Loophole 对浏览器应用 WebSocket 的完整、当前、可测试承诺。
- 未查到 localhost.run 免费限速、Tailscale Funnel 带宽限制的具体数值。
- Tailscale 动态价格页本轮无法稳定抓取，未复核 2026-09 的账号套餐精确单价；只能确认 Funnel 对所有计划开放。
- 未确认 Loophole `.cloud` 与 `.run` 的产品/团队关系；两边能力和商业叙述冲突。
- 未验证新品的真实全球节点；只有厂商明确披露的网络归属才写入正文。

## 11. 最终判断

隧道层的“不同”已经非常拥挤：SSH、WSS、QUIC、TCP/UDP、MCP、QR、请求面板都有人做。playtest.run 最可信的空间不是再造一个功能更多的 tunnel，而是把同一件事做完整：

**开发者第一次就拿到能发的链接；玩家看到可信的作品邀请并一按就玩；大构建自动走适合大构建的路径；Web 游戏常见头和 WSS 默认正确；电脑离线有产品化解释；这一版有没有打开、加载成功、报错和收到一句反馈，能直接回答。**

这支持“在同一件事上做得更好”，而不是为了差异而差异。最危险的两种偏航分别是：把门禁页做成新的 ngrok warning，以及在核心闭环尚未验证前把“知道结果”膨胀成分析、曝光和孵化平台。
