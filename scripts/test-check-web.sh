#!/usr/bin/env bash
# Сторож гейта веб-части (правило 0315, фича 0531).
#
# Мутациями доказывается, что `check-web.sh` ловит то, ради чего заведён:
#
#   B1 — разметка ссылается на файл, которого сборка не кладёт;
#   B2 — скрипт страницы не разбирается (сломанный синтаксис);
#   B3 — в `web/` появился список ключевых слов Takt;
#   B4 — согласованное дерево принимается (иначе гейт красен всегда);
#   B5 — без `node` мягкий пропуск, под `PRECHECK_STRICT=1` — ошибка;
#   B6 — роли подсветки кода разошлись с темой документа `book/takt.tmTheme`.
#
# ⚠️ Мутации ставятся на КОПИИ дерева (`WEB_ROOT`), рабочие файлы не трогаются:
# сторож, который правит проект, однажды оставит правку после падения.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "Сторож гейта веб-части (фича 0531)..."

NODE="${TAKT_NODE:-node}"
if ! command -v "$NODE" >/dev/null 2>&1; then
  if [[ "${PRECHECK_STRICT:-0}" == "1" ]]; then
    echo "  ОШИБКА: не найден node (PRECHECK_STRICT=1)"
    exit 1
  fi
  echo "  пропуск: не найден node"
  exit 0
fi

BIN_DIR="$("$(dirname "${BASH_SOURCE[0]}")/target-dir.sh")"
TARGET_DIR="$(dirname "$BIN_DIR")"
PROFILE="${TAKT_WASM_PROFILE:-wasm}"
WASM="$TARGET_DIR/wasm32-unknown-unknown/$PROFILE/takt_wasm.wasm"
if [[ ! -f "$WASM" ]]; then
  if [[ "${PRECHECK_STRICT:-0}" == "1" ]]; then
    echo "  ОШИБКА: модуль не собран (PRECHECK_STRICT=1)"
    exit 1
  fi
  echo "  пропуск: модуль не собран"
  exit 0
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# Копия дерева: только то, что гейту нужно.
TREE="$WORK/tree"
mkdir -p "$TREE/scripts" "$TREE/web" "$TREE/takt-lang" "$TREE/book" "$TREE/target/precheck/wasm32-unknown-unknown/$PROFILE"
cp -R "$ROOT/web/static" "$ROOT/web/tests" "$TREE/web/"
cp "$ROOT/scripts/check-web.sh" "$ROOT/scripts/build-web.sh" "$ROOT/scripts/target-dir.sh" "$TREE/scripts/"
cp "$ROOT/takt-lang/Cargo.toml" "$TREE/takt-lang/"
# Тема документа: по ней проверяется реестр ролей подсветки (задача 06).
# ⚠️ Копия дерева — не «весь проект»: файл, который тесты читают, а сторож не
# кладёт, роняет B4 с чужой причиной (нашлось первым же прогоном 2026-09-04).
cp "$ROOT/book/takt.tmTheme" "$TREE/book/"
cp "$WASM" "$TREE/target/precheck/wasm32-unknown-unknown/$PROFILE/"

run_gate() {  # запускает гейт на копии дерева
  ( cd "$TREE" && CARGO_TARGET_DIR="$TREE/target/precheck" bash scripts/check-web.sh 2>&1 )
}

# ── B4: согласованное дерево принимается ─────────────────────────────────────
if out="$(run_gate)"; then
  echo "  OK: B4 согласованное дерево принимается"
else
  echo "  ПРОВАЛ: B4 согласованное дерево отвергнуто:"
  echo "$out" | sed 's/^/    /'
  exit 1
fi

# ── B1: ссылка на несобранный файл ловится ───────────────────────────────────
cp "$TREE/web/static/index.html" "$WORK/index.html.bak"
sed -i.bak 's|<link rel="stylesheet" href="app.css">|<link rel="stylesheet" href="theme.css">|' \
  "$TREE/web/static/index.html"
if out="$(run_gate)"; then
  echo "  ПРОВАЛ: B1 ссылка на отсутствующий файл не поймана"
  exit 1
