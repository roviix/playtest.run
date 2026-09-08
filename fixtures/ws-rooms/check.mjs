// 自检：起两个 socket.io 客户端，把「两台手机能对局」拆成能断言的几条。
// 用法：node check.mjs [地址，默认 http://127.0.0.1:5174]
//
// 直连本机跑一遍是基线；同一条命令指到 https://<slug>.playtest.run 就是隧道路径的验收。

import { io } from 'socket.io-client';

const BASE = (process.argv[2] ?? 'http://127.0.0.1:5174').replace(/\/+$/, '');
const STEP_TIMEOUT = 15000;

let failed = 0;

function ok(name, detail = '') {
  console.log(`PASS  ${name}${detail ? '  ' + detail : ''}`);
}

function bad(name, detail = '') {
  failed++;
  console.log(`FAIL  ${name}${detail ? '  ' + detail : ''}`);
}

function assert(cond, name, detail = '') {
  cond ? ok(name, detail) : bad(name, detail);
  return cond;
}

/** 等一个事件等到满足 predicate，超时就抛（抛出来的信息要说清等的是什么）。 */
function waitFor(emitter, event, predicate = () => true, what = event, ms = STEP_TIMEOUT) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      emitter.off(event, onEvent);
      reject(new Error(`等 ${what} 超时（${ms} ms）`));
    }, ms);
    function onEvent(...args) {
      let hit = false;
      try {
        hit = predicate(...args);
      } catch {
        hit = false;
      }
      if (!hit) return;
      clearTimeout(timer);
      emitter.off(event, onEvent);
      resolve(args.length > 1 ? args : args[0]);
    }
    emitter.on(event, onEvent);
  });
}

function connect(label) {
  const socket = io(BASE, { reconnectionDelay: 200, reconnectionDelayMax: 500 });
  socket.label = label;
  // players 是状态不是事件：等的时候多半已经收到过了，只等下一条会死等
  socket.lastPlayers = null;
  socket.on('players', (list) => {
    socket.lastPlayers = list;
  });
  return socket;
}

function waitPlayers(socket, predicate, what) {
  if (socket.lastPlayers && predicate(socket.lastPlayers)) return Promise.resolve(socket.lastPlayers);
  return waitFor(socket, 'players', predicate, what);
}

function stampOf(v) {
  if (v instanceof ArrayBuffer) return new DataView(v).getFloat64(0, true);
  if (ArrayBuffer.isView(v)) return new DataView(v.buffer, v.byteOffset, v.byteLength).getFloat64(0, true);
  return NaN;
}

const sockets = [];

