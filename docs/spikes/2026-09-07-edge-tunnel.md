# 2026-09-07 · 边缘这一头的隧道：握手、会话登记、按 Host 反代（本机）

把 DESIGN §4.3 里边缘负责的那一段接上：CLI 的出站 WebSocket 进来 → 验签 → 变成 yamux 通道 →
玩家的每个请求开一条流走到开发者机器上。下面每一条都是实际执行过的命令和实际看到的输出。

CLI 那一头这时候还没有，所以「完整链路」是在集成测试里用 `common` 的 `handshake_request` +
`tokio_tungstenite` + `Mux::spawn(_, Role::Acceptor)` 伪造出来的，形状和真 CLI 一样，只是没有重连退避。
手工那一段只打了握手的拒绝分支——签一个真令牌需要控制面的私钥，curl 造不出来。

## 1. 测试

```
$ cargo test -p playtest-edge
     Running unittests src/lib.rs
running 90 tests
test result: ok. 90 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/breaker.rs
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
     Running tests/gate_hardlines.rs
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
     Running tests/serving.rs
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.84s
     Running tests/tunnel.rs
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
```

隧道自己的 22 条单测：

```
$ cargo test -p playtest-edge --lib tunnel::
test tunnel::keys::tests::reads_the_file_and_notices_when_it_changes ... ok
test tunnel::keys::tests::a_file_that_is_not_a_key_is_not_a_key ... ok
test tunnel::registry::tests::a_new_tunnel_pushes_the_old_one_out ... ok
test tunnel::registry::tests::a_departing_old_session_cannot_kick_the_new_one ... ok
test tunnel::registry::tests::going_offline_leaves_something_for_the_offline_page ... ok
test tunnel::registry::tests::the_synthetic_manifest_says_what_the_token_says ... ok
test tunnel::registry::tests::a_slug_that_is_not_a_slug_never_becomes_a_path ... ok
test tunnel::handshake::tests::only_a_bearer_token_counts ... ok
test tunnel::handshake::tests::the_local_port_must_be_a_port ... ok
test tunnel::handshake::tests::tokens_in_a_header_are_matched_case_insensitively_across_a_list ... ok
test tunnel::proxy::tests::the_host_is_rewritten_and_hop_by_hop_headers_are_dropped ... ok
test tunnel::proxy::tests::upstream_hop_by_hop_headers_do_not_reach_the_player ... ok
test tunnel::proxy::tests::an_upgrade_request_keeps_the_two_headers_that_make_it_one ... ok
test tunnel::proxy::tests::what_counts_as_an_upgrade_request ... ok
test tunnel::proxy::tests::the_concurrency_limit_is_the_tier_quota ... ok
test tunnel::offline::tests::says_who_what_and_when ... ok
test tunnel::offline::tests::everything_from_the_token_is_escaped ... ok
test tunnel::offline::tests::a_time_we_cannot_read_never_reaches_the_player ... ok
test tunnel::offline::tests::the_other_three_pages_are_complete_html ... ok
test tunnel::tests::the_handshake_path_is_the_one_common_declares ... ok
test tunnel::tests::handshake_errors_are_machine_readable ... ok
test tunnel::tests::only_a_document_navigation_gets_the_gate_page ... ok
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 68 filtered out; finished in 0.03s
```

`edge/tests/tunnel.rs` 里三个真的进程角色同时跑：边缘 Router（`axum::serve` 绑 `127.0.0.1:0`）、
一个当 dev server 的 axum（也绑 0 端口，`/`、`/game.wasm`、`/big` 2 MiB、`/echo` WebSocket 回显）、
一个伪 CLI。玩家用 hyper 的 HTTP/1.1 客户端打，Host 写 `brisk-otter-41.localhost`。七条：

| 测试 | 断的是什么 |
|---|---|
| `a_player_plays_what_is_running_on_the_developers_machine` | 门禁页在版本位置写「在线」且不含 `v0`；`POST /_playtest/start` 之后 `GET /` 出的是上游那份 HTML 且**字节完全相同**；dev server 把 `game.wasm` 说成 `application/octet-stream`，边缘换回 `application/wasm`；2 MiB 过一条 yamux 流后内容一致；`ws://边缘/echo` 回显成功；长连接开着时 `open_streams == 1`，玩家关掉后归零 |
| `a_second_process_takes_over_and_the_old_token_is_finished` | 第二个 CLI 用新令牌连上后，第一个的 `mux.closed()` 完成；第一个拿旧令牌重连拿到 409、body 的 `code` 是 `tunnel_replaced` |
| `the_offline_page_remembers_who_was_here` | 没连过是 404；连上再断开变 503 离线页，含开发者名、作品名和「上次在线」；`<DATA_DIR>/tunnels/<slug>.json` 落了盘 |
| `an_isolated_site_keeps_its_cross_origin_headers_through_the_tunnel` | 令牌里 `isolated: true` 时上游响应带上 COOP / COEP / CORP |
| `a_dead_dev_server_is_a_rendered_502` | CLI 报了一个没人监听的端口，玩家拿到的是一整页 502，不是裸状态码 |
| `the_handshake_says_why_it_said_no` | 七个拒绝分支各自的状态码与 `ErrorBody.code` |
| `without_a_verifying_key_the_edge_says_so_instead_of_guessing` | 公钥不在时 503；**不重启进程**、把公钥文件写进去之后同一个握手就能验签 |

2 MiB 那条是故意的：yamux 单流初始接收窗口 256 KiB，比它大才能把流控真的走一遍。

## 2. 手工：握手的拒绝分支

