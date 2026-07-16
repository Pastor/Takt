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
