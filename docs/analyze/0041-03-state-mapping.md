# Анализ 0041-03: Состояния, переходы и композиция моделей в ST

> Обзор: [0041-st-backend.md](0041-st-backend.md) · Фича: [../features/0041-st-backend.md](../features/0041-st-backend.md) · ADR: [../adr/0041-st-backend.md](../adr/0041-st-backend.md) (вопрос 1) · Разработка: [../development/0041-03-state-mapping.md](../development/0041-03-state-mapping.md)

## Цель

Задать нормативное отображение автоматной части модели Lam (состояния, переходы,
блоки `enter`/`always`/`exit`, композиция `M1 + M2` / `M1 | M2`) на `CASE state OF`
внутри `FUNCTION_BLOCK`. Опора — принятый ADR (вопрос 1, Option A) и **зонд
реального вывода C-бэкенда**, снятый 2026-07-15.

## Эталон: что делает C-бэкенд (зонд)

Команда: `lamc compile -t c examples/stacker.lam -o <tmp>`. Ниже — факты,
проверенные по порождённому `stacker.c`/`stacker.h`, а не по предположениям.

### Ф1. Модель → структура + четыре функции

```c
struct StackerLiftController { enum { …_INIT, …_LIFT_IDLE, …_LIFT_OPERATING,
                                      …_LIFT_DONE, …_END } state; };
static void StackerLiftController_init(StackerLiftController *model, Stacker *main);
static void StackerLiftController_tick(StackerLiftController *model, Stacker *main);
static bool StackerLiftController_is_done(const StackerLiftController *model, Stacker *main);
```

Плюс `_reset`, вызывающий `_init` (`stacker.c:117-119`). Обратим внимание:
под-модель получает **два** указателя — свой `model` и корневой `main`, через
который читает переменные и порты корня.

### Ф2. Состояния → `switch (model->state)` (`c_model.rs:554`)

Синтетические состояния: **`INIT`** первым и **`END`** последним. Значения enum
неявные (C-нумерация с 0), т.е. `INIT = 0`.

### Ф3. `INIT`-состояние исполняет `enter` стартового состояния и переходит в него

```c
case STACKER_LIFT_CONTROLLER_INIT: {
    (*main->write_bit)(STACKER_CMD_FORK, 0, main->userdata);   /* enter LiftIdle */
    model->state = STACKER_LIFT_CONTROLLER_LIFT_IDLE;
    break;
}
```

(`stacker.c:69-73`; исходник — `stacker.lam:357-362`, `start LiftIdle { enter {
cmd_fork := 0; } … }`.)

### Ф4. `enter` **целевого** состояния инлайнится в переход

```c
case STACKER_LIFT_CONTROLLER_LIFT_IDLE: {
    if (main->lift_request) {
        (*main->write_bit)(STACKER_CMD_FORK, 1, main->userdata);  /* enter LiftOperating! */
        model->state = STACKER_LIFT_CONTROLLER_LIFT_OPERATING;
        break;
    }
    break;
}
```

(`stacker.c:74-81`.) Это **важнейшая деталь семантики**: `enter` целевого
состояния исполняется **в том же такте, что и переход**, а не в следующем.

### Ф5. Несколько `ref` → последовательность независимых `if` с `break`

```c
case STACKER_LIFT_CONTROLLER_LIFT_OPERATING: {
    if (lift_request && !lift_op && sense_loaded)  { … state = LIFT_DONE; break; }
    if (lift_request &&  lift_op && !sense_loaded) { … state = LIFT_DONE; break; }
    if (!lift_request)                             { … state = LIFT_IDLE; break; }
    break;
}
```

(`stacker.c:82-101`; исходник — `stacker.lam:365-374`, три `ref` подряд.)
**Порядок объявления `ref` = порядок проверки**; первый сработавший выигрывает
(`break`). Это, по сути, `ELSIF`-цепочка.

### Ф6. Параллельная композиция `A | B | C`

Исходник — `stacker.lam:388`: `start Stacker = CommandReceiver |
MovementController | LiftController;`. Порождённое:

```c
struct Stacker {
    /* переменные корня: lift_op, tgt_section, …, busy, tgt_stack */
    enum { STACKER_INIT, STACKER_STACKER, STACKER_END } state;
    struct {                                 /* NOTICE: Определение extend */
        StackerCommandReceiver    command_receiver0;
        StackerMovementController movement_controller1;
        StackerLiftController     lift_controller2;
        enum { STACKER_STACKER_INIT, STACKER_STACKER_TICK, STACKER_STACKER_END } state;
    } stacker;
    /* колбэки HAL */
};

void Stacker_tick(Stacker *model) {
    switch (model->state) {
        case STACKER_INIT: {
            StackerCommandReceiver_init(&model->stacker.command_receiver0, model);
            StackerMovementController_init(&model->stacker.movement_controller1, model);
            StackerLiftController_init(&model->stacker.lift_controller2, model);
            model->stacker.state = STACKER_STACKER_INIT;
            model->state = STACKER_STACKER;
            break;
        }
        case STACKER_STACKER: {
            StackerCommandReceiver_tick(&model->stacker.command_receiver0, model);
            StackerMovementController_tick(&model->stacker.movement_controller1, model);
            StackerLiftController_tick(&model->stacker.lift_controller2, model);
            if (StackerCommandReceiver_is_done(…) && StackerMovementController_is_done(…)
                && StackerLiftController_is_done(…)) {
                model->state = STACKER_END;
                break;
            }
            break;
        }
        case STACKER_END: { break; }
    }
}
```

