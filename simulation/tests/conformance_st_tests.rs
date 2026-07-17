//! Потактовая сверка цели **Structured Text** (IEC 61131-3) с симулятором —
//! задача 0065-03, отдача долга фичи 0041.
//!
//! ## Зачем эта сверка вообще нужна
//!
//! Гейт `iec2c` доказывает, что порождённый ST **компилируется**, но **не** что
//! он **верен**: молча-неверная трансляция компилируется тоже. Ровно так дожил
//! [фикс 0041-01](../../docs/fixes/0041-01-st-fn-dedup-silent.md) (Tier 1): цель
//! `st` склеивала одноимённые `fn` разных моделей по голому имени, `iec2c`
//! принимал результат (`rc = 0`), а модель `B` молча считала не то. До этой
//! сверки поведение ST не проверялось **никогда** (были `conformance_{c,sv,rust}`,
//! `st` — нет). Правило проекта (уроки 0045/0050): сверку заводить вместе с
//! бэкендом; для `st` это долг с 0041, и он здесь отдан.
//!
//! ## Как исполняется порождённый ST
//!
//! Рантайма к ST не прилагается, поэтому: `lamc -t st` → `iec2c` транслирует ST
//! в C (`POUS.c`/`POUS.h`) → к нему пишется драйвер, который делает такт вызовом
//! `{Root}_body__`, читает наблюдаемые переменные и печатает их. Наблюдение
//! берётся оттуда, откуда его берёт потребитель (правило проекта): у ПЛК — это
//! поля экземпляров `FUNCTION_BLOCK` (аналог `dut.<сигнал>` у цели `sv`).
//!
//! ⚠️ **Капкан рантайма MatIEC — ноль-инициализация.** `__INIT_VAR` выставляет
//! флаги через `|=`, не обнуляя их, поэтому по неинициализированной структуре
//! (`{Root}_data__ fb;` — мусор на стеке) мусорный бит `__IEC_FORCE_FLAG`
//! блокирует `__SET_VAR` — автомат стоит на месте. Драйвер обязан обнулить
//! структуру: `{Root}_data__ fb = {0};`. `EN` каждого FB `init` ставит в TRUE
//! сам — руками взводить не нужно (снято спайком 0065-03).
//!
//! ## Мягкая деградация
//!
//! `iec2c` собирается `scripts/ensure-iec2c.sh` и пакетом не поставляется. Как и
//! гейт ST в `precheck.sh`, сверка **не валит** сборку при отсутствии
//! инструмента: нет `iec2c`/`cc`/заголовков MatIEC → тест-пропуск, а не красный.

use grammar::semantic::tree::construct_model;
use simulation::{TickResult, Unit, Value, build_unit};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Фикстура фикса 0041-01: две модели с одноимённой `fn helper` (тела `x+1` и
/// `x+2`), переменные названы по-разному (`wa`/`wb`) ради раздельного наблюдения.
const FIXTURE: &str = "tests/data/eval/st_dup_fn.lam";

/// Имя корневой модели в C-символах `iec2c`. Идентификаторы IEC
/// **регистронезависимы**, и iec2c печатает их в ВЕРХНЕМ регистре
/// (`STDUPFN_data__`/`STDUPFN_body__`), поэтому здесь — не `StDupFn`.
const ROOT: &str = "STDUPFN";

/// Тактов на прогон каждой стороны до установившегося состояния (с запасом).
const MAX_TICKS: usize = 8;

/// Наблюдаемые точки: `(имя в симуляторе, путь поля в структуре POUS)`.
///
/// Путь поля задан именами экземпляров MatIEC (`A0`/`B1` — модель + порядковый
/// номер) и полями (`WA`/`WB` — имя переменной в верхнем регистре). Оба
/// детерминированы (генерация 0048 + правила именования iec2c), поэтому для этой
/// фикстуры фиксируются здесь.
const OBSERVED: &[(&str, &str)] = &[("wa", "A0.WA"), ("wb", "B1.WB")];

/// Директория установки MatIEC (та же, что у `scripts/ensure-iec2c.sh`).
fn iec2c_prefix() -> PathBuf {
    if let Ok(p) = std::env::var("IEC2C_PREFIX") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".local")
}

/// `(бинарник iec2c, каталог lib MatIEC)` — если оба на месте.
fn iec2c_available() -> Option<(PathBuf, PathBuf)> {
    let prefix = iec2c_prefix();
    let bin = prefix.join("bin").join("iec2c");
    let lib = prefix.join("share").join("matiec").join("lib");
    // `-I lib` обязателен (иначе iec2c падает на любом входе), а `lib/C` — путь
    // для `cc` к рантайм-заголовкам. Проверяем ключевой заголовок.
    if bin.is_file() && lib.join("C").join("iec_std_lib.h").is_file() {
        Some((bin, lib))
    } else {
        None
    }
}

