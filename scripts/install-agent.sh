#!/usr/bin/env bash
set -euo pipefail

REPO="${ELEVON_REPO:-elevon-sh/elevon}"
BIN_NAME="elevon-agent"
VERSION="${VERSION:-latest}"

INSTALL_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/elevon/bin"
LINK_PATH="$HOME/.local/bin/$BIN_NAME"

die()  { echo "error: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null || die "missing dependency: $1"; }

detect_target() {
  local os arch
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"

  case "$arch" in
    x86_64|amd64)  arch="x86_64" ;;
    aarch64|arm64) arch="aarch64" ;;
    *) die "unsupported arch: $arch" ;;
  esac

  case "$os" in
    linux|darwin) ;;
    *) die "unsupported os: $os" ;;
  esac

  echo "${os}-${arch}"
}

release_url() {
  local asset="$1"
  if [[ "$VERSION" == "latest" ]]; then
    echo "https://github.com/${REPO}/releases/latest/download/${asset}"
  else
    echo "https://github.com/${REPO}/releases/download/elevon-agent-v${VERSION#v}/${asset}"
  fi
}

download_and_extract() {
  local url="$1" asset tmp
  asset="$(basename "$url")"
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT

  echo "Downloading $url"
  curl -fL "$url" -o "$tmp/$asset"
  tar -xzf "$tmp/$asset" -C "$tmp"

  find "$tmp" -type f -name "$BIN_NAME" -print -quit
}

install_binary() {
  local src="$1"
  mkdir -p "$INSTALL_DIR" "$(dirname "$LINK_PATH")"
  install -m 0755 "$src" "$INSTALL_DIR/$BIN_NAME"
  ln -sfn "$INSTALL_DIR/$BIN_NAME" "$LINK_PATH"
}

ensure_path_hint() {
  if command -v "$BIN_NAME" >/dev/null; then
    echo "Ready: $(command -v "$BIN_NAME")"
    return
  fi

  cat <<EOF

Add ~/.local/bin to your PATH, then restart the shell:
  echo 'export PATH="\$HOME/.local/bin:\$PATH"' >> ~/.bashrc
EOF
}

main() {
  need curl
  need tar

  local target asset url src
  target="$(detect_target)"
  asset="${BIN_NAME}-${target}.tar.gz"
  url="$(release_url "$asset")"
  src="$(download_and_extract "$url")"

  [[ -n "$src" ]] || die "binary '$BIN_NAME' not found in archive"

  install_binary "$src"
  echo "Installed $INSTALL_DIR/$BIN_NAME"
  ensure_path_hint

  echo
  echo "Optional: sudo $BIN_NAME install-systemd --enable"
}

main "$@"
