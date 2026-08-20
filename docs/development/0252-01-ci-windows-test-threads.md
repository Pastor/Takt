# Задача 0252-01: Снятие `--test-threads=1` и правка комментариев

> Фича: [../features/0252-ci-windows-test-threads.md](../features/0252-ci-windows-test-threads.md) · ADR: [../adr/0252-ci-windows-test-threads.md](../adr/0252-ci-windows-test-threads.md) · анализ: [../analyze/0252-ci-windows-test-threads.md](../analyze/0252-ci-windows-test-threads.md)

## Что сделано

**`.github/workflows/ci.yml`** — задание `windows`: флаг снят, шаг называется
«cargo test (все фичи)». Комментарий переписан и теперь говорит три вещи:

1. предкоммит однопоточности **не требует** с фичи 0190, и прежний текст
   утверждал снятое правило;
2. ключ уникальности временного каталога — **имя потока**, поэтому флаг
   изоляцию не усиливал, а ослаблял;
3. правка **прогоном не проверена** — задания Actions не стартуют (фича 0175).

**`scripts/coverage.sh`** — те же три флага сняты (обоснования у них не было
вовсе), рядом записано, почему.

## Проверка

```sh
grep -n 'test-threads' .github/workflows/ci.yml scripts/coverage.sh  # только в комментариях-истории
sh scripts/coverage.sh --files                                        # профиль тот же
```
