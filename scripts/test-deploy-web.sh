#!/usr/bin/env bash
# Сторож выкладки и скриптов стенда (фича 0531, задача 07c; правило 0315).
#
# Что доказывает:
#
#   E1  выкладка кладёт на стенд то, на что ссылается разметка;
#   E2  **подмена модуля под тем же адресом — отказ**, а не перезапись:
#       адрес `wasm/<версия>/` обещает неизменность, и молчаливая замена
#       портит страницу у каждого, кто уже кешировал;
#   E3  прежние бандлы копятся не бесконечно, а свежий не снимается никогда;
#   E4  скрипт стенда знает ровно четыре действия и **не удаляет тома**;
#   E5  нет `node` либо модуля — мягкий пропуск, под `PRECHECK_STRICT=1` ошибка.
#
# ⚠️ Настройка nginx судится ОТДЕЛЬНЫМ сторожем (`test-setup-nginx-takt.sh`):
# у неё другой предмет — совместная жизнь с соседним сервисом на одном стенде.
#
# ⚠️ Docker здесь НЕ запускается: предмет проверки — скрипты и раскладка, а
# подъём стенда проверяется человеком на стенде (в проекте нет исполняемого CI).

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STRICT="${PRECHECK_STRICT:-0}"
NODE="${TAKT_NODE:-node}"
BIN_DIR="$("$ROOT/scripts/target-dir.sh")"
TARGET_DIR="$(dirname "$BIN_DIR")"
PROFILE="${TAKT_WASM_PROFILE:-wasm}"
WASM="$TARGET_DIR/wasm32-unknown-unknown/$PROFILE/takt_wasm.wasm"

skip_or_fail() {
  if [[ "$STRICT" == "1" ]]; then
    echo "  ОШИБКА: $1 (PRECHECK_STRICT=1)"
    exit 1
  fi
  echo "  пропуск: $1"
  exit 0
}

echo "Сторож выкладки статики (фича 0531)..."

# ── E4 не требует сборки: проверяется текстом ────────────────────────────────
# ⚠️ Шаблона nginx здесь больше нет: он был вторым носителем правил прокси и
# уехал в генератор `setup-nginx-takt.sh` — его судит свой сторож
# (`test-setup-nginx-takt.sh`). Две копии одних правил разошлись бы молча.

STAND_SH="$ROOT/scripts/stand.sh"
for action in up down status restart; do
  grep -qE "^  $action\)" "$STAND_SH" || {
    echo "  ПРОВАЛ: E4 у стенда нет действия '$action'"
    exit 1
  }
done
# ⚠️ Ключ `-v` уносит тома — базу и чужие исходники. Такое делают руками.
# Строки комментариев отброшены: сам запрет описан словами и именно словом
# `down -v` — грепом по всему файлу сторож поймал бы собственное объяснение.
if grep -vE '^[[:space:]]*#' "$STAND_SH" | grep -qE 'down[^|]*-v( |$)'; then
  echo "  ПРОВАЛ: E4 скрипт стенда удаляет тома"
  exit 1
fi
grep -q '/health' "$STAND_SH" || {
  echo "  ПРОВАЛ: E4 состояние стенда не спрашивает /health"
  exit 1
}
echo "  OK: E4 стенд знает четыре действия, томов не трогает, спрашивает /health"

# ── Остальное требует собранного модуля ──────────────────────────────────────
command -v "$NODE" >/dev/null 2>&1 || skip_or_fail "не найден node (сборка статики его требует)"
[[ -f "$WASM" ]] || skip_or_fail "модуль не собран (см. check-wasm.sh)"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
STAND="$WORK/stand"

# ── E1: выкладка кладёт то, на что ссылается разметка ────────────────────────
"$ROOT/scripts/deploy-web.sh" -d "$STAND" >/dev/null || {
  echo "  ПРОВАЛ: E1 выкладка не отработала"
  exit 1
}
missing=0
while read -r asset; do
  [[ -z "$asset" ]] && continue
  case "$asset" in http*|"#"*|data:*|/) continue ;; esac
  [[ -f "$STAND/$asset" ]] || { echo "    нет '$asset'"; missing=1; }
done < <(sed 's/<base [^>]*>//g' "$STAND/index.html" \
         | grep -oE '(href|src)="[^"]+"' | sed 's/.*="//; s/"//')
[[ "$missing" == "0" ]] || { echo "  ПРОВАЛ: E1 на стенде нет того, на что ссылается разметка"; exit 1; }
[[ -f "$STAND/wasm/index.json" ]] || { echo "  ПРОВАЛ: E1 нет описи версий модуля"; exit 1; }
echo "  OK: E1 выкладка разложила статику целиком"

