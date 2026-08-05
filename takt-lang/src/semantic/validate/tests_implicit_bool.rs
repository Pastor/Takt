//! Юнит-тесты проверки неявной булевости условий (`SE-037`, Ce11).
//!
//! Вынесены из `tests.rs` фичей 0232: файл упёрся в предел размера модуля, а
//! эти тесты — самостоятельная тема (предикаты булевости, описание условия и
//! обход переходов), у которой уже есть соседи-прецеденты
//! (`tests_ce4_declarations.rs`, `tests_ce15_array_size.rs`).

use super::implicit_bool::{ast_condition_summary, is_boolean_ast_condition};
use super::*;
use crate::parse;
use crate::semantic::tree::construct_model;

fn build_rc(src: &str) -> Rc<RefCell<ModelNode>> {
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    construct_model(&ast, None, &[]).expect("ошибка семантики")
}

// ── Юнит-тесты is_boolean_ast_condition и ast_condition_summary ────────────
//
// Вспомогательные функции для построения моделей и AST-условий.

/// Строит пустую семантическую модель (без переменных и состояний).
fn empty_model() -> Rc<RefCell<ModelNode>> {
    build_rc("")
}

/// Строит модель с переменными: `flag: bool`, `bit1: bit`, `timer: [bit;8]`.
fn model_with_vars() -> Rc<RefCell<ModelNode>> {
    build_rc(
        "var flag: bool := false; \
         var bit1: bit := 0; \
         var timer: [bit;8] := 0;",
    )
}

/// Строит модель с именованным условием `cond Full = timer = 255;`.
fn model_with_named_cond() -> Rc<RefCell<ModelNode>> {
    build_rc("var timer: [bit;8] := 0; cond Full = timer = 255;")
}

use crate::diagnostics::Location as Loc;
use crate::parser::ast::Condition as AC;
use crate::parser::ast::Identifier;

fn loc() -> Loc {
    Loc::Builtin
}

fn id(name: &str) -> Identifier {
    Identifier::new(name)
}

// ── Явно булевые условия ────────────────────────────────────────────────

/// `Bool(true)` → булево.
#[test]
fn ast_cond_bool_literal_is_true() {
    assert!(is_boolean_ast_condition(
        &AC::Bool(loc(), true),
        &empty_model()
    ));
}

/// `Bool(false)` → булево.
#[test]
fn ast_cond_bool_false_literal_is_true() {
    assert!(is_boolean_ast_condition(
        &AC::Bool(loc(), false),
        &empty_model()
    ));
}

/// `Equal` → булево.
#[test]
fn ast_cond_equal_is_true() {
    let cond = AC::Equal(
        loc(),
        Box::new(AC::Number(loc(), 0)),
        Box::new(AC::Number(loc(), 0)),
    );
    assert!(is_boolean_ast_condition(&cond, &empty_model()));
}

/// `NotEqual` → булево.
#[test]
fn ast_cond_not_equal_is_true() {
    let cond = AC::NotEqual(
        loc(),
        Box::new(AC::Number(loc(), 0)),
        Box::new(AC::Number(loc(), 1)),
    );
    assert!(is_boolean_ast_condition(&cond, &empty_model()));
}

/// `Less` → булево.
#[test]
fn ast_cond_less_is_true() {
    let cond = AC::Less(
        loc(),
        Box::new(AC::Number(loc(), 0)),
        Box::new(AC::Number(loc(), 1)),
    );
    assert!(is_boolean_ast_condition(&cond, &empty_model()));
}

/// `More` → булево.
#[test]
fn ast_cond_more_is_true() {
    let cond = AC::More(
        loc(),
        Box::new(AC::Number(loc(), 5)),
        Box::new(AC::Number(loc(), 1)),
    );
    assert!(is_boolean_ast_condition(&cond, &empty_model()));
}

/// `LessEqual` → булево.
#[test]
fn ast_cond_less_equal_is_true() {
    let cond = AC::LessEqual(
        loc(),
        Box::new(AC::Number(loc(), 0)),
        Box::new(AC::Number(loc(), 0)),
    );
    assert!(is_boolean_ast_condition(&cond, &empty_model()));
}

/// `MoreEqual` → булево.
#[test]
fn ast_cond_more_equal_is_true() {
    let cond = AC::MoreEqual(
        loc(),
        Box::new(AC::Number(loc(), 5)),
        Box::new(AC::Number(loc(), 5)),
    );
    assert!(is_boolean_ast_condition(&cond, &empty_model()));
}

