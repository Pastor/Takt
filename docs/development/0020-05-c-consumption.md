# Задача 0020-05: потребление адреса в C (цель `c-hal`)

> Фича: [../features/0020-port-address-decl.md](../features/0020-port-address-decl.md) · ADR: [../adr/0020-port-address-decl.md](../adr/0020-port-address-decl.md) · анализ: [../analyze/0020-port-address-decl.md](../analyze/0020-port-address-decl.md)

> **Статус:** ВЫПОЛНЕНО (резолв `AddressMap` + цель `c-hal` + эмиссия таблицы и
> дефолтного HAL; вывод компилируется `cc -std=c99 -Wall`; тесты зелёные).

## Решения заказчика

- **Форма вывода:** таблица адресов + дефолтный HAL (Вариант 1), деградация 2→3 по
  осуществимости.
- **Гейтинг:** новая цель `-t c-hal`; цель `c` — байт-в-байт как прежде (регресс = 0).
- **Внешняя карта:** участвует в резолве `AddressMap` для `c-hal`.

## Что сделано (факт)

### Резолв `AddressMap` (`address_map.rs`)

- `resolve_addresses(model, external) -> AddressResolution`: приоритет
  **inline < `address` < внешняя карта**; понижение выражения-адреса в `(addr,
  bit)` (`lower_addr_expr`); источник-победитель (`AddressSource`).
- Диагностики: **SE-052** (used-порт без адреса → ошибка полноты), **SE-050**
  (оверлей карты), **SE-051** (висячая запись). Конфликт inline+`address` уже
  исключён семантикой (SE-049).

### Пламбинг режима `c-hal`

- `generator/mod.rs` `GenerateOptions`: поля `hal: bool` и
  `address_map: HashMap<String, ResolvedAddress>` (обновлены `new`/`Default`).
- `lib.rs` `compile_to_c_hal(filename, source, out, search_paths, external,
  options) -> Result<Vec<Diagnostic>, Diagnostic>`: разрешает адреса, при
  SE-052-ошибке возвращает `Err`, иначе включает `hal` + карту и генерирует C;
  возвращает предупреждения (SE-050/051).
- CLI: цель `-t c-hal`; `--address-map` протянут в резолв; справка дополнена.

### Эмиссия C (`generator/c/c_header.rs`)

- В режиме `hal` перед `#endif` эмитится блок: тип `{Root}_PortBinding { uintptr_t
  addr; int8_t bit; uint8_t width; }`, таблицы `static const {Enum}__ADDR[]`
  (индексы — те же enum-варианты), дефолтные `read_*`/`write_*` через
  `*(volatile T*)addr` (ширина из `get_c_type`; бит `<0` = весь регистр), и
  `{Root}_bind_default_hal({Root} *m)` (связывает только присутствующие
  классы/направления).
- Для одиночной модели без под-моделей в режиме `hal` добавляется `typedef struct
  {Root} {Root};` — иначе прототипы `{Root}_init({Root} *)` невалидны в C. Режим
  `c` не затронут.

## Проверки

- Сквозной прогон `lamc … -t c-hal --address-map`: оверлей BTN→`0x40000000`
  (SE-050), inline `:bit`, оператор `address`, дефолтный HAL.
- **Вывод компилируется** `cc -std=c99 -Wall -c` без ошибок.
- Режим `c` — 0 HAL-артефактов (регресс = 0, подтверждено тестом и codegen-тестами).
- Тесты: `address_map_tests.rs` (резолвер — 6), `codegen_tests.rs` (c-hal — 4:
  эмиссия/регресс/SE-052/оверлей), CLI-разбор `-t c-hal`.
- `cargo test --features lsp -- --test-threads=1` — все зелёные; fmt/clippy чисты.

## Найденное попутно (вне 0020)

Базовый генератор `c` **не** эмитит `typedef struct {Root} {Root};` для одиночной
модели без под-моделей → её `.h` не компилируется как отдельный C (прототипы
`{Root}_init({Root} *)`). В `c-hal` исправлено локально; для цели `c` —
пред­существующий баг, оформить отдельно (кандидат в `FEATURES.md`).

## Границы

- Ширина bit-порта в таблице информативна (`int`→4); дефолтные bit-функции всегда
  используют `uint8_t` + извлечение бита.
- Оверлей/таблица покрывают порты корневой модели (и под-модели через
  `collect_ports_by_class`); резолв полноты — по достижимости из `unused.rs`.