fi
if grep -q "theme.css" <<< "$out"; then
  echo "  OK: B1 ссылка на несобранный файл ловится и названа"
else
  echo "  ПРОВАЛ: B1 отказ не называет файл:"
  echo "$out" | sed 's/^/    /'
  exit 1
fi
cp "$WORK/index.html.bak" "$TREE/web/static/index.html"

# ── B2: неразбираемый скрипт ловится ─────────────────────────────────────────
cp "$TREE/web/static/app.js" "$WORK/app.js.bak"
printf '\nfunction( {\n' >> "$TREE/web/static/app.js"
if out="$(run_gate)"; then
  echo "  ПРОВАЛ: B2 сломанный скрипт не пойман"
  exit 1
fi
if grep -q "не разбирается" <<< "$out"; then
  echo "  OK: B2 неразбираемый скрипт ловится"
else
  echo "  ПРОВАЛ: B2 отказ не про разбор:"
  echo "$out" | sed 's/^/    /'
  exit 1
fi
cp "$WORK/app.js.bak" "$TREE/web/static/app.js"

# ── B3: список ключевых слов Takt ловится ────────────────────────────────────
# Самый дорогой класс: свой словарь красит текст правдоподобно и расходится с
# лексером молча — ровно то, чем платили параллельные списки LSP (0232).
cat > "$TREE/web/static/words.js" <<'EOF'
export const KEYWORDS = ["start", "state", "model", "invariant", "always"];
EOF
if out="$(run_gate)"; then
  echo "  ПРОВАЛ: B3 список ключевых слов Takt в web/ не пойман"
  exit 1
fi
if grep -q "ключевых слов" <<< "$out"; then
  echo "  OK: B3 список ключевых слов ловится"
else
  echo "  ПРОВАЛ: B3 отказ не про словарь:"
  echo "$out" | sed 's/^/    /'
  exit 1
fi
rm "$TREE/web/static/words.js"

# ── B6: роли подсветки разошлись с темой документа ───────────────────────────
# Реестр ролей — `book/takt.tmTheme` (замер задачи 06): блоки кода в PDF и
# вкладка цели красят одни и те же виды токенов. Роль, заведённая только с одной
# стороны, — это документ и редактор, разошедшиеся глазами; сличить их может
# только человек, и только положив две картинки рядом.
cp "$TREE/web/static/app.css" "$WORK/app.css.bak"
sed -i.bak 's|  --tok-comment: #6b7280;|  --tok-comment: #6b7280;\n  --tok-macro: #123456;|' \
  "$TREE/web/static/app.css"
if out="$(run_gate)"; then
  echo "  ПРОВАЛ: B6 роль вне темы документа не поймана"
  exit 1
fi
if grep -q "роли кода" <<< "$out"; then
  echo "  OK: B6 роль вне темы документа ловится"
else
  echo "  ПРОВАЛ: B6 отказ не про роли:"
  echo "$out" | sed 's/^/    /'
  exit 1
fi
cp "$WORK/app.css.bak" "$TREE/web/static/app.css"

# ── B5: политика внешнего инструмента ────────────────────────────────────────
if out="$(TAKT_NODE="$WORK/нет-такого-node" bash "$ROOT/scripts/check-web.sh" 2>&1)"; then
  if grep -q "пропуск" <<< "$out"; then
    echo "  OK: B5 без node — мягкий пропуск"
  else
    echo "  ПРОВАЛ: B5 без node гейт не сказал о пропуске:"
    echo "$out" | sed 's/^/    /'
    exit 1
  fi
else
  echo "  ПРОВАЛ: B5 без node гейт уронил прогон:"
  echo "$out" | sed 's/^/    /'
  exit 1
fi
if out="$(PRECHECK_STRICT=1 TAKT_NODE="$WORK/нет-такого-node" bash "$ROOT/scripts/check-web.sh" 2>&1)"; then
  echo "  ПРОВАЛ: B5 под PRECHECK_STRICT=1 отсутствие node принято"
  exit 1
fi
echo "  OK: B5 под PRECHECK_STRICT=1 отсутствие node — ошибка"

echo "  Сторож гейта веб-части: все проверки пройдены (B1…B6)."
