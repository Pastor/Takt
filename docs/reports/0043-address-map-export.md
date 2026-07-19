# Отчёт о тестировании — Фича 0043: Экспорт карты адресов во внешний формат

- **Фича:** [0043](../features/0043-address-map-export.md)
- **ADR:** [0043](../adr/0043-address-map-export.md) · **Анализ:** [0043](../analyze/0043-address-map-export.md) · **Тест-план:** [0043](../tests/0043-address-map-export.md)
- **Дата:** 2026-07-19
- **Вердикт:** ✅ **ГОТОВО**. `./scripts/precheck.sh` — EXIT=0; тесты 2082 (default) / 2183 (`lsp`); гейт размеров зелёный.

## Сводка

Реализована подкоманда `lamc address-map --emit map|json`: выгружает
**фактически разрешённую** карту адресов портов (тем же `resolve_addresses`, что
потребляет `-t c-hal`) в два формата. Формат `map` (`.ld`-подобный) **замыкается**
через `--address-map` (круговой рейс байт-в-байт); `json` — версионированная
машиночитаемая выгрузка с типом, направлением и источником адреса. CMSIS-SVD не
поставляется (решение ADR) — `--emit svd` даёт внятную ошибку.

## Окружение

| Компонент | Значение |
|---|---|
| Ядро | `grammar/src/address_map/` — `resolve` (обогащён), `export` (эмиттеры map/json), `export_cli` (CLI-обвязка) |
| CLI | `lamc address-map` (тонкий диспетчер в `bin/lamc.rs` → `run_export_subcommand`) |
| Тесты | `grammar/tests/address_export_tests.rs` (15) + юниты `export_cli` (11) |
| Фикстуры | `grammar/tests/data/address_export/` — `probe.lam`, `plat.map`, `ghost.map`, `broken.map`, `dead_port.lam`, `collide.lam` |

## Сверка с тест-планом

| # | Проверка | Результат |
|---|---|---|
| T1 | `--emit` по умолчанию `map` | ✅ `cli_default_emit_is_map` + `args_default_emit_is_map` |
| T2/T3/T5 | Три источника, бит сохранён | ✅ `map_export_three_sources_probe` (`BTN=inline, LED=address, SW=inline:3`) |
| T4 | Карта бьёт модель | ✅ `map_export_external_overrides_model` |
| T6 | Круговой рейс разбирается без `AM-*` | ✅ `round_trip_reparses_without_diagnostics` |
| T7 | Круговой рейс байт-в-байт | ✅ `round_trip_is_byte_identical` |
| T8 | Рейс не сохраняет источник (всё `external`) | ✅ (семантика оверлея; проверено пробой json после реимпорта) |
| T9 | Сверка адресов с `c-hal __ADDR[]` | ✅ `export_addresses_match_chal_table` (значения сверены пробой `-t c-hal`) |
| T10/T11/T12 | `json` валиден, полон, версионирован | ✅ `json_export_is_valid_complete_and_versioned` (`serde_json::from_str`) |
| T14 | Мёртвый порт: нет в `map`, `null` в `json`, **нет `0x0`** | ✅ `dead_port_absent_in_map_and_null_in_json` |
| T15 | Достижимый порт без адреса → `SE-052`, rc≠0 | ✅ `cli_reachable_port_without_address_is_se052` (elevator_mini, 34×SE-052) |
| T16/T17 | Предупреждения в stderr; stdout — чистая карта; `GHOST` не в выгрузке | ✅ `cli_warnings_go_to_stderr_not_stdout` |
| T18 | `--emit svd` → ошибка, упоминание SVD | ✅ `cli_unknown_format_is_rejected` + `emit_format_rejects_svd_with_mention` |
| T19 | Вывод в файл `-o` = stdout-варианту | ✅ `cli_output_to_file_matches_stdout` |
| T20 | Корпусный круговой рейс = тождество | ✅ `corpus_round_trip_is_identity` |
| T21 | Плоский ключ: последний побеждает | ✅ `flat_key_collision_last_wins` (зафиксировано, сигнал при исправлении Р2) |
| T22 | Регресс `compile`/`fmt`/тесты/precheck | ✅ EXIT=0, вывод всех целей байт-в-байт прежний |
| K5 | Битая входная карта → `AM-002`, rc≠0 | ✅ `cli_broken_input_map_is_rejected` |

## Примеры и контрпримеры (формат — правило 16)

- **П1** (`probe.lam` + `plat.map`) — выгрузка `map`:
  ```
  BTN = 0x40000000;
  LED = 0x00200004;
  SW = 0x00300000:3;
  ```
  сверена пробой с таблицей `c-hal`: `PROBE_BTN=0x40000000`, `PROBE_LED=0x200004`,
  `PROBE_SW=0x300000:3`.
- **П3** — та же модель в `json`: на каждый порт имя/тип/направление/адрес/бит/
  источник + `format`/`format_version`.
- **К2** (`dead_port.lam`) — `DEAD` без адреса: в `map` **нет** записи и `0x0` не
  появляется; в `json` — `"address": null`.
- **К4** — `--emit svd` → «формат CMSIS-SVD не поставляется — у Lam нет требуемых
  им данных».

## Находки

- **Обогащение `ResolvedAddress`** (тип/направление) заполняется тем же обходом,
  что и адрес — второго прохода по модели нет (Action item ADR). Мёртвые порты
  записываются в новое поле `AddressResolution::address_less` там же.
- **Размер модуля.** `bin/lamc.rs` пришпилен к baseline (2039) — новый код
  подкоманды вынесен в библиотечный `address_map/export_cli.rs`; в бинарнике —
  тонкий диспетчер. Побочно `split_include_dirs` переехал в библиотеку (его нужна
  и CLI-обвязке); baseline `lamc.rs` понижён 2039 → 2009.
- **Плоский ключ (Р2)** — задокументирован и зафиксирован тестом `T21`, не чинится
  (правка ключа задевает `c-hal`, вне объёма 0043).

## Дефекты

Не найдено. Фиксы (`docs/fixes/0043-YY-*`) не заводились.

## Итог

Критерии A1–A10 и требования R1–R11 выполнены. Версия языка не менялась (0.2.0);
крейт `grammar` — минорный бамп 0.5.0 → 0.6.0 (добавлен публичный API). Фича
закрыта.
