//! Координата отказа цели `c` — место употребления (фича 0277).
//!
//! # Что здесь сторожится
//!
//! У выражения нет позиции **употребления**: `ExpressionNode::loc()` выводит её
//! из объявлений операндов (решение 0056 — не тащить координату через ~40
//! вариантов ради одного потребителя). Поэтому отказ цели указывал на
//! объявление:
//!
//! | Вход | Координата до фичи | После |
//! |---|---|---|
//! | `res := mem[1:2];` в строке 7 (`mem` объявлена в строке 1) | **`1:1`** | `7:9` |
//!
//! ⚠️ **Это ложь, а не пустота:** координата выглядит настоящей, и автор идёт
//! читать строку объявления. Тот же класс, что 0264 и 0276.
//!
//! Позицию несёт оператор (`StatementNode::Expression`, фича 0264); цель берёт
//! её из общего носителя `generator::site::StatementSite`, не меняя ни
//! `ExpressionNode`, ни сигнатуры рекурсивных печатников.
//!
//! ⚠️ **Границы объёма названы:** цели `rust`, `st`, `sv` печатают отказ с
//! `Location::Codegen`, то есть **без** координаты — это честная пустота, а не
//! ложь; их случай вынесен кандидатом. Отказ `CC-023` (узел не прошёл
//! понижение) недостижим из корректной программы (0236) и координаты тоже не
//! меняет.

use std::path::PathBuf;
use takt_lang::diagnostics::Location;
use takt_lang::generator::GenerateOptions;

/// Срез массива в СКАЛЯРНЫЙ приёмник: `res: u8` элементов не имеет, поэтому
/// поэлементная форма (фича 0355) неприменима и цель отвечает `CC-022`.
/// Эталон такой вход тоже не исполняет (`SIM-006` «значение массив нельзя
/// привести к типу u8»), то есть пример остаётся непереводимым по существу.
const SLICE: &str = "var mem: [u8; 4] := { 0, 0, 0, 0 };\n\
                     var res: u8 := 0;\n\
                     \n\
                     start Run {\n\
                     \x20   always {\n\
                     \x20       mem[0] := 1;\n\
                     \x20       res := mem[1:2];\n\
                     \x20   }\n\
                     \x20   ref Run: res < 3;\n\
                     }\n";

fn build_dir(tag: &str) -> PathBuf {
    let thread = std::thread::current()
        .name()
        .unwrap_or("single")
        .replace(':', "_");
    let dir = std::env::temp_dir()
        .join(format!("takt_pid{}", std::process::id()))
        .join(format!("takt_0277_{thread}_{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("создание каталога");
    dir
}

/// Отказ цели `c` на исходнике.
fn refusal(tag: &str, source: &str) -> takt_lang::diagnostics::Diagnostic {
    let dir = build_dir(tag);
    takt_lang::compile_to_c(
        tag,
        source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &GenerateOptions::default(),
    )
    .expect_err("вход обязан отвергаться")
}

/// Строка по смещению диагностики.
fn line_of(src: &str, diagnostic: &takt_lang::diagnostics::Diagnostic) -> usize {
    let Location::Source(_, start, _) = diagnostic.loc else {
        panic!("у отказа нет позиции в исходнике: {:?}", diagnostic.loc);
    };
    src[..start as usize].matches('\n').count() + 1
}

/// **T1.** Отказ указывает на строку употребления, а не объявления.
#[test]
fn refusal_points_at_the_usage_line() {
    let err = refusal("slice", SLICE);
    assert_eq!(err.code.as_deref(), Some("CC-022"), "код отказа");
    assert_eq!(
        line_of(SLICE, &err),
        7,
        "координата обязана указывать на строку `res := mem[1:2];`, а не на объявление `mem`"
    );
}

/// **T2.** Отказ ВНЕ оператора чужой координаты не получает.
///
/// Инициализатор объявления печатается до тела, оператор ещё не начат — и
/// координата остаётся такой, какой была (у `CC-017` её нет вовсе). Без этой
/// проверки правка «всегда брать позицию оператора» подставила бы сюда место
/// последнего оператора предыдущей модели — ту же ложь, от которой фича уходит.
#[test]
fn refusal_outside_a_statement_gets_no_borrowed_position() {
    const SCALAR_INIT: &str = "var mem: [u8; 4] := 0;\n\
                               var n: u8 := 0;\n\
                               \n\
                               start Run {\n\
                               \x20   always { n := mem[0]; }\n\
                               \x20   ref Run: n < 3;\n\
                               }\n";
    let err = refusal("scalar_init", SCALAR_INIT);
    assert_eq!(err.code.as_deref(), Some("CC-017"));
    assert!(
        matches!(err.loc, Location::Codegen),
        "отказ вне оператора обязан остаться без координаты, а не занять чужую: {:?}",
        err.loc
    );
}

/// **T3.** Позиция не «залипает»: следующий оператор объявляет своё место.
///
/// Носитель — изменяемое состояние на время генерации, и это его главный риск:
/// не обнови печатник операторов позицию, все последующие отказы указывали бы
/// на первый оператор модели.
#[test]
fn position_follows_the_current_statement() {
    const LATER: &str = "var mem: [u8; 4] := { 0, 0, 0, 0 };\n\
                         var res: u8 := 0;\n\
                         \n\
                         start Run {\n\
                         \x20   always {\n\
                         \x20       res := 1;\n\
                         \x20       res := 2;\n\
                         \x20       res := 3;\n\
                         \x20       res := mem[1:2];\n\
                         \x20   }\n\
                         \x20   ref Run: res < 3;\n\
                         }\n";
    let err = refusal("later", LATER);
    assert_eq!(
        line_of(LATER, &err),
        9,
        "координата обязана указывать на последний оператор, а не на первый"
    );
}
