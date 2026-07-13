# Тест-план фичи 0023: Плагин IntelliJ IDEA — навигация к декларации и include

> Фича: [../features/0023-intellij-navigation-include.md](../features/0023-intellij-navigation-include.md) · ADR: [../adr/0023-intellij-navigation-include.md](../adr/0023-intellij-navigation-include.md) · анализ: [../analyze/0023-intellij-navigation-include.md](../analyze/0023-intellij-navigation-include.md)

## Область и цель

Проверить переход к декларации имён Lam и навигацию по директивам `import` к
файлу (требования R1–R5, критерии A1–A6 анализа). Языковые синтаксис/семантика
не затрагиваются — тестов компилятора/интерпретатора не требуется (правило 16 н/п:
фича не меняет язык). Проверки автоматизированы на `BasePlatformTestCase`.

## Проверки (условие → ожидаемый результат)

| # | Проверка | Предусловие | Ожидаемый результат | R/A |
|---|---|---|---|---|
| T1 | Индекс находит объявления всех форм | текст с `model/state/start/type/enum/cond/var/const/fn` | все имена в индексе; варианты `enum` — нет | R1/A1 |
| T2 | Импорт-переименования в индексе | `import { A as C } from "f";`, `import "p" as P;`, `import * as Q from "p";` | локальные имена `C/P/Q`; источник `A` — нет | R1/A1 |
| T3 | Диапазон декларации указывает на имя | `model Widget { }` | диапазон покрывает `Widget` | R1/A1 |
| T4 | Переход от использования к декларации | `model Producer{} … Producer` | цель — имя в `model Producer` | R2/A2 |
| T5 | Переход по псевдониму импорта | `import { SharedModel as M } … M` | цель — алиас `M` в `import` | R2/A2 |
| T6 | Нет перехода на ключевом слове | каретка на `model` | целей нет | R2/A3 |
| T7 | Нет перехода на самой декларации | каретка на имени в объявлении | целей нет | R2/A3 |
| T8 | Нет перехода для неизвестного имени | использование без объявления | целей нет | R2/A3 |
| T9 | Импорт-путь → файл (3 формы) | `import "shared.lam"` / `… from "shared.lam"` / `"shared.lam" as S` | открывается `shared.lam` | R3/A4 |
| T10 | Отсутствующий файл | путь к несуществующему файлу | цели нет, без исключений | R4/A5 |
| T11 | Строка вне import не навигируется | строка в `formula "…"` | файловой цели нет | R3/A4 |
| T12 | Сборка и весь набор тестов | — | `buildPlugin` OK, все тесты зелёные | R5/A6 |

## Разбивка проверок по функциональности

Затрагивается только редакторная оснастка плагина (Kotlin). Обратная
функциональность `grammar`/`simulation` и языковой слой — **не применимо** (—):
фича аддитивна, ни один языковой артефакт не изменён. Подсветка/эргономика 0022
проверяются существующими тестами (регресс).

- Плагин `intellij-lam`: ✅ (T1–T12)
- `grammar`/`simulation`, синтаксис/семантика языка: — (не затронуты)

## Тестовые данные и окружение

- Тесты: `LamSymbolScannerTest`, `LamGotoDeclarationTest`, `LamImportReferenceTest`
  (+ регресс 0022: лексер/highlighter/FileType/цвета/эргономика).
- Окружение: IntelliJ Platform IC `2024.1.7`, JDK 17, Gradle wrapper 8.10.2,
  `TestFrameworkType.Platform`. Прогон: `./gradlew --offline clean buildPlugin test`.
