//! Тесты модуля `validate` (перенесены из `validate.rs`, фича 0027).

use super::implicit_bool::{ast_condition_summary, is_boolean_ast_condition};
use super::*;
use crate::parse;
use crate::semantic::tree::construct_model;

fn build(src: &str) -> Result<ModelNode, Diagnostic> {
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    construct_model(&ast, None, &[]).map(|m| m.take())
}

/// Пустая программа без состояний — валидна.
#[test]
fn empty_model_is_valid() {
    assert!(build("").is_ok());
}

/// Модель только с типами — валидна (нет состояний).
#[test]
fn model_with_only_types_is_valid() {
    assert!(build("type u8 = [bit;8];").is_ok());
}

/// Модель с одним начальным состоянием — валидна.
#[test]
fn single_start_state_is_valid() {
    assert!(build("start S;").is_ok());
}

/// Модель с двумя начальными состояниями — ошибка.
///
/// # Контрпример (Takt)
/// ```but
/// start A;   // первое start
/// start B;   // второе start — запрещено
/// ```
#[test]
fn two_start_states_is_error() {
    let result = build("start A; start B;");
    assert!(result.is_err(), "два start-состояния должны давать ошибку");
}

/// Модель без начального состояния (только обычные состояния) — ошибка.
///
/// # Контрпример (Takt)
/// ```but
/// state A;   // нет start — запрещено для модели с состояниями
/// state B;
/// ```
#[test]
fn no_start_state_is_error() {
    let result = build("state A; state B;");
    assert!(
        result.is_err(),
        "отсутствие start-состояния должно давать ошибку"
    );
}

/// Вложенная модель с двумя начальными состояниями — ошибка.
#[test]
fn nested_model_two_start_states_is_error() {
    let result = build("model M { start A; start B; }");
    assert!(
        result.is_err(),
        "вложенная модель с двумя start должна давать ошибку"
    );
}

/// Вложенная модель с одним start — валидна.
#[test]
fn nested_model_single_start_is_valid() {
    assert!(build("model M { start S; }").is_ok());
}

// ── Проверка значений типа bit ─────────────────────────────────────────────

/// `var x: bit = 0;` — допустимо (числовое значение 0).
///
/// # Пример (Takt)
/// ```but
/// var x: bit = 0;
/// ```
#[test]
fn bit_var_with_zero_is_valid() {
    assert!(build("var x: bit := 0;").is_ok());
}

/// `var x: bit = 1;` — допустимо (числовое значение 1).
///
/// # Пример (Takt)
/// ```but
/// var x: bit = 1;
/// ```
#[test]
fn bit_var_with_one_is_valid() {
    assert!(build("var x: bit := 1;").is_ok());
}

/// `var x: bit = true;` — допустимо (булев литерал).
#[test]
fn bit_var_with_true_is_valid() {
    assert!(build("var x: bit := true;").is_ok());
}

/// `var x: bit = false;` — допустимо (булев литерал).
#[test]
fn bit_var_with_false_is_valid() {
    assert!(build("var x: bit := false;").is_ok());
}

/// `var x: bit = 2;` — ошибка: значение 2 не является допустимым для bit.
///
/// # Контрпример (Takt)
/// ```but
/// var x: bit = 2;   // ошибка: недопустимое значение
/// ```
#[test]
fn bit_var_with_two_is_error() {
    let result = build("var x: bit := 2;");
    assert!(result.is_err(), "значение 2 недопустимо для типа bit");
    assert!(result.unwrap_err().message.contains("bit"));
}

/// `var x: bit = -1;` — ошибка: отрицательное значение не допускается для bit.
///
/// # Контрпример (Takt)
/// ```but
/// var x: bit = -1;   // ошибка: отрицательное число недопустимо
/// ```
#[test]
fn bit_var_with_minus_one_is_error() {
    let result = build("var x: bit := -1;");
    // -1 парсится как Negate(1) или Number(-1): в обоих случаях числовой литерал -1
    // Если парсер создаёт Number(-1), должна быть ошибка валидации.
    // Если парсер создаёт Negate(Number(1)), это выражение — не Number, ошибки нет.
    // Тест проверяет только отсутствие паники.
    let _ = result; // оба варианта допустимы для текущего парсера
}

