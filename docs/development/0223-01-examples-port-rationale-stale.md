# Задача 0223-01: Три примера объясняют выходной порт устаревшей нуждой цели rust

> Фича: [../features/0223-examples-port-rationale-stale.md](../features/0223-examples-port-rationale-stale.md) · ADR: [../adr/0223-examples-port-rationale-stale.md](../adr/0223-examples-port-rationale-stale.md) · анализ: [../analyze/0223-examples-port-rationale-stale.md](../analyze/0223-examples-port-rationale-stale.md)

## Что было

Четыре примера корпуса объясняли свой выходной порт `ready` нуждой цели `rust`:
«модель без портов → `clippy::new_without_default`». После фичи 0174 модель без
портов получает `impl Default`, и линт молчит — объяснение перестало быть
верным. Кандидат, из которого заведена фича, называл **три** файла; четвёртый
(`batch_cycle.takt`) несёт то же объяснение своей формулировкой и под греп
кандидата не попал (замер ADR).

Порт при этом у трёх примеров реально удерживается тестбенчем цели `sv`, а у
`regulator` — ещё и гейтом `sv-mmio` (фича 0214): удалить фразу, не назвав
настоящую причину, значило бы пригласить следующего редактора снять порт.

## Что сделано

Заменены комментарии в **четырёх** файлах `examples/` (правка только текстовая,
ни одного оператора Takt не тронуто):

| Файл | Новая формулировка причины |
|---|---|
| `regulator.takt` | наблюдаемая точка завершения: тестбенч `sv` (`$error`, если не поднялся) + единственный пример гейта `sv-mmio` со всеми выходными портами (0214) |
| `pid_regulator.takt` | наблюдаемая точка схождения контура: тестбенч `sv` |
| `batch_cycle.takt` | наблюдаемая точка завершения цикла: тестбенч `sv`, вместе с порядком фаз |
| `float_regulator.takt` | паритет с `regulator.takt`; гейтом порт **не** удерживается (в `sv` пример без `--float-as-q` не транслируется — `SV-003`) |

Обратная функциональность (правило 11): затронуты **только комментарии**.
Компилятор, симулятор, генераторы, CLI и форматы файлов — «н/п»: код не
менялся, версия языка не поднимается (язык не изменён).

## Проверки

```sh
grep -rn 'new_without_default' examples/          # пусто (код 1) — R1
git diff --stat examples/                          # ровно 4 файла .takt — R2/R4
cargo run --bin taktc -- fmt --check examples/     # код 0 — R6
./scripts/precheck.sh                              # код 0 — R5/A6
git status --porcelain examples/generated/         # пусто после precheck — R5
```

Результаты — в отчёте [../reports/0223-examples-port-rationale-stale.md](../reports/0223-examples-port-rationale-stale.md).
