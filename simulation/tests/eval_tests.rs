//! Интеграционные тесты вычислителя: модель `.lam` → прогон → **значения**.
//!
//! # Зачем этот слой существует
//!
//! Восемь дефектов фичи 0025 прожили при 1484 зелёных тестах, потому что такого
//! слоя не было: `simulation` покрывался только inline-юнитами, и **ни один тест
//! не брал `.lam`, не прогонял его и не сверял вычисленные значения**.
//! Проверялся факт перехода — а он у сломанного вычислителя выглядел
//! правдоподобно.
//!
//! Поэтому здесь сверяются именно **значения** (`c=6`, `x=44`, `eta=7`), а
//! фикстуры лежат в репозитории (`tests/data/eval/`), а не во временном
//! каталоге: проверка обязана быть воспроизводимой кем угодно.
//!
//! # Почему `build_unit`, а не `SimulationRunner`
//!
//! Тест-план предписывал слой поверх `runner::SimulationRunner`. Его `new`
//! требует девять параметров (каталог вывода, режим графики, конфигурация GIF,
//! имена портов…) — для сверки значений это лишний вес. Путь исполнения при этом
//! **тот же**: `SimulationRunner::run` в цикле зовёт `Unit::tick`, а
//! `RunResult::EvalFailed` — тонкая обёртка над `TickResult::Failed`, который
//! наблюдаем и отсюда. Сценарии с портами и guard'ами покрывает
//! `scripts/run_simulations.sh` (5 сценариев `stacker_*`).

use grammar::semantic::tree::construct_model;
use simulation::{TickResult, Unit, Value, build_unit};

// ── Вспомогательное ───────────────────────────────────────────────────────────

fn unit_from(fixture: &str) -> Unit {
    let path = format!("tests/data/eval/{fixture}");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("не прочитать фикстуру {path}: {e}"));
    let (ast, _) = grammar::parse(&source, 0).unwrap_or_else(|e| panic!("{path}: разбор: {e:?}"));
    let model =
        construct_model(&ast, None, &[]).unwrap_or_else(|e| panic!("{path}: семантика: {e:?}"));
    build_unit(model).unwrap_or_else(|e| panic!("{path}: построение юнита: {e:?}"))
}

/// Прогоняет до `steps` шагов; останавливается на терминальном состоянии или ошибке.
fn run(fixture: &str, steps: usize) -> (Unit, TickResult) {
    let mut unit = unit_from(fixture);
    let mut last = TickResult::Processing;
    for _ in 0..steps {
        last = unit.tick();
        if last != TickResult::Processing {
            break;
        }
    }
    (unit, last)
}

/// Целочисленное значение переменной — иначе внятный провал.
fn num(unit: &Unit, name: &str) -> i64 {
    match unit.variable(name) {
        Some(Value::Number(n)) => n,
        other => panic!("переменная '{name}': ожидалось целое, получено {other:?}"),
    }
}

/// Представление `q(m, n)`-переменной — иначе внятный провал.
fn fixed_repr(unit: &Unit, name: &str) -> i64 {
    match unit.variable(name) {
        Some(Value::Fixed { repr, .. }) => repr,
        other => panic!("переменная '{name}': ожидалось fixed-point, получено {other:?}"),
    }
}

// ── Фича 0061: Q-арифметика симулятора (эталон сверки) ────────────────────────

/// Полный путь: инициализация литерала → арифметика в теле → наблюдение.
/// T9 (floor к −∞ на отрицательном) и T19 (wraparound) — через `.lam`, а не
/// только юниты `eval::fixed`.
#[test]
fn fixed_point_arithmetic_matches_normative_rules() {
    let (unit, _) = run("fixed_point.lam", 1);
    // sum = 1.5 + 0.5 = 2.0 → 512 (сложение представлений).
    assert_eq!(fixed_repr(&unit, "sum"), 512, "q: сложение представлений");
    // prod = −1.5 · 2.0 = −3.0 → −768 (floor к −∞; на положительном был бы невидим).
    assert_eq!(
        fixed_repr(&unit, "prod"),
        -768,
        "T9: `*` округляет floor к −∞"
    );
    // scaled = 1.5 + (3 as q) = 4.5 → 1152 (каст масштабирует 3 → 768).
    assert_eq!(fixed_repr(&unit, "scaled"), 1152, "T7: каст масштабирует");
    // wrap = 100.0 + 100.0 = 200.0 (вне q(8,8)) → 51200 mod 2¹⁶ = −14336 (−56.0).
    assert_eq!(
        fixed_repr(&unit, "wrap"),
        -14336,
        "T19: переполнение `+` — wraparound"
    );
}

// ── Д1/Д2: арифметика в теле блока ───────────────────────────────────────────

