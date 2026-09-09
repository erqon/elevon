#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-latest}"

REPO_PATH="${ELEVON_REPO:-elevon-sh/elevon}"

CRATE_NAME="elevon-cli"
BIN_NAME="elevon"
INSTALL_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/elevon/bin"
LINK_PATH="$HOME/.local/bin/$BIN_NAME"

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
        sed 's/^elevon-cli-v//' |
        head -n 1
}

release_url() {
    local version="$1"
    local asset="$2"

    echo "https://github.com/${REPO_PATH}/releases/download/elevon-cli-v${version}/${asset}"
}

main() {
    need curl
    need tar
    need install

    local version asset url tmp_dir source
    version="$(resolve_version)"
    [[ -n "$version" ]] || die "could not resolve the latest CLI release"

    asset="$(bash "$SCRIPT_DIR/resolve-release-archive.sh" "$CRATE_NAME" "$version")"
    url="$(release_url "$version" "$asset")"
    tmp_dir="$(mktemp -d)"
    trap 'rm -rf "$tmp_dir"' EXIT

    echo "Downloading ${url}"
    curl --fail --location --show-error "$url" --output "$tmp_dir/$asset"
    tar -xzf "$tmp_dir/$asset" -C "$tmp_dir"

    source="$(find "$tmp_dir" -type f -name "$BIN_NAME" -print -quit)"
    [[ -n "$source" ]] || die "binary '$BIN_NAME' not found in archive"

    mkdir -p "$INSTALL_DIR" "$(dirname "$LINK_PATH")"
    install -m 0755 "$source" "$INSTALL_DIR/$BIN_NAME"
    ln -sfn "$INSTALL_DIR/$BIN_NAME" "$LINK_PATH"

    echo "Installed $INSTALL_DIR/$BIN_NAME"
    if command -v "$BIN_NAME" >/dev/null; then
        echo "Ready: $(command -v "$BIN_NAME")"
    else
        echo "Add $HOME/.local/bin to PATH to use $BIN_NAME."
    fi
}

main "$@"
