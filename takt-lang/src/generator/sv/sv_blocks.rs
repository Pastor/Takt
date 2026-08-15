//! Эмиссия именованных блоков (`enter`/`always`/`exit`) и охранных формул в
//! цель SystemVerilog.
//!
//! Вынесено из `sv_fsm.rs` (фича 0083: лимит размера модуля). Два источника
//! блоков — состояние и **сама модель** (model-level `always`, фича 0083);
//! обе печати идут в `always_comb` над `_next`-сигналами.
//!
//! Охранные формулы (фича 0235) печатаются **там же, где их печатает цель `c`**:
//! формулы модели — перед `unique case`, формулы состояния — в начале его ветви.

use crate::diagnostics::Diagnostic;
use crate::generator::indent::Printer;
use crate::generator::sv::sv_expr::print_condition;
use crate::generator::sv::sv_fsm::Fsm;
use crate::generator::sv::sv_stmt::print_statement;
use crate::semantic::formula::Formula;
use crate::semantic::{ModelNode, StateNode};

/// Печатает именованные блоки состояния (`enter`/`always`/`exit`).
pub(crate) fn emit_named_blocks(
    p: &mut Printer,
    state: &StateNode,
    fsm: &Fsm,
    block: &str,
) -> Result<(), Diagnostic> {
    for b in state.get_named_blocks(block) {
        if let Some(stmt) = b.statement() {
            print_statement(p, stmt, &fsm.scope())?;
        }
    }
    Ok(())
}

/// Печатает именованные блоки **уровня модели** (фича 0083): `always` вне
/// состояния. Аналог [`emit_named_blocks`], но источник — сама модель.
pub(crate) fn emit_model_named_blocks(
    p: &mut Printer,
    model: &ModelNode,
    fsm: &Fsm,
    block: &str,
) -> Result<(), Diagnostic> {
    for b in model.get_named_blocks(block) {
        if let Some(stmt) = b.statement() {
            print_statement(p, stmt, &fsm.scope())?;
        }
    }
    Ok(())
}

/// Преамбула ветви состояния: охранные формулы, затем блоки `always`.
///
/// Одна функция вместо двух вызовов подряд — потому что порядок здесь **часть
/// контракта**: проверка стоит ДО действий такта, как `assert` в начале `_tick`
/// цели `c`. Разведи их по вызывающему коду — и следующая правка переставит их
/// местами, не заметив, что меняет семантику.
pub(crate) fn emit_state_prelude(
    p: &mut Printer,
    map: &crate::generator::sv::sv_map::SvMap,
    state: &StateNode,
    fsm: &Fsm,
) -> Result<(), Diagnostic> {
    if map.guard_enable() {
        emit_state_guards(p, state, fsm)?;
    }
    emit_named_blocks(p, state, fsm, "always")
}

/// Преамбула уровня модели: охранные формулы уровня модели, затем model-level
/// `always` (фича 0083). Порядок — тот же и по той же причине.
pub(crate) fn emit_model_prelude(
    p: &mut Printer,
    map: &crate::generator::sv::sv_map::SvMap,
    model: &ModelNode,
    fsm: &Fsm,
) -> Result<(), Diagnostic> {
    emit_model_named_blocks(p, model, fsm, "always")?;
    if map.guard_enable() {
        emit_model_guards(p, model, fsm)?;
    }
    Ok(())
}

/// Печатает охранные формулы **состояния** (фича 0235).
///
/// До этой фичи цель `sv` формулы не печатала вовсе — ни при опечатке, ни при
/// верном имени: автор получал прошивку FPGA **без** объявленного им средства
/// безопасности, а компилятор рапортовал об успехе (находка фичи 0203).
fn emit_state_guards(p: &mut Printer, state: &StateNode, fsm: &Fsm) -> Result<(), Diagnostic> {
    for formula in state.formulas() {
        emit_guard(p, formula, fsm)?;
    }
    Ok(())
}

/// Печатает охранные формулы **модели** — перед `unique case`, как в цели `c`.
fn emit_model_guards(p: &mut Printer, model: &ModelNode, fsm: &Fsm) -> Result<(), Diagnostic> {
    for formula in &model.formulas {
        emit_guard(p, formula, fsm)?;
    }
    Ok(())
}

/// Печатает одну формулу как immediate assertion.
///
/// ⚠️ **Форма выбрана пробой, а не вкусом** (2026-08-15): `assert (условие);`
/// принимают ОБА инструмента гейта, а `assert (условие) else $error("…");`
/// verilator принимает, но **yosys отвергает** (`syntax error, unexpected
/// TOK_ELSE`). Поэтому имя инварианта (фича 0044) в цель `sv` **не переносится**:
/// цель `rust` его печатает, `c` игнорирует, `sv` не может по устройству
/// синтезатора. Добавите `else` — гейт покраснеет на yosys, пройдя verilator.
///
/// ⚠️ Место — `always_comb`, рядом с телом уровня. Условие **читает регистры**
/// (`Scope::registered`: чтение из `name`, запись в `name_next`), то есть
/// значения, устойчивые в пределах такта, — комбинационных ложных срабатываний
/// не будет. Синтез не задет: yosys кладёт проверку в ячейку `$check`, а
/// логика остаётся прежней (проба: 8 `$_DFF_PN0_` с проверкой и без).
fn emit_guard(p: &mut Printer, formula: &Formula, fsm: &Fsm) -> Result<(), Diagnostic> {
    match formula {
        // Имя инварианта не печатается — см. оговорку о yosys выше.
        Formula::Guard(cond, _) => {
            let text = print_condition(cond, &fsm.scope())?;
            if !text.is_empty() {
                p.ident(&format!("assert ({});", text)).nl();
            }
            Ok(())
        }
        Formula::Formulas(items) => {
            for item in items {
                emit_guard(p, item, fsm)?;
            }
            Ok(())
        }
        // LTL цель `sv` не верифицирует (как и прочие цели): о ней говорит
        // SE-055 из семантики. Пустая формула объявлением не является.
        Formula::LTL(_) | Formula::None => Ok(()),
    }
}
