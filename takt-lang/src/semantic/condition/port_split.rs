//! Порт СОСТАВНОГО типа разворачивается в скалярные (фича 0390, Option C).
//!
//! # Что было
//!
//! `out po: Pair at 0x100;` при `struct Pair { lo: u8, hi: u8 }`. Замер
//! 2026-08-23 (`scripts/probe.sh`): эталон исполняет, `st`, `sv` и `plantuml`
//! переводят, а **пять** целей отказывают — `CC-015` (колбэк HAL принимает
//! скаляр), `ST-004` (размещённая переменная составного типа), `RS-016`
//! (метод трейта не ложится), `SV-002` (распакованный порт в шапке модуля).
//! То есть язык описывал возможность, которой у большинства целей нет.
//!
//! Отказы объясняющие и **сами называют обход**: «Разложите порт на
//! скалярные». Проход делает это за автора.
//!
//! # Правило
//!
//! Порт структурного типа заменяется портами по **листам** структуры: имя —
//! `<порт>_<поле>` (рекурсивно), тип — тип листа, направление — прежнее.
//! Обращения переписываются: агрегатное присваивание — по листам, доступ к
//! полю — прямым обращением к порту листа.
//!
//! За границей семантики составного порта не существует, и печатники целей о
//! нём не знают (приём 0143/0192/0397/0400).
//!
//! ⚠️ **Разворот зовут только те цели, что порт не умеют** (`c`, `c-hal`,
//! `rust`, `st-at`, `sv-mmio`): у `st` и `sv` он работает как есть, и общий
//! разворот изменил бы их вывод без нужды — тот же довод, что в 0397.
//!
//! ⚠️ **Адрес листа — базовый плюс смещение поля в байтах**: у HAL-целей порт
//! ложится на регистр, и раскладка обязана быть предсказуемой. Знание о
//! размере типа берётся у общего носителя `type_size::bytes_of`.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;

use crate::diagnostics::{Diagnostic, Location};
use crate::parser::ast::Member;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{
    ExpressionNode, ModelNode, NamedCodeBlockDefinitionNode, StateNode, StatementNode, VariableNode,
};

/// Шаг пути к листу: поле структуры либо элемент массива (фича 0417).
///
/// ⚠️ Путь ШАГАМИ, а не готовой строкой: форма обращения у поля и у элемента
/// разная (`po.lo` против `bus[0]`), и различать их обязан разворот — тот же
/// приём, что у носителя вложенных агрегатов (0366).
#[derive(Clone, PartialEq)]
enum Step {
    Field(String),
    Index(i128),
}

/// Карта разворота: имя исходного порта → его листы `(путь, ячейка порта)`.
type LeafCells = BTreeMap<String, Vec<(Vec<Step>, Rc<RefCell<VariableNode>>)>>;

/// Лист развёрнутого порта: имя, путь, тип и смещение адреса в байтах.
struct Leaf {
    name: String,
    path: Vec<Step>,
    /// Позиции шагов в ПОЗИЦИОННОМ агрегате: индекс элемента массива либо номер
    /// поля структуры в порядке объявления (правило 0034).
    ///
    /// ⚠️ Ведётся параллельно пути: по самому пути позицию поля не восстановить
    /// — тип структуры лежит в модели, а начальное значение раздаётся листам
    /// уже без неё (фича 0451).
    positions: Vec<usize>,
    ty: TypeNode,
    offset: i128,
}

/// Что разворачивать (фича 0417).
///
/// ⚠️ Вид составного типа значим: порт-СТРУКТУРУ цели `st` и `sv` печатают
/// сами (0390), а порт-МАССИВ не умеет никто — у `st` вывод отвергает `iec2c`
/// при нулевом коде возврата, у `sv` цель отказывает `SV-002`. Поэтому массив
/// разворачивается всем, а структура — только тем, кто её не умеет.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum PortSplit {
    /// Только массивы (цели `st` и `sv`).
    ArraysOnly,
    /// Массивы и структуры (цели, у которых составного порта нет вовсе).
    All,
}

/// Разворачивает составные порты по всему дереву.
pub(crate) fn split_composite_ports(
    root: &Rc<RefCell<ModelNode>>,
    what: PortSplit,
) -> Result<(), Diagnostic> {
    let mut visited = HashSet::new();
    split_model(root, what, &mut visited)
}

