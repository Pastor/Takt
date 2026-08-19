# Задача 0295-01: Хвостовой комментарий тела и его хозяин

> Фича: [../features/0295-format-element-comment-binding.md](../features/0295-format-element-comment-binding.md) · ADR: [../adr/0295-format-element-comment-binding.md](../adr/0295-format-element-comment-binding.md) · анализ: [../analyze/0295-format-element-comment-binding.md](../analyze/0295-format-element-comment-binding.md)

## Что было

Печать состояния и вложенной модели закрывала тело парой `out.down();
out.line("}")` — без выдачи хвостовых комментариев. Комментарий последней
строкой тела терял хозяина и всплывал **в конец файла**; форматтер пишет на
месте, поэтому страдал исходник автора.

Тот же класс уже правился трижды — и каждый раз **в своей ветке** (0198,
0197-01, 0198-01), общего носителя не появилось.

## Что сделано

Заведена функция `close_body(out, loc)`:

```rust
if let Some((_, end)) = comments::span(loc) {
    out.comments_before(end.saturating_sub(1));
}
out.down();
out.node_line(loc, "}");
```

Её зовут `print_state` и `print_nested_model` (обе получили `loc` из
`print_element_inner`). `print_enum`/`print_struct` делали то же самое своими
руками — их поведение не изменилось.

⚠️ **Правило теперь не в дисциплине, а в сторожe:** тест
`format_comment_binding_tests::closing_brace_goes_through_one_function` грепает
`format/mod.rs` и падает **списком мест**, где закрывающая скобка печатается в
обход. Новая конструкция с телом иначе завелась бы с прежним дефектом, и
заметить это было бы нечем (комментариев внутри таких записей в корпусе нет).

## Проверки

```sh
cargo test --test syntax format_comment_binding   # 4 теста
cargo test --all-features                         # 3260 тестов
./scripts/precheck.sh                             # включая канон examples/
```

**Мутация** «вернуть прежнее закрытие в `print_state`» валит **три** теста из
четырёх (T1, T3, T4).

⚠️ **Первая редакция T1 мутацию не ловила:** она проверяла порядок строк, а
уезжающие комментарии сохраняют взаимный порядок. Переписана на проверку
**отступа** — признака того, чьё это тело.