try {
  console.log(`目标 ${BASE}\n`);

  // ---- 1. HTTP：页面与 engine.io 握手 -------------------------------------

  const pageRes = await fetch(`${BASE}/`);
  // 数字节不数字符：DESIGN §4.3 要求隧道对响应体一个字节不动，中文页面上这两个数差得远
  const pageBytes = new Uint8Array(await pageRes.arrayBuffer());
  const pageText = new TextDecoder().decode(pageBytes);
  assert(
    pageRes.status === 200 && pageText.includes('<canvas'),
    '页面可取',
    `status=${pageRes.status} ${pageBytes.byteLength} 字节`,
  );

  const hsRes = await fetch(`${BASE}/socket.io/?EIO=4&transport=polling`);
  const hsBody = await hsRes.text();
  const sid = hsBody.match(/"sid"\s*:\s*"([^"]+)"/)?.[1];
  assert(hsRes.status === 200 && !!sid, 'engine.io polling 握手', `status=${hsRes.status} sid=${sid ?? '(没有)'}`);

  const clientRes = await fetch(`${BASE}/socket.io/socket.io.js`);
  assert(
    clientRes.status === 200 && (clientRes.headers.get('content-type') ?? '').includes('javascript'),
    '/socket.io/socket.io.js 由服务器提供',
    `status=${clientRes.status} type=${clientRes.headers.get('content-type')}`,
  );

  // ---- 2. 两个客户端进同一个房间 ------------------------------------------

  const a = connect('A');
  const b = connect('B');
  sockets.push(a, b);

  // io() 返回时 engine 和第一个 transport 已经建好、升级探测还没跑完，
  // 所以这里同步读到的才是「起手用的是什么」。
  const aFirstTransport = a.io.engine?.transport?.name ?? '(读不到)';
  const upgrades = new Map();
  for (const s of [a, b]) {
    s.io.engine?.on('upgrade', (t) => upgrades.set(s.label, t.name));
  }

  // 四个监听器要在等任何一个之前全挂好：走公网时 B 的 connect / welcome 常在 A 的还没等到时就到了，
  // 晚挂监听器会把已经发生的事件丢掉（本机直连太快，暴露不出这个竞态）。
  const aConnected = waitFor(a, 'connect', () => true, 'A 连上');
  const bConnected = waitFor(b, 'connect', () => true, 'B 连上');
  const aWelcome = waitFor(a, 'welcome', () => true, 'A 的 welcome');
  const bWelcome = waitFor(b, 'welcome', () => true, 'B 的 welcome');
  await aConnected;
  const meA = (await aWelcome).you;
  await bConnected;
  const meB = (await bWelcome).you;

  assert(aFirstTransport === 'polling', '起手是 polling（没跳过升级这一关）', `transport=${aFirstTransport}`);
  assert(
    !!meA?.name && !!meA?.color && meA.id !== meB.id,
    '各自拿到昵称和颜色',
    `A=${meA?.name}/${meA?.color}  B=${meB?.name}/${meB?.color}`,
  );

  // ---- 3. 升级到 websocket ------------------------------------------------

  for (const s of [a, b]) {
    if (s.io.engine.transport.name !== 'websocket') {
      await waitFor(s.io.engine, 'upgrade', () => true, `${s.label} 升级到 websocket`);
    }
  }
  assert(
    a.io.engine.transport.name === 'websocket' && b.io.engine.transport.name === 'websocket',
    'polling → websocket 升级完成',
    `A=${a.io.engine.transport.name} B=${b.io.engine.transport.name}` +
      `，升级事件 ${[...upgrades.entries()].map(([k, v]) => `${k}→${v}`).join(' ') || '（没捕到）'}`,
  );

  // ---- 4. players：双方都看得见对方 ---------------------------------------

  // 只断言「互相看得见」，不断言房里正好两个人——同事拿手机连上来时房里就不止两个
  const aSeesB = await waitPlayers(a, (list) => list.some((p) => p.id === meB.id), 'A 看到 B');
  const bSeesA = await waitPlayers(b, (list) => list.some((p) => p.id === meA.id), 'B 看到 A');
  assert(
    aSeesB.some((p) => p.id === meA.id) && bSeesA.some((p) => p.id === meB.id),
    '双方 players 里都有自己和对方',
    `房里 ${aSeesB.length} 人：${aSeesB.map((p) => p.name).join('、')}`,
  );

  // ---- 5. move 与 tap 互通 ------------------------------------------------

  const moved = waitFor(a, 'move', (m) => m.id === meB.id, 'A 收到 B 的 move');
  b.emit('move', { x: 0.25, y: 0.75 });
  const m = await moved;
  assert(
    Math.abs(m.x - 0.25) < 1e-9 && Math.abs(m.y - 0.75) < 1e-9,
    'move 坐标原样到达',
    `收到 x=${m.x} y=${m.y}`,
  );

  const tappedOnB = waitFor(b, 'tap', (t) => t.id === meA.id, 'B 收到 A 的 tap');
  const tappedOnA = waitFor(a, 'tap', (t) => t.id === meA.id, 'A 收到自己的 tap 回声');
  a.emit('tap', { x: 0.1, y: 0.9 });
  const [tb] = await Promise.all([tappedOnB, tappedOnA]);
  assert(Math.abs(tb.x - 0.1) < 1e-9 && tb.color === meA.color, 'tap 广播到全房间', `color=${tb.color}`);

  // ---- 6. tick 驱动的往返，以及 ack 里的二进制帧 ---------------------------

  const rtts = [];
  let binOk = false;
  let binIn = -1;
  for (let i = 0; i < 3; i++) {
    await waitFor(a, 'tick', (t) => Number.isFinite(t.serverTime), '服务器 tick');
    const sent = performance.now();
    const reply = await new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error('等 echo 的 ack 超时')), STEP_TIMEOUT);
      a.emit('echo', sent, new Uint8Array([1, 2, 3, 4]), (r) => {
        clearTimeout(timer);
        resolve(r);
      });
    });
    rtts.push(performance.now() - sent);
    binIn = reply.binIn;
    binOk = stampOf(reply.stamp) === reply.serverTime;
  }
  const avg = rtts.reduce((s, v) => s + v, 0) / rtts.length;
  assert(
    rtts.every((v) => Number.isFinite(v) && v >= 0),
    'tick 的往返算得出来',
    `三次 ${rtts.map((v) => v.toFixed(1)).join(' / ')} ms，平均 ${avg.toFixed(1)} ms`,
  );
  assert(binOk && binIn === 4, '二进制帧双向原样过', `上行 ${binIn} 字节，下行 8 字节时间戳解回一致=${binOk}`);

  // ---- 7. 断线重连 --------------------------------------------------------

  const oldId = meA.id;
  const back = waitFor(a, 'connect', () => true, 'A 自动重连');
  a.io.engine.close();
  await back;
  const rejoined = await waitPlayers(
    b,
    (list) => !list.some((p) => p.id === oldId) && list.some((p) => p.id === a.id),
    'B 看到 A 换了身份回来',
  );
  assert(a.connected && a.id !== oldId, '断线后客户端自己回来了', `旧 id=${oldId} 新 id=${a.id}`);
  assert(
    rejoined.some((p) => p.id === a.id),
    '重连后的 A 重新出现在 players 里',
    `房里 ${rejoined.length} 人：${rejoined.map((p) => p.name).join('、')}`,
  );

  // ---- 8. 离开就移出 ------------------------------------------------------

  const aId = a.id;
  const gone = waitPlayers(b, (list) => !list.some((p) => p.id === aId), 'B 看到 A 离开');
  a.close();
  const left = await gone;
  assert(
    !left.some((p) => p.id === aId) && left.some((p) => p.id === meB.id),
    '断开就移出房间',
    `剩 ${left.map((p) => p.name).join('、')}`,
  );
} catch (err) {
  bad('执行中断', err instanceof Error ? err.message : String(err));
} finally {
  for (const s of sockets) s.close();
}

console.log(failed === 0 ? '\n全部通过' : `\n${failed} 条没过`);
process.exit(failed === 0 ? 0 : 1);
