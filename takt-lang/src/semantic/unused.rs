//! Диагностика Ce13: обнаружение неиспользуемых переменных.
//!
//! Функция [`check_unused_variables`] обходит все выражения, операторы
//! и условия модели, собирает множество используемых переменных и
//! возвращает предупреждения для каждой объявленной, но неиспользуемой.
//!
//! Функция [`compute_usage`] вычисляет множество используемых имён
//! по всем категориям элементов модели.

use crate::diagnostics::Diagnostic;
use crate::semantic::{
    ConditionDefinitionNode, ConditionNode, ExpressionNode, Formula, FunctionDefinitionNode,
    ModelNode, NamedCodeBlockDefinitionNode, StatementNode, VariableNode,
};
use crate::verification::ltl::Ltl;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

/// Множество использованных имён в модели.
#[derive(Debug, Default)]
pub struct UsageSet {
    /// Используемые переменные (var)
    pub variables: HashSet<String>,
    /// Используемые константы (const) — ключом [`const_key`], а не голым именем
    /// (фича 0193).
    pub constants: HashSet<String>,
    /// Используемые порты (port)
    pub ports: HashSet<String>,
    /// Используемые функции (fn / extern fn)
    pub functions: HashSet<String>,
}

/// Ключ константы в [`UsageSet::constants`]: **владелец + имя** (фича 0193).
///
/// # Почему не голое имя
///
/// Голым именем `A::K` и `B::K` делят одну запись, и если используется хотя бы
/// одна — фильтр пропускает обе. Пока цели печатали константу голым именем, это
/// было безвредно (объявление всё равно оставалось одно); после квалификации
/// (0193) печатаются **обе**, и неиспользуемая — это отказ сборки, а не
/// предупреждение: `error: constant 'K' is never used` под `-D warnings` цели
/// `rust` (замер 2026-08-03).
///
/// # Инвариант
///
/// ⚠️ Продюсер (этот модуль) и потребители (`rust_decl`, `sv_const`, `c_decl`)
/// обязаны строить **одну** строку — урок [`qualified_port_key`] фичи 0084:
/// разъехавшись на символ, они дают либо потерянное объявление, либо ссылку в
/// пустоту. Поэтому владелец берётся **из `upper` узла константы**, а не из
/// «модели, которую печатаем», — как и печатаемое имя.
///
/// Разделитель `\u{1}` в идентификаторах языка невозможен, поэтому склейка
/// однозначна (тот же приём, что у ключа карты адресов).
///
/// [`qualified_port_key`]: crate::address_map::resolve::qualified_port_key
pub fn const_key(upper: Option<&std::rc::Weak<RefCell<ModelNode>>>, name: &str) -> String {
    let Some(owner) = upper.and_then(|u| u.upgrade()) else {
        return name.to_string();
    };
    let model: crate::semantic::minimap::Name = owner.into();
    format!("{}\u{1}{name}", model.unique())
}

/// Отмечает использование переменной/константы/порта в [`UsageSet`].
///
/// Вынесено (фича 0193): один и тот же `match` стоял в **четырёх** местах, и
/// ключ константы обязан строиться везде одинаково — четыре копии правила
/// разъехались бы при первой же правке.
fn note_variable_usage(var: &VariableNode, set: &mut UsageSet) {
    match var {
        VariableNode::Simple { name, .. } => {
            set.variables.insert(name.clone());
        }
        VariableNode::Port { name, .. } => {
            set.ports.insert(name.clone());
        }
        VariableNode::Const { upper, name, .. } => {
            set.constants.insert(const_key(upper.as_ref(), name));
        }
        VariableNode::Unresolved => {}
    }
}

/// Вычисляет множество используемых имён для всех элементов модели.
///
/// Обходит те же элементы, что и [`check_unused_variables`]:
/// переменные, функции, именованные условия, блоки, состояния.
pub fn compute_usage(model: Rc<RefCell<ModelNode>>) -> UsageSet {
    let mut set = UsageSet {
        variables: HashSet::new(),
        constants: HashSet::new(),
        ports: HashSet::new(),
        functions: HashSet::new(),
    };
    collect_model_usage(model, &mut set);
    set
}

