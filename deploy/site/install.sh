#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf '错误: %s\n' "$1" >&2
  exit 1
}

VERSION="${PLAYTEST_VERSION:-}"
DESTINATION="${PLAYTEST_INSTALL_DIR:-$HOME/.local/bin}"
REPOSITORY="roviix/playtest.run"

if [ -z "$VERSION" ]; then
  if command -v curl >/dev/null; then
    LATEST_REDIRECT="$(curl --silent --show-error --head --location --output /dev/null --write-out '%{url_effective}' "https://github.com/$REPOSITORY/releases/latest" 2>/dev/null || true)"
    if [[ "$LATEST_REDIRECT" =~ /tag/(v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?)$ ]]; then
      VERSION="${BASH_REMATCH[1]}"
    fi
  fi
  if [ -z "$VERSION" ] && command -v curl >/dev/null; then
    API_TAG="$(curl --silent --show-error --max-time 10 "https://api.github.com/repos/$REPOSITORY/releases/latest" 2>/dev/null | grep -m1 '"tag_name":' | sed -E 's/.*"tag_name":\s*"([^"]+)".*/\1/' || true)"
    if [[ "$API_TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?$ ]]; then
      VERSION="$API_TAG"
    fi
  fi
  if [ -z "$VERSION" ]; then
    VERSION="v0.3.0"
  fi
fi

[[ "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?$ ]] || fail '未能确认有效的发布版本；可显式指定版本运行，例如 PLAYTEST_VERSION=v0.3.0 bash install.sh'
[[ "$DESTINATION" = /* ]] || fail '安装目录必须是绝对路径。'
for tool in uname tar mktemp; do command -v "$tool" >/dev/null || fail "缺少 $tool。"; done

case "$(uname -s):$(uname -m)" in
  Darwin:arm64) TARGET=aarch64-apple-darwin ;;
  Darwin:x86_64) TARGET=x86_64-apple-darwin ;;
  Linux:x86_64) TARGET=x86_64-unknown-linux-musl ;;
  Linux:aarch64|Linux:arm64) TARGET=aarch64-unknown-linux-musl ;;
  *) fail '这台设备不在此脚本的范围内。Windows 请下载对应 zip；不猜架构、不改系统设置。' ;;
esac

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
ARCHIVE="playtest-${VERSION#v}-${TARGET}.tar.gz"
if command -v gh >/dev/null && gh auth status >/dev/null 2>&1; then
  gh release download "$VERSION" --repo "$REPOSITORY" --pattern "$ARCHIVE" --pattern SHA256SUMS --dir "$WORK" || fail '下载失败。私测版本需要仓库访问权限；请向邀请你的人确认版本和权限。'
else
  command -v curl >/dev/null || fail '缺少 curl。'
  R2_BASE="${PLAYTEST_DOWNLOAD_BASE:-https://dl.roviix.com/files}"
  DOWNLOADED=0

  # 1. 优先尝试 Cloudflare R2 / dl.roviix.com 高速分发（免代理、国内及全球 CDN 边缘直连）
  if [ -n "$R2_BASE" ]; then
    if curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --connect-timeout 6 --max-time 60 "$R2_BASE/$ARCHIVE" --output "$WORK/$ARCHIVE" 2>/dev/null && \
       curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --connect-timeout 6 --max-time 15 "$R2_BASE/SHA256SUMS" --output "$WORK/SHA256SUMS" 2>/dev/null; then
      DOWNLOADED=1
    fi
  fi

  # 2. 若 CDN 镜像未命中或未上传，自动回退至 GitHub Releases
  if [ "$DOWNLOADED" -eq 0 ]; then
    BASE="https://github.com/$REPOSITORY/releases/download/$VERSION"
    curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --max-time 180 "$BASE/$ARCHIVE" --output "$WORK/$ARCHIVE" || fail '下载失败。当前版本未在加速线路或 GitHub Releases 公开；私测请先用 GitHub CLI 登录受邀账号。'
    curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --max-time 30 "$BASE/SHA256SUMS" --output "$WORK/SHA256SUMS" || fail '缺少校验文件，拒绝安装。'
  fi
fi

EXPECTED="$(awk -v name="$ARCHIVE" '$2 == name || $2 == "*" name { print $1 }' "$WORK/SHA256SUMS")"
[[ "$EXPECTED" =~ ^[a-fA-F0-9]{64}$ ]] || fail '校验文件里没有唯一、合法的安装包校验值。'
if command -v shasum >/dev/null; then
  ACTUAL="$(shasum -a 256 "$WORK/$ARCHIVE" | awk '{print $1}')"
elif command -v sha256sum >/dev/null; then
  ACTUAL="$(sha256sum "$WORK/$ARCHIVE" | awk '{print $1}')"
else
  fail '缺少 shasum 或 sha256sum，无法校验安装包。'
fi
[[ "$ACTUAL" == "$EXPECTED" ]] || fail '安装包校验失败，现有程序没有改动。'
ENTRY="playtest-${VERSION#v}-${TARGET}/playtest"
tar -tzf "$WORK/$ARCHIVE" > "$WORK/entries" || fail '安装包不能读取。'
[[ "$(awk -v entry="$ENTRY" '$0 == entry { count++ } END {print count+0}' "$WORK/entries")" == 1 ]] || fail '安装包中没有唯一的 playtest 程序。'
tar -xOzf "$WORK/$ARCHIVE" "$ENTRY" > "$WORK/playtest" || fail '提取程序失败。'
[[ -s "$WORK/playtest" ]] || fail '程序内容为空。'
mkdir -p "$DESTINATION"
TEMPORARY="$(mktemp "$DESTINATION/.playtest-install.XXXXXX")"
trap 'rm -rf "$WORK"; rm -f "${TEMPORARY:-}"' EXIT
cat "$WORK/playtest" > "$TEMPORARY"
chmod 755 "$TEMPORARY"
mv -f "$TEMPORARY" "$DESTINATION/playtest"
printf '已安装 %s 到 %s/playtest（SHA-256 校验通过）。\n' "$VERSION" "$DESTINATION"
printf '把这个目录加入 PATH 后，运行 playtest --version，再运行 playtest ./dist。\n'
printf '本脚本不改 shell 配置、不用 sudo、不关闭 macOS 安全检查；未签名版本可能需要在系统设置中手动放行。\n'
