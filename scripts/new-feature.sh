#!/bin/sh
# new-feature.sh — генератор заготовок артефактов фичи из docs/templates/
# (правило 17, фича 0015; доработка — фича 0094). POSIX sh + sed + awk, без
# внешних зависимостей.
#
# Использование:
#   scripts/new-feature.sh [--with-dev] [--register] XXXX slug "Заголовок"
#   scripts/new-feature.sh --stage NAME [--register] [--subtask NN] XXXX slug "Заголовок"
#
#   --with-dev     дополнительно создать заготовки development (XXXX-01) и tests
#   --register     дописать строки в реестры README соответствующих папок
#   --stage NAME   обработать ОДНУ стадию: feature|adr|analyze|dev|tests|report
#                  (добор поздней стадии по мере жизненного цикла; report ранее
#                  был недоступен). Прочие стадии не трогаются.
#   --subtask NN   номер dev-подзадачи для --stage dev (по умолчанию 01)
#
# Идемпотентность (фича 0094): --register НЕ дублирует строки — если строка с
# ключом-номером фичи в реестре уже есть, вставка пропускается. Повторный прогон
# безопасен.
#
# Переменная окружения NF_ROOT переопределяет корень репозитория (по умолчанию —
# каталог скрипта/..). Нужна для тестируемости: scripts/test-new-feature.sh гоняет
# генератор в temp-дереве, не трогая рабочие реестры.
#
# Примеры:
#   scripts/new-feature.sh 0032 my-feature "Моя фича"
#   scripts/new-feature.sh --with-dev --register 0032 my-feature "Моя фича"
#   scripts/new-feature.sh --stage report --register 0032 my-feature "Моя фича"
#   scripts/new-feature.sh --stage dev --subtask 03 --register 0032 my-feature "Моя фича"
set -eu

WITH_DEV=0
REGISTER=0
STAGE=""
SUBTASK="01"
while [ $# -gt 0 ]; do
  case "$1" in
    --with-dev) WITH_DEV=1; shift ;;
    --register) REGISTER=1; shift ;;
    --stage) STAGE="${2:-}"; shift 2 ;;
    --subtask) SUBTASK="${2:-}"; shift 2 ;;
    --) shift; break ;;
    -*) echo "Неизвестный флаг: $1" >&2; exit 2 ;;
    *) break ;;
  esac
done

if [ $# -ne 3 ]; then
  echo "Использование: $0 [--with-dev] [--register] [--stage NAME] [--subtask NN] XXXX slug \"Заголовок\"" >&2
  exit 2
fi

NUM="$1"; SLUG="$2"; TITLE="$3"

# Валидация
case "$NUM" in
  [0-9][0-9][0-9][0-9]) : ;;
  *) echo "Ошибка: XXXX должен быть 4 цифры (напр. 0032), получено: '$NUM'" >&2; exit 2 ;;
esac
case "$SLUG" in
  *[!a-z0-9-]*|"" ) echo "Ошибка: slug — kebab-case латиницей [a-z0-9-], получено: '$SLUG'" >&2; exit 2 ;;
  *) : ;;
esac
case "$SUBTASK" in
  [0-9][0-9]) : ;;
  *) echo "Ошибка: --subtask NN — две цифры (напр. 03), получено: '$SUBTASK'" >&2; exit 2 ;;
esac
case "$STAGE" in
  ""|feature|adr|analyze|dev|tests|report) : ;;
  *) echo "Ошибка: --stage NAME ∈ {feature,adr,analyze,dev,tests,report}, получено: '$STAGE'" >&2; exit 2 ;;
esac

# Корень репозитория: NF_ROOT (для тестов) либо каталог скрипта/..
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT="${NF_ROOT:-$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)}"
TPL="$ROOT/docs/templates"
DATE=$(date +%F)

[ -d "$TPL" ] || { echo "Не найден каталог шаблонов: $TPL" >&2; exit 1; }

# Экранирование заголовка для replacement-части sed (делимитер '|')
TITLE_ESC=$(printf '%s' "$TITLE" | sed -e 's/[\\&|]/\\\&/g')

# render <шаблон> <куда> <repl_yy:0|1>: заполнить шаблон плейсхолдерами. При
# repl_yy=1 подставляет номер подзадачи ($SUBTASK) вместо YY. Существующий файл
# НЕ перезаписывается (идемпотентно).
render() {
  tpl="$1"; dest="$2"; repl_yy="$3"
  [ -f "$TPL/$tpl" ] || { echo "Нет шаблона: $TPL/$tpl" >&2; exit 1; }
  if [ -e "$dest" ]; then
    echo "Пропуск (уже существует): $dest" >&2
    return 0
  fi
  mkdir -p "$(dirname -- "$dest")"
  # Порядок важен: YYYY-MM-DD до YY (иначе YY затрёт часть даты)
  if [ "$repl_yy" = "1" ]; then
    sed -e "s|YYYY-MM-DD|$DATE|g" \
        -e "s|XXXX|$NUM|g" \
        -e "s|YY|$SUBTASK|g" \
        -e "s|slug|$SLUG|g" \
        -e "s|<заголовок>|$TITLE_ESC|g" \
        "$TPL/$tpl" > "$dest"
  else
    sed -e "s|YYYY-MM-DD|$DATE|g" \
        -e "s|XXXX|$NUM|g" \
        -e "s|slug|$SLUG|g" \
        -e "s|<заголовок>|$TITLE_ESC|g" \
        "$TPL/$tpl" > "$dest"
  fi
  echo "Создано: ${dest#$ROOT/}"
}

