//! Фича 0229: отказ форматтера — диагностика с позицией.
//!
//! До 0229 `FormatError::Unsupported` несла **строку**, и отказ выглядел так:
//!
//! ```text
//! Ошибка форматирования 'f1.takt': печать узла 'Formula' пока не поддерживается форматтером
//! ```
//!
//! — своя форма вместо общей и **ни слова о месте**: в большом файле непокрытый
//! узел приходилось искать грепом. Позиция у узла при этом есть всегда.
//!
//! Хуже было на операторе: ветка собирала сообщение как `Statement::{other:?}`,
//! то есть печатала **`Debug`-дамп** со всей внутренней структурой узла —
//! `Statement::Assembly { loc: Source(0, 53, 71), dialect: … }`. Ровно тот класс,
//! который фича 0202 закрывала в соседней ветви (`Parse`).
//!
//! # Что здесь ловится
//!
//! 1. **Позиция указывает на сам узел** — не на начало файла и не в пустоту.
//! 2. **Форма общая с ветвью `Parse`** — `путь:строка:колонка: Ошибка компиляции
//!    [код]: текст`; формат диагностики есть её свойство (ADR 0053).
//! 3. **Внутреннее представление наружу не выходит** — текст называет **вид**
//!    узла, а не дампит его поля.
//! 4. **Ветвь `--stdin`** — код и текст есть, префикса нет: пути не существует, и
//!    выдумывать координаты нельзя.
//! 5. **Структура доступна вызывающему** — LSP получает `Diagnostic`, а не
//!    строку, и может показать отказ в редакторе.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Элемент `formula` — узел, который форматтер не печатает (строка 2, колонка 5).
const FORMULA_ELEMENT: &str = "\
model M {
    formula \"ltl\" { }
    start S { always { } }
}
start Main = M;
";

/// Оператор `assembly` в теле блока (строка 4, колонка 13) — та ветвь, что
/// печатала `Debug`-дамп.
const ASSEMBLY_STATEMENT: &str = "\
model M {
    start S {
        always {
            assembly \"x86\" { }
        }
    }
}
start Main = M;
";

/// Файл с синтаксической ошибкой — эталон формы для ветви `Parse`.
const PARSE_ERROR: &str = "\
model M {
    var x u8 := 1;
    start S { always { } }
}
start Main = M;
";

fn taktc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_taktc"))
}

