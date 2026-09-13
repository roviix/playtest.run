# 2026-09-14 单主域 playtest.run 收敛、统一账号体系与设备流上线验证

## 背景与目标

根据建造者规约第 7 条（「一个产品，一个主域」）及用户要求，彻底收敛产品架构与账号体验：
1. **彻底弃用旧域名**：全站统一收敛至 `playtest.run` 单主域，移除 `playtest.roviix.com`。
2. **路由与控制面统一**：控制台挂载在 `https://playtest.run/console/`，控制面 API 挂载在 `https://playtest.run/v1/*`，作品仍由独立的 `<slug>.playtest.run` 子域安全交付。
3. **账号体系与登录收敛**：
   - 建立单主域平台会话与统一账号系统（`008_accounts.sql`、`api/src/account.rs`、`api/src/device.rs`）。
   - 网页端使用 host-only `pt_session` / `__Host-pt_session` HttpOnly Cookie。
   - 广场与全站挂载统一登录弹窗（`ui/account.html` + `ui/account.js`），支持邮箱一键登录与 GitHub OAuth。
   - 命令行 `playtest login` 接入自建设备码授权流（Device Authorization Flow），在 `https://playtest.run/console/#/device` 完成授权，无缝收敛认领匿名上传的作品。
4. **历史数据置零与生产发布**：不保留历史兼容负担，数据置零，修复全量自动化测试，完成香港服务器真机生产部署与公网在线验收。

## 改动与实现

- **单主域网关配置与清理**：
  - 删除 `deploy/sites/api.caddy`，所有请求统一由 `deploy/sites/content.caddy` 接管。
  - `/console/*` 静态资源由 Caddy 直接提供，`/v1/*` 代理给 API 容器，其他路由由 Edge 容器处理。
  - Caddy 与 API 精确校验 `playtest.run` Origin，防范跨子域 Cookie 污染与伪造。
- **账号体系与设备授权**：
  - 新增 `008_accounts.sql`，建立 `browser_sessions` 与 `device_authorizations` 表。
  - `api/src/account.rs` 提供 `/v1/account` 状态查询、邮箱发信、会话保护与登出逻辑。
  - `api/src/device.rs` 提供设备码发放、网页授权核验与轮询绑定机制。
  - 控制台新增 `/device` 页面（`console/src/pages/device.tsx`），支持直接输入代码或扫码授权。
- **边缘与社交回访适配**：
  - `edge/src/plaza.rs` 侧栏导航整合「我的作品」「我的合集」「使用文档」「登录」，外壳挂载 `ui/account.html` 与 `ui/account.js`。
  - `edge/src/identity.rs` 支持优先读取 `__Host-pt_session` / `pt_session`，兼顾 `pt_me` 兼容。
  - 邮箱关注确认与退订流程更新，确保 Host-only Cookie 正确种下与注销。
- **测试修复与契约更新**：
  - 修复 `follow_boost.rs` 设备流与头像字段测试。
  - 修复 `upload_flow.rs` 匿名凭据提示。
  - 运行 `playtest-contract` 重新生成 `console/src/generated/api.ts`。
  - 适配 `edge/tests/serving.rs`、`edge/tests/social.rs`、`edge/tests/tunnel.rs` 中涉及单主域与账号弹窗的断言。
  - 全工作区 `cargo test --all`（195 + 113 + 70 项测试）全部通过。

## 生产环境验证（playtest-hk 真机）

部署通过 `deploy/push.sh` 执行完毕，服务器容器编排正常（TAG=20260913-233321），线上公网实测记录：

1. **服务健康检查**：
   - `curl -ILsS https://playtest.run/healthz` → `HTTP/2 200`（API 容器直通）
   - `curl -ILsS https://playtest.run/_playtest/healthz` → `HTTP/2 200`（Edge 容器）
2. **控制台静态托管**：
   - `curl -ILsS https://playtest.run/console/` → `HTTP/2 200`（Caddy 静态卷托管，资源加载正常）
3. **广场与账号状态**：
   - `curl -ILsS https://playtest.run/` → `HTTP/2 200`（HTML 包含 `#account-login` 统一弹窗与 `account.js` 逻辑）
   - `curl -sS https://playtest.run/v1/account` → `{"account":null,"email_available":true,"github_available":true}`
4. **GitHub OAuth 与设备码流**：
   - `curl -ILsS https://playtest.run/v1/login/github/start` → `302` 重定向至 GitHub 官网授权页，返回 `200`。
   - `curl -sS -X POST https://playtest.run/v1/login/device -H 'Content-Type: application/json' -d '{}'` → 成功生成 `device_code`、`user_code`，验证 URI 指向 `https://playtest.run/console/#/device`。
   - `curl -sS -X POST https://playtest.run/v1/login/device/poll -d '{"device_code":"..."}'` → 正常返回 `{"status":"pending","interval":5}`。
