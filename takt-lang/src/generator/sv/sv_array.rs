//! Распакованный массив у цели `sv`: сброс и индекс (фича 0365).
//!
//! Массив скаляров печатается **распакованным** (`logic [7:0] a [0:1]`), и две
//! операции над ним требуют своих форм — иначе вывод не проходит гейт
//! собственной цели при нулевом коде возврата `taktc`:
//!
//! - **сброс без инициализатора.** `a <= '0;` verilator встречает **ошибкой**
//!   «CONST '8'h0' is not an unpacked array, but is in an unpacked array
//!   context». Форма `'{default: '0}` не годится: её **не принимает yosys**
//!   («syntax error, unexpected TOK_DEFAULT»), а форма выбирается по тому, что
//!   принимают **оба** инструмента (урок 0235). Годная — тот же агрегат, каким
//!   печатается инициализатор (фича 0309): `'{8'd0, 8'd0}`;
//! - **переменный индекс.** `a[i]` при `i: u8` и массиве из двух элементов даёт
//!   `%Warning-WIDTHTRUNC: Bit extraction of array[1:0] requires 1 bit index,
//!   not 8 bits`, а гейт цели считает предупреждение ошибкой. Лечится явным
//!   сужением `a[1'(i)]`.
//!
//! ⚠️ Бит-вектор `[bit;N≤64]` под правило **не подпадает**: это упакованный
//! скаляр (правило 0078), у него и сброс `'0`, и индексация разряда законны.

use std::collections::BTreeMap;

use crate::diagnostics::Diagnostic;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{ConditionNode, ExpressionNode};

use super::sv_type::scalar_width;

/// Значение сброса распакованного массива — агрегат нулей (фича 0365).
///
/// `None`, если тип массивом не является либо его элемент не имеет одной
/// скалярной ширины (массив структур: их сброс печатает свой путь).
pub(crate) fn reset_literal(ty: &TypeNode) -> Option<String> {
    let TypeNode::Array(size, elem) = ty else {
        return None;
    };
    if crate::semantic::bit_vector::is_bit_vector(ty).is_some() {
        return None;
    }
    // Вложенный массив — тот же агрегат уровнем ниже: правило рекурсивно, а
    // второго знания о раскладке заводить нельзя.
    let zero = match &**elem {
        TypeNode::Array(_, _) => reset_literal(elem)?,
        other => format!("{}'d0", scalar_width(other)?),
    };
    let items = vec![zero; usize::from(*size)];
    Some(format!("'{{{}}}", items.join(", ")))
}

/// Печатает индекс массива, сужая его до ширины, которую требует размер.
///
/// `base_ty` — тип **базы** индексации (если известен), `index_ty` — тип
/// индекса (только именованное значение: у литерала ширины нет, он
/// подстраивается под контекст). Приведение печатается **по нужде** — когда
/// ширина индекса заведомо больше требуемой; лишнее приведение сузило бы
/// вывод там, где предупреждения нет (класс 0263 у цели `rust`).
pub(crate) fn index_text(
    base_ty: Option<&TypeNode>,
    index_ty: Option<&TypeNode>,
    printed: String,
) -> String {
    let Some(TypeNode::Array(size, _)) = base_ty else {
        return printed;
    };
    if base_ty.is_some_and(|ty| crate::semantic::bit_vector::is_bit_vector(ty).is_some()) {
        return printed;
    }
    let Some(index_width) = index_ty.and_then(scalar_width) else {
        return printed;
    };
    let needed = index_width_for(*size);
    if index_width <= needed {
        return printed;
    }
    format!("{needed}'({printed})")
}

/// Тип массива-базы индексации — для выражений (фича 0365).
///
/// Разбирается цепочка `переменная([индекс])*`: у вложенной индексации база
/// сама является индексацией, и без спуска `cells[i][j]` сузил бы только
/// внешний индекс (замер: verilator по-прежнему отвечал `WIDTHTRUNC`).
///
/// ⚠️ Опирается на «именованное значение» (`mixed_sign::operand_type_expr`):
/// у литерала и произвольного выражения типа здесь нет, и догадываться о нём
/// нельзя.
pub(crate) fn array_type_expr(expr: &ExpressionNode) -> Option<TypeNode> {
    match expr {
        ExpressionNode::Parenthesis(inner) => array_type_expr(inner),
        ExpressionNode::ArraySubscript(base, _) => match array_type_expr(base)? {
            TypeNode::Array(_, elem) => Some(*elem),
            _ => None,
        },
        other => crate::generator::mixed_sign::operand_type_expr(other),
    }
}

