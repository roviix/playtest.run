# 2026-09-11 重写第一批：一个原语、CLI 收成一份实现、给助手的四个文件

`docs/REWRITE.md` 起草之后的第一轮施工。三件事落地：契约层收成一个 `Project`，CLI 的人话与
`--json` 收成一份实现，控制面开始自己出 agent 能读的文件。全程 `cargo test --workspace` 与
`cargo clippy --workspace --all-targets -- -D warnings` 保持绿。

## 一、开工前那四条红测试

工作区里有 8,257 行未提交的「方向 B」改动，`cargo test --workspace` 是红的——三份 09-09 的
spike 都写着「没有对着真的控制面和边缘跑过」，四个会话按 crate 分层并行、从没整合过。

两个真问题，都在层与层的接缝上：

1. `api/tests/club.rs` 的 `session_id(seed)` 造的是 `"hhhh…"` 32 个字符，而
   `ingest::is_session_id` 要求 32 个**十六进制**字符。夹具造的会话 id 门禁页永远不会种出来。
   改成 `format!("{:032x}", seed as u32)`。
2. `api/tests/follow_boost.rs` 要求通知正文里带 `from=notice`，而正文是开发者写的那句话，
   链接在 `notifications.url` 那一列上——邮件由 `notify::render` 把两者拼起来，推送把链接放进
   负载。断言搬到 url 那一列。

另外两条 clippy 警告（测试里的 `from_edge` 撞 `wrong_self_convention`）会让 CI 的 `-D warnings`
挂，一并改名。

## 二、一个原语

`common/src/project.rs`：`Project` 六组属性（身份 / 交付 / 呈现 / 访问 / 上架 / 社会），
`ProjectCard` 与 `ProjectLive` 是它的投影，`api/src/project.rs::compose` 是控制面**唯一**的
拼装点。原来 `live.rs`、`plaza.rs`、`routes/sites.rs` 各读各的列、各填各的字段。

计划里写的是四个投影，实际做成三个：`ProjectCard`（控制台作品墙）和 `ProjectPublic`
（`plaza.json` 的一项）是同一张卡，合成一个——广场和控制台用同一个类型，同一个作品在两个
地方才会说同一句话。

两处「判断」也跟着收进契约，因为边缘和控制台原来各判一次：

- `ProjectCard::fact()`——卡底右边那**一件事实**（名额 / 还剩多久 / 几人玩过 / 多久以前）。
- `ProjectCard::blurb()`——作品名下面那一行说哪一句（在找人测就说这一版的话，否则说简介）。

边缘只剩「把它排成 HTML」和「拿现在去算时间」。

### 没做的：换数据库

REWRITE §9.5 原本写着换 Postgres。写到这里改了主意，理由记在 §9.5：真正的缺陷是那把单连接
互斥锁而不是 SQLite，而这台机器上既没有 Postgres 也没有跑着的 Docker——换过去等于写一份
跑不了也测不了的代码。**这条还没做**：单锁换 WAL 连接池、`db.rs` 按聚合切分，都还在 §6 的
M0 里挂着。

对象存储的目录名也仍是 `sites/`，没改成 `projects/`，理由同 §9.6。

## 三、CLI 收成一份实现

`report.rs`（结果是什么）+ `commands.rs`（怎么做到）+ `session.rs`（本机状态）取代了
`sites.rs` 与 `output.rs` 里两套平行实现。

修掉的契约洞——**这是这一轮最实在的一个 bug**：

```
$ playtest rollback <slug> v3 --json
https://<slug>.playtest.run        ← stdout 上一条裸链接，不是 JSON
$ playtest versions <slug> --json
                                   ← stdout 空的
$ playtest unlist <slug> --json
                                   ← stdout 空的
```

`ls / rm / open / card / followers` 在两个文件里各写了一遍，`versions / rollback / unlist`
只写了人话那一半，而 `cli/tests/json_output.rs` 里没有这三条的任何断言。现在每条命令产出一个
`Report`，`output::say` 是唯一的渲染入口，做不到「只实现一半」。新增的
`every_command_answers_in_one_object_in_machine_mode` 把这三条钉住。

参数面按 REWRITE §3.1 收敛（`--seek` 并进 `-m`、`--isolated` 三态、`--card -`、`--to`、
`-y`、`followers` 并进 `ls`、`--api` 提到全局），DESIGN §3.2 已改。

### MCP 里的隧道：低报也是失真

