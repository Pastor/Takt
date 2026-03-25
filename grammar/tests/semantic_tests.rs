//! Дополнительные интеграционные тесты семантического анализа BuT.
//!
//! Проверяют:
//! - поиск моделей и переменных в дереве видимости;
//! - построение типов из различных вариантов [`ast::Type`];
//! - компоновку реализаций (`+`, `|`, скобки);
//! - обнаружение дублирующихся имён моделей;
//! - ошибочные пути: некорректный тип порта, несуществующий псевдоним и др.;
//! - импорт моделей из файлов (`import "file.but"`, `import "file.but" as Name`);
//! - файлы-примеры из `tests/data/sematic/`.

use grammar::parse;
use grammar::semantic::tree::construct_model;
use grammar::semantic::{Implement, StateNode, TypeNode, VariableNode};

// ─── Вспомогательная функция ──────────────────────────────────────────────────

/// Разбирает BuT-программу и возвращает корневой [`ModelNode`].
fn build(src: &str) -> grammar::semantic::ModelNode {
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    construct_model(&ast, None, &[])
        .expect("ошибка построения семантического дерева")
        .take()
}

/// Разбирает BuT-программу и ожидает ошибку семантического анализа.
fn build_err(src: &str) -> grammar::diagnostics::Diagnostic {
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    construct_model(&ast, None, &[]).expect_err("ожидалась ошибка")
}

// ─── Тесты search_model ───────────────────────────────────────────────────────

/// `search_model` находит вложенную модель по имени.
#[test]
fn search_model_finds_nested_model() {
    let (ast, _) = parse("model Inner { start S; }", 0).unwrap();
    let root = construct_model(&ast, None, &[]).unwrap();
    assert!(
        root.borrow().search_model("Inner").is_some(),
        "Inner должна быть найдена в корневом контексте"
    );
}

/// `search_model` возвращает `None` для несуществующей модели.
#[test]
fn search_model_returns_none_for_unknown() {
    let (ast, _) = parse("model Inner { start S; }", 0).unwrap();
    let root = construct_model(&ast, None, &[]).unwrap();
    assert!(
        root.borrow().search_model("Ghost").is_none(),
        "Несуществующая модель должна давать None"
    );
}

/// `search_model` поднимается по цепочке `upper` до родителя.
#[test]
fn search_model_walks_upper_chain() {
    let (ast, _) = parse(
        "model Outer { model Inner { start S; } start A = Inner; }",
        0,
    )
    .unwrap();
    let root = construct_model(&ast, None, &[]).unwrap();
    let outer = root.borrow().search_model("Outer").unwrap();
    // Inner вложена в Outer — можно найти из Outer
    assert!(
        outer.borrow().search_model("Inner").is_some(),
        "Inner должна быть найдена изнутри Outer"
    );
}

// ─── Тесты search_var ────────────────────────────────────────────────────────

/// `search_var` находит переменную на верхнем уровне.
#[test]
fn search_var_finds_global_variable() {
    let node = build("var x: bit = false;");
    assert!(
        node.search_var("x").is_some(),
        "Переменная x должна быть найдена"
    );
}

/// `search_var` возвращает `None` для необъявленной переменной.
#[test]
fn search_var_returns_none_for_unknown() {
    let node = build("var x: bit = false;");
    assert!(
        node.search_var("y").is_none(),
        "Необъявленная переменная y должна давать None"
    );
}

/// `search_var` находит константу.
#[test]
fn search_var_finds_const() {
    let node = build("type u8 = [bit;8]; const C: u8 = 0xFF;");
    assert!(
        node.search_var("C").is_some(),
        "Константа C должна быть найдена"
    );
    assert!(
        matches!(node.search_var("C").unwrap(), VariableNode::Const(..)),
        "C должна быть VariableNode::Const"
    );
}

/// `search_var` находит порт.
#[test]
fn search_var_finds_port() {
    let node = build("type u8 = [bit;8]; port P: u8 = 0x00100000;");
    assert!(node.search_var("P").is_some(), "Порт P должен быть найден");
    assert!(
        matches!(node.search_var("P").unwrap(), VariableNode::Port(..)),
        "P должна быть VariableNode::Port"
    );
}

// ─── Тесты construct_type ────────────────────────────────────────────────────

