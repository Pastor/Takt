//! Интеграционные тесты синтаксического анализатора BuT.
//!
//! # Структура АСД
//!
//! Функция [`parse`] всегда возвращает **корневую анонимную модель** (`name: None`),
//! содержащую все элементы верхнего уровня. Именованные модели (`model M { ... }`)
//! находятся внутри `root.elements` как `ModelElement::Model`.
//!
//! # Особенности грамматики
//!
//! - `var`-объявления поддерживаются только на уровне модели/состояния (`ModelElement::Variable`,
//!   `StateElement::Variable`), но НЕ внутри блоков операторов (`always {}`, `fn {}`).
//! - Управляющие конструкции `while`, `for`, `do-while` требуют скобки вокруг условия.
//! - `if` НЕ требует скобок вокруг условия.
//! - Условие в `cond` использует `=` (не `==`) для проверки равенства.

use std::fs;
use std::path::Path;

use grammar::ast::{Identifier, Location, ModelElement, StateElement, StateKind};
use grammar::parse;

// ─────────────────────────────── Вспомогательные функции ────────────────────

/// Разбирает строку `src` и возвращает корневую модель, либо паникует с описанием ошибок.
fn must_parse(src: &str) -> grammar::ast::Model {
    parse(src, 0).unwrap_or_else(|diagnostics| {
        let msgs: Vec<_> = diagnostics.iter().map(|d| d.message.clone()).collect();
        panic!("Разбор завершился с ошибками: {:?}", msgs);
    }).0
}

/// Извлекает первую именованную модель из корневой модели.
fn first_named_model(src: &str) -> grammar::ast::Model {
    let root = must_parse(src);
    root.elements.into_iter().find_map(|e| {
        if let ModelElement::Model(m) = e { Some(*m) } else { None }
    }).expect("В исходном коде должна быть именованная модель")
}

// ─────────────────────────── Позитивные тесты (по файлам) ───────────────────

/// Проверяет, что все `.but`-файлы из директории `valid` разбираются без ошибок.
#[test]
fn valid_files_parse_without_errors() {
    let dir = Path::new("tests/data/parser/valid");
    let entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("Не удалось прочитать директорию {:?}: {}", dir, e))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "but"))
        .collect();

    assert!(
        !entries.is_empty(),
        "Директория {:?} не содержит .but файлов",
        dir
    );

    for entry in entries {
        let path = entry.path();
        let src = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Не удалось прочитать {:?}: {}", path, e));

        parse(&src, 0).unwrap_or_else(|diagnostics| {
            let msgs: Vec<_> = diagnostics.iter().map(|d| d.message.clone()).collect();
            panic!("Файл {:?} вызвал ошибки разбора: {:?}", path, msgs);
        });
    }
}

// ─────────────────────── Негативные тесты (контр-примеры по файлам) ─────────

/// Проверяет, что все `.but`-файлы из директории `invalid` порождают диагностику.
#[test]
fn invalid_files_produce_parse_errors() {
    let dir = Path::new("tests/data/parser/invalid");
    let entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("Не удалось прочитать директорию {:?}: {}", dir, e))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "but"))
        .collect();

    assert!(
        !entries.is_empty(),
        "Директория {:?} не содержит .but файлов",
        dir
    );

    for entry in entries {
        let path = entry.path();
        let src = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Не удалось прочитать {:?}: {}", path, e));

        let result = parse(&src, 0);
        assert!(
            result.is_err(),
            "Файл {:?} ожидался с ошибкой, но разобрался успешно",
            path
        );
    }
}

// ─────────────────────────────── Тесты структуры корневой модели ────────────

/// Корневая модель всегда анонимна (`name == None`).
#[test]
fn root_model_is_always_anonymous() {
    let root = must_parse("model M { start S; }");
    assert!(root.name.is_none(), "Корневая модель всегда анонимна");
}

/// `model M { ... }` создаёт элемент `ModelElement::Model` внутри корня.
#[test]
fn named_model_is_inside_root_elements() {
    let root = must_parse("model M { start S; }");
    assert_eq!(root.elements.len(), 1, "Корень должен содержать 1 элемент");
    assert!(
        matches!(&root.elements[0], ModelElement::Model(m) if m.name.as_ref().map(|id| id.name.as_str()) == Some("M")),
        "Первый элемент должен быть моделью M"
    );
}

