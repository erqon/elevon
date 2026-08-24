#!/usr/bin/env bash
set -euo pipefail

TAGS_URL="${1:?error: URL argument (\$1) is required}"
VERSION="${2:?error: VERSION argument (\$2) is required}"

if [[ "$VERSION" =~ ^(.*v)[0-9] ]]; then
    PREFIX="${BASH_REMATCH[1]}"
elif [[ "$VERSION" == *v ]]; then
    PREFIX="$VERSION"
else
    # Fallback: keep full input as prefix if no version digits are present
    PREFIX="$VERSION"
fi

RESPONSE="$(curl -s "$TAGS_URL")"

SELECTED_TAG=$(echo "$RESPONSE" | jq -r \
    --arg target "$VERSION" \
    --arg prefix "$PREFIX" '
    # 1. Try exact match for target version
    (map(select(.name == $target))[0].name)
    //
    # 2. Fallback to first tag matching the extracted prefix
    (map(select(.name | startswith($prefix)))[0].name)
    //
    empty
')

if [[ -z "$SELECTED_TAG" ]]; then
    echo "error: no matching tag found for version '$VERSION' or prefix '$PREFIX'" >&2
    exit 1
fi

echo "$SELECTED_TAG"
