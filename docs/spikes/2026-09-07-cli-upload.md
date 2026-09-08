# 2026-09-07 · `playtest <目录>` 在本机跑通

**结论**：`playtest ./dist` 这条命令在本机走通了 DESIGN §4.2 的全部四步——申请匿名会话、建作品、
问服务器缺哪些哈希、只传缺的、提交清单，拿到 `vN` 和链接；终端里画出二维码；同一个目录再跑一次
`missing` 为空，一个字节都不传，直接出 `v2`。`ls / rm / open` 也走通了。**隧道模式没做**，
`playtest 5173` 明说这一点并以退出码 2 结束。

机器：macOS 26.2，rustc 1.96.0 (ac68faa20 2026-05-25)。全部命令从仓库根目录执行。
控制面是同一台机器上跑着的 `playtest-api`（`127.0.0.1:8787`，别人起的那一个），
导出物用 `fixtures/` 里现成的两个。

下面第二、三、五、六、七节的输出是把 stdout 和 stderr 分别通过管道抓下来再合并的，
两条流之间的先后不保证——链接和它前后的空行会错位。终端里的真实形态见第四节。

## 一、还没做的功能，命令自己说

```
$ cargo run -q -p playtest -- 5173
隧道模式（把本地端口接出去）还没做好，现在只支持上传目录，例如：playtest ./dist
退出码 2

$ cargo run -q -p playtest -- login
登录还没做好。现在每次运行拿到的是匿名链接，24 小时后失效。
退出码 2
```

参数是纯数字但不在 1–65535 时不当成端口，也不当成「还没做好」，而是用法错（退出码 1）：

```
$ cargo run -q -p playtest -- 70000
端口号要在 1 到 65535 之间，「70000」不是。如果这是一个目录的名字，写成 ./70000。
退出码 1
```

## 二、第一次上传

`fixtures/vite-vanilla/export` 是七个文件、45.1 KB 的 Vite 导出物。

```
$ PLAYTEST_API=http://127.0.0.1:8787 cargo run -q -p playtest -- \
    ./fixtures/vite-vanilla/export -n "小球试玩" -m "第一次真机跑通"
正在整理 ./fixtures/vite-vanilla/export：7 个文件，45.1 KB
需要上传 7 个文件（45.1 KB），其余 0 个服务器上已有
已发布 v1


http://zesty-macaw-46.localhost:8443
这是匿名链接，2026-09-08 12:09 后失效。保留、改名、查看结果需要登录（登录还没做好）。
退出码 0
```

stdout 不是终端，所以没画二维码。到期时间显示的是本地时间——控制面给的是
`2026-09-08T04:09:03Z`，这台机器在 UTC+8。

控制面那边的日志对得上，已经被记进 `2026-09-07-api-upload-flow.md` 第七节：

```
04:09:03  建了一个作品 slug=zesty-macaw-46
04:09:03  准备上传 slug=zesty-macaw-46 files=7 missing=7 missing_bytes=46185
04:09:03  提交了一个版本 slug=zesty-macaw-46 version=1 file_count=7 total_bytes=46185
```

## 三、第二次上传：内容没变就一个字节都不传

不带任何参数再跑一遍同一个目录。作品沿用上次那个（配置文件记住了目录 → slug）：

```
$ PLAYTEST_API=http://127.0.0.1:8787 cargo run -q -p playtest -- ./fixtures/vite-vanilla/export
正在整理 ./fixtures/vite-vanilla/export：7 个文件，45.1 KB
7 个文件服务器上都已经有了，不用传。
已发布 v2


这是匿名链接，2026-09-08 12:09 后失效。保留、改名、查看结果需要登录（登录还没做好）。
http://zesty-macaw-46.localhost:8443
退出码 0
```

（链接跑到最后一行，就是上面说的两条流错位；终端里不会这样，见下一节。）

## 四、终端里的样子（含二维码）

上面几次都是从管道里捕获的，stdout 不是终端所以不画二维码。用伪终端跑一次看真实形态：

