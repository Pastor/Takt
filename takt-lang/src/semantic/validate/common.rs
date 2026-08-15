//! Общие помощники проверок: условия, выражения, доступ к состояниям.
//!
//! Часть модуля `validate` (фича 0027: деление по логике).

use super::*;
use crate::semantic::condition::state_of::state_of_model;

pub(super) fn validate_cond(
    context: Option<ConditionNode>,
    cond: &ConditionNode,
    model: Rc<RefCell<ModelNode>>,
) -> Result<(), Diagnostic> {
    let _borrowed = model.borrow();
    match cond.clone() {
        ConditionNode::None => {}
        ConditionNode::Unresolved(cond) => {
            #[allow(clippy::collapsible_if)]
            if let Some(context) = context
                && let ast::Condition::Variable(id) = cond.clone()
            {
                // Левый операнд — «текущее состояние модели»? Форму паттерна
                // разбирает ОДНА функция на проект (фича 0203): прежде судья
                // знал только `S(Модель)`, тогда как цели `c` и `rust` знают и
                // краткое `Модель`, — и `ref X: E != End;` отвергался `SE-025`
                // на записи, которую генератор переводит.
                if let Some(model) = state_of_model(&context) {
                    let model = model.borrow();
                    let model_name = model
                        .name
                        .clone()
                        .unwrap_or_else(|| "<анонимная>".to_string());
                    model.search_state(&id.name).ok_or_else(|| {
                        Diagnostic::error(
                            id.loc,
                            format!(
                                "Состояние '{}' не найдено в моделе '{}'",
                                id.name, model_name
                            ),
                        )
                        .with_code("SE-033")
                    })?;
                    return Ok(());
                }
            }

            if let ConditionNode::Unresolved(_) = resolve_condition(&cond, model.clone())? {
                // ⚠️ Цитата — текст исходника, а не `Debug`-дамп узла (фича
                // 0231): прежде сообщение выглядело как «Неразрешённое условие:
                // Variable(Identifier { loc: Source(0, 51, 54), name: "qqq" })»
                // — внутреннее представление вместо записи автора. Печатью
                // занимается форматтер; узел, который он не умеет, оставляет
                // сообщение без цитаты — но дампа не будет никогда.
                let quoted = crate::format::condition_text(&cond)
                    .map(|text| format!(" '{text}'"))
                    .unwrap_or_default();
                return Err(Diagnostic::error(
                    cond.loc(),
                    format!(
                        "неразрешённое условие перехода{quoted}: имя не найдено среди \
                         переменных, портов, условий `cond` и состояний"
                    ),
                )
                .with_code("SE-025"));
            }
        }
        ConditionNode::ArraySubscript(_, _) => {}
        ConditionNode::Parenthesis(cond) => {
            validate_cond(None, &cond, model.clone())?;
        }
        ConditionNode::BitAccess(cond, _) => {
            validate_cond(None, &cond, model.clone())?;
        }
        ConditionNode::Function(_, conds, _) => {
            for cond in conds {
                validate_cond(None, &cond, model.clone())?;
            }
        }
        ConditionNode::Not(cond) => {
            validate_cond(None, &cond, model.clone())?;
        }
        ConditionNode::Add(left, right) => {
            validate_cond(None, &left, model.clone())?;
            validate_cond(None, &right, model.clone())?;
        }
        ConditionNode::Subtract(left, right) => {
            validate_cond(None, &left, model.clone())?;
            validate_cond(None, &right, model.clone())?;
        }
        ConditionNode::And(left, right) => {
            validate_cond(None, &left, model.clone())?;
            validate_cond(None, &right, model.clone())?;
        }
        ConditionNode::Or(left, right) => {
            validate_cond(None, &left, model.clone())?;
            validate_cond(None, &right, model.clone())?;
        }
        ConditionNode::Less(left, right) => {
            validate_cond(None, &left, model.clone())?;
            validate_cond(None, &right, model.clone())?;
        }
        ConditionNode::More(left, right) => {
            validate_cond(None, &left, model.clone())?;
            validate_cond(None, &right, model.clone())?;
        }
        ConditionNode::LessEqual(left, right) => {
            validate_cond(None, &left, model.clone())?;
            validate_cond(None, &right, model.clone())?;
        }
        ConditionNode::MoreEqual(left, right) => {
            validate_cond(None, &left, model.clone())?;
            validate_cond(None, &right, model.clone())?;
        }
        ConditionNode::Equal(left, right) => {
            validate_cond(None, &left, model.clone())?;
            validate_cond(Some(*left.clone()), &right, model.clone())?;
        }
        ConditionNode::NotEqual(left, right) => {
            validate_cond(None, &left, model.clone())?;
            // Передаём контекст левого операнда — как в Equal — для проверки
            // паттерна `S(Model) != СостояниеИмя`: имя состояния должно быть
            // валидным в указанной модели.
            validate_cond(Some(*left.clone()), &right, model.clone())?;
        }
        ConditionNode::Number(_) => {}
        ConditionNode::Duration(_) | ConditionNode::After(_) | ConditionNode::AfterTicks(_) => {}
        // Вычисляемая выдержка (фича 0183): вложенное выражение — обычное
        // условие, и его проверки (чтение `out`-порта, неизвестное имя) обязаны
        // работать так же, как везде.
        ConditionNode::AfterExpr(inner) => {
            validate_cond(None, &inner, model.clone())?;
        }
        ConditionNode::Rational(_, _) => {}
        ConditionNode::String(_) => {}
        ConditionNode::Bool(_) => {}
        // Обращение к ячейке в условии (фича 0189) — только чтение, проверять нечего.
        ConditionNode::AnonPort(_) => {}
        ConditionNode::Variable(var_rc, _) => {
            // Чтение из `out`-порта запрещено в условии (SE-027)
            if let VariableNode::Port {
                direction: PortDirection::Out,
                name,
                loc,
                ..
            } = &*var_rc.borrow()
            {
                return Err(Diagnostic::error(
                    *loc,
                    format!("Чтение из выходного порта '{}' запрещено", name),
                )
                .with_code("SE-027"));
            }
        }
        ConditionNode::Model(_model, _) => {}
        ConditionNode::State(_state, _) => {}
        ConditionNode::EnumVariant(_, _, _) => {}
    }
    Ok(())
}