/// То же для условий: у них своё дерево (ADR 0019).
pub(crate) fn array_type_cond(cond: &ConditionNode) -> Option<TypeNode> {
    match cond {
        ConditionNode::Parenthesis(inner) => array_type_cond(inner),
        ConditionNode::ArraySubscript(base, _) => match array_type_cond(base)? {
            TypeNode::Array(_, elem) => Some(*elem),
            _ => None,
        },
        other => crate::generator::mixed_sign::operand_type_cond(other),
    }
}

/// Ширина индекса, которую требует массив из `size` элементов.
///
/// Диапазон `[0:size-1]` адресуется `ceil(log2(size))` разрядами, а массив из
/// одного элемента — одним (нулевой ширины в SystemVerilog не бывает).
fn index_width_for(size: u16) -> u32 {
    let last = u32::from(size.saturating_sub(1));
    if last == 0 {
        1
    } else {
        u32::BITS - last.leading_zeros()
    }
}

/// Листья ТИПА: суффикс и тип каждого скалярного места (фича 0367).
///
/// `None` — тип листьями не раскладывается, то есть присваивается целиком.
/// Раскладка нужна там, где внутри распакованного массива лежит структура:
/// yosys принимает шаблон присваивания только для массива целиком, а частичную
/// запись поля элемента при умолчании массива целиком объявляет защёлкой
/// («Latch inferred» — замер 2026-08-21). Значит и умолчание, и сброс такого
/// регистра печатаются **по полям**.
pub(crate) fn type_leaves(
    ty: &TypeNode,
    fields_of: &crate::generator::aggregate::FieldsOf<'_>,
) -> Option<Vec<(String, TypeNode)>> {
    if !needs_leafwise(ty, fields_of) {
        return None;
    }
    let mut out = Vec::new();
    walk_type(ty, fields_of, &mut String::new(), &mut out);
    Some(out)
}

/// Лежит ли структура ВНУТРИ распакованного массива.
///
/// Только этот случай синтезатор не принимает целиком: массив скаляров
/// присваивается whole-array (проверено прогоном), а структура вне массива —
/// упакованная, и её присваивание тоже законно.
pub(crate) fn needs_leafwise(
    ty: &TypeNode,
    fields_of: &crate::generator::aggregate::FieldsOf<'_>,
) -> bool {
    match ty {
        TypeNode::Array(_, elem) if crate::semantic::bit_vector::is_bit_vector(ty).is_none() => {
            contains_struct(elem, fields_of)
        }
        _ => false,
    }
}

fn contains_struct(ty: &TypeNode, fields_of: &crate::generator::aggregate::FieldsOf<'_>) -> bool {
    match ty {
        TypeNode::Struct(name) => fields_of(name).is_some(),
        TypeNode::Array(_, elem) if crate::semantic::bit_vector::is_bit_vector(ty).is_none() => {
            contains_struct(elem, fields_of)
        }
        _ => false,
    }
}

fn walk_type(
    ty: &TypeNode,
    fields_of: &crate::generator::aggregate::FieldsOf<'_>,
    prefix: &mut String,
    out: &mut Vec<(String, TypeNode)>,
) {
    match ty {
        TypeNode::Array(size, elem) if crate::semantic::bit_vector::is_bit_vector(ty).is_none() => {
            for index in 0..usize::from(*size) {
                let saved = prefix.len();
                prefix.push_str(&format!("[{index}]"));
                walk_type(elem, fields_of, prefix, out);
                prefix.truncate(saved);
            }
        }
        TypeNode::Struct(name) => match fields_of(name) {
            Some(fields) => {
                for (field, field_ty) in fields {
                    let saved = prefix.len();
                    prefix.push('.');
                    prefix.push_str(&field);
                    walk_type(&field_ty, fields_of, prefix, out);
                    prefix.truncate(saved);
                }
            }
            None => out.push((prefix.clone(), ty.clone())),
        },
        other => out.push((prefix.clone(), other.clone())),
    }
}

