#!/usr/bin/env bash
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
API="${PLAYTEST_API:-https://playtest.roviix.com}"
FAILURES=0
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

fail() { printf '未通过：%s\n' "$*"; FAILURES=$((FAILURES + 1)); }
pass() { printf '通过：%s\n' "$*"; }

printf '发布前只读检查 · %s\n' "$(date '+%Y-%m-%d %H:%M:%S %Z')"
case "$API" in https://*|http://localhost:*|http://127.0.0.1:*) ;; *) fail 'API 地址需要 HTTPS；本机检查才允许 HTTP。'; exit 1 ;; esac
if [[ -n "$(git -C "$ROOT" status --porcelain)" ]]; then
  fail '工作区还有改动，不能把这次检查归到一个固定提交。不会替你提交或覆盖。'
else
  pass "工作区固定在 $(git -C "$ROOT" rev-parse --verify HEAD)"
fi

HEAD="$(git -C "$ROOT" rev-parse --verify HEAD)"
VERSION="$(awk '
  /^\[package\]$/ { package=1; next }
  /^\[/ { package=0 }
  package && /^version = "/ { split($0, parts, "\""); print parts[2]; exit }
' "$ROOT/cli/Cargo.toml")"
TAG="v$VERSION"
if [[ -z "$VERSION" ]]; then
  fail '读不到 CLI 版本。'
elif TAG_HEAD="$(git -C "$ROOT" rev-parse --verify "$TAG^{commit}" 2>/dev/null)"; then
  if [[ "$TAG_HEAD" = "$HEAD" ]]; then
    pass "$TAG 指向当前提交"
  else
    fail "$TAG 没有指向当前提交；不能用旧 tag 发布新源码"
  fi
else
  fail "当前 CLI 是 ${VERSION}，但仓库里没有 $TAG"
fi

if REMOTE_HEAD="$(git -C "$ROOT" ls-remote origin refs/heads/main 2>/dev/null | awk 'NR == 1 {print $1}')" && [[ -n "$REMOTE_HEAD" ]]; then
  if [[ "$REMOTE_HEAD" = "$HEAD" ]]; then
    pass 'origin/main 与当前提交一致'
  else
    fail 'origin/main 与当前提交不一致；发布前先推送并让 CI 检查这个提交'
  fi
else
  fail '读不到 origin/main，无法确认待发布提交已经推送'
fi

for endpoint in healthz llms.txt llms-full.txt skill.md openapi.json .well-known/agent.json; do
  if ! curl --silent --show-error --max-time 15 --output "$WORK/body" --write-out '%{http_code}' "${API%/}/$endpoint" > "$WORK/status"; then
    fail "$endpoint 连接失败"
  elif [[ "$(cat "$WORK/status")" != 200 || ! -s "$WORK/body" ]]; then
    fail "$endpoint 返回 $(cat "$WORK/status") 或空内容"
  elif [[ "$endpoint" = openapi.json ]] && ! node -e 'const spec=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8")); if(!spec.paths?.["/v1/me/token"]?.delete || !spec.paths?.["/v1/projects"]) process.exit(1)' "$WORK/body"; then
    fail 'OpenAPI 未包含当前发布与令牌撤销接口，可能仍是旧服务'
  else
    pass "$endpoint 可读取"
  fi
done

if command -v gh >/dev/null; then
  if gh repo view roviix/playtest.run --json isPrivate > "$WORK/repo" 2>/dev/null; then
    if node -e 'process.exit(JSON.parse(require("fs").readFileSync(process.argv[1],"utf8")).isPrivate ? 0 : 1)' "$WORK/repo"; then
      fail '仓库仍为私有，发布包仅受邀账号能下载；不满足公开安装'
    else
      pass '仓库可公开读取（仍需在全新设备验证安装）'
    fi
  else
    fail '无法确认发布包权限，需要无登录浏览器核对安装入口'
  fi
  if gh release view "$TAG" --repo roviix/playtest.run --json tagName,isDraft,assets > "$WORK/release" 2>/dev/null; then
    if node - "$WORK/release" "$VERSION" <<'NODE'
const fs = require("fs");
const release = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const version = process.argv[3];
const targets = [
  "aarch64-apple-darwin",
  "x86_64-apple-darwin",
  "x86_64-unknown-linux-musl",
  "aarch64-unknown-linux-musl",
  "x86_64-pc-windows-msvc",
];
const names = new Set(release.assets.map((asset) => asset.name));
const complete = release.tagName === `v${version}` && !release.isDraft &&
  names.has("SHA256SUMS") && targets.every((target) =>
    names.has(`playtest-${version}-${target}.${target.includes("windows") ? "zip" : "tar.gz"}`));
process.exit(complete ? 0 : 1);
NODE
    then
      pass "$TAG 的五个平台产物与 SHA256SUMS 齐全"
    else
      fail "$TAG 的 Release 仍是草稿、版本不符或缺少产物"
    fi
  else
    fail "GitHub 上还没有 $TAG 的 Release"
  fi
else
  fail '未安装 GitHub CLI，未核对发布包权限'
fi

printf '\n仍需人工记录：真实手机和微信、三网晚高峰、邮件收件与退订、真实作者首次安装、备份异机恢复。\n'
printf '自动检查不能证明正式发布条件全部成立；本次自动阻断项：%s。\n' "$FAILURES"
[[ "$FAILURES" -eq 0 ]]
