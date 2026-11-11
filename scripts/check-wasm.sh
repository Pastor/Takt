#!/usr/bin/env bash
# Проверка модуля WebAssembly: сборка под `wasm32-unknown-unknown` и
# сверка вывода модуля с инструментами (`check-wasm-identity.mjs`).
#
# Что доказывает: то, что показывает браузер, равно тому, что печатают `taktc`
# и `takt-sim`. Одной сборки мало - модуль собирается и тогда, когда мост теряет
# хвост файла или строку трассы: такой вывод остаётся валидным и оказывается
# Другим.
#
# Политика внешних инструментов - как у ST-арбитра: нет `node` или
# не установлен таргет - мягкий пропуск; под `PRECHECK_STRICT=1` это ошибка.
# Причина: предкоммит обязан идти на машине без веб-оснастки, а CI обязан
# требовать полноты.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

STRICT="${PRECHECK_STRICT:-0}"
# `target-dir.sh` печатает каталог отладочных бинарников (`.../debug`), а модуль
# лежит рядом - в каталоге своего профиля под таргетом. Корень берётся отсечением
# последнего сегмента: знание о каталоге сборки остаётся у одного носителя
# а профиль знает этот проверка.
BIN_DIR="$("$(dirname "${BASH_SOURCE[0]}")/target-dir.sh")"
TARGET_DIR="$(dirname "$BIN_DIR")"
# Профиль модуля: свой, а не общий `release` (см. `[profile.wasm]` корневого
# Cargo.toml) - браузеру важен размер, а `taktc` собирается для машины.
PROFILE="${TAKT_WASM_PROFILE:-wasm}"
WASM="$TARGET_DIR/wasm32-unknown-unknown/$PROFILE/takt_wasm.wasm"
EXPORT_WASM="$TARGET_DIR/wasm32-unknown-unknown/$PROFILE/takt_wasm_export.wasm"
# Предел веса ядра, КиБ. Ядро грузится при каждом открытии страницы, и всё, что
# нужно одной кнопке, живёт в модуле экспорта; ядро, переросшее предел, значит,
# что в него вернулось чужое - растеризатор, шрифты, кодировщик.
CORE_LIMIT_KIB="${TAKT_WASM_CORE_LIMIT_KIB:-3700}"

skip_or_fail() {  # $1 = причина
  if [[ "$STRICT" == "1" ]]; then
    echo "  ОШИБКА: $1 (PRECHECK_STRICT=1)"
    exit 1
  fi
  echo "  пропуск: $1"
  exit 0
}

echo "Гейт модуля WebAssembly (фича 0531)..."

# Имя интерпретатора - переменная: так тест проверки может проверить политику
# "нет инструмента" (мягкий пропуск против ошибки под PRECHECK_STRICT), не
# вычищая PATH целиком, - иначе вместе с `node` исчезли бы `bash` и `cargo`.
NODE="${TAKT_NODE:-node}"
command -v "$NODE" >/dev/null 2>&1 || skip_or_fail "не найден node"
rustup target list --installed 2>/dev/null | grep -qx "wasm32-unknown-unknown" \
  || skip_or_fail "не установлен таргет wasm32-unknown-unknown (rustup target add wasm32-unknown-unknown)"

CARGO_CMD="${CARGO_CMD:-cargo}"
# Ядро и модуль экспорта - отдельными вызовами: общий вызов объединил бы фичи, и
# ядро получило бы графику эталона.
mkdir -p -- "$TARGET_DIR"
BUILD_TARGET="$(mktemp -d "$TARGET_DIR/.wasm-build.XXXXXX")"
BUILD_TARGET="$(cd "$BUILD_TARGET" && pwd -P)"
trap 'rm -rf -- "$BUILD_TARGET"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
CARGO_TARGET_DIR="$BUILD_TARGET" CARGO_INCREMENTAL=0 \
  $CARGO_CMD build -p takt-wasm --profile "$PROFILE" --target wasm32-unknown-unknown
CARGO_TARGET_DIR="$BUILD_TARGET" CARGO_INCREMENTAL=0 \
  $CARGO_CMD build -p takt-wasm-export --profile "$PROFILE" --target wasm32-unknown-unknown

BUILT_WASM="$BUILD_TARGET/wasm32-unknown-unknown/$PROFILE/takt_wasm.wasm"
BUILT_EXPORT_WASM="$BUILD_TARGET/wasm32-unknown-unknown/$PROFILE/takt_wasm_export.wasm"
[[ -f "$BUILT_WASM" ]] || {
  echo "  ОШИБКА: модуль не собран: $BUILT_WASM"
  exit 1
}
[[ -f "$BUILT_EXPORT_WASM" ]] || {
  echo "  ОШИБКА: модуль экспорта не собран: $BUILT_EXPORT_WASM"
  exit 1
}
size_kib=$(( $(wc -c < "$BUILT_WASM") / 1024 ))
export_kib=$(( $(wc -c < "$BUILT_EXPORT_WASM") / 1024 ))
echo "  модули собраны: ядро ${size_kib} КиБ, модуль экспорта ${export_kib} КиБ"
if (( size_kib > CORE_LIMIT_KIB )); then
  echo "  ОШИБКА: ядро ${size_kib} КиБ сверх предела ${CORE_LIMIT_KIB} КиБ — в ядро вернулось то,"
  echo "  что нужно только экспорту (растеризатор, шрифты, кодировщик)"
  exit 1
fi

TAKTC="${TAKTC:-$BIN_DIR/taktc}"
TAKT_SIM="${TAKT_SIM:-$BIN_DIR/takt-sim}"
for tool in "$TAKTC" "$TAKT_SIM"; do
  [[ -x "$tool" ]] || {
    echo "  ОШИБКА: не найден инструмент сверки: $tool"
    exit 1
  }
done

"$NODE" "$ROOT/scripts/check-wasm-identity.mjs" \
  "$BUILT_WASM" "$TAKTC" "$TAKT_SIM" "$BUILT_EXPORT_WASM"
mkdir -p -- "$(dirname "$WASM")"
mv -f -- "$BUILT_WASM" "$WASM"
mv -f -- "$BUILT_EXPORT_WASM" "$EXPORT_WASM"