/// Сброс ПО ЛИСТЬЯМ, если тип того требует (фича 0367).
///
/// Пустой вектор — регистр сбрасывается целиком, как прежде. Значения листьев
/// берутся у инициализатора, когда он агрегат, и у умолчания типа — когда его
/// нет.
pub(crate) fn leafwise_reset(
    expr: &crate::semantic::ExpressionNode,
    ty: &crate::semantic::type_node::TypeNode,
    enums: &BTreeMap<String, Vec<(String, i128)>>,
    structs: &BTreeMap<String, Vec<(String, crate::semantic::type_node::TypeNode)>>,
    loc: crate::diagnostics::Location,
    what: &str,
) -> Result<Vec<(String, String)>, Diagnostic> {
    use crate::semantic::ExpressionNode;

    let fields_of = |name: &str| structs.get(name).cloned();
    let Some(type_leaves) = type_leaves(ty, &fields_of) else {
        return Ok(Vec::new());
    };
    if let ExpressionNode::Initializer(items) | ExpressionNode::Array(items) = expr {
        let mut out = Vec::new();
        for leaf in crate::generator::aggregate::leaves(Some(ty), items, &fields_of) {
            let suffix = crate::generator::aggregate::c_like_suffix(&leaf.path);
            let leaf_ty = leaf.ty.clone().unwrap_or_else(|| ty.clone());
            let value = super::sv_const::reset_value(leaf.value, &leaf_ty, enums, what, loc, None)?;
            out.push((suffix, value));
        }
        return Ok(out);
    }
    // Инициализатора нет: каждый лист получает умолчание своего типа.
    let mut out = Vec::new();
    for (suffix, leaf_ty) in type_leaves {
        let value =
            super::sv_const::reset_value(&ExpressionNode::None, &leaf_ty, enums, what, loc, None)?;
        out.push((suffix, value));
    }
    Ok(out)
}

/// Имя плоского параметра, которым передаётся массив (фича 0369).
pub(crate) fn flat_param_name(param: &str) -> String {
    format!("{param}_flat")
}

/// Ширина плоского вектора, которым передаётся массив: `размер × ширина элемента`.
///
/// `None` — тип не распакованный массив скаляров: бит-вектор (0078) передаётся
/// как есть, а массив структур или массив массивов плоским вектором не
/// выражается — там форма не измерена, и догадываться о ней нельзя.
pub(crate) fn flat_param_width(ty: &TypeNode) -> Option<(u32, u16, u32)> {
    let TypeNode::Array(size, elem) = ty else {
        return None;
    };
    if crate::semantic::bit_vector::is_bit_vector(ty).is_some() {
        return None;
    }
    let elem_width = scalar_width(elem)?;
    Some((elem_width * u32::from(*size), *size, elem_width))
}

