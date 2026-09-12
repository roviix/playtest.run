# 2026-09-12 · 公开发布前的安全、滥用与规模审查

## 范围与证据等级

回答「准备公开发布，现在的防护够不够；子域名能不能有几万甚至更多」。本次检查的是本机当前工作区，包含已有未提交改动；未修改业务代码、生产配置或 DESIGN，未对生产发送攻击、批量创建、压测或邮件。下文是风险审查与待采纳建议，不替代 `docs/DESIGN.md` 的产品定义。

证据分为源码路径、官方文档核对、尝试执行的本机测试。没有本次生产配置审计、恶意请求端到端复现、真手机测试或容量验收。当前工作区存在编译错误，不能把本次记录称为「防护已验证」。

## 结论

**不建议把当前状态直接作为面向任意陌生发布者、匿名无限自助的正式版本。** 有基础防线，但玩家身份隔离、匿名上传总量、可信事件写入和成本上限仍有缺口。几万子域名不是 DNS 架构的主要难点，也不构成当前服务器能承受几万活跃作品的证据。

不需要先建设大型风控系统或分布式平台。建议先完成本记录的发布阻断项，再小范围开放、记录真实使用与恶意流量下的失败形态。建设应服务 DESIGN §1.2 的 M1（顺利拿到链接）、M2（链接可靠）、M3（结果可信），不把登录、安装、验证码或额外跳转加到普通试玩者身上。

## 1. 子域名：DNS 数量、随机名字和运行容量是三件事

### DNS 与证书

`deploy/sites/content.caddy:3` 已使用 `playtest.run, *.playtest.run` 与 DNS-01；作品由边缘按 Host 路由，不是每新增作品创建一条 DNS 记录或一个 Caddy 站点块。

- Cloudflare 官方文档允许一条 wildcard 记录映射许多子域名。因此，从 DNS 记录配置方式推断，几万、几十万个一级作品名不需要同样数量的记录。[S1]
- 本次核对的 Cloudflare 配额按记录计：2024-09-01 起新建的 Free zone 默认 200 条；更早的 Free zone 为 1,000 条；Pro / Business 为 3,500 条。这不是 wildcard 下作品数量的上限。没有读取本账号套餐或实际 quota，不推断它属于哪档。[S2]
- 当前 Caddy 的 wildcard 地址匹配一个标签，`a.playtest.run` 与 `a.b.playtest.run` 不是同一种覆盖；根域也要另外包含。不要把 DNS wildcard 可以向更深层匹配与 TLS / Caddy 的匹配规则混淆。[S1][S3]
- 不要改成按 slug 单独签证书。Let's Encrypt 的默认新证书限额包含同一注册域每 7 天最多 50 张，续期与其他限额有各自规则。共享 wildcard 不需要每新增作品消耗一次签发。[S4]

### 随机名字是当前真正有限的部分

`common/src/slug.rs:124` 与 `common/src/slug.rs:134` 的词表，本机计数分别为 64 和 64；`api/src/routes/sites.rs:311` 再取 10–99 的两位数：

```text
64 × 64 × 90 = 368,640 个自动生成的候选名
```

生成器最多重抽 20 次。`api/src/db.rs:413` 的 `slug_taken` 连已删除的作品也算占用，因此消耗的是累计分配量，不只是当前活跃量。这个数字只限制自动命名组合池，不是所有合法自定义 slug 的总量。几万个作品并不会马上耗尽，但不能把它作为长期大规模公开分配方案。

建议保留短、易读的前缀，扩大随机后缀空间，继续依靠数据库唯一约束和冲突重试；不要通过把旧作品地址重新分给陌生人来节省名字。随机地址也不是私密访问权限的替代品。采纳命名变化前先更新 DESIGN。

### 运行容量没有被本次证明

