# 2026-09-15 · 翻公开、发 v0.3.0，以及英文首发下暴露的语言缺口

日期：2026-09-15 17:45–20:50 CST。环境：macOS arm64 本机，`main` 在 `d877f69`，生产 `playtest.run`。接上一条 `2026-09-15-stranger-install-path-audit.md`。

## 做成了什么

- **仓库已公开。** `roviix/playtest.run` 由 PRIVATE 改为 PUBLIC，补了描述、homepage 与 10 个 topic；开启 private vulnerability reporting（`SECURITY.md` 指向它）、secret scanning 与 push protection。翻公开前对全部已跟踪文件再扫一遍 `cfoat_ / cfort_ / ghp_ / github_pat_ / account id`，无命中。
- **v0.3.0 已发布。** `git tag -a v0.3.0` 触发的 release 工作流五个 target 全绿，产出 5 个包 + `SHA256SUMS` + `VERSION`，Release 页对未登录访客可见。
- **R2 已同步到 v0.3.0。** 仓库没有配 `CLOUDFLARE_API_TOKEN`，工作流里的 R2 步骤按预期跳过；改用本机 wrangler OAuth 登录跑 `deploy/upload-release-to-r2.sh --input`，7 个文件逐一 CDN 回读且 SHA-256 全部一致。`https://dl.roviix.com/files/VERSION` 现为 `v0.3.0`。
- **陌生人闭环用「装出来的」二进制走通了**（上一条 spike 是用本地构建的二进制，这次是真下载）：空 `HOME` 跑 `curl -fsSL https://playtest.run/install.sh | bash` 装到 `playtest 0.3.0`；`playtest . --json` 发布 Phaser 夹具 `elapsed_ms` 1071（文件已在服务器上，`upload_ms` 0）拿到 `lucky-crane-65`；未登录 curl 验证 `/p/lucky-crane-65` 200、`lucky-crane-65.playtest.run/` 200、`card.png` 70 KB 200；`playtest rm -y` 后子域 404。

## 暴露出来的问题

- **产品并不是「全站英文」，DESIGN §1.5 的说法与线上不符。** 线上作品页同一屏里混着 `ouyz invites you to test 想象空间`、`‹ 返回`、`作者想问：`、`原声与交流`、`☕ 治愈满分`，聊天流的运行时文案（`我`、`刚刚`、`发送未成功，请稍后重试`、`网络中断，请稍后重试`）也是中文。CLI 全部输出是中文：安装脚本说「已安装 playtest v0.3.0 到 …」，发布说「正在整理 .：2 个文件，1.1 MB」「看起来是 Phaser 做的」，`rm` 无 TTY 时说「这里不是终端，没法问你确认」。
  统计（字符串字面量里含中文的行，不含注释）：`cli/src` 517 行、`edge/src` 426 行、`deploy/site/install.sh` 20 行。
  §0.10 今天刚改成英文首发、面向海外网页导出作者，这意味着目标用户装完第一步就看不懂。**在这批文案改完之前，不应该往任何英文社群发帖。**
- **生产机 SSH 不通，`deploy/push.sh` 跑不了。** `ssh playtest-hk` 稳定失败在 `kex_exchange_identification: Connection closed by remote host`；`nc -vz 18.163.174.245 22` 显示 TCP 握手是成功的，说明不是安全组拦截，是 sshd 层面把连接关掉（fail2ban 封禁、`MaxStartups`、或机器资源不足）。站点本身正常：根域 200、耗时 0.98 s。
  后果：线上 `skill.md` 的 Install 段仍是旧的 `Releases: https://github.com/roviix/playtest.run/releases`（现在这个链接对外能打开了，所以不再是死链，但不是我们想让 agent 走的 `install.sh` 路径）。

## 没验的

- 没有 Windows、Linux 真机装过 v0.3.0；没有手机扫码；仍然没有第二个人用过。
- 没有海外网络实测——边缘在香港，欧美玩家的打开耗时是空白（DESIGN §4.4 已标注）。
- 没查清 sshd 为什么关连接；没有登上机器，机器负载与磁盘未知。
