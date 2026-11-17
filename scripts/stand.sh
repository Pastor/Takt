#!/usr/bin/env bash
# Управление стендом сервиса проектов.
#
# Четыре действия и ни одним больше - запуск, гашение, проверка, перезапуск:
#
# scripts/stand.sh up поднять (собрать образ и запустить)
# scripts/stand.sh down погасить (данные остаются в томах)
# scripts/stand.sh status проверить: что запущено и жив ли сервис
# scripts/stand.sh restart перезапустить сервер, не трогая базу
#
# Скрипт **не удаляет тома**: `docker compose down -v` уносит и базу, и
# исходники пользователей. Такое делают руками и осознанно, а не командой,
# которую набирают по привычке.
#
# `status` спрашивает `/health`, а не "поднят ли контейнер": контейнер,
# который поднят и не видит базы, живым не является - ровно за этим `/health`
# и ходит в базу.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEPLOY="$ROOT/web/deploy"
# Префикс входит в адрес проверки живости: сервер вкладывает под него весь
# роутер, и при `TAKT_WEB_BASE_PATH=/takt` `/health` живёт по `/takt/health`.
# Без этого `status` спрашивал бы несуществующий адрес и объявлял живой стенд
# мёртвым (нашлось при разборе выкатки под префиксом, d).
# Префикс и адрес привязки берутся из файла окружения, если их нет в самом
# окружении: `docker compose` читает `web/deploy/.env` сам, а скрипт - нет, и
# проверка живости стучалась в `/health` вместо `/takt/health`. Сервер отвечал
# `404`, скрипт объявлял подъём неудавшимся и печатал журнал - при живом
# стенде. Класс найден выкаткой 2026-09-06: "ошибка: сервис не ответил" пришла
# сразу после "Started", и сервис при этом работал.
#
# Читается ровно две переменные, а не весь файл: рядом с ними лежат пароль
# базы и секрет подписи, и втягивать их в окружение ради адреса незачем.
env_value() {
  [[ -f "$DEPLOY/.env" ]] || return 0
  sed -n "s/^$1=//p" "$DEPLOY/.env" | tail -1 | tr -d '"'"'"'"'
}
BIND="${TAKT_WEB_BIND:-$(env_value TAKT_WEB_BIND)}"
BIND="${BIND:-127.0.0.1:8730}"
PREFIX="${TAKT_WEB_BASE_PATH:-$(env_value TAKT_WEB_BASE_PATH)}"
PREFIX="${PREFIX:-/}"
[[ "$PREFIX" == "/" ]] && PREFIX=""
HEALTH="http://$BIND${PREFIX}/health"
PROJECT="${COMPOSE_PROJECT_NAME:-takt}"
export TAKT_WEB_IMAGE="${TAKT_WEB_IMAGE:-$PROJECT-server:stand}"

compose() {
  # `docker compose` (плагин) либо `docker-compose` (старый бинарник): на
  # стендах встречаются оба, и падать из-за этого незачем.
  #
  # Имя проекта - `takt`, и оно задано и здесь, и в файле стека: на стенде
  # рядом работает другой сервис, а имя по умолчанию берётся у каталога.
  if docker compose version >/dev/null 2>&1; then
    docker compose -p "$PROJECT" --project-directory "$DEPLOY" -f "$DEPLOY/docker-compose.yml" "$@"
  elif command -v docker-compose >/dev/null 2>&1; then
    docker-compose -p "$PROJECT" --project-directory "$DEPLOY" -f "$DEPLOY/docker-compose.yml" "$@"
  else
    echo "ОШИБКА: не найден ни 'docker compose', ни 'docker-compose'" >&2
    exit 1
  fi
}

# Отдаётся ли сервис наружу: включение локейшенов Takt в сайте соседа.
#
# Проверка нужна потому, что включение живёт в чужом файле. Домен занят
# соседним сервисом, Takt приносит только локейшены, и его
# `include` дописывается в сайт соседа. Собственный скрипт соседа переписывает
# этот файл целиком - и включение исчезает. 2026-09-06 так и случилось: сервер
# работал, `/health` изнутри отвечал `200`, а снаружи приходил `404`, и понять
# это можно было только чтением конфигурации nginx.
#
# Проверка мягкая: без nginx (своя машина, контейнерный прокси) она молчит -
# отсутствие снаружи-прокси не делает стенд сломанным. Она говорит лишь тогда,
# когда сниппет положен, а включения на него нет: это и есть та самая пропажа.
check_nginx_include() {
  local snippet=/etc/nginx/snippets/takt-locations.conf
  [[ -r "$snippet" ]] || return 0
  if grep -rqs "takt-locations.conf" /etc/nginx/sites-enabled/ /etc/nginx/conf.d/ 2>/dev/null; then
    echo "  nginx: включение локейшенов Takt на месте"
    return 0
  fi
  echo "  nginx: локейшены Takt НЕ включены — снаружи сервис отдаваться не будет."
  echo "         Сниппет положен ($snippet), а include на него нет: скорее всего"
  echo "         сайт соседа переписан его собственной настройкой."
  echo "         Вернуть: sudo scripts/setup-nginx-takt.sh"
  return 1
}

