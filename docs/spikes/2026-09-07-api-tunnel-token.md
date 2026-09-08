# 2026-09-07 · 控制面能签隧道令牌，公钥发得出去

**结论**：`POST /v1/sites/{slug}/tunnel` 在本机走通了——匿名会话 → 建作品 → 拿到一个一小时有效的
Ed25519 签名令牌和一个 `wss://…/_playtest/tunnel` 的握手地址。控制面把公钥写进对象存储
`store/keys/tunnel.pub`，**用这份公钥（不碰私钥、不问控制面）能验开真实进程签出来的令牌**——
这正是边缘要做的那一步（DESIGN §4.3、§4.5）。私钥文件是 0600，重启不换钥匙，
`PLAYTEST_TUNNEL_SIGNING_KEY` 优先于文件且不覆盖文件。

机器：macOS。本机 8787 / 38787 上那两个 api 是别的会话的，本文全程用 `127.0.0.1:8790` 和
`.data/tunnel-api/`，跑完已停。日志时间戳是 UTC，`ls` 的时间是本地时间（+08）。

## 一、起服务：第一次启动生成密钥

```
$ PLAYTEST_API_LISTEN=127.0.0.1:8790 PLAYTEST_DATA_DIR=.data/tunnel-api cargo run -p playtest-api
   Compiling playtest-api v0.1.0 (/Users/zhongshangwu/workspace/github/playtest.run/api)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.27s
     Running `target/debug/playtest-api`
2026-09-07T09:37:56.411737Z  INFO 数据库迁移已应用 version=1
2026-09-07T09:37:56.412526Z  INFO 数据库迁移已应用 version=2
2026-09-07T09:37:56.417571Z  INFO 已生成新的隧道签名密钥 path=.data/tunnel-api/tunnel-signing.key
2026-09-07T09:37:56.417796Z  INFO 隧道公钥已发布，边缘从这里读 path=.data/tunnel-api/store/keys/tunnel.pub
2026-09-07T09:37:56.417926Z  INFO playtest 控制面已启动：http://127.0.0.1:8790
2026-09-07T09:37:56.417941Z  INFO 数据目录：.data/tunnel-api
2026-09-07T09:37:56.417944Z  INFO 玩家链接：http://{slug}.localhost:8443
```

两个文件的权限：私钥只有自己能读，公钥是给边缘读的所以是 0644。

```
$ ls -l .data/tunnel-api/
total 408
-rw-r--r--@ 1 zhongshangwu  staff    4096 Sep  7 17:37 api.sqlite
-rw-r--r--@ 1 zhongshangwu  staff   32768 Sep  7 17:37 api.sqlite-shm
-rw-r--r--@ 1 zhongshangwu  staff  164832 Sep  7 17:38 api.sqlite-wal
drwxr-xr-x@ 3 zhongshangwu  staff      96 Sep  7 17:37 store
-rw-------@ 1 zhongshangwu  staff      44 Sep  7 17:37 tunnel-signing.key

$ ls -l .data/tunnel-api/store/keys/
total 8
-rw-r--r--@ 1 zhongshangwu  staff  44 Sep  7 17:37 tunnel.pub

$ cat .data/tunnel-api/store/keys/tunnel.pub
tSKkNnJbnP1hufPy-9wdf6pkLRYeAyDaumL1lZcueEA
```

## 二、匿名会话 → 建作品 → 拿令牌

