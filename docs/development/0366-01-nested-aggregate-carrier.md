# Задача 0366-01: Раскрытие вложенного агрегата — общий носитель

> Фича: [../features/0366-nested-aggregate-carrier.md](../features/0366-nested-aggregate-carrier.md) · ADR: [../adr/0366-nested-aggregate-carrier.md](../adr/0366-nested-aggregate-carrier.md) · анализ: [../analyze/0366-nested-aggregate-carrier.md](../analyze/0366-nested-aggregate-carrier.md)

## Что было

Носитель `aggregate::places` раскрывал **один** уровень агрегата; рекурсию
писала каждая цель. Написали её две из четырёх (обе в цели `c`, фича 0364), а
`st` и `sv` отвергали вход (`ST-011`, `SV-002`).

## Что сделано

**`takt-lang/src/generator/aggregate.rs`** — `leaves(ty, items, fields_of)`:
раскрытие до листьев с путём `Vec<Step>` (`Index` / `Field`); `c_like_suffix`
— форма адресации C-подобных целей. Поиск полей структуры принимается
замыканием: цели хранят объявления по-разному.

**`takt-lang/src/generator/st/st_multidim.rs`** — `iec_suffix`: подряд идущие
индексы сливаются в одну пару скобок (форма IEC, фича 0363).

**Цели**: `st_stmt` (присваивание и локальное объявление), `sv_stmt` (те же
две позиции), `c_model_init` и `c_expr/aggregate` переведены на носитель;
собственные рекурсии, заведённые 0364, сняты.

**`sv_stmt::refuse_struct_in_array`** — поле структуры внутри массива у цели
`sv` отвергается `SV-002` с названной причиной.

**Сверка**: `takt-sim/tests/data/eval/st_nested_aggregate.takt` +
`st_nested_aggregate_trace_matches_reference`.

⚠️ **Формы для цели `sv` измерены, но не применены.** Пробы 2026-08-21 (yosys):
шаблон присваивания поддерживается только для массива целиком; сброс элемента
структуры работает **конкатенацией** (`pts[0] <= {8'd1, 8'd2};`), а умолчание
в `always_comb` обязано печататься **по полям** (`pts_next[0].x = pts[0].x;`).
Это шире агрегата — вынесено кандидатом.

⚠️ **`git checkout` файла посреди мутаций стёр правку** — восстановлена
повторным применением. Мутации на рабочем дереве проверять безопаснее
временными копиями (`cp`), а не откатом из индекса.

## Проверки

```sh
cargo test --test conformance st_nested_aggregate
cargo test --test conformance       # 163
cargo test --test targets           # 391
cargo test -p takt-lang --lib       # 1104
for f in examples/*.takt; do scripts/probe.sh -n 1 "$f"; done   # корпус чист
./scripts/precheck.sh
```

Мутации (обе пойманы): отключить спуск в носителе; печатать у `st` путь
C-подобной формой.
