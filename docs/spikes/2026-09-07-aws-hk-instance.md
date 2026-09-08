# 2026-09-07 · 第一台边缘机器：AWS 香港 `ap-east-1`

**结论**：私测用的第一台机器开起来了，SSH 可登、出网正常。这是 DESIGN §4.4「先各起一台在晚高峰压一周再定」的第一个候选，
**不是**对「香港哪家云」的最终决定（DESIGN §9 第 1 条仍开放）；腾讯云 / 阿里云香港的轻量机留作对比。

为什么单开一台而不是复用 roviix 那台 `us-east-2` 的 EC2：位置在俄亥俄，大陆玩家 200 ms 起，会把私测的 T3 / T5 测歪；
那台机 3.7 GB 内存已满载；用户内容域一旦被封 IP 会连累 `relay.roviix.com`。

## 创建了什么（账号 306903156946，全部打了 `Project=playtest` 标签）

| 资源 | 标识 | 说明 |
|---|---|---|
| 区域 | `ap-east-1`（香港） | 原为未启用，`aws account enable-region` 后约 1 分钟变 `ENABLED` |
| 实例 | `i-045359549230d195b` | `t4g.small`（2 vCPU / 2 GB，ARM64），`ap-east-1a`，Ubuntu 24.04.4 LTS（`ami-0539915b86b9ed971`，Canonical 2026-09-04 版），IMDSv2 强制，CPU 积分 standard |
| 根盘 | 30 GB gp3，加密，随实例删除 | 开机后可用 26 GB |
| 弹性 IP | `18.163.174.245`（`eipalloc-062114118c04cb639`） | `*.playtest.run` 与根域都解析到它 |
| 安全组 | `sg-0796ee4f1c0fa2e29` `playtest-edge` | 入站：tcp 80、tcp 443、udp 443（给 v0.2 的 QUIC 预留）对 `0.0.0.0/0` 与 `::/0`；tcp 22 只对创始人当时的公网 IP `/32`；ICMP echo 对全网（晚高峰 ping 用） |
| 密钥对 | `playtest-hk`（ed25519） | 私钥在创始人机器 `~/.ssh/playtest-hk.pem`，0600，不入库 |
| 主机名 | `playtest-hk` | 由 user-data 设置 |

用的 IAM 身份是 `mira-startup`，临时加了一段只含 `account:EnableRegion` 与 EC2 创建类动作的内联策略；
机器建好后应把这段策略删掉，让它回到只管 S3。

## 实际执行与观察

```
$ aws account get-region-opt-status --region-name ap-east-1
ap-east-1  DISABLED
$ aws account enable-region --region-name ap-east-1
13:49:51 ENABLING … 13:50:57 ENABLED

$ aws ec2 run-instances --image-id ami-0539915b86b9ed971 --instance-type t4g.small \
    --key-name playtest-hk --security-group-ids sg-0796ee4f1c0fa2e29 --subnet-id subnet-0e4a938d578d0368b \
    --block-device-mappings 'DeviceName=/dev/sda1,Ebs={VolumeSize=30,VolumeType=gp3,DeleteOnTermination=true,Encrypted=true}' \
    --metadata-options HttpTokens=required,HttpEndpoint=enabled --credit-specification CpuCredits=standard
实例：i-045359549230d195b → running
$ aws ec2 allocate-address --domain vpc → eipalloc-062114118c04cb639；associate → eipassoc-037d4119cbf8c509b

$ ssh -i ~/.ssh/playtest-hk.pem ubuntu@18.163.174.245 'hostname; uname -m; free -m | sed -n 2p; df -h / | tail -1'
playtest-hk
aarch64
Mem:  1835  379  1279  0  318  1455
/dev/root  29G  2.1G  26G  8% /
```

## 没验到的

- **大陆到香港的回程质量没测。** 创始人这台 Mac 挂着 Clash（TUN 模式），ping 141 ms、TCP 握手 0 ms 都是代理的数字，
  不能用。要在 20:00–23:00 用不挂代理的大陆网络（手机流量最干净）ping 与下载一个几十 MB 的文件，连测一周，
  再和腾讯云 / 阿里云香港的机器比。
- 机器上还什么都没装：Docker、Caddy、api、edge 都没有。DNS 也还没指过来。

## 成本

按 `ap-east-1` 按需价：实例约 $15/月，gp3 30 GB 约 $3.6/月，弹性 IP 约 $3.6/月，出网 $0.12/GB。
