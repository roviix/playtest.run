// 生成 export/ 里那组用来检验边缘响应头的静态文件。
// 用法：node gen.mjs   （会先删掉 export/ 再整个重建）
//
// 检验的是 DESIGN §4.2 里列的四条：.wasm 的 MIME、.br/.gz 预压缩、
// --isolated 的 COOP/COEP、Range。

import { brotliCompressSync, gzipSync, constants as zlibConstants } from 'node:zlib';
import { mkdirSync, rmSync, writeFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const out = join(here, 'export');

// 一个最小的合法 wasm 模块，导出 add(i32, i32) -> i32。
// 手写字节，避免为了 41 个字节引入一条工具链依赖。
function buildWasm() {
  const section = (id, body) => [id, body.length, ...body];

  const header = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]; // "\0asm" + version 1
  // type: 一个函数类型 (i32, i32) -> i32
  const typeSec = section(0x01, [0x01, 0x60, 0x02, 0x7f, 0x7f, 0x01, 0x7f]);
  // function: 一个函数，用 0 号类型
  const funcSec = section(0x03, [0x01, 0x00]);
  // export: 名字 "add"，导出 0 号函数
  const exportSec = section(0x07, [0x01, 0x03, 0x61, 0x64, 0x64, 0x00, 0x00]);
  // code: local.get 0; local.get 1; i32.add; end
  const funcBody = [0x00, 0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b];
  const codeSec = section(0x0a, [0x01, funcBody.length, ...funcBody]);

  return Buffer.from([...header, ...typeSec, ...funcSec, ...exportSec, ...codeSec]);
}

// 一段确定性字节，用来测 Range：第 i 个字节是 i % 251。
// 251 是小于 256 的最大质数，跨 256 边界时不会和页大小对齐，取任意区间都不重样。
function buildDataBin(size) {
  const buf = Buffer.allocUnsafe(size);
  for (let i = 0; i < size; i++) buf[i] = i % 251;
  return buf;
}

const indexHtml = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>headers-lab · 边缘响应头自检</title>
<style>
  :root { color-scheme: light dark; }
  body {
    margin: 0 auto; padding: 24px 16px 64px; max-width: 46rem;
    font: 15px/1.6 -apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans SC", sans-serif;
  }
  h1 { font-size: 1.4rem; margin: 0 0 4px; }
  h2 { font-size: 1.1rem; margin: 32px 0 8px; }
  .sub { margin: 0 0 24px; opacity: .7; }
  ol { list-style: none; margin: 0; padding: 0; }
  li { border: 1px solid rgba(128,128,128,.35); border-radius: 10px; padding: 12px 14px; margin-bottom: 12px; }
  li.ok { border-color: rgba(46,160,67,.7); }
  li.bad { border-color: rgba(218,54,51,.7); }
  .title { font-weight: 600; }
  .status { margin-top: 4px; }
  .detail { margin: 8px 0 0; padding: 8px 10px; border-radius: 6px; white-space: pre-wrap; word-break: break-all;
            background: rgba(128,128,128,.12); font: 12px/1.5 ui-monospace, SFMono-Regular, Menlo, monospace; }
  .detail:empty { display: none; }
  button { font: inherit; padding: 10px 18px; border-radius: 8px; border: 1px solid rgba(128,128,128,.5);
           background: rgba(128,128,128,.12); cursor: pointer; }
  button:active { transform: translateY(1px); }
  code { font: 12px/1.5 ui-monospace, SFMono-Regular, Menlo, monospace; }
</style>
</head>
<body>
<h1>边缘响应头自检</h1>
<p class="sub">这一页检查当前服务器是否按 DESIGN §4.2 的约定发响应头。整个目录由 <code>fixtures/headers-lab/gen.mjs</code> 生成，页面本身不依赖任何外部文件。</p>

<ol id="checks"></ol>

<h2>5. 用户手势解锁音频</h2>
<p class="sub">门禁页「开始」那一下要能解锁 WebAudio，这里单独试一次。</p>
<button id="beep" type="button">点一下响一声</button>
<pre class="detail" id="audio-detail">还没点。</pre>

