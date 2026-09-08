// 两个源：9301 是门禁页那一侧，9302 只用来做「跨源 iframe」里的游戏页。
// 9301 上多一个 POST /start → 303，形状和边缘的 /_playtest/start 一样。

import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = fileURLToPath(new URL('.', import.meta.url));
const PORTS = { main: Number(process.env.MAIN_PORT) || 9301, other: Number(process.env.OTHER_PORT) || 9302 };

const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.wav': 'audio/wav',
};

/** 只允许回到同源的相对路径，和边缘 same_origin_target 一个意思。 */
function safeTarget(raw) {
  if (!raw || !raw.startsWith('/') || raw.startsWith('//')) return '/game.html?v=v2b';
  return raw;
}

// 探针的结果走这里回收，不走 Playwright 的 evaluate——page.evaluate() 在 Chrome 里
// 会被当成一次用户手势（Playwright 的 CDP 调用带 userGesture: true），
// 测量还没做完就去 evaluate，等于自己给自己发了张通行证。
// 两个端口是同一个 node 进程里的两个 server，所以跨源 iframe 的结果也落在这一个数组里。
let reports = [];

const readBody = (req) =>
  new Promise((resolve) => {
    let s = '';
    req.on('data', (c) => (s += c));
    req.on('end', () => resolve(s));
  });

async function handle(req, res, role) {
  const url = new URL(req.url, `http://localhost:${PORTS[role]}`);

  if (req.method === 'POST' && url.pathname === '/start') {
    const to = safeTarget(new URLSearchParams(await readBody(req)).get('to'));
    res.writeHead(303, { location: to, 'cache-control': 'no-store' });
    res.end();
    return;
  }

  if (req.method === 'POST' && url.pathname === '/report') {
    try {
      reports.push(JSON.parse(await readBody(req)));
    } catch (e) {
      reports.push({ parseError: String(e) });
    }
    res.writeHead(204, { 'cache-control': 'no-store' }).end();
    return;
  }

  if (url.pathname === '/reports') {
    if (req.method === 'DELETE') reports = [];
    res.writeHead(200, { 'content-type': 'application/json', 'cache-control': 'no-store' });
    res.end(JSON.stringify(reports));
    return;
  }

  const rel = normalize(decodeURIComponent(url.pathname)).replace(/^(\.\.[/\\])+/, '');
  const file = join(ROOT, rel === '/' ? 'gate.html' : rel);
  if (!file.startsWith(ROOT)) {
    res.writeHead(403).end('nope');
    return;
  }
  try {
    const buf = await readFile(file);
    res.writeHead(200, {
      'content-type': TYPES[extname(file)] || 'application/octet-stream',
      'cache-control': 'no-store',
    });
    res.end(buf);
  } catch {
    res.writeHead(404, { 'content-type': 'text/plain; charset=utf-8' }).end('404');
  }
}

for (const [role, port] of Object.entries(PORTS)) {
  createServer((req, res) => {
    handle(req, res, role).catch((e) => {
      res.writeHead(500).end(String(e));
    });
  }).listen(port, '127.0.0.1', () => console.log(`${role} → http://localhost:${port}/`));
}