/// `bit` разрешается в `TypeNode::Bit`.
#[test]
fn type_bit_resolves_to_type_node_bit() {
    let node = build("var x: bit = false;");
    if let Some(VariableNode::Simple(_, ty, _)) = node.search_var("x") {
        assert_eq!(ty, TypeNode::Bit, "bit должен разрешаться в TypeNode::Bit");
    } else {
        panic!("переменная x не найдена или не является Simple");
    }
}

/// `[bit;8]` разрешается в `TypeNode::Array(8, Box<TypeNode::Bit>)`.
#[test]
fn type_array_resolves_correctly() {
    let node = build("var x: [bit;8] = 0;");
    if let Some(VariableNode::Simple(_, ty, _)) = node.search_var("x") {
        assert_eq!(
            ty,
            TypeNode::Array(8, Box::new(TypeNode::Bit)),
            "Массив должен разрешаться в Array(8, Bit)"
        );
    } else {
        panic!("переменная x не найдена");
    }
}

/// Псевдоним типа `u8 = [bit;8]` раскрывается в `TypeNode::Array`.
#[test]
fn type_alias_resolves_through_map() {
    let node = build("type u8 = [bit;8]; var x: u8 = 0;");
    if let Some(VariableNode::Simple(_, ty, _)) = node.search_var("x") {
        assert_eq!(
            ty,
            TypeNode::Array(8, Box::new(TypeNode::Bit)),
            "Псевдоним u8 должен раскрыться в Array(8, Bit)"
        );
    } else {
        panic!("переменная x не найдена");
    }
}

/// Встроенный псевдоним `bool` разрешается в `TypeNode::Bit`.
#[test]
fn type_alias_bool_resolves_to_bit() {
    let node = build("var flag: bool = false;");
    if let Some(VariableNode::Simple(_, ty, _)) = node.search_var("flag") {
        assert_eq!(ty, TypeNode::Bit);
    } else {
        panic!("переменная flag не найдена");
    }
}

/// Встроенный псевдоним `float` разрешается в `TypeNode::Rational`.
#[test]
fn type_alias_float_resolves_to_rational() {
    let node = build("var r: float = 0;");
    if let Some(VariableNode::Simple(_, ty, _)) = node.search_var("r") {
        assert_eq!(ty, TypeNode::Rational);
    } else {
        panic!("переменная r не найдена");
    }
}

/// Несуществующий псевдоним типа — ошибка.
#[test]
fn unknown_type_alias_is_error() {
    let (ast, _) = parse("var x: UnknownType = 0;", 0).unwrap();
    let result = construct_model(&ast, None, &[]);
    assert!(
        result.is_err(),
        "Неизвестный псевдоним типа должен давать ошибку"
    );
}

// ─── Тесты construct_implement ───────────────────────────────────────────────

/// Простая реализация (`= M`) разрешается в `Implement::Model`.
#[test]
fn implement_single_model_resolves() {
    let node = build("start A = M { } state B; model M { start S; }");
    if let StateNode::Implement { implements, .. } = &node.states["A"] {
        assert!(
            matches!(implements, Implement::Model(_)),
            "Простая реализация должна разрешаться в Implement::Model"
        );
    } else {
        panic!("ожидался StateNode::Implement");
    }
}

/// Реализация с `+` (последовательная компоновка) разрешается в `Implement::Add`.
#[test]
fn implement_add_composition_resolves() {
    let node = build("start Entry = M1 + M2; model M1 { start S; } model M2 { start T; }");
    if let StateNode::Implement { implements, .. } = &node.states["Entry"] {
        assert!(
            matches!(implements, Implement::Add(_, _)),
            "Компоновка + должна разрешаться в Implement::Add"
        );
    } else {
        panic!("ожидался StateNode::Implement для Entry");
    }
}

/// Реализация с `|` (параллельная компоновка) разрешается в `Implement::Or`.
#[test]
fn implement_or_composition_resolves() {
    let node = build("start Entry = M1 | M2; model M1 { start S; } model M2 { start T; }");
    if let StateNode::Implement { implements, .. } = &node.states["Entry"] {
        assert!(
            matches!(implements, Implement::Or(_, _)),
            "Компоновка | должна разрешаться в Implement::Or"
        );
    } else {
        panic!("ожидался StateNode::Implement для Entry");
    }
}

