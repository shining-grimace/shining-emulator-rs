#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

SUITE_ID="mts-20240926-1737-443f6e1"
SUITE_URL="https://gekkio.fi/files/mooneye-test-suite/$SUITE_ID/$SUITE_ID.tar.xz"
ROM_ROOT="$REPO_ROOT/.test-roms/mooneye"
ACCEPTANCE_ROOT="$ROM_ROOT/acceptance"
LICENSE_PATH="$ROM_ROOT/LICENSE"
READY_MARKER="$ROM_ROOT/.acceptance-suite-$SUITE_ID.ready"

needs_download=false
if [[ ! -f "$LICENSE_PATH" ]]; then
    needs_download=true
fi
if [[ ! -f "$READY_MARKER" ]]; then
    needs_download=true
fi

if [[ "$needs_download" == true ]]; then
    TMP_DIR="$(mktemp -d)"
    trap 'rm -rf "$TMP_DIR"' EXIT

    echo "Downloading Mooneye Test Suite $SUITE_ID..."
    curl -fL "$SUITE_URL" -o "$TMP_DIR/$SUITE_ID.tar.xz"

    rm -rf "$ACCEPTANCE_ROOT"
    mkdir -p "$ROM_ROOT"
    tar -xf "$TMP_DIR/$SUITE_ID.tar.xz" -C "$ROM_ROOT" --strip-components=1 "$SUITE_ID/acceptance"
    tar -xOf "$TMP_DIR/$SUITE_ID.tar.xz" "$SUITE_ID/LICENSE" > "$LICENSE_PATH"
    touch "$READY_MARKER"
fi

SHINING_MOONEYE_ROM_ROOT="$ROM_ROOT" cargo test -p app mooneye -- --ignored "$@"
