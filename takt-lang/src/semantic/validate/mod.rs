//! Валидация семантических узлов языка Takt.
//!
//! Проверяет семантические инварианты после построения дерева.
//! Проверки выполняются рекурсивно для всех вложенных моделей.
//!
//! ## Текущие проверки
//!
//! - Модель, содержащая состояния, должна иметь ровно одно начальное
//!   состояние (`start`). Модели без состояний (только с объявлениями
//!   переменных, типов и т.п.) от этой проверки освобождены.
//!
//! - Переменная типа `bit` может быть инициализирована только значениями
//!   `0`, `1`, `true` или `false`. Любое другое числовое значение — ошибка.
//!
//! - Условие перехода (`ref`) не должно содержать неявного приведения
//!   числового типа к булевому. Использование переменных числового типа
//!   (например, `[bit;8]`) без явного сравнения порождает предупреждение
//!   [`check_implicit_bool_conditions`].

use crate::diagnostics::{Diagnostic, Location};
use crate::parser::ast;
use crate::semantic::condition::resolve_condition;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{
    ConditionNode, ExpressionNode, FunctionDefinitionNode, ModelNode, PortDirection, ReferenceNode,
    StateNode, StateNodeKind, StatementNode, VariableNode,
};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

mod anon_init;
mod assignment_position;
mod bodies;
mod common;
mod constant_conditions;
pub mod depth;
mod entry;
mod enums;
mod fixed;
mod formulas;
mod implicit_bool;
// `pub(crate)`, а не `mod`: границы целочисленного типа (`type_range`) нужны и
// свёртке инициализатора (фича 0207) — вторая копия границ разошлась бы с
// проверкой `SE-089`.
pub(crate) mod literal_range;
mod member_access;
mod name_collisions;
mod nondeterminism;
mod port_init;
mod ports;
mod states;
mod structs;
mod types;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_ce15_array_size;
#[cfg(test)]
mod tests_ce4_declarations;
#[cfg(test)]
mod tests_implicit_bool;

// Внутреннее: помощники, которые зовут `validate_model` и соседние подмодули.
// `use super::*` в каждом подмодуле подхватывает их отсюда.
use common::{
    get_state_loc, get_state_name, validate_cond, validate_conditions, validate_expression,
    validate_reference,
};
use enums::{validate_bit_values, validate_enum_type_declarations, validate_enum_values};
use fixed::check_fixed_mixing;
use ports::{check_port_addresses, validate_variables};
use states::{model_only_one_start_state, validate_state_references};
use types::check_array_sizes;

// Реэкспорт: внешние пути импорта НЕ меняются. Потребители — `semantic/tree.rs`
// (5 имён) и `lib.rs` (6 имён по пути `semantic::validate::…`) — не правятся.
pub(crate) use common::reachable_targets;
pub use constant_conditions::check_constant_conditions;
pub use entry::validate_entry_model;
pub use enums::check_enum_type_safety;
pub use implicit_bool::check_implicit_bool_conditions;
pub use nondeterminism::check_nondeterministic_transitions;
pub use ports::{check_port_address_completeness, warn_nested_model_ports};
pub use states::{check_transition_completeness, check_unreachable_states};
pub use structs::{check_duplicate_struct_fields, check_struct_field_types};
pub use types::{check_recursive_type_aliases, check_type_alias_cycles_ast};

/// Проверяет модель, останавливаясь на первой ошибке.
///
/// Контракт прежний (183 вызова в проекте), но реализация — «первая из
/// [`validate_model_all`]»: два входа, написанные порознь, разошлись бы, и
/// пользователь получал бы разный ответ в зависимости от того, кто спрашивает
/// (фича 0130).
pub fn validate_model(model: Rc<RefCell<ModelNode>>) -> Result<(), Diagnostic> {
    match validate_model_all(model).into_iter().next() {
        Some(diagnostic) => Err(diagnostic),
        None => Ok(()),
    }
}

