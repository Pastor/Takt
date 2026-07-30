//! Специализация моделей по аргументам инстанцирования — фича 0185,
//! режим `--parameters=specialize` (ADR 0185, Option E).
//!
//! `M(Y := 200)` порождает **копию** `ModelNode` с подставленным значением;
//! ссылка на месте инстанцирования заменяется копией, и дальше по конвейеру
//! идёт **обычная** модель — стадии 2–6, проверки, цели и симулятор о
//! специализации не знают. Копии с одинаковым набором значений дедуплицируются.
//!
//! ## Почему проход стоит МЕЖДУ стадиями 1 и 2
//!
//! Стадия 1 разрешает выражения реализации — аргументы уже разобраны и
//! вычислены (`extend_args` + `const_eval`). Тела же разрешаются стадиями 2–6,
//! и в них появляются `Rc`-ячейки переменных. Копировать модель **после**
//! разрешения тел нельзя: `clone` разделяет `Rc`, тела копии ссылались бы на
//! переменные исходной модели, и генераторы печатали бы доступ к чужому полю —
//! ровно засада 0184, только теперь исходная модель жива и перепривязать ячейку
//! «на себя» значит сломать соседа. До стадии 2 ячеек не существует: копия
//! разрешит тела **на свои** переменные штатным конвейером.
//!
//! ## Имя специализации
//!
//! `{Имя}P{n}` — порядковый номер различного набора значений в порядке обхода
//! (детерминированного: `BTreeMap` моделей и состояний, слева направо по
//! композиции; фича 0048). Полагаться на конкретный вид имени не вправе никто,
//! кроме гейта воспроизводимости (ADR 0185, п. 6). Занятое имя — `SE-086`, а не
//! молчаливое переименование: пространство имён кодогена проверок не имеет
//! (кандидаты 0182), и молчание здесь стоило бы коллизии в выводе.

use crate::diagnostics::{Diagnostic, Location};
use crate::semantic::extend::{Extend, ParameterArgument};
use crate::semantic::import::adopt::adopt_declaration;
use crate::semantic::{ExpressionNode, ModelNode, StateNode, VariableNode};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// Ключ дедупликации: уникальное имя исходной модели + значения всех параметров
/// в объявленном порядке (незаданный — его значение по умолчанию).
type DedupKey = String;

/// Состояние прохода: дедупликация и счётчики имён.
struct Specialization {
    /// Ключ набора значений → готовая специализация.
    by_values: BTreeMap<DedupKey, Rc<RefCell<ModelNode>>>,
    /// Число специализаций каждой исходной модели (для порядкового номера).
    counters: BTreeMap<String, usize>,
}

/// Специализирует все инстанцирования с аргументами в дереве `root`.
///
/// Вызывается из [`construct_stages`](crate::semantic::stages::construct_stages)
/// между стадиями 1 и 2 — только в режиме `--parameters=specialize`.
pub(crate) fn specialize_instantiations(root: &Rc<RefCell<ModelNode>>) -> Result<(), Diagnostic> {
    let mut ctx = Specialization {
        by_values: BTreeMap::new(),
        counters: BTreeMap::new(),
    };
    walk_model(&mut ctx, root)
}

/// Обходит модель: реализации её состояний, затем вложенные модели.
fn walk_model(ctx: &mut Specialization, model: &Rc<RefCell<ModelNode>>) -> Result<(), Diagnostic> {
    // Состояния клонируются на время обхода: замена ссылки мутирует implements.
    let mut states = model.borrow().states.clone();
    let mut changed = false;
    for state in states.values_mut() {
        if let StateNode::Implement { implements, .. } = state {
            changed |= walk_extend(ctx, implements)?;
        }
    }
    if changed {
        model.borrow_mut().states = states;
    }

    let nested: Vec<Rc<RefCell<ModelNode>>> =
        model.borrow().models.values().map(Rc::clone).collect();
    for sub in nested {
        walk_model(ctx, &sub)?;
    }
    Ok(())
}

/// Обходит реализацию; возвращает `true`, если ссылка заменена специализацией.
fn walk_extend(ctx: &mut Specialization, extend: &mut Extend) -> Result<bool, Diagnostic> {
    match extend {
        Extend::None | Extend::Unresolved(_) => Ok(false),
        Extend::Model(source, loc, args) => {
            if args.is_empty() {
                return Ok(false);
            }
            let specialized = specialize_one(ctx, source, *loc, args)?;
            *extend = Extend::Model(specialized, *loc, Vec::new());
            Ok(true)
        }
        Extend::Parentless(inner) => walk_extend(ctx, inner),
        Extend::Concatenation(items) | Extend::Parallel(items) => {
            let mut changed = false;
            for item in items {
                changed |= walk_extend(ctx, item)?;
            }
            Ok(changed)
        }
    }
}

