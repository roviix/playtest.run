#!/usr/bin/env bash
# 发布到香港机器。默认和 roviix / nexus 一样**在服务器上构建**：把源码 rsync 过去，主机上 docker build，
# 不经任何镜像仓库；然后按 .env 决定启用哪些站点、compose up、健康检查。
#
# 三个镜像：server（api + edge 两个 Rust 二进制）、caddy（带 Cloudflare DNS 插件）、console（控制台静态产物）。
#
# 用法（仓库根目录）：
#   deploy/push.sh                     同步源码，主机上构建三个镜像并发布
#   TARGET=server deploy/push.sh       只重建 api/edge（改 Rust 代码后；依赖有缓存，只编改动的 crate）
#   TARGET=console deploy/push.sh      只重建控制台
#   TARGET=caddy  deploy/push.sh       只重建 Caddy 镜像（升级 caddy 或换插件）
#   TARGET=none   deploy/push.sh       不构建，只同步配置并重载（改 Caddyfile / compose / .env 后）
#   BUILD_ON=local deploy/push.sh      改在本机构建 linux/arm64 镜像再经 ssh 装载（要本机 Docker 健康、磁盘够）
#   TAG=<标签>    deploy/push.sh       指定镜像标签
#   DEPLOY_HOST=ubuntu@1.2.3.4         主机，默认用 ~/.ssh/config 里的 playtest-hk
#
# 主机上的 deploy/.env 不会被这里覆盖；第一次上线先照 .env.example 手工建好。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

HOST="${DEPLOY_HOST:-playtest-hk}"
REMOTE_ROOT="${REMOTE_ROOT:-workspace/playtest.run}"
TARGET="${TARGET:-all}"
BUILD_ON="${BUILD_ON:-server}"
PLATFORM="${PLATFORM:-linux/arm64}"
TAG="${TAG:-$(date -u +%Y%m%d-%H%M%S)}"
SSH=(ssh -o BatchMode=yes -o ConnectTimeout=20 -o ConnectionAttempts=5 "$HOST")
RSYNC=(rsync -az -e "ssh -o BatchMode=yes -o ConnectTimeout=20 -o ConnectionAttempts=5")

case "$TARGET" in all|server|caddy|console|none) ;; *) echo "TARGET 只能是 all / server / caddy / console / none，不是「$TARGET」" >&2; exit 2 ;; esac
case "$BUILD_ON" in server|local) ;; *) echo "BUILD_ON 只能是 server 或 local，不是「$BUILD_ON」" >&2; exit 2 ;; esac

want() { [[ "$TARGET" == all || "$TARGET" == "$1" ]]; }

echo "== 同步源码与配置 → $HOST:${REMOTE_ROOT}（不碰 .env）"
"${SSH[@]}" "mkdir -p $REMOTE_ROOT/deploy/sites.active"
# 只送构建需要的东西：不送 target/、.data/、node_modules/、fixtures/、docs/。
"${RSYNC[@]}" --delete \
  --include '/Cargo.toml' --include '/Cargo.lock' --include '/Dockerfile' --include '/.dockerignore' \
  --include '/common/***' --include '/api/***' --include '/edge/***' --include '/cli/***' \
  --include '/ui/***' \
  --include '/sdk/' --include '/sdk/dist/***' \
  --include '/console/' --include '/console/src/***' --include '/console/public/***' --include '/console/index.html' \
  --include '/console/package.json' --include '/console/pnpm-lock.yaml' --include '/console/pnpm-workspace.yaml' \
  --include '/console/tsconfig.json' --include '/console/vite.config.ts' \
  --exclude '*' \
  ./ "$HOST:$REMOTE_ROOT/"
"${RSYNC[@]}" --exclude '.env' --exclude '.env.example' --exclude 'sites.active' \
  deploy/ "$HOST:$REMOTE_ROOT/deploy/"

if [[ "$BUILD_ON" == local && "$TARGET" != none ]]; then
  export DOCKER_BUILDKIT=1
  images=()
  if want server;  then docker buildx build --platform "$PLATFORM" -t "playtest-server:$TAG"  --load -f Dockerfile .;                         images+=("playtest-server:$TAG");  fi
  if want caddy;   then docker buildx build --platform "$PLATFORM" -t "playtest-caddy:$TAG"   --load -f deploy/caddy.Dockerfile deploy;      images+=("playtest-caddy:$TAG");   fi
  if want console; then docker buildx build --platform "$PLATFORM" -t "playtest-console:$TAG" --load -f deploy/console.Dockerfile .;         images+=("playtest-console:$TAG"); fi
  echo "== 装载镜像到 $HOST：${images[*]}"
  docker save "${images[@]}" | gzip -1 | "${SSH[@]}" 'gunzip | docker load'
