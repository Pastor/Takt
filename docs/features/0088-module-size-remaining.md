# Фича 0088: Остальные нарушители лимита размера модуля (17 файлов)

- **Номер:** 0088
- **Статус:** ГОТОВО (см. «Итог (что сделано)» ниже)
- **Зависит от:** не проставлено — проставит аналитик на стадии анализа (правило 17)
- **Приоритет / Tier:** **Tier 3** (качество кода/процесс; не багфикс) — проставлен ADR 0088
- **Крейт:** `grammar`, `simulation`
- **Связанные issue (анализ):** новая фича (перевод кандидата из `FEATURES.md`)

## Ссылки на артефакты (жизненный цикл, правило 17)

| Стадия | Артефакт |
|---|---|
| Архитектура (ADR) | [`docs/adr/0088-module-size-remaining.md`](../adr/0088-module-size-remaining.md) — **Accepted** (Option B: безопасная часть, 12 файлов; ядро+пришпиленные — новая фича) |
| Анализ | [`docs/analyze/0088-module-size-remaining.md`](../analyze/0088-module-size-remaining.md) — декомпозиция 0088-01…06 |
| Разработка | [`0088-01`](../development/0088-01-module-size-remaining.md) c_model init · [`0088-02`](../development/0088-02-module-size-remaining.md) rust_cond · [`0088-03`](../development/0088-03-module-size-remaining.md) rust_tick · [`0088-04`](../development/0088-04-module-size-remaining.md) Token · [`0088-05`](../development/0088-05-module-size-remaining.md) Expression · [`0088-06`](../development/0088-06-module-size-remaining.md) lexer_tests · [`0088-07`](../development/0088-07-module-size-remaining.md) lsp_tests · [`0088-08`](../development/0088-08-module-size-remaining.md) codegen_tests · [`0088-09`](../development/0088-09-module-size-remaining.md) c_source · [`0088-10`](../development/0088-10-module-size-remaining.md) parser_tests · [`0088-11`](../development/0088-11-module-size-remaining.md) semantic_tests · [`0088-12`](../development/0088-12-module-size-remaining.md) viewport (готовы). **Безопасная часть закрыта** (реестр 18 → 6); остаток — ядро+пришпиленные (фича-преемник) |
| Тест-план | [`docs/tests/`](../tests/README.md) (`0088-*`) |
| Отчёт о тестировании | [`docs/reports/`](../reports/README.md) (`0088-*`) |
| Исправления | [`docs/fixes/`](../fixes/README.md) (при необходимости `0088-YY-*`) |

## Краткое описание

[0027](0027-module-size-split.md) взяла в объём **три** худших модуля и
**заморозила** остальные храповиком `scripts/check-module-size.sh` (реестр
`scripts/module-size-baseline.txt` — **узаконенный долг**, а не разрешение).

Остаток:

- **крупнейший файл проекта — `grammar/tests/semantic_tests.rs`, 4766 строк**, на
  31% больше `validate.rs`, а правило распространяется на тесты **явным текстом**
  «вместе с тестами»;
- следом — `semantic/mod.rs` (1708);
- суммарное превышение по 15 модулям `src` — **6255 строк**.

Смежно с фичей [0069](0069-address-map-eval-split.md) (новый нарушитель
`address_map.rs`, вне реестра).

Вынесено из [0027](0027-module-size-split.md).

> Фича зарегистрирована **2026-07-17** переводом кандидата из `FEATURES.md`
> (решение заказчика: «завести фичи по кандидатам, пока без проработки»).
> **Проработка не проводилась:** ADR, анализ, зависимости, Tier и объём — за
> стадиями 2–3 (правило 17). Текст ниже — **перенос находки кандидата** вместе с
> подтверждающими её пробами; это описание проблемы, а **не** принятое решение.

## Итог (что сделано)

**Закрыта 2026-07-24** (`ГОТОВО`, вердикт [отчёта](../reports/0088-module-size-remaining.md)
— ГОТОВО). Принята **Option B** ([ADR](../adr/0088-module-size-remaining.md)):
в объём взята **безопасная часть** (чистое перемещение, вывод байт-в-байт
неизменен); ядро семантики и пришпиленные — фича-преемник.

Сделано (12 подзадач, каждая — отдельный коммит с зелёным `precheck.sh`; реестр
`scripts/module-size-baseline.txt` **18 → 6**):

- **Генераторы** (`grammar`): 0088-01 init-группа C → `c_model_init.rs`; 0088-02
  печатник условий Rust → `rust_cond.rs`; 0088-03 такт+переходы Rust →
  `rust_tick.rs`; 0088-09 inline-тесты `c_source` → `c_source/tests/part2.rs`.
- **Парсер** (`grammar`): 0088-04 `enum Token` → `parser/token.rs`; 0088-05 узел
  `Expression` → `parser/ast_expr.rs`.
- **Тесты** (`grammar`): 0088-06 `lexer_tests`→`part2`, 0088-07 `lsp_tests`→`more`,
  0088-08 `codegen_tests`→`part2`, 0088-10 `parser_tests`→`part2`, 0088-11
  `semantic_tests` (крупнейший файл проекта, 5015) → 5 подмодулей `part2..6`.
- **Симуляция** (`simulation`): 0088-12 `unit/viewport.rs` → вынос подмодуля
  `graph` в файл; `create_svg` не тронут → SVG-вывод неизменен.

Ключевые находки (детали — [отчёт](../reports/0088-module-size-remaining.md)):
приём «директория-подмодуль + `use super::*`» переносится и на inline-тесты в
`src` (вложенный `mod part2`); при расколе `semantic_tests` два top-level `use`
из середины файла пришлось поднять в шапку родителя; врезка о `viewport` как
«одной функции ~1128 строк» оказалась неверной — естественная граница модуля
`graph` решила задачу перемещением, без дробления тела и риска для SVG.

**Остаток → фича-преемник** [0099](0099-module-size-core.md) (критерий A5
анализа): 6 файлов — ядро семантики (`tree`, `mod`, `expression`,
`type_inference`) и пришпиленные (`lamc`, `lib`), где вынос не сводится к чистому
перемещению. Язык не менялся → версии не поднимаются (правило 22); исправлений
(`docs/fixes/`) не потребовалось.
