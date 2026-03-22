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
    parse(src, 0)
        .unwrap_or_else(|diagnostics| {
            let msgs: Vec<_> = diagnostics.iter().map(|d| d.message.clone()).collect();
            panic!("Разбор завершился с ошибками: {:?}", msgs);
        })
        .0
}

/// Извлекает первую именованную модель из корневой модели.
fn first_named_model(src: &str) -> grammar::ast::Model {
    let root = must_parse(src);
    root.elements
        .into_iter()
        .find_map(|e| {
            if let ModelElement::Model(m) = e {
                Some(*m)
            } else {
                None
            }
        })
        .expect("В исходном коде должна быть именованная модель")
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
    let model_count = root
        .elements
        .iter()
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
    assert!(
        root.elements.is_empty(),
        "Только комментарии → элементов нет"
    );
    assert_eq!(comments.len(), 3, "Ожидалось 3 комментария");
}

// ─────────────────────────────── Тесты моделей ──────────────────────────────

/// Минимальная модель содержит одно состояние.
#[test]
fn parse_minimal_model() {
    let m = first_named_model("model M { start S; }");
    assert_eq!(m.name.as_ref().map(|id| id.name.as_str()), Some("M"));
    assert_eq!(m.elements.len(), 1, "Модель M должна содержать 1 элемент");
    assert!(
        matches!(m.elements[0], ModelElement::State(_)),
        "Ожидалось состояние"
    );
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
    let has_inner = outer
        .elements
        .iter()
        .any(|e| matches!(e, ModelElement::Model(_)));
    assert!(has_inner, "Ожидалась вложенная модель Inner");
}

/// Модель без имени — ошибка восстановления: `name == None`.
#[test]
fn parse_model_with_missing_name_recovers() {
    let result = parse("model { start S; }", 0);
    match result {
        Ok((root, _)) => {
            // Восстановление: в корне есть элемент-модель с name=None
            let has_anonymous = root
                .elements
                .iter()
                .any(|e| matches!(e, ModelElement::Model(m) if m.name.is_none()));
            assert!(has_anonymous, "Модель без имени должна иметь name=None");
        }
        Err(diagnostics) => {
            assert!(
                !diagnostics.is_empty(),
                "Должна быть хотя бы одна диагностика"
            );
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
        let has_ref = s
            .elements
            .iter()
            .any(|e| matches!(e, StateElement::Reference(_, id, None) if id.name == "E"));
        assert!(has_ref, "Ожидалась 'ref E' без условия");
    }
}

/// `ref E: true` — ссылка с булевым условием.
#[test]
fn parse_ref_with_bool_condition() {
    let m = first_named_model("model M { start S { ref E: true; } state E; }");
    if let ModelElement::State(s) = &m.elements[0] {
        let has_ref = s
            .elements
            .iter()
            .any(|e| matches!(e, StateElement::Reference(_, _, Some(_))));
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
        if let ModelElement::Type(t) = e {
            Some(t.as_ref())
        } else {
            None
        }
    });
    assert!(alias.is_some(), "Ожидался псевдоним типа на верхнем уровне");
    assert_eq!(alias.unwrap().name.name, "Flag");
}

/// `type u8 = [bit;8]`.
#[test]
fn parse_type_alias_array() {
    let root = must_parse("type u8 = [bit;8]; model M { start S; }");
    let alias = root.elements.iter().find_map(|e| {
        if let ModelElement::Type(t) = e {
            Some(t.as_ref())
        } else {
            None
        }
    });
    assert!(alias.is_some());
    assert_eq!(alias.unwrap().name.name, "u8");
    assert!(
        matches!(
            alias.unwrap().ty,
            grammar::ast::Type::Array {
                element_count: 8,
                ..
            }
        ),
        "u8 = [bit;8]"
    );
}

// ─────────────────────────── Тесты переменных ───────────────────────────────

/// `var x: bit = false` на верхнем уровне.
#[test]
fn parse_mutable_variable() {
    let root = must_parse("var x: bit = false; model M { start S; }");
    let var = root.elements.iter().find_map(|e| {
        if let ModelElement::Variable(v) = e {
            Some(v.as_ref())
        } else {
            None
        }
    });
    assert!(var.is_some(), "Ожидалась переменная");
    assert!(var.unwrap().mutability, "var должна быть изменяемой");
}

