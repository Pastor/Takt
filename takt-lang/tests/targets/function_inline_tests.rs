//! Подстановка тела функции: атрибут, эвристика и диагностики (фича 0444).
//!
//! # Что доказывает набор
//!
//! 1. **Атрибут `[inline]` действует всегда** — даже при умолчании
//!    `--inline=off`: вызова в выводе нет, а функция без вызовов не печатается.
//! 2. **`[noinline]` сильнее эвристики**: при `--inline=auto` вызов на месте.
//! 3. **Умолчание вывод не меняет** — контроль: без атрибута и без флага
//!    порождается ровно прежний текст.
//! 4. **Диагностики**: неизвестный атрибут (`SE-126`), `[inline]` на `extern
//!    fn` (`SE-127`), возврат не хвостовой (`SE-128`).
//! 5. **Вывод принимают инструменты целей** — теми же флагами, что у гейтов.
//!
//! ⚠️ Тождественность ПОВЕДЕНИЯ этим набором не доказывается: её предмет —
//! потактовая сверка `conformance_inline_tests` (крейт `takt-sim`).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Проба: `[inline]`-функция с локальной переменной, `[noinline]`-соседка и
/// параметр, одноимённый переменной модели.
const PROBE: &str = "\
model Worker {
    var n: u8 := 0;
    var v: u8 := 5;
    out led: u8 at 0x40000100;

    [inline] fn twice(v: u8) -> u8 {
        var half: u8 := v + 1;
        return half * 2;
    }

    [noinline] fn keep(v: u8) -> u8 {
        return v + 3;
    }

    start Run {
        always {
            n := n + 1;
            v := v + 2;
            led := twice(n) + keep(v);
        }
        ref Run: n < 20;
    }
}
start Main = Worker;
";

fn taktc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_taktc"))
}

fn tool(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Уникальный по тесту каталог (инвариант 0190/0429).
fn work_dir(tag: &str) -> PathBuf {
    let thread = std::thread::current()
        .name()
        .unwrap_or("main")
        .replace(':', "_");
    let dir = std::env::temp_dir()
        .join(format!("takt_pid{}", std::process::id()))
        .join(format!("takt_0444_emit_{tag}_{thread}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог теста");
    dir
}

/// Компилирует пробу; отдаёт `(успех, stderr, текст вывода)`.
fn compile(
    dir: &Path,
    source: &str,
    target: &str,
    ext: &str,
    extra: &[&str],
) -> (bool, String, String) {
    let input = dir.join("probe.takt");
    std::fs::write(&input, source).expect("запись пробы");
    let out = taktc()
        .arg("compile")
        .args(["-t", target])
        .args(extra)
        .arg(&input)
        .arg("-o")
        .arg(dir.join("out"))
        .output()
        .expect("запуск taktc compile");
    let text =
        std::fs::read_to_string(dir.join("out").join(format!("probe.{ext}"))).unwrap_or_default();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        text,
    )
}

#[test]
fn attribute_inlines_without_the_flag() {
    let dir = work_dir("attr");
    let (ok, stderr, text) = compile(&dir, PROBE, "c", "c", &[]);
    assert!(ok, "компиляция пробы: {stderr}");
    assert!(
        text.contains("takt_inline_1_v") && text.contains("takt_inline_1_half"),
        "тело функции не подставлено:\n{text}"
    );
    // Параметр и локальная переименованы: подставленное тело не должно читать
    // переменную МЕСТА вызова (в пробе она одноимённая).
    assert!(
        text.contains("uint8_t takt_inline_1_v = model->n;"),
        "параметр не получил значения аргумента:\n{text}"
    );
    // Функция без вызовов не печатается — иначе `cc -Werror` ответит
    // `-Wunused-function`.
    assert!(
        !text.contains("ProbeWorker_twice"),
        "функция осталась в выводе, хотя вызовов не осталось:\n{text}"
    );
    // `[noinline]` — вызов на месте.
    assert!(
        text.contains("ProbeWorker_keep("),
        "функция с атрибутом 'noinline' подставлена:\n{text}"
    );
}

#[test]
fn heuristic_needs_the_flag_and_noinline_beats_it() {
    let dir = work_dir("auto");
    let (ok, stderr, text) = compile(&dir, PROBE, "c", "c", &["--inline=auto"]);
    assert!(ok, "компиляция с эвристикой: {stderr}");
    assert!(
        text.contains("ProbeWorker_keep("),
        "атрибут 'noinline' не пересилил эвристику:\n{text}"
    );
}

/// Без атрибутов и без флага вывод прежний — контроль умолчания.
#[test]
fn default_does_not_inline() {
    let plain = PROBE.replace("[inline] ", "").replace("[noinline] ", "");
    let dir = work_dir("default");
    let (ok, stderr, text) = compile(&dir, &plain, "c", "c", &[]);
    assert!(ok, "компиляция без атрибутов: {stderr}");
    assert!(
        !text.contains("takt_inline_"),
        "умолчание изменилось — подстановка произошла без флага:\n{text}"
    );
    assert!(
        text.contains("ProbeWorker_twice("),
        "вызов пропал без всякой на то причины:\n{text}"
    );
}

/// Ту же подстановку печатают все цели: она живёт в семантике.
#[test]
fn every_target_sees_the_substitution() {
    for (target, ext) in [("rust", "rs"), ("st", "st"), ("sv", "sv")] {
        let dir = work_dir(&format!("target_{target}"));
        let (ok, stderr, text) = compile(&dir, PROBE, target, ext, &[]);
        assert!(ok, "компиляция целью {target}: {stderr}");
        assert!(
            text.contains("takt_inline_1_half"),
            "цель {target} не увидела подстановки:\n{text}"
        );
    }
}

#[test]
fn unknown_attribute_is_refused() {
    let dir = work_dir("se126");
    let source = PROBE.replace("[inline]", "[fast]");
    let (ok, stderr, _) = compile(&dir, &source, "c", "c", &[]);
    assert!(!ok, "неизвестный атрибут принят молча");
    assert!(
        stderr.contains("SE-126") && stderr.contains("fast"),
        "отказ не называет ни кода, ни имени атрибута: {stderr}"
    );
}

#[test]
fn inline_on_extern_function_is_refused() {
    let dir = work_dir("se127");
    let source = "\
model Worker {
    var n: u8 := 0;
    out led: u8 at 0x40000100;
    [inline] extern fn sensor(v: u8) -> u8;
    start Run {
        always { led := sensor(n); }
        ref Run: n < 3;
    }
}
start Main = Worker;
";
    let (ok, stderr, _) = compile(&dir, source, "c", "c", &[]);
    assert!(!ok, "атрибут 'inline' на внешней функции принят молча");
    assert!(
        stderr.contains("SE-127"),
        "отказ пришёл не тем кодом: {stderr}"
    );
}

