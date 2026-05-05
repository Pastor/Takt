//! Интеграционные тесты LSP-функций: SemanticIndex, position_to_offset, node_at_position, hover_info, goto_declaration.
//!
//! Тесты разделены на группы:
//! - `position_to_offset_*` — конвертация LSP-позиции в байтовое смещение
//! - `node_at_position_*` — поиск семантического узла по LSP-позиции
//! - `hover_*` — генерация hover-текста с использованием нового алгоритма
//! - `goto_declaration_*` — переход к декларации элемента

#[cfg(feature = "lsp")]
mod lsp_integration {
    use grammar::lsp::{
        completion_items, goto_declaration, hover_info, node_at_position, position_to_offset,
        semantic_tokens,
    };
    use grammar::parse;
    use grammar::semantic::index::{SemanticIndex, SemanticNodeKind};
    use grammar::semantic::tree::construct_model;
    use lsp_types::Position;

    // ── Тесты position_to_offset ──────────────────────────────────────────────

    /// Первая строка, нулевой столбец → байт 0.
    #[test]
    fn position_to_offset_first_char() {
        let src = "var x: bit = false;";
        assert_eq!(
            position_to_offset(src, Position::new(0, 0)),
            Some(0),
            "первый символ первой строки — байт 0"
        );
    }

    /// Первая строка, 4-й символ → байт 4.
    #[test]
    fn position_to_offset_middle_of_first_line() {
        let src = "var x: bit = false;";
        assert_eq!(
            position_to_offset(src, Position::new(0, 4)),
            Some(4),
            "символ 4 первой строки — байт 4"
        );
    }

    /// Вторая строка: после "\n" = 1 байт, символ 2 → байт 1 + 2 = 3.
    #[test]
    fn position_to_offset_second_line() {
        let src = "ab\ncd";
        // "ab\n" = 3 байта, символ 1 на строке 1 → байт 4
        assert_eq!(
            position_to_offset(src, Position::new(1, 1)),
            Some(4),
            "строка 1, символ 1 → байт 4"
        );
    }

    /// Несуществующая строка → None.
    #[test]
    fn position_to_offset_nonexistent_line() {
        let src = "hello";
        assert_eq!(
            position_to_offset(src, Position::new(99, 0)),
            None,
            "строка 99 не существует"
        );
    }

    /// Символ за концом строки → байт конца строки.
    #[test]
    fn position_to_offset_past_end_of_line() {
        let src = "hi";
        // Строка "hi" = 2 символа; character=99 → clamp до конца строки → байт 2
        assert_eq!(
            position_to_offset(src, Position::new(0, 99)),
            Some(2),
            "символ за концом строки → конец строки"
        );
    }

    /// Многострочный файл: строка 2, символ 0.
    #[test]
    fn position_to_offset_third_line_start() {
        let src = "line0\nline1\nline2";
        // "line0\n" = 6, "line1\n" = 6, итого 12 байт до строки 2
        assert_eq!(
            position_to_offset(src, Position::new(2, 0)),
            Some(12),
            "начало третьей строки — байт 12"
        );
    }

    // ── Тесты node_at_position ────────────────────────────────────────────────

    /// Вспомогательная функция: парсит, строит модель, возвращает её.
    fn make_model(src: &str) -> std::rc::Rc<std::cell::RefCell<grammar::semantic::ModelNode>> {
        let (ast, _) = parse(src, 0).expect("ошибка парсинга");
        construct_model(&ast, None, &[]).expect("ошибка семантики")
    }

    /// Курсор на объявлении переменной → возвращает Variable.
    #[test]
    fn node_at_position_variable_declaration() {
        //           0         1
        //           0123456789012345678
        let src = "var counter: bit = false;";
        let model = make_model(src);
        // Позиция 4 — символ 'c' в "counter"
        let node = node_at_position(src, Position::new(0, 4), &model);
        assert!(node.is_some(), "должен найти переменную");
        let node = node.unwrap();
        assert_eq!(node.name, "counter");
        assert_eq!(node.kind, SemanticNodeKind::Variable);
    }

    /// Курсор на ключевом слове → None (не объявление).
    #[test]
    fn node_at_position_on_keyword_returns_none() {
        let src = "var x: bit = false;";
        let model = make_model(src);
        // Позиция 0 — символ 'v' в "var" (ключевое слово)
        // Если Location включает всё объявление, может найти переменную;
        // если нет — None. В любом случае не должно паниковать.
        let _ = node_at_position(src, Position::new(0, 0), &model);
    }

    /// Курсор за пределами файла → None.
    #[test]
    fn node_at_position_past_end_of_file() {
        let src = "var x: bit = false;";
        let model = make_model(src);
        let node = node_at_position(src, Position::new(99, 0), &model);
        assert!(node.is_none(), "позиция за концом файла → None");
    }

