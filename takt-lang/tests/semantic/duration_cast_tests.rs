//! Приведение к `duration` и обратно вычисляется при компиляции (фича 0318).
//!
//! # Что было
//!
//! Мост «число ↔ длительность» задан ADR 0134 (единица — **миллисекунда**), и
//! пересчёт делает единственный носитель `semantic::duration`. Компилятор его
//! не звал, и обе стороны моста расходились. Замер 2026-08-20:
//!
//! | Вход | Эталон | Цели |
//! |---|---|---|
//! | `var v: duration := 250 as duration;` | `250ms` | семь переводят, **`sv`/`sv-mmio` — `SV-002`** |
//! | `var ms: u32 := D as u32;` (`const D := 250ms`) | `250` | `st`, `rust` переводят; **`c`/`c-hal` — `CC-023`**, `sv` — `SV-002` |
//!
//! Вторая строка хуже: `CC-023` означает «узел не прошёл понижение», то есть
//! код дефекта инструмента, а не ответ автору.

use takt_lang::parse;
use takt_lang::semantic::tree::construct_model;

fn value_of(src: &str, name: &str) -> Result<String, takt_lang::diagnostics::Diagnostic> {
    let (ast, _) = parse(src, 0).expect("разбор");
    let model = construct_model(&ast, None, &[])?;
    let borrowed = model.borrow();
    Ok(format!(
        "{:?}",
        borrowed.variables.get(name).expect("объявление")
    ))
}

/// Предмет: число к длительности — мост через миллисекунды.
#[test]
fn integer_to_duration_is_folded() {
    let text = value_of(
        "var v: duration := 250 as duration;\nstart Run { ref Run; }\n",
        "v",
    )
    .expect("вход законен");
    assert!(
        text.contains("250000000"),
        "инициализатор обязан свернуться в наносекунды:\n{text}"
    );
}

/// Обратное направление — длительность к целому — тем же мостом.
#[test]
fn duration_to_integer_is_folded() {
    let text = value_of(
        "const D: duration := 250ms;\nvar v: u32 := D as u32;\nstart Run { ref Run; }\n",
        "v",
    )
    .expect("вход законен");
    assert!(
        text.contains("250"),
        "инициализатор обязан свернуться в миллисекунды:\n{text}"
    );
}

/// **Контроль:** литерал длительности работает как прежде.
///
/// Без него «приведение считается» означало бы «свёртка трогает всё подряд».
#[test]
fn plain_duration_literal_is_unchanged() {
    let text =
        value_of("var v: duration := 250ms;\nstart Run { ref Run; }\n", "v").expect("вход законен");
    assert!(text.contains("250000000"), "{text}");
}

/// **Граница:** миллисекунды, не помещающиеся в целевой тип, судит правило
/// целого (`SE-121`, фича 0310) — второго знания о переносе не заводится.
#[test]
fn overflowing_duration_uses_the_integer_rule() {
    let err = value_of(
        "const D: duration := 250ms;\nvar v: i8 := D as i8;\nstart Run { ref Run; }\n",
        "v",
    )
    .expect_err("250 не помещается в i8");
    assert_eq!(err.code.as_deref(), Some("SE-121"), "{err:?}");
}
