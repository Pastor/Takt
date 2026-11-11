---
okf_version: 0.2
format: 2
type: Task
key: TAKT-613
list: ТАКТ
title: "[0132-01] символ вне шрифта документа ломал сборку PDF предупреждением"
status: done
priority: none
author: pastor
model: gpt-6
links:
  part:
  - TAKT-131
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:09:26Z
published: 2026-11-11T07:09:26Z
created: 2026-11-11T07:09:26Z
updated: 2026-11-11T07:09:26Z
anal:
  project: 22
  id: 1667
  version: 1
checksum: sha256:2813df4c44aed962e38283e8080914ae279bd0c32b982003ca2076abfd3752f8
---

Источник: [docs/fixes/0132-01-book-glyph-outside-font.md](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/fixes/0132-01-book-glyph-outside-font.md)

# Исправление: символ вне шрифта документа ломал сборку PDF предупреждением

> Фича: [../features/0132-sim-named-port-scenarios.md](../features/0132-sim-named-port-scenarios.md) ·
> обнаружено сборкой `make -C book build` по запросу заказчика 2026-07-27

- **Статус:** **ИСПРАВЛЕН** (2026-07-27). Сборка книги чистая: ни одного
  предупреждения, `book/book/pdf/takt-language.pdf` (351 КиБ) собран.
- **Класс:** дефект документа, внесённый закрытой фичей.
- **Область:** `book/src/16-tools/index.typ` (одна врезка). Код не затронут.

## Что было

Фича, описывая формат файла сценария, поставила во врезку символ `⚠️`
(U+26A0 + U+FE0F). В шрифте документа — **Fira Code** — этих глифов нет, и
сборка PDF выдавала:

```
[WARNING] Missing character: There is no ⚠ (U+26A0) in font Fira Code Regular
[WARNING] Missing character: There is no ️ (U+FE0F) in font Fira Code Regular
```

PDF при этом собирался — символ просто **молча выпадал** из вывода, оставляя
врезку без пометки.

Замер: во всём `book/src` это было **единственное** вхождение — то есть
остальной документ такого приёма не использует, и стиль был нарушен заодно с
шрифтом.

## Что сделано

Врезка переписана в принятом для документа виде — цитатным блоком с полужирным
зачином:

```markdown
> **Осторожно.** Эта форма опасна: добавление или переименование порта сдвигает
> весь массив, и шаг начинает описывать другое событие — молча.
```

## Почему прошло незамеченным

**Сборка `book/` не входит в `precheck.sh`** (требует `mdbook`,
`mdbook-pandoc`, `pandoc` и `xelatex` — их нет ни в предкоммите, ни в CI).
Проверка фичи проверяет **примеры** документа компиляцией и симуляцией, но не
собирает сам документ, поэтому дефект вёрстки для него невидим.

Вынесено кандидатом: дешёвая проверка «в `book/src` нет символов вне шрифта
документа» — она не требует TeX и ловит именно этот класс.

## Проверки

```sh
make -C book build     # предупреждений нет, PDF собран
grep -rc "⚠" book/src/ # 0
./scripts/precheck.sh  # EXIT=0 (не затронут)
```
