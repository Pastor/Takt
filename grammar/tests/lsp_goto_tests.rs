//! Кросс-файловый переход к декларации (фича 0056).
//!
//! Вынесен из `lsp_tests.rs` фичей 0027: тот файл сверх лимита размера, и
//! храповик `scripts/check-module-size.sh` не дал ему вырасти. Ровно то, что
//! советует сообщение отказа: «вынесите новое в отдельный модуль».

/// Кросс-файловый переход к декларации (фича 0056).
///
/// Фикстуры — `tests/data/goto56/`, пара «угадывание сработало бы / не сработало
/// бы» (правило 16). Проверяется **поведение** по нажатию «перейти к
/// декларации»: какой файл откроется и на каком месте, — а не наличие `file_no`
/// в индексе: ровно из-за отсутствия потребителя ветка угадывания и прожила
/// непроверенной.
#[cfg(feature = "lsp")]
mod goto_exact_file {
    use lsp_types::Position;

    // ── 0056: кросс-файловый переход к декларации ────────────────────────────
    //
    // Фикстуры — `tests/data/goto56/`, пара «угадывание сработало бы / не
    // сработало бы» (правило 16). Проверяется ПОВЕДЕНИЕ по нажатию «перейти к
    // декларации»: какой файл откроется и на каком месте, — а не наличие
    // `file_no` в индексе: ровно из-за отсутствия потребителя ветка угадывания и
    // прожила непроверенной.

    /// Каталог фикстур 0056.
    fn goto56_dir() -> String {
        "tests/data/goto56".to_string()
    }

    fn goto56_source(file: &str) -> String {
        std::fs::read_to_string(format!("tests/data/goto56/{file}"))
            .unwrap_or_else(|e| panic!("не прочитать фикстуру {file}: {e}"))
    }

    /// Позиция курсора на первом использовании `needle` **после** `= ` — то есть
    /// на имени модели в `start Main = X;`, а не на алиасе в строке `import`.
    fn cursor_on_use(source: &str, needle: &str) -> Position {
        let offset = source
            .find(&format!("= {needle}"))
            .unwrap_or_else(|| panic!("в фикстуре нет использования `= {needle}`"))
            + 2;
        let head = &source[..offset];
        let line = head.matches('\n').count() as u32;
        let col = head.rsplit('\n').next().map_or(0, |l| l.chars().count()) as u32;
        Position::new(line, col)
    }

    /// T3 (сторож зонда): узел под курсором — из **текущего** файла.
    ///
    /// Зонд фичи 0056: курсор на `Helper` возвращал переменную `speed` **из
    /// `helper.lam`** — её диапазон (19..37) там накрыл смещение курсора (35)
    /// здесь. Не «не тот файл», а **не тот узел**.
    ///
    /// ⚠️ Тест сначала доказывает, что **ловушка взведена**: в чужом файле есть
    /// узел, чей диапазон накрывает смещение курсора. Без этой проверки правка
    /// фикстуры (например, добавленный комментарий сдвинет смещения) молча
    /// разоружила бы сторожа, и он зеленел бы впустую.
    #[test]
    fn t3_node_under_cursor_belongs_to_current_file() {
        use grammar::diagnostics::{FileTable, Location};
        use grammar::semantic::index::SemanticIndex;
        use grammar::semantic::tree::construct_model_with_files;

        let source = goto56_source("uses_helper.lam");
        let offset = source.find("= Helper").expect("нет использования") + 2;

        let (ast, _) = grammar::parse(&source, 0).expect("разбор");
        let mut files = FileTable::new("uses_helper.lam");
        let model =
            construct_model_with_files(&ast, None, &[goto56_dir()], &mut files).expect("семантика");
        let index = SemanticIndex::build(&model);

        // Шаг 1. Ловушка взведена? В чужом файле обязан быть узел, накрывающий
        // смещение курсора, — иначе путать нечего и сторож зеленеет впустую.
        let foreign = index.node_at_offset_in_file(1, offset).expect(
            "ЛОВУШКА РАЗОРУЖЕНА: в helper.lam нет узла, накрывающего смещение курсора. \
             Верните фикстуры к виду, где смещения пересекаются",
        );
        assert!(
            matches!(foreign.loc, Location::Source(1, _, _)),
            "поиск по файлу 1 вернул узел не из файла 1 — ФИЛЬТР ПО ФАЙЛУ СЛОМАН: {:?}",
            foreign.loc
        );
        assert_eq!(
            foreign.name, "speed",
            "ЛОВУШКА РАЗОРУЖЕНА: ожидался узел `speed` из зонда 0056 (его диапазон \
             19..37 в helper.lam накрывает смещение курсора здесь)"
        );

        // Шаг 2. То, ради чего сторож: под курсором — узел СВОЕГО файла.
        let node = index.node_at_offset(offset).expect("узел под курсором");
        assert!(
            matches!(node.loc, Location::Source(0, _, _)),
            "узел под курсором обязан принадлежать корневому файлу: {:?}",
            node.loc
        );
        assert_eq!(
            node.name, "Helper",
            "под курсором обязана быть ссылка на модель текущего файла"
        );
    }