当前 `FsStore` 是本地文件系统，API / edge 共用数据目录；SQLite 已有 WAL 和读连接池，边缘缓存也已有容量控制，不应笼统说「单机或 SQLite 所以不能有几万作品」。但广场重建遍历候选作品并查询举报数，页面渲染遍历作品，blob GC 扫描清单和 blob，发布提交有全局串行锁。真正要测的是活跃请求、上传并发、文件数、热缓存、磁盘、事件写入和后台任务，而不是能解析多少个名字。

## 2. 公开发布前的阻断项

### P0-A：平台玩家身份被带进不可信隧道

源码路径：

- `edge/src/app.rs:513` 把 `pt_me` 写为 `Domain=.playtest.run` 的共享 Cookie。
- `edge/src/tunnel/proxy.rs:225` 转发除 Host 和逐跳头外的请求头，未剥离平台 Cookie；现有单元测试还明确检查 Cookie 被保留。
- 普通响应和 WebSocket 升级响应没有针对平台 Cookie / 父域 `Set-Cookie` 的隔离规则。
- `api/src/routes/follow.rs:240` 等接口把 `me_token` 当作读取关注页、取消关注等操作的凭据。

**源码推断：已激活玩家访问恶意开发者的隧道时，共享身份 Cookie 会被带给对方的本地服务器。** `HttpOnly` 阻止的是页面 JavaScript 直接读 Cookie，不阻止浏览器在 HTTP 请求里发送，也不阻止收到请求的服务器读取。[S5] 此风险不能用「我们不运行用户代码」排除。

建议优先让全站身份只属于可信根域（例如 host-only、`__Host-` Cookie），作品运行域只拿当前作品所需的最小凭据；同时处理现有 Cookie 的迁移和撤销。即使短期保留共享身份，也必须在隧道请求中剥离平台凭据，并拦截上游试图设置平台保留 Cookie 或跨作品父域 Cookie 的响应。普通游戏自己的、仅当前主机有效的 Cookie 不应一刀切删除。HTTP 与 WebSocket 握手都要覆盖。

根域的关注 / 取消关注入口还应加入精确 Origin 校验与 CSRF 防护，不能仅依赖 SameSite。是否加入 Public Suffix List 可以后续评估，但它会改变 Cookie 继承边界，不能在保留全域共享 Cookie 假设的同时当作一项无影响的 DNS 优化。[S5][S6]

### P0-B：匿名身份和上传缺少平台总量约束

`api/src/routes/sessions.rs:16` 每次请求直接创建匿名用户和令牌，没有看到来源 / 全局创建限流。当前每匿名身份 3 个作品、登录身份 10 个作品是 `api/src/routes/sites.rs:22` 的私测值，不等于公开计划表里的限制；不断创建匿名身份可以绕开「每身份」限制。

更直接的问题在 `api/src/routes/blobs.rs:22`：只要有有效 Caller 和正确哈希，就能接收一个 blob，没有要求它属于已批准的上传清单、没有按上传会话预留字节，也没有账号累计存储上限。单文件 200 MiB 的流式限制确实存在，但不是总磁盘保护。`api/src/sweeper.rs` 的孤儿回收有 24 小时保护期并每天运行一次，不能代替写入准入。

建议建设：匿名创建按可信来源及全平台限速；上传会话绑定用户 / 作品 / 哈希 / 大小 / 到期时间；写入前预留账户和平台额度；上传并发、总时长及无进展超时；磁盘水位停止接收新上传；回收未提交 blob 与临时文件。限制不能只有 IP 一维，避免让一个学校或公司出口下的正常开发者互相挤掉。

### P0-C：流量熔断尚未覆盖整个成本面

已有静态字节熔断不能低报为「什么都没有」：`edge/src/breaker.rs` 按作品滚动一小时计数，匿名 512 MiB、其他作品 3 GiB。它在进程内，重启归零，多节点各算各的。

但是：