#[test]
fn t1_arithmetic_in_always_is_evaluated() {
    // Ядро фичи: `c := a + 1` молча пропускалось (c оставалось 0).
    let (unit, _) = run("assign_arith.lam", 1);
    assert_eq!(num(&unit, "a"), 5);
    assert_eq!(num(&unit, "b"), 5, "присваивание переменной из переменной");
    assert_eq!(num(&unit, "c"), 6, "Д1/Д2: арифметика обязана исполняться");
}

// ── S1/S9: усечение по объявленному типу (сверено с C) ───────────────────────

#[test]
fn t9_t17_assignment_truncates_to_declared_type() {
    // Сверено с cc -std=c11: uint8_t a=255; a+1 → 0; uint8_t b = 300 → 44.
    let (unit, _) = run("overflow_u8.lam", 1);
    assert_eq!(num(&unit, "wrapped"), 0, "S1: 255 + 1 в u8 обязано дать 0");
    assert_eq!(num(&unit, "truncated"), 44, "S9: 300 в u8 обязано дать 44");
}

#[test]
fn t12_shift_promotes_then_truncates_like_c() {
    // S4: в C `uint8_t x = 1; x = x << 8;` даёт 0 БЕЗ UB (продвижение до int).
    // Первоначальная формулировка S4 (UB → диагностика) была ошибочной.
    let (unit, result) = run("shift_promo.lam", 1);
    assert_eq!(num(&unit, "x"), 0);
    assert_ne!(
        result,
        TickResult::Failed(String::new()),
        "сдвиг на 8 у u8 — определённое поведение, а не ошибка"
    );
}

// ── Д3: вызовы ───────────────────────────────────────────────────────────────

#[test]
fn t3_bare_extern_procedure_call_does_not_block_block() {
    // `log_temp(x);` молча отбрасывался; проверяем, что блок исполняется целиком.
    let (unit, _) = run("call_stmt.lam", 3);
    assert_eq!(unit.current_state(), Some("Hot"), "переход при x > 7");
    assert_eq!(num(&unit, "x"), 8);
}

#[test]
fn t20_local_function_call_returns_correct_value() {
    // Критерий A7: метрика Чебышёва max(5, 3, 7) = 7 — как travel_time в stacker.
    let (unit, _) = run("local_fn_call.lam", 1);
    assert_eq!(
        num(&unit, "eta"),
        7,
        "вызов локальной fn обязан вычисляться"
    );
}

// ── Д4/Д6/Д7/Д8: условия переходов ───────────────────────────────────────────

#[test]
fn t4_function_call_in_condition_fires_transition() {
    // Раньше: паника unimplemented!().
    let (unit, _) = run("fn_cond.lam", 1);
    assert_eq!(unit.current_state(), Some("Hot"));
}

#[test]
fn t6_mixed_int_real_condition_fires_transition() {
    // Раньше: паника unwrap() on None. 1 + 2.5 = 3.5 > 3.
    let (unit, _) = run("mixed_num_cond.lam", 1);
    assert_eq!(unit.current_state(), Some("Hot"));
}

#[test]
fn t7_parenthesised_condition_fires_transition() {
    // Раньше: скобки не вычислялись → (5 + 1) > 2 было ложным.
    let (unit, _) = run("paren_cond.lam", 1);
    assert_eq!(unit.current_state(), Some("Hot"));
}

#[test]
fn t8_enum_variant_condition_fires_transition() {
    // Раньше: EnumVariant → Err → «условие ложно».
    let (unit, _) = run("enum_cond.lam", 1);
    assert_eq!(unit.current_state(), Some("Hot"));
}

// ── Д5: enter стартового состояния ───────────────────────────────────────────

#[test]
fn t5_enter_of_start_state_runs_exactly_once() {
    let (unit, _) = run("start_enter.lam", 4);
    assert_eq!(
        num(&unit, "e"),
        7,
        "Д5: enter стартового состояния обязан идти"
    );
    assert_eq!(num(&unit, "n"), 1, "и ровно один раз — за 4 тика");
    assert_eq!(num(&unit, "t"), 4, "always при этом идёт каждый тик");
}

// ── Контрпримеры (правило 16): отказ вместо тишины ────────────────────────────

#[test]
fn t11_division_by_zero_fails_loudly() {
    // R5: ошибка вычисления обязана быть ОТЛИЧИМА от «ничего не произошло».
    let (_, result) = run("div_zero.lam", 1);
    match result {
        TickResult::Failed(details) => {
            assert!(details.contains("деление на ноль"), "детали: {details}");
            assert!(
                details.contains("SIM-001"),
                "код обязан быть в деталях: {details}"
            );
        }
        other => panic!("деление на ноль обязано давать Failed, получено {other:?}"),
    }
}

#[test]
fn t23_extern_function_with_return_fails_loudly() {
    // Решение ADR: тела нет → отказ, а не тихий ноль.
    let (_, result) = run("extern_ret.lam", 1);
    match result {
        TickResult::Failed(details) => {
            assert!(details.contains("SIM-019"), "детали: {details}");
        }
        other => panic!("внешняя функция со значением обязана давать Failed, получено {other:?}"),
    }
}

