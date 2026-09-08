// 跑「变体 × 浏览器」这张表。点击一律用 page.click()（真实鼠标事件），
// 合成的 element.click() 在多数浏览器里根本不算用户手势，用了这次 spike 就白做。
//
//   node run.mjs                                  跑全部浏览器全部变体
//   node run.mjs --browsers chrome --variants v2,v2b
//   node run.mjs --bonus                          只跑附带观察（全屏 / 横屏锁定）

import { chromium, webkit, firefox } from 'playwright';
import { appendFile, mkdir, writeFile } from 'node:fs/promises';

const MAIN = 'http://localhost:9301';
const OTHER = 'http://localhost:9302';
const OUT = new URL('results/', import.meta.url);

const VARIANTS = [
  { id: 'v0', how: 'direct', what: '直接打开 game.html，不点任何东西' },
  { id: 'v1-direct', how: 'same', what: '点击回调里就地跑那四件事' },
  { id: 'v1-fetch', how: 'same', what: '点击回调里 await fetch + 换 body + 重跑 script' },
  { id: 'v2', how: 'nav', what: 'location.href 导航到同源 game.html' },
  { id: 'v2b', how: 'nav', what: '表单 POST → 服务端 303 → game.html（边缘现在的做法）' },
  { id: 'v3', how: 'frame', what: '同源 iframe' },
  { id: 'v4', how: 'frame', what: '跨源 iframe（另一个端口）' },
  { id: 'v5', how: 'nav', what: '导航之后游戏页干等 3 秒再碰音频', extra: 'delay=3000' },
];

// 自动播放策略这一栏是这次 spike 最容易搞错的地方，所以真 Chrome 跑三档：
//   chrome        不加参数 = 带界面的真 Chrome 默认行为，全新 profile 没有站点记忆，
//                 这一档最接近一个真玩家第一次点开陌生链接。
//   chrome-ugr    --autoplay-policy=user-gesture-required。名字听着最严，
//                 但 Chrome 里这一档对 AudioContext 只管跨源 iframe，反而比默认松。
//   chrome-strict --autoplay-policy=document-user-activation-required，最严的一档。
const BROWSERS = {
  chrome: {
    label: '真 Chrome（channel: chrome，headed，自动播放策略用默认值）',
    launch: () => chromium.launch({ channel: 'chrome', headless: false }),
  },
  'chrome-ugr': {
    label: '真 Chrome + --autoplay-policy=user-gesture-required',
    launch: () =>
      chromium.launch({ channel: 'chrome', headless: false, args: ['--autoplay-policy=user-gesture-required'] }),
  },
  'chrome-strict': {
    label: '真 Chrome + --autoplay-policy=document-user-activation-required',
    launch: () =>
      chromium.launch({
        channel: 'chrome',
        headless: false,
        args: ['--autoplay-policy=document-user-activation-required'],
      }),
  },
  // Playwright 的 WebKit 是近似 Safari，不是 Safari。结论要照这个口径写。
  webkit: { label: 'Playwright WebKit（近似 Safari，headed）', launch: () => webkit.launch({ headless: false }) },
  firefox: {
    label: 'Playwright Firefox（headed）',
    launch: () =>
      firefox.launch({
        headless: false,
        // 1 = 挡有声的自动播放，这是 Firefox 自己的默认值；Playwright 会放开，得写回来。
        firefoxUserPrefs: { 'media.autoplay.default': 1, 'media.autoplay.blocking_policy': 0 },
      }),
  },
};

function arg(name, fallback) {
  const i = process.argv.indexOf('--' + name);
  return i === -1 ? fallback : process.argv[i + 1];
}
const bonus = process.argv.includes('--bonus');
const pickedBrowsers = arg('browsers', 'chrome,chrome-ugr,chrome-strict,webkit,firefox').split(',');
const pickedVariants = arg('variants', VARIANTS.map((v) => v.id).join(',')).split(',');

