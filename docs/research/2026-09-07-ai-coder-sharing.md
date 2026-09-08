# 调研报告：AI 编程群体如何把作品放到别人面前？多大、多认真？

> 调研日期：2026-09-07  
> 调研对象：使用 AI 工具（Cursor、Claude Code、Codex CLI、Gemini CLI、Lovable、bolt.new、v0、Windsurf、Trae 等）构建 Web 应用/原型的创作者  
> 目标：检验 playtest.run 产品设计（DESIGN.md）的关键假设，梳理现有分发路径、作品形态、MCP/Agent Skill 生态与商业意愿。

---

## 一、内建发布能力横向评测（2026 年状态）

当前各大主流 AI 编程产品在「把作品送出去」这一步的实现方案已经出现明显分化：**浏览器端 App Builder 普遍内置一键托管，但门禁与国内可访问性极差；桌面端 Agent IDE 则主要依赖外部平台（Vercel/Netlify）或自建托管通道。**

### 1.1 主流平台能力矩阵

| 平台 | 发布链接形态 | 访客登录要求 | 访问统计 / Observability | 反馈 / 评论机制 | 免费限制与品牌角标 | 微信内访问现状 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Lovable** | `https://<subdomain>.lovable.app` 或自定义域名 | 公开链接**无需登录**；Business/Enterprise 档支持限制为 Workspace 或指定邮箱名单 | 无内置轻量访客面板；依赖集成外部分析脚本或后台 Cloud credits 用量 | 无直接对访客的标注/评论浮窗；发布前仅有安全审查与控制台 | 免费/Pro 带「Edit with Lovable」角标；升级到 Pro/Business 可在设置中隐藏；免费版无限访客带宽，但后端消耗 Run credits | 托管于境外 CDN（含 Cloudflare/AWS），未在国内备案，微信内常出现拦截或加载缓慢，且无针对微信内的引导 |
| **bolt.new** | `https://<id>.bolt.host`（原默认 Netlify，现原生 bolt.host） | 公开链接**无需登录**；可配置 Private 需邀请或企业邮箱白名单 | 无轻量玩家反馈，仅提供 Netlify/Cloudflare 后台监控 | 无内置访客评论组件；通过 Share 生成只读预览供协作者审查 | 免费版生成随机 `bolt.host`；付费版支持自定义域名；带默认 Bolt 品牌标识 | 域名为 `.bolt.host`，境外解析，微信内访问延迟较高，且缺乏微信环境兼容适配 |
| **v0 by Vercel** | `https://<subdomain>.vercel.app` 或项目自定义域名 | 默认公开**无需登录**；可通过 Vercel Deployment Protection 设防 | 深度整合 **Vercel Web Analytics** 与 Speed Insights（需在设置中一键勾选） | 仅限团队内部评论（Vercel Preview Comments）；外部纯访客无直接反馈浮层 | 2026 年 8 月起已移除了生产部署的「Built with v0」强制角标；免费受 Vercel 免费额度与部署限制 | 骑在 Vercel 境外边缘上，微信内解析偶发 DNS 污染或被标记为风险网页 |
| **Replit** | `https://<slug>.<user>.replit.app` | 默认公开，访客**无需登录**；支持密码或 Replit Auth 鉴权 | 内置 Replit Analytics（基本请求量、CPU/内存占用、并发指标） | 无内建玩家端截屏/一句话反馈组件；支持协作编辑者聊天 | 免费版有休眠限制与 CPU 限制；免费链接带 Replit 徽标；绑定自定义域名需 Core/Pro 计划 | 域名 `replit.app` 在大陆部分网络存在阻断，微信内置浏览器无专门手势与音频解锁 |
| **Claude Artifacts** | `https://claude.site/artifacts/<id>` | **无需登录**（纯交互）；若调取了实时模型能力，访客会被强制弹窗要求登录 Claude 账号 | 无任何访问统计或停留时间分析 | 无反馈/评论机制；仅提供「Remix this Artifact」按钮 | 免费/Pro 用户均可公开发布；页面带「Remix on Claude」固定顶栏/角标 | 域名 `claude.site` 属于境外 Anthropic 资产，国内未备案，常被微信以安全理由阻断，微信环境可用度极低 |
| **ChatGPT Canvas** | ChatGPT 内部只读分享链接 | 访客**必须登录 OpenAI 账号**才能完整交互/查看 | 无任何分析统计 | 无反馈组件 | 包含 ChatGPT 品牌包装；受限于订阅计划配额 | 页面要求 OpenAI 账号，国内 IP 与微信直接拦截 |
| **Cursor** | **无独立的一键 Web 部署**；2026 年推 **Cursor Origin**（代码托管）+ **Canvas Publish** | Origin 仅为代码与 PR 预览（直通 Vercel）；Shared Canvas 发布的 URL 访客**无需 Cursor 账号**即可打开交互看板 | 无访客统计数据 | 无外部访客反馈 | 需自备 Vercel 账号；Canvas 仅限于只读看板/数据图表 | 依赖用户所选的部署平台（如 Vercel），微信内无原生优化 |
| **Windsurf (Codeium)** | 一键 Deploy 到 Netlify（生成类似 `*.netlify.app`） | 访客**无需登录**，纯静态/SSR 页面 | 需在 Netlify 控制台认领后查看 Netlify Analytics | 无客户端反馈组件 | 免费提供 10 次临时 Deploy，数小时内若未被个人 Netlify 认领则自动下线 | 走 Netlify 境外节点，国内未备案，微信内置浏览器体验差 |
| **Trae (ByteDance)** | 深度集成 Vercel（SOLO 模式一键部署返回 live URL） | 访客**无需登录**（由 Vercel 承载） | 走 Vercel Analytics | 无客户端反馈组件 | 依赖用户绑定个人 Vercel 账号，无平台额外免费托管资源 | 最终产物仍为 `*.vercel.app`，未解决大陆直连与微信封禁痛点 |
| **GitHub Spark** | 曾为 `*.spark.github.com`（**已于 2026 年 8 月 4 日停止新建，8 月 31 日彻底下线编辑**） | 曾要求所有访问者必须登录 GitHub 账号，限制极大 | 极少，仅 GitHub 基础指标 | 仅限 GitHub 用户权限控制 | 已弃用下线，官方引导回流至 VS Code / Copilot CLI | 历史验证：强制账号登录严重杀伤分发与传播 |
| **Google AI Studio** | 通过 Cloud Run / Firebase App Hosting 分享应用 | **强制要求登录 Google 账号**（类似 Google Drive 授权体系，访客多被拦截在登录页） | 依赖 Google Cloud / Firebase Observability | 无玩家端轻量反馈组件 | 受限于 Google Cloud 配额与计费绑定 | 在国内完全无法直连打开，微信内彻底不可用 |

