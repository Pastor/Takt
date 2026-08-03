# Задача 0199-02: сторожа и документирование

> Фича: [../features/0199-model-implements-brace-form.md](../features/0199-model-implements-brace-form.md) · ADR: [../adr/0199-model-implements-brace-form.md](../adr/0199-model-implements-brace-form.md) · анализ: [../analyze/0199-model-implements-brace-form.md](../analyze/0199-model-implements-brace-form.md)

## Что сделано

**Формы и диагностики** — `takt-lang/tests/model_implement_form_tests.rs` (5):

| Тест | Что доказывает |
|---|---|
| `implement_with_body_is_accepted_by_every_target` | форма переводится **всеми четырьмя** целями |
| `implement_with_own_state_is_rejected` | конфликт → `SE-101` с примечанием, во всех целях |
| `implement_without_body_stays_a_syntax_error` | `model M = A \| B;` по-прежнему `SY-002` |
| `equivalent_form_still_works` | эквивалент не задет |
| `implement_is_expanded_into_a_state` | разворот сделан **состоянием**, а не особым случаем |

⚠️ Первый тест проверяет все четыре цели не для полноты счёта: первая редакция
правки чинила только эталон, и это выявил именно прогон целей.

**Значения** — `takt-sim/tests/conformance_c_tests/model_implement.rs`:
потактовая сверка эталона и цели `c` на **накапливающем** теле. Доказывает и то,
что `always` владельца исполняется, и то, что **ровно раз за такт** (инвариант
0194) — на идемпотентном теле эти два дефекта неразличимы.

**Документирование** (правило 24): раздел о моделях получил подраздел «Модель
как композиция» с формой, её эквивалентом и правилом «модель не может быть и
композицией, и автоматом»; `SE-101` внесён в «Частые ошибки» и в приложение об
ошибках. Значок предупреждения заменён врезкой — шрифт документа его не рисует.

## Проверки

```sh
cargo test -p takt-lang --test model_implement_form_tests
cargo test -p takt-sim --test conformance_c_tests model_implement
```
