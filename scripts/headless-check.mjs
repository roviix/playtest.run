#!/usr/bin/env node
// 用无头 Chrome 走一遍玩家路径：门禁页 → 真实鼠标点「开始」→ 作品本体，抄下页面文字、截图、收 Console 报错。
// 只用 Node 22 自带的 fetch 与 WebSocket，不装依赖。写 docs/spikes/ 时用它，不是产品的一部分。
//
// 用法：
//   node scripts/headless-check.mjs <url> [--settle <点击前等多久 ms>] [--click "<css 选择器>"] [--wait <点击后等多久 ms>] [--shot <png 路径>] [--viewport 1380x900]
// 先要有一个开着远程调试的无头 Chrome：
//   "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new --remote-debugging-port=9222 \
//     --user-data-dir=/tmp/pt-chrome --window-size=420,860 --hide-scrollbars about:blank
//
// 每次运行都是新标签页，cookie 在同一个 user-data-dir 里共享，所以「第二次打开不再出门禁页」也能验。

const args = process.argv.slice(2);
const url = args.find((a) => !a.startsWith("--"));
if (!url) {
  console.error("缺少 url");
  process.exit(2);
}
const opt = (name, def) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : def;
};
const clickSel = opt("--click", null);
const settleMs = Number(opt("--settle", "300"));
const waitMs = Number(opt("--wait", "2500"));
const shot = opt("--shot", null);
const debugPort = opt("--port", "9222");
// 默认按手机竖屏看；审桌面版面时传 --viewport 1380x900。
const [vw, vh] = opt("--viewport", "420x860").split("x").map(Number);

const log = (s) => process.stderr.write(`[headless-check] ${s}\n`);
// 任何一步卡住都不该让脚本挂着：整体最多 45 秒。
setTimeout(() => {
  log("总超时（45 s），放弃");
  process.exit(1);
}, 45000).unref();

const target = await (await fetch(`http://127.0.0.1:${debugPort}/json/new?about:blank`, { method: "PUT" })).json();
log(`新标签 ${target.id}`);
const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  ws.onopen = resolve;
  ws.onerror = (e) => reject(new Error(`连不上 DevTools：${e.message ?? e}`));
});
log("DevTools 已连接");

let seq = 0;
const pending = new Map();
const events = [];
const consoleLines = [];
ws.onmessage = (m) => {
  const msg = JSON.parse(m.data);
  if (msg.id && pending.has(msg.id)) {
    pending.get(msg.id)(msg);
    pending.delete(msg.id);
  } else if (msg.method) {
    events.push(msg);
    if (msg.method === "Runtime.consoleAPICalled") {
      consoleLines.push(`[console.${msg.params.type}] ${msg.params.args.map((a) => a.value ?? a.description ?? "").join(" ")}`);
    } else if (msg.method === "Runtime.exceptionThrown") {
      consoleLines.push(`[exception] ${msg.params.exceptionDetails.text} ${msg.params.exceptionDetails.exception?.description ?? ""}`);
    } else if (msg.method === "Log.entryAdded") {
      consoleLines.push(`[${msg.params.entry.source}/${msg.params.entry.level}] ${msg.params.entry.text} ${msg.params.entry.url ?? ""}`);
    }
  }
};
const send = (method, params = {}) =>
  new Promise((resolve, reject) => {
    const id = ++seq;
    pending.set(id, (msg) => (msg.error ? reject(new Error(`${method}: ${msg.error.message}`)) : resolve(msg.result)));
    ws.send(JSON.stringify({ id, method, params }));
  });
const waitEvent = (method, timeout = 10000) =>
  new Promise((resolve, reject) => {
    const t = setTimeout(() => reject(new Error(`等 ${method} 超时`)), timeout);
    const tick = () => {
      const i = events.findIndex((e) => e.method === method);
      if (i >= 0) {
        clearTimeout(t);
        resolve(events.splice(i, 1)[0]);
      } else setTimeout(tick, 20);
    };
    tick();
  });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const evalJs = async (expression) => (await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true })).result.value;

await send("Page.enable");
await send("Runtime.enable");
await send("Log.enable");
await send("Emulation.setDeviceMetricsOverride", { width: vw, height: vh, deviceScaleFactor: 1, mobile: vw < 700 });

log(`打开 ${url}`);
await send("Page.navigate", { url });
await waitEvent("Page.loadEventFired");
// 游戏引擎在 load 之后还要启动一会儿（Phaser 解析 1 MB 脚本再建场景），点太早会点在空画布上。
await sleep(settleMs);
log("首屏已加载");

// --eval 在首屏之后跑一段 JS（比如往 localStorage 放令牌再 reload），审控制台登录后的页面用。
const evalAfterLoad = opt("--eval", null);
if (evalAfterLoad) {
  await evalJs(evalAfterLoad);
  await sleep(settleMs);
}

const report = { url, status: await evalJs("document.title"), first_page_text: await evalJs("document.body.innerText") };

if (clickSel) {
  const rect = await evalJs(`(() => { const el = document.querySelector(${JSON.stringify(clickSel)}); if (!el) return null; el.scrollIntoView({ block: "center" }); const r = el.getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`);
  if (!rect) {
    report.click = `没找到 ${clickSel}`;
  } else {
    // 用真实的鼠标事件而不是 element.click()：只有可信的用户手势才能解锁 AudioContext。
    await send("Input.dispatchMouseEvent", { type: "mouseMoved", x: rect.x, y: rect.y });
    await send("Input.dispatchMouseEvent", { type: "mousePressed", x: rect.x, y: rect.y, button: "left", clickCount: 1 });
    await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: rect.x, y: rect.y, button: "left", clickCount: 1 });
    report.click = `已点 ${clickSel} @ (${rect.x.toFixed(0)}, ${rect.y.toFixed(0)})`;
    try {
      await waitEvent("Page.loadEventFired", 5000);
    } catch {
      report.click += "（点击后没有整页跳转）";
    }
  }
}

await sleep(waitMs);
report.final_url = await evalJs("location.href");
report.final_title = await evalJs("document.title");
report.final_text = await evalJs("document.body.innerText");
report.canvas = await evalJs(
  "(() => { const c = document.querySelector('canvas'); if (!c) return null; const gl = c.getContext('webgl2') || c.getContext('webgl'); return { width: c.width, height: c.height, css: c.style.cssText, webgl: !!gl }; })()"
);
report.console = consoleLines;

if (shot) {
  const { data } = await send("Page.captureScreenshot", { format: "png" });
  const fs = await import("node:fs");
  fs.writeFileSync(shot, Buffer.from(data, "base64"));
  report.screenshot = shot;
}

console.log(JSON.stringify(report, null, 2));
await send("Page.close").catch(() => {});
ws.close();
process.exit(0);
