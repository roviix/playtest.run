// 从 results.jsonl 重建每个浏览器一份的汇总 JSON。
// run.mjs 每次调用只写它这一次跑到的变体，分批跑完之后那些 <browser>.json 是残缺的；
// jsonl 是只追加的流水，才是全的。同一个 browser+variant 有多条时取最后一条。
//
//   node collect.mjs

import { readFile, writeFile } from 'node:fs/promises';

const OUT = new URL('results/', import.meta.url);
const VARIANTS = ['v0', 'v1-direct', 'v1-fetch', 'v2', 'v2b', 'v3', 'v4', 'v5'];

const lines = (await readFile(new URL('results.jsonl', OUT), 'utf8'))
  .trim()
  .split('\n')
  .map((l) => JSON.parse(l));

const latest = new Map();
for (const l of lines) latest.set(`${l.browser}/${l.variant}`, l);

for (const browser of [...new Set(lines.map((l) => l.browser))]) {
  const rows = VARIANTS.map((v) => latest.get(`${browser}/${v}`)).filter(Boolean);
  const missing = VARIANTS.filter((v) => !latest.has(`${browser}/${v}`));
  const { version } = rows[rows.length - 1];
  await writeFile(
    new URL(`${browser}.json`, OUT),
    JSON.stringify({ browser, version, variants: rows.length, missing, rows }, null, 2)
  );
  console.log(
    `${browser.padEnd(14)} ${rows.length}/${VARIANTS.length} 个变体  引擎 ${version}` +
      (missing.length ? `  缺：${missing.join(',')}` : '')
  );
}
