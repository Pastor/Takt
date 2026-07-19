//! Позиции в диагностиках: файл, строка, колонка (фича 0053).
//!
//! Тесты — на то, **что увидит пользователь**: путь файла в диагностике и
//! координаты. Прежде позиция не печаталась вовсе, и ошибка внутри
//! импортированной библиотеки была неотличима от своей — обе давали дословно
//! `Ошибка компиляции [SE-002]: Ссылка 'Nowhere' не найдена`.

use grammar::diagnostics::{FileTable, Location, line_column};

const DIR: &str = "tests/data/diag53";

/// Компилирует фикстуру и возвращает диагностику (фикстуры заведомо ошибочны).
fn error_of(fixture: &str) -> grammar::diagnostics::Diagnostic {
    let path = format!("{DIR}/{fixture}");
    let source = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let out = std::env::temp_dir().join("lam_diag53_out.c");
    grammar::compile_to_c(
        &path,
        &source,
        out.to_str().expect("путь"),
        &[DIR.to_string()],
        &grammar::GenerateOptions::default(),
    )
    .expect_err("фикстура обязана быть ошибочной")
}

// ─── Путь файла в диагностике (A1–A3) ────────────────────────────────────────

/// A1: ошибка своего файла названа своим файлом.
#[test]
fn error_in_own_file_names_own_file() {
    let d = error_of("lib_bad.lam");
    assert_eq!(d.file.as_deref(), Some("tests/data/diag53/lib_bad.lam"));
}

/// A2: ошибка ВНУТРИ импортированного файла названа именем библиотеки, а не
/// импортёра. Это и есть суть фичи.
#[test]
fn error_inside_import_names_the_library() {
    let d = error_of("importer.lam");
    assert_eq!(
        d.file.as_deref(),
        Some("tests/data/diag53/lib_bad.lam"),
        "виновник — библиотека; импортёр её не писал и чинить не вправе"
    );
}

/// A3: вложенный импорт (top → mid → deep) называет САМЫЙ ВНУТРЕННИЙ файл.
///
/// Сторож правила «первый проставивший выигрывает»: затирание пути на каждом
/// уровне всплытия дало бы имя импортёра вместо имени виновника.
#[test]
fn nested_import_names_the_deepest_file() {
    let d = error_of("top.lam");
    assert_eq!(d.file.as_deref(), Some("tests/data/diag53/deep_bad.lam"));
}

/// Координаты указывают на место ошибки, а не на начало файла.
#[test]
fn position_points_at_the_offending_reference() {
    let d = error_of("lib_bad.lam");
    let Location::Source(_, start, _) = d.loc else {
        panic!("ожидалась файловая позиция, получено {:?}", d.loc);
    };
    let text = std::fs::read_to_string("tests/data/diag53/lib_bad.lam").expect("чтение");
    let (line, column) = line_column(&text, start as usize);
    assert_eq!(line, 4, "ссылка 'Nowhere' — на 4-й строке");
    assert!(
        column > 1,
        "колонка указывает внутрь строки, а не на её начало"
    );
}

/// Настоящий `file_no`: файлы получают РАЗНЫЕ номера.
///
/// Прежде номер везде был нулём — из-за этого диагностику из импорта нельзя
/// было отличить от своей.
#[test]
fn imported_file_gets_its_own_file_no() {
    let d = error_of("importer.lam");
    let Location::Source(file_no, _, _) = d.loc else {
        panic!("ожидалась файловая позиция");
    };
    assert_ne!(file_no, 0, "0 — корневой файл; ошибка пришла из импорта");
}

// ─── Реестр файлов ───────────────────────────────────────────────────────────

#[test]
fn file_table_registers_root_as_zero() {
    let files = FileTable::new("main.lam");
    assert_eq!(files.path(0), Some("main.lam"));
}

/// Один путь — один номер: номер обозначает файл, а не факт загрузки.
#[test]
fn file_table_deduplicates_paths() {
    let mut files = FileTable::new("main.lam");
    let first = files.add("lib.lam");
    let second = files.add("lib.lam");
    assert_eq!(first, second);
    assert_ne!(first, 0);
}

#[test]
fn file_table_returns_none_for_unknown_and_non_source() {
    let files = FileTable::new("main.lam");
    assert_eq!(files.path(42), None);
    assert_eq!(files.path_of(&Location::Codegen), None);
    assert_eq!(files.path_of(&Location::Implicit), None);
}

/// T1 (фикс [0053-01]): импорт **никогда** не получает номер корня — при любом
/// способе создания реестра.
///
/// Прежде `Default` создавал пустой реестр, и первый импорт получал `0` — тот же
/// номер, что корень у `new`. Позиция из импортированного файла становилась
/// неотличима от корневой: дефект, который фича 0053 как раз закрывала.
///
/// [0053-01]: ../../docs/fixes/0053-01-file-table-default-collision.md
#[test]
fn file_table_default_never_gives_import_the_root_number() {
    let mut files = FileTable::default();
    assert_ne!(
        files.add("lib.lam"),
        0,
        "номер 0 означает «корень» и импорту достаться не может"
    );
}

/// Слот `0` реестра без корня занят, но путь честно неизвестен.
///
/// Альтернатива («корень = первый добавленный») и была дефектом: реестр выдавал
/// бы чужой файл за корневой.
#[test]
fn file_table_default_reports_unknown_root() {
    let files = FileTable::default();
    assert_eq!(
        files.path(0),
        None,
        "корень неизвестен — врать о нём нельзя"
    );
}

/// T2: у реестра с корнем нумерация не изменилась.
#[test]
fn file_table_new_keeps_root_at_zero_and_import_at_one() {
    let mut files = FileTable::new("main.lam");
    assert_eq!(files.path(0), Some("main.lam"));
    assert_eq!(files.add("lib.lam"), 1);
}

// ─── Строка и колонка ────────────────────────────────────────────────────────

/// Нумерация с единицы — как в rustc/gcc (внутри Location смещения с нуля).
#[test]
fn line_column_counts_from_one() {
    assert_eq!(line_column("abc", 0), (1, 1));
    assert_eq!(line_column("abc\ndef", 4), (2, 1));
    assert_eq!(line_column("abc\ndef", 6), (2, 3));
}

/// Колонка — в СИМВОЛАХ, а не в байтах: в `.lam` есть кириллица (комментарии,
/// строки), и байтовая колонка указывала бы мимо.
#[test]
fn line_column_counts_characters_not_bytes() {
    let text = "// абв\nstart S;";
    let offset = text.find("start").expect("есть");
    assert_eq!(line_column(text, offset), (2, 1));
    // Внутри кириллицы: 'в' — третий символ после "// ".
    let inside = text.find('в').expect("есть");
    assert_eq!(line_column(text, inside), (1, 6));
}

/// Смещение за концом текста не паникует.
#[test]
fn line_column_clamps_offset_past_end() {
    assert_eq!(line_column("ab", 99), (1, 3));
}
