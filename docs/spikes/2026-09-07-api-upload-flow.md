# 2026-09-07 · 控制面上传路径在本机跑通

**结论**：`playtest-api` 在本机把 DESIGN §4.2 的四步走通了——匿名会话 → 建作品 → 问缺哪些哈希 →
只传缺的 → 提交清单，拿到 `v1`；同样的文件再来一遍 `missing` 为空，直接拿到 `v2`。
清单和「当前版本」指针落在对象存储里，形状和 `common::store` 的布局一致，边缘可以直接读。

机器：macOS，rustc 1.96.0 (ac68faa20 2026-05-25)。全部命令从仓库根目录执行。

## 起服务

```
$ cargo run -p playtest-api
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.26s
     Running `target/debug/playtest-api`
2026-09-07T04:01:29.569854Z  INFO 数据库迁移已应用 version=1
2026-09-07T04:01:29.570055Z  INFO playtest 控制面已启动：http://127.0.0.1:8787
2026-09-07T04:01:29.570072Z  INFO 数据目录：.data
2026-09-07T04:01:29.570075Z  INFO 玩家链接：http://{slug}.localhost:8443
```

测试用的文件是一个 89 字节的 `index.html`：

```
$ shasum -a 256 /tmp/pt-spike/index.html
67ca67157d142078b28cfaf5287c6cdbad2a5f3ee26c602046d5595576643f27  /tmp/pt-spike/index.html
```

## 一、四步走通

```
$ curl -s -i http://127.0.0.1:8787/healthz
HTTP/1.1 200 OK
content-type: text/plain; charset=utf-8
content-length: 2

ok

$ curl -s -X POST http://127.0.0.1:8787/v1/anon/sessions
{"token":"vBoqXNsF…（已截短，本机测试令牌，早已过期）","expires_at":"2026-09-08T04:02:15Z"}

$ export TOKEN=vBoqXNsF…（已截短，本机测试令牌，早已过期）
$ curl -s -X POST http://127.0.0.1:8787/v1/sites -H "Authorization: Bearer $TOKEN" \
    -H 'Content-Type: application/json' -d '{}'
{"slug":"gentle-walrus-71","url":"http://gentle-walrus-71.localhost:8443","title":"gentle-walrus-71","created_at":"2026-09-07T04:02:23Z","expires_at":"2026-09-08T04:02:15Z"}

$ export SLUG=gentle-walrus-71
$ export HASH=67ca67157d142078b28cfaf5287c6cdbad2a5f3ee26c602046d5595576643f27
$ curl -s -X POST http://127.0.0.1:8787/v1/sites/$SLUG/uploads -H "Authorization: Bearer $TOKEN" \
    -H 'Content-Type: application/json' \
    -d "{\"files\":[{\"path\":\"index.html\",\"hash\":\"$HASH\",\"size\":89}],\"title\":\"小球\",\"note\":\"第一版：能动了\"}"
{"upload_id":"df5c34f7-b3a3-451e-b6f5-320bc0ea33ba","missing":["67ca67157d142078b28cfaf5287c6cdbad2a5f3ee26c602046d5595576643f27"],"missing_bytes":89}
```

传内容。第一次 201，同样的内容再传一次 200（已经有了，不重复占盘）：

```
$ curl -s -o /dev/null -w '%{http_code}\n' -X PUT http://127.0.0.1:8787/v1/blobs/$HASH \
    -H "Authorization: Bearer $TOKEN" --data-binary @/tmp/pt-spike/index.html
201
$ curl -s -o /dev/null -w '%{http_code}\n' -X PUT http://127.0.0.1:8787/v1/blobs/$HASH \
    -H "Authorization: Bearer $TOKEN" --data-binary @/tmp/pt-spike/index.html
200

$ export UPLOAD=df5c34f7-b3a3-451e-b6f5-320bc0ea33ba
$ curl -s -X POST http://127.0.0.1:8787/v1/sites/$SLUG/uploads/$UPLOAD/commit \
    -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' -d '{}'
{"slug":"gentle-walrus-71","version":1,"url":"http://gentle-walrus-71.localhost:8443","expires_at":"2026-09-08T04:02:15Z"}
```

