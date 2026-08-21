# Задача 0365-01: Распакованный массив у цели sv

> Фича: [../features/0365-sv-unpacked-array.md](../features/0365-sv-unpacked-array.md) · ADR: [../adr/0365-sv-unpacked-array.md](../adr/0365-sv-unpacked-array.md) · анализ: [../analyze/0365-sv-unpacked-array.md](../analyze/0365-sv-unpacked-array.md)

## Что было

Сброс массива без инициализатора печатался `a <= '0;` (verilator: «CONST is
not an unpacked array»), переменный индекс — полной шириной операнда
(`WIDTHTRUNC`, который гейт цели считает ошибкой). Обе формы принадлежат
**упакованному** значению, а массив скаляров у цели распакован.

## Что сделано

**`takt-lang/src/generator/sv/sv_array.rs`** (новый модуль) — два носителя:

- `reset_literal(ty)` — агрегат нулей (`'{8'd0, 8'd0}`), рекурсивный для
  вложенного массива; `None` для бит-вектора (правило 0078) и не-массива;
- `index_text(base_ty, index_ty, printed)` — сужение `W'(…)`, где `W` —
  ширина, которую требует размер (`ceil(log2(size))`, минимум 1); печатается
  **по нужде**;
- `array_type_expr` / `array_type_cond` — тип базы индексации со спуском по
  цепочке: у вложенной индексации база сама является индексацией.

**`sv_const.rs`** — ветвь `ExpressionNode::None` спрашивает носитель.
**`sv_expr.rs`** — оба печатника индексации (выражения и условия).
**`sv_stmt.rs`** — печать **цели записи**.

**`takt-sim/tests/data/eval/conformance_sv_array_index.takt`** +
два теста в `conformance_sv_array_tests.rs`: потактовая сверка значений и
прогон **линта цели**.

⚠️ **Форму сброса выбрал прогон, а не стандарт.** Каноничная `'{default: '0}`
принимается verilator и **отвергается yosys** («syntax error, unexpected
TOK_DEFAULT»); годной оказалась та же форма, которой печатается инициализатор
(0309).

⚠️ **Печатников индексации оказалось ТРИ.** После правки чтения линт всё ещё
отвергал модуль — на строке записи `slots[idx] := …`, которую печатает
`print_assign_target`. Сверка значений этого не показывала: тестбенч
собирается с `-Wno-fatal`.

## Проверки

```sh
cargo test --test conformance unpacked_array   # сверка значений + линт цели
cargo test --test conformance                  # 162 теста
cargo test --test targets                      # 391 тест
for f in examples/*.takt; do scripts/probe.sh -n 1 "$f"; done   # корпус чист
./scripts/precheck.sh
```

Мутации (все три пойманы): вернуть `'0` в сброс; снять сужение индекса; снять
сужение только в цели записи.
