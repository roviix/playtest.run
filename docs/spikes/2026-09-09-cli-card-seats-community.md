# 2026-09-09 · CLI：邀请卡、名额、群，以及「发完去哪看结果」（本机，对着假控制面 + 假边缘）

**结论**：`playtest ./dist` 发完之后打出来的那一屏，已经是 DESIGN §3.2 列的顺序：链接 →
二维码 → 邀请卡存到哪 → 广场状态 → 名额 → 匿名到期 → **「来的人玩成什么样，控制台里看得见」** →
没有封面的提醒 → 本次耗时。邀请卡真的从 `/_playtest/card.png` 下载并落到当前目录，
`--json` 里多了 `card_url` / `card_path` / `seats` / `plaza_url` / `console_url`，MCP 的返回里
多了一个 image 内容块。dogfood spike（`2026-09-08-dogfood-mofish-airdrop.md` 第四节第 4 条）
「发完没有一行告诉我怎么看结果」这条解掉了。

**没验的（重要）**：**没有对着真的控制面和真的边缘跑过**。这台机器上今天有别的会话在改
`api/` 和 `edge/`，而运行外部二进制这一步被本机的审批钩子挡住了（`cargo test` 可以跑，
直接运行 `target/*/debug/playtest-api`、`cargo run` 都被拦）。所以下面所有输出都来自
`cli/tests/` 里那个假服务器——**它同时扮控制面和边缘**：控制面的路由用
`playtest_common::api::routes` 的常量注册，边缘那一侧只有一个 `playtest_common::CARD_PATH`，
回 `Content-Type: image/png` 和一段以 PNG 魔数开头的字节。跑的是真的 `playtest` 二进制
（`CARGO_BIN_EXE_playtest`），真的走 HTTP，真的写文件。

---

## 一、发布成功那一屏（真二进制打出来的原文）

```
$ playtest <tmp>/dist --no-qr --seats 10 --community https://t.me/playtest --summary "三关，五分钟能玩完"

正在整理 /var/folders/…/dist：2 个文件，29 B
2 个文件服务器上都已经有了，不用传。
已发布《dist》v7

http://127.0.0.1:62423            ← 这一行在 stdout 上，其余都在 stderr

邀请卡已存到 ./dist-邀请卡.png——发到群里，别人长按识别就能玩
已放到广场上，标了「正在找人测」：http://0.0.1:62423/
想找 10 位试玩者，门禁页和邀请卡上都写着；留了名字的人算加入
这是匿名链接，2026-09-08 11:30 后失效。想让它留下来：playtest login。
来的人玩成什么样，控制台里看得见：https://playtest.roviix.com/console/#/s/brisk-otter-41
没有封面：加 --cover 一张图，广场和邀请卡都会好看很多
本次 0.0 秒
```

几处要说明的：

- **`http://0.0.1:62423/` 是假服务器造成的假象**，不是新 bug。广场地址由
  `upload::plaza_url` 从作品链接去掉最前面那一级算出来（`brisk-otter-41.playtest.run`
  → `playtest.run`），假服务器的作品链接是一个 IP，去掉「127」就成了这样。真作品链接
  是 `<slug>.<域名>`，算出来是对的。这一版没动它。
- 二维码没画：`ui::can_draw_qr()` 要求 stdout 和 stderr 都是终端，测试里是管道。
- 到期时间是假服务器给的一个过去的时刻，所以走的是「这是匿名链接」那一支；真快到期时
  换成「提醒：这条匿名链接只剩 …」，两句都带「匿名链接」四个字。
- 控制台地址用的是 `playtest_common::DEVELOPER_API_URL`，**不跟 `--api` 走**：本机跑测试时
  控制台并不在那个假地址上，指过去只会给出一条打不开的链接。代价是本机开发时这一行指向线上。

## 二、`--json`（同一次上传，`--json --no-qr`）

