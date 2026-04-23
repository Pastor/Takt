#!/usr/bin/env bash
# Генерирует C-файлы из Lam-примеров в grammar/.output/
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

$CARGO_CMD build --bin lamc 2>/dev/null
$CARGO_CMD build --features lsp --bin lam-lsp

LAMC="./target/debug/lamc"
C_OUTPUT="examples/generated/c"
PLANTUML_OUTPUT="examples/generated/plantuml"

echo "Генерация C-кода из примеров Lam..."
for lam_file in examples/*.lam; do
  name="$(basename "$lam_file" .lam)"
  echo "  $lam_file → $C_OUTPUT/${name}.c / ${name}.h"
  $LAMC compile "$lam_file" -o "$C_OUTPUT" || echo "    [предупреждение] ошибка генерации $lam_file"
  $LAMC compile "$lam_file" -t plantuml -o "$PLANTUML_OUTPUT" || echo "    [предупреждение] ошибка генерации $lam_file"
done
echo "Готово. Файлы в $C_OUTPUT/"
cmake -DCMAKE_BUILD_TYPE=Debug -G Ninja -S $C_OUTPUT -B $C_OUTPUT/cmake-build-debug/
cd $C_OUTPUT/cmake-build-debug/ && ninja
cd -
