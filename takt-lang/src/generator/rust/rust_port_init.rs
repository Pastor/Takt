//! Начальные значения выходных портов в цели `rust` (фича 0187, задача 03).
//!
//! ## Почему это делает корень, а не владелец
//!
//! У цели `c` каждая модель выставляет свои порты сама: в `_init` под-модели
//! есть указатель `main`, то есть доступ к HAL. В цели `rust` его нет —
//! под-модель конструируется `Sub::new()` без аргументов, а `init(&mut self)`
//! HAL не получает (это осознанный контракт 0050: HAL приходит **параметром
//! такта**). Поэтому значения портов всего дерева выставляет **корень**, у
//! которого `self.hal` есть.
//!
//! Это законно ровно потому, что перечисления портов в цели `rust` — **общие
//! на файл** (`rust_decl::collect_ports` обходит все модели): вариант
//! `OutF64Port::Temperature` не зависит от того, чей это порт. А значение к
//! этому моменту — литерал (свёртка в семантике,
//! `declaration::resolve_port_init`), поэтому и контекст печати выражения роли
//! не играет.
//!
//! ## Два места, а не одно
//!
//! Записи печатаются и в `new()`, и в `init()`: это разные входы (0185 уже
//! ловил на этом аргументы инстанцирования). `new()` строит модель, `init()`
//! возвращает её в начальное состояние — если выставить значение только в
//! одном, сброс либо не дошёл бы до порта, либо порт получил бы значение
//! только после `init()`.

use crate::diagnostics::Diagnostic;
use crate::generator::rust::rust_expr::{Scope, coerce_to};
use crate::generator::rust::rust_map::RustMap;
use crate::generator::rust::rust_name::rust_type_name;
use crate::generator::rust::rust_port::port_class;
use crate::semantic::minimap::Element;
use crate::semantic::{ExpressionNode, ModelNode, PortDirection, VariableNode};
use std::collections::BTreeSet;

/// Собирает записи начальных значений портов всего дерева — по одной строке
/// вида `write_f64(OutF64Port::Temperature, 0.0);` **без** приёмника.
///
/// Приёмник (`self.hal` в `new()`/`init()`) дописывает вызывающий: в `new()`
/// модель ещё строится и обращение идёт к `this`, а не к `self`.
///
/// Порядок обхода задан картой (`BTreeMap` моделей, `using_models`) —
/// детерминизм вывода (0048) держится типом контейнера, а не сортировкой здесь.
pub(crate) fn port_initial_writes(
    map: &RustMap,
    root: &ModelNode,
) -> Result<Vec<String>, Diagnostic> {
    let scope = Scope {
        model: root,
        shared: Vec::new(),
        shared_via_self: false,
        locals: Vec::new(),
        assigned: BTreeSet::new(),
        hal: String::new(),
        has_self: false,
        hal_is_ref: false,
        instances: Vec::new(),
        time_profile: map.time_profile(),
        return_type: None,
        // Подсказка о приёмнике степени ставится в `coerce_to` (фича 0415).
        power_target: None,
    };
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out = Vec::new();
    for element in map.using_models() {
        let Element::Model { name, .. } = element else {
            continue;
        };
        let model = map.raw_model_at(name)?;
        collect_from(&model.borrow(), &scope, &mut seen, &mut out)?;
    }
    collect_from(root, &scope, &mut seen, &mut out)?;
    Ok(out)
}

/// Добавляет записи по портам одной модели.
fn collect_from(
    model: &ModelNode,
    scope: &Scope,
    seen: &mut BTreeSet<String>,
    out: &mut Vec<String>,
) -> Result<(), Diagnostic> {
    for var in model.variables.values() {
        let VariableNode::Port {
            name,
            ty,
            init,
            direction,
            loc,
            ..
        } = var
        else {
            continue;
        };
        if matches!(init, ExpressionNode::None) || *direction == PortDirection::In {
            continue;
        }
        // Одно имя — один вариант перечисления (то же правило, что в
        // `collect_ports`): порт, объявленный в двух моделях под одним именем,
        // даёт одну запись, а не две одинаковых.
        if !seen.insert(name.clone()) {
            continue;
        }
        let class = port_class(ty, name, *loc)?;
        out.push(format!(
            "{}({}::{}, {});",
            class.write_fn(),
            class.out_enum(),
            rust_type_name(name, *loc)?,
            coerce_to(init, ty, scope)?
        ));
    }
    Ok(())
}