## 二、对象存储里长什么样

```
$ find .data -type f | sort
.data/api.sqlite
.data/api.sqlite-shm
.data/api.sqlite-wal
.data/store/blobs/67/67ca67157d142078b28cfaf5287c6cdbad2a5f3ee26c602046d5595576643f27
.data/store/sites/gentle-walrus-71/current.json
.data/store/sites/gentle-walrus-71/manifests/1.json

$ cat .data/store/sites/gentle-walrus-71/current.json
{"version":1,"updated_at":"2026-09-07T04:02:55Z"}

$ cat .data/store/sites/gentle-walrus-71/manifests/1.json
{
  "schema": 1,
  "slug": "gentle-walrus-71",
  "version": 1,
  "title": "小球",
  "developer": "匿名开发者",
  "note": "第一版：能动了",
  "created_at": "2026-09-07T04:02:55Z",
  "expires_at": "2026-09-08T04:02:15Z",
  "badge": true,
  "gate": "once",
  "isolated": false,
  "spa": false,
  "files": [
    {
      "path": "index.html",
      "hash": "67ca67157d142078b28cfaf5287c6cdbad2a5f3ee26c602046d5595576643f27",
      "size": 89
    }
  ]
}
```

WAL 和迁移版本：

```
$ sqlite3 .data/api.sqlite "SELECT * FROM schema_version; PRAGMA journal_mode;"
1
wal
```

## 三、第二个版本：内容没变就一个字节都不传

```
$ curl -s -X POST http://127.0.0.1:8787/v1/sites/$SLUG/uploads -H "Authorization: Bearer $TOKEN" \
    -H 'Content-Type: application/json' \
    -d "{\"files\":[{\"path\":\"index.html\",\"hash\":\"$HASH\",\"size\":89}],\"note\":\"第二版：只改了一行\"}"
{"upload_id":"0e460ed1-7d4f-4cfa-b302-62985c07d931","missing":[],"missing_bytes":0}

$ curl -s -X POST http://127.0.0.1:8787/v1/sites/$SLUG/uploads/0e460ed1-7d4f-4cfa-b302-62985c07d931/commit \
    -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' -d '{}'
{"slug":"gentle-walrus-71","version":2,"url":"http://gentle-walrus-71.localhost:8443","expires_at":"2026-09-08T04:02:15Z"}

$ cat .data/store/sites/$SLUG/current.json
{"version":2,"updated_at":"2026-09-07T04:03:25Z"}

$ curl -s http://127.0.0.1:8787/v1/sites/$SLUG -H "Authorization: Bearer $TOKEN"
{"slug":"gentle-walrus-71","url":"http://gentle-walrus-71.localhost:8443","title":"小球","current_version":2,"created_at":"2026-09-07T04:02:23Z","expires_at":"2026-09-08T04:02:15Z"}
```

`v1` 的清单还在（`manifests/1.json` 没被覆盖），回滚有东西可指。

```
$ sqlite3 -header -column .data/api.sqlite "SELECT slug, version, file_count, total_bytes, note FROM versions;"
slug              version  file_count  total_bytes  note
----------------  -------  ----------  -----------  ------------------
gentle-walrus-71  1        1           89           第一版：能动了
gentle-walrus-71  2        1           89           第二版：只改了一行
bold-squid-58     1        1           89
```

## 四、失败长什么样

内容和地址里的哈希对不上（故意传一段别的字节）：

```
$ curl -s -X PUT http://127.0.0.1:8787/v1/blobs/$HASH -H "Authorization: Bearer $TOKEN" \
    --data-binary '这不是那个文件'
{"code":"hash_mismatch","message":"这些字节算出来的内容哈希是 6ca5448a5ecfd62057c3db001d93d1b446be26904cec63a5fe3b09b6474d49eb，和地址里的 67ca67157d142078b28cfaf5287c6cdbad2a5f3ee26c602046d5595576643f27 对不上。文件在路上变了或者传错了地方，重传一次。"}
```

这次之后上面第二节那条 `find .data -type f` 再跑一遍，`blobs/` 下没有多出文件，`tmp/` 下一个文件都没有。