fn split_model(
    model: &Rc<RefCell<ModelNode>>,
    what: PortSplit,
    visited: &mut HashSet<*const RefCell<ModelNode>>,
) -> Result<(), Diagnostic> {
    if !visited.insert(Rc::as_ptr(model)) {
        return Ok(());
    }
    let nested: Vec<Rc<RefCell<ModelNode>>> = model.borrow().models.values().cloned().collect();
    split_here(model, what)?;
    for child in &nested {
        split_model(child, what, visited)?;
    }
    Ok(())
}

/// Разворачивает порты ОДНОЙ модели и переписывает её тела.
fn split_here(model: &Rc<RefCell<ModelNode>>, what: PortSplit) -> Result<(), Diagnostic> {
    // Какие порты разворачивать — решается до правки: обход тел спрашивает
    // готовую карту, а не ищет объявление заново.
    let targets: Vec<(String, VariableNode)> = model
        .borrow()
        .variables
        .iter()
        .filter(|(_, var)| matches!(var, VariableNode::Port { ty, .. } if is_composite(ty, what)))
        .map(|(name, var)| (name.clone(), var.clone()))
        .collect();
    if targets.is_empty() {
        return Ok(());
    }

    let mut cells: LeafCells = BTreeMap::new();
    for (name, var) in &targets {
        let VariableNode::Port {
            ty,
            address,
            direction,
            loc,
            init,
            ..
        } = var
        else {
            continue;
        };
        let mut leaves = Vec::new();
        collect_leaves(name, &[], &[], ty, 0, &model.borrow(), &mut leaves)?;
        let base = literal_address(address);
        let mut made = Vec::new();
        for leaf in &leaves {
            let port = VariableNode::Port {
                upper: Some(Rc::downgrade(model)),
                loc: *loc,
                name: leaf.name.clone(),
                ty: leaf.ty.clone(),
                // Адрес листа — базовый плюс смещение поля, ЧИСЛОМ: форма
                // `Address` несёт ещё и позицию бита, а у поля структуры её
                // нет — `SE-060` отвергал бы любой лист.
                address: match base {
                    Some(value) => ExpressionNode::Number(value + leaf.offset),
                    None => ExpressionNode::None,
                },
                // Начальное значение листа берётся из АГРЕГАТА исходного порта
                // (фича 0451). Прежде оно выбрасывалось: `out a: [u8;3] := {4,
                // 5, 6}` не писал ничего, тогда как эталон показывал `[4,5,6]`,
                // — молчаливое расхождение при валидном выводе всех целей.
                init: leaf_initializer(init, &leaf.positions),
                direction: *direction,
            };
            model
                .borrow_mut()
                .variables
                .insert(leaf.name.clone(), port.clone());
            made.push((leaf.path.clone(), Rc::new(RefCell::new(port))));
        }
        cells.insert(name.clone(), made);
        model.borrow_mut().variables.remove(name);
    }

    rewrite_bodies(model, &cells);
    Ok(())
}

/// Составной ли тип порта: структура или массив (фича 0417).
///
/// ⚠️ Бит-вектор `[bit; N ≤ 64]` — **скаляр** (правило 0078), а не массив:
/// разворачивать его значило бы превратить упакованное слово в набор портов.
fn is_composite(ty: &TypeNode, what: PortSplit) -> bool {
    match ty {
        TypeNode::Struct(_) => what == PortSplit::All,
        TypeNode::Array(..) => crate::semantic::bit_vector::is_bit_vector(ty).is_none(),
        _ => false,
    }
}