#[test]
fn early_return_under_inline_is_refused() {
    let dir = work_dir("se128");
    let source = "\
model Worker {
    var n: u8 := 0;
    out led: u8 at 0x40000100;
    [inline] fn pick(v: u8) -> u8 {
        if v > 3 {
            return 1;
        }
        return v;
    }
    start Run {
        always { led := pick(n); }
        ref Run: n < 3;
    }
}
start Main = Worker;
";
    let (ok, stderr, _) = compile(&dir, source, "c", "c", &[]);
    assert!(!ok, "нехвостовой возврат под 'inline' принят молча");
    assert!(
        stderr.contains("SE-128"),
        "отказ пришёл не тем кодом: {stderr}"
    );
    // Контроль: без атрибута та же функция законна — правило судит атрибут, а
    // не форму тела вообще.
    let dir = work_dir("se128_control");
    let (ok, stderr, _) = compile(&dir, &source.replace("[inline] ", ""), "c", "c", &[]);
    assert!(ok, "функция с ранним возвратом стала незаконной: {stderr}");
}

/// Вывод с подстановкой принимают инструменты целей — теми же флагами, что у
/// гейтов предкоммита.
#[test]
fn inlined_output_is_accepted_by_target_tools() {
    if tool("cc") {
        let dir = work_dir("cc");
        let (ok, stderr, _) = compile(&dir, PROBE, "c", "c", &[]);
        assert!(ok, "компиляция: {stderr}");
        let out = Command::new("cc")
            .args([
                "-std=c11",
                "-Wall",
                "-Wextra",
                "-Wno-unused-parameter",
                "-Werror",
                "-c",
            ])
            .arg(dir.join("out").join("probe.c"))
            .arg("-o")
            .arg(dir.join("probe.o"))
            .arg("-I")
            .arg(dir.join("out"))
            .output()
            .expect("запуск cc");
        assert!(
            out.status.success(),
            "cc отверг вывод с подстановкой:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    if tool("clippy-driver") {
        let dir = work_dir("clippy");
        let (ok, stderr, _) = compile(&dir, PROBE, "rust", "rs", &[]);
        assert!(ok, "компиляция: {stderr}");
        let out = Command::new("clippy-driver")
            .current_dir(&dir)
            .args(["--edition", "2021", "--crate-type", "lib", "-D", "warnings"])
            .arg(dir.join("out").join("probe.rs"))
            .output()
            .expect("запуск clippy-driver");
        assert!(
            out.status.success(),
            "clippy отверг вывод с подстановкой:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    if tool("verilator") {
        let dir = work_dir("verilator");
        let (ok, stderr, _) = compile(&dir, PROBE, "sv", "sv", &[]);
        assert!(ok, "компиляция: {stderr}");
        let out = Command::new("verilator")
            .args(["--lint-only", "-Wall"])
            .arg(dir.join("out").join("probe.sv"))
            .output()
            .expect("запуск verilator");
        assert!(
            out.status.success(),
            "verilator отверг вывод с подстановкой:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
