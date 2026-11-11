---
okf_version: 0.2
format: 2
type: Task
key: TAKT-645
list: ТАКТ
title: "[0196-01] цвет имени типа не задан ни по умолчанию, ни в настройках"
status: done
priority: none
author: pastor
model: gpt-6
links:
  part:
  - TAKT-195
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:09:37Z
published: 2026-11-11T07:09:37Z
created: 2026-11-11T07:09:37Z
updated: 2026-11-11T07:09:37Z
anal:
  project: 22
  id: 1699
  version: 1
checksum: sha256:7f893c466a4c5af41c899488bac513b26c0b029750c8805a30f8ffbbfca1b7e6
---

Источник: [docs/fixes/0196-01-type-color-defaults.md](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/fixes/0196-01-type-color-defaults.md)

# Исправление: цвет имени типа не задан ни по умолчанию, ни в настройках

> Фича: [../features/0196-editor-type-highlighting.md](../features/0196-editor-type-highlighting.md) · анализ: [../features/0196-editor-type-highlighting.md#анализ](../features/0196-editor-type-highlighting.md#анализ) · отчёт: [../features/0196-editor-type-highlighting.md#отчёт-о-тестировании](../features/0196-editor-type-highlighting.md#отчёт-о-тестировании)

## Проблема

После установки плагина имена типов (`bit`, `bool`, `u8`, `q(m, n)`, псевдонимы
`type`) выглядят **как обычный текст** — подсветки не видно. Обнаружено
заказчиком в эксплуатации сразу после закрытия 0196.

Классификация при этом **исправна**: прогон `semantic_tokens` на
`examples/regulator.takt` даёт `bit` → `type` и `q` → `type`. То есть фича
довела до конца, **что** считать типом, но не довела, **как это выглядит**, — и
цель заказчика («цвет типов, отличный от зарезервированных слов») осталась
недостигнутой.

## Причина

Две, и каждая по отдельности скрывает цвет полностью.

1. **У ключа нет значения по умолчанию.** `TaktHighlighterColors.TYPE` объявлен
   как `key("LAM_TYPE", DefaultLanguageHighlighterColors.CLASS_REFERENCE)`, то
   есть наследует платформенную категорию. Собственного цвета плагин не
   поставлял вовсе: в `plugin.xml` не было ни одного `additionalTextAttributes`,
   а `CLASS_REFERENCE` в поставляемых схемах рисуется цветом обычного текста.
2. **Ключа нет в панели настройки.** `TaktColorSettingsPage` перечисляла только
   **лексические** ключи (ключевое слово, число, строка, скобки…). Все четыре
   семантических (`TYPE`, `FUNCTION`, `CLASS`, `ENUM_MEMBER`) в неё не выведены,
   поэтому пользователь не мог задать цвет и руками.

Класс — «сделано в коде, невидимо в продукте»: тесты фичи проверяли
классификацию токена, а не то, что у категории есть чем отличаться. Проверить
это автоматически нечем — цвет живёт в схеме IDE, — и потому пробел прошёл
приёмку.

## Решение

- Плагин поставляет **значения по умолчанию** для `LAM_TYPE`:
  `colorSchemes/TaktDefault.xml` (светлые схемы) и
  `colorSchemes/TaktDarcula.xml` (тёмные), подключены через
  `additionalTextAttributes` в `plugin.xml`. Цвет подобран так, чтобы отличаться
  и от ключевых слов, и от обычного идентификатора.
- **Только `LAM_TYPE`.** Остальные семантические ключи наследуют категории, у
  которых цвет в схемах есть (`FUNCTION_DECLARATION`, `CLASS_NAME`,
  `STATIC_FIELD`); переопределять их значило бы навязывать вкус там, где
  платформа уже решила.
- Все четыре семантических ключа **выведены в панель** настройки цветов
  (Settings → Editor → Color Scheme → Takt) — цвет стал видимым и настраиваемым.

Значения по умолчанию **перекрываются** пользовательской настройкой: исправление
задаёт отправную точку, а не диктует палитру.

## Проверки

```sh
JAVA_HOME=$(/usr/libexec/java_home -v 17) \
  ./extensions/intellij-takt/gradlew -p extensions/intellij-takt --offline test
```

- Тесты плагина — `BUILD SUCCESSFUL` (в том числе `TaktSemanticTokensColorsTest`,
  контроль полноты маппинга типов токенов в ключи цвета).
- Схемы **попали в артефакт**: `colorSchemes/TaktDefault.xml` и
  `colorSchemes/TaktDarcula.xml` присутствуют в собранном
  `intellij-takt-0.7.0.jar` (проверено `unzip -l`) — иначе
  `additionalTextAttributes` ссылался бы в пустоту, а IDE промолчала бы.
- Плагин переустановлен в RustRover; применяется **после перезапуска IDE**.

**Машиной это не контролируется.** Проверить «цвет отличим на глаз» автотестом
нечем, а Gradle-сборка плагина вне `precheck.sh`/CI (нужен JDK 21).
Регресс такого рода снова обнаружится только глазами — ограничение записано
честно, а не закрыто видимостью проверки.
