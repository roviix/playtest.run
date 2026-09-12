import assert from "node:assert/strict";
import { createHash, randomUUID } from "node:crypto";
import { createServer } from "node:http";
import { createServer as createTcpServer } from "node:net";
import { spawn, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { resolve, extname } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
mkdirSync(resolve(root, ".data"), { recursive: true });
const data = mkdtempSync(resolve(root, ".data/collections-check-"));
const children = [];
const keep = process.argv.includes("--keep");
const shotDirectory = resolve(root, "docs/spikes/img");
const stamp = `2026-09-12-collections-${new Date().toISOString().replace(/[^0-9]/g, "").slice(8, 14)}`;
const sleep = milliseconds => new Promise(accept => setTimeout(accept, milliseconds));
async function until(check, label, timeout = 15000) {
  const limit = Date.now() + timeout;
  while (Date.now() < limit) {
    try { const result = await check(); if (result) return result; } catch {}
    await sleep(80);
  }
  throw new Error(`等待超时：${label}`);
}
async function freePort() {
  const server = createTcpServer();
  await new Promise(accept => server.listen(0, "127.0.0.1", accept));
  const port = server.address().port;
  await new Promise(accept => server.close(accept));
  return port;
}
function launch(command, args, environment = {}) {
  const child = spawn(command, args, { cwd: root, env: { ...process.env, ...environment }, stdio: ["ignore", "pipe", "pipe"] });
  const lines = [];
  child.stdout.on("data", bytes => lines.push(bytes.toString()));
  child.stderr.on("data", bytes => lines.push(bytes.toString()));
  children.push(child);
  child.on("error", error => { lines.push(String(error)); });
  child.logs = lines;
  return child;
}
function sql(statement) {
  const result = spawnSync("sqlite3", [resolve(data, "api.sqlite"), statement], { encoding: "utf8" });
  assert.equal(result.status, 0, result.stderr);
  return result.stdout.trim();
}
const quote = value => `'${String(value).replaceAll("'", "''")}'`;
const hash = value => createHash("sha256").update(value).digest("hex");
let connection;
let consoleServer;
try {
  const apiPort = await freePort();
  const edgePort = await freePort();
  const apiBase = `http://127.0.0.1:${apiPort}`;
  const playerBase = `http://localhost:${edgePort}`;
  const apiProcess = launch(resolve(root, "target/debug/playtest-api"), [], {
    PLAYTEST_DATA_DIR: data, PLAYTEST_API_LISTEN: `127.0.0.1:${apiPort}`,
    PLAYTEST_EMAIL_PROVIDER: "log", PLAYTEST_PUBLIC_ROOT_URL: playerBase,
    PLAYTEST_SITE_URL_TEMPLATE: `http://{slug}.localhost:${edgePort}`,
    PLAYTEST_ADMIN_TOKEN: "", PLAYTEST_GITHUB_CLIENT_ID: "", PLAYTEST_GITHUB_CLIENT_SECRET: "",
  });
  await until(async () => (await fetch(`${apiBase}/health`)).ok, "控制面启动").catch(error => { throw new Error(`${error}\n${apiProcess.logs.join("")}`); });
  const edgeProcess = launch(resolve(root, "target/debug/playtest-edge"), [], {
    PLAYTEST_DATA_DIR: data, PLAYTEST_HOST_SUFFIX: "localhost", PLAYTEST_EDGE_LISTEN: `127.0.0.1:${edgePort}`,
    PLAYTEST_API_INTERNAL_URL: apiBase, PLAYTEST_API_PUBLIC_URL: apiBase,
  });
  await until(async () => (await fetch(`${playerBase}/`)).ok, "玩家站启动").catch(error => { throw new Error(`${error}\n${edgeProcess.logs.join("")}`); });
  const tokens = [0, 1, 2].map(index => {
    const user = `local-demo-${index}`;
    const token = randomUUID();
    const now = new Date().toISOString().replace(/\.\d{3}Z$/, "Z");
    sql(`INSERT INTO users(id,kind,display_name,created_at) VALUES(${quote(user)},'github',${quote(["本地演示 · 小雨", "本地演示 · 阿木", "本地演示 · 沈青"][index])},${quote(now)}); INSERT INTO tokens(token_hash,user_id,created_at) VALUES(${quote(hash(token))},${quote(user)},${quote(now)});`);
    return token;
  });
  async function api(method, path, token, body) {
    const response = await fetch(`${apiBase}${path}`, { method, headers: { "content-type": "application/json", authorization: `Bearer ${token}` }, body: body === undefined ? undefined : JSON.stringify(body) });
    const text = await response.text();
    assert.ok(response.ok, `${method} ${path}: ${response.status} ${text}`);
    return text ? JSON.parse(text) : null;
  }
  const source = readFileSync(resolve(root, "fixtures/pelican-bicycle/export/index.html"), "utf8");
  const works = [];
  for (const [index, title] of ["今天，骑去海边", "慢一点也没关系", "夜色里的第一圈", "追上一阵晚风", "一只鹈鹕的周末", "骑到太阳落下"].entries()) {
    const token = tokens[Math.floor(index / 2)];
    const slug = `local-pelican-${index}`;
    const bytes = Buffer.from(source.replaceAll("今天，骑去海边", title).replaceAll("--sky:#1c2930", `--sky:${["#1c2930", "#27332f", "#26283b", "#342d32", "#25343b", "#353225"][index]}`));
    await api("POST", "/v1/projects", token, { slug, title });
    const work = { slug, title, token, bytes };
    works.push(work);
  }
  async function publish(work, cover) {
    const files = [{ path: "index.html", hash: hash(work.bytes), size: work.bytes.length }];
    const prepared = await api("POST", `/v1/projects/${work.slug}/uploads`, work.token, { files, title: work.title, summary: "原创本地演示动画，非模型评测结果。点击可以暂停或换一片天空。", gate: "once", isolated: false, spa: false, ...(cover ? { cover: { hash: hash(cover), mime: "image/png", size: cover.length } } : {}) });
    for (const missing of prepared.missing) {
      const bytes = missing === hash(work.bytes) ? work.bytes : cover;
      const response = await fetch(`${apiBase}/v1/blobs/${missing}`, { method: "PUT", headers: { authorization: `Bearer ${work.token}` }, body: bytes });
      assert.ok(response.ok, await response.text());
    }
    await api("POST", `/v1/projects/${work.slug}/uploads/${prepared.upload_id}/commit`, work.token, undefined);
    await api("PATCH", `/v1/projects/${work.slug}`, work.token, { public: true });
  }
  for (const work of works) await publish(work);
  const chrome = process.env.CHROME ?? "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
  assert.ok(existsSync(chrome), "需要 Chrome，可用 CHROME 指定可执行文件。");
  const browserDirectory = resolve(data, "chrome");
  launch(chrome, ["--headless=new", "--no-first-run", "--no-default-browser-check", "--remote-debugging-port=0", `--user-data-dir=${browserDirectory}`, "about:blank"]);
  const debugPort = await until(() => {
    const path = resolve(browserDirectory, "DevToolsActivePort");
    return existsSync(path) && Number(readFileSync(path, "utf8").split("\n")[0]);
  }, "Chrome 启动");
  const target = await (await fetch(`http://127.0.0.1:${debugPort}/json/new?about:blank`, { method: "PUT" })).json();
  connection = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((accept, reject) => { connection.onopen = accept; connection.onerror = reject; });
  let sequence = 0;
  const pending = new Map();
  const exceptions = [];
  connection.onmessage = event => {
    const reply = JSON.parse(event.data);
    if (reply.id) {
      const completion = pending.get(reply.id); if (!completion) return;
      pending.delete(reply.id); reply.error ? completion.reject(new Error(JSON.stringify(reply.error))) : completion.accept(reply.result);
    } else if (reply.method === "Runtime.exceptionThrown") exceptions.push(reply.params.exceptionDetails);
  };
  function command(method, params = {}) {
    return new Promise((accept, reject) => { const id = ++sequence; pending.set(id, { accept, reject }); connection.send(JSON.stringify({ id, method, params })); });
  }
  async function evaluate(expression) {
    const result = await command("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
    if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
    return result.result.value;
  }
  await command("Page.enable"); await command("Runtime.enable");
  async function navigate(url) {
    await command("Page.navigate", { url });
    await until(() => evaluate(`document.readyState === 'complete' && location.href.startsWith(${JSON.stringify(url.split("#")[0])})`), url);
  }
  async function click(selector) {
    const rect = await evaluate(`(() => { const element = document.querySelector(${JSON.stringify(selector)}); if (!element) return null; element.scrollIntoView({block:'center'}); const bounds = element.getBoundingClientRect(); return {x:bounds.x+bounds.width/2,y:bounds.y+bounds.height/2}; })()`);
    assert.ok(rect, `控件不存在：${selector}`);
    await command("Input.dispatchMouseEvent", { type: "mousePressed", ...rect, button: "left", clickCount: 1 });
    await command("Input.dispatchMouseEvent", { type: "mouseReleased", ...rect, button: "left", clickCount: 1 });
  }
  async function fill(selector, value) {
    await click(selector);
    await command("Input.dispatchKeyEvent", { type: "keyDown", key: "a", code: "KeyA", modifiers: 4 });
    await command("Input.dispatchKeyEvent", { type: "keyUp", key: "a", code: "KeyA", modifiers: 4 });
    await command("Input.insertText", { text: value });
  }
  async function viewport(width, height) { await command("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false }); }
  async function screenshot(name) {
    const shot = await command("Page.captureScreenshot", { format: "png", captureBeyondViewport: false });
    const path = resolve(shotDirectory, `${stamp}-${name}.png`); writeFileSync(path, Buffer.from(shot.data, "base64")); return path;
  }
  await viewport(600, 470);
  for (const work of works) {
    const origin = `http://${work.slug}.localhost:${edgePort}`;
    await command("Network.setCookie", { name: "pt_gate", value: "1", url: origin });
    await navigate(origin);
    await until(() => evaluate("!!document.querySelector('svg .bird')"), "原创动画加载");
    await evaluate("document.body.classList.add('pause')");
    const cover = await command("Page.captureScreenshot", { format: "png" });
    await publish(work, Buffer.from(cover.data, "base64"));
  }
  const challenge = await api("POST", "/v1/collections", tokens[0], { slug: "pelican-bicycle", title: "鹈鹕骑单车", kind: "challenge", summary: "同一道题，看看大家能骑出多少种答案。这里的数据仅供本地演示。", prompt: "请用一个 HTML 文件做一只鹈鹕骑单车的动画。让轮子转起来，让它像真的在骑。你也可以加入自己的小巧思。", rules: "只提交自己有权发布的浏览器作品。\n请如实填写创作过程；不设模型名次。\n本地演示作品不是模型能力测试样本。", public: true });
  for (const work of works.slice(0, 5)) await api("POST", `/v1/collections/${challenge.slug}/entries`, work.token, { slug: work.slug, model: "本地演示 · 非模型输出", method: "edited", prompt: "用于验证平台体验的原创演示素材。" });
  await api("POST", "/v1/collections", tokens[1], { slug: "weekend-collection", title: "周末做的两个小东西", kind: "collection", summary: "一个人的创意编程练习。本地演示作品集。", public: true });
  await api("POST", "/v1/collections/weekend-collection/entries", tokens[1], { slug: works[2].slug });
  await api("POST", "/v1/collections/weekend-collection/entries", tokens[1], { slug: works[3].slug });
  consoleServer = createServer(async (request, response) => {
    try {
      if (request.url.startsWith("/v1/")) {
        const buffers = []; for await (const bytes of request) buffers.push(bytes);
        const upstream = await fetch(`${apiBase}${request.url}`, { method: request.method, headers: { "content-type": "application/json", authorization: request.headers.authorization ?? "" }, body: ["GET", "HEAD"].includes(request.method) ? undefined : Buffer.concat(buffers) });
        response.writeHead(upstream.status, { "content-type": upstream.headers.get("content-type") ?? "application/json" }); response.end(Buffer.from(await upstream.arrayBuffer())); return;
      }
      const path = new URL(request.url, "http://localhost").pathname;
      const relative = path === "/" || path === "/console/" ? "index.html" : path.replace(/^\/console\//, "");
      const file = resolve(root, "console/dist", relative);
      if (!file.startsWith(resolve(root, "console/dist") + "/")) { response.writeHead(404); response.end(); return; }
      const mime = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css" }[extname(file)] ?? "application/octet-stream";
      response.writeHead(200, { "content-type": mime }); response.end(readFileSync(file));
    } catch { response.writeHead(404); response.end("not found"); }
  });
  await new Promise(accept => consoleServer.listen(0, "127.0.0.1", accept));
  const consoleBase = `http://127.0.0.1:${consoleServer.address().port}/console/`;
  await sleep(31000);
  const shots = [];
  await viewport(1440, 1040); await navigate(playerBase);
  await until(() => evaluate("!!document.querySelector('.collection-card')"), "首页挑战入口");
  assert.equal(await evaluate("document.querySelectorAll('.tile').length"), 6);
  shots.push(await screenshot("plaza-desktop"));
  await click('a.collection-card[href="/c/pelican-bicycle"]');
  await until(() => evaluate("!!document.querySelector('#challenge-prompt')"), "挑战页");
  assert.equal(await evaluate("document.querySelectorAll('.collection-entries article').length"), 5);
  await click('[data-copy-prompt]');
  await until(() => evaluate("document.querySelector('[data-collection-status]').textContent.includes('复制')"), "复制题目状态");
  shots.push(await screenshot("challenge-desktop"));
  await fill('input[name="q"]', "慢一点"); await click('.discover-search button[type="submit"]');
  await until(() => evaluate("document.querySelectorAll('.collection-entries article').length===1"), "无脚本兼容搜索提交");
  assert.ok(await evaluate("location.search.includes('q=')"));
  await click('.collection-entries .tile');
  await until(() => evaluate("!!document.querySelector('.collection-context')"), "邀请函合集上下文");
  assert.ok(await evaluate("document.querySelector('.collection-context').innerText.includes('下一件')"));
  await navigate(`${playerBase}/c/pelican-bicycle`); await viewport(390, 844);
  assert.ok(await evaluate("document.documentElement.scrollWidth<=innerWidth+1"), "挑战页手机布局溢出");
  shots.push(await screenshot("challenge-mobile"));
  await command("Emulation.setScriptExecutionDisabled", { value: true });
  await navigate(`${playerBase}/c/pelican-bicycle?q=${encodeURIComponent("晚风")}`);
  assert.equal(await evaluate("document.querySelectorAll('.collection-entries article').length"), 1);
  await command("Emulation.setScriptExecutionDisabled", { value: false });
  await navigate(consoleBase);
  await evaluate(`localStorage.setItem('playtest.token',${JSON.stringify(tokens[2])}); location.hash='#/collections/pelican-bicycle'; location.reload()`);
  await until(() => evaluate("document.body.innerText.includes('把我的作品放进来')"), "控制台真实令牌读取");
  await viewport(1440, 1040);
  await evaluate(`(() => {const select=document.querySelector('.collection-panel select'); select.value='local-pelican-5';select.dispatchEvent(new Event('change',{bubbles:true}));})()`);
  await click('.collection-panel form button[type="submit"]');
  await until(() => evaluate("document.body.innerText.includes('作品已加入')"), "通过控制台提交作品");
  shots.push(await screenshot("console-submit"));
  const actual = await api("GET", "/v1/collections/pelican-bicycle", tokens[2]);
  assert.equal(actual.entries.length, 6);
  await viewport(390, 844);
  assert.ok(await evaluate("document.documentElement.scrollWidth<=innerWidth+1"), "控制台手机布局溢出");
  shots.push(await screenshot("console-mobile"));
  await navigate(`${playerBase}/c/pelican-bicycle`);
  await click('.collection-actions details.tell>summary');
  await fill('input[type="email"]', "viewer@example.com");
  await click('.collection-actions details.tell button[type="submit"]');
  await until(() => evaluate("document.body.innerText.includes('确认')"), "关注结果");
  const notification = sql("SELECT body FROM notifications WHERE kind='confirm' ORDER BY id DESC LIMIT 1");
  const confirmation = notification.match(/http:\/\/localhost:\d+\/me\/confirm\/[^\s]+/)[0];
  await navigate(confirmation);
  await until(() => evaluate("location.pathname==='/me' && document.body.innerText.includes('鹈鹕骑单车')"), "确认后关注页显示合集");
  await navigate(`${playerBase}/c/pelican-bicycle`);
  await until(() => evaluate("document.body.innerText.includes('已关注')"), "已关注状态");
  shots.push(await screenshot("subscribed-mobile"));
  assert.equal(exceptions.length, 0, JSON.stringify(exceptions));
  const report = { date: new Date().toISOString(), data, playerBase, consoleBase, shots, checks: ["真实数据库迁移与上传", "合集聚合与基础搜索", "桌面与 390px 布局", "关闭 JavaScript 后搜索", "复制题目与反馈", "邀请函返回合集和下一件", "控制台真实投稿", "邮箱确认与根域关注状态"], exceptions };
  writeFileSync(resolve(data, "report.json"), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report, null, 2));
  if (keep) {
    console.log("本地演示保持运行；按 Ctrl-C 关闭本次启动的服务。未部署、未寄出真实邮件。");
    await new Promise(accept => { process.once("SIGINT", accept); process.once("SIGTERM", accept); });
  }
} finally {
  connection?.close();
  consoleServer?.close();
  for (const child of children.reverse()) child.kill("SIGTERM");
}
