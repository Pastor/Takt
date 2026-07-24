#!/usr/bin/env bash
# Запускает все симуляции из examples/simulations/ по очереди.
# Для каждого файла вида <модель>_<сценарий>.json ищет examples/<модель>.lam.
# Запускать из любого каталога.

set -euo pipefail

# Корень репозитория = каталог этого скрипта /..
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SIM_DIR="$ROOT/examples/simulations"
BINARY="$ROOT/target/debug/takt-sim"

if [[ ! -x "$BINARY" ]]; then
  echo "Бинарник не найден: $BINARY"
  echo "Запустите: cargo build --bin takt-sim"
  exit 1
fi

pass=0
fail=0
skip=0

for sim_file in "$SIM_DIR"/*.json; do
  [[ -f "$sim_file" ]] || continue

  # Имя файла без пути и расширения: stacker_loading
  base="$(basename "$sim_file" .json)"

  # Имя модели — самый ДЛИННЫЙ префикс `base` (по `_`), для которого есть .lam.
  # Прежде бралась часть до ПЕРВОГО `_` (`${base%%_*}`), что ломалось на именах
  # моделей с подчёркиванием: `elevator_mini_floor2` → `elevator` вместо
  # `elevator_mini` (фича 0079). Отсекаем суффикс справа, пока не найдём .lam.
  candidate="$base"
  lam_file=""
  model="$candidate"
  while :; do
    if [[ -f "$ROOT/examples/${candidate}.lam" ]]; then
      model="$candidate"
      lam_file="$ROOT/examples/${candidate}.lam"
      break
    fi
    [[ "$candidate" == *_* ]] || break
    candidate="${candidate%_*}"
  done
  output_path="$ROOT/examples/simulations/graphics"
  config_file="$ROOT/examples/graphics-configs/default_svg.json"

  if [[ -z "$lam_file" ]]; then
    echo "[ ПРОПУСК ] $base  (не найден ${model}.lam)"
    ((skip++)) || true
    continue
  fi

  # Количество шагов из JSON (опционально: ограничивает сценарий снаружи)
  n_steps="$(python3 -c "import json,sys; print(len(json.load(open('$sim_file'))))" 2>/dev/null || echo "")"
  step_arg=""
  [[ -n "$n_steps" ]] && step_arg="-n $n_steps"

  # Запуск симуляции
  # shellcheck disable=SC2086
  if output="$("$BINARY" "$lam_file" -s "$sim_file" -o "$output_path" --graphics-config $config_file $step_arg 2>&1)"; then
    echo "[  OK  ] $base"
    ((pass++)) || true
  else
    echo "[ FAIL ] $base"
    echo "$output" | sed 's/^/         /'
    ((fail++)) || true
  fi
done

echo ""
echo "Итого: $pass прошло, $fail упало, $skip пропущено."
[[ $fail -eq 0 ]]