- `common/src/plan.rs` 的月流量和 `QuotaState` 没有在本次检查的服务执行路径中形成月累计、重置、超限拒绝的闭环。
- `edge/src/app.rs:643` 的整作品隧道直接转发；`edge/src/tunnel/proxy.rs` 的 HTTP / WebSocket 字节只更新会话统计，未接入静态流量 breaker。混合后端也需要单独覆盖。
- 静态 breaker 的检查与记账分开，不应把上限宣传为并发下精确到字节的硬封顶。
- 按作品限字节不等于限制整个平台的请求数、TLS 连接、CPU、文件 I/O 或所有作品相加的流量。

建议建设：平台 / 账号 / 作品分层预算，秒或分钟级请求与并发限制，上传与隧道全路径执行；明确允许的并发超额和故障策略；短窗口自动暂停新建、上传、隧道或高成本功能，保留已发布静态作品的可用性。告警是提醒，自动止损才是执行；应用停止响应也不保证上游云厂商不再产生任何费用。[S7]

### P0-D：边缘事件入口并未认证边缘身份

`api/src/routes/mod.rs:89` 把 `ingest_paths::EDGE` 注册在公开路由；`api/src/routes/events.rs:161` 无边缘令牌或签名验证，缺少 Origin 的请求也允许进入。提交的数据可以包含 `report`、`start` 等种类；`api/src/db.rs:861` 按不同 session ID 统计举报，`api/src/plaza.rs` 在达到阈值时自动撤下广场作品。

源码推断：外部调用者可伪造这些事件，不只是污染自己的 SDK 统计，还可能把其他作品从广场移除。本次没有发送真实举报或完成本机端到端复现。

建议把边缘上报与玩家 SDK 上报分开认证；边缘入口仅可信内网并带专用令牌 / 签名，公网反代明确拒绝直接访问。玩家零登录不等于允许客户端制造「可信边缘事件」。举报应有可信来源和抗重复机制；三个任意 session ID 不能当作三个独立真人。

### P0-E：滥用处置必须能停止内容分发

`api/src/routes/admin.rs:185` 的 hide 是「从广场隐藏」，不是封禁作品。它更新 listing 并结束推广，没有删除 / 禁用字节入口或主动关闭隧道。不能把它称为「恶意站点下架完成」。

建议完成举报 → 人工处理 → 按作品 / 账号暂停分发 → 关闭活动隧道、拒绝重连 → 留下操作记录与申诉入口。必须覆盖直接链接、旧版本资源、封面与缓存；如果以后引入 CDN，还要处理缓存失效。恶意内容托管和大量小请求获取大文件本身就是风险，即便服务器没有执行上传者代码。[S8]

## 3. 上游防护、邮件和运营最低配置

- DESIGN §4.7 明确 Cloudflare 只做 DNS、不做代理。按该配置，Cloudflare 不在 HTTP 请求路径上，不能据「DNS 放在 Cloudflare」宣称源站已获得它的 HTTP WAF / DDoS 防护。[S9] 本次未核对生产的真实 proxy 状态或云厂商已有防护。
- 在上游抗 DDoS / WAF / 高防线路中选经过目标网络测试的方案；不要为了防护直接牺牲大陆访问和长连接体验。若启用 Cloudflare 代理，源站访问限制和可信代理头必须一起设计，不能留下可绕过的源站入口。
- 不能直接把开发者 API 打开代理就算完成：官方 Free / Pro 的单次上传上限为 100 MB，仓库当前允许 200 MiB 单文件并按整个 blob 上传，存在冲突。需要分块协议或相容的受保护上传入口，并回归 Range、WSS、预压缩资源和弱网行为。[S10]
- 邮件应有全局 / 收件人限额、投诉和退信停发、紧急停发按钮。现有邮箱双重确认与按邮箱桶值得保留。`api/src/routes/follow.rs:447` 读取 `X-Forwarded-For`，但 `edge/src/upstream.rs` 的 JSON 请求没有转交这个字段，因此正常边缘调用会落入共享 `unknown` 桶；修复时要明确信任链，不能照单全信外部请求头。本次不认定生产可通过伪造此头绕过 Caddy。
- 明确监控 CPU、内存、磁盘、水位增长、出网速率、错误率、匿名创建量、上传量、通知队列、证书到期和备份。限制正常攻击路径，而不只是增加机器。异机备份与恢复演练应在上线前留有证据，不能把同盘备份等同于灾备。

