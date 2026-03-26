#!/usr/bin/env bash
# install.sh — Install the BuT Zed extension on macOS.
#
# The script copies the extension into Zed's installed-extensions directory and
# updates the extension index so Zed recognises it on next launch.
#
# For the grammar, two modes are available:
#
#   (default) Skip grammar — copy language files only and let Zed handle the
#             grammar through its own mechanism (preferred when tree-sitter +
#             Emscripten are not available).
#
#   --compile Compile grammar/but/grammar.js → but.wasm and include it in
#             the installed extension.  Requires tree-sitter-cli + Emscripten.
#             See scripts/build.sh for dependency details.
#
# Usage:
#   ./scripts/install.sh              # install (skip WASM compile)
#   ./scripts/install.sh --compile    # build WASM then install
#   ./scripts/install.sh --uninstall  # remove extension
#
# Zed macOS paths:
#   Extensions index: ~/Library/Application Support/Zed/extensions/index.json
#   Installed:        ~/Library/Application Support/Zed/extensions/installed/<id>/
#   Grammars cache:   ~/Library/Application Support/Zed/grammars/<grammar>.wasm

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXT_DIR="$(dirname "$SCRIPT_DIR")"

ZED_BASE="$HOME/Library/Application Support/Zed"
ZED_EXT_DIR="$ZED_BASE/extensions/installed"
ZED_INDEX="$ZED_BASE/extensions/index.json"
ZED_GRAMMARS="$ZED_BASE/grammars"    # Zed's compiled-grammar cache

EXT_ID="but"
INSTALL_DIR="$ZED_EXT_DIR/$EXT_ID"

MODE="${1:-}"

# ─────────────────────────────────────────────────────────────────────────────

die() { echo "ERROR: $*" >&2; exit 1; }

require_python() {
    command -v python3 &>/dev/null || die "python3 is required for JSON updates."
}

# ── Uninstall ─────────────────────────────────────────────────────────────────

if [[ "$MODE" == "--uninstall" ]]; then
    echo "==> Uninstalling BuT Zed extension …"

    # Remove installed extension directory
    if [[ -d "$INSTALL_DIR" ]]; then
        rm -rf "$INSTALL_DIR"
        echo "    Removed: $INSTALL_DIR"
    fi

    # Remove grammar WASM from Zed's grammar cache
    if [[ -f "$ZED_GRAMMARS/but.wasm" ]]; then
        rm -f "$ZED_GRAMMARS/but.wasm"
        echo "    Removed: $ZED_GRAMMARS/but.wasm"
    fi

    # Remove entry from index.json
    require_python
    if [[ -f "$ZED_INDEX" ]]; then
        python3 - "$ZED_INDEX" "$EXT_ID" <<'PYEOF'
import json, sys
path, eid = sys.argv[1], sys.argv[2]
with open(path) as f: data = json.load(f)
exts = data.get("extensions", {})
if eid in exts:
    del exts[eid]
    with open(path, "w") as f: json.dump(data, f, indent=2)
    print(f"    Removed '{eid}' from index.json")
else:
    print(f"    '{eid}' not in index.json (already clean)")
PYEOF
    fi

    echo ""
    echo "==> Done. Restart Zed to apply."
    exit 0
fi

# ── Compile mode ──────────────────────────────────────────────────────────────

if [[ "$MODE" == "--compile" ]]; then
    echo "==> Compiling Tree-sitter grammar …"
    bash "$SCRIPT_DIR/build.sh"
    echo ""
fi

# ── Install ───────────────────────────────────────────────────────────────────

echo "==> Installing BuT extension for Zed"
echo "    Source:  $EXT_DIR"
echo "    Target:  $INSTALL_DIR"

require_python

# Ensure extension dir exists
mkdir -p "$INSTALL_DIR/languages" "$INSTALL_DIR/grammars"

# Copy extension.toml
cp "$EXT_DIR/extension.toml" "$INSTALL_DIR/"

# Copy language files
if [[ -d "$EXT_DIR/languages/but" ]]; then
    rm -rf "$INSTALL_DIR/languages/but"
    cp -r  "$EXT_DIR/languages/but" "$INSTALL_DIR/languages/"
    echo "    Language files copied."
fi

# Copy grammar WASM if compiled
WASM_SRC="$EXT_DIR/grammars/but.wasm"
if [[ -f "$WASM_SRC" ]]; then
    # Place in extension grammars dir AND in Zed's shared grammar cache
    cp "$WASM_SRC" "$INSTALL_DIR/grammars/but.wasm"
    mkdir -p "$ZED_GRAMMARS"
    cp "$WASM_SRC" "$ZED_GRAMMARS/but.wasm"
    echo "    Grammar WASM installed: $ZED_GRAMMARS/but.wasm"
else
    echo "    Grammar WASM not found (run with --compile to build it)."
    echo "    Zed will attempt to download/compile the grammar automatically."
fi

# ── Update index.json ─────────────────────────────────────────────────────────

echo ""
echo "==> Updating Zed extension index …"

# Create index.json if missing
if [[ ! -f "$ZED_INDEX" ]]; then
    echo '{"extensions":{}}' > "$ZED_INDEX"
fi

WASM_READY="$([[ -f "$WASM_SRC" ]] && echo true || echo false)"

python3 - "$ZED_INDEX" "$EXT_ID" "$WASM_READY" <<'PYEOF'
import json, sys
path, eid, wasm_ready = sys.argv[1], sys.argv[2], sys.argv[3] == "true"

with open(path) as f:
    data = json.load(f)

exts = data.setdefault("extensions", {})
exts[eid] = {
    "manifest": {
        "id": eid,
        "name": "BuT",
        "version": "0.1.0",
        "schema_version": 1,
        "description": "BuT FSM language support: syntax highlighting for .but files",
        "repository": "https://github.com/BuT/BuT",
        "authors": ["BuT Team"],
        "lib": {"kind": None, "version": None},
        "themes": [],
        "icon_themes": [],
        "languages": ["languages/but"],
        "grammars": {
            eid: {
                "repository": "https://github.com/BuT/tree-sitter-but",
                "rev": "local",
                "path": None,
            }
        },
        "language_servers": {},
        "context_servers": {},
        "agent_servers": {},
        "slash_commands": {},
        "snippets": None,
        "capabilities": [],
    },
    "dev": False,
}

with open(path, "w") as f:
    json.dump(data, f, indent=2)

print(f"    Registered '{eid}' in index.json")
PYEOF

# ── Summary ───────────────────────────────────────────────────────────────────

echo ""
echo "==> Installation complete!"
echo ""
echo "    Extension directory: $INSTALL_DIR"
echo ""
echo "Next steps:"
echo "  1. Restart Zed (or Cmd+Shift+P → 'zed: reload extensions')."
echo "  2. Open any .but file — syntax highlighting should be active."
echo ""
if [[ ! -f "$WASM_SRC" ]]; then
    echo "NOTE: Grammar WASM was not compiled."
    echo "  Option A — Compile manually (requires tree-sitter + Emscripten):"
    echo "    cd extensions/zed-but && npm install tree-sitter-cli"
    echo "    ./scripts/build.sh && ./scripts/install.sh"
    echo ""
    echo "  Option B — Install as a dev extension via Zed UI:"
    echo "    Zed → Extensions → Install Dev Extension"
    echo "    Select directory: $EXT_DIR"
    echo "    Zed will compile the grammar automatically."
fi
