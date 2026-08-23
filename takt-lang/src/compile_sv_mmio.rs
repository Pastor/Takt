//! Публичная точка входа цели `sv-mmio` (фича 0062).
//!
//! Вынесена из `lib.rs` (тот пришпилен к лимиту размера, реестр долга): в корне
//! крейта остаётся тонкий реэкспорт [`compile_to_sv_mmio`]. Функция соединяет два
//! существующих образца — разрешение адресов `c-hal` (`resolve_addresses` +
//! приоритет источников) и понижение `float → q` цели `sv`.

use crate::address_map::{self, AddressEnv, AddressMapEntry};
use crate::diagnostics::{self, Diagnostic};
use crate::generator::{self, GenerateOptions};
use crate::{apply_float_lowering, parse_and_construct};
use std::path::Path;

/// Компилирует Takt в синтезируемый SystemVerilog в режиме `sv-mmio` (фича 0062):
/// порт **с** адресом становится битом регистрового файла на шинно-агностичном
/// интерфейсе, порт **без** адреса — портом модуля. Парная цель к
/// [`compile_to_sv`](crate::compile_to_sv), как `c-hal` парная к `c`.
///
/// Адреса разрешаются тем же слоем, что и у `c-hal`
/// ([`resolve_addresses`](address_map::resolve_addresses), приоритет inline <
/// `address` < внешняя карта), поэтому цель принимает `--address-map` (правило 7
/// ADR 0062). Ошибка разрешения (например `SE-060` — бит вне `[0, 63]`) —
/// немедленный отказ. Возвращает предупреждения оверлея/висячих записей карты
/// (как `compile_to_c_hal`).
///
/// В отличие от `c-hal`, флаг [`GenerateOptions::hal`] **не** взводится: режим
/// регистрового файла выбирает вариант
/// [`Language::SvMmio`](generator::Language::SvMmio), а адреса читаются из
/// [`GenerateOptions::address_map`].
#[allow(clippy::too_many_arguments)]
pub fn compile_to_sv_mmio(
    filename: &str,
    source: &str,
    output_path: &str,
    search_paths: &[String],
    external: &[AddressMapEntry],
    env: &AddressEnv,
    options: &GenerateOptions,
) -> Result<Vec<Diagnostic>, Diagnostic> {
    let unit = parse_and_construct(filename, source, search_paths, options.specialize)?;

    if unit.model.borrow().name.is_none() {
        let stem = Path::new(filename)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.split('.').next().unwrap_or(s).to_owned())
            .unwrap_or_else(|| "Root".to_owned());
        unit.model.borrow_mut().name = Some(stem);
    }

    // Порт составного типа разворачивается в скалярные (фича 0390): у
    // регистрового файла поле ложится в своё слово, и `SV-002` о «ширине, не
    // определённой в битах» становится недостижим.
    crate::semantic::condition::observe::lower_for_target(&unit.model, true, false)?;

    // Разрешаем адреса (inline < address < внешняя карта) — тот же слой, что у
    // `c-hal`/`st-at`. `SE-060` (бит вне [0, 63]) → отказ.
    let mut resolution =
        address_map::resolve_addresses(std::rc::Rc::clone(&unit.model), external, env);

    // ⚠️ `SE-052` («used-порт без адреса») для `sv-mmio` — НЕ ошибка: порт без
    // адреса штатно становится портом модуля (правило 2 ADR 0062), в отличие от
    // `c-hal`, где адрес обязателен каждому порту. Поэтому `SE-052` снимается —
    // и из фатальной проверки, и из возвращаемых предупреждений. Прочие ошибки
    // (`SE-060` бит вне диапазона, `SE-054`/`SE-055` сломанное выражение адреса)
    // остаются фатальными.
    resolution
        .diagnostics
        .retain(|d| d.code.as_deref() != Some("SE-052"));

    if let Some(err) = resolution
        .diagnostics
        .iter()
        .find(|d| d.level == diagnostics::Level::Error)
    {
        return Err(err.clone());
    }

    let mut mmio_options = options.clone();
    mmio_options.address_map = resolution.map;

    // Фича 0096: `float → q(m, n)` при `--float-as-q` — как у `sv` (без
    // `--float-embedded`, третий аргумент `false`).
    apply_float_lowering(&unit.model, options, false)?;

    // Предупреждения генератора (`SV-009`) присоединяются к адресным (фича
    // 0168). Прежде у одного вызова было **две** судьбы: адресные возвращались
    // и глушились `--quiet`, а `SV-009` печаталась `eprintln!` из библиотеки и
    // не глушилась ничем.
    let mut warnings = resolution.diagnostics;
    warnings.extend(unit.emit(generator::Language::SvMmio, output_path, &mmio_options)?);

    Ok(warnings)
}