/// Уникальный по тесту каталог (фича 0190: тесты идут параллельно).
fn work_dir(tag: &str) -> PathBuf {
    let thread = std::thread::current()
        .name()
        .unwrap_or("main")
        .replace(':', "_");
    let dir = std::env::temp_dir().join(format!("takt_0229_{tag}_{thread}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог теста");
    dir
}

fn fixture(tag: &str, source: &str) -> PathBuf {
    let path = work_dir(tag).join("probe.takt");
    std::fs::write(&path, source).expect("запись фикстуры");
    path
}

/// stderr и код возврата `taktc fmt --check <файл>`.
fn fmt_check(path: &Path) -> (String, i32) {
    let out = taktc()
        .arg("fmt")
        .arg("--check")
        .arg(path)
        .output()
        .expect("запуск taktc fmt --check");
    (
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

/// **Позиция указывает на сам элемент.**
#[test]
fn formula_element_refusal_points_at_the_element() {
    let path = fixture("formula", FORMULA_ELEMENT);
    let (stderr, code) = fmt_check(&path);

    assert!(
        stderr.contains("probe.takt:2:5: Ошибка компиляции [FM-001]:"),
        "ожидалась позиция элемента `formula`: {stderr:?}"
    );
    assert!(
        stderr.contains("печать узла 'formula' пока не поддерживается форматтером"),
        "текст обязан называть вид узла: {stderr:?}"
    );
    assert_eq!(code, 1, "отказ — это код 1");
}

/// **Оператор: позиция своя, и никакого `Debug`-дампа.**
///
/// Прежде здесь печаталась вся внутренняя структура узла вместе с байтовыми
/// смещениями — пользователь получал представление компилятора вместо сообщения.
#[test]
fn assembly_statement_refusal_has_position_and_no_debug_dump() {
    let path = fixture("assembly", ASSEMBLY_STATEMENT);
    let (stderr, code) = fmt_check(&path);

    assert!(
        stderr.contains("probe.takt:4:13: Ошибка компиляции [FM-001]:"),
        "позиция обязана указывать на сам оператор, а не на начало файла: {stderr:?}"
    );
    assert!(
        stderr.contains("печать узла 'assembly' пока не поддерживается форматтером"),
        "текст обязан называть вид узла: {stderr:?}"
    );
    for forbidden in [
        "Source(",
        "loc:",
        "dialect:",
        "StringLiteral",
        "Statement::",
    ] {
        assert!(
            !stderr.contains(forbidden),
            "внутреннее представление наружу не выходит ({forbidden:?}): {stderr:?}"
        );
    }
    assert_eq!(code, 1);
}

/// **Форма отказа — та же, что у ошибки разбора.**
///
/// Сравнивается не текст (он разный по существу), а форма: путь, строка,
/// колонка, вид и код на своих местах. Две ветви одного `FormatError` печатались
/// по-разному — это и был предмет фичи.
#[test]
fn refusal_shape_matches_parse_error_shape() {
    let unsupported = fmt_check(&fixture("shape_u", FORMULA_ELEMENT)).0;
    let parse = fmt_check(&fixture("shape_p", PARSE_ERROR)).0;

    let shape = |line: &str| -> Option<(String, u32, u32, String)> {
        let (head, tail) = line.split_once(": Ошибка компиляции [")?;
        let code = tail.split(']').next()?.to_string();
        let mut parts = head.rsplitn(3, ':');
        let column = parts.next()?.parse().ok()?;
        let line_no = parts.next()?.parse().ok()?;
        Some((parts.next()?.to_string(), line_no, column, code))
    };

    let u = unsupported
        .lines()
        .find_map(shape)
        .unwrap_or_else(|| panic!("отказ печати не в общей форме: {unsupported:?}"));
    let p = parse
        .lines()
        .find_map(shape)
        .unwrap_or_else(|| panic!("ошибка разбора не в общей форме: {parse:?}"));

    assert!(u.0.ends_with("probe.takt") && p.0.ends_with("probe.takt"));
    assert_eq!((u.3.as_str(), p.3.as_str()), ("FM-001", "SY-002"), "коды");
    assert!(
        u.1 > 0 && u.2 > 0 && p.1 > 0 && p.2 > 0,
        "позиции ненулевые"
    );
}

/// **`--stdin`:** код и текст есть, префикса нет — пути не существует.
#[test]
fn stdin_refusal_has_code_but_no_path_prefix() {
    let mut child = taktc()
        .arg("fmt")
        .arg("--stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("запуск taktc fmt --stdin");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(FORMULA_ELEMENT.as_bytes())
        .expect("запись в stdin");
    let out = child.wait_with_output().expect("ожидание taktc");
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        stderr.contains("Ошибка компиляции [FM-001]: печать узла 'formula'"),
        "код и текст обязаны быть и без файла: {stderr:?}"
    );
    assert!(
        !stderr.contains(".takt:"),
        "пути у stdin нет — координаты выдумывать нельзя: {stderr:?}"
    );
    assert_eq!(out.status.code(), Some(1));
}

/// **Вызывающему достаётся структура, а не строка.**
///
/// Библиотечный уровень: у отказа есть код и позиция, то есть языковой сервер
/// волен показать его диагностикой в редакторе, а не только записать в журнал.
#[test]
fn library_refusal_carries_code_and_location() {
    let err = takt_lang::format::format_source(FORMULA_ELEMENT)
        .expect_err("узел не печатается — обязан быть отказ");
    let takt_lang::format::FormatError::Unsupported(diagnostic) = err else {
        panic!("ожидалась ветвь Unsupported, получено: {err:?}");
    };

    assert_eq!(diagnostic.code.as_deref(), Some("FM-001"));
    assert!(
        matches!(
            diagnostic.loc,
            takt_lang::diagnostics::Location::Source(0, _, _)
        ),
        "позиция обязана указывать в разбираемый файл: {:?}",
        diagnostic.loc
    );
    assert!(
        diagnostic.message.contains("formula"),
        "текст называет вид узла: {:?}",
        diagnostic.message
    );
}
