#!/usr/bin/env bash
# Запускает все симуляции из examples/simulations/ по очереди.
# Для каждого файла вида <модель>_<сценарий>.json ищет examples/<модель>.lam.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SIM_DIR="$SCRIPT_DIR/examples/simulations"
BINARY="$SCRIPT_DIR/target/debug/simulation"

if [[ ! -x "$BINARY" ]]; then
  echo "Бинарник не найден: $BINARY"
  echo "Запустите: cargo build --bin simulation"
  exit 1
fi

pass=0
fail=0
skip=0

for sim_file in "$SIM_DIR"/*.json; do
  [[ -f "$sim_file" ]] || continue

  # Имя файла без пути и расширения: stacker_loading
  base="$(basename "$sim_file" .json)"

  # Имя модели — часть до первого подчёркивания: stacker
  model="${base%%_*}"
  lam_file="$SCRIPT_DIR/examples/${model}.lam"

  if [[ ! -f "$lam_file" ]]; then
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
  if output="$("$BINARY" "$lam_file" -s "$sim_file" $step_arg 2>&1)"; then
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
