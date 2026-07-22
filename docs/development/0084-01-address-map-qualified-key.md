# Задача 0084-01: Ключ карты адресов — квалифицированный (модель + порт)

> Фича: [../features/0084-address-map-qualified-key.md](../features/0084-address-map-qualified-key.md) · ADR: [../adr/0084-address-map-qualified-key.md](../adr/0084-address-map-qualified-key.md) · анализ: [../analyze/0084-address-map-qualified-key.md](../analyze/0084-address-map-qualified-key.md)

## Что было

`AddressResolution.map` — `HashMap<голое_имя_порта, ResolvedAddress>`;
`resolve_model` вставлял по `name.clone()`. Потребитель `c-hal` строил
квалифицированный enum-вариант `{МОДЕЛЬ}_{ПОРТ}`, но адрес брал по голому имени
(`addr_map.get(port_name)`). Одноимённые порты разных под-моделей делили ключ —
адрес первого терялся при вставке.

## Что сделано

Реализована **Option A** [ADR 0084](../adr/0084-address-map-qualified-key.md).

- **Ключ карты квалифицирован** (`address_map/resolve.rs`): хелвер
  `qualified_port_key(model_unique, port)` (разделитель `\u{1}`). Продюсер
  `resolve_model` строит `model_unique` реплицированным `model_unique_name`
  (обход `upper`, совпадает байт-в-байт с `minimap::unique_model_name` и, значит,
  с `Name::unique()` у потребителей — драйвер 3 ADR).
- **`ResolvedAddress += name`** (голое имя порта): для потребителей
  пользовательского имени. Заполняется во всех трёх ветвях (External/Operator/
  Inline).
- **`SE-051`** теперь проверяет наличие голого имени **среди значений**
  (`values().any(|r| r.name == e.name)`), а не `contains_key` (ключ квалифицирован;
  внешняя `.ld` адресует по голому имени).
- **Потребители:**
  - `c-hal` (`c_hal.rs`): lookup по `qualified_port_key(model_name.unique(), port)`.
  - `st-at` (`st/mod.rs`): lookup по квалиф. ключу per-модель (`blocks` несёт
    `Name`) — st-at обходит **все** модели, поэтому голый lookup промахнулся бы.
  - `sv-mmio` (`sv_mmio.rs`): имя регистра — `resolved.name` (голое), не ключ.
  - экспорт `map`/`json` (`address_map/export.rs`): плоский дедуп по голому имени
    (`sorted_entries` схлопывает одноимённые, побеждает макс. квалиф. ключ =
    последняя под-модель) — круговой рейс `.ld` (R4 0043) остаётся тождеством,
    выгрузка идёт по `resolved.name`.

| Стек | Статус |
|---|---|
| `grammar` продюсер (`resolve.rs`) | ✅ квалиф. ключ + `name` + SE-051 |
| `grammar` потребители (c-hal/st-at/sv-mmio/export) | ✅ согласованы |
| `simulation` | н/п — симулятор адреса не потребляет |
| язык `.lam` | н/п — не меняется |

## Проверки

- `cargo test -p grammar address` → 59 passed (включая обновлённые
  `flat_key_collision_last_wins` и `export_addresses_match_chal_table`).
- Новый сторож `c_hal::address_collision_qualified_key_distinct_addresses` →
  `COLL_A_SIG = 0x10`, `COLL_B_SIG = 0x20` (до 0084 оба брали 0x20).
- `cargo clippy -p grammar --all-targets --all-features -D warnings` → чисто.
- Аддитивность: перегенерация `sv-mmio/stacker` → `git diff` пуст (байт-в-байт).
- Полный `./scripts/precheck.sh` → зелёный (см.
  [отчёт](../reports/0084-address-map-qualified-key.md)).
