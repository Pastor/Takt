#!/usr/bin/env bash
# Предкоммит-проверка: ссылки в Markdown + fmt + check + clippy + test +
# формат примеров (lamc fmt --check) +
# генерация C/PlantUML из примеров Lam и сборка сгенерированного кода.
# Запускать из любого каталога.
set -euo pipefail

if command -v rtk &>/dev/null; then
  CARGO_CMD="rtk cargo"
else
  CARGO_CMD="cargo"
fi

if command -v python3 &>/dev/null; then
  echo "Проверка ссылок в Markdown (правило 14)..."
  "$(dirname "$0")/check-links.py"
else
  echo "  [пропуск] python3 не найден — проверка ссылок пропущена"
fi

$CARGO_CMD +nightly fmt
$CARGO_CMD check
$CARGO_CMD clippy --all-targets --all-features
$CARGO_CMD test -- --test-threads=1

# Корень репозитория = каталог этого скрипта /..
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

$CARGO_CMD build --bin lamc 2>/dev/null
$CARGO_CMD build --features lsp --bin lam-lsp

LAMC="./target/debug/lamc"

# Канон форматирования примеров (фича 0024). Проверка НЕразрушающая: только код
# возврата. Область — `examples/`: они являются документацией по языку
# (правила 15, 16) и обязаны быть в каноне. Фикстуры `tests/data/` намеренно НЕ
# проверяются: часть тестов завязана на их раскладку и позиции.
echo "Проверка формата примеров (lamc fmt --check)..."
$LAMC fmt --check examples/ || {
  echo "  Примеры не в каноне. Исправить: $LAMC fmt examples/"
  exit 1
}

C_OUTPUT="examples/generated/c"
PLANTUML_OUTPUT="examples/generated/plantuml"
ST_OUTPUT="examples/generated/st"

echo "Генерация C-кода из примеров Lam..."
for lam_file in examples/*.lam; do
  name="$(basename "$lam_file" .lam)"
  echo "  $lam_file → $C_OUTPUT/${name}.c / ${name}.h"
  $LAMC compile "$lam_file" -o "$C_OUTPUT" || echo "    [предупреждение] ошибка генерации $lam_file"
  $LAMC compile "$lam_file" -t plantuml -o "$PLANTUML_OUTPUT" || echo "    [предупреждение] ошибка генерации $lam_file"
  # Цель st (фича 0041). Отказ не валит предкоммит: бэкенд дописывается
  # (задачи 0041-03, 0041-04 часть 3), и на непокрытом узле он ЗАКОНОМЕРНО
  # отвечает ST-011 — это замысел («никакого тихого пропуска»), а не поломка.
  $LAMC compile "$lam_file" -t st -o "$ST_OUTPUT" \
    || echo "    [предупреждение] цель st: $lam_file не транслируется (бэкенд не закончен)"
done
echo "Готово. Файлы в $C_OUTPUT/"
cmake -DCMAKE_BUILD_TYPE=Debug -G Ninja -S $C_OUTPUT -B $C_OUTPUT/cmake-build-debug/
cd $C_OUTPUT/cmake-build-debug/ && ninja
cd -

BUILD_DIR="$ROOT/$C_OUTPUT/cmake-build-debug"
if [ -x "$BUILD_DIR/stacker" ]; then
  echo "Запуск симуляции stacker..."
  "$BUILD_DIR/stacker" > /tmp/stacker_sim.log
  echo "  лог: /tmp/stacker_sim.log ($(wc -l < /tmp/stacker_sim.log) строк)"
else
  echo "  [пропуск] stacker не собран — симуляция пропущена"
fi

# Арбитр валидности порождённого Structured Text (фича 0041). Здесь только
# ОБЕСПЕЧЕНИЕ инструмента: при отсутствии он собирается из исходников и ставится
# в ~/.local/bin/iec2c. Скрипт никогда не валит предкоммит — `iec2c` внешний и
# для сборки/тестов `lamc` не нужен.
#
# Сам ГЕЙТ (прогон iec2c по порождённым .st) пока не включён: у порождённых
# `FUNCTION_BLOCK` нет тела — это задача 0041-03, — и iec2c их закономерно
# отвергает. Включение гейта — задача 0041-05 (см. docs/development/0041-06).
echo "Проверка ST-арбитра (MatIEC iec2c)..."
"$ROOT/scripts/ensure-iec2c.sh" || true

# ГЕЙТ ST: порождённый Structured Text обязан компилироваться (фича 0041).
# Единственный автоматизируемый способ доказать, что вывод валиден по стандарту:
# юнит-тесты проверяют лишь, что генератор напечатал задуманное.
#
# Гейт НЕ валит предкоммит, если инструмента нет: `iec2c` внешний и для сборки
# `lamc` не нужен. Но если он есть — невалидный ST это ОШИБКА.
IEC2C_BIN="${IEC2C_PREFIX:-$HOME/.local}/bin/iec2c"
IEC2C_LIB="${IEC2C_PREFIX:-$HOME/.local}/share/matiec/lib"
if [ -x "$IEC2C_BIN" ] && [ -f "$IEC2C_LIB/ieclib.txt" ]; then
  echo "Гейт ST: проверка порождённого кода транспилятором iec2c..."
  st_failed=0
  for st_file in "$ST_OUTPUT"/*.st; do
    [ -e "$st_file" ] || continue
    name="$(basename "$st_file" .st)"
    out_dir="$(mktemp -d)"
    if "$IEC2C_BIN" -I "$IEC2C_LIB" -T "$out_dir" "$st_file" >/dev/null 2>"$out_dir/err"; then
      echo "  $name → валиден"
    else
      echo "  $name → НЕВАЛИДЕН:"
      sed 's/^/    /' "$out_dir/err" | head -5
      st_failed=1
    fi
    rm -rf "$out_dir"
  done
  if [ "$st_failed" -ne 0 ]; then
    echo "  Порождённый ST не принимается iec2c — предкоммит провален."
    exit 1
  fi
else
  echo "  [пропуск] iec2c недоступен — гейт ST пропущен"
fi