/// `Not` — логическое НЕ → всегда булево.
#[test]
fn ast_cond_not_is_true() {
    let cond = AC::Not(loc(), Box::new(AC::Number(loc(), 0)));
    assert!(is_boolean_ast_condition(&cond, &empty_model()));
}

/// `Not` вокруг переменной → булево.
#[test]
fn ast_cond_not_of_var_is_true() {
    let model = model_with_vars();
    let cond = AC::Not(loc(), Box::new(AC::Variable(id("timer"))));
    assert!(is_boolean_ast_condition(&cond, &model));
}

/// `Function(…)` — тип возврата неизвестен → булево.
#[test]
fn ast_cond_function_is_true() {
    let cond = AC::Function(loc(), id("f"), vec![]);
    assert!(is_boolean_ast_condition(&cond, &empty_model()));
}

/// `Parenthesis(Equal)` → булево.
#[test]
fn ast_cond_paren_cmp_is_true() {
    let inner = AC::Equal(
        loc(),
        Box::new(AC::Number(loc(), 0)),
        Box::new(AC::Number(loc(), 1)),
    );
    let cond = AC::Parenthesis(loc(), Box::new(inner));
    assert!(is_boolean_ast_condition(&cond, &empty_model()));
}

/// `Variable("flag")` где `flag: bool` → булево.
#[test]
fn ast_cond_bool_var_is_true() {
    let model = model_with_vars();
    assert!(is_boolean_ast_condition(&AC::Variable(id("flag")), &model));
}

/// `Variable("bit1")` где `bit1: bit` → булево.
#[test]
fn ast_cond_bit_var_is_true() {
    let model = model_with_vars();
    assert!(is_boolean_ast_condition(&AC::Variable(id("bit1")), &model));
}

/// `Variable("Full")` где `Full` — именованное условие → булево.
#[test]
fn ast_cond_named_cond_var_is_true() {
    let model = model_with_named_cond();
    assert!(is_boolean_ast_condition(&AC::Variable(id("Full")), &model));
}

/// `Variable("unknown")` — неизвестное имя → не предупреждаем (булево).
#[test]
fn ast_cond_unknown_var_is_true() {
    assert!(is_boolean_ast_condition(
        &AC::Variable(id("unknown")),
        &empty_model()
    ));
}

// ── Явно числовые условия ───────────────────────────────────────────────

/// `Number(5)` → числовое.
#[test]
fn ast_cond_number_is_false() {
    assert!(!is_boolean_ast_condition(
        &AC::Number(loc(), 5),
        &empty_model()
    ));
}

/// `Number(0)` → числовое (даже 0).
#[test]
fn ast_cond_zero_number_is_false() {
    assert!(!is_boolean_ast_condition(
        &AC::Number(loc(), 0),
        &empty_model()
    ));
}

/// `Rational` → числовое.
#[test]
fn ast_cond_rational_is_false() {
    let cond = AC::Rational(loc(), "3.14".to_string(), false);
    assert!(!is_boolean_ast_condition(&cond, &empty_model()));
}

/// `String` → числовое.
#[test]
fn ast_cond_string_is_false() {
    let cond = AC::String(vec![]);
    assert!(!is_boolean_ast_condition(&cond, &empty_model()));
}

/// `Add` → числовое.
#[test]
fn ast_cond_add_is_false() {
    let cond = AC::Add(
        loc(),
        Box::new(AC::Number(loc(), 1)),
        Box::new(AC::Number(loc(), 2)),
    );
    assert!(!is_boolean_ast_condition(&cond, &empty_model()));
}

/// `Subtract` → числовое.
#[test]
fn ast_cond_subtract_is_false() {
    let cond = AC::Subtract(
        loc(),
        Box::new(AC::Number(loc(), 5)),
        Box::new(AC::Number(loc(), 1)),
    );
    assert!(!is_boolean_ast_condition(&cond, &empty_model()));
}

/// `And` (побитовое И) → числовое.
#[test]
fn ast_cond_and_is_false() {
    let cond = AC::And(
        loc(),
        Box::new(AC::Number(loc(), 3)),
        Box::new(AC::Number(loc(), 1)),
    );
    assert!(!is_boolean_ast_condition(&cond, &empty_model()));
}

/// `Or` (побитовое ИЛИ) → числовое.
#[test]
fn ast_cond_or_is_false() {
    let cond = AC::Or(
        loc(),
        Box::new(AC::Number(loc(), 3)),
        Box::new(AC::Number(loc(), 1)),
    );
    assert!(!is_boolean_ast_condition(&cond, &empty_model()));
}

