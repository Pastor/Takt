# Задача 0253-01: Переименование хелперов фиксированной точки в целях `c` и `st`

> Фича: [../features/0253-legacy-names-in-generated-code.md](../features/0253-legacy-names-in-generated-code.md) · ADR: [../adr/0253-legacy-names-in-generated-code.md](../adr/0253-legacy-names-in-generated-code.md) · анализ: [../analyze/0253-legacy-names-in-generated-code.md](../analyze/0253-legacy-names-in-generated-code.md)

## Что было

Цель `c` печатала пять хелперов фиксированной точки — `lam_q_floordiv`,
`lam_q_mul`, `lam_q_div`, `lam_q_sat`, `lam_q_wrap`; цель `st` — три:
`LAM_Q_FLOORDIV`, `LAM_Q_WRAP`, `LAM_Q_SAT`. Префикс `lam_` остался от
упразднённого имени языка (`Lam`, снято фичей
[0100](../features/0100-language-rename-takt.md)) и уезжал **в прошивку
пользователя**: имена видны в снапшотах `examples/generated/c/regulator.c` и
`examples/generated/st/regulator.st`, в текстах сообщений потактовых сверок и
в любом файле, который инструмент порождает для Q-арифметики.

## Что сделано

Префикс сменён на `takt_q_` (цель `c`) и `TAKT_Q_` (цель `st`) — **и только
он**: тела хелперов, сигнатуры, условия вставки и порядок определений прежние
(R2 анализа).

- `takt-lang/src/generator/c/c_expr/fixed.rs` — 31 место: эмиссия вызовов,
  тексты определений, признаки вставки (`source.contains("takt_q_mul(")`),
  имена Rust-констант `TAKT_Q_*` и док-комментарии.
- `takt-lang/src/generator/st/st_fixed.rs` — 30 мест: то же для цели `st`,
  включая тела `FUNCTION TAKT_Q_FLOORDIV/WRAP/SAT` и условия вставки.
- `takt-lang/src/generator/st/mod.rs` — комментарий о вставке хелпера.
- `takt-sim/tests/conformance/conformance_st_tests.rs`,
  `…/conformance_st_tests/fixed_sat.rs` — тексты сообщений об отказе `iec2c` и
  комментарии (9 мест).
- `examples/generated/{c/regulator.c,st/regulator.st}` — снапшоты
  перегенерированы штатной командой.

**Rust-константы переименованы вместе с содержимым намеренно.** `const
LAM_Q_SAT: &str = "…lam_q_sat…"` — идентификатор инструмента, а не вывода
(формально предмет фичи [0254](../features/0254-legacy-names-internal-identifiers.md)),
но он **называет** порождаемый хелпер: разъехавшись с ним, он стал бы врать
читателю прямо в месте правки.

**Обратная функциональность (правило 11).** Прочие цели — `rust`, `sv`,
`sv-mmio`, `plantuml` — **н/п**: хелперов они не эмитят (`rust` печатает
`wrapping_*` и пару сдвигов, `sv` считает по ширине `W`), проверено грепом.
Цели `c-hal` и `st-at` наследуют печать выражений у `c`/`st` и получают новое
имя тем же изменением.

## Проверки

```sh
cargo build --bin taktc
target/precheck/debug/taktc compile examples/regulator.takt    -o examples/generated/c
target/precheck/debug/taktc compile examples/regulator.takt -t st -o examples/generated/st
git diff examples/generated          # только смена префикса — A6
cargo test --all-features            # 3304 теста, 0 провалов — R3
```

- **R1/A1:** `grep -rI 'lam_q_\|LAM_Q_'` по рабочим файлам — пусто.
- **R3/A5:** потактовые сверки (`conformance`, 117 тестов) зелены **без правки
  ожидаемых значений**: переименование поведения не изменило.
- **R4/A3, A4:** сверки цели `st` (`fixed_sat`, `fixed_wrap_to_width`,
  `per_tick`) реально прогоняют `iec2c` и `cc` — сообщения «сверка пропущена»
  в выводе нет; корпус собирается шагом `precheck.sh`.
- **A6:** дифф снапшотов — 8 строк на два файла, все до одной суть смена
  префикса.
