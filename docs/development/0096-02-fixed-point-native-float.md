# Задача 0096-02: цель `sv` — `float` → `q(m, n)` под `--float-as-q`, снятие `SV-003`

> Фича: [../features/0096-fixed-point-native-float.md](../features/0096-fixed-point-native-float.md) · ADR: [../adr/0096-fixed-point-native-float.md](../adr/0096-fixed-point-native-float.md) · анализ: [../analyze/0096-fixed-point-native-float.md](../analyze/0096-fixed-point-native-float.md) · тест-план: [../tests/0096-fixed-point-native-float.md](../tests/0096-fixed-point-native-float.md)

> **Задача 0096-02 реализована** (2026-07-19). Ниже — что сделано и почему так.

## Что было

Задача 0096-01 завела флаги `--float-as-q=m.n` / `--float-embedded`
(`GenerateOptions.float_as_q`, `float_embedded`), **не трогая кодоген**. Цель `sv`
на `float` давала `SV-003` (в синтезируемом RTL плавающей точки нет).

## Что сделано

### Трансформация `float → q(m, n)` (новый модуль `semantic/lower_float.rs`)

Единый мутирующий проход `lower_float_to_fixed(model, m, n)` над `ModelNode`
**перед** генерацией. Заменяет `TypeNode::Rational → Fixed{m,n}` и понижает
литерал-инициализатор `float`-переменной в `Number(repr)`, переиспользуя всю
Q-инфраструктуру 0061 (`type_node::lower_fixed_var`/`lower_fixed_literal`, кодоген
`sv_fixed`, эталон `eval::fixed`).

**Точки применения (0096-02):**

- `lib.rs::compile_to_sv` — при `options.float_as_q == Some((m, n))` (для `sv`
  флаг применяется **всегда**, не требует `--float-embedded`).
- Эталон-симулятор — тот же проход над моделью перед `build_unit` (conformance-тест
  `simulate_trace_float_q`). Так сверка идёт **внутри** Q-режима (ADR 0096, драйвер 2).

**Что понижается:**

| Позиция | Тип | Литерал |
|---|---|---|
| `variables`-map (объявления) | `Rational → Fixed` | понижается (инициализатор, `lower_fixed_var`) |
| `Rc`-ячейки переменных в выражениях/условиях | `Rational → Fixed` | — (тип нужен `fixed_format`/`extract_type`) |
| цель `Cast`, params/ret функций, локальные `var` | `Rational → Fixed` | — |
| литералы `Rational` в **телах** | — | **не трогаются** |

### Ключевые засады (и как сняты)

1. **Переменная в ДВУХ представлениях.** `VariableNode` — owned в
   `model.variables` (объявления/reset) **и** за `Rc<RefCell>` в
   `ExpressionNode::Variable`/`ConditionNode::Variable` (тип для `fixed_format`).
   Проход мутирует **оба**: иначе объявление `q`, а арифметика `float` (`*` без
   сдвига) — молча неверный код. Сторож — юнит-тест `variable_cell_in_body_is_retyped`.
2. **Идемпотентность.** `lower_fixed_var` понизила бы уже понижённый `Number(repr)`
   **ещё раз** (трактуя repr как целое → `repr · 2ⁿ`, вне диапазона → `SE-058`).
   Гейт: инициализатор понижается **только** если исходный тип был `Rational`.
   Тот же гейт защищает **явный** `q(m, n)` 0061 (его инициализатор уже понижён на
   этапе построения). Сторож — `transformation_is_idempotent`.
3. **Литерал в теле арифметики не понижается.** Симулятор `eval::fixed::binary`
   требует **оба** операнда `Fixed`; SV трактует `Number` как **сырой repr**.
   Понизь мы `x + 1.5` → `Fixed + Number(384)`, SV посчитал бы (repr 384 = 1.5),
   а симулятор упал бы (mismatch) — расхождение. Поэтому литералы тел оставлены
   `Rational` (громкая `SV-003`, а не тихий расчёт), тела пишутся на переменных
   (образец — `regulator`). Паритет с явным `q` 0061 (тот тоже не понижает тела).
4. **Смешение `float`/`q` уже запрещено.** `float`(Rational) в арифметике с явным
   `q`(Fixed) даёт `SE-059` **на этапе validate** (до трансформации) — поэтому
   после validate `float`-арифметика не смешана с `q`, и понижение безопасно.
5. **Рёбра `ref` — `Unresolved(ast::Condition)`** (инвариант проекта): кодоген
   разрешает их против уже понижённой `variables`-map, поэтому проход их не трогает
   (и обход `references[].object` создал бы цикл по состояниям).

## Проверки (тест-план)

| # | Проверка | Тест / способ | Статус |
|---|---|---|---|
| T1 | Корпус без флагов неизменен | `git diff examples/generated` пуст (флаг не задаётся при обычной генерации) | ✅ |
| T4/T7 | `sv`: `float`→q(8,8), синтез, **побитовая** сверка | `conformance_sv_tests::float_as_q_matches_generated_sv` (verilator; трасса = явная q-версия `-768,-384,-2,510`) | ✅ |
| T9 | Сторож направления: без флага — `SV-003` | `conformance_sv_tests::float_without_flag_is_sv003` | ✅ |
| — | Трансформация корректна и идемпотентна | 5 юнит-тестов `semantic::lower_float::tests` | ✅ |

**Зонд вывода** (`lamc -t sv --float-as-q=8.8` на фикстуре): `logic signed [15:0]`,
`* … >>> 8` (floor у `*`), `<<< 8 / …` (деление), инициализаторы-repr
(`x <= -384`, `two <= 512`, `tiny <= 1`) — структурно **идентично** явной
q(8,8)-модели, поэтому синтезируемость под yosys наследуется от уже-гейченного `q`
0061.

## Осталось (0096-03/04)

- **0096-03:** цели `c`/`rust`/`st` embedded-путь `float→q` при `--float-embedded`;
  двухрежимный эталон для них; `conformance_{c,rust,st}` в Q-режиме; сторож T9 для них.
- **0096-04:** корпусной пример-регулятор на `float` (синтезируем под `sv`, с
  yosys-гейтом в `precheck.sh` и сценарным контрактом), README, `CLAUDE.md`
  (ослабление 0042, A-7), отчёт — закрытие фичи.
