//! Копирование массива целиком: переменная и константа справа (фича 0490).
//!
//! # Что было
//!
//! Матрица 2026-09-02 «агрегат × источник значения × восемь целей» (прогон
//! инструментов, а не только `taktc`):
//!
//! | Правая часть | массив | структура |
//! |---|---|---|
//! | литерал `{3, 4}` | все приняли | все приняли |
//! | переменная | `c`, `c-hal`: «array type is not assignable»; `st`, `st-at`: «Incompatible data types» | все приняли |
//! | константа | то же, плюс у `c` макрос `#define X {1, 2}` давал `{1, 2}[0]` | все приняли |
//!
//! Во всех случаях `taktc` отвечал **нулевым** кодом возврата, а `rust` и `sv`
//! тот же вход переводили — то есть потребители расходились молча.
//!
//! # Что сторожится
//!
//! Копирование массива печатается поэлементно у `c` и `st`; константа-массив у
//! `c` объявляется `static const`, а не макросом. Структура из объёма
//! исключена **замером**: её присваивают целиком и C, и IEC.
//!
//! ⚠️ Разворот берёт только **переменную** справа: результат вызова
//! индексировать нельзя, его поднимает во временную свой проход (0431). Первая
//! редакция этого не различала и сломала уже починенный случай — `iec2c`
//! отвечал «';' missing» на `got[0] := pair(k)[0];`.

use takt_lang::GenerateOptions;

fn out_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir()
        .join(format!("takt_pid{}", std::process::id()))
        .join(format!(
            "takt_0490_{tag}_{}",
            std::thread::current()
                .name()
                .unwrap_or("single")
                .replace(':', "_")
        ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог");
    dir
}

/// Модель, копирующая массив из `source` (переменной либо константы).
fn source(decl: &str, rhs: &str) -> String {
    format!(
        "model Probe {{\n\
         {decl}\
         \x20   var seen: [u8; 2] := {{0, 0}};\n\
         \x20   var ticks: u8 := 0;\n\
         \x20   out ticks_out: u8 at 0x300;\n\
         \x20   start Cycle {{\n\
         \x20       always {{ ticks := ticks + 1; seen := {rhs}; ticks_out := ticks; }}\n\
         \x20       ref Cycle: ticks < 200;\n\
         \x20   }}\n\
         }}\n\
         start Main = Probe;\n"
    )
}

fn from_variable() -> String {
    source("    var src: [u8; 2] := {1, 2};\n", "src")
}

fn from_constant() -> String {
    source("    const SETTING: [u8; 2] := {1, 2};\n", "SETTING")
}

fn emit_c(src: &str, tag: &str) -> String {
    let dir = out_dir(tag);
    takt_lang::compile_to_c(
        "probe",
        src,
        dir.to_str().expect("путь"),
        &[],
        &GenerateOptions::default(),
    )
    .expect("цель `c` переводит");
    let text = std::fs::read_to_string(dir.join("probe.c")).expect("вывод читается");
    let _ = std::fs::remove_dir_all(&dir);
    text
}

fn emit_st(src: &str, tag: &str) -> String {
    let dir = out_dir(tag);
    takt_lang::compile_to_st(
        "probe",
        src,
        dir.to_str().expect("путь"),
        &[],
        &GenerateOptions::default(),
    )
    .expect("цель `st` переводит");
    let text = std::fs::read_to_string(dir.join("probe.st")).expect("вывод читается");
    let _ = std::fs::remove_dir_all(&dir);
    text
}

/// Цель `c` копирует массив поэлементно — и из переменной, и из константы.
#[test]
fn c_copies_array_elementwise() {
    let from_var = emit_c(&from_variable(), "c_var");
    assert!(
        from_var.contains("seen[0] = ") && from_var.contains("seen[1] = "),
        "копия обязана быть поэлементной:\n{from_var}"
    );
    let from_const = emit_c(&from_constant(), "c_const");
    assert!(
        from_const.contains("seen[0] = "),
        "копия из константы — тоже поэлементная:\n{from_const}"
    );
}

/// Константа-массив объявляется `static const`, а не макросом.
///
/// ⚠️ `#define X {1, 2}` разворачивается в `{1, 2}[0]` — «expected expression».
/// Скаляру макрос по-прежнему верен, и бит-вектор (упакованный скаляр, 0078)
/// в объём не входит.
#[test]
fn c_array_constant_is_static_const() {
    let text = emit_c(&from_constant(), "c_decl");
    assert!(
        text.contains("static const uint8_t CONST_PROBE_PROBE_SETTING[2] = {1, 2};"),
        "константа-массив объявляется массивом:\n{text}"
    );
}

/// Цель `st` копирует массив поэлементно.
#[test]
fn st_copies_array_elementwise() {
    let text = emit_st(&from_variable(), "st_var");
    assert!(
        text.contains("seen[0] := src[0];") && text.contains("seen[1] := src[1];"),
        "в IEC массив целиком не присваивается:\n{text}"
    );
}

/// **Контроль:** структура присваивается целиком — разворачивать её не нужно.
///
/// Замер: тот же вход со структурой принимают все восемь потребителей, и
/// поэлементная печать была бы лишней работой без предмета.
#[test]
fn struct_assignment_stays_whole() {
    let src = "struct Pair { lo: u8, hi: u8 }\n\
               model Probe {\n\
               \x20   var src: Pair := {1, 2};\n\
               \x20   var seen: Pair := {0, 0};\n\
               \x20   var ticks: u8 := 0;\n\
               \x20   out ticks_out: u8 at 0x300;\n\
               \x20   start Cycle {\n\
               \x20       always { ticks := ticks + 1; seen := src; ticks_out := ticks; }\n\
               \x20       ref Cycle: ticks < 200;\n\
               \x20   }\n\
               }\n\
               start Main = Probe;\n";
    let text = emit_c(src, "c_struct");
    assert!(
        text.contains("seen = ") && !text.contains("seen.lo = model->src.lo"),
        "структура присваивается целиком:\n{text}"
    );
}
