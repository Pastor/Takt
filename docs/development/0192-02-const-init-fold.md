# Задача 0192-02: тесты значений и перенос тестовых площадок

> Фича: [../features/0192-const-init-fold.md](../features/0192-const-init-fold.md) · ADR: [../adr/0192-const-init-fold.md](../adr/0192-const-init-fold.md) · анализ: [../analyze/0192-const-init-fold.md](../analyze/0192-const-init-fold.md)

## Что было

Свёртки не было, поэтому и сторожей у неё не было. Зато **28 существующих
тестов** использовали инициализатор `var` как *площадку* для проверки
построения и печати выражений (`var x: u8 := n + m * 2;` → проверка, что узел
`Add`, что ST печатает `n + (m * 2)`).

## Что сделано

**Новые сторожа** — `takt-lang/tests/const_init_fold_tests.rs`:

| Тест | Что доказывает |
|---|---|
| `arithmetic_initializer_is_folded_for_every_target` | `1 + 2` → `3` в выводе целей `c`/`rust`/`st`, и выражения `1 + 2` в выводе **нет** |
| `st_no_longer_drops_the_initializer` | `probe : USINT := 3;` — цель `st` больше не теряет значение молча |
| `initializer_may_reference_a_variable_declared_above` | `var base := 5; var probe := base + 1;` → `6` у всех целей (решение заказчика, Option D) |
| `folding_runs_after_type_inference` | `var flag: bit := false; var probe := flag;` → тип `probe` выводится из `flag`, а не из литерала |

⚠️ Последний тест — сторож против регресса, **который уже случался** при
разработке: свёртка до вывода типов давала `bool` вместо `bit`.

**Площадки перенесены, а не подогнаны.** 28 тестов (`semantic/expression/tests.rs`
и `generator/st/st_expr.rs`) наблюдали выражение в инициализаторе — там его
больше нет **по замыслу** ADR. Площадка перенесена в **тело блока**
(`always { x := <выражение>; }`): утверждения остались дословно теми же, сторожа
сохранились и перестали зависеть от того, сворачивается инициализатор или нет.

⚠️ Удалять их было нельзя: они сторожат построение и печать узлов выражения —
предмет, к свёртке отношения не имеющий.

## Проверки

```sh
cargo test -p takt-lang --test const_init_fold_tests -- --test-threads=1
cargo test -p takt-lang --lib -- --test-threads=1
```

- Новые тесты: 4 зелёных.
- Перенесённые площадки: `semantic::expression::tests` — 74 зелёных,
  `generator::st::st_expr` — 15 зелёных.