```
$ PLAYTEST_API=http://127.0.0.1:8787 python3 -c \
    "import pty; pty.spawn(['./target/debug/playtest','./fixtures/vite-vanilla/export','-m','真终端里看一眼'])" \
    | tr -d '\r'
正在整理 ./fixtures/vite-vanilla/export：7 个文件，45.1 KB
7 个文件服务器上都已经有了，不用传。
已发布 v3

http://zesty-macaw-46.localhost:8443

                                     
                                     
    █▀▀▀▀▀█ █ ▄█▀█ ▀ ██▀█ █▀▀▀▀▀█    
    █ ███ █ ▄▀▄▀▀ ▄▄█▄ ▄▀ █ ███ █    
    █ ▀▀▀ █  ▀ ▄▄▄▀▀▄▄▄█▀ █ ▀▀▀ █    
    ▀▀▀▀▀▀▀ █▄▀ █ ▀ █▄█▄▀ ▀▀▀▀▀▀▀    
    █ ██▄█▀▀ ▀▄▄▀█▄██▀██▀▄█ ▄▀ ▀█    
     ▀▀█▄▀▀█ █▄█▄▀ ▄█ ▄▄███ ▄▄▀▀▄    
    █▄ ▀  ▀▀▀▄ ▀▄ █▀▀█ ▄█ ▄▀ ▀▀▄▄    
    ▀▄ ▀▄▀▀ █▀▀▄▄▀▄▄▀  ▀█▀ ▄▄▀▀█▀    
    ▀▀▄ █▄▀▀█▀ ▀  █ █▄▀▀▀▀ ▄▀ ▄█     
    ▀ ▀██▀▀█▀▄ █▀███ ▄ ▄▀█ █ ▀█      
     ▀   ▀▀▀█▄ █▀▀▀▀ ████▀▀▀███▄▄    
    █▀▀▀▀▀█ ██▄ █▄█  ▀ ▀█ ▀ █▀ ▀▄    
    █ ███ █ ▄██▄   ██▀ ▄██▀▀█▀▀▀▄    
    █ ▀▀▀ █ ▀▄▄█▄▀█  ▀▀▀▄▄ █▄█▀▄▀    
    ▀▀▀▀▀▀▀ ▀  ▀▀   ▀ ▀▀▀▀▀    ▀     
                                     
                                     
手机扫码就能玩。
这是匿名链接，2026-09-08 12:09 后失效。保留、改名、查看结果需要登录（登录还没做好）。
```

二维码没有用手机扫过——这台机器上没有别的设备可用，`*.localhost` 手机也解析不到（KICKOFF §3）。
真机扫码要等域名，另记。

链接单独走 stdout，别的都走 stderr，所以 `| pbcopy` 拿到的就是链接本身：

```
$ PLAYTEST_API=http://127.0.0.1:8787 cargo run -q -p playtest -- ./fixtures/vite-vanilla/export 2>/dev/null
http://zesty-macaw-46.localhost:8443
```

## 五、认出导出物的特征

`fixtures/headers-lab/export` 里有 `.br` / `.gz` 和一段用到 `SharedArrayBuffer` 的脚本。
没加 `--isolated` 时会提醒：

```
$ PLAYTEST_API=http://127.0.0.1:8787 cargo run -q -p playtest -- ./fixtures/headers-lab/export -n "响应头实验"
正在整理 ./fixtures/headers-lab/export：7 个文件，1.0 MB
提醒：这个导出用到了 SharedArrayBuffer（线程），加 --isolated 才能运行。
检测到预压缩文件，会按原样带 Content-Encoding 返回。
需要上传 7 个文件（1.0 MB），其余 0 个服务器上已有
已发布 v1


http://nimble-robin-87.localhost:8443
这是匿名链接，2026-09-08 12:09 后失效。保留、改名、查看结果需要登录（登录还没做好）。
退出码 0
```

加上 `--isolated` 之后那句提醒就没有了；这次还顺带验证了跨作品去重——同样的字节刚才传过，
新作品一个文件都不用再传：

```
$ PLAYTEST_API=http://127.0.0.1:8787 cargo run -q -p playtest -- ./fixtures/headers-lab/export \
    --new --isolated --spa --gate always -n "响应头实验" --no-qr
正在整理 ./fixtures/headers-lab/export：7 个文件，1.0 MB
检测到预压缩文件，会按原样带 Content-Encoding 返回。
7 个文件服务器上都已经有了，不用传。
已发布 v1


http://lucky-gecko-68.localhost:8443
这是匿名链接，2026-09-08 12:09 后失效。保留、改名、查看结果需要登录（登录还没做好）。
退出码 0
```

