#!/bin/sh
# Гейт старых имён языка и инструментов (фича 0161).
#
# Правило (ADR 0161): в РАБОЧИХ файлах дерева не встречаются имена, упразднённые
# переименованиями `BuT` → `Lam` → `Takt` (фича 0100): сам язык (`BuT`, `Lam`),
# инструменты (`butc`, `lamc`, `lam-lsp`), расширения исходников (`.but`, `.lam`)
# и пути упразднённых крейтов (`grammar/…`, `simulation/…`, `-p simulation`).
#
# Повод — не косметика. Замер 2026-08-18 нашёл 36 таких мест, и одно из них было
# дефектом документации: `examples/graphics-configs/README.md` предлагал команду
# `cargo run -p simulation --bin simulation -- model.but --gif-config
# examples/gif-configs/dark.json`, у которой неверны ВСЕ ЧЕТЫРЕ элемента — крейт,
# бинарник, расширение и путь к пресету. Остальные 35 были комментариями, но
# отличить одно от другого можно только прочитав каждое: без гейта старое имя
# возвращается первым же скопированным комментарием.
#
# ⚠️ ГРАНИЦА ГЕЙТА: имена в ПОРОЖДАЕМОМ коде он НЕ проверяет. Хелперы
# `lam_q_floordiv`, `lam_q_mul`, `lam_q_wrap`, `lam_q_sat` (цель `c`),
# `LAM_Q_FLOORDIV`, `LAM_Q_WRAP`, `LAM_Q_SAT`, `LAM_Q_MUL`, `LAM_Q_DIV`
# (цель `st`) и служебные `lam_generated`/`lam_file` симулятора остаются: это
# НАБЛЮДАЕМЫЙ ВЫВОД инструмента — имена уезжают в прошивку пользователя, их
# видят снапшоты `examples/generated/`, харнессы и потактовые сверки. Их смена —
# ломающее изменение контракта вывода со своим жизненным циклом (кандидат в
# `FEATURES.md`), а не правка комментария. Шаблонам ниже они намеренно не
# отвечают (`LAM` ≠ `Lam`, `lam_q` ≠ `Lam`). По той же причине не проверяется
# идентификатор `BUT_KEYWORDS` (`takt-lang/src/lsp/keywords.rs`).
#
# ⚠️ ИСКЛЮЧЕНИЯ ЗАДАНЫ ПРЕФИКСАМИ ПУТЕЙ, а не пофайловым списком. В `docs/` и
# журналах старое имя — СВИДЕТЕЛЬСТВО (правило 21), а не ошибка, и цитировать
# его там будут всегда. Префикс не устаревает, поэтому ратчет исключений (как у
# `scripts/module-size-baseline.txt`) здесь не нужен: устареть нечему.
#
# Использование:
#   scripts/check-legacy-names.sh            # весь индекс git (шаг precheck.sh)
#   scripts/check-legacy-names.sh ФАЙЛ …     # только указанные (сторож 0161)
#
# POSIX sh, без внешних зависимостей (образец — scripts/check-repo-url.sh).
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# --- Что запрещено ----------------------------------------------------------
# ERE. Границы слова записаны классами POSIX `[[:<:]]`/`[[:>:]]`? НЕТ: они есть у
# BSD grep и отсутствуют у GNU. Переносимо и достаточно — обрамление символами,
# не входящими в идентификатор, через отрицаемые классы.
LEGACY='(^|[^A-Za-z0-9_])(BuT|Lam|butc|lamc|lam-lsp|lam-sim)($|[^A-Za-z0-9_-])|\.(but|lam)($|[^A-Za-z0-9_])|(^|[^A-Za-z0-9_/])(grammar|simulation)/(src|tests)/|-p +(grammar|simulation)( |$)'

# --- Где не проверяем -------------------------------------------------------
# docs/, CHANGES.md — история фичи 0100 и всех, кто её цитирует (правило 21).
# CLAUDE.md, AGENTS.md — ссылка на карточку `0081-lamc-print-warnings.md`.
# FEATURES.md — адрес репозитория `github.com/Pastor/BuT` (действующий, ADR 0179).
# .claude/ — артефакты сессий инструмента, не исходники проекта.
# Два гейта ниже описывают САМ переезд и обязаны называть старое имя.
is_excluded() {
    case "$1" in
        docs/*|CHANGES.md|CLAUDE.md|AGENTS.md|FEATURES.md|.claude/*) return 0 ;;
        scripts/check-repo-url.sh|scripts/check-legacy-names.sh) return 0 ;;
        scripts/test-legacy-names.sh|scripts/precheck.sh) return 0 ;;
        *) return 1 ;;
    esac
}

echo "Гейт старых имён (BuT/Lam → Takt, фича 0161)..."

if [ $# -gt 0 ]; then
    FILES="$*"
else
    FILES="$(cd "$ROOT" && git ls-files)"
fi

BAD=0
COUNT=0
for REL in $FILES; do
    is_excluded "$REL" && continue
    F="$ROOT/$REL"
    [ -f "$F" ] || F="$REL"
    [ -f "$F" ] || continue
    COUNT=$((COUNT + 1))
    # -I: двоичные файлы пропускаются (шрифты, снимки).
    HITS="$(grep -nIE "$LEGACY" "$F" 2>/dev/null || true)"
    [ -z "$HITS" ] && continue
    # Падаем СПИСКОМ: одно место за прогон означало бы столько прогонов, сколько
    # мест, — а их бывает 36.
    echo "$HITS" | while IFS= read -r LINE; do
        echo "  ОШИБКА: $REL:$LINE" >&2
    done
    BAD=1
done

if [ "$BAD" -ne 0 ]; then
    echo "  Старое имя языка/инструмента в рабочем файле (ADR 0161)." >&2
    echo "  Замены: BuT|Lam → Takt, butc|lamc → taktc, lam-lsp → takt-lsp," >&2
    echo "          .but|.lam → .takt, grammar/ → takt-lang/, simulation/ → takt-sim/." >&2
    echo "  Историю цитировать в docs/ — там гейт не смотрит." >&2
    exit 1
fi

echo "  OK: старых имён нет (проверено файлов: $COUNT)"
