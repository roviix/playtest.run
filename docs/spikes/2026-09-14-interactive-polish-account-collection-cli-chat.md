# 交互设计与体验深度打磨（个人中心、合集返回与标题、CLI 一键 curl 安装、聊天消息正序）

日期：2026-09-14
分支：main
范围：`console`（个人中心、合集、发布、文档）、`edge`（门禁聊天渲染、广场发布弹窗、合集上下文）、`deploy`（一键安装脚本）

---

## 背景与动机

根据用户对当前产品交互与体验的直接反馈，系统性打磨 4 个关键环节：

1. **个人中心（Account / Profile）过往粗糙生硬**：原有个人中心为极简且缺乏样式的表单堆叠，缺乏现代 Creator Dashboard 的质感、层级与即时反馈；
2. **合集详情页返回按钮与合集名称布局失调**：控制台合集详情页中，「← 我的合集」作为孤立文字链接漂浮在顶部，与下方合集徽标、标题、slug 与右侧操作按钮缺乏整合；邀请函中的合集返回也是裸字符排版；
3. **CLI 下载跳转 GitHub 阻断操作流程**：发布弹窗与文档过去引导用户跳转至外部 GitHub Releases 手动寻找架构包，用户希望像 oh-my-zsh、Bun 一样直接提供一键可运行的 `curl` 脚本指令；
4. **原声聊天历史顺序倒置**：数据库按 `ts DESC`（最新在前）查询出反馈数据，服务端渲染直接正向输出导致最新消息挂在最顶端，而新发送的消息却追加在最底端，违背通用聊天心智。

---

## 落地与改进细节

### 1. 个人中心现代化卡片式仪表盘重构 (`console/src/pages/token.tsx` & `auth.css`)
- **创作者展台 (Hero Profile Card)**：
  - 呈现 58px 高清微光头像与单字母冷翡翠渐变徽标兜底；
  - 创作者展示名、`Creator` 绿色微标、GitHub `@handle` 统一编排；
  - 右侧提供「关注作品与动态（`/me`）」与「退出登录」等高频行动。
- **公开资料卡片 (Public Profile Card)**：
  - 展示名就地编辑与即时保存，附带 `0/40` 字符动态计数器与聚焦光晕；
  - 仅在修改且有效时激活「Save Name」，保存成功后提供平滑状态反馈。
- **登录方式与身份连接 (Sign-in Methods Card)**：
  - 邮箱与 GitHub 采用分立卡片网格，明确显示已绑定（带绿色状态点）与未关联状态；
  - 强化单账号心智提示：「双通道均汇聚至此唯一账户」。
- **开发者令牌管理 (Developer Settings Card)**：
  - 废除原生 `<details>`，改为现代卡片结构；
  - 提供一键「Generate New Token」；生成后通过深墨绿色高亮安全工作区展示令牌，支持点击复制；
  - 令牌列表显示创建时间与「Active」徽标，撤销操作提供二次安全确认。

### 2. 合集详情返回与标题布局重整 (`console/src/pages/collections.tsx`、`collections.css` & `ui/invitation.css`)
- **控制台合集面包屑导航**：
  - 引入 `<nav class="collection-breadcrumb">`，左侧带有 SVG 矢量回退箭头的「我的合集」链接，与当前合集名称形成自然面包屑层级；
- **合集 Hero 标题与 Slug 规范**：
  - 合集名称 `h1` 与徽标、slug 徽章横向自然对齐，slug 增加等宽微底色框，支持一键双击选中文本；
  - 右侧「打开玩家页面 ↗」与「复制链接」与标题行基线平齐，消除响应式下的拥挤换行；
- **邀请函合集上下文优化**：
  - 邀请函顶栏返回链接采用内联 SVG 箭头与截短保护，防止长合集标题破坏顶栏平衡。

### 3. CLI 一键 curl 安装指令集成 (`console/src/publish.tsx`、`edge/src/plaza.rs`、`deploy/site/install.sh` & `docs.tsx`)
- **发布弹窗集成一键安装**：
  - 控制台与广场的发布弹窗均增加「Install CLI」标签与指令面板，直接显示：
    ```bash
    curl -fsSL https://playtest.run/install.sh | bash
    ```
  - 配套终端圆点图标、一键复制按钮与 Copied 反馈；
  - 弹窗底部「Install CLI (curl)」按钮就地切换至安装面板，不再跳转至外部 GitHub。
- **安装脚本自动探测增强 (`deploy/site/install.sh`)**：
  - 当未指定 `PLAYTEST_VERSION` 时，脚本自动通过 GitHub Releases 探测最新版本，并以已知稳定版本作为优雅兜底，使普通用户运行 `curl ... | bash` 即可无缝安装。
- **文档显要位置同步 (`console/src/pages/docs.tsx`)**：
  - 第一步安装指南优先呈现一行命令一键安装。

### 4. 聊天历史正序与自动触底滚动 (`edge/src/gate.rs`、`edge/src/app.rs`、`ui/dialog.js` & `ui/presentation.js`)
- **服务端渲染正序反转**：
  - 服务端提取 `public_feedback` 后执行 `items.reverse()`，使时间最久的消息在最上方，最新发布的消息在最下方；
- **客户端轮询保持一致**：
  - `/v1/feedback?chat=1` 接口返回的消息列表同步保持时间正序；
- **进入聊天自动触底滚动**：
  - 页面初次加载、通过 hash 打开或点击反馈按钮唤起面板时，自动平滑滚动至容器底部（`stream.scrollTop = stream.scrollHeight`），确保用户第一时间聚焦最新讨论。

---

## 验证与测试

1. **控制台前端构建**：
   - `pnpm --prefix console build`：TypeScript 0 错误，Vite 编译打包顺利输出；
2. **Rust 后端单元测试**：
   - `cargo test -p playtest-edge --lib other_players_words_are_shown_but_never_discussed`：通过，断言验证 HTML 输出中老消息在先、新消息在后；
   - `cargo test -p playtest-edge --lib plaza::tests`：通过全部 14 项广场与发布弹窗测试；
   - `cargo test -p playtest-edge --test discovery`：通过全部 6 项合集与发现页测试；
   - `cargo test -p playtest-api --test collections`：通过全部 5 项合集测试；
   - `cargo test -p playtest-api --test accounts`：通过全部 8 项账户测试。
