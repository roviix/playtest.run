#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

VERSION="${PLAYTEST_VERSION:-}"
DESTINATION="${PLAYTEST_INSTALL_DIR:-$HOME/.local/bin}"
REPOSITORY="roviix/playtest.run"

if [ -z "$VERSION" ]; then
  if command -v curl >/dev/null; then
    REMOTE_VERSION="$(curl --silent --show-error --max-time 5 "${PLAYTEST_DOWNLOAD_BASE:-https://dl.roviix.com/files}/VERSION" 2>/dev/null | tr -d ' \r\n' || true)"
    if [[ "$REMOTE_VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?$ ]]; then
      VERSION="$REMOTE_VERSION"
    fi
  fi
  if [ -z "$VERSION" ] && command -v curl >/dev/null; then
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

[[ "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?$ ]] || fail 'could not determine a release version. Pass one explicitly, e.g. PLAYTEST_VERSION=v0.3.0 bash install.sh'
[[ "$DESTINATION" = /* ]] || fail 'the install directory must be an absolute path.'
for tool in uname tar mktemp; do command -v "$tool" >/dev/null || fail "$tool is required but not installed."; done

case "$(uname -s):$(uname -m)" in
  Darwin:arm64) TARGET=aarch64-apple-darwin ;;
  Darwin:x86_64) TARGET=x86_64-apple-darwin ;;
  Linux:x86_64) TARGET=x86_64-unknown-linux-musl ;;
  Linux:aarch64|Linux:arm64) TARGET=aarch64-unknown-linux-musl ;;
  *) fail 'this script does not cover your platform. On Windows, download the .zip from the releases page. It will not guess an architecture or change system settings.' ;;
esac

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
ARCHIVE="playtest-${VERSION#v}-${TARGET}.tar.gz"
command -v curl >/dev/null || fail 'curl is required but not installed.'
R2_BASE="${PLAYTEST_DOWNLOAD_BASE:-https://dl.roviix.com/files}"
DOWNLOADED=0

# 1. 优先走 Cloudflare R2 / dl.roviix.com 的边缘分发，全球和大陆都不用代理
if [ -n "$R2_BASE" ]; then
  if curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --connect-timeout 6 --max-time 60 "$R2_BASE/$ARCHIVE" --output "$WORK/$ARCHIVE" 2>/dev/null && \
     curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --connect-timeout 6 --max-time 15 "$R2_BASE/SHA256SUMS" --output "$WORK/SHA256SUMS" 2>/dev/null; then
    DOWNLOADED=1
  fi
fi

# 2. CDN 没命中且本机登录过 GitHub CLI，就走 gh release download
if [ "$DOWNLOADED" -eq 0 ] && command -v gh >/dev/null && gh auth status >/dev/null 2>&1; then
  if gh release download "$VERSION" --repo "$REPOSITORY" --pattern "$ARCHIVE" --pattern SHA256SUMS --dir "$WORK" 2>/dev/null; then
    DOWNLOADED=1
  fi
fi

# 3. 还是没拿到，回退到 GitHub Releases 的公开直链
if [ "$DOWNLOADED" -eq 0 ]; then
  BASE="https://github.com/$REPOSITORY/releases/download/$VERSION"
  curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --max-time 180 "$BASE/$ARCHIVE" --output "$WORK/$ARCHIVE" || fail "download failed: $ARCHIVE was not reachable at $R2_BASE or on GitHub Releases. Check your network, download it by hand from https://github.com/$REPOSITORY/releases, or retry with a pinned version: PLAYTEST_VERSION=$VERSION bash install.sh"
  curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --max-time 30 "$BASE/SHA256SUMS" --output "$WORK/SHA256SUMS" || fail 'the checksum file is missing. Refusing to install.'
fi

EXPECTED="$(awk -v name="$ARCHIVE" '$2 == name || $2 == "*" name { print $1 }' "$WORK/SHA256SUMS")"
[[ "$EXPECTED" =~ ^[a-fA-F0-9]{64}$ ]] || fail 'the checksum file has no single valid checksum for this archive.'
if command -v shasum >/dev/null; then
  ACTUAL="$(shasum -a 256 "$WORK/$ARCHIVE" | awk '{print $1}')"
elif command -v sha256sum >/dev/null; then
  ACTUAL="$(sha256sum "$WORK/$ARCHIVE" | awk '{print $1}')"
else
  fail 'neither shasum nor sha256sum is available, so the archive cannot be verified.'
fi
[[ "$ACTUAL" == "$EXPECTED" ]] || fail 'checksum mismatch. Nothing was installed and any existing binary is untouched.'
ENTRY="playtest-${VERSION#v}-${TARGET}/playtest"
tar -tzf "$WORK/$ARCHIVE" > "$WORK/entries" || fail 'the archive could not be read.'
[[ "$(awk -v entry="$ENTRY" '$0 == entry { count++ } END {print count+0}' "$WORK/entries")" == 1 ]] || fail 'the archive does not contain exactly one playtest binary.'
tar -xOzf "$WORK/$ARCHIVE" "$ENTRY" > "$WORK/playtest" || fail 'extracting the binary failed.'
[[ -s "$WORK/playtest" ]] || fail 'the extracted binary is empty.'
mkdir -p "$DESTINATION"
TEMPORARY="$(mktemp "$DESTINATION/.playtest-install.XXXXXX")"
trap 'rm -rf "$WORK"; rm -f "${TEMPORARY:-}"' EXIT
cat "$WORK/playtest" > "$TEMPORARY"
chmod 755 "$TEMPORARY"
mv -f "$TEMPORARY" "$DESTINATION/playtest"
printf 'Installed playtest %s to %s/playtest\n' "$VERSION" "$DESTINATION"
case ":$PATH:" in
  *:"$DESTINATION":*) ;;
  *) printf 'Add %s to your PATH to run it as `playtest`.\n' "$DESTINATION" ;;
esac