/// Рекурсивный обход модели для сбора множества используемых имён.
fn collect_model_usage(model: Rc<RefCell<ModelNode>>, set: &mut UsageSet) {
    let borrowed = model.borrow();

    // Инициализаторы переменных
    for var in borrowed.variables.values() {
        usage_from_var(var, set);
    }

    // Тела функций
    for func in borrowed.functions.values() {
        usage_from_func(func, set);
    }

    // Именованные условия
    for cond in borrowed.conditions.values() {
        usage_from_condition_node(cond, set);
    }

    // Именованные блоки модели
    for block in &borrowed.named_blocks {
        usage_from_named_block(block, set);
    }

    // Состояния
    let states: Vec<_> = borrowed.states.values().cloned().collect();
    drop(borrowed);

    for state in &states {
        usage_from_state(state, set);
    }

    // Рекурсивно для вложенных моделей
    let borrowed = model.borrow();
    let nested: Vec<Rc<RefCell<ModelNode>>> = borrowed.models.values().map(Rc::clone).collect();
    drop(borrowed);

    for nested_model in nested {
        collect_model_usage(nested_model, set);
    }
}

/// Записывает имена из инициализатора переменной в соответствующие множества.
fn usage_from_var(var: &VariableNode, set: &mut UsageSet) {
    match var {
        VariableNode::Simple { expr, .. } | VariableNode::Const { expr, .. } => {
            usage_from_expr(expr, set)
        }
        // Порт несёт два выражения (0187): пропустив адрес, проверка «переменная
        // нигде не используется» солгала бы про константу из `at BASE + 4`.
        VariableNode::Port {
            name,
            address,
            init,
            ..
        } => {
            usage_from_expr(address, set);
            usage_from_expr(init, set);
            // Начальное значение — это ЗАПИСЬ в порт (фича 0187): порт задействован,
            // даже если тело автомата к нему не обращается. Иначе цель `rust`
            // (она эмитит только использованные порты) не завела бы вариант
            // перечисления, а эмиссия значения сослалась бы на несуществующее имя.
            if !matches!(init, ExpressionNode::None) {
                set.ports.insert(name.clone());
            }
        }
        VariableNode::Unresolved => {}
    }
}

/// Записывает имена из выражения в соответствующие множества.
fn usage_from_expr(expr: &ExpressionNode, set: &mut UsageSet) {
    match expr {
        ExpressionNode::Variable(var_rc) => note_variable_usage(&var_rc.borrow(), set),
        // ⚠️ Индекс — тоже использование (фича 0210): `got := mem[pc];` читает
        // `pc`. Прежде второй элемент игнорировался (`_`), и переменная,
        // стоявшая ТОЛЬКО индексом, получала ложное `SE-036` «объявлена, но
        // нигде не используется» — компилятор говорил автору неправду о его же
        // коде. С фичи 0210 индекс — произвольное выражение, и умолчание стало
        // бы неверным чаще: в нём появляются целые подвыражения.
        ExpressionNode::ArraySubscript(var_rc, index) => {
            note_variable_usage(&var_rc.borrow(), set);
            usage_from_expr(index, set);
        }
        // Границы среза — числа (грамматика), читать в них нечего.
        ExpressionNode::ArraySlice(var_rc, _, _) => note_variable_usage(&var_rc.borrow(), set),
        ExpressionNode::Function(func_rc, args) => {
            // Регистрируем использованную функцию
            let func_name = func_rc.borrow().name().to_string();
            if !func_name.is_empty() {
                set.functions.insert(func_name);
            }
            for arg in args {
                usage_from_expr(arg, set);
            }
        }
        ExpressionNode::Not(e)
        | ExpressionNode::BitwiseNot(e)
        | ExpressionNode::UnaryPlus(e)
        | ExpressionNode::Negate(e)
        | ExpressionNode::Parenthesis(e)
        | ExpressionNode::BitAccess(e, _)
        | ExpressionNode::Cast(e, _) => usage_from_expr(e, set),
        ExpressionNode::Add(l, r)
        | ExpressionNode::Subtract(l, r)
        | ExpressionNode::Multiply(l, r)
        | ExpressionNode::Divide(l, r)
        | ExpressionNode::Modulo(l, r)
        | ExpressionNode::Power(l, r)
        | ExpressionNode::BitwiseAnd(l, r)
        | ExpressionNode::BitwiseXor(l, r)
        | ExpressionNode::BitwiseOr(l, r)
        | ExpressionNode::ShiftLeft(l, r)
        | ExpressionNode::ShiftRight(l, r)
        | ExpressionNode::And(l, r)
        | ExpressionNode::Or(l, r)
        | ExpressionNode::Equal(l, r)
        | ExpressionNode::NotEqual(l, r)
        | ExpressionNode::Less(l, r)
        | ExpressionNode::More(l, r)
        | ExpressionNode::LessEqual(l, r)
        | ExpressionNode::MoreEqual(l, r)
        | ExpressionNode::Assign(l, r) => {
            usage_from_expr(l, set);
            usage_from_expr(r, set);
        }
        ExpressionNode::ConditionalOperator(cond, then_e, else_e) => {
            usage_from_expr(cond, set);
            usage_from_expr(then_e, set);
            usage_from_expr(else_e, set);
        }
        ExpressionNode::Array(items) | ExpressionNode::Initializer(items) => {
            for item in items {
                usage_from_expr(item, set);
            }
        }
        _ => {}
    }
}

