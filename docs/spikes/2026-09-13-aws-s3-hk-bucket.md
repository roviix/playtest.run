# 2026-09-13 · AWS 香港独立 S3 存储桶开通与真机探针实测

**结论**：已在生产 AWS 账号（`306903156946`）下开通专属香港私有 S3 存储桶 `playtest-assets-306903156946-hk`（`ap-east-1`），全盘开启公网封锁、版本控制与分段上传 7 天自动清理规则。写、读、删端到端探针通过，具备生产迁移就绪条件。

未直接混用 `mira` 位于 `us-east-2` 的现有业务桶，原因符合 DESIGN §4.2 / §4.4：
1. 避免跨洋访问延迟（香港内网 < 2ms vs 俄亥俄 200ms+）及出网费用；
2. 隔离垃圾回收与对象删除的爆炸半径，保护各业务线上资产安全；
3. 建立独立的成本审计边界。

## 一、开通的资源与配置

| 项目 | 配置值 | 说明 |
|---|---|---|
| **AWS 账号** | `306903156946` | 与香港边缘实例 `playtest-hk` 同账号 |
| **存储桶** | `playtest-assets-306903156946-hk` | 独立私有桶，专供 Playtest 作品物料 |
| **区域** | `ap-east-1`（香港） | 与边缘实例同区域内网通信（免回源流量费） |
| **公网封锁** | Block Public Access (All Enabled) | 严格私有，不提供任何公开签名直链 |
| **版本控制** | `Status=Enabled` | 防误删与灾备恢复 |
| **生命周期** | `AbortIncompleteMultipartUpload: 7 days` | 自动清理未完成的分段上传碎片，杜绝暗中扣费 |
| **标签** | `Project=playtest, Environment=production` | 便于 Cost Explorer 独立统计成本 |

## 二、端到端真机探针验证

使用该账号的 IAM 身份对香港桶执行写、读、删探针验证：

```text
1. 写入对象：s3://playtest-assets-306903156946-hk/system/probes/test-probe.txt -> 成功（exit code 0）
2. 读回校验：内容精确为 "playtest-s3-test" -> 成功（exit code 0）
3. 清理删除：删除探针对象 -> 成功（exit code 0）
```

## 三、下一步操作

1. 将存储桶配置更新至 `playtest-hk` 的 `deploy/.env`。
2. 停止 API 写入，运行迁移工具将 `~/playtest-data/store` 增量校验同步入 S3。
3. 重启 API 与 Edge 服务，由启动探针自动完成最终上线验证。
