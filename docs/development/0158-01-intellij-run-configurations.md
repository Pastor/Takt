# Задача 0158-01: Конфигурации запуска, командная строка и фильтр вывода

> Фича: [../features/0158-intellij-run-configurations.md](../features/0158-intellij-run-configurations.md) · ADR: [../adr/0158-intellij-run-configurations.md](../adr/0158-intellij-run-configurations.md) · анализ: [../analyze/0158-intellij-run-configurations.md](../analyze/0158-intellij-run-configurations.md)

## Что было

Настройки 0125 (`compilerPath`, `simulatorPath`, `includeDirs`, `compilerArgs`,
`outputDir`) хранились и **не исполнялись**: запустить `taktc` или `takt-sim` из
IDE было нечем, автор уходил в терминал.

## Что сделано

Новый пакет `org.takt.intellij.run`:

- **`TaktCommandLine`** — сборка командной строки как **чистая функция**
  `build(mode, params, tools) → Ready | Refused`. Ни `Project`, ни `Editor`, ни
  сервисов: плагин вне `precheck.sh`, и проверяемо ровно то, что не требует окна
  IDE. Порядок аргументов закреплён (файл последним, `-o` перед ним), пустое
  поле не даёт пустого флага, каталоги импортов идут **повторяемым** `-I`
  (разделитель у компилятора платформозависим, и второе знание о нём заводить
  незачем).
- **`TaktRunConfiguration` + `TaktRunConfigurationType`** — один тип, две
  фабрики (**Compile**, **Simulate**). Незаданный путь к инструменту даёт
  `RuntimeConfigurationError` в диалоге, а не исключение в логе.
- **`TaktRunConfigurationEditor`** — раскладка полей; поля различаются по режиму
  (цель — у компиляции, сценарий и шаги — у симуляции).
- **`TaktOutputFilter`** — позиция `файл:строка:колонка` в выводе становится
  ссылкой.

## Проверки

```sh
cd extensions/intellij-takt && ./gradlew --offline test   # 97 тестов, зелено
```

⚠️ **Ловушка карточки подтвердилась и закреплена тестом.** Колонка диагностики
Takt считается в **символах** (в `.takt` законна кириллица), а документ IDEA
адресуется кодовыми единицами UTF-16 — значит на символе вне BMP счёт снова
расходится. `charColumnToOffset` идёт по кодовым точкам; **мутация** «наивное
`колонка - 1`» валит тест с эмодзи (1 провал из 97).

⚠️ **Первая редакция двух тестов ошиблась в арифметике колонок** (я считал
позиции неверно, код был прав) — ожидания исправлены по факту, а не наоборот.
