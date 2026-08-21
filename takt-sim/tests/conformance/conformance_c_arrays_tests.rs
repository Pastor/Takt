//! Сверка симулятора с порождённым `taktc -t c` по **массивам** (фича 0076).
//!
//! Вынесено из `conformance_c_tests.rs` (тот упёрся в лимит размера модуля —
//! правило CLAUDE.md; границы модулей = границы ответственности). Здесь — только
//! сверка исполнения массивов симулятором с эталоном C.
//!
//! `Array` **сверяется** с 0076: симулятор исполняет запись в элемент
//! (`data[i] := v`) и список-инициализатор `{…}`, значения совпадают с
//! порождённым C. Тест пришёл на смену сторожу `a9_bit_and_array_conformance_gap`,
//! который фиксировал препятствие `SIM-017` (записи в элемент массива не было).
//!
//! **Вне сверки остаётся `[bit;N]`** — вопрос семантики языка (фича 0078), а не
//! дефект генератора; скалярный инициализатор массива C сам отвергает (CC-017),
//! эталона у него нет.

use std::path::PathBuf;
use std::process::Command;
use takt_lang::semantic::tree::construct_model;
use takt_sim::{Value, build_unit};

/// Тактов на прогон C до установившегося состояния (с запасом на `INIT`).
const MAX_TICKS: usize = 8;