function gateUrl(v) {
  const q = new URLSearchParams({ v: v.id, xo: OTHER });
  if (bonus) q.set('bonus', '1');
  const base = v.how === 'direct' ? `${MAIN}/game.html` : `${MAIN}/gate.html`;
  return `${base}?${q}${v.extra ? '&' + v.extra : ''}`;
}

const WAIT = 25000;

// 测量做完之前，除了 goto 和 click，什么都不许对页面做。
// 尤其是 evaluate / waitForFunction / waitForSelector / title——它们都走 CDP 的
// Runtime.callFunctionOn(userGesture: true)，在 Chrome 里等于凭空点了一下。
async function waitForReport(deadline) {
  while (Date.now() < deadline) {
    const list = await fetch(`${MAIN}/reports`).then((r) => r.json());
    if (list.length) return list;
    await new Promise((r) => setTimeout(r, 150));
  }
  return null;
}

async function runOne(context, v) {
  const page = await context.newPage();
  const console_ = [];
  page.on('console', (m) => m.type() === 'error' && console_.push(m.text()));
  page.on('pageerror', (e) => console_.push('pageerror: ' + e.message));

  const row = { variant: v.id, what: v.what, url: gateUrl(v) };
  try {
    await fetch(`${MAIN}/reports`, { method: 'DELETE' });
    await page.goto(row.url, { waitUntil: 'load' });
    if (v.how !== 'direct') await page.click('button');

    const list = await waitForReport(Date.now() + WAIT);
    if (!list) throw new Error(`${WAIT}ms 内没收到探针结果`);
    row.result = list[0];
    // 到这里测量已经结束，再读 title 就不会污染结果了。
    row.title = row.result.title;
    row.titleFromBrowser = v.how === 'frame' ? await page.frames()[1].title() : await page.title();
    if (v.how === 'frame') row.parentTitle = await page.title();
    row.finalUrl = page.url();
  } catch (e) {
    row.error = e.message.split('\n')[0];
  }
  if (console_.length) row.console = console_;
  await page.close();
  return row;
}

function line(row) {
  if (row.error) return `${row.variant.padEnd(10)} ✗ ${row.error}`;
  const r = row.result;
  const act = r.actBefore ? `${r.actBefore.isActive}/${r.actBefore.hasBeenActive}` : '无此API';
  return (
    `${row.variant.padEnd(10)} ac=${String(r.acStateNew).padEnd(9)}→${String(r.acStateAfterResume).padEnd(9)} ` +
    `resume=${String(r.resume).padEnd(10)} osc=${r.oscHeard ? '响' : '哑'}(+${r.clockAdvance}s) ` +
    `audio=${String(r.audioPlay + '/' + (r.audioHeard ? '响' : '哑')).padEnd(18)} act=${act}` +
    (r.bonus ? `  全屏=${r.bonus.fullscreen} 横屏=${r.bonus.orientation}` : '')
  );
}

await mkdir(OUT, { recursive: true });

for (const key of pickedBrowsers) {
  const spec = BROWSERS[key];
  if (!spec) throw new Error('不认识的浏览器：' + key);
  const browser = await spec.launch();
  const version = browser.version();
  console.log(`\n=== ${spec.label}  引擎版本 ${version}${bonus ? '  [附带观察]' : ''} ===`);
  const rows = [];
  for (const v of VARIANTS.filter((v) => pickedVariants.includes(v.id))) {
    // 每个变体一个全新 context：cookie、自动播放的站点记忆都不带过去。
    const context = await browser.newContext({ viewport: { width: 900, height: 700 } });
    const row = await runOne(context, v);
    await context.close();
    rows.push(row);
    console.log(line(row));
    if (row.title) console.log(`           title: ${row.title}`);
    await appendFile(new URL('results.jsonl', OUT), JSON.stringify({ browser: key, version, bonus, ...row }) + '\n');
  }
  await browser.close();
  await writeFile(new URL(`${key}${bonus ? '-bonus' : ''}.json`, OUT), JSON.stringify({ browser: key, label: spec.label, version, rows }, null, 2));
}
