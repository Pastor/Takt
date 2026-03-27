//! Дополнительные интеграционные тесты семантического анализа BuT.
//!
//! Проверяют:
//! - поиск моделей и переменных в дереве видимости;
//! - построение типов из различных вариантов [`ast::Type`];
//! - компоновку реализаций (`+`, `|`, скобки);
//! - обнаружение дублирующихся имён моделей;
//! - ошибочные пути: некорректный тип порта, несуществующий псевдоним и др.;
//! - импорт моделей из файлов (`import "file.but"`, `import "file.but" as Name`);
//! - файлы-примеры из `tests/data/semantic/`.

use grammar::parse;
use grammar::semantic::tree::{construct_model, construct_model_with_docs, implicit_bool_warnings};
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
        matches!(node.search_var("C").unwrap(), VariableNode::Const { .. }),
        "C должна быть VariableNode::Const"
    );
}

/// `search_var` находит порт.
#[test]
fn search_var_finds_port() {
    let node = build("type u8 = [bit;8]; port P: u8 = 0x00100000;");
    assert!(node.search_var("P").is_some(), "Порт P должен быть найден");
    assert!(
        matches!(node.search_var("P").unwrap(), VariableNode::Port { .. }),
        "P должна быть VariableNode::Port"
    );
}

// ─── Тесты construct_type ────────────────────────────────────────────────────

/// `bit` разрешается в `TypeNode::Bit`.
#[test]
fn type_bit_resolves_to_type_node_bit() {
    let node = build("var x: bit = false;");
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("x") {
        assert_eq!(ty, TypeNode::Bit, "bit должен разрешаться в TypeNode::Bit");
    } else {
        panic!("переменная x не найдена или не является Simple");
    }
}

