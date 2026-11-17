#!/usr/bin/env bash
# Локальная Docker-проверка stand.sh с двумя собранными каталогами статики.
# По умолчанию HTTP обслуживает nginx. TAKT_TEST_SERVER_IMAGE включает
# проверку API на образе Takt с временной базой и синтетическим проектом.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[[ $# -eq 2 ]] || { echo "Использование: $0 OLD_DIST NEW_DIST" >&2; exit 2; }
old="$(cd "$1" && pwd)"
new="$(cd "$2" && pwd)"
server_image="${TAKT_TEST_SERVER_IMAGE:-}"
web_image="${server_image:-nginx:latest}"
db_image="${TAKT_TEST_DB_IMAGE:-postgres:18-alpine}"
docker image inspect "$web_image" "$db_image" >/dev/null

work="$(mktemp -d)"
export COMPOSE_PROJECT_NAME="takt-wasm-test-$$"
export TAKT_WEB_IMAGE="$COMPOSE_PROJECT_NAME-server:stand"
legacy_image="$COMPOSE_PROJECT_NAME-legacy:local"
export TAKT_WEB_BASE_PATH=/takt
export TAKT_WEB_BIND="127.0.0.1:${TAKT_TEST_PORT:-18730}"
export TAKT_WEB_JWT_SECRET=temporary-test-secret-for-takt-wasm
export POSTGRES_PASSWORD=temporary-test-password
mkdir -p "$work/scripts" "$work/web/deploy"
cp "$ROOT/scripts/"{stand.sh,merge-wasm.py} "$work/scripts/"
sed "s|image: postgres:18-alpine|image: $db_image|" "$ROOT/web/deploy/docker-compose.yml" \
  > "$work/web/deploy/docker-compose.yml"
compose() {
  docker compose -p "$COMPOSE_PROJECT_NAME" -f "$work/web/deploy/docker-compose.yml" "$@"
}
cleanup() {
  compose down --volumes >/dev/null
  docker image rm "$TAKT_WEB_IMAGE" "$legacy_image" >/dev/null 2>&1 || true
  rm -rf "$work"
}
trap cleanup EXIT

printf 'FROM %s\n' "$web_image" > "$work/web/deploy/Dockerfile"
if [[ -n "$server_image" ]]; then
  cat >> "$work/web/deploy/Dockerfile" <<'DOCKERFILE'
USER root
RUN rm -rf /srv/takt/static
COPY static/ /srv/takt/static/
USER takt
DOCKERFILE
else
  cat >> "$work/web/deploy/Dockerfile" <<'DOCKERFILE'
COPY static/ /srv/takt/static/
COPY test-nginx.conf /etc/nginx/conf.d/default.conf
DOCKERFILE
fi
cat > "$work/test-nginx.conf" <<'NGINX'
server {
  listen 8730;
  location = /takt/health { return 200 "ok"; }
  location /takt/wasm/ { alias /srv/takt/static/wasm/; }
  location ~ ^/takt/(wasm/[0-9.]+/.*)$ {
    alias /srv/takt/static/$1;
    add_header Cache-Control "public, max-age=31536000, immutable";
  }
}
NGINX
install_static() {
  rm -rf "$work/static"
  cp -R "$1" "$work/static"
}
url="http://$TAKT_WEB_BIND/takt"
fetch() {
  curl -fsS --max-time 5 "$url/wasm/$1"
}
assert_version() {
  local version="$1" dist="$2" file
  for file in takt.wasm takt-export.wasm manifest.json; do
    curl -fsS --max-time 5 -D "$work/headers" "$url/wasm/$version/$file" > "$work/response"
    grep -qi '^Cache-Control: .*immutable' "$work/headers"
    cmp "$dist/wasm/$version/$file" "$work/response"
  done
}

# Первый контейнер создан обычным Compose, как в прежней выкатке.
install_static "$old"
TAKT_WEB_IMAGE="$legacy_image" compose up -d --build
for _ in $(seq 1 30); do
  if curl -fsS --max-time 2 "$url/health" >/dev/null 2>&1; then break; fi
  sleep 1
done
assert_version 0.62.2 "$old"
compose ps -q server > "$work/legacy"

if [[ -n "$server_image" ]]; then
  token="$(curl -fsS "$url/api/register" -H 'Content-Type: application/json' \
    --data '{"login":"takt714","password":"temporary-test-password"}' \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["access_token"])')"
  curl -fsS "$url/api/projects" -H "Authorization: Bearer $token" \
    -H 'Content-Type: application/json' --data '{"name":"TAKT-714"}' > "$work/project-before"
  project="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])' < "$work/project-before")"
  curl -fsS -X PUT "$url/api/projects/$project/files/Pump.takt" \
    -H "Authorization: Bearer $token" -H 'Content-Type: application/json' \
    --data '{"text":"in level: u8; out speed: u8; start Run { always { speed := level; } ref Run; }"}' > /dev/null
  curl -fsS "$url/api/projects/$project" -H "Authorization: Bearer $token" > "$work/project-before"
