import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";

const root = fileURLToPath(new URL("../", import.meta.url));
const read = (path) => readFileSync(resolve(root, path), "utf8");
const docs = read("console/src/pages/docs.tsx");
const router = read("console/src/router.ts");
const app = read("console/src/app.tsx");
const publish = read("console/src/publish.tsx");
const home = read("console/src/pages/home.tsx");
const sections = ["start", "publish", "share", "manage", "automation", "troubleshoot"];
for (const section of sections) {
  assert.ok(router.includes(`"${section}"`), `missing route: ${section}`);
  assert.equal([...docs.matchAll(new RegExp(`<Chapter name="${section}">`, "g"))].length, 1);
}
assert.ok(app.indexOf('route.name === "docs"') < app.indexOf('!me'));
const publishCommands = [...publish.matchAll(/commandText: "([^"\n]+)"/g)].map((match) => match[1]);
const publishModes = publishCommands.filter((command) => command.startsWith("playtest"));
assert.equal(publishModes.length, 3, "all three publish modes remain available");
assert.ok(publishModes.every((command) => !command.includes("--public")), "default publish must not opt into plaza");
assert.ok(!home.includes("--public --seats"));
assert.ok(docs.includes("Default publish does not save an image") || docs.includes("默认发布不写图片"));
assert.ok(docs.includes("Unlisted ≠ Private access") || docs.includes("不上广场 ≠ 私密访问"));
assert.ok(docs.includes("--gate") && (docs.includes("option has been removed") || docs.includes("这个选项已撤出")));
assert.ok(docs.includes("no second confirmation") || docs.includes("没有第二次确认"));
assert.ok(!docs.includes("https://playtest.run/console"));
const metadata = spawnSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], { cwd: root, encoding: "utf8" });
assert.equal(metadata.status, 0, metadata.stderr);
const binary = process.env.PLAYTEST_CLI || resolve(JSON.parse(metadata.stdout).target_directory, "debug", process.platform === "win32" ? "playtest.exe" : "playtest");
const version = spawnSync(binary, ["--version"], { cwd: root, encoding: "utf8" });
assert.equal(version.status, 0, "需要先 cargo build -p playtest，或用 PLAYTEST_CLI 指定待检查的 CLI 二进制。");
const commands = [...docs.matchAll(/<CommandBlock command="([^"]+)"/g)].map((match) => match[1]);
commands.push(...publishModes, "playtest ./dist --seats 10", "playtest ./dist --card invite.png", "playtest rm ./dist -y --json", "playtest files ./dist --version v3");
for (const command of new Set(commands.filter((c) => c.startsWith("playtest")))) {
  const words = command.split(/\s+/).slice(1);
  const checked = spawnSync(binary, [...words, "--help"], { cwd: root, encoding: "utf8" });
  assert.equal(checked.status, 0, `${command}\n${checked.stderr}`);
}
console.log(`通过：6 个章节、免登录路由、关键语义与 ${new Set(commands).size} 条 CLI 用法。使用 ${version.stdout.trim()}，仅解析帮助，不发布作品。`);
