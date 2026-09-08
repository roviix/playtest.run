# 2026-09-08 · 上传那一刻的检查：三个真导出物加一个源码目录（本机真机）

**结论**：`cli/src/inspect/` 对着真控制面跑了四个目录，说的话都对得上文件里的事实：
Vite、Phaser 被认出来；`headers-lab` 被认成 Godot（有 `.wasm` + `SharedArrayBuffer` 字样，
这是已知的误认，见末尾）并且**自动加上了 `--isolated`**、话里说清「wasm 的字节里没看出来，这是猜的」；
把 `fixtures/ws-rooms`（一个 Node 源码目录，`index.html` 在 `public/` 下）当导出物传时，
被**拦下**：退出码 6，指出该传哪一层，`--force` 才放行。

这次改了一处行为：原实现对「传上去一定打不开」级别（`blocker`）的发现只提醒、照传。
改成默认拦下——匿名作品只有三个名额，一个 404 的链接会白占一个；`--force` 是逃生口。
`cli/tests/inspect.rs` 里对应那条用例随之改写，10 条全过。

## 一、三个真导出物（`--json` 里的 `findings`）

```
$ playtest ./fixtures/vite-vanilla/export --api http://127.0.0.1:8790 --json --no-qr -n vite-vanilla
ok=True url=http://violet-macaw-29.localhost:8446 v1
  note | 看起来是 Vite 做的

$ playtest ./fixtures/phaser-jump/export … -n phaser-jump
ok=True url=http://witty-walrus-67.localhost:8446 v1
  note | 看起来是 Phaser 做的

$ playtest ./fixtures/headers-lab/export … -n headers-lab
ok=True url=http://wise-mink-28.localhost:8446 v1
  note | 看起来是 Godot 做的
  note | 目录里有压好的 .br / .gz，会按原样直接给玩家，由浏览器解压
  note | 这个构建可能用了多线程（index.html 里提到 SharedArrayBuffer，但 wasm 的字节里没看出来，这是猜的），已经自动加上 --isolated（跨源隔离） | hint: 不想要就加 --no-isolated
```

人类模式（stderr）同一份话一条一行，最后「7 个文件服务器上都已经有了，不用传。已发布 v2」。

## 二、源码目录被拦下

```
$ playtest ./fixtures/ws-rooms --api http://127.0.0.1:8790 --no-qr
正在整理 ./fixtures/ws-rooms：5 个文件，33.0 KB
最外层没有 index.html，它在 public/index.html 里，玩家点开链接会是 404
把 public 这一层直接传上来：playtest <刚才那个目录>/public。确定要照传就加 --force。
退出码 6

$ playtest ./fixtures/ws-rooms … --json
{"ok":false,"code":"bad_input","message":"最外层没有 index.html，它在 public/index.html 里，玩家点开链接会是 404","hint":"把 public 这一层直接传上来：playtest <刚才那个目录>/public。确定要照传就加 --force。", …}

$ playtest ./fixtures/ws-rooms … --force
已发布（indigo-parrot-58）
```

拦下时没有向控制面发任何请求（集成测试断言 `prepared` 为空）。

## 三、已知的不准

- `headers-lab` 不是 Godot 导出物，只是有 `.wasm` 和 `SharedArrayBuffer` 字样。认引擎不改行为，认错只是多说一句；
  但「自动加 `--isolated`」是改行为的，这里它加对了（自检页确实要 `crossOriginIsolated`）——靠的是 `SharedArrayBuffer`
  字样而不是引擎判断。真 Godot 4 导出物到手后要复核 `wasm::memory_kind` 那条「硬证据」路径。
- 只测了四个目录；Unity 的两条互斥压缩路（`.br` 与 decompression fallback）只有集成测试里的合成样本。
