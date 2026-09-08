# 2026-09-07 · fixtures/ws-rooms：socket.io 联机小游戏（隧道路径的靶子）

做了 `fixtures/ws-rooms/`，给 DESIGN §4.3 的隧道路径当测试输入。前三个 fixture 是上传路径的静态导出物，这个不是——它是一个要一直跑着的进程，没有 `export/`。

**结论先写在前面：只在本机 `127.0.0.1:5174` 直连验证过。没有穿过隧道，没有在手机上开过，也没有人在浏览器里看过这一页长什么样。** DESIGN §8 v0.1 那句「一个 Colyseus 或 socket.io 的联机小游戏走隧道路径两台手机能对局」，这次只把「小游戏」这一半做出来并在本机验了，「走隧道」「两台手机」都还没有。

## 机器与版本

```
$ node --version
v22.22.0
$ pnpm --version
11.25.0
```

`package.json` 写的是 `socket.io: ^4`（实际填 `^4.8.1`），装到 4.8.3：

```
$ pnpm install --reporter=append-only
Packages: +25
Progress: resolved 25, reused 0, downloaded 25, added 25, done

dependencies:
+ socket.io 4.8.3

devDependencies:
+ socket.io-client 4.8.3

Done in 4.3s using pnpm v11.25.0
```

传递依赖里和隧道有关的几个（从 `pnpm-lock.yaml`）：

```
engine.io@6.6.10        engine.io-client@6.6.6
engine.io-parser@5.2.3  socket.io-parser@4.2.7
socket.io-adapter@2.5.8 ws@8.21.3
```

`node_modules` 装完 8.4 MB，13 个包（25 个含 peer/types）。

## 1. 起服务

```
$ node server.mjs 5174
ws-rooms 已启动：http://127.0.0.1:5174  （socket.io 挂在同一端口）
自检：node check.mjs http://127.0.0.1:5174
隧道：playtest 5174；下面每条日志都会带上 Host 与 X-Forwarded-Host
只监听本机。要让同一 Wi-Fi 下的手机直连，用 HOST=0.0.0.0 node server.mjs
```

## 2. curl：页面、engine.io 握手、隧道会改写的两个头

```
$ curl -s http://127.0.0.1:5174/ | head -5
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta

$ curl -s 'http://127.0.0.1:5174/socket.io/?EIO=4&transport=polling'
0{"sid":"2TGDLyfHx5Fcr_6RAAAB","upgrades":["websocket"],"pingInterval":25000,"pingTimeout":20000,"maxPayload":1000000}

$ curl -s -o /dev/null -w 'status=%{http_code} bytes=%{size_download}\n' \
    -H 'Host: localhost:5174' \
    -H 'X-Forwarded-Host: brisk-otter-41.playtest.run' \
    http://127.0.0.1:5174/
status=200 bytes=9581
```

握手回的 `upgrades:["websocket"]` 就是这个 fixture 要考的那一步——边缘要把随后的 101 原样放过去。

服务器日志里对应的两行，第二行是模拟隧道改写之后的样子：

```
09:26:32.159 GET /  Host=127.0.0.1:5174  X-Forwarded-Host=(无)
09:26:32.174 GET /  Host=localhost:5174  X-Forwarded-Host=brisk-otter-41.playtest.run
```

进房和升级也都打出来了：

```
09:32:30.882 + 迟钝海雀 11 进房 (NowVHjXU9kl0XXDQAAAG)  传输=polling  Host=127.0.0.1:5174  X-Forwarded-Host=(无)
09:32:30.885 + 倔强章鱼 24 进房 (haXEVLbVhqDwDgMYAAAH)  传输=polling  Host=127.0.0.1:5174  X-Forwarded-Host=(无)
09:32:30.885 ↑ 迟钝海雀 11 传输升级 polling → websocket
09:32:30.887 ↑ 倔强章鱼 24 传输升级 polling → websocket
09:32:32.950 - 迟钝海雀 11 离开 (transport close)  剩 2 人
09:32:33.212 + 柔软鲸鱼 37 进房 (C_9JzFS4ODHQ9jdcAAAI)  传输=polling  Host=127.0.0.1:5174  X-Forwarded-Host=(无)
```