`playtest_share_port` 的工具描述硬写着 `NOT AVAILABLE YET — this call always fails`，而 CLI
的隧道早就能用、有 601 行集成测试；`cli/tests/mcp.rs` 还有一条测试把这句话钉死。

改成 `playtest_share`：`tunnel::run_with` 多一个 `oneshot::Sender<Online>`，连上那一刻发一次；
MCP 把隧道 spawn 到后台、等这一声、立刻回答，`JoinHandle` 存在 server 状态里（**不能 detach**
——落地之后没人持有它，tokio 会把任务收走，链接当场断）。回答里带 `"lasts": "while this MCP
server is running"`，助手不会把它当永久链接转述。

## 四、给助手的四个文件

`/llms.txt`、`/llms-full.txt`、`/skill.md`、`/openapi.json`、`/.well-known/agent.json`，
由控制面按 `common::api::routes` 的路径常量生成，测试盯着端点表与 OpenAPI 两边对齐。
玩家域根上加一张 `/llms.txt` 字条指向开发者域。

起因是 2026-09-10 一个助手把 playtest.run 判成「不对口」。**它判断得对**——我们确实不是通用
静态托管——但它是从首页猜的。所以 `llms.txt` 里最要紧的一段是「什么时候别用我们」，
点名了报告 / 仪表盘 / 文档站这一类，并指出没有自定义域名、不跑服务端代码。

## 五、真的跑起来看过

控制面起在 `127.0.0.1:8791`（`PLAYTEST_DATA_DIR=/tmp/pt-agent`、`PLAYTEST_EMAIL_PROVIDER=log`），
`curl` 拿到 `/llms.txt` 全文，内容与预期一致（截图不必，它是纯文本，内容由
`api/tests/agents.rs` 九条断言盯着）。

其余全部是自动化的：

| 层 | 命令 | 结果 |
|---|---|---|
| 全仓 | `cargo test --workspace` | 全绿 |
| 全仓 | `cargo clippy --workspace --all-targets -- -D warnings` | 无输出 |
| 全仓 | `cargo fmt --all` | 已应用 |
| 控制台 | `npx tsc --noEmit` | 无错 |

## 六、顺手删掉的死路

`POST /v1/sites/{slug}/cover/from-feedback/{id}` 整条链（控制面 handler、契约里的路径常量与
helper、`console` 的 wrapper 与那个按钮、`FeedbackItem.screenshot_hash` 字段）。守卫条件是
「这条反馈带截图」，而截图是 v0.2 的事、那一列永远写 NULL——三层代码没有任何客户端能触达。
截图做出来的时候和它一起加回来，比留一个永远拒绝的端点诚实。

CLI 的 `Code::NotImplemented` 与 `not_implemented()` 也一起删了：隧道上线之后，CLI 里已经
没有「还没做好」的分支。

## 七、新增能力

- `GET /v1/sites/{slug}/versions/{version}/files` 与 `playtest files <slug>`：线上这一版到底
  有哪些文件（路径、大小、sha256、直接能打开的地址）。换了电脑、或者一个 agent 接手别人发的
  作品时，这是「现在线上是什么」唯一不用重新构建就能回答的路子。here.now 有这个，我们缺。
- `playtest whoami`：这个令牌是谁、几个作品、匿名身份什么时候到期。

## 八、没验的

- **真机、真手机、真微信一次都没碰。** 这一轮全部在本机，香港边缘没有重新部署。
  方向 B 的那条主线（卡 → 扫码 → 留名 → 关注 → 第二版 → 收到信）仍然没有一条端到端记录，
  `scripts/e2e.sh` 还没写——它是 REWRITE §6 M0 的交付项，这一轮没做到。
- `playtest_share` 只验了「端口上没东西时说人话」这一支；**真的接出一个 dev server 并从
  另一台设备打开，没试过**。
- `playtest files` 的控制面那一端有集成测试，CLI 那一端只有 `Report` 的单元测试，
  没有对着假控制面跑过。
- 边缘的渲染层、缓存层、`upstream` 收敛（REWRITE §5.4）一行没动；两套 CSS 还在打架，
  五套同形缓存还在，`sites.rs` 的清单缓存仍然无上限无 TTL。
- 控制面 6 个后台循环还是 6 个；`db.rs` 还是 1,713 行一个文件。
- i18n 文案表（REWRITE §5.2）没开工，CLI 与门禁页仍然只有中文；给助手的那几个文件是英文。
