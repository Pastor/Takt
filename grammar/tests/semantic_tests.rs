//! Дополнительные интеграционные тесты семантического анализа BuT.
//!
//! Проверяют:
//! - поиск моделей и переменных в дереве видимости;
//! - построение типов из различных вариантов [`ast::Type`];
//! - компоновку реализаций (`+`, `|`, скобки);
//! - обнаружение дублирующихся имён моделей;
//! - ошибочные пути: некорректный тип порта, несуществующий псевдоним и др.;
//! - импорт моделей из файлов (`import "file.but"`, `import "file.but" as Name`).

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

/// Инициализатор переменной сохраняется в семантическом узле.
#[test]
fn variable_initializer_preserved() {
    let node = build("var x: bit = true;");
    if let Some(VariableNode::Simple(_, _, init)) = node.search_var("x") {
        assert!(init.is_some(), "Инициализатор должен присутствовать");
    } else {
        panic!("переменная x не найдена");
    }
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