/// Возвращает специализацию модели `source` под набор `args` (создаёт либо
/// берёт готовую по дедупликации).
fn specialize_one(
    ctx: &mut Specialization,
    source: &Rc<RefCell<ModelNode>>,
    call_loc: Location,
    args: &[ParameterArgument],
) -> Result<Rc<RefCell<ModelNode>>, Diagnostic> {
    let source_unique = unique_name(source);
    let key = dedup_key(&source_unique, source, args);
    if let Some(ready) = ctx.by_values.get(&key) {
        return Ok(Rc::clone(ready));
    }

    // Специализация модели с собственными вложенными моделями — за границей
    // задачи: `copy` разделил бы под-модели по `Rc`, и их владелец (а с ним
    // уникальное имя пути) разъехался бы с местом в дереве — `CC-004`, урок
    // adopt 0184 («владельца под-моделей менять нельзя»). Честный отказ.
    if !source.borrow().models.is_empty() {
        return Err(Diagnostic::error(
            call_loc,
            format!(
                "Специализация модели '{}' с вложенными моделями не поддерживается: \
                 задайте значения параметров вложенных моделей в их собственных \
                 инстанцированиях либо соберите в режиме --parameters=assign",
                source.borrow().name.clone().unwrap_or_default()
            ),
        )
        .with_code("SE-087"));
    }

    let parent = source
        .borrow()
        .upper
        .as_ref()
        .and_then(|weak| weak.upgrade())
        .ok_or_else(|| {
            Diagnostic::error(
                call_loc,
                "Специализируемая модель не имеет родителя в дереве".to_string(),
            )
            .with_code("SE-086")
        })?;

    // Порядковый номер различного набора — имя специализации.
    let counter = ctx.counters.entry(source_unique).or_insert(0);
    *counter += 1;
    let base = source.borrow().name.clone().unwrap_or_default();
    let new_name = format!("{}P{}", base, counter);
    if parent.borrow().models.contains_key(&new_name) {
        return Err(Diagnostic::error(
            call_loc,
            format!(
                "Имя специализации '{new_name}' уже занято моделью — переименуйте её: \
                 пространство имён кодогена общее, и молчаливое переименование \
                 специализации скрыло бы коллизию в выводе"
            ),
        )
        .with_code("SE-086"));
    }

    // Копия под новым именем, в карте моделей родителя исходной.
    let copy = Rc::new(RefCell::new(
        source
            .borrow()
            .copy(Some(new_name.clone()), Some(Rc::clone(&parent))),
    ));
    // `copy` клонирует переменные вместе с их `upper` — Weak на ИСХОДНУЮ модель.
    // Перепривязка обязательна: по `upper` строится доступ в генераторах.
    // Приём — adopt_declaration (фича 0184), имя остаётся своим.
    {
        let mut model = copy.borrow_mut();
        let mut rebound = BTreeMap::new();
        for (name, mut var) in std::mem::take(&mut model.variables) {
            adopt_declaration(&mut var, &copy, &name);
            rebound.insert(name, var);
        }
        model.variables = rebound;
    }
    // Значения аргументов — инициализаторами параметров копии. Узел уже
    // понижен (`const_eval` + `construct_expression` в 0185-02/03): стадия 2
    // разрешать его не будет, а вывод типов возьмёт тип из аннотации.
    for arg in args {
        set_initializer(&copy, &arg.name, arg.value.clone(), arg.loc)?;
    }
    parent
        .borrow_mut()
        .models
        .insert(new_name, Rc::clone(&copy));
    ctx.by_values.insert(key, Rc::clone(&copy));
    Ok(copy)
}

/// Подставляет значение в инициализатор параметра копии.
fn set_initializer(
    copy: &Rc<RefCell<ModelNode>>,
    param: &str,
    value: ExpressionNode,
    loc: Location,
) -> Result<(), Diagnostic> {
    let mut model = copy.borrow_mut();
    match model.variables.get_mut(param) {
        Some(VariableNode::Simple { expr, .. }) => {
            *expr = value;
            Ok(())
        }
        // Проверка 0185-02 (`SE-077…079`) гарантирует, что имя — параметр, а
        // параметр в дереве — Simple; иное здесь означает регресс конвейера.
        _ => Err(Diagnostic::error(
            loc,
            format!("Параметр '{param}' не найден в специализируемой модели"),
        )
        .with_code("SE-086")),
    }
}

/// Уникальное имя модели — путь по цепочке `upper` (как у продюсера карты
/// адресов 0084: правя одно, правь другое).
fn unique_name(model: &Rc<RefCell<ModelNode>>) -> String {
    let mut parts = vec![model.borrow().name.clone().unwrap_or_default()];
    let mut current = model.borrow().upper.as_ref().and_then(|w| w.upgrade());
    while let Some(node) = current {
        parts.push(node.borrow().name.clone().unwrap_or_default());
        current = node.borrow().upper.as_ref().and_then(|w| w.upgrade());
    }
    parts.reverse();
    parts.join(":")
}

/// Ключ дедупликации: значения **всех** параметров в объявленном порядке —
/// заданное аргументом либо значение по умолчанию из объявления (ADR 0185,
/// п. 6: «исходная модель + значения в объявленном порядке»).
fn dedup_key(
    source_unique: &str,
    source: &Rc<RefCell<ModelNode>>,
    args: &[ParameterArgument],
) -> String {
    let model = source.borrow();
    let mut parts = vec![source_unique.to_string()];
    for param in &model.parameters {
        let value = args
            .iter()
            .find(|a| a.name == param.name)
            .map(|a| format!("{:?}", a.value))
            .unwrap_or_else(|| "default".to_string());
        parts.push(format!("{}={}", param.name, value));
    }
    parts.join(";")
}