(`stacker.c:414-439`, `stacker.h`.) Семантика параллелизма: **последовательный
вызов `_tick` под-моделей в одном такте родителя**, в порядке объявления;
завершение — **конъюнкция** `_is_done`. Никакой настоящей конкурентности —
чередование детерминировано. Это ровно то, что нужно для скан-цикла ПЛК.

Заметим: имена экземпляров получают **числовой суффикс по порядку** —
`command_receiver0`, `movement_controller1`, `lift_controller2`.

### Ф7. Переменные корня — общие для параллельных моделей

`stacker.lam:85-87` объявляет координационные `var lift_request/lift_op/lift_done`
в корне; под-модели пишут в них через `main->lift_request` (`stacker.c:75, 85`).
То есть параллельные модели общаются **через переменные корневой модели**.

## Отображение на ST (нормативно)

| # | Конструкция Lam | C (факт зонда) | **ST** |
|---|---|---|---|
| **S1** | `model M { … }` | `struct M` + `M_init/_tick/_reset/_is_done` | `FUNCTION_BLOCK M` … `END_FUNCTION_BLOCK`; тело FB = один вызов ≡ `_tick` |
| **S2** | переменная состояния | `enum { … } state` в структуре | `state : USINT := 0;` в `VAR` (либо `TYPE M_State : (…); END_TYPE`, если проба 0041-06 разрешит перечислимые типы) |
| **S3** | синтетический `INIT` | `case …_INIT` первым, значение 0 | `0: (* INIT *)`. **Значение 0 — не случайно:** при холодном старте ПЛК `VAR` инициализируется нулём → `_init()` из C **не нужен** отдельным вызовом |
| **S4** | синтетический `END` | `case …_END: break;` | `<n>: ;` (пустой оператор) |
| **S5** | `state S { … }` | `case …_S: { … }` | `<k>: (* S *) …` |
| **S6** | `enter` стартового | инлайн в `case …_INIT` (Ф3) | инлайн в ветвь `0:` |
| **S7** | `enter` целевого при переходе | инлайн в тело `if` перехода (Ф4) | инлайн в тело `IF`/`ELSIF` |
| **S8** | `always` | тело `case` до проверок `ref` | операторы ветви `CASE` до `IF` |
| **S9** | `ref T: cond;` (несколько) | цепочка `if (…) { …; state = …; break; }` (Ф5) | `IF <c1> THEN … ELSIF <c2> THEN … END_IF;` — семантика «первый сработавший» сохраняется |
| **S10** | `next T;` (безусловный) | `state = …_T; break;` | `state := <k_T>;` |
| **S11** | терминальность | `_is_done()` → `state == …_END` | `is_done : BOOL` в `VAR_OUTPUT`; `is_done := (state = <n_END>);` в конце тела |
| **S12** | `M1 \| M2` | экземпляры + последовательные `_tick` + конъюнкция `_is_done` (Ф6) | экземпляры под-FB в `VAR`; последовательные вызовы в ветви `CASE`; `IF a.is_done AND b.is_done THEN state := <END> END_IF;` |
| **S13** | `M1 + M2` | шаги по `state` родителя (`c_model.rs:615, 751`) | ветви `CASE` родителя: пока `NOT M1.is_done` — вызывать `M1`; затем переход к шагу `M2` |
| **S14** | переменные корня | поля `struct Stacker`, доступ через `main->` (Ф7) | `VAR` корневого FB; передача в под-FB через `VAR_INPUT`/`VAR_OUTPUT` — **см. открытый вопрос О1** |

### `exit`-блоки

`always`/`enter`/`exit` — именованные блоки прохода 4 (`semantic/tree.rs`). Зонд
`stacker.lam` `exit` не содержит; `elevator.lam` — тоже. **Действие:** снять
отдельный зонд на фикстуре с `exit` **до** реализации S7 (правило `CLAUDE.md`:
«сперва зонд для захвата реального вывода, затем assertions против захваченных
значений»). Ожидание: `exit` источника инлайнится в переход **перед** `enter`
цели, — но это **предположение**, а не факт, и подлежит проверке.