目录里没有 `index.html` 时警告一声，但照传（`/tmp/pt-noindex` 里只有一个 `main.js`）：

```
$ PLAYTEST_API=http://127.0.0.1:8787 cargo run -q -p playtest -- /tmp/pt-noindex --no-qr
正在整理 /tmp/pt-noindex：1 个文件，52 B
提醒：目录里没有 index.html，链接打开会是 404；引擎导出物一般都有。
需要上传 1 个文件（52 B），其余 0 个服务器上已有
已发布 v1


http://rapid-beaver-50.localhost:8443
这是匿名链接，2026-09-08 12:09 后失效。保留、改名、查看结果需要登录（登录还没做好）。
退出码 0
```

（上面这三个作品里的后两个是为这条记录临时建的，跑完用 `playtest rm -y` 删掉了。）

## 六、出错时说什么

`/tmp/pt-spike-x/` 下放一个 `index.html` 和一个空目录 `empty/`：

```
$ cargo run -q -p playtest -- /tmp/pt-spike-x/nope
找不到 /tmp/pt-spike-x/nope。检查一下路径；如果还没构建，先构建出这个目录。
退出码 1

$ cargo run -q -p playtest -- /tmp/pt-spike-x/index.html
/tmp/pt-spike-x/index.html 是一个文件。请给目录，不是文件——引擎导出的那个文件夹，里面通常有 index.html。
退出码 1

$ cargo run -q -p playtest -- /tmp/pt-spike-x/empty
/tmp/pt-spike-x/empty 是空的，没有可以上传的文件。（以「.」开头的文件和目录不会上传。）
退出码 1
```

连不上控制面。第一条是端口上没人听，第二条是把地址指到一个不会回包的网段，
用来验 10 秒的连接超时（这一条真的等了 10 秒）：

```
$ PLAYTEST_API=http://127.0.0.1:1 cargo run -q -p playtest -- ./fixtures/vite-vanilla/export
正在整理 ./fixtures/vite-vanilla/export：7 个文件，45.1 KB
连不上服务器（http://127.0.0.1:1）：Connection refused (os error 61)。检查网络，或用 --api 指定地址。
退出码 1

$ PLAYTEST_API=http://10.255.255.1:8787 cargo run -q -p playtest -- ./fixtures/vite-vanilla/export
正在整理 ./fixtures/vite-vanilla/export：7 个文件，45.1 KB
连不上服务器（http://10.255.255.1:8787）：等了 10 秒没连上。检查网络，或用 --api 指定地址。
退出码 1
```

## 七、`ls` / `rm` / `open`

```
$ PLAYTEST_API=http://127.0.0.1:8787 cargo run -q -p playtest -- ls
响应头实验（v1）
  http://nimble-robin-87.localhost:8443
  slug：nimble-robin-87
  2026-09-08 12:09 后失效

export（v3）
  http://zesty-macaw-46.localhost:8443
  slug：zesty-macaw-46
  2026-09-08 12:09 后失效

退出码 0
```

不是终端的时候不会闷头删，也不会卡在那里等输入：

```
$ PLAYTEST_API=http://127.0.0.1:8787 cargo run -q -p playtest -- rm nimble-robin-87
这里不是终端，没法问你确认。确定要删就加 -y：playtest rm nimble-robin-87 -y
退出码 1

$ PLAYTEST_API=http://127.0.0.1:8787 cargo run -q -p playtest -- rm nimble-robin-87 -y
已删掉 nimble-robin-87。
退出码 0
```

在伪终端里回答 `n`（开头那个孤零零的 `n` 是伪终端把输入回显了一遍，真终端里是先出问句）：

```
$ echo n | PLAYTEST_API=http://127.0.0.1:8787 python3 -c \
    "import pty,sys; sys.exit(pty.spawn(['./target/debug/playtest','rm','zesty-macaw-46']))"
n
要删掉 zesty-macaw-46 吗？删了它的链接就打不开了。输入 y 确认：没有删。
```

