// ws-rooms —— 一个房间的联机小游戏，给隧道路径（DESIGN §4.3）当靶子。
// 用法：node server.mjs [端口，默认 5174]
//
// 这个 fixture 的重点不是好玩，是「隧道最容易坏的地方都在这条路径上」：
// socket.io 默认的 polling → websocket 升级、Host 被改写之后还能不能握手、
// 二进制帧和文本帧能不能原样过、断线之后客户端会不会自己回来。

import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Server } from 'socket.io';

const PORT = parsePort(process.argv[2]);
const HOST = process.env.HOST ?? '127.0.0.1';
const PUBLIC_DIR = resolve(fileURLToPath(new URL('./public/', import.meta.url)));
const ROOM = 'main';

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.ico': 'image/x-icon',
  '.png': 'image/png',
};

const ADJECTIVES = ['敏捷', '安静', '莽撞', '闪亮', '倔强', '轻盈', '滚烫', '好奇', '锋利', '柔软', '嘹亮', '迟钝'];
const ANIMALS = ['水獭', '海雀', '狐狸', '鲸鱼', '刺猬', '山猫', '鹦鹉', '树懒', '章鱼', '麋鹿', '蜂鸟', '獾'];

/** @type {Map<string, {id: string, name: string, color: string, x: number, y: number}>} */
const players = new Map();
let joinSeq = 0;

// ---------------------------------------------------------------- HTTP 静态

const httpServer = createServer(async (req, res) => {
  // socket.io 的 attach 会把自己的 /socket.io/ 请求截在这个 handler 之前，
  // 所以这里看到的都是页面和静态资源。
  logHeaders(req);

  const pathname = safePathname(req.url);
  if (pathname === null) {
    return send(res, 400, 'text/plain; charset=utf-8', '路径不合法');
  }

  const filePath = pathname === '/' ? join(PUBLIC_DIR, 'index.html') : join(PUBLIC_DIR, pathname);
  // join 之后再确认没跑出 public/，防目录穿越
  if (filePath !== PUBLIC_DIR && !filePath.startsWith(PUBLIC_DIR + sep)) {
    return send(res, 403, 'text/plain; charset=utf-8', '越界');
  }

  try {
    const body = await readFile(filePath);
    const type = MIME[extname(filePath).toLowerCase()] ?? 'application/octet-stream';
    // 这是开发用 fixture，改一行就要立刻看到，不给缓存
    res.setHeader('Cache-Control', 'no-store');
    send(res, 200, type, body);
  } catch {
    send(res, 404, 'text/plain; charset=utf-8', `没有这个文件：${pathname}`);
  }
});

// ---------------------------------------------------------------- socket.io

// 不设 transports —— 保留默认的 polling 起手再升级到 websocket。
// 这一步正是隧道最容易坏的地方，写死成 websocket 就把考题跳过去了。
const io = new Server(httpServer, {
  serveClient: true,
  cors: { origin: false },
});

io.engine.on('connection_error', (err) => {
  log(`× 握手失败  code=${err.code}  ${err.message}  url=${err.req?.url ?? '?'}`);
});