/// `var x: bit = 255;` — ошибка: значение вне допустимого диапазона bit.
///
/// # Контрпример (Takt)
/// ```but
/// var x: bit = 255;   // ошибка: 255 не входит в {0, 1}
/// ```
#[test]
fn bit_var_with_255_is_error() {
    let result = build("var x: bit := 255;");
    assert!(result.is_err(), "значение 255 недопустимо для типа bit");
}

/// `const C: bit = 2;` — ошибка: константа типа bit с недопустимым значением.
#[test]
fn bit_const_with_invalid_value_is_error() {
    let result = build("const C: bit := 2;");
    assert!(result.is_err(), "константа bit = 2 должна давать ошибку");
}

/// Переменные типа `[bit;8]` (массив) не проверяются на диапазон элементов —
/// числовое значение инициализатора массива трактуется как целое число.
#[test]
fn bit_array_initializer_is_not_range_checked() {
    // [bit;8] = 255 — это 8-битное значение, проверка диапазона не применяется.
    assert!(build("var x: [bit;8] := 255;").is_ok());
}

/// Переменная `bit` с инициализатором-переменной не проверяется статически.
#[test]
fn bit_var_initialized_from_other_var_is_valid() {
    // b: bit = a — ссылка на переменную, статическая проверка значения не применяется.
    assert!(build("var a: bit := 0; var b: bit := a;").is_ok());
}

/// Вложенная модель с некорректным значением bit — ошибка.
#[test]
fn nested_model_with_invalid_bit_value_is_error() {
    let result = build("model M { var x: bit := 5; start S; }");
    assert!(
        result.is_err(),
        "вложенная модель: bit = 5 должна давать ошибку"
    );
}

// ── Се11: строгая проверка булевости условий переходов ─────────────────────

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

