# Задача 0179-01: Правка 16 мест со старым адресом

> Фича: [../features/0179-repo-url-cleanup.md](../features/0179-repo-url-cleanup.md) · ADR: [../adr/0179-repo-url-cleanup.md](../adr/0179-repo-url-cleanup.md) · анализ: [../analyze/0179-repo-url-cleanup.md](../analyze/0179-repo-url-cleanup.md)

## Что было

После переезда `origin` на `https://github.com/Pastor/Takt.git` старое имя
осталось в **16 живых местах** в 10 файлах: метаданные обоих крейтов, команда
`git clone` и дерево проекта в README, четыре doc-ссылки в исходниках, манифесты
обоих плагинов.

Держалось на редиректе GitHub (`301`), **кроме одного места**: `cd BuT` сразу
после `git clone` — клон нового URL создаёт каталог `Takt`, и эта строка не
работала независимо от редиректа.

## Что сделано

- **10 вхождений в форме URL** заменены на `https://github.com/Pastor/Takt`:
  `takt-lang/Cargo.toml`, `takt-sim/Cargo.toml`, `README.md` (`git clone`),
  `takt-lang/src/lib.rs` (×3), `takt-lang/src/diagnostics/mod.rs`,
  `extensions/intellij-takt/…/META-INF/plugin.xml`,
  `extensions/zed-takt/extension.toml`, `extensions/zed-takt/scripts/install.sh`.
- **2 вхождения в форме голого имени** в `README.md`: `cd BuT` → `cd Takt`
  (сломанная строка инструкции) и корень дерева проекта `BuT/` → `Takt/`.

⚠️ **Не тронуты** (правило 21 — артефакты стадий суть история решения):
`docs/adr/0100-*`, `docs/features/0100-*`, `docs/reports/0100-*`, где переезд
описан как вынесенный за объём фичи 0100. Также не тронуты `docs/DIFF.md`,
`docs/analyze/0018`, `docs/analyze/0030`: там `BuT` означает историю языка и
локальный путь, а не адрес репозитория (A-4 ADR).

⚠️ **Версии крейтов не подняты, версия языка не поднята** (правило 22):
изменение касается метаданных публикации, а не API, поведения или языка.

## Проверки

- Греп по дереву: живых вхождений `Pastor/BuT` вне артефактов 0100/0179 и
  журналов (`CHANGES.md`, `FEATURES.md`) не осталось.
- Инструкция клонирования читается целиком: `git clone …/Takt.git` + `cd Takt`.
- Вывод генераторов байт-в-байт прежний — правки не касаются кодогенерации
  (проверка A6, см. [отчёт](../reports/0179-repo-url-cleanup.md)).
- Машинная проверка — задача [0179-02](0179-02-repo-url-gate.md).