/// Несколько моделей верхнего уровня образуют несколько элементов корня.
#[test]
fn multiple_top_level_models() {
    let src = "model A { start S; } model B { start S; }";
    let root = must_parse(src);
    let model_count = root.elements.iter()
        .filter(|e| matches!(e, ModelElement::Model(_)))
        .count();
    assert_eq!(model_count, 2, "Ожидалось 2 именованных модели");
}

/// Пустая программа разбирается в пустую корневую модель.
#[test]
fn empty_source_parses_to_empty_model() {
    let root = must_parse("");
    assert!(root.elements.is_empty(), "Пустой файл → пустая модель");
    assert!(root.name.is_none(), "Корневая модель не имеет имени");
}

/// Только комментарии → пустая модель, комментарии в векторе.
#[test]
fn comments_only_parse_to_empty_model() {
    let src = "// это комментарий\n/// документация\n// ещё";
    let (root, comments) = parse(src, 0).unwrap();
    assert!(root.elements.is_empty(), "Только комментарии → элементов нет");
    assert_eq!(comments.len(), 3, "Ожидалось 3 комментария");
}

// ─────────────────────────────── Тесты моделей ──────────────────────────────

/// Минимальная модель содержит одно состояние.
#[test]
fn parse_minimal_model() {
    let m = first_named_model("model M { start S; }");
    assert_eq!(m.name.as_ref().map(|id| id.name.as_str()), Some("M"));
    assert_eq!(m.elements.len(), 1, "Модель M должна содержать 1 элемент");
    assert!(matches!(m.elements[0], ModelElement::State(_)), "Ожидалось состояние");
}

/// Вложенные модели.
#[test]
fn parse_nested_models() {
    let src = r#"
        model Outer {
            model Inner { start S; }
            start Begin;
        }
    "#;
    let outer = first_named_model(src);
    let has_inner = outer.elements.iter().any(|e| matches!(e, ModelElement::Model(_)));
    assert!(has_inner, "Ожидалась вложенная модель Inner");
}

/// Модель без имени — ошибка восстановления: `name == None`.
#[test]
fn parse_model_with_missing_name_recovers() {
    let result = parse("model { start S; }", 0);
    match result {
        Ok((root, _)) => {
            // Восстановление: в корне есть элемент-модель с name=None
            let has_anonymous = root.elements.iter().any(|e| {
                matches!(e, ModelElement::Model(m) if m.name.is_none())
            });
            assert!(has_anonymous, "Модель без имени должна иметь name=None");
        }
        Err(diagnostics) => {
            assert!(!diagnostics.is_empty(), "Должна быть хотя бы одна диагностика");
        }
    }
}

// ──────────────────────────── Тесты состояний ───────────────────────────────

/// Начальное (`start`) и обычное (`state`) состояния модели.
#[test]
fn parse_start_and_state() {
    let m = first_named_model("model M { start Begin; state End; }");
    assert_eq!(m.elements.len(), 2, "Модель M должна иметь 2 элемента");

    if let ModelElement::State(s) = &m.elements[0] {
        assert_eq!(s.kind, Some(StateKind::Start), "Первое должно быть 'start'");
        assert_eq!(s.name.as_ref().map(|id| id.name.as_str()), Some("Begin"));
    } else {
        panic!("Первый элемент должен быть состоянием");
    }

    if let ModelElement::State(s) = &m.elements[1] {
        assert!(s.kind.is_none(), "Второе не должно быть 'start'");
        assert_eq!(s.name.as_ref().map(|id| id.name.as_str()), Some("End"));
    } else {
        panic!("Второй элемент должен быть состоянием");
    }
}

/// Состояние с реализацией: `state Sub = ModelName`.
#[test]
fn parse_state_with_implements() {
    let src = r#"
        model M { start S; }
        model Wrapper {
            state Sub = M { next End; }
            state End;
        }
    "#;
    let root = must_parse(src);
    assert!(!root.elements.is_empty());
}

// ─────────────────────────── Тесты ссылок (ref) ─────────────────────────────

/// `ref End` — ссылка без условия.
#[test]
fn parse_ref_without_condition() {
    let m = first_named_model("model M { start S { ref E; } state E; }");
    if let ModelElement::State(s) = &m.elements[0] {
        let has_ref = s.elements.iter().any(|e| {
            matches!(e, StateElement::Reference(_, id, None) if id.name == "E")
        });
        assert!(has_ref, "Ожидалась 'ref E' без условия");
    }
}

