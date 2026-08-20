//! Сдвиг на величину, не меньшую ширины типа, в цели `rust` (фича 0326).
//!
//! # Что было
//!
//! `var a: i8 := -8; v := a >> 8;` — цель печатала `self.a >> 8`, и **`rustc`
//! отвергал** такой код: «attempt to shift right by `8_i32`, which would
//! overflow». Код возврата `taktc` при этом **ноль** — класс «инструмент
//! рапортует об успехе, а вывод невалиден» (0262, 0287).
//!
//! Замер 2026-08-20: эталон даёт **−1** (знак заполняет разряды) и **0** для
//! беззнакового; цель `c` печатает сдвиг как есть и даёт то же (операнды
//! продвигаются до `int`), `st` — floor-деление, `sv` — `>>>` (проверено
//! прогоном verilator: `-1`).

use std::path::PathBuf;
use takt_lang::generator::GenerateOptions;

const SRC: &str = "var a: i8 := -8;\nvar b: u8 := 200;\nvar v: i8 := 0;\nvar w: u8 := 0;\n\
     out probe: i8 at 0;\n\
     start Run { always { v := a >> 8; w := b >> 8; probe := v; } ref Run: w < 100; }\n";

fn out_dir(tag: &str) -> PathBuf {
    let thread = std::thread::current()
        .name()
        .unwrap_or("single")
        .replace(':', "_");
    let dir = std::env::temp_dir().join(format!("takt_shift_{tag}_{thread}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог вывода");
    dir
}

fn generate(tag: &str, src: &str) -> String {
    let dir = out_dir(tag);
    takt_lang::compile_to_rust(
        tag,
        src,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &GenerateOptions::default(),
    )
    .expect("порождение Rust");
    std::fs::read_to_string(dir.join(format!("{tag}.rs"))).expect("чтение вывода")
}

/// Предмет: сдвиг на ширину типа печатается выразимой формой.
///
/// Знаковый — сдвиг на `ширина − 1` (там остаётся только знак, то есть −1 либо
/// 0); беззнаковый — `0`.
#[test]
fn shift_by_type_width_is_printed_as_saturated() {
    let text = generate("shiftw", SRC);
    assert!(
        text.contains(">> 7"),
        "знаковый сдвиг обязан насыщаться до ширины−1:\n{text}"
    );
    assert!(
        !text.contains(">> 8"),
        "сдвиг на всю ширину rustc отвергает — его в выводе быть не должно:\n{text}"
    );
}

/// **Контроль:** обычный сдвиг печатается как прежде.
///
/// Без него правило читалось бы как «сдвиги переписываются всегда».
#[test]
fn ordinary_shift_is_unchanged() {
    let src = SRC.replace(">> 8", ">> 1");
    let text = generate("shiftn", &src);
    assert!(text.contains(">> 1"), "{text}");
}

/// **Граница:** переменная величина сдвига не трогается.
///
/// ⚠️ Её значение известно только в такте; `checked_shr` в каждом выражении
/// стоил бы дороже пользы. Названная граница, вынесенная кандидатом.
#[test]
fn variable_shift_amount_is_left_alone() {
    let src = "var a: i8 := -8;\nvar n: u8 := 1;\nvar v: i8 := 0;\n\
         out probe: i8 at 0;\n\
         start Run { always { v := a >> n; probe := v; } ref Run: v < 100; }\n";
    let text = generate("shiftv", src);
    assert!(
        text.contains(">> self.n"),
        "переменная величина обязана печататься как есть:\n{text}"
    );
}