/// `const MAX: u8 = 255`.
#[test]
fn parse_const_variable() {
    let root = must_parse("type u8 = [bit;8]; const MAX: u8 = 255; model M { start S; }");
    let cst = root.elements.iter().find_map(|e| {
        if let ModelElement::Variable(v) = e {
            Some(v.as_ref())
        } else {
            None
        }
    });
    assert!(cst.is_some(), "Ожидалась константа");
    assert!(!cst.unwrap().mutability, "const не должна быть изменяемой");
}

/// `var toggle = false` внутри состояния (как `StateElement::Variable`).
#[test]
fn parse_variable_inside_state() {
    let m = first_named_model("model M { start S { var toggle = false; } }");
    if let ModelElement::State(s) = &m.elements[0] {
        let has_var = s
            .elements
            .iter()
            .any(|e| matches!(e, StateElement::Variable(_)));
        assert!(has_var, "Ожидалась переменная в состоянии");
    }
}

// ─────────────────────────── Тесты портов ───────────────────────────────────

/// `port A: bit = 0x00548835:4`.
#[test]
fn parse_port_with_address() {
    let root = must_parse("port A: bit = 0x00548835:4; model M { start S; }");
    let port = root.elements.iter().find_map(|e| {
        if let ModelElement::Port(p) = e {
            Some(p.as_ref())
        } else {
            None
        }
    });
    assert!(port.is_some(), "Ожидался порт");
    assert_eq!(
        port.unwrap().name.as_ref().map(|id| id.name.as_str()),
        Some("A")
    );
}

// ───────────────────────── Тесты условий (cond) ─────────────────────────────

/// `cond IsZero = x = 0`.
#[test]
fn parse_condition_equality() {
    let root = must_parse("var x: bit = false; cond IsZero = x = 0; model M { start S; }");
    let cond = root.elements.iter().find_map(|e| {
        if let ModelElement::Condition(c) = e {
            Some(c.as_ref())
        } else {
            None
        }
    });
    assert!(cond.is_some(), "Ожидалось условие");
    assert_eq!(
        cond.unwrap().name.as_ref().map(|id| id.name.as_str()),
        Some("IsZero")
    );
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
    let has_cond = root
        .elements
        .iter()
        .any(|e| matches!(e, ModelElement::Condition(_)));
    assert!(has_cond, "Ожидалось условие Both");
}

// ────────────────────── Тесты именованных блоков ────────────────────────────

