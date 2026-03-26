#!/usr/bin/env bash
# build.sh — Compile the BuT Tree-sitter grammar to WASM.
#
# Requirements:
#   - Node.js >= 18
#   - One of: emcc (Emscripten), docker, or podman
#     Docker:      brew install --cask docker
#     Emscripten:  brew install emscripten
#     Podman:      brew install podman
#
# Usage:
#   ./scripts/build.sh [--skip-generate]
#
# Output:
#   grammars/but.wasm   — compiled WASM grammar (used by install.sh)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXT_DIR="$(dirname "$SCRIPT_DIR")"
GRAMMAR_DIR="$EXT_DIR/grammars/but"
OUT_WASM="$EXT_DIR/grammars/but.wasm"

echo "==> BuT Tree-sitter grammar builder"
echo "    Extension:  $EXT_DIR"
echo "    Grammar:    $GRAMMAR_DIR"

# ── Check dependencies ────────────────────────────────────────────────────────

check_cmd() {
    if ! command -v "$1" &>/dev/null; then
        echo "ERROR: '$1' not found.  Install: $2"
        exit 1
    fi
}

check_cmd node "brew install node  OR  https://nodejs.org"
check_cmd npm  "included with Node.js"

# WASM compilation requires one of: emcc, docker, or podman
if ! command -v emcc &>/dev/null && ! command -v docker &>/dev/null && ! command -v podman &>/dev/null; then
    echo "ERROR: WASM compilation requires emcc, docker, or podman."
    echo "  Docker:      brew install --cask docker"
    echo "  Emscripten:  brew install emscripten"
    echo "  Podman:      brew install podman"
    exit 1
fi

echo "    Node: $(node --version)"
if command -v docker &>/dev/null; then
    echo "    WASM compiler: docker ($(docker --version | cut -d' ' -f3 | tr -d ','))"
elif command -v podman &>/dev/null; then
    echo "    WASM compiler: podman"
else
    echo "    WASM compiler: emscripten ($(emcc --version | head -1))"
fi

# ── Install npm dependencies (includes tree-sitter-cli locally) ───────────────

echo ""
echo "==> Installing grammar npm dependencies …"
(cd "$GRAMMAR_DIR" && npm install --silent)

TREE_SITTER="$GRAMMAR_DIR/node_modules/.bin/tree-sitter"

# ── Generate parser source (grammar.js → src/parser.c) ───────────────────────

if [[ "${1:-}" != "--skip-generate" ]]; then
    echo ""
    echo "==> Generating parser source from grammar.js …"
    (cd "$GRAMMAR_DIR" && "$TREE_SITTER" generate)
fi

# ── Build WASM ────────────────────────────────────────────────────────────────

echo ""
echo "==> Compiling grammar to WASM …"
# tree-sitter build --wasm uses emcc, docker, or podman automatically.
(cd "$GRAMMAR_DIR" && "$TREE_SITTER" build --wasm --output "$OUT_WASM")

# Fallback: some CLI versions output to CWD
if [[ ! -f "$OUT_WASM" ]]; then
    for candidate in \
        "$GRAMMAR_DIR/tree-sitter-but.wasm" \
        "$EXT_DIR/tree-sitter-but.wasm"; do
        if [[ -f "$candidate" ]]; then
            mv "$candidate" "$OUT_WASM"
            break
        fi
    done
fi

[[ -f "$OUT_WASM" ]] || { echo "ERROR: WASM file not found after build."; exit 1; }

echo ""
echo "==> Build complete: $OUT_WASM"
ls -lh "$OUT_WASM"
echo ""
echo "Run  ./scripts/install.sh  to install the extension into Zed."
