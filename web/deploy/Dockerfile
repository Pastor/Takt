# Образ сервиса проектов Takt (фича 0531, задача 07c).
#
# Многостадийная сборка: инструменты сборки в готовый образ не едут.
#
# ⚠️ Собирается ДВОЕ, и оба обязательны:
#   1. модуль `takt-wasm` и статика — их отдаёт браузеру сервер;
#   2. сам сервер — он же исполняет тот же модуль при выгрузке архива (09g).
# Разъедься они по версиям — вывод целей в архиве разошёлся бы с браузером
# молча. Здесь этого не случится: обе половины собираются из одного дерева.

FROM rust:1.97-bookworm AS build
WORKDIR /src

# Толчейн пинуется деревом (`rust-toolchain.toml`), а не строкой здесь: второй
# носитель версии разошёлся бы с первым (класс 0084).
COPY rust-toolchain.toml ./
RUN rustup target add wasm32-unknown-unknown

COPY . .

# Статика: модуль + страница. Она же нужна серверу — из неё он берёт
# `wasm/<версия>/takt.wasm` для архива.
# ⚠️ Профиль — `wasm`, а не `release`: `build-web.sh` ищет модуль по
# `TAKT_WASM_PROFILE` (умолчание `wasm`), и собранный `release` он не находит —
# «Модуль не собран», сборка образа падает. Профиль назван ОДИН раз, здесь же и
# передаётся скрипту: два носителя одного знания уже разошлись однажды
# (выкладка на стенд 2026-09-05).
ARG WASM_PROFILE=wasm
RUN cargo build --profile ${WASM_PROFILE} --target wasm32-unknown-unknown -p takt-wasm \
 && TAKT_WASM_PROFILE=${WASM_PROFILE} TAKT_WEB_DIST=/out/static scripts/build-web.sh

# Сервер — свой workspace (полторы сотни зависимостей не должны собираться на
# каждой правке компилятора).
# ⚠️ Каталог сборки задаётся ЯВНО: `.cargo/config.toml` корня переносит его в
# `target/precheck` (фича 0251), и конфигурация достаёт до `web/server` — тот же
# класс, что у расширения Zed (0414). Без этого `cargo` собирает успешно, а
# `cp` ищет бинарник по несуществующему пути: «No such file or directory» после
# полутора сотен скомпилированных крейтов (выкладка на стенд 2026-09-05).
# Переменная окружения сильнее конфигурации — этим и пользуемся.
ARG SERVER_TARGET_DIR=/build/web-server
RUN cd web/server \
 && CARGO_TARGET_DIR=${SERVER_TARGET_DIR} cargo build --release \
 && cp ${SERVER_TARGET_DIR}/release/takt-web-server /out/

FROM debian:bookworm-slim AS runtime
# `ca-certificates` — для исходящих обращений (вход через площадки, задача 09f);
# `curl` — для проверки живости из `docker-compose`.
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/*

# Служба работает НЕ от root: у неё есть каталог с чужими исходниками.
RUN useradd --system --home /srv/takt --create-home takt
WORKDIR /srv/takt

COPY --from=build /out/takt-web-server /usr/local/bin/takt-web-server
COPY --from=build /out/static /srv/takt/static

# Каталог исходников — том: он переживает пересборку образа (задача 09h).
RUN mkdir -p /srv/takt/projects && chown -R takt:takt /srv/takt
VOLUME ["/srv/takt/projects"]
USER takt

ENV TAKT_WEB_HOST=0.0.0.0 \
    TAKT_WEB_PORT=8730 \
    TAKT_WEB_STATIC=/srv/takt/static \
    TAKT_WEB_PROJECTS=/srv/takt/projects
EXPOSE 8730

# ⚠️ Живость спрашивает БАЗУ (`/health` ходит в неё): сервис, у которого базы
# нет, живым не является, и оркестратор обязан узнать об этом сам.
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
  CMD curl -fsS http://127.0.0.1:8730/health || exit 1

ENTRYPOINT ["takt-web-server"]
