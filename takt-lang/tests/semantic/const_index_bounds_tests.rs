//! Константа в позиции индекса и поле у нествкатурного значения (фича 0434).
//!
//! # Что было
//!
//! Замер 2026-08-23 (запрос заказчика: «если есть константа — можно проверить
//! статически»): проверка границ смотрела на **форму записи**, а не на
//! значение.
//!
//! | Запись | До | После |
//! |---|---|---|
//! | `d[7]` при `[u8; 3]` | `SE-028` | `SE-028` |
//! | `d[K]` при `const K := 5` | **принималось молча** | `SE-028` |
//! | `x.foo` при `x: u8` | **принималось молча** | `SE-030` |
//!
//! Цена молчания измерена: эталон отвечал `SIM-010`/`SIM-012` **в такте**, а
//! прошивка цели `c` читала **за границей массива** (трасса `2 3 3 1 0` — то
//! есть соседние поля структуры), при нулевом коде возврата `taktc` и
//! молчащем `cc -Wall -Wextra -Werror`.

use takt_lang::parse;
use takt_lang::semantic::tree::construct_model;

/// Компилирует исходник и отдаёт код диагностики (либо `None` при успехе).
fn code_of(src: &str) -> Option<String> {
    let (ast, _) = parse(src, 0).expect("разбор");
    match construct_model(&ast, None, &[]) {
        Ok(_) => None,
        Err(d) => Some(d.code.unwrap_or_default()),
    }
}

/// Предмет: индекс-КОНСТАНТА за границей отвергается так же, как литерал.
#[test]
fn const_index_out_of_bounds_is_rejected() {
    let src = "const K: u8 := 5;\n\
               var d: [u8; 3] := {1, 2, 3};\n\
               out p: u8 at 0;\n\
               start R { always { p := d[K]; } ref R; }\n";
    assert_eq!(code_of(src).as_deref(), Some("SE-028"));
}

/// **Контроль:** та же запись в границах принимается.
///
/// Без него «отвергается» значило бы лишь «константа в индексе запрещена».
#[test]
fn const_index_inside_bounds_is_accepted() {
    let src = "const K: u8 := 1;\n\
               var d: [u8; 3] := {1, 2, 3};\n\
               out p: u8 at 0;\n\
               start R { always { p := d[K]; } ref R; }\n";
    assert_eq!(code_of(src), None, "индекс 1 в массиве из трёх законен");
}

/// **Граница:** ПЕРЕМЕННАЯ в индексе статически не судится.
///
/// Её значение известно только в такте — там отвечает `SIM-010` эталона (а по
/// фиче 0433 будет отвечать guard порождённого кода).
#[test]
fn variable_index_is_not_judged_statically() {
    let src = "var d: [u8; 3] := {1, 2, 3};\n\
               var i: u8 := 9;\n\
               out p: u8 at 0;\n\
               start R { always { p := d[i]; } ref R; }\n";
    assert_eq!(
        code_of(src),
        None,
        "значение переменной известно лишь в такте"
    );
}

/// Литеральный индекс за границей отвергается по-прежнему — контроль 0028.
#[test]
fn literal_index_out_of_bounds_is_rejected() {
    let src = "var d: [u8; 3] := {1, 2, 3};\n\
               out p: u8 at 0;\n\
               start R { always { p := d[7]; } ref R; }\n";
    assert_eq!(code_of(src).as_deref(), Some("SE-028"));
}

/// Предмет: поле у значения, которое структурой НЕ является.
#[test]
fn field_of_non_struct_is_rejected() {
    let src = "var x: u8 := 3;\n\
               out p: u8 at 0;\n\
               start R { always { p := x.nosuch; } ref R; }\n";
    assert_eq!(code_of(src).as_deref(), Some("SE-030"));
}

/// **Контроль:** разряд по НОМЕРУ остаётся законным.
///
/// `x.3` — не поле: член здесь число, и судит его `SE-125` (0394).
#[test]
fn bit_by_number_stays_legal() {
    let src = "var x: u8 := 3;\n\
               out p: u8 at 0;\n\
               start R { always { p := x.3; } ref R; }\n";
    assert_eq!(code_of(src), None, "разряд по номеру — не обращение к полю");
}

/// **Контроль:** поле настоящей структуры принимается.
#[test]
fn field_of_struct_is_accepted() {
    let src = "struct S { a: u8 }\n\
               var s: S := {1};\n\
               out p: u8 at 0;\n\
               start R { always { p := s.a; } ref R; }\n";
    assert_eq!(code_of(src), None);
}
