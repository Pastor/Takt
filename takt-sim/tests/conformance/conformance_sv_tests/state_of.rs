//! Наблюдатель-сосед видит состояние соседа — эталон ≡ `sv` (фича 0267).
//!
//! # Что проверяется
//!
//! Запись `S(Модель) = Состояние`, сделанную **из соседней модели** композиции,
//! цель `sv` до фичи отвергала (`SV-002` «ссылка на модель в условии»), хотя
//! средства у неё есть: композиция уплощается, и регистр состояния соседа лежит
//! в том же `always_comb`.
//!
//! # Почему вердикт даёт трасса, а не гейт
//!
//! `verilator --lint-only` и `yosys` принимают модуль независимо от того, из
//! какого сигнала читается состояние: регистра (`..._state`) или рабочей копии
//! (`..._state_next`). Разница между ними — **такт**: наблюдатель обязан увидеть
//! переход соседа на том же такте, на котором тот произошёл, как в эталоне и в
//! цели `c` (правило 0245). Ошибка здесь молча сдвигает трассу на такт, и
//! поймать её может только сверка значений.
//!
//! Наблюдаемая — `seen`: такт, на котором наблюдатель увидел чужое состояние.

use super::*;

/// `Worker` уходит `Busy → Done` на третьем такте; `Watcher` следит за ним и с
/// этого же такта копит `seen`.
const SRC: &str = "model Worker {\n\
    \x20   var n: u8 := 0;\n\n\
    \x20   start Busy {\n\
    \x20       always { n := n + 1; }\n\
    \x20       ref Done: n = 3;\n\
    \x20   }\n\n\
    \x20   state Done;\n\
    }\n\n\
    model Watcher {\n\
    \x20   var seen: u8 := 0;\n\n\
    \x20   start Wait {\n\
    \x20       ref Seen: S(Worker) = Done;\n\
    \x20   }\n\n\
    \x20   state Seen {\n\
    \x20       always { seen := seen + 1; }\n\
    \x20   }\n\
    }\n\n\
    start Main = Worker | Watcher;\n";

#[test]
fn neighbour_state_check_matches_generated_sv() {
    if !verilator_available() {
        eprintln!("verilator недоступен — сверка `sv` пропущена");
        return;
    }
    let dir = build_dir("state_of");
    let path = fixture(&dir, "dual", SRC);

    let expected = simulate_trace(
        path.to_str().expect("путь в UTF-8"),
        &["Worker::n", "Watcher::seen"],
    );
    let actual = sv_trace(
        &dir,
        path.to_str().expect("путь в UTF-8"),
        "dual",
        &["dual_worker_n", "dual_watcher_seen"],
        6,
    );
    // Эталон останавливается по завершении автомата, RTL тикает дальше:
    // сравниваются такты, которые прошли оба. Обрезка — по длине эталона, а не
    // по фиксированному числу: иначе тест сверял бы меньше, чем мог.
    let common: Vec<Vec<i128>> = actual.iter().take(expected.len()).cloned().collect();
    assert_eq!(
        common, expected,
        "трасса порождённого SystemVerilog разошлась с эталоном:\nsv       = {actual:?}\nэталон   = {expected:?}"
    );
    assert!(
        expected.len() >= 4,
        "эталон обязан дойти до такта, на котором наблюдатель увидел соседа: {expected:?}"
    );
}
