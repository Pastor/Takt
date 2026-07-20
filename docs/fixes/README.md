# Реестр исправлений

Стадия 7 жизненного цикла (правило 17 в [../RULE.md](../RULE.md)). Исправления
`XXXX-YY-slug.md` заводятся при необходимости (по результатам тестирования) и
проходят весь путь фичи. `XXXX` — номер исходной фичи, `YY` — номер фикса.

Заготовка создаётся из шаблона [`../templates/fixes.md`](../templates/fixes.md).

| Фикс | Фича | Заголовок | Документ |
|------|------|-----------|----------|
| 0010-01 | 0010 | Вырожденное условие принятия автомата Бюхи (GPVW) | [0010-01-buchi-acceptance.md](0010-01-buchi-acceptance.md) |
| 0022-01 | 0022 | Открытый верхний диапазон совместимости IDE (until-build) | [0022-01-untilbuild-open-range.md](0022-01-untilbuild-open-range.md) |
| 0003-01 | 0003 (+0050) | Тихая мистрансляция `+` внутри `\|` в целях C и Rust — ЗАВЕДЁН (Tier 1, молчаливо неверный вывод) | [0003-01-concat-in-parallel-silent.md](0003-01-concat-in-parallel-silent.md) |
| 0023-01 | 0023 | Проверка совместимости с новыми IDE (verifyPlugin) + валидность дескриптора | [0023-01-verifyplugin-descriptor.md](0023-01-verifyplugin-descriptor.md) |
| 0045-01 | 0045 | Устаревшие стабы SV в каталоге вывода (доделка гейта 0045-02) — ИСПРАВЛЕН | [0045-01-stale-sv-artifacts.md](0045-01-stale-sv-artifacts.md) |
| 0053-01 | 0053 | `FileTable::default()` сталкивает первый импорт с корнем (`file_no` 0) — ЗАВЕДЁН, блокирует 0056-01 | [0053-01-file-table-default-collision.md](0053-01-file-table-default-collision.md) |
| 0005-01 | 0005 (+0029) | Цель `c` теряет знак перечисления — переход мёртв молча (Tier 1) — ЗАВЕДЁН, чинится в объёме 0060 | [0005-01-c-enum-signedness.md](0005-01-c-enum-signedness.md) |
| 0020-01 | 0020 | Бит адреса порта не проверяется — `c-hal` читает не тот бит (8…31) либо даёт UB (≥32); корпус содержит случай (Tier 1) — **ИСПРАВЛЕН** фичей [0098](../features/0098-port-bit-range-safe-hal.md) (SE-060 + HAL по слову + гейт c-hal); разблокировал 0062 | [0020-01-port-bit-out-of-range.md](0020-01-port-bit-out-of-range.md) |
| 0041-01 | 0041 | Цель `st` молча склеивает одноимённые `fn` разных моделей — автомат стоит; `iec2c` принимает (Tier 1) — ЗАВЕДЁН, чинится в объёме 0065 | [0041-01-st-fn-dedup-silent.md](0041-01-st-fn-dedup-silent.md) |
| 0057-01 | 0057 | Симулятор не исполняет композицию с переходом `next` — берёт `next` сразу (Tier 2, расхождение эталонов); SV верен (сверен с C) — ЗАВЕДЁН | [0057-01-sim-composition-next.md](0057-01-sim-composition-next.md) |
| 0097-01 | 0097 (+0096) | Пример ПИД переведён с явного `q(8, 8)` на прозрачный `float`; q формируется флагами сборки (`--float-as-q` для sv, `--float-embedded` для c-hal/st-at) — ИСПРАВЛЕН | [0097-01-pid-native-float.md](0097-01-pid-native-float.md) |
| 0038-01 | 0038 | `semantic_tokens` не классифицирует члены под-моделей (`fn`/`state` в `model X` → `variable`) — молчаливо неверная подсветка на реальных файлах (Tier 2) — ИСПРАВЛЕН | [0038-01-subtree-token-classification.md](0038-01-subtree-token-classification.md) |
