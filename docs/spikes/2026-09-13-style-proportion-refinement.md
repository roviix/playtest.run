# 2026-09-13 黑曜冷萃绿：尺寸、可读性与材质的渐进打磨

## 范围与取舍

本轮依据用户「保留黑曜冷萃绿，一点点改善」的要求，先补充 DESIGN §3.13 的尺寸落实标准，再修改现有样式。没有新增页面、依赖、发布步骤或账号能力，没有部署。

- 保留 `#09090b` 黑曜底、`#75cdb5` 冷萃绿、原创字标及原有封面构图。按钮共用暗绿灰纵向渐变和近距离阴影，卡片悬停去掉外发光。控制台原来的实心绿主按钮改用同一材质。
- 修复玩家侧桌面导航缺少高度、整卡与导航出现默认下划线的问题。主导航与主要按钮为 14px / 44px；窄屏不让导航文字拆行，作品数量仍可在内容标题里查看。
- 发布弹窗仍为 672px 上限，标题 18px、命令 15px、提示 14px、模式按钮 14px。去掉终端三色灯的视觉装饰和重复分隔线，减轻面板阴影；恢复面板自身纵向滚动，避免矮屏截断底部入口。
- 控制台身份栏封面由 132px 收到 112px、比例与作品墙统一为 16:10；标题 26px。身份、结果与设置阅读宽度上限 960px；辅助信息与标签恢复至少 12px。补齐此前缺失的 `--faint`、`--cool`、`--accent-soft`，避免状态和焦点样式退回继承值。
- 修复控制台发布命令继承普通命令框样式后出现额外 `$` 和代码字号缩小的问题。主按钮材质集中在基础样式，删掉覆盖层中的重复主按钮规则。
- 邀请函姓名框和窄屏控制台输入为 16px。文档正文、代码与辅助字级同步调整，保持现有章节和本地滚动区域。

## 本机浏览器证据

macOS Google Chrome，真实桌面浏览器；1280×900、375×812、320×740 / 568、740×320 均为视口模拟，使用 `innerWidth` / `innerHeight` 核对。本轮没有手机真机或微信验收。

- 原有本机边缘 `localhost:8443` 留作修改前对照；新构建在 `localhost:8446` 读取现有本机数据，实际打开广场、`/p/ivory-beaver-33` 邀请函与关注空态。姓名输入最终计算字号为 16px，开始按钮高 44px，375px 页面无横向溢出。未点击开始、发布、关注或发送邮件。
- `design_preview` 使用生产渲染函数生成 `/tmp/playtest-style-preview`，经本机 5284 端口检查满墙、长标题与空简介等虚构样本；这些不是用户实际作品。1280px 下三列，320px 下单列；导航与发布按钮计算高度均为 44px。
- 发布弹窗 1280px 下实测 672px 宽；切换到带后端服务后复制反馈显示「已复制命令」。Escape 返回发布入口。320px 下文档宽为 320px，长命令容器宽 256px、内容宽 312px，仅代码局部滚动。
- 移除样本全部 script 后，`document.scripts.length = 0`，仍可通过锚点打开发布说明并切换到后端命令。740×320 矮屏下弹窗位于 y=12、高 296px，内部可视高度 294px、内容高度 363px，底部内容可以滚动到达。
- 独立控制台 Vite 5285 连接仓库自带 mock API 8798，使用 `local-style-preview-not-a-credential` 模拟值，不读取真实令牌。检查作品墙、结果、设置、发布说明及使用文档。1280px 下身份栏宽 960px、页签 14px / 44px；320px 下导航保持单行，页面无横向溢出，设置输入计算字号 16px。文档命令计算字号 14px，表格在自己的区域滚动。
- mock API 没有版本管理列表接口，设置页对应区域如实显示「假控制面没有这条路径」；不把本轮样式观察记为版本管理功能验收，也没有操作回滚、删除或保存设置。

## 检查结果

- `pnpm --dir console build`：TypeScript 和 Vite 生产构建通过。
- `node scripts/check-console-docs.mjs`：6 个章节、路由与关键语义、17 条 CLI 用法检查通过；只解析帮助，没有发布作品。
- `cargo test -p playtest-edge --lib --test discovery --test social --test gate_hardlines --quiet`：187 + 6 + 20 + 6 = 219 项通过。
- `cargo build -p playtest-edge` 与 `design_preview` 生成通过。
- 卡页共享样式 6131 字节，工作区共享样式 21939 字节，分别低于现有 6.5KB / 23KB 上限；没有扩大预算。广场另有既有 discovery 样式，本轮该文件净增 46 字节。
- CSS 变量引用静态检查：未定义项只剩由页面提供的 `--p` 和有回退值的 `--docs-top`。
- `git diff --check` 通过。未提交，未将同一工作树中的存储、安全、路由等其他改动记为本轮成果。

## 截图

- [原桌面广场](img/2026-09-13-style-refinement-before-plaza.png)
- [原发布弹窗](img/2026-09-13-style-refinement-before-publish.png)
- [打磨后作品墙（虚构样本）](img/2026-09-13-style-refinement-gallery.png)
- [打磨后发布弹窗](img/2026-09-13-style-refinement-publish.png)
- [发布面板材质细节](img/2026-09-13-style-refinement-detail.png)
- [320px 发布说明](img/2026-09-13-style-refinement-publish-320.png)
- [最终本机邀请函](img/2026-09-13-style-refinement-invitation-final.png)
- [关注空态](img/2026-09-13-style-refinement-follow.png)
- [控制台结果（模拟数据）](img/2026-09-13-style-refinement-console.png)
- [320px 控制台导航与身份（模拟数据）](img/2026-09-13-style-refinement-console-320.png)