/// `ref E: true` — ссылка с булевым условием.
#[test]
fn parse_ref_with_bool_condition() {
    let m = first_named_model("model M { start S { ref E: true; } state E; }");
    if let ModelElement::State(s) = &m.elements[0] {
        let has_ref = s.elements.iter().any(|e| {
            matches!(e, StateElement::Reference(_, _, Some(_)))
        });
        assert!(has_ref, "Ожидалась ref с условием");
    }
}

/// `ref E: x > 0` — ссылка со сложным условием.
#[test]
fn parse_ref_with_complex_condition() {
    let src = r#"
        var x: bit = false;
        model M { start S { ref E: x > 0; } state E; }
    "#;
    must_parse(src);
}

// ─────────────────────── Тесты псевдонимов типов ────────────────────────────

/// `type Flag = bit`.
#[test]
fn parse_type_alias_bit() {
    let root = must_parse("type Flag = bit; model M { start S; }");
    let alias = root.elements.iter().find_map(|e| {
        if let ModelElement::Type(t) = e { Some(t.as_ref()) } else { None }
    });
    assert!(alias.is_some(), "Ожидался псевдоним типа на верхнем уровне");
    assert_eq!(alias.unwrap().name.name, "Flag");
}

/// `type u8 = [bit;8]`.
#[test]
fn parse_type_alias_array() {
    let root = must_parse("type u8 = [bit;8]; model M { start S; }");
    let alias = root.elements.iter().find_map(|e| {
        if let ModelElement::Type(t) = e { Some(t.as_ref()) } else { None }
    });
    assert!(alias.is_some());
    assert_eq!(alias.unwrap().name.name, "u8");
    assert!(
        matches!(alias.unwrap().ty, grammar::ast::Type::Array { element_count: 8, .. }),
        "u8 = [bit;8]"
    );
}

// ─────────────────────────── Тесты переменных ───────────────────────────────

/// `var x: bit = false` на верхнем уровне.
#[test]
fn parse_mutable_variable() {
    let root = must_parse("var x: bit = false; model M { start S; }");
    let var = root.elements.iter().find_map(|e| {
        if let ModelElement::Variable(v) = e { Some(v.as_ref()) } else { None }
    });
    assert!(var.is_some(), "Ожидалась переменная");
    assert!(var.unwrap().mutability, "var должна быть изменяемой");
}

/// `const MAX: u8 = 255`.
#[test]
fn parse_const_variable() {
    let root = must_parse("type u8 = [bit;8]; const MAX: u8 = 255; model M { start S; }");
    let cst = root.elements.iter().find_map(|e| {
        if let ModelElement::Variable(v) = e { Some(v.as_ref()) } else { None }
    });
    assert!(cst.is_some(), "Ожидалась константа");
    assert!(!cst.unwrap().mutability, "const не должна быть изменяемой");
}

/// `var toggle = false` внутри состояния (как `StateElement::Variable`).
#[test]
fn parse_variable_inside_state() {
    let m = first_named_model("model M { start S { var toggle = false; } }");
    if let ModelElement::State(s) = &m.elements[0] {
        let has_var = s.elements.iter().any(|e| matches!(e, StateElement::Variable(_)));
        assert!(has_var, "Ожидалась переменная в состоянии");
    }
}

// ─────────────────────────── Тесты портов ───────────────────────────────────

/// `port A: bit = 00548835`.
#[test]
fn parse_port_with_address() {
    let root = must_parse("port A: bit = 00548835; model M { start S; }");
    let port = root.elements.iter().find_map(|e| {
        if let ModelElement::Port(p) = e { Some(p.as_ref()) } else { None }
    });
    assert!(port.is_some(), "Ожидался порт");
    assert_eq!(port.unwrap().name.as_ref().map(|id| id.name.as_str()), Some("A"));
}

// ───────────────────────── Тесты условий (cond) ─────────────────────────────

/// `cond IsZero = x = 0`.
#[test]
fn parse_condition_equality() {
    let root = must_parse("var x: bit = false; cond IsZero = x = 0; model M { start S; }");
    let cond = root.elements.iter().find_map(|e| {
        if let ModelElement::Condition(c) = e { Some(c.as_ref()) } else { None }
    });
    assert!(cond.is_some(), "Ожидалось условие");
    assert_eq!(cond.unwrap().name.as_ref().map(|id| id.name.as_str()), Some("IsZero"));
}

