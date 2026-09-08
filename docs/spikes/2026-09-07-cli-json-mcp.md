# 2026-09-07 · 机器模式与 MCP：`--json` 的形状、退出码、手写 JSON-RPC 走通四个工具

**结论**：`playtest --json` 与 `playtest mcp` 在本机成立。`--json` 模式下 stdout 只有一个 JSON 对象，
说明与进度全在 stderr；退出码按 `output::Code` 分层（本次实测到 0 / 2 / 6）。MCP 那条路不引任何客户端库，
用手写的 JSON-RPC 从 stdio 走 `initialize → tools/list → tools/call`，四个工具都应答，
**回给对话的 JSON 和 `--json` 是同一个形状**，上传时的检查结果（`findings`）两条路都带。
一次冷上传（7 个文件 45.1 KB，本机回环）**98 ms**；服务器已有全部文件时 **24 ms**；
从 MCP 调同一次上传 **16 ms**。

**没成立的**：`playtest_share_port`（隧道）按设计一定失败，本次验的是「它说清楚了自己没做好」，
不是它能用；没有用真实编辑器（Cursor / Claude Code）接过，`@modelcontextprotocol/inspector` 没装
（磁盘只剩 790 MB，见第六节），所以「Cursor 里能用」这句话现在**没有真机依据**，只有协议层面的依据；
只发了 `vite-vanilla` 一个导出物，其余三个见 `2026-09-07-e2e-upload-localhost.md`；
二维码只验到「原文进了 JSON、19 行」，没有手机扫过。

机器：macOS 26.2（Apple Silicon），rustc 1.96.0。全部命令从仓库根目录执行。
CLI 的配置写到 `HOME=/tmp/pt-mcp-home`（干净的一台「新机器」），没有动 `~/.config/playtest/config.json`。

## 一、起两个进程

端口用 28787 / 28443 而不是默认的 8787 / 8443，数据目录另开一个，免得和别的调试撞上：

```
$ PLAYTEST_DATA_DIR=.data/mcp PLAYTEST_API_LISTEN=127.0.0.1:28787 \
  PLAYTEST_SITE_URL_TEMPLATE='http://{slug}.localhost:28443' cargo run -q -p playtest-api
INFO 数据库迁移已应用 version=1
INFO 数据库迁移已应用 version=2
INFO playtest 控制面已启动：http://127.0.0.1:28787
INFO 数据目录：.data/mcp
INFO 玩家链接：http://{slug}.localhost:28443

$ PLAYTEST_DATA_DIR=.data/mcp PLAYTEST_EDGE_LISTEN=127.0.0.1:28443 cargo run -q -p playtest-edge
INFO 边缘在 http://127.0.0.1:28443 上（明文，没有 TLS）
INFO 作品从 .data/mcp/store 读，事件写到 .data/mcp/edge-events.jsonl

$ curl -s http://127.0.0.1:28787/healthz; curl -s -H 'Host: localhost' http://127.0.0.1:28443/_playtest/healthz
ok
ok
```

## 二、`--json` 上传：一个对象，两处耗时

第一次（服务器什么都没有）：

```
$ playtest --json ./fixtures/vite-vanilla/export --api http://127.0.0.1:28787
```

stdout（一行，这里为了看得清换了行）：

```json
{"ok":true,"action":"upload","slug":"glad-fox-76","url":"http://glad-fox-76.localhost:28443",
 "version":1,"elapsed_ms":98,"timings":{"hash_ms":2,"upload_ms":56,"commit_ms":26},
 "expires_at":"2026-09-08T08:47:09Z","qr_text":"…19 行，█▀▀▀▀▀█ 开头…",
 "findings":[{"level":"note","message":"看起来是 Vite 做的"}]}
```

stderr：

```
正在整理 ./fixtures/vite-vanilla/export：7 个文件，45.1 KB
需要上传 7 个文件（45.1 KB），其余 0 个服务器上已有
```

**98 ms**，分段 哈希 2 · 上传 56 · 提交 26。三段加起来 84，差的 14 ms 是建匿名会话与建作品那两次往返
（`elapsed_ms` 从进程启动算起，`timings` 只是其中三段，这个差是设计里就有的）。

第二次（同一个目录，加 `--no-qr`）：

```json
{"ok":true,"action":"upload","slug":"glad-fox-76","url":"http://glad-fox-76.localhost:28443",
 "version":2,"elapsed_ms":24,"timings":{"hash_ms":1,"upload_ms":0,"commit_ms":14},
 "expires_at":"2026-09-08T08:47:09Z","findings":[{"level":"note","message":"看起来是 Vite 做的"}]}
```

