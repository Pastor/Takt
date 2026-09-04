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
RUN cargo build --release --target wasm32-unknown-unknown -p takt-wasm \
 && TAKT_WEB_DIST=/out/static scripts/build-web.sh

# Сервер — свой workspace (полторы сотни зависимостей не должны собираться на
# каждой правке компилятора).
RUN cd web/server && cargo build --release && cp target/release/takt-web-server /out/

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
