# Задача 0293-03: Структуры в цели `rust`

> Фича: [../features/0293-struct-in-st-rust.md](../features/0293-struct-in-st-rust.md) · ADR: [../adr/0293-struct-in-st-rust.md](../adr/0293-struct-in-st-rust.md) · анализ: [../analyze/0293-struct-in-st-rust.md](../analyze/0293-struct-in-st-rust.md)

## Что было

`RS-011`: «доступ к члену '.kp': поля структур в цели rust пока не
транслируются». Структура объявлялась переменной, но дальше не шла.

## Что сделано

- `rust_decl::emit_structs` печатает `pub struct` с полями;
- `rust_bit::field_access` — доступ `<база>.<поле>` (прежде `member_index`
  отвечал отказом на `Member::Identifier`); то же в печатнике условий;
- `rust_struct::struct_literal` — агрегат `Gains { kp: 2, ki: 3 }`: без типа
  приёмника общий печатник выражений давал массив `[2, 3]`, то есть невалидный
  Rust.

⚠️ **`Eq` не выводится:** у поля `float` (`f64`) его нет, и вывод корпуса
перестал бы компилироваться. Сравнение структур язык и так запрещает (`SE-059`).

⚠️ **Запись в поле делает переменную изменяемой:** `collect_assigned` учитывает
`r.output := …`, иначе `rustc` отвечает `E0594` («cannot assign … not declared
as mutable»). Случай не возникал, пока поля не переводились, — и всплыл на
корпусе (`pid_heater`).

## Проверки

```sh
cargo test --test conformance struct_fields_match_generated_rust
./scripts/precheck.sh
```

Гейт цели (`rustc` + `clippy -D warnings`) на корпусе зелёный; потактовая сверка
через порт-наблюдатель совпадает с эталоном.
