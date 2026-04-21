#!/usr/bin/env bash
# Генерирует C-файлы из BuT-примеров в grammar/.output/
set -euo pipefail

if command -v rtk &>/dev/null; then
  CARGO_CMD="rtk cargo"
else
  CARGO_CMD="cargo"
fi

$CARGO_CMD +nightly fmt
$CARGO_CMD check
$CARGO_CMD clippy --all-targets --all-features
$CARGO_CMD test -- --test-threads=1

#if command -v shfmt &>/dev/null; then
#  shfmt -w **/*.sh
#fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

$CARGO_CMD build --bin butc 2>/dev/null
$CARGO_CMD build --features lsp --bin but-lsp

BUTC="./target/debug/butc"
OUTPUT="examples/generated/c"

echo "Генерация C-кода из примеров BuT..."
for but_file in examples/*.but; do
  name="$(basename "$but_file" .but)"
  echo "  $but_file → $OUTPUT/${name}.c / ${name}.h"
  $BUTC compile "$but_file" -o "$OUTPUT" || echo "    [предупреждение] ошибка генерации $but_file"
done
echo "Готово. Файлы в $OUTPUT/"
cmake -DCMAKE_BUILD_TYPE=Debug -G Ninja -S $OUTPUT -B $OUTPUT/cmake-build-debug/
cd $OUTPUT/cmake-build-debug/ && ninja
cd -
