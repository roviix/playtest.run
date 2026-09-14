import { createServer, request as forward } from 'node:http';
import { spawn } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, existsSync, createWriteStream } from 'node:fs';
import { resolve, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
mkdirSync(resolve(root, '.data'), { recursive: true });
const data = process.env.PLAYTEST_LOCAL_DATA ? resolve(root, process.env.PLAYTEST_LOCAL_DATA) : mkdtempSync(resolve(root, '.data/unified-'));
const basePort = Number(process.env.PORT || 48550);
const origin = `http://localhost:${basePort}`;
const apiPort = basePort + 1;
const edgePort = basePort + 2;
const githubPort = basePort + 3;
const children = [];
const servers = [];
let stopping = false;
const stop = () => { if (stopping) return; stopping = true; children.forEach(child => child.kill('SIGINT')); servers.forEach(server => server.close()); setTimeout(() => process.exit(), 1000).unref(); };
process.on('SIGINT', stop); process.on('SIGTERM', stop);

function launch(binary, environment) {
  const child = spawn(resolve(root, `target/debug/${binary}`), [], { cwd: root, env: { ...process.env, ...environment }, stdio: ['ignore', 'pipe', 'pipe'] });
  const log = createWriteStream(resolve(data, `${binary}.log`));
  child.stdout.pipe(log); child.stderr.pipe(log);

  let buffer = '';
  const monitor = (chunk) => {
    buffer += chunk.toString();
    if (buffer.includes('---- 信到这里为止 ----') || (buffer.includes('email_token=') && buffer.includes('\n\n'))) {
      const mail = buffer;
      buffer = '';
      const urlMatch = mail.match(/https?:\/\/[^\s]+/);
      console.log('\n' + '━'.repeat(64));
      console.log('📬  【本地模拟邮件】已生成登录链接：');
      for (const line of mail.split('\n')) {
        const trimmed = line.trim();
        if (trimmed.startsWith('发件人:') || trimmed.startsWith('收件人:') || trimmed.startsWith('主题:')) {
          console.log(`    ${trimmed}`);
        }
      }
      if (urlMatch) {
        console.log(`\n👉  点击或复制链接直接登录（可 Cmd/Ctrl 点击）：`);
        console.log(`    \x1b[36m\x1b[1m${urlMatch[0]}\x1b[0m`);
      }
      console.log('━'.repeat(64) + '\n');
    }
    if (buffer.length > 50000) buffer = buffer.slice(-10000);
  };
  child.stdout.on('data', monitor);
  child.stderr.on('data', monitor);

  child.on('exit', code => { if (!stopping) { console.error(`${binary} stopped: ${code}; logs: ${data}`); stop(); } });
  children.push(child);
}

const fakeGithub = createServer(async (request, response) => {
  const url = new URL(request.url, `http://localhost:${githubPort}`);
  response.setHeader('content-type', 'application/json');
  if (url.pathname === '/login/oauth/authorize') {
    const callback = new URL(url.searchParams.get('redirect_uri'));
    callback.searchParams.set('code', 'local-test-code'); callback.searchParams.set('state', url.searchParams.get('state'));
    response.writeHead(302, { location: callback.href }); response.end();
  } else if (url.pathname === '/login/oauth/access_token') { response.end(JSON.stringify({ access_token: 'local-test-github-token' })); }
  else if (url.pathname === '/user') response.end(JSON.stringify({ id: 854242, login: 'local-creator', name: '本地验收 · 创作者' }));
  else { response.writeHead(404); response.end('{}'); }
});
await new Promise(accept => fakeGithub.listen(githubPort, '127.0.0.1', accept)); servers.push(fakeGithub);

launch('playtest-api', { PLAYTEST_API_LISTEN: `127.0.0.1:${apiPort}`, PLAYTEST_DATA_DIR: data, PLAYTEST_SITE_URL_TEMPLATE: `http://{slug}.localhost:${basePort}`, PLAYTEST_STORAGE_BACKEND: 'fs', PLAYTEST_EMAIL_PROVIDER: 'log', PLAYTEST_PUBLIC_ROOT_URL: origin, PLAYTEST_CONSOLE_URL: `${origin}/console/`, PLAYTEST_GITHUB_CLIENT_ID: 'local-fixture', PLAYTEST_GITHUB_CLIENT_SECRET: 'local-fixture', PLAYTEST_GITHUB_BASE_URL: `http://127.0.0.1:${githubPort}`, PLAYTEST_EDGE_INGEST_TOKEN: 'local-unified-ingest-test-only-00000' });
launch('playtest-edge', { PLAYTEST_EDGE_LISTEN: `127.0.0.1:${edgePort}`, PLAYTEST_DATA_DIR: data, PLAYTEST_HOST_SUFFIX: 'localhost', PLAYTEST_PUBLIC_SCHEME: 'http', PLAYTEST_STORAGE_BACKEND: 'fs', PLAYTEST_API_INTERNAL_URL: `http://127.0.0.1:${apiPort}`, PLAYTEST_API_PUBLIC_URL: origin, PLAYTEST_EDGE_INGEST_TOKEN: 'local-unified-ingest-test-only-00000' });

const server = createServer((request, response) => {
  const host = (request.headers.host || '').split(':')[0];
  if (host === '127.0.0.1') {
    response.writeHead(302, { location: `http://localhost:${basePort}${request.url}` });
    response.end();
    return;
  }
  const url = new URL(request.url, origin);
  const platform = host === 'localhost';
  if (platform && url.pathname.startsWith('/console/')) {
    const relative = decodeURIComponent(url.pathname.slice('/console/'.length));
    const directory = resolve(root, 'console/dist');
    let file = resolve(directory, relative || 'index.html');
    if (!file.startsWith(directory + '/')) { response.writeHead(403); response.end(); return; }
    if (!existsSync(file)) file = resolve(directory, 'index.html');
    const types = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml' };
    response.writeHead(200, { 'content-type': types[extname(file)] || 'application/octet-stream', 'cache-control': 'no-store', 'referrer-policy': 'no-referrer' }); response.end(readFileSync(file)); return;
  }
  if (platform && url.pathname === '/console') { response.writeHead(302, { location: '/console/' }); response.end(); return; }
  const port = platform && url.pathname.startsWith('/v1/') ? apiPort : edgePort;
  const upstream = forward({ hostname: '127.0.0.1', port, path: request.url, method: request.method, headers: request.headers }, incoming => { response.writeHead(incoming.statusCode, incoming.headers); incoming.pipe(response); });
  upstream.on('error', () => { if (!response.headersSent) response.writeHead(503); response.end('服务正在启动，请稍后重试。'); });
  request.pipe(upstream);
});
await new Promise(accept => server.listen(basePort, '127.0.0.1', accept)); servers.push(server);
const deadline = Date.now() + 20000;
let ready = false;
while (Date.now() < deadline) {
  try { if ((await fetch(`http://127.0.0.1:${basePort}/v1/account`)).ok) { ready = true; break; } } catch {}
  await new Promise(accept => setTimeout(accept, 100));
}
if (!ready) { console.error(`服务没有启动成功，请检查 ${data} 中的日志。`); stop(); process.exitCode = 1; }
console.log(`
┌─────────────────────────────────────────────────────────────┐
│  🚀 playtest.run 本地统一开发环境已就绪                      │
├─────────────────────────────────────────────────────────────┤
│  • 控制台地址:   \x1b[32m${origin}/console/\x1b[0m
│  • 发现与主站:   \x1b[32m${origin}/\x1b[0m
│  • 本地数据目录: ${data}
│
│  【登录方式说明】
│  1. 邮箱登录: 页面输入任意邮箱，登录链接将直接在下方终端打印
│  2. GitHub:   本地替身已就绪，点击「使用 GitHub 继续」直接登录
└─────────────────────────────────────────────────────────────┘
`);