io.on('connection', (socket) => {
  const h = socket.handshake.headers;
  const player = {
    id: socket.id,
    name: makeName(++joinSeq),
    color: makeColor(joinSeq),
    x: 0.5,
    y: 0.5,
  };
  players.set(socket.id, player);
  socket.join(ROOM);

  const firstTransport = socket.conn.transport.name;
  log(
    `+ ${player.name} 进房 (${socket.id})  传输=${firstTransport}` +
      `  Host=${h.host ?? '(无)'}  X-Forwarded-Host=${h['x-forwarded-host'] ?? '(无)'}`,
  );

  socket.conn.on('upgrade', (transport) => {
    log(`↑ ${player.name} 传输升级 ${firstTransport} → ${transport.name}`);
  });

  socket.emit('welcome', { you: player, serverTime: Date.now() });
  broadcastPlayers();

  socket.on('move', (p) => {
    player.x = clamp01(p?.x);
    player.y = clamp01(p?.y);
    socket.to(ROOM).emit('move', { id: player.id, x: player.x, y: player.y });
  });

  socket.on('tap', (p) => {
    // 连自己的涟漪也从服务器绕一圈回来，隧道慢的时候手感上直接看得见
    io.to(ROOM).emit('tap', { id: player.id, x: clamp01(p?.x), y: clamp01(p?.y), color: player.color });
  });

  // 前端收到 tick 之后打这个回来量往返。ack 里带 8 字节 Buffer，
  // socket.io 会把它编成独立的二进制帧——顺带验隧道的二进制透传。
  socket.on('echo', (clientSent, bytes, ack) => {
    if (typeof ack !== 'function') return;
    const serverTime = Date.now();
    const stamp = Buffer.alloc(8);
    stamp.writeDoubleLE(serverTime);
    ack({ clientSent, serverTime, binIn: byteLengthOf(bytes), stamp });
  });

  socket.on('disconnect', (reason) => {
    players.delete(socket.id);
    log(`- ${player.name} 离开 (${reason})  剩 ${players.size} 人`);
    broadcastPlayers();
  });
});

setInterval(() => {
  io.to(ROOM).emit('tick', { serverTime: Date.now(), players: players.size });
}, 1000);

httpServer.listen(PORT, HOST, () => {
  console.log(`ws-rooms 已启动：http://${HOST}:${PORT}  （socket.io 挂在同一端口）`);
  console.log(`自检：node check.mjs http://${HOST}:${PORT}`);
  console.log(`隧道：playtest ${PORT}；下面每条日志都会带上 Host 与 X-Forwarded-Host`);
  if (HOST === '127.0.0.1') {
    console.log('只监听本机。要让同一 Wi-Fi 下的手机直连，用 HOST=0.0.0.0 node server.mjs');
  }
});

// ---------------------------------------------------------------- 工具

function broadcastPlayers() {
  io.to(ROOM).emit('players', [...players.values()]);
}

function logHeaders(req) {
  const h = req.headers;
  log(`${req.method} ${req.url}  Host=${h.host ?? '(无)'}  X-Forwarded-Host=${h['x-forwarded-host'] ?? '(无)'}`);
}

function send(res, status, type, body) {
  res.writeHead(status, { 'Content-Type': type });
  res.end(body);
}

function safePathname(rawUrl) {
  try {
    // base 只是为了能解析相对路径，不参与寻址
    const { pathname } = new URL(rawUrl ?? '/', 'http://ws-rooms.invalid');
    const decoded = decodeURIComponent(pathname);
    return decoded.includes('\0') ? null : decoded;
  } catch {
    return null;
  }
}

function clamp01(v) {
  return typeof v === 'number' && Number.isFinite(v) ? Math.min(1, Math.max(0, v)) : 0.5;
}

function byteLengthOf(v) {
  if (v instanceof ArrayBuffer) return v.byteLength;
  if (ArrayBuffer.isView(v)) return v.byteLength;
  return 0;
}

function makeName(seq) {
  const a = ADJECTIVES[(seq * 5) % ADJECTIVES.length];
  const b = ANIMALS[(seq * 7) % ANIMALS.length];
  return `${a}${b} ${10 + ((seq * 13) % 90)}`;
}

function makeColor(seq) {
  // 黄金角步进，连续进房的人颜色不会撞
  return `hsl(${Math.round((seq * 137.508) % 360)} 72% 58%)`;
}

function parsePort(arg) {
  if (arg === undefined) return 5174;
  const n = Number(arg);
  if (!Number.isInteger(n) || n < 1 || n > 65535) {
    console.error(`端口不合法：${arg}。用法：node server.mjs [端口，默认 5174]`);
    process.exit(2);
  }
  return n;
}

function log(msg) {
  console.log(`${new Date().toISOString().slice(11, 23)} ${msg}`);
}