/// Является ли переменная **выходным** портом.
///
/// Вынесено (фича 0188): предикат нужен и проверке чтения, и исключению для
/// левой части присваивания — два места, где ошибиться значит либо запретить
/// законную запись, либо разрешить незаконное чтение.
fn is_out_port(var: &Rc<RefCell<VariableNode>>) -> bool {
    matches!(
        &*var.borrow(),
        VariableNode::Port {
            direction: PortDirection::Out,
            ..
        }
    )
}

pub(super) fn validate_expression(
    expr: &ExpressionNode,
    model: Rc<RefCell<ModelNode>>,
) -> Result<(), Diagnostic> {
    let _borrowed = model.borrow();
    match expr {
        ExpressionNode::None => {}
        ExpressionNode::Unresolved(_) => {}
        ExpressionNode::ArraySubscript(_, _) => {}
        ExpressionNode::ArraySlice(_, _, _) => {}
        ExpressionNode::Parenthesis(expr)
        | ExpressionNode::BitAccess(expr, _)
        | ExpressionNode::CodeBlock(expr, _)
        | ExpressionNode::NamedFunctionBox(expr, _)
        | ExpressionNode::Not(expr)
        | ExpressionNode::UnaryPlus(expr)
        | ExpressionNode::Negate(expr)
        | ExpressionNode::Cast(expr, _)
        | ExpressionNode::BitwiseNot(expr) => {
            validate_expression(expr, model.clone())?;
        }
        ExpressionNode::Power(left, right)
        | ExpressionNode::Multiply(left, right)
        | ExpressionNode::Divide(left, right)
        | ExpressionNode::Modulo(left, right)
        | ExpressionNode::Add(left, right)
        | ExpressionNode::Subtract(left, right)
        | ExpressionNode::ShiftLeft(left, right)
        | ExpressionNode::ShiftRight(left, right)
        | ExpressionNode::BitwiseAnd(left, right)
        | ExpressionNode::BitwiseXor(left, right)
        | ExpressionNode::BitwiseOr(left, right)
        | ExpressionNode::Less(left, right)
        | ExpressionNode::More(left, right)
        | ExpressionNode::LessEqual(left, right)
        | ExpressionNode::MoreEqual(left, right)
        | ExpressionNode::Equal(left, right)
        | ExpressionNode::NotEqual(left, right)
        | ExpressionNode::And(left, right)
        | ExpressionNode::Or(left, right) => {
            validate_expression(left, model.clone())?;
            validate_expression(right, model.clone())?;
        }
        ExpressionNode::Assign(left, right) => {
            // Запись в `in`-порт запрещена (SE-026)
            let check_port = |expr: &ExpressionNode| {
                if let ExpressionNode::Variable(v) = expr {
                    return Some(v.clone());
                }
                if let ExpressionNode::BitAccess(inner, _) = expr
                    && let ExpressionNode::Variable(v) = inner.as_ref()
                {
                    return Some(v.clone());
                }
                None
            };
            if let Some(var_rc) = check_port(left)
                && let VariableNode::Port {
                    direction: PortDirection::In,
                    name,
                    loc,
                    ..
                } = &*var_rc.borrow()
            {
                return Err(Diagnostic::error(
                    *loc,
                    format!("Запись в входной порт '{}' запрещена", name),
                )
                .with_code("SE-026"));
            }
            // Левая часть присваивания — **место записи**, а не чтение:
            // рекурсировать в неё нельзя, иначе законное `led := 1;` для
            // выходного порта дало бы `SE-027` «чтение выходного порта».
            //
            // ⚠️ Прежде исключение было написано только для `BitAccess`
            // (`led.0 := 1;`), потому что проверка работала лишь на условиях, где
            // присваиваний не бывает. Фича 0188 распространила её на тела блоков
            // — и форма `led := 1;` стала достижимой, поэтому исключение
            // обобщено на обе формы цели записи.
            let target_is_out_port = match left.as_ref() {
                ExpressionNode::Variable(v) => is_out_port(v),
                ExpressionNode::BitAccess(inner, _) => match inner.as_ref() {
                    ExpressionNode::Variable(v) => is_out_port(v),
                    _ => false,
                },
                _ => false,
            };
            if !target_is_out_port {
                validate_expression(left, model.clone())?;
            }
            validate_expression(right, model.clone())?;
        }
        ExpressionNode::ConditionalOperator(left, right, other) => {
            validate_expression(left, model.clone())?;
            validate_expression(right, model.clone())?;
            validate_expression(other, model.clone())?;
        }
        ExpressionNode::Number(_) => {}
        ExpressionNode::Duration(_) => {}
        ExpressionNode::Rational(_, _) => {}
        ExpressionNode::String(_) => {}
        ExpressionNode::Type(_) => {}
        ExpressionNode::Address(_, _) => {}
        // Обращение к ячейке по адресу (фича 0189): направления у неё нет —
        // проверять нечего. Запись даёт **предупреждение** `SE-096`, а
        // предупреждения вырабатывает слой `semantic::warnings` (0081), не судья.
        ExpressionNode::AnonPort(_) => {}
        ExpressionNode::Bool(_) => {}
        ExpressionNode::Variable(var_rc) => {
            // Чтение из `out`-порта запрещено (SE-027)
            if let VariableNode::Port {
                direction: PortDirection::Out,
                name,
                loc,
                ..
            } = &*var_rc.borrow()
            {
                return Err(Diagnostic::error(
                    *loc,
                    format!("Чтение из выходного порта '{}' запрещено", name),
                )
                .with_code("SE-027"));
            }
        }
        ExpressionNode::Model(_model) => {}
        ExpressionNode::Condition(cond) => {
            validate_cond(None, &cond.borrow().value, model.clone())?;
        }
        ExpressionNode::List(_) => {}
        ExpressionNode::Array(exprs)
        | ExpressionNode::Initializer(exprs)
        | ExpressionNode::Function(_, exprs) => {
            for expr in exprs {
                validate_expression(expr, model.clone())?;
            }
        }
    }
    Ok(())
}