/// Листья структуры: имя `<порт>_<поле>`, тип и смещение в байтах.
fn collect_leaves(
    prefix: &str,
    path: &[Step],
    positions: &[usize],
    ty: &TypeNode,
    offset: i128,
    model: &ModelNode,
    out: &mut Vec<Leaf>,
) -> Result<(), Diagnostic> {
    // Массив раскрывается по элементам: имя `<порт>_<индекс>` (фича 0417).
    // Бит-вектор сюда не попадает — он скаляр (правило 0078).
    if let TypeNode::Array(size, elem) = ty
        && crate::semantic::bit_vector::is_bit_vector(ty).is_none()
    {
        let step = size_of(elem, model);
        for index in 0..i128::from(*size) {
            let mut next = path.to_vec();
            next.push(Step::Index(index));
            let mut next_positions = positions.to_vec();
            next_positions.push(usize::try_from(index).unwrap_or(0));
            collect_leaves(
                &format!("{prefix}_{index}"),
                &next,
                &next_positions,
                elem,
                offset + index * step,
                model,
                out,
            )?;
        }
        return Ok(());
    }
    let TypeNode::Struct(name) = ty else {
        out.push(Leaf {
            name: prefix.to_string(),
            path: path.to_vec(),
            positions: positions.to_vec(),
            ty: ty.clone(),
            offset,
        });
        return Ok(());
    };
    let def = model.search_struct(name).ok_or_else(|| {
        Diagnostic::error(
            Location::Codegen,
            format!("структура '{name}' не объявлена: порт не разворачивается"),
        )
        .with_code("SE-119")
    })?;
    let mut at = offset;
    for (position, (field, field_ty)) in def.fields.iter().enumerate() {
        let mut next = path.to_vec();
        next.push(Step::Field(field.clone()));
        let mut next_positions = positions.to_vec();
        next_positions.push(position);
        collect_leaves(
            &format!("{prefix}_{field}"),
            &next,
            &next_positions,
            field_ty,
            at,
            model,
            out,
        )?;
        at += size_of(field_ty, model);
    }
    Ok(())
}

/// Размер типа в байтах — для смещения адреса листа.
///
/// ⚠️ Раскладка **плотная**, без выравнивания: у регистрового файла и MMIO
/// поля идут подряд, а выравнивание — свойство хоста, о котором модель не
/// говорит.
fn size_of(ty: &TypeNode, model: &ModelNode) -> i128 {
    match ty {
        TypeNode::Bit | TypeNode::Bool => 1,
        TypeNode::Integer { bits, .. } => i128::from(bits.div_ceil(8)),
        TypeNode::Duration => 4,
        TypeNode::Fixed { m, n, .. } => i128::from((m + n).div_ceil(8)),
        TypeNode::Enum(_) => 1,
        TypeNode::Array(size, elem) => i128::from(*size) * size_of(elem, model),
        TypeNode::Struct(name) => model.search_struct(name).map_or(1, |def| {
            def.fields.iter().map(|(_, t)| size_of(t, model)).sum()
        }),
        _ => 1,
    }
}

/// Базовый адрес порта, если он задан литералом.
fn literal_address(expr: &ExpressionNode) -> Option<i128> {
    match expr {
        ExpressionNode::Address(value, _) => Some(i128::from(*value)),
        ExpressionNode::Number(value) => Some(*value),
        _ => None,
    }
}

/// Переписывает тела модели: агрегат — по листам, поле — прямым обращением.
fn rewrite_bodies(model: &Rc<RefCell<ModelNode>>, cells: &LeafCells) {
    let (mut functions, mut named_blocks, mut states) = {
        let mut b = model.borrow_mut();
        (
            std::mem::take(&mut b.functions),
            std::mem::take(&mut b.named_blocks),
            std::mem::take(&mut b.states),
        )
    };
    for func in functions.values_mut() {
        if let crate::semantic::FunctionDefinitionNode::Local { body, .. } = func {
            rewrite_stmt(body, cells);
        }
    }
    for blk in named_blocks.iter_mut() {
        rewrite_block(blk, cells);
    }
    for state in states.values_mut() {
        if let StateNode::Simple { named_blocks, .. } | StateNode::Implement { named_blocks, .. } =
            state
        {
            for blk in named_blocks.iter_mut() {
                rewrite_block(blk, cells);
            }
        }
    }
    let mut b = model.borrow_mut();
    b.functions = functions;
    b.named_blocks = named_blocks;
    b.states = states;
}

fn rewrite_block(blk: &mut NamedCodeBlockDefinitionNode, cells: &LeafCells) {
    match blk {
        NamedCodeBlockDefinitionNode::Enter { body, .. }
        | NamedCodeBlockDefinitionNode::Exit { body, .. }
        | NamedCodeBlockDefinitionNode::Always { body, .. }
        | NamedCodeBlockDefinitionNode::Unknown { body, .. }
        | NamedCodeBlockDefinitionNode::Every { body, .. } => rewrite_stmt(body, cells),
        NamedCodeBlockDefinitionNode::None | NamedCodeBlockDefinitionNode::Unresolved(_, _) => {}
    }
}

