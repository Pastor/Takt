# Задача 0305-01: `SE-084` на вызов внешней функции в инициализаторе

> Фича: [../features/0305-extern-call-in-initializer.md](../features/0305-extern-call-in-initializer.md) · ADR: [../adr/0305-extern-call-in-initializer.md](../adr/0305-extern-call-in-initializer.md) · анализ: [../analyze/0305-extern-call-in-initializer.md](../analyze/0305-extern-call-in-initializer.md)

## Что сделано

**`semantic/validate/init_undefined_read.rs`** — третий случай в **том же**
обходе, что `SE-099` (ячейка) и `SE-113` (порт):
`UndefinedRead::ExternCall(имя)` → `SE-084`. Текст называет функцию и штатный
путь: `always { x := f(); }`.

⚠️ **Форм внешней функции в ячейке две.** Инициализатор разрешается на стадии
2, тела функций строятся на стадии 5 — поэтому в снимке лежит
`Unresolved(FunctionDefine { external: true, … })`, а не `External`. Первая
редакция проверяла только вторую форму и **не ловила ничего**; поймано пробой,
а не чтением.

**`verification/verify.rs`** — причина `InitialValueUnknown` помечена
**защитной**: единственный вход, который её давал, теперь отвергается
семантикой. Проверено на трёх кандидатах (локальная функция, приведение,
параметр) — ни один её не даёт.

**Тесты:**

- `takt-lang/tests/semantic/extern_initializer_tests.rs` — отказ, рекурсия по
  выражению и **два контроля** (локальная функция; `extern` в теле);
- `unsupported_reason_tests` — тест A4 переписан: он проверяет **причину
  недостижимости** (вход отвергается с `SE-084`), а не молчит о ней.

**`docs/diagnostics/README.md`** — у `SE-084` отмечено второе место эмиссии.

## Проверка

```sh
cargo test --test semantic extern_initializer_tests::
cargo test --test verify
sh scripts/probe.sh -n 2 <модель с extern в инициализаторе>
```
