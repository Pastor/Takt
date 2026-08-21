# Задача 0357-01: Умолчание общей переменной

- **Фича:** [../features/0357-rust-shared-default-value.md](../features/0357-rust-shared-default-value.md)
- **ADR:** [../adr/0357-rust-shared-default-value.md](../adr/0357-rust-shared-default-value.md)
- **Анализ:** [../analyze/0357-rust-shared-default-value.md](../analyze/0357-rust-shared-default-value.md)
- **Дата:** 2026-08-21

## Что сделано

`emit_shared_new_block`: вместо строки `"Default::default()"` вызывается
`rust_decl::default_value(ty, &root.borrow())`. Тип берётся из `union`, корень —
у карты.

Замер зондом и его дата записаны в док-комментарий функции: следующая фича не
должна принять защитную ветвь за живую.

## Ловушки

⚠️ **Соблазн удалить ветвь.** Замер говорит «недостижимо сегодня» — этого мало
для удаления: `expect` превратил бы недостижимое в панику инструмента, который
обязан отвечать диагностикой.

⚠️ **Корня может не быть** (`root_model_node()` возвращает `Option`) — там
сохранена прежняя строка: умолчание строить не из чего.