fn rewrite_stmt(stmt: &mut StatementNode, cells: &LeafCells) {
    match stmt {
        StatementNode::Block(items) => {
            let mut out = Vec::with_capacity(items.len());
            for mut item in std::mem::take(items) {
                rewrite_stmt(&mut item, cells);
                match split_aggregate_assign(&item, cells) {
                    Some(parts) => out.extend(parts),
                    None => out.push(item),
                }
            }
            *items = out;
        }
        StatementNode::Expression(..) => {
            // Агрегатное присваивание порту разворачивается в блок по листам;
            // прочее — обычный обход выражения.
            if let Some(parts) = split_aggregate_assign(stmt, cells) {
                *stmt = StatementNode::Block(parts);
                return;
            }
            if let StatementNode::Expression(expr, _) = stmt {
                rewrite_expr(expr, cells);
            }
        }
        StatementNode::If { then_, else_, .. } => {
            rewrite_stmt(then_, cells);
            if let Some(alt) = else_ {
                rewrite_stmt(alt, cells);
            }
        }
        StatementNode::Loop { body, .. } => rewrite_stmt(body, cells),
        StatementNode::For { init, body, .. } => {
            if let Some(i) = init {
                rewrite_stmt(i, cells);
            }
            rewrite_stmt(body, cells);
        }
        StatementNode::Match { arms, .. } => {
            for arm in arms.iter_mut() {
                rewrite_stmt(&mut arm.body, cells);
            }
        }
        StatementNode::Return(Some(expr)) => rewrite_expr(expr, cells),
        StatementNode::Variable(_, _, Some(init), _) => rewrite_expr(init, cells),
        _ => {}
    }
}

/// `po := {a, b};` → по присваиванию на лист. `None` — не тот случай.
fn split_aggregate_assign(stmt: &StatementNode, cells: &LeafCells) -> Option<Vec<StatementNode>> {
    let StatementNode::Expression(expr, loc) = stmt else {
        return None;
    };
    let ExpressionNode::Assign(target, value) = &**expr else {
        return None;
    };
    let ExpressionNode::Variable(var) = &**target else {
        return None;
    };
    let name = var.borrow().name().to_string();
    let leaves = cells.get(&name)?;
    // Правая часть бывает двух видов, и оба штатны: агрегат (`po := {1, 2};`)
    // и значение структурного типа (`po := v;`). Второй раскладывается
    // доступом к полю — иначе цель получала бы структуру в скалярном колбэке.
    let parts: Vec<ExpressionNode> = match &**value {
        ExpressionNode::Initializer(items) | ExpressionNode::Array(items) => {
            if items.len() != leaves.len() {
                return None; // длину судит `SE-123` — второй проверки здесь нет
            }
            items.to_vec()
        }
        other => leaves
            .iter()
            .map(|(path, _)| access_by_path(other, path, *loc))
            .collect(),
    };
    Some(
        leaves
            .iter()
            .zip(parts)
            .map(|((_, cell), item)| {
                let mut value = item;
                rewrite_expr(&mut value, cells);
                StatementNode::Expression(
                    Box::new(ExpressionNode::Assign(
                        Box::new(ExpressionNode::Variable(Rc::clone(cell))),
                        Box::new(value),
                    )),
                    *loc,
                )
            })
            .collect(),
    )
}

/// Ячейка листа по одному шагу от имени порта.
fn leaf_of(base: &ExpressionNode, step: &Step, cells: &LeafCells) -> Option<ExpressionNode> {
    let ExpressionNode::Variable(var) = base else {
        return None;
    };
    cells
        .get(var.borrow().name())
        .and_then(|leaves| {
            leaves
                .iter()
                .find(|(path, _)| path.as_slice() == [step.clone()])
        })
        .map(|(_, cell)| ExpressionNode::Variable(Rc::clone(cell)))
}