`open` 可以给 slug，也可以给发过的目录。下面这条打印了链接，并且真的在 macOS 上弹出了浏览器：

```
$ PLAYTEST_API=http://127.0.0.1:8787 cargo run -q -p playtest -- open ./fixtures/vite-vanilla/export
http://zesty-macaw-46.localhost:8443
退出码 0

$ PLAYTEST_API=http://127.0.0.1:8787 cargo run -q -p playtest -- open ./fixtures/phaser-jump/export
./fixtures/phaser-jump/export 这个目录还没发过。先运行 playtest ./fixtures/phaser-jump/export 把它发出去。
退出码 1
```

## 八、配置文件

`~/.config/playtest/config.json`，权限 0600（里面有令牌）：

```
$ ls -l ~/.config/playtest/config.json
-rw-------@ 1 zhongshangwu  staff  262 Sep  7 12:15 /Users/zhongshangwu/.config/playtest/config.json

$ cat ~/.config/playtest/config.json
{
  "api": "http://127.0.0.1:8787",
  "token": "…",
  "token_expires_at": "2026-09-08T04:09:03Z",
  "sites": {
    "/Users/zhongshangwu/workspace/github/playtest.run/fixtures/vite-vanilla/export": "zesty-macaw-46"
  }
}
```

（令牌原文这里抹掉了。`playtest rm` 之后，指向那个 slug 的目录记录也一起没了。）

## 九、自动化测试

```
$ cargo test -p playtest
running 34 tests
test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

running 8 tests
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.51s
```

集成测试（`cli/tests/upload_flow.rs`）用 axum 起一个假控制面绑 `127.0.0.1:0`，跑的是真的
`playtest` 二进制。假服务器的路由直接用 `playtest_common::api::routes` 里的常量注册，
路径写错了测试就挂。覆盖的分支里有两条真机上不好造的：

- **记住的作品过期**：服务器对配置里那个 slug 回 401 `token_expired`，CLI 换新会话、
  建新作品、打印「上次的匿名链接已过期（24 小时），这是一个新链接。」再重来一遍。
- **`hash_mismatch`**：服务器一律拒收，CLI 重算哈希确认文件没变、重试到第三次为止，
  然后报「上传时文件变了……」并且**不**提交，退出码 1。

## 没验证的

- **手机真机扫码**。二维码画出来了，但没有第二台设备扫过；`*.localhost` 手机也解析不到，
  要等域名（KICKOFF §3）。DESIGN §8「手机上打开自己的作品不到 60 秒」这一句还不能指到这里。
- **大文件与断点**。最大只传过 1 MB 的目录。并发 4 路、指数退避重试、`Body::wrap_stream`
  的流式读都只在假服务器和小文件上跑过；200 MB 上限、传到一半断网这两条路径没有真机记录。
- **进度条**。到目前为止每次上传都在一秒内结束，进度条基本没露面，没有真机上看它走完的记录。
- **Windows 与 Linux**。只在 macOS 上跑过。`%APPDATA%` 配置路径、`cmd /c start` 和
  `xdg-open` 三条分支都只有代码。反斜杠路径的单元测试在 macOS 上验的是「反斜杠被拒」，
  Windows 上「反斜杠被换成正斜杠」那一条没跑过。
- **源码目录提示**。「这看起来是源码目录」只在文件数超过 5000 时才出，真机上没造出这个条件，
  只有单元测试。
- **登录之后的路径**。`--site` 指定别人的作品、免费档 500 MB 上限，都还没有入口。

## 已知的不够好

- **帮助和用法错里混着英文**。我们自己写的每一句都是中文，但 clap 的骨架是英文：
  `Usage:` / `Commands:` / `Options:` / `Print help` / `error: invalid value … For more
  information, try '--help'.`。要全中文得自己接管 clap 的帮助渲染，这次没做。
- **重跑时会悄悄改作品名**。作品名每次上传都跟着请求走，不带 `-n` 就用目录名。所以第一次
  `-n "小球试玩"`、第二次不带，门禁页上的名字就变回了目录名 `export`。KICKOFF 第三周做改名时
  要重新想这里该不该覆盖。
