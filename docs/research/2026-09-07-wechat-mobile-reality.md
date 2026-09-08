# 2026 年微信与移动端现实：playtest.run 调研

调研日期：2026-09-07

## 0. 结论先行

1. **“微信里点开就能玩”目前不是一个可以公开承诺的稳定能力，而是政策与真机双重待验证假设。** 微信于 2025-10-23 更新并生效的《微信外部链接内容管理规范》把“以游戏、测试等方式吸引用户参与互动”的“H5 游戏、测试类内容”列为常见违规情形，例子直接包括“网页小游戏”。这不是某个用户上传违规内容后才出现的风险，而是 playtest.run 正常用途本身落在规则文字覆盖范围内。[一手来源：微信外部链接内容管理规范](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)
2. **域名整域处理确实存在，但“一个页面被举报就自动按注册根域连坐”不是官方机制。** 官方写的是：先处理违规链接；同域名下“大量链接”违规，处理后仍未及时、有效整改，才可处理该域名及其全部链接。域名分离仍能保护品牌与登录安全，但 Public Suffix List 不等于微信会把每个 slug 当独立域名。[一手来源：规范 §3.1.1、§3.1.5](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)
3. **“大陆到香港 30–60 ms”只能是优质回程的目标，不是地理事实。** 2026 年公开 ITDOG 样本中，优化线全国三网平均约 34–56 ms；同样标称香港的普通国际回程，全国平均可达 194–319 ms，最慢节点达 376 ms，路由甚至绕美欧。因此，“普通线路可达 300–400 ms”的**数值范围有存在性样本支持**，但页面没有晚高峰时间戳，不能支持“这是晚高峰造成”或“普通线路晚高峰普遍如此”。[二手测速样本：吉云香港](https://vpsxb.net/8108/)、[Zenlayer 香港](https://vpsxb.net/8190/)
4. **微信分享卡片不能按现设计正式保证。** 官方自定义标题、摘要、图片依赖 JS-SDK 签名；JS 接口安全域名须 ICP 备案，最多 5 个父域，且不接受短链。playtest.run 若坚持境外、不备案，只能提供服务端直出的 `<title>`/Open Graph 元数据并标注“尽力而为”，不能承诺微信会抓到。[一手来源：微信 JS-SDK](https://developers.weixin.qq.com/doc/service/guide/h5/jssdk.html)
5. **GitHub 不能称为“大陆基本可用”的可靠登录依赖。** GreatFire 在 2026-09-06 的页面称最近 55 次有效测试中 67% 受到干扰；OONI 对 `https://github.com/` 的中国探针在 2026-07-01 至 09-07 共 76 条中，41 条 anomaly、7 条 failure、28 条 clear。两者都不能证明 OAuth 回调必然失败，但足以否定“稳定可用”。[开放测量：GreatFire](https://en.greatfire.org/https/github.com)、[OONI API](https://api.ooni.io/api/v1/measurements?probe_cc=CN&input=https%3A%2F%2Fgithub.com%2F&since=2026-07-01T00%3A00%3A00&until=2026-09-07T23%3A59%3A59&limit=100)

## 1. 方法、证据等级与限制

- **一手来源**：微信/腾讯、Apple/WebKit、Chromium/Google、Telegram 官方文档或公告。
- **开放测量**：OONI、GreatFire、ITDOG 截图。它们是观测，不自动等于封锁归因；OONI 的 `anomaly` 也不等于已确认封锁。[OONI Web Connectivity 方法说明](https://ooni.org/nettest/web-connectivity/)
- **用户原话**：微信开放社区问题页。它们能证明有人在某机型/版本遇到问题，不能证明全体设备支持率。
- **二手/推断**：VPS 测评站、MDN Browser Compatibility Data，以及从浏览器内核能力推断宿主 WebView 的部分。
- 本轮没有新增大规模抓取，使用中断前已收集的官方页、开放 API、社区记录与测速原页。
- **没有微信 Android/iOS 真机、没有微信账号内分享实验、没有大陆三运营商测试卡，也没有在晚高峰连续压测。** 因而“当前可打开/当前被封”的域名不能凭桌面公网访问代替微信真机结论。
- 微信开放社区正文当前由 JavaScript 加载；下文“用户原话”取自公开搜索索引与页面记录，保留原链接，但不把它提升为官方结论。

## 2. 微信可达性现实表

| 能力 | 平台 | 结论 | 来源 |
|---|---|---|---|
| 内核版本 | Android 微信（XWeb） | **没查到微信版本→XWeb→Chromium 的官方稳定映射。** 历史 UA 可出现 `Chrome/107` 与 `XWEB/5315`，说明内核可独立更新；不能按微信 App 版本或“X5”三个字推断能力 | [用户原话/历史 UA，2023-11-29](https://developers.weixin.qq.com/community/develop/doc/0002cc182483c0ac49b09eef666800) |
| 内核版本 | iOS 微信（WKWebView/WebKit） | 主要随 iOS 的 WebKit 版本变化，不随微信单独携带 Chromium；具体微信版本映射没查到 | [一手：Apple App Review Guidelines §2.5.6](https://developer.apple.com/app-store/review/guidelines/) |
| X5/TBS 版本参照 | Android 腾讯 X5 SDK；**不是微信 XWeb 证明** | 腾讯商业 X5 最新公开版 `48445`，2026-05-08 更新，基于 Chromium 121，支持 Android 6–16；旧免费 SDK 基线 Chromium 89 | [一手：X5 新版本](https://st.tencent-cloud.com/jax-static/tbs/AboutX5NewVersion%20072301.md)、[一手：新旧版对比](https://st.tencent-cloud.com/jax-static/tbs/SDKCompare073102.md) |
| WebGL 2 | Android 微信 | Chromium 89/121 基线应支持；腾讯宣称 X5 “WebGL 适配度 95%”，但未披露样本与微信版本。预计多数新机可用；**没查到 2024–2026 微信 H5 多机型实测覆盖率**，须启动时创建 context 检测 | [一手但属厂商指标：X5 产品简介](https://st.tencent-cloud.com/jax-static/tbs/aboutX5073101.md) |
| WebGL 2 | iOS 微信 | WebKit 已长期支持 WebGL 2，但宿主、机型和内存仍影响结果；须运行时检测 | [二手兼容数据：MDN](https://developer.mozilla.org/en-US/docs/Web/API/WebGL2RenderingContext#browser_compatibility) |
| WebGPU | Android 微信 | Chromium 121 的 Android WebGPU 只在 Android 12+、特定 Qualcomm/ARM GPU 默认开放；微信 XWeb 是否同步没查到。DESIGN 所说“不可靠”成立 | [一手：Chrome 121 WebGPU](https://developer.chrome.com/blog/webgpu-release) |
| WebGPU | iOS 微信 | Safari/WebKit 26 在 iOS/iPadOS 26 正式提供 WebGPU；旧系统不能由升级微信补齐。按 `navigator.gpu` 检测，不能按微信 UA 判断 | [一手：Safari 26](https://webkit.org/blog/17333/webkit-features-in-safari-26-0/) |
| WebAssembly 单线程 | Android/iOS 微信 | 新旧 Chromium 与现代 WebKit 均有基线支持；单线程 Wasm 可作为基线，但压缩、MIME、内存与大包仍须作品级实测 | [一手：X5 89/121 基线](https://st.tencent-cloud.com/jax-static/tbs/SDKCompare073102.md)、[一手：Safari 26 WebAssembly 更新](https://webkit.org/blog/17333/webkit-features-in-safari-26-0/) |
| Wasm 线程 / SharedArrayBuffer | Android 微信 | 需要安全上下文、COOP/COEP 与宿主允许跨源隔离；**没查到 2024–2026 微信 Android 对 `crossOriginIsolated` 的可靠真机矩阵** | [一手：SharedArrayBuffer 安全要求](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer#security_requirements) |
| Wasm 线程 / SharedArrayBuffer | iOS 微信 | WebKit 从 Safari 15.2 支持 COOP/COEP；两头正确时可启用 SharedArrayBuffer/Wasm 线程，但微信 WKWebView 路径仍未真机复核 | [一手：Safari 15.2](https://webkit.org/blog/12140/new-webkit-features-in-safari-15-2/) |
| COOP/COEP 头生效 | Android/iOS 微信 | 浏览器基线具备不等于微信宿主一定放行；边缘正确发头是必要非充分条件。先检测 `crossOriginIsolated`、`SharedArrayBuffer`，失败才提示换浏览器 | [一手：跨源隔离指南](https://web.dev/articles/coop-coep)、[一手：Safari 15.2](https://webkit.org/blog/12140/new-webkit-features-in-safari-15-2/) |
| Web Audio / 有声媒体自动播放 | Android 微信 | 腾讯 TBS FAQ 明确微信不允许视频自动播放，首次播放需用户操作；Chrome Web Audio 自 Chrome 71 起受自动播放策略限制 | [一手：TBS FAQ](https://st.tencent-cloud.com/jax-static/tbs/QA.md)、[一手：Chrome autoplay](https://developer.chrome.com/blog/autoplay/)、[用户原话，2024-05-23](https://developers.weixin.qq.com/community/develop/doc/000e640d77cfa001132a6cb8456c01) |
| Web Audio / 有声媒体自动播放 | iOS 微信 | WebKit 对有声媒体要求直接用户手势；静音视频可例外。门禁页“开始”按钮是正确设计，音频初始化必须直接发生在 click/touch 回调内 | [一手：WebKit iOS 媒体策略](https://webkit.org/blog/6784/new-video-policies-for-ios/) |
| Fullscreen API | Android 微信 | Chromium API 存在，但 TBS 文档要求宿主实现全屏回调；微信是否完整实现没查到。游戏不能依赖 Fullscreen 成功 | [一手邻近证据：TBS WebView 特性](https://st.tencent-cloud.com/jax-static/TBS-WebIndex/1003/webview-feature.md) |
| Fullscreen API | iOS 微信 | MDN BCD 当前记录：iOS Safari 仅 iPad 部分支持，iPhone 不支持；iOS WebView 标为不支持。沉浸模式只能渐进增强 | [二手兼容数据](https://github.com/mdn/browser-compat-data/blob/main/api/Element.json) |
| DeviceOrientation / 陀螺仪 | Android 微信 | Chromium/WebView 通常可用；微信真机授权与精度矩阵没查到 | [二手兼容数据](https://github.com/mdn/browser-compat-data/blob/main/api/DeviceOrientationEvent.json) |
| DeviceOrientation / 陀螺仪 | iOS 微信 | iOS 14.5+ 的 `requestPermission()` 可用，须由用户手势触发。失败应给触控替代 | [二手兼容数据](https://github.com/mdn/browser-compat-data/blob/main/api/DeviceOrientationEvent.json) |
| 横屏锁定 | Android 微信 | Android Chrome/WebView 支持 `screen.orientation.lock()`，但宿主/全屏条件可能影响 | [二手兼容数据](https://github.com/mdn/browser-compat-data/blob/main/api/ScreenOrientation.json) |
| 横屏锁定 | iOS 微信 | Safari/iOS 当前不支持 `ScreenOrientation.lock()`；iPhone 不能可靠锁横屏，只能自适应并提示旋转 | [二手兼容数据](https://github.com/mdn/browser-compat-data/blob/main/api/ScreenOrientation.json) |
| 摄像头 `getUserMedia` | Android 微信 | TBS 文档称支持且需授权；用户报告同微信版本、不同手机结果不一致。属于“尝试并降级”能力 | [一手邻近证据：TBS FAQ](https://st.tencent-cloud.com/jax-static/tbs/QA.md)、[用户原话，2024-04-10](https://developers.weixin.qq.com/community/develop/doc/0002a4828ecfd887333a62e895c800) |
| 摄像头 `getUserMedia` | iOS 微信 | 用户报告 iOS 17 以下微信失败而 Safari 正常，Android 微信可用；这是单例，不是覆盖率统计 | [用户原话](https://developers.weixin.qq.com/community/develop/doc/000646a15c03104adcda8c72451800) |
| Service Worker | Android 微信 | TBS FAQ 称 X5 SDK 支持；微信 XWeb 的注册、保活、清理策略没查到。不应作为首次打开必要条件 | [一手邻近证据：TBS FAQ](https://st.tencent-cloud.com/jax-static/tbs/QA.md) |
| Service Worker | iOS 微信 | WebKit 支持；Safari 的 ITP 会在 7 天无用户交互后清除脚本可写存储，包括 Service Worker 注册与缓存；该规则不能直接等同微信 WKWebView | [一手：WebKit ITP](https://webkit.org/blog/10218/full-third-party-cookie-blocking-and-more/) |
| 30–100 MB 下载与缓存 | Android 微信 | TBS SDK 可配置 HTTP 磁盘缓存仅 20–80 MB；这不是微信默认配额。**没有证据支持 100 MB 构建能稳定二次命中缓存** | [一手邻近证据：TBS WebView 特性](https://st.tencent-cloud.com/jax-static/TBS-WebIndex/1003/webview-feature.md) |
| 30–100 MB 下载与缓存 | iOS 微信 | 微信 WKWebView 的单站点额度与清理时机没查到；应测首包、断点、后台切回、低存储设备 | 没查到 |
| `<canvas>` / WebGL 性能 | Android/iOS 微信 | 没查到 2024–2026 微信 H5 的统一 FPS、内存上限或热降频实测；不能给性能承诺 | 没查到 |
| 软键盘与 viewport | iOS 微信 | 用户报告 iPhone 微信输入框突然无法录入而 Safari 正常；另有键盘收起后视觉恢复但点击区域仍错位。反馈框需做 `visualViewport` 真机测试 | [用户原话，2025-09-01](https://developers.weixin.qq.com/community/develop/doc/000626cd4f4be8f2b262a9b1b61000)、[用户原话](https://developers.weixin.qq.com/community/develop/doc/0004e4304f8d78c83562648396b400) |
| `<a>` 文件下载 | Android/iOS 微信 | 微信宿主如何处理通用下载没查到；官方把“文件下载类链接”列为来源不明/安全性未知风险链接例子。玩家路径不应依赖下载 | [一手：规范 §2.18.3](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification) |
| PWA/添加桌面 | 微信内置浏览器 | 微信内不是可依赖的安装入口；PWA 是系统浏览器回访优化，不是玩家首开流程 | [Google：Chrome 安装条件](https://web.dev/articles/install-criteria) |
| PWA/添加桌面 | iOS Safari 26 | iOS/iPadOS 26 的 Safari 可把任意网站加入主屏并按 Web App 打开；这不等于微信内可直接安装 | [一手：Safari 26](https://webkit.org/blog/17333/webkit-features-in-safari-26-0/) |

### 2.1 能力判断的产品底线

- “支持”应定义为某个真实导出物在指定微信/iOS/Android 组合中通过，而不是“对应 Chromium/WebKit 支持 API”。
- 门禁页的同一次明确点击，应同时用于音频解锁、传感器授权和开始加载，避免新增玩家步骤。
- 首屏必须不依赖 WebGPU、Wasm 线程、Service Worker、Fullscreen、横屏锁定或摄像头。
- `--isolated` 应采用能力探测，而不是 UA 黑名单。检测失败时再显示“复制链接/系统浏览器打开”；这仍需评估是否触及微信规范 §2.3.3 的“强制跳转外部 APP”。[一手来源：微信外部链接规范 §2.3.3](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)

## 3. 微信封禁：规则、申诉与案例

### 3.1 官方规则实际写了什么

2025-10-23 生效的规范适用于“非由微信公众平台产生且在微信内传播的外部链接”，列出的常见违规与 playtest.run 直接相关：

- **§2.5 H5 游戏、测试类内容**：“以游戏、测试等方式，吸引用户参与互动”，例子包括“网页小游戏”。这是本次最重要的新事实。[一手来源](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)
- **§2.3.3 强制跳转**：页面内容不完整，通过弹窗、频繁提示等强制用户下载或跳到外部 App，影响正常浏览。若门禁页在微信内只展示“去浏览器打开”而不给可玩的路径，有规则风险。[一手来源](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)
- **§2.18.3–2.18.4 风险链接**：来源不明、安全性未知、文件下载、存在漏洞，以及未依法备案或难以追溯运营者身份的网站链接，都可能被处理。[一手来源](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)
- **§3.1.1**：可处理单链接、域名或 IP，也可停止传播或在朋友圈不可见。[一手来源](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)
- **§3.1.5**：整域处理的明文条件是“同域名下大量链接违规，经处理后仍未及时、有效整改”。没有写“一次举报自动封注册根域”。[一手来源](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)
- **§3.2.2、§3.2.4**：多域名、多账号规避，以及嵌套多级跳转骗取点击，属于对抗。域名轮换、短链套娃不能成为产品的“抗封架构”。[一手来源](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)
- **§3.3**：关联主体多次违规时，腾讯可不再提供申诉渠道，并限制关联主体链接/域名。[一手来源](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)

### 3.2 申诉入口、时长与成功率

- 首次违规修改后，可在拦截页点击“申请恢复访问”提交资料；**不申请不会自动解封**。[一手来源：腾讯客服](https://kf.qq.com/faq/170118UnqeUZ170118mUb6fu.html)
- 最近半年累计违规的公开阶梯为：第二次 **12 小时**、第三次 **1 天**、第四次及以上 **1 周**；重复、严重或多次违规可另行加重。以上是 2026-09-07 抓取页面时的公开口径。[一手来源：腾讯客服](https://kf.qq.com/faq/170118UnqeUZ170118mUb6fu.html)
- 朋友圈“仅自己可见”另有 `moment@tencent.com` 反馈路径。未备案一级域名每天分享至朋友圈有频率限制，达到一定次数后再分享仅自己可见。[一手来源：腾讯客服](https://kf.qq.com/faq/170118UnqeUZ170118mUb6fu.html)
- 微信开放社区管理员曾回复“三个工作日处理”，但这只是社区处理口径，不是恢复 SLA；腾讯客服还明确写了申请过多时无法较快处理。[一手/社区公告口径](https://developers.weixin.qq.com/community/develop/doc/000e4e18d581809f129dcca3651400)
- **没查到 2024–2026 可复核的申诉成功率统计，也没查到能承诺“24 小时恢复”的官方 SLA。** DESIGN 的“24 小时内提交”只能是内部响应目标。

### 3.3 域名封禁案例表

检查口径：截至 2026-09-07 的公开资料扫描；未做微信真机访问。**“没查到整域封禁证据”不等于当前正常。**

| 平台 | 是否被封过 | 时间 | 结果 |
|---|---:|---:|---|
| 某新上线企业站 | 是 | 2025-10-13 | 用户称先出现“无法确认安全性”，后变为“可能被恶意利用”并停止访问；单案例，未披露最终解封结果。[用户原话](https://developers.weixin.qq.com/community/develop/doc/000406b38c870090cb641e0cb6bc00) |
| `newmom.cn` | 是 | 2025（具体日没查到） | 用户称购入旧域后继承历史微信封禁，提示域名信誉可能跨所有者延续；单案例。[用户原话](https://developers.weixin.qq.com/community/minihome/question/1522255900122628102?page=16&tag=hot&type=0) |
| [草料二维码](https://cli.im/) | 没查到当前整域证据 | 2024–2026 | 未真机验证；不能据此判定正常 |
| [金数据](https://jinshuju.net/) | 没查到当前整域证据 | 2024–2026 | 未真机验证 |
| [麦客表单](https://www.mikecrm.com/) | 没查到当前整域证据 | 2024–2026 | 未真机验证 |
| [问卷星](https://www.wjx.cn/) | 没查到当前整域证据 | 2024–2026 | 未真机验证 |
| [Notion 公开页](https://www.notion.so/) | 没查到当前整域证据 | 2024–2026 | 大陆公网可达性与微信拦截是两个问题，未真机验证 |
| [语雀](https://www.yuque.com/) | 没查到当前整域证据 | 2024–2026 | 未真机验证 |
| [飞书文档](https://www.feishu.cn/product/docs) | 没查到当前整域证据 | 2024–2026 | 未真机验证 |
| [腾讯文档](https://docs.qq.com/) | 没查到当前整域证据 | 2024–2026 | 未真机验证 |
| `*.github.io` / GitHub Pages | 没查到“所有兄弟子域因一站违规被整域封”的新案例 | 2024–2026 | GitHub 网络干扰另见 §7；未真机验证 |
| `*.vercel.app` | 没查到可复核的新整域案例 | 2024–2026 | 找到过较早“微信打不开、复制到浏览器”的搜索摘要，但无法复核原文，故不作为证据 |
| `*.netlify.app` | 没查到当前整域证据 | 2024–2026 | 未真机验证 |
| `*.pages.dev` | 没查到当前整域证据 | 2024–2026 | 未真机验证 |
| [itch.io](https://itch.io/) | 没查到微信整域封禁证据 | 2024–2026 | 大陆速度/连接问题不等于微信封禁；未真机验证 |
| [Glitch](https://glitch.com/) | 没查到当前整域证据 | 2024–2026 | 未真机验证 |
| [CodePen](https://codepen.io/) | 没查到当前整域证据 | 2024–2026 | 未真机验证 |
| `*.repl.co` / `*.replit.app` | 没查到当前整域证据 | 2024–2026 | 未真机验证 |
| `*.lovable.app` | 没查到当前整域证据 | 2024–2026 | 未真机验证 |

这张表不能支持“用户内容域迟早必然整域被封”的概率判断：公开案例存在，官方也保留整域处理权，但没有平台级基准率、分母、申诉成功率或兄弟子域连坐统计。能确认的是，playtest.run 的风险比普通用户内容平台更直接，因为官方 §2.5 明列 H5 游戏。[一手来源：微信外部链接规范](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)

## 4. 微信里的常见通行做法

### 4.1 “右上角在浏览器中打开”遮罩

- **没查到 2024–2026 有分母、有实验设计的转化率数据**，无法回答多少玩家会照做。
- 〔推断〕它会多出理解提示、点菜单、找选项、切浏览器等步骤，与“玩家零门槛”冲突。
- 若页面在微信内故意不提供完整内容、只强制跳系统浏览器，可能触及规范 §2.3.3；如果 `--isolated` 运行时检测失败，应解释为设备兼容降级，而不是把所有微信访问统一挡掉。[一手来源：微信规范](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)

### 4.2 短链与二维码中转

- 微信 JS-SDK 明确不允许把短链域名配置为 JS 接口安全域名。[一手来源](https://developers.weixin.qq.com/doc/service/guide/h5/jssdk.html)
- 多域名规避和嵌套多级跳转可被认定为对抗。草料、缩我等只适合打印/跨设备输入便利，**不能设计成防封轮换层**。[一手来源：规范 §3.2.2、§3.2.4](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)
- 〔推断〕二维码作为电脑终端到手机的首跳是合理的；在手机微信里再展示二维码，通常还需要长按识别或另一台设备，不是可靠“第二条零门槛路”。

### 4.3 分享卡片

- 微信官方可控路径是先绑定服务号 JS 接口安全域名，完成 `wx.config` 签名，再调用 `updateAppMessageShareData` / `updateTimelineShareData`。安全域名最多 5 个父域，不支持 IP、端口、短链，且须通过 ICP 备案验证。[一手来源：微信 JS-SDK](https://developers.weixin.qq.com/doc/service/guide/h5/jssdk.html)
- **没查到微信官方对普通未备案网页的 `<title>`、首图或 `og:*` 抓取保证。** 搜索到的经验说法无法形成 2026 可承诺规则。
- 〔推断〕门禁页应服务端直接输出 `<title>`、描述、绝对地址图片、favicon/`apple-touch-icon` 与 Open Graph，供支持它们的抓取器尽力使用；微信结果必须标“尽力而为”。
- iMessage 官方明确使用 Open Graph；抓取器不执行 JavaScript、不跟随 meta refresh，但跟随服务端重定向；主页面上限 1 MB，关联资源合计建议不超过 10 MB，`og:image` 建议至少 900 px 宽。[一手来源：Apple TN3156，2024-04-30 重发](https://developer.apple.com/documentation/technotes/tn3156-create-rich-previews-for-messages)

### 4.4 下载

没查到 2026 微信内 `<a download>` 对所有文件类型、Android/iOS 的统一拦截规则。可确认的是微信规范把“文件下载类链接”列为风险链接例子；playtest.run 的玩家路径应流式加载作品资源，而不是让玩家下载 zip/apk。[一手来源：规范 §2.18.3](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)

## 5. 香港边缘：公开证据与证据缺口

### 5.1 香港边缘延迟证据表

| 云/线路 | 运营商 | 晚高峰延迟/丢包 | 来源 |
|---|---|---|---|
| 吉云香港 CUG/CMI 优化 | ITDOG 128 个中国节点；分电信/联通/移动 | 2026-06-08，**页面未给晚高峰时间戳**。全国平均：电信 39 ms、联通 34 ms、移动 56 ms、全部 45 ms；最慢节点分别 73/67/85 ms；**丢包没给出** | [二手测速原页及截图](https://vpsxb.net/8108/) |
| Neburst 香港 CTG GIA/9929/CMIN2 | 回程到北京/上海/广东三网 | 2026-04-11，**非晚高峰证据**。北京终点约电信 41.73、联通 47.16、移动 53.39 ms；上海约 26.94/34.19/39.31 ms；广东约 11.72/8.38/24.08 ms；**丢包没给出** | [二手测速原页](https://vpsxb.net/8037/) |
| Zenlayer 香港普通国际回程（TATA/Arelion） | ITDOG 270 个中国节点；分电信/联通/移动 | 2026-08-18，**页面未给晚高峰时间戳**。全国平均：电信 319 ms、联通 240 ms、移动 194 ms、全部 239 ms；最慢节点 376/318/261 ms；**丢包没给出** | [二手测速原页及截图](https://vpsxb.net/8190/) |
| 阿里云香港 | 电信/联通/移动家宽与移动网络 | 2025–2026 晚高峰：**没查到同时给测试时段、三网延迟与丢包的可复核公开数据** | 没查到 |
| 腾讯云香港 | 同上 | 2025–2026 晚高峰：**没查到合格新数据**；公开测评多为更早年份或营销口径 | 没查到 |
| AWS 香港 `ap-east-1` | 同上 | 2025–2026 晚高峰：**没查到合格数据** | 没查到 |
| Azure 香港 | 同上 | 2025–2026 晚高峰：**没查到合格数据** | 没查到 |
| Cloudflare 香港节点 | 同上 | 2025–2026 晚高峰：**没查到能固定命中香港 PoP 且有三网丢包的数据** | 没查到 |
| Bunny 香港 | 同上 | 2025–2026 晚高峰：**没查到合格数据** | 没查到 |
| Vultr 香港 | 同上 | 2025–2026 晚高峰：**没查到合格新数据** | 没查到 |
| DMIT 香港 | 同上 | 找到 2025 促销与旧测评，**没查到满足本表口径的新晚高峰数据** | 没查到 |
| CN2 GIA 香港 | 同上 | 上述 Neburst 是路线样本，但无晚高峰时间戳/丢包百分比；不能把“CN2 GIA”标签当 SLA | [二手测速原页](https://vpsxb.net/8037/) |
| Akamai 香港 | 同上 | 2025–2026 晚高峰：**没查到公开可复核数据** | 没查到 |

### 5.2 对“普通线路晚高峰 300–400 ms”的明确裁决

- **支持数值可达，不支持晚高峰归因，也不支持普遍化。** Zenlayer 香港普通国际回程样本中，电信全国平均 319 ms、最慢 376 ms，直接证明香港普通线路确实可能落进 300–400 ms；但该页没有测试时刻与连续时序，不能证明这是晚高峰拥堵，更不能代表所有普通线路。[二手测速原页](https://vpsxb.net/8190/)
- **因此不能用这条数据反驳 30–60 ms 优化线目标，却能反驳“香港天然就是 30–60 ms”。** 吉云优化线样本的三网平均 34–56 ms，与普通回程样本的 194–319 ms 并存，说明主变量是回程而不是物理距离。[二手测速：吉云](https://vpsxb.net/8108/)、[Zenlayer](https://vpsxb.net/8190/)
- Zenlayer 样本中的移动平均 194 ms、广东移动路由终点约 363 ms，能证明糟糕的国际选路；**不能单凭这一条归因为 CMI 晚高峰拥堵**。本轮没查到带运营商确认或连续时序的 2025–2026 CMI 拥堵证据。[二手测速原页](https://vpsxb.net/8190/)
- **没查到 2025–2026 大陆玩家对“香港服”延迟接受度的可复核原话样本**，因此“休闲/回合制够用、竞技射击不够”是合理工程推断，不是用户研究结论。
- **没查到口径一致、可比较 2025 与 2026 的大陆国际出口带宽公开数据**，不能写“国际出口已经改善/恶化”。
- 隧道“约多 100–150 ms”还取决于玩家、开发者与香港边缘三者位置、双向路由和排队；公开样本不足以验证。应以端到端 WebSocket RTT/p95 而不是 ping 单点推算。

### 5.3 一周压测应记录

现有 DESIGN 的“20:00–23:00 压一周再定”方向正确。〔推断〕验收表至少应包含：

- 电信、联通、移动各不少于两个大陆城市；家宽与 4G/5G 分开；
- 每 1 分钟记录 ICMP/TCP/HTTPS p50、p95、p99、丢包、抖动、首字节、30/100 MB 持续下载；
- 上传与隧道分开；隧道记录玩家→边缘→开发者的实际 WebSocket RTT；
- 保存 traceroute/MTR，识别是否绕日本、新加坡、美欧以及高峰时路由切换；
- 将“全国 p95 < 80 ms、丢包 < 1%”之类写成待实测 SLO，而不是预先写死供应商。

## 6. 其它 IM 与系统浏览器

| 客户端 | 2026 可确认的行为 | 门禁页含义 | 来源 |
|---|---|---|---|
| QQ | 腾讯 TBS/X5 面向包括 QQ 在内的宿主，但**没查到 2025–2026 QQ 任意外链预览和实际内核版本的官方规则** | 用纯 HTML、服务端元数据、无 UA 特供；真机测 | [一手邻近证据：X5 产品资料](https://st.tencent-cloud.com/jax-static/tbs/aboutX5073101.md) |
| 钉钉 | 没查到普通聊天外链的当前抓取规则与内置浏览器版本映射 | 未验证，不承诺预览图 | [钉钉开放平台](https://open.dingtalk.com/) |
| 飞书 | 没查到普通外链默认预览的稳定公开规则 | 未验证；服务端 OG 作为渐进增强 | [飞书开放平台](https://open.feishu.cn/) |
| 企业微信 | 官方有企业应用网页授权，但这不等于普通聊天外链抓取规则 | 不要求玩家授权；按微信类似风险做真机测试 | [企业微信开发者中心](https://developer.work.weixin.qq.com/document/path/91022) |
| Discord | 本轮官方支持页受反自动化页阻挡，**没取得可引用的 2026 抓取规范** | 服务端 OG、绝对图片 URL、快速返回；实际卡片待测 | 没查到可复核官方正文 |
| Telegram | 2024-07-31 官方上线支持多标签的内置浏览器，可折叠后回聊天，再返回网页且不丢进度 | 页面要正确处理 `visibilitychange`、后台恢复和音频暂停 | [一手：Telegram Browser](https://telegram.org/blog/w3-browser-mini-app-store) |
| iMessage | 默认显示页面标题、域名、图标；Open Graph 可提供图片与标题；抓取器不运行 JS | 门禁页元数据必须首个响应直接给出 | [一手：Apple TN3156](https://developer.apple.com/documentation/technotes/tn3156-create-rich-previews-for-messages) |
| X | 当前文档站迁移，本轮没找到可复核的最新 Cards 规则正文 | 仍输出 OG/Twitter 元数据，但标为待验证 | [X 开发者文档入口](https://developer.x.com/) |

### 6.1 iOS Safari 与 Android Chrome

- **安装/PWA**：iOS/iPadOS 26 的 Safari 可将任何网站加入主屏并按 Web App 打开，不再要求 manifest 才有“可安装性”；Chrome 的浏览器安装提示仍要求 HTTPS、manifest、图标、展示模式和用户参与条件。[一手：Safari 26](https://webkit.org/blog/17333/webkit-features-in-safari-26-0/)、[Google：安装条件](https://web.dev/articles/install-criteria)
- **全屏**：Android Chrome 支持 Fullscreen；MDN BCD 当前记录 iOS Safari 只在 iPad 部分支持且会显示不可关闭的覆盖按钮，iPhone 不支持，iOS WebView 也不支持。这对横屏游戏是实质差异。[二手兼容数据](https://github.com/mdn/browser-compat-data/blob/main/api/Element.json)
- **音频**：两边都不应假设有声自动播放。Chrome 允许静音自动播放，有声播放通常要求用户互动；WebKit 对有声媒体也要求直接用户手势。[一手：Chrome autoplay](https://developer.chrome.com/blog/autoplay/)、[一手：WebKit iOS 媒体策略](https://webkit.org/blog/6784/new-video-policies-for-ios/)

## 7. GitHub 在大陆：登录不能只看“首页偶尔能开”

### 7.1 开放测量

- GreatFire 页面在 2026-09-06 显示：`https://github.com` 最近 55 次有效测试中 **67% disrupted**，并称自 2024-01-26 起记录到干扰；同一根域不同路径也呈 mixed。GreatFire 是第三方测量，不是官方封锁公告。[开放测量](https://en.greatfire.org/https/github.com)
- OONI 精确输入 `https://github.com/` 的最近样本：
  - 2025 上半年查询返回的最近 100 条：约 50 clear、27 anomaly、23 failure；
  - 2026 上半年查询返回的最近 100 条：约 63 clear、33 anomaly、4 failure；
  - 2026-07-01 至 09-07 全部 76 条：28 clear、41 anomaly、7 failure。
  这些样本覆盖 AS4134、AS4837、AS9808、AS24400 等，显示同运营商、同时段也可能不同。[开放测量 API：2025](https://api.ooni.io/api/v1/measurements?probe_cc=CN&input=https%3A%2F%2Fgithub.com%2F&since=2025-01-01T00%3A00%3A00&until=2025-06-29T00%3A00%3A00&limit=100)、[2026 上半年](https://api.ooni.io/api/v1/measurements?probe_cc=CN&input=https%3A%2F%2Fgithub.com%2F&since=2026-01-01T00%3A00%3A00&until=2026-06-29T00%3A00%3A00&limit=100)、[2026 最新](https://api.ooni.io/api/v1/measurements?probe_cc=CN&input=https%3A%2F%2Fgithub.com%2F&since=2026-07-01T00%3A00%3A00&until=2026-09-07T23%3A59%3A59&limit=100)
- OONI 的 anomaly 是控制测量不一致，不应全部解释为 GFW；failure 也可能是本地网络故障。[方法说明](https://ooni.org/nettest/web-connectivity/)

### 7.2 没查到的部分

- OONI 对精确 `https://github.com/login` 的 2026 上半年查询没有结果。
- **没查到 2025–2026 有分母的中国大陆 GitHub OAuth 授权→回调成功率，也没查到 playtest.sh 实际回调链路测试。**
- 因此不能从首页 clear 推导 OAuth 成功，也不能从 anomaly 推导 OAuth 必败。产品结论应是“间歇可用、不可作为首次发布唯一门槛”。

## 8. 对 DESIGN §5 的逐条核对

| DESIGN §5 判断 | 核对 | 证据与修正 |
|---|---|---|
| 微信是主渠道、最大不可控 | **风险判断成立，而且 DESIGN 低估了政策风险；“主渠道”本身尚无用户数据** | 官方 §2.5 直接列 H5 游戏/网页小游戏为常见违规，不只是用户上传了钓鱼内容后才出问题。[一手：微信规范](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification) |
| 微信按根域封禁被举报网页，一个用户会让整域迟早被封 | **方向成立，机制表述过度简化、概率判断过于悲观** | 可处理链接、域名或 IP；全域明文条件是同域大量违规且处理后未有效整改。没找到“一例自动根域连坐”的 2024–2026 统计。[一手：微信规范 §3.1](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification) |
| 域名分离 + 门禁页/举报/快速下架 + 24 小时申诉 | **大部分成立，但不是充分对策** | 域名分离保护品牌/登录安全；快速下架符合整改要求；24 小时只能承诺“提交”，没查到恢复 SLA。门禁页不能消除 §2.5，若只强制外跳还可能触及 §2.3.3。Public Suffix List 不保证微信按 slug 隔离执法。[一手：微信规范](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)、[一手：腾讯客服](https://kf.qq.com/faq/170118UnqeUZ170118mUb6fu.html) |
| 微信 Android X5 对 SAB/WebGPU 不可靠 | **结论成立，内核称谓与处理方式需修正** | 微信 Android 的 XWeb 版本映射没查到；商业 X5 121 不是微信证明。WebGPU 还受 Android/GPU 限制，SAB 需实际隔离成功。应能力检测，不应见微信就劝退。[一手：Chrome WebGPU](https://developer.chrome.com/blog/webgpu-release)、[一手：Safari 15.2](https://webkit.org/blog/12140/new-webkit-features-in-safari-15-2/) |
| 整个域名被墙是存在性风险 | **成立，但本轮没找到 playtest 类用户内容域的 2025–2026 概率数据** | GitHub 测量证明境外技术域名可呈持续、路径/运营商混合干扰；不能据此量化 playtest.run 被墙概率。[开放测量：GreatFire](https://en.greatfire.org/https/github.com)、[OONI](https://api.ooni.io/api/v1/measurements?probe_cc=CN&input=https%3A%2F%2Fgithub.com%2F&since=2026-07-01T00%3A00%3A00&until=2026-09-07T23%3A59%3A59&limit=100) |
| 不落大陆、不备案 | **工程上自洽，但“唯一现实路径”过度绝对，微信体验代价被低估** | 不备案避免大陆服务器合规路线；同时未备案域名朋友圈分享有频次限制、微信 JS-SDK 安全域名无法配置，且 §2.18.4 将未备案/难追溯列为风险。[一手：腾讯客服](https://kf.qq.com/faq/170118UnqeUZ170118mUb6fu.html)、[一手：微信 JS-SDK](https://developers.weixin.qq.com/doc/service/guide/h5/jssdk.html) |
| GitHub 在大陆基本可用且开发者都有 | **过于乐观，且“都有”没证据** | GreatFire 与 OONI 都显示明显间歇性；OAuth 成功率没查到。匿名首次发布不能被 GitHub 登录卡住。[开放测量：GreatFire](https://en.greatfire.org/https/github.com)、[OONI](https://api.ooni.io/api/v1/measurements?probe_cc=CN&input=https%3A%2F%2Fgithub.com%2F&since=2026-07-01T00%3A00%3A00&until=2026-09-07T23%3A59%3A59&limit=100) |

## 9. 对 playtest.run 的含义

### 9.1 支持 DESIGN 的发现

- **品牌域与内容域分离应保留。** 〔推断〕它同时缩小用户 JS 的 cookie/同站安全边界和微信处置爆炸半径；理由不应写成“PSL 能防微信根域封禁”。
- **门禁页不引第三方脚本、快速出现、服务端直出元数据是正确方向。** 〔推断〕这对 iMessage、Telegram/Discord 类抓取器、弱网和滥用处置都有价值。
- **香港供应商必须晚高峰压测后选。** 公开样本显示线路差异远大于机房地理差异。[二手测速：吉云](https://vpsxb.net/8108/)、[Zenlayer](https://vpsxb.net/8190/)
- **微信能力不可靠、需要无线程导出建议的方向成立。** 但降级应由能力检测触发。[一手：SharedArrayBuffer 安全要求](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer#security_requirements)
- **举报、快速下架和申诉预案必要。** 官方全域处置条件明确关注大量违规与整改是否及时有效。[一手：微信规范](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)

### 9.2 与 DESIGN 冲突或过于乐观

1. **核心承诺冲突**：正式规则明列 H5 游戏/网页小游戏为常见违规。发布前必须把“微信里点开就能玩”降为待验证假设，取得腾讯口径、法律/合规评估和真实账号传播实验后再决定能否公开承诺。[一手：微信规范](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)
2. **分享卡片冲突**：不备案就不能走官方 JS-SDK 安全域名路径。将“微信抓标题和封面”改为“服务端提供元数据，微信展示尽力而为”；Discord/iMessage 分开验证。[一手：微信 JS-SDK](https://developers.weixin.qq.com/doc/service/guide/h5/jssdk.html)
3. **网络数字过度确定**：把“大陆到香港 30–60 ms”改成“优质回程目标”；“隧道多 100–150 ms”改成待端到端一周实测的预算，不要作为事实。[二手测速：吉云](https://vpsxb.net/8108/)、[Zenlayer](https://vpsxb.net/8190/)
4. **GitHub 登录过于乐观**：不能成为首次匿名发布或故障恢复的唯一门槛。至少让匿名 24 小时链接完整覆盖第一次体验，并在 GitHub 授权失败时清楚说明、可重试。[开放测量：GreatFire](https://en.greatfire.org/https/github.com)、[OONI](https://api.ooni.io/api/v1/measurements?probe_cc=CN&input=https%3A%2F%2Fgithub.com%2F&since=2026-07-01T00%3A00%3A00&until=2026-09-07T23%3A59%3A59&limit=100)
5. **未备案代价被低估**：除无法在大陆落服务器外，它还影响朋友圈传播频率、官方微信分享能力，并被规则列为风险因素。[一手：腾讯客服](https://kf.qq.com/faq/170118UnqeUZ170118mUb6fu.html)、[一手：微信规范](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)

### 9.3 与 DESIGN 冲突或过于悲观

1. **`--isolated` 见微信就劝退过于悲观。** iOS 新 WebKit 已支持 COOP/COEP、SAB 条件与 WebGPU；Android 新内核也可能具备。应先运行时检测，失败才提示。[一手：Safari 15.2](https://webkit.org/blog/12140/new-webkit-features-in-safari-15-2/)、[一手：Safari 26](https://webkit.org/blog/17333/webkit-features-in-safari-26-0/)
2. **“一个违规页面自动封整个注册根域”过于悲观。** 官方写的是同域大量违规且未有效整改；但大型用户平台仍可能被升级处理，不能因此放松治理。[一手：微信规范 §3.1.5](https://weixin.qq.com/agreement/weixin_external_links_content_management_specification)

### 9.4 建议修改方向（不直接改 DESIGN）

1. 在 §0、§5 和 v0.1 完成标准前加一个**发布阻断条件**：用真实微信账号、Android 至少 6 台/iOS 至少 4 个系统版本，分别测试私聊、群聊、朋友圈的门禁页与实际游戏；并向微信/腾讯客服书面确认 §2.5 对非营销、私人 playtest 链接的适用方式。
2. 把产品承诺改成分层事实：系统浏览器“可玩”；微信“优先直接玩，能力或规则阻断时明确说明”。在确认 §2.5 前，不把微信可玩写入公开首页承诺。
3. 保留稳定内容根域和品牌域，不做域名轮换、短链套娃或“防封跳板”；记录投诉→下架→申诉证据链和处理时间。
4. 门禁页只做一次明确“开始”手势；服务端直出元数据；运行时检测 WebGL2、WebGPU、`crossOriginIsolated`、SAB、音频、摄像头与传感器，并把失败原因回传结果层。
5. 香港选型按 §5.3 的矩阵压一周，再把实测 p50/p95/丢包写入 DESIGN；供应商名和 30–60 ms 数字在此之前保持未定。
6. GitHub OAuth 做成可失败的开发者增强路径，而不是第一次把链接发给玩家的前置条件；对登录页、授权页、回调域逐段做大陆探针。

## 10. 仍未查到或未验证

- 2024–2026 微信 Android XWeb 与 Chromium 的官方逐版本映射。
- 微信 Android/iOS 对 COOP/COEP、SAB/Wasm 线程、WebGPU、Service Worker、30–100 MB 缓存、Canvas FPS 的完整真机矩阵。
- “在浏览器中打开”遮罩的真实转化率。
- 普通未备案网页在 2026 微信分享中对 `<title>`、首图、Open Graph 的稳定抓取规则。
- 题目所列各用户内容平台在 2026-09-07 的微信真机可达性及整域封禁率。
- 申诉成功率与 24 小时恢复概率。
- 主流香港云 2025–2026 三网晚高峰、含丢包且口径一致的数据；CMI 拥堵的连续时序证据。
- 大陆玩家对香港服延迟接受度的近期原话样本，以及 2025–2026 国际出口带宽可比变化。
- QQ、钉钉、飞书、企业微信、Discord、X 的最新任意外链抓取规则和真机矩阵。
- GitHub OAuth 在大陆从授权到回调的成功率。
