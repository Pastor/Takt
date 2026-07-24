//! Зонд-тесты фичи 0021 (задача 0021-01): смена операторов (Option B).
//!
//! Проверяют новую операторную семантику на уровне разбора:
//! - `:=` — присваивание/инициализация значения (`Expression::Assign`, инициализатор);
//! - `=` — сравнение на равенство в выражениях (`Expression::Equal`);
//! - `<=` — по-прежнему «меньше-или-равно» (`Expression::LessEqual`), а не присваивание;
//! - `==` — **выведен** из языка (ошибка разбора);
//! - `=` в условиях (`cond`) — по-прежнему равенство (`Condition::Equal`).
//!
//! Легаси-фикстуры (старый синтаксис `=`/`==`) мигрируются отдельно (задача 0021-03).

use takt_lang::parse;
use takt_lang::parser::ast::{Condition, Expression, ModelElement, VariableDefine};

/// Разбирает `src` и возвращает корневую модель (паникует при ошибках разбора).
fn must_parse(src: &str) -> takt_lang::parser::ast::Model {
    parse(src, 0)
        .unwrap_or_else(|d| {
            let msgs: Vec<_> = d.iter().map(|x| x.message.clone()).collect();
            panic!("Разбор завершился с ошибками: {msgs:?}");
        })
        .0
}

/// Инициализатор первого `var` в корневой модели.
fn first_var_initializer(src: &str) -> Option<Expression> {
    let root = must_parse(src);
    root.elements.into_iter().find_map(|e| match e {
        ModelElement::Variable(v) => match *v {
            VariableDefine::Variable { initializer, .. } => Some(initializer),
            _ => None,
        },
        _ => None,
    })?
}

/// `:=` инициализирует значение переменной (`var x := 5`).
#[test]
fn colon_assign_is_value_initializer() {
    let init = first_var_initializer("var x: u8 := 5; model M { start S; }");
    assert!(
        init.is_some(),
        "Инициализатор через := должен присутствовать"
    );
}

/// `=` в позиции выражения — это равенство (`Expression::Equal`), а не присваивание.
#[test]
fn equals_is_equality_in_expression() {
    let init = first_var_initializer("var b: bit := 1 = 1; model M { start S; }")
        .expect("ожидался инициализатор");
    assert!(
        matches!(init, Expression::Equal(..)),
        "`=` в выражении должно давать Expression::Equal, получено: {init:?}"
    );
}

/// `<=` по-прежнему реляционный оператор (не превращён в присваивание).
#[test]
fn less_equal_stays_relational() {
    let init = first_var_initializer("var b: bit := 3 <= 5; model M { start S; }")
        .expect("ожидался инициализатор");
    assert!(
        matches!(init, Expression::LessEqual(..)),
        "`<=` должно оставаться Expression::LessEqual, получено: {init:?}"
    );
}

/// `:=` как выражение-присваивание даёт `Expression::Assign`.
#[test]
fn colon_assign_expression_is_assign_node() {
    // Инициализатор — присваивание-выражение в скобках: (x := 7).
    let src = "var x: u8 := 0; var y: u8 := (x := 7); model M { start S; }";
    let root = must_parse(src);
    let second = root
        .elements
        .into_iter()
        .filter_map(|e| match e {
            ModelElement::Variable(v) => match *v {
                VariableDefine::Variable { initializer, .. } => initializer,
                _ => None,
            },
            _ => None,
        })
        .nth(1)
        .expect("ожидался второй инициализатор");
    // Скобки оборачивают присваивание.
    let inner = match second {
        Expression::Parenthesis(_, e) => *e,
        other => other,
    };
    assert!(
        matches!(inner, Expression::Assign(..)),
        "`:=` должно давать Expression::Assign, получено: {inner:?}"
    );
}

/// `==` выведен из языка — ошибка разбора (подсказка «использовать `=`»).
#[test]
fn double_equals_is_parse_error() {
    let result = parse("var b: bit := 1 == 1; model M { start S; }", 0);
    assert!(
        result.is_err(),
        "`==` должно давать ошибку разбора (оператор выведен)"
    );
}

/// `=` в `cond` по-прежнему равенство (`Condition::Equal`) — инвариант условий.
#[test]
fn condition_equality_unchanged() {
    let root = must_parse("var x: bit := false; cond IsZero = x = 0; model M { start S; }");
    let cond = root
        .elements
        .into_iter()
        .find_map(|e| match e {
            ModelElement::Condition(c) => Some(c.value),
            _ => None,
        })
        .expect("ожидалось условие IsZero");
    assert!(
        matches!(cond, Condition::Equal(..)),
        "`=` в cond должно оставаться Condition::Equal, получено: {cond:?}"
    );
}
