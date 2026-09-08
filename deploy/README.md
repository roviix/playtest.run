# 部署：一台机器跑通

香港边缘（`playtest-hk`，AWS `ap-east-1`，见 `docs/spikes/2026-09-07-aws-hk-instance.md`）用的就是这套；
自托管也是同一份 `compose.yaml`（DESIGN §7）。套路沿用 roviix / nexus 那台机器：源码 rsync 到
`~/workspace/playtest.run/`，**在服务器上 `docker build`**，`docker compose` 拉起，Caddy 自动签证书，
配置在 `deploy/`。不一样的两处：不经任何镜像仓库（v0.1 公开前不外泄）；`.env` 只在服务器上，永不入库。
Rust 依赖的编译缓存留在主机的 BuildKit 里，第一次构建慢（十几分钟），之后只重编改动的 crate。

## 第一次上线

```
# 1. 机器初始化（Docker、2 GB swap、日志上限、数据目录、备份 cron），只跑一次
ssh playtest-hk 'bash -s' < deploy/provision.sh

# 2. 服务器上建 .env（照 .env.example），至少填 ACME_EMAIL
ssh playtest-hk 'mkdir -p ~/workspace/playtest.run/deploy && vi ~/workspace/playtest.run/deploy/.env'

# 3. 同步源码、服务器上构建并发布（本机不需要 Docker）
deploy/push.sh
# 本机 Docker 健康且想快一点：BUILD_ON=local deploy/push.sh（Apple Silicon 上 linux/arm64 是原生构建，经 ssh 装载）
```

DNS 指过来、`.env` 里填了 `CLOUDFLARE_API_TOKEN` 之后再跑一次 `TARGET=none deploy/push.sh`，
Caddy 会加载 `sites/content.caddy` 并向 Let's Encrypt 签 `playtest.run` + `*.playtest.run`；
`docker compose logs -f caddy` 能看到签发过程。

## 日常

| 做什么 | 命令 |
|---|---|
| 改了 Rust 代码 | `TARGET=server deploy/push.sh` |
| 改了 Caddyfile / compose / .env | `TARGET=none deploy/push.sh` |
| 回滚 | 服务器上 `cd ~/workspace/playtest.run/deploy && SERVER_TAG=<旧标签> CADDY_TAG=<当前> docker compose up -d`（标签在 `deploy/.tag` 与 `docker images`，只留最近三个） |
| 看日志 | `ssh playtest-hk 'cd workspace/playtest.run/deploy && docker compose logs -f --tail 100'` |
| 用 CLI 打服务器上的控制面（`API_HOST` 还没配时） | `ssh -N -L 8787:127.0.0.1:8787 playtest-hk &` 然后 `PLAYTEST_API=http://127.0.0.1:8787 playtest ./dist` |

数据都在主机 `~/playtest-data/`（`store/` 对象存储、`api.sqlite`、`edge-events.jsonl`）。
`backup.sh` 每小时把 SQLite 和清单备到 `~/db-backups/playtest/`，保留 7 天；blob 靠 EBS 快照。

## 还没有的

- `api.playtest.sh`：域名没买。买到后 Cloudflare 加 A 记录指向同一个 IP，`.env` 填 `API_HOST=api.playtest.sh`，`TARGET=none deploy/push.sh`。
- 多台边缘、S3 对象存储、令牌撤销推送、用量上报：都在 DESIGN 第三四周。
