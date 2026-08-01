//! Неявная булевость условий перехода (Ce11 / SE-037).
//!
//! Часть модуля `validate` (фича 0027: деление по логике).

use super::*;

/// Возвращает `true`, если AST-условие перехода гарантированно является булевым.
///
/// Используется для условий на рёбрах `ref`, которые в текущем конвейере
/// хранятся как [`ConditionNode::Unresolved`] и содержат «сырой» [`ast::Condition`].
///
/// ## Правила классификации
///
/// | Условие                                     | Результат |
/// |---------------------------------------------|-----------|
/// | Булев литерал (`true`, `false`)             | булево    |
/// | Сравнение (`=`, `!=`, `<`, `>`, `<=`, `>=`) | булево    |
/// | Логическое НЕ (`!x`)                        | булево    |
/// | Скобки (`(…)`)                              | рекурсия  |
/// | Вызов функции — тип неизвестен              | булево    |
/// | Переменная типа `bool` или `bit`            | булево    |
/// | Именованное условие (`cond`)                | булево    |
/// | Неизвестное имя                             | булево    |
/// | Числовой литерал                            | числовое  |
/// | Вещественный литерал                        | числовое  |
/// | Строковый литерал                           | числовое  |
/// | Арифметика (`+`, `-`)                       | числовое  |
/// | Побитовые операции (`&`, `\|`)              | числовое  |
/// | Элемент массива (`arr[n]`)                  | числовое  |
/// | Доступ к биту (`.n`)                        | числовое  |
/// | Переменная числового типа (`[bit;N]`)       | числовое  |
pub(super) fn is_boolean_ast_condition(
    cond: &ast_types::Condition,
    model: &Rc<RefCell<ModelNode>>,
) -> bool {
    use ast_types::Condition as AC;
    match cond {
        // ── Явно булевые ──────────────────────────────────────────────────────
        // Булев литерал
        AC::Bool(_, _) => true,
        // Результат операции сравнения — всегда булево
        AC::Equal(_, _, _)
        | AC::NotEqual(_, _, _)
        | AC::Less(_, _, _)
        | AC::More(_, _, _)
        | AC::LessEqual(_, _, _)
        | AC::MoreEqual(_, _, _) => true,
        // Логическое НЕ (`!x`) всегда возвращает булев результат
        AC::Not(_, _) => true,
        // Скобки прозрачны — рекурсивно проверяем вложенное условие
        AC::Parenthesis(_, inner) => is_boolean_ast_condition(inner, model),
        // Вызов функции — тип возврата неизвестен, не предупреждаем
        AC::Function(_, _, _) => true,
        // Переменная: ищем в семантической модели и проверяем тип
        AC::Variable(id) => {
            let borrowed = model.borrow();
            // Переменная типа bool или bit — допустимо
            if let Some(var) = borrowed.search_var(&id.name) {
                return match &var {
                    VariableNode::Simple { ty, .. }
                    | VariableNode::Port { ty, .. }
                    | VariableNode::Const { ty, .. } => {
                        matches!(ty, TypeNode::Bool | TypeNode::Bit)
                    }
                    // Тип не разрешён — не предупреждаем
                    VariableNode::Unresolved => true,
                };
            }
            // Именованное условие (`cond Full = …`) — само является булевым
            if borrowed.search_cond(&id.name).is_some() {
                return true;
            }
            // Имя не найдено — ошибку выдаст другая проверка, не дублируем
            true
        }
        // ── Явно числовые ────────────────────────────────────────────────────
        // Целочисленный литерал
        AC::Number(_, _) => false,
        // Литерал длительности сам по себе не булев; `after 3s` — булево
        // (истекла ли выдержка), поэтому неявным приведением не является.
        AC::Duration(_, _, _) => false,
        // Именная форма (фича 0143) — та же выдержка: «истекла ли» булево.
        AC::After(_, _, _) | AC::AfterTicks(_, _, _) | AC::AfterExpr(_, _) => true,
        // Вещественный литерал
        AC::Rational(_, _, _) => false,
        // Строковый литерал (нетипичный в условии, но не булево)
        AC::String(_) => false,
        // Арифметические операции возвращают числовой тип
        AC::Add(_, _, _) | AC::Subtract(_, _, _) => false,
        // Побитовые операции возвращают числовой тип
        AC::And(_, _, _) | AC::Or(_, _, _) => false,
        // Индексация массива возвращает элемент числового типа
        AC::ArraySubscript(_, _, _) => false,
        // Доступ к битовому полю возвращает числовое значение
        AC::BitAccess(_, _, _) => false,
        // Обращение к ячейке по адресу (фича 0189) возвращает значение поля, а не
        // булево. ⚠️ Голая форма до сюда не доходит — её отвергает `SE-097`.
        AC::AnonAddress(_, _, _) => false,
    }
}