下面几条用 `-w '%{http_code} '` 把状态码打在响应体前面：

```
$ curl -s -o /tmp/o1 -w '%{http_code} ' -X POST http://127.0.0.1:8787/v1/sites \
    -H 'Content-Type: application/json' -d '{}'; cat /tmp/o1
401 {"code":"unauthorized","message":"这个请求没带令牌。第一次用直接运行 playtest，它会自动申请一个 24 小时的匿名链接。"}

$ curl -s -o /tmp/o2 -w '%{http_code} ' -X POST http://127.0.0.1:8787/v1/sites/$SLUG/uploads \
    -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
    -d "{\"files\":[{\"path\":\"../secrets.env\",\"hash\":\"$HASH\",\"size\":89}]}"; cat /tmp/o2
400 {"code":"invalid","message":"路径里有空段、「.」或「..」：../secrets.env"}

$ curl -s -o /tmp/o3 -w '%{http_code} ' -X POST \
    http://127.0.0.1:8787/v1/sites/$SLUG/uploads/0e460ed1-7d4f-4cfa-b302-62985c07d931/commit \
    -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' -d '{}'; cat /tmp/o3
400 {"code":"invalid","message":"这次上传已经提交过了。要再发一版，重新走一遍准备上传。"}
```

配额：匿名用户建到第四个被挡住。

```
$ curl -s -o /dev/null -w '%{http_code} ' -X POST http://127.0.0.1:8787/v1/sites \
    -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' -d '{}'   # 第二个
$ curl -s -o /dev/null -w '%{http_code} ' -X POST http://127.0.0.1:8787/v1/sites \
    -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' -d '{}'   # 第三个
200 200
$ curl -s -o /tmp/o4 -w '第四个: %{http_code} ' -X POST http://127.0.0.1:8787/v1/sites \
    -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' -d '{}'; cat /tmp/o4
第四个: 403 {"code":"quota_exceeded","message":"匿名链接最多同时留 3 个作品，你已经有 3 个了。先用 playtest rm 删掉一个再建。"}
```

删作品：链接立刻失效，内容本身留着（blob 跨作品去重）。

```
$ curl -s -o /dev/null -w 'DELETE: %{http_code}\n' -X DELETE http://127.0.0.1:8787/v1/sites/$SLUG \
    -H "Authorization: Bearer $TOKEN"
DELETE: 204
$ find .data/store -type f | sort
.data/store/blobs/67/67ca67157d142078b28cfaf5287c6cdbad2a5f3ee26c602046d5595576643f27
$ curl -s -o /tmp/o5 -w 'GET 已删的: %{http_code} ' http://127.0.0.1:8787/v1/sites/$SLUG \
    -H "Authorization: Bearer $TOKEN"; cat /tmp/o5
GET 已删的: 404 {"code":"not_found","message":"没有这个作品，或者它不是你的。用 playtest ls 看看你有哪些作品。"}
```

令牌过期。不等 24 小时，直接把库里的到期时间拨到 2020 年：

```
$ sqlite3 .data/api.sqlite "UPDATE users SET expires_at = '2020-01-01T00:00:00Z'; UPDATE tokens SET expires_at = '2020-01-01T00:00:00Z';"
$ curl -s -o /tmp/o6 -w '过期令牌: %{http_code} ' http://127.0.0.1:8787/v1/sites \
    -H "Authorization: Bearer $TOKEN"; cat /tmp/o6
过期令牌: 401 {"code":"token_expired","message":"匿名链接的 24 小时已到，这个令牌和它创建的作品都失效了。重新运行 playtest 会拿到一个新链接。"}
```

## 五、过期的匿名作品会自己消失

先建一个作品并提交一版，再把它主人的到期时间拨到过去，然后等后台任务那一轮（每 10 分钟一次）。
服务是 04:01:29 起的，所以下一轮在 04:11:29：

```
$ sqlite3 .data/api.sqlite "UPDATE users SET expires_at='2020-01-01T00:00:00Z' WHERE expires_at > '2026';"
$ date -u +%H:%M:%S
04:04:38
```

七分钟后，服务日志里出现：