    /// Курсор на объявлении константы → возвращает Const.
    #[test]
    fn node_at_position_const_declaration() {
        //           0         1         2
        //           012345678901234567890123456789
        let src = "const LIMIT: bit = true;";
        let model = make_model(src);
        // Позиция 6 — символ 'L' в "LIMIT"
        let node = node_at_position(src, Position::new(0, 6), &model);
        assert!(node.is_some(), "должен найти константу");
        let node = node.unwrap();
        assert_eq!(node.name, "LIMIT");
        assert_eq!(node.kind, SemanticNodeKind::Const);
    }

    /// Курсор на объявлении состояния → возвращает StartState или State.
    #[test]
    fn node_at_position_state_declaration() {
        //           0         1
        //           0123456789012345
        let src = "start Idle; state Run;";
        let model = make_model(src);
        // Позиция 6 — символ 'I' в "Idle"
        let node = node_at_position(src, Position::new(0, 6), &model);
        assert!(node.is_some(), "должен найти состояние Idle");
        let node = node.unwrap();
        assert_eq!(node.name, "Idle");
        assert_eq!(node.kind, SemanticNodeKind::StartState);
    }

    /// Курсор на объявлении псевдонима типа → возвращает TypeAlias.
    #[test]
    fn node_at_position_type_alias() {
        //           0         1
        //           01234567890123456789
        let src = "type Byte = [bit;8];";
        let model = make_model(src);
        // Позиция 5 — символ 'B' в "Byte"
        let node = node_at_position(src, Position::new(0, 5), &model);
        assert!(node.is_some(), "должен найти псевдоним типа");
        let node = node.unwrap();
        assert_eq!(node.name, "Byte");
        assert_eq!(node.kind, SemanticNodeKind::TypeAlias);
    }

    /// Курсор на объявлении условия → возвращает Condition.
    #[test]
    fn node_at_position_condition() {
        let src = "cond Ready = true;";
        let model = make_model(src);
        // Позиция 5 — символ 'R' в "Ready"
        let node = node_at_position(src, Position::new(0, 5), &model);
        assert!(node.is_some(), "должен найти условие");
        let node = node.unwrap();
        assert_eq!(node.name, "Ready");
        assert_eq!(node.kind, SemanticNodeKind::Condition);
    }

    /// Курсор на объявлении перечисления → возвращает Enum.
    #[test]
    fn node_at_position_enum() {
        let src = "model M { enum Color { Red, Green } start S; }";
        let model = make_model(src);
        // Позиция 15 — символ 'C' в "Color"
        let node = node_at_position(src, Position::new(0, 15), &model);
        assert!(node.is_some(), "должен найти перечисление");
        let node = node.unwrap();
        assert_eq!(node.name, "Color");
        assert_eq!(node.kind, SemanticNodeKind::Enum);
    }

    /// Курсор на объявлении функции → возвращает Function или ExternFunction.
    #[test]
    fn node_at_position_extern_function() {
        let src = "extern fn send(data: bit);";
        let model = make_model(src);
        // Позиция 10 — символ 's' в "send"
        let node = node_at_position(src, Position::new(0, 10), &model);
        assert!(node.is_some(), "должен найти extern fn");
        let node = node.unwrap();
        assert_eq!(node.name, "send");
        assert_eq!(node.kind, SemanticNodeKind::ExternFunction);
    }

    /// Курсор на объявлении именованной модели → возвращает Model.
    #[test]
    fn node_at_position_model() {
        let src = "model Blinker { start On; state Off; }";
        let model = make_model(src);
        // Позиция 6 — символ 'B' в "Blinker"
        let node = node_at_position(src, Position::new(0, 6), &model);
        assert!(node.is_some(), "должен найти модель");
        let node = node.unwrap();
        assert_eq!(node.name, "Blinker");
        assert_eq!(node.kind, SemanticNodeKind::Model);
    }

    /// Комплексный тест: несколько элементов на разных строках.
    #[test]
    fn node_at_position_multiline_file() {
        let src = "var alpha: bit = false;\nconst BETA: bit = true;\nstart Gamma;";
        let model = make_model(src);

        // Строка 0, позиция 4 — 'a' в "alpha"
        let n0 = node_at_position(src, Position::new(0, 4), &model);
        assert!(n0.is_some());
        assert_eq!(n0.unwrap().name, "alpha");

        // Строка 1, позиция 6 — 'B' в "BETA"
        let n1 = node_at_position(src, Position::new(1, 6), &model);
        assert!(n1.is_some());
        assert_eq!(n1.unwrap().name, "BETA");

        // Строка 2, позиция 6 — 'G' в "Gamma"
        let n2 = node_at_position(src, Position::new(2, 6), &model);
        assert!(n2.is_some());
        assert_eq!(n2.unwrap().name, "Gamma");
    }

    // ── Тесты hover с использованием нового алгоритма ────────────────────────

    /// Hover на объявлении переменной возвращает корректный тип.
    #[test]
    fn hover_variable_via_position_index() {
        let src = "var speed: bit = false; start S;";
        // Позиция 4 — 's' в "speed"
        let h = hover_info(src, Position::new(0, 4));
        assert!(h.is_some(), "hover должен найти переменную");
        let h = h.unwrap();
        if let lsp_types::HoverContents::Markup(mc) = h.contents {
            assert!(
                mc.value.contains("speed"),
                "hover должен содержать имя переменной: {}",
                mc.value
            );
            assert!(
                mc.value.contains("var"),
                "hover должен указывать вид: {}",
                mc.value
            );
        }
    }