/// Возвращает краткое описание небулевого AST-условия для диагностического сообщения.
///
/// Вызывается только когда [`is_boolean_ast_condition`] вернул `false`,
/// поэтому покрывает только «числовые» ветви.
pub(super) fn ast_condition_summary(
    cond: &ast_types::Condition,
    model: &Rc<RefCell<ModelNode>>,
) -> String {
    use ast_types::Condition as AC;
    match cond {
        AC::Number(_, n) => format!("числовой литерал {}", n),
        AC::AnonAddress(_, addr, _) => format!("обращение к ячейке #0x{:X}", *addr as u64),
        AC::Duration(_, _, text) => format!("литерал длительности {}", text),
        AC::Rational(_, r, neg) => {
            format!("вещественный литерал {}{}", if *neg { "-" } else { "" }, r)
        }
        AC::String(_) => "строковый литерал".to_string(),
        AC::Variable(id) => {
            // Ищем тип переменной для информативного сообщения
            let ty_str = model
                .borrow()
                .search_var(&id.name)
                .map(|var| match var {
                    VariableNode::Simple { ty, .. }
                    | VariableNode::Port { ty, .. }
                    | VariableNode::Const { ty, .. } => format!("{:?}", ty),
                    VariableNode::Unresolved => "?".to_string(),
                })
                .unwrap_or_else(|| "?".to_string());
            format!("переменная '{}' типа {}", id.name, ty_str)
        }
        AC::Add(_, _, _) => "арифметическое сложение".to_string(),
        AC::Subtract(_, _, _) => "арифметическое вычитание".to_string(),
        AC::And(_, _, _) => "побитовое И".to_string(),
        AC::Or(_, _, _) => "побитовое ИЛИ".to_string(),
        AC::ArraySubscript(_, id, idx) => {
            let idx_str = match idx.as_ref() {
                AC::Number(_, n) => n.to_string(),
                AC::Variable(v) => v.name.clone(),
                _ => "expr".to_string(),
            };
            format!("элемент массива '{}[{}]'", id.name, idx_str)
        }
        AC::BitAccess(_, _, _) => "доступ к битовому полю".to_string(),
        // Остальные варианты сюда попасть не должны (они булевые)
        _ => "числовое выражение".to_string(),
    }
}

/// Добавляет предупреждение Се11 в `out`.
///
/// Выносит форматирование сообщения в отдельную функцию, чтобы не дублировать его.
/// `loc` — координаты перехода в исходном файле (берётся из [`ReferenceNode::location`]).
#[inline]
fn emit_implicit_bool_warning(
    loc: Location,
    prefix: &str,
    target_name: &str,
    summary: &str,
    is_next: bool,
    out: &mut Vec<Diagnostic>,
) {
    let verb = if is_next { "next к" } else { "к" };
    out.push(
        Diagnostic::warning(
            loc,
            format!(
                "{}: условие перехода {} '{}' содержит {} — \
                 рекомендуется явное сравнение (например, '!= 0')",
                prefix, verb, target_name, summary
            ),
        )
        .with_code("SE-037"),
    );
}

