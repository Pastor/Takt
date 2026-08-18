# Задача 0251-01: Настройка, документация и проверки

> Фича: [../features/0251-cargo-target-dir.md](../features/0251-cargo-target-dir.md) · ADR: [../adr/0251-cargo-target-dir.md](../adr/0251-cargo-target-dir.md) · анализ: [../analyze/0251-cargo-target-dir.md](../analyze/0251-cargo-target-dir.md)

## Что было

Каталогов сборки было четыре: `target/debug` и `target/release` (ручные
команды), `target/flycheck0` (редактор), `target/precheck` (гейт) — и
самоочистка 0234 следила только за последним. Остальные росли молча: замер
2026-08-18 дал **47.7 ГиБ и 198 621 файл**.

## Что сделано

- **`.cargo/config.toml`** с `[build] target-dir = "target/precheck"`. Шапка
  файла объясняет, зачем он и чем за это платится, — там, где на объяснение
  наткнётся тот, кто откроет файл, а не только читатель `README.md`.
- **`README.md`** (раздел «Сборка из исходников»): общий каталог, замер,
  блокировка и обход `CARGO_TARGET_DIR=target/scratch`.
- **`CLAUDE.md`** (раздел «Документ и процесс»): то же для будущих сессий, плюс
  профиль здорового прогона и оговорка про потребителя с явным каталогом.

Кода это не касается: ни крейты, ни скрипты не правились. `precheck.sh`
продолжает экспортировать `CARGO_TARGET_DIR` — переменная сильнее конфига, а
значение совпадает.

## Проверки

| Что | Как | Итог |
|---|---|---|
| `cargo build` без переменных | `ls target` после сборки | только `precheck/`, `debug/` не создан |
| `cargo` из подкаталога крейта | сборка из `takt-lang/` | туда же (путь относителен корню репозитория) |
| Обход через переменную | `CARGO_TARGET_DIR=target/scratch cargo build` | создан `target/scratch` |
| Блокировка (цена) | два `cargo` в один каталог | `Blocking waiting for file lock on build directory` |
| Инвариант гигиены | `scripts/test-precheck-hygiene.sh` | зелёный (A1–A4) |
| Полный предкоммит | `./scripts/precheck.sh` | см. отчёт |
