#!/usr/bin/env bash
set -euo pipefail

CRATE="${1:?usage: $0 <crate> <tag>}"
VERSION_TAG="${2:-}"

os="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"

case "$os" in
    linux) os="linux" ;;
    darwin) os="darwin" ;;
    *) echo "unsupported os: $os" >&2; exit 1 ;;
esac

case "$arch" in
    x86_64|amd64) arch="x86_64" ;;
    arm64|aarch64) arch="aarch64" ;;
    *) echo "unsupported arch: $arch" >&2; exit 1 ;;
esac

ARCHIVE="${CRATE}-${VERSION_TAG}-${os}-${arch}.tar.gz"
echo "$ARCHIVE"
