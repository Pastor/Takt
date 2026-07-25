# Сборка документа «Язык Takt — описание» (mdBook → PDF).
#
# Вывод по умолчанию — PDF (профиль mdbook-pandoc `pdf`, шрифт текста Fira Code).
#
# Кросс-ссылки (оглавление, ссылки на приложения) формируются корректно: движок
# сборки — **latexmk** поверх xelatex (профиль `pdf` в `book.toml`). latexmk сам
# перезапускает xelatex до сходимости перекрёстных ссылок, поэтому одиночный проход
# с «??»/устаревшими номерами исключён — ручные повторные проходы не нужны.
#
# Зависимости:
#   - mdBook            — cargo install mdbook
#   - mdbook-pandoc     — cargo install mdbook-pandoc   (бэкенд PDF)
#   - pandoc            — https://pandoc.org/  (конвертер)
#   - TeX-движок xelatex — TeX Live / MacTeX (система шрифтов fontspec)
#   - latexmk           — TeX Live / MacTeX (сходимость кросс-ссылок)
#   - шрифт Fira Code   — установить в систему (https://github.com/tonsky/FiraCode)
#   - для АВТО-РЕГЕНЕРАЦИИ графов верификации (цель `graphs`, зависимость `build`):
#     cargo (сборка taktc), dot (Graphviz), python3 + fontTools, шрифт ГОСТ 2.304-81
#     тип А (наклонный, ставит цель `fonts`). Все — best-effort: нет инструмента →
#     регенерация пропускается, берутся закоммиченные (самодостаточные) SVG.
#
# Результат: book/pdf/takt-language.pdf (каталог сборки игнорируется git).
#
# Цели:
#   make build   — регенерировать графы + собрать PDF (кросс-ссылки — latexmk)
#   make check   — то же «на прочность» (для CI)
#   make graphs  — регенерировать SVG-графы верификации (taktc → dot → флаттеринг)
#   make fonts   — установить шрифт ГОСТ тип А, если его нет
#   make clean   — удалить каталог сборки

MDBOOK ?= mdbook

# Корень репозитория (Makefile лежит в book/). Нужен для сборки taktc и запуска
# скрипта регенерации графов.
ROOT := $(abspath $(CURDIR)/..)

# Шрифт подписей графов верификации — ГОСТ тип А, НАКЛОННЫЙ (ГОСТ 2.304-81,
# стандартное чертёжное начертание). Диаграммы Крипке/Бюхи (`taktc verify
# --emit-graph` → SVG) подписаны им; без шрифта rsvg-convert подставит запасной.
# Свободный шрифт из релиза проекта Metrolog/Font.GOST2.304-81.
GOST_FONT_URL  := https://github.com/Metrolog/Font.GOST2.304-81/releases/download/v0.7.6/GOST2.304-81TypeA-Slanted.ttf
GOST_FONT_FILE := GOST2.304-81TypeA-Slanted.ttf
# Каталог пользовательских шрифтов по ОС (macOS / прочее).
FONT_DIR := $(if $(filter Darwin,$(shell uname)),$(HOME)/Library/Fonts,$(HOME)/.local/share/fonts)

# TeX ставится в PATH не всегда (MacTeX кладёт бинарники в /Library/TeX/texbin,
# который в интерактивном shell есть, а в make — не обязательно). Добавляем
# типичные каталоги TeX в PATH — безвредно, если их нет.
export PATH := /Library/TeX/texbin:/usr/local/texlive/2026/bin/universal-darwin:$(PATH)

.PHONY: build check clean deps fonts graphs

