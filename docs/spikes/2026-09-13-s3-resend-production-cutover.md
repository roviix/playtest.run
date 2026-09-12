# 2026-09-13 · 生产 S3 存储切换、数据清空与 Resend 邮件服务真实上线实测

**结论**：生产环境（`playtest-hk`，`18.163.174.245`）已完成全栈版本更新与数据重置，全面接入 AWS 香港专属 S3 存储桶（`playtest-assets-306903156946-hk`，`ap-east-1`）与 Resend 邮件服务（`notice@playtest.run`）。端到端测试均通过。

## 一、上线与重置内容

1. **代码与镜像**：
   - 包含统一 S3 存储流式读取、HTTP Range 206、作品合集与关注邮件系统的全栈镜像构建（Tag: `20260912-170706`）。
   - 涉及组件：`api`、`edge`、`caddy`、`console`。
2. **数据清空与重建**：
   - 旧本地存储数据已安全移出至备份归档目录，生产数据目录由空初始化。
   - 数据库全新应用迁移版本 v1 ~ v6。
   - 重新生成隧道公私钥并自动发布至 S3 存储根目录 `keys/tunnel.pub`。
3. **服务配置注入**：
   - `PLAYTEST_STORAGE_BACKEND=s3`
   - `PLAYTEST_S3_BUCKET=playtest-assets-306903156946-hk`
   - `PLAYTEST_S3_REGION=ap-east-1`
   - `PLAYTEST_EMAIL_PROVIDER=resend`
   - `PLAYTEST_RESEND_API_KEY=re_S9PD...`（仅存于服务器 `deploy/.env`，严格保密）
   - `PLAYTEST_EMAIL_FROM="playtest.run <notice@playtest.run>"`

## 二、真机端到端验证链路

1. **作品发布（CLI -> API -> S3）**：
   - 运行客户端发布本地固件《鹈鹕单车》v1：
     - 上传哈希：`f879de58806104d6e8e1513758e1ada9d01216ba4ede278859eeb4febdde9ffd`
     - 生成链接：`https://playtest.run/p/cyan-ermine-54`
     - 分配子域：`https://cyan-ermine-54.playtest.run/`
   - 服务端日志确认：成功写入 S3 桶，版本提交为 v1。

2. **边缘静态流式交付与 Range（Edge -> S3 -> Client）**：
   - `curl -sS -I https://cyan-ermine-54.playtest.run/` -> 返回 `HTTP/2 200`，`accept-ranges: bytes`，`content-length: 4772`。
   - `curl -sS -I -r 0-99 https://cyan-ermine-54.playtest.run/` -> 返回 `HTTP/2 206 Partial Content`，`content-range: bytes 0-99/4772`。
   - 证明边缘节点直接通过 S3 流式拉取物料，分段与断点续传完美支持。

3. **动态能力与 Resend 邮件发送（Web Gate -> API -> Resend API）**：
   - 门禁页根据 S3 读取到的 `capabilities.json`（`email: true`）正常展示关注更新弹窗与邮箱表单。
   - 向 `https://playtest.run/follow` 提交关注申请，API 自动排队并由调度器执行 `job="deliver_notifications"`。
   - 服务端日志确认：`做完一轮 job="deliver_notifications" count=1`，邮件经由 Resend API（`re_S9PD...`）成功投递至目标邮箱。