# ── E2: подмена модуля отвергается ───────────────────────────────────────────
VERSION="$(sed -n 's/.*"takt_lang": "\([^"]*\)".*/\1/p' "$STAND/version.json" | head -1)"
printf 'подмена' >> "$STAND/wasm/$VERSION/takt.wasm"
if "$ROOT/scripts/deploy-web.sh" -d "$STAND" --dry-run >/dev/null 2>&1; then
  echo "  ПРОВАЛ: E2 подмена модуля под тем же адресом прошла"
  exit 1
fi
echo "  OK: E2 подмена модуля под адресом с версией отвергнута"

# ── E3: прежние бандлы копятся не бесконечно ─────────────────────────────────
rm -rf "$STAND"
"$ROOT/scripts/deploy-web.sh" -d "$STAND" >/dev/null
CURRENT="$(sed -n 's/.*"bundle": "\([^"]*\)".*/\1/p' "$STAND/version.json" | head -1)"
# Подкладываем прежние бандлы — так же, как их оставили бы прошлые выкладки.
for old in aaaaaaaaaaaa bbbbbbbbbbbb cccccccccccc dddddddddddd; do
  mkdir -p "$STAND/b/$old"
  printf 'старое' > "$STAND/b/$old/app.js"
done
"$ROOT/scripts/deploy-web.sh" -d "$STAND" -k 2 >/dev/null
COUNT="$(find "$STAND/b" -mindepth 1 -maxdepth 1 -type d | wc -l | tr -d ' ')"
if [[ "$COUNT" -gt 3 ]]; then
  echo "  ПРОВАЛ: E3 бандлов осталось $COUNT при пределе 2"
  exit 1
fi
[[ -d "$STAND/b/$CURRENT" ]] || {
  echo "  ПРОВАЛ: E3 снят СВЕЖИЙ бандл — страница перестала бы открываться"
  exit 1
}
echo "  OK: E3 прежние бандлы снимаются, свежий остаётся"

# ── E6: политика пропуска ────────────────────────────────────────────────────
if TAKT_NODE=такого-нет "$0" 2>&1 | grep -q "пропуск"; then
  echo "  OK: E5 без node — мягкий пропуск"
else
  echo "  ПРОВАЛ: E5 без node пропуск не назван"
  exit 1
fi
if TAKT_NODE=такого-нет PRECHECK_STRICT=1 "$0" >/dev/null 2>&1; then
  echo "  ПРОВАЛ: E5 под PRECHECK_STRICT=1 отсутствие node прошло"
  exit 1
fi
echo "  OK: E5 под PRECHECK_STRICT=1 отсутствие node — ошибка"

# ── E6: стек РАЗБИРАЕТСЯ ─────────────────────────────────────────────────────
# ⚠️ Предмет — не «файл на месте», а «docker его читает». Значение с двоеточием
# и пробелом (адрес базы, текст отказа `:?`) без кавычек YAML читает как
# отображение, и стек не поднимается вовсе: «mapping values are not allowed in
# this context». Класс нашла выкладка на стенд 2026-09-05 — дома стек не
# поднимали ни разу, и гейты его не читали.
if docker compose version >/dev/null 2>&1; then
  if TAKT_WEB_JWT_SECRET=проба-сторожа docker compose -p takt-guard \
       --project-directory "$ROOT/web/deploy" \
       -f "$ROOT/web/deploy/docker-compose.yml" config -q >/dev/null 2>&1; then
    echo "  OK: E6 стек разбирается docker compose"
  else
    echo "  ПРОВАЛ: E6 стек не разбирается — на стенде он не поднимется:"
    TAKT_WEB_JWT_SECRET=проба-сторожа docker compose -p takt-guard \
      --project-directory "$ROOT/web/deploy" \
      -f "$ROOT/web/deploy/docker-compose.yml" config -q 2>&1 | head -3 | sed 's/^/    /'
    exit 1
  fi
else
  echo "  пропуск: E6 нет docker compose — разбор стека не проверен"
fi

# ── E7: образ собирает модуль ТЕМ ЖЕ профилем, что ищет сборка статики ───────
# ⚠️ `build-web.sh` ищет модуль по `TAKT_WASM_PROFILE` (умолчание `wasm`), а
# образ собирал его `--release` — и сборка падала на «Модуль не собран» уже НА
# СТЕНДЕ, после полутора сотен скомпилированных крейтов. Два носителя одного
# знания разошлись молча (выкладка 2026-09-05).
DOCKER_PROFILE="$(grep -oE 'cargo build --profile \$\{WASM_PROFILE\}|cargo build --release --target wasm32' "$ROOT/web/deploy/Dockerfile" | head -1)"
if [[ "$DOCKER_PROFILE" != 'cargo build --profile ${WASM_PROFILE}' ]]; then
  echo "  ПРОВАЛ: E7 образ собирает модуль не тем профилем, что ищет build-web.sh"
  echo "          (нашлось: '${DOCKER_PROFILE:-ничего}')"
  exit 1
fi
grep -q 'TAKT_WASM_PROFILE=\${WASM_PROFILE}' "$ROOT/web/deploy/Dockerfile" || {
  echo "  ПРОВАЛ: E7 профиль не передан скрипту сборки статики — он возьмёт своё умолчание"
  exit 1
}
echo "  OK: E7 профиль модуля назван один раз и доезжает до сборки статики"

echo "  Сторож выкладки: все проверки пройдены (E1…E7)."
