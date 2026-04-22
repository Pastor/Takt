#!/usr/bin/env bash
# install.sh — Install the Lam Zed extension (language files only).
#
# Syntax highlighting, outline, and hover are provided by the lam-lsp server.
# Tree-sitter grammar is NOT used.
#
# Usage:
#   ./scripts/install.sh              # install extension files
#   ./scripts/install.sh --uninstall  # remove extension

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXT_DIR="$(dirname "$SCRIPT_DIR")"

ZED_BASE="$HOME/Library/Application Support/Zed"
ZED_EXT_DIR="$ZED_BASE/extensions/installed"
ZED_INDEX="$ZED_BASE/extensions/index.json"

EXT_ID="lam"
INSTALL_DIR="$ZED_EXT_DIR/$EXT_ID"

MODE="${1:-}"

die() { echo "ERROR: $*" >&2; exit 1; }
require_python() { command -v python3 &>/dev/null || die "python3 is required."; }

# ── Uninstall ─────────────────────────────────────────────────────────────────

if [[ "$MODE" == "--uninstall" ]]; then
    echo "==> Uninstalling Lam Zed extension …"
    [[ -d "$INSTALL_DIR" ]] && rm -rf "$INSTALL_DIR" && echo "    Removed: $INSTALL_DIR"
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

# ── Install ───────────────────────────────────────────────────────────────────

echo "==> Installing Lam extension for Zed"
echo "    Source:  $EXT_DIR"
echo "    Target:  $INSTALL_DIR"

require_python

mkdir -p "$INSTALL_DIR/languages"

cp "$EXT_DIR/extension.toml" "$INSTALL_DIR/"

if [[ -d "$EXT_DIR/languages/lam" ]]; then
    rm -rf "$INSTALL_DIR/languages/lam"
    cp -r  "$EXT_DIR/languages/lam" "$INSTALL_DIR/languages/"
    echo "    Language files copied."
fi

# ── Update index.json ─────────────────────────────────────────────────────────

echo ""
echo "==> Updating Zed extension index …"

[[ ! -f "$ZED_INDEX" ]] && echo '{"extensions":{}}' > "$ZED_INDEX"

python3 - "$ZED_INDEX" "$EXT_ID" <<'PYEOF'
import json, sys
path, eid = sys.argv[1], sys.argv[2]

with open(path) as f:
    data = json.load(f)

exts = data.setdefault("extensions", {})
exts[eid] = {
    "manifest": {
        "id": eid,
        "name": "Lam",
        "version": "0.1.0",
        "schema_version": 1,
        "description": "Lam FSM language support: LSP-based highlighting and outline for .lam files",
        "repository": "https://github.com/Pastor/BuT",
        "authors": ["Lam Team"],
        "lib": {"kind": None, "version": None},
        "themes": [],
        "icon_themes": [],
        "languages": ["languages/lam"],
        "grammars": {},
        "language_servers": {
            "lam-lsp": {"name": "lam-lsp"}
        },
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
echo "  1. Ensure lam-lsp is installed:"
echo "       cargo install --path grammar --features lsp --bin lam-lsp"
echo "  2. Restart Zed (or Cmd+Shift+P → 'zed: reload extensions')."
echo "  3. Open any .lam file — LSP will provide highlighting and outline."
