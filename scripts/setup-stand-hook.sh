#!/usr/bin/env bash
#
# Установка выкатки стенда по уведомлению GitHub и по опросу.
#
# Что кладётся:
#   /etc/takt/hook.env                        секрет и настройки, права 600
#   /etc/systemd/system/takt-hook.service     приёмник уведомлений
#   /etc/systemd/system/takt-hook-poll.service опрос ветки (одна сверка)
#   /etc/systemd/system/takt-hook-poll.timer  опрос раз в 15 минут
#
# Приёмник - служба systemd, а не контейнер стека: выкатка перезапускает стек, и
# слушатель внутри него умер бы посреди собственной работы. Служба идёт от
# пользователя клона: ему нужны `git` в клоне и `docker compose` (группа
# `docker`), а не права root.
#
# Секрет создаётся один раз и не перезаписывается: повторный прогон не должен
# молча разорвать связь с уже настроенным уведомлением GitHub. Секрет в вывод
# не печатается - его читают из файла.
#
# nginx: адреса приёмника (`<префикс>/hooks/github`, `<префикс>/hooks/status`)
# кладёт `scripts/setup-nginx-takt.sh` - прогоните его после этого скрипта.
#
# Запуск: sudo STAND_USER=deploy scripts/setup-stand-hook.sh
# Настройки: STAND_USER (владелец клона), STAND_DIR (клон, умолчание - корень
# дерева скрипта), BRANCH (v2), PREFIX (/takt), LISTEN (127.0.0.1:8739).
# Для теста: `--print` печатает всё, что положил бы, и ничего не трогает.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAND_USER="${STAND_USER:-${SUDO_USER:-$(id -un)}}"
STAND_DIR="${STAND_DIR:-$ROOT}"
BRANCH="${BRANCH:-v2}"
PREFIX="${PREFIX:-/takt}"
LISTEN="${LISTEN:-127.0.0.1:8739}"
ENV_FILE=/etc/takt/hook.env
UNITS=/etc/systemd/system

MODE=install
case "${1:-}" in
  --print) MODE=print ;;
  "") ;;
  *) echo "неизвестный ключ '$1'" >&2; exit 2 ;;
esac

die() { printf '[x] %s\n' "$*" >&2; exit 1; }

[[ "$PREFIX" =~ ^/[A-Za-z0-9_-]+$ ]] || die "префикс '$PREFIX' негоден: ожидается вида /takt"
[[ "$BRANCH" =~ ^[A-Za-z0-9._/-]+$ ]] || die "ветка '$BRANCH' негодна"
[[ "$LISTEN" =~ ^127\.0\.0\.1:[0-9]+$ ]] || die "приёмник слушает только петлю: '$LISTEN'"

state_dir() {
  local home
  home="$(getent passwd "$STAND_USER" 2>/dev/null | cut -d: -f6 || true)"
  echo "${home:-/home/$STAND_USER}/.local/state/takt-hook"
}

common_env() {
cat <<UNIT
User=${STAND_USER}
WorkingDirectory=${STAND_DIR}
EnvironmentFile=${ENV_FILE}
Environment=TAKT_HOOK_DIR=${STAND_DIR}
Environment=TAKT_HOOK_BRANCH=${BRANCH}
Environment=TAKT_HOOK_PREFIX=${PREFIX}
Environment=TAKT_HOOK_STATE=$(state_dir)
UNIT
}

service_text() {
cat <<UNIT
# Приёмник уведомлений GitHub стенда Takt.
# Файл создан scripts/setup-stand-hook.sh - правьте скрипт, не файл.
[Unit]
Description=Takt: выкатка стенда по уведомлению GitHub
After=network-online.target docker.service
Wants=network-online.target

[Service]
$(common_env)
Environment=TAKT_HOOK_LISTEN=${LISTEN}
ExecStart=/usr/bin/env python3 ${STAND_DIR}/scripts/stand-hook.py serve
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
UNIT
}

poll_service_text() {
cat <<UNIT
# Опрос ветки выкатки стенда Takt: одна сверка origin с выкаченным.
# Файл создан scripts/setup-stand-hook.sh - правьте скрипт, не файл.
[Unit]
Description=Takt: сверка ветки выкатки с выкаченной
After=network-online.target docker.service

[Service]
Type=oneshot
$(common_env)
ExecStart=/usr/bin/env python3 ${STAND_DIR}/scripts/stand-hook.py poll
UNIT
}

timer_text() {
cat <<UNIT
# Таймер опроса ветки выкатки стенда Takt.
# Файл создан scripts/setup-stand-hook.sh - правьте скрипт, не файл.
[Unit]
Description=Takt: опрос ветки выкатки раз в 15 минут

[Timer]
OnBootSec=5min
OnUnitActiveSec=15min
Persistent=true

[Install]
WantedBy=timers.target
UNIT
}

env_template() {
cat <<ENV
# Секрет уведомлений GitHub: тот же, что в Settings -> Webhooks репозитория.
# Файл создан scripts/setup-stand-hook.sh; повторный прогон его не трогает.
TAKT_HOOK_SECRET=<секрет>
ENV
}

if [[ "$MODE" == "print" ]]; then
  echo "### ${ENV_FILE}"; env_template
  echo "### ${UNITS}/takt-hook.service"; service_text
  echo "### ${UNITS}/takt-hook-poll.service"; poll_service_text
  echo "### ${UNITS}/takt-hook-poll.timer"; timer_text
  exit 0
fi

[ "$(id -u)" -eq 0 ] || die "нужны права root: sudo $0"
id "$STAND_USER" >/dev/null 2>&1 || die "пользователя '$STAND_USER' нет"
[[ -f "$STAND_DIR/scripts/stand-hook.py" ]] || die "в '$STAND_DIR' нет клона Takt"
id -nG "$STAND_USER" | tr ' ' '\n' | grep -qx docker \
  || die "пользователь '$STAND_USER' не в группе docker: выкатка не поднимет стек"

mkdir -p /etc/takt
if [[ -f "$ENV_FILE" ]]; then
  echo "== секрет: ${ENV_FILE} уже есть — не трогаем"
else
  secret="$(python3 -c 'import secrets; print(secrets.token_hex(32))')"
  env_template | sed "s/<секрет>/${secret}/" > "$ENV_FILE"
  echo "== секрет создан: ${ENV_FILE}"
fi
chmod 600 "$ENV_FILE"
chown root:root "$ENV_FILE"

service_text > "$UNITS/takt-hook.service"
poll_service_text > "$UNITS/takt-hook-poll.service"
timer_text > "$UNITS/takt-hook-poll.timer"
systemctl daemon-reload
systemctl enable --now takt-hook.service takt-hook-poll.timer
systemctl restart takt-hook.service

echo "== приёмник: $(systemctl is-active takt-hook.service), опрос: $(systemctl is-active takt-hook-poll.timer)"
echo
echo "Осталось:"
echo "  1. sudo PREFIX=${PREFIX} scripts/setup-nginx-takt.sh   # адреса приёмника в nginx"
echo "  2. GitHub -> Settings -> Webhooks -> Add webhook:"
echo "       Payload URL:  https://<стенд>${PREFIX}/hooks/github"
echo "       Content type: application/json"
echo "       Secret:       sudo sed -n 's/^TAKT_HOOK_SECRET=//p' ${ENV_FILE}"
echo "       Events:       Just the push event"
echo "  3. Проверка: curl -s https://<стенд>${PREFIX}/hooks/status"