## Открытые вопросы

**О1. Как параллельные под-FB видят переменные корня — главный вопрос задачи.**

В C всё просто: под-модель получает указатель `Stacker *main` и пишет
`main->lift_request = 1` — **разделяемая память**. В ST указателей в переносимом
подмножестве нет. Варианты:

| Вариант | Суть | Оценка |
|---|---|---|
| **О1-а** | Координационные переменные корня → `VAR_GLOBAL`; под-FB обращаются к ним напрямую | Проще всего, точно транслируется; **но** ломает экземплярность FB (два экземпляра модели разделят глобалы) и загрязняет глобальное пространство имён |
| **О1-б** | `VAR_INPUT`/`VAR_OUTPUT` под-FB + явная перепись значений родителем до/после вызова | Идиоматично для IEC, экземплярность сохраняется; **но** родитель обязан копировать переменные туда-обратно, а **порядок копирования меняет семантику**: в C под-модель видит запись предыдущей под-модели **в том же такте** (`lift_request` пишет `MovementController`, читает `LiftController` — `stacker.c:426-428`). Наивное «скопировать всё до, вернуть всё после» **сломает** эту семантику |
| **О1-в** | `VAR_IN_OUT` под-FB (передача по ссылке — есть в стандарте) | Ближе всего к C-семантике (разделяемая переменная), экземплярность сохранена; **но** поддержка `VAR_IN_OUT` в MatIEC требует пробы |

**Предварительное предпочтение: О1-в**, откат — О1-б с **копированием после
каждого вызова** (а не после всех), что воспроизводит порядок C. **Решается пробой
0041-06 до начала 0041-03.** Это самый рискованный пункт задачи: неверный выбор
даёт ST, который компилируется, но ведёт себя не как модель, — то есть тихое
расхождение, худший класс дефекта по меркам проекта (0025).

**О2.** Нужен ли отдельный `_reset`? В C он есть (`stacker.c:441-444`), в ST
холодный рестарт ПЛК сам обнуляет `VAR`. **Предложение:** не эмитить; при
необходимости — вход `reset : BOOL` в `VAR_INPUT`. Решение — при реализации.

**О3.** `is_done` для корневой модели: выход FB или отдельная `PROGRAM`-логика?
**Предложение:** единообразно — `VAR_OUTPUT is_done` у **каждого** FB (S11).

## Требования (детализация R6, R7)

- **R6.1.** Каждая модель → отдельный `FUNCTION_BLOCK`; имя — как в C
  (`normalize_camelcase_name`, `Stacker` + `LiftController` → `StackerLiftController`).
- **R6.2.** `CASE state OF` содержит ветвь на каждое состояние + синтетические
  `INIT` (значение 0) и `END`.
- **R6.3.** `enter` целевого состояния инлайнится в переход (S7) — **изоморфно C**
  (Ф4).
- **R6.4.** Порядок `ref` сохраняется как порядок `IF`/`ELSIF` (S9) — **изоморфно
  C** (Ф5).
- **R7.1.** `M1 | M2` → последовательные вызовы под-FB в одном скане, в порядке
  объявления; завершение — конъюнкция `is_done` (S12) — **изоморфно C** (Ф6).
- **R7.2.** `M1 + M2` → шаги `CASE` родителя (S13).
- **R7.3.** Разделение переменных корня между параллельными под-моделями
  сохраняет **порядок видимости записей внутри такта**, как в C (О1).

## Критерии приёмки (детализация A2, A6)

| # | Критерий | Способ проверки |
|---|---|---|
| A2.1 | `-t st examples/stacker.lam` → 4 `FUNCTION_BLOCK`: `Stacker`, `StackerCommandReceiver`, `StackerMovementController`, `StackerLiftController` | Интеграционный тест на подстроки |
| A2.2 | Каждый FB содержит `CASE state OF` … `END_CASE;` | Тест |
| A6.1 | ST-ветвь `LiftOperating` содержит **три** условия в том же порядке, что `stacker.c:82-101`, с тем же инлайном `enter` | Сверка с зондом C, зафиксированным в ADR и здесь (Ф5) |
| A6.2 | ST-ветвь `INIT` `LiftController` содержит `cmd_fork := FALSE;` перед переходом (Ф3) | Тест |
| A6.3 | Корневой FB `Stacker` вызывает три под-FB в порядке `command_receiver0`, `movement_controller1`, `lift_controller2` и завершается по конъюнкции `is_done` (Ф6) | Тест |
| A6.4 | `exit`-блок отображается согласно **снятому зонду** (не предположению) | Зонд на фикстуре с `exit` → затем assertions |
| A6.5 | Порядок видимости записей в такте между параллельными моделями совпадает с C | Сверка трасс на `stacker.lam`; **только установившиеся значения** — потактовая сверка невозможна из-за `INIT`-тактов (фича 0033) |