/// Неизвестная модель в `implements` — ошибка.
#[test]
fn implement_unknown_model_is_error() {
    let (ast, _) = parse("start A = Ghost { }", 0).unwrap();
    let result = construct_model(&ast, None, &[]);
    assert!(
        result.is_err(),
        "Ссылка на несуществующую модель должна давать ошибку"
    );
    let err = result.unwrap_err();
    assert!(
        err.message.contains("Ghost"),
        "Сообщение ошибки должно содержать имя модели: {}",
        err.message
    );
}

// ─── Тесты дублирования имён ─────────────────────────────────────────────────

/// Два состояния с одинаковым именем — второе перезаписывает первое.
/// (Семантика допускает, т.к. HashMap не хранит дубликаты.)
#[test]
fn duplicate_state_names_last_wins() {
    // Парсер принимает — семантика оставляет последнее
    let result = build("start S; state S;");
    assert!(result.states.contains_key("S"));
}

/// Два вложенных `model` с одинаковым именем — ошибка (реализация TODO).
#[test]
fn duplicate_nested_model_name_is_error() {
    let (ast, _) = parse(
        "model Outer { model M { start S; } model M { start T; } start A; }",
        0,
    )
    .unwrap();
    let result = construct_model(&ast, None, &[]);
    assert!(
        result.is_err(),
        "Дублирующееся имя вложенной модели должно давать ошибку"
    );
    let err = result.unwrap_err();
    assert!(
        err.message.contains("M"),
        "Сообщение ошибки должно содержать имя модели: {}",
        err.message
    );
}

// ─── Тесты ошибок портов ──────────────────────────────────────────────────────

/// Порт без явного типа — ошибка.
#[test]
fn port_without_type_is_error() {
    let (ast, _) = parse("port P = 0x00100000;", 0).unwrap();
    let result = construct_model(&ast, None, &[]);
    assert!(result.is_err(), "Порт без типа должен давать ошибку");
}

/// Порт с инициализатором не-адресом — ошибка.
#[test]
fn port_with_non_address_initializer_is_error() {
    let (ast, _) = parse("type u8 = [bit;8]; port P: u8 = true;", 0).unwrap();
    let result = construct_model(&ast, None, &[]);
    assert!(
        result.is_err(),
        "Порт с неверным инициализатором должен давать ошибку"
    );
}

// ─── Тесты корневой модели ───────────────────────────────────────────────────

/// Корневая модель без объявлений пуста и не имеет имени.
#[test]
fn root_model_empty_has_no_name_and_no_states() {
    let node = build("");
    assert_eq!(node.name, None);
    assert!(!node.has_states());
    assert!(node.variables.is_empty());
    assert!(node.models.is_empty());
    assert!(node.types.is_empty());
}

/// Несколько именованных моделей регистрируются в корне.
#[test]
fn multiple_named_models_all_registered() {
    let node = build("model A { start S; } model B { start T; } model C { start U; }");
    assert!(node.search_model("A").is_some(), "A не найдена");
    assert!(node.search_model("B").is_some(), "B не найдена");
    assert!(node.search_model("C").is_some(), "C не найдена");
    assert_eq!(node.models.len(), 3, "ожидались 3 модели");
}

/// Несколько переменных регистрируются в корне.
#[test]
fn multiple_global_variables_all_registered() {
    let node = build("var a: bit = false; var b: bit = true; var c: bit = false;");
    assert!(node.search_var("a").is_some());
    assert!(node.search_var("b").is_some());
    assert!(node.search_var("c").is_some());
}

// ─── Контр-примеры ───────────────────────────────────────────────────────────

/// `ref` к несуществующему состоянию в модели — ошибка.
#[test]
fn ref_to_nonexistent_state_in_model_is_error() {
    let (ast, _) = parse("model M { start A { ref Z; } }", 0).unwrap();
    let result = construct_model(&ast, None, &[]);
    assert!(
        result.is_err(),
        "Ссылка на несуществующее состояние должна давать ошибку"
    );
}

/// Два оператора `next` в одном Implement-состоянии — ошибка.
#[test]
fn two_next_in_same_state_is_error() {
    let (ast, _) = parse(
        "start A = M { next B; next C; } state B; state C; model M { start S; }",
        0,
    )
    .unwrap();
    let result = construct_model(&ast, None, &[]);
    assert!(
        result.is_err(),
        "Два next в одном состоянии должны давать ошибку"
    );
}

