//! Вычислимое приведение в инициализаторе — эталон ≡ `sv` (фича 0286).
//!
//! # Что проверяется
//!
//! `var v := 5 as u16;` цель `sv` отвергала (`SV-002`), тогда как
//! `var v: u16 := 5;` и `var v: u16 := 2 + 3;` принимала: разницу делала
//! свёртка 0192, которая приведения намеренно не берёт. Отказ шёл по **виду
//! узла**, а не по вычислимости.
//!
//! # Почему сверка значений
//!
//! Правка касается **ветви сброса**: ошибись она в значении — модуль всё равно
//! валиден и синтезируем, а регистр стартует не с того числа. `verilator` и
//! `yosys` об этом молчат; вердикт даёт трасса.

use super::*;

/// Инициализатор — приведение, значение помещается в целевой тип.
const SRC: &str = "model Probe {\n\
    \x20   var v := 5 as u16;\n\
    \x20   var n: u8 := 0;\n\n\
    \x20   start Run {\n\
    \x20       always { n := n + 1; v := v + 1; }\n\
    \x20       ref Done: n = 3;\n\
    \x20   }\n\n\
    \x20   state Done;\n\
    }\n\n\
    start Main = Probe;\n";

#[test]
fn computable_cast_initializer_matches_generated_sv() {
    if !verilator_available() {
        eprintln!("verilator недоступен — сверка `sv` пропущена");
        return;
    }
    let dir = build_dir("const_cast_init");
    let path = fixture(&dir, "castinit", SRC);

    let expected = simulate_trace(path.to_str().expect("путь в UTF-8"), &["Probe::v"]);
    let actual = sv_trace(
        &dir,
        path.to_str().expect("путь в UTF-8"),
        "castinit",
        &["castinit_probe_v"],
        4,
    );
    let common: Vec<Vec<i128>> = actual.iter().take(expected.len()).cloned().collect();
    assert_eq!(
        common, expected,
        "трасса SystemVerilog разошлась с эталоном:\nsv     = {actual:?}\nэталон = {expected:?}"
    );
    assert!(
        expected.first().is_some_and(|row| row[0] == 6),
        "контроль: на первом такте значение обязано быть 6 (5 из инициализатора + 1): {expected:?}"
    );
}

/// **Контроль границы: приведение, меняющее значение, по-прежнему отвергается.**
///
/// Правила усечения и обёртки принадлежат эталону (`takt-sim::eval`), и копии их
/// в `takt-lang` быть не должно — вычислитель берётся лишь за тождественное
/// приведение (ADR 0286).
#[test]
fn value_changing_cast_is_still_refused() {
    let src = SRC.replace("5 as u16", "300 as u8");
    let err = takt_lang::compile_to_sv(
        "castinit",
        &src,
        std::env::temp_dir().to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect_err("приведение, меняющее значение, обязано отвергаться");
    assert_eq!(err.code.as_deref(), Some("SV-002"), "{}", err.message);
}
