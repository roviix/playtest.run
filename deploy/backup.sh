#!/usr/bin/env bash
# 服务器上的每小时备份：SQLite 用在线备份 API 拷一份一致的快照，清单与指针打个小包；保留 7 天。
# blob 不在这里备——体积大且按内容哈希可重传，靠 EBS 每日快照兜底。
set -euo pipefail

DATA="${PLAYTEST_DATA_HOST_DIR:-$HOME/playtest-data}"
OUT="$HOME/db-backups/playtest"
STAMP="$(date -u +%Y%m%d-%H%M)"
mkdir -p "$OUT"

if [[ -f "$DATA/api.sqlite" ]]; then
  sudo sqlite3 "$DATA/api.sqlite" ".backup '$OUT/api-$STAMP.sqlite'"
  gzip -f "$OUT/api-$STAMP.sqlite"
fi
if [[ -d "$DATA/store/sites" ]]; then
  sudo tar -czf "$OUT/sites-$STAMP.tgz" -C "$DATA/store" sites
fi
sudo chown "$USER:$USER" "$OUT"/*-"$STAMP".* 2>/dev/null || true

find "$OUT" -type f \( -name 'api-*.sqlite.gz' -o -name 'sites-*.tgz' \) -mtime +7 -delete
echo "$(date -u +%FT%TZ) ok $(ls "$OUT" | wc -l) 个文件"