/// `enter`, `exit`, `always` — именованные блоки в состоянии.
#[test]
fn parse_named_blocks_in_state() {
    let m = first_named_model(
        r#"
        model M {
            start S {
                enter  { }
                exit   { }
                always { }
            }
        }
    "#,
    );
    if let ModelElement::State(s) = &m.elements[0] {
        let block_names: Vec<&str> = s
            .elements
            .iter()
            .filter_map(|e| {
                if let StateElement::NamedBlockCode(b) = e {
                    b.name.as_ref().map(|id| id.name.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert!(block_names.contains(&"enter"), "Ожидался 'enter'");
        assert!(block_names.contains(&"exit"), "Ожидался 'exit'");
        assert!(block_names.contains(&"always"), "Ожидался 'always'");
    }
}

// ─────────────────── Тесты управляющих конструкций ──────────────────────────
// Примечание: в BuT условие `if` не требует скобок,
// а условия `while`, `for`, `do-while` — требуют.

/// `if cond { }` — без скобок вокруг условия.
#[test]
fn parse_if_without_parens() {
    let m = first_named_model(
        r#"
        model M { start S { always { if true { } } } }
    "#,
    );
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
    let m = first_named_model(
        r#"
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
    "#,
    );
    assert!(!m.elements.is_empty());
}

/// `for (; cond; step) { }` — условие В скобках, init — выражение.
#[test]
fn parse_for_loop_with_parens() {
    let m = first_named_model(
        r#"
        model M {
            var i: [bit;8] = 0;
            start S {
                var i: [bit;8] = 0;
                always {
                    for (i = 0; i < 10; i = i + 1) { }
                }
            }
        }
    "#,
    );
    assert!(!m.elements.is_empty());
}

/// `do { } while (cond);` — условие В скобках.
#[test]
fn parse_do_while_loop_with_parens() {
    let m = first_named_model(
        r#"
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
    "#,
    );
    assert!(!m.elements.is_empty());
}

// ──────────────────────── Тесты выражений ───────────────────────────────────
// Выражения внутри always {} работают только как `expr;` (не var).

/// Арифметические выражения внутри `always`.
#[test]
fn parse_arithmetic_expressions_in_always() {
    let m = first_named_model(
        r#"
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
    "#,
    );
    assert!(!m.elements.is_empty());
}

/// Побитовые операции внутри `always`.
#[test]
fn parse_bitwise_expressions_in_always() {
    let m = first_named_model(
        r#"
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
    "#,
    );
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
    let root = must_parse(
        r#"
        type u8 = [bit;8];
        const VALS: u8 = { 0, 1, 2, 3 };
        model M { start S; }
    "#,
    );
    assert!(!root.elements.is_empty());
}

/// Доступ к биту `.0`, `.7`.
#[test]
fn parse_bit_access() {
    let m = first_named_model(
        r#"
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
    "#,
    );
    assert!(!m.elements.is_empty());
}

/// Приведение типа `as`.
#[test]
fn parse_cast_expression() {
    let m = first_named_model(
        r#"
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
    "#,
    );
    assert!(!m.elements.is_empty());
}

// ────────────────────────── Тесты импортов ──────────────────────────────────

/// `import "path";`.
#[test]
fn parse_import_plain() {
    let root = must_parse(r#"import "std.but"; model M { start S; }"#);
    let has_import = root
        .elements
        .iter()
        .any(|e| matches!(e, ModelElement::Import(_)));
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
    let root = must_parse(
        r#"
        fn log(msg: bit);
        model M { start S; }
    "#,
    );
    let has_fn = root
        .elements
        .iter()
        .any(|e| matches!(e, ModelElement::Function(_)));
    assert!(has_fn, "Ожидалась функция");
}

/// Функция с телом и типом возврата.
#[test]
fn parse_function_with_body_and_return() {
    let root = must_parse(
        r#"
        type u8 = [bit;8];
        fn double(x: u8) -> u8 {
            return x + x;
        }
        model M { start S; }
    "#,
    );
    let fn_def = root.elements.iter().find_map(|e| {
        if let ModelElement::Function(f) = e {
            Some(f.as_ref())
        } else {
            None
        }
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
    let m = first_named_model(
        r#"
        model M {
            start A { next B; }
            state B;
        }
    "#,
    );
    if let ModelElement::State(s) = &m.elements[0] {
        let has_next = s
            .elements
            .iter()
            .any(|e| matches!(e, StateElement::Next(id) if id.name == "B"));
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
    let root = must_parse(
        r#"
        fn voidFn();
        fn nonVoid() -> bit;
        model M { start S; }
    "#,
    );
    let fns: Vec<_> = root
        .elements
        .iter()
        .filter_map(|e| {
            if let ModelElement::Function(f) = e {
                Some(f.as_ref())
            } else {
                None
            }
        })
        .collect();

    assert_eq!(fns.len(), 2);
    assert!(fns[0].is_void(), "voidFn должна быть void");
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
    let add_expr = Expression::Add(
        loc.clone(),
        Box::new(var_a.clone()),
        Box::new(var_b.clone()),
    );
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
    let doc = Comment::DocLine(Location::default(), "/// doc".into());

    assert!(!line.is_doc(), "Line — не документационный");
    assert!(line.is_line(), "Line — строчный");
    assert!(doc.is_doc(), "DocLine — документационный");
    assert!(doc.is_line(), "DocLine — тоже строчный");
    assert_eq!(line.value(), "// comment");
    assert_eq!(doc.value(), "/// doc");
}

// ──────────────────── Тесты extern-функций ──────────────────────────────────

/// `extern fn` разбирается корректно и флаг `external` установлен.
#[test]
fn parse_extern_function() {
    let root = must_parse(
        r#"
        type u8 = [bit;8];
        extern fn debug(msg: u8);
        extern fn reset();
        model M { start S; }
    "#,
    );
    let fns: Vec<_> = root
        .elements
        .iter()
        .filter_map(|e| {
            if let ModelElement::Function(f) = e {
                Some(f.as_ref())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(fns.len(), 2, "Ожидались 2 внешние функции");
    assert!(fns[0].external, "debug должна быть external");
    assert!(fns[1].external, "reset должна быть external");
}

/// Обычная функция (не extern) имеет `external == false`.
#[test]
fn parse_non_extern_function_has_false_external_flag() {
    let root = must_parse(
        r#"
        fn myFn();
        model M { start S; }
    "#,
    );
    let f = root
        .elements
        .iter()
        .find_map(|e| {
            if let ModelElement::Function(f) = e {
                Some(f.as_ref())
            } else {
                None
            }
        })
        .expect("Функция не найдена");
    assert!(!f.external, "Обычная функция не должна быть external");
}

// ────────────────── Тесты блоков assembly и formula ─────────────────────────

/// `assembly { }` внутри `always`.
#[test]
fn parse_assembly_block_in_always() {
    must_parse(
        r#"
        model M {
            start S {
                always {
                    assembly { }
                }
            }
        }
    "#,
    );
}

/// `assembly "dialect" { }` с диалектом.
#[test]
fn parse_assembly_with_dialect() {
    must_parse(
        r#"
        model M {
            start S {
                always {
                    assembly "arm" { }
                }
            }
        }
    "#,
    );
}

/// `formula { }` как элемент модели.
#[test]
fn parse_formula_as_model_element() {
    let root = must_parse(
        r#"
        model M {
            formula { }
            start S;
        }
    "#,
    );
    assert!(!root.elements.is_empty());
}

/// `formula { fn() }` с вызовом функции внутри.
#[test]
fn parse_formula_with_function_call() {
    must_parse(
        r#"
        model M {
            formula {
                init()
                step(1)
            }
            start S;
        }
    "#,
    );
}

// ──────────────────── Тесты управляющих конструкций ─────────────────────────

/// `continue` внутри цикла.
#[test]
fn parse_continue_in_loop() {
    must_parse(
        r#"
        model M {
            var i: [bit;8] = 0;
            start S {
                var i: [bit;8] = 0;
                always {
                    while (i < 10) {
                        i = i + 1;
                        continue;
                    }
                }
            }
        }
    "#,
    );
}

/// `break` внутри цикла.
#[test]
fn parse_break_in_loop() {
    must_parse(
        r#"
        model M {
            var i: [bit;8] = 0;
            start S {
                var i: [bit;8] = 0;
                always {
                    while (i < 10) {
                        break;
                    }
                }
            }
        }
    "#,
    );
}

/// `return` без значения.
#[test]
fn parse_return_without_value() {
    must_parse(
        r#"
        fn voidFn() {
            return;
        }
        model M { start S; }
    "#,
    );
}

/// `return expr` с возвращаемым значением.
#[test]
fn parse_return_with_value() {
    must_parse(
        r#"
        type u8 = [bit;8];
        fn getValue() -> u8 {
            return 42;
        }
        model M { start S; }
    "#,
    );
}

/// Цикл `for` без тела (с точкой с запятой).
#[test]
fn parse_for_loop_without_body() {
    must_parse(
        r#"
        model M {
            var i: [bit;8] = 0;
            start S {
                var i: [bit;8] = 0;
                always {
                    for (i = 0; i < 10; i = i + 1);
                }
            }
        }
    "#,
    );
}

/// `if` без `else` внутри `for` (open statement).
#[test]
fn parse_if_open_statement_in_for() {
    must_parse(
        r#"
        model M {
            var i: [bit;8] = 0;
            var x: [bit;8] = 0;
            start S {
                var i: [bit;8] = 0;
                var x: [bit;8] = 0;
                always {
                    for (i = 0; i < 10; i = i + 1)
                        if x > 5 { x = 0; }
                }
            }
        }
    "#,
    );
}

// ──────────────────────── Тесты импортов ─────────────────────────────────────

/// `import * as X from "path"` — глобальный импорт.
#[test]
fn parse_import_global_symbol() {
    let root = must_parse(r#"import * as Lib from "lib.but"; model M { start S; }"#);
    let has_import = root
        .elements
        .iter()
        .any(|e| matches!(e, ModelElement::Import(_)));
    assert!(has_import, "Ожидался импорт");
}

/// `import { a, b as c } from "path"` — импорт с переименованием.
#[test]
fn parse_import_rename() {
    let root = must_parse(
        r#"import { MyModel, OldName as NewName } from "path.but"; model M { start S; }"#,
    );
    let has_import = root
        .elements
        .iter()
        .any(|e| matches!(e, ModelElement::Import(_)));
    assert!(has_import, "Ожидался импорт с переименованием");
}

// ───────────────────── Тесты выражений (срезы массивов) ─────────────────────

/// `arr[0:3]` — срез массива.
#[test]
fn parse_array_slice_expression() {
    must_parse(
        r#"
        type u8 = [bit;8];
        var arr: u8 = 0;
        model M {
            var arr: u8 = 0;
            start S {
                var arr: u8 = 0;
                always {
                    arr[0:3] = 0;
                }
            }
        }
    "#,
    );
}

// ────────────────────── Тесты методов Location ───────────────────────────────

/// Методы изменения Location: `use_start_from`, `use_end_from`.
#[test]
fn location_mutation_methods() {
    let mut loc = Location::Source(0, 5, 15);
    let other = Location::Source(0, 1, 20);

    loc.use_start_from(&other);
    assert_eq!(
        loc.start(),
        1,
        "use_start_from должен установить начало из other"
    );
    assert_eq!(loc.end(), 15, "end не должен меняться");

    loc.use_end_from(&other);
    assert_eq!(
        loc.end(),
        20,
        "use_end_from должен установить конец из other"
    );
    assert_eq!(loc.start(), 1, "start не должен меняться");
}

/// Методы `with_start_from` и `with_end_from` возвращают копию.
#[test]
fn location_with_start_end_from() {
    let loc = Location::Source(0, 5, 15);
    let other = Location::Source(0, 1, 20);

    let new_loc = loc.clone().with_start_from(&other);
    assert_eq!(new_loc.start(), 1);
    assert_eq!(new_loc.end(), 15);

    let new_loc2 = loc.clone().with_end_from(&other);
    assert_eq!(new_loc2.start(), 5);
    assert_eq!(new_loc2.end(), 20);
}

/// Методы `with_start` и `with_end` возвращают копию с заменённой позицией.
#[test]
fn location_with_start_and_with_end() {
    let loc = Location::Source(0, 5, 15);

    let new_loc = loc.clone().with_start(0);
    assert_eq!(new_loc.start(), 0);
    assert_eq!(new_loc.end(), 15);

    let new_loc2 = loc.with_end(100);
    assert_eq!(new_loc2.start(), 5);
    assert_eq!(new_loc2.end(), 100);
}

/// Метод `range` возвращает диапазон `start..end`.
#[test]
fn location_range_method() {
    let loc = Location::Source(0, 3, 10);
    let range = loc.range();
    assert_eq!(range, 3..10);
}

/// Не-Source варианты Location: `try_file_no` возвращает `None`,
/// `begin_range` и `end_range` возвращают тот же вариант.
#[test]
fn location_non_source_variants() {
    let variants = [
        Location::Builtin,
        Location::CommandLine,
        Location::Implicit,
        Location::Codegen,
    ];
    for loc in &variants {
        assert_eq!(
            loc.try_file_no(),
            None,
            "{:?}.try_file_no() должен быть None",
            loc
        );
        // begin_range и end_range для не-Source возвращают тот же вариант
        assert_eq!(loc.begin_range(), loc.clone());
        assert_eq!(loc.end_range(), loc.clone());
    }
}

// ──────────────────── Тесты Statement::is_empty ──────────────────────────────

/// `Statement::is_empty` для пустого блока — `true`.
#[test]
fn statement_is_empty_for_empty_block() {
    use grammar::ast::Statement;

    let empty_block = Statement::Block {
        loc: Location::default(),
        unchecked: false,
        statements: Vec::new(),
    };
    assert!(empty_block.is_empty(), "Пустой блок должен быть пустым");
}

/// `Statement::is_empty` для непустого блока — `false`.
#[test]
fn statement_is_empty_for_nonempty_block() {
    use grammar::ast::Statement;

    let inner = Statement::Continue(Location::default());
    let block = Statement::Block {
        loc: Location::default(),
        unchecked: false,
        statements: vec![inner],
    };
    assert!(!block.is_empty(), "Непустой блок не должен быть пустым");
}

/// `Statement::is_empty` для не-блочных операторов — `false`.
#[test]
fn statement_is_empty_for_non_block_statements() {
    use grammar::ast::Statement;

    let stmts = vec![
        Statement::Continue(Location::default()),
        Statement::Break(Location::default()),
        Statement::Return(Location::default(), None),
        Statement::Error(Location::default()),
    ];
    for stmt in &stmts {
        assert!(
            !stmt.is_empty(),
            "Не-блочный оператор {:?} не должен быть пустым",
            stmt
        );
    }
}

// ──────────────────────── Тесты Diagnostic ───────────────────────────────────

/// Конструкторы `Diagnostic` создают объекты с нужными полями.
#[test]
fn diagnostic_constructors() {
    use grammar::diagnostics::{Diagnostic, ErrorType, Level, Note};

    let loc = Location::Source(0, 0, 5);

    // debug
    let d = Diagnostic::debug(loc.clone(), "msg".into());
    assert_eq!(d.level, Level::Debug);
    assert_eq!(d.ty, ErrorType::None);
    assert!(d.notes.is_empty());

    // info
    let d = Diagnostic::info(loc.clone(), "msg".into());
    assert_eq!(d.level, Level::Info);

    // parser_error
    let d = Diagnostic::parser_error(loc.clone(), "parse err".into());
    assert_eq!(d.level, Level::Error);
    assert_eq!(d.ty, ErrorType::ParserError);

    // error
    let d = Diagnostic::error(loc.clone(), "syntax err".into());
    assert_eq!(d.ty, ErrorType::SyntaxError);

    // declaration_error
    let d = Diagnostic::declaration_error(loc.clone(), "decl err".into());
    assert_eq!(d.ty, ErrorType::DeclarationError);

    // cast_error
    let d = Diagnostic::cast_error(loc.clone(), "cast err".into());
    assert_eq!(d.ty, ErrorType::CastError);
    assert_eq!(d.level, Level::Error);

    // type_error
    let d = Diagnostic::type_error(loc.clone(), "type err".into());
    assert_eq!(d.ty, ErrorType::TypeError);

    // warning
    let d = Diagnostic::warning(loc.clone(), "warn".into());
    assert_eq!(d.level, Level::Warning);
    assert_eq!(d.ty, ErrorType::Warning);

    // cast_warning
    let d = Diagnostic::cast_warning(loc.clone(), "cast warn".into());
    assert_eq!(d.level, Level::Warning);
    assert_eq!(d.ty, ErrorType::CastError);

    // cast_error_with_note
    let d = Diagnostic::cast_error_with_note(
        loc.clone(),
        "cast err".into(),
        loc.clone(),
        "note".into(),
    );
    assert_eq!(d.notes.len(), 1);
    assert_eq!(d.notes[0].message, "note");

    // error_with_note
    let d = Diagnostic::error_with_note(loc.clone(), "err".into(), loc.clone(), "note here".into());
    assert_eq!(d.notes.len(), 1);
    assert_eq!(d.level, Level::Error);

    // error_with_notes
    let notes = vec![
        Note {
            loc: loc.clone(),
            message: "n1".into(),
        },
        Note {
            loc: loc.clone(),
            message: "n2".into(),
        },
    ];
    let d = Diagnostic::error_with_notes(loc.clone(), "err".into(), notes);
    assert_eq!(d.notes.len(), 2);

    // warning_with_note
    let d = Diagnostic::warning_with_note(loc.clone(), "warn".into(), loc.clone(), "note".into());
    assert_eq!(d.level, Level::Warning);
    assert_eq!(d.notes.len(), 1);

    // warning_with_notes
    let notes2 = vec![Note {
        loc: loc.clone(),
        message: "wn".into(),
    }];
    let d = Diagnostic::warning_with_notes(loc.clone(), "warn".into(), notes2);
    assert_eq!(d.notes.len(), 1);
}

/// `Level` — методы `as_str` и Display.
#[test]
fn level_display_and_as_str() {
    use grammar::diagnostics::Level;

    assert_eq!(Level::Debug.as_str(), "debug");
    assert_eq!(Level::Info.as_str(), "info");
    assert_eq!(Level::Warning.as_str(), "warning");
    assert_eq!(Level::Error.as_str(), "error");

    assert_eq!(Level::Debug.to_string(), "debug");
    assert_eq!(Level::Error.to_string(), "error");
}

// ─────────────────── Тесты дополнительных конструкций ────────────────────────

/// Оператор `next` с переходом к состоянию.
#[test]
fn parse_next_operator_in_composition() {
    must_parse(
        r#"
        model A { start S; state E; }
        model B { start S; state E; }
        start Main = A + B;
    "#,
    );
}

/// Параллельная компоновка моделей.
#[test]
fn parse_parallel_composition() {
    must_parse(
        r#"
        model A { start S; state E; }
        model B { start S; state E; }
        start Parallel = A | B;
    "#,
    );
}

/// Условный оператор: `if` вложенный в `else`.
#[test]
fn parse_nested_if_else() {
    must_parse(
        r#"
        model M {
            var x: [bit;8] = 0;
            start S {
                var x: [bit;8] = 0;
                always {
                    if x > 10 {
                        x = 10;
                    } else if x < 0 {
                        x = 0;
                    } else {
                        x = x + 1;
                    }
                }
            }
        }
    "#,
    );
}

/// `ref E: S(Model) = State` — ссылка с путём к состоянию другой модели.
#[test]
fn parse_ref_with_state_function_condition() {
    must_parse(
        r#"
        model Ping { start S; state End; }
        model M {
            start S {
                ref E: S(Ping) = End;
            }
            state E;
        }
    "#,
    );
}

/// Отрицательное числовое значение как инициализатор константы.
#[test]
fn parse_negative_number_as_initializer() {
    must_parse(
        r#"
        type u8 = [bit;8];
        const NEG: u8 = -1;
        model M { start S; }
    "#,
    );
}

/// `Expression::has_space_around` для унарных операторов.
#[test]
fn expression_has_space_around() {
    use grammar::ast::Expression;

    let loc = Location::default();
    let inner = Expression::Number(loc.clone(), 1);

    // Унарные — без пробелов
    assert!(!Expression::Not(loc.clone(), Box::new(inner.clone())).has_space_around());
    assert!(!Expression::BitwiseNot(loc.clone(), Box::new(inner.clone())).has_space_around());
    assert!(!Expression::UnaryPlus(loc.clone(), Box::new(inner.clone())).has_space_around());
    assert!(!Expression::Negate(loc.clone(), Box::new(inner.clone())).has_space_around());

    // Бинарные — с пробелами
    assert!(
        Expression::Add(
            loc.clone(),
            Box::new(inner.clone()),
            Box::new(inner.clone())
        )
        .has_space_around()
    );
    assert!(Expression::Number(loc.clone(), 42).has_space_around());
}

/// `Expression::loc()` возвращает корректное местоположение для разных вариантов.
#[test]
fn expression_loc_method() {
    use grammar::ast::Expression;

    let loc = Location::Source(0, 5, 10);
    let inner = Box::new(Expression::Number(loc.clone(), 0));

    // Проверяем несколько вариантов
    assert_eq!(Expression::Number(loc.clone(), 42).loc(), loc);
    assert_eq!(Expression::Bool(loc.clone(), true).loc(), loc);
    assert_eq!(Expression::Not(loc.clone(), inner.clone()).loc(), loc);
    assert_eq!(
        Expression::Add(loc.clone(), inner.clone(), inner.clone()).loc(),
        loc
    );
    assert_eq!(
        Expression::Assign(loc.clone(), inner.clone(), inner.clone()).loc(),
        loc
    );
}