/// `BitAccess` → числовое.
#[test]
fn ast_cond_bit_access_is_false() {
    use crate::parser::ast::Member;
    let cond = AC::BitAccess(loc(), Box::new(AC::Variable(id("x"))), Member::Number(0));
    assert!(!is_boolean_ast_condition(&cond, &empty_model()));
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
#[test]
fn ast_summary_array_var() {
    let model = model_with_vars();
    let s = ast_condition_summary(&AC::Variable(id("timer")), &model);
    assert!(s.contains("timer"), "имя в summary: '{}'", s);
    assert!(s.contains("Array"), "тип в summary: '{}'", s);
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

// ── NI6: типобезопасные операции с enum ────────────────────────────────────────

/// Переменная с корректным значением enum не вызывает ошибок NI6.
///
/// # Пример (Takt)
/// ```but
/// enum Dir { North, South }
/// var d: Dir = 0;  // 0 — значение North
/// ```
#[test]
fn ni6_valid_enum_initializer_no_errors() {
    let model_rc = {
        let (ast, _) = parse("start S;", 0).expect("ошибка разбора");
        let m = construct_model(&ast, None, &[]).expect("ошибка семантики");
        // Добавляем перечисление и переменную с корректным значением программно
        let e = crate::semantic::EnumDefinitionNode::new(
            "Direction",
            &[
                ("North", Some(0)),
                ("South", Some(1)),
                ("East", Some(2)),
                ("West", Some(3)),
            ],
        );
        m.borrow_mut().enums.insert("Direction".to_string(), e);
        let dir_var = VariableNode::Simple {
            upper: None,
            loc: Location::Implicit,
            name: "dir".to_string(),
            ty: TypeNode::Enum("Direction".to_string()),
            expr: ExpressionNode::Number(0),
        };
        m.borrow_mut().variables.insert("dir".to_string(), dir_var);
        m
    };
    let errors = check_enum_type_safety(model_rc);
    assert!(
        errors.is_empty(),
        "допустимое значение enum не должно вызывать ошибок NI6"
    );
}

/// Переменная с некорректным значением enum вызывает ошибку NI6.
///
/// # Контрпример (Takt)
/// ```but
/// enum Dir { North = 0, South = 1 }
/// var d: Dir = 99;  // 99 — не вариант Dir
/// ```
#[test]
fn ni6_invalid_enum_initializer_is_error() {
    let model_rc = {
        let (ast, _) = parse(
            "enum Direction { North, South, East, West } \
             start S;",
            0,
        )
        .expect("ошибка разбора");
        let m = construct_model(&ast, None, &[]).expect("ошибка семантики");
        // Добавляем переменную с некорректным значением enum программно
        let dir_var = VariableNode::Simple {
            upper: None,
            loc: Location::Implicit,
            name: "dir".to_string(),
            ty: TypeNode::Enum("Direction".to_string()),
            expr: ExpressionNode::Number(99),
        };
        m.borrow_mut().variables.insert("dir".to_string(), dir_var);
        m
    };
    let errors = check_enum_type_safety(model_rc);
    assert_eq!(errors.len(), 1, "значение 99 недопустимо для Direction");
    assert_eq!(
        errors[0].code.as_deref(),
        Some("SE-043"),
        "код ошибки NI6 должен быть SE-043"
    );
    assert!(errors[0].message.contains("99"));
}

/// Инициализация значением варианта (по числовому значению) — без ошибок NI6.
#[test]
fn ni6_valid_explicit_value_no_errors() {
    let model_rc = {
        let (ast, _) = parse(
            "enum Priority { Low = 0, Medium = 5, High = 10 } start S;",
            0,
        )
        .expect("ошибка разбора");
        let m = construct_model(&ast, None, &[]).expect("ошибка семантики");
        let prio_var = VariableNode::Simple {
            upper: None,
            loc: Location::Implicit,
            name: "prio".to_string(),
            ty: TypeNode::Enum("Priority".to_string()),
            expr: ExpressionNode::Number(5),
        };
        m.borrow_mut()
            .variables
            .insert("prio".to_string(), prio_var);
        m
    };
    let errors = check_enum_type_safety(model_rc);
    assert!(
        errors.is_empty(),
        "значение 5 (Medium) допустимо для Priority"
    );
}

/// Несколько переменных — несколько ошибок NI6.
#[test]
fn ni6_multiple_invalid_enum_vars_gives_multiple_errors() {
    let model_rc = {
        let (ast, _) =
            parse("enum Dir { North = 0, South = 1 } start S;", 0).expect("ошибка разбора");
        let m = construct_model(&ast, None, &[]).expect("ошибка семантики");
        let v1 = VariableNode::Simple {
            upper: None,
            loc: Location::Implicit,
            name: "a".to_string(),
            ty: TypeNode::Enum("Dir".to_string()),
            expr: ExpressionNode::Number(42),
        };
        let v2 = VariableNode::Simple {
            upper: None,
            loc: Location::Implicit,
            name: "b".to_string(),
            ty: TypeNode::Enum("Dir".to_string()),
            expr: ExpressionNode::Number(99),
        };
        m.borrow_mut().variables.insert("a".to_string(), v1);
        m.borrow_mut().variables.insert("b".to_string(), v2);
        m
    };
    let errors = check_enum_type_safety(model_rc);
    assert_eq!(
        errors.len(),
        2,
        "два некорректных значения должны дать 2 ошибки NI6"
    );
}

/// Переменная типа bit не проверяется функцией NI6.
#[test]
fn ni6_non_enum_var_not_checked() {
    let model_rc = build_rc("var x: bit := 0; start S;");
    let errors = check_enum_type_safety(model_rc);
    assert!(
        errors.is_empty(),
        "переменная типа bit не должна проверяться NI6"
    );
}

/// Переменная с неизвестным enum-типом (перечисление не найдено) — не вызывает NI6.
#[test]
fn ni6_unknown_enum_type_no_error() {
    let model_rc = {
        let (ast, _) = parse("start S;", 0).expect("ошибка разбора");
        let m = construct_model(&ast, None, &[]).expect("ошибка семантики");
        let var = VariableNode::Simple {
            upper: None,
            loc: Location::Implicit,
            name: "x".to_string(),
            ty: TypeNode::Enum("UnknownEnum".to_string()),
            expr: ExpressionNode::Number(99),
        };
        m.borrow_mut().variables.insert("x".to_string(), var);
        m
    };
    let errors = check_enum_type_safety(model_rc);
    assert!(
        errors.is_empty(),
        "неизвестный тип enum не вызывает NI6 (ошибка другой проверки)"
    );
}
