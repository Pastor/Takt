#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd -P)"
WORK="$(mktemp -d)"
WORK="$(cd "$WORK" && pwd -P)"
trap 'rm -rf -- "$WORK"' EXIT
. "$ROOT/scripts/build-artifacts.sh"

target="$WORK/target/precheck"
cmake="$WORK/cmake"
mkdir -p "$target/debug"/{.fingerprint,build,deps,incremental}
mkdir -p "$target/wasm32-unknown-unknown/wasm/deps" "$cmake/CMakeFiles"
touch "$target/debug/taktc" "$target/debug/taktc.d" "$target/debug/libtakt.rlib"
touch "$target/wasm32-unknown-unknown/wasm/takt_wasm.wasm" "$cmake/stacker"
chmod +x "$target/debug/taktc" "$cmake/stacker"
(
    build_artifacts_start "$target" "$cmake"
    [[ -d "$target/debug/incremental" && -d "$cmake/CMakeFiles" ]]
    if (build_artifacts_start "$target") 2>/dev/null; then
        echo "Второй запуск получил занятый каталог" >&2
        exit 1
    fi
)
[[ -x "$target/debug/taktc" && -x "$cmake/stacker" ]]
[[ -f "$target/wasm32-unknown-unknown/wasm/takt_wasm.wasm" ]]
[[ ! -e "$target/debug/incremental" && ! -e "$target/debug/deps" ]]
[[ ! -e "$target/debug/.fingerprint" && ! -e "$target/debug/build" ]]
[[ ! -e "$target/debug/taktc.d" && ! -e "$target/debug/libtakt.rlib" ]]
[[ ! -e "$target/wasm32-unknown-unknown/wasm/deps" && ! -e "$cmake/CMakeFiles" ]]
[[ ! -e "$target.lock" ]]

target="$WORK/target/install"
outside="$WORK/outside"
mkdir -p "$target/release/deps" "$outside/tmp" "$outside/wasm"
touch "$outside/tmp/keep" "$outside/wasm/keep"
ln -s "$outside" "$target/wasm32-unknown-unknown"
status=0
(
    build_artifacts_start "$target"
    exit 27
) || status=$?
[[ "$status" == 27 && ! -e "$target/release/deps" && ! -e "$target.lock" ]]
[[ -f "$outside/tmp/keep" && -f "$outside/wasm/keep" ]]
echo "Очистка сборки: модули сохранены, кэши удалены, код ошибки сохранён."
