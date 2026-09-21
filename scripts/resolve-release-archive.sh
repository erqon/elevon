#!/usr/bin/env bash
set -euo pipefail

CRATE="${1:?usage: $0 <crate> <version> [target]}"
VERSION="${2:?usage: $0 <crate> <version> [target]}"
TARGET="${3:-}"

if [[ -n "$TARGET" ]]; then
	case "$TARGET" in
	x86_64-unknown-linux-musl)
		os="linux"
		arch="x86_64"
		;;
	x86_64-apple-darwin)
		os="darwin"
		arch="x86_64"
		;;
	aarch64-apple-darwin)
		os="darwin"
		arch="aarch64"
		;;
	*)
		echo "unsupported target: $TARGET" >&2
		exit 1
		;;
	esac
else
	os="$(uname -s | tr '[:upper:]' '[:lower:]')"
	arch="$(uname -m)"
fi

if [[ -z "$TARGET" ]]; then
	case "$os" in
	linux | darwin) ;;
	mingw* | msys* | cygwin* | windows*)
		echo "Windows is not supported" >&2
		exit 1
		;;
	*)
		echo "unsupported os: $os" >&2
		exit 1
		;;
	esac

	case "$arch" in
	x86_64 | amd64) arch="x86_64" ;;
	arm64 | aarch64) arch="aarch64" ;;
	*)
		echo "unsupported arch: $arch" >&2
		exit 1
		;;
	esac
fi

ARCHIVE="${CRATE}-v${VERSION#v}-${os}-${arch}.tar.gz"
echo "$ARCHIVE"
