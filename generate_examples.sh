#!/usr/bin/env bash
# Генерирует C-файлы из BuT-примеров в grammar/.output/
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

cargo build --bin butc 2>/dev/null

BUTC="./target/debug/butc"
OUTPUT="examples/generated/c"

echo "Генерация C-кода из примеров BuT..."
for but_file in examples/*.but; do
    name="$(basename "$but_file" .but)"
    echo "  $but_file → $OUTPUT/${name}.c / ${name}.h"
    $BUTC compile "$but_file" -o "$OUTPUT" || echo "    [предупреждение] ошибка генерации $but_file"
done
echo "Готово. Файлы в $OUTPUT/"
