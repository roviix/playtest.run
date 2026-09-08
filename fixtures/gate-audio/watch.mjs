// V0 里没人点过，用户激活却在两秒后自己变成 true——先弄清是谁给的，
// 不然表里每一个「能播」都不可信。这里在页面最早的时机装监听，把事件和激活翻转都记下来。

import { chromium } from 'playwright';

const browser = await chromium.launch({ channel: 'chrome', headless: false });
const page = await browser.newPage();

await page.addInitScript(() => {
  window.__log = [];
  const t0 = performance.now();
  const at = () => Math.round(performance.now() - t0);
  for (const type of ['mousedown', 'mouseup', 'pointerdown', 'pointerup', 'click', 'keydown', 'touchend', 'focus', 'blur', 'visibilitychange']) {
    window.addEventListener(type, (e) => window.__log.push(`${at()}ms 事件 ${type} isTrusted=${e.isTrusted}`), true);
  }
  let last = null;
  setInterval(() => {
    const ua = navigator.userActivation;
    const now = ua ? `${ua.isActive}/${ua.hasBeenActive}` : '无';
    if (now !== last) {
      window.__log.push(`${at()}ms 激活 ${last} → ${now}`);
      last = now;
    }
  }, 50);
});

await page.goto('http://localhost:9301/game.html?v=watch', { waitUntil: 'load' });
await page.evaluate(() => {
  const C = window.AudioContext || window.webkitAudioContext;
  const ctx = new C();
  window.__ctx = ctx;
  const t0 = performance.now();
  let last = ctx.state;
  window.__log.push(`${Math.round(performance.now() - t0)}ms AudioContext 建好，state=${ctx.state}`);
  setInterval(() => {
    if (ctx.state !== last) {
      window.__log.push(`${Math.round(performance.now() - t0)}ms AudioContext ${last} → ${ctx.state}`);
      last = ctx.state;
    }
  }, 50);
});

await page.waitForTimeout(6000);
console.log((await page.evaluate(() => window.__log)).join('\n'));
console.log('六秒后 AudioContext.state =', await page.evaluate(() => window.__ctx.state));
await browser.close();
