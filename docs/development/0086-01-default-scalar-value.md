# Задача 0086-01: нулевой дефолт по типу для переменной без инициализатора

> Фича: [../features/0086-sim-var-without-initializer.md](../features/0086-sim-var-without-initializer.md) · ADR: [../adr/0086-sim-var-without-initializer.md](../adr/0086-sim-var-without-initializer.md) · анализ: [../analyze/0086-sim-var-without-initializer.md](../analyze/0086-sim-var-without-initializer.md)

## Что было

`ModelNodeContext::get_value` (`simulation/src/unit/builder.rs`) вычисляет
значение переменной лениво из её инициализатора. У `var q: u8;` инициализатора
нет → `eval_expr(var_expr(var))` даёт `None`, а запасной путь
`default_struct(var, &borrowed)` возвращает нулевые поля **только структуре** и
`None` для скаляра:

```rust
match eval_expr(var_expr(var)) {
    Some(v) => Some(coerce_initial(v, var, &borrowed)),
    None => default_struct(var, &borrowed),   // только struct → Some
}
```

`None` уводит поиск к родителю → имя не находится → `SIM-009` на чтении. Гэп был
прямо помечен в комментарии как осознанный (0034-04).

## Что сделано

Реализовано по **Option A** ADR 0086:

1. **Ветка «нет инициализатора»** отдаёт нулевое значение **по типу** через уже
   существующий `default_field` (он покрывает bool/rational/fixed/array/struct/
   целое единообразно; `default_struct` был его частным случаем):

   ```rust
   None => var_type(var).map(|ty| default_field(ty, &borrowed)),
   ```

   `var_type` даёт `None` только для `Unresolved` → там сохраняется `SIM-009`
   (типа нет — дефолт неоткуда взять; верно).
2. **Удалён** `default_struct` — частный случай `default_field`, дублирование.
3. **Фикстура** `tests/data/eval/var_no_init.lam` (`u8`/`bit`/`q(8,8)` без init +
   наблюдатели) и значенческий тест `var_without_initializer_defaults_to_zero`.

**Статус по функциональности (правило 11):**

| Функциональность | Работа | Обоснование |
|---|---|---|
| Симулятор (`unit/builder.rs`) | **да** | Общий дефолт по типу; `default_struct` удалён |
| Публичный API `simulation` | **н/п** | `default_struct` приватна; сигнатуры не менялись |
| Язык / семантика / кодоген | **н/п** | Не затрагиваются; версия языка **0.2.0** без изменений |
| Массивы | **н/п** | Регистрация нулей уже работает; исполнение — нерабочая 0076, вне объёма |

## Проверки

- Значенческий тест `var_without_initializer_defaults_to_zero` (u8→0, bit→0,
  q→repr 0; `seen := q` проходит без SIM-009) — зелёный. Значения захвачены
  зондом, не угаданы.
- `cargo test -p simulation -- --test-threads=1` — весь корпус (вкл. conformance)
  зелёный; регресса портов/констант нет.
- `predicate.rs` SIM-009-тест (истинно неизвестное имя) — зелёный (не задет).
- `./scripts/precheck.sh` зелёный; `git diff examples/generated/` пуст.