/// Тип без переменных не регистрируется в `variables`.
#[test]
fn type_definition_not_in_variables() {
    let node = build("type MyType = bit;");
    assert!(
        node.search_var("MyType").is_none(),
        "Псевдоним типа не должен попадать в переменные"
    );
    assert!(
        node.types.contains_key("MyType"),
        "Псевдоним типа должен быть в types"
    );
}

// ─── Интеграционные тесты импорта ────────────────────────────────────────────

/// Вспомогательная функция: создаёт временную директорию с .but-файлом.
/// Возвращает (TempDir, путь_к_файлу) — TempDir нужно держать живым до конца теста.
fn write_tmp_but(name: &str, content: &str) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join(name);
    std::fs::write(&p, content).unwrap();
    let dir_str = dir.path().to_string_lossy().into_owned();
    (dir, dir_str)
}

/// `import "file.but"` — успешный импорт простой модели из файла.
/// Импортированная модель доступна по нормализованному имени.
#[test]
fn plain_import_registers_model() {
    let (_dir, dir_str) = write_tmp_but("ping.but", "model Ping { start S; }");

    let src = r#"import "ping.but";"#;
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[dir_str])
        .expect("ошибка построения семантики");

    assert!(
        root.borrow().search_model("Ping").is_some(),
        "Модель Ping должна быть импортирована"
    );
}

/// Имя модели из импортированного файла нормализуется в CamelCase:
/// `my_model.but` → `MyModel`.
#[test]
fn plain_import_normalizes_filename_to_camel_case() {
    let (_dir, dir_str) = write_tmp_but("my_model.but", "start S;");

    let src = r#"import "my_model.but";"#;
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[dir_str])
        .expect("ошибка построения семантики");

    assert!(
        root.borrow().search_model("MyModel").is_some(),
        "my_model.but должен регистрироваться как MyModel"
    );
    assert!(
        root.borrow().search_model("my_model").is_none(),
        "имя в snake_case не должно быть зарегистрировано"
    );
}

/// `import "file.but" as Alias` — модель доступна под заданным именем.
#[test]
fn global_symbol_import_registers_under_alias() {
    let (_dir, dir_str) = write_tmp_but("engine.but", "start S;");

    let src = r#"import "engine.but" as Motor;"#;
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[dir_str])
        .expect("ошибка построения семантики");

    assert!(
        root.borrow().search_model("Motor").is_some(),
        "Модель должна быть доступна под именем Motor"
    );
}

/// Дублирующийся `import` одного и того же имени → ошибка.
#[test]
fn duplicate_import_plain_is_error() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("dup.but"), "start S;").unwrap();
    let dir_str = dir.path().to_string_lossy().into_owned();

    // Два одинаковых импорта
    let src = r#"import "dup.but"; import "dup.but";"#;
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let result = construct_model(&ast, None, &[dir_str]);
    assert!(
        result.is_err(),
        "Дублирующийся импорт должен давать ошибку"
    );
    let err = result.unwrap_err();
    assert!(
        err.message.contains("уже объявлена"),
        "Сообщение должно содержать «уже объявлена»: {}",
        err.message
    );
}

/// Файл импорта не найден → ошибка с понятным сообщением.
#[test]
fn import_missing_file_is_error() {
    let src = r#"import "ghost.but";"#;
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let result = construct_model(&ast, None, &["/nonexistent_dir_xyz".to_string()]);
    assert!(result.is_err(), "Импорт несуществующего файла должен давать ошибку");
    let err = result.unwrap_err();
    assert!(
        err.message.contains("не найден"),
        "Сообщение об ошибке должно содержать «не найден»: {}",
        err.message
    );
}

/// Файл импорта содержит синтаксическую ошибку → ошибка при построении семантики.
#[test]
fn import_file_with_parse_error_is_error() {
    let (_dir, dir_str) = write_tmp_but("broken.but", "model {"); // синтаксическая ошибка

    let src = r#"import "broken.but";"#;
    let (ast, _) = parse(src, 0).expect("ошибка разбора основного файла");
    let result = construct_model(&ast, None, &[dir_str]);
    assert!(
        result.is_err(),
        "Импорт файла с ошибкой разбора должен давать ошибку"
    );
}

/// Импортированная модель видна при разрешении `implements` в основном файле.
#[test]
fn imported_model_usable_in_implements() {
    let (_dir, dir_str) = write_tmp_but("worker.but", "model Worker { start S; }");

    let src = r#"
        import "worker.but";
        start Entry = Worker { }
        state Done;
    "#;
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[dir_str])
        .expect("ошибка построения семантики");

    // Entry реализует Worker — должно быть найдено без ошибок
    assert!(root.borrow().states.contains_key("Entry"));
}

