#!/bin/sh
# Регенерация SVG-диаграмм верификации раздела «Практический пример» (фикс 0124-01).
#
# Конвейер: taktc verify --emit-graph → dot -Tsvg → svg_flatten_text.py.
# Последний шаг переводит подписи в векторные контуры наклонным ГОСТ тип А, так что
# итоговый SVG самодостаточен (шрифт при ПРОСМОТРЕ/сборке PDF не нужен). Шрифт нужен
# только здесь, на генерации — ставит `make -C book fonts`.
#
# Использование: book/scripts/render_verify_graphs.sh
set -eu

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TAKTC="${TAKTC:-$ROOT/target/debug/taktc}"
MODEL="$ROOT/book/src/17-showcase/examples/lift.takt"
IMG="$ROOT/book/src/17-showcase/images"
FLATTEN="$ROOT/book/scripts/svg_flatten_text.py"
PROP="F Boarding"

# Файл шрифта — из каталога пользовательских шрифтов (ставит `make fonts`).
FONT="${GOST_FONT:-}"
if [ -z "$FONT" ]; then
    for d in "$HOME/Library/Fonts" "$HOME/.local/share/fonts"; do
        [ -f "$d/GOST2.304-81TypeA-Slanted.ttf" ] && FONT="$d/GOST2.304-81TypeA-Slanted.ttf" && break
    done
fi
[ -n "$FONT" ] || { echo "Шрифт ГОСТ не найден — выполните: make -C book fonts" >&2; exit 1; }

render() { # <kind> <файл-без-расширения> [свойство]
    kind="$1"; out="$2"; prop="${3:-}"
    if [ -n "$prop" ]; then
        "$TAKTC" verify --emit-graph "$kind" -p "$prop" "$MODEL" > "$IMG/$out.dot"
    else
        "$TAKTC" verify --emit-graph "$kind" "$MODEL" > "$IMG/$out.dot"
    fi
    dot -Tsvg "$IMG/$out.dot" | python3 "$FLATTEN" "$FONT" > "$IMG/$out.svg"
    echo "  $IMG/$out.svg"
}

echo "Регенерация диаграмм верификации (наклонный ГОСТ, контуры):"
render kripke  lift-kripke
render buchi   lift-buchi   "$PROP"
render product lift-product "$PROP"
echo "Готово."