fi

echo "== 主机上$([[ "$BUILD_ON" == server && "$TARGET" != none ]] && echo '构建、')启用站点并重建容器"
"${SSH[@]}" TAG="$TAG" TARGET="$TARGET" BUILD_ON="$BUILD_ON" REMOTE_ROOT="$REMOTE_ROOT" bash -s <<'EOF'
set -euo pipefail
cd ~/"$REMOTE_ROOT"
export DOCKER_BUILDKIT=1
want() { [[ "$TARGET" == all || "$TARGET" == "$1" ]]; }

if [[ "$BUILD_ON" == server && "$TARGET" != none ]]; then
  if pgrep -f 'docker build' >/dev/null; then echo "主机上已有构建在跑，本次退出。" >&2; exit 3; fi
  if want server; then
    echo "-- docker build playtest-server:${TAG}（首次要编全部依赖，之后有缓存）"
    time docker build --progress=plain -t "playtest-server:$TAG" -f Dockerfile . 2>&1 | grep -E '^#[0-9]+ [0-9.]+ +(Compiling|Finished|error|warning: unused)|ERROR|error\[|DONE [0-9]+\.[0-9]s$' | grep -vE 'DONE 0\.[0-9]s$' | tail -25
  fi
  if want caddy; then
    echo "-- docker build playtest-caddy:${TAG}"
    time docker build -t "playtest-caddy:$TAG" -f deploy/caddy.Dockerfile deploy 2>&1 | tail -3
  fi
  if want console; then
    echo "-- docker build playtest-console:${TAG}"
    time docker build -t "playtest-console:$TAG" -f deploy/console.Dockerfile . 2>&1 | grep -E 'error|Error|ERROR|built in|dist/' | tail -8
  fi
fi

cd deploy
[[ -f .env ]] || { echo "主机上缺 $REMOTE_ROOT/deploy/.env：照 .env.example 建一份再来。" >&2; exit 2; }
chmod 600 .env; chmod +x backup.sh
set -a; . ./.env; set +a

# 只把 .env 里配齐了的站点放进 sites.active/，Caddy 不会因为缺令牌起不来。
rm -f sites.active/*.caddy
if [[ -n "${CLOUDFLARE_API_TOKEN:-}" ]]; then cp sites/content.caddy sites.active/; echo "启用 playtest.run + *.playtest.run（DNS-01）"; else echo "未启用玩家域名：.env 里 CLOUDFLARE_API_TOKEN 为空"; fi
echo "平台与 API 统一使用 playtest.run；作品仍由独立子域交付"

# 没重建的镜像沿用正在跑的标签；第一次没有旧容器就用本次 TAG。
current_tag() { docker inspect --format '{{.Config.Image}}' "$1" 2>/dev/null | sed 's/.*://' || true; }
pick() { local name="$1" container="$2" t; if want "$name"; then echo "$TAG"; else t="$(current_tag "$container")"; echo "${t:-$TAG}"; fi; }
server_tag="$(pick server playtest-edge-1)"
caddy_tag="$(pick caddy playtest-caddy-1)"
console_tag="$(pick console playtest-console-1)"
printf 'SERVER_TAG=%s\nCADDY_TAG=%s\nCONSOLE_TAG=%s\n' "$server_tag" "$caddy_tag" "$console_tag" > .tag
SERVER_TAG="$server_tag" CADDY_TAG="$caddy_tag" CONSOLE_TAG="$console_tag" docker compose up -d --remove-orphans
sleep 3
docker compose ps -a --format 'table {{.Name}}\t{{.Image}}\t{{.Status}}'
echo "-- Caddy 最近日志"
docker compose logs --no-log-prefix --tail 8 caddy
echo "-- api / edge 健康"
curl -fsS http://127.0.0.1:8787/healthz && echo "  ← api"
docker compose exec -T edge curl -fsS -H "Host: ${PLAYTEST_HOST_SUFFIX}" http://127.0.0.1:8443/_playtest/healthz && echo "  ← edge"
# 旧镜像只留最近三个标签，别让盘被镜像塞满。
for img in playtest-server playtest-caddy playtest-console; do
  docker images "$img" --format '{{.Tag}}' | sort -r | tail -n +4 | xargs -r -I{} docker rmi -f "$img:{}" >/dev/null 2>&1 || true
done
EOF

echo "== 完成：标签见主机 ${REMOTE_ROOT}/deploy/.tag（本次 TAG=${TAG}）"
