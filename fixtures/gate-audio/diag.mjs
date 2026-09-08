// 诊断用：V0 基线必须是「不能播」，否则整张表都是假的。
// 这里把 Chrome 实际拿到的命令行打出来，再用几种自动播放策略各跑一次 V0。

import { chromium } from 'playwright';

const COMBOS = [
  { name: '不加任何 autoplay 参数', args: [] },
  { name: '--autoplay-policy=user-gesture-required', args: ['--autoplay-policy=user-gesture-required'] },
  {
    name: '--autoplay-policy=document-user-activation-required',
    args: ['--autoplay-policy=document-user-activation-required'],
  },
];

for (const combo of COMBOS) {
  const browser = await chromium.launch({ channel: 'chrome', headless: false, args: combo.args });
  const context = await browser.newContext();
  const page = await context.newPage();

  if (process.argv.includes('--cmdline')) {
    const v = await context.newPage();
    await v.goto('chrome://version');
    const cmd = await v.evaluate(() => document.getElementById('command_line')?.textContent || '(读不到)');
    console.log('\n命令行：' + cmd.trim().split(' --').join('\n  --'));
    await v.close();
  }

  await page.goto('http://localhost:9301/game.html?v=v0', { waitUntil: 'load' });
  await page.waitForFunction(() => window.__probeResult, null, { timeout: 20000 });
  const r = await page.evaluate(() => window.__probeResult);
  console.log(`${combo.name.padEnd(52)} → ac=${r.acStateNew}→${r.acStateAfterResume} audio=${r.audioPlay}`);
  await browser.close();
}