#[test]
fn healthy_model_is_not_reported_as_failed() {
    // Контрпример к контрпримерам: исправная модель НЕ должна давать Failed.
    // Без этого теста «объявлять ошибкой всё подряд» прошло бы проверки выше.
    let (_, result) = run("assign_arith.lam", 1);
    assert!(
        !matches!(result, TickResult::Failed(_)),
        "исправная модель не должна отмечаться как ошибочная: {result:?}"
    );
}

// ── Фича 0031: композиция функций (f → g в одной модели) ──────────────────────

/// Композиция функций внутри модели исполняется симулятором:
/// r = f(5) = g(5) + 10 = (5 + 1) + 10 = 16. Сверено с порождённым C (r=16).
#[test]
fn fn_composition_is_evaluated() {
    let (unit, _) = run("fn_composition.lam", 1);
    assert_eq!(num(&unit, "r"), 16, "f→g: (5+1)+10 = 16");
}

// ── Фича 0044: инварианты и assert в симуляторе ──────────────────────────────

/// T14/T15 (A9): нарушение инварианта модели останавливает прогон с SIM-025 и
/// именем 'P'. Значение `c == 1` — проверка сработала ДО `always` второго такта
/// (эталон C: assert до switch), а не после.
#[test]
fn invariant_model_violation_stops_with_sim025() {
    let (unit, last) = run("invariant_violated.lam", 5);
    let TickResult::Failed(msg) = last else {
        panic!("ожидался Failed на нарушенном инварианте, получено {last:?}");
    };
    assert!(msg.contains("SIM-025"), "код SIM-025 в сообщении: {msg}");
    assert!(msg.contains("'P'"), "имя инварианта P в сообщении: {msg}");
    assert_eq!(num(&unit, "c"), 1, "остановка ДО always второго такта");
}

/// T19: истинный инвариант прогону не мешает.
#[test]
fn invariant_holds_does_not_interfere() {
    let (unit, last) = run("invariant_holds.lam", 3);
    assert!(
        !matches!(last, TickResult::Failed(_)),
        "истинный инвариант не должен ронять прогон: {last:?}"
    );
    assert_eq!(num(&unit, "c"), 2, "c растёт нормально");
}

/// T16 (A10): инвариант СОСТОЯНИЯ Q нарушается (проверяется, пока автомат в A).
#[test]
fn invariant_state_violation_stops_with_name() {
    let (_unit, last) = run("invariant_state_violated.lam", 5);
    let TickResult::Failed(msg) = last else {
        panic!("ожидался Failed на инварианте состояния, получено {last:?}");
    };
    assert!(
        msg.contains("SIM-025") && msg.contains("'Q'"),
        "SIM-025 + имя Q: {msg}"
    );
}

/// T17 (A10): `: c;` (assert языка Lam) в блоке нарушается — так же, как invariant.
#[test]
fn assert_in_block_violation_stops() {
    let (_unit, last) = run("assert_in_block.lam", 3);
    let TickResult::Failed(msg) = last else {
        panic!("ожидался Failed на assert в блоке, получено {last:?}");
    };
    assert!(msg.contains("SIM-025"), "код SIM-025: {msg}");
}

/// Фича 0086: переменная без инициализатора существует со значением по умолчанию
/// (нулевым, как default-init в C), а не даёт SIM-009 «переменная не найдена».
///
/// До фикса `var q: u8;` (и любой скаляр без init) в симуляторе не
/// регистрировался — чтение давало SIM-009 (гэп 0034-04, регистрировалась лишь
/// структура). Зонд-значения захвачены прогоном, не угаданы.
#[test]
fn var_without_initializer_defaults_to_zero() {
    let (unit, last) = run("var_no_init.lam", 1);
    assert!(
        !matches!(last, TickResult::Failed(_)),
        "прогон не должен падать (в т.ч. SIM-009), получено {last:?}"
    );

    // Прямое чтение переменных без инициализатора → нулевое значение по типу.
    assert_eq!(num(&unit, "q"), 0, "u8 без init → 0");
    assert_eq!(num(&unit, "flag"), 0, "bit без init → 0");
    assert_eq!(fixed_repr(&unit, "ratio"), 0, "q(8,8) без init → repr 0");

    // Чтение в теле (`seen := var`) прошло без SIM-009 и увидело нули —
    // значит переменная существует, а не «не найдена».
    assert_eq!(num(&unit, "seen_q"), 0, "seen_q := q → 0");
    assert_eq!(num(&unit, "seen_flag"), 0, "seen_flag := flag → 0");
    assert_eq!(
        fixed_repr(&unit, "seen_ratio"),
        0,
        "seen_ratio := ratio → repr 0"
    );
}
