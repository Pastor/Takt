# Разработка 0337-01: неиспользуемый параметр функции

> Фича: [../features/0337-unused-function-parameter.md](../features/0337-unused-function-parameter.md) · ADR: [../adr/0337-unused-function-parameter.md](../adr/0337-unused-function-parameter.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/c/c_decl.rs` | заглушка печатается и **объявленным** параметрам, не только `model` |
| `takt-lang/src/generator/rust/rust_unused.rs` | **новый**: признак и идиома `let _ = v;` |
| `takt-lang/src/generator/rust/rust_func.rs` | тело печатается в буфер (`Printer::fork`), заглушки — перед ним |
| `takt-lang/src/generator/sv/sv_unused.rs` | **новый**: признак и поглощение редукцией |
| `takt-lang/src/generator/sv/sv_fsm.rs` | то же для тела функции цели `sv` |
| `takt-lang/tests/targets/unused_param_targets_tests.rs` | три цели: текст **и** прогон `cc -Werror`, `rustc -D warnings`, `verilator -Wall` |

## Проверено

- `yosys` синтезирует модуль с заглушкой (форма выбрана пробой **обоих**
  инструментов).
- Контрольная функция (`echo`, параметром пользуется) заглушки не получает.
- Вывод корпуса не изменился.
