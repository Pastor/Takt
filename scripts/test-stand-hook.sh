#!/usr/bin/env bash
# Проверка выкатки стенда по уведомлению (`scripts/stand-hook.py`).
#
# Стенд подменён: `origin` - голый репозиторий во временном каталоге, клон
# стенда - его клон, подъём стека - команда, которая записывает выкатываемый
# коммит и время. Проверяется то, что ломается молча:
#
# H1 без секрета приёмник не стартует;
# H2 негодная и отсутствующая подпись - 401, выкатки нет;
# H3 чужая ветка и чужое событие - 202 "пропущено", выкатки нет;
# H4 годное уведомление - 202 до конца выкатки, затем выкатка этого коммита;
# H5 повтор выкаченного коммита выкатки не запускает;
# H6 уведомления во время выкатки схлопываются в одну следующую;
# H7 статус отдаёт коммит и исход, без журнала;
# H8 опрос выкатывает только при расхождении с origin;
# H9 провал подъёма записывается исходом failure, следующая выкатка его чинит;
# H10 две ручные выкатки разом не пересекаются (замок);
# H11 тело больше предела - 413;
# H12 ручная выкатка `deploy-stand.sh` зовёт тот же носитель;
# H13 установка служб печатается без root: приёмник от пользователя клона,
#     секрет из файла окружения, опрос раз в 15 минут, секрета в выводе нет.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOOK="$ROOT/scripts/stand-hook.py"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "Проверка выкатки стенда по уведомлению..."

# H12 первой: она не требует стенда.
grep -q 'stand-hook.py' "$ROOT/scripts/deploy-stand.sh" \
  || { echo "  ПРОВАЛ: H12 deploy-stand.sh не зовёт stand-hook.py"; exit 1; }
if grep -qE 'stand\.sh up' "$ROOT/scripts/deploy-stand.sh"; then
  echo "  ПРОВАЛ: H12 deploy-stand.sh поднимает стек сам, мимо носителя"
  exit 1
fi
echo "  + H12 ручная выкатка зовёт общий носитель"

PRINTED="$(STAND_USER=deploy STAND_DIR=/srv/takt "$ROOT/scripts/setup-stand-hook.sh" --print)"
for needle in "User=deploy" "EnvironmentFile=/etc/takt/hook.env" \
              "ExecStart=/usr/bin/env python3 /srv/takt/scripts/stand-hook.py serve" \
              "ExecStart=/usr/bin/env python3 /srv/takt/scripts/stand-hook.py poll" \
              "OnUnitActiveSec=15min" "Environment=TAKT_HOOK_LISTEN=127.0.0.1:8739" \
              "Environment=TAKT_HOOK_BRANCH=v2"; do
  grep -qF "$needle" <<<"$PRINTED" || { echo "  ПРОВАЛ: H13 нет '$needle'"; exit 1; }
done
if grep -qE 'TAKT_HOOK_SECRET=[0-9a-f]{16,}' <<<"$PRINTED"; then
  echo "  ПРОВАЛ: H13 печать несёт секрет"
  exit 1
fi
if LISTEN=0.0.0.0:8739 "$ROOT/scripts/setup-stand-hook.sh" --print >/dev/null 2>&1; then
  echo "  ПРОВАЛ: H13 приёмник на внешнем адресе принят"
  exit 1
fi
echo "  + H13 установка служб печатается, приёмник только на петле"

run_cases() {
  local hook="$1" work
  work="$(mktemp -d "$WORK/cases.XXXXXX")"
  HOOK="$hook" WORK="$work" python3 "$ROOT/scripts/test-stand-hook-cases.py"
}

run_cases "$HOOK"

# Контроль H2: приёмник, не сверяющий подпись, обязан проверку провалить.
MUTANT="$WORK/mutant.py"
sed 's/if not signature_ok(secret, body, self.headers.get("X-Hub-Signature-256")):/if False:/' "$HOOK" > "$MUTANT"
if cmp -s "$HOOK" "$MUTANT"; then
  echo "  ПРОВАЛ: контроль не нашёл места сверки подписи"
  exit 1
fi
if run_cases "$MUTANT" >/dev/null 2>&1; then
  echo "  ПРОВАЛ: контроль - приёмник без сверки подписи проверку прошёл"
  exit 1
fi
echo "  + контроль: приёмник без сверки подписи отвергнут"
echo "Выкатка стенда по уведомлению: все проверки пройдены"
