# Задача 0171-01: Строгие флаги в гейтах цели `c`

> Фича: [../features/0171-c-gate-werror.md](../features/0171-c-gate-werror.md) · ADR: [../adr/0171-c-gate-werror.md](../adr/0171-c-gate-werror.md) · анализ: [../analyze/0171-c-gate-werror.md](../analyze/0171-c-gate-werror.md)

## Что было

Оба гейта цели `c` собирали порождённый код **без единого флага предупреждений**:
корпус — `cmake -DCMAKE_BUILD_TYPE=Debug -G Ninja` + `ninja`, `c-hal` —
`cc -std=c11 -c`. Поэтому гейт принял тождественно ложное `if (model->lv == -5)`
при `uint8_t lv`, о котором `cc` **предупреждал**.

## Что сделано

**`scripts/precheck.sh`** — строгость в обоих гейтах:

- корпус: `cmake … -DCMAKE_C_FLAGS="-Wall -Werror"`;
- `c-hal`: `cc -std=c11 -Wall -Werror -c`.

⚠️ **Флаги заданы в гейте, а не в `CMakeLists`** порождённого примера: `-Werror`
там навязал бы строгость всякому, кто скопирует `examples/generated/c` себе.

**`takt-lang/src/generator/c/c_hal.rs`** — помощник `<Root>_bind_default_hal`
эмитится **`static inline`** вместо `static`.

⚠️ **Это починка порождённого кода, а не обход гейта.** Помощник объявлен в
заголовке и вызывается не всякой единицей трансляции — у голого `static` это
`-Wunused-function` в **любой** сборке со строгими флагами. Замер до правки: по
одному предупреждению на каждый пример корпуса, то есть пользователь со своим
`-Wall -Werror` наш HAL не собрал бы вовсе. Неиспользованная `static inline`
предупреждения не даёт (проба на `cc`: `static` → 1, `static inline` → 0).

**`CLAUDE.md`** — в пункт о цели `c` внесены: строгость обоих гейтов, замер
стоимости (`-Wall` — 0, `-Wextra` — 38), правило «флаги в гейте, не в примере» и
причина `static inline`.

## Проверки

| Что | Результат |
|---|---|
| корпус под `-Wall` (замер до включения) | 0 предупреждений |
| `c-hal` под `-Wall -Werror` после правки | **0** (было 7) |
| мутация: `if (main->current_floor == -5)` в `elevator.c` | `FAILED`, `error: … always false [-Werror,-Wtautological-constant-out-of-range-compare]` |
| `./scripts/precheck.sh` | код возврата `0` |

⚠️ Первая мутация (`model->state == -5`) **не сработала** — поле перечислимого
типа, сравнение не тавтологично. Заменена на заведомо беззнаковое поле
(`uint8_t current_floor`): сторож обязан проверять взведённость ловушки, а не
удачу выбора поля.