```
2026-09-07T04:11:29.584256Z  INFO 清掉了过期的匿名作品和令牌 sites=3 tokens=1
```

清完之后 `bold-squid-58`（刚才提交过 v1 的那个）在对象存储里的清单和指针都没了，库里 `deleted_at` 也填上了：

```
$ sqlite3 -header -column .data/api.sqlite "SELECT slug, deleted_at FROM sites;"
slug              deleted_at
----------------  --------------------
gentle-walrus-71  2026-09-07T04:03:57Z
sturdy-salmon-69  2026-09-07T04:11:29Z
sage-lynx-30      2026-09-07T04:11:29Z
bold-squid-58     2026-09-07T04:11:29Z
```

## 六、环境变量覆盖

第二个实例，换端口、换数据目录、换链接模板：

```
$ PLAYTEST_API_LISTEN=127.0.0.1:8799 PLAYTEST_DATA_DIR=/tmp/pt-spike/data2 \
  PLAYTEST_SITE_URL_TEMPLATE='https://{slug}.playtest.run' cargo run -p playtest-api
2026-09-07T04:12:08.434822Z  INFO 数据库迁移已应用 version=1
2026-09-07T04:12:08.435015Z  INFO playtest 控制面已启动：http://127.0.0.1:8799
2026-09-07T04:12:08.435036Z  INFO 数据目录：/tmp/pt-spike/data2
2026-09-07T04:12:08.435048Z  INFO 玩家链接：https://{slug}.playtest.run

$ export T3=$(curl -s -X POST http://127.0.0.1:8799/v1/anon/sessions | sed -n 's/.*"token":"\([^"]*\)".*/\1/p')
$ curl -s -X POST http://127.0.0.1:8799/v1/sites -H "Authorization: Bearer $T3" \
    -H 'Content-Type: application/json' -d '{}'
{"slug":"scarlet-raven-18","url":"https://scarlet-raven-18.playtest.run","title":"scarlet-raven-18","created_at":"2026-09-07T04:12:28Z","expires_at":"2026-09-08T04:12:28Z"}
```

## 七、旁证：另一个客户端也走通了

上面那台服务开着的时候，同一台机器上另一个进程（不是本文的 curl）在 04:09 打进来走了两遍完整流程，
日志原文：

```
04:09:03  建了一个作品 slug=zesty-macaw-46 user_id=b8b3dd67-…
04:09:03  准备上传 slug=zesty-macaw-46 files=7 missing=7 missing_bytes=46185
04:09:03  提交了一个版本 slug=zesty-macaw-46 version=1 file_count=7 total_bytes=46185
04:09:16  准备上传 slug=zesty-macaw-46 files=7 missing=0 missing_bytes=0
04:09:16  提交了一个版本 slug=zesty-macaw-46 version=2 file_count=7 total_bytes=46185
04:09:17  建了一个作品 slug=nimble-robin-87 user_id=b8b3dd67-…
04:09:17  准备上传 slug=nimble-robin-87 files=7 missing=6 missing_bytes=1058000
04:09:17  提交了一个版本 slug=nimble-robin-87 version=1 file_count=7 total_bytes=1058042
```

七个文件的目录、第二版增量为零、另一个目录里六个新文件一共 1 MB，全程没有 4xx / 5xx。
整段日志里所有的 4xx 都是本文第四节故意造的那几条，没有一条 5xx，没有 panic。

## 没验证的

- **大文件**。最大只传过 1 MB（还是别人传的）。`PUT /v1/blobs` 是边收边写、边算哈希的流式实现，
  但 200 MB 上限和「收到一半被掐断」这两条路径只有自动化测试覆盖，真机上没跑过。
- **收到一半断线**。`interrupted()` 用已收字节数区分「太大了」和「网断了」，这个判断没有真机记录。
- **优雅退出**。Ctrl-C 的处理写了，但这次没能在真机上发信号验证。
- **边缘读**。这里只确认了对象存储里的字节长什么样，没有让 `edge` 真的按这份清单服务过一次。
  那一条要等 `edge` 那边就绪，另记一条 spike。
- **登录用户**。v0.1 没有登录入口，指定 slug、免费档 500 MB 上限这两条分支只有代码，没有走过。
