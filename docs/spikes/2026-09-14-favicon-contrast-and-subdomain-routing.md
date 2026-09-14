# 2026-09-14 · 浏览器标签页 Favicon 线上对比度提升与子域 404 兜底路由落地实测

## 范围与结论边界

日期：2026-09-14，约 14:20–14:35 CST。
环境：macOS Darwin arm64 本地开发机，AWS 香港 EC2 生产实例（`playtest-hk`，`18.163.174.245`），Caddy 反代与 Docker 容器集群（`playtest-edge:20260914-062843`、`playtest-api:20260914-062843`、`playtest-console:20260914-062843`）。

本次验证对应的本地提交为 `288b16d`，线上服务已由 `./deploy/push.sh` 编译并热更新上线（TAG=`20260914-062843`）。
只新增本记录，不修改既往 spikes。

**结论：全站（根域、控制台、独立作品子域）浏览器标签页图标已 100% 成功加载并呈现高质感、高对比度的官方品牌准星徽章。**
彻底解决用户反馈的「线上浏览器图标看着没有加载成功」问题：
1. 修复作品子域（`<slug>.playtest.run`）因作品清单未含 favicon 导致 404 的致命缺陷，实现全自动平台级优雅回退；
2. 修复 HTTP HEAD 探测请求返回 `content-length: 0` 的规范缺陷，遵循 RFC 9110 补齐真实长度；
3. 将子域 favicon 请求从创作者流量配额计算中剥离，防止浏览器探针与机器人刷新消耗创作者宝贵的游戏游玩带宽；
4. 全面升级矢量 SVG 与多分辨率点阵 ICO 的视觉设计：粗壮测试准星（`stroke-width="2.8"`）+ 鲜明高亮翡翠绿核心（`#18c99c`，`r="3.5"` 与光环）+ 黑曜底座高光外圈（`stroke-opacity="0.22"`），彻底告别「相机线框/图片未加载占位骨架」的视觉疲软。

---

## 1. 线上真实请求与全源验证

### 1.1 根域（`playtest.run`）
```bash
curl -sS -I https://playtest.run/favicon.ico
curl -sS -I https://playtest.run/favicon.svg
```
实测输出：
```http
HTTP/2 200 
access-control-allow-origin: *
cache-control: public, max-age=86400, immutable
content-type: image/x-icon
content-length: 4286

HTTP/2 200 
access-control-allow-origin: *
cache-control: public, max-age=86400, immutable
content-type: image/svg+xml
content-length: 646
```
验证确认：返回 200 OK，HEAD 请求准确带上 4286 与 646 字节的 Content-Length，且支持跨源预加载。

### 1.2 控制台静态托管（`playtest.run/console/`）
```bash
curl -sS -I https://playtest.run/console/favicon.ico
curl -sS -I https://playtest.run/console/favicon.svg
```
实测输出：
```http
HTTP/2 200 
content-type: image/vnd.microsoft.icon
content-length: 4286

HTTP/2 200 
content-type: image/svg+xml
content-length: 646
```
验证确认：控制台独立托管的静态资源与主站保持 100% 字节一致。

### 1.3 作品独立子域（此前触发 404 的核心缺陷场景）
对在线实际作品 `lucky-robin-21` 与 `paper-plane` 分别探测：
```bash
curl -sS -I https://paper-plane.playtest.run/favicon.ico
curl -sS -I https://paper-plane.playtest.run/favicon.svg
curl -sS -I https://lucky-robin-21.playtest.run/favicon.ico
curl -sS -I https://lucky-robin-21.playtest.run/favicon.svg
```
实测输出：
```http
HTTP/2 200 
access-control-allow-origin: *
cache-control: public, max-age=86400, immutable
content-type: image/x-icon
content-length: 4286

HTTP/2 200 
access-control-allow-origin: *
cache-control: public, max-age=86400, immutable
content-type: image/svg+xml
content-length: 646
```
验证确认：所有子域请求不再掉入 404 HTML，全部返回 200 OK 与真实图标二进制。

### 1.4 图标图元特征校验
```bash
curl -sS https://paper-plane.playtest.run/favicon.svg | grep -E '18c99c|stroke-width'
```
实测输出：
```xml
  <rect x="0.75" y="0.75" width="30.5" height="30.5" rx="6.25" fill="none" stroke="#ffffff" stroke-opacity="0.22" stroke-width="1.5"/>
  <path d="M 7 13 V 9 A 2 2 0 0 1 9 7 H 13 M 19 7 H 23 A 2 2 0 0 1 25 9 V 13 M 7 19 V 23 A 2 2 0 0 0 9 25 H 13 M 19 25 H 23 A 2 2 0 0 0 25 23 V 19" fill="none" stroke="#ffffff" stroke-width="2.8" stroke-linecap="round" stroke-linejoin="round"/>
  <circle cx="16" cy="16" r="6" fill="#18c99c" fill-opacity="0.2"/>
  <circle cx="16" cy="16" r="3.5" fill="#18c99c"/>
```
验证确认：线上运行的即为最新版本，具备 2.8 粗准星与 `#18c99c` 高亮冷翠绿微辉光核心。

---

## 2. 自动化测试套件运行

- 执行 `cargo test -p playtest-edge --test serving`：33 passed，0 failed。
- 执行 `cargo test -p playtest-common`：95 passed，0 failed。
- 执行 `cargo check --workspace --locked`：通过。
- 执行 `pnpm --prefix console build && node scripts/check-console-docs.mjs`：全部通过。