`upload_ms` 是 0——七个文件服务器上都已经有了，一个字节都没重传（stderr：`7 个文件服务器上都已经有了，不用传。`）。
`--no-qr` 之后 `qr_text` 这个字段直接不出现，不是空字符串。

链接是真的能打开的（不是只拿到一个字符串）：

```
$ curl -s -o /dev/null -w '%{http_code}\n' -H 'Host: glad-fox-76.localhost' -H 'Accept: text/html' http://127.0.0.1:28443/
200                                   ← 门禁页
$ curl -s -o /dev/null -w '%{http_code}\n' -H 'Host: glad-fox-76.localhost' -H 'Accept: text/html' -H 'Cookie: pt_gate=1' http://127.0.0.1:28443/
200                                   ← 作品本身
```

三个版本共用同一批 blob，对象存储里只有 7 个（`.data/mcp` 全部 428 KB）：

```
$ find .data/mcp/store -type f | sed 's|/[0-9a-f]\{64\}$|/<hash>|' | sort | uniq -c
   1 .data/mcp/store/blobs/14/<hash>   …（共 7 个）
   1 .data/mcp/store/sites/glad-fox-76/current.json
   1 .data/mcp/store/sites/glad-fox-76/manifests/1.json
   1 .data/mcp/store/sites/glad-fox-76/manifests/2.json
   1 .data/mcp/store/sites/glad-fox-76/manifests/3.json
```

## 三、`--json` 的其余形状与退出码

```
$ playtest --json ls --api http://127.0.0.1:28787
{"ok":true,"action":"list","elapsed_ms":8,"sites":[{"slug":"glad-fox-76",
 "url":"http://glad-fox-76.localhost:28443","title":"export","version":2,
 "expires_at":"2026-09-08T08:47:09Z"}]}                                        exit 0
```

四种失败／未做好，退出码各自分开：

```
$ playtest --json ./没有这个目录
{"ok":false,"code":"bad_input","message":"找不到 ./没有这个目录。检查一下路径；如果还没构建，先构建出这个目录。","elapsed_ms":2}
exit 6

$ playtest --json login
{"ok":false,"code":"not_implemented","message":"登录还没做好。现在每次运行拿到的是匿名链接，24 小时后失效。","elapsed_ms":2}
exit 2

$ playtest --json 5173
{"ok":false,"code":"not_implemented","message":"隧道模式（把本地端口接出去）还没做好，现在只支持上传目录，例如：playtest ./dist","elapsed_ms":2}
exit 2

$ playtest --json ls --api http://127.0.0.1:1
{"ok":true,"action":"list","elapsed_ms":7,"sites":[]}
exit 0
```

最后一条值得说明：`--api` 指到一个没人监听的端口，结果不是「网络不通」而是**空表加成功**。
因为令牌是按控制面地址存的，那个地址上没有令牌就等于「没在那儿发过东西」，问「我有什么」的答案是「没有」，
一个网络请求都不该发。真发过东西再断网才是退出码 4。

`playtest --json login` 这条同时验了 `args.rs` 去掉 `args_conflicts_with_subcommands` 之后的行为：
`login` 被当成子命令，不是一个叫 login 的目录。

## 四、手写 JSON-RPC 走 MCP

不引 MCP 客户端库——要验的正是「别人家的客户端接上来能不能用」，用同一个 SDK 两头对拍证明不了这件事。
六行 JSON 一次喂给 stdin：

```
$ printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"handwritten","version":"0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"playtest_list","arguments":{}}}' \
  '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"playtest_upload","arguments":{"dir":"./fixtures/vite-vanilla/export","name":"Vite 计数器","note":"从对话里发的"}}}' \
  '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"playtest_share_port","arguments":{"port":5173}}}' \
  | playtest mcp --api http://127.0.0.1:28787 > out 2> err
```

stdout 收到 5 行、5456 字节，**每一行都是合法 JSON**（stdout 是协议通道，混进一个字节的日志真客户端就解析失败了）。
stderr 上是给人看的那几句：

```
playtest 正在以 MCP server 的方式运行，等编辑器接进来（stdout 只走协议，说明都在这条流上）。
正在整理 ./fixtures/vite-vanilla/export：7 个文件，45.1 KB
看起来是 Vite 做的
7 个文件服务器上都已经有了，不用传。
MCP 连接结束：Closed
```

回应的 id 顺序是 `[1, 2, 5, 3, 4]`——工具调用是并发处理的，先做完的先回。按 id 配对是客户端的事，
这里记一笔是因为顺着读会以为丢了消息。

### initialize（id 1）