/// `ArraySubscript` → числовое.
#[test]
fn ast_cond_array_subscript_is_false() {
    let cond = AC::ArraySubscript(loc(), id("arr"), Box::new(AC::Number(loc(), 0)));
    assert!(!is_boolean_ast_condition(&cond, &empty_model()));
}

/// `BitAccess` (`x.0`) → **булево**.
///
/// ⚠️ Ожидание изменено фичей 0232 (решение заказчика): доступ к **одному** биту
/// даёт 0 или 1, и требовать от него `!= 0` значит спорить с идиомой языка —
/// именно так читают дискретный вход. Прежде ветвь считалась числовой, и
/// проверка горела на законных записях корпуса.
#[test]
fn ast_cond_bit_access_is_boolean() {
    use crate::parser::ast::Member;
    let cond = AC::BitAccess(loc(), Box::new(AC::Variable(id("x"))), Member::Number(0));
    assert!(is_boolean_ast_condition(&cond, &empty_model()));
}

/// `&`/`|` булевы, когда булевы **оба** операнда, и числовые иначе (фича 0232).
///
/// Логических `&&`/`||` условная грамматика не принимает — `&`/`|` суть
/// единственная форма конъюнкции условий; предупреждать о ней значило бы
/// предупреждать о единственной доступной записи. Числовой операнд при этом
/// по-прежнему ловится.
#[test]
fn ast_cond_bitwise_over_booleans_is_boolean() {
    let both_bool = AC::And(
        loc(),
        Box::new(AC::Bool(loc(), true)),
        Box::new(AC::Not(loc(), Box::new(AC::Bool(loc(), false)))),
    );
    assert!(is_boolean_ast_condition(&both_bool, &empty_model()));

    let numeric_operand = AC::Or(
        loc(),
        Box::new(AC::Bool(loc(), true)),
        Box::new(AC::Number(loc(), 3)),
    );
    assert!(
        !is_boolean_ast_condition(&numeric_operand, &empty_model()),
        "числовой операнд обязан остаться нарушением"
    );
}

/// `Variable("timer")` где `timer: [bit;8]` → числовое.
#[test]
fn ast_cond_array_var_is_false() {
    let model = model_with_vars();
    assert!(!is_boolean_ast_condition(
        &AC::Variable(id("timer")),
        &model
    ));
}

/// `Parenthesis(Number)` → числовое.
#[test]
fn ast_cond_paren_number_is_false() {
    let cond = AC::Parenthesis(loc(), Box::new(AC::Number(loc(), 42)));
    assert!(!is_boolean_ast_condition(&cond, &empty_model()));
}

// ── Юнит-тесты ast_condition_summary ────────────────────────────────────

/// Summary для числового литерала содержит значение.
#[test]
fn ast_summary_number() {
    let s = ast_condition_summary(&AC::Number(loc(), 42), &empty_model());
    assert!(s.contains("42"), "summary для 42: '{}'", s);
}

/// Summary для вещественного числа содержит значение.
#[test]
fn ast_summary_rational() {
    let cond = AC::Rational(loc(), "1.5".to_string(), false);
    let s = ast_condition_summary(&cond, &empty_model());
    assert!(s.contains("1.5"), "summary для 1.5: '{}'", s);
}

/// Summary для отрицательного вещественного числа содержит минус.
#[test]
fn ast_summary_rational_negative() {
    let cond = AC::Rational(loc(), "2.0".to_string(), true);
    let s = ast_condition_summary(&cond, &empty_model());
    assert!(s.contains("-2.0"), "summary для -2.0: '{}'", s);
}

/// Summary для строки содержит слово "строковый".
#[test]
fn ast_summary_string() {
    let s = ast_condition_summary(&AC::String(vec![]), &empty_model());
    assert!(s.contains("строковый"), "summary для String: '{}'", s);
}

/// Summary для переменной числового типа содержит имя и тип.
///
/// ⚠️ Ожидание **изменено фичей 0231**: прежде тест требовал подстроку `Array` —
/// то есть пришпиливал `Debug`-форму `TypeNode`. Тип печатается так, как его
/// написал автор (`[bit;8]`), потому что сообщение читает он, а не компилятор.
#[test]
fn ast_summary_array_var() {
    let model = model_with_vars();
    let s = ast_condition_summary(&AC::Variable(id("timer")), &model);
    assert!(s.contains("timer"), "имя в summary: '{}'", s);
    assert!(s.contains("[bit;8]"), "тип в summary: '{}'", s);
}

