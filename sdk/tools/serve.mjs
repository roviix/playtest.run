// 手动试 SDK 用的静态服务：dev/index.html 出在 /，构建产物出在 /playtest.js。
//
// 为什么不用现成的静态服务器：多一个依赖，而这里只要三十行。为什么必须从
// `<slug>.localhost` 打开：写入端点只认玩家域下的来源（api 的 events.rs），
// 而且 SDK 拿不到 /_playtest/me 时用域名第一段当 slug。

import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';

const port = Number(process.env.PORT || 38080);
const files = {
  '/': ['dev/index.html', 'text/html; charset=utf-8'],
  '/playtest.js': ['dist/playtest.js', 'text/javascript; charset=utf-8'],
};

createServer(async (req, res) => {
  const hit = files[new URL(req.url, 'http://x').pathname];
  if (!hit) {
    res.writeHead(404, { 'content-type': 'text/plain; charset=utf-8' });
    res.end('没有这个地址\n');
    return;
  }
  try {
    const body = await readFile(hit[0]);
    res.writeHead(200, { 'content-type': hit[1], 'cache-control': 'no-store' });
    res.end(body);
  } catch (err) {
    res.writeHead(500, { 'content-type': 'text/plain; charset=utf-8' });
    res.end(`${hit[0]} 读不出来：先跑一次 pnpm build\n`);
  }
}).listen(port, '127.0.0.1', () => {
  console.log(`打开 http://<你的-slug>.localhost:${port}/`);
});
