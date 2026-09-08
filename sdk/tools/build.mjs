// 打包 dist/playtest.js。
//
// 用 esbuild 的 JS 接口而不是它的命令行：命令行要走包管理器生成的 .bin 包装脚本，
// 而 esbuild 的安装脚本会把 bin/esbuild 从一段 JS 换成本机可执行文件——包装脚本按前者
// 生成、按后者执行，就会以 `SyntaxError: Invalid or unexpected token` 挂掉。
// JS 接口自己找平台包，跟安装脚本跑没跑无关（所以 pnpm-workspace.yaml 里不让它跑）。

import { build } from 'esbuild';

// es2018 是能覆盖到微信内置浏览器的下限；iife 保证这一行 <script> 不需要 type=module。
await build({
  entryPoints: ['src/playtest.ts'],
  outfile: 'dist/playtest.js',
  bundle: true,
  minify: true,
  format: 'iife',
  target: 'es2018',
  legalComments: 'none',
});