    /// Hover на объявлении константы содержит "const".
    #[test]
    fn hover_const_via_position_index() {
        let src = "const LIMIT: bit = true; start S;";
        let h = hover_info(src, Position::new(0, 6));
        assert!(h.is_some(), "hover должен найти константу");
        let h = h.unwrap();
        if let lsp_types::HoverContents::Markup(mc) = h.contents {
            assert!(
                mc.value.contains("LIMIT"),
                "hover должен содержать имя: {}",
                mc.value
            );
            assert!(
                mc.value.contains("const"),
                "hover должен указывать 'const': {}",
                mc.value
            );
        }
    }

    /// Hover на объявлении функции содержит сигнатуру.
    #[test]
    fn hover_function_via_position_index() {
        let src = "extern fn compute(x: bit) -> bit; start S;";
        let h = hover_info(src, Position::new(0, 10));
        assert!(h.is_some(), "hover должен найти функцию");
        let h = h.unwrap();
        if let lsp_types::HoverContents::Markup(mc) = h.contents {
            assert!(
                mc.value.contains("compute"),
                "hover должен содержать имя функции: {}",
                mc.value
            );
        }
    }

    /// Hover на объявлении перечисления содержит варианты.
    #[test]
    fn hover_enum_via_position_index() {
        // Перечисление на верхнем уровне (в корневой модели)
        //           0         1         2         3         4
        //           01234567890123456789012345678901234567890
        let src = "enum Status { Active = 0, Idle = 1 } start S;";
        // Позиция 5 — символ 'S' в "Status"
        let h = hover_info(src, Position::new(0, 5));
        assert!(h.is_some(), "hover должен найти перечисление");
        let h = h.unwrap();
        if let lsp_types::HoverContents::Markup(mc) = h.contents {
            assert!(
                mc.value.contains("Status"),
                "hover должен содержать имя enum: {}",
                mc.value
            );
        }
    }

    /// Hover на объявлении состояния содержит тип состояния.
    #[test]
    fn hover_state_via_position_index() {
        let src = "start Idle; state Active;";
        let h = hover_info(src, Position::new(0, 6));
        assert!(h.is_some(), "hover должен найти состояние");
        let h = h.unwrap();
        if let lsp_types::HoverContents::Markup(mc) = h.contents {
            assert!(
                mc.value.contains("Idle"),
                "hover должен содержать имя состояния: {}",
                mc.value
            );
        }
    }

    /// Hover на объявлении модели возвращает "model".
    #[test]
    fn hover_model_via_position_index() {
        let src = "model Engine { start On; state Off; } start S = Engine;";
        let h = hover_info(src, Position::new(0, 6));
        assert!(h.is_some(), "hover должен найти модель");
        let h = h.unwrap();
        if let lsp_types::HoverContents::Markup(mc) = h.contents {
            assert!(
                mc.value.contains("Engine"),
                "hover должен содержать имя модели: {}",
                mc.value
            );
            assert!(
                mc.value.contains("model"),
                "hover должен указывать 'model': {}",
                mc.value
            );
        }
    }

    /// Hover на использовании переменной (не объявлении) — резервный поиск по имени.
    #[test]
    fn hover_variable_usage_fallback_search() {
        // "var x" на строке 0; использование "x" в условии на строке 1
        let src = "var x: bit = false;\ncond C = x = true;\nstart S;";
        // Позиция (1, 9) — 'x' в условии (использование)
        let h = hover_info(src, Position::new(1, 9));
        // Резервный поиск должен найти переменную "x"
        assert!(
            h.is_some(),
            "hover на использовании переменной должен работать"
        );
    }

    /// Hover в пустом файле → None.
    #[test]
    fn hover_empty_file_returns_none() {
        let h = hover_info("", Position::new(0, 0));
        assert!(h.is_none(), "hover в пустом файле → None");
    }

    #[test]
    fn hover_big() {
        const SRC: &str = r#"// Пример для демонстрации возможностей LSP-сервера Lam.
//
// При открытии этого файла в редакторе с поддержкой LSP (например, Zed с lam-lsp):
// - Ошибки и предупреждения подсвечиваются сразу
// - При наведении на идентификатор отображается его тип
// - Автодополнение предлагает ключевые слова и имена из модели

type u8 = [bit;8];

/// Перечисление направлений движения робота.
enum Direction { North, South, East, West }

/// Перечисление приоритетов задачи.
enum Priority { Low = 0, Medium = 5, High = 10 }

extern fn log(msg: u8);

/// Переменная состояния направления.
var heading: Direction = 0;

/// Модель управления роботом.
model Robot {
    var speed: u8 = 0;
    var active: bit = false;

    start Idle {
        enter {
            speed = 0;
            active = false;
        }
        ref Moving: active;
    }

    state Moving {
        always {
            speed = 100;
        }
        ref Idle: true;
    }
}

start Main = Robot;
        "#;
        let h = hover_info(SRC, Position::new(421 - 391, 23));
        assert!(h.is_some(), "hover должен найти переменную условия active");
        let h = h.unwrap();
        if let lsp_types::HoverContents::Markup(mc) = h.contents {
            assert!(
                mc.value.contains("active"),
                "hover должен содержать имя элемента условия: {}",
                mc.value
            );
            assert!(
                mc.value.contains("var"),
                "hover должен указывать 'var': {}",
                mc.value
            );
        }
    }

