#!/bin/sh
# new-feature.sh — генератор заготовок артефактов фичи из docs/templates/
# (правило 17, фича 0015). Без внешних зависимостей: POSIX sh + sed + awk.
#
# Использование:
#   scripts/new-feature.sh [--with-dev] [--register] XXXX slug "Заголовок"
#
#   --with-dev   дополнительно создать заготовки development (XXXX-01) и tests
#   --register   дописать строки в реестры README соответствующих папок
#
# Примеры:
#   scripts/new-feature.sh 0032 my-feature "Моя фича"
#   scripts/new-feature.sh --with-dev --register 0032 my-feature "Моя фича"
set -eu

WITH_DEV=0
REGISTER=0
while [ $# -gt 0 ]; do
  case "$1" in
    --with-dev) WITH_DEV=1; shift ;;
    --register) REGISTER=1; shift ;;
    --) shift; break ;;
    -*) echo "Неизвестный флаг: $1" >&2; exit 2 ;;
    *) break ;;
  esac
done

if [ $# -ne 3 ]; then
  echo "Использование: $0 [--with-dev] [--register] XXXX slug \"Заголовок\"" >&2
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

# Корень репозитория = каталог этого скрипта /..
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
TPL="$ROOT/docs/templates"
DATE=$(date +%F)

[ -d "$TPL" ] || { echo "Не найден каталог шаблонов: $TPL" >&2; exit 1; }

# Экранирование заголовка для replacement-части sed (делимитер '|')
TITLE_ESC=$(printf '%s' "$TITLE" | sed -e 's/[\\&|]/\\\&/g')

# render <шаблон> <куда> <repl_YY:0|1>
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
        -e "s|YY|01|g" \
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

# insert_row <README> <строка>: вставить строку после последней строки-строки таблицы ('| ...')
insert_row() {
  readme="$1"; row="$2"
  [ -f "$readme" ] || { echo "Нет реестра: $readme (пропуск)" >&2; return 0; }
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

# --- Заготовки стадий ---
render feature.md  "$ROOT/docs/features/$NUM-$SLUG.md" 0
render adr.md      "$ROOT/docs/adr/$NUM-$SLUG.md"      0
render analyze.md  "$ROOT/docs/analyze/$NUM-$SLUG.md"  0
if [ "$WITH_DEV" = "1" ]; then
  render development.md "$ROOT/docs/development/$NUM-01-$SLUG.md" 1
  render tests.md       "$ROOT/docs/tests/$NUM-$SLUG.md"         0
fi

# --- Реестры ---
if [ "$REGISTER" = "1" ]; then
  insert_row "$ROOT/docs/features/README.md" \
    "| [$NUM](./$NUM-$SLUG.md) | $TITLE_ESC | [ADR](../adr/$NUM-$SLUG.md) · [анализ](../analyze/$NUM-$SLUG.md) · [тест-план](../tests/README.md) · [отчёт](../reports/README.md) | СОЗДАНА |"
  insert_row "$ROOT/docs/adr/README.md" \
    "| [$NUM](./$NUM-$SLUG.md) | $TITLE_ESC | Accepted | фича $NUM |"
  insert_row "$ROOT/docs/analyze/README.md" \
    "| $NUM | $TITLE_ESC | [$NUM-$SLUG.md]($NUM-$SLUG.md) | — (новая фича) |"
  if [ "$WITH_DEV" = "1" ]; then
    insert_row "$ROOT/docs/development/README.md" \
      "| $NUM-01 | $NUM | $TITLE_ESC | [$NUM-01-$SLUG.md]($NUM-01-$SLUG.md) |"
    insert_row "$ROOT/docs/tests/README.md" \
      "| $NUM | $TITLE_ESC | [$NUM-$SLUG.md]($NUM-$SLUG.md) | СОЗДАНА |"
  fi
fi

echo "Готово. Не забудьте: заполнить заготовки, обновить статус в FEATURES.md и внести запись в CHANGES.md."
