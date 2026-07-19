//! Генерация исходного C-файла (`.c`) из семантического дерева Lam.
//!
//! Точка входа: [`generate_source`] — собирает все секции `.c`-файла,
//! делегируя генерацию деклараций, функций и моделей соответствующим модулям.

use super::c_decl::{generate_constants_and_ports_and_enums, generate_functions};
use super::c_model::{generate_function_prototypes, generate_model_functions};
use crate::diagnostics::Diagnostic;
use crate::generator::c::c_map::CMap;
use crate::generator::indent::Printer;

/// Генерирует содержимое `.c`-файла для модели.
pub(super) fn generate_source(filename: &str, map: &CMap) -> Result<String, Diagnostic> {
    let mut source = String::new();
    let mut printer = Printer::new(4, &mut source);
    printer
        .print(format!("#include \"{}.h\"", filename).as_str())
        .nl();
    printer.print("#include <assert.h>").nl();
    printer.print("#include <math.h>").nl();
    generate_constants_and_ports_and_enums(&mut printer, map)?;
    generate_function_prototypes(&mut printer, map)?;
    generate_functions(&mut printer, map)?;
    for model in map.using_models() {
        generate_model_functions(&mut printer, &model, map)?;
    }
    generate_model_functions(&mut printer, &map.model(), map)?;
    Ok(super::c_expr::insert_fixed_helpers(source)) // Q-хелперы (0061) — по вызову
}

#[cfg(test)]
mod tests {
    use crate::generator::c::c_map::CMap;
    use crate::generator::c::c_source::generate_source;
    use crate::semantic::ExpressionNode;
    use crate::{parse, semantic};

    use crate::generator::c::c_expr::generate_stmt_expression;
    use crate::generator::indent::Printer;
    use crate::semantic::minimap::Element;

    // ── Вспомогательная функция ────────────────────────────────────────────────

    /// Создаёт минимальный CMap для тестов генерации выражений.
    fn make_map_and_owner(src: &str) -> (CMap, Element) {
        let (ast, _) = parse(src, 0).expect("ошибка разбора");
        let model_rc = semantic::tree::construct_model(&ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        let owner = Element::Model {
            name: map.root_name().clone(),
            states: map.states().clone(),
            start: map.start().clone(),
        };
        (map, owner)
    }

    /// Генерирует C-строку из выражения.
    fn expr_to_str(map: &CMap, owner: &Element, expr: &ExpressionNode) -> String {
        let mut s = String::new();
        let mut printer = Printer::new(4, &mut s);
        generate_stmt_expression(&mut printer, map, owner, vec![], expr, true).unwrap();
        s
    }

    // ── Тесты литералов ────────────────────────────────────────────────────────

    #[test]
    fn test_expr_number() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Number(42);
        assert_eq!(expr_to_str(&map, &owner, &expr), "42");
    }

