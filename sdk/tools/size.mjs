// 体积是这个 SDK 的一条硬约束：它加在别人的游戏前面，先于游戏下载。
// 超了就让构建失败，而不是在某次评审里被发现（DESIGN §4.6「几 KB，无依赖」）。

import { gzipSync } from 'node:zlib';
import { readFileSync } from 'node:fs';

const BUDGET = 4096;
const file = 'dist/playtest.js';
const raw = readFileSync(file);
const gzipped = gzipSync(raw, { level: 9 }).length;

console.log(`${file}: ${raw.length} 字节，gzip 后 ${gzipped} 字节（上限 ${BUDGET}）`);
if (gzipped > BUDGET) {
  console.error(`超了 ${gzipped - BUDGET} 字节。要么删功能，要么先改 DESIGN §4.6。`);
  process.exit(1);
}
