// server.mjs + run.mjs 合成一个进程。
// 背景进程活不过一次工具调用，`node server.mjs &` 起的服务会在下一次调用前被收走，
// 所以这里让服务和跑批共用一个进程，跑完就退出。参数原样透传给 run.mjs。
//
//   node solo.mjs --browsers chrome-strict --variants v0,v2b

import './server.mjs';

await new Promise((r) => setTimeout(r, 300));
await import('./run.mjs');
process.exit(0);
