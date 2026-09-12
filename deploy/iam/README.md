# AWS S3 与 IAM 生产配置指南

Playtest 生产环境推荐在与边缘实例相同的区域（香港 `ap-east-1`）开通**独立私有 S3 存储桶**。

---

## 为什么建议独立 Bucket，而不是直接共用 mira 或 roviix 现有业务桶？

1. **安全爆炸半径（Blast Radius）**：Playtest 有定时垃圾回收机制（`DailyCleanup`），会扫描并删除未被清单引用的孤儿 blob。即使有前缀隔离，一旦配置遗漏或手抖，也存在影响外部生产资产的风险。
2. **权限最小化**：Playtest Edge 面对全球公网玩家流量。如果持有能访问外部业务桶的凭据，一旦发生安全隐患可能造成跨业务横向泄露。
3. **成本审计**：新建私有 S3 桶本身免费（按存量和流量计费）。独立桶能在 AWS Cost Explorer 中一目了然统计作品容量与回源费用（DESIGN §6）。

---

## 方式一：推荐方案（EC2 实例角色，零密钥落盘）

如果 API 与 Edge 均跑在香港实例 `playtest-hk`（`i-045359549230d195b`）上，最安全的做法是为该 EC2 附加 IAM 角色：

### 1. 创建私有存储桶（香港 ap-east-1）
```bash
# 替换为你的桶名，例如 playtest-assets-hk
export BUCKET_NAME=playtest-assets-hk

aws s3api create-bucket \
  --bucket "$BUCKET_NAME" \
  --region ap-east-1 \
  --create-bucket-configuration LocationConstraint=ap-east-1

# 强制开启全部公网访问封锁（Block Public Access）
aws s3api put-public-access-block \
  --bucket "$BUCKET_NAME" \
  --public-access-block-configuration "BlockPublicAcls=true,IgnorePublicAcls=true,BlockPublicPolicy=true,RestrictPublicBuckets=true"

# 开启版本控制（灾备与防误删，DESIGN §4.2）
aws s3api put-bucket-versioning \
  --bucket "$BUCKET_NAME" \
  --versioning-configuration Status=Enabled

# 自动清理 7 天未完成的分段上传（避免孤儿分段暗中计费）
aws s3api put-bucket-lifecycle-configuration \
  --bucket "$BUCKET_NAME" \
  --lifecycle-configuration file://deploy/iam/lifecycle-rule.json
```

### 2. 创建 IAM 策略并附加到 EC2 实例
将 `api-policy.json` 中的 `${PLAYTEST_S3_BUCKET}` 替换为真实桶名后创建策略：
```bash
sed "s/\${PLAYTEST_S3_BUCKET}/$BUCKET_NAME/g" deploy/iam/api-policy.json > /tmp/playtest-s3-policy.json

aws iam create-policy \
  --policy-name PlaytestS3Access \
  --policy-document file:///tmp/playtest-s3-policy.json

# 创建 Role 并附加信任关系（EC2）
aws iam create-role \
  --role-name PlaytestHkInstanceRole \
  --assume-role-policy-document '{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"Service":"ec2.amazonaws.com"},"Action":"sts:AssumeRole"}]}'

aws iam attach-role-policy \
  --role-name PlaytestHkInstanceRole \
  --policy-arn "arn:aws:iam::306903156946:policy/PlaytestS3Access"

# 创建实例配置文件并绑定到 playtest-hk
aws iam create-instance-profile --instance-profile-name PlaytestHkInstanceProfile
aws iam add-role-to-instance-profile --instance-profile-name PlaytestHkInstanceProfile --role-name PlaytestHkInstanceRole

aws ec2 associate-iam-instance-profile \
  --instance-id i-045359549230d195b \
  --iam-instance-profile Name=PlaytestHkInstanceProfile
```

### 3. 服务器 `.env` 配置
在服务器 `deploy/.env` 中，**完全不用填 Access Key / Secret Key**，只填：
```ini
PLAYTEST_STORAGE_BACKEND=s3
PLAYTEST_S3_BUCKET=playtest-assets-hk
PLAYTEST_S3_REGION=ap-east-1
PLAYTEST_S3_ENDPOINT=
PLAYTEST_S3_PREFIX=
```
`object_store` 会自动通过 IMDSv2 获取实例凭据，优雅且安全。

---

## 方式二：静态凭据（读写分离）

如果出于架构考虑希望 API 与 Edge 使用独立的静态凭据：
1. **API 用户**：附加 `api-policy.json`（具备 `GetObject`, `PutObject`, `DeleteObject`, `AbortMultipartUpload`, `ListBucket`）。将生成的 Key 填入 `PLAYTEST_S3_API_ACCESS_KEY_ID` 和 `PLAYTEST_S3_API_SECRET_ACCESS_KEY`。
2. **Edge 用户**：附加 `edge-policy.json`（**仅**具备 `GetObject` 与 `ListBucket` 只读权限）。将生成的 Key 填入 `PLAYTEST_S3_EDGE_ACCESS_KEY_ID` 和 `PLAYTEST_S3_EDGE_SECRET_ACCESS_KEY`。
