# Тест-план 0319: приведение агрегата к массиву

> Фича: [../features/0319-array-cast-folding.md](../features/0319-array-cast-folding.md) · ADR: [../adr/0319-array-cast-folding.md](../adr/0319-array-cast-folding.md) · отчёт: [../reports/0319-array-cast-folding.md](../reports/0319-array-cast-folding.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | Приведение сворачивается, `Cast` исчезает | `aggregate_cast_is_folded` |
| П2 | Элементы приводятся правилом целого | `elements_use_the_integer_rule` |
| П3 | **Контроль:** агрегат без приведения не задет | `plain_aggregate_is_unchanged` |
| П4 | **Граница:** несовпадение длины — прежнее поведение | `length_mismatch_keeps_previous_behaviour` |
| П5 | Регрессия и предкоммит | `cargo test --all-features`, `./scripts/precheck.sh` |

## Почему тесты именно такие

- **П1 проверяет отсутствие `Cast` в дереве**, а не только значения: именно
  нераскрытое приведение и заставляло цель `c` отвечать `CC-017`.
- **П4 фиксирует границу словами:** длину агрегата сегодня не судит никто, и
  тест сторожит, что фича этого не изменила — иначе граница читалась бы как
  недоделка.
