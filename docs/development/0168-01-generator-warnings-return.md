# Задача 0168-01: Единый контракт — цели возвращают предупреждения

> Фича: [../features/0168-generator-warnings-return.md](../features/0168-generator-warnings-return.md) · ADR: [../adr/0168-generator-warnings-return.md](../adr/0168-generator-warnings-return.md) · анализ: [../analyze/0168-generator-warnings-return.md](../analyze/0168-generator-warnings-return.md)

## Что было

Три цели заканчивали генерацию собственной копией функции `report`, печатавшей
`eprintln!` **из библиотеки**. Копий было три потому, что другого выхода наружу
у генератора не существовало **по типу**: `Generator::generate`, диспетчер
`generator::generate` и публичные входы `compile_to_{c,st,rust,sv,plantuml}`
возвращали `Result<(), Diagnostic>`.

Следствия (замеры ADR): `--quiet` не глушил, формат разошёлся с общим, у цели
`sv-mmio` уживались два канала с разной судьбой.

## Что сделано

**Тип как выход.** Правка идёт сверху вниз — иначе публичная сигнатура вернула
бы «всегда пустой `Vec`»:

- `Generator::generate` и `generator::generate` → `Result<Vec<Diagnostic>, Diagnostic>`;
- цели `c` и `plantuml` возвращают **пустой** список: канал есть, говорить по
  нему пока нечего;
- цель `rust`: `generate_program` отдаёт `(String, Vec<Diagnostic>)`;
- цель `sv`: то же, приёмник — `fsm.warnings`;
- цель `st`: приёмников **три** (функции, тела блоков, конфигурация), поэтому
  `emit_configuration` и `emit_function_block` тоже стали возвращать список, а
  `generate_program` собирает их вклад;
- `report` удалён из трёх модулей.

**Доставка у вызывающих:**

- пять публичных входов → `Result<Vec<Diagnostic>, Diagnostic>`;
- `compile_to_c_hal`, `compile_to_st_at`, `compile_to_sv_mmio` **присоединяют**
  предупреждения генератора к адресным — смешанной доставки не остаётся;
- CLI печатает единой точкой `print_warnings` (она знает `--quiet`, реестр
  файлов и общий формат): `report_simple_result` принимает новый тип, ветка
  цели `c` получила такую же печать.

**Правка вызывающего кода** (замер: ломается только там, где тип назван):
`type Compile` в `type_inference_chain_tests.rs`, явные аннотации
`Result<(), Diagnostic>` в четырёх тестах, `match` с разнотипными ветвями в
`port_access_contract_tests.rs`, внутренние тесты трёх генераторов
(`generate_program(...).unwrap()` → `.unwrap().0`). Остальные 29 файлов
переживают смену типа `Ok` без правки — как и предсказывал анализ.

**Побочная находка (исправлена здесь же).** Док-строка `FormulaSite::loc`
обещала «позицию объявления в исходном тексте», тогда как поле несёт позицию
**вместилища** — модели либо состояния. Наблюдаемо: два `invariant` в одном
файле печатаются с одной координатой (`1:1` обе). Док-строка исправлена по
факту; появление настоящей позиции у формулы — отдельный предмет (кандидат).

Функциональности вне Rust-крейтов: `н/п`.

## Проверки

```sh
cargo test --all-features
./scripts/precheck.sh
```

Сторож — `takt-lang/tests/generator_warnings_tests.rs`:

| Тест | Условие анализа |
|---|---|
| `st_returns_guard_warning` | R1/R7 (`ST-022` возвращается) |
| `st_returns_extern_stub_warning` | R1/R7 (`ST-009`) |
| `sv_returns_divider_warning` | R1/R7 (`SV-009`) |
| `sv_mmio_returns_generator_warning_too` | R6 (одна судьба у всей диагностики) |
| `c_and_plantuml_return_an_empty_channel` | R1 (канал есть у всех целей) |
| `generator_does_not_print` | R2 (в `generator/` не осталось печати) |

⚠️ `generator_does_not_print` — греп по исходнику, а не по поведению: перехват
stderr из процесса теста ненадёжен, а `report` был именно текстом в трёх
модулях. Тест падает **списком** мест, чтобы четвёртая копия не завелась
незамеченной.