fi

install_static "$new"
"$work/scripts/stand.sh" up
[[ "$(compose ps -q server)" != "$(cat "$work/legacy")" ]]
assert_version 0.62.2 "$old"
assert_version 0.62.3 "$new"
fetch index.json > "$work/index"
python3 - "$work/index" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as index:
    assert json.load(index) == {"latest": "0.62.3", "versions": ["0.62.2", "0.62.3"]}
PY

if [[ -n "$server_image" ]]; then
  curl -fsS "$url/p/$project" > /dev/null
  curl -fsS "$url/api/projects/$project" -H "Authorization: Bearer $token" > "$work/project-after"
  python3 - "$work/project-before" "$work/project-after" <<'PYTEST'
import json
import sys
from pathlib import Path

before, after = (json.loads(Path(file).read_text()) for file in sys.argv[1:])
assert before["takt_lang"] == after["takt_lang"] == "0.62.2"
assert before["revision"] == after["revision"]
assert before["files"] == after["files"] and before["files"]
PYTEST
  curl -fsS "$url/api/projects/$project/archive?target=c" -H "Authorization: Bearer $token" \
    -o "$work/project.zip"
  python3 - "$work/project.zip" <<'PYTEST'
import json
import sys
from zipfile import ZipFile

with ZipFile(sys.argv[1]) as archive:
    files = archive.namelist()
    assert "generated/REFUSAL.txt" not in files, archive.read("generated/REFUSAL.txt")
    assert any(name.startswith("generated/") and name.endswith((".c", ".h")) for name in files)
    assert json.loads(archive.read("takt-project.json"))["takt_lang"] == "0.62.2"
PYTEST
  echo "OK: проект 0.62.2 открывается и выгружается с генерацией C, версия и ревизия сохранены"
fi

"$work/scripts/stand.sh" up
fetch index.json | cmp "$work/index" -
assert_version 0.62.2 "$old"
assert_version 0.62.3 "$new"
echo "OK: обе версии, описи и индекс пережили замену контейнера и повтор"

# Конфликт с корректной описью должен выявляться по совпадению URL.
python3 - "$work/static/wasm/0.62.3" <<'PY'
import hashlib
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
module = root / "takt-export.wasm"
data = module.read_bytes() + b"conflict"
module.write_bytes(data)
manifest = root / "manifest.json"
doc = json.loads(manifest.read_text())
doc["export"].update(size=len(data), sha256=hashlib.sha256(data).hexdigest())
manifest.write_text(json.dumps(doc))
PY
before="$(compose ps -q server)"
if "$work/scripts/stand.sh" up > "$work/conflict.log" 2>&1; then
  echo "Конфликт immutable прошёл" >&2
  exit 1
fi
grep 'уже опубликован другой модуль' "$work/conflict.log"
[[ "$(compose ps -q server)" == "$before" ]]
fetch index.json | cmp "$work/index" -
assert_version 0.62.2 "$old"
assert_version 0.62.3 "$new"
echo "OK: конфликт экспорта остановил переключение"

# Смена контейнера проходит, но новый сервер выдаёт неверный индекс.
install_static "$new"
cp "$work/web/deploy/Dockerfile" "$work/Dockerfile.healthy"
cp "$work/test-nginx.conf" "$work/nginx.healthy"
if [[ -n "$server_image" ]]; then
  mkdir -p "$work/wrong/wasm"
  cp "$old/version.json" "$work/wrong/version.json"
  cp "$old/wasm/index.json" "$work/wrong/wasm/index.json"
  cat >> "$work/web/deploy/Dockerfile" <<'DOCKERFILE'
COPY wrong/ /srv/takt/wrong/
ENV TAKT_WEB_STATIC=/srv/takt/wrong
DOCKERFILE
else
  printf '%s\n' 'server { listen 8730; location / { return 200 "wrong"; } }' \
    > "$work/test-nginx.conf"
fi
if "$work/scripts/stand.sh" up > "$work/rollback.log" 2>&1; then
  echo "Проверка ошибочно приняла неверный индекс" >&2
  exit 1
fi
grep 'Прежний образ снова обслуживает запросы' "$work/rollback.log"
[[ "$(compose ps -q server)" != "$before" ]]
fetch index.json | cmp "$work/index" -
assert_version 0.62.2 "$old"
assert_version 0.62.3 "$new"
echo "OK: после отказа новый контейнер заменён прежним образом"

# После штатного down старый контейнер отсутствует, а архив остался в образе.
mv "$work/Dockerfile.healthy" "$work/web/deploy/Dockerfile"
mv "$work/nginx.healthy" "$work/test-nginx.conf"
"$work/scripts/stand.sh" down
"$work/scripts/stand.sh" up
fetch index.json | cmp "$work/index" -
assert_version 0.62.2 "$old"
assert_version 0.62.3 "$new"
echo "OK: архив восстановлен из образа после down"
