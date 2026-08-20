# Разработка 0306-01: отказ с причиной в свёртке инициализаторов

> Фича: [../features/0306-unfoldable-call-in-initializer.md](../features/0306-unfoldable-call-in-initializer.md) · ADR: [../adr/0306-unfoldable-call-in-initializer.md](../adr/0306-unfoldable-call-in-initializer.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/semantic/declaration.rs` | `let Ok(…) else` → `match` с именованным `cause`; функции `initializer_calls_function` и `unfoldable_call`; `initializer_calls_extern` научена второй форме узла |
| `takt-lang/src/semantic/validate/init_undefined_read.rs` | снят вариант `ExternCall` — ветвь стала мёртвой (подтверждено мутацией) |
| `takt-lang/src/semantic/declaration/init_refusal.rs` | новый модуль: четыре судьи инициализатора (`SE-109`, `SE-114`, два случая `SE-084`) |
| `takt-lang/tests/semantic/unfoldable_call_tests.rs` | шесть проверок: отказ, причина, рекурсивность, дробный случай, два контроля, устройство |

⚠️ **Модуль выделен по границе ответственности, а не ради строк**, хотя поводом
был гейт размера: `declaration.rs` дорос до 1015 строк при лимите 1000. Свёртка
решает, **чем стало** значение; судьи решают, **законна ли запись**. Файл
переехал в каталог (`declaration/mod.rs` + `declaration/init_refusal.rs`), а не
объявился в `semantic/mod.rs`: тот сам сверх лимита и расти не имеет права.

## Текст диагностики

```
[SE-084] инициализатор 'scale' зовёт функцию, которую компилятор вычислить не
может: 'base' — переменная: её значение известно только в такте. Начальное
значение выставляется до первого такта, и прежде потребители расходились молча:
эталон оставлял ноль, а цель 'st' теряла инициализатор без единого слова.
Присвойте в теле состояния — 'always { scale := …; }'
```

⚠️ Цитируется **причина**, а не всё сообщение вычислителя: его вводная
(«выражение не вычисляется при компиляции») здесь уже сказана своими словами, и
оставленная целиком читалась бы заиканием.

## Проверено

- `cargo test --test semantic unfoldable_call` — 6/6.
- `cargo test --all-features` — провалов нет.
- `scripts/probe.sh` до и после (таблица — в ADR и отчёте).
