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
#
# Результат: book/pdf/takt-language.pdf (каталог сборки игнорируется git).
#
# Цели:
#   make build   — собрать PDF (кросс-ссылки разрешаются latexmk)
#   make check   — сборка «на прочность» (для CI)
#   make clean   — удалить каталог сборки

MDBOOK ?= mdbook

# TeX ставится в PATH не всегда (MacTeX кладёт бинарники в /Library/TeX/texbin,
# который в интерактивном shell есть, а в make — не обязательно). Добавляем
# типичные каталоги TeX в PATH — безвредно, если их нет.
export PATH := /Library/TeX/texbin:/usr/local/texlive/2026/bin/universal-darwin:$(PATH)

.PHONY: build check clean deps

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

build: deps
	$(MDBOOK) build

check: deps
	$(MDBOOK) build

clean:
	$(MDBOOK) clean 2>/dev/null || rm -rf book
