# Задача 0370-01: Понижение q-литерала доходит до полей структуры

> Фича: [../features/0370-struct-field-fixed.md](../features/0370-struct-field-fixed.md) · ADR: [../adr/0370-struct-field-fixed.md](../adr/0370-struct-field-fixed.md) · анализ: [../analyze/0370-struct-field-fixed.md](../analyze/0370-struct-field-fixed.md)

## Что было

`lower_folded_fixed` понижал литерал по типу объявления и по типам элементов
массива (0368), но полей структуры не видел: у помощника не было модели.

## Что сделано

**`takt-lang/src/semantic/declaration/mod.rs`** — `lower_folded_fixed`
принимает `&Rc<RefCell<ModelNode>>` и понижает элементы агрегата структуры по
типам её полей (`search_struct`), в порядке объявления. Ветвь массива зовёт ту
же функцию, поэтому `[Gains; 2]` покрывается рекурсией.

**Сверка**: `struct_field_fixed_matches_generated_c` — представления `1.5` →
384 и `1.0` → 256 у поля и у поля внутри массива.

⚠️ **Границу создавала сигнатура, а не устройство:** модель у
`fold_variable_initializers` была всё это время.

⚠️ **Тест сначала мерил не тот класс.** Наблюдаемые `(g.kp + g.ki) as u8`
давали 4 у эталона и 0 у C — но из-за **приведения из поля**, которое цель `c`
печатает без деления на 2ⁿ. Наблюдаемые заменены на q-значения, соседний класс
вынесен кандидатом.

## Проверки

```sh
cargo test --test conformance struct_field_fixed
cargo test --test conformance   # 169
cargo test --test targets       # 391
cargo test -p takt-lang --lib   # 1108
scripts/probe.sh -n 2 qfield.takt; scripts/probe.sh -n 2 qstructarr.takt
./scripts/precheck.sh
```

Мутация «не понижать поля структуры» — сверка красная («порождённый C не
компилируется»).