中间那次 `transport close` → 立刻新进房，是 `check.mjs` 掐掉传输测重连，不是异常。

## 3. check.mjs：两个客户端对局

```
$ node check.mjs http://127.0.0.1:5174
目标 http://127.0.0.1:5174

PASS  页面可取  status=200 9581 字节
PASS  engine.io polling 握手  status=200 sid=1IrvQXW8EDUbxj8bAAAB
PASS  /socket.io/socket.io.js 由服务器提供  status=200 type=application/javascript; charset=utf-8
PASS  起手是 polling（没跳过升级这一关）  transport=polling
PASS  各自拿到昵称和颜色  A=嘹亮狐狸 36/hsl(275 72% 58%)  B=闪亮麋鹿 49/hsl(53 72% 58%)
PASS  polling → websocket 升级完成  A=websocket B=websocket，升级事件 B→websocket A→websocket
PASS  双方 players 里都有自己和对方  房里 3 人：轻盈树懒 23、嘹亮狐狸 36、闪亮麋鹿 49
PASS  move 坐标原样到达  收到 x=0.25 y=0.75
PASS  tap 广播到全房间  color=hsl(275 72% 58%)
PASS  tick 的往返算得出来  三次 1.0 / 2.3 / 0.4 ms，平均 1.2 ms
PASS  二进制帧双向原样过  上行 4 字节，下行 8 字节时间戳解回一致=true
PASS  断线后客户端自己回来了  旧 id=XyqsSqTqRi_G7KG4AAAB 新 id=Kroxm0qcIEQ5StYZAAAD
PASS  重连后的 A 重新出现在 players 里  房里 3 人：轻盈树懒 23、闪亮麋鹿 49、锋利刺猬 62
PASS  断开就移出房间  剩 轻盈树懒 23、闪亮麋鹿 49

全部通过
$ echo $?
0
```

一共跑了六次：前两次红，原因见下一节；后四次全绿，14 条都不是抖的。上面贴的是倒数第二次；最后一次是改完 `server.mjs` 里一处写法之后的复跑，同样 14 条全绿（往返 0.7 / 3.2 / 4.9 ms）。

「页面可取」那一行的 `9581 字节`和上面 `curl` 的 `bytes=9581` 是同一个数，这是有意对上的——DESIGN §4.3 要求隧道对响应体一个字节不动，所以这里数的是字节不是字符（这一页有中文，字符数只有 9172，差 409）。隧道那头这两个数要还相等。

往返 0.4–2.3 ms 是本机回环的数字，**它唯一的用处是证明「算得出来」**；隧道路径上这个数会大两三个数量级，那时候它才有意义。

### 中途改过三次，都是测试写错不是服务器写错

- 第一版 `waitFor(a, 'players', …)` 死等下一条 `players`，但 B 进房触发的那条在断言注册之前就到了。`players` 是状态不是事件，改成先看已收到的最后一条（`waitPlayers`）。
- 第二版断言「房里正好 2 人」，结果房里有 3 个。第三个是 **Cursor 自带的浏览器预览自动打开了 `http://127.0.0.1:5174/`**：

  ```
  $ lsof -nP -iTCP:5174 -sTCP:ESTABLISHED
  Cursor  84940  127.0.0.1:63736->127.0.0.1:5174
  node    74672  127.0.0.1:5174->127.0.0.1:63736
  ```

  这不是 bug，是这个 fixture 将来的常态——同事拿手机连上来时房里本来就不止两个人。断言改成「双方看得见自己和对方」，不再断言人数。
- 第三版把页面大小从 `pageBody.length`（字符数）改成 `arrayBuffer()` 的字节数，好和 `curl` 的 `size_download` 直接对上。中文页面上这两个数差 409，拿字符数去验「一个字节不动」是验不出来的。

### 意外收获：一个真的浏览器把这页跑起来了

上面那个 `轻盈树懒 23` 是 Cursor 的 Chromium 预览，它 `GET /` 之后自己连上了 socket.io、升级到 websocket，并且在整个测试过程中挂了六分多钟没掉线（engine.io 的心跳一直有人应答）。只有 `public/index.html` 里那段内联脚本会做这件事，所以可以说：**这一页的 JS 在真实 Chromium 里能加载、能建连、能升级、能保活。**