/// Summary для неизвестной переменной содержит имя и `?`.
#[test]
fn ast_summary_unknown_var() {
    let s = ast_condition_summary(&AC::Variable(id("ghost")), &empty_model());
    assert!(s.contains("ghost"), "имя в summary: '{}'", s);
}

/// Summary для сложения.
#[test]
fn ast_summary_add() {
    let cond = AC::Add(
        loc(),
        Box::new(AC::Number(loc(), 1)),
        Box::new(AC::Number(loc(), 2)),
    );
    let s = ast_condition_summary(&cond, &empty_model());
    assert!(s.contains("сложение"), "summary для Add: '{}'", s);
}

/// Summary для вычитания.
#[test]
fn ast_summary_subtract() {
    let cond = AC::Subtract(
        loc(),
        Box::new(AC::Number(loc(), 5)),
        Box::new(AC::Number(loc(), 1)),
    );
    let s = ast_condition_summary(&cond, &empty_model());
    assert!(s.contains("вычитание"), "summary для Subtract: '{}'", s);
}

/// Summary для побитового И.
#[test]
fn ast_summary_and() {
    let cond = AC::And(
        loc(),
        Box::new(AC::Number(loc(), 1)),
        Box::new(AC::Number(loc(), 1)),
    );
    let s = ast_condition_summary(&cond, &empty_model());
    assert!(s.contains('И'), "summary для And: '{}'", s);
}

/// Summary для побитового ИЛИ.
#[test]
fn ast_summary_or() {
    let cond = AC::Or(
        loc(),
        Box::new(AC::Number(loc(), 1)),
        Box::new(AC::Number(loc(), 1)),
    );
    let s = ast_condition_summary(&cond, &empty_model());
    assert!(s.contains('И'), "summary для Or: '{}'", s);
}

/// Summary для элемента массива содержит имя и индекс.
#[test]
fn ast_summary_array_subscript() {
    let cond = AC::ArraySubscript(loc(), id("buf"), Box::new(AC::Number(loc(), 3)));
    let s = ast_condition_summary(&cond, &empty_model());
    assert!(s.contains("buf"), "имя массива в summary: '{}'", s);
    assert!(s.contains('3'), "индекс в summary: '{}'", s);
}

/// Summary для доступа к биту.
#[test]
fn ast_summary_bit_access() {
    use crate::parser::ast::Member;
    let cond = AC::BitAccess(loc(), Box::new(AC::Variable(id("x"))), Member::Number(0));
    let s = ast_condition_summary(&cond, &empty_model());
    assert!(s.contains("бит"), "summary для BitAccess: '{}'", s);
}

// ── Юнит-тесты check_implicit_bool_conditions ────────────────────────────

/// Безусловный переход (`ref Next;`) — нет предупреждений.
#[test]
fn unconditional_ref_no_warning() {
    let model = build_rc("start S { ref T; } state T;");
    let warnings = check_implicit_bool_conditions(&model);
    assert!(
        warnings.is_empty(),
        "безусловный переход не должен давать предупреждений"
    );
}

/// Булев литерал в условии (`ref Next: true;`) — нет предупреждений.
#[test]
fn bool_literal_cond_no_warning() {
    let model = build_rc("start S { ref T: true; } state T;");
    let warnings = check_implicit_bool_conditions(&model);
    assert!(
        warnings.is_empty(),
        "булев литерал не должен давать предупреждений"
    );
}

/// Переменная типа `bool` в условии — нет предупреждений.
#[test]
fn bool_var_cond_no_warning() {
    let model = build_rc("var flag: bool := false; start S { ref T: flag; } state T;");
    let warnings = check_implicit_bool_conditions(&model);
    assert!(
        warnings.is_empty(),
        "переменная bool не должна давать предупреждений"
    );
}

/// Переменная типа `bit` (один бит) в условии — нет предупреждений.
#[test]
fn bit_var_cond_no_warning() {
    let model = build_rc("var flag: bit := 0; start S { ref T: flag; } state T;");
    let warnings = check_implicit_bool_conditions(&model);
    assert!(
        warnings.is_empty(),
        "переменная bit не должна давать предупреждений"
    );
}

/// Явное сравнение `!= 0` — нет предупреждений.
#[test]
fn explicit_ne_comparison_no_warning() {
    let model = build_rc("var timer: [bit;8] := 0; start S { ref T: timer != 0; } state T;");
    let warnings = check_implicit_bool_conditions(&model);
    assert!(
        warnings.is_empty(),
        "явное != не должно давать предупреждений"
    );
}

