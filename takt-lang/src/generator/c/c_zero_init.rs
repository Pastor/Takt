//! Умолчание переменной у цели `c` (фича 0353).
//!
//! # Что было сломано
//!
//! `generate_model_init` **пропускал** переменную без инициализатора
//! (`ExpressionNode::None => continue`), поэтому после `<Root>_init` её поле
//! содержало то, что лежало в памяти. Замер 2026-08-21: эталон, `st`, `sv` и
//! `rust` дают **ноль**, `c` и `c-hal` — **мусор**; харнесс сверки прочитал 339
//! вместо 3. Код возврата `taktc` при этом нулевой, и `cc -Wall -Wextra
//! -Werror` молчит: это поле структуры, а не локальная переменная.
//!
//! Контракт `_init` обещает обратное с ADR 0033: «привести память в
//! определённое состояние ДО первого `_tick`, чтобы чтение полей между `_init`
//! и `_tick` перестало быть UB». Правило было, исполнялось наполовину.
//!
//! # Почему не `memset`
//!
//! `_init` служит и сбросом (`_reset` зовёт его), а `memset` затёр бы
//! указатели HAL, которые пользователь привязывает **до** `_init` — часть
//! контракта цели с фичи 0187.
//!
//! ⚠️ **Умолчание перечисления — ПЕРВЫЙ по тексту вариант** (фича 0391,
//! решение заказчика 2026-08-23), а не ноль: ноль может не входить в набор
//! (`enum Mode { Idle = 5, Work = 7 }`), и тогда автомат стартует со
//! значения, о котором не знает ни один `match`. Правило живёт одним
//! носителем `semantic::enum_default` — его же зовут эталон, `st` и `sv`.

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::indent::Printer;
use crate::semantic::ModelNode;
use crate::semantic::type_node::TypeNode;

/// Печатает нулевое значение переменной `field` типа `ty` в `_init`.
///
/// Форма зависит от типа, потому что в C **не всё присваивается**: массив не
/// присваивается вовсе, структура — не поэлементно, а целиком только из другой
/// структуры. Поэтому раскладка повторяет ту, которой уже пользуются печатники
/// инициализаторов: массив — по элементам, бит-вектор шире 64 бит — по словам
/// (0078), структура — по полям, рекурсивно.
///
/// # Ошибки
/// [`Diagnostic`], если структура типа не объявлена: печатать нечего, а
/// молчаливый пропуск вернул бы исходный дефект.
pub(super) fn emit_zero_init(
    printer: &mut Printer,
    field: &str,
    ty: &TypeNode,
    model: &ModelNode,
) -> Result<(), Diagnostic> {
    // Бит-вектор шире 64 бит — массив СЛОВ (0078): проверяется раньше массива,
    // иначе он ушёл бы в общую ветвь и получил ⌈N/64⌉ ≠ N элементов.
    if let Some(count) = crate::generator::c::c_bits::words_of_type(ty) {
        for i in 0..count {
            printer.ident(&format!("model->{field}[{i}] = 0;")).nl();
        }
        return Ok(());
    }
    // ⚠️ Бит-вектор `[bit; N ≤ 64]` — СКАЛЯР (правило 0078), и обнуляется он
    // одним присваиванием. Прежде он доставался общей ветви массива и получал
    // `model->flags[0] = 0;` при поле `uint8_t flags` — `cc`: «subscripted
    // value is not an array», при нулевом коде возврата `taktc` (замер 0533).
    if crate::semantic::bit_vector::is_bit_vector(ty).is_some() {
        printer.ident(&format!("model->{field} = 0;")).nl();
        return Ok(());
    }
    match ty {
        TypeNode::Array(size, elem) => {
            for i in 0..*size {
                emit_zero_init(printer, &format!("{field}[{i}]"), elem, model)?;
            }
            Ok(())
        }
        TypeNode::Struct(name) => {
            let def = model.search_struct(name).ok_or_else(|| {
                Diagnostic::error(
                    Location::Codegen,
                    format!(
                        "структура '{name}' не объявлена: умолчание поля '{field}' не строится"
                    ),
                )
                .with_code("CC-023")
            })?;
            for (sub, sub_ty) in &def.fields {
                emit_zero_init(printer, &format!("{field}.{sub}"), sub_ty, model)?;
            }
            Ok(())
        }
        // Перечисление — первый по тексту вариант (фича 0391). Значение
        // печатается ИМЕНОВАННОЙ константой (0167): голое число разошлось бы
        // с формой, которой цель печатает варианты в теле.
        TypeNode::Enum(name) => {
            // Имя константы строится ТОЙ ЖЕ функцией, что и объявление
            // `#define` (0167/0195), а владелец берётся у САМОГО УЗЛА:
            // перечисление могло быть унаследовано от родителя (класс 0193).
            let named = model.search_enum(name).and_then(|def| {
                let (variant, _) = crate::semantic::enum_default(&def.variants)?;
                let owner = def.upper.as_ref().and_then(|w| w.upgrade())?;
                Some(crate::generator::c::c_names::enum_constant(
                    &crate::semantic::minimap::Name::from(owner),
                    name,
                    &variant,
                ))
            });
            // Перечисления без вариантов не бывает (`SE-105`, 0172), и владелец
            // у узла есть всегда — ветвь `None` защитная, ноль в ней прежнее
            // поведение.
            let value = named.unwrap_or_else(|| "0".to_string());
            printer.ident(&format!("model->{field} = {value};")).nl();
            Ok(())
        }
        // Скаляр — включая `duration` (целое мс) и `q(m, n)` (целый код):
        // нулевой код означает ноль величины у обоих.
        _ => {
            printer.ident(&format!("model->{field} = 0;")).nl();
            Ok(())
        }
    }
}
