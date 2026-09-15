# 安全披露 · Security Policy

`playtest` 是一个会从你的电脑向外发布文件、或把本地端口接到公网的程序；边缘与控制面托管别人的作品。我们把安全问题当成产品缺陷处理，不当成公关问题。

## 报告渠道 · How to report

- 首选：GitHub **Security → Report a vulnerability**（私密，仅维护者可见）。
- 请不要在公开 issue、讨论区或社交平台披露未修复的问题。

Preferred: open a private report via GitHub *Security → Report a vulnerability* on this repository. Please do not file public issues for unfixed vulnerabilities.

## 我们承诺 · What you can expect

- 3 个工作日内确认收到；修复或缓解后在 Release 说明中致谢（除非你不希望）。
- 不对善意研究者采取法律行动，只要你不访问、修改或删除他人数据，不做拒绝服务测试。

Acknowledgement within 3 business days; credit in release notes unless you prefer otherwise. No legal action against good-faith research that avoids other users' data and denial-of-service testing.

## 范围 · Scope

- `cli/`：`playtest` 命令行与 MCP server
- `edge/`：`*.playtest.run` 作品分发、隧道、邀请卡、广场
- `api/`：`playtest.run` 控制面、登录、上传、通知
- `sdk/`：作品内可选的 `playtest.js`
- 线上服务 `playtest.run` 与 `*.playtest.run`

我们尤其关心：作品子域拿到平台会话或父域 Cookie；根域执行了作者脚本；绕过每作品每小时熔断或月配额；未授权读取、覆盖或删除他人作品与版本；隧道令牌被冒用；邮件确认与退订令牌可被猜测。

Especially relevant: platform credentials reaching a project subdomain, creator scripts executing on the root domain, quota/breaker bypass, unauthorized read/write of another creator's projects, tunnel token misuse, guessable email tokens.

## 不在范围 · Out of scope

- 作者自己上传的作品内容本身的漏洞（请联系作者；平台不运行也不修改作者代码）
- 需要物理接触设备或已被攻破的开发者机器
- 缺少某个安全响应头但没有可利用后果的报告
- 第三方服务（GitHub、Cloudflare、邮件服务商）自身的问题

## 支持的版本 · Supported versions

只有最新的 Release 与 `main` 分支会收到安全修复。旧版 CLI 连不上当前控制面时会明确报错，不会静默降级。