# Регенерирует SVG-диаграммы верификации ПЕРЕД сборкой — устраняет ручной шаг.
# Конвейер: taktc verify --emit-graph → dot -Tsvg → svg_flatten_text.py (см.
# book/scripts/render_verify_graphs.sh). Требует cargo (taktc), dot (Graphviz),
# python3 + fontTools и шрифт ГОСТ (цель fonts). BEST-EFFORT: если инструмента или
# шрифта нет, оставляет уже закоммиченные (самодостаточные) SVG и НЕ валит сборку —
# так минимальная PDF-сборка без Rust/Graphviz продолжает работать.
graphs: fonts
	@ok=1; \
	for t in cargo dot python3; do command -v $$t >/dev/null 2>&1 || ok=0; done; \
	python3 -c "import fontTools" >/dev/null 2>&1 || ok=0; \
	fc-list 2>/dev/null | grep -qi "GOST 2.304" || ok=0; \
	if [ "$$ok" = "1" ]; then \
	  echo "Регенерация графов верификации…"; \
	  ( cd "$(ROOT)" && cargo build --quiet --bin taktc ) \
	    && sh "$(ROOT)/book/scripts/render_verify_graphs.sh" \
	    || echo "  ⚠ регенерация не удалась — использую закоммиченные SVG."; \
	else \
	  echo "  ⚠ нет cargo/dot/python3+fontTools/шрифта — пропускаю регенерацию (использую закоммиченные SVG)."; \
	fi

# Устанавливает шрифт ГОСТ тип А (наклонный), если его ещё нет в системе.
# Идемпотентно (при наличии семейства ничего не качает) и BEST-EFFORT: сбой
# загрузки (офлайн, нет curl/wget) НЕ валит сборку — подписи графов уже впечатаны
# в SVG контурами (см. book/scripts/svg_flatten_text.py), поэтому для сборки PDF
# шрифт не требуется. Он нужен лишь для РЕГЕНЕРАЦИИ графов
# (book/scripts/render_verify_graphs.sh).
fonts:
	@if fc-list 2>/dev/null | grep -qi "GOST 2.304"; then \
	  echo "Шрифт ГОСТ 2.304-81 уже установлен."; \
	else \
	  echo "Ставлю шрифт ГОСТ тип А (наклонный) в $(FONT_DIR) (best-effort)…"; \
	  mkdir -p "$(FONT_DIR)"; \
	  if command -v curl >/dev/null 2>&1; then \
	    curl -fsSL "$(GOST_FONT_URL)" -o "$(FONT_DIR)/$(GOST_FONT_FILE)" \
	      || echo "  ⚠ загрузка шрифта не удалась — не критично (SVG самодостаточны)."; \
	  elif command -v wget >/dev/null 2>&1; then \
	    wget -qO "$(FONT_DIR)/$(GOST_FONT_FILE)" "$(GOST_FONT_URL)" \
	      || echo "  ⚠ загрузка шрифта не удалась — не критично (SVG самодостаточны)."; \
	  else \
	    echo "  ⚠ нет ни curl, ни wget — пропускаю (SVG самодостаточны)."; \
	  fi; \
	  command -v fc-cache >/dev/null 2>&1 && fc-cache -f "$(FONT_DIR)" >/dev/null 2>&1 || true; \
	fi

deps:
	@ok=1; \
	for t in $(MDBOOK) pandoc xelatex latexmk; do \
	  command -v $$t >/dev/null 2>&1 || { echo "ОШИБКА: не найден '$$t' в PATH."; ok=0; }; \
	done; \
	if [ "$$ok" != "1" ]; then \
	  echo "Установите зависимости PDF-сборки (см. шапку Makefile):"; \
	  echo "  cargo install mdbook mdbook-pandoc"; \
	  echo "  + pandoc, xelatex, latexmk (TeX Live/MacTeX), шрифт Fira Code"; \
	  exit 1; \
	fi
	@command -v mdbook-pandoc >/dev/null 2>&1 || { \
	  echo "ОШИБКА: не найден бэкенд 'mdbook-pandoc'. Установите: cargo install mdbook-pandoc"; \
	  exit 1; }

build: deps graphs
	$(MDBOOK) build

check: deps graphs
	$(MDBOOK) build

clean:
	$(MDBOOK) clean 2>/dev/null || rm -rf book