```
$ mkdir -p .data/tunnel-edge
$ PLAYTEST_EDGE_LISTEN=127.0.0.1:8446 PLAYTEST_DATA_DIR=.data/tunnel-edge cargo run -p playtest-edge
2026-09-07T10:24:21.438654Z  INFO 边缘在 http://127.0.0.1:8446 上（明文，没有 TLS）
2026-09-07T10:24:21.438815Z  INFO 作品从 .data/tunnel-edge/store 读，事件写到 .data/tunnel-edge/edge-events.jsonl
2026-09-07T10:24:21.438824Z  INFO 一个作品就是一个 http://<slug>.localhost
2026-09-07T10:24:21.438841Z  WARN .data/tunnel-edge/store 还不存在——api 往里写过东西之后作品才打得开
```

下面四条的 Host 都是 `brisk-otter-41.localhost`，这个 slug 从来没上传过东西——**握手不看作品状态**，
它在保留路径上，先于 `resolve` 处理。

普通 GET，一个升级头都没有：

```
$ curl -s -w '%{http_code}\n' -H 'Host: brisk-otter-41.localhost' \
    http://127.0.0.1:8446/_playtest/tunnel
400
{"code":"invalid","message":"这个地址只接受 WebSocket 握手：GET、Upgrade: websocket、Sec-WebSocket-Version: 13、Sec-WebSocket-Key"}
```

形状对了但没报子协议名：

```
$ curl -s -w '%{http_code}\n' -H 'Host: brisk-otter-41.localhost' \
    -H 'Connection: Upgrade' -H 'Upgrade: websocket' -H 'Sec-WebSocket-Version: 13' \
    -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==' \
    http://127.0.0.1:8446/_playtest/tunnel
400
{"code":"invalid","message":"握手要带 Sec-WebSocket-Protocol: playtest-tunnel-v1"}
```

加上 `Sec-WebSocket-Protocol: playtest-tunnel-v1`，不带 `Authorization`：

```
401
{"code":"unauthorized","message":"缺少 Authorization: Bearer <令牌>"}
```

带上令牌，但这台边缘还没拿到公钥（`.data/tunnel-edge/store/keys/tunnel.pub` 不存在）：

```
$ curl ... -H 'Authorization: Bearer pt1.aaa.bbb' -H 'X-Playtest-Local-Port: 5173' \
    http://127.0.0.1:8446/_playtest/tunnel
503
{"code":"internal","message":"边缘还没拿到验签公钥"}
```

## 3. 手工：公钥可以在边缘起来之后才出现

同一台机器上 api 后起是常态，所以公钥不是启动时读一次。**不重启上面那个进程**，把公钥写进去
（这里用 RFC 8032 的 ed25519 测试公钥，只为了它是一个合法的点，对应的私钥没人拿它签过令牌）：

```
$ python3 -c "import base64;print(base64.urlsafe_b64encode(bytes.fromhex('d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a')).decode().rstrip('='))"
11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo
$ mkdir -p .data/tunnel-edge/store/keys
$ printf '11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo\n' > .data/tunnel-edge/store/keys/tunnel.pub
```

同一条 curl 再打一次，503 变成了 401——公钥读到了，令牌是假的：

```
503 → 401
{"code":"unauthorized","message":"令牌验不过"}
```

环境变量那条路单独起一个实例试（数据目录是空的，磁盘上没有任何公钥文件）：

```
$ PLAYTEST_EDGE_LISTEN=127.0.0.1:8447 PLAYTEST_DATA_DIR=.data/tunnel-edge-env \
    PLAYTEST_TUNNEL_VERIFYING_KEY=11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo \
    cargo run -p playtest-edge
$ curl ... -H 'Authorization: Bearer pt1.aaa.bbb' http://127.0.0.1:8447/_playtest/tunnel
401
{"code":"unauthorized","message":"令牌验不过"}
```

没连过隧道、也没上传过的 slug 仍然是完整的一页 404，不是裸状态码：

```
$ curl -s -w '%{http_code} %{content_type}\n' -H 'Host: brisk-otter-41.localhost' \
    -H 'Accept: text/html' http://127.0.0.1:8447/
404 text/html; charset=utf-8
<!doctype html>
<html lang="zh-CN">
...
<title>这个链接不存在</title>
```

两个实例跑完都停掉了，8446 / 8447 现在都没人听。

## 没验的

- **真 CLI 没接过。** 上面所有「连上了」的证据都来自集成测试里的伪 CLI。CLI 那一头做完之前，
  「playtest dev 起来就能玩」这句话还不能说。
- **浏览器里没跑过。** WebSocket 是 tokio-tungstenite 打的，不是 Chrome；socket.io 的
  polling → websocket 升级只验到了 101 之后双向对拷这一层，没有真的跑过一个 socket.io 客户端。
- **409 只在集成测试里见过。** 驱逐名单要先有一个真会话被挤掉才会有条目，curl 造不出来。
- **上游只支持 HTTP/1.1。** dev server 如果只说 HTTP/2 明文，连不上。
- **trailer 和 `Expect: 100-continue` 没处理。** 前者被当逐跳头丢掉，后者原样转给上游，
  上游不理就会让玩家的客户端等到超时。
- **熔断没接到隧道上。** `breaker.rs` 现在只管上传的作品。
- **上下线只有 tracing 日志，没进 `edge-events.jsonl`。** `events.rs` 的 `Kind` 是固定的五种，
  加类型要改别人的文件。
- **关闭码发不出去。** `common` 的 `WsByteStream` 还没有 `control()`，被挤掉的旧连接现在是
  直接 `mux.close()`，对面看到的是 yamux 层的关闭而不是 `close::REPLACED`。
- **空闲看门狗只在集成测试的时间尺度上跑过。** 60 秒的 `IDLE_TIMEOUT_SECS` 没有真的等过。
- **没跑过并发上限。** `max_players` 的 CAS 逻辑有单测，但没有真的开 51 个连接。
