# 本地验收统一产品

产品定义以 DESIGN.md 为准。本机与生产都使用同一平台源承载广场、关注、登录与管理，不能把 Vite 和边缘分别打开后当成同域会话验收。

## 启动

```sh
cargo build --workspace --bins
npm run build --prefix console
node scripts/dev-unified.mjs
```

默认入口是 `http://localhost:48550`，管理为同一地址下的 `/console/`，作品为 `http://<slug>.localhost:48550`。API、边缘与 GitHub 替身分别使用后续三个端口；用 PORT 覆盖整组起始端口。每次启动创建独立的 `.data/unified-*` 数据目录，路径打印在终端，不读取生产凭据。

- 邮箱使用 log，不真的发信；只可用于本地验证。登录链接在该目录 api.sqlite 的 notifications.url 中，先查到自己刚请求的记录，再在浏览器中打开。
- GitHub 使用本机替身，代表一个明确标记的本地验收作者；它不是外网 GitHub 验收证据。
- 平台会话使用 host-only pt_session Cookie。CLI/助手令牌单独签发；不要写进 localStorage 或发到作品子域。
- Ctrl-C 结束本次服务；临时数据保留便于检查，不会自动清空别的开发目录。

## 核对路径

1. 无痕访问广场，作品／合集切换；我的作品、我的合集原地登录，Esc 返回原处；公开作品与文档不要求身份。
2. 邮箱首次登录、再次登录、过期和重放；成功回原页面。打开新页仍为同一账号，退出当前会话后管理与关注均失效。
3. GitHub 登录、邮箱关联、另一个账号冲突拒绝；署名不公开邮箱，登录不自动订阅。
4. `target/debug/playtest --api http://localhost:48550 login` 在网页输入设备码、明确授权、回终端；匿名作品由原凭据接管。
5. 作品关注、合集投稿、公开页返回、管理页返回；确认信预览不消费令牌，退订不退出账号。
6. 窄屏、键盘焦点、空状态、失败与重试。桌面浏览器模拟窄屏不能声称手机真机已验收。

## 自动化测试

`cargo test --workspace` 覆盖契约、权限、发布和组件输出；`api/tests/accounts.rs` 和 `api/tests/login.rs` 覆盖账号与设备授权。测试替身不代替生产邮件、OAuth、实际域名和浏览器往返。带日期的真实观察另加到 docs/spikes，不修改旧记录。
