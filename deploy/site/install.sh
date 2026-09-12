#!/usr/bin/env bash
set -euo pipefail

VERSION="${PLAYTEST_VERSION:-}"
DESTINATION="${PLAYTEST_INSTALL_DIR:-$HOME/.local/bin}"
REPOSITORY="roviix/playtest.run"

fail() { printf '安装未完成：%s\n' "$*" >&2; exit 1; }
[[ "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?$ ]] || fail '请先指定发布版本，例如 PLAYTEST_VERSION=v0.2.0 bash install.sh；不要安装无法确认来源的 latest。'
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
  BASE="https://github.com/$REPOSITORY/releases/download/$VERSION"
  curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --max-time 180 "$BASE/$ARCHIVE" --output "$WORK/$ARCHIVE" || fail '这个版本暂不能公开下载。私测请先用 GitHub CLI 登录受邀账号；不会继续安装。'
  curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --max-time 30 "$BASE/SHA256SUMS" --output "$WORK/SHA256SUMS" || fail '缺少校验文件，拒绝安装。'
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