<script>
(function () {
  'use strict';

  var list = document.getElementById('checks');

  function add(title) {
    var li = document.createElement('li');
    var h = document.createElement('div');
    h.className = 'title';
    h.textContent = title;
    var s = document.createElement('div');
    s.className = 'status';
    s.textContent = '检查中…';
    var d = document.createElement('pre');
    d.className = 'detail';
    li.appendChild(h);
    li.appendChild(s);
    li.appendChild(d);
    list.appendChild(li);
    return {
      pass: function (msg) { s.textContent = '\\u2705 ' + msg; li.className = 'ok'; },
      fail: function (msg) { s.textContent = '\\u274C ' + msg; li.className = 'bad'; },
      note: function (msg) { d.textContent += msg + '\\n'; }
    };
  }

  function headerOf(res, name) {
    var v = res.headers.get(name);
    return v === null ? '(无)' : v;
  }

  // 1. mod.wasm 流式编译，要求 Content-Type: application/wasm
  (async function () {
    var c = add('1. mod.wasm 流式编译（要求 Content-Type: application/wasm）');
    try {
      var res = await fetch('mod.wasm');
      c.note('HTTP ' + res.status + ' ' + res.statusText);
      c.note('Content-Type: ' + headerOf(res, 'content-type'));
      c.note('Content-Length: ' + headerOf(res, 'content-length'));
      var mod = await WebAssembly.instantiateStreaming(res);
      var sum = mod.instance.exports.add(2, 3);
      if (sum === 5) {
        c.pass('流式编译成功，add(2, 3) = 5');
      } else {
        c.fail('编译成功但 add(2, 3) = ' + sum + '，应该是 5');
      }
    } catch (err) {
      c.fail('失败：' + err);
      c.note('最常见的原因是 Content-Type 不是 application/wasm。给成 application/octet-stream 时，');
      c.note('instantiateStreaming 会直接拒绝，不会退化成先下载再编译。');
    }
  })();

  // 2. Build/hello.wasm.br 预压缩，要求服务器带 Content-Encoding: br
  (async function () {
    var c = add('2. Build/hello.wasm.br 预压缩（要求 Content-Encoding: br，Unity 布局）');
    try {
      var res = await fetch('Build/hello.wasm.br');
      c.note('HTTP ' + res.status + ' ' + res.statusText);
      c.note('Content-Type: ' + headerOf(res, 'content-type'));
      c.note('Content-Encoding: ' + headerOf(res, 'content-encoding'));
      c.note('（浏览器解压之后常会把 Content-Encoding 从这里抹掉，所以上面这行显示「无」不一定代表服务器没发；');
      c.note('真正的判据是下面这次编译成不成。）');
      var mod = await WebAssembly.instantiateStreaming(res);
      var sum = mod.instance.exports.add(2, 3);
      if (sum === 5) {
        c.pass('解压 + 流式编译成功，add(2, 3) = 5');
      } else {
        c.fail('编译成功但 add(2, 3) = ' + sum + '，应该是 5');
      }
    } catch (err) {
      c.fail('失败：' + err);
      c.note('服务器要对 .wasm.br 同时发 Content-Type: application/wasm 和 Content-Encoding: br。');
      c.note('少了 Content-Encoding，浏览器拿到的就是没解压的 brotli 字节，magic number 对不上。');
    }
  })();

  // 3. Range 请求
  (async function () {
    var c = add('3. data.bin 的 Range 请求（要求 206 + Content-Range）');
    try {
      var res = await fetch('data.bin', { headers: { Range: 'bytes=0-9' } });
      c.note('请求头 Range: bytes=0-9');
      c.note('HTTP ' + res.status + ' ' + res.statusText);
      c.note('Content-Range: ' + headerOf(res, 'content-range'));
      c.note('Accept-Ranges: ' + headerOf(res, 'accept-ranges'));
      var bytes = new Uint8Array(await res.arrayBuffer());
      c.note('拿到 ' + bytes.length + ' 字节：[' + Array.prototype.join.call(bytes, ', ') + ']');
      var wanted = '0, 1, 2, 3, 4, 5, 6, 7, 8, 9';
      if (res.status !== 206) {
        c.fail('状态是 ' + res.status + '，不是 206，服务器把整个文件发回来了');
        c.note('引擎的大 .data / .pck 靠 Range 续传和跳读，整发会让首屏白等。');
      } else if (Array.prototype.join.call(bytes, ', ') !== wanted) {
        c.fail('206 但前 10 字节不对，期望 [' + wanted + ']');
      } else {
        c.pass('206，Content-Range 与前 10 字节都对');
      }
    } catch (err) {
      c.fail('失败：' + err);
    }
  })();

  // 4. 跨源隔离，对应 CLI 的 --isolated
  (async function () {
    var c = add('4. 跨源隔离（对应 --isolated 的 COOP/COEP）');
    var isolated = (typeof crossOriginIsolated !== 'undefined') && crossOriginIsolated === true;
    c.note('crossOriginIsolated: ' + (typeof crossOriginIsolated === 'undefined' ? '(这个浏览器没有这个属性)' : crossOriginIsolated));
    c.note('typeof SharedArrayBuffer: ' + typeof SharedArrayBuffer);
    try {
      var res = await fetch(location.href, { method: 'GET', cache: 'no-store' });
      c.note('本页 Cross-Origin-Opener-Policy: ' + headerOf(res, 'cross-origin-opener-policy'));
      c.note('本页 Cross-Origin-Embedder-Policy: ' + headerOf(res, 'cross-origin-embedder-policy'));
    } catch (err) {
      c.note('读本页响应头失败：' + err);
    }
    if (isolated && typeof SharedArrayBuffer === 'function') {
      c.pass('已跨源隔离，SharedArrayBuffer 可用');
    } else {
      c.fail('未跨源隔离，SharedArrayBuffer 不可用');
      c.note('这次如果没开 --isolated，这里就该是这样，不是 bug。');
      c.note('开了 --isolated 还是这样，才说明边缘没发 COOP: same-origin 与 COEP: require-corp——');
      c.note('Godot 4 默认的线程导出在这种情况下会直接报错打不开。');
    }
  })();

  // 5. 用户手势解锁 WebAudio
  var detail = document.getElementById('audio-detail');
  var ac = null;
  document.getElementById('beep').addEventListener('click', function () {
    var Ctx = window.AudioContext || window.webkitAudioContext;
    if (!Ctx) {
      detail.textContent = '\\u274C 这个浏览器没有 AudioContext。';
      return;
    }
    var lines = [];
    try {
      if (!ac) {
        ac = new Ctx();
        lines.push('刚创建 AudioContext，state = ' + ac.state);
      }
      var before = ac.state;
      ac.resume();
      var osc = ac.createOscillator();
      var gain = ac.createGain();
      osc.type = 'sine';
      osc.frequency.value = 660;
      var t = ac.currentTime;
      gain.gain.setValueAtTime(0.0001, t);
      gain.gain.exponentialRampToValueAtTime(0.2, t + 0.01);
      gain.gain.exponentialRampToValueAtTime(0.0001, t + 0.25);
      osc.connect(gain);
      gain.connect(ac.destination);
      osc.start(t);
      osc.stop(t + 0.26);
      lines.push('resume() 之前 state = ' + before);
      lines.push('resume() 之后 state = ' + ac.state);
      lines.push('sampleRate = ' + ac.sampleRate + ' Hz');
      lines.push(ac.state === 'running' ? '\\u2705 已解锁，应该听到一声 660 Hz。' : '\\u274C 还是 ' + ac.state + '，这一下手势没解锁。');
      detail.textContent = lines.join('\\n');
    } catch (err) {
      detail.textContent = '\\u274C 出错：' + err;
    }
  });
})();
</script>
</body>
</html>
`;

const subHtml = `<!doctype html>
<meta charset="utf-8">
<title>headers-lab · sub</title>
<p>这一行来自 /sub/index.html，用来测目录索引与 /sub 到 /sub/ 的重定向。</p>
`;

// ---- 生成 ----

rmSync(out, { recursive: true, force: true });
mkdirSync(join(out, 'Build'), { recursive: true });
mkdirSync(join(out, 'sub'), { recursive: true });

const wasm = buildWasm();

// 写盘之前先在本进程里跑一次，不合法就别产出。
const { instance } = await WebAssembly.instantiate(wasm, {});
const sum = instance.exports.add(2, 3);
if (sum !== 5) throw new Error(`wasm 自检失败：add(2, 3) = ${sum}，应为 5`);
console.log(`wasm 自检通过：${wasm.length} 字节，add(2, 3) = ${sum}`);

const brotli = (buf) =>
  brotliCompressSync(buf, {
    params: {
      [zlibConstants.BROTLI_PARAM_QUALITY]: 11,
      [zlibConstants.BROTLI_PARAM_SIZE_HINT]: buf.length,
    },
  });

writeFileSync(join(out, 'mod.wasm'), wasm);
writeFileSync(join(out, 'mod.wasm.br'), brotli(wasm));
writeFileSync(join(out, 'mod.wasm.gz'), gzipSync(wasm, { level: 9 }));
writeFileSync(join(out, 'Build', 'hello.wasm.br'), brotli(wasm));
writeFileSync(join(out, 'data.bin'), buildDataBin(1024 * 1024));
writeFileSync(join(out, 'index.html'), indexHtml);
writeFileSync(join(out, 'sub', 'index.html'), subHtml);

function walk(dir) {
  return readdirSync(dir, { withFileTypes: true })
    .sort((a, b) => a.name.localeCompare(b.name))
    .flatMap((e) => (e.isDirectory() ? walk(join(dir, e.name)) : [join(dir, e.name)]));
}

const files = walk(out);
let total = 0;
console.log(`\n生成 ${files.length} 个文件到 export/：`);
for (const f of files) {
  const size = statSync(f).size;
  total += size;
  console.log(`  ${String(size).padStart(9)}  ${relative(out, f)}`);
}
console.log(`  ${String(total).padStart(9)}  = 合计`);
