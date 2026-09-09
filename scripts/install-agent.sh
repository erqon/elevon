#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-latest}"

REPO_PATH="erqon/elevon"

CRATE_NAME="elevon-agent"
BIN_NAME="elevon-agent"
INSTALL_DIR="${ELEVON_INSTALL_DIR:-/usr/local/bin}"

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

die() {
    echo "error: $*" >&2
    exit 1
}

need() {
    command -v "$1" >/dev/null || die "missing dependency: $1"
}

resolve_version() {
    if [[ "$VERSION" != "latest" ]]; then
        echo "${VERSION#v}"
        return
    fi

    curl --fail --location --silent --show-error \
        "https://api.github.com/repos/${REPO_PATH}/releases/latest" |
        sed -n 's/^[[:space:]]*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' |
        sed 's/^elevon-agent-v//' |
        head -n 1
}

release_url() {
    local version="$1"
    local asset="$2"

    echo "https://github.com/${REPO_PATH}/releases/download/elevon-agent-v${version}/${asset}"
}

main() {
    [[ "$EUID" -eq 0 ]] || die "agent installation must be run as root (try: sudo $0)"

    need curl
    need tar
    need install

    local version asset url tmp_dir source
    version="$(resolve_version)"
    [[ -n "$version" ]] || die "could not resolve the latest agent release"

    asset="$(bash "$SCRIPT_DIR/resolve-release-archive.sh" "$CRATE_NAME" "$version")"
    url="$(release_url "$version" "$asset")"
    tmp_dir="$(mktemp -d)"
    trap 'rm -rf "$tmp_dir"' EXIT

    echo "Downloading ${url}"
    curl --fail --location --show-error "$url" --output "$tmp_dir/$asset"
    tar -xzf "$tmp_dir/$asset" -C "$tmp_dir"

    source="$(find "$tmp_dir" -type f -name "$BIN_NAME" -print -quit)"
    [[ -n "$source" ]] || die "binary '$BIN_NAME' not found in archive"

    mkdir -p "$INSTALL_DIR"
    install -m 0755 "$source" "$INSTALL_DIR/$BIN_NAME"

    echo "Installed $INSTALL_DIR/$BIN_NAME"
    if command -v "$BIN_NAME" >/dev/null; then
        echo "Ready: $(command -v "$BIN_NAME")"
    else
        echo "Add $HOME/.local/bin to PATH to use $BIN_NAME."
    fi
}

main "$@"
