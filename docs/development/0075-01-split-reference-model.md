# Задача 0075-01: разделить эталонную модель (разбор vs. компиляция)

> Фича: [../features/0075-lib-src-reference-model.md](../features/0075-lib-src-reference-model.md) · ADR: [../adr/0075-lib-src-reference-model.md](../adr/0075-lib-src-reference-model.md)

## Что было

`lib.rs::syntax_simple` стоял на модели-«всё сразу» (`SRC`), которая не
компилируется целью `c` (11 ошибок `cc`: `read_numeric` для `out`-порта с
бит-доступом; const `[bit;8]` печатается массивом, бит-доступ — числом). Тест мог
проверять лишь **строку** в `.c`.

## Что сделано

1. **`tests/reference_model_tests.rs`** — новый интеграционный тест
   `reference_model_compiles_and_translates_state_ref`: `SYNTH_SRC` →
   `compile_to_c` → **`cc -c`** (скип без `cc`) + строка `S(Ping) = End`.
2. **`SYNTH_SRC`** — компилируемая модель: композиция `(Ping | Pong) + Toggle`,
   `S(Ping) = End`, `ref`/`next` по условию/состоянию/`cond`. Без
   numeric-порт-бит-доступа и const-массив-бит-доступа (упираются в 0078).
3. **`syntax_simple` удалён** из `lib.rs`: он стоял на некомпилируемом `SRC`;
   инвариант `ref` теперь охраняет компиляционный тест. `lib.rs` уменьшен (1450 →
   1405, `module-size-baseline.txt` обновлён вниз).
4. **`parse_simple`** остаётся на полном `SRC` (покрытие парсера).

## Проверки

- **T1/A1:** `SYNTH_SRC` → `cc -c` rc=0.
- **T2/A2:** вывод содержит `S(Ping) = End` сравнением состояния.
- **T4/A3:** `parse_simple` разбирает полный `SRC`.
- **T5/A4:** `precheck.sh` зелёный.
