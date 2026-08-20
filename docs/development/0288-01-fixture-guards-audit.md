# Задача 0288-01: Усиленные сторожа и ратчет констатаций

> Фича: [../features/0288-fixture-guards-audit.md](../features/0288-fixture-guards-audit.md) · ADR: [../adr/0288-fixture-guards-audit.md](../adr/0288-fixture-guards-audit.md) · анализ: [../analyze/0288-fixture-guards-audit.md](../analyze/0288-fixture-guards-audit.md)

## Что сделано

**`takt-lang/tests/semantic/fixture_promises_tests.rs`** — пять сторожей,
проверяющих **обещание** фикстуры:

| Фикстура | Обещание | Что проверяется |
|---|---|---|
| `ce6_type_from_func.takt` | тип из возвращаемого значения функции | `ty_of(result) == "Bool"` |
| `local_var_in_block.takt` | локальная переменная доступна в блоке | узел `name: "x"` **и** разрешённое тело `Always { body: Block(…) }` |
| `enum_basic.takt` | перечисление несёт варианты | все четыре варианта в дереве вложенной модели |
| `array_access.takt` | индексация элемента | узел `ArraySubscript` в инициализаторах |
| `named_blocks.takt` | `enter`/`exit`/`always` | все три варианта в дереве состояния |

⚠️ **Две ловушки, найденные прогоном:**

1. проверка «в дереве нет `Unresolved`» **неверна**: условия рёбер `ref` по
   инварианту проекта остаются неразрешёнными, и сторож падал бы на любой
   корректной модели;
2. смотреть надо **туда, где написано**: индексация `array_access` стоит в
   инициализаторах объявлений, перечисление `enum_basic` — во вложенной модели.

⚠️ Фикстура `ce4_enum_basic.takt`, вопреки имени, перечислений не содержит
вовсе (её комментарий: «enum добавляется программно») — усиливать там нечего.

**`scripts/fixture-guard-baseline.txt`** — реестр 28 констатаций с причиной и
правилом правки (только удаление вместе с усилением).

**`scripts/check-fixture-guards.py`** (шаг предкоммита, самопроверка первой):
`F1` — новый слабый сторож, `F2` — протухшая запись реестра.

## Проверка

```sh
cargo test --test semantic fixture_promises_tests::   # 5 тестов
python3 scripts/check-fixture-guards.py --self-test
python3 scripts/check-fixture-guards.py               # 62 теста, 28 в реестре
```