但这只到「脚本跑起来了」为止。**没有人看过屏幕**，所以画布画得对不对、状态条长什么样、涟漪好不好看、手指拖动跟不跟手，一概不知道。

页面本身另外只做了两项静态检查：

```
内联脚本编译通过，6951 字符
外部字体 / 脚本 / 样式：无
```

## 4. 收尾

`node_modules` 删掉，`package.json` 与 `pnpm-lock.yaml` 留在仓库里。这次执行环境里 `rm` 要交互确认而确认框弹不出来，所以实际删除用的是 Node：

```
$ node -e "require('fs').rmSync('…/ws-rooms/node_modules',{recursive:true,force:true})"
删除前存在: true
删除后存在: false
```

服务器停掉，端口确认已释放：

```
$ lsof -nP -iTCP:5174
（端口已释放）
```

剩下 5 个文件，合计 33435 B（32.7 KiB），`du -sh` 报 44 KB：

```
     9087  check.mjs           两个客户端的自检
      390  package.json        socket.io ^4 是唯一依赖，socket.io-client 是 devDependency
     7152  pnpm-lock.yaml
     9581  public/index.html   全屏画布 + 状态条，内联 CSS/JS，无外部资源
     7225  server.mjs          http 静态 + socket.io，node server.mjs [端口，默认 5174]
```

## 磁盘：这次是贴着地面飞的

开工时 `/System/Volumes/Data` 只剩 494 MB，刚好压在「低于 500 MB 就停」的红线之下（同一时间另一个会话在 `fixtures/gate-audio/` 装 Playwright，全程在 115 MB–1004 MB 之间大幅波动）。处理方式是先把源码写完（32 KB，可忽略），等可用空间回到 513 MB 的那一刻再装，并另外设了 150 MB 的硬下限：

```
装之前可用 513 MB
装之后可用 501 MB（净变化 -12 MB）
node_modules 8.4M
```

`socket.io` + `socket.io-client` 整套净增 12 MB。`du -sh node_modules` 报 8.4 MB，但 pnpm 在 APFS 上是从 store 克隆文件、块是共享的，这个数和 12 MB 不能直接相减。**没有跑 `pnpm store prune`**——store 是全机共享的，另一个会话正在用。

跑第二次 `check.mjs` 期间磁盘被另一个会话打到 115 MB，触发过一次 `No space left on device`（编辑器写临时文件失败）。`check.mjs` 本身不写盘，所以验证没受影响。后来另一个会话释放到 1004 MB，才补跑了改字节数之后的那一次；收工时 507 MB。

## 没验到的（这一条最重要）

1. **没穿过隧道。** 全程 `127.0.0.1` 直连。边缘和 CLI 那条路上会不会在 polling→websocket 升级这一步掉链子，这次一个字都没证明。真正的验收是同事把 `edge/` + `cli/` 跑起来之后执行 `node check.mjs https://<slug>.playtest.run`，14 条断言原样全绿。
2. **没在手机上开过。** viewport、`touch-action: none`、pointer 事件、安全区内边距全是照规矩写的，没有任何一台真机确认过。「两台手机能对局」还差这一整步。
3. **没有人看过这一页。** 见上面「意外收获」一节——脚本在 Chromium 里跑起来了，但渲染结果没人看过。`window.__pt` 也没有被无头 Chrome 真的读过一次，只是写在那里。
4. **只有一个房间、没有并发压力。** 默认房间写死 `main`，测的是 2–3 个连接。十个人同时在房间里、隧道单条 TCP 上的队头阻塞（DESIGN §4.3 那条缺点）都没碰。
5. **重连只测到客户端那一层。** `check.mjs` 掐的是 engine.io 的传输。隧道自己那条 1 s → 30 s 退避重连是另一层，得等 CLI 能跑才测得到。
6. **`HOST=0.0.0.0` 没试过。** 默认只监听 `127.0.0.1`（隧道要连的正是这个），局域网直连那条口子写了但没验，第一次用可能会撞上 macOS 的防火墙确认框。
