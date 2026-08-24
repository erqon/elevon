#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"

REPO_PATH="elevon-sh/elevon"
REPO_URL="https://github.com/$REPO_PATH"
TAGS_URL="https://api.github.com/repos/$REPO_PATH/tags"

CRATE_NAME="elevon-cli"
BIN_NAME="elevon"
INSTALL_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/elevon/bin"
LINK_PATH="$HOME/.local/bin/$BIN_NAME"

ASSET="$(bash ./resolve-release-archive.sh $CRATE_NAME)"

die() {
    echo "error: $*" >&2
    exit 1
}
need() { command -v "$1" >/dev/null || die "missing dependency: $1"; }

version() {
    local version
    if [[ -z "$VERSION" ]]; then
        
    fi
}

release_url() {
    local version
    version="$(version)"
}

main() {
    need curl
    need tar

}

main "$@"