/// Записывает имена из оператора в соответствующие множества.
/// Собирает использованные имена из оператора.
///
/// Открыт для генератора ST (фича 0041): там по телу функции нужно узнать, какие
/// переменные модели она трогает, — в IEC `FUNCTION` чистая и видит только свои
/// входы, поэтому такие переменные передаются ей через `VAR_IN_OUT`.
pub(crate) fn usage_from_stmt(stmt: &StatementNode, set: &mut UsageSet) {
    match stmt {
        StatementNode::Block(stmts) => {
            for s in stmts {
                usage_from_stmt(s, set);
            }
        }
        StatementNode::Expression(e) => usage_from_expr(e, set),
        StatementNode::If { cond, then_, else_ } => {
            usage_from_expr(cond, set);
            usage_from_stmt(then_, set);
            if let Some(e) = else_ {
                usage_from_stmt(e, set);
            }
        }
        StatementNode::Loop { cond, body } => {
            if let Some(c) = cond {
                usage_from_expr(c, set);
            }
            usage_from_stmt(body, set);
        }
        StatementNode::For {
            init,
            cond,
            step,
            body,
        } => {
            if let Some(s) = init {
                usage_from_stmt(s, set);
            }
            if let Some(c) = cond {
                usage_from_expr(c, set);
            }
            if let Some(s) = step {
                usage_from_expr(s, set);
            }
            usage_from_stmt(body, set);
        }
        StatementNode::Variable(_, _, Some(e)) => usage_from_expr(e, set),
        StatementNode::Return(Some(e)) => usage_from_expr(e, set),
        StatementNode::Match { expr, arms } => {
            usage_from_expr(expr, set);
            for arm in arms {
                usage_from_stmt(&arm.body, set);
            }
        }
        _ => {}
    }
}

