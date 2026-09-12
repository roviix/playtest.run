# 2026-09-13 · S3 兼容存储迁移与网页路径实测

**结论**：在 macOS 本机的 S3rver 3.7.1 上，文件系统已有物料可以校验迁入 S3 兼容
存储；切换后的 API 能直接分段写入新版本，edge 能流式读取及返回 Range，回滚、删除、
存储中断后的 503 与恢复均符合设计。迁移工具重复执行会跳过已一致的对象，旧目录没有被
修改或删除。

这只是 **S3 协议兼容服务上的本机演练**。它不等于 AWS S3、香港生产链路、真实域名、
备份恢复或生产迁移已经验收，不能用来改写 DESIGN §4.2、§8.2 的未完成状态。

## 环境与边界

- macOS；仓库里的 `playtest-api`、`playtest-edge` 与迁移 example。
- S3rver 3.7.1，监听 `127.0.0.1:4569`，私有测试 bucket `playtest-test`，对象前缀
  `e2e-20260913`。
- 测试物料来自仓库原创 fixture：`fixtures/pelican-bicycle/export/index.html`
  （4,772 字节）和 `fixtures/headers-lab/export/index.html`（9,110 字节）。
- API 与 edge 使用同一 bucket / prefix，但用各自进程环境变量注入凭据；凭据没有写进仓库。

## 一、先从文件系统发布 v1，再停写迁移

API 先以 `PLAYTEST_STORAGE_BACKEND=fs` 启动，通过与 CLI 相同的 HTTP 四步上传路径发布
`cobalt-finch-62` 的 v1。提交结果：

```text
{"slug":"cobalt-finch-62","version":1,"url":"http://cobalt-finch-62.localhost:8878",...}
```

停掉 API 后运行：

```text
$ PLAYTEST_STORAGE_BACKEND=s3 ... \
  cargo run -p playtest-common --example migrate_store -- \
  /tmp/playtest-s3-e2e.z2AiyH/api/store

源：本机文件系统 /tmp/playtest-s3-e2e.z2AiyH/api/store
目标：S3 http://127.0.0.1:4569 bucket=playtest-test/e2e-20260913 region=us-east-1
迁移期间必须停止 API 写入；旧目录不会被修改或删除。
校验完成：8 个对象，复制 8，已一致 0，共 5778 字节
```

紧接着用相同命令重跑：

```text
校验完成：8 个对象，复制 0，已一致 8，共 5778 字节
```

旧目录里仍是原来的 8 个对象。迁移实现按源与目标 SHA-256 写后复读校验；blob 键还会和
内容哈希核对，不覆盖内容不同的 blob 或版本清单。`current.json` 最后迁移，避免先把玩家
指向还没复制完的版本。

## 二、edge 从迁移后的 S3 读取与 Range

edge 以 S3 后端启动，未挂载旧物料目录。对迁移后的 v1 请求：

```text
HEAD / HTTP/1.1
HTTP/1.1 200 OK
etag: "f879de58806104d6e8e1513758e1ada9d01216ba4ede278859eeb4febdde9ffd"
accept-ranges: bytes
content-length: 4772

Range: bytes=0-14
HTTP/1.1 206 Partial Content
content-range: bytes 0-14/4772
content-length: 15

<!doctype html>
```

整页也能读到 fixture 的标题「今天，骑去海边」。HEAD 只取元数据，GET 直接把对象流交给
响应体；Range 由对象存储请求承接，没有把整个文件读入内存再切片。

## 三、切换后的 API 直接写 S3、回滚与删除

同一份 SQLite 数据改以 S3 后端重启 API。启动时写、读、删探针通过后才开始监听。上传
9,110 字节的新 `index.html` 时，S3rver 记录了 multipart initiate、part 1 与 complete；
API 返回 201，提交得到 v2。edge 的下一次读取出现「边缘响应头自检」，证明当前指针与
新 blob 都来自 S3。

激活 v1 后 API 返回 `"current_version":1`；edge 再次出现「今天，骑去海边」，且不再
出现 v2 标题。最后删除作品得到 204，edge 在缓存到期后得到 404。共享 blob 没有随作品
立即删除，仍由带 24 小时安全期的垃圾回收处理。

## 四、存储故障不会伪装成 404 或落回本机

停止 S3rver、等当前指针缓存到期后，请求刚才已知存在的作品：

```text
HTTP/1.1 503 Service Unavailable
cache-control: no-store

作品暂时取不出来
我们暂时连不上作品存储，不能确认这份内容现在是否允许访问。稍后再试，你的设备没有问题。
```

另起一个 API、让它在 S3 仍不可达时启动，进程以 1 退出，并明确报告写探针连接被拒绝；
本机物料目录里没有出现替代写入。恢复同一 S3rver 数据目录后，原 edge 无需重启即可重新
读到 v1。

## 五、还没有验证什么

- 没有使用真实 AWS S3 bucket、IAM 角色、TLS endpoint 或香港网络，生产凭据最小权限尚未
  实测。
- 没有演练数据库与 bucket 的一致备份、版本化误删恢复或跨区域恢复。
- 垃圾回收的 S3 列表 / 删除代码有自动化测试，但本次没有把对象时间拨过 24 小时后做真机
  回收，因此不记作 S3 垃圾回收验收。
- 没有做大于 8 MiB 的远端 multipart、并发上传、真实手机或三网弱网测试。

所以当前能说的是「S3 兼容后端及迁移路径已在本机跑通」，不能说「生产 S3 迁移完成」。