wait_for_release() {
  local index="$1"
  for _ in $(seq 1 60); do
    if curl -fsS --max-time 2 "$HEALTH" >/dev/null 2>&1 &&
       curl -fsS --max-time 2 "http://$BIND${PREFIX}/wasm/index.json" | cmp -s "$index" -; then
      return 0
    fi
    sleep 2
  done
  return 1
}

up() (
  local release_image="$TAKT_WEB_IMAGE"
  local work previous old_image="" source="" raw_container="" archive_container=""
  work="$(mktemp -d)"
  local raw_image="takt-server-build:${work##*/}" prepared_image="takt-server-prepared:${work##*/}"
  cleanup() {
    if [[ -n "$raw_container" ]]; then docker rm -v "$raw_container" >/dev/null || true; fi
    if [[ -n "$archive_container" ]]; then docker rm -v "$archive_container" >/dev/null || true; fi
    docker image rm "$raw_image" "$prepared_image" >/dev/null 2>&1 || true
    rm -rf "$work"
  }
  trap cleanup EXIT

  # Берём архив даже из остановленного контейнера. После down источником
  # служит локальный тег последнего образа.
  previous="$(compose ps -a -q server)"
  if [[ -n "$previous" ]]; then
    old_image="$(docker inspect --format '{{.Image}}' "$previous")"
    source="$previous"
  else
    old_image="$(docker image ls --no-trunc -q "$release_image")"
    if [[ -n "$old_image" ]]; then
      archive_container="$(docker create "$old_image")"
      source="$archive_container"
    fi
  fi
  if [[ -n "$source" ]]; then
    docker cp "$source:/srv/takt/static/wasm" "$work/published"
  fi

  TAKT_WEB_IMAGE="$raw_image" compose build server
  raw_container="$(docker create "$raw_image")"
  docker cp "$raw_container:/srv/takt/static/wasm" "$work/wasm"
  python3 "$ROOT/scripts/merge-wasm.py" "$work/wasm" "$work/published"

  # Слой содержит только проверенный архив. Статика прежнего контейнера
  # остаётся в его образе до переключения.
  printf 'FROM %s\nCOPY wasm/ /srv/takt/static/wasm/\n' "$raw_image" > "$work/Dockerfile"
  printf 'published\n' > "$work/.dockerignore"
  docker build -t "$prepared_image" "$work"
  docker image tag "$prepared_image" "$release_image"

  if compose up -d --no-build && wait_for_release "$work/wasm/index.json"; then
    echo "  Стенд поднят: http://$BIND${PREFIX}/"
    return 0
  fi

  echo "  ОШИБКА: новый контейнер не подтвердил /health и готовый индекс WASM" >&2
  compose logs --tail=40 server >&2 || true
  if [[ -n "$old_image" ]]; then
    echo "  Возвращаем прежний образ..." >&2
    if docker image tag "$old_image" "$release_image" &&
       compose up -d --no-build --no-deps server && wait_for_release "$work/published/index.json"; then
      echo "  Прежний образ снова обслуживает запросы." >&2
    else
      echo "  ОШИБКА: прежний образ не прошёл проверку после отката" >&2
    fi
  fi
  return 1
)

case "${1:-}" in
  up)
    echo "Стенд Takt: подъём..."
    up
    ;;
  down)
    echo "Стенд Takt: гашение..."
    compose down
    echo "  Погашен. Тома (база и исходники) целы — их снимают руками."
    ;;
  status)
    compose ps
    echo
    if curl -fsS "$HEALTH" >/dev/null 2>&1; then
      echo "  /health: сервис отвечает и видит базу"
    else
      echo "  /health: НЕ отвечает (контейнер может быть поднят — этого мало)"
      exit 1
    fi
    check_nginx_include
    ;;
  restart)
    echo "Стенд Takt: перезапуск сервера..."
    # Только сервер: база перезапуска не требует, а лишний её останов - это
    # разорванные соединения на ровном месте.
    compose restart server
    for _ in $(seq 1 60); do
      if curl -fsS "$HEALTH" >/dev/null 2>&1; then
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
