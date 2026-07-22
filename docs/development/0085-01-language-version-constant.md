# Задача 0085-01: Константа версии языка в коде + гейт синхронизации с README

> Фича: [../features/0085-language-version-constant.md](../features/0085-language-version-constant.md) · ADR: [../adr/0085-language-version-constant.md](../adr/0085-language-version-constant.md) · анализ: [../analyze/0085-language-version-constant.md](../analyze/0085-language-version-constant.md)

## Что было

Версия языка жила **только** в `README.md` (`**Версия языка: 0.3.0**`). Константы
в коде не было, `lamc` версию не печатал, рассинхрон дока↔факт ничем не ловился.
**Живой дефект:** фича 0078 подняла язык `0.3.0 → 0.4.0` (`CHANGES.md`/`CLAUDE.md`),
но `README.md` остался на `0.3.0` — прошло незамеченным при закрытии 0078.

## Что сделано

Реализована **Option A** ADR 0085.

- **Константа (`grammar`).** Новый модуль `grammar/src/version.rs`:
  `pub const LANGUAGE_VERSION: &str = "0.4.0";` — единственный источник истины.
  Реэкспорт `grammar::LANGUAGE_VERSION` в `lib.rs`. Два юнит-теста: формат
  SemVer-тройки и наличие реэкспорта из корня крейта.
- **Гейт (`scripts`).** `scripts/check-language-version.sh` (POSIX `sh`, образец
  `check-diagnostic-codes.sh`): извлекает версию из литерала `version.rs` и
  каноническую строку `**Версия языка: X.Y.Z**` из `README.md`, сверяет. Три
  условия отказа: расхождение / нет якоря / дубль якоря. Подключён в `precheck.sh`
  в блоке быстрых проверок (после реестра кодов, до долгих тестов) — CI получает
  его автоматически (0090).
- **Устранение живого рассинхрона (`README`).** Каноническая строка приведена
  `0.3.0 → 0.4.0`; добавлены заметка о единственном источнике истины (гейт) и
  историческая врезка `0.3.0 → 0.4.0` (фича 0078). Это **не** подъём версии
  (правило 22) — фиксация уже действующей `0.4.0`.
- **Компенсация размера `lib.rs`.** Модуль-декларация + реэкспорт (+3 строки)
  нарушили бы храповик размера (`lib.rs` на baseline 1402). Ужат crate-doctest
  ровно на 3 строки (fmt-стабильно, doctest остаётся валидным) — `lib.rs` вновь
  1402, baseline не трогается. `lamc.rs` (тоже на baseline 1993) **не тронут** —
  CLI-подкоманда `version` вне объёма (Option B заблокирован размером; кандидат).

| Стек | Статус |
|---|---|
| `grammar` | ✅ константа + реэкспорт + юнит-тесты, компенсация размера `lib.rs` |
| `scripts` | ✅ новый гейт + подключение в `precheck.sh` |
| `README.md` | ✅ каноническая строка `0.4.0` + заметки |
| `simulation` | н/п — фича не затрагивает симулятор |
| генераторы целей | н/п — вывод байт-в-байт неизменен (правило 11) |
| `lamc.rs` (CLI) | н/п — `lamc version` вне объёма (baseline размера; кандидат) |

## Проверки

- `cargo test -p grammar version:: -- --test-threads=1` → 2 passed (формат +
  реэкспорт).
- `cargo test -p grammar --doc` → 35 passed (ужатый crate-doctest компилируется).
- `cargo clippy -p grammar --all-targets --all-features -- -D warnings` → чисто.
- `./scripts/check-language-version.sh` → OK (код 0), версия `0.4.0` согласована.
- **Проба A4 (рассинхрон):** до правки README гейт дал код 1 с диагностикой
  `LANGUAGE_VERSION 0.4.0 vs README 0.3.0` — красный на живом дефекте.
- **Проба A5 (нет/дубль якоря):** на временных копиях README `anchors=0` и
  `anchors=2` → обе ветки красные.
- `./scripts/check-module-size.sh` → код 0 (`lib.rs` = 1402, baseline цел).
- Полный `./scripts/precheck.sh` → зелёный (см. отчёт
  [../reports/0085-language-version-constant.md](../reports/0085-language-version-constant.md)).
