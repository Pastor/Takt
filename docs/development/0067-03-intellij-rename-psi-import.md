# Задача 0067-03: антидивергентная сверка PSI + регресс

> Фича: [../features/0067-intellij-rename-psi-import.md](../features/0067-intellij-rename-psi-import.md) · ADR: [../adr/0067-intellij-rename-psi-import.md](../adr/0067-intellij-rename-psi-import.md) · анализ: [../analyze/0067-intellij-rename-psi-import.md](../analyze/0067-intellij-rename-psi-import.md)

## Что было

Оборачивание одиночных токенов в композиты (`IMPORT_PATH`/`NAME_DECL`/`NAME_REF`,
0067-01/02) меняет форму PSI-дерева. Нужен сторож, что оно не теряет и не
переставляет текст исходника — по образцу `format_tests.rs` (форматтер по всему
корпусу).

## Что сделано

- **`LamPsiCorpusTest`** (`BasePlatformTestCase`): прогоняет парсер по **всем**
  `.lam` из `examples/` и `grammar/tests/data/` (195 файлов) и требует
  (а) **round-trip байт-в-байт** — `psi.node.text == исходник` (текст корневого
  AST-узла = конкатенация листьев в порядке дерева; потеря/перестановка токена
  изменила бы его); (б) **отсутствие `PsiErrorElement`**.
- Корень репозитория ищется подъёмом от рабочего каталога (как `LamKeywordSyncTest`).

⚠️ **Сверка вердикта с оракулом `lamc` (план 0040) для Option B вырождена:**
парсер тотальный (ADR 0023 — «разбор всегда успешен», принимает любой поток
токенов) и `PsiErrorElement` не порождает — синтаксической валидности плагин не
заявляет (это работа `lamc`/LSP, 0038). Реальный сторож — round-trip; отдельный
файл-оракул не заводился (его сверка была бы тавтологией `no-error == no-error`).
Новый узел, теряющий текст, завалит `psi.node.text`-проверку.

## Проверки

`cd extensions/intellij-lam && ./gradlew --offline test` — **зелёный, 80 тестов,
0 падений**:
- `LamPsiCorpusTest` — round-trip + отсутствие ошибок на 195 файлах корпуса;
- регресс 0022/0023 (Go to Declaration, import-навигация, подсветка, лексер,
  keyword-sync, semantic tokens) — **без правки ожиданий**;
- R5 (`LamImportPsiReferenceTest`, 7) и R3 (`LamRenameTest`, 11) — зелены.

Plugin Verifier (A9) / арбитраж PSI↔LSP4IJ — визуальная проверка (`runIde`) и не
в CI (плагин собирается только локально; `ci.yml` — `cargo`), как остаточные
пункты 0022/0023.
