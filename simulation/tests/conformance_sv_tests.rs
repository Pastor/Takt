//! Сверка симулятора с RTL, порождённым `lamc -t sv` (критерий A10 фичи 0045).
//!
//! # Зачем отдельная сверка, если есть сверка с C
//!
//! Сверка с C (`conformance_c_tests.rs`) доказывает, что симулятор совпадает с
//! **прошивкой**. Она ничего не говорит про **кристалл**: у цели `sv` иная
//! модель исполнения (такт ≡ фронт `clk`, а не итерация цикла сканирования) и
//! иной генератор. Совпадение симулятора с C совпадения с RTL не влечёт.
//!
//! # Что здесь проверяется, чего C-сверка проверить не может
//!
//! **Сдвиг = 0 достаётся конструктивно.** У цели `c` синтетическое состояние
//! `INIT` стоило по такту на уровень вложенности, и его потребовалось *убирать*
//! правкой (фича 0033). В RTL его нет вовсе: ветвь сброса кладёт стартовые
//! состояния **всех** уровней одним фронтом. Поэтому сдвиг равен нулю на любой
//! глубине **без единой правки** — и тесты `shift_is_zero_at_depth_*` проверяют
//! это на глубинах 1, 2 и 3.
//!
//! **`Bit` включён в область сверки** — в отличие от C-сверки, где он был
//! исключён из-за дефекта 0029 (`bit` → `int`). В RTL `bit` → `logic`, то есть
//! один провод: соответствие идеальное, сверять нечему мешать.
//!
//! # Как наблюдается состояние модели
//!
//! Иерархической ссылкой из тестбенча (`dut.<сигнал>`), а не через порты.
//! Переменная модели портом не является, и выводить её наружу ради теста
//! значило бы менять **продукт** ради **проверки**: у кристалла появился бы
//! лишний вывод. Проба 2026-07-16 подтвердила, что `verilator --binary` такую
//! ссылку разрешает.
//!
//! Имя сигнала — с префиксом уровня (`conformance_ticks_counter_n`): модуль SV
//! один на корневую модель, композиция уплощается, и переменные всех уровней
//! живут в одном пространстве имён (см. `generator/sv/sv_fsm.rs`).
//!
//! # Мягкая деградация
//!
//! Нет Verilator → тест **пропускается с сообщением**, а не падает (образец —
//! `cc_available()` в `conformance_c_tests.rs`). Verilator для сборки и тестов
//! `lamc` не нужен. В CI он обязателен — там гейт краснеет сам (задача 0045-02).

use grammar::semantic::tree::construct_model;
use simulation::{TickResult, Unit, Value, build_unit};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Фикстура потактовой сверки: `n` принимает 1 → 2 → 3 по мере переходов.
///
/// Переиспользуется из сверки с C **как есть**: одна и та же модель, сверяемая с
/// двумя эталонами, — это и есть смысл сверки. Модель эволюционирует несколько
/// тактов, поэтому сдвиг на такт (если бы он появился) сместил бы всю трассу.
const TICKS_FIXTURE: &str = "tests/data/eval/conformance_ticks.lam";

/// Тактов в трассе — с запасом над её длиной.
const TRACE_TICKS: usize = 6;