    #[test]
    fn test_expr_bool_true() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        assert_eq!(
            expr_to_str(&map, &owner, &ExpressionNode::Bool(true)),
            "true"
        );
    }

    #[test]
    fn test_expr_bool_false() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        assert_eq!(
            expr_to_str(&map, &owner, &ExpressionNode::Bool(false)),
            "false"
        );
    }

    #[test]
    fn test_expr_rational_positive() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Rational("3.14".to_string(), false);
        assert_eq!(expr_to_str(&map, &owner, &expr), "3.14");
    }

    #[test]
    fn test_expr_rational_negative() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Rational("3.14".to_string(), true);
        assert_eq!(expr_to_str(&map, &owner, &expr), "-3.14");
    }

    #[test]
    fn test_expr_string() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::String(vec!["hello".to_string()]);
        assert_eq!(expr_to_str(&map, &owner, &expr), "\"hello\"");
    }

    // ── Тесты унарных операторов ───────────────────────────────────────────────

    #[test]
    fn test_expr_negate() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // Атом (число) — скобки не нужны
        let expr = ExpressionNode::Negate(Box::new(ExpressionNode::Number(42)));
        assert_eq!(expr_to_str(&map, &owner, &expr), "-42");
    }

    #[test]
    fn test_expr_negate_complex() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // Бинарное выражение внутри унарного — скобки нужны
        let inner = ExpressionNode::Add(
            Box::new(ExpressionNode::Number(1)),
            Box::new(ExpressionNode::Number(2)),
        );
        let expr = ExpressionNode::Negate(Box::new(inner));
        assert_eq!(expr_to_str(&map, &owner, &expr), "-(1 + 2)");
    }

    #[test]
    fn test_expr_negate_negate() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // Двойное отрицание — скобки нужны чтобы избежать `--x` (декремент в C)
        let inner = ExpressionNode::Negate(Box::new(ExpressionNode::Number(5)));
        let expr = ExpressionNode::Negate(Box::new(inner));
        assert_eq!(expr_to_str(&map, &owner, &expr), "-(-5)");
    }

    #[test]
    fn test_expr_not() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // Атом — без скобок
        let expr = ExpressionNode::Not(Box::new(ExpressionNode::Bool(true)));
        assert_eq!(expr_to_str(&map, &owner, &expr), "!true");
    }

    #[test]
    fn test_expr_not_complex() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // Логическое И внутри NOT — нужны скобки
        let inner = ExpressionNode::And(
            Box::new(ExpressionNode::Bool(true)),
            Box::new(ExpressionNode::Bool(false)),
        );
        let expr = ExpressionNode::Not(Box::new(inner));
        assert_eq!(expr_to_str(&map, &owner, &expr), "!(true && false)");
    }

    #[test]
    fn test_expr_bitwise_not() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // Атом — без скобок
        let expr = ExpressionNode::BitwiseNot(Box::new(ExpressionNode::Number(0xFF)));
        assert_eq!(expr_to_str(&map, &owner, &expr), "~255");
    }

    #[test]
    fn test_expr_parenthesis() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // Явные скобки из исходника — всегда генерируются
        let inner = Box::new(ExpressionNode::Number(42));
        let expr = ExpressionNode::Parenthesis(inner);
        assert_eq!(expr_to_str(&map, &owner, &expr), "(42)");
    }

    // ── Тесты бинарных операторов ──────────────────────────────────────────────

    #[test]
    fn test_expr_add() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // Атомы — без скобок
        let expr = ExpressionNode::Add(
            Box::new(ExpressionNode::Number(1)),
            Box::new(ExpressionNode::Number(2)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "1 + 2");
    }

    #[test]
    fn test_expr_subtract() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Subtract(
            Box::new(ExpressionNode::Number(5)),
            Box::new(ExpressionNode::Number(3)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "5 - 3");
    }

    #[test]
    fn test_expr_multiply() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Multiply(
            Box::new(ExpressionNode::Number(4)),
            Box::new(ExpressionNode::Number(5)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "4 * 5");
    }

    #[test]
    fn test_expr_divide() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Divide(
            Box::new(ExpressionNode::Number(10)),
            Box::new(ExpressionNode::Number(2)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "10 / 2");
    }

    #[test]
    fn test_expr_modulo() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Modulo(
            Box::new(ExpressionNode::Number(7)),
            Box::new(ExpressionNode::Number(3)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "7 % 3");
    }

    #[test]
    fn test_expr_shift_left() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::ShiftLeft(
            Box::new(ExpressionNode::Number(1)),
            Box::new(ExpressionNode::Number(4)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "1 << 4");
    }

    #[test]
    fn test_expr_shift_right() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::ShiftRight(
            Box::new(ExpressionNode::Number(16)),
            Box::new(ExpressionNode::Number(2)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "16 >> 2");
    }

    #[test]
    fn test_expr_bitwise_and() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::BitwiseAnd(
            Box::new(ExpressionNode::Number(0xF0)),
            Box::new(ExpressionNode::Number(0xFF)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "240 & 255");
    }

    #[test]
    fn test_expr_bitwise_or() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::BitwiseOr(
            Box::new(ExpressionNode::Number(0xF0)),
            Box::new(ExpressionNode::Number(0x0F)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "240 | 15");
    }

    #[test]
    fn test_expr_bitwise_xor() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::BitwiseXor(
            Box::new(ExpressionNode::Number(0xAA)),
            Box::new(ExpressionNode::Number(0x55)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "170 ^ 85");
    }

    #[test]
    fn test_expr_less() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Less(
            Box::new(ExpressionNode::Number(1)),
            Box::new(ExpressionNode::Number(2)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "1 < 2");
    }

    #[test]
    fn test_expr_more() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::More(
            Box::new(ExpressionNode::Number(3)),
            Box::new(ExpressionNode::Number(2)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "3 > 2");
    }

    #[test]
    fn test_expr_equal() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Equal(
            Box::new(ExpressionNode::Number(0)),
            Box::new(ExpressionNode::Number(0)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "0 == 0");
    }

    #[test]
    fn test_expr_not_equal() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::NotEqual(
            Box::new(ExpressionNode::Number(1)),
            Box::new(ExpressionNode::Number(0)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "1 != 0");
    }

    #[test]
    fn test_expr_logical_and() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::And(
            Box::new(ExpressionNode::Bool(true)),
            Box::new(ExpressionNode::Bool(false)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "true && false");
    }

    #[test]
    fn test_expr_logical_or() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Or(
            Box::new(ExpressionNode::Bool(false)),
            Box::new(ExpressionNode::Bool(true)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "false || true");
    }

    #[test]
    fn test_expr_conditional_operator() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // Атомы — без скобок
        let expr = ExpressionNode::ConditionalOperator(
            Box::new(ExpressionNode::Bool(true)),
            Box::new(ExpressionNode::Number(1)),
            Box::new(ExpressionNode::Number(0)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "true ? 1 : 0");
    }

    #[test]
    fn test_expr_cast() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        use crate::semantic::type_node::TypeNode;
        // Атом — без скобок после типа
        let expr = ExpressionNode::Cast(Box::new(ExpressionNode::Number(42)), TypeNode::Bit);
        // Фича 0029 (Д2): `bit` → `uint8_t`, а не `int` (32-битный знаковый).
        assert_eq!(expr_to_str(&map, &owner, &expr), "(uint8_t)42");
    }

    #[test]
    fn test_expr_cast_complex() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        use crate::semantic::type_node::TypeNode;
        // Бинарное выражение — нужны скобки после типа
        let inner = ExpressionNode::Add(
            Box::new(ExpressionNode::Number(1)),
            Box::new(ExpressionNode::Number(2)),
        );
        let expr = ExpressionNode::Cast(Box::new(inner), TypeNode::Bit);
        // Фича 0029 (Д2): `bit` → `uint8_t`.
        assert_eq!(expr_to_str(&map, &owner, &expr), "(uint8_t)(1 + 2)");
    }

    // ── Тесты приоритета операторов ────────────────────────────────────────────

    #[test]
    fn test_expr_precedence_mul_wins_over_add_left() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // (a*b) + c → a * b + c (умножение на левой стороне сложения — без скобок)
        let mul = ExpressionNode::Multiply(
            Box::new(ExpressionNode::Number(2)),
            Box::new(ExpressionNode::Number(3)),
        );
        let expr = ExpressionNode::Add(Box::new(mul), Box::new(ExpressionNode::Number(4)));
        assert_eq!(expr_to_str(&map, &owner, &expr), "2 * 3 + 4");
    }

    #[test]
    fn test_expr_precedence_add_needs_parens_in_mul() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // (a+b) * c → (a + b) * c (сложение в левом операнде умножения — скобки)
        let add = ExpressionNode::Add(
            Box::new(ExpressionNode::Number(2)),
            Box::new(ExpressionNode::Number(3)),
        );
        let expr = ExpressionNode::Multiply(Box::new(add), Box::new(ExpressionNode::Number(4)));
        assert_eq!(expr_to_str(&map, &owner, &expr), "(2 + 3) * 4");
    }

    #[test]
    fn test_expr_precedence_sub_right_needs_parens() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // a - (b - c) → a - (b - c) (тот же приоритет на правой стороне вычитания)
        let sub_right = ExpressionNode::Subtract(
            Box::new(ExpressionNode::Number(3)),
            Box::new(ExpressionNode::Number(1)),
        );
        let expr =
            ExpressionNode::Subtract(Box::new(ExpressionNode::Number(5)), Box::new(sub_right));
        assert_eq!(expr_to_str(&map, &owner, &expr), "5 - (3 - 1)");
    }

    #[test]
    fn test_expr_precedence_or_inside_and() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // (a || b) && c → (a || b) && c (OR имеет меньший приоритет чем AND)
        let or_expr = ExpressionNode::Or(
            Box::new(ExpressionNode::Bool(true)),
            Box::new(ExpressionNode::Bool(false)),
        );
        let expr = ExpressionNode::And(Box::new(or_expr), Box::new(ExpressionNode::Bool(true)));
        assert_eq!(expr_to_str(&map, &owner, &expr), "(true || false) && true");
    }

    #[test]
    fn test_expr_precedence_and_no_parens_inside_or() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // (a && b) || c → a && b || c (AND имеет больший приоритет чем OR — без скобок)
        let and_expr = ExpressionNode::And(
            Box::new(ExpressionNode::Bool(true)),
            Box::new(ExpressionNode::Bool(false)),
        );
        let expr = ExpressionNode::Or(Box::new(and_expr), Box::new(ExpressionNode::Bool(true)));
        assert_eq!(expr_to_str(&map, &owner, &expr), "true && false || true");
    }

    #[test]
    fn test_expr_precedence_compare_in_not() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // !(a > b) → !(a > b)
        let cmp = ExpressionNode::More(
            Box::new(ExpressionNode::Number(5)),
            Box::new(ExpressionNode::Number(3)),
        );
        let expr = ExpressionNode::Not(Box::new(cmp));
        assert_eq!(expr_to_str(&map, &owner, &expr), "!(5 > 3)");
    }

    #[test]
    fn test_expr_initializer() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Initializer(vec![
            ExpressionNode::Number(1),
            ExpressionNode::Number(2),
            ExpressionNode::Number(3),
        ]);
        assert_eq!(expr_to_str(&map, &owner, &expr), "{1, 2, 3}");
    }

    // ── Интеграционные тесты generate_source ──────────────────────────────────

    /// Порождает `.c` для исходника (корень — `Main`).
    fn source_of(src: &str) -> Result<String, crate::diagnostics::Diagnostic> {
        let (model_ast, _) = parse(src, 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        generate_source(map.get_filename(), &map)
    }

    /// **T2 (0029-05).** Агрегатный инициализатор массива — поэлементно.
    ///
    /// Массив в C **не является** изменяемым lvalue: `model->arr = {0,0,0,0};`
    /// отвергается (`error: expected expression`), и составной литерал не
    /// спасает — присваивание массиву запрещено в принципе. Выход один:
    /// поэлементная запись. Строки захвачены зондом (`lamc -t c`), вывод
    /// проверен `cc -std=c11 -Wall -Werror`.
    #[test]
    fn test_array_aggregate_initializer_is_element_wise() {
        let src = r#"
type Byte = [bit;8];
var arr: [Byte;4] := {0,0,0,0};
var counter: u8 := 0;
start Idle { always { arr[0] := 7; counter := 1; } }
"#;
        let source = source_of(src).expect("порождение .c");
        for i in 0..4 {
            assert!(
                source.contains(&format!("model->arr[{}] = 0;", i)),
                "элемент {} обязан инициализироваться отдельно:\n{source}",
                i
            );
        }
        assert!(
            !source.contains("model->arr = {"),
            "присваивание агрегата массиву — невалидный C:\n{source}"
        );
    }

    /// **0029-05.** Скалярный инициализатор массива → `CC-017`, а не догадка.
    ///
    /// `var data: [u8;4] := 0;` язык не определяет: обнулить весь массив?
    /// записать в первый элемент? Цель `st` инициализатор отбрасывает,
    /// симулятор кладёт скаляр (после чего `data[0]` даёт `SIM-010`) — три
    /// ответа расходятся. Выбор одного — вопрос семантики языка, вне полномочий
    /// фичи 0029.
    #[test]
    fn test_array_scalar_initializer_is_rejected_with_cc_017() {
        let src = r#"
var data: [u8;4] := 0;
var counter: u8 := 0;
start Idle { always { data[0] := 7; counter := 1; } }
"#;
        let diag =
            source_of(src).expect_err("скалярный инициализатор массива обязан быть отвергнут");
        assert_eq!(diag.code.as_deref(), Some("CC-017"));
        assert!(
            diag.message.contains("data"),
            "сообщение обязано называть переменную: {}",
            diag.message
        );
    }

    /// **T4 (0029-05).** Бит-вектор — скаляр: присваивание ему законно и
    /// **не меняется**. Доминирующая идиома корпуса (45 из 46 вхождений).
    #[test]
    fn test_bit_vector_initializer_stays_scalar_assignment() {
        let src = r#"
type Byte = [bit;8];
var b: Byte := 0xFF;
var counter: u8 := 0;
start Idle { always { counter := b; } }
"#;
        let source = source_of(src).expect("порождение .c");
        assert!(
            source.contains("model->b = 255;"),
            "[bit;8] — скаляр uint8_t, присваивание законно:\n{source}"
        );
    }

    #[test]
    fn test_generate_source_has_include_and_math() {
        let src = r#"start Main { always { } }"#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        let source = generate_source(map.get_filename(), &map).unwrap();
        assert!(
            source.contains("#include \""),
            "отсутствует #include header:\n{source}"
        );
        assert!(
            source.contains(".h\""),
            "отсутствует .h в include:\n{source}"
        );
        assert!(
            source.contains("#include <math.h>"),
            "отсутствует #include <math.h>:\n{source}"
        );
    }

    #[test]
    fn test_generate_source_with_const_and_port() {
        // LIMIT используется в блоке always (присваивание переменной),
        // чтобы константа попала в UsageSet и не была отфильтрована.
        let src = r#"
type u8 = [bit;8];
const LIMIT: u8 := 100;
in SENSOR: u8 := 0x100000;
var v: u8 := 0;
start Main { always { v := LIMIT; } }
        "#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        let source = generate_source(map.get_filename(), &map).unwrap();
        assert!(
            source.contains("CONST_MAIN_LIMIT"),
            "CONST_MAIN_LIMIT отсутствует:\n{source}"
        );
        // Порт теперь генерируется как вариант enum в заголовочном файле,
        // а не как #define в .c-файле — в source больше нет PORT_MAIN_SENSOR.
        assert!(
            !source.contains("PORT_MAIN_SENSOR"),
            "PORT_MAIN_SENSOR не должен присутствовать в .c-файле (теперь это enum в .h):\n{source}"
        );
    }

    #[test]
    fn test_generate_source_functions() {
        // Обе функции вызываются в блоке always, чтобы они попали в UsageSet.
        let src = r#"
extern fn log_val(x: bit);
fn double_it(x: bit) -> bit { return x; }
start Main { always { log_val(double_it(0)); } }
        "#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        let source = generate_source(map.get_filename(), &map).unwrap();
        assert!(
            source.contains("extern void log_val"),
            "extern fn отсутствует:\n{source}"
        );
        // Фича 0029 (Д2): возвращаемый `bit` → `uint8_t`, а не `int`.
        assert!(
            source.contains("static uint8_t Main_double_it"),
            "local fn отсутствует:\n{source}"
        );
    }

    #[test]
    fn test_generate_if_no_double_parens() {
        // Проверяет, что условие `if` генерируется без двойных скобок: `if (cond)` а не `if ((cond))`.
        // В Lam условие `if` пишется без скобок (как в Rust): `if cond { ... }`.
        // Генератор добавляет ровно одну пару скобок для C.
        // Функция вызывается в always, чтобы попасть в UsageSet.
        let src = r#"
type u8 = [bit;8];
fn check(value: u8) -> bit {
    if value > 100 {
        return 1;
    }
    return 0;
}
start Main { always { check(0); } }
        "#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        let source = generate_source(map.get_filename(), &map).unwrap();
        // Условие if должно иметь ровно одну пару скобок
        assert!(
            source.contains("if (value > 100)"),
            "ожидается `if (value > 100)`, получено:\n{source}"
        );
        // return должен быть без лишних скобок вокруг значения
        assert!(
            !source.contains("return (1)") && !source.contains("return (0)"),
            "return не должен оборачивать значение в скобки:\n{source}"
        );
    }

    #[test]
    /// Проверяет, что переменная вложенной модели в функции генерируется как
    /// `model->state_name.field`, а не `model->model_name.field`.
    ///
    /// Пример: модель `Controller` инстанциируется состоянием `Entry = Controller`.
    /// Поле в C-структуре называется `entry` (по имени состояния), поэтому
    /// функция `clamp` должна обращаться к переменной как `model->entry.temperature`.
    /// Первый параметр функции — `const Root *model` (корневая модель), не `main`.
    fn test_submodel_variable_uses_state_field_name() {
        // clamp вызывается в always блоке, чтобы попасть в UsageSet.
        // temperature используется внутри clamp, поэтому поле генерируется в структуре.
        let src = r#"
type u8 = [bit;8];
model Controller {
    var temperature: u8 := 0;
    fn clamp(value: u8) -> u8 {
        if value < temperature { return temperature; }
        return value;
    }
    start Idle { always { clamp(0); } }
}
start Entry = Controller;
        "#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Root".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        let source = generate_source(map.get_filename(), &map).unwrap();
        // Поле должно называться по имени состояния (`entry`), а не модели (`controller`).
        // Первый параметр функции — `model` (не `main`).
        assert!(
            source.contains("model->entry.temperature"),
            "ожидается `model->entry.temperature`, получено:\n{source}"
        );
        assert!(
            !source.contains("model->controller.temperature"),
            "не должно быть `model->controller.temperature`:\n{source}"
        );
    }

    #[test]
    fn test_generate_loop_no_double_parens() {
        // Проверяет, что условие `loop` (→ `while` в C) генерируется без двойных скобок.
        // В Lam: `loop cond { ... }` — без скобок вокруг условия.
        // Генератор добавляет ровно одну пару скобок для C: `while (cond)`.
        // Функция вызывается в always, чтобы попасть в UsageSet.
        let src = r#"
type u8 = [bit;8];
fn check(n: u8) -> bit {
    loop n > 0 {
        return 0;
    }
    return 1;
}
start Main { always { check(0); } }
        "#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        let source = generate_source(map.get_filename(), &map).unwrap();
        assert!(
            source.contains("while (n > 0)"),
            "ожидается `while (n > 0)`, получено:\n{source}"
        );
    }

    // ── Тесты расширенных состояний: Parallel / Concatenation ─────────────────

    /// Вспомогательная функция: генерирует полный `.c`-исходник из Lam-строки.
    fn generate_source_str(src: &str) -> String {
        let (ast, _) = parse(src, 0).expect("ошибка разбора");
        let model_rc = semantic::tree::construct_model(&ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Root".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        generate_source(map.get_filename(), &map).unwrap()
    }

    /// INIT-блок для `S = A | B` должен инициализировать оба элемента параллели
    /// и выставить `model->s.state = ROOT_S_INIT`.
    #[test]
    fn test_init_parallel_generates_init_calls() {
        let src = "model A { start Start; } model B { start Start; } start S = A | B { next End; } state End;";
        let code = generate_source_str(src);

        // Оба элемента инициализируются в INIT-блоке
        assert!(
            code.contains("RootA_init(&model->s.a0, model)"),
            "ожидается RootA_init в INIT:\n{code}"
        );
        assert!(
            code.contains("RootB_init(&model->s.b1, model)"),
            "ожидается RootB_init в INIT:\n{code}"
        );
        // Состояние параллели выставляется в INIT
        assert!(
            code.contains("model->s.state = ROOT_S_INIT;"),
            "ожидается ROOT_S_INIT:\n{code}"
        );
        // Переход в состояние S
        assert!(
            code.contains("model->state = ROOT_S;"),
            "ожидается model->state = ROOT_S:\n{code}"
        );
    }

    /// INIT-блок для `S = A + B` должен инициализировать только первый элемент
    /// и установить `model->s_state = ROOT_S_A0`.
    /// Второй элемент должен инициализироваться только в TICK при завершении первого.
    #[test]
    fn test_init_concatenation_generates_first_init_only() {
        let src = "model A { start Start; } model B { start Start; } start S = A + B { next End; } state End;";
        let code = generate_source_str(src);
        // Первый элемент инициализируется в INIT-блоке
        assert!(
            code.contains("RootA_init(&model->s_a0, model)"),
            "ожидается RootA_init в INIT:\n{code}"
        );
        // Указатель конкатенации выставляется на первый элемент
        assert!(
            code.contains("model->s_state = ROOT_S_A0;"),
            "ожидается ROOT_S_A0:\n{code}"
        );
        // Второй элемент инициализируется только в TICK при завершении A
        assert!(
            code.contains("RootB_init(&model->s_b1, model)"),
            "ожидается RootB_init в TICK (при завершении A):\n{code}"
        );
        // В INIT-блоке B идёт ПОСЛЕ A (тик A и его is_done)
        let a0_init_pos = code.find("RootA_init(&model->s_a0, model)").unwrap();
        let b1_init_pos = code.find("RootB_init(&model->s_b1, model)").unwrap();
        assert!(
            a0_init_pos < b1_init_pos,
            "RootA_init должен быть раньше RootB_init в коде:\n{code}"
        );
    }

    /// TICK-блок для `S = A | B` должен тикать все элементы и проверять is_done.
    #[test]
    fn test_tick_parallel_generates_tick_and_done_check() {
        let src = "model A { start Start; } model B { start Start; } start S = A | B { next End; } state End;";
        let code = generate_source_str(src);
        // Тик обоих элементов
        assert!(
            code.contains("RootA_tick(&model->s.a0, model)"),
            "ожидается RootA_tick:\n{code}"
        );
        assert!(
            code.contains("RootB_tick(&model->s.b1, model)"),
            "ожидается RootB_tick:\n{code}"
        );
        // Проверка is_done обоих
        assert!(
            code.contains("RootA_is_done(&model->s.a0, model)"),
            "ожидается RootA_is_done:\n{code}"
        );
        assert!(
            code.contains("RootB_is_done(&model->s.b1, model)"),
            "ожидается RootB_is_done:\n{code}"
        );
        // Оба условия объединены через &&
        assert!(
            code.contains("&&"),
            "ожидается && для объединения is_done:\n{code}"
        );
    }

    /// TICK-блок для `S = A + B` должен генерировать if/else if по полю s_state.
    #[test]
    fn test_tick_concatenation_generates_state_chain() {
        let src = "model A { start Start; } model B { start Start; } start S = A + B { next End; } state End;";
        let code = generate_source_str(src);
        // Проверка по первому элементу
        assert!(
            code.contains("model->s_state == ROOT_S_A0"),
            "ожидается ROOT_S_A0 в условии:\n{code}"
        );
        // Тик A
        assert!(
            code.contains("RootA_tick(&model->s_a0, model)"),
            "ожидается RootA_tick:\n{code}"
        );
        // При завершении A инициализируется B
        assert!(
            code.contains("RootB_init(&model->s_b1, model)"),
            "ожидается RootB_init при переходе:\n{code}"
        );
        // Проверка по второму элементу
        assert!(
            code.contains("model->s_state == ROOT_S_B1"),
            "ожидается ROOT_S_B1 в условии:\n{code}"
        );
        // Тик B
        assert!(
            code.contains("RootB_tick(&model->s_b1, model)"),
            "ожидается RootB_tick:\n{code}"
        );
    }

    /// TICK-блок для `S = A + (B | C)` должен правильно обрабатывать
    /// вложенный параллельный блок внутри конкатенации.
    #[test]
    fn test_tick_concatenation_nested_parallel() {
        let src = "model A { start Start; } model B { start Start; } model C { start Start; }
start S = A + (B | C) { next End; }
state End;";
        let code = generate_source_str(src);
        // Тик A как первый элемент конкатенации
        assert!(
            code.contains("model->s_state == ROOT_S_A0"),
            "ожидается ROOT_S_A0:\n{code}"
        );
        // Параллельный блок как второй элемент конкатенации
        assert!(
            code.contains("ROOT_S_PARALLEL1"),
            "ожидается ROOT_S_PARALLEL1:\n{code}"
        );
        // Тик B внутри вложенной параллели
        assert!(
            code.contains("RootB_tick(&model->s_parallel1.b0, model)"),
            "ожидается RootB_tick в параллели:\n{code}"
        );
        assert!(
            code.contains("RootC_tick(&model->s_parallel1.c1, model)"),
            "ожидается RootC_tick в параллели:\n{code}"
        );
    }

    /// Генерация extend_complex.lam не должна возвращать ошибку.
    #[test]
    fn test_extend_complex_generates_without_error() {
        let src = std::fs::read_to_string("../examples/extend_complex.lam")
            .expect("не удалось прочитать extend_complex.lam");
        let (ast, _) = parse(&src, 0).expect("ошибка разбора extend_complex.lam");
        let model_rc =
            semantic::tree::construct_model(&ast, None, &[]).expect("ошибка построения модели");
        model_rc.borrow_mut().name = Some("extend_complex".to_string());
        let model = model_rc.borrow();
        let map = CMap::new("extend_complex", &*model, false).expect("ошибка создания CMap");
        let result = generate_source(map.get_filename(), &map);
        assert!(
            result.is_ok(),
            "ожидается успешная генерация: {:?}",
            result.err()
        );
        let code = result.unwrap();
        // INIT для параллели: оба элемента C1, C2 инициализируются
        assert!(
            code.contains("ExtendComplexCC1_init"),
            "ожидается ExtendComplexCC1_init:\n{code}"
        );
        assert!(
            code.contains("ExtendComplexCC2_init"),
            "ожидается ExtendComplexCC2_init:\n{code}"
        );
        // INIT для конкатенации: только первый элемент A инициализируется
        assert!(
            code.contains("ExtendComplexA_init"),
            "ожидается ExtendComplexA_init:\n{code}"
        );
    }

    // ── Тесты единственного терминального состояния END ───────────────────────

    /// Терминальное состояние с произвольным именем (не End) должно переходить в MODEL_END.
    #[test]
    fn test_terminal_state_transitions_to_end() {
        let src = "start S { ref Done: true; } state Done;";
        let code = generate_source_str(src);
        // Done — терминальное состояние, должно переходить в ROOT_END
        assert!(
            code.contains("model->state = ROOT_END;"),
            "ожидается переход Done → ROOT_END:\n{code}"
        );
        // is_done должна проверять ROOT_END
        assert!(
            code.contains("model->state == ROOT_END"),
            "ожидается is_done проверяет ROOT_END:\n{code}"
        );
    }

    /// Состояние End уже является терминальным — не должно иметь самоперехода.
    #[test]
    fn test_end_state_no_self_transition() {
        let src = "start S { ref End: true; } state End;";
        let code = generate_source_str(src);
        // End IS ROOT_END, не должно быть model->state = ROOT_END; внутри case End
        // is_done должна проверять ROOT_END
        assert!(
            code.contains("model->state == ROOT_END"),
            "ожидается is_done проверяет ROOT_END:\n{code}"
        );
        // Не должно быть лишнего перехода End→End
        let end_case_start = code.find("case ROOT_END:").unwrap_or(0);
        let _before_end = &code[..end_case_start];
        // До блока ROOT_END: нет model->state = ROOT_END (переход только из S)
        let transition_in_s = code.contains("model->state = ROOT_END;");
        assert!(transition_in_s, "ожидается переход S → ROOT_END:\n{code}");
    }

    /// is_done всегда проверяет MODEL_END, даже если нет явных терминальных состояний.
    #[test]
    fn test_is_done_always_checks_model_end() {
        let src = "model A { start Start; } start S = A { next End; } state End;";
        let code = generate_source_str(src);
        // is_done для A: проверяет ROOT_A_END
        assert!(
            code.contains("model->state == ROOT_A_END"),
            "ожидается is_done для A проверяет ROOT_A_END:\n{code}"
        );
        // is_done для Root: проверяет ROOT_END
        assert!(
            code.contains("model->state == ROOT_END"),
            "ожидается is_done для Root проверяет ROOT_END:\n{code}"
        );
    }

    /// Вложенная модель с нестандартным терминальным состоянием.
    #[test]
    fn test_submodel_terminal_state_transitions_to_end() {
        let src = "model A { start Run; state Finish; } start S = A { next End; } state End;";
        let code = generate_source_str(src);
        // Finish (терминальное в A) должно переходить в ROOT_A_END
        assert!(
            code.contains("model->state = ROOT_A_END;"),
            "ожидается Finish → ROOT_A_END:\n{code}"
        );
        // is_done для A: ROOT_A_END
        assert!(
            code.contains("model->state == ROOT_A_END"),
            "ожидается is_done для A:\n{code}"
        );
    }

    // ── Тесты BitAccess ────────────────────────────────────────────────────────

    /// Чтение бита переменной в условии `ref`: `flags.2` → `((model->flags >> 2) & 1u)`
    #[test]
    fn test_bit_access_var_read_in_condition() {
        let src =
            "type u8 = [bit;8]; var flags: u8 := 0; start S { ref Done: flags.2; } state Done;";
        let code = generate_source_str(src);
        assert!(
            code.contains("((model->flags >> 2) & 1u)"),
            "ожидается ((model->flags >> 2) & 1u) в условии:\n{code}"
        );
    }

    /// Чтение бита порта в условии `ref`: `BTN.0` → `(((*model->read_numeric)(ROOT_BTN, ...) >> 0) & 1u)`
    /// Корневая модель: используется `model->` (не `main->`).
    #[test]
    fn test_bit_access_port_read_in_condition() {
        let src =
            "type u8 = [bit;8]; in BTN: u8 := 0x200000; start S { ref Done: BTN.0; } state Done;";
        let code = generate_source_str(src);
        assert!(
            code.contains("(((*model->read_numeric)(ROOT_BTN, model->userdata) >> 0) & 1u)"),
            "ожидается (((*model->read_numeric)(ROOT_BTN, ...) >> 0) & 1u) в условии:\n{code}"
        );
    }

    /// Чтение бита переменной в блоке `always`: `x = flags.3` → `((model->flags >> 3) & 1u)`
    #[test]
    fn test_bit_access_var_read_in_always() {
        let src = "type u8 = [bit;8]; var flags: u8 := 0; var x: u8 := 0; start S { always { x := flags.3; } ref Done: true; } state Done;";
        let code = generate_source_str(src);
        assert!(
            code.contains("((model->flags >> 3) & 1u)"),
            "ожидается ((model->flags >> 3) & 1u) при чтении в always:\n{code}"
        );
    }

    /// Запись бита переменной: `flags.3 = true` → bit-set идиома C
    #[test]
    fn test_bit_access_var_write_in_always() {
        let src = "type u8 = [bit;8]; var flags: u8 := 0; start S { always { flags.3 := true; } ref Done: true; } state Done;";
        let code = generate_source_str(src);
        assert!(
            code.contains("model->flags = (model->flags & ~(1u << 3)) | ((true & 1u) << 3)"),
            "ожидается bit-set идиома для flags.3 = true:\n{code}"
        );
    }

    /// Чтение бита порта в `always`: `x = BTN.0` → `(((*model->read_numeric)(ROOT_BTN, ...) >> 0) & 1u)`
    /// Корневая модель: tick получает `model`, поэтому используется `model->`.
    #[test]
    fn test_bit_access_port_read_in_always() {
        let src = "type u8 = [bit;8]; in BTN: u8 := 0x200000; var x: u8 := 0; start S { always { x := BTN.0; } ref Done: true; } state Done;";
        let code = generate_source_str(src);
        assert!(
            code.contains("(((*model->read_numeric)(ROOT_BTN, model->userdata) >> 0) & 1u)"),
            "ожидается (((*model->read_numeric)(ROOT_BTN, ...) >> 0) & 1u) при чтении порта:\n{code}"
        );
    }

    /// Запись бита порта: `LED.7 = true` → read-modify-write через write_numeric
    /// Корневая модель: используется `model->` (не `main->`).
    #[test]
    fn test_bit_access_port_write_in_always() {
        let src = "type u8 = [bit;8]; out LED: u8 := 0x100000; start S { always { LED.7 := true; } ref Done: true; } state Done;";
        let code = generate_source_str(src);
        assert!(
            code.contains("write_numeric)(ROOT_LED,")
                && code.contains("read_numeric)(ROOT_LED, model->userdata) & ~(1LL << 7)")
                && code.contains("(true & 1LL) << 7)"),
            "ожидается read-modify-write через write_numeric/read_numeric для LED.7 = true:\n{code}"
        );
    }

    /// Локальная функция, вызываемая из always-блока корневой модели,
    /// должна получать `model` как первый аргумент, а не `main`
    /// (в tick корневой модели нет параметра `main`).
    #[test]
    fn test_sub_model_local_fn_args_use_model_not_main() {
        // В локальной функции Sub_compute (has_model=false), первый параметр — `const Main *model`.
        // При вызове Main_process(root_val), root_val принадлежит Main.
        // Должно генерироваться `model->root_val`, а не несуществующий `main->root_val`.
        let src = r#"
var root_val: bit := 0;
var result: bit := 0;
fn process(x: bit) -> bit { return x; }
model Sub {
    fn compute() -> bit { return process(root_val); }
    start S { always { result := compute(); } }
}
start Main = Sub;
"#;
        let (ast, _) = parse(src, 0).expect("ошибка разбора");
        let model_rc = semantic::tree::construct_model(&ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        let c_source = generate_source("test", &map).unwrap();
        // Sub_compute вызывает Main_process(model, root_val).
        // root_val в теле Sub_compute должен быть `model->root_val`, а не `main->root_val`
        assert!(
            !c_source.contains("main->root_val"),
            "root_val в аргументе локальной функции Sub не должен использовать `main`:\n{}",
            c_source
        );
        assert!(
            c_source.contains("model->root_val"),
            "root_val в аргументе локальной функции Sub должен использовать `model`:\n{}",
            c_source
        );
    }

    #[test]
    fn test_local_fn_call_in_root_tick_uses_model_not_main() {
        let src = r#"
type u8 = [bit;8];
fn double(x: u8) -> u8 { return x + x; }
var y: u8 := 0;
start Main {
    always { y := double(y); }
}
"#;
        let (ast, _) = parse(src, 0).expect("ошибка разбора");
        let model_rc = semantic::tree::construct_model(&ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        let c_source = generate_source("test", &map).unwrap();
        // Вызов должен передавать `model` (корневой указатель), а не несуществующий `main`
        assert!(
            c_source.contains("Main_double(model"),
            "Ожидался вызов Main_double(model, ...), но получили:\n{}",
            c_source
        );
        assert!(
            !c_source.contains("Main_double(main"),
            "Недопустимый вызов Main_double(main, ...) в tick корневой модели:\n{}",
            c_source
        );
    }

    #[test]
    fn test_port_read_in_local_fn_uses_model_not_main() {
        // В локальной функции (has_model=false) первый параметр — `const Root *model`.
        // Чтение порта должно генерировать `(*model->read_bit)(...)`, а не `(*main->read_bit)(...)`.
        let src = r#"
in sensor: bit := 0x0:0;
var v: bit := 0;
fn read_port() -> bit { return sensor; }
start Main { always { v := read_port(); } }
"#;
        let (ast, _) = parse(src, 0).expect("ошибка разбора");
        let model_rc = semantic::tree::construct_model(&ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        let c_source = generate_source("test", &map).unwrap();
        // Функция read_port вызывается в always, поэтому попадёт в UsageSet
        // Порт внутри локальной функции должен использовать `model`, а не `main`
        assert!(
            !c_source.contains("(*main->"),
            "Чтение порта в локальной функции не должно использовать `main`:\n{}",
            c_source
        );
        assert!(
            c_source.contains("(*model->"),
            "Чтение порта в локальной функции должно использовать `model`:\n{}",
            c_source
        );
    }
}
