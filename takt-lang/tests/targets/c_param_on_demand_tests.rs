//! Указатель на состояние печатается ПО НУЖДЕ (фича 0396).
//!
//! # Что было
//!
//! Протокол вызова цели `c` единообразен: пользовательская функция получает
//! `model`, под-модель — `main`. Тело пользуется ими не всегда; фича 0260
//! погасила предупреждения заглушкой `(void)параметр;`, но параметр в
//! сигнатуре остался. Замер 2026-08-22 по корпусу: **53** таких места.
//!
//! Класс **косметический** — оба гейта цели зелены, заглушка делает вывод
//! валидным; цена в интерфейсе и читаемости порождённого кода.
//!
//! ⚠️ **Признак ОДИН на две цели**: `c_needs::needs_state` спрашивает тот же
//! `rust_needs::function_needs`, которым живёт цель `rust` (0050). Своё знание
//! здесь разошлось бы с тем — класс 0084/0193/0195, о котором предупреждает
//! ADR 0396.

use std::path::PathBuf;
use std::process::Command;
use takt_lang::generator::GenerateOptions;

/// Функция без обращения к состоянию — и вторая, с обращением: контроль
/// обязателен, иначе правка читается как «параметр не печатается никогда».
const SRC: &str = "var acc: u8 := 0;\n\
     fn constant() -> u8 {\n    return 7;\n}\n\
     fn accumulated() -> u8 {\n    return acc + 1;\n}\n\
     var o: u8 := 0;\nout probe: u8 at 0;\n\
     start Run {\n    always {\n        o := constant() + accumulated();\n\
     \x20       probe := o;\n    }\n    ref Run;\n}\n";

fn out_dir(tag: &str) -> PathBuf {
    let thread = std::thread::current()
        .name()
        .unwrap_or("single")
        .replace(':', "_");
    let dir = std::env::temp_dir().join(format!("takt_0396_{tag}_{thread}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог вывода");
    dir
}

fn generate(tag: &str, src: &str) -> (PathBuf, String) {
    let dir = out_dir(tag);
    takt_lang::compile_to_c(
        tag,
        src,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &GenerateOptions::default(),
    )
    .expect("порождение C");
    let text = std::fs::read_to_string(dir.join(format!("{tag}.c"))).expect("чтение вывода");
    (dir, text)
}

fn cc_available() -> bool {
    Command::new("cc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Предмет: функция, не трогающая состояние, параметра не получает.
#[test]
fn function_without_state_has_no_pointer() {
    let (_, text) = generate("cp0396", SRC);
    assert!(
        text.contains("static uint8_t Cp0396_constant(void)"),
        "функция без обращения к состоянию обязана печататься без параметра:\n{text}"
    );
    assert!(
        !text.contains("(void)model;"),
        "заглушка не нужна там, где параметра нет:\n{text}"
    );
}

/// **Контроль:** функция, читающая переменную модели, параметр получает.
#[test]
fn function_with_state_keeps_the_pointer() {
    let (_, text) = generate("cp0396c", SRC);
    assert!(
        text.contains("_accumulated(const ") && text.contains("*model)"),
        "функция, читающая переменную модели, обязана получить указатель:\n{text}"
    );
}

/// Пустой список параметров печатается `void`, а не пустотой.
///
/// ⚠️ `f()` в C означает «список НЕИЗВЕСТЕН» (K&R), и `-Wstrict-prototypes`
/// отвечает «a function declaration without a prototype is deprecated». До
/// фичи случай не возникал — указатель стоял всегда.
#[test]
fn empty_parameter_list_is_void() {
    let (dir, text) = generate("cp0396v", SRC);
    assert!(
        text.contains("(void);") && text.contains("(void) {"),
        "пустой список обязан печататься `void`:\n{text}"
    );
    if !cc_available() {
        eprintln!("[ПРОПУСК] `cc` не найден; текст вывода уже проверен");
        return;
    }
    let out = Command::new("cc")
        .args([
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Wstrict-prototypes",
            "-Wno-unused-parameter",
            "-Werror",
            "-c",
        ])
        .arg("-I")
        .arg(&dir)
        .arg(dir.join("cp0396v.c"))
        .arg("-o")
        .arg(dir.join("cp0396v.o"))
        .output()
        .expect("запуск cc");
    assert!(
        out.status.success(),
        "порождённый C обязан собираться под -Wstrict-prototypes:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Под-модель без обращения к корню параметра `main` не получает.
///
/// ⚠️ Порт требует `main` **всегда**, даже объявленный в самой под-модели:
/// колбэки HAL живут в структуре корня. Поэтому у фикстуры портов нет.
#[test]
fn submodel_without_root_access_has_no_main() {
    let src = "model Child {\n    var n: u8 := 0;\n\
         \x20   start Go { always { n := n + 1; } ref Go; }\n}\n\
         start Main = Child;\n";
    let (_, text) = generate("cp0396s", src);
    assert!(
        text.contains("Child_tick(Cp0396sChild *model);"),
        "под-модель без обращения к корню обязана печататься без `main`:\n{text}"
    );
    assert!(
        text.contains("Child_tick(&model->main);"),
        "вызов обязан согласоваться с сигнатурой:\n{text}"
    );
}
