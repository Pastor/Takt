#!/bin/sh
# test-new-feature.sh — регресс-тест генератора заготовок (фича 0094).
#
# Гоняет scripts/new-feature.sh в ВРЕМЕННОМ дереве (через NF_ROOT), не трогая
# рабочие реестры, и проверяет:
#   A1 — повторный --register НЕ дублирует строки (идемпотентность);
#   A2 — ADR-строка реестра и шаблон — Draft, не Accepted;
#   A3 — --stage report создаёт и регистрирует отчёт (ранее невозможно);
#   A4 — --subtask NN добирает XXXX-NN идемпотентно;
#   A5 — дефолтный путь заведения даёт ожидаемый набор файлов.
#
# POSIX sh, без внешних зависимостей. Подключён в precheck.sh.
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GEN="$ROOT/scripts/new-feature.sh"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# Слепок дерева: шаблоны (реальные) + пустые реестры с таблицей.
mkdir -p "$TMP/docs/templates"
cp "$ROOT/docs/templates/"*.md "$TMP/docs/templates/"
for r in features adr analyze development tests reports; do
  mkdir -p "$TMP/docs/$r"
  printf '# Реестр\n\n| A | B |\n|---|---|\n' > "$TMP/docs/$r/README.md"
done

fail=0
check() { # <описание> <ожидаемо> <фактически>
  if [ "$2" = "$3" ]; then
    echo "  OK: $1"
  else
    echo "  ПРОВАЛ: $1 — ожидалось '$2', получено '$3'" >&2
    fail=1
  fi
}

echo "test-new-feature: идемпотентность и стадии (фича 0094)..."

# Два прогона заведения одной фичи — строки не должны дублироваться (A1).
NF_ROOT="$TMP" "$GEN" --with-dev --register 0099 test-feat "Тест фича" >/dev/null 2>&1
NF_ROOT="$TMP" "$GEN" --with-dev --register 0099 test-feat "Тест фича" >/dev/null 2>&1

for r in features adr analyze tests; do
  n="$(grep -c '0099' "$TMP/docs/$r/README.md" || true)"
  check "A1 реестр $r — одна строка 0099" "1" "$n"
done
n="$(grep -c '0099-01' "$TMP/docs/development/README.md" || true)"
check "A1 реестр development — одна строка 0099-01" "1" "$n"

# A2: ADR-строка реестра — Draft, не Accepted; шаблон тоже Draft.
draft="$(grep -c 'Draft' "$TMP/docs/adr/README.md" || true)"
check "A2 ADR-строка реестра содержит Draft" "1" "$draft"
acc="$(grep -c 'Accepted' "$TMP/docs/adr/README.md" || true)"
check "A2 ADR-строка реестра НЕ Accepted" "0" "$acc"
tpl_acc="$(grep -c '^- \*\*Status:\*\* Accepted' "$ROOT/docs/templates/adr.md" || true)"
check "A2 шаблон adr.md НЕ Accepted" "0" "$tpl_acc"

# A3: --stage report создаёт и регистрирует отчёт (ранее невозможно).
NF_ROOT="$TMP" "$GEN" --stage report --register 0099 test-feat "Тест фича" >/dev/null 2>&1
[ -f "$TMP/docs/reports/0099-test-feat.md" ] && rep_file=1 || rep_file=0
check "A3 отчёт-заготовка создана" "1" "$rep_file"
rep_row="$(grep -c '0099' "$TMP/docs/reports/README.md" || true)"
check "A3 строка reports/README одна" "1" "$rep_row"
# Повтор --stage report не дублирует.
NF_ROOT="$TMP" "$GEN" --stage report --register 0099 test-feat "Тест фича" >/dev/null 2>&1
rep_row2="$(grep -c '0099' "$TMP/docs/reports/README.md" || true)"
check "A3 повтор --stage report не дублирует" "1" "$rep_row2"

# A4: --subtask 03 добирает 0099-03 идемпотентно.
NF_ROOT="$TMP" "$GEN" --stage dev --subtask 03 --register 0099 test-feat "Тест фича" >/dev/null 2>&1
NF_ROOT="$TMP" "$GEN" --stage dev --subtask 03 --register 0099 test-feat "Тест фича" >/dev/null 2>&1
[ -f "$TMP/docs/development/0099-03-test-feat.md" ] && sub_file=1 || sub_file=0
check "A4 dev-подзадача 0099-03 создана" "1" "$sub_file"
sub_row="$(grep -c '0099-03' "$TMP/docs/development/README.md" || true)"
check "A4 строка 0099-03 одна (идемпотентно)" "1" "$sub_row"

# A5: дефолтный путь для НОВОЙ фичи создаёт feature/adr/analyze.
NF_ROOT="$TMP" "$GEN" 0088 other-feat "Другая" >/dev/null 2>&1
for f in features/0088-other-feat adr/0088-other-feat analyze/0088-other-feat; do
  [ -f "$TMP/docs/$f.md" ] || { echo "  ПРОВАЛ: A5 нет $f.md" >&2; fail=1; }
done
[ "$fail" = 0 ] && echo "  OK: A5 дефолтный путь создаёт feature/adr/analyze"

if [ "$fail" != 0 ]; then
  echo "test-new-feature: ПРОВАЛЕНО" >&2
  exit 1
fi
echo "test-new-feature: все проверки пройдены (A1–A5)."
