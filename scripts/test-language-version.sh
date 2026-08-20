#!/bin/sh
# Сторож гейта версии языка (фича 0315).
#
# `check-language-version.sh` сверяет три источника — константу
# `LANGUAGE_VERSION`, README и живой контекст (фичи 0085, 0149). Проверялось
# это ничем: гейт, который никогда не падал, неотличим от гейта, который не
# смотрит (урок 0255).
#
# Работает на КОПИИ дерева (`LV_ROOT`).
#
# POSIX sh.
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TOOL="$ROOT/scripts/check-language-version.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

FAILED=0
ok()   { echo "  OK: $1"; }
fail() { echo "  ПРОВАЛ: $1" >&2; FAILED=1; }

echo "Сторож гейта версии языка (фича 0315)..."

# Синтетическое дерево: три файла, которые читает гейт.
make_tree() {
    version="$1"; readme="$2"; claude="$3"
    rm -rf "$TMP/tree"
    mkdir -p "$TMP/tree/takt-lang/src"
    printf 'pub const LANGUAGE_VERSION: &str = "%s";\n' "$version" \
        > "$TMP/tree/takt-lang/src/version.rs"
    printf '**Версия языка: %s**\n' "$readme" > "$TMP/tree/README.md"
    # Якорь живого контекста — «**сейчас `X.Y.Z`**» рядом с упоминанием
    # константы (гейт 0149 требует именно его).
    printf 'LANGUAGE_VERSION, **сейчас `%s`**\n' "$claude" > "$TMP/tree/CLAUDE.md"
}

# --- 1. Согласованные источники принимаются -----------------------------------
make_tree "1.2.3" "1.2.3" "1.2.3"
if LV_ROOT="$TMP/tree" sh "$TOOL" >"$TMP/out" 2>&1; then
    ok "согласованные источники принимаются"
else
    fail "согласованные источники отвергнуты: $(cat "$TMP/out")"
fi

# --- 2. Отставший README ловится ----------------------------------------------
make_tree "1.2.3" "1.2.2" "1.2.3"
if ! LV_ROOT="$TMP/tree" sh "$TOOL" >"$TMP/out" 2>&1; then
    ok "отставший README ловится"
else
    fail "отставший README НЕ пойман: $(cat "$TMP/out")"
fi

# --- 3. Отставший живой контекст ловится --------------------------------------
make_tree "1.2.3" "1.2.3" "0.9.0"
if ! LV_ROOT="$TMP/tree" sh "$TOOL" >"$TMP/out" 2>&1; then
    ok "отставший живой контекст ловится"
else
    fail "отставший CLAUDE.md НЕ пойман: $(cat "$TMP/out")"
fi

# --- 4. Отсутствие якоря в README — отказ, а не молчание -----------------------
# ⚠️ Гейт без якоря сверял бы пустоту с пустотой и рапортовал об успехе (класс
# фикса 0202-01).
make_tree "1.2.3" "1.2.3" "1.2.3"
printf 'без якоря\n' > "$TMP/tree/README.md"
if ! LV_ROOT="$TMP/tree" sh "$TOOL" >"$TMP/out" 2>&1; then
    ok "отсутствие якоря ловится"
else
    fail "README без якоря принят: $(cat "$TMP/out")"
fi

# --- 5. Двойной якорь ловится --------------------------------------------------
make_tree "1.2.3" "1.2.3" "1.2.3"
printf '**Версия языка: 1.2.3**\n**Версия языка: 1.2.3**\n' > "$TMP/tree/README.md"
if ! LV_ROOT="$TMP/tree" sh "$TOOL" >"$TMP/out" 2>&1; then
    ok "неоднозначный якорь ловится"
else
    fail "двойной якорь принят: $(cat "$TMP/out")"
fi

[ "$FAILED" -eq 0 ] || { echo "  Сторож гейта версии языка: ПРОВАЛ" >&2; exit 1; }
echo "  Сторож гейта версии языка: все проверки пройдены"