fn verilator_available() -> bool {
    Command::new("verilator")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn sim_value(unit: &Unit, name: &str) -> i64 {
    match unit.variable(name) {
        Some(Value::Number(n)) => n,
        Some(Value::Boolean(b)) => i64::from(b),
        // q(m, n) (0061): наблюдаемое — представление (сигнал `logic signed [W-1:0]`
        // = repr), ровно то, что тестбенч читает через `$signed(dut.<сигнал>)`.
        Some(Value::Fixed { repr, .. }) => repr,
        other => panic!("переменная '{name}': неожиданное значение {other:?}"),
    }
}

/// Потактовая трасса симулятора: значения `vars` после каждого такта.
fn simulate_trace(fixture: &str, vars: &[&str]) -> Vec<Vec<i64>> {
    let source = std::fs::read_to_string(fixture).expect("фикстура читается");
    let (ast, _) = grammar::parse(&source, 0).expect("разбор");
    let model = construct_model(&ast, None, &[]).expect("семантика");
    let mut unit = build_unit(model).expect("построение юнита");
    let mut trace = Vec::new();
    for _ in 0..TRACE_TICKS {
        let result = unit.tick();
        assert!(
            !matches!(result, TickResult::Failed(_)),
            "симуляция не должна падать: {result:?}"
        );
        trace.push(vars.iter().map(|v| sim_value(&unit, v)).collect());
        if result == TickResult::Terminated {
            break;
        }
    }
    trace
}

/// Потактовая трасса порождённого RTL.
///
/// Порождает `.sv` тем же `lamc`, пишет тестбенч, собирает его настоящим
/// Verilator (`--binary --timing`), запускает и разбирает печать.
///
/// `signals` — имена **сигналов SV** (с префиксом уровня), а не имена Lam.
///
/// **Тестбенч пишется здесь, а не порождается `lamc`.** Тестбенч — принадлежность
/// проверки, а не продукта: генератор не должен уметь то, что нужно только
/// тестам (решение открытого вопроса задачи 0045-07).
fn sv_trace(
    dir: &Path,
    fixture: &str,
    basename: &str,
    signals: &[&str],
    ticks: usize,
) -> Vec<Vec<i64>> {
    let source = std::fs::read_to_string(fixture).expect("фикстура читается");
    grammar::compile_to_sv(
        basename,
        &source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &grammar::generator::GenerateOptions::default(),
    )
    .expect("порождение SystemVerilog");

    // Печатаются значения ПОСЛЕ фронта (`#1` после `@(posedge clk)`), иначе
    // читалось бы состояние до защёлкивания — артефакт, о котором предупреждает
    // задача 0045-07.
    let prints = signals
        .iter()
        .map(|s| format!("dut.{}", s))
        .collect::<Vec<_>>()
        .join(", ");
    let fmt = vec!["%0d"; signals.len()].join(" ");
    let tb = format!(
        r#"module tb;
    logic clk = 0, rst_n = 0;
    logic is_done;
    {basename} dut (.clk(clk), .rst_n(rst_n), .is_done(is_done));
    always #5 clk = ~clk;
    initial begin
        // Первый фронт снимает сброс: стартовые состояния всех уровней уже в
        // регистрах, поэтому СЛЕДУЮЩИЙ фронт — такт 1 модели.
        @(posedge clk);
        rst_n <= 1'b1;
        repeat ({ticks}) begin
            @(posedge clk);
            #1 $display("TICK {fmt}", {prints});
        end
        $finish;
    end
endmodule
"#
    );
    let tb_path = dir.join("tb.sv");
    std::fs::write(&tb_path, tb).expect("запись тестбенча");

    let build = Command::new("verilator")
        .current_dir(dir)
        .args([
            "--binary",
            "--timing",
            "-Wno-fatal",
            "--top-module",
            "tb",
            "tb.sv",
            &format!("{}.sv", basename),
            "-o",
            "simtb",
        ])
        .output()
        .expect("запуск verilator");
    assert!(
        build.status.success(),
        "verilator не собрал тестбенч:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let run = Command::new(dir.join("obj_dir").join("simtb"))
        .current_dir(dir)
        .output()
        .expect("запуск собранной симуляции");
    let stdout = String::from_utf8_lossy(&run.stdout);
    stdout
        .lines()
        .filter_map(|line| line.strip_prefix("TICK "))
        .map(|rest| {
            rest.split_whitespace()
                .map(|v| v.parse::<i64>().expect("значение — целое"))
                .collect()
        })
        .collect()
}

/// Каталог сборки под конкретный тест (тесты идут однопоточно, но каталоги
/// разные — чтобы падение одного не путало вывод другого).
fn build_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lam_conformance_sv_{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог сборки");
    dir
}

/// Пишет временную фикстуру и возвращает путь к ней.
fn fixture(dir: &Path, name: &str, source: &str) -> PathBuf {
    let path = dir.join(format!("{name}.lam"));
    std::fs::write(&path, source).expect("запись фикстуры");
    path
}

/// **T32/T33 (R7/A10): потактовые трассы симулятора и RTL совпадают.**
///
/// Не «установившиеся значения», а значение на **каждом такте**. C-сверка
/// позволить себе этого долго не могла: синтетический `INIT` сдвигал трассу
/// (фича 0033). Здесь сдвига нет с самого начала — сверять потактово можно
/// сразу.
#[test]
fn per_tick_trace_matches_generated_sv() {
    let vars = ["n"];
    let sim = simulate_trace(TICKS_FIXTURE, &vars);
    // Пиннинг трассы симулятора: если она изменится, тест обязан упасть здесь,
    // а не «подстроиться» под RTL.
    assert_eq!(
        sim,
        vec![vec![1], vec![2], vec![3]],
        "ожидаемая потактовая трасса симулятора: n = 1, 2, 3"
    );

    if !verilator_available() {
        eprintln!(
            "[ПРОПУСК] per_tick_trace_matches_generated_sv: verilator не найден — \
             потактовая сверка с RTL не выполнена (трасса симулятора пришпилена выше)"
        );
        return;
    }
    let dir = build_dir("trace");
    let sv = sv_trace(
        &dir,
        TICKS_FIXTURE,
        "conformance_ticks",
        &["conformance_ticks_counter_n"],
        sim.len(),
    );
    assert_eq!(
        sim, sv,
        "потактовые трассы симулятора и порождённого RTL обязаны совпадать НА \
         КАЖДОМ такте (R7/A10).\nсимулятор={sim:?}\nRTL={sv:?}"
    );
}

/// Модель глубины 1 (без обёртки): тело стартового состояния — на такте 1.
#[test]
fn shift_is_zero_at_depth_1() {
    check_shift_is_zero(
        "depth1",
        "var n: u8 := 0; \
         start S0 { always { n := 1; } ref S1; } \
         state S1 { always { n := 2; } }",
        &["depth1_n"],
    );
}

/// Глубина 2 (`start E = M;`): лишний уровень трассу не сдвигает.
#[test]
fn shift_is_zero_at_depth_2() {
    check_shift_is_zero(
        "depth2",
        "model M { var n: u8 := 0; start S0 { always { n := 1; } ref S1; } \
         state S1 { always { n := 2; } } } \
         start E = M;",
        &["depth2_m_n"],
    );
}

/// **Глубина 3 — где у цели `c` сдвиг был максимальным.**
///
/// В C уровни входили последовательно, поэтому тело исполнялось на такте,
/// равном глубине. В RTL все `always_ff` сбрасываются **одним фронтом**, и
/// глубина не стоит ничего.
#[test]
fn shift_is_zero_at_depth_3() {
    check_shift_is_zero(
        "depth3",
        "model Inner { var n: u8 := 0; start S0 { always { n := 1; } ref S1; } \
         state S1 { always { n := 2; } } } \
         model Mid { start M = Inner; } \
         start E = Mid;",
        // ⚠️ Имя снято ЗОНДОМ, а не выведено из пути: уникальное имя модели не
        // повторяет промежуточный уровень — сигнал `depth3_inner_n`, а не
        // `depth3_mid_inner_n`.
        &["depth3_inner_n"],
    );
}

/// Общая проверка «сдвиг = 0»: на такте 1 переменная уже равна 1.
///
/// Сверяется не только первый такт, но и вся трасса — со стороной симулятора.
fn check_shift_is_zero(tag: &str, source: &str, signals: &[&str]) {
    let dir = build_dir(tag);
    let path = fixture(&dir, tag, source);
    let sim = simulate_trace(path.to_str().expect("путь в UTF-8"), &["n"]);
    assert_eq!(
        sim.first().map(|t| t[0]),
        Some(1),
        "симулятор: тело стартового состояния обязано исполниться на такте 1 \
         (контракт ADR 0033), трасса={sim:?}"
    );

    if !verilator_available() {
        eprintln!("[ПРОПУСК] {tag}: verilator не найден — сверка с RTL не выполнена");
        return;
    }
    let sv = sv_trace(
        &dir,
        path.to_str().expect("путь в UTF-8"),
        tag,
        signals,
        sim.len(),
    );
    assert_eq!(
        sv.first().map(|t| t[0]),
        Some(1),
        "RTL: тело стартового состояния обязано исполниться на ТАКТЕ 1 после \
         снятия rst_n — сдвиг = 0 на любой глубине.\nRTL={sv:?}"
    );
    assert_eq!(
        sim, sv,
        "потактовые трассы обязаны совпадать.\nсимулятор={sim:?}\nRTL={sv:?}"
    );
}

/// **T34 (R5/A10): `Bit` входит в область сверки.**
///
/// В C-сверке он был исключён: генератор давал `bit` → `int` (дефект 0029), и
/// сверяться с дефектным эталоном значило бы узаконить дефект. В RTL `bit` →
/// `logic` — один провод, соответствие идеальное.
#[test]
fn bit_is_within_conformance_scope() {
    let dir = build_dir("bit");
    let source = "var f: bit := 0; \
                  start S0 { always { f := 1; } ref S1; } \
                  state S1 { always { f := 0; } }";
    let path = fixture(&dir, "bitconf", source);
    let sim = simulate_trace(path.to_str().expect("путь в UTF-8"), &["f"]);
    assert_eq!(
        sim,
        vec![vec![1], vec![0]],
        "симулятор: f принимает 1, затем 0"
    );

    if !verilator_available() {
        eprintln!("[ПРОПУСК] bit_is_within_conformance_scope: verilator не найден");
        return;
    }
    let sv = sv_trace(
        &dir,
        path.to_str().expect("путь в UTF-8"),
        "bitconf",
        &["bitconf_f"],
        sim.len(),
    );
    assert_eq!(
        sim, sv,
        "тип `bit` обязан сверяться потактово: в RTL он `logic`, то есть один \
         провод, и дефекта 0029 (`bit` → `int`) здесь нет.\nсимулятор={sim:?}\nRTL={sv:?}"
    );
}

// ── Сверка `+` идёт с целью C, а не с симулятором (0057) ──────────────────────
//
// ADR 0057 называет эталоном поведения `+` **обе** реализации — C
// (`generate_concat_tick`) и симулятор (`Unit::Sequential`), — но при разработке
// вскрылось: **симулятор не исполняет композицию, несущую переход `next`** —
// состояние-реализация с `next` немедленно уходит по `next`, не тикнув шаги
// (проба: `start P = A + B { next Done; }` даёт `Terminated` на такте 1, шаги не
// исполнены). Ровно поэтому `extend_complex.lam` (корневая `+` с `next Next`)
// **исключён** из `examples_scenario_tests`. Дефект симулятора заведён фиксом
// `docs/fixes/0057-01-sim-composition-next.md` (Tier 2) и к цели `sv` отношения
// не имеет.
//
// Поэтому сверка `+` ведётся с **порождённым C** — вторым named-эталоном ADR
// 0057. Проба 2026-07-18 показала совпадение SV и C потактово (stage: 1,1,2,2 и
// done на такте 5). Когда дефект симулятора будет закрыт, сюда добавится и
// сверка sim↔SV.

fn cc_available() -> bool {
    Command::new("cc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Потактовая трасса порождённого C для КОРНЕВЫХ переменных (`m.<var>`).
///
/// Собственный минимальный аналог `conformance_c_tests::c_trace` — тот адресует
/// **вложенную** модель (`m.<accessor>.<var>`), а здесь наблюдаемые лежат в
/// корне (shared-переменные, записанные под-моделями композиции).
fn c_root_trace(
    dir: &Path,
    fixture: &str,
    basename: &str,
    root_type: &str,
    vars: &[&str],
) -> Vec<Vec<i64>> {
    let source = std::fs::read_to_string(fixture).expect("фикстура читается");
    grammar::compile_to_c(
        basename,
        &source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &grammar::generator::GenerateOptions::default(),
    )
    .expect("порождение C");
    let prints = vars
        .iter()
        .map(|v| format!(r#"        printf("%d:{v}=%d\n", t, (int)m.{v});"#))
        .collect::<Vec<_>>()
        .join("\n");
    let harness = format!(
        r#"#include <stdio.h>
#include "{basename}.h"
int main(void) {{
    {root_type} m;
    {root_type}_init(&m);
    for (int t = 0; t < {TRACE_TICKS}; t++) {{
        {root_type}_tick(&m);
{prints}
        if ({root_type}_is_done(&m)) break;
    }}
    return 0;
}}
"#
    );
    let harness_path = dir.join("c_root_harness.c");
    std::fs::write(&harness_path, harness).expect("запись харнесса");
    let bin = dir.join("c_root_bin");
    let compile = Command::new("cc")
        .args(["-std=c11", "-I"])
        .arg(dir)
        .arg(dir.join(format!("{basename}.c")))
        .arg(&harness_path)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("запуск cc");
    assert!(
        compile.status.success(),
        "порождённый C не компилируется:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let run = Command::new(&bin).output().expect("запуск собранного C");
    assert!(run.status.success(), "собранный C завершился с ошибкой");
    let out = String::from_utf8_lossy(&run.stdout).into_owned();
    let mut trace: Vec<Vec<i64>> = Vec::new();
    for line in out.lines() {
        let (t_str, rest) = line.split_once(':').expect("формат T:var=val");
        let t: usize = t_str.parse().expect("индекс такта");
        let (_, val) = rest.split_once('=').expect("формат var=val");
        let val: i64 = val.trim().parse().expect("значение");
        if trace.len() <= t {
            trace.resize(t + 1, Vec::new());
        }
        trace[t].push(val);
    }
    trace
}

/// **0057 (R1/R2/R4/R5): последовательная композиция `A + B` — SV == C потактово.**
///
/// Каждый шаг под-модели исполняется до завершения, затем шаг-регистр
/// продвигается, и следующий шаг тикается на такте **после** (эталон — `break`
/// в C `generate_concat_tick`). Наблюдаемая — корневая `stage`, которую пишет
/// текущий шаг (shared-переменная). Завершение последнего шага переводит
/// **родительское** состояние в `Done` (R5) — видно по `is_done`.
#[test]
fn sequential_composition_matches_generated_c() {
    let dir = build_dir("concat");
    let source = "var stage: u8 := 0; \
                  model A { start S1 { always { stage := 1; } ref S2: 1 = 1; } state S2; } \
                  model B { start S1 { always { stage := 2; } ref S2: 1 = 1; } state S2; } \
                  start P = A + B { next Done; } state Done;";
    let path = fixture(&dir, "seqc", source);

    if !cc_available() {
        eprintln!("[ПРОПУСК] sequential_composition_matches_generated_c: `cc` не найден");
        return;
    }
    let c = c_root_trace(
        &dir,
        path.to_str().expect("путь в UTF-8"),
        "seqc",
        "Seqc",
        &["stage"],
    );
    // Пиннинг C-эталона: `A` пишет 1 (такты 1–2), `B` — 2 (такты 3–4), затем
    // родитель уходит в `Done`. Если обе стороны сломать одинаково — упадёт тут.
    assert_eq!(
        c,
        vec![vec![1], vec![1], vec![2], vec![2], vec![2]],
        "эталон C: шаг A держит stage=1 два такта, шаг B — stage=2, затем Done"
    );

    if !verilator_available() {
        eprintln!("[ПРОПУСК] sequential_composition_matches_generated_c: verilator не найден");
        return;
    }
    let sv = sv_trace(
        &dir,
        path.to_str().expect("путь в UTF-8"),
        "seqc",
        &["seqc_stage"],
        c.len(),
    );
    assert_eq!(
        c, sv,
        "потактовые трассы `+` (SV и C) обязаны совпадать: шаг активируется на \
         такте ПОСЛЕ завершения предыдущего (регистр шага разрывает такт, как \
         `break` в C).\nC={c:?}\nRTL={sv:?}"
    );
}

/// **0057 (R3): параллельный шаг `(B | C)` внутри цепочки `+` — SV == C.**
///
/// Средний шаг — параллельная композиция: цепочка переходит к следующему шагу,
/// когда завершены **обе** ветви (конъюнкция done). `s1` пишет шаг A, `s2` —
/// обе ветви параллельного шага (идемпотентно, порядок ветвей не важен).
#[test]
fn parallel_step_inside_concatenation_matches_generated_c() {
    let dir = build_dir("concat_par");
    let source = "var s1: u8 := 0; var s2: u8 := 0; \
                  model A { start S1 { always { s1 := 1; } ref S2: 1 = 1; } state S2; } \
                  model B { start S1 { always { s2 := 1; } ref S2: 1 = 1; } state S2; } \
                  model C { start S1 { always { s2 := 1; } ref S2: 1 = 1; } state S2; } \
                  start P = A + (B | C) { next Done; } state Done;";
    let path = fixture(&dir, "parc", source);

    if !cc_available() {
        eprintln!(
            "[ПРОПУСК] parallel_step_inside_concatenation_matches_generated_c: `cc` не найден"
        );
        return;
    }
    let c = c_root_trace(
        &dir,
        path.to_str().expect("путь в UTF-8"),
        "parc",
        "Parc",
        &["s1", "s2"],
    );
    assert_eq!(
        c,
        vec![vec![1, 0], vec![1, 0], vec![1, 1], vec![1, 1], vec![1, 1]],
        "эталон C: A даёт s1=1 (такты 1–2), затем параллельный шаг даёт s2=1 по \
         завершении обеих ветвей"
    );

    if !verilator_available() {
        eprintln!(
            "[ПРОПУСК] parallel_step_inside_concatenation_matches_generated_c: verilator не найден"
        );
        return;
    }
    let sv = sv_trace(
        &dir,
        path.to_str().expect("путь в UTF-8"),
        "parc",
        &["parc_s1", "parc_s2"],
        c.len(),
    );
    assert_eq!(
        c, sv,
        "потактовые трассы `(|)` внутри `+` (SV и C) обязаны совпадать: \
         параллельный шаг завершается по конъюнкции done обеих ветвей.\nC={c:?}\nRTL={sv:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Q-арифметика fixed-point (фича 0061, задача 0061-04): T8/T9/T10 для цели sv
//
// Сигнал q — `logic signed [W-1:0]` = repr; наблюдается `$signed(dut.<сигнал>)`
// (знаковая печать) и сверяется потактово с симулятором. Гейты verilator+yosys
// доказывают синтезируемость (T8), сверка — верность (T10, включая floor T9).
// ─────────────────────────────────────────────────────────────────────────────

/// Как [`sv_trace`], но наблюдаемые сигналы печатаются **знаково**
/// (`$signed(dut.<сигнал>)`): представление q бывает отрицательным, а `%0d` над
/// `logic` печатает без знака.
fn sv_trace_signed(
    dir: &Path,
    fixture: &str,
    basename: &str,
    signals: &[&str],
    ticks: usize,
    opts: &grammar::generator::GenerateOptions,
) -> Vec<Vec<i64>> {
    let source = std::fs::read_to_string(fixture).expect("фикстура читается");
    grammar::compile_to_sv(
        basename,
        &source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        opts,
    )
    .expect("порождение SystemVerilog");

    let prints = signals
        .iter()
        .map(|s| format!("$signed(dut.{})", s))
        .collect::<Vec<_>>()
        .join(", ");
    let fmt = vec!["%0d"; signals.len()].join(" ");
    let tb = format!(
        r#"module tb;
    logic clk = 0, rst_n = 0;
    logic is_done;
    {basename} dut (.clk(clk), .rst_n(rst_n), .is_done(is_done));
    always #5 clk = ~clk;
    initial begin
        @(posedge clk);
        rst_n <= 1'b1;
        repeat ({ticks}) begin
            @(posedge clk);
            #1 $display("TICK {fmt}", {prints});
        end
        $finish;
    end
endmodule
"#
    );
    std::fs::write(dir.join("tb.sv"), tb).expect("запись тестбенча");
    let build = Command::new("verilator")
        .current_dir(dir)
        .args([
            "--binary",
            "--timing",
            "-Wno-fatal",
            "--top-module",
            "tb",
            "tb.sv",
            &format!("{}.sv", basename),
            "-o",
            "simtb",
        ])
        .output()
        .expect("запуск verilator");
    assert!(
        build.status.success(),
        "verilator не собрал Q-тестбенч:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(dir.join("obj_dir").join("simtb"))
        .current_dir(dir)
        .output()
        .expect("запуск симуляции");
    String::from_utf8_lossy(&run.stdout)
        .lines()
        .filter_map(|line| line.strip_prefix("TICK "))
        .map(|rest| {
            rest.split_whitespace()
                .map(|v| v.parse::<i64>().expect("значение — целое"))
                .collect()
        })
        .collect()
}

/// T10/A4 (цель sv, **ради которой фича**): побитовая потактовая сверка
/// Q-арифметики с симулятором — включая отрицательные и floor к −∞ у `*` (S2:
/// repr −2, а не −1). Гейты verilator/yosys прогоняются в `precheck.sh`.
#[test]
fn fixed_point_arithmetic_matches_generated_sv() {
    let vars = ["acc"];
    let sim = simulate_trace("tests/data/eval/conformance_fixed.lam", &vars);
    assert_eq!(
        sim,
        vec![vec![-768], vec![-384], vec![-2], vec![510]],
        "трасса представлений q(8,8) — эталон Q-арифметики симулятора"
    );

    if !verilator_available() {
        eprintln!("[ПРОПУСК] fixed_point_arithmetic_matches_generated_sv: verilator не найден");
        return;
    }
    let dir = build_dir("fixed");
    let sv = sv_trace_signed(
        &dir,
        "tests/data/eval/conformance_fixed.lam",
        "conformance_fixed",
        &["conformance_fixed_fixed_acc"],
        sim.len(),
        &grammar::generator::GenerateOptions::default(),
    );
    assert_eq!(
        sim, sv,
        "Q-арифметика симулятора и порождённого RTL обязана совпасть ПОБИТОВО на \
         каждом такте (repr q(8,8)).\nсимулятор={sim:?}\nRTL={sv:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Прозрачный float → q(m, n) (фича 0096, задача 0096-02): T4/T7/T9 для цели sv
//
// Под флагом `--float-as-q=8.8` цель sv понижает `float` в q(8,8) — SV-003
// снимается, вещественная модель синтезируется. Трасса `acc` обязана совпасть с
// q-версией (`conformance_fixed`) ПОБИТОВО: float→q(8,8) — это и есть q(8,8).
// ─────────────────────────────────────────────────────────────────────────────

/// Опции генерации с глобальной Q-точностью `q(m, n)` для `float` (фича 0096).
#[allow(clippy::field_reassign_with_default)] // GenerateOptions — #[non_exhaustive]
fn float_as_q_opts(m: u8, n: u8) -> grammar::generator::GenerateOptions {
    let mut o = grammar::generator::GenerateOptions::default();
    o.float_as_q = Some((m, n));
    o
}

/// Q-режим эталона: понижает `float → q(m, n)` в модели симулятора тем же
/// проходом, что и цель (ADR 0096, драйвер 2 — сверка ВНУТРИ режима).
fn simulate_trace_float_q(fixture: &str, m: u8, n: u8, vars: &[&str]) -> Vec<Vec<i64>> {
    let source = std::fs::read_to_string(fixture).expect("фикстура читается");
    let (ast, _) = grammar::parse(&source, 0).expect("разбор");
    let model = construct_model(&ast, None, &[]).expect("семантика");
    grammar::semantic::lower_float::lower_float_to_fixed(model.clone(), m, n)
        .expect("float → q(m, n)");
    let mut unit = build_unit(model).expect("построение юнита");
    let mut trace = Vec::new();
    for _ in 0..TRACE_TICKS {
        let result = unit.tick();
        assert!(
            !matches!(result, TickResult::Failed(_)),
            "симуляция не должна падать: {result:?}"
        );
        trace.push(vars.iter().map(|v| sim_value(&unit, v)).collect());
        if result == TickResult::Terminated {
            break;
        }
    }
    trace
}

/// T4/T7/A2/A5 (цель sv, **ради которой фича 0096**): под `--float-as-q=8.8`
/// `float` → q(8,8), `SV-003` снят, RTL синтезируется (verilator) и **побитово**
/// совпадает с Q-эталоном симулятора на каждом такте. Ожидаемая трасса — та же,
/// что у явной q-версии `conformance_fixed` (float→q(8,8) ≡ q(8,8)).
#[test]
fn float_as_q_matches_generated_sv() {
    let sim = simulate_trace_float_q("tests/data/eval/conformance_float_q.lam", 8, 8, &["acc"]);
    assert_eq!(
        sim,
        vec![vec![-768], vec![-384], vec![-2], vec![510]],
        "Q-эталон float→q(8,8) обязан совпасть с трассой явной q-версии (0061)"
    );

    if !verilator_available() {
        eprintln!("[ПРОПУСК] float_as_q_matches_generated_sv: verilator не найден");
        return;
    }
    let dir = build_dir("float_q");
    let sv = sv_trace_signed(
        &dir,
        "tests/data/eval/conformance_float_q.lam",
        "conformance_float_q",
        &["conformance_float_q_float_fixed_acc"],
        sim.len(),
        &float_as_q_opts(8, 8),
    );
    assert_eq!(
        sim, sv,
        "float→q(8,8) в симуляторе и порождённом RTL обязаны совпасть ПОБИТОВО на \
         каждом такте (repr q(8,8)).\nсимулятор={sim:?}\nRTL={sv:?}"
    );
}

/// T9/A1 (сторож направления): без флага `float` в цели sv остаётся `SV-003`.
/// Это и есть «выбор формата автором» (ADR 0061/0096): флаг — единственное, что
/// снимает запрет. Мутация «убрать флаг у цели» обязана вернуть ошибку, а не
/// молча собрать несопоставимый (native) вывод.
#[test]
fn float_without_flag_is_sv003() {
    let source = std::fs::read_to_string("tests/data/eval/conformance_float_q.lam")
        .expect("фикстура читается");
    let dir = build_dir("float_no_flag");
    let err = grammar::compile_to_sv(
        "conformance_float_q",
        &source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &grammar::generator::GenerateOptions::default(),
    )
    .expect_err("float без --float-as-q обязан дать SV-003");
    assert_eq!(
        err.code.as_deref(),
        Some("SV-003"),
        "без флага float в sv — SV-003 (флаг = выбор формата автором)"
    );
}