fn cc_available() -> bool {
    Command::new("cc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Прогоняет симулятор до завершения и возвращает наблюдаемые значения.
fn simulate() -> Vec<(String, i64)> {
    let source = std::fs::read_to_string(FIXTURE).expect("фикстура читается");
    let (ast, _) = grammar::parse(&source, 0).expect("разбор");
    let model = construct_model(&ast, None, &[]).expect("семантика");
    let mut unit = build_unit(model).expect("построение юнита");
    for _ in 0..MAX_TICKS {
        let result = unit.tick();
        assert!(
            !matches!(result, TickResult::Failed(_)),
            "симуляция не должна падать: {result:?}"
        );
        if result == TickResult::Terminated {
            break;
        }
    }
    OBSERVED
        .iter()
        .map(|(name, _)| (name.to_string(), sim_value(&unit, name)))
        .collect()
}

fn sim_value(unit: &Unit, name: &str) -> i64 {
    match unit.variable(name) {
        Some(Value::Number(n)) => n,
        Some(Value::Boolean(b)) => i64::from(b),
        other => panic!("переменная '{name}': неожиданное значение {other:?}"),
    }
}

/// Порождает ST, транслирует его `iec2c` в C, собирает с драйвером и возвращает
/// значения, напечатанные исполнённым ST.
fn run_generated_st(dir: &Path, iec2c: &Path, lib: &Path) -> Vec<(String, i64)> {
    // 1. Порождение ST — тем же путём, что `lamc compile -t st`.
    let source = std::fs::read_to_string(FIXTURE).expect("фикстура читается");
    let st_dir = dir.join("st");
    std::fs::create_dir_all(&st_dir).expect("каталог ST");
    grammar::compile_to_st(
        "st_dup_fn.lam",
        &source,
        st_dir.to_str().expect("путь в UTF-8"),
        &[],
        &grammar::generator::GenerateOptions::default(),
    )
    .expect("порождение ST");
    let st_file = st_dir.join("st_dup_fn.st");

    // 2. Трансляция ST → C через iec2c (POUS.c/POUS.h кладутся в рабочий каталог).
    let work = dir.join("iec2c");
    std::fs::create_dir_all(&work).expect("рабочий каталог iec2c");
    let transpile = Command::new(iec2c)
        .arg("-I")
        .arg(lib)
        .arg(st_file)
        .current_dir(&work)
        .output()
        .expect("запуск iec2c");
    assert!(
        transpile.status.success() && work.join("POUS.c").is_file(),
        "iec2c не оттранслировал порождённый ST:\n{}",
        String::from_utf8_lossy(&transpile.stderr)
    );

    // 3. Драйвер: обнуляет структуру (см. заголовок про мусорные флаги),
    //    инициализирует, крутит такты до завершения, печатает наблюдаемое.
    let prints = OBSERVED
        .iter()
        .map(|(name, path)| format!(r#"    printf("{name}=%u\n", (unsigned)fb.{path}.value);"#))
        .collect::<Vec<_>>()
        .join("\n");
    let harness = format!(
        r#"#include <stdio.h>
#include "iec_std_lib.h"
TIME __CURRENT_TIME;
BOOL __DEBUG = 0;
#include "POUS.h"
#include "POUS.c"

int main(void) {{
    {ROOT}_data__ fb = {{0}};
    {ROOT}_init__(&fb, __BOOL_LITERAL(FALSE));
    for (int i = 0; i < {MAX_TICKS}; i++) {{
        {ROOT}_body__(&fb);
        if (fb.IS_DONE.value) break;
    }}
{prints}
    return 0;
}}
"#
    );
    let harness_path = work.join("harness.c");
    std::fs::write(&harness_path, harness).expect("запись драйвера");

    // 4. Сборка: `-w` глушит шум заголовков MatIEC (-Wpointer-sign/-Wvarargs) —
    //    он не относится к порождённому нами коду.
    let bin = work.join("st_conformance_bin");
    let compile = Command::new("cc")
        .args(["-std=c99", "-w", "-I"])
        .arg(lib.join("C"))
        .arg("-I")
        .arg(&work)
        .arg(&harness_path)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("запуск cc");
    assert!(
        compile.status.success(),
        "порождённый ST (через iec2c) не собирается:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );

    // 5. Запуск и разбор.
    let run = Command::new(&bin).output().expect("запуск драйвера ST");
    assert!(run.status.success(), "драйвер ST завершился с ошибкой");
    String::from_utf8_lossy(&run.stdout)
        .lines()
        .filter_map(|line| {
            let (name, value) = line.split_once('=')?;
            Some((name.to_string(), value.trim().parse().ok()?))
        })
        .collect()
}

/// **T3/A3 — ОСНОВНАЯ.** Трасса порождённого ST совпадает с симулятором на
/// фикстуре одноимённых `fn`: модель `B` обязана дать `wb = 3` (своей функцией
/// `x+2`), а не `2` (склейка с телом модели `A`).
///
/// ⚠️ **Мутация (T13):** вернуть дедупликацию по голому имени в
/// `st_func.rs::emit_functions` → `wb` станет `2`, сверка **упадёт**, а гейт
/// `iec2c` останется зелёным. Это и есть смысл теста: он ловит то, чего гейт не
/// видит.
#[test]
fn per_tick_trace_matches_generated_st() {
    let Some((iec2c, lib)) = iec2c_available() else {
        eprintln!("iec2c/заголовки MatIEC недоступны — сверка ST пропущена (мягкая деградация)");
        return;
    };
    if !cc_available() {
        eprintln!("cc недоступен — сверка ST пропущена (мягкая деградация)");
        return;
    }

    let dir = std::env::temp_dir().join(format!("st_conf_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("рабочий каталог");

    let from_sim = simulate();
    let from_st = run_generated_st(&dir, &iec2c, &lib);

    // Обе стороны обязаны дать эталон фикса: wa = 2, wb = 3.
    assert_eq!(
        from_sim,
        vec![("wa".to_string(), 2), ("wb".to_string(), 3)],
        "эталон симулятора нарушен: ожидались wa=2, wb=3"
    );
    assert_eq!(
        from_st, from_sim,
        "трасса ST разошлась с симулятором — молча-неверная трансляция ST \
         (класс фикса 0041-01)"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