/// Записывает имена из условия в соответствующие множества.
fn usage_from_condition(cond: &ConditionNode, set: &mut UsageSet) {
    match cond {
        ConditionNode::Variable(var_rc, _) => note_variable_usage(&var_rc.borrow(), set),
        ConditionNode::Not(c) | ConditionNode::Parenthesis(c) => usage_from_condition(c, set),
        ConditionNode::And(l, r)
        | ConditionNode::Or(l, r)
        | ConditionNode::Equal(l, r)
        | ConditionNode::NotEqual(l, r)
        | ConditionNode::Less(l, r)
        | ConditionNode::More(l, r)
        | ConditionNode::LessEqual(l, r)
        | ConditionNode::MoreEqual(l, r)
        | ConditionNode::Add(l, r)
        | ConditionNode::Subtract(l, r) => {
            usage_from_condition(l, set);
            usage_from_condition(r, set);
        }
        ConditionNode::Function(func_rc, args, _) => {
            // Регистрируем использованную функцию
            let func_name = func_rc.borrow().name().to_string();
            if !func_name.is_empty() {
                set.functions.insert(func_name);
            }
            for arg in args {
                usage_from_condition(arg, set);
            }
        }
        ConditionNode::ArraySubscript(var_rc, index) => {
            note_variable_usage(&var_rc.borrow(), set);
            usage_from_condition(index, set);
        }
        ConditionNode::BitAccess(inner, _) => usage_from_condition(inner, set),
        // Вычисляемая выдержка (фича 0183) читает переменные и порты — это
        // настоящее использование. ⚠️ Без этой ветви `after (base + 500ms)`
        // давало ложное «переменная 'base' нигде не используется», то есть
        // компилятор говорил автору неправду о его же коде (класс, разобранный
        // задачей 0134-03). Компилятор здесь не помогает: разбор кончается
        // `_ => {}`.
        ConditionNode::AfterExpr(inner) => usage_from_condition(inner, set),
        _ => {}
    }
}

/// Записывает имена из именованного условия в соответствующие множества.
fn usage_from_condition_node(cond_node: &ConditionDefinitionNode, set: &mut UsageSet) {
    usage_from_condition(&cond_node.value, set);
}

/// Записывает имена из именованного блока кода в соответствующие множества.
fn usage_from_named_block(block: &NamedCodeBlockDefinitionNode, set: &mut UsageSet) {
    if let Some(stmt) = block.statement() {
        usage_from_stmt(stmt, set);
    }
}

/// Записывает имена из тела функции в соответствующие множества.
fn usage_from_func(func: &FunctionDefinitionNode, set: &mut UsageSet) {
    if let FunctionDefinitionNode::Local { body, .. } = func {
        usage_from_stmt(body, set);
    }
}

/// Записывает имена из состояния в соответствующие множества.
fn usage_from_state(state: &crate::semantic::StateNode, set: &mut UsageSet) {
    use crate::semantic::StateNode;
    match state {
        StateNode::Simple {
            named_blocks,
            references,
            ..
        }
        | StateNode::Implement {
            named_blocks,
            references,
            ..
        } => {
            for block in named_blocks {
                usage_from_named_block(block, set);
            }
            for reference in references {
                usage_from_condition(&reference.cond, set);
            }
        }
        StateNode::Unresolved => {}
    }
}

/// Проверяет наличие неиспользуемых переменных в модели.
///
/// Возвращает список [`Diagnostic`] уровня Warning для каждой переменной,
/// объявленной через `var`, но ни разу не упомянутой в выражениях, условиях
/// или именованных блоках модели.
/// Порты и константы не проверяются — они могут быть внешним интерфейсом.
pub fn check_unused_variables(model: Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let mut warnings = Vec::new();
    check_model_unused(model, &mut warnings);
    warnings
}

/// Рекурсивно собирает все имена, используемые в `model` и во всех её вложенных моделях.
///
/// Вложенные модели могут обращаться к переменным родительской модели
/// (семантика видимости Takt: подмодель видит переменные родителя).
/// Чтобы не считать такие переменные неиспользуемыми, сбор использований
/// охватывает всё дерево моделей, начиная с `model`.
fn collect_from_model_tree(model: &Rc<RefCell<ModelNode>>, used: &mut HashSet<String>) {
    let borrowed = model.borrow();
    for var in borrowed.variables.values() {
        collect_from_var(var, used);
    }
    for func in borrowed.functions.values() {
        collect_from_func(func, used);
    }
    for cond in borrowed.conditions.values() {
        collect_from_condition_node(cond, used);
    }
    for block in &borrowed.named_blocks {
        collect_from_named_block(block, used);
    }
    // Формулы уровня модели (`invariant`, LTL): переменная, используемая только в
    // свойстве верификации, — тоже использование (фича 0082).
    for formula in &borrowed.formulas {
        collect_from_formula(formula, used);
    }
    let states: Vec<_> = borrowed.states.values().cloned().collect();
    let nested: Vec<Rc<RefCell<ModelNode>>> = borrowed.models.values().map(Rc::clone).collect();
    drop(borrowed);
    for state in &states {
        collect_from_state(state, used);
    }
    for nested_model in &nested {
        collect_from_model_tree(nested_model, used);
    }
}

