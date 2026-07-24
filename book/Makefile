# Сборка документа «Язык Takt — описание» (mdBook → PDF).
#
# Вывод по умолчанию — PDF (профиль mdbook-pandoc `pdf`, шрифт текста Fira Code).
#
# Зависимости:
#   - mdBook            — cargo install mdbook
#   - mdbook-pandoc     — cargo install mdbook-pandoc   (бэкенд PDF)
#   - pandoc            — https://pandoc.org/  (конвертер)
#   - TeX-движок xelatex — TeX Live / MacTeX (система шрифтов fontspec)
#   - шрифт Fira Code   — установить в систему (https://github.com/tonsky/FiraCode)
#
# Результат: book/pandoc/pdf/takt-language.pdf (каталог сборки игнорируется git).
#
# Цели:
#   make build   — собрать PDF
#   make check   — сборка «на прочность» (для CI)
#   make clean   — удалить каталог сборки

MDBOOK ?= mdbook

.PHONY: build check clean deps

deps:
	@ok=1; \
	for t in $(MDBOOK) pandoc xelatex; do \
	  command -v $$t >/dev/null 2>&1 || { echo "ОШИБКА: не найден '$$t' в PATH."; ok=0; }; \
	done; \
	$(MDBOOK) --help 2>/dev/null | grep -q . || true; \
	if [ "$$ok" != "1" ]; then \
	  echo "Установите зависимости PDF-сборки (см. шапку Makefile):"; \
	  echo "  cargo install mdbook mdbook-pandoc"; \
	  echo "  + pandoc, xelatex (TeX Live/MacTeX), шрифт Fira Code"; \
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