pub(super) fn validate_reference(
    reference: &ReferenceNode<StateNode>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<(), Diagnostic> {
    validate_cond(None, &reference.cond, model.clone())?;
    Ok(())
}

pub(super) fn validate_conditions(model: Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let borrowed = model.borrow();
    // Накопление по именованным условиям (фича 0151): каждое `cond` — своё
    // объявление.
    let mut out = Vec::new();
    for cond in borrowed.conditions.values() {
        out.extend(validate_cond(None, &cond.value, model.clone()).err());
    }
    out
}

pub(super) fn get_state_name(state: &StateNode) -> &str {
    match state {
        StateNode::Simple { name, .. } | StateNode::Implement { name, .. } => name.as_str(),
        StateNode::Unresolved => "",
    }
}

pub(super) fn get_state_loc(state: &StateNode) -> Location {
    match state {
        StateNode::Simple { loc, .. } | StateNode::Implement { loc, .. } => *loc,
        StateNode::Unresolved => Location::Builtin,
    }
}

/// Имена состояний, достижимых из `state` за один переход: цели `ref`-ссылок и
/// (для состояния-реализации) цель `next`.
///
/// Общий источник истины о рёбрах графа FSM: используется анализом
/// достижимости (SE-046) и построением структуры Крипке (фича 0049,
/// [`build_kripke`](crate::verification::kripke::build_kripke)).
pub(crate) fn reachable_targets(state: &StateNode) -> Vec<String> {
    match state {
        StateNode::Simple { references, .. } => references.iter().map(|r| r.name.clone()).collect(),
        StateNode::Implement {
            references, next, ..
        } => {
            let mut targets: Vec<String> = references.iter().map(|r| r.name.clone()).collect();
            if let Some(n) = next {
                targets.push(n.name.clone());
            }
            targets
        }
        StateNode::Unresolved => vec![],
    }
}
