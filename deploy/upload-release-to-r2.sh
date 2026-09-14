#!/usr/bin/env bash
# 将 playtest CLI 发布包上传至 Cloudflare R2 (dl.roviix.com) 进行边缘分发。
# 借鉴思路：通过 Cloudflare R2 配合自定义域名 dl.roviix.com 避免 GitHub Releases 国内网络慢与出网流量费，
# 上传后逐一进行 CDN 回读与 SHA-256 校验，确认无误。
set -euo pipefail

usage() {
  cat <<'EOF'
用法:
  deploy/upload-release-to-r2.sh --input <包含发布包的目录，如 dist> [--bucket <r2-bucket>] [--base-url <url>]

环境变量:
  R2_BUCKET           目标 R2 存储桶（默认: nexus-desktop）
  DOWNLOAD_BASE_URL   CDN 公网基础地址（默认: https://dl.roviix.com/files）
  WRANGLER            执行 wrangler 命令（默认: npx --yes wrangler@4）
  CLOUDFLARE_API_TOKEN 非交互式鉴权令牌 (CI 环境)
EOF
}

INPUT_DIR=""
BUCKET="${R2_BUCKET:-nexus-desktop}"
BASE_URL="${DOWNLOAD_BASE_URL:-https://dl.roviix.com/files}"
WRANGLER_BIN="${WRANGLER:-npx --yes wrangler@4}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --input)
      INPUT_DIR="${2:-}"
      shift 2
      ;;
    --bucket)
      BUCKET="${2:-}"
      shift 2
      ;;
    --base-url)
      BASE_URL="${2:-}"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "未知参数: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

[[ -n "$INPUT_DIR" && -d "$INPUT_DIR" ]] || { echo "错误: --input 必须指向存在的构建产物目录。" >&2; exit 2; }
command -v curl >/dev/null 2>&1 || { echo "错误: 缺少 curl。" >&2; exit 2; }

# 去除 BASE_URL 末尾斜杠
BASE_URL="${BASE_URL%/}"
# 提取路径前缀，例如 https://dl.roviix.com/files -> files
KEY_PREFIX="$(echo "$BASE_URL" | sed -E 's|^https?://[^/]+/?||; s|/$||')"

content_type_for() {
  case "$1" in
    *.tar.gz) echo "application/gzip" ;;
    *.zip) echo "application/zip" ;;
    *.json) echo "application/json; charset=utf-8" ;;
    *SHA256SUMS*) echo "text/plain; charset=utf-8" ;;
    *) echo "application/octet-stream" ;;
  esac
}

echo "==> 检查发布产物目录: $INPUT_DIR"
FILES_TO_UPLOAD=()
while IFS= read -r -d '' file; do
  name="$(basename "$file")"
  # 仅上传压缩归档与校验文件
  if [[ "$name" =~ \.(tar\.gz|zip)$ || "$name" == "SHA256SUMS" ]]; then
    FILES_TO_UPLOAD+=("$file")
  fi
done < <(find "$INPUT_DIR" -maxdepth 1 -type f -print0 | sort -z)

if [[ ${#FILES_TO_UPLOAD[@]} -eq 0 ]]; then
  echo "错误: 未在 $INPUT_DIR 中找到可发布的压缩包或校验文件。" >&2
  exit 2
fi

echo "==> 找到待上传文件 (${#FILES_TO_UPLOAD[@]} 个):"
for f in "${FILES_TO_UPLOAD[@]}"; do
  echo "    - $(basename "$f")"
done

for file in "${FILES_TO_UPLOAD[@]}"; do
  name="$(basename "$file")"
  key="${KEY_PREFIX:+$KEY_PREFIX/}$name"
  cache_control="public, max-age=31536000, immutable"
  if [[ "$name" == "SHA256SUMS" ]]; then
    cache_control="public, max-age=300, must-revalidate"
  fi

  echo "==> 上传至 r2://$BUCKET/$key ..."
  $WRANGLER_BIN r2 object put "$BUCKET/$key" \
    --remote \
    --file "$file" \
    --content-type "$(content_type_for "$name")" \
    --content-disposition "attachment; filename=\"$name\"" \
    --cache-control "$cache_control" >/dev/null
done

echo "==> 逐一回读与验证 CDN 可用性 ($BASE_URL) ..."
TEMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TEMP_DIR"' EXIT

for file in "${FILES_TO_UPLOAD[@]}"; do
  name="$(basename "$file")"
  target_url="$BASE_URL/$name"
  echo "    验证 $target_url ..."
  status="$(curl -s -o "$TEMP_DIR/$name" -w "%{http_code}" "$target_url" || true)"
  if [[ "$status" != "200" ]]; then
    echo "错误: CDN 回读失败，HTTP 状态码 $status ($target_url)" >&2
    exit 3
  fi

  # 对比原始文件与 CDN 读回文件的 SHA-256
  expected_sum="$(shasum -a 256 "$file" 2>/dev/null | awk '{print $1}' || sha256sum "$file" | awk '{print $1}')"
  actual_sum="$(shasum -a 256 "$TEMP_DIR/$name" 2>/dev/null | awk '{print $1}' || sha256sum "$TEMP_DIR/$name" | awk '{print $1}')"
  if [[ "$expected_sum" != "$actual_sum" ]]; then
    echo "错误: CDN 回读校验和不匹配 ($name)" >&2
    exit 3
  fi
  echo "    ✓ 校验和一致: $expected_sum"
done

echo "==> 全部发布文件已成功上传至 dl.roviix.com 并校验通过！"