```json
{
  "ok": true,
  "action": "upload",
  "slug": "brisk-otter-41",
  "url": "http://127.0.0.1:62403",
  "title": "dist",
  "version": 7,
  "elapsed_ms": 7,
  "timings": { "hash_ms": 0, "prepare_ms": 3, "upload_ms": 0, "commit_ms": 0 },
  "expires_at": "2026-09-08T03:30:00Z",
  "card_url": "http://127.0.0.1:62403/_playtest/card.png",
  "card_path": "./dist-邀请卡.png",
  "console_url": "https://playtest.roviix.com/console/#/s/brisk-otter-41",
  "findings": [
    { "level": "note", "message": "没有封面：加 --cover 一张图，广场和邀请卡都会好看很多" }
  ]
}
```

加了 `--seats 10` 时多 `"seats": 10` 与 `"plaza_url"`、`"plaza"`；`--no-card` 时没有
`card_path`（`card_url` 照给）。字段的约定：

- `card_url` **总有**——卡由边缘按当前版本渲染，这个地址一直有效。
- `card_path` 只有真的存下来了才有。存的是相对当前目录的路径（`./…`），因为人要照着它找文件。
- `seats` 报的是**服务器认下来的那个数**（PATCH 的响应里的 `listing.seats`），不是命令行里写的那个。
  控制面还没实现这一项时门禁页上不会有「还差几位」，那这里也不该说「想找 10 位」。
- 卡的字节**不进 JSON**。几百 KB 的 base64 塞进 stdout 那一行，对着管道读的脚本会很难受；
  要图的是 MCP，那条路单独给（见下）。

`playtest card <slug> --json` 与 `playtest followers <slug> --json` 各自也是一个对象
（`"action": "card"` / `"followers"`），保住「`--json` 时 stdout 只有一个对象」这条。

## 三、MCP：邀请卡贴回对话

`tools/list` 现在是 DESIGN §3.2 说的五件事：`playtest_share_port`（还没上线，调了会如实失败）、
`playtest_upload`、`playtest_list`、`playtest_site`、`playtest_card`。

`playtest_card` 的真实返回（手写 JSON-RPC 客户端收到的原文，`data` 是那段假 PNG 的 base64）：

```json
{
  "id": 2,
  "jsonrpc": "2.0",
  "result": {
    "isError": false,
    "content": [
      {
        "type": "text",
        "text": "{\n  \"action\": \"card\",\n  \"card_url\": \"http://127.0.0.1:62434/_playtest/card.png\",\n  \"elapsed_ms\": 1,\n  \"ok\": true,\n  \"slug\": \"brisk-otter-41\",\n  \"title\": \"小球试玩\",\n  \"url\": \"http://127.0.0.1:62434\"\n}"
      },
      {
        "type": "image",
        "data": "iVBORw0KGgpub3QtcmVhbGx5LWEtcG5n",
        "mimeType": "image/png"
      }
    ]
  }
}
```

两条约定：**第一块永远是那个 JSON**（读结果的是程序，它按顺序取第一块），图排在后面；
**拿不到卡不算这次调用失败**——链接已经能玩了，那一块换成一句「稍后在 … 能拿到」。
`playtest_upload` 的返回是同样的形状。`playtest_site` 的 JSON 里多了 `console_url` 和
`site.followers`。

## 四、拿不到卡的时候说什么

边缘按清单渲染那张卡，而清单是刚提交的——两者之间有一小段时间差。所以取卡会重试，
最多 5 次、隔 400 ms；连不上（不是「还没好」）时只试 2 次，因为 400 ms 后多半还是连不上。
仍然拿不到就打这一行，不报错、不改退出码：

```
邀请卡稍后可以在 http://…/_playtest/card.png 拿到，或 playtest card brisk-otter-41
```

`--no-card` 时这一行一个字都不说。文件名从作品名来：去掉控制字符和 `/ \ : * ? " < > |`
（Windows 上这些根本不让用），超过 40 个字截断，清干净之后什么都不剩就退回 slug。
同名文件直接覆盖——这张卡就是这个作品此刻的样子，留着上一版的没有意义。

## 五、本地就拦下的输入

```
$ playtest ./dist --seats 99999 --json
{"ok":false,"code":"bad_input","message":"--seats 最多 500，你写了 99999。真要这么多人，分几批发更好组织。", …}   退出码 6

$ playtest ./dist --community qq://12345 --json
{"ok":false,"code":"bad_input","message":"--community 要一条 http:// 或 https:// 开头的链接，你给的是「qq://12345」。\n微信群、QQ 群没有网址的话，把群二维码传成一张图片放在网上，给那张图的地址。", …}   退出码 6
```

