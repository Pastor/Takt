//! Перечисления состояний и шагов у цели `sv` (вынесено фичей 0367).
//!
//! Печать `typedef enum` для состояний каждого уровня и для ступеней
//! последовательной композиции. Вынесено из `sv_fsm.rs` по правилу размера
//! модуля: границы модулей — границы ответственности, и печать перечислений
//! от сборки автомата не зависит.

use crate::diagnostics::Diagnostic;
use crate::generator::indent::Printer;
use crate::generator::sv::sv_fsm::{
    Block, Fsm, state_enum_name, state_variants, step_enum_name, step_variant,
};
use crate::generator::sv::sv_map::SvMap;
use crate::generator::sv::sv_names;
use crate::generator::sv::sv_type::enum_width;
use crate::semantic::minimap::Element;

/// Печатает перечисления состояний всех уровней.
pub(crate) fn emit_state_enums(
    p: &mut Printer,
    map: &SvMap,
    blocks: &[Block],
) -> Result<(), Diagnostic> {
    for (name, _) in blocks {
        let Some(Element::Model { states, .. }) = map.model_element_of(name) else {
            continue;
        };
        // Алфавит имени состояния (фича 0200): перечислитель печатается здесь,
        // и без этой проверки не-ASCII имя доехало бы до `verilator`. ⚠️ Дыру
        // нашёл тест по видам объявлений, а не чтение: у переменных, портов и
        // функций проверка была, у состояний — нет.
        sv_names::check_state_names(map, name)?;
        let variants = state_variants(name, &states);
        // Ширина — по диапазону значений (задача 0045-03). Значения назначает
        // генератор (0..n-1), поэтому формула вырождается в ⌈log₂(n)⌉ — то есть
        // совпадает с формулой ADR именно здесь, где та была верна.
        let numbered: Vec<(String, i128)> = variants
            .iter()
            .enumerate()
            .map(|(i, v)| (v.clone(), i as i128))
            .collect();
        let (width, _) = enum_width(&numbered, &format!("состояния модели '{}'", name))?;
        p.ident(&format!(
            "// Состояния модели '{}'. Синтетического INIT нет: стартовое",
            name
        ))
        .nl();
        p.ident("// состояние живёт в ветви сброса (контракт ADR 0033).")
            .nl();
        p.ident(&format!("typedef enum logic [{}:0] {{", width - 1))
            .nl();
        p.up();
        for (i, (variant, value)) in numbered.iter().enumerate() {
            let comma = if i + 1 == numbered.len() { "" } else { "," };
            p.ident(&format!("{} = {}'d{}{}", variant, width, value, comma))
                .nl();
        }
        p.down();
        p.ident(&format!("}} {};", state_enum_name(name))).nl().nl();
    }
    Ok(())
}

/// Печатает перечисления шага для цепочек `+` (задача 0057-01).
///
/// Значения назначает генератор (0..n-1), поэтому ширина — ⌈log₂(n)⌉, как у
/// перечислений состояний. Порядок — обхода `Fsm::build` (детерминизм 0048).
pub(crate) fn emit_step_enums(p: &mut Printer, fsm: &Fsm) -> Result<(), Diagnostic> {
    for (state, count) in &fsm.step_enums {
        let numbered: Vec<(String, i128)> = (0..*count)
            .map(|i| (step_variant(state, i), i as i128))
            .collect();
        let (width, _) = enum_width(&numbered, &format!("шаг цепочки '{}'", state))?;
        p.ident(&format!(
            "// Шаг последовательной композиции '{}' (`+`).",
            state
        ))
        .nl();
        p.ident(&format!("typedef enum logic [{}:0] {{", width - 1))
            .nl();
        p.up();
        for (i, (variant, value)) in numbered.iter().enumerate() {
            let comma = if i + 1 == numbered.len() { "" } else { "," };
            p.ident(&format!("{} = {}'d{}{}", variant, width, value, comma))
                .nl();
        }
        p.down();
        p.ident(&format!("}} {};", step_enum_name(state))).nl().nl();
    }
    Ok(())
}
