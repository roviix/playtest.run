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

账号、权限、反馈、事件与用量账本仍在主机 `~/playtest-data/`；作品 blob、版本清单、当前指针、
封面与边缘读取快照都在私有 S3 桶。`backup.sh` 只负责本机数据库，不能当作 S3 灾备。

## 对象存储

生产 compose 默认 `PLAYTEST_STORAGE_BACKEND=s3`。在 `.env` 配桶名与区域；AWS 原生 S3
不写 endpoint，MinIO 等兼容服务才写完整 endpoint。API 凭据需要 `GetObject`、`PutObject`、
`DeleteObject` 与桶前缀内的 `ListBucket`，edge 凭据只需 `GetObject` 与 `ListBucket`。也可以不写
两组静态凭据，改用实例或容器角色。桶保持私有，不给玩家签名直链。

API 启动会写入、读回并删除一个小探针，edge 启动会验证读取；配置、权限或连通性失败时容器
直接失败，不会回退到 `/data/store` 后仍报告成功。旧共享盘迁移完成前不要切生产写入源；先复制、
校验，再用同一个真实域名核对现有链接、Range、回滚和删除，旧盘保留到回退验收完成。

迁移工具只接受本地旧目录作为源、S3 作为目标；逐对象计算 SHA-256，blob 还会核对哈希键，
写后从目标再读一遍。已一致的对象会跳过，不同内容的 blob / 历史清单拒绝覆盖，当前指针最后写：

```
# 先停止 API，edge 可继续从旧盘服务；把 deploy/.env 里的 S3 变量导入当前 shell 后运行
cargo run -p playtest-common --example migrate_store -- /home/ubuntu/playtest-data/store
```

工具成功只说明复制校验完成，不等于切换验收完成。随后把 API 与 edge 一起切到 S3，核对旧链接、
回滚、Range、删除和垃圾回收；回退演练完成前不要删 `/home/ubuntu/playtest-data/store`。

## 邮件启用前

Compose 默认 `PLAYTEST_EMAIL_PROVIDER=off`，不向玩家提供邮件关注。直接运行本机 API 的默认值仍是 `log`，只在终端预览；日志和队列的 `sent` 不是收件箱送达证明。不要把 `log` 用作生产发信。

要启用托管版，在服务器私有配置中设置 `PLAYTEST_EMAIL_PROVIDER=resend`、`PLAYTEST_RESEND_API_KEY`、`PLAYTEST_EMAIL_FROM=playtest.run <notice@playtest.run>` 和 `PLAYTEST_PUBLIC_ROOT_URL=https://playtest.run`。自托管使用 `smtp` 与 `PLAYTEST_SMTP_URL`，玩家根地址改成自己的域名。**必须显式配置玩家根地址**；默认 `http://localhost:8443` 只供本机使用，不能出现在真实确认信里。`resend` / `smtp` 在根地址不是 HTTPS、是 localhost / 回环地址、包含路径或发件人格式错误时拒绝启动，不静默发出坏链接。密钥不写进仓库、不打印到核查记录。

启用前逐项验收，而不是「设置了 key 就算好」：

1. 邮件服务商确认发信域验证完成，核对 SPF / DKIM / DMARC。不能只凭存在一条 DMARC TXT 记录认定其余项也正确。
2. `cargo run -p playtest-api --example email_preview` 生成本机 HTML、纯文本与 MIME 样例；预览复用真实队列文案和模板，不发信。
3. 先向自己有权使用的 Gmail / QQ / 163 测试邮箱分别发送确认信、换设备信和更新信，记录收件位置、手机显示、身份链接往返、24 小时限频、退订后不再收到。没有完成收件实测就写「未验收」。
4. 把有日期的真实记录追加到 `docs/spikes/`，再启用面向玩家的邮件路径。服务商接受不等于收件箱送达；当前未接入投递、退信与投诉事件回执，也不能把队列 `sent` 称为「已送达」。

## 还没有的

- 多台边缘与令牌撤销推送仍未完成；S3 后端代码存在不等于生产数据已经迁完，真实切换状态只以
  `docs/spikes/` 的当日记录为准。