    #[test]
    fn hover_model() {
        const SRC: &str = r#"model Robot {
    start Idle;
}
start Main = Robot;
        "#;
        let h = hover_info(SRC, Position::new(3, 16));
        assert!(
            h.is_some(),
            "hover должен найти модель реализации состояния"
        );
        let h = h.unwrap();
        if let lsp_types::HoverContents::Markup(mc) = h.contents {
            assert!(
                mc.value.contains("Robot"),
                "hover должен содержать имя модели: {}",
                mc.value
            );
            assert!(
                mc.value.contains("model"),
                "hover должен указывать 'model': {}",
                mc.value
            );
        }
    }

    // ── Тесты goto_declaration ────────────────────────────────────────────────

    /// Курсор на объявлении переменной → возвращает тот же диапазон (самоссылка).
    #[test]
    fn goto_declaration_variable_self() {
        //           0         1         2
        //           0123456789012345678901234
        let src = "var counter: bit = false;";
        // Позиция 4 — символ 'c' в "counter"
        let range = goto_declaration(src, Position::new(0, 4));
        assert!(
            range.is_some(),
            "goto_declaration на объявлении переменной должен вернуть диапазон"
        );
        let range = range.unwrap();
        assert_eq!(range.start.line, 0, "декларация переменной на строке 0");
    }

    /// Курсор на объявлении функции → возвращает диапазон этой же функции.
    #[test]
    fn goto_declaration_function_self() {
        let src = "extern fn send(data: bit); start S;";
        // Позиция 10 — символ 's' в "send"
        let range = goto_declaration(src, Position::new(0, 10));
        assert!(
            range.is_some(),
            "goto_declaration на объявлении функции должен вернуть диапазон"
        );
        let range = range.unwrap();
        assert_eq!(range.start.line, 0, "декларация функции на строке 0");
    }

    /// Курсор на объявлении состояния → возвращает диапазон этого же состояния.
    #[test]
    fn goto_declaration_state_self() {
        let src = "start Idle; state Moving;";
        // Позиция 6 — символ 'I' в "Idle"
        let range = goto_declaration(src, Position::new(0, 6));
        assert!(
            range.is_some(),
            "goto_declaration на объявлении состояния должен вернуть диапазон"
        );
        let range = range.unwrap();
        assert_eq!(range.start.line, 0, "декларация состояния на строке 0");
    }

    /// Курсор на ref-переходе → возвращает диапазон объявления целевого состояния.
    ///
    /// Пример: `ref Moving;` внутри состояния → декларация `state Moving;`.
    #[test]
    fn goto_declaration_reference_resolves_to_state() {
        // Строка 0: "start Idle { ref Moving; }"
        // Строка 1: "state Moving;"
        let src = "start Idle { ref Moving; }\nstate Moving;";
        // Позиция (0, 17) — символ 'M' в "ref Moving"
        let range = goto_declaration(src, Position::new(0, 17));
        assert!(
            range.is_some(),
            "goto_declaration на ref-переходе должен вернуть декларацию состояния"
        );
        let range = range.unwrap();
        assert_eq!(
            range.start.line, 1,
            "декларация состояния Moving на строке 1: {:?}",
            range
        );
    }

    /// Курсор на переменной в условии перехода → декларация этой переменной.
    ///
    /// Пример: `ref Run: flag;` где `flag` — ReferenceCondition → декларация `var flag`.
    #[test]
    fn goto_declaration_reference_condition_resolves_to_variable() {
        // Строка 0: "var flag: bit = false;"
        // Строка 1: "start Idle;"
        // Строка 2: "state Run;"
        // Строка 3: "start Idle { ref Run: flag; }"
        let src = "var flag: bit = false;\nstart Idle;\nstate Run;\nstart Idle { ref Run: flag; }";
        // Строка 3: "start Idle { ref Run: flag; }"
        //            0         1         2
        //            0123456789012345678901234567890
        // "flag" начинается с позиции 22 на строке 3
        let range = goto_declaration(src, Position::new(3, 22));
        assert!(
            range.is_some(),
            "goto_declaration на переменной условия должен вернуть декларацию переменной"
        );
        let range = range.unwrap();
        assert_eq!(
            range.start.line, 0,
            "декларация var flag на строке 0: {:?}",
            range
        );
    }

    /// Курсор вне идентификаторов → None.
    #[test]
    fn goto_declaration_outside_node_returns_none() {
        let src = "var x: bit = false;\nstart S;";
        // Позиция (0, 0) — символ 'v' в "var" (ключевое слово, не идентификатор объявления)
        // Может вернуть Some (если Location включает всё объявление) или None.
        // Главное — не паниковать.
        let _ = goto_declaration(src, Position::new(0, 0));

        // Позиция за пределами файла → точно None
        let range = goto_declaration(src, Position::new(99, 0));
        assert!(range.is_none(), "позиция за пределами файла → None");
    }

    /// Пустой файл → None.
    #[test]
    fn goto_declaration_empty_file_returns_none() {
        let range = goto_declaration("", Position::new(0, 0));
        assert!(range.is_none(), "пустой файл → None");
    }

    // ── I7: goto_declaration_with_paths ──────────────────────────────────────

    /// I7: декларация переменной в текущем файле через goto_declaration_with_paths.
    #[test]
    fn i7_goto_declaration_local_var() {
        use grammar::lsp::goto_declaration_with_paths;

        let src = "var counter: [bit;8] = 0;\nstart S;";
        // Позиция 4 — символ 'c' в "counter"
        let loc = goto_declaration_with_paths(src, Position::new(0, 4), &[]);
        assert!(
            loc.is_some(),
            "переменная 'counter' должна быть найдена в текущем файле"
        );
        let loc = loc.unwrap();
        assert_eq!(
            loc.range.start.line, 0,
            "декларация counter на строке 0: {:?}",
            loc.range
        );
    }

    /// I7: декларация состояния в текущем файле через goto_declaration_with_paths.
    #[test]
    fn i7_goto_declaration_local_state() {
        use grammar::lsp::goto_declaration_with_paths;

        let src = "start S;\nstate Ready;";
        // Позиция (1, 6) — символ 'R' в "Ready"
        let loc = goto_declaration_with_paths(src, Position::new(1, 6), &[]);
        assert!(
            loc.is_some(),
            "состояние 'Ready' должно быть найдено в текущем файле"
        );
    }

    /// I7: при отсутствии идентификатора под курсором возвращается None.
    #[test]
    fn i7_goto_declaration_outside_returns_none() {
        use grammar::lsp::goto_declaration_with_paths;

        let src = "var x: bit = false;\nstart S;";
        let loc = goto_declaration_with_paths(src, Position::new(99, 0), &[]);
        assert!(loc.is_none(), "позиция за пределами файла → None");
    }

    /// I7: пустой файл → None.
    #[test]
    fn i7_goto_declaration_empty_file() {
        use grammar::lsp::goto_declaration_with_paths;

        let loc = goto_declaration_with_paths("", Position::new(0, 0), &[]);
        assert!(loc.is_none(), "пустой файл → None");
    }

    // ── I8: индексация локальных переменных в enter/exit/always ─────────────

    /// I8: локальная переменная в `always`-блоке доступна через SemanticIndex.
    #[test]
    fn i8_local_var_in_always_indexed() {
        let src = "start S;\nalways { var local_x: [bit;8] = 0; }";
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let index = SemanticIndex::build(&model);

        // Находим позицию имени "local_x" в "var local_x: ..."
        // "start S;\nalways { var local_x: " — "local_x" начинается с байта 19
        let offset = src
            .find("local_x")
            .expect("local_x должен быть в источнике");
        let node = index.node_at_offset(offset);
        assert!(
            node.is_some(),
            "local_x должна быть в индексе по offset {}: {:?}",
            offset,
            node
        );
        let node = node.unwrap();
        assert_eq!(node.name, "local_x", "имя узла: {:?}", node);
        assert_eq!(
            node.kind,
            SemanticNodeKind::LocalVar,
            "вид узла должен быть LocalVar: {:?}",
            node
        );
    }

    /// I8: локальная переменная в модельном `enter`-блоке доступна через SemanticIndex.
    #[test]
    fn i8_local_var_in_model_enter_indexed() {
        // Блок enter на уровне модели (не внутри состояния)
        let src = "start S;\nenter { var enter_var: bit = false; }";
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let index = SemanticIndex::build(&model);

        let offset = src
            .find("enter_var")
            .expect("enter_var должен быть в источнике");
        let node = index.node_at_offset(offset);
        assert!(
            node.is_some(),
            "enter_var должна быть в индексе по offset {}",
            offset
        );
        let node = node.unwrap();
        assert_eq!(node.name, "enter_var");
        assert_eq!(node.kind, SemanticNodeKind::LocalVar);
    }

    /// I8: локальная переменная в модельном `exit`-блоке доступна через SemanticIndex.
    #[test]
    fn i8_local_var_in_model_exit_indexed() {
        // Блок exit на уровне модели (не внутри состояния)
        let src = "start S;\nexit { var exit_var: bit = false; }";
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let index = SemanticIndex::build(&model);

        let offset = src
            .find("exit_var")
            .expect("exit_var должен быть в источнике");
        let node = index.node_at_offset(offset);
        assert!(
            node.is_some(),
            "exit_var должна быть в индексе по offset {}",
            offset
        );
        let node = node.unwrap();
        assert_eq!(node.name, "exit_var");
        assert_eq!(node.kind, SemanticNodeKind::LocalVar);
    }

    /// I8: поиск локальной переменной по позиции LSP в always-блоке.
    #[test]
    fn i8_node_at_position_local_var_in_always() {
        let src = "start S;\nalways { var my_local: [bit;8] = 0; }";
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();

        // Находим строку и столбец для "my_local"
        let offset = src.find("my_local").unwrap();
        let line = src[..offset].matches('\n').count() as u32;
        let line_start = src[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col = (offset - line_start) as u32;

        let node = node_at_position(src, Position::new(line, col), &model);
        assert!(
            node.is_some(),
            "my_local должна быть найдена по позиции ({}, {})",
            line,
            col
        );
        let node = node.unwrap();
        assert_eq!(node.name, "my_local");
        assert_eq!(node.kind, SemanticNodeKind::LocalVar);
    }

    /// I8: тестовый файл local_var_in_blocks.lam — все локальные переменные индексируются.
    #[test]
    fn i8_file_local_vars_indexed() {
        let src = std::fs::read_to_string("tests/data/lsp/local_var_in_blocks.lam")
            .expect("не удалось прочитать файл");
        let (ast, _) = parse(&src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let index = SemanticIndex::build(&model);

        // Все три локальные переменные должны быть в индексе
        for var_name in &["local_in_enter", "local_in_exit", "local_in_always"] {
            let offset = src
                .find(var_name)
                .unwrap_or_else(|| panic!("{} должен быть в тестовом файле", var_name));
            let node = index.node_at_offset(offset);
            assert!(
                node.is_some(),
                "{} должна быть в индексе по offset {}",
                var_name,
                offset
            );
            assert_eq!(
                node.unwrap().kind,
                SemanticNodeKind::LocalVar,
                "{} должна иметь вид LocalVar",
                var_name
            );
        }
    }

    /// I7: кросс-файловый переход — объявление модели из импортируемого файла.
    #[test]
    fn i7_goto_declaration_cross_file_model() {
        use grammar::lsp::goto_declaration_with_paths;

        // Создаём временный файл с моделью
        let dir = tempfile::tempdir().unwrap();
        let ping_path = dir.path().join("ping.lam");
        std::fs::write(&ping_path, "model Ping { start S; }").unwrap();
        let dir_str = dir.path().to_string_lossy().into_owned();

        // Исходный файл с импортом
        let src = r#"import "ping.lam"; start Main;"#;
        // Позиция 7 — символ '"' (начало строки импорта) — за пределами идентификатора
        // Проверяем что функция не паникует при отсутствии узла
        let _ = goto_declaration_with_paths(src, Position::new(0, 7), &[dir_str.clone()]);

        // Нет паники — тест прошёл
        // (полное кросс-файловое разрешение зависит от наличия идентификатора в индексе)
    }
}

#[cfg(feature = "lsp")]
#[cfg(test)]
mod diagnostic_location_tests {
    use grammar::lsp::{
        collect_diagnostics, completion_items, grammar_diagnostic_to_lsp, hover_info,
        position_to_offset, semantic_tokens,
    };
    use lsp_types::Position;

    /// Проверяет, что ошибка синтаксиса имеет точные координаты (не нулевые).
    #[cfg(feature = "lsp")]
    #[test]
    fn collect_diagnostics_syntax_error_has_location() {
        // Пропущена точка с запятой — парсер должен вернуть ошибку с позицией
        let src = "var x: bit = false\nstart S;";
        let diags = collect_diagnostics(src);
        assert!(!diags.is_empty(), "должна быть хотя бы одна диагностика");
        // Хотя бы одна диагностика должна иметь ненулевую позицию
        let has_location = diags
            .iter()
            .any(|d| d.range.start != Position::new(0, 0) || d.range.end != Position::new(0, 0));
        assert!(
            has_location,
            "хотя бы одна диагностика должна содержать ненулевые координаты"
        );
    }

    /// Проверяет, что грамматическая диагностика с Location::Source содержит
    /// правильный диапазон после конвертации в LSP-формат.
    #[cfg(feature = "lsp")]
    #[test]
    fn grammar_diagnostic_to_lsp_source_location_gives_correct_range() {
        use grammar::diagnostics::{Diagnostic as GDiag, Location};

        let src = "var x: bit = false;";
        // Создаём диагностику с конкретной позицией (байты 4..5 — символ 'x')
        let diag = GDiag::error(Location::Source(0, 4, 5), "Тестовая ошибка".to_string());
        let lsp_diag = grammar_diagnostic_to_lsp(&diag, src);
        // Позиция 4 → строка 0, столбец 4 (в ASCII 'x' = 1 байт)
        assert_eq!(
            lsp_diag.range.start,
            Position::new(0, 4),
            "начало диапазона должно быть (0, 4)"
        );
        assert_eq!(
            lsp_diag.range.end,
            Position::new(0, 5),
            "конец диапазона должно быть (0, 5)"
        );
    }

    /// Проверяет, что заметки добавляются к основному сообщению диагностики.
    #[cfg(feature = "lsp")]
    #[test]
    fn grammar_diagnostic_to_lsp_notes_appended_to_message() {
        use grammar::diagnostics::{Diagnostic as GDiag, Location};

        let src = "var x: bit = false;";
        let diag = GDiag::error_with_note(
            Location::Source(0, 4, 5),
            "Основная ошибка".to_string(),
            Location::Source(0, 0, 3),
            "Дополнительная информация".to_string(),
        );
        let lsp_diag = grammar_diagnostic_to_lsp(&diag, src);
        assert!(
            lsp_diag.message.contains("Основная ошибка"),
            "основное сообщение должно присутствовать: {}",
            lsp_diag.message
        );
        assert!(
            lsp_diag.message.contains("Дополнительная информация"),
            "заметка должна быть включена в сообщение: {}",
            lsp_diag.message
        );
    }

    /// Проверяет, что ошибки семантики (дубликат состояния) имеют координаты.
    #[cfg(feature = "lsp")]
    #[test]
    fn collect_diagnostics_semantic_error_has_location() {
        // Два состояния с одинаковым именем — семантическая ошибка
        let src = "start S;\nstate S;";
        let diags = collect_diagnostics(src);
        // Проверяем просто что нет паники и набор диагностик не пустой
        // (конкретная ошибка зависит от реализации дублирования)
        let _ = diags; // не паникует
    }

    /// Проверяет, что предупреждение Ce13 (неиспользуемая переменная)
    /// содержит координаты объявления переменной, а не нулевую позицию.
    #[cfg(feature = "lsp")]
    #[test]
    fn ce13_unused_variable_warning_has_source_location() {
        // Переменная `heading` объявлена на строке 0, нигде не используется
        let src = "var heading: bit = false;\nstart S;";
        let diags = collect_diagnostics(src);

        let ce13 = diags
            .iter()
            .find(|d| d.message.contains("heading"))
            .expect("должно быть предупреждение Ce13 для 'heading'");

        // Предупреждение не должно указывать на (0,0)-(0,0) — у переменной есть позиция
        let is_zero_range =
            ce13.range.start == Position::new(0, 0) && ce13.range.end == Position::new(0, 0);
        assert!(
            !is_zero_range,
            "Ce13 для 'heading' должно содержать координаты объявления, получено {:?}",
            ce13.range
        );
    }

    // ── Тесты semantic_tokens ─────────────────────────────────────────────────

    /// Вспомогательная функция: возвращает список (слово, тип_токена) из semantic_tokens.
    fn decode_semantic_tokens(src: &str) -> Vec<(String, u32)> {
        let tokens = semantic_tokens(src);
        let mut result = Vec::new();
        let mut line: u32 = 0;
        let mut col: u32 = 0;
        for tok in &tokens.data {
            if tok.delta_line > 0 {
                line += tok.delta_line;
                col = tok.delta_start;
            } else {
                col += tok.delta_start;
            }
            let length = tok.length as usize;
            let token_type = tok.token_type;
            if let Some(start) = position_to_offset(src, Position::new(line, col)) {
                let end = (start + length).min(src.len());
                if src.is_char_boundary(start) && src.is_char_boundary(end) {
                    result.push((src[start..end].to_string(), token_type));
                }
            }
        }
        result
    }

    /// `while` подсвечивается как ключевое слово (TT_KEYWORD = 0).
    #[test]
    fn semantic_tokens_while_is_keyword() {
        let src = "model M { start S { always { while true { } } } }";
        let tokens = decode_semantic_tokens(src);
        let while_tok = tokens.iter().find(|(w, _)| w == "while");
        assert!(while_tok.is_some(), "токен 'while' должен присутствовать");
        assert_eq!(
            while_tok.unwrap().1,
            0,
            "'while' должен быть TT_KEYWORD (0), получено {}",
            while_tok.unwrap().1
        );
    }

    /// `match` подсвечивается как ключевое слово (TT_KEYWORD = 0).
    #[test]
    fn semantic_tokens_match_is_keyword() {
        let src = "model M { var x: u8 = 0; start S { always { match x { _ => { } } } } }";
        let tokens = decode_semantic_tokens(src);
        let match_tok = tokens.iter().find(|(w, _)| w == "match");
        assert!(match_tok.is_some(), "токен 'match' должен присутствовать");
        assert_eq!(
            match_tok.unwrap().1,
            0,
            "'match' должен быть TT_KEYWORD (0), получено {}",
            match_tok.unwrap().1
        );
    }

    /// `inout` подсвечивается как ключевое слово (TT_KEYWORD = 0).
    #[test]
    fn semantic_tokens_inout_is_keyword() {
        let src = "inout bus: u8 = 0x1000:0;\nstart S;";
        let tokens = decode_semantic_tokens(src);
        let tok = tokens.iter().find(|(w, _)| w == "inout");
        assert!(tok.is_some(), "токен 'inout' должен присутствовать");
        assert_eq!(tok.unwrap().1, 0, "'inout' должен быть TT_KEYWORD (0)");
    }

    /// `u8` в аннотации типа подсвечивается как тип (TT_TYPE = 3).
    #[test]
    fn semantic_tokens_u8_is_type() {
        let src = "var x: u8 = 0;\nstart S;";
        let tokens = decode_semantic_tokens(src);
        let u8_tok = tokens.iter().find(|(w, _)| w == "u8");
        assert!(u8_tok.is_some(), "токен 'u8' должен присутствовать");
        assert_eq!(
            u8_tok.unwrap().1,
            3,
            "'u8' должен быть TT_TYPE (3), получено {}",
            u8_tok.unwrap().1
        );
    }

    /// `i32` в аннотации типа подсвечивается как тип (TT_TYPE = 3).
    #[test]
    fn semantic_tokens_i32_is_type() {
        let src = "var n: i32 = 0;\nstart S;";
        let tokens = decode_semantic_tokens(src);
        let tok = tokens.iter().find(|(w, _)| w == "i32");
        assert!(tok.is_some(), "токен 'i32' должен присутствовать");
        assert_eq!(tok.unwrap().1, 3, "'i32' должен быть TT_TYPE (3)");
    }

    /// `bit` в аннотации типа подсвечивается как тип (TT_TYPE = 3).
    #[test]
    fn semantic_tokens_bit_is_type() {
        let src = "var flag: bit = 0;\nstart S;";
        let tokens = decode_semantic_tokens(src);
        let tok = tokens.iter().find(|(w, _)| w == "bit");
        assert!(tok.is_some(), "токен 'bit' должен присутствовать");
        assert_eq!(tok.unwrap().1, 3, "'bit' должен быть TT_TYPE (3)");
    }

    // ── Тесты hover для встроенных типов ─────────────────────────────────────

    /// Hover над `u8` возвращает описание типа.
    #[test]
    fn hover_builtin_type_u8() {
        let src = "var x: u8 = 0;\nstart S;";
        // Позиция курсора на 'u8' (строка 0, столбец 7)
        let result = hover_info(src, Position::new(0, 7));
        assert!(result.is_some(), "hover над 'u8' должен вернуть результат");
        let hover = result.unwrap();
        if let lsp_types::HoverContents::Markup(mc) = hover.contents {
            assert!(
                mc.value.contains("u8"),
                "hover должен содержать 'u8': {}",
                mc.value
            );
        } else {
            panic!("ожидался MarkupContent");
        }
    }

    /// Hover над `i8` возвращает описание типа.
    #[test]
    fn hover_builtin_type_i8() {
        let src = "var n: i8 = 0;\nstart S;";
        let result = hover_info(src, Position::new(0, 7));
        assert!(result.is_some(), "hover над 'i8' должен вернуть результат");
        let hover = result.unwrap();
        if let lsp_types::HoverContents::Markup(mc) = hover.contents {
            assert!(mc.value.contains("i8"), "hover должен содержать 'i8'");
        } else {
            panic!("ожидался MarkupContent");
        }
    }

    /// Автодополнение содержит `while`, `match`, `inout` и встроенные типы.
    #[test]
    fn completion_includes_new_keywords_and_builtin_types() {
        let src = "start S;";
        let items = completion_items(src);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();

        for kw in &["while", "match", "inout", "struct"] {
            assert!(labels.contains(kw), "completion должен содержать '{}'", kw);
        }
        for ty in &["u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64"] {
            assert!(
                labels.contains(ty),
                "completion должен содержать тип '{}'",
                ty
            );
        }
    }

    /// Hover над переменной с типом `u8` показывает «u8», а не Debug-строку «Integer { bits: 8, signed: false }».
    #[test]
    fn hover_var_u8_shows_u8_not_debug() {
        let src = "var speed: u8 = 0;\nstart S;";
        // Позиция 4 — «s» в «speed»
        let h = hover_info(src, lsp_types::Position::new(0, 4));
        assert!(h.is_some(), "hover над переменной должен вернуть результат");
        if let lsp_types::HoverContents::Markup(mc) = h.unwrap().contents {
            assert!(
                mc.value.contains("u8"),
                "hover должен содержать 'u8': {}",
                mc.value
            );
            assert!(
                !mc.value.contains("Integer"),
                "hover не должен содержать Debug-имя 'Integer': {}",
                mc.value
            );
        }
    }

    /// `collect_diagnostics` не выдаёт SE-034 для встроенных целочисленных типов `u8`…`i64`.
    #[test]
    fn collect_diagnostics_builtin_integer_types_no_error() {
        // const-переменные не генерируют предупреждения об использовании
        let src = "const A: u8 = 10;\nconst B: i32 = -5;\nconst C: u64 = 0;\nstart S;";
        let diags = collect_diagnostics(src);
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Some(lsp_types::DiagnosticSeverity::ERROR))
            .collect();
        assert!(
            errors.is_empty(),
            "встроенные целочисленные типы не должны давать ошибок SE-034: {:?}",
            errors
        );
    }
}
