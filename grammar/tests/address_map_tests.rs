//! Интеграционные тесты внешней карты адресов (фича 0020-03).
//!
//! Проверяют наложение внешней `.ld`-подобной карты на семантическую модель:
//! предупреждения об оверлее (SE-050) и висячих записях (SE-051). Разбор самого
//! формата покрыт юнит-тестами модуля `grammar::address_map`.

use grammar::semantic::tree::construct_model;
use grammar::{
    AddressSource, address_map_overlay_warnings, parse, parse_address_map, resolve_addresses,
};

/// Строит семантическую модель из исходника Lam.
fn model_of(src: &str) -> std::rc::Rc<std::cell::RefCell<grammar::semantic::ModelNode>> {
    let (ast, _) = parse(src, 0).expect("ошибка разбора .lam");
    construct_model(&ast, None, &[]).expect("ошибка построения модели")
}

/// Коды всех предупреждений оверлея для пары (модель, карта).
fn overlay_codes(model_src: &str, map_src: &str) -> Vec<String> {
    let model = model_of(model_src);
    let entries = parse_address_map(map_src, 0).expect("карта должна разобраться");
    address_map_overlay_warnings(model, &entries)
        .into_iter()
        .map(|d| d.code.unwrap_or_default())
        .collect()
}

/// Карта переопределяет адрес, заданный inline (`:=`) → SE-050.
#[test]
fn overrides_inline_address_warns_se050() {
    let codes = overlay_codes(
        "type u8 = [bit;8]; in BTN: u8 := 0x00100000; start Idle;",
        "BTN = 0x00200000;",
    );
    assert_eq!(codes, vec!["SE-050"]);
}

/// Карта переопределяет адрес, заданный оператором `address` → SE-050.
#[test]
fn overrides_operator_address_warns_se050() {
    let codes = overlay_codes(
        "out LED: bit; address LED = 0x00100004; start Idle;",
        "LED = 0x00200004:3;",
    );
    assert_eq!(codes, vec!["SE-050"]);
}

/// Порт без адреса в модели: карта — единственный источник → без предупреждений.
#[test]
fn fills_port_without_model_address_is_silent() {
    let codes = overlay_codes(
        "type u8 = [bit;8]; in BTN: u8; start Idle;",
        "BTN = 0x00200000;",
    );
    assert!(
        codes.is_empty(),
        "не должно быть предупреждений: {:?}",
        codes
    );
}

/// Запись карты для несуществующего порта → SE-051.
#[test]
fn dangling_map_entry_warns_se051() {
    let codes = overlay_codes(
        "type u8 = [bit;8]; in BTN: u8; start Idle;",
        "GHOST = 0x00200000;",
    );
    assert_eq!(codes, vec!["SE-051"]);
}

/// Смешанный случай: оверлей + висячая запись — оба предупреждения.
#[test]
fn mixed_overlay_and_dangling() {
    let mut codes = overlay_codes(
        "type u8 = [bit;8]; in BTN: u8 := 0x00100000; start Idle;",
        "BTN = 0x00200000; GHOST = 0x00200008;",
    );
    codes.sort();
    assert_eq!(codes, vec!["SE-050", "SE-051"]);
}

// ───────────────────────── Резолвер AddressMap (0020-05) ─────────────────────

/// Только inline-адрес → источник Inline, значение понижено.
#[test]
fn resolve_inline_only() {
    let model = model_of("type u8 = [bit;8]; in BTN: u8 := 0x00200000; start Idle;");
    let r = resolve_addresses(model, &[]);
    let a = r.map.get("BTN").expect("BTN должен быть разрешён");
    assert_eq!(a.addr, 0x0020_0000);
    assert_eq!(a.bit, None);
    assert_eq!(a.source, AddressSource::Inline);
    assert!(r.diagnostics.is_empty());
}

/// Только оператор `address` → источник Operator; форма `:bit` понижается.
#[test]
fn resolve_operator_with_bit() {
    let model = model_of("out LED: bit; address LED = 0x00200004:3; start Idle;");
    let r = resolve_addresses(model, &[]);
    let a = r.map.get("LED").expect("LED должен быть разрешён");
    assert_eq!(a.addr, 0x0020_0004);
    assert_eq!(a.bit, Some(3));
    assert_eq!(a.source, AddressSource::Operator);
}

/// Внешняя карта перекрывает inline → источник External + предупреждение SE-050.
#[test]
fn resolve_external_overrides_inline() {
    let model = model_of("type u8 = [bit;8]; in BTN: u8 := 0x00100000; start Idle;");
    let entries = parse_address_map("BTN = 0x00200000;", 0).unwrap();
    let r = resolve_addresses(model, &entries);
    let a = r.map.get("BTN").unwrap();
    assert_eq!(a.addr, 0x0020_0000);
    assert_eq!(a.source, AddressSource::External);
    assert_eq!(
        r.diagnostics
            .iter()
            .filter(|d| d.code.as_deref() == Some("SE-050"))
            .count(),
        1
    );
}

/// Используемый порт без адреса → ошибка полноты SE-052.
#[test]
fn resolve_used_port_without_address_is_se052() {
    let model = model_of("in BTN: bit; start S { ref T: BTN; } state T;");
    let r = resolve_addresses(model, &[]);
    assert!(!r.map.contains_key("BTN"));
    assert_eq!(
        r.diagnostics
            .iter()
            .filter(|d| d.code.as_deref() == Some("SE-052"))
            .count(),
        1
    );
}

/// Внешняя карта закрывает used-порт без адреса модели → нет SE-052.
#[test]
fn resolve_external_fills_used_port() {
    let model = model_of("in BTN: bit; start S { ref T: BTN; } state T;");
    let entries = parse_address_map("BTN = 0x00200000;", 0).unwrap();
    let r = resolve_addresses(model, &entries);
    assert_eq!(r.map.get("BTN").unwrap().source, AddressSource::External);
    assert!(
        r.diagnostics
            .iter()
            .all(|d| d.code.as_deref() != Some("SE-052"))
    );
}

/// Висячая запись карты → SE-051.
#[test]
fn resolve_dangling_external_is_se051() {
    let model = model_of("type u8 = [bit;8]; in BTN: u8 := 0x1; start Idle;");
    let entries = parse_address_map("GHOST = 0x00200000;", 0).unwrap();
    let r = resolve_addresses(model, &entries);
    assert_eq!(
        r.diagnostics
            .iter()
            .filter(|d| d.code.as_deref() == Some("SE-051"))
            .count(),
        1
    );
}