fn check_model_unused(model: Rc<RefCell<ModelNode>>, warnings: &mut Vec<Diagnostic>) {
    let mut used: HashSet<String> = HashSet::new();

    // Собираем использования из кода текущей модели И всех вложенных моделей.
    // Подмодели могут обращаться к переменным родительской модели, поэтому
    // сканирование только текущего уровня приводит к ложным предупреждениям.
    collect_from_model_tree(&model, &mut used);

    // Проверяем каждую простую переменную (var)
    let borrowed = model.borrow();
    for (name, var) in &borrowed.variables {
        // Порты и константы не предупреждаем — они могут быть внешним интерфейсом
        if matches!(var, VariableNode::Port { .. } | VariableNode::Const { .. }) {
            continue;
        }
        if !used.contains(name.as_str()) {
            warnings.push(
                Diagnostic::warning(
                    var.loc(),
                    format!("переменная '{}' объявлена, но нигде не используется", name),
                )
                .with_code("SE-036"),
            );
        }
    }

    // Рекурсивно для вложенных моделей
    let nested: Vec<Rc<RefCell<ModelNode>>> = borrowed.models.values().map(Rc::clone).collect();
    drop(borrowed);

    for nested_model in nested {
        check_model_unused(nested_model, warnings);
    }
}

fn collect_from_var(var: &VariableNode, used: &mut HashSet<String>) {
    match var {
        VariableNode::Simple { expr, .. } | VariableNode::Const { expr, .. } => {
            collect_from_expr(expr, used)
        }
        VariableNode::Port { address, init, .. } => {
            collect_from_expr(address, used);
            collect_from_expr(init, used);
        }
        VariableNode::Unresolved => {}
    }
}

fn collect_from_expr(expr: &ExpressionNode, used: &mut HashSet<String>) {
    match expr {
        ExpressionNode::Variable(var_rc) => {
            let borrowed = var_rc.borrow();
            if let VariableNode::Simple { name, .. }
            | VariableNode::Port { name, .. }
            | VariableNode::Const { name, .. } = &*borrowed
            {
                used.insert(name.clone());
            }
        }
        ExpressionNode::ArraySubscript(var_rc, index) => {
            let borrowed = var_rc.borrow();
            if let VariableNode::Simple { name, .. }
            | VariableNode::Port { name, .. }
            | VariableNode::Const { name, .. } = &*borrowed
            {
                used.insert(name.clone());
            }
            // Индекс — использование (фича 0210); см. `usage_from_expr`.
            collect_from_expr(index, used);
        }
        ExpressionNode::ArraySlice(var_rc, _, _) => {
            let borrowed = var_rc.borrow();
            if let VariableNode::Simple { name, .. }
            | VariableNode::Port { name, .. }
            | VariableNode::Const { name, .. } = &*borrowed
            {
                used.insert(name.clone());
            }
        }
        ExpressionNode::Not(e)
        | ExpressionNode::BitwiseNot(e)
        | ExpressionNode::UnaryPlus(e)
        | ExpressionNode::Negate(e)
        | ExpressionNode::Parenthesis(e)
        | ExpressionNode::BitAccess(e, _)
        | ExpressionNode::Cast(e, _) => collect_from_expr(e, used),
        ExpressionNode::Add(l, r)
        | ExpressionNode::Subtract(l, r)
        | ExpressionNode::Multiply(l, r)
        | ExpressionNode::Divide(l, r)
        | ExpressionNode::Modulo(l, r)
        | ExpressionNode::Power(l, r)
        | ExpressionNode::BitwiseAnd(l, r)
        | ExpressionNode::BitwiseXor(l, r)
        | ExpressionNode::BitwiseOr(l, r)
        | ExpressionNode::ShiftLeft(l, r)
        | ExpressionNode::ShiftRight(l, r)
        | ExpressionNode::And(l, r)
        | ExpressionNode::Or(l, r)
        | ExpressionNode::Equal(l, r)
        | ExpressionNode::NotEqual(l, r)
        | ExpressionNode::Less(l, r)
        | ExpressionNode::More(l, r)
        | ExpressionNode::LessEqual(l, r)
        | ExpressionNode::MoreEqual(l, r)
        | ExpressionNode::Assign(l, r) => {
            collect_from_expr(l, used);
            collect_from_expr(r, used);
        }
        ExpressionNode::ConditionalOperator(cond, then_e, else_e) => {
            collect_from_expr(cond, used);
            collect_from_expr(then_e, used);
            collect_from_expr(else_e, used);
        }
        ExpressionNode::Function(_, args) => {
            for arg in args {
                collect_from_expr(arg, used);
            }
        }
        ExpressionNode::Array(items) | ExpressionNode::Initializer(items) => {
            for item in items {
                collect_from_expr(item, used);
            }
        }
        _ => {}
    }
}