/// Проверяет модель, собирая **все** найденные ошибки (фича 0130).
///
/// Проверки идут по **готовому** дереву и независимы друг от друга, поэтому
/// каждая может сообщить о своём, не мешая соседям: пользователь видит причины,
/// а не первую попавшуюся.
///
/// ## Что накапливается, а что нет
///
/// - **Между проверками** — да: все четырнадцать высказываются.
/// - **Внутри проверки** — по-разному: пять из них возвращают `Vec` и отдают всё
///   найденное, остальные устроены как цикл с ранним возвратом и дают по одной
///   ошибке на модель. Углубление — отдельная работа (граница ADR 0130).
/// - **Вложенные модели** обходятся рекурсивно, и их диагностики добавляются к
///   общему списку.
///
/// Порядок здесь — порядок проверок и обхода (детерминированного, фича 0048);
/// упорядочить по позиции в тексте — задача выдачи
/// ([`diagnostics::normalize`](crate::diagnostics::normalize)).
pub fn validate_model_all(model: Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let mut found = Vec::new();

    // ⚠️ Каждая проверка **накапливает** (фича 0151): прежде все одиннадцать
    // отдавали `Result<(), Diagnostic>` — не более одной ошибки на модель, — и
    // две неверные переменные давали одно сообщение. Правило накопления:
    // **одна диагностика на элемент, все элементы высказываются**; внутри
    // одного выражения ранний выход сохранён, потому что дальше по нему пошли
    // бы следствия первой ошибки (тот же довод, которым 0152 оставила
    // терминальными стадии построения дерева).
    let checks: [Vec<Diagnostic>; 11] = [
        model_only_one_start_state(model.clone()),
        // SE-099 (0189): обращение к ячейке в инициализаторе объявления. Без
        // запрета эталон дал бы ноль, а `c-hal` — чтение регистра, и молча.
        anon_init::validate_anon_in_initializers(model.clone()),
        validate_bit_values(model.clone()),
        validate_enum_values(model.clone()),
        validate_enum_type_declarations(model.clone()),
        validate_state_references(model.clone()),
        validate_variables(model.clone()),
        validate_conditions(model.clone()),
        check_array_sizes(model.clone()),
        check_port_addresses(model.clone()),
        check_fixed_mixing(model.clone()), // T6 (0061): запрет смешения q(m, n)
    ];
    found.extend(checks.into_iter().flatten());

    // Ce16: рекурсивные псевдонимы — проверка отдаёт все циклы сразу.
    // ⚠️ Прежде вызывающий брал из них первую: накопление здесь было написано,
    // но выбрасывалось.
    found.extend(check_recursive_type_aliases(model.clone()));

    // Ce17/Ce18: структуры — по одной ошибке от каждой проверки.
    found.extend(check_duplicate_struct_fields(model.clone()));
    found.extend(check_struct_field_types(model.clone()));

    // Ce19 (SE-061): доступ к несуществующему полю структуры (0080, дефект 3)
    found.extend(member_access::check_struct_field_access(model.clone()).err());

    // SE-089 (0157): литерал не помещается в тип приёмника. Отдаёт все находки:
    // одна ошибка на литерал, а не «первая на модель».
    found.extend(literal_range::check_literal_ranges(model.clone()));

    // Тела модели — один обход, два судьи (`validate/bodies.rs`):
    //  * SE-026/SE-027 (0188): направление порта во ВСЕХ позициях. Тела блоков и
    //    функций прежде не обходились, и нарушение уезжало в цели, где
    //    расходилось вплоть до записи по адресу другого порта;
    //  * SE-095 (0187, ось 4): присваивание — оператор, а не выражение. Прежде
    //    `x := (led := 1) + 1` не отвергал никто: диагностику давали чужие
    //    инструменты на порождённом файле, а цель `c` для переменных исполняла
    //    то, чего не исполняет эталон.
    found.extend(bodies::check_bodies(model.clone()));

    // SE-100/SE-101 (0195): столкновения имён, неразрешимые в пространстве имён
    // цели. Отказ переносится из чужих инструментов (`cc`, `iec2c`) в свой
    // компилятор: обе формы не работают НИ В ОДНОЙ цели, поэтому запрещается не
    // работающая запись, а уже существующий отказ получает позицию.
    found.extend(name_collisions::check_name_collisions(model.clone()));

    // SE-025 и прочие проверки условия — на ОХРАННЫХ ФОРМУЛАХ (0203). Прежде
    // формулы не обходила ни одна проверка, и `: [Guard] опечатка < 3;`
    // принималось молча: средство безопасности переставало сторожить, а цель
    // `c` печатала `assert( < 3);` — отказ приходил от `cc`, а не от языка.
    // Судья здесь тот же, что у `cond` и рёбер: проверка лишь доставляет ему
    // условия.
    found.extend(formulas::validate_formulas(model.clone()));

    // SE-092 (0187): начальное значение у входного порта — ошибка. Временное
    // SE-093 («выставляют не все цели») снято задачей 0187-04: цели умеют все.
    found.extend(port_init::check_port_initializers(model.clone()));

    let nested: Vec<Rc<RefCell<ModelNode>>> =
        model.borrow().models.values().map(Rc::clone).collect();
    for nested_model in nested {
        found.extend(validate_model_all(nested_model));
    }
    found
}
