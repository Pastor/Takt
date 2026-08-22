//! Фича 0229: отказ форматтера — диагностика с позицией. Обновлено фичей 0405.
//!
//! До 0229 `FormatError::Unsupported` несла **строку**, и отказ выглядел так:
//!
//! ```text
//! Ошибка форматирования 'f1.takt': печать узла 'Formula' пока не поддерживается форматтером
//! ```
//!
//! — своя форма вместо общей и **ни слова о месте**. Хуже было на операторе:
//! ветка собирала сообщение как `Statement::{other:?}`, то есть печатала
//! `Debug`-дамп со всей внутренней структурой узла.
//!
//! # Почему набор переписан (фича 0405)
//!
//! Прежние входы — элемент `formula` и оператор `assembly` — были **примерами
//! непечатаемых узлов**. Фича 0405 завела их печать, и вместе с ней исчез
//! последний достижимый вход, дающий `FM-001`: оставшиеся ветви отказа
//! (`Expression::CodeBlock`, `Expression::NamedFunction`, `Type::Function`,
//! `Statement::Args`, `Statement::StraySemicolon`, ошибка в блоке `formula`)
//! грамматикой **не строятся** — это узлы без правила, класс фичи 0201.
//!
//! Поэтому проверки разделены по достижимости:
//!
//! - форма печати диагностики (путь, строка, колонка, код) проверяется на
//!   ветви `Parse` — она достижима;
//! - свойства самого отказа `FM-001` (код, позиция узла, текст без
//!   `Debug`-дампа) — юнит-тестами модуля `format`, где конструктор виден;
//! - здесь же стоит **контроль**: прежние примеры непечатаемых узлов теперь
//!   печатаются, и это утверждение сторожится, а не подразумевается.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Элемент `formula` — прежний пример непечатаемого узла (фича 0405 его печатает).
const FORMULA_ELEMENT: &str = "\
model M {
    formula \"ltl\" { }
    start S { always { } }
}
start Main = M;
";

/// Оператор `assembly` в теле блока — та ветвь, что печатала `Debug`-дамп.
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

/// Файл с синтаксической ошибкой — единственная достижимая ветвь отказа.
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

fn work_dir(tag: &str) -> PathBuf {
    // Каталог уникален по тесту: прогон параллельный, а помощник начинает с
    // очистки (инвариант фичи 0190). Имя потока несёт `::` — его чистим (0244).
    let thread = std::thread::current()
        .name()
        .unwrap_or("unnamed")
        .replace(':', "_");
    let dir = std::env::temp_dir().join(format!("takt_fmt_unsupported_{thread}_{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("создание временного каталога");
    dir
}

fn fixture(tag: &str, source: &str) -> PathBuf {
    let path = work_dir(tag).join("probe.takt");
    std::fs::write(&path, source).expect("запись пробы");
    path
}

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

/// **Контроль:** прежние примеры непечатаемых узлов теперь печатаются.
///
/// Без него набор молча превратился бы в кладбище: тесты, требовавшие отказа,
/// сняты, а утверждение, которое их заменило, нигде бы не проверялось.
#[test]
fn formula_and_assembly_are_printed_now() {
    for (tag, source) in [
        ("formula_ok", FORMULA_ELEMENT),
        ("assembly_ok", ASSEMBLY_STATEMENT),
    ] {
        let printed = takt_lang::format::format_source(source)
            .unwrap_or_else(|e| panic!("{tag}: печать обязана удаваться: {e:?}"));
        assert!(!printed.is_empty(), "{tag}: вывод пуст");
        // Отказ ушёл насовсем: результат разбирается обратно.
        takt_lang::parse(&printed, 0).unwrap_or_else(|e| panic!("{tag}: {e:?}"));
    }
}

/// **Форма печати диагностики:** путь, строка, колонка, код — на своих местах.
///
/// Проверяется на ветви `Parse`: она единственная достижимая. Формат общий у
/// обеих ветвей `FormatError` — это и было предметом фичи 0229.
#[test]
fn refusal_shape_carries_path_line_column_and_code() {
    let (stderr, code) = fmt_check(&fixture("shape_p", PARSE_ERROR));

    let shape = |line: &str| -> Option<(String, u32, u32, String)> {
        let (head, tail) = line.split_once(": Ошибка компиляции [")?;
        let diagnostic = tail.split(']').next()?.to_string();
        let mut parts = head.rsplitn(3, ':');
        let column = parts.next()?.parse().ok()?;
        let line_no = parts.next()?.parse().ok()?;
        Some((parts.next()?.to_string(), line_no, column, diagnostic))
    };

    let parsed = stderr
        .lines()
        .find_map(shape)
        .unwrap_or_else(|| panic!("ошибка разбора не в общей форме: {stderr:?}"));

    assert!(parsed.0.ends_with("probe.takt"), "путь: {parsed:?}");
    assert_eq!(parsed.3, "SY-002", "код: {parsed:?}");
    assert!(
        parsed.1 > 0 && parsed.2 > 0,
        "позиции ненулевые: {parsed:?}"
    );
    assert_eq!(code, 1, "отказ — это код 1");
}

/// **`--stdin`:** код и текст есть, префикса пути нет — файла не существует.
#[test]
fn stdin_refusal_has_code_but_no_path_prefix() {
    use std::io::Write;
    use std::process::Stdio;

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
        .write_all(PARSE_ERROR.as_bytes())
        .expect("запись в stdin");
    let out = child.wait_with_output().expect("ожидание taktc");
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        stderr.contains("Ошибка компиляции [SY-002]"),
        "код обязан быть и без файла: {stderr:?}"
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
    let err = takt_lang::format::format_source(PARSE_ERROR)
        .expect_err("исходник не разбирается — обязан быть отказ");
    let takt_lang::format::FormatError::Parse(diagnostics) = err else {
        panic!("ожидалась ветвь Parse, получено: {err:?}");
    };
    let first = diagnostics.first().expect("диагностика есть");

    assert_eq!(first.code.as_deref(), Some("SY-002"));
    assert!(
        matches!(first.loc, takt_lang::diagnostics::Location::Source(0, _, _)),
        "позиция обязана указывать в разбираемый файл: {:?}",
        first.loc
    );
}