fn collect_from_stmt(stmt: &StatementNode, used: &mut HashSet<String>) {
    match stmt {
        StatementNode::Block(stmts) => {
            for s in stmts {
                collect_from_stmt(s, used);
            }
        }
        StatementNode::Expression(e) => collect_from_expr(e, used),
        StatementNode::If { cond, then_, else_ } => {
            collect_from_expr(cond, used);
            collect_from_stmt(then_, used);
            if let Some(e) = else_ {
                collect_from_stmt(e, used);
            }
        }
        StatementNode::Loop { cond, body } => {
            if let Some(c) = cond {
                collect_from_expr(c, used);
            }
            collect_from_stmt(body, used);
        }
        StatementNode::For {
            init,
            cond,
            step,
            body,
        } => {
            if let Some(s) = init {
                collect_from_stmt(s, used);
            }
            if let Some(c) = cond {
                collect_from_expr(c, used);
            }
            if let Some(s) = step {
                collect_from_expr(s, used);
            }
            collect_from_stmt(body, used);
        }
        StatementNode::Variable(_, _, Some(e)) => collect_from_expr(e, used),
        StatementNode::Return(Some(e)) => collect_from_expr(e, used),
        StatementNode::Match { expr, arms } => {
            collect_from_expr(expr, used);
            for arm in arms {
                collect_from_stmt(&arm.body, used);
            }
        }
        _ => {}
    }
}

/// Собирает использования из формулы состояния/модели (фича 0082).
///
/// Переменная, встречающаяся **только** в LTL/Guard-формуле (`: [G] flag = 1;`,
/// `invariant Имя = flag;`), — это использование: свойство верификации на неё
/// опирается. Без обхода формул Ce13 (`SE-036`) давал **ложное** предупреждение
/// «переменная не используется».
fn collect_from_formula(formula: &Formula, used: &mut HashSet<String>) {
    match formula {
        Formula::None => {}
        Formula::Formulas(inner) => {
            for f in inner {
                collect_from_formula(f, used);
            }
        }
        // Guard несёт `ConditionNode` — тот же обход, что и у условий рёбер.
        Formula::Guard(cond, _) => collect_from_condition(cond, used),
        Formula::LTL(ltl) => collect_from_ltl(ltl, used),
    }
}

