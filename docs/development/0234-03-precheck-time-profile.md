# Задача 0234-03: Профилирование и ускорение предкоммита

> Фича: [../features/0234-precheck-time-profile.md](../features/0234-precheck-time-profile.md) · ADR: [../adr/0234-precheck-time-profile.md](../adr/0234-precheck-time-profile.md) · анализ: [../analyze/0234-precheck-time-profile.md](../analyze/0234-precheck-time-profile.md)

## Что было

Замер «до» существовал только как наблюдение при закрытии
[0203](../features/0203-validate-formulas-traversal.md) («более суток»), а
инвариант о каталоге гейта нигде не был записан: следующий, кто тронет
`precheck.sh`, не узнал бы ни причины, ни цены.

## Что сделано

**Замер «после» — полный прогон:**

| Что | До (0203, 2026-08-15) | После |
|---|---|---|
| `./scripts/precheck.sh` целиком | **> 24 ч** | **230 с** |
| `cargo test` (исполнение) | 64.31 с | 56.35 с |
| каталог сборки | 83 ГБ / 820 386 файлов | 1.4 ГБ |
| запись `debug/deps` | 19.2 МиБ | 0.07 МиБ |

**Закрытие:** `CHANGES.md`, живой контекст (`CLAUDE.md` — инвариант каталога
гейта и цена его нарушения), карточка фичи, реестры, тест-план и отчёт.

Функциональности: **документ проекта** — да; **`book/`** — н/п (правило 24: язык
не меняется, фича процессная).

## Проверки

```sh
./scripts/precheck.sh            # 230 с, код возврата 0
python3 scripts/check-claude-md.py
python3 scripts/check-feature-status.py
python3 scripts/check-links.py
```
