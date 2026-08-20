# Разработка 0314-01: `CC-024` в цели `c`

> Фича: [../features/0314-c-dropped-builtin-warning.md](../features/0314-c-dropped-builtin-warning.md) · ADR: [../adr/0314-c-dropped-builtin-warning.md](../adr/0314-c-dropped-builtin-warning.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/c/c_map.rs` | накопитель `warnings: RefCell<Vec<Diagnostic>>`, методы `warn`/`take_warnings` |
| `takt-lang/src/generator/c/c_expr/stmt.rs` | пропуск встроенного вызова сопровождается `CC-024`; имя функции берётся из узла |
| `takt-lang/src/generator/c/mod.rs` | `generate` возвращает накопленное (канал 0168 больше не пуст) |
| `takt-lang/tests/targets/c_builtin_dropped_tests.rs` | три проверки: предмет, счёт, контроль |
| `docs/diagnostics/README.md`, `book/src/appendix-errors/index.typ` | `CC-024` зарегистрирован |

## Проверено

- `cargo test --test targets c_builtin_dropped` — 3/3.
- Прогон `taktc compile -t c`: предупреждение печатается с позицией оператора,
  под `--quiet` — молчит.
- `cargo test --all-features` — провалов нет.