```json
{"protocolVersion":"2025-06-18","serverInfo":{"name":"playtest","version":"0.1.0"},
 "capabilities":{"tools":{}},
 "instructions":"playtest puts a playable build in front of specific people. Call playtest_upload with the directory your build step prod…"}
```

`version` 与 `CARGO_PKG_VERSION` 一致（`cli/tests/mcp.rs` 里那条握手测试比的就是这个值）。

### tools/list（id 2）

```
playtest_list        required 无        参数 无
playtest_share_port  required ["port"]  参数 port
playtest_site        required ["slug"]  参数 slug
playtest_upload      required ["dir"]   参数 dir / isolated / name / note
```

每条说明都是英文一句加中文一句（模型读英文，人在配置界面里读中文），集成测试里有一条专门盯这个。

### tools/call playtest_list（id 3）

`isError: false`，正文就是 `--json ls` 那个对象：

```json
{"ok":true,"action":"list","elapsed_ms":2,
 "sites":[{"slug":"glad-fox-76","url":"http://glad-fox-76.localhost:28443","title":"export",
           "version":2,"expires_at":"2026-09-08T08:47:09Z"}]}
```

### tools/call playtest_upload（id 4）

`isError: false`，字段和 `--json` 完全一样，一个不多一个不少：

```json
{"ok":true,"action":"upload","slug":"glad-fox-76","url":"http://glad-fox-76.localhost:28443",
 "version":3,"elapsed_ms":16,"timings":{"hash_ms":1,"upload_ms":0,"commit_ms":10},
 "expires_at":"2026-09-08T08:47:09Z","qr_text":"…19 行…",
 "findings":[{"level":"note","message":"看起来是 Vite 做的"}]}
```

`elapsed_ms: 16` 是**这一次工具调用**的耗时，不是进程活了多久——MCP server 从编辑器早上启动起就开着，
报进程寿命没有意义。这个改写在 `mcp.rs` 里做（`report.elapsed_ms = output::ms_since(started)`）。

### tools/call playtest_share_port（id 5）

`isError: true`，正文是失败对象：

```json
{"ok":false,"code":"not_implemented",
 "message":"隧道路径还没上线，接不了本地端口 5173。现在能做的是把构建好的目录发出去：先跑一次构建，再调 playtest_upload。",
 "elapsed_ms":0}
```

用 `CallToolResult::error` 而不是 JSON-RPC 协议错：协议错在多数客户端里只显示「内部错误」，
上面这句话对面根本看不到。这条工具存在的理由就是这个——见第五节。

## 五、`playtest mcp --setup`

```
$ playtest mcp --setup --api http://127.0.0.1:28787
```

stdout（可以整段粘走）：

```json
{
  "mcpServers": {
    "playtest": {
      "args": ["mcp", "--api", "http://127.0.0.1:28787"],
      "command": "/Users/zhongshangwu/workspace/github/playtest.run/target/debug/playtest"
    }
  }
}
```

stderr（粘的时候不会跟着进去）：

```
把下面这段放进编辑器的 MCP 配置里：
  Cursor：这个项目用 .cursor/mcp.json，所有项目都用 ~/.cursor/mcp.json
  Claude Code：项目根目录的 .mcp.json，或者 claude mcp add playtest -- <上面的 command 和 args>
已经有别的 server 的话，只把 "playtest" 那一项加进 mcpServers 里。

装好之后在对话里说「把 ./dist 发出去」，助手会调 playtest_upload 并把链接贴回来。
```

`command` 是当前二进制的绝对路径，不是 `playtest`：编辑器起 MCP server 时的 PATH 常常和终端里的不一样，
写死路径能省掉一整类「为什么它说找不到命令」。

## 六、环境与没做的事

- **`@modelcontextprotocol/inspector` 没装**：磁盘只剩 790 MB（不是预估的 4 GB），装它要拉一整棵 npm 依赖树。
  手写 JSON-RPC 覆盖的是同一个协议面，但**「在真实编辑器的 UI 里点一下能用」这件事本次没有验**。
- 本次全程用 `cargo run -q -p …` 启动进程（直接执行 `./target/debug/…` 在本次会话里被拦），
  `-q` 时 cargo 不往 stdout 写东西，所以不影响「stdout 只有协议」这个结论——5 行全是合法 JSON 就是证据。
- 边缘记下 2 条事件（`gate_view` + `html_view`，UA 是 curl），第一层数据是通的。
- 收尾：28787 与 28443 两个进程都已停掉（`curl` 两个 healthz 都连不上），`cargo test -p playtest`
  **113 条全绿**（77 单元 + 10 `config` + 14 `json_output` + 4 `mcp` + 8 `upload_flow`），编译零警告。
