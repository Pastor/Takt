# Тест-план фичи 0305: Вызов внешней функции в инициализаторе

> Фича: [../features/0305-extern-call-in-initializer.md](../features/0305-extern-call-in-initializer.md) · анализ: [../analyze/0305-extern-call-in-initializer.md](../analyze/0305-extern-call-in-initializer.md) · отчёт: [../reports/0305-extern-call-in-initializer.md](../reports/0305-extern-call-in-initializer.md)

## Условия проверок

| # | Условие | Как проверяется | Ожидаемый результат |
|---|---|---|---|
| П1 | Отказ на прямом вызове | `extern_call_in_initializer_is_rejected` | `SE-084`, имя функции и `always` в тексте |
| П2 | Отказ на вложенном вызове | `extern_call_nested_in_expression_is_rejected` | `SE-084` |
| П3 | **Контроль:** локальная функция | `local_call_in_initializer_is_accepted` | принимается |
| П4 | **Контроль:** `extern` в теле | `extern_call_in_body_is_accepted` | принимается |
| П5 | Причина вердикта защитна | `extern_call_initializer_is_rejected_before_verification` | вход отвергается `SE-084` |
| П6 | Все девять отвечают одинаково | `scripts/probe.sh` | у всех `SE-084` |
| П7 | Реестр диагностик | `scripts/check-diagnostic-codes.sh` | согласован |
| П8 | Регрессия | `cargo test --all-features` | провалов нет |
| П9 | Предкоммит | `./scripts/precheck.sh` | код 0 |

## Примеры и контрпримеры (правило 16)

**Контрпример** (отвергается):

```takt
extern fn sensor() -> u8;
var mirror: u8 := sensor();   // SE-084
```

**Примеры** (законны — оба контроля):

```takt
fn seed() -> u8 { return 7; }
var mirror: u8 := seed();     // вычисляется при компиляции
```

```takt
extern fn sensor() -> u8;
var mirror: u8 := 0;
start Run { always { mirror := sensor(); } ref Run; }   // штатный путь
```