两条都在**连服务器之前**就拦下（上面那两次跑的 `--api` 指着 `127.0.0.1:1`，没有人在听）：
这是输入的问题，不该先把几十 MB 传上去再被退回来。

## 六、传上去的是什么（PATCH 的请求体，假控制面收到的原文）

`--seats 10 --community https://t.me/playtest` 变成一次 `PATCH /v1/sites/{slug}`：

```json
{ "public": true, "seeking": true, "seats": 10, "community_url": "https://t.me/playtest" }
```

- **`--seats` 蕴含「正在找人测」，从而蕴含上广场**：说了想找 10 位试玩者却不挂出去，
  那 10 个人不会自己出现。没给 `--seek` 的文案就只标一下，不替他编一句「想让你看什么」。
- **只给 `--community` 不上广场**：群是给已经点进来的玩家看的（DESIGN §3.3），
  不能因为填了个群号就把作品挂出去。这时候 `public` / `seeking` 都不带，只改 `community_url`。
- **`feedback_public` 一定不带**。这一轮 CLI 不暴露它，控制台里改（DESIGN §3.5）。

## 七、没验的、以及对控制面 / 边缘的假设

1. **没跑真控制面 / 真边缘**（原因见开头）。下一次能跑时要看三件事：卡在提交后多久能拿到
   （现在假设 5 次 × 400 ms 的窗口够）、真 PNG 的字节数（现在的假卡只有几十字节，没验过
   几百 KB 的图在 MCP 里贴回去是什么体验）、`PATCH` 的 `seats` / `community_url` 控制面认不认。
2. **`Listing.has_cover` 的语义**假设是「最新版本有没有封面」。CLI 据此打那一行提醒。
   旧控制面不返回 `listing` 时它默认 `false`，会**误报一次「没有封面」**——只是一句提醒，
   不改行为，先接受。
3. **隧道那条路的卡没验过**。代码里第一次连上会取一次卡、打那两行（隧道也有门禁页，
   DESIGN §3.4），但边缘会不会为隧道作品渲染 `/_playtest/card.png` 没有确认过。
   拿不到就是上面第四节那句「稍后可以在 … 拿到」，不影响链接能不能玩。
4. **MCP 的 `playtest_share_port` 仍然是「还没上线」的拒答**，所以那条路没有卡。
5. **MCP 里上传也会往当前目录写一个卡文件**（和命令行一样，路径在 `card_path` 里）。
   编辑器起 MCP server 时的工作目录通常是项目根目录；写不下去只是少一个文件，图照样贴回对话。
6. `--community` 生效之后**没有一行确认**。玩家在门禁页上看到「开发者的群」，开发者在
   CLI 这边看不到自己刚设的是什么——等控制台那一页做完再说，还是这一版就补一行，待定。

## 怎么复现

```
export CARGO_TARGET_DIR=target/cli-agent
cargo test -p playtest                       # 128 个单元测试 + 5 个集成套件
cargo clippy -p playtest --all-targets -- -D warnings
```

这一轮真跑过的：`cargo test -p playtest` 全绿（单元 128；`json_output` 22、`mcp` 6、
`upload_flow` 7、`tunnel_flow` 4、`inspect` 10）、`cargo clippy -p playtest --all-targets
-- -D warnings` 没有一条、`cargo fmt -p playtest -- --check` 干净。收尾时这台机器的磁盘被
别的会话占满过一次（一度只剩 117 MB），审批钩子也把 `cargo` 挡了一段时间，所以**最后那几处
只删了三行临时 `println!`、加了两段注释的改动没有再跑一遍全量测试**——`cargo fmt --check`
在最终这一版上是过的（rustfmt 能解析全部文件），但严格说这句「全绿」对应的是删那三行之前的树。

上面那些原文来自这几条用例：`cli/tests/json_output.rs` 的
`what_a_first_timer_sees_after_publishing_comes_in_one_useful_order`、
`an_upload_saves_the_invite_card_next_to_you_and_says_so_in_the_object`、
`seats_and_a_group_link_go_up_with_the_version`，以及 `cli/tests/mcp.rs` 的
`the_card_comes_back_as_an_image_the_assistant_can_hand_over`。
