#!/usr/bin/env bash
# Гейт веб-части (фича 0531, задача 03).
#
# Что доказывает:
#
#   1. страница СОБИРАЕТСЯ — `build-web.sh` кладёт всё, на что ссылается
#      разметка (пропавший файл иначе обнаружился бы только в браузере);
#   2. её код РАБОТАЕТ — тесты в `node`: круговой рейс ссылки, перевод
#      координат, черновик, форма ответов моста;
#   3. в `web/` НЕТ списка ключевых слов Takt — знание о языке живёт в лексере,
#      и вторая его копия разошлась бы молча (критерий 2 фичи, класс 0232);
#   4. каждый скрипт страницы РАЗБИРАЕТСЯ — `node --check` вместо браузера.
#
# Политика внешнего инструмента — как у ST-арбитра (0041): нет `node` — мягкий
# пропуск, под `PRECHECK_STRICT=1` — ошибка.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

STRICT="${PRECHECK_STRICT:-0}"
NODE="${TAKT_NODE:-node}"
BIN_DIR="$("$(dirname "${BASH_SOURCE[0]}")/target-dir.sh")"
TARGET_DIR="$(dirname "$BIN_DIR")"
PROFILE="${TAKT_WASM_PROFILE:-wasm}"
WASM="$TARGET_DIR/wasm32-unknown-unknown/$PROFILE/takt_wasm.wasm"

skip_or_fail() {  # $1 = причина
  if [[ "$STRICT" == "1" ]]; then
    echo "  ОШИБКА: $1 (PRECHECK_STRICT=1)"
    exit 1
  fi
  echo "  пропуск: $1"
  exit 0
}

echo "Гейт веб-части (фича 0531)..."

command -v "$NODE" >/dev/null 2>&1 || skip_or_fail "не найден node"
[[ -f "$WASM" ]] || skip_or_fail "модуль не собран (см. check-wasm.sh)"

# ── 1. Сборка статики ────────────────────────────────────────────────────────
DIST="$(mktemp -d)/dist"
TAKT_WEB_DIST="$DIST" "$ROOT/scripts/build-web.sh"

# Разметка ссылается только на то, что собрано: пропавший файл — белая страница
# в браузере и ни слова в консоли сборки.
missing=0
while read -r asset; do
  [[ -z "$asset" ]] && continue
  case "$asset" in
    http*|"#"*|data:*) continue ;;
  esac
  if [[ ! -f "$DIST/$asset" ]]; then
    echo "  ОШИБКА: разметка ссылается на '$asset', которого нет в собранной статике"
    missing=1
  fi
done < <(grep -oE '(href|src)="[^"]+"' "$DIST/index.html" | sed 's/.*="//; s/"//')
[[ "$missing" == "0" ]] || exit 1

# ── 2. Разбор каждого скрипта ────────────────────────────────────────────────
for script in "$ROOT"/web/static/*.js; do
  "$NODE" --check "$script" || {
    echo "  ОШИБКА: не разбирается $script"
    exit 1
  }
done

# ── 3. Списка ключевых слов Takt в вебе нет ──────────────────────────────────
# Признак — набор слов языка рядом друг с другом. Ищутся те, которые нигде,
# кроме словаря, вместе не встретятся: страница красит по ответу модуля и
# знать их не должна.
if grep -REn '"(start|state|model|invariant)"[[:space:]]*,[[:space:]]*"(start|state|model|invariant|always|enter|exit)"' \
     "$ROOT/web/static" "$ROOT/web/tests" >/dev/null 2>&1; then
  echo "  ОШИБКА: в web/ появился список ключевых слов Takt — знание о языке живёт в лексере"
  grep -REn '"(start|state|model|invariant)"[[:space:]]*,' "$ROOT/web/static" "$ROOT/web/tests" | head -5
  exit 1
fi

# ── 4. Проверки в node ───────────────────────────────────────────────────────
"$NODE" "$ROOT/web/tests/web-tests.mjs" "$WASM"

rm -rf "$(dirname "$DIST")"