fn cc_available() -> bool {
    Command::new("cc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// **T18 (0029) / фича 0076.** `Array` теперь **сверяется** с порождённым C.
///
/// До 0076 сверки не было (сторож `a9_bit_and_array_conformance_gap` фиксировал
/// препятствие `SIM-017`): симулятор не писал элемент массива и не
/// инициализировал массив. Теперь `data[i] := v` исполняется, список `{…}`
/// приводится поэлементно — сверяем значения `data[i]`/`counter` с синтезом C.
///
/// ⚠️ Модель на `[u8;4]` (элемент `u8` = `Integer{8}` — **скаляр**), поэтому к
/// двойственности `[bit;N]` (0078) отношения нет. Скалярный инициализатор массива
/// (`[u8;4] := 0`) и `[bit;N]` — вне объёма 0076 (0078); C сам scalar-init
/// отвергает (CC-017), поэтому эталона у него нет.
///
/// Наблюдение: поле-массив C печатается поэлементно (`m.entry.data[i]`), симулятор
/// индексирует `Value::Array` — оба против одного эталона (C).
#[test]
fn array_element_matches_generated_c() {
    if !cc_available() {
        eprintln!(
            "[ПРОПУСК] array_element_matches_generated_c: компилятор `cc` не найден — \
             сверка симулятора с порождённым C по массиву не выполнена"
        );
        return;
    }

    // `data` — список-инициализатор (валидная форма); в теле пишем два элемента.
    let source = "\
model ArrConf {
    var data: [u8;4] := {0, 0, 0, 0};
    var counter: u8 := 0;
    start Idle {
        always {
            data[0] := 7;
            data[1] := 200;
            counter := 1;
        }
    }
}
start Entry = ArrConf;
";
    // Симулятор: один такт исполняет тело `always`.
    let (ast, _) = takt_lang::parse(source, 0).expect("разбор");
    let model = construct_model(&ast, None, &[]).expect("семантика");
    let mut unit = build_unit(model).expect("построение юнита");
    let _ = unit.tick();
    let sim_data = match unit.variable("data") {
        Some(Value::Array(items)) => items,
        other => panic!("`data` обязана быть массивом, получено {other:?}"),
    };
    let sim_elem = |i: usize| -> i128 {
        match &sim_data[i] {
            Value::Number(n) => *n,
            other => panic!("data[{i}]: не целое {other:?}"),
        }
    };

    // Порождённый C: собираем харнесс, печатающий data[i] и counter.
    let dir: PathBuf = std::env::temp_dir().join("takt_conformance_0076_array");
    std::fs::create_dir_all(&dir).expect("каталог сборки");
    takt_lang::compile_to_c(
        "arrconf",
        source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect("порождение C");

    let harness = format!(
        r#"#include <stdio.h>
#include "arrconf.h"

int main(void) {{
    Arrconf m;
    Arrconf_init(&m);
    for (int i = 0; i < {MAX_TICKS}; i++) {{
        Arrconf_tick(&m);
        if (Arrconf_is_done(&m)) break;
    }}
    for (int i = 0; i < 4; i++) {{
        printf("data%d=%d\n", i, (int)m.entry.data[i]);
    }}
    printf("counter=%d\n", (int)m.entry.counter);
    return 0;
}}
"#
    );
    let harness_path = dir.join("harness.c");
    std::fs::write(&harness_path, harness).expect("запись харнесса");
    let bin = dir.join("arrconf_bin");
    let compile = Command::new("cc")
        .args(["-std=c11", "-I"])
        .arg(&dir)
        .arg(dir.join("arrconf.c"))
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
    let out = String::from_utf8_lossy(&run.stdout);
    let c_val = |key: &str| -> i128 {
        out.lines()
            .find_map(|l| l.strip_prefix(&format!("{key}="))?.trim().parse().ok())
            .unwrap_or_else(|| panic!("C не напечатал '{key}': {out}"))
    };

    // Сверка: эталон data = [7, 200, 0, 0], counter = 1.
    for i in 0..4 {
        assert_eq!(
            sim_elem(i),
            c_val(&format!("data{i}")),
            "расхождение data[{i}]: симулятор={}, C={}",
            sim_elem(i),
            c_val(&format!("data{i}"))
        );
    }
    assert_eq!(
        sim_elem(0),
        7,
        "эталон data[0] = 7 (запись элемента исполнена)"
    );
    assert_eq!(sim_elem(1), 200, "эталон data[1] = 200");
    assert_eq!(c_val("counter"), 1, "counter = 1 (тело always исполнилось)");
}

/// **Фича 0364.** Вложенный массив `[[u8; 2]; 2]` — значения совпадают с C.
///
/// Прежде цель `c` этот вход **отвергала** (`CC-015` «тип не представим в C»),
/// тогда как эталон, `rust`, `sv` и (после 0363) `st` его исполняют. Дефектов
/// было три, и каждый давал свою форму невалидного C после снятия предыдущего:
/// объявление (`uint8_t grid[2][2]`), инициализация переменной
/// (`model->grid[0] = {1, 2};` — формы нет в C) и присваивание агрегата в теле
/// (та же форма, другой печатник).
///
/// ⚠️ Сверяются ЗНАЧЕНИЯ, а не факт компиляции: перестановка индексов даёт
/// валидный C с другим поведением. Поэтому элементы различны, а один из них
/// перезаписывается в теле.
#[test]
fn nested_array_matches_generated_c() {
    if !cc_available() {
        eprintln!(
            "[ПРОПУСК] nested_array_matches_generated_c: компилятор `cc` не найден — \
             сверка вложенного массива не выполнена"
        );
        return;
    }

    // Размерности РАЗНЫЕ (2 строки по 3): на квадратной матрице перестановка
    // размерностей в объявлении неразличима — мутация «печатать `[3][2]`»
    // компилируется и даёт те же значения.
    let source = "\
model NestConf {
    var grid: [[u8;3];2] := {{1, 2, 3}, {4, 5, 6}};
    var mirror: [[u8;3];2] := {{0, 0, 0}, {0, 0, 0}};
    var picked: u8 := 0;
    var sum: u8 := 0;
    var copied: u8 := 0;
    start Idle {
        always {
            grid[0][1] := 9;
            picked := grid[1][0];
            sum := grid[0][0] + grid[0][1] + grid[1][2];
            mirror := {{7, 8, 9}, {10, 11, 12}};
            copied := mirror[1][2];
        }
    }
}
start Entry = NestConf;
";
    let (ast, _) = takt_lang::parse(source, 0).expect("разбор");
    let model = construct_model(&ast, None, &[]).expect("семантика");
    let mut unit = build_unit(model).expect("построение юнита");
    let _ = unit.tick();
    let sim_scalar = |unit: &takt_sim::Unit, name: &str| -> i128 {
        match unit.variable(name) {
            Some(Value::Number(n)) => n,
            other => panic!("{name}: не целое {other:?}"),
        }
    };
    let sim_picked = sim_scalar(&unit, "picked");
    let sim_sum = sim_scalar(&unit, "sum");
    let sim_copied = sim_scalar(&unit, "copied");

    let dir: PathBuf = std::env::temp_dir().join("takt_conformance_0364_nested");
    std::fs::create_dir_all(&dir).expect("каталог сборки");
    takt_lang::compile_to_c(
        "nestconf",
        source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect("порождение C");

    let harness = format!(
        r#"#include <stdio.h>
#include "nestconf.h"

int main(void) {{
    Nestconf m;
    Nestconf_init(&m);
    for (int i = 0; i < {MAX_TICKS}; i++) {{
        Nestconf_tick(&m);
        if (Nestconf_is_done(&m)) break;
    }}
    printf("picked=%d\n", (int)m.entry.picked);
    printf("sum=%d\n", (int)m.entry.sum);
    printf("copied=%d\n", (int)m.entry.copied);
    for (int r = 0; r < 2; r++) {{
        for (int c = 0; c < 3; c++) {{
            printf("g%d%d=%d\n", r, c, (int)m.entry.grid[r][c]);
        }}
    }}
    return 0;
}}
"#
    );
    let harness_path = dir.join("harness_nested.c");
    std::fs::write(&harness_path, harness).expect("запись харнесса");
    let bin = dir.join("nestconf_bin");
    let compile = Command::new("cc")
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-I"])
        .arg(&dir)
        .arg(dir.join("nestconf.c"))
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
    let out = String::from_utf8_lossy(&run.stdout);
    let c_val = |key: &str| -> i128 {
        out.lines()
            .find_map(|l| l.strip_prefix(&format!("{key}="))?.trim().parse().ok())
            .unwrap_or_else(|| panic!("C не напечатал '{key}': {out}"))
    };

    assert_eq!(
        sim_picked,
        c_val("picked"),
        "расхождение picked: симулятор={sim_picked}, C={}",
        c_val("picked")
    );
    assert_eq!(
        sim_sum,
        c_val("sum"),
        "расхождение sum: симулятор={sim_sum}, C={}",
        c_val("sum")
    );
    assert_eq!(
        sim_copied,
        c_val("copied"),
        "расхождение copied: симулятор={sim_copied}, C={}",
        c_val("copied")
    );
    // Значения эталона: элементы различны, одна ячейка перезаписана в теле, а
    // вторая переменная получает агрегат целиком — на симметричной матрице и
    // на одинаковых значениях перестановка индексов была бы неразличима.
    assert_eq!(
        sim_picked, 4,
        "grid[1][0] = 4 (вторая строка, первый столбец)"
    );
    assert_eq!(sim_sum, 16, "1 + 9 + 6");
    assert_eq!(sim_copied, 12, "mirror[1][2] после присваивания агрегата");
    assert_eq!(c_val("g01"), 9, "запись grid[0][1] дошла до C");
    assert_eq!(c_val("g10"), 4, "grid[1][0] не затронут записью");
}

/// **Фича 0368.** Элемент агрегата печатается ПО ТИПУ ЭЛЕМЕНТА.
///
/// Замер 2026-08-21: `var gains: [q(8, 8); 2] := {1.5, 2.5};` доезжало до
/// целей дробным литералом — `cc -Werror` отвечал «implicit conversion from
/// 'double' to 'int16_t' changes value from 1.5 to 1», `rustc` — `E0308`, а
/// `sv` — `SV-002`; `var modes: [Mode; 2] := {Idle, Work};` давало у `rust`
/// `[0, 1]` в поле `[Mode; 2]`. Та же запись **скаляром** работает у всех
/// девяти потребителей.
///
/// ⚠️ Сверяются ЗНАЧЕНИЯ: понижение q-литерала — это умножение на 2ⁿ, и
/// ошибка в нём даёт валидный C с другим числом.
#[test]
fn aggregate_element_types_match_generated_c() {
    if !cc_available() {
        eprintln!("[ПРОПУСК] aggregate_element_types_match_generated_c: компилятор `cc` не найден");
        return;
    }

    let source = "\
enum Mode { Idle = 0, Work = 7 }
model AggElem {
    var gains: [q(8,8); 2] := {1.5, 2.5};
    var modes: [Mode; 2] := {Idle, Work};
    var whole: u8 := 0;
    var code: u8 := 0;
    start Idle2 {
        always {
            whole := (gains[0] + gains[1]) as u8;
            modes[0] := Work;
            code := modes[0] as u8;
        }
    }
}
start Entry = AggElem;
";
    let (ast, _) = takt_lang::parse(source, 0).expect("разбор");
    let model = construct_model(&ast, None, &[]).expect("семантика");
    let mut unit = build_unit(model).expect("построение юнита");
    let _ = unit.tick();
    let sim = |name: &str| -> i128 {
        match unit.variable(name) {
            Some(Value::Number(n)) => n,
            other => panic!("{name}: не целое {other:?}"),
        }
    };
    let sim_whole = sim("whole");
    let sim_code = sim("code");
    assert_eq!(sim_whole, 4, "1.5 + 2.5 = 4.0");
    assert_eq!(sim_code, 7, "modes[0] := Work → 7");

    let dir: PathBuf = std::env::temp_dir().join("takt_conformance_0368_aggelem");
    std::fs::create_dir_all(&dir).expect("каталог сборки");
    takt_lang::compile_to_c(
        "aggelem",
        source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect("порождение C");

    let harness = format!(
        r#"#include <stdio.h>
#include "aggelem.h"

int main(void) {{
    Aggelem m;
    Aggelem_init(&m);
    for (int i = 0; i < {MAX_TICKS}; i++) {{
        Aggelem_tick(&m);
        if (Aggelem_is_done(&m)) break;
    }}
    printf("whole=%d\n", (int)m.entry.whole);
    printf("code=%d\n", (int)m.entry.code);
    printf("g0=%d\n", (int)m.entry.gains[0]);
    return 0;
}}
"#
    );
    let harness_path = dir.join("harness_aggelem.c");
    std::fs::write(&harness_path, harness).expect("запись харнесса");
    let bin = dir.join("aggelem_bin");
    let compile = Command::new("cc")
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-I"])
        .arg(&dir)
        .arg(dir.join("aggelem.c"))
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
    let out = String::from_utf8_lossy(&run.stdout);
    let c_val = |key: &str| -> i128 {
        out.lines()
            .find_map(|l| l.strip_prefix(&format!("{key}="))?.trim().parse().ok())
            .unwrap_or_else(|| panic!("C не напечатал '{key}': {out}"))
    };

    assert_eq!(sim_whole, c_val("whole"), "расхождение whole");
    assert_eq!(sim_code, c_val("code"), "расхождение code");
    // Понижение q-литерала: 1.5 в q(8, 8) — это 384, а не 1.
    assert_eq!(c_val("g0"), 384, "литерал 1.5 понижен в q-представление");
}
