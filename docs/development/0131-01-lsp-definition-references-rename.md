# Задача 0131-01: `textDocument/definition` как алиас `declaration`

> Фича: [../features/0131-lsp-definition-references-rename.md](../features/0131-lsp-definition-references-rename.md) · ADR: [../adr/0131-lsp-definition-references-rename.md](../adr/0131-lsp-definition-references-rename.md) · анализ: [../analyze/0131-lsp-definition-references-rename.md](../analyze/0131-lsp-definition-references-rename.md)

## Что было

Сервер объявлял `declarationProvider`, но не `definitionProvider`, и метод
`textDocument/definition` возвращал «метод не найден». В редакторах, где F12 идёт
через `definition` (VS Code — основной случай), переход **не работал вовсе**,
хотя вся его логика в проекте есть с фичи 0056.

Список возможностей строился литералом внутри `bin/takt_lsp.rs`, то есть «что
объявлено» никакой тест не видел: бинарник тестами не покрывается.

## Что сделано

1. **Список возможностей вынесен в библиотеку** — `lsp::server_capabilities()`
   (`takt-lang/src/lsp/capabilities.rs`, новый модуль). Бинарник теперь зовёт
   функцию, а не строит структуру сам. Вынос сделан **ради проверяемости**, а не
   ради размера: тот же довод, по которому фича 0072 вынесла в библиотеку разбор
   `initializationOptions`.
2. **Добавлен `definition_provider`** рядом с `declaration_provider`.
3. **Один обработчик на оба метода:** ветка `match` в `handle_request` стала
   `GotoDeclaration::METHOD | GotoDefinition::METHOD`. Типы параметров и ответа у
   методов совпадают (`GotoDeclarationParams = GotoDefinitionParams`,
   `GotoDeclarationResponse = GotoDefinitionResponse` в `lsp-types` 0.97),
   поэтому объединение бесплатно и не требует второго разбора.

⚠️ **Почему именно одна ветка, а не два одинаковых обработчика.** Разъехаться
`definition` и `declaration` могут ровно одним способом — раздвоением кода;
одинаковый текст в двух местах живёт до первой правки одного из них. Прецеденты
проекта на этом уже стоят: `is_state_of` ↔ `state_of_model`, `function_needs` ↔
печатник. Сторож в тестах ловит именно раздвоение.

**Что НЕ менялось:** ни `goto_declaration_at`, ни индекс, ни семантика. Логика
перехода прежняя — расширился только список входов, через которые её зовут.
Правка аддитивна (правило 11): прежние возможности объявляются в том же составе,
их тесты не тронуты.

## Проверки

```sh
cargo build --features lsp --bin takt-lsp
cargo test --features lsp -- --test-threads=1     # все зелёные (989 в lib + интеграционные)
cargo clippy --all-features --all-targets         # 0 предупреждений
./scripts/check-module-size.sh                    # 0; новых нарушителей нет
```

Новый файл тестов — `takt-lang/tests/lsp_definition_tests.rs` (4 теста):

| Тест | Что доказывает | Критерий |
|---|---|---|
| `both_providers_are_advertised` | объявлены обе возможности | A1 |
| `server_handles_both_methods_in_one_branch` | у методов **один** обработчик; второй `GotoDefinition::METHOD` в файле = красный тест | A2 |
| `shared_entry_resolves_every_reference_kind` | общий вход разрешает переход по переменной в условии, по имени состояния и по имени модели | A2 |
| `cursor_beyond_text_yields_nothing` | позиция вне текста не роняет сервер | — |

Плюс два юнит-теста в `capabilities.rs`: обе возможности перехода объявлены и
прежний состав возможностей не потерян при выносе.

⚠️ Замечено при написании тестов: курсор **на ключевом слове `model`** даёт
переход к самой модели — её диапазон покрывает всё тело. Это поведение с фичи
0056, задача его не меняет; сторож «нет перехода» поэтому стоит на позиции **за
пределами текста**, а не на ключевом слове.