/// `import "file.but" as Name` с несуществующим файлом → ошибка.
#[test]
fn global_symbol_import_missing_file_is_error() {
    let src = r#"import "ghost.but" as Ghost;"#;
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let result = construct_model(&ast, None, &["/nonexistent".to_string()]);
    assert!(result.is_err(), "Импорт несуществующего файла должен давать ошибку");
}

/// Имя из импорта через `as` не совпадает с нормализованным именем файла.
/// Проверяем, что старое имя (по имени файла) НЕ регистрируется.
#[test]
fn global_symbol_import_only_alias_registered() {
    let (_dir, dir_str) = write_tmp_but("engine.but", "start S;");

    let src = r#"import "engine.but" as Motor;"#;
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[dir_str])
        .expect("ошибка построения семантики");

    // Только алиас должен быть зарегистрирован
    assert!(root.borrow().search_model("Motor").is_some());
    assert!(
        root.borrow().search_model("Engine").is_none(),
        "Нормализованное имя файла не должно регистрироваться при использовании as"
    );
}

// ─── Тесты search_func и search_cond ─────────────────────────────────────────

/// `search_cond` находит именованное условие по имени.
#[test]
fn search_cond_finds_named_condition() {
    let node = build("cond done = true;");
    assert!(
        node.search_cond("done").is_some(),
        "условие 'done' должно быть найдено"
    );
}

/// `search_cond` возвращает `None` для несуществующего условия.
#[test]
fn search_cond_returns_none_for_unknown() {
    let node = build("cond done = true;");
    assert!(
        node.search_cond("missing").is_none(),
        "несуществующее условие должно давать None"
    );
}

/// `search_func` возвращает `None` когда функций нет.
#[test]
fn search_func_returns_none_when_no_functions() {
    let node = build("var x: bit = false;");
    assert!(
        node.search_func("any_func").is_none(),
        "search_func должен вернуть None, если функций нет"
    );
}

// ─── Тесты construct_implement (исправление переполнения стека) ───────────────

/// Implement-состояние без `next` успешно строится без переполнения стека.
///
/// **Регрессионный тест**: ранее приводил к бесконечной рекурсии в
/// `construct_implement` из-за заглушки `construct_expression`, которая
/// всегда возвращала `Expression::Unresolved`.
#[test]
fn implement_without_next_no_stack_overflow() {
    let node = build("start A = M { } state B; model M { start S; }");
    if let StateNode::Implement { next, implements, .. } = &node.states["A"] {
        assert!(next.is_none(), "next должен быть None");
        assert!(
            matches!(implements, grammar::semantic::Implement::Model(_)),
            "реализация должна разрешиться в Implement::Model"
        );
    } else {
        panic!("ожидался StateNode::Implement для A");
    }
}

/// Скобочная компоновка `(M1 + M2)` разрешается корректно.
///
/// Проверяет ветку `ast::Expression::Parenthesis` в `construct_implement_ast`.
#[test]
fn implement_parenthesized_add_resolves() {
    let node = build("start E = (M1 + M2) { } model M1 { start S; } model M2 { start T; }");
    if let StateNode::Implement { implements, .. } = &node.states["E"] {
        assert!(
            matches!(implements, Implement::Add(_, _)),
            "скобочная компоновка должна давать Implement::Add"
        );
    } else {
        panic!("ожидался StateNode::Implement для E");
    }
}

// ─── Тесты поиска переменных в цепочке upper ─────────────────────────────────

/// Переменная из родительской области видимости видна во вложенной модели.
#[test]
fn nested_model_sees_parent_variable() {
    let (ast, _) = parse(
        "var global_flag: bit = false; model Inner { start S; }",
        0,
    )
    .unwrap();
    let root = construct_model(&ast, None, &[]).unwrap();
    let inner = root.borrow().search_model("Inner").unwrap();
    // Inner должна видеть переменную из родительского контекста через upper
    assert!(
        inner.borrow().search_var("global_flag").is_some(),
        "вложенная модель должна видеть переменную родителя"
    );
}

