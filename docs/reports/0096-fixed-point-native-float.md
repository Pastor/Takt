# Отчёт о тестировании фичи 0096: прозрачный `float` через глобальную Q-точность

> Фича: [../features/0096-fixed-point-native-float.md](../features/0096-fixed-point-native-float.md) · ADR: [../adr/0096-fixed-point-native-float.md](../adr/0096-fixed-point-native-float.md) · тест-план: [../tests/0096-fixed-point-native-float.md](../tests/0096-fixed-point-native-float.md)

- **Дата:** 2026-07-19
- **Окружение:** macOS (darwin 25.5.0); cc/cmake/ninja, rustc/clippy, MatIEC
  `iec2c`, Verilator 5.050, yosys — все гейты доступны.
- **Вердикт:** **готово. Фича закрыта.** Автор пишет `float`; представление
  выбирают флаги генерации `--float-as-q=m.n` / `--float-embedded`. Цель `sv`
  синтезирует `float` (снятие `SV-003`); `c`/`rust`/`st` — native по умолчанию,
  целочисленный Q по флагу. `precheck.sh` — зелёный (EXIT 0).

## Задачи

| Задача | Объём | Коммит |
|---|---|---|
| 0096-01 | CLI-флаги `--float-as-q` / `--float-embedded` (инфраструктура) | `8412c69` |
| 0096-02 | Цель `sv`: `float → q(m, n)`, снятие `SV-003` | `f8f1649` |
| 0096-03 | Цели `c`/`rust`/`st` embedded-путь при `--float-embedded` | `9471276` |
| 0096-04 | Пример-регулятор на `float` + README + `CLAUDE.md` + отчёт | (этот) |

## Сверка с критериями приёмки (ADR)

| # | Критерий | Результат | Способ |
|---|---|---|---|
| A1 | Без флагов корпус неизменен | ✅ | `git diff examples/generated` пуст (флаг не задаётся при обычной генерации); гейт воспроизводимости precheck |
| A2 | `sv`+`--float-as-q=10.22`: `float`→q, синтез, `SV-003` не сработал | ✅ | `conformance_sv_tests::float_as_q_matches_generated_sv` (q(8,8), verilator); зонд: `logic signed [15:0]`, `>>> 8` |
| A3 | `c`/`rust`/`st`+`--float-as-q` **без** embedded — native | ✅ | `float_as_q_without_embedded_is_native_{c,rust,st}` (`double`/`f64`/`LREAL`) |
| A4 | `+--float-embedded` — Q-путь, побитово | ✅ | `float_embedded_q_matches_generated_c` (runtime, cc); `float_embedded_matches_explicit_q_{rust,st}` (byte-equality) |
| A5 | Симулятор двухрежимен, `sv`-сверка побитовая | ✅ | `float_native_and_q_modes_differ` (Real ≠ Fixed); conformance_sv (repr) |
| A6 | Явный `q(m, n)` (0061) не задет | ✅ | сверки 0061 зелены; гейт «исходный тип — Rational» не трогает `Fixed` |
| A7 | `CLAUDE.md` фиксирует ослабление 0042 | ✅ | заметка 0096 (A-7): `--float-*` — осознанное исключение, оба режима сверяются |
| A8 | README документирует прозрачный `float` | ✅ | README §«Прозрачный `float`» (таблица цель×флаг, примеры, ⚠️ native ≠ Q) |

## Реализация (ключевое)

**Единая мутирующая трансформация `float → q(m, n)`** над `ModelNode` **перед**
генерацией (`semantic::lower_float::lower_float_to_fixed`), применяемая к цели
**и** эталону-симулятору (сверка ВНУТРИ режима). Переиспользует всю
Q-инфраструктуру 0061 (`sv_fixed`/`c_expr::fixed`/`rust_fixed`/`st_fixed`,
`eval::fixed`, `lower_fixed_var`). Точки применения — общий помощник
`lib.rs::apply_float_lowering` с embedded-гейтом (`sv` — всегда, `c`/`c-hal`/`st`/
`st-at`/`rust` — при `--float-embedded`).

**Три засады (сняты, со сторожами):**
1. `VariableNode` в **двух** представлениях (owned map + `Rc<RefCell>` в
   выражениях) — мутируются **оба** `ty` (`variable_cell_in_body_is_retyped`).
2. Литерал понижается только в **инициализаторе** (не в телах: `Fixed + Number`
   разошёлся бы с `eval::fixed::binary`, требующим два `Fixed`).
3. **Идемпотентность** — гейт «исходный тип был `Rational`»: иначе `lower_fixed_var`
   понизила бы `Number(repr)` повторно (`transformation_is_idempotent`).

**Двухрежимный эталон (A-1)** достигнут **той же** трансформацией (`eval::fixed`
не тронут): Q = проход применён, native = нет. Риск рассинхрона снят
конструктивно.

## Пример

`examples/float_regulator.lam` — пропорциональный регулятор на `float` (тот же,
что `regulator.lam`, но прозрачный `float`). Гейты: `c`/`plantuml`/`st`/`rust`
(native), `sv` → `SV-003` (закономерно, не в `SV_TRANSLATABLE`), симулятор
(`examples_scenario_tests`: `Adjust → Settled → Done`, budget 50). Float-специфику
`sv` (float→q) покрывает `conformance_sv` (q-двойник — `regulator.lam` уже под
yosys-гейтом).

## Отклонения от плана

- **Литералы тел не понижаются** (план допускал «Rational→Number везде»): у
  арифметики симулятора `binary` требует два `Fixed`, `Fixed + Number` разошёлся
  бы с SV. Оставлено как у явного `q` 0061 — тела на переменных.
- **rust/st сверка — byte-equality, а не runtime:** поля цели `rust` приватны, а
  q-**выходной порт** rust даёт `RS-016`, поэтому Q-модель без портов рантайм
  ненаблюдаема. Byte-equality с q-двойником доказывает, что float→q даёт **ровно**
  проверенный 0061 q-кодоген. У `c` (поля публичны) — полная потактовая сверка.
- **Пример — native (не sv):** решение заказчика (два примера) — float-регулятор
  на c/rust/st, sv покрывает существующий q-регулятор + conformance.

## Итог

Все критерии A1–A8 выполнены; `precheck.sh` зелёный. Версия языка **не менялась**
(флаги — свойство генерации; `float`/`q` синтаксически прежние). Фича закрыта.