```
$ curl -s -X POST http://127.0.0.1:8790/v1/anon/sessions
{"token":"OR_wJGSC…（已截短，本机测试令牌，早已过期）","expires_at":"2026-09-08T09:38:31Z"}

$ export TOKEN=OR_wJGSC…（已截短，本机测试令牌，早已过期）
$ curl -s -X POST http://127.0.0.1:8790/v1/sites -H "Authorization: Bearer $TOKEN" \
    -H 'Content-Type: application/json' -d '{}'
{"slug":"bright-owl-95","url":"http://bright-owl-95.localhost:8443","title":"bright-owl-95","created_at":"2026-09-07T09:38:31Z","expires_at":"2026-09-08T09:38:31Z"}

$ export SLUG=bright-owl-95
$ curl -s -X POST http://127.0.0.1:8790/v1/sites/$SLUG/tunnel -H "Authorization: Bearer $TOKEN" \
    -H 'Content-Type: application/json' -d '{}'
{"slug":"bright-owl-95","url":"http://bright-owl-95.localhost:8443","connect_url":"ws://bright-owl-95.localhost:8443/_playtest/tunnel","token":"pt1.eyJ2IjoxLCJzbHVn…（已截短，本机测试密钥签的，早已过期）","expires_at":"2026-09-07T10:38:31Z","site_expires_at":"2026-09-08T09:38:31Z"}
```

令牌中间那段就是声明本身（base64url 的 JSON），谁都能读，所以里面没有秘密：

```
$ echo "$GRANT_TOKEN" | cut -d. -f2 | python3 -c "import sys,base64,json;s=sys.stdin.read().strip();print(json.dumps(json.loads(base64.urlsafe_b64decode(s+'='*(-len(s)%4))),ensure_ascii=False,indent=2))"
{
  "v": 1,
  "slug": "bright-owl-95",
  "sub": "f8a591e2-8884-48af-afe1-a2157b0ffe08",
  "title": "bright-owl-95",
  "developer": "匿名开发者",
  "badge": true,
  "gate": "once",
  "isolated": false,
  "max_players": 50,
  "iat": 1788773911,
  "exp": 1788777511,
  "jti": "a8166740-dcfc-46a1-88e6-9051e9636534"
}
```

`exp - iat = 3600`，就是 `common::tunnel::TOKEN_TTL_SECS`。没给作品名就用建作品时那个。

请求里给的作品名、门禁策略、`--isolated` 原样进令牌——边缘只看令牌，看不到我们的库：

```
$ curl -s -X POST http://127.0.0.1:8790/v1/sites/$SLUG/tunnel -H "Authorization: Bearer $TOKEN" \
    -H 'Content-Type: application/json' -d '{"title":"小球","gate":"always","isolated":true}'
{"slug":"bright-owl-95","url":"http://bright-owl-95.localhost:8443","connect_url":"ws://bright-owl-95.localhost:8443/_playtest/tunnel","token":"pt1.eyJ2IjoxLCJzbHVn…（已截短，本机测试密钥签的，早已过期）","expires_at":"2026-09-07T10:40:29Z","site_expires_at":"2026-09-08T09:38:31Z"}
```

解出来的声明里 `"title": "小球"`、`"gate": "always"`、`"isolated": true`，其余同上；两次的 `jti` 不同
（`a8166740-…` 与 `405b7ce2-…`），撤销名单按它记才记得住是哪一次。

服务端日志里只有 slug、jti、过期时间，没有令牌本身：

```
09:38:31  INFO 签发了一个隧道令牌 slug=bright-owl-95 jti=a8166740-dcfc-46a1-88e6-9051e9636534 exp=1788777511
09:40:29  INFO 签发了一个隧道令牌 slug=bright-owl-95 jti=405b7ce2-56ee-4888-8f1d-183e24272d3e exp=1788777629
```

## 三、边缘那一步：只拿公钥，验得开

用一段只依赖 Python 标准库的 RFC 8032 参考实现（`/tmp/pt-verify.py`，一次性脚本，没进仓库）——
换一套实现来验，才能说明「边缘只要这份公钥就够了」，而不是我们自己的库自说自话。

```
$ python3 /tmp/pt-verify.py .data/tunnel-api/store/keys/tunnel.pub "$GRANT2_TOKEN"
公钥： tSKkNnJbnP1hufPy-9wdf6pkLRYeAyDaumL1lZcueEA
验签： 通过
声明： {"v": 1, "slug": "bright-owl-95", "sub": "f8a591e2-8884-48af-afe1-a2157b0ffe08", "title": "小球", "developer": "匿名开发者", "badge": true, "gate": "always", "isolated": true, "max_players": 50, "iat": 1788774029, "exp": 1788777629, "jti": "405b7ce2-56ee-4888-8f1d-183e24272d3e"}
```