/// Проверяет, является ли разрешённое семантическое условие гарантированно булевым.
///
/// Применяется для условий на рёбрах `ref`, разрешённых на этапе 6 конвейера.
///
/// ## Правила классификации
///
/// | Условие                                          | Результат  |
/// |--------------------------------------------------|------------|
/// | Безусловный переход (`None`)                     | булево     |
/// | Булев литерал (`true`, `false`)                  | булево     |
/// | Операции сравнения (`=`, `!=`, `<`, `>`, …)     | булево     |
/// | Логическое НЕ (`!x`)                             | булево     |
/// | Скобки (`(…)`)                                   | рекурсия   |
/// | Вызов функции — тип возврата неизвестен          | булево     |
/// | Переменная типа `bool` или `bit`                 | булево     |
/// | Переменная числового типа (`[bit;N]`)            | числовое   |
/// | Числовой / вещественный / строковый литерал      | числовое   |
/// | Арифметика (`+`, `-`)                            | числовое   |
/// | Побитовые операции (`&`, `\|`)                   | числовое   |
/// | Элемент массива (`arr[n]`)                       | числовое   |
/// | Доступ к битовому полю (`.n`)                    | числовое   |
fn is_boolean_semantic_condition(cond: &ConditionNode) -> bool {
    match cond {
        ConditionNode::None => true,
        ConditionNode::Bool(_) => true,
        ConditionNode::Equal(_, _)
        | ConditionNode::NotEqual(_, _)
        | ConditionNode::Less(_, _)
        | ConditionNode::More(_, _)
        | ConditionNode::LessEqual(_, _)
        | ConditionNode::MoreEqual(_, _) => true,
        ConditionNode::Not(_) => true,
        ConditionNode::Parenthesis(inner) => is_boolean_semantic_condition(inner),
        // Тип возврата функции неизвестен — не предупреждаем
        ConditionNode::Function(_, _, _) => true,
        ConditionNode::Variable(v, _) => {
            let borrowed = v.borrow();
            match &*borrowed {
                VariableNode::Simple { ty, .. }
                | VariableNode::Port { ty, .. }
                | VariableNode::Const { ty, .. } => matches!(ty, TypeNode::Bool | TypeNode::Bit),
                VariableNode::Unresolved => true, // тип неизвестен — не предупреждаем
            }
        }
        _ => false,
    }
}

/// Возвращает краткое описание небулевого разрешённого семантического условия.
///
/// Вызывается только когда [`is_boolean_semantic_condition`] вернул `false`,
/// поэтому покрывает только «числовые» ветви.
fn semantic_condition_summary(cond: &ConditionNode) -> String {
    match cond {
        ConditionNode::Number(n) => format!("числовой литерал {}", n),
        ConditionNode::Rational(s, neg) => {
            format!("вещественный литерал {}{}", if *neg { "-" } else { "" }, s)
        }
        ConditionNode::String(_) => "строковый литерал".to_string(),
        ConditionNode::Variable(v, _) => {
            let borrowed = v.borrow();
            let (name_str, ty) = match &*borrowed {
                VariableNode::Simple { name, ty, .. }
                | VariableNode::Port { name, ty, .. }
                | VariableNode::Const { name, ty, .. } => (name.clone(), ty.clone()),
                VariableNode::Unresolved => return "переменная (неизвестный тип)".to_string(),
            };
            format!("переменная '{}' типа {}", name_str, ty)
        }
        ConditionNode::Add(_, _) => "арифметическое сложение".to_string(),
        ConditionNode::Subtract(_, _) => "арифметическое вычитание".to_string(),
        ConditionNode::And(_, _) => "побитовое И".to_string(),
        ConditionNode::Or(_, _) => "побитовое ИЛИ".to_string(),
        ConditionNode::ArraySubscript(var, idx) => {
            let name = match &*var.borrow() {
                VariableNode::Simple { name, .. }
                | VariableNode::Port { name, .. }
                | VariableNode::Const { name, .. } => name.clone(),
                VariableNode::Unresolved => "?".to_string(),
            };
            let idx_str = match idx.as_ref() {
                ConditionNode::Number(n) => n.to_string(),
                _ => "expr".to_string(),
            };
            format!("элемент массива '{}[{}]'", name, idx_str)
        }
        ConditionNode::BitAccess(_, _) => "доступ к битовому полю".to_string(),
        _ => "числовое выражение".to_string(),
    }
}

