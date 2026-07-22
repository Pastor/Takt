//! Сверка симулятора с порождённым `lamc -t c` по **массивам** (фича 0076).
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

use grammar::semantic::tree::construct_model;
use simulation::{Value, build_unit};
use std::path::PathBuf;
use std::process::Command;

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
    let (ast, _) = grammar::parse(source, 0).expect("разбор");
    let model = construct_model(&ast, None, &[]).expect("семантика");
    let mut unit = build_unit(model).expect("построение юнита");
    let _ = unit.tick();
    let sim_data = match unit.variable("data") {
        Some(Value::Array(items)) => items,
        other => panic!("`data` обязана быть массивом, получено {other:?}"),
    };
    let sim_elem = |i: usize| -> i64 {
        match &sim_data[i] {
            Value::Number(n) => *n,
            other => panic!("data[{i}]: не целое {other:?}"),
        }
    };

    // Порождённый C: собираем харнесс, печатающий data[i] и counter.
    let dir: PathBuf = std::env::temp_dir().join("lam_conformance_0076_array");
    std::fs::create_dir_all(&dir).expect("каталог сборки");
    grammar::compile_to_c(
        "arrconf",
        source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &grammar::generator::GenerateOptions::default(),
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
    let c_val = |key: &str| -> i64 {
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