/// Составное условие: `cond Both = a = 1 | b = 1`.
#[test]
fn parse_condition_complex() {
    let src = r#"
        var a: bit = false;
        var b: bit = false;
        cond Both = a = 1 | b = 1;
        model M { start S; }
    "#;
    let root = must_parse(src);
    let has_cond = root.elements.iter().any(|e| matches!(e, ModelElement::Condition(_)));
    assert!(has_cond, "Ожидалось условие Both");
}

// ────────────────────── Тесты именованных блоков ────────────────────────────

/// `enter`, `exit`, `always` — именованные блоки в состоянии.
#[test]
fn parse_named_blocks_in_state() {
    let m = first_named_model(r#"
        model M {
            start S {
                enter  { }
                exit   { }
                always { }
            }
        }
    "#);
    if let ModelElement::State(s) = &m.elements[0] {
        let block_names: Vec<&str> = s.elements.iter().filter_map(|e| {
            if let StateElement::NamedBlockCode(b) = e {
                b.name.as_ref().map(|id| id.name.as_str())
            } else {
                None
            }
        }).collect();
        assert!(block_names.contains(&"enter"),  "Ожидался 'enter'");
        assert!(block_names.contains(&"exit"),   "Ожидался 'exit'");
        assert!(block_names.contains(&"always"), "Ожидался 'always'");
    }
}

// ─────────────────── Тесты управляющих конструкций ──────────────────────────
// Примечание: в BuT условие `if` не требует скобок,
// а условия `while`, `for`, `do-while` — требуют.

/// `if cond { }` — без скобок вокруг условия.
#[test]
fn parse_if_without_parens() {
    let m = first_named_model(r#"
        model M { start S { always { if true { } } } }
    "#);
    assert!(!m.elements.is_empty());
}

/// `if cond { } else { }`.
#[test]
fn parse_if_else() {
    must_parse(r#"model M { start S { always { if true { } else { } } } }"#);
}

/// `while (cond) { }` — условие В скобках.
#[test]
fn parse_while_loop_with_parens() {
    let m = first_named_model(r#"
        model M {
            var i: [bit;8] = 0;
            start S {
                var i: [bit;8] = 0;
                always {
                    while (i < 10) {
                        i = i + 1;
                    }
                }
            }
        }
    "#);
    assert!(!m.elements.is_empty());
}

/// `for (; cond; step) { }` — условие В скобках, init — выражение.
#[test]
fn parse_for_loop_with_parens() {
    let m = first_named_model(r#"
        model M {
            var i: [bit;8] = 0;
            start S {
                var i: [bit;8] = 0;
                always {
                    for (i = 0; i < 10; i = i + 1) { }
                }
            }
        }
    "#);
    assert!(!m.elements.is_empty());
}

/// `do { } while (cond);` — условие В скобках.
#[test]
fn parse_do_while_loop_with_parens() {
    let m = first_named_model(r#"
        model M {
            var x: [bit;8] = 5;
            start S {
                var x: [bit;8] = 5;
                always {
                    do {
                        x = x - 1;
                    } while (x > 0);
                }
            }
        }
    "#);
    assert!(!m.elements.is_empty());
}

// ──────────────────────── Тесты выражений ───────────────────────────────────
// Выражения внутри always {} работают только как `expr;` (не var).

/// Арифметические выражения внутри `always`.
#[test]
fn parse_arithmetic_expressions_in_always() {
    let m = first_named_model(r#"
        model M {
            var a: [bit;8] = 0;
            start S {
                var a: [bit;8] = 0;
                always {
                    a = 1 + 2;
                    a = 10 - 3;
                    a = 4 * 5;
                    a = 20 / 4;
                    a = 10 % 3;
                    a = 2 ** 8;
                }
            }
        }
    "#);
    assert!(!m.elements.is_empty());
}

/// Побитовые операции внутри `always`.
#[test]
fn parse_bitwise_expressions_in_always() {
    let m = first_named_model(r#"
        model M {
            var a: [bit;8] = 0;
            start S {
                var a: [bit;8] = 0;
                always {
                    a = 0xFF & 0x0F;
                    a = 0xF0 | 0x0F;
                    a = 0xAA ^ 0x55;
                    a = ~a;
                    a = 1 << 4;
                    a = 256 >> 4;
                }
            }
        }
    "#);
    assert!(!m.elements.is_empty());
}

/// Вызов функции внутри `always`.
#[test]
fn parse_function_call_in_always() {
    must_parse(r#"model M { start S { always { debug("hello"); } } }"#);
}

/// Инициализатор: `{ 0, 1, 2, 3 }`.
#[test]
fn parse_initializer_expression() {
    let root = must_parse(r#"
        type u8 = [bit;8];
        const VALS: u8 = { 0, 1, 2, 3 };
        model M { start S; }
    "#);
    assert!(!root.elements.is_empty());
}

/// Доступ к биту `.0`, `.7`.
#[test]
fn parse_bit_access() {
    let m = first_named_model(r#"
        type u8 = [bit;8];
        model M {
            var x: u8 = 0;
            start S {
                var x: u8 = 0;
                always {
                    x.0 = true;
                    x.7 = false;
                }
            }
        }
    "#);
    assert!(!m.elements.is_empty());
}

/// Приведение типа `as`.
#[test]
fn parse_cast_expression() {
    let m = first_named_model(r#"
        type u8  = [bit;8];
        type u16 = [bit;16];
        model M {
            var x: u8  = 0;
            var y: u16 = 0;
            start S {
                var x: u8  = 0;
                var y: u16 = 0;
                always {
                    y = x as u16;
                }
            }
        }
    "#);
    assert!(!m.elements.is_empty());
}

// ────────────────────────── Тесты импортов ──────────────────────────────────

/// `import "path";`.
#[test]
fn parse_import_plain() {
    let root = must_parse(r#"import "std.but"; model M { start S; }"#);
    let has_import = root.elements.iter().any(|e| matches!(e, ModelElement::Import(_)));
    assert!(has_import, "Ожидался импорт");
}

/// `import "path" as Alias;`.
#[test]
fn parse_import_with_alias() {
    must_parse(r#"import "utils.but" as Utils; model M { start S; }"#);
}

// ──────────────────────── Тесты функций ────────────────────────────────────

/// `fn log(msg: bit);` — объявление без тела.
#[test]
fn parse_function_declaration() {
    let root = must_parse(r#"
        fn log(msg: bit);
        model M { start S; }
    "#);
    let has_fn = root.elements.iter().any(|e| matches!(e, ModelElement::Function(_)));
    assert!(has_fn, "Ожидалась функция");
}

/// Функция с телом и типом возврата.
#[test]
fn parse_function_with_body_and_return() {
    let root = must_parse(r#"
        type u8 = [bit;8];
        fn double(x: u8) -> u8 {
            return x + x;
        }
        model M { start S; }
    "#);
    let fn_def = root.elements.iter().find_map(|e| {
        if let ModelElement::Function(f) = e { Some(f.as_ref()) } else { None }
    });
    assert!(fn_def.is_some());
    let f = fn_def.unwrap();
    assert!(!f.is_void(), "double должна возвращать значение");
    assert!(!f.is_empty(), "double должна иметь тело");
}

// ───────────────────── Тесты оператора `next` ───────────────────────────────

/// `next B` внутри состояния.
#[test]
fn parse_next_in_state() {
    let m = first_named_model(r#"
        model M {
            start A { next B; }
            state B;
        }
    "#);
    if let ModelElement::State(s) = &m.elements[0] {
        let has_next = s.elements.iter().any(|e| {
            matches!(e, StateElement::Next(id) if id.name == "B")
        });
        assert!(has_next, "Ожидался 'next B'");
    }
}

// ─────────────────── Тесты обработки ошибок ─────────────────────────────────

/// Незакрытая фигурная скобка.
#[test]
fn unclosed_brace_produces_error() {
    assert!(parse("model M { start S {", 0).is_err());
}

/// Символ '@' — лексическая ошибка, распространяется в парсер.
#[test]
fn lexical_error_propagates_to_parser() {
    assert!(parse("model M { start @ { } }", 0).is_err());
}

// ──────────────────────── Тесты методов АСД ────────────────────────────────

/// `FunctionDefine::is_void` и `is_empty`.
#[test]
fn function_define_methods() {
    let root = must_parse(r#"
        fn voidFn();
        fn nonVoid() -> bit;
        model M { start S; }
    "#);
    let fns: Vec<_> = root.elements.iter().filter_map(|e| {
        if let ModelElement::Function(f) = e { Some(f.as_ref()) } else { None }
    }).collect();

    assert_eq!(fns.len(), 2);
    assert!(fns[0].is_void(),  "voidFn должна быть void");
    assert!(fns[0].is_empty(), "voidFn должна быть пустой");
    assert!(!fns[1].is_void(), "nonVoid не void");
}

/// `Expression::components` для унарных, бинарных и листовых выражений.
#[test]
fn expression_components() {
    use grammar::ast::Expression;

    let loc = Location::default();
    let var_a = Expression::Variable(Identifier::new("a"));
    let var_b = Expression::Variable(Identifier::new("b"));

    // Унарный (None, Some)
    let not_expr = Expression::Not(loc.clone(), Box::new(var_a.clone()));
    assert!(not_expr.components().0.is_none());
    assert!(not_expr.components().1.is_some());

    // Бинарный (Some, Some)
    let add_expr = Expression::Add(loc.clone(), Box::new(var_a.clone()), Box::new(var_b.clone()));
    assert_eq!(add_expr.components(), (Some(&var_a), Some(&var_b)));

    // Листовой (None, None)
    let num = Expression::Number(loc, 42);
    assert_eq!(num.components(), (None, None));
}

/// `Expression::strip_parentheses` убирает все уровни скобок.
#[test]
fn expression_strip_parentheses() {
    use grammar::ast::Expression;

    let loc = Location::default();
    let inner = Expression::Number(loc.clone(), 7);
    let paren = Expression::Parenthesis(loc.clone(), Box::new(inner.clone()));
    let double_paren = Expression::Parenthesis(loc, Box::new(paren.clone()));

    assert_eq!(paren.strip_parentheses(), &inner);
    assert_eq!(double_paren.strip_parentheses(), &inner);
    assert_eq!(inner.strip_parentheses(), &inner);
}

/// `Expression::is_literal` возвращает `true` только для литеральных выражений.
#[test]
fn expression_is_literal() {
    use grammar::ast::Expression;

    let loc = Location::default();
    assert!(Expression::Number(loc.clone(), 42).is_literal());
    assert!(Expression::Float(loc.clone(), "3.14".into(), false).is_literal());
    // Bool и Variable не считаются литералами в is_literal
    assert!(!Expression::Variable(Identifier::new("x")).is_literal());
}

/// Местоположения в АСД корректно установлены.
#[test]
fn ast_locations_are_set() {
    let root = must_parse("model M { start S; }");
    assert!(
        matches!(root.loc, Location::Source(0, _, _)),
        "Местоположение корневой модели должно быть Source(0,...)"
    );
}

/// `UserDefinedOperator` — методы классификации.
#[test]
fn user_defined_operator_classification() {
    use grammar::ast::UserDefinedOperator::*;

    // Унарные
    assert!(BitwiseNot.is_unary());
    assert!(Negate.is_unary());
    assert_eq!(BitwiseNot.args(), 1);

    // Бинарные
    assert!(Add.is_binary());
    assert_eq!(Add.args(), 2);

    // Побитовые
    assert!(BitwiseAnd.is_bitwise());
    assert!(BitwiseOr.is_bitwise());
    assert!(!Add.is_bitwise());

    // Арифметические
    assert!(Add.is_arithmetic());
    assert!(Multiply.is_arithmetic());
    assert!(!BitwiseAnd.is_arithmetic());

    // Сравнения
    assert!(Equal.is_comparison());
    assert!(Less.is_comparison());
    assert!(!Add.is_comparison());
}

/// `Location` — методы работы с диапазонами.
#[test]
fn location_methods() {
    let loc = Location::Source(0, 10, 20);

    assert_eq!(loc.start(), 10);
    assert_eq!(loc.end(), 20);
    assert_eq!(loc.exclusive_end(), 21);
    assert_eq!(loc.filename(), "0");
    assert_eq!(loc.try_file_no(), Some("0".to_string()));

    let begin = loc.begin_range();
    assert_eq!(begin.start(), 10);
    assert_eq!(begin.end(), 10);

    let end = loc.end_range();
    assert_eq!(end.start(), 20);
    assert_eq!(end.end(), 20);
}

/// `Comment` — методы `is_doc` и `is_line`.
#[test]
fn comment_methods() {
    use grammar::ast::Comment;

    let line = Comment::Line(Location::default(), "// comment".into());
    let doc  = Comment::DocLine(Location::default(), "/// doc".into());

    assert!(!line.is_doc(),  "Line — не документационный");
    assert!(line.is_line(),  "Line — строчный");
    assert!(doc.is_doc(),    "DocLine — документационный");
    assert!(doc.is_line(),   "DocLine — тоже строчный");
    assert_eq!(line.value(), "// comment");
    assert_eq!(doc.value(),  "/// doc");
}
