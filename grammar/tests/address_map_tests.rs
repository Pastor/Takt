//! Интеграционные тесты внешней карты адресов (фича 0020-03).
//!
//! Проверяют наложение внешней `.ld`-подобной карты на семантическую модель:
//! предупреждения об оверлее (SE-050) и висячих записях (SE-051). Разбор самого
//! формата покрыт юнит-тестами модуля `grammar::address_map`.

use grammar::semantic::tree::construct_model;
use grammar::{address_map_overlay_warnings, parse, parse_address_map};

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
