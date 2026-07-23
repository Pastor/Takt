# Тест-план фичи 0067: Rename и PsiReference для import в плагине IntelliJ

> Фича: [../features/0067-intellij-rename-psi-import.md](../features/0067-intellij-rename-psi-import.md) · ADR: [../adr/0067-intellij-rename-psi-import.md](../adr/0067-intellij-rename-psi-import.md) · анализ: [../analyze/0067-intellij-rename-psi-import.md](../analyze/0067-intellij-rename-psi-import.md)

## Область и цель

Проверить R5 (`PsiReference` для `import` + rename-on-move) и R3 (нативный rename
имён Lam) на текущем плоском PSI, дополненном хирургическим оборачиванием
одиночных токенов (ADR 0067, Option B), а также антидивергенцию и регресс
0022/0023. Автопроверки — headless `BasePlatformTestCase`; визуальное (диалог/
превью/арбитраж) — `runIde` (остаточные пункты, как A11 0022/0023).

## Проверки (условие → ожидаемый результат)

| # | Проверка | Предусловие | Ожидаемый результат | R/A |
|---|---|---|---|---|
| T1 | `resolve()` пути `import "P";` | файл `P` есть | `PsiFile` целевого файла | R5.1/A1 |
| T2 | `resolve()` `import "P" as X;` | — | `PsiFile` | R5.1/A1 |
| T3 | `resolve()` `import * as X from "P";` | — | `PsiFile` | R5.1/A1 |
| T4 | `resolve()` `import { A as C } from "P";` | — | `PsiFile` | R5.1/A1 |
| T5 | `renameElement(psiFile, "T.lam")` | путь ведёт к файлу | текст → `import "T.lam";` | R5.2/A2 |
| T6 | битый путь `import "нет.lam";` | файла нет | `resolve()==null`, без исключений | R5.3/A3 |
| T7 | строка в `formula "…"` | вне `import` | `getReferenceAtCaretPosition()==null` | R5.3/A3 |
| T8 | rename `model` из декларации | `model P{} … P` | все вхождения переименованы | R3.1/A4 |
| T9 | rename `model` из использования | — | декларация + использования | R3.1/A4 |
| T10 | rename `type`/`var`/вариант `enum`/порт/`fn`/алиас `import` | по виду | все вхождения в файле | R3.1/A4 |
| T11 | комментарий и строка с тем же именем | `//`, `"…"` | **не** изменены | R3.2/A5 |
| T12 | `NamesValidator` | ключевые слова / идентификаторы | `model`/`address` отвергнуты; `Producer`/`_x1` приняты; `1abc`/пустое отвергнуты | R3.3/A6 |
| T13 | round-trip PSI по корпусу | 195 `.lam` | `psi.node.text == исходник` для каждого | Анти-R/A7 |
| T14 | нет `PsiErrorElement` по корпусу | 195 `.lam` | ни одного | Анти-R/A7 |
| T15 | регресс 0022/0023 | прежние тесты | зелёные без правки ожиданий | Регресс/A8 |
| T16 | Plugin Verifier | IC-2024.3/2025.1/2025.2 | Compatible | A9 |
| T17 | Undo, конфликт имён, превью rename, арбитраж PSI↔LSP | `runIde` | штатное поведение | R3.3 (визуально) |

## Разбивка проверок по функциональности

Фича редакторная — затрагивает **только** плагин IntelliJ (`extensions/intellij-lam`).

| Функциональность | Статус |
|---|---|
| Плагин IntelliJ (Kotlin) — R5/R3/антидивергенция/регресс | ✅ (T1–T16) |
| Визуальное (диалог/превью/Undo/арбитраж) | ⬜ `runIde` (T17) |
| `grammar` (компилятор/LSP) | — (не затрагивается) |
| `simulation` | — (не затрагивается) |
| Версия языка | — (не меняется) |

<!-- Легенда: ✅ пройдено · ❌ провалено · ⬜ не проверялось · — не применимо -->

## Тестовые данные и окружение

- **Окружение:** JDK 21 (toolchain), IntelliJ Platform 2024.2.5 (IC), LSP4IJ
  0.20.1, Gradle 8.10.2. Прогон: `cd extensions/intellij-lam && ./gradlew
  --offline test` (+ `buildPlugin`, `verifyPlugin`).
- **Тест-классы:** `LamImportPsiReferenceTest` (R5), `LamRenameTest` (R3),
  `LamPsiCorpusTest` (антидивергенция); плюс прежние 0022/0023.
- **Корпус:** все `.lam` из `examples/` и `grammar/tests/data/` (195 файлов).
- **Примеры/контрпримеры** (правило 16): контрпример R5 — строка в `formula`
  (ссылки нет); контрпример R3 — имя в комментарии/строке (rename не задевает);
  контрпример валидатора — ключевое слово как имя (отвергнуто).
