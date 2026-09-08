#!/bin/bash
# 把一个浏览器的 8 个变体分小批跑完，再重建汇总。
#
# 为什么分批：一次跑满 8 个要一分多钟，某些终端/工具链会在半路把长命令收走，
# 收走之后 9301 上那个服务也就没了，剩下的变体全报「25000ms 内没收到探针结果」。
# 每批 2–3 个变体、单批十几秒，跑完就退，稳。
#
#   ./once.sh chrome-ugr
#   ./once.sh webkit

set -u
cd "$(dirname "$0")" || exit 1

BROWSER=${1:-chrome}
BATCHES=('v0,v1-direct' 'v1-fetch,v2' 'v2b,v3' 'v4,v5')

for batch in "${BATCHES[@]}"; do
  echo "=== $BROWSER : $batch"
  node solo.mjs --browsers "$BROWSER" --variants "$batch" || {
    echo "这一批挂了，后面的还是照跑，缺哪个变体最后 collect.mjs 会报出来"
  }
done

node collect.mjs
