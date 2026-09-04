#!/usr/bin/env bash
# Сборка статики онлайн-редактора (фича 0531, задача 03).
#
# Собирает `web/dist`: страница и её скрипты из `web/static` плюс модуль
# WebAssembly. Раскладка взята у референса (замер 2026-09-04): исходники
# читаемы и правятся, `dist` — производный каталог, в дерево не коммитится.
#
# ⚠️ Бандлера здесь нет (решение заказчика 2026-09-04): страница написана
# модулями браузера и грузится как есть. Значит и минификации нет — цена
# принята ради того, чтобы предкоммит не требовал ни сети, ни `npm`.
#
# ⚠️ Модуль кладётся ДВАЖДЫ: `wasm/<версия>/takt.wasm` — адрес, который несёт
# версию (решение A5: публикация открывается своим модулем), и `takt.wasm`
# рядом со страницей — тот, которым открывается новый документ. Раскладку
# версий по-настоящему ведёт скрипт выкладки (задача 05).

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BIN_DIR="$("$(dirname "${BASH_SOURCE[0]}")/target-dir.sh")"
TARGET_DIR="$(dirname "$BIN_DIR")"
PROFILE="${TAKT_WASM_PROFILE:-wasm}"
WASM="$TARGET_DIR/wasm32-unknown-unknown/$PROFILE/takt_wasm.wasm"
DIST="${TAKT_WEB_DIST:-$ROOT/web/dist}"

if [[ ! -f "$WASM" ]]; then
  echo "Модуль не собран: $WASM"
  echo "Соберите: cargo build -p takt-wasm --profile $PROFILE --target wasm32-unknown-unknown"
  exit 1
fi

# Версия крейта модуля — из его манифеста: второй записи версии в проекте быть
# не должно (класс 0084).
VERSION="$(awk -F'"' '/^version = /{print $2; exit}' "$ROOT/takt-lang/Cargo.toml")"

mkdir -p "$DIST/wasm/$VERSION" "$DIST/i18n"
cp "$ROOT"/web/static/*.html "$ROOT"/web/static/*.css "$ROOT"/web/static/*.js \
   "$ROOT"/web/static/*.svg "$DIST/"
# Словари оболочки (задача 10a). Хеша в имени пока нет — раскладку кеша ведёт
# задача 07b; здесь важно лишь то, что словарь ЛЕЖИТ рядом со страницей: без
# него подписи выродятся в ключи, и заметит это только глаз.
cp "$ROOT"/web/static/i18n/*.json "$DIST/i18n/"
cp "$WASM" "$DIST/takt.wasm"
cp "$WASM" "$DIST/wasm/$VERSION/takt.wasm"

# Опись версии: по ней страница узнаёт, какой модуль последний, а выкладка —
# не подменяют ли уже выложенный (задача 07b).
cat > "$DIST/version.json" <<JSON
{
  "takt_lang": "$VERSION",
  "wasm": "wasm/$VERSION/takt.wasm",
  "sha256": "$(shasum -a 256 "$WASM" | awk '{print $1}')",
  "built_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
JSON

size_kib=$(( $(wc -c < "$DIST/takt.wasm") / 1024 ))
files=$(find "$DIST" -type f | wc -l | tr -d ' ')
echo "  статика собрана: $DIST ($files файлов, модуль ${size_kib} КиБ, версия $VERSION)"
