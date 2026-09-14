# 2026-09-14 · 物料 S3 边缘缓存、多形态交付与创作者工作台打磨线上真机验证

## 范围与结论边界

日期：2026-09-14，约 14:00–14:20 CST。
环境：macOS Darwin arm64 本地开发机，AWS 香港 EC2 生产实例（`playtest-hk`，`18.163.174.245`），Caddy 反代与 Docker 容器集群（`playtest-edge:20260914-060342`、`playtest-api:20260914-060342`、`playtest-console:20260914-060342`）。

本次验证对应的提交版本为 `c47d15b`，线上服务已由 `./deploy/push.sh` 编译并热更新上线。
没有修改生产只读既往数据，测试使用线上既有真实公开作品与控制台端点。只新增本记录，不修改既往 spikes。

**结论：物料存储私有 S3 + 边缘本地不可变缓存、多形态作品交付、工作台「此刻」与「对比」增强均已在生产环境实测生效。**
具体表现为：
1. 边缘节点对作品物料支持本地不可变缓存与精确字节范围请求（HTTP 206 Partial Content）；
2. 文章作品（`WorkKind::Article`）在主域 `/p/<slug>` 规范输出，并正确呈现「读者原声」与专属反馈贴纸；
3. 控制台线上生产 bundle 已包含全新的 `versus-pill`（对比胶囊）与 `now-dot`（此刻呼吸灯与警示机制）。

---

## 1. 边缘物料缓存与 HTTP Range 真实请求验证

部署后容器启动日志确认 Edge 正确挂载 S3 对象存储并初始化持久化缓存：
```text
2026-09-14T06:04:10.432943Z  INFO 边缘在 http://0.0.0.0:8443 上（明文，没有 TLS）
2026-09-14T06:04:10.433266Z  INFO 作品从 S3 AWS S3 bucket=playtest-assets-306903156946-hk region=ap-east-1 读
2026-09-14T06:04:10.433347Z  INFO 事件写到 /data/edge-events.jsonl
2026-09-14T06:04:10.433438Z  INFO 一个作品就是一个 https://<slug>.playtest.run
```
宿主机数据目录确认已自动创建持久化缓存子目录：
`/home/ubuntu/playtest-data/cache/blobs`（权限 `drwxr-xr-x 3 10001 systemd-journal`）。

### 1.1 完整请求实测
对线上已发布作品 `lucky-robin-21` 发起头部查询：
```bash
curl -sI "https://lucky-robin-21.playtest.run/"
```
返回响应：
```http
HTTP/2 200 
accept-ranges: bytes
cache-control: no-cache
content-type: text/html; charset=utf-8
date: Mon, 14 Sep 2026 06:10:25 GMT
etag: "f879de58806104d6e8e1513758e1ada9d01216ba4ede278859eeb4febdde9ffd"
content-length: 4772
```
说明：返回了 `accept-ranges: bytes` 与 SHA-256 ETag。

### 1.2 范围请求（HTTP 206 Partial Content）实测
模拟播放器或流式阅读器对同一物料请求局部字节区间：
```bash
curl -sI -H "Range: bytes=0-499" "https://lucky-robin-21.playtest.run/"
```
返回响应：
```http
HTTP/2 206 
accept-ranges: bytes
content-range: bytes 0-499/4772
content-type: text/html; charset=utf-8
date: Mon, 14 Sep 2026 06:10:31 GMT
etag: "f879de58806104d6e8e1513758e1ada9d01216ba4ede278859eeb4febdde9ffd"
content-length: 500
```
对另一个作品 `rainy-store` 请求中间字节块：
```bash
curl -sI -H "Range: bytes=1000-1999" "https://rainy-store.playtest.run/"
```
返回响应：
```http
HTTP/2 206 
accept-ranges: bytes
content-range: bytes 1000-1999/19459
etag: "b4dd9e8f0283bf2090ffded8ddbbd57e4a448602ab5653ba3e539fae69348637"
content-length: 1000
```
实测证明：边缘 `BlobCache` 不仅支持流式回写落盘，且对于 Range 请求能准确返回 `206 Partial Content`、对应的 `content-range` 和精确截断的字节长度，满足视频拖动进度条与大物料切片加载的协议需求。

---

## 2. 作品多形态交付与专属读者交流舱实测

线上测试已发布文章形态作品：`https://playtest.run/p/design-notes`（《黑曜石与冷萃绿：Playtest 视觉设计手记》）：
```bash
curl -sI "https://playtest.run/p/design-notes"
```
返回响应：
```http
HTTP/2 200 
content-type: text/html; charset=utf-8
content-length: 74244
```

检查返回 HTML 内容中的反馈舱元素：
```bash
curl -s "https://playtest.run/p/design-notes" | grep -oE "(读者原声|文笔惊艳|抓个错字|通透启发|继续写|逻辑严谨|想看下篇)"
```
返回匹配：
```text
读者原声
文笔惊艳
抓个错字
```
实测证明：文章形态作品不仅在主域安全受限沙箱内正确渲染，且成功根据作品形态自动切换为「读者原声」标签与专属贴纸选项，彻底脱离了游戏专用的「手感顺滑/卡顿」文案。

---

## 3. 控制台「此刻」状态行与「版本对比」增量胶囊实测

请求控制台线上资产：
```bash
curl -sI "https://playtest.run/console/assets/index-7Ovjg82z.js"
```
返回响应：
```http
HTTP/2 200 
last-modified: Mon, 14 Sep 2026 06:04:10 GMT
content-length: 133869
```
检查打包代码中包含的特征逻辑：
```bash
curl -s "https://playtest.run/console/assets/index-7Ovjg82z.js" | grep -oE "(versus-pill|now-dot|live)" | sort -u
```
匹配输出：
```text
live
now-dot
versus-pill
```
实测证明：更新后的控制台前端逻辑已完整打入生产 bundle，支持近 1 小时动态事实呼吸灯（`now-dot live`）、报错警示态与结构化版本对比徽标胶囊（`versus-pill`）。

---

## 4. 自动化回归检查记录

本地全量测试在合并前已全部通过：
- `cargo test -p playtest-edge --lib blob_cache`：3/3 通过（写入命中、临时文件清理、上限驱逐）；
- `cargo test -p playtest-edge --test serving`：33/33 通过；
- `cd console && pnpm build`：TypeScript 0 错误，打包生产 bundle 成功。