    /// T5/A1: переход открывает импортированный файл на объявлении модели.
    #[test]
    fn t5_goto_opens_imported_file() {
        use grammar::lsp::goto_declaration_at;

        let source = goto56_source("uses_helper.lam");
        let pos = cursor_on_use(&source, "Helper");
        let loc = goto_declaration_at("uses_helper.lam", &source, pos, &[goto56_dir()])
            .expect("переход на имени импортированной модели обязан находиться");

        assert!(
            loc.uri.ends_with("goto56/helper.lam"),
            "обязан открыться helper.lam, получено: {}",
            loc.uri
        );
        assert_eq!(
            loc.range.start.line, 0,
            "объявление модели — с первой строки"
        );
    }

    /// T6/A2: **контрпример угадыванию** — алиас.
    ///
    /// `import "engine.lam" as Motor;` связывает имя `Motor` с файлом
    /// `engine.lam`. Прежний код строил кандидатов из ИМЕНИ МОДЕЛИ
    /// (`to_snake_case("Motor")` → `motor.lam`) и не нашёл бы цель **никогда**.
    #[test]
    fn t6_goto_follows_alias_not_model_name() {
        use grammar::lsp::goto_declaration_at;

        let source = goto56_source("uses_alias.lam");
        let pos = cursor_on_use(&source, "Motor");
        let loc = goto_declaration_at("uses_alias.lam", &source, pos, &[goto56_dir()])
            .expect("переход по алиасу обязан находиться");

        assert!(
            loc.uri.ends_with("goto56/engine.lam"),
            "обязан открыться engine.lam (а не выдуманный motor.lam), получено: {}",
            loc.uri
        );
        assert!(
            !loc.uri.contains("motor"),
            "имя файла угадывалось из имени модели — этого больше быть не должно: {}",
            loc.uri
        );
    }

    /// T8/R4: диапазон в чужом файле считается по **его** тексту.
    ///
    /// Смещения чужого файла к своему тексту не относятся: наложи их на текущий
    /// документ — получишь мусорный диапазон (так и было до 0056).
    #[test]
    fn t8_range_in_foreign_file_uses_its_own_text() {
        use grammar::lsp::goto_declaration_at;

        let source = goto56_source("uses_alias.lam");
        let target = goto56_source("engine.lam");
        let pos = cursor_on_use(&source, "Motor");
        let loc = goto_declaration_at("uses_alias.lam", &source, pos, &[goto56_dir()])
            .expect("переход обязан находиться");

        // `model Engine` объявлена после шапки-комментария — на 4-й строке.
        let declaration_line = target
            .lines()
            .position(|l| l.starts_with("model Engine"))
            .expect("в engine.lam нет объявления модели") as u32;
        assert_eq!(
            loc.range.start.line, declaration_line,
            "диапазон обязан указывать на объявление в engine.lam (строка {}), \
             а не на случайное место: {:?}",
            declaration_line, loc.range
        );
    }

    /// T7/A4/R6: переход внутри своего файла не изменился — URI пуст.
    ///
    /// Пустой `uri` — контракт «это текущий файл»: его подставляет вызывающий
    /// (ADR 0056, A4).
    #[test]
    fn t7_goto_within_own_file_keeps_contract() {
        use grammar::lsp::goto_declaration_at;

        let src = "var counter: [bit;8] := 0;\nstart S;";
        let loc = goto_declaration_at("main.lam", src, Position::new(0, 4), &[])
            .expect("переменная своего файла обязана находиться");
        assert!(
            loc.uri.is_empty(),
            "для своего файла URI подставляет вызывающий: {}",
            loc.uri
        );
        assert_eq!(loc.range.start.line, 0);
    }

    /// T9/A5: угадывание удалено. Каталог целиком — 0027 разделила `lsp.rs`, и
    /// проверка одного пути после переезда функции молча зеленела бы.
    #[test]
    fn t9_guessing_is_gone() {
        let src: String = std::fs::read_dir("src/lsp")
            .expect("нет каталога src/lsp")
            .filter_map(|e| std::fs::read_to_string(e.ok()?.path()).ok())
            .collect();
        assert!(!src.is_empty(), "каталог src/lsp пуст — проверять нечего");
        assert!(
            !src.contains("fn to_snake_case"),
            "`to_snake_case` (угадывание файла по имени модели) обязана быть \
             удалена: путь берётся из реестра файлов (0053)"
        );
    }

    /// T10/A6: сервер зовёт кросс-файловый вариант.
    ///
    /// Иначе проверять нечего: код без потребителя молча гниёт (урок 0010/0049),
    /// а вся работа 0056 осталась бы невидимой в редакторе.
    #[test]
    fn t10_server_calls_cross_file_goto() {
        let server =
            std::fs::read_to_string("src/bin/lam_lsp.rs").expect("не прочитать src/bin/lam_lsp.rs");
        assert!(
            server.contains("goto_declaration_at("),
            "сервер обязан звать кросс-файловый вариант с путём документа"
        );
        assert!(
            !server.contains("lsp::goto_declaration(text"),
            "однофайловый вариант в обработчике declaration больше не место: \
             URI ответа был бы всегда текущим документом"
        );
    }
}