/// Начальное значение листа: часть агрегата по позициям пути (фича 0451).
///
/// `ExpressionNode::None` — начального значения не было либо оно не агрегат:
/// подставлять целое значение каждому листу нельзя, а гадать не о чем.
///
/// ⚠️ Разбирается **значение**, а не форма записи: агрегат массива и
/// инициализатор структуры — разные узлы, а позиционный доступ у них общий
/// (порядок элементов значим — правило 0034).
fn leaf_initializer(init: &ExpressionNode, positions: &[usize]) -> ExpressionNode {
    let mut value = init;
    for position in positions {
        while let ExpressionNode::Parenthesis(inner) = value {
            value = inner;
        }
        let items = match value {
            ExpressionNode::Array(items) | ExpressionNode::Initializer(items) => items,
            _ => return ExpressionNode::None,
        };
        match items.get(*position) {
            Some(next) => value = next,
            None => return ExpressionNode::None,
        }
    }
    value.clone()
}

/// Обращение к части значения по пути листа: поле — точкой, элемент — индексом.
fn access_by_path(value: &ExpressionNode, path: &[Step], loc: Location) -> ExpressionNode {
    let mut expr = value.clone();
    for step in path {
        expr = match step {
            Step::Field(name) => ExpressionNode::BitAccess(
                Box::new(expr),
                Member::Identifier(crate::parser::ast::Identifier {
                    loc,
                    name: name.clone(),
                }),
            ),
            Step::Index(index) => ExpressionNode::ArraySubscript(
                Box::new(expr),
                Box::new(ExpressionNode::Number(*index)),
            ),
        };
    }
    expr
}

/// `po.lo` → порт листа; прочее — рекурсия.
fn rewrite_expr(expr: &mut ExpressionNode, cells: &LeafCells) {
    // Замена вычисляется ОТДЕЛЬНО: заимствование `expr` живо, пока в нём
    // ищут лист, и присвоение внутри `if let` компилятор не пропустит.
    let replacement = match expr {
        ExpressionNode::BitAccess(base, Member::Identifier(field)) => {
            leaf_of(base, &Step::Field(field.name.clone()), cells)
        }
        // Элемент массива-порта (фича 0417). Индекс обязан быть ЛИТЕРАЛОМ:
        // при переменном лист неизвестен, и такой вход уходит прежним путём —
        // к отказу цели, а не к молчаливо неверному обращению.
        ExpressionNode::ArraySubscript(base, index) => match &**index {
            ExpressionNode::Number(value) => leaf_of(base, &Step::Index(*value), cells),
            _ => None,
        },
        _ => None,
    };
    if let Some(new_expr) = replacement {
        *expr = new_expr;
        return;
    }
    match expr {
        ExpressionNode::Assign(l, r) => {
            rewrite_expr(l, cells);
            rewrite_expr(r, cells);
        }
        ExpressionNode::Parenthesis(inner)
        | ExpressionNode::Not(inner)
        | ExpressionNode::Negate(inner)
        | ExpressionNode::BitwiseNot(inner)
        | ExpressionNode::UnaryPlus(inner)
        | ExpressionNode::Cast(inner, _) => rewrite_expr(inner, cells),
        ExpressionNode::Add(l, r)
        | ExpressionNode::Subtract(l, r)
        | ExpressionNode::Multiply(l, r)
        | ExpressionNode::Divide(l, r)
        | ExpressionNode::Modulo(l, r)
        | ExpressionNode::Power(l, r)
        | ExpressionNode::ShiftLeft(l, r)
        | ExpressionNode::ShiftRight(l, r)
        | ExpressionNode::BitwiseAnd(l, r)
        | ExpressionNode::BitwiseOr(l, r)
        | ExpressionNode::BitwiseXor(l, r)
        | ExpressionNode::Equal(l, r)
        | ExpressionNode::NotEqual(l, r)
        | ExpressionNode::Less(l, r)
        | ExpressionNode::LessEqual(l, r)
        | ExpressionNode::More(l, r)
        | ExpressionNode::MoreEqual(l, r)
        | ExpressionNode::And(l, r)
        | ExpressionNode::Or(l, r) => {
            rewrite_expr(l, cells);
            rewrite_expr(r, cells);
        }
        ExpressionNode::Function(_, args)
        | ExpressionNode::Array(args)
        | ExpressionNode::Initializer(args) => {
            for a in args.iter_mut() {
                rewrite_expr(a, cells);
            }
        }
        _ => {}
    }
}