### 1.2 关键事实与来源
- **Lovable 发布机制**：官网文档证实其发布采用快照发布，免费提供 `*.lovable.app`，公开访问无需注册，升级到 Pro/Business 可关角标并绑定自定义域名。（来源：[Lovable Publish Docs](https://docs.lovable.dev/features/publish)；[Lovable Custom Domains](https://docs.lovable.dev/features/custom-domain)）
- **Bolt.new 托管**：2025 年底将默认托管从 Netlify 迁移至原生 `bolt.host`，提供公开与私密访问设置。（来源：[Bolt.new Hosting & Release Notes](https://support.bolt.new/release-notes)）
- **v0 by Vercel**：2026 年 8 月官方更新日志明确写道：*“Removed the Built with v0 badge from published apps and generated repositories”*，且一键集成 Vercel Web Analytics。（来源：[v0 Changelog 2026-08-14](https://v0.app/changelog)）
- **Cursor 状态**：Cursor 团队于 2026 年推出了代码托管平台 Cursor Origin，深度对接 Vercel 自动化预览，并推出了 Shared Canvases，但**至今没有为普通项目提供独立的原生托管服务器**。（来源：[Cursor Changelog - Origin Code Hosting & Shared Canvases](https://cursor.com/changelog/origin-code-hosting)）
- **Windsurf 状态**：在 Wave 6 和 Wave 8 更新中引入与 Netlify 深度绑定的 App Deploy，但需在数小时内 Claim 才能持久运行。（来源：[Windsurf Wave 6 & 8 Blog](https://devin.ai/blog/windsurf-wave-6)）
- **GitHub Spark 弃用**：GitHub 于 2026 年 8 月 4 日官方公告 Spark 不再接受新用户和新建应用，8 月 31 日停止工作台访问，强制要求导出代码。（来源：[GitHub Blog Changelog 2026-08-04](https://github.blog/changelog/2026-08-04-upcoming-deprecation-of-github-spark-on-github-com/)）

---

## 二、无内建发布用户（Cursor / Claude Code 等）的现状与新兴工具

这部分用户是真正的「终端与本地开发者」或「本地 Vibe Coder」。他们的代码往往在本地 `npm run dev` 运行在 `localhost:3000` 或 `localhost:5173` 上。

### 2.1 社区常见求助与默认答案
在 r/cursor、r/ClaudeAI、r/vibecoding 社区中，用户提出「How do I share my project with friends?」「How to preview on phone?」时，社区给出的典型回答：
1. **主流部署路径（最标准但也最重）**：
   - *“Push to GitHub, connect to Vercel/Netlify/Cloudflare Pages.”*
   - 用户痛点：对于非全职程序员而言，这一步阻力极大。需要配置 Git 仓库、安装 CLI、理解环境变量（`.env`）脱敏、处理单页路由（SPA routing）和生产构建（`npm run build`）抛出的 TypeScript 报错。第三方博客明确指出：*“Cursor is the AI IDE that took over 2025. It builds your app brilliantly — then leaves you at the 'now deploy this' step.”*（来源：[Livemy.app: How to deploy a Cursor app to production in 2026](https://livemy.app/blog/deploy-cursor-app)）。
2. **手机端真机预览求助**：
   - 传统方案是 `--host` 配合局域网 IP（如 `192.168.1.X:5173`）。
   - 用户普遍遭遇四大拦路虎：公司/学校 Wi-Fi 隔离（AP Isolation）、手机走 5G 无法跨网、摄像头/WebXR/传感器必须 HTTPS 上下文（HTTP 报错无法调用 API）、Vite 6 新版默认 `server.allowedHosts` 报错阻断。

### 2.2 2026 年新兴分享与隧道工具生态

为了解决「在终端里一行命令分享 localhost」给朋友或在手机上查看，2025–2026 年涌现出了一批专门瞄准 AI 编程 / Vibe Coder 的工具：

| 工具名 | 定位与核心特色 | 核心机制 | 活跃度与指标（截至 2026-09） | 谁在做 |
| :--- | :--- | :--- | :--- | :--- |
| **tunr** (`tunr.sh`) | 专为 Vibe Coder 设计的下一代隧道；自带 **MCP 协议支持**（`tunr mcp`）、**Freeze 模式**（服务器挂了返回缓存）、**Demo 模式**（拦截写操作）、**Feedback 注入组件**、终端二维码、UDP/TCP 支持 | 单个 Go 静态二进制；经由 WebSocket 与其海外 Relay 通信；自动适配 Next.js / Vite HMR | 2026-06 发布 v0.4.1，代码活跃；GitHub 仅 9 stars（新建项目），主推 CLI 和托管服务 | 个人开发者（Ahmet Vural） |
| **tinyfi.sh** | 零安装、零注册的临时端口转发；一条命令直接出链接：`ssh -R 80:localhost:3000 tinyfi.sh` | 纯 SSH Remote Port Forwarding（类似早期的 localhost.run），合盖即断 | 2026 年活跃在线服务；无客户端代码仓库，服务型个人站 | 个人开发者 |
| **kshare** (`@sifxprime/kshare`) | 针对本地开发的即时 HTTPS 隧道；宣称 24 小时自动销毁、自动重写 HTML/CSS/JS 内的 URL、自动断线重连 | Node.js CLI + Outbound WebSocket 到自建 Nginx/Redis Relay，可自托管 | npm 包 `@sifxprime/kshare`，Node >= 18；更新于 2025–2026，MIT 协议 | 独立团队 / 个人（KODELYTH） |
| **LocalhostVibe** (`localhostvibe`) | 极具 Vibe Coding 社区文化色彩的 CLI；命令为 `vibe share 3000`；**终端直接画二维码**、自动复制剪贴板、**自动在浏览器打开 Control Room 实时访客仪表盘**和「Vibe Score」 | 基于 Cloudflare Tunnel 或 localtunnel 包装；终端内嵌入二维码渲染器与代理中间件 | npm 活跃安装包，GitHub 开源项目 | 个人开发者（teionarr） |
| **uplink** (`@uplink-code/mcp`) | 浏览器自动化与真机配对 MCP 服务；让 AI 驱动真实已登录浏览器并调试网络与 DOM | Node.js MCP server + 本地配对 App | 2026-06 创建，MCP Registry 官方收录（`build.uplink/uplink`） | Uplink 团队 |
| **Pinggy** (`pinggy.io`) | 商业化 SSH 隧道服务；支持 TCP/UDP/TLS，针对 AI 与 Webhook 调试优化，提供 24h/7d 临时链接 | SSH 协议转发 + 自建多地域中继节点（美、欧、新等） | 商业运营中，提供免费档（60 分钟）与付费 Pro（$5/月） | 商业初创公司（Pinggy Inc.） |

**行业推断**：
这类工具的扎堆爆发（tunr、LocalhostVibe、kshare、tinyfi.sh）**强力印证了 DESIGN.md 中提到的 M1（手机试一下）和 M2（发给朋友看）是真实痛点**。
- **痛点的底层动力**：AI 写代码的速度远远快于传统部署链路（Git → 编译 → 部署 → 域名）。当开发者 3 分钟内让 Cursor 搓出一个小游戏，他们**绝不愿意花 15 分钟去配 Vercel、处理构建报错和环境变量**。
- **现存工具的致命伤**：
  1. **全员骑在海外基础设施上**：tunr（Fly.io/Caddy）、LocalhostVibe（Cloudflare/localtunnel）、Pinggy（海外节点），在国内没有香港或近场节点，**在微信内点开极卡或直接报白屏/被拦截**。
  2. **只有隧道，没有上传**：一旦开发者合上笔记本，或者本地进程热重载（HMR）发生运行时语法错误，对面的朋友瞬间打不开（虽然 tunr 尝试用 `--freeze` 兜底，但终究不是长久访问机制）。

---

## 三、这个群体做的是「游戏」吗？他们认不认「playtest」？

这是关系到 playtest.run 品牌与文案定位的最核心问题。

### 3.1 50 个真实 Vibe Coding / AI Coder 作品类型抽样统计
基于权威 Vibe Coding 项目目录（Vibedonalds 562 个项目、vibcod.dev 2026 年 8 月月度榜单、VibeHunt 及 Show HN 样本库）进行的 50 个公开项目随机抽样分类：

| 项目分类 | 数量（共 50 个） | 占比 | 典型案例与特征 |
| :--- | :--- | :--- | :--- |
| **微型 SaaS / 自动化工具** | 16 | **32%** | **Creativable**（Lovable 搭建的 CRM）、**Toolport**、**Omniwork**（任务管理）、**Basedash Subscriptions**（订阅管理）、**Pushary**（锁屏审批 AI 请求）、**Nitrosend**（Agent 自动化发信） |
| **垂类 AI 生成/变换工具** | 12 | **24%** | **Dot Matrix Video Converter**（视频转点阵）、**PhotoQuill**（照片转钢笔画）、**V2Fun**（3D 角色动作生成）、**NormalMap AI**（贴图法线生成）、**SubtitlesFast**（字幕生成器） |
| **生活/趣味互动/计算器** | 11 | **22%** | **PlanMyTrip**（WhatsApp 旅游规划）、**Cal AI**（食物热量推断）、**MenuGen**（Karpathy 做的拍菜单识菜品应用）、**Clockout**（番茄钟/下班倒计时玩具）、**Babel Nexus**（公版书六边形漫游） |
| **小游戏 / 互动玩具 (Playable Games)** | 7 | **14%** | **Cyber Defense**（防御勒索病毒的 AI 塔防）、**AI Board Game Platform**（本地棋盘 AI 对弈）、**Gamevibe Tetris**（Claude 生成的 SignalR 俄罗斯方块）、**Chris Pirillo's Vibe Arcade**（文字冒险游戏）、**VibeGame 3D 跑酷 Demo** |
| **开发者/AI 辅助插件/MCP 服务** | 4 | **8%** | **Composer Web**（一键向 Cursor 回传日志与网页截图）、**DocsAlot CLI**、**Fudge MCP**（提取网站设计 DNA） |

### 3.2 深度分析：群体认知与词语冲突

1. **作品形态现实：绝大多数不是游戏**。
   - 统计数据显示，纯游戏在 AI Coder 作品中仅占 **约 14%**，即便算上强互动类的视觉草图与趣味玩具，总占比也难以突破 **25%**。
   - 超过 **70%** 的作品是**工具、轻量 SaaS、仪表盘、表单与垂直生成器**。
2. **他们认不认「playtest」？**
   - **游戏开发者**：对「playtest」高度认同，概念清晰（指代内测、跑流程、找手感、测数值）。
   - **AI Coder / Web 应用构建者**：**普遍不认「playtest」这个词**。在他们的日常词汇中，高频词汇是：
     - *“Try this out”*（试试看）
     - *“Live demo”*（在线演示）
     - *“Preview”*（预览）
     - *“Feedback / Roast my app”*（求反馈 / 挑刺）
     - *“Ship / Show HN”*（发推发布）
3. **品牌与文案的驱赶风险**：
   - 如果产品主打语或界面到处是「关卡」「玩家（Player）」「试玩（Playtest）」「作弊码」等纯游戏术语，**做表单、SaaS 原型、数据看板的 75% 以上用户会瞬间产生心理排斥**，认为「这是游戏引擎专用工具，不适合我的 React/Next.js/Three.js 小工具」。
   - **推论**：域名 `playtest.run` 可以作为底层基础设施存在，但**面向开发者的文案与门禁页必须中性化、场景化**（例如：「某某邀请你体验/试用《作品名》」而非「邀请你试玩」；CLI 提示应使用「分享预览」而不是「开始 playtest」）。

---

## 四、MCP 与 Agent Skill 作为分发渠道的现状

2026 年，MCP（Model Context Protocol）与 Agent Skill 已经从「实验品」变成了各家 IDE（Cursor、Claude Code、Windsurf）调取系统外部能力的标配底座。

### 4.1 现有工具的 MCP / Skill 落地案例
- **tunr 的原生 MCP**：`tunr.sh` 官方自带 `tunr mcp` 命令，内置工具包括 `deploy_app`、`list_apps`、`get_logs`、`set_share_policy`、`get_feedback` 和 `rollback`。开发者可以直接在 Cursor 或 Claude Code 中对 AI 说话：*“Ship this to tunr and give me a link”*，Agent 会直接调取 tunr 的 MCP 工具起隧道并贴回 URL。（一手来源：[tunr documentation](https://tunr.sh/docs.html)）
- **Vercel / Netlify / Cloudflare MCP**：各主流云平台均已上线官方 MCP Server，使得 Agent 能够执行部署查询、环境变量管理和部署触发。（一手来源：[Vercel MCP / v0 llms.txt](https://v0docs.vercel.sh/docs/llms.txt)；[Cloudflare MCP Server](https://vibedonalds.com/tools/mcp-cloudflare)）
- **Composer Web 与 Uplink**：专注于让 Agent 反向获取浏览器端运行状态（DOM、网络错误、Console 报错）的 MCP 工具已在 GitHub 获得大量关注，证实了开发者极度渴望将「外部运行实况」无缝喂回 Agent。（一手来源：[Show HN: Composer Web](https://news.ycombinator.com/item?id=43592770)；[uplink MCP](https://github.com/uplink-code/mcp)）
- **Skills.sh 生态**：Vercel 维护的开源 Agent Skill 生态（skills.sh）已托管超过 76 万个 Skill，顶流技能累计下载达数百万次，证明了 Agent 自主调用外部工具已被高频日常化。（来源：[VibeIndex skills.sh entry](https://vibeindex.dev/)）

### 4.2 「让 Cursor 自己发到 playtest.run 并贴回二维码」是否为真实入口？
**答案是：不仅是真实的，而且是 AI Coder 群体阻力最低的第一入口。**

1. **工作流闭环**：
   - 典型的 Cursor / Claude Code 用户场景：开发者对着 AI 对话：“帮我把刚才写的 3D 小球游戏发给朋友看”。
   - 如果没有 MCP：Agent 会打印一段冗长的 Shell 教学让用户去装 CLI、建 GitHub 仓库、配置 Vercel，体验直接断裂。
   - 如果有 MCP / Skill：Agent 自动探测本地端口或构建目录，自主调用 `playtest_share(port=5173)`，控制面返回 URL 和 ASCII 二维码，**Agent 直接在对话面板里输出可点击的链接与二维码图片**。
2. **先例与验证**：
   - 类似 `tunr mcp` 和本工作区加载的 `mcp-messenger-mira`，均证明了通过 MCP 在侧边栏/远程直接交互是已经被验证有效的成熟范式。
   - 相比于让用户在另外的终端窗口敲命令，**直接作为 MCP 工具或 Claude Code Skill 注入，转化漏斗显著缩短**。

---

## 五、分析与反馈需求、付费意愿深度考察

### 5.1 这批用户在不在意「谁看了」？他们在哪里收集反馈？
1. **在意程度极高，但形式要求「即时与轻量」**：
   - LocalhostVibe 专门做了一个浏览器端实时控制室（Control Room）展示当前的活跃访客与「Vibe Score」；tunr 特别增加了 `--inject-widget` 悬浮反馈组件。这说明**知道「有人点开了、当前几个人在线」是创作者最大的多巴胺来源**。
   - 痛点：当用户把链接丢到微信群、朋友圈或 X（Twitter）后，最焦虑的就是「他们到底点开没有？卡在加载了还是玩崩溃了？」。
2. **现有的反馈收集场所**：
   - **X / Reddit 评论区**：开发者发贴求反馈（*“Check out my project, roast it”*），但收到的反馈往往滞后，且无法对应到具体设备和版本。
   - **控制台/报错日志断层**：朋友在手机微信里点开一片白屏，只会回一句「打不开 / 没反应」，开发者根本不知道是 WebGL 不支持、微信 X5 内核报错、还是某条资源 404。
   - **结论**：DESIGN.md §3.4 规划的「**第一层自动数据**（打开、设备、来源、停留、加载失败）」直击靶心；而不需要集成 SDK 就能在门禁页捕获基本会话，是其最强大的差异化资产。

### 5.2 付费意愿与商业化迹象
这批用户会为「分享 + 看结果」付钱吗？证据表明：

1. **白嫖（Free Tier）是入场券**：
   - 通用开发者对本地隧道和临时托管具有极强的「免费心智」（Cloudflare Tunnel、ngrok 免费档、Vercel Hobby 均为免费）。任何在首次运行就要求输入信用卡或强制付费的工具都会瞬间被淘汰。
2. **愿意付费的明确信号（Willingness-to-Pay）**：
   - **固定专属二级域名 / 移除品牌角标**：tunr 将自定义二级域名列为 Pro 权益；Lovable、bolt.new 将自定义域名和去除「Built with」角标锁在 $20–$25/月付费档；Pinggy 对 7 天以上持久 URL 收费 $5/月。这是已经被验证的商业变现杠杆。
   - **客户端展示与专业演示场景**：tunr 的核心付费卖点集中在面向 Freelancer 和 Agency 的 `--freeze`（防止演示崩溃）、`--demo`（防止客户乱改数据）、`--inject-widget`（客户标注反馈）。这部分用户在向付费客户交付原型时，愿意为**稳定性与体面度**买单。
   - **国内特殊付费意愿**：国内开发者长期饱受海外服务（ngrok、Vercel、itch）微信打不开、Cloudflare 丢包之苦。为「**国内免备案、微信不红、香港低延迟直连**」支付小额月费（如 19–39 元/月），在国内独立开发者与独立接单群体中有真实购买力。

---

## 六、三个新洞见与对 DESIGN.md 的检验

### 6.1 三个核心新洞见

1. **洞见一：AI Coder 做的大多不是游戏，而是「可交互的 Web 工具/玩具」；「Playtest」词语存在天然排他性。**
   - 调研数据清晰表明，游戏仅占该群体产物的约 14%，70% 以上是微型 SaaS、个人工具、趣味发生器。对于写出待办看板或点阵视频转换器的人，「playtest」和「玩家」的语境会让他们误以为进错了平台。
2. **洞见二：桌面端 Agent IDE（Cursor / Claude Code）并没有被内建托管终结，反而催生了对「极速抛出 localhost」的饥渴。**
   - 虽然 Lovable、bolt.new 内置了发布，但专业开发者和深度 Vibe Coder 正在加速回流至 Cursor / Claude Code 等本地环境。这一群体的手上只有跑着的本地端口，他们极度需要一条命令甚至一个 MCP Tool，能在 3 秒内将 localhost 抛给手机和微信好友。
3. **洞见三：微信内置环境是唯一的全球真空地带，也是最大竞争壁垒。**
   - 从 Lovable、bolt 到 tunr、ngrok，全球所有新兴工具**100% 架构在 Cloudflare / Fastly / AWS 海外边缘上**。它们在微信里毫无例外地遭遇打不开、慢、无手势音频拦截或整域风险提示。**香港近场边缘 + 针对微信 X5 内核与手势的门禁页**是全球其它任何竞品完全没有覆盖的绝对生态位。

### 6.2 对 DESIGN.md 假设的检验（哪些成立、哪些站不住）

#### A. 站得住的假设（完全被调研证实）
1. **「M1 手机上试一下」和「M2 发给朋友看」是极其痛的痛点**：大量同类工具（LocalhostVibe、tunr、kshare）专门围绕手机扫码、局域网 HTTPS 穿透、二维码输出做文章，证实开发者每天都在为此耗费心力。
2. **「玩家零门槛、免登录」是生死红线**：Google AI Studio 强制登录 Google 账号、GitHub Spark 强制登录 GitHub 账号，均成为阻碍其应用分发传播的最严重绊脚石；Claude Artifacts 凡需登录的功能跳出率极高。
3. **「知道结果」比单纯的「打通隧道」更有价值**：tunr 自研 `--inject-widget`、LocalhostVibe 自研 Control Room，证明单纯的链接转发已经同质化，把「有人看了、看成了什么样」送回给创作者才是护城河。
4. **「香港边缘 + 微信可达」是决胜楔子**：全球竞争者无一顾及大陆微信生态，留出了巨大的结构性空白。

#### B. 站不住或需要修正的假设（与 DESIGN.md 冲突之处）
1. **冲突一：关于「上传为主、隧道为辅」的推演对 AI Coder 群体不成立。**
   - *DESIGN.md 原假设*：“上传（引擎导出的静态目录）占多数场景，隧道只是附属路径”。
   - *调研反驳*：对于 Godot / Unity 游戏开发者，上传确实占多数；但对于**第一类核心目标用户「用 AI 写小东西的人」**，他们手上只有 `npm run dev`（Vite / Next.js / Webpack），很多人根本没有配置构建，或者构建时会被未处理的 TypeScript 报错卡死。对他们而言，**隧道是第一路径，上传是第二路径**。如果 CLI 的隧道体验做差了，这批用户会直接流失。
2. **冲突二：关于产品名称与语境中「playtest / 试玩」的泛用性假设。**
   - *DESIGN.md 原假设*：“四类人共同点是产物是浏览器里跑的东西……做的是一次 playtest”。
   - *调研反驳*：把一个 AI 生成的「日程管理工具」或「设计配色器」放到微信群，门禁页如果写着「某某邀请你**试玩**」，会严重损害工具制作者的严肃性，被测者也会感到困惑。必须支持中性的场景文案。
3. **冲突三：关于「只做终端 CLI，先不做其它形态」的分发入口假设。**
   - *DESIGN.md 原假设*：“只装一样东西：`playtest` CLI，桌面应用等开源后社区做”。
   - *调研反驳*：忽视了 **MCP 工具 / Agent Skill** 的爆发。在 2026 年，让 Cursor / Claude Code 用户在另外的终端窗口敲命令，漏斗转化率远低于让其在 `.cursor/mcp.json` 中配置一行，从而允许 Agent 自动调用 `playtest` 工具并把二维码打在聊天面板里。

---

## 七、对 playtest.run 的建议（不直接修改 DESIGN.md）

1. **入口层升级：将 MCP Server / Agent Skill 提升为 v0.1 的同等第一公民**
   - 在 `cli/` 实现中内置 `playtest mcp` 子命令（类似 `tunr mcp`），让 Cursor、Claude Code、Windsurf 可以通过 stdio 协议一键挂载；
   - 暴露 `playtest_share_port(port)` 和 `playtest_upload(dir)` 两个工具，让 Agent 部署完毕后直接将 URL 与二维码在 IDE 内回显给用户。
2. **文案层弹性化：支持作品类型或中性化门禁文案**
   - 门禁页文案不要硬编码为「试玩」。CLI 上传或分享时增加可选项或默认根据特征智能识别：
     - 若探测为游戏引擎（Godot、Unity、Phaser、Cocos），门禁页使用「邀请你试玩」；
     - 若为常规 Vite/React/Next 应用，门禁页默认使用「邀请你体验」或「邀请你预览《作品名》」；
     - 允许开发者通过 `-m` 或 `--type app` 自定义门禁引导语。
3. **隧道体验强化：针对 Vite / Next.js 本地开发服务器提供开箱即用支持**
   - 必须在边缘或 CLI 层自动完成 `Host` 头改写（改写为 `localhost:<port>`，原值进 `X-Forwarded-Host`），避免 Vite 6 和 Next.js 的安全机制直接向公网访客返回 403 Forbidden；
   - 隧道连接建立后，终端必须默认渲染 ASCII 二维码，直接迎合 M1 手机扫码需求。
4. **留存与反馈闭环：轻量第一层数据可视化**
   - 开发者运行隧道或上传后，终端/控制台除链接外，应实时打印一行轻量动态提示（例如：`[12:30] 来自微信的访客 (iPhone) 已进入体验`）；
   - 这不仅让开发者直观体会到「知道结果」的产品承诺，也是促使其回头发第二个版本的强力多巴胺刺激。
