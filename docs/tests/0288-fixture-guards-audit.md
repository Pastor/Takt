# Тест-план фичи 0288: Сторожа фикстур

> Фича: [../features/0288-fixture-guards-audit.md](../features/0288-fixture-guards-audit.md) · анализ: [../analyze/0288-fixture-guards-audit.md](../analyze/0288-fixture-guards-audit.md) · отчёт: [../reports/0288-fixture-guards-audit.md](../reports/0288-fixture-guards-audit.md)

## Условия проверок

| # | Условие | Как проверяется | Ожидаемый результат |
|---|---|---|---|
| П1 | Тип из функции | `type_is_inferred_from_function_return` | `Bool` |
| П2 | Локальная переменная блока | `local_variable_of_block_is_resolved` | узел есть, тело разрешено |
| П3 | Варианты перечисления | `enum_fixture_carries_its_variants` | все четыре |
| П4 | Индексация массива | `array_access_fixture_has_subscript` | узел `ArraySubscript` |
| П5 | Именованные блоки | `named_blocks_fixture_carries_all_three` | `Enter`, `Exit`, `Always` |
| П6 | F1: новый слабый сторож | проба (временный тест) | отказ, тест назван |
| П7 | F2: протухшая запись | `--self-test` | находка `F2` |
| П8 | **Контроль:** согласованное дерево | прогон гейта | «новых нет» |
| П9 | Предкоммит | `./scripts/precheck.sh` | код 0 |

## Мутационные проверки

- **М1.** Ослабить сторож `type_is_inferred_from_function_return` до
  `build(...)` → гейт обязан дать `F1`.
- **М2.** Удалить запись из реестра, не усилив тест → гейт обязан дать `F1` на
  него же.