/// Аргумент-массив печатается КОНКАТЕНАЦИЕЙ элементов (фича 0369).
///
/// Порядок обратный: `{a[N-1], …, a[0]}` — так `a[0]` ложится в младшие
/// разряды, и распаковка в прологе функции читает те же элементы.
pub(crate) fn flatten_argument(base: &str, size: u16) -> String {
    let parts: Vec<String> = (0..size).rev().map(|i| format!("{base}[{i}]")).collect();
    format!("{{{}}}", parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(bits: u8) -> TypeNode {
        TypeNode::Integer {
            bits,
            signed: false,
        }
    }

    #[test]
    fn reset_of_unpacked_array_is_aggregate_of_zeros() {
        let ty = TypeNode::Array(2, Box::new(u(8)));
        assert_eq!(reset_literal(&ty).as_deref(), Some("'{8'd0, 8'd0}"));
    }

    /// Вложенный массив — агрегат агрегатов: правило рекурсивно.
    #[test]
    fn reset_of_nested_array_is_nested_aggregate() {
        let inner = TypeNode::Array(2, Box::new(u(8)));
        let ty = TypeNode::Array(2, Box::new(inner));
        assert_eq!(
            reset_literal(&ty).as_deref(),
            Some("'{'{8'd0, 8'd0}, '{8'd0, 8'd0}}")
        );
    }

    /// Бит-вектор `[bit;8]` — упакованный СКАЛЯР (правило 0078): его сброс
    /// печатает прежний путь (`'0`), и агрегата здесь быть не должно.
    #[test]
    fn packed_bit_vector_is_not_an_unpacked_array() {
        let ty = TypeNode::Array(8, Box::new(TypeNode::Bit));
        assert_eq!(reset_literal(&ty), None);
    }

    #[test]
    fn wide_index_is_narrowed_to_the_width_the_size_requires() {
        let base = TypeNode::Array(2, Box::new(u(8)));
        assert_eq!(
            index_text(Some(&base), Some(&u(8)), "i".to_string()),
            "1'(i)"
        );
        let base4 = TypeNode::Array(4, Box::new(u(8)));
        assert_eq!(
            index_text(Some(&base4), Some(&u(8)), "i".to_string()),
            "2'(i)"
        );
    }

    /// Приведение печатается ПО НУЖДЕ: индекс, чья ширина уже не больше
    /// требуемой, остаётся как есть — лишнее приведение сужало бы вывод там,
    /// где предупреждения нет.
    #[test]
    fn narrow_index_is_printed_as_is() {
        let base = TypeNode::Array(2, Box::new(u(8)));
        assert_eq!(
            index_text(Some(&base), Some(&TypeNode::Bit), "b".to_string()),
            "b"
        );
        // Тип индекса неизвестен (литерал, выражение) — не трогаем.
        assert_eq!(index_text(Some(&base), None, "1".to_string()), "1");
        // База не массив — не трогаем.
        assert_eq!(index_text(None, Some(&u(8)), "i".to_string()), "i");
    }

    /// Массив структур раскладывается по полям: whole-array присваивание
    /// синтезатор принимает, а частичную запись поля при нём — нет.
    #[test]
    fn array_of_structs_is_split_into_field_leaves() {
        let ty = TypeNode::Array(2, Box::new(TypeNode::Struct("Point".to_string())));
        let fields = |name: &str| {
            (name == "Point").then(|| vec![("x".to_string(), u(8)), ("y".to_string(), u(8))])
        };
        let leaves = type_leaves(&ty, &fields).expect("массив структур раскладывается");
        let names: Vec<_> = leaves.iter().map(|(s, _)| s.clone()).collect();
        assert_eq!(names, vec!["[0].x", "[0].y", "[1].x", "[1].y"]);
    }

    /// Массив СКАЛЯРОВ присваивается целиком — раскладка не нужна.
    #[test]
    fn array_of_scalars_is_assigned_as_a_whole() {
        let ty = TypeNode::Array(2, Box::new(u(8)));
        let no_fields = |_: &str| None;
        assert!(type_leaves(&ty, &no_fields).is_none());
    }

    /// Аргумент-массив печатается конкатенацией в ОБРАТНОМ порядке: `a[0]`
    /// обязан лечь в младшие разряды, иначе распаковка вернёт другой элемент.
    #[test]
    fn array_argument_is_concatenated_low_element_last() {
        assert_eq!(flatten_argument("data", 3), "{data[2], data[1], data[0]}");
    }

    /// Плоский вектор шире элемента ровно во столько раз, сколько элементов.
    #[test]
    fn flat_width_is_size_times_element_width() {
        let ty = TypeNode::Array(3, Box::new(u(8)));
        assert_eq!(flat_param_width(&ty), Some((24, 3, 8)));
        // Бит-вектор передаётся как есть — он упакованный скаляр (0078).
        let bits = TypeNode::Array(8, Box::new(TypeNode::Bit));
        assert_eq!(flat_param_width(&bits), None);
    }

    #[test]
    fn index_width_covers_the_last_element() {
        assert_eq!(index_width_for(1), 1);
        assert_eq!(index_width_for(2), 1);
        assert_eq!(index_width_for(3), 2);
        assert_eq!(index_width_for(4), 2);
        assert_eq!(index_width_for(5), 3);
        assert_eq!(index_width_for(256), 8);
    }
}