/// Явное сравнение `= 100` — нет предупреждений.
#[test]
fn explicit_eq_comparison_no_warning() {
    let model = build_rc("var timer: [bit;8] := 0; start S { ref T: timer = 100; } state T;");
    let warnings = check_implicit_bool_conditions(&model);
    assert!(
        warnings.is_empty(),
        "явное = не должно давать предупреждений"
    );
}

/// Именованное условие в ref — нет предупреждений.
#[test]
fn named_cond_in_ref_no_warning() {
    let model = build_rc(
        "var timer: [bit;8] := 0; \
         cond Full = timer = 255; \
         start S { ref T: Full; } state T;",
    );
    let warnings = check_implicit_bool_conditions(&model);
    assert!(
        warnings.is_empty(),
        "именованное условие не должно давать предупреждений"
    );
}

/// Переменная числового типа `[bit;8]` без сравнения — предупреждение.
#[test]
fn array_var_cond_gives_warning() {
    let model = build_rc("var timer: [bit;8] := 0; start S { ref T: timer; } state T;");
    let warnings = check_implicit_bool_conditions(&model);
    assert_eq!(
        warnings.len(),
        1,
        "переменная [bit;8] должна давать предупреждение"
    );
    assert!(
        warnings[0].message.contains("timer"),
        "сообщение должно упоминать 'timer'"
    );
}

/// Числовой литерал в условии — предупреждение.
#[test]
fn number_literal_cond_gives_warning() {
    let model = build_rc("start S { ref T: 5; } state T;");
    let warnings = check_implicit_bool_conditions(&model);
    assert_eq!(
        warnings.len(),
        1,
        "числовой литерал должен давать предупреждение"
    );
    assert!(
        warnings[0].message.contains('5'),
        "сообщение должно упоминать значение 5"
    );
}

/// Предупреждение содержит имя целевого состояния.
#[test]
fn warning_message_contains_target_state() {
    let model = build_rc("var x: [bit;8] := 0; start S { ref MyTarget: x; } state MyTarget;");
    let warnings = check_implicit_bool_conditions(&model);
    assert_eq!(warnings.len(), 1);
    assert!(
        warnings[0].message.contains("MyTarget"),
        "сообщение должно упоминать состояние-цель: {}",
        warnings[0].message
    );
}

/// Несколько переходов: один числовой, один булев — одно предупреждение.
#[test]
fn mixed_refs_one_warning() {
    let model = build_rc(
        "var timer: [bit;8] := 0; var flag: bool := false; \
         start S { ref T: timer; ref U: flag; } state T; state U;",
    );
    let warnings = check_implicit_bool_conditions(&model);
    assert_eq!(warnings.len(), 1, "должно быть ровно одно предупреждение");
}

/// Два числовых условия — два предупреждения.
#[test]
fn two_numeric_refs_two_warnings() {
    let model = build_rc(
        "var a: [bit;8] := 0; var b: [bit;8] := 0; \
         start S { ref T: a; ref U: b; } state T; state U;",
    );
    let warnings = check_implicit_bool_conditions(&model);
    assert_eq!(
        warnings.len(),
        2,
        "два числовых условия — два предупреждения"
    );
}

/// Вложенная модель с числовым условием — предупреждение упоминает имя модели.
#[test]
fn nested_model_implicit_bool_gives_warning() {
    let model = build_rc("model M { var timer: [bit;8] := 0; start S { ref T: timer; } state T; }");
    let warnings = check_implicit_bool_conditions(&model);
    assert_eq!(
        warnings.len(),
        1,
        "вложенная модель должна давать предупреждение"
    );
    assert!(
        warnings[0].message.contains('M'),
        "сообщение должно упоминать 'M'"
    );
}

/// Модель без состояний — нет предупреждений.
#[test]
fn model_without_states_no_warnings() {
    let model = build_rc("var timer: [bit;8] := 0;");
    let warnings = check_implicit_bool_conditions(&model);
    assert!(
        warnings.is_empty(),
        "модель без состояний не должна давать предупреждений"
    );
}

/// Предупреждение Се11 имеет уровень Warning.
#[test]
fn warning_has_correct_level() {
    use crate::diagnostics::Level;
    let model = build_rc("start S { ref T: 5; } state T;");
    let warnings = check_implicit_bool_conditions(&model);
    assert_eq!(warnings.len(), 1);
    assert_eq!(
        warnings[0].level,
        Level::Warning,
        "уровень должен быть Warning"
    );
}
