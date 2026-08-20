# Разработка 0339-01: шаблон отказа цели `sv`

> Фича: [../features/0339-sv-refusal-template.md](../features/0339-sv-refusal-template.md) · ADR: [../adr/0339-sv-refusal-template.md](../adr/0339-sv-refusal-template.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/sv/sv_expr.rs` | шаблон стал **префиксом**; объяснение политики — заметкой |
| `takt-lang/src/generator/sv/sv_fsm.rs` | копия шаблона снята (была байт-в-байт) |
| `takt-lang/src/generator/sv/sv_stmt.rs` | копия снята (говорила «пропустить **оператор**») |
| `takt-lang/src/generator/sv/sv_type.rs` | `sv002_type` строит текст и зовёт общий носитель |
| `takt-lang/src/generator/sv/sv_mmio.rs` | `sv002_width` — то же |
| `takt-lang/tests/targets/sv_refusal_text_tests.rs` | текст читается как предложение; носитель один (греп, падает списком) |

## Проверено

- Прогон `taktc` на входе с досрочным возвратом: сообщение и заметка печатаются
  раздельно.
- `cargo test --test targets` — 336 passed.