/// `[bit;8]` разрешается в `TypeNode::Array(8, Box<TypeNode::Bit>)`.
#[test]
fn type_array_resolves_correctly() {
    let node = build("var x: [bit;8] = 0;");
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("x") {
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
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("x") {
        assert_eq!(
            ty,
            TypeNode::Array(8, Box::new(TypeNode::Bit)),
            "Псевдоним u8 должен раскрыться в Array(8, Bit)"
        );
    } else {
        panic!("переменная x не найдена");
    }
}

/// Встроенный псевдоним `bool` разрешается в `TypeNode::Bool`.
#[test]
fn type_alias_bool_resolves_to_bit() {
    let node = build("var flag: bool = false;");
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("flag") {
        assert_eq!(ty, TypeNode::Bool);
    } else {
        panic!("переменная flag не найдена");
    }
}

/// Встроенный псевдоним `float` разрешается в `TypeNode::Rational`.
#[test]
fn type_alias_float_resolves_to_rational() {
    let node = build("var r: float = 0;");
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("r") {
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

/// Два состояния с одинаковым именем: второе (`state S`) перезаписывает
/// первое (`start S`) в HashMap, после чего модель остаётся без start-состояния.
/// Это корректно обнаруживается валидатором как ошибка.
///
/// # Контрпример (BuT)
/// ```but
/// start S;   // добавляется как Start
/// state S;   // перезаписывает — Start исчезает → ошибка валидации
/// ```
#[test]
fn duplicate_state_names_overwrite_causes_validation_error() {
    let (ast, _) = parse("start S; state S;", 0).unwrap();
    let result = construct_model(&ast, None, &[]);
    assert!(
        result.is_err(),
        "дублирующееся имя состояния удаляет start — должна быть ошибка валидации"
    );
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
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("x") {
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
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("x") {
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

// ─── Тесты файлов-примеров из tests/data/semantic/ ────────────────────────────

/// Вспомогательная функция: читает .but-файл и строит семантическое дерево.
fn build_file(path: &str) -> Result<grammar::semantic::ModelNode, grammar::diagnostics::Diagnostic> {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("не могу прочитать {}: {}", path, e));
    let (ast, _) = parse(&src, 0).expect("ошибка разбора файла");
    construct_model(&ast, None, &[]).map(|m| m.take())
}

/// `tests/data/semantic/valid/simple_fsm.but` — строится без ошибок.
#[test]
fn example_simple_fsm_is_valid() {
    let node = build_file("tests/data/semantic/valid/simple_fsm.but").unwrap();
    assert!(node.has_states(), "FSM должен иметь состояния");
    assert!(node.states.contains_key("Start"), "состояние Start должно присутствовать");
    assert!(node.states.contains_key("Finish"), "состояние Finish должно присутствовать");
}

/// `tests/data/semantic/valid/type_aliases.but` — псевдонимы типов разрешаются.
#[test]
fn example_type_aliases_is_valid() {
    let node = build_file("tests/data/semantic/valid/type_aliases.but").unwrap();
    assert!(node.types.contains_key("u8"), "тип u8 должен быть объявлен");
    assert!(node.types.contains_key("u16"), "тип u16 должен быть объявлен");
    assert!(node.search_var("counter").is_some(), "переменная counter должна быть найдена");
    assert!(node.search_var("STATUS").is_some(), "порт STATUS должен быть найден");
}

/// `tests/data/semantic/valid/conditions.but` — все условия разрешаются.
#[test]
fn example_conditions_is_valid() {
    let node = build_file("tests/data/semantic/valid/conditions.but").unwrap();
    assert!(node.conditions.contains_key("always_true"), "условие always_true должно быть");
    assert!(node.conditions.contains_key("always_false"), "условие always_false должно быть");
    assert!(node.conditions.contains_key("is_flag_set"), "условие is_flag_set должно быть");
    assert!(node.conditions.contains_key("negated"), "условие negated должно быть");
    assert!(node.conditions.contains_key("grouped"), "условие grouped должно быть");
}

/// `tests/data/semantic/valid/composition.but` — компоновка моделей корректна.
#[test]
fn example_composition_is_valid() {
    let node = build_file("tests/data/semantic/valid/composition.but").unwrap();
    // Модели Step1, Step2, Step3 должны быть в контексте
    assert!(node.search_model("Step1").is_some(), "Step1 должна быть найдена");
    assert!(node.search_model("Step2").is_some(), "Step2 должна быть найдена");
    assert!(node.search_model("Step3").is_some(), "Step3 должна быть найдена");
    // Состояния Sequential, Parallel, Combined должны быть Implement-узлами
    assert!(node.states.contains_key("Sequential"), "состояние Sequential должно быть");
    assert!(node.states.contains_key("Parallel"), "состояние Parallel должно быть");
    assert!(node.states.contains_key("Combined"), "состояние Combined должно быть");
}

/// `tests/data/semantic/invalid/missing_var.but` — должна возникнуть ошибка.
#[test]
fn example_missing_var_is_error() {
    let result = build_file("tests/data/semantic/invalid/missing_var.but");
    assert!(result.is_err(), "missing_var.but должен давать ошибку семантики");
}

/// `tests/data/semantic/invalid/unknown_model.but` — должна возникнуть ошибка.
#[test]
fn example_unknown_model_is_error() {
    let result = build_file("tests/data/semantic/invalid/unknown_model.but");
    assert!(result.is_err(), "unknown_model.but должен давать ошибку семантики");
}

/// `tests/data/semantic/invalid/double_next.but` — должна возникнуть ошибка.
#[test]
fn example_double_next_is_error() {
    let result = build_file("tests/data/semantic/invalid/double_next.but");
    assert!(result.is_err(), "double_next.but должен давать ошибку семантики");
}

/// `tests/data/semantic/invalid/dangling_ref.but` — должна возникнуть ошибка.
#[test]
fn example_dangling_ref_is_error() {
    let result = build_file("tests/data/semantic/invalid/dangling_ref.but");
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

// ─── Тесты выборочного импорта (ImportDefine::Rename) ────────────────────────

/// Вспомогательная функция: строит модель из inline-кода с путём поиска shared.but.
fn build_with_includes(src: &str) -> Result<grammar::semantic::ModelNode, grammar::diagnostics::Diagnostic> {
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    construct_model(&ast, None, &["tests/data/include".to_string()]).map(|m| m.take())
}

/// `import { SharedModel } from "shared.but"` — модель доступна под оригинальным именем.
#[test]
fn rename_import_model_no_alias() {
    let node = build_with_includes(
        r#"import { SharedModel } from "shared.but"; start E = SharedModel { }"#,
    ).unwrap();
    assert!(
        node.search_model("SharedModel").is_some(),
        "SharedModel должна быть доступна после импорта"
    );
}

/// `import { SharedModel as M } from "shared.but"` — модель доступна под псевдонимом M.
#[test]
fn rename_import_model_with_alias() {
    let node = build_with_includes(
        r#"import { SharedModel as M } from "shared.but"; start E = M { }"#,
    ).unwrap();
    assert!(
        node.search_model("M").is_some(),
        "модель должна быть доступна под псевдонимом M"
    );
    assert!(
        node.search_model("SharedModel").is_none(),
        "оригинальное имя SharedModel не должно быть видно"
    );
}

/// `import { SharedType } from "shared.but"` — тип-псевдоним импортируется в контекст.
#[test]
fn rename_import_type() {
    let node = build_with_includes(
        r#"import { SharedType } from "shared.but"; var x: SharedType = 0; start S;"#,
    ).unwrap();
    assert!(
        node.types.contains_key("SharedType"),
        "тип SharedType должен быть в контексте после импорта"
    );
    assert!(
        node.search_var("x").is_some(),
        "переменная x должна быть объявлена"
    );
}

/// `import { SharedType as ST }` — тип импортируется под псевдонимом.
#[test]
fn rename_import_type_with_alias() {
    let node = build_with_includes(
        r#"import { SharedType as ST } from "shared.but"; var x: ST = 0; start S;"#,
    ).unwrap();
    assert!(
        node.types.contains_key("ST"),
        "псевдоним ST должен быть в контексте"
    );
    assert!(
        !node.types.contains_key("SharedType"),
        "оригинальное имя SharedType не должно быть видно"
    );
}

/// `import { shared_var }` — переменная импортируется в контекст.
#[test]
fn rename_import_variable() {
    let node = build_with_includes(
        r#"import { shared_var } from "shared.but"; start S;"#,
    ).unwrap();
    assert!(
        node.search_var("shared_var").is_some(),
        "переменная shared_var должна быть в контексте после импорта"
    );
}

/// `import { shared_var as sv }` — переменная импортируется под псевдонимом.
#[test]
fn rename_import_variable_with_alias() {
    let node = build_with_includes(
        r#"import { shared_var as sv } from "shared.but"; start S;"#,
    ).unwrap();
    assert!(
        node.search_var("sv").is_some(),
        "переменная должна быть видна под псевдонимом sv"
    );
    assert!(
        node.search_var("shared_var").is_none(),
        "оригинальное имя shared_var не должно быть видно"
    );
}

/// `import { shared_cond }` — условие импортируется в контекст.
#[test]
fn rename_import_condition() {
    let node = build_with_includes(
        r#"import { shared_cond } from "shared.but"; start S { ref E: shared_cond; } state E;"#,
    ).unwrap();
    assert!(
        node.conditions.contains_key("shared_cond"),
        "условие shared_cond должно быть в контексте"
    );
}

/// Импорт нескольких символов в одном выражении.
#[test]
fn rename_import_multiple_symbols() {
    let node = build_with_includes(
        r#"import { SharedModel as M, SharedType as ST, shared_var as sv } from "shared.but"; start E = M { }"#,
    ).unwrap();
    assert!(node.search_model("M").is_some(), "M должна быть видна");
    assert!(node.types.contains_key("ST"), "ST должен быть виден");
    assert!(node.search_var("sv").is_some(), "sv должна быть видна");
}

/// Импорт несуществующего символа — ошибка.
#[test]
fn rename_import_missing_symbol_is_error() {
    let result = build_with_includes(
        r#"import { NonExistent } from "shared.but"; start S;"#,
    );
    assert!(result.is_err(), "импорт несуществующего символа должен давать ошибку");
    let err = result.unwrap_err();
    assert!(
        err.message.contains("NonExistent"),
        "сообщение должно содержать имя символа: {}",
        err.message
    );
}

/// Дублирующееся имя при импорте с псевдонимом — ошибка.
#[test]
fn rename_import_duplicate_alias_is_error() {
    // Объявляем модель M локально, затем пробуем импортировать SharedModel as M
    let result = build_with_includes(
        r#"model M { start S; } import { SharedModel as M } from "shared.but"; start E = M { }"#,
    );
    assert!(result.is_err(), "дублирующееся имя M должно давать ошибку");
}

/// `example_rename_import.but` — файл-пример строится без ошибок.
#[test]
fn example_rename_import_is_valid() {
    let src = std::fs::read_to_string("tests/data/semantic/valid/rename_import.but")
        .expect("файл rename_import.but не найден");
    let (ast, _) = parse(&src, 0).expect("ошибка разбора файла");
    let node = construct_model(&ast, None, &["tests/data/include".to_string()])
        .map(|m| m.take())
        .unwrap();
    // ST — псевдоним SharedType, M — псевдоним SharedModel
    assert!(node.types.contains_key("ST"), "тип ST должен быть импортирован");
    assert!(node.search_model("M").is_some(), "модель M должна быть импортирована");
}

// ─── Тесты проверки типа и границ массива ─────────────────────────────────────

/// ArraySubscript на переменной с корректным индексом — строится без ошибок.
#[test]
fn array_subscript_valid_index() {
    let node = build("var buf: [bit;8] = 0; var x: bit = buf[0];");
    assert!(node.search_var("x").is_some());
}

/// ArraySubscript: последний допустимый индекс (size-1) — ок.
#[test]
fn array_subscript_last_valid_index() {
    let node = build("var buf: [bit;8] = 0; var x: bit = buf[7];");
    assert!(node.search_var("x").is_some());
}

/// ArraySubscript: индекс равный размеру массива — ошибка (out of bounds).
#[test]
fn array_subscript_out_of_bounds_is_error() {
    let (ast, _) = parse("var buf: [bit;8] = 0; var x: bit = buf[8]; start S;", 0).unwrap();
    let result = construct_model(&ast, None, &[]);
    assert!(result.is_err(), "индекс buf[8] должен давать ошибку для массива размером 8");
}

/// ArraySubscript: отрицательный индекс — ошибка.
#[test]
fn array_subscript_negative_index_is_error() {
    // Отрицательные индексы не поддерживаются
    let (ast, _) = parse("var buf: [bit;8] = 0; var x: bit = buf[-1]; start S;", 0).unwrap();
    let result = construct_model(&ast, None, &[]);
    assert!(result.is_err(), "отрицательный индекс должен давать ошибку");
}

/// ArraySubscript на переменной с типом Bit — ошибка (не массив).
#[test]
fn array_subscript_on_bit_is_error() {
    let (ast, _) = parse("var flag: bit = false; var x: bit = flag[0]; start S;", 0).unwrap();
    let result = construct_model(&ast, None, &[]);
    assert!(result.is_err(), "индексирование Bit-переменной должно давать ошибку");
    let err = result.unwrap_err();
    assert!(
        err.message.contains("flag"),
        "сообщение должно содержать имя переменной: {}",
        err.message
    );
}

/// `example_array_access.but` — файл с корректными операциями над массивом строится без ошибок.
#[test]
fn example_array_access_is_valid() {
    let result = build_file("tests/data/semantic/valid/array_access.but").unwrap();
    assert!(result.search_var("bit0").is_some());
    assert!(result.search_var("bit7").is_some());
}

/// `example_array_out_of_bounds.but` — должна возникнуть ошибка.
#[test]
fn example_array_out_of_bounds_is_error() {
    let result = build_file("tests/data/semantic/invalid/array_out_of_bounds.but");
    assert!(result.is_err(), "array_out_of_bounds.but должен давать ошибку");
}

/// `example_non_array_subscript.but` — должна возникнуть ошибка.
#[test]
fn example_non_array_subscript_is_error() {
    let result = build_file("tests/data/semantic/invalid/non_array_subscript.but");
    assert!(result.is_err(), "non_array_subscript.but должен давать ошибку");
}

/// `example_rename_import_missing.but` — должна возникнуть ошибка.
#[test]
fn example_rename_import_missing_is_error() {
    let src = std::fs::read_to_string("tests/data/semantic/invalid/rename_import_missing.but")
        .expect("файл не найден");
    let (ast, _) = parse(&src, 0).expect("ошибка разбора");
    let result = construct_model(&ast, None, &["tests/data/include".to_string()]).map(|m| m.take());
    assert!(result.is_err(), "импорт несуществующего символа должен давать ошибку");
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

// ─── Тесты resolve_statement и named blocks ──────────────────────────────────

use grammar::semantic::Statement;

/// Model-level `always` block с известной переменной → блок разрешается.
#[test]
fn model_always_block_with_known_var_resolves() {
    let node = build("var led: bit = false; always { led = led; } start S;");
    let nb = node.get_named_block("always").expect("always должен быть");
    let stmt = nb.statement().expect("оператор должен быть");
    assert!(
        !matches!(stmt, Statement::Unresolved(_)),
        "always должен быть разрешён: {:?}", stmt
    );
}

/// State-level `enter` block → присутствует в state.named_blocks.
#[test]
fn state_enter_block_is_populated() {
    let node = build("var x: bit = false; start S { enter { x = x; } }");
    let state = node.states.get("S").unwrap();
    assert!(state.get_named_block("enter").is_some(), "enter должен быть в state.named_blocks");
}

/// State-level `enter` с известной переменной → разрешается (не Unresolved).
#[test]
fn state_enter_block_resolves() {
    let node = build("var x: bit = false; start S { enter { x = x; } }");
    let state = node.states.get("S").unwrap();
    let enter = state.get_named_block("enter").expect("enter не найден");
    let stmt = enter.statement().expect("оператор должен быть");
    assert!(
        !matches!(stmt, Statement::Unresolved(_)),
        "enter должен быть разрешён: {:?}", stmt
    );
}

/// State-level `enter` + `exit` → оба присутствуют в named_blocks состояния.
#[test]
fn state_enter_exit_blocks_both_present() {
    let node = build("var x: bit = false; start S { enter { x = x; } exit { x = x; } }");
    let state = node.states.get("S").unwrap();
    assert!(state.get_named_block("enter").is_some(), "enter отсутствует");
    assert!(state.get_named_block("exit").is_some(), "exit отсутствует");
}

/// `if cond { ... }` в named block разрешается через Statement::Block.
#[test]
fn state_named_block_if_resolves() {
    let node = build("var f: bit = false; start S { always { if f { f = f; } } }");
    let state = node.states.get("S").unwrap();
    let always = state.get_named_block("always").expect("always не найден");
    let stmt = always.statement().expect("оператор должен быть");
    // Блок разрешён — не остаётся как Unresolved на верхнем уровне
    assert!(
        !matches!(stmt, Statement::Unresolved(_)),
        "always должен быть разрешён: {:?}", stmt
    );
}

/// Named blocks вложенной модели разрешаются в контексте вложенной модели.
#[test]
fn nested_model_named_blocks_resolve_with_own_context() {
    let node = build(
        "model Inner { var t: bit = false; start On { enter { t = t; } } state Off; } \
         start Root = Inner { }",
    );
    // Находим вложенную модель Inner
    let inner = node.search_model("Inner").expect("Inner не найдена");
    let inner = inner.borrow();
    let state = inner.states.get("On").expect("состояние On не найдено");
    let enter = state.get_named_block("enter").expect("enter не найден в On");
    let stmt = enter.statement().expect("оператор должен быть");
    assert!(
        !matches!(stmt, Statement::Unresolved(_)),
        "enter во Inner::On должен быть разрешён"
    );
}

/// `return x;` в always block разрешается в Statement::Block([Return(...)]).
#[test]
fn return_statement_in_named_block_resolves() {
    let node = build("var x: bit = false; always { return x; } start S;");
    let nb = node.get_named_block("always").expect("always не найден");
    let stmt = nb.statement().expect("оператор должен быть");
    assert!(
        !matches!(stmt, Statement::Unresolved(_)),
        "return должен быть разрешён: {:?}", stmt
    );
}

/// `always { debug(\"msg\"); }` с вызовом необъявленной встроенной функции —
/// строится без ошибок, блок разрешается (через заглушку FunctionNode).
#[test]
fn named_block_with_builtin_func_call_does_not_error() {
    let node = build(r#"always { debug("msg"); } start S;"#);
    let nb = node.get_named_block("always").expect("always не найден");
    let stmt = nb.statement().expect("оператор должен быть");
    // С заглушкой FunctionNode разрешение успешно
    assert!(
        !matches!(stmt, Statement::Unresolved(_)),
        "always с встроенной функцией должен быть разрешён: {:?}", stmt
    );
}

/// `syntax_simple` регрессионный тест: сложный SRC со всеми конструкциями
/// строится без паники (todo!() устранён).
#[test]
fn syntax_simple_does_not_panic() {
    // Копия SRC из lib.rs — проверяем что construct_model успешен
    let src = r#"
type u8 = [bit;8];
const MATRIX: u8 = { 0, 0, 0, 0, 0, 0, 0, 0 };
const NUMB: u8 = 0xFF;
cond IsEmpty = it = 0;
port A : u8  = 0x00548835;
port B1: bit = 0x00648835:6;
var it: [bit;64] = 0;
model Ping {
    start Start {
        ref End: B1;
        enter { A.0 = true; }
        exit  { A.0 = false; }
        always { A.2 = toggle; }
        always { toggle = !toggle; }
    }
    state End;
    var toggle = false;
}
model Pong {
    start Begin {
        ref Stop: S(Ping) = End;
        always { A.5 = MATRIX.5; }
    }
    state Stop {
        enter { A.6 = MATRIX.3; }
    }
}
start Entry = (Ping | Pong) + Ping;
always {
    it = it + 1;
}
"#;
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    construct_model(&ast, None, &[]).expect("construct_model не должен паниковать");
}

/// Файл named_blocks.but строится без ошибок, named_blocks заполнены.
#[test]
fn example_named_blocks_is_valid() {
    let node = build_file("tests/data/semantic/valid/named_blocks.but").unwrap();
    assert!(node.has_states(), "named_blocks.but должен иметь состояния");
    let active = node.states.get("Active").expect("Active не найдено");
    assert!(active.get_named_block("enter").is_some(), "enter должен быть в Active");
    assert!(active.get_named_block("exit").is_some(),  "exit должен быть в Active");
    assert!(active.get_named_block("always").is_some(), "always должен быть в Active");
}

/// Файл if_while_for.but строится без ошибок.
#[test]
fn example_if_while_for_is_valid() {
    build_file("tests/data/semantic/valid/if_while_for.but").unwrap();
}

/// Файл nested_model_blocks.but строится без ошибок, enter разрешён.
#[test]
fn example_nested_model_blocks_is_valid() {
    let node = build_file("tests/data/semantic/valid/nested_model_blocks.but").unwrap();
    let inner = node.search_model("Inner").expect("Inner не найдена");
    let inner = inner.borrow();
    let on = inner.states.get("On").expect("On не найдено");
    assert!(on.get_named_block("enter").is_some(), "enter должен быть в On");
}

/// named_block_undeclared_var.but (порт без адреса) → ошибка семантики.
#[test]
fn example_named_block_invalid_port_is_error() {
    let result = build_file("tests/data/semantic/invalid/named_block_undeclared_var.but");
    assert!(result.is_err(), "файл с некорректным портом должен давать ошибку");
}

/// Несколько именованных блоков с одним и тем же именем (например, два `enter`)
/// корректно сохраняются и разрешаются.
#[test]
fn multiple_named_blocks_with_same_name_resolve() {
    let node = build("var a: bit = 0; var b: bit = 0; start S { enter { a = 1; } enter { b = 1; } }");
    let state = node.states.get("S").expect("S не найден");
    let blocks = state.get_named_blocks("enter");
    assert_eq!(blocks.len(), 2, "Должно быть два блока enter");

    // Проверяем, что оба разрешены
    for block in blocks {
        let stmt = block.statement().expect("оператор должен быть");
        assert!(!matches!(stmt, Statement::Unresolved(_)), "блок должен быть разрешён");
    }
}

/// Несколько `always` блоков на уровне модели.
#[test]
fn multiple_model_level_always_blocks() {
    let node = build("var a: bit = 0; var b: bit = 0; always { a = 1; } always { b = 1; } start S;");
    let blocks = node.get_named_blocks("always");
    assert_eq!(blocks.len(), 2, "Должно быть два блока always");

    for block in blocks {
        let stmt = block.statement().expect("оператор должен быть");
        assert!(!matches!(stmt, Statement::Unresolved(_)), "блок должен быть разрешён");
    }
}

/// Файл multiple_named_blocks.but строится без ошибок, блоки извлекаются.
#[test]
fn example_multiple_named_blocks_is_valid() {
    let node = build_file("tests/data/semantic/valid/multiple_named_blocks.but").unwrap();
    let initial = node.states.get("Initial").expect("Initial не найдено");
    assert_eq!(initial.get_named_blocks("enter").len(), 2, "Должно быть два enter в Initial");
    assert_eq!(initial.get_named_blocks("exit").len(), 2, "Должно быть два exit в Initial");
    assert_eq!(node.get_named_blocks("always").len(), 3, "Должно быть три always на уровне модели");
}

// ─── Тесты корректности значений типа bit ──────────────────────────────────────

/// `tests/data/semantic/valid/bit_values.but` — допустимые значения bit строятся без ошибок.
///
/// Проверяет: 0, 1, true, false, ссылка на переменную, константы, массив [bit;N].
#[test]
fn example_bit_values_valid_is_valid() {
    let node = build_file("tests/data/semantic/valid/bit_values.but").unwrap();
    assert!(node.search_var("a").is_some(), "переменная a должна быть найдена");
    assert!(node.search_var("b").is_some(), "переменная b должна быть найдена");
    assert!(node.search_var("c").is_some(), "переменная c должна быть найдена");
    assert!(node.search_var("d").is_some(), "переменная d должна быть найдена");
}

/// `tests/data/semantic/invalid/bit_out_of_range.but` — недопустимое bit-значение → ошибка.
///
/// Тип `bit` принимает только 0, 1, true, false. Значение 2 — ошибка.
#[test]
fn example_bit_out_of_range_is_error() {
    let result = build_file("tests/data/semantic/invalid/bit_out_of_range.but");
    assert!(result.is_err(), "bit_out_of_range.but должен давать ошибку семантики");
    let err = result.unwrap_err();
    assert!(
        err.message.contains("bit"),
        "сообщение об ошибке должно упоминать тип bit: {}",
        err.message
    );
}

/// `tests/data/semantic/valid/type_inference_numbers.but` — вывод целочисленных типов.
///
/// 0..=255 → `[bit;8]`, 256..=65535 → `[bit;16]`, 65536..= → `[bit;32]`.
#[test]
fn example_type_inference_numbers_is_valid() {
    let node = build_file("tests/data/semantic/valid/type_inference_numbers.but").unwrap();
    // 8-битные
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("a") {
        assert_eq!(ty, TypeNode::Array(8, Box::new(TypeNode::Bit)), "a=0 → [bit;8]");
    }
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("c") {
        assert_eq!(ty, TypeNode::Array(8, Box::new(TypeNode::Bit)), "c=255 → [bit;8]");
    }
    // 16-битные
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("d") {
        assert_eq!(ty, TypeNode::Array(16, Box::new(TypeNode::Bit)), "d=256 → [bit;16]");
    }
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("f") {
        assert_eq!(ty, TypeNode::Array(16, Box::new(TypeNode::Bit)), "f=65535 → [bit;16]");
    }
    // 32-битные
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("g") {
        assert_eq!(ty, TypeNode::Array(32, Box::new(TypeNode::Bit)), "g=65536 → [bit;32]");
    }
}

/// `tests/data/semantic/valid/type_inference_bool.but` — вывод типа bool из литерала.
///
/// `true`/`false` без аннотации → `TypeNode::Bool`.
/// Явная аннотация `: bool` → `TypeNode::Bool`.
/// Явная аннотация `: bit` → `TypeNode::Bit`.
#[test]
fn example_type_inference_bool_is_valid() {
    let node = build_file("tests/data/semantic/valid/type_inference_bool.but").unwrap();
    // Вывод из литерала
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("flag") {
        assert_eq!(ty, TypeNode::Bool, "flag=true → Bool");
    }
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("done") {
        assert_eq!(ty, TypeNode::Bool, "done=false → Bool");
    }
    // Явная аннотация bool
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("ready") {
        assert_eq!(ty, TypeNode::Bool, "ready: bool → Bool");
    }
    // Явная аннотация bit
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("signal") {
        assert_eq!(ty, TypeNode::Bit, "signal: bit → Bit");
    }
}

// ─── Тесты новых файлов-примеров ─────────────────────────────────────────────

/// `tests/data/semantic/valid/functions.but` — локальные и внешние функции.
#[test]
fn example_functions_is_valid() {
    let node = build_file("tests/data/semantic/valid/functions.but").unwrap();
    assert!(node.functions.contains_key("send"), "внешняя функция send");
    assert!(node.functions.contains_key("recv"), "внешняя функция recv");
    assert!(node.functions.contains_key("noop"), "внешняя функция noop");
    assert!(node.functions.contains_key("identity"), "локальная функция identity");
    assert!(node.functions.contains_key("init"), "локальная функция init");
}

/// `tests/data/semantic/valid/bool_type.but` — переменные типа bool.
#[test]
fn example_bool_type_is_valid() {
    let node = build_file("tests/data/semantic/valid/bool_type.but").unwrap();
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("ready") {
        assert_eq!(ty, TypeNode::Bool, "ready: bool → TypeNode::Bool");
    } else {
        panic!("переменная ready не найдена");
    }
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("inferred_true") {
        assert_eq!(ty, TypeNode::Bool, "inferred_true = true → TypeNode::Bool");
    } else {
        panic!("переменная inferred_true не найдена");
    }
}

/// `tests/data/semantic/valid/integer_types.but` — числовые псевдонимы типов.
#[test]
fn example_integer_types_is_valid() {
    let node = build_file("tests/data/semantic/valid/integer_types.but").unwrap();
    assert!(node.types.contains_key("u8"), "тип u8 должен быть объявлен");
    assert!(node.types.contains_key("u16"), "тип u16 должен быть объявлен");
    assert!(node.types.contains_key("u32"), "тип u32 должен быть объявлен");
    // Проверяем вывод типа из числовых литералов
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("small") {
        assert_eq!(ty, TypeNode::Array(8, Box::new(TypeNode::Bit)), "small=42 → [bit;8]");
    }
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("medium") {
        assert_eq!(ty, TypeNode::Array(16, Box::new(TypeNode::Bit)), "medium=300 → [bit;16]");
    }
    if let Some(VariableNode::Simple { ty, .. }) = node.search_var("large") {
        assert_eq!(ty, TypeNode::Array(32, Box::new(TypeNode::Bit)), "large=70000 → [bit;32]");
    }
}

/// `tests/data/semantic/valid/state_machine_full.but` — полный автомат светофора.
#[test]
fn example_state_machine_full_is_valid() {
    let node = build_file("tests/data/semantic/valid/state_machine_full.but").unwrap();
    let tl = node.search_model("TrafficLight").expect("модель TrafficLight не найдена");
    let tl = tl.borrow();
    assert!(tl.states.contains_key("Red"), "состояние Red");
    assert!(tl.states.contains_key("Green"), "состояние Green");
    assert!(tl.states.contains_key("Yellow"), "состояние Yellow");
}

/// `tests/data/semantic/invalid/duplicate_model.but` — дублирующееся имя модели → ошибка.
#[test]
fn example_duplicate_model_is_error() {
    let result = build_file("tests/data/semantic/invalid/duplicate_model.but");
    assert!(result.is_err(), "дублирующееся имя модели должно давать ошибку");
}

/// `tests/data/semantic/invalid/bit_value_in_const.but` — бит-константа с недопустимым значением → ошибка.
#[test]
fn example_bit_value_in_const_is_error() {
    let result = build_file("tests/data/semantic/invalid/bit_value_in_const.but");
    assert!(result.is_err(), "bit = 5 должно давать ошибку");
}

/// `tests/data/semantic/invalid/no_start_state.but` — модель без start → ошибка.
#[test]
fn example_no_start_state_is_error() {
    let result = build_file("tests/data/semantic/invalid/no_start_state.but");
    assert!(result.is_err(), "модель без start должна давать ошибку");
}

/// `tests/data/semantic/invalid/unknown_type_in_function.but` — неизвестный тип параметра → ошибка.
#[test]
fn example_unknown_type_in_function_is_error() {
    let result = build_file("tests/data/semantic/invalid/unknown_type_in_function.but");
    assert!(result.is_err(), "неизвестный тип параметра должен давать ошибку");
}

// ─── Тесты Се1: обнаружение циклических импортов ──────────────────────────────
//
// Реализация Се1: семантический анализатор обнаруживает циклические зависимости
// между файлами импорта. При обнаружении цикла возвращается ошибка вида:
//   «Циклический импорт: /path/a.but → /path/b.but → /path/a.but»
//
// Поддерживаемые сценарии:
//   - прямой цикл между двумя файлами: a → b → a
//   - длинная цепочка: a → b → c → a
//   - самоссылающийся файл: a → a

/// Вспомогательная функция: создаёт временный `.but`-файл в директории `dir`.
fn write_tmp_in_dir(dir: &tempfile::TempDir, name: &str, content: &str) -> String {
    let path = dir.path().join(name);
    std::fs::write(&path, content).unwrap();
    dir.path().to_string_lossy().into_owned()
}

/// Прямой цикл между двумя файлами: `a.but` импортирует `b.but`, `b.but` — `a.but`.
///
/// Ожидается ошибка «Циклический импорт» с упоминанием обоих файлов в цепочке.
#[test]
fn circular_import_two_files_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let dir_str = dir.path().to_string_lossy().into_owned();

    // a.but → b.but
    write_tmp_in_dir(&dir, "a.but", r#"import "b.but"; start Entry = B { } state Done;"#);
    // b.but → a.but  (замыкает цикл)
    write_tmp_in_dir(&dir, "b.but", r#"import "a.but"; model B { start S; }"#);

    let src = r#"import "a.but";"#;
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let result = construct_model(&ast, None, &[dir_str]);

    assert!(result.is_err(), "циклический импорт должен давать ошибку");
    let err = result.unwrap_err();
    assert!(
        err.message.contains("циклический") || err.message.to_lowercase().contains("цикл"),
        "сообщение должно содержать слово о цикле: {}",
        err.message
    );
    assert!(
        err.message.contains("a.but"),
        "сообщение должно упоминать файл a.but: {}",
        err.message
    );
    assert!(
        err.message.contains("b.but"),
        "сообщение должно упоминать файл b.but: {}",
        err.message
    );
}

/// Длинная цепочка циклического импорта: `a → b → c → a`.
///
/// Ожидается ошибка с цепочкой из трёх файлов.
#[test]
fn circular_import_three_files_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let dir_str = dir.path().to_string_lossy().into_owned();

    write_tmp_in_dir(&dir, "ca.but", r#"import "cb.but"; start Entry = Cb { } state Done;"#);
    write_tmp_in_dir(&dir, "cb.but", r#"import "cc.but"; model Cb { start S; }"#);
    // cc.but → ca.but  (замыкает цикл длиной 3)
    write_tmp_in_dir(&dir, "cc.but", r#"import "ca.but"; model Cc { start S; }"#);

    let src = r#"import "ca.but";"#;
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let result = construct_model(&ast, None, &[dir_str]);

    assert!(result.is_err(), "трёхзвенный циклический импорт должен давать ошибку");
    let err = result.unwrap_err();
    assert!(
        err.message.contains("циклический") || err.message.to_lowercase().contains("цикл"),
        "сообщение должно содержать слово о цикле: {}",
        err.message
    );
}

/// Самоссылающийся файл: `self.but` импортирует самого себя.
///
/// Это частный случай прямого цикла длиной 1.
#[test]
fn circular_import_self_reference_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let dir_str = dir.path().to_string_lossy().into_owned();

    // self.but импортирует себя же
    write_tmp_in_dir(&dir, "self_ref.but", r#"import "self_ref.but"; start S;"#);

    let src = r#"import "self_ref.but";"#;
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let result = construct_model(&ast, None, &[dir_str]);

    assert!(result.is_err(), "самоссылающийся импорт должен давать ошибку");
    let err = result.unwrap_err();
    assert!(
        err.message.contains("циклический") || err.message.to_lowercase().contains("цикл"),
        "сообщение должно содержать слово о цикле: {}",
        err.message
    );
    assert!(
        err.message.contains("self_ref.but"),
        "сообщение должно упоминать файл self_ref.but: {}",
        err.message
    );
}

/// Алмазная зависимость (diamond): `a` импортирует `b` и `c`, оба — `d`.
///
/// Это НЕ цикл: `d` дважды импортируется по разным путям,
/// но каждая ветвь не формирует петлю. Ожидается ошибка
/// «уже объявлена» (повторный импорт), а НЕ ошибка цикла.
#[test]
fn diamond_import_is_not_cycle_error() {
    let dir = tempfile::tempdir().unwrap();
    let dir_str = dir.path().to_string_lossy().into_owned();

    // d.but — общая зависимость
    write_tmp_in_dir(&dir, "d.but", r#"model D { start S; }"#);
    // b.but и c.but оба импортируют d.but
    write_tmp_in_dir(&dir, "db.but", r#"import "d.but"; model Db { start S; }"#);
    write_tmp_in_dir(&dir, "dc.but", r#"import "d.but"; model Dc { start S; }"#);
    // a.but импортирует оба
    write_tmp_in_dir(&dir, "da.but", r#"import "db.but"; import "dc.but"; start S;"#);

    let src = r#"import "da.but";"#;
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let result = construct_model(&ast, None, &[dir_str]);

    // Ожидаем ошибку (повторный импорт D), но НЕ «циклический»
    if let Err(err) = &result {
        assert!(
            !err.message.contains("циклический") && !err.message.to_lowercase().contains("цикл"),
            "алмазная зависимость не является циклом: {}",
            err.message
        );
    }
    // Результат может быть как Ok, так и Err (в зависимости от реализации dedup),
    // но НЕ должен быть ошибкой цикла.
}

/// Цикл через `import "..." as Alias` — GlobalSymbol вариант импорта.
///
/// Обнаружение цикла должно работать для всех форм импорта.
#[test]
fn circular_import_via_global_symbol_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let dir_str = dir.path().to_string_lossy().into_owned();

    write_tmp_in_dir(&dir, "ga.but", r#"import "gb.but" as Gb; start Entry = Gb { } state Done;"#);
    write_tmp_in_dir(&dir, "gb.but", r#"import "ga.but" as Ga; model Gb { start S; }"#);

    let src = r#"import "ga.but" as Ga;"#;
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let result = construct_model(&ast, None, &[dir_str]);

    assert!(result.is_err(), "циклический as-импорт должен давать ошибку");
    let err = result.unwrap_err();
    assert!(
        err.message.contains("циклический") || err.message.to_lowercase().contains("цикл"),
        "сообщение должно содержать слово о цикле: {}",
        err.message
    );
}

/// Цикл через `import {{ A }} from "..."` — Rename вариант импорта.
///
/// Обнаружение цикла должно работать для всех форм импорта.
#[test]
fn circular_import_via_rename_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let dir_str = dir.path().to_string_lossy().into_owned();

    write_tmp_in_dir(&dir, "ra.but", r#"import { Rb } from "rb.but"; start Entry = Rb { } state Done;"#);
    write_tmp_in_dir(&dir, "rb.but", r#"import { Ra } from "ra.but"; model Ra { start S; } model Rb { start S; }"#);

    // Инициируем цикл через Plain-импорт (ra.but содержит rename-импорт rb.but, который замкнёт цикл)
    let src = r#"import "ra.but";"#;
    let (ast, _) = parse(&src, 0).expect("ошибка разбора");
    let result = construct_model(&ast, None, &[dir_str]);

    assert!(result.is_err(), "циклический rename-импорт должен давать ошибку");
    let err = result.unwrap_err();
    assert!(
        err.message.contains("циклический") || err.message.to_lowercase().contains("цикл"),
        "сообщение должно содержать слово о цикле: {}",
        err.message
    );
}

/// Нециклический линейный импорт `a → b → c` (без петли) — должен успешно строиться.
///
/// Проверяет, что детектор не ложно срабатывает на корректные цепочки.
#[test]
fn linear_import_chain_is_valid() {
    let dir = tempfile::tempdir().unwrap();
    let dir_str = dir.path().to_string_lossy().into_owned();

    write_tmp_in_dir(&dir, "lc.but", r#"model Lc { start S; }"#);
    write_tmp_in_dir(&dir, "lb.but", r#"import "lc.but"; model Lb { start S; }"#);
    write_tmp_in_dir(&dir, "la.but", r#"import "lb.but"; model La { start S; }"#);

    let src = r#"import "la.but";"#;
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let result = construct_model(&ast, None, &[dir_str]);

    assert!(result.is_ok(), "линейная цепочка импортов должна успешно строиться: {:?}", result.err());
}

// ─── Тесты Се12: документационные комментарии в семантическом дереве ──────────
//
// Реализация Се12: `///`-комментарии включаются в семантическое дерево через
// `construct_model_with_docs`. Доступ:
//   - `model_node.own_doc()`                → документация самой модели
//   - `model_node.element_doc("Имя")`       → документация именованного элемента
//   - `model_node.docs["Имя"]`              → то же через HashMap

/// `construct_model_with_docs` — состояние получает свой doc-комментарий.
#[test]
fn doc_comment_for_state() {
    let src = "/// Начальное состояние.\nstart S;";
    let (ast, comments) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model_with_docs(&ast, None, &[], &comments)
        .expect("ошибка построения семантики");
    assert_eq!(
        root.borrow().element_doc("S"),
        ["Начальное состояние."],
        "doc-комментарий должен быть привязан к состоянию S"
    );
}

/// Переменная получает свой doc-комментарий.
#[test]
fn doc_comment_for_variable() {
    let src = "/// Счётчик.\nvar counter: bit = false;";
    let (ast, comments) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model_with_docs(&ast, None, &[], &comments)
        .expect("ошибка построения семантики");
    assert_eq!(
        root.borrow().element_doc("counter"),
        ["Счётчик."],
        "doc-комментарий должен быть привязан к переменной counter"
    );
}

/// Тип (`type`) получает свой doc-комментарий.
#[test]
fn doc_comment_for_type() {
    let src = "/// Байт.\ntype u8 = [bit;8];";
    let (ast, comments) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model_with_docs(&ast, None, &[], &comments)
        .expect("ошибка построения семантики");
    assert_eq!(
        root.borrow().element_doc("u8"),
        ["Байт."],
        "doc-комментарий должен быть привязан к типу u8"
    );
}

/// Условие (`cond`) получает свой doc-комментарий.
#[test]
fn doc_comment_for_condition() {
    let src = "/// Истинно всегда.\ncond always_true = true;";
    let (ast, comments) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model_with_docs(&ast, None, &[], &comments)
        .expect("ошибка построения семантики");
    assert_eq!(
        root.borrow().element_doc("always_true"),
        ["Истинно всегда."],
        "doc-комментарий должен быть привязан к условию always_true"
    );
}

/// Вложенная модель получает doc-комментарий через `root.element_doc("M")`.
#[test]
fn doc_comment_for_nested_model_from_parent() {
    let src = "/// Вложенная модель.\nmodel M { start S; }";
    let (ast, comments) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model_with_docs(&ast, None, &[], &comments)
        .expect("ошибка построения семантики");
    assert_eq!(
        root.borrow().element_doc("M"),
        ["Вложенная модель."],
        "doc-комментарий должен быть доступен через родительскую модель"
    );
}

/// Собственный `doc`-поле вложенной модели содержит тот же текст.
#[test]
fn doc_comment_for_nested_model_own_doc() {
    let src = "/// Своя документация.\nmodel Inner { start S; }";
    let (ast, comments) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model_with_docs(&ast, None, &[], &comments)
        .expect("ошибка построения семантики");
    let inner = root.borrow().search_model("Inner").unwrap();
    assert_eq!(
        inner.borrow().own_doc(),
        ["Своя документация."],
        "дочерний узел должен хранить свою документацию в поле doc"
    );
}

/// Многострочный doc-блок из нескольких `///` привязывается как список строк.
#[test]
fn multi_line_doc_comment_for_state() {
    let src = "/// Строка 1.\n/// Строка 2.\n/// Строка 3.\nstart S;";
    let (ast, comments) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model_with_docs(&ast, None, &[], &comments)
        .expect("ошибка построения семантики");
    let doc = root.borrow().element_doc("S").to_vec();
    assert_eq!(doc.len(), 3, "должно быть три строки документации");
    assert_eq!(doc[0], "Строка 1.");
    assert_eq!(doc[1], "Строка 2.");
    assert_eq!(doc[2], "Строка 3.");
}

/// Обычный `//`-комментарий НЕ попадает в документацию.
#[test]
fn regular_comment_not_in_docs() {
    let src = "// Обычный комментарий.\nstart S;";
    let (ast, comments) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model_with_docs(&ast, None, &[], &comments)
        .expect("ошибка построения семантики");
    assert!(
        root.borrow().element_doc("S").is_empty(),
        "обычный // комментарий не должен попасть в документацию"
    );
}

/// Без комментариев — `element_doc` возвращает пустой срез.
#[test]
fn no_doc_comment_returns_empty() {
    let src = "start S; state Done;";
    let (ast, comments) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model_with_docs(&ast, None, &[], &comments)
        .expect("ошибка построения семантики");
    assert!(root.borrow().element_doc("S").is_empty());
    assert!(root.borrow().element_doc("Done").is_empty());
    assert!(root.borrow().own_doc().is_empty());
}

/// Каждый элемент получает свой doc-комментарий — не чужой.
#[test]
fn each_element_gets_its_own_doc() {
    let src = concat!(
        "/// Состояние A.\n",
        "start A;\n",
        "/// Состояние B.\n",
        "state B;\n",
    );
    let (ast, comments) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model_with_docs(&ast, None, &[], &comments)
        .expect("ошибка построения семантики");
    assert_eq!(root.borrow().element_doc("A"), ["Состояние A."]);
    assert_eq!(root.borrow().element_doc("B"), ["Состояние B."]);
}

/// `construct_model` (без docs) → поля doc и docs остаются пустыми.
#[test]
fn construct_model_without_docs_leaves_fields_empty() {
    let src = "/// Документация.\nstart S;";
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения семантики");
    assert!(
        root.borrow().element_doc("S").is_empty(),
        "construct_model без docs должен оставить поле docs пустым"
    );
    assert!(root.borrow().own_doc().is_empty());
}

/// Документация вложенного состояния в модели (`model M { /// doc\n state S... }`).
#[test]
fn doc_comment_for_state_inside_model() {
    let src = concat!(
        "model M {\n",
        "    /// Документация состояния внутри модели.\n",
        "    start S;\n",
        "    state Done;\n",
        "}\n",
    );
    let (ast, comments) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model_with_docs(&ast, None, &[], &comments)
        .expect("ошибка построения семантики");
    let m = root.borrow().search_model("M").unwrap();
    assert_eq!(
        m.borrow().element_doc("S"),
        ["Документация состояния внутри модели."],
    );
    assert!(
        m.borrow().element_doc("Done").is_empty(),
        "Done не имеет doc-комментария"
    );
}

/// `tests/data/semantic/valid/doc_comments.but` — файл с doc-комментариями строится корректно.
#[test]
fn example_doc_comments_file_is_valid() {
    let src = std::fs::read_to_string("tests/data/semantic/valid/doc_comments.but")
        .expect("не могу прочитать doc_comments.but");
    let (ast, comments) = parse(&src, 0).expect("ошибка разбора");
    let root = construct_model_with_docs(&ast, None, &[], &comments)
        .expect("ошибка построения семантики");

    // Проверяем документацию на верхнем уровне
    let rb = root.borrow();
    assert!(!rb.element_doc("u8").is_empty(), "тип u8 должен иметь doc");
    assert!(!rb.element_doc("counter").is_empty(), "переменная counter должна иметь doc");
    assert!(!rb.element_doc("MaxReached").is_empty(), "условие MaxReached должно иметь doc");
    assert!(!rb.element_doc("TrafficLight").is_empty(), "модель TrafficLight должна иметь doc");

    // Проверяем документацию состояний внутри TrafficLight
    let tl = rb.search_model("TrafficLight").expect("TrafficLight не найдена");
    let tl = tl.borrow();
    assert!(!tl.own_doc().is_empty(), "TrafficLight должна иметь собственную документацию");
    assert!(!tl.element_doc("timer").is_empty(), "переменная timer должна иметь doc");
    assert!(!tl.element_doc("Red").is_empty(), "состояние Red должно иметь doc");
    assert!(!tl.element_doc("Green").is_empty(), "состояние Green должно иметь doc");
    assert!(!tl.element_doc("Yellow").is_empty(), "состояние Yellow должно иметь doc");
}

/// Документация модели содержит несколько строк из многострочного `///`-блока.
#[test]
fn multi_line_doc_for_model() {
    let src = concat!(
        "/// Первая строка.\n",
        "/// Вторая строка.\n",
        "model M { start S; }\n",
    );
    let (ast, comments) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model_with_docs(&ast, None, &[], &comments)
        .expect("ошибка построения семантики");
    let doc = root.borrow().element_doc("M").to_vec();
    assert_eq!(doc.len(), 2, "должно быть две строки документации для M");
    assert_eq!(doc[0], "Первая строка.");
    assert_eq!(doc[1], "Вторая строка.");
    let m = root.borrow().search_model("M").unwrap();
    assert_eq!(m.borrow().own_doc().len(), 2, "M.doc тоже должен содержать обе строки");
}

// ─── Се11: строгая проверка булевости условий переходов ──────────────────────

/// Явное сравнение в условии перехода — нет предупреждений.
///
/// # BuT
/// ```but
/// var timer: [bit;8] = 0;
/// start S { ref T: timer != 0; }
/// state T;
/// ```
#[test]
fn se11_explicit_comparison_no_warnings() {
    let src = "var timer: [bit;8] = 0; start S { ref T: timer != 0; } state T;";
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert!(warnings.is_empty(), "явное сравнение не должно давать предупреждений");
}

/// Числовая переменная без сравнения — предупреждение Се11.
///
/// # BuT
/// ```but
/// var timer: [bit;8] = 0;
/// start S { ref T: timer; }   // ← Предупреждение
/// state T;
/// ```
#[test]
fn se11_numeric_var_in_ref_gives_warning() {
    let src = "var timer: [bit;8] = 0; start S { ref T: timer; } state T;";
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert_eq!(warnings.len(), 1, "числовая переменная в условии должна давать предупреждение");
    assert!(
        warnings[0].message.contains("timer"),
        "предупреждение должно упоминать имя переменной"
    );
}

/// Числовой литерал в условии перехода — предупреждение Се11.
#[test]
fn se11_number_literal_in_ref_gives_warning() {
    let src = "start S { ref T: 5; } state T;";
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert_eq!(warnings.len(), 1, "числовой литерал в условии должен давать предупреждение");
}

/// Переменная типа `bool` в условии — нет предупреждений.
#[test]
fn se11_bool_var_in_ref_no_warnings() {
    let src = "var flag: bool = false; start S { ref T: flag; } state T;";
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert!(warnings.is_empty(), "переменная bool не должна давать предупреждений");
}

/// Переменная типа `bit` (1 бит) в условии — нет предупреждений.
#[test]
fn se11_bit_var_in_ref_no_warnings() {
    let src = "var flag: bit = 0; start S { ref T: flag; } state T;";
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert!(warnings.is_empty(), "переменная bit не должна давать предупреждений");
}

/// Булев литерал в условии — нет предупреждений.
#[test]
fn se11_bool_literal_in_ref_no_warnings() {
    let src = "start S { ref T: true; } state T;";
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert!(warnings.is_empty(), "булев литерал не должен давать предупреждений");
}

/// Безусловный переход (без условия) — нет предупреждений.
#[test]
fn se11_unconditional_ref_no_warnings() {
    let src = "start S { ref T; } state T;";
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert!(warnings.is_empty(), "безусловный переход не должен давать предупреждений");
}

/// Несколько переходов: один числовой, один явный — одно предупреждение.
#[test]
fn se11_one_numeric_one_explicit_ref() {
    let src = concat!(
        "var timer: [bit;8] = 0;\n",
        "var flag: bool = false;\n",
        "start S { ref T: timer; ref U: flag; }\n",
        "state T;\n",
        "state U;\n",
    );
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert_eq!(warnings.len(), 1, "должно быть ровно одно предупреждение");
}

/// Вложенная модель с числовым условием — предупреждение включает имя модели.
#[test]
fn se11_nested_model_numeric_ref_warning() {
    let src = concat!(
        "model M {\n",
        "    var timer: [bit;8] = 0;\n",
        "    start S { ref T: timer; }\n",
        "    state T;\n",
        "}\n",
    );
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert_eq!(warnings.len(), 1, "предупреждение из вложенной модели");
    assert!(
        warnings[0].message.contains("M"),
        "предупреждение должно упоминать имя вложенной модели"
    );
}

/// Файл `implicit_bool_warn.but` из тестовых данных — без предупреждений.
///
/// Все переходы в файле используют явные сравнения или булевы переменные.
#[test]
fn se11_valid_file_no_warnings() {
    let src = std::fs::read_to_string(
        "tests/data/semantic/valid/implicit_bool_warn.but",
    )
    .expect("не удалось прочитать файл");
    let (ast, _) = parse(&src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert!(
        warnings.is_empty(),
        "файл с явными сравнениями не должен давать предупреждений: {:?}",
        warnings
    );
}

/// Файл `implicit_bool_numeric.but` — одно предупреждение о числовом условии.
#[test]
fn se11_numeric_file_gives_one_warning() {
    let src = std::fs::read_to_string(
        "tests/data/semantic/valid/implicit_bool_numeric.but",
    )
    .expect("не удалось прочитать файл");
    let (ast, _) = parse(&src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert_eq!(
        warnings.len(),
        1,
        "файл с числовым условием должен давать ровно одно предупреждение"
    );
    assert!(
        warnings[0].message.contains("timer"),
        "предупреждение должно упоминать переменную timer"
    );
}

/// Условие сравнения `<` — нет предупреждений Се11.
#[test]
fn se11_less_comparison_no_warnings() {
    let src = "var timer: [bit;8] = 0; start S { ref T: timer < 100; } state T;";
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert!(warnings.is_empty(), "условие '<' не должно давать предупреждений");
}

/// Условия `>`, `<=`, `>=` — нет предупреждений Се11.
#[test]
fn se11_other_comparisons_no_warnings() {
    let src = concat!(
        "var a: [bit;8] = 0;\n",
        "var b: [bit;8] = 0;\n",
        "start S { ref T: a > 0; ref U: a <= b; ref V: a >= b; }\n",
        "state T; state U; state V;\n",
    );
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert!(
        warnings.is_empty(),
        "условия >, <=, >= не должны давать предупреждений"
    );
}

/// Логическое НЕ в условии — нет предупреждений Се11.
#[test]
fn se11_not_condition_no_warnings() {
    let src = "var flag: bool = false; start S { ref T: !flag; } state T;";
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert!(warnings.is_empty(), "условие '!' не должно давать предупреждений");
}

/// Именованное условие в ref — нет предупреждений Се11.
#[test]
fn se11_named_cond_in_ref_no_warnings() {
    let src = concat!(
        "var counter: [bit;8] = 0;\n",
        "cond Full = counter = 255;\n",
        "start S { ref T: Full; }\n",
        "state T;\n",
    );
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert!(
        warnings.is_empty(),
        "именованное условие не должно давать предупреждений"
    );
}

/// Арифметическое выражение в условии — предупреждение Се11.
#[test]
fn se11_arithmetic_in_ref_gives_warning() {
    let src = "var timer: [bit;8] = 0; start S { ref T: timer + 1; } state T;";
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert_eq!(
        warnings.len(),
        1,
        "арифметическое выражение в условии должно давать предупреждение"
    );
    assert!(
        warnings[0].message.contains("сложение"),
        "предупреждение должно упоминать тип выражения: {}",
        warnings[0].message
    );
}

/// Файл с арифметическим условием — одно предупреждение.
#[test]
fn se11_arithmetic_file_gives_one_warning() {
    let src = std::fs::read_to_string(
        "tests/data/semantic/valid/implicit_bool_arithmetic.but",
    )
    .expect("не удалось прочитать файл");
    let (ast, _) = parse(&src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert_eq!(
        warnings.len(),
        1,
        "файл с арифметическим условием должен давать ровно одно предупреждение"
    );
}

/// Файл с именованными условиями — нет предупреждений Се11.
#[test]
fn se11_named_cond_file_no_warnings() {
    let src = std::fs::read_to_string(
        "tests/data/semantic/valid/implicit_bool_named_cond.but",
    )
    .expect("не удалось прочитать файл");
    let (ast, _) = parse(&src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert!(
        warnings.is_empty(),
        "файл с именованными условиями не должен давать предупреждений: {:?}",
        warnings
    );
}

/// Предупреждение Се11 содержит имя исходного состояния.
#[test]
fn se11_warning_contains_source_state_name() {
    let src = "var x: [bit;8] = 0; start SourceState { ref T: x; } state T;";
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert_eq!(warnings.len(), 1);
    assert!(
        warnings[0].message.contains("SourceState"),
        "предупреждение должно упоминать состояние-источник: {}",
        warnings[0].message
    );
}

// ─── Тесты разыменования condition в ref-переходах (этап 6 конвейера) ───────

/// Условие ref с bit-переменной разрешается в `Condition::Variable`, не в `Condition::Unresolved`.
///
/// # Пример (BuT)
/// ```but
/// var flag: bit = false;
/// start A { ref B: flag; }
/// state B;
/// ```
#[test]
fn ref_cond_bit_var_is_resolved() {
    use grammar::semantic::Condition;
    let src = "var flag: bit = false; start A { ref B: flag; } state B;";
    let node = build(src);
    let state_a = &node.states["A"];
    if let grammar::semantic::StateNode::Simple { references, .. } = state_a {
        assert_eq!(references.len(), 1);
        assert!(
            matches!(references[0].cond, Condition::Variable(_)),
            "условие должно быть разрешено в Variable, получено: {:?}",
            references[0].cond
        );
    } else {
        panic!("ожидался StateNode::Simple для A");
    }
}

/// Условие ref с bool-переменной разрешается в `Condition::Variable`.
///
/// # Пример (BuT)
/// ```but
/// var done: bool = false;
/// start A { ref B: done; }
/// state B;
/// ```
#[test]
fn ref_cond_bool_var_is_resolved() {
    use grammar::semantic::Condition;
    let src = "var done: bool = false; start A { ref B: done; } state B;";
    let node = build(src);
    let state_a = &node.states["A"];
    if let grammar::semantic::StateNode::Simple { references, .. } = state_a {
        assert!(
            matches!(references[0].cond, Condition::Variable(_)),
            "условие должно быть разрешено в Variable"
        );
    } else {
        panic!("ожидался StateNode::Simple для A");
    }
}

/// Именованное условие (`cond`) в ref разрешается до его значения (не `Unresolved`).
///
/// # Пример (BuT)
/// ```but
/// var x: [bit;8] = 0;
/// cond full = x = 255;
/// start A { ref B: full; }
/// state B;
/// ```
#[test]
fn ref_cond_named_cond_is_resolved() {
    use grammar::semantic::Condition;
    let src = "var x: [bit;8] = 0; cond full = x = 255; start A { ref B: full; } state B;";
    let node = build(src);
    let state_a = &node.states["A"];
    if let grammar::semantic::StateNode::Simple { references, .. } = state_a {
        assert_eq!(references.len(), 1);
        // Именованное условие раскрывается до значения (Equal или аналог)
        assert!(
            !matches!(references[0].cond, Condition::Unresolved(_)),
            "условие не должно оставаться Unresolved после этапа 6"
        );
    } else {
        panic!("ожидался StateNode::Simple для A");
    }
}

/// Безусловный переход (`ref B`) оставляет `Condition::None`.
///
/// # Пример (BuT)
/// ```but
/// start A { ref B; }
/// state B;
/// ```
#[test]
fn ref_no_cond_is_none() {
    use grammar::semantic::Condition;
    let src = "start A { ref B; } state B;";
    let node = build(src);
    let state_a = &node.states["A"];
    if let grammar::semantic::StateNode::Simple { references, .. } = state_a {
        assert_eq!(references.len(), 1);
        assert_eq!(references[0].cond, Condition::None);
    } else {
        panic!("ожидался StateNode::Simple для A");
    }
}

/// Булев литерал `true` в ref разрешается в `Condition::Bool(true)`.
///
/// # Пример (BuT)
/// ```but
/// start A { ref B: true; }
/// state B;
/// ```
#[test]
fn ref_cond_bool_literal_is_resolved() {
    use grammar::semantic::Condition;
    let src = "start A { ref B: true; } state B;";
    let node = build(src);
    let state_a = &node.states["A"];
    if let grammar::semantic::StateNode::Simple { references, .. } = state_a {
        assert_eq!(references[0].cond, Condition::Bool(true));
    } else {
        panic!("ожидался StateNode::Simple для A");
    }
}

/// Сравнение в ref разрешается в `Condition::Equal`.
///
/// # Пример (BuT)
/// ```but
/// var x: [bit;8] = 0;
/// start A { ref B: x = 255; }
/// state B;
/// ```
#[test]
fn ref_cond_comparison_is_resolved() {
    use grammar::semantic::Condition;
    let src = "var x: [bit;8] = 0; start A { ref B: x = 255; } state B;";
    let node = build(src);
    let state_a = &node.states["A"];
    if let grammar::semantic::StateNode::Simple { references, .. } = state_a {
        assert!(
            matches!(references[0].cond, Condition::Equal(_, _)),
            "ожидалось Condition::Equal, получено {:?}",
            references[0].cond
        );
    } else {
        panic!("ожидался StateNode::Simple");
    }
}

/// Контрпример: арифметика в ref-условии даёт предупреждение «арифметическое вычитание».
///
/// # Контрпример (BuT)
/// ```but
/// var x: [bit;8] = 0;
/// start A { ref B: x - 1; }
/// state B;
/// ```
#[test]
fn se11_subtract_in_ref_gives_warning() {
    let src = "var x: [bit;8] = 0; start A { ref B: x - 1; } state B;";
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert_eq!(warnings.len(), 1, "вычитание должно давать предупреждение");
    assert!(
        warnings[0].message.contains("вычитание"),
        "сообщение должно упоминать вычитание: {}",
        warnings[0].message
    );
}

/// Контрпример: побитовое И в ref-условии даёт предупреждение «побитовое И».
///
/// # Контрпример (BuT)
/// ```but
/// var x: [bit;8] = 0;
/// start A { ref B: x & 1; }
/// state B;
/// ```
#[test]
fn se11_bitwise_and_in_ref_gives_warning() {
    let src = "var x: [bit;8] = 0; start A { ref B: x & 1; } state B;";
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert_eq!(warnings.len(), 1, "побитовое И должно давать предупреждение");
    assert!(
        warnings[0].message.contains("побитовое И"),
        "сообщение должно упоминать тип операции: {}",
        warnings[0].message
    );
}

/// Контрпример: числовой литерал в ref-условии даёт предупреждение с указанием числа.
///
/// # Контрпример (BuT)
/// ```but
/// start A { ref B: 42; }
/// state B;
/// ```
#[test]
fn se11_resolved_number_literal_in_ref_has_value_in_message() {
    let src = "start A { ref B: 42; } state B;";
    let (ast, _) = parse(src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let warnings = implicit_bool_warnings(&root);
    assert_eq!(warnings.len(), 1, "числовой литерал должен давать предупреждение");
    assert!(
        warnings[0].message.contains("42"),
        "сообщение должно упоминать значение: {}",
        warnings[0].message
    );
}

/// Пример файла с разрешёнными условиями — без ошибок и предупреждений.
#[test]
fn ref_cond_resolved_file_is_valid() {
    let src = std::fs::read_to_string(
        "tests/data/semantic/valid/ref_cond_resolved.but",
    )
    .expect("не удалось прочитать файл");
    let (ast, _) = parse(&src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения семантики");
    let warnings = implicit_bool_warnings(&root);
    assert!(
        warnings.is_empty(),
        "файл с разрешёнными условиями не должен давать предупреждений: {:?}",
        warnings
    );
}

/// Контрпример файла с арифметическим условием — одно предупреждение Се11.
#[test]
fn ref_cond_arithmetic_file_gives_warning() {
    let src = std::fs::read_to_string(
        "tests/data/semantic/valid/ref_cond_arithmetic.but",
    )
    .expect("не удалось прочитать файл");
    let (ast, _) = parse(&src, 0).expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения семантики");
    let warnings = implicit_bool_warnings(&root);
    assert_eq!(
        warnings.len(),
        1,
        "арифметический файл должен давать ровно одно предупреждение"
    );
    assert!(
        warnings[0].message.contains("сложение"),
        "сообщение должно упоминать тип операции: {}",
        warnings[0].message
    );
}

// ─── Тесты родительских ссылок (требование 3) ────────────────────────────────

/// Переменная хранит ссылку на родительскую модель (`upper`).
///
/// Тест использует `Rc` напрямую (не `.take()`), чтобы родительская
/// модель оставалась живой и Weak-ссылка могла быть разыменована.
///
/// # Пример (BuT)
/// ```but
/// var flag: bit = false;
/// ```
#[test]
fn variable_node_has_parent_upper() {
    let (ast, _) = parse("var flag: bit = false;", 0).unwrap();
    let root = construct_model(&ast, None, &[]).unwrap();
    let var = root.borrow().search_var("flag").expect("переменная flag не найдена");
    let upper = var.upper();
    assert!(
        upper.is_some(),
        "переменная должна иметь ссылку на родительскую модель"
    );
}

/// Константа хранит ссылку на родительскую модель.
///
/// Тест использует `Rc` напрямую (не `.take()`), чтобы родительская
/// модель оставалась живой и Weak-ссылка могла быть разыменована.
#[test]
fn const_node_has_parent_upper() {
    let (ast, _) = parse("type u8 = [bit;8]; const C: u8 = 0;", 0).unwrap();
    let root = construct_model(&ast, None, &[]).unwrap();
    let var = root.borrow().search_var("C").expect("константа C не найдена");
    assert!(
        var.upper().is_some(),
        "константа должна иметь ссылку на родительскую модель"
    );
}

/// Именованное условие хранит ссылку на родительскую модель.
///
/// # Пример (BuT)
/// ```but
/// cond done = true;
/// ```
#[test]
fn condition_node_has_parent_upper() {
    let node = build("cond done = true;");
    let cond = node.conditions.get("done").expect("условие done не найдено");
    assert!(
        cond.upper.is_some(),
        "именованное условие должно иметь ссылку на родительскую модель"
    );
}

/// Вложенная переменная ссылается на свою (вложенную) модель, не на корень.
///
/// # Пример (BuT)
/// ```but
/// model Inner { var x: bit = false; start S; }
/// ```
#[test]
fn nested_variable_upper_points_to_inner_model() {
    use grammar::semantic::VariableNode;
    let (ast, _) = parse("model Inner { var x: bit = false; start S; }", 0).unwrap();
    let root = construct_model(&ast, None, &[]).unwrap();
    let inner = root.borrow().search_model("Inner").expect("Inner не найдена");
    let var = inner.borrow().search_var("x").expect("переменная x не найдена");
    let upper = var.upper().expect("переменная должна иметь upper");
    // upper должен ссылаться на Inner, а не на корневую модель
    assert_eq!(
        upper.borrow().name,
        Some("Inner".to_string()),
        "upper переменной должен указывать на модель Inner"
    );
}

/// Метод `upper()` у VariableNode::Unresolved возвращает None.
#[test]
fn unresolved_variable_upper_is_none() {
    use grammar::semantic::VariableNode;
    let unresolved = VariableNode::Unresolved;
    assert!(unresolved.upper().is_none());
}

/// Вспомогательные методы `name()` и `ty()` у VariableNode работают корректно.
#[test]
fn variable_node_name_and_ty_methods() {
    use grammar::semantic::{TypeNode, VariableNode};
    let node = build("var flag: bit = false;");
    let var = node.search_var("flag").expect("flag не найдена");
    assert_eq!(var.name(), "flag");
    assert_eq!(*var.ty(), TypeNode::Bit);
}

// ─── С4: интеграционные тесты локальных переменных в блоках ──────────────────

/// `tests/data/semantic/valid/local_var_in_block.but` — var внутри always — без ошибок.
///
/// # Пример (BuT)
/// ```but
/// var flag: bit = false;
/// start Running {
///     always {
///         var x: bit = false;
///         x = true;
///         flag = x;
///     }
/// }
/// ```
#[test]
fn example_local_var_in_block_is_valid() {
    build_file("tests/data/semantic/valid/local_var_in_block.but").unwrap();
}

/// `tests/data/semantic/valid/local_var_in_for.but` — var в инициализаторе for — без ошибок.
///
/// # Пример (BuT)
/// ```but
/// var result: bit = false;
/// start S {
///     always {
///         for var i: bit = false; i; i = false { result = i; }
///     }
/// }
/// ```
#[test]
fn example_local_var_in_for_is_valid() {
    build_file("tests/data/semantic/valid/local_var_in_for.but").unwrap();
}

/// `tests/data/semantic/valid/local_var_nested.but` — вложенные блоки с затенением — без ошибок.
///
/// # Пример (BuT)
/// ```but
/// var x: bit = true;
/// start S {
///     always {
///         { var x: bit = false; x = false; }
///         x = true;   // ← model-level x, не локальная
///     }
/// }
/// ```
#[test]
fn example_local_var_nested_is_valid() {
    build_file("tests/data/semantic/valid/local_var_nested.but").unwrap();
}

/// Переменная через `upper()` позволяет найти другие переменные той же модели.
///
/// Демонстрирует, что `upper` действительно предоставляет доступ к контексту.
/// Используем `Rc<RefCell<ModelNode>>` напрямую (без `.take()`), чтобы `upper`
/// внутри переменных ссылался на живой узел модели.
#[test]
fn variable_upper_gives_access_to_sibling_vars() {
    use grammar::semantic::VariableNode;
    let (ast, _) = parse("var a: bit = false; var b: bit = false;", 0)
        .expect("ошибка разбора");
    let root = construct_model(&ast, None, &[]).expect("ошибка построения");
    let var_a = root.borrow().search_var("a").expect("a не найдена");
    let upper = var_a.upper().expect("upper должен быть Some");
    // Через upper можно найти переменную b
    assert!(
        upper.borrow().search_var("b").is_some(),
        "через upper переменной a должна быть доступна переменная b"
    );
}

// ─── SA8: тесты отсутствия циклических сильных Rc-ссылок ──────────────────

/// Модель с условиями не создаёт сильных циклов (SA8).
#[test]
fn no_strong_cycle_with_conditions() {
    use std::rc::Rc;
    let (ast, _) = parse("var x: bit = false; cond done = x = false; start S;", 0).unwrap();
    let root = grammar::semantic::tree::construct_model(&ast, None, &[]).unwrap();
    assert_eq!(Rc::strong_count(&root), 1, "модель с условиями: счётчик Rc должен быть 1");
}

/// Модель с именованными блоками не создаёт сильных циклов (SA8).
#[test]
fn no_strong_cycle_with_named_blocks() {
    use std::rc::Rc;
    let (ast, _) = parse("var x: bit = false; start S { always { x = x; } }", 0).unwrap();
    let root = grammar::semantic::tree::construct_model(&ast, None, &[]).unwrap();
    assert_eq!(Rc::strong_count(&root), 1, "модель с блоками: счётчик Rc должен быть 1");
}
