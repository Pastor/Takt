#!/bin/sh
# Гейт синхронизации версии языка (фича 0085) — единый источник истины в коде.
#
# Правило (ADR 0085): номер версии языка Takt живёт в коде ровно один раз —
# константа `takt_lang::version::LANGUAGE_VERSION` (`takt-lang/src/version.rs`).
# `README.md` и живой контекст `CLAUDE.md` обязаны ей соответствовать.
#
# ТРЕТИЙ ИСТОЧНИК — CLAUDE.md (фича 0149). Прежде гейт сверял два источника из
# трёх, и живой контекст показывал 0.4.0 при фактических 0.6.0; ошибка успела
# разойтись ПО ТРЁМ КАРТОЧКАМ закрытых фич через цитирование. Проверка версии
# держится ЗДЕСЬ, а не в scripts/check-claude-md.py: два гейта, проверяющие один
# предмет, неизбежно разъезжаются (прецеденты — CI-гейты 0090, формат позиции
# 0028-01). Прежде версия жила ТОЛЬКО в README, и
# рассинхрон дока↔факт ничем не ловился: фича 0078 подняла язык 0.3.0 → 0.4.0, а
# README остался на 0.3.0 — незамеченно. README без гейта повторяет судьбу
# доков, отстающих от кода (прецедент ADR 0027 — размер модуля, ADR 0077 —
# реестр кодов): правило, которое нельзя проверить командой, — не правило.
#
# Три условия отказа:
#   1. Расхождение — константа и каноническая строка README не совпадают.
#   2. Нет якоря — в README нет строки `**Версия языка: X.Y.Z**`.
#   3. Дубль якоря — таких строк в README больше одной (якорь неоднозначен).
# Исторические упоминания версий в README (`0.2.0 → 0.3.0`, «версия 0.3.0
# накапливает», ссылки на фичи) имеют иную форму и под якорь НЕ подходят —
# гейт их не трогает по замыслу (узкий якорь).
#
# POSIX sh, без внешних зависимостей (образец — scripts/check-diagnostic-codes.sh).
set -eu

# Корень переопределяется переменной (фича 0315) — для сторожа гейта.
ROOT="${LV_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
VERSION_RS="$ROOT/takt-lang/src/version.rs"
README="$ROOT/README.md"
CLAUDE_MD="$ROOT/CLAUDE.md"
VER_RE='[0-9]+\.[0-9]+\.[0-9]+'

if [ ! -f "$VERSION_RS" ]; then
    echo "check-language-version: не найден источник константы $VERSION_RS" >&2
    exit 1
fi
if [ ! -f "$README" ]; then
    echo "check-language-version: не найден $README" >&2
    exit 1
fi
if [ ! -f "$CLAUDE_MD" ]; then
    echo "check-language-version: не найден $CLAUDE_MD" >&2
    exit 1
fi

echo "Гейт версии языка: LANGUAGE_VERSION ↔ README ↔ CLAUDE.md (фичи 0085, 0149)..."

# Значение константы: `pub const LANGUAGE_VERSION: &str = "X.Y.Z";`
CONST_VER="$(grep -oE "LANGUAGE_VERSION: &str = \"$VER_RE\"" "$VERSION_RS" \
    | grep -oE "$VER_RE" | head -n1)"
if [ -z "${CONST_VER:-}" ]; then
    echo "  ОШИБКА: не удалось извлечь LANGUAGE_VERSION из $VERSION_RS" >&2
    echo "  Ожидается объявление вида: pub const LANGUAGE_VERSION: &str = \"X.Y.Z\";" >&2
    exit 1
fi

# Каноническая строка README: `**Версия языка: X.Y.Z**` — ровно одна.
ANCHORS="$(grep -cE "\*\*Версия языка: $VER_RE\*\*" "$README" || true)"
if [ "$ANCHORS" -eq 0 ]; then
    echo "  ОШИБКА: в $README нет канонической строки '**Версия языка: X.Y.Z**'." >&2
    echo "  Добавьте её в раздел версий (значение = $CONST_VER)." >&2
    exit 1
fi
if [ "$ANCHORS" -gt 1 ]; then
    echo "  ОШИБКА: в $README найдено $ANCHORS канонических строк версии — якорь неоднозначен." >&2
    echo "  Каноническая строка '**Версия языка: X.Y.Z**' должна быть ровно одна." >&2
    exit 1
fi

README_VER="$(grep -oE "\*\*Версия языка: $VER_RE\*\*" "$README" \
    | grep -oE "$VER_RE" | head -n1)"

if [ "$CONST_VER" != "$README_VER" ]; then
    echo "  ОШИБКА: версия языка рассинхронизирована." >&2
    echo "    LANGUAGE_VERSION ($VERSION_RS): $CONST_VER" >&2
    echo "    README.md (**Версия языка: …**):  $README_VER" >&2
    echo "  Приведите README к константе (или наоборот, если менялась версия языка — правило 22)." >&2
    exit 1
fi

# Каноническая строка живого контекста: `**сейчас `X.Y.Z`**` рядом с упоминанием
# LANGUAGE_VERSION — ровно одна. Якорь узкий по той же причине, что и у README:
# исторические упоминания версий («0.2.0 → 0.3.0», «версия языка 0.1.0» в записях
# о закрытых фичах) под него не подходят и гейтом не трогаются.
CLAUDE_ANCHORS="$(grep -cE '\*\*сейчас `'"$VER_RE"'`\*\*' "$CLAUDE_MD" || true)"
if [ "$CLAUDE_ANCHORS" -eq 0 ]; then
    echo "  ОШИБКА: в $CLAUDE_MD нет канонической строки '**сейчас \`X.Y.Z\`**'." >&2
    echo "  Она обязана стоять при упоминании LANGUAGE_VERSION (значение = $CONST_VER)." >&2
    exit 1
fi
if [ "$CLAUDE_ANCHORS" -gt 1 ]; then
    echo "  ОШИБКА: в $CLAUDE_MD найдено $CLAUDE_ANCHORS канонических строк версии — якорь неоднозначен." >&2
    exit 1
fi

CLAUDE_VER="$(grep -oE '\*\*сейчас `'"$VER_RE"'`\*\*' "$CLAUDE_MD" \
    | grep -oE "$VER_RE" | head -n1)"

if [ "$CONST_VER" != "$CLAUDE_VER" ]; then
    echo "  ОШИБКА: версия языка в живом контексте рассинхронизирована." >&2
    echo "    LANGUAGE_VERSION ($VERSION_RS): $CONST_VER" >&2
    echo "    CLAUDE.md (**сейчас \`…\`**):       $CLAUDE_VER" >&2
    echo "  Живой контекст читается в начале каждой сессии — ложный факт здесь" >&2
    echo "  становится предпосылкой всей работы (фича 0149)." >&2
    exit 1
fi

echo "  OK: версия языка = $CONST_VER (константа, README и CLAUDE.md согласованы)"