# insert_row <README> <key> <row>: идемпотентно вставить строку после последней
# строки таблицы. `key` — литерал первого столбца, однозначно определяющий строку
# фичи (напр. "| [0094]", "| 0094 |", "| 0094-01 |"). Если строка с ключом уже
# есть — вставка ПРОПУСКАЕТСЯ (фича 0094: --register не дублирует).
insert_row() {
  readme="$1"; key="$2"; row="$3"
  [ -f "$readme" ] || { echo "Нет реестра: $readme (пропуск)" >&2; return 0; }
  if grep -Fq "$key" "$readme"; then
    echo "Реестр уже содержит запись (пропуск): ${readme#$ROOT/} [$key]" >&2
    return 0
  fi
  tmp="$readme.tmp.$$"
  awk -v row="$row" '
    /^\|/ { last = NR }
    { lines[NR] = $0 }
    END {
      for (i = 1; i <= NR; i++) {
        print lines[i]
        if (i == last) print row
      }
    }
  ' "$readme" > "$tmp" && mv "$tmp" "$readme"
  echo "Реестр обновлён: ${readme#$ROOT/}"
}

# do_stage <name>: рендер заготовки стадии и (при --register) идемпотентная
# регистрация её строки в реестре. Обе операции безопасны при повторе.
# ⚠️ Регистрация — через `if … then … fi`, а НЕ `[ … ] && insert_row`: под
# `set -e` последняя форма при REGISTER=0 возвращает 1 и обрывает do_stage
# (ломало дефолтный путь без --register; поймано регресс-тестом 0094).
do_stage() {
  case "$1" in
    feature)
      render feature.md "$ROOT/docs/features/$NUM-$SLUG.md" 0
      if [ "$REGISTER" = 1 ]; then
        insert_row "$ROOT/docs/features/README.md" "| [$NUM]" \
          "| [$NUM](./$NUM-$SLUG.md) | $TITLE_ESC | [ADR](../adr/$NUM-$SLUG.md) · [анализ](../analyze/$NUM-$SLUG.md) · [тест-план](../tests/README.md) · [отчёт](../reports/README.md) | СОЗДАНА |"
      fi
      ;;
    adr)
      render adr.md "$ROOT/docs/adr/$NUM-$SLUG.md" 0
      # Статус заготовки — Draft (фича 0094): Accepted проставляется по факту
      # принятия решения на стадии 2, а не при создании заготовки.
      if [ "$REGISTER" = 1 ]; then
        insert_row "$ROOT/docs/adr/README.md" "| [$NUM]" \
          "| [$NUM](./$NUM-$SLUG.md) | $TITLE_ESC | Draft | фича $NUM |"
      fi
      ;;
    analyze)
      render analyze.md "$ROOT/docs/analyze/$NUM-$SLUG.md" 0
      if [ "$REGISTER" = 1 ]; then
        insert_row "$ROOT/docs/analyze/README.md" "| $NUM |" \
          "| $NUM | $TITLE_ESC | [$NUM-$SLUG.md]($NUM-$SLUG.md) | — (новая фича) |"
      fi
      ;;
    dev)
      render development.md "$ROOT/docs/development/$NUM-$SUBTASK-$SLUG.md" 1
      if [ "$REGISTER" = 1 ]; then
        insert_row "$ROOT/docs/development/README.md" "| $NUM-$SUBTASK |" \
          "| $NUM-$SUBTASK | $NUM | $TITLE_ESC | [$NUM-$SUBTASK-$SLUG.md]($NUM-$SUBTASK-$SLUG.md) |"
      fi
      ;;
    tests)
      render tests.md "$ROOT/docs/tests/$NUM-$SLUG.md" 0
      if [ "$REGISTER" = 1 ]; then
        insert_row "$ROOT/docs/tests/README.md" "| $NUM |" \
          "| $NUM | $TITLE_ESC | [$NUM-$SLUG.md]($NUM-$SLUG.md) | СОЗДАНА |"
      fi
      ;;
    report)
      render reports.md "$ROOT/docs/reports/$NUM-$SLUG.md" 0
      if [ "$REGISTER" = 1 ]; then
        insert_row "$ROOT/docs/reports/README.md" "| $NUM |" \
          "| $NUM | $TITLE_ESC | [$NUM-$SLUG.md]($NUM-$SLUG.md) | СОЗДАНА |"
      fi
      ;;
  esac
}

if [ -n "$STAGE" ]; then
  # Режим добора одной стадии.
  do_stage "$STAGE"
else
  # Дефолтный режим заведения фичи (обратно совместим): feature/adr/analyze
  # всегда, dev/tests — при --with-dev.
  do_stage feature
  do_stage adr
  do_stage analyze
  if [ "$WITH_DEV" = "1" ]; then
    do_stage dev
    do_stage tests
  fi
fi

echo "Готово. Не забудьте: заполнить заготовки, обновить статус в FEATURES.md и внести запись в CHANGES.md."