改掉令牌里一个字节再验（对照组，证明上面那个「通过」不是白给的）：

```
$ python3 /tmp/pt-verify.py .data/tunnel-api/store/keys/tunnel.pub "$FORGED"
公钥： tSKkNnJbnP1hufPy-9wdf6pkLRYeAyDaumL1lZcueEA
验签： 不通过
```

## 四、失败长什么样

```
$ curl -s -o /tmp/e1 -w '%{http_code} ' -X POST http://127.0.0.1:8790/v1/sites/$SLUG/tunnel \
    -H 'Content-Type: application/json' -d '{}'; cat /tmp/e1
401 {"code":"unauthorized","message":"这个请求没带令牌。第一次用直接运行 playtest，它会自动申请一个 24 小时的匿名链接。"}

$ curl -s -o /tmp/e2 -w '%{http_code} ' -X POST http://127.0.0.1:8790/v1/sites/$SLUG/tunnel \
    -H "Authorization: Bearer $OTHER" -H 'Content-Type: application/json' -d '{}'; cat /tmp/e2
404 {"code":"not_found","message":"没有这个作品，或者它不是你的。用 playtest ls 看看你有哪些作品。"}

$ curl -s -o /tmp/e3 -w '%{http_code} ' -X POST http://127.0.0.1:8790/v1/sites/$SLUG/tunnel \
    -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
    -d "{\"title\":\"$(python3 -c 'print("字"*81)')\"}"; cat /tmp/e3
400 {"code":"invalid","message":"作品名最多 80 个字，这个有 81 个。"}
```

`$OTHER` 是另一个匿名会话的令牌。别人的作品和不存在的 slug 一样是 404，不透露这个 slug 存不存在。

## 五、重启不换钥匙

先 Ctrl-C（这次是给进程发 SIGINT），退得干净：

```
2026-09-07T09:52:03.699268Z  INFO 收到停止信号，等手上的请求做完再退出
（进程退出码 0）
```

同样的数据目录再起一次，日志里**没有**「已生成新的隧道签名密钥」，公钥也没变，
上一轮签的令牌照样验得开——重启不会把在线的隧道全踢下线：

```
2026-09-07T09:53:00.439035Z  INFO 隧道公钥已发布，边缘从这里读 path=.data/tunnel-api/store/keys/tunnel.pub
2026-09-07T09:53:00.439232Z  INFO playtest 控制面已启动：http://127.0.0.1:8790

$ cat .data/tunnel-api/store/keys/tunnel.pub
tSKkNnJbnP1hufPy-9wdf6pkLRYeAyDaumL1lZcueEA
$ python3 /tmp/pt-verify.py .data/tunnel-api/store/keys/tunnel.pub "$GRANT2_TOKEN" | head -2
公钥： tSKkNnJbnP1hufPy-9wdf6pkLRYeAyDaumL1lZcueEA
验签： 通过
```

## 六、环境变量里的私钥优先，且不动密钥文件

第三次启动带上 `PLAYTEST_TUNNEL_SIGNING_KEY`（一把临时生成的钥匙，不记在这里），
顺便把链接模板换成正式域名：

```
$ PLAYTEST_API_LISTEN=127.0.0.1:8790 PLAYTEST_DATA_DIR=.data/tunnel-api \
  PLAYTEST_SITE_URL_TEMPLATE='https://{slug}.playtest.run' \
  PLAYTEST_TUNNEL_SIGNING_KEY="$(cat /tmp/pt-envkey.txt)" cargo run -q -p playtest-api

$ curl -s -X POST http://127.0.0.1:8790/v1/sites/$SLUG/tunnel -H "Authorization: Bearer $TOKEN" \
    -H 'Content-Type: application/json' -d '{}'
{"slug":"bright-owl-95","url":"https://bright-owl-95.playtest.run","connect_url":"wss://bright-owl-95.playtest.run/_playtest/tunnel","token":"pt1.eyJ2IjoxLCJzbHVn…（已截短，本机测试密钥签的，早已过期）","expires_at":"2026-09-07T10:56:16Z","site_expires_at":"2026-09-08T09:38:31Z"}
```

