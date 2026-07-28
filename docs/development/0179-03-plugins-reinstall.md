# Задача 0179-03: Пересборка и переустановка плагинов

> Фича: [../features/0179-repo-url-cleanup.md](../features/0179-repo-url-cleanup.md) · ADR: [../adr/0179-repo-url-cleanup.md](../adr/0179-repo-url-cleanup.md) · анализ: [../analyze/0179-repo-url-cleanup.md](../analyze/0179-repo-url-cleanup.md)

## Что было

Старый адрес лежал не только в исходниках, но и в **уже установленных**
артефактах: в дистрибутиве плагина RustRover (`<vendor url>` из `plugin.xml`) и
в `index.json` Zed — туда его записал `install.sh` при установке 2026-07-29.
Правка исходников такие копии не обновляет (A-3 ADR).

## Что сделано

- **IntelliJ:** `extensions/install-rustrover-plugin.sh` — пересборка
  (`buildPlugin`) и переустановка в `RustRover2026.2`.
  ⚠️ Сборка требует **JDK 21** (`JAVA_HOME` на `openjdk-21.0.2`): системная java
  — 25.0.2, под ней Kotlin 2.0.21 падает с `IllegalArgumentException: 25.0.2`.
  Это подтверждает кандидат [0159](../features/0159-intellij-jdk21-build.md)
  живой пробой.
- **Zed:** `extensions/zed-takt/scripts/install.sh` — переустановка языковых
  файлов и перезапись манифеста в `index.json`.

## Проверки

Греп по **установленным** артефактам (критерий A8):

| Артефакт | Старый адрес | Новый адрес |
|---|---|---|
| `…/RustRover2026.2/plugins/intellij-takt` | не найден ✅ | — |
| `…/Zed/extensions/index.json` | не найден ✅ | `Pastor/Takt` ✅ |
| `…/Zed/extensions/installed/takt/extension.toml` | не найден ✅ | — |

Тесты плагина IntelliJ прогонялись отдельно (90 тестов, 0 падений), в том числе
`TaktKeywordSyncTest` — сторож ключевых слов из фичи 0178.

⚠️ **Уточнение к живому контексту:** `CLAUDE.md` утверждает, что плагин
собирается «только локально (`./gradlew --offline test`)». Проба показала, что
`--offline` **не работает на пустом кеше** — `foojay-resolver-convention` не
скачан, и первый прогон требует сети. Офлайн годится лишь для повторных сборок.

⚠️ **Переустановка не автоматизирована и в `precheck.sh` не входит:** плагины
собираются вне толчейна (Gradle + JDK 21). Пользователю нужно **перезапустить**
RustRover и Zed — плагины подхватываются при старте.