/// Переменная из вложенной модели недоступна в родительской (область видимости строга).
#[test]
fn parent_does_not_see_nested_variable() {
    let node = build("model Inner { var local: bit = false; start S; } start Root;");
    // Корневая модель не знает о переменной Inner
    assert!(
        node.search_var("local").is_none(),
        "родитель не должен видеть переменные вложенной модели"
    );
}

// ─── Тесты типов: вложенные массивы и массивы массивов ───────────────────────

/// Тип `[[bit;4];2]` разрешается в `Array(2, Array(4, Bit))`.
#[test]
fn type_nested_array_resolves() {
    let node = build("var x: [[bit;4];2] = 0;");
    if let Some(VariableNode::Simple(_, ty, _)) = node.search_var("x") {
        assert_eq!(
            ty,
            TypeNode::Array(2, Box::new(TypeNode::Array(4, Box::new(TypeNode::Bit)))),
            "вложенный массив должен разрешаться рекурсивно"
        );
    } else {
        panic!("переменная x не найдена");
    }
}

/// Псевдоним, используемый внутри типа массива, раскрывается правильно.
#[test]
fn type_alias_inside_array_resolves() {
    // u4 = [bit;4], затем var x: [u4; 3]
    let node = build("type u4 = [bit;4]; var x: [u4;3] = 0;");
    if let Some(VariableNode::Simple(_, ty, _)) = node.search_var("x") {
        assert_eq!(
            ty,
            TypeNode::Array(3, Box::new(TypeNode::Array(4, Box::new(TypeNode::Bit)))),
            "псевдоним внутри массива должен раскрываться"
        );
    } else {
        panic!("переменная x не найдена");
    }
}

// ─── Тесты сообщений об ошибках ───────────────────────────────────────────────

/// Сообщение об ошибке для неизвестной модели содержит её имя.
#[test]
fn error_message_contains_missing_model_name() {
    let err = build_err("start A = Phantom { }");
    assert!(
        err.message.contains("Phantom"),
        "сообщение об ошибке должно содержать 'Phantom': {}",
        err.message
    );
}

/// Сообщение об ошибке для неизвестного псевдонима типа содержит его имя.
#[test]
fn error_message_contains_missing_type_name() {
    let (ast, _) = parse("var x: NoSuchType = 0;", 0).unwrap();
    let err = construct_model(&ast, None, &[]).unwrap_err();
    assert!(
        err.message.contains("NoSuchType"),
        "сообщение должно содержать 'NoSuchType': {}",
        err.message
    );
}

/// Сообщение об ошибке для неизвестной ссылки ref содержит имя состояния.
#[test]
fn error_message_contains_missing_ref_name() {
    let (ast, _) = parse("start A { ref Xyz; }", 0).unwrap();
    let err = construct_model(&ast, None, &[]).unwrap_err();
    assert!(
        err.message.contains("Xyz"),
        "сообщение должно содержать 'Xyz': {}",
        err.message
    );
}

// ─── Тесты файлов-примеров из tests/data/sematic/ ────────────────────────────

/// Вспомогательная функция: читает .but-файл и строит семантическое дерево.
fn build_file(path: &str) -> Result<grammar::semantic::ModelNode, grammar::diagnostics::Diagnostic> {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("не могу прочитать {}: {}", path, e));
    let (ast, _) = parse(&src, 0).expect("ошибка разбора файла");
    construct_model(&ast, None, &[]).map(|m| m.take())
}

/// `tests/data/sematic/valid/simple_fsm.but` — строится без ошибок.
#[test]
fn example_simple_fsm_is_valid() {
    let node = build_file("tests/data/sematic/valid/simple_fsm.but").unwrap();
    assert!(node.has_states(), "FSM должен иметь состояния");
    assert!(node.states.contains_key("Start"), "состояние Start должно присутствовать");
    assert!(node.states.contains_key("Finish"), "состояние Finish должно присутствовать");
}

/// `tests/data/sematic/valid/type_aliases.but` — псевдонимы типов разрешаются.
#[test]
fn example_type_aliases_is_valid() {
    let node = build_file("tests/data/sematic/valid/type_aliases.but").unwrap();
    assert!(node.types.contains_key("u8"), "тип u8 должен быть объявлен");
    assert!(node.types.contains_key("u16"), "тип u16 должен быть объявлен");
    assert!(node.search_var("counter").is_some(), "переменная counter должна быть найдена");
    assert!(node.search_var("STATUS").is_some(), "порт STATUS должен быть найден");
}

