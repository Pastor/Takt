# Анализ фичи 0122: Сборка PDF через latexmk (корректные кросс-ссылки) + Makefile

> Фича: [../features/0122-book-pdf-latexmk.md](../features/0122-book-pdf-latexmk.md) · ADR: [../adr/0122-book-pdf-latexmk.md](../adr/0122-book-pdf-latexmk.md) · тест-план: [../tests/README.md](../tests/README.md)

## Цель

`make build` должен формировать **корректные перекрёстные ссылки** (оглавление,
кросс-главные ссылки на приложения) без ручных повторных проходов. Инфраструктура
(не язык, Tier 3).

## Найдено (проверено на рабочей машине)

- **latexmk доступен** (`/Library/TeX/texbin/latexmk`, v4.88) — перезапускает
  движок до сходимости меток.
- **Профиль mdbook-pandoc = pandoc defaults-файл** → принимает `pdf-engine` и
  `pdf-engine-opts`. `pdf-engine = "latexmk"`, `pdf-engine-opts = ["-xelatex"]`
  заставляет latexmk гнать **xelatex** (нужен для fontspec/Fira Code).
- **TeX не в PATH у `make`**: MacTeX кладёт бинарники в `/Library/TeX/texbin`,
  который в интерактивном shell есть, а в `make` — не обязательно. Решение —
  `export PATH := /Library/TeX/texbin:…:$(PATH)` в Makefile.

## Проверка

- `mdbook build` (через профиль latexmk) → PDF собран (EXIT 0).
- `pdftotext takt-language.pdf | grep -c '??'` = **0** (все кросс-ссылки
  разрешены; при одиночном xelatex оставались бы «??»).
- `make build` с **урезанным** PATH (`env -i`) — успешно (Makefile добавляет
  каталоги TeX сам). Perl-варнинги latexmk под `env -i` — артефакт стрип-окружения,
  под нормальным shell их нет.

## Ограничения

Пути TeX в PATH заданы под macOS/MacTeX; на иных ОС — системный PATH. Язык не
меняется; вывод генераторов/тесты не затронуты.
