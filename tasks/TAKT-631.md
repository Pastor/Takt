---
okf_version: 0.2
format: 2
type: Task
key: TAKT-631
list: ТАКТ
title: "[0022-01] открытый верхний диапазон совместимости IDE"
status: done
priority: none
author: pastor
model: gpt-6
links:
  part:
  - TAKT-22
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:09:37Z
published: 2026-11-11T07:09:37Z
created: 2026-11-11T07:09:37Z
updated: 2026-11-11T07:09:37Z
anal:
  project: 22
  id: 1685
  version: 1
checksum: sha256:4a4bb0adb57e6ef1e54b1c9f6b964b0669434765901b419f5668be17f940d9b2
---

Источник: [docs/fixes/0022-01-untilbuild-open-range.md](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/fixes/0022-01-untilbuild-open-range.md)

# Исправление: открытый верхний диапазон совместимости IDE

> Фича: [../features/0022-intellij-syntax-highlight.md](../features/0022-intellij-syntax-highlight.md) · анализ: [../features/0022-intellij-syntax-highlight.md#анализ](../features/0022-intellij-syntax-highlight.md#анализ) · отчёт: [../features/0022-intellij-syntax-highlight.md#отчёт-о-тестировании](../features/0022-intellij-syntax-highlight.md#отчёт-о-тестировании)

## Проблема

При установке собранного плагина `Lam 0.1.0` в актуальную IDE пользователь получил
ошибку:

> Plugin 'Lam' (version '0.1.0') is not compatible with the current version of the
> IDE, because it requires build 243.* or older but the current build is
> RR-261.26222.73

То есть плагин не ставится в IDE новее сборки 243 (напр. RustRover 261 / 2026.1).
Обнаружено при эксплуатации (реальная установка), на стадии приёмки фичи.

## Причина

Верхняя граница совместимости была задана жёстко: `pluginUntilBuild = 243.*`
(выбрана при 0022-01 под целевую линейку платформы 2024.1.x). Значение попадает в
`<idea-version until-build="243.*">` и **блокирует** установку в любые IDE со
сборкой > 243, хотя плагин собран под 2024.1.x лишь из-за ограничения окружения
(JDK 17), а фактически использует только **давно стабильные** API платформы
(`FileType`, `Language`, `SyntaxHighlighter`, `LexerBase`, `Commenter`,
`PairedBraceMatcher`, `ColorSettingsPage`), совместимые с новыми версиями.

## Решение

Убрана верхняя граница совместимости — `until-build` больше не ограничивает
установку (открытый диапазон, документированный JetBrains способ):

- `gradle.properties`: `pluginUntilBuild =` (пусто); `pluginVersion 0.1.0 → 0.1.1`.
- `build.gradle.kts`: `untilBuild = providers.gradleProperty("pluginUntilBuild").orElse("")`
  — пустое значение → `<idea-version since-build="241" until-build="" />`
  (IntelliJ трактует пустой `until-build` как отсутствие верхней границы).
- `sinceBuild = 241` сохранён (нижняя граница; плагин собран под 2024.1.x).

Сборка под платформу 2024.1.x/JDK 17 не менялась — правка затрагивает только
метаданные диапазона совместимости.

## Проверки

- `./gradlew buildPlugin` → BUILD SUCCESSFUL; в `build/tmp/patchPluginXml/plugin.xml`
  — `<idea-version since-build="241" until-build="" />` (верхняя граница снята),
  `<version>0.1.1</version>`; собран `intellij-lam-0.1.1.zip`.
- `./gradlew test` → **27/27 зелёные** (правка конфигурационная, код не затронут).
- Соответствие критерию приёмки A7 (совместимость) — теперь плагин ставится и в
  IDE новее 243 (RustRover 261 и далее).
