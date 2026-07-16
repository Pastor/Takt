# Задача 0049-04: Подкоманда lamc verify

> Фича: [../features/0049-model-checking-ltl.md](../features/0049-model-checking-ltl.md) · ADR: [../adr/0049-model-checking-ltl.md](../adr/0049-model-checking-ltl.md) · анализ: [../analyze/0049-model-checking-ltl.md](../analyze/0049-model-checking-ltl.md)

## Что было

CLI `grammar/src/bin/lamc.rs`: подкоманды **`compile`** и **`fmt`**,
диспетчеризация вручную в `main()` (`lamc.rs:532`; `if args[1] == "fmt"`
`:540`, иначе `compile` `:552`). `parse_compile_args` (`:171`), `print_usage`
(`:489`). Подкоманды `verify` **нет**.

## Что сделано

> **Планируется (разработка не начата).** План по ADR 0049 (R8).

1. Ветка `verify` в `main()` (по образцу `fmt`, `lamc.rs:540-550`):
   `lamc verify <file> [-I <dirs>] [--property "φ"]`.
2. `parse_verify_args(args) -> VerifyOptions { file, includes, property: Option<String> }`.
3. Логика: `parse` + `construct_model` файла. Формулы для проверки:
   - `--property "φ"` — разобрать строку через **`ltl_ast_to_semantic`** (не
     `parse_ltl` — тот тестовый, паникует, атом = 1 символ); при её отсутствии
     как публичного парсера строки — распарсить как `.lam`-фрагмент `: [LTL] φ;`
     и извлечь `Formula::LTL`;
   - без `--property` — все `Formula::LTL` модели (`ltl_check`-обход) —
     проверяются по очереди.
4. Для каждой формулы — `grammar::verify_model(&model, &phi)`; печать вердикта:
   - `Holds` → «свойство держится»;
   - `Violated(lasso)` → «свойство НАРУШЕНО», путь-лассо в именах состояний,
     пометка «абстракция управления»;
   - `Unsupported(names)` → «атом не является именем состояния (в абстракции
     управления не поддержан)».
5. Код возврата: `0` — все держатся; `≠0` — есть нарушение/неподдержанное.
6. Строка в `print_usage` (`lamc.rs:489`).

## Проверки

- Проба CLI: `lamc verify verify_holds.lam` → «держится» (код 0);
  `lamc verify verify_fails.lam` → «нарушено» + лассо (код ≠0).
- `lamc verify examples/*.lam` завершается (A5).
- Соответствие анализу: R8; критерии A1, A5.
