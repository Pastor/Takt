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
