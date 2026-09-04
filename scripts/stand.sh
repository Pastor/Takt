#!/usr/bin/env bash
# Управление стендом сервиса проектов (фича 0531, задача 07c, требование R14).
#
# Четыре действия и ни одним больше — запуск, гашение, проверка, перезапуск:
#
#   scripts/stand.sh up        поднять (собрать образ и запустить)
#   scripts/stand.sh down      погасить (данные остаются в томах)
#   scripts/stand.sh status    проверить: что запущено и жив ли сервис
#   scripts/stand.sh restart   перезапустить сервер, не трогая базу
#
# ⚠️ Скрипт **не удаляет тома**: `docker compose down -v` уносит и базу, и
# исходники пользователей. Такое делают руками и осознанно, а не командой,
# которую набирают по привычке.
#
# ⚠️ `status` спрашивает `/health`, а не «поднят ли контейнер»: контейнер,
# который поднят и не видит базы, живым не является — ровно за этим `/health`
# и ходит в базу.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEPLOY="$ROOT/web/deploy"
BIND="${TAKT_WEB_BIND:-127.0.0.1:8730}"

compose() {
  # `docker compose` (плагин) либо `docker-compose` (старый бинарник): на
  # стендах встречаются оба, и падать из-за этого незачем.
  if docker compose version >/dev/null 2>&1; then
    docker compose --project-directory "$DEPLOY" -f "$DEPLOY/docker-compose.yml" "$@"
  elif command -v docker-compose >/dev/null 2>&1; then
    docker-compose --project-directory "$DEPLOY" -f "$DEPLOY/docker-compose.yml" "$@"
  else
    echo "ОШИБКА: не найден ни 'docker compose', ни 'docker-compose'" >&2
    exit 1
  fi
}

case "${1:-}" in
  up)
    echo "Стенд Takt: подъём..."
    compose up -d --build
    echo "  Ждём готовности сервиса..."
    for _ in $(seq 1 60); do
      if curl -fsS "http://$BIND/health" >/dev/null 2>&1; then
        echo "  Стенд поднят: http://$BIND/"
        exit 0
      fi
      sleep 2
    done
    # ⚠️ Молчаливого «наверное, поднялся» здесь нет: подъём, о котором нельзя
    # сказать, удался ли он, — это отказ, о котором узнают позже и хуже.
    echo "  ОШИБКА: сервис не ответил на /health за две минуты"
    compose logs --tail=40 server
    exit 1
    ;;
  down)
    echo "Стенд Takt: гашение..."
    compose down
    echo "  Погашен. Тома (база и исходники) целы — их снимают руками."
    ;;
  status)
    compose ps
    echo
    if curl -fsS "http://$BIND/health" >/dev/null 2>&1; then
      echo "  /health: сервис отвечает и видит базу"
    else
      echo "  /health: НЕ отвечает (контейнер может быть поднят — этого мало)"
      exit 1
    fi
    ;;
  restart)
    echo "Стенд Takt: перезапуск сервера..."
    # Только сервер: база перезапуска не требует, а лишний её останов — это
    # разорванные соединения на ровном месте.
    compose restart server
    for _ in $(seq 1 60); do
      if curl -fsS "http://$BIND/health" >/dev/null 2>&1; then
        echo "  Перезапущен."
        exit 0
      fi
      sleep 2
    done
    echo "  ОШИБКА: после перезапуска сервис не отвечает"
    exit 1
    ;;
  *)
    sed -n '2,20p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
    exit 2
    ;;
esac
