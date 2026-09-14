# 发布提示极简克制化、登录弹窗降噪与通知设置卡片式深度重构

日期：2026-09-14
分支：main
范围：`console`（发布弹窗与登录弹窗）、`edge`（广场发布弹窗、关注通知设置弹窗）、`ui`（通知与设置样式）

---

## 背景与真实体验反馈

在清空线上环境并由首位创作者进行真机端到端走查后，识别出 4 个明确影响产品质感与呼吸感的交互细节：

1. **CLI 安装说明冗长换行**：发布弹窗中的 Install CLI 提示由于罗列了路径、sudo 与各平台兼容性，在 400px~480px 的弹窗内折成 3 行，视觉沉重；
2. **静态+后端代理说明啰嗦**：`Serves static files as usual; unmatched requests forward to your local backend.` 同样折行，缺少工业克制感；
3. **登录弹窗底部标语冗余**：`One account for following and creating.` 作为口号式文案悬在表单底部，有说教感；
4. **通知设置（Notification Settings）粗糙且心智模糊**：
   - 视觉上只是简单的两端对齐文字与孤立按钮，邮箱与切换按钮首尾割裂；
   - 针对浏览器通知（Web Push），用户提出关键疑问：**“我们真的需要浏览器通知吗？它是没有打开网站也可以通知到吗？”**

---

## 落地改进与设计决策

### 1. 发布弹窗说明极简化（绝不折行）
- **Install CLI**：
  - 由长句精简为单行克制短语：`macOS & Linux · Installs to ~/.local/bin`；
  - 移除多余的平台罗列与说明，保持弹窗终端代码舱的纯粹焦点。
- **Static + Backend**：
  - 由长句精简为：`Proxies unmatched routes to your local backend.`；
  - 双端（控制台 `console/src/publish.tsx` 与广场原生弹窗 `edge/src/plaza.rs`）保持统一。

### 2. 登录弹窗降噪（彻底移除口号）
- 彻底移除 `console/src/auth.tsx` 与 `ui/account.html` 中的 `<p class="login-caption">One account for following and creating.</p>`；
- 使登录弹窗直奔核心行动（邮箱快速输入 / GitHub 一键连接），空间紧凑干练。

### 3. 通知设置（Notification Settings）卡片式重构与质感升级
- **身份卡片条（Identity Bar）**：
  - 如果用户已绑定邮箱，顶部呈现独立的黑曜石微光条：包含翠绿实心指示点、等宽字体脱敏邮箱，以及自然右对齐的 `Switch` 切换操作，消除原有的首尾碎屑感；
- **设置卡片组体系（Settings Group & Rows）**：
  - 将通知通道收纳进 `.notice-group` 卡片容器（黑曜石暗调底色、`1px solid rgba(255,255,255,0.08)` 边框、12px 圆角）；
  - **Weekly Digest（精选周报）**：
    - 左侧：标题 `Weekly Digest`，副标题 `Curated projects & updates`；
    - 右侧：若已订阅呈现带指示点的翠绿胶囊徽章 `Subscribed` + 幽灵退订按钮；未订阅提供一键订阅；未登录保留原生平滑折叠展开表单；
  - **Desktop Push（桌面系统通知）**：
    - 左侧：标题 `Desktop Push`，副标题明确说明 `System alerts when playtest is closed`（如实说明关闭网页由操作系统接收的特性）；
    - 右侧：如果已开启显示 `Active` 徽标与关闭选项；如果当前浏览器不支持则优雅提示；
- **克制注脚**：
  - 弹窗底部加入统一弱化说明：`No spam. Unsubscribe anytime with one click.`。

---

## 浏览器通知（Web Push）的技术机制与产品取舍探讨

针对用户的提问：**“另外就是我们真的需要浏览器通知么，或者这个浏览器通知是没有打开网站也可以通知到？”**

1. **它是如何工作的（“关掉网页也能收到通知吗？”）**：
   - **是的，在标准支持的环境下确实可以**；
   - 它基于 W3C Push API + Service Worker + 操作系统的通知守护进程（macOS 通知中心 / Windows 动作中心 / Android 系统通知）。用户在浏览器中授权后，服务器通过 VAPID 协议将加密推送发往浏览器的 Push Service（如 Apple APNs / Mozilla autopush / Google FCM）。即使浏览器标签页全部关闭，操作系统收到后台推送后，也会唤起系统横幅通知，点击直达作品。
2. **我们真的需要它吗？（产品取舍）**：
   - **现实困境**：
     - 大陆 Chrome 依赖 Google FCM，国内直连收不到；
     - iOS Safari 强制要求必须是 PWA（“添加到主屏幕”）后才允许申请权限；
     - 微信内置浏览器完全不支持 Web Push；
     - 频繁向用户索取通知权限极易引起反感和戒备。
   - **设计结论（符合 DESIGN §7.1）**：
     - **邮件周报（Email / Weekly Digest）才是当前最可靠、跨端体验最佳、无平台封锁的触达主渠道**；
     - Web Push 在本切片作为底层就绪但次级的可选能力，绝不喧宾夺主。我们在弹窗中把视觉重心完全留给「Weekly Digest」，对 Desktop Push 仅如实标注能力。

---

## 验证与真机上线记录

1. **前端编译与类型核验**：
   - `pnpm --prefix console build`：TypeScript 0 错误，打包生成静态资产；
2. **后端自动化测试**：
   - `cargo test -p playtest-edge --lib`：通过全量 205 项单元测试；
   - `cargo test -p playtest-edge --test discovery`：通过全部 6 项合集与发现页测试；
   - `cargo test -p playtest-api --test accounts`：通过全部 8 项账户与令牌测试；
   - `cargo test -p playtest-api --test collections`：通过全部 5 项合集业务测试；
3. **真机环境同步与部署**：
   - `TARGET=server deploy/push.sh`：服务器自动完成 Docker 增量构建并重启双容器；
   - `TARGET=console deploy/push.sh`：控制台静态资产装载完成；
   - 线上 Caddy 与 API/Edge 健康核验返回 `ok`，变更已全部在 `https://playtest.run` 生效。