`connect_url` 跟着模板变成了 `wss://`。发布出去的公钥换成了环境变量那把，私钥文件一个字节没动
（时间还是 17:37）：

```
$ cat .data/tunnel-api/store/keys/tunnel.pub
nWDqAiYWaIQ0mZsThJyeikwhLFaVf-r6lEHXUnG_Mes
$ ls -l .data/tunnel-api/tunnel-signing.key
-rw-------@ 1 zhongshangwu  staff  44 Sep  7 17:37 .data/tunnel-api/tunnel-signing.key

$ python3 /tmp/pt-verify.py .data/tunnel-api/store/keys/tunnel.pub "$GRANT3_TOKEN" | head -2   # 新钥匙签的
公钥： nWDqAiYWaIQ0mZsThJyeikwhLFaVf-r6lEHXUnG_Mes
验签： 通过
$ python3 /tmp/pt-verify.py .data/tunnel-api/store/keys/tunnel.pub "$GRANT2_TOKEN" | head -2   # 旧钥匙签的
公钥： nWDqAiYWaIQ0mZsThJyeikwhLFaVf-r6lEHXUnG_Mes
验签： 不通过
```

换钥匙就是这个代价：旧令牌全部作废，CLI 得重新要一个。撤掉环境变量再起一次，
公钥回到密钥文件那把，旧令牌又验得开了：

```
$ cat .data/tunnel-api/store/keys/tunnel.pub
tSKkNnJbnP1hufPy-9wdf6pkLRYeAyDaumL1lZcueEA
$ python3 /tmp/pt-verify.py .data/tunnel-api/store/keys/tunnel.pub "$GRANT2_TOKEN" | head -2
公钥： tSKkNnJbnP1hufPy-9wdf6pkLRYeAyDaumL1lZcueEA
验签： 通过
```

跑完四个实例都用 SIGINT 停掉了，`ps` 里只剩别的会话那两个进程。

## 自动化测试

```
$ cargo test -p playtest-api
   ...
     Running tests/tunnel_flow.rs
running 8 tests
test the_environment_key_wins_over_the_file ... ok
test the_request_decides_title_gate_and_isolation ... ok
test restarting_keeps_the_same_key ... ok
test an_overlong_title_is_refused ... ok
test only_my_own_site_gets_a_token ... ok
test the_production_template_gives_wss ... ok
test anonymous_developer_gets_a_token_the_edge_can_verify ... ok
test a_token_is_required_and_expiry_says_so ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
```

整个 crate（17 条单元测试 + 35 条集成测试）全绿，另外三个集成测试文件是别的会话的，没被这次改动碰坏。

## 没验证的

- **边缘那一端**。这里只证明了「拿公钥能验开令牌」，`edge` 还没有真的读过
  `store/keys/tunnel.pub`，也没有真的接过一条隧道。那一条要等 edge 的隧道代码，另记一条 spike。
- **CLI 那一端**。没有 CLI 调过这个端点，`playtest 5173` 还不存在。
- **令牌到期与换新**。TTL 一小时，本文没有等到过期，也没有测「CLI 在到期前换新」这条路；
  `Claims::is_valid_at` 的边界只有 `common` 的单元测试覆盖。
- **撤销**。第三周的事：现在没有撤销名单，签出去的令牌在一小时内只能靠它自己过期。
  作品被删、匿名用户到期之后，已经签出去的令牌**仍然验得过**——边缘那边要有名单才拦得住。
- **用量**。令牌里带了 `max_players: 50`，但没有人按它计数；每 60 秒上报也还不存在。
- **多个控制面实例**。密钥文件的抢写用 `create_new` 处理了「谁先写谁算数」，但没有真的并发起过两个。
- **登录用户**。v0.1 没有登录入口，`developer` 只见过「匿名开发者」这一种值。