/// `tests/data/sematic/valid/conditions.but` — все условия разрешаются.
#[test]
fn example_conditions_is_valid() {
    let node = build_file("tests/data/sematic/valid/conditions.but").unwrap();
    assert!(node.conditions.contains_key("always_true"), "условие always_true должно быть");
    assert!(node.conditions.contains_key("always_false"), "условие always_false должно быть");
    assert!(node.conditions.contains_key("is_flag_set"), "условие is_flag_set должно быть");
    assert!(node.conditions.contains_key("negated"), "условие negated должно быть");
    assert!(node.conditions.contains_key("grouped"), "условие grouped должно быть");
}

/// `tests/data/sematic/valid/composition.but` — компоновка моделей корректна.
#[test]
fn example_composition_is_valid() {
    let node = build_file("tests/data/sematic/valid/composition.but").unwrap();
    // Модели Step1, Step2, Step3 должны быть в контексте
    assert!(node.search_model("Step1").is_some(), "Step1 должна быть найдена");
    assert!(node.search_model("Step2").is_some(), "Step2 должна быть найдена");
    assert!(node.search_model("Step3").is_some(), "Step3 должна быть найдена");
    // Состояния Sequential, Parallel, Combined должны быть Implement-узлами
    assert!(node.states.contains_key("Sequential"), "состояние Sequential должно быть");
    assert!(node.states.contains_key("Parallel"), "состояние Parallel должно быть");
    assert!(node.states.contains_key("Combined"), "состояние Combined должно быть");
}

/// `tests/data/sematic/invalid/missing_var.but` — должна возникнуть ошибка.
#[test]
fn example_missing_var_is_error() {
    let result = build_file("tests/data/sematic/invalid/missing_var.but");
    assert!(result.is_err(), "missing_var.but должен давать ошибку семантики");
}

/// `tests/data/sematic/invalid/unknown_model.but` — должна возникнуть ошибка.
#[test]
fn example_unknown_model_is_error() {
    let result = build_file("tests/data/sematic/invalid/unknown_model.but");
    assert!(result.is_err(), "unknown_model.but должен давать ошибку семантики");
}

/// `tests/data/sematic/invalid/double_next.but` — должна возникнуть ошибка.
#[test]
fn example_double_next_is_error() {
    let result = build_file("tests/data/sematic/invalid/double_next.but");
    assert!(result.is_err(), "double_next.but должен давать ошибку семантики");
}

/// `tests/data/sematic/invalid/dangling_ref.but` — должна возникнуть ошибка.
#[test]
fn example_dangling_ref_is_error() {
    let result = build_file("tests/data/sematic/invalid/dangling_ref.but");
    assert!(result.is_err(), "dangling_ref.but должен давать ошибку семантики");
}

// ─── Тесты импорта std.but ────────────────────────────────────────────────────

/// `import "std.but"` из стандартной библиотеки подключается без ошибок.
#[test]
fn std_but_import_works() {
    let src = r#"import "std.but";"#;
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(
        &ast,
        None,
        &["tests/data/include".to_string()],
    );
    assert!(root.is_ok(), "импорт std.but должен завершаться без ошибок");
    let root = root.unwrap();
    // Нормализованное имя файла std.but → Std
    assert!(
        root.borrow().search_model("Std").is_some(),
        "модель Std должна быть зарегистрирована после импорта std.but"
    );
}

/// После импорта `std.but` типы u8, u16, … доступны внутри импортированной модели.
#[test]
fn std_but_contains_u8_u16_types() {
    let src = r#"import "std.but";"#;
    let (ast, _) = parse(src, 0).unwrap();
    let root = construct_model(&ast, None, &["tests/data/include".to_string()]).unwrap();
    let std_model = root.borrow().search_model("Std").unwrap();
    assert!(
        std_model.borrow().types.contains_key("u8"),
        "std.but должен содержать тип u8"
    );
    assert!(
        std_model.borrow().types.contains_key("u16"),
        "std.but должен содержать тип u16"
    );
    assert!(
        std_model.borrow().types.contains_key("u32"),
        "std.but должен содержать тип u32"
    );
    assert!(
        std_model.borrow().types.contains_key("u64"),
        "std.but должен содержать тип u64"
    );
    assert!(
        std_model.borrow().types.contains_key("u128"),
        "std.but должен содержать тип u128"
    );
    assert!(
        std_model.borrow().types.contains_key("bool"),
        "std.but должен содержать тип bool"
    );
}