/// Собирает имена атомов LTL-формулы. Атом — имя переменной **или** состояния;
/// имя состояния в `used` безвредно (проверяются только имена переменных).
fn collect_from_ltl(ltl: &Ltl, used: &mut HashSet<String>) {
    match ltl {
        Ltl::True | Ltl::False => {}
        Ltl::Atom(name) => {
            used.insert(name.clone());
        }
        Ltl::Not(a) | Ltl::Next(a) | Ltl::Finally(a) | Ltl::Globally(a) => {
            collect_from_ltl(a, used)
        }
        Ltl::And(a, b)
        | Ltl::Or(a, b)
        | Ltl::Implies(a, b)
        | Ltl::Until(a, b)
        | Ltl::Release(a, b) => {
            collect_from_ltl(a, used);
            collect_from_ltl(b, used);
        }
    }
}

fn collect_from_condition(cond: &ConditionNode, used: &mut HashSet<String>) {
    match cond {
        ConditionNode::Variable(var_rc, _) => {
            let borrowed = var_rc.borrow();
            if let VariableNode::Simple { name, .. }
            | VariableNode::Port { name, .. }
            | VariableNode::Const { name, .. } = &*borrowed
            {
                used.insert(name.clone());
            }
        }
        ConditionNode::Not(c) | ConditionNode::Parenthesis(c) => collect_from_condition(c, used),
        ConditionNode::And(l, r)
        | ConditionNode::Or(l, r)
        | ConditionNode::Equal(l, r)
        | ConditionNode::NotEqual(l, r)
        | ConditionNode::Less(l, r)
        | ConditionNode::More(l, r)
        | ConditionNode::LessEqual(l, r)
        | ConditionNode::MoreEqual(l, r)
        | ConditionNode::Add(l, r)
        | ConditionNode::Subtract(l, r) => {
            collect_from_condition(l, used);
            collect_from_condition(r, used);
        }
        ConditionNode::Function(_, args, _) => {
            for arg in args {
                collect_from_condition(arg, used);
            }
        }
        ConditionNode::ArraySubscript(var_rc, index) => {
            let borrowed = var_rc.borrow();
            if let VariableNode::Simple { name, .. }
            | VariableNode::Port { name, .. }
            | VariableNode::Const { name, .. } = &*borrowed
            {
                used.insert(name.clone());
            }
            collect_from_condition(index, used);
        }
        ConditionNode::BitAccess(inner, _) => collect_from_condition(inner, used),
        // Вычисляемая выдержка (фича 0183) читает переменные и порты.
        //
        // ⚠️ Это **второй** сборщик использований по условиям в этом же модуле
        // (первый — `usage_from_condition`), и правка одного оставляла ложное
        // «переменная 'base' нигде не используется» — компилятор говорил автору
        // неправду о его же коде. Оба кончаются `_ => {}`, поэтому ни один узел
        // языка здесь не защищён исчерпаемостью: правя один, правь второй.
        ConditionNode::AfterExpr(inner) => collect_from_condition(inner, used),
        _ => {}
    }
}

fn collect_from_condition_node(cond_node: &ConditionDefinitionNode, used: &mut HashSet<String>) {
    collect_from_condition(&cond_node.value, used);
}

fn collect_from_named_block(block: &NamedCodeBlockDefinitionNode, used: &mut HashSet<String>) {
    if let Some(stmt) = block.statement() {
        collect_from_stmt(stmt, used);
    }
}

fn collect_from_func(func: &FunctionDefinitionNode, used: &mut HashSet<String>) {
    if let FunctionDefinitionNode::Local { body, .. } = func {
        collect_from_stmt(body, used);
    }
}

fn collect_from_state(state: &crate::semantic::StateNode, used: &mut HashSet<String>) {
    use crate::semantic::StateNode;
    match state {
        StateNode::Simple {
            named_blocks,
            references,
            formulas,
            ..
        }
        | StateNode::Implement {
            named_blocks,
            references,
            formulas,
            ..
        } => {
            for block in named_blocks {
                collect_from_named_block(block, used);
            }
            for reference in references {
                collect_from_condition(&reference.cond, used);
            }
            // Формулы состояния (`: [G] φ;`): переменная в свойстве — использование
            // (фича 0082).
            for formula in formulas {
                collect_from_formula(formula, used);
            }
        }
        StateNode::Unresolved => {}
    }
}