/// Проверяет условие одного перехода и при необходимости добавляет предупреждение Се11.
///
/// Основной путь — условие уже разрешено на этапе 6 конвейера
/// ([`crate::semantic::tree`]). Неразрешённый вариант [`ConditionNode::Unresolved`]
/// используется только как запасной (для паттернов вида `S(Model).StateName`,
/// которые не могут быть разрешены в текущем контексте).
///
/// Описание условия для сообщения вычисляется **лениво** — только при наличии
/// реального нарушения.
fn check_one_ref(
    loc: Location,
    prefix: &str,
    target_name: &str,
    cond: &ConditionNode,
    model: &Rc<RefCell<ModelNode>>,
    is_next: bool,
    out: &mut Vec<Diagnostic>,
) {
    match cond {
        // ── Основной путь: разрешённое семантическое условие ──────────────────
        cond if !matches!(cond, ConditionNode::Unresolved(_))
            && !is_boolean_semantic_condition(cond) =>
        {
            let summary = semantic_condition_summary(cond);
            emit_implicit_bool_warning(loc, prefix, target_name, &summary, is_next, out);
        }
        // ── Запасной путь: условие не разрешено (например, S(Model).StateName) ──
        ConditionNode::Unresolved(ast_cond) if !is_boolean_ast_condition(ast_cond, model) => {
            let summary = ast_condition_summary(ast_cond, model);
            emit_implicit_bool_warning(loc, prefix, target_name, &summary, is_next, out);
        }
        _ => {}
    }
}

/// Рекурсивно собирает предупреждения Се11 для всех состояний модели.
///
/// Обходит все состояния текущей модели и вложенных моделей.
/// Для каждого перехода вызывает [`check_one_ref`].
fn collect_implicit_bool_warnings(model: &Rc<RefCell<ModelNode>>, out: &mut Vec<Diagnostic>) {
    let borrowed = model.borrow();
    let model_name = borrowed.name.clone().unwrap_or_default();

    // Строит префикс диагностического сообщения вида
    // "состояние 'S'" или "модель 'M', состояние 'S'".
    let prefix_for = |state_name: &str| -> String {
        if model_name.is_empty() {
            format!("состояние '{}'", state_name)
        } else {
            format!("модель '{}', состояние '{}'", model_name, state_name)
        }
    };

    for state in borrowed.states.values() {
        match state {
            StateNode::Simple {
                name, references, ..
            } => {
                let prefix = prefix_for(name);
                for r in references {
                    check_one_ref(r.location, &prefix, &r.name, &r.cond, model, false, out);
                }
            }
            StateNode::Implement {
                name,
                references,
                next,
                ..
            } => {
                let prefix = prefix_for(name);
                for r in references {
                    check_one_ref(r.location, &prefix, &r.name, &r.cond, model, false, out);
                }
                if let Some(nr) = next {
                    check_one_ref(nr.location, &prefix, &nr.name, &nr.cond, model, true, out);
                }
            }
            StateNode::Unresolved => {}
        }
    }

    // Рекурсивный спуск во вложенные модели
    let nested: Vec<Rc<RefCell<ModelNode>>> = borrowed.models.values().map(Rc::clone).collect();
    drop(borrowed); // освобождаем заимствование перед рекурсией

    for nested_model in nested {
        collect_implicit_bool_warnings(&nested_model, out);
    }
}

/// Проверяет условия переходов в модели и возвращает предупреждения
/// о неявном приведении числового типа к булевому.
///
/// Предупреждение выдаётся, когда условие перехода (`ref`/`next`) содержит
/// выражение числового типа, используемое как булево без явного сравнения.
///
/// # Примеры (Takt)
///
/// ```but
/// var timer: [bit;8] = 0;
/// start Red {
///     ref Green: timer;       // Предупреждение: timer — числовой тип [bit;8]
///     ref Blue:  timer != 0;  // Нет предупреждения: явное сравнение
/// }
/// ```
///
/// # Возвращаемое значение
///
/// Вектор [`Diagnostic`] уровня `Warning` для каждого обнаруженного случая.
/// Пустой вектор означает, что числовых условий не найдено.
pub fn check_implicit_bool_conditions(model: &Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let mut warnings = Vec::new();
    collect_implicit_bool_warnings(model, &mut warnings);
    warnings
}