## 4. 验收建议，不是已完成记录

1. 用仅本机的恶意隧道夹具检查平台 Cookie 永不传出、父域 Cookie 永不写入；重复 Cookie、普通 HTTP 和 WSS 都测。正常游戏自己的 Cookie 保持工作。
2. 在隔离数据目录测试重复匿名建号、未批准 blob、并发上传、断传、磁盘水位，确认拒绝发生在额度透支前；不拿生产磁盘做填满实验。
3. 验证未认证的边缘事件被拒绝，真实 edge 仍可上报；恶意伪造的三个会话不能让正常作品被撤下。
4. 静态 / Range / 隧道 / WebSocket 同时跑预算边界，确认重启、多进程、断流和控制面故障时的规则；记录超额容忍度。
5. 暂停一个作品后，直接链接和活动隧道均失效；其他作品继续工作。平台停止新建 / 上传 / 发信的开关分别演练。
6. 用 1 万 / 5 万条作品元数据测查找、广场、缓存和后台清理；另外测明确的活跃作品数、QPS、上传并发、大文件吞吐和 WebSocket 连接数。记录机器配置、数据分布、p95、内存和磁盘增长，不能只写「支持 5 万子域名」。

## 5. 本机测试尝试

尝试执行下列现有测试，没有修改测试或顺手修复其他工作：

```sh
cargo test -p playtest-edge --lib the_host_is_rewritten_and_hop_by_hop_headers_are_dropped -- --exact tunnel::proxy::tests::the_host_is_rewritten_and_hop_by_hop_headers_are_dropped
cargo test -p playtest-api --test events_feedback a_bare_request_still_counts -- --exact
cargo test -p playtest-api --test upload_flow orphaned_blobs_are_collected_but_never_live_ones -- --exact
```

三条命令均以 101 结束，测试未进入执行：

- edge：`edge/src/plaza.rs:415` 的测试初始化缺少 `collections`；`edge/src/follow.rs:539` 的 match 未覆盖 `FollowTarget::Collection`。
- api：`api/src/routes/collections.rs:147` 将 SQL 结果读为未实现 `FromSql` 的 `usize`。

因此，本记录的漏洞路径是静态审查结论，绝非通过上述测试得出的生产利用证明；以后修复应另增带日期的验收记录，不修改本记录。

## 官方来源（2026-09-12 核对）

- [S1] Cloudflare Wildcard DNS records：`https://developers.cloudflare.com/dns/manage-dns-records/reference/wildcard-dns-records/`
- [S2] Cloudflare DNS records / quotas：`https://developers.cloudflare.com/dns/manage-dns-records/`
- [S3] Caddyfile Concepts / Addresses：`https://caddyserver.com/docs/caddyfile/concepts`
- [S4] Let's Encrypt Rate Limits：`https://letsencrypt.org/docs/rate-limits/`
- [S5] MDN Set-Cookie：`https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Set-Cookie`
- [S6] Public Suffix List / Learn：`https://publicsuffix.org/learn/`
- [S7] OWASP Denial of Service Cheat Sheet：`https://cheatsheetseries.owasp.org/cheatsheets/Denial_of_Service_Cheat_Sheet.html`
- [S8] OWASP File Upload Cheat Sheet：`https://cheatsheetseries.owasp.org/cheatsheets/File_Upload_Cheat_Sheet.html`
- [S9] Cloudflare Proxy status：`https://developers.cloudflare.com/dns/proxy-status/`
- [S10] Cloudflare Error 413：`https://developers.cloudflare.com/support/troubleshooting/http-status-codes/4xx-client-error/error-413/`
