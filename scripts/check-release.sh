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
else
  fail '未安装 GitHub CLI，未核对发布包权限'
fi

printf '\n仍需人工记录：真实手机和微信、三网晚高峰、邮件收件与退订、真实作者首次安装、备份异机恢复。\n'
printf '自动检查不能证明正式发布条件全部成立；本次自动阻断项：%s。\n' "$FAILURES"
[[ "$FAILURES" -eq 0 ]]
