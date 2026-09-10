#!/usr/bin/env bash
# 运营者的手动闸门（DESIGN §3.11、§4.5）：赠送推广、放行或拒掉、撤下作品、看通知队列。
#
# 只有一个运营者，所以这里不做后台界面，只有一层 curl 包装。
#   ADMIN_TOKEN=... API=https://playtest.roviix.com scripts/admin.sh boosts
# API 默认本机 8787。出错时原样打印控制面的中文 message，不另编一套话。
set -euo pipefail

API="${API:-http://127.0.0.1:8787}"
API="${API%/}"

usage() {
  cat <<'EOF'
用法：ADMIN_TOKEN=... [API=...] scripts/admin.sh <子命令>

  boosts                              列出全部推广（含排队中的）
  grant <slug> <days3|days7|digest>   赠送一段推广；不给开始时间就排到最早能上的那天
  review <id> <approve|reject> [理由]  人工看过之后放行或拒掉
  end <id>                            提前结束一段推广
  hide <slug>                         从广场撤下（作品链接照常能开）
  unhide <slug>                       复核之后恢复
  notifications                       通知队列现在多长

环境变量：
  ADMIN_TOKEN  必填，和控制面的 PLAYTEST_ADMIN_TOKEN 一样。没配这组接口在控制面上根本不存在（404）。
  API          控制面地址，默认 http://127.0.0.1:8787
EOF
}

die() {
  echo "$*" >&2
  exit 1
}

# 调一次控制面。成功打 body，失败把控制面那句中文原样打出来再退出。
call() {
  local method="$1" path="$2" body="${3:-}"
  local args=(-sS -o /tmp/playtest-admin.$$ -w '%{http_code}'
    -X "$method" "$API$path"
    -H "Authorization: Bearer $ADMIN_TOKEN")
  if [[ -n "$body" ]]; then
    args+=(-H 'Content-Type: application/json' -d "$body")
  fi

  local code
  code="$(curl "${args[@]}")" || { rm -f "/tmp/playtest-admin.$$"; die "连不上 $API"; }
  local out
  out="$(cat "/tmp/playtest-admin.$$")"
  rm -f "/tmp/playtest-admin.$$"

  if [[ "$code" =~ ^2 ]]; then
    if [[ -n "$out" ]]; then
      # 有 jq 就排版，没有就原样——这台机器上没装 jq 不该让命令失败。
      if command -v jq >/dev/null 2>&1; then echo "$out" | jq .; else echo "$out"; fi
    else
      echo "好了。"
    fi
    return 0
  fi

  local message=""
  if command -v jq >/dev/null 2>&1; then
    message="$(echo "$out" | jq -r '.message // empty' 2>/dev/null || true)"
  fi
  [[ -n "$message" ]] || message="$out"
  case "$code" in
    401) [[ -n "$message" ]] || message="管理令牌不对。" ;;
    404) [[ -n "$message" ]] || message="没有这个地址：这台控制面多半没配 PLAYTEST_ADMIN_TOKEN。" ;;
  esac
  echo "$message" >&2
  exit 1
}

json_escape() {
  # 理由里可能有引号和换行，交给 python3 escape；没有 python3 就退回一个保守的替换。
  if command -v python3 >/dev/null 2>&1; then
    python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$1"
  else
    printf '"%s"' "${1//\"/\\\"}"
  fi
}

[[ $# -ge 1 ]] || { usage; exit 2; }
cmd="$1"; shift

case "$cmd" in
  -h|--help|help) usage; exit 0 ;;
esac

[[ -n "${ADMIN_TOKEN:-}" ]] || die "先设 ADMIN_TOKEN，和控制面的 PLAYTEST_ADMIN_TOKEN 一样。"

case "$cmd" in
  boosts)
    call GET /admin/boosts
    ;;
  grant)
    [[ $# -ge 2 ]] || die "用法：grant <slug> <days3|days7|digest> [开始时间 2026-09-14T01:00:00Z]"
    slug="$1"; kind="$2"; starts="${3:-}"
    case "$kind" in
      days3|days7|digest) ;;
      *) die "推广只有 days3、days7、digest 三种，现在是「$kind」。" ;;
    esac
    if [[ -n "$starts" ]]; then
      call POST /admin/boosts "{\"slug\":$(json_escape "$slug"),\"kind\":\"$kind\",\"starts_at\":$(json_escape "$starts")}"
    else
      call POST /admin/boosts "{\"slug\":$(json_escape "$slug"),\"kind\":\"$kind\"}"
    fi
    ;;
  review)
    [[ $# -ge 2 ]] || die "用法：review <id> <approve|reject> [理由]"
    id="$1"; verdict="$2"; reason="${3:-}"
    case "$verdict" in
      approve) approve=true ;;
      reject) approve=false ;;
      *) die "第二个参数只能是 approve 或 reject，现在是「$verdict」。" ;;
    esac
    if [[ -n "$reason" ]]; then
      call POST "/admin/boosts/$id/review" "{\"approve\":$approve,\"reason\":$(json_escape "$reason")}"
    else
      call POST "/admin/boosts/$id/review" "{\"approve\":$approve}"
    fi
    ;;
  end)
    [[ $# -ge 1 ]] || die "用法：end <id>"
    call DELETE "/admin/boosts/$1"
    ;;
  hide)
    [[ $# -ge 1 ]] || die "用法：hide <slug>"
    call POST "/admin/plaza/$1/hide"
    ;;
  unhide)
    [[ $# -ge 1 ]] || die "用法：unhide <slug>"
    call DELETE "/admin/plaza/$1/hide"
    ;;
  notifications)
    call GET /admin/notifications
    ;;
  *)
    usage
    exit 2
    ;;
esac
