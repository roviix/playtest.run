# 2026-09-07 · 第一批真链接：`https://<slug>.playtest.run` 在香港边缘上线

**结论**：上传路径在公网成立。香港机器上跑着 Caddy + api + edge（`deploy/`），Let's Encrypt 签下了
`playtest.run` 与 `*.playtest.run`（DNS-01，Cloudflare API），CLI 经 ssh 隧道把三个导出物发布成
三个真链接；从外部 curl 与无头 Chrome 验证：门禁页、`Secure` cookie、wasm MIME、Unity 布局的 `.br`、
Range 206、COOP/COEP 全部和本机一致，响应头自检页四项全绿，Phaser 可玩。

**还没成立的**：控制面没有公网入口（`playtest.sh` 没买，CLI 走 ssh 隧道）；**手机还没扫过码**——
这是 DESIGN §8 T1 的定义，现在链接是真的了，等创始人用手机流量点开；大陆晚高峰回程没测。

机器：`playtest-hk`（`docs/spikes/2026-09-07-aws-hk-instance.md`）。DNS 与令牌由创始人在 Cloudflare 完成
（区域 active，`A @` 与 `A *` → `18.163.174.245`，DNS-only）。

## 一、服务器初始化与构建

```
$ ssh playtest-hk 'bash -s' < deploy/provision.sh
Docker version 29.8.0 / Docker Compose version v5.5.1；2 GB swap；journald 200M；~/playtest-data 归 uid 10001；备份 cron 每小时

$ deploy/push.sh            # 默认 BUILD_ON=server：rsync 源码 → 主机 docker build → compose up
-- docker build playtest-server:20260907-082221
   Finished `release` profile [optimized] target(s) in 3m 04s      ← t4g.small，2 vCPU / 2 GB + 2 GB swap，没有 OOM
-- docker build playtest-caddy:20260907-082221                      ← xcaddy + caddy-dns/cloudflare
real 2m50s
启用 playtest.run + *.playtest.run（DNS-01）
未对公网暴露控制面：.env 里 API_HOST 为空（CLI 走 ssh 隧道）
playtest-api-1 / playtest-caddy-1 / playtest-edge-1  Up
ok ← api      ok ← edge
```

为了让 2 GB 的机器编得动，根 `Cargo.toml` 的 release profile 去掉了 `lto = "thin"` 与 `codegen-units = 1`。
本机 Docker Desktop 那次构建失败（`input/output error`），是宿主机磁盘满导致的虚拟盘写错，不是镜像的问题；
这也是把默认构建位置改到服务器的直接原因。

Caddy 日志（`docker compose logs caddy`）：

```
obtaining certificate | playtest.run
obtaining certificate | *.playtest.run
trying to solve challenge | challenge_type: dns-01
validations succeeded; finalizing order
certificate obtained successfully | *.playtest.run
certificate obtained successfully | playtest.run
```

从提交到两张证书到手不到一分钟。

## 二、发布三个导出物

```
$ ssh -N -L 18787:127.0.0.1:8787 playtest-hk &
$ export PLAYTEST_API=http://127.0.0.1:18787
$ playtest ./fixtures/phaser-jump/export -n "Phaser 跳一跳" -m "香港边缘第一版"
已发布 v1
https://eager-vole-11.playtest.run
本次 4.8 秒（哈希 0.1 · 上传 3.5 · 提交 0.2）
$ playtest ./fixtures/headers-lab/export --isolated -n "响应头实验页" -m "香港边缘第一版"
https://witty-sable-47.playtest.run        本次 4.6 秒
$ playtest ./fixtures/vite-vanilla/export -n "Vite 计数器" -m "香港边缘第一版"
https://rapid-sable-38.playtest.run        本次 2.0 秒
$ playtest ls                              三个都在同一个匿名身份下
```

上传经的是 Clash 代理出口到香港，1.1 MB 3.5 秒；直连会快。

## 三、外部验证（curl）

```
$ curl -sS -D - -o /dev/null -H 'Accept: text/html' https://witty-sable-47.playtest.run/
HTTP/2 200
alt-svc: h3=":443"; ma=2592000
cache-control: no-store
cross-origin-embedder-policy: require-corp
cross-origin-opener-policy: same-origin
strict-transport-security: max-age=31536000
（页面含：邀请你试玩《响应头实验页》· v1 · 由 playtest.run 提供）

$ curl -sS -D - -o /dev/null -X POST -d 'to=/' https://witty-sable-47.playtest.run/_playtest/start
HTTP/2 303  location: /
set-cookie: pt_gate=1; Path=/; SameSite=Lax; HttpOnly; Max-Age=86400; Secure
set-cookie: pt_sid=…; Path=/; SameSite=Lax; HttpOnly; Max-Age=7776000; Secure

$ curl -sS -D - -o /dev/null -H 'Accept-Encoding: br' https://witty-sable-47.playtest.run/mod.wasm
HTTP/2 200  content-encoding: br  content-type: application/wasm  content-length: 42
$ curl -sS -D - -o /dev/null https://witty-sable-47.playtest.run/Build/hello.wasm.br
HTTP/2 200  content-encoding: br  content-type: application/wasm
$ curl -sS -D - -o /tmp/r.bin -H 'Range: bytes=0-9' https://witty-sable-47.playtest.run/data.bin
HTTP/2 206  content-range: bytes 0-9/1048576  content-length: 10   bytes=00010203040506070809

https://nope-nope-1.playtest.run/            404
http://eager-vole-11.playtest.run/           308 → https://eager-vole-11.playtest.run/
https://playtest.run/                        200
```

证书：`CN=playtest.run`，Let's Encrypt，2026-09-07 → 2026-12-06。

## 四、无头 Chrome（真域名）

```
$ node scripts/headless-check.mjs "https://witty-sable-47.playtest.run/" --click 'form[action="/_playtest/start"] button' --wait 4000
gate title：匿名开发者 邀请你试玩《响应头实验页》
1. mod.wasm 流式编译            ✅ add(2, 3) = 5
2. Build/hello.wasm.br 预压缩   ✅ 解压 + 流式编译成功
3. data.bin 的 Range 请求       ✅ 206，Content-Range 与前 10 字节都对
4. 跨源隔离                     ✅ crossOriginIsolated，SharedArrayBuffer 可用

$ node scripts/headless-check.mjs "https://eager-vole-11.playtest.run/" --click 'form[action="/_playtest/start"] button' --wait 3000
gate title：匿名开发者 邀请你试玩《Phaser 跳一跳》 → phaser-jump，Phaser v3.90.0 (Canvas | Web Audio)
$ node scripts/headless-check.mjs "https://eager-vole-11.playtest.run/" --settle 2500 --click canvas --wait 300
第二次打开没有门禁页；截图 HUD：跳了 1 次 · AudioContext.state: running
```

## 五、顺手发现的两个 CLI 小问题（没修）

- `fixtures/headers-lab` 被认成「看起来是 Godot 做的」——有 `.wasm` 加 `SharedArrayBuffer` 字样就算 Godot，判据太宽。
- 配置里 `sites` 只按目录记 slug，不看 `api`：同一目录先发本机再发香港，第二次会撞 404 再自动新建——能用，但换 API 时会把本机那边记着的 slug 覆盖掉。第三周做登录时一起改成按（api, 目录）记。

## 六、安全备忘

- Cloudflare 令牌是通过聊天窗口交给我的，会留在 Cursor 的对话记录里；稳定后在 Cloudflare 里 Roll 一次并更新服务器 `deploy/.env`。
- `mira-startup` 上临时加的 EC2 策略还没删。
