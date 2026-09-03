//! Список зарезервированных имён IEC проверяется ПРОГОНОМ `iec2c` (фича 0342).
//!
//! # Что было
//!
//! Список `IEC_RESERVED` собирался чтением стандарта и **отстал**: замер
//! 2026-08-20 (прогон `iec2c` по 62 именам) показал, что инструмент отвергает
//! **все** 62, а список знал 47. Не хватало `ln`, `log`, `exp`, `sin`, `cos`,
//! `tan`, `asin`, `acos`, `atan`, `trunc`, `mod`, `and`, `or`, `xor`, `not`,
//! `add`, `sub`, `mul`, `div`, `move`, `adr`, `size`, `bcd_to_int`,
//! `int_to_bcd`.
//!
//! Цена: `var ln: u8 := 1;` давал **невалидный** ST при **нулевом** коде
//! возврата `taktc`, а сообщение инструмента («invalid located variable
//! declaration») причины не называло — ровно тот довод, по которому диагностика
//! `ST-014` и заведена.
//!
//! ⚠️ Класс найден **случайно**: имя `ln` стояло в пробе фичи 0341 и дало
//! посторонний отказ, который сперва приписали вложенной структуре (правило 30
//! — снимать причины по одной).
//!
//! # Что здесь
//!
//! Сторож **прогоняет** `iec2c` по каждому имени списка: имя, которое
//! инструмент принимает, в списке лишнее (ложный отказ цели), а контрольное имя
//! обязано приниматься — иначе тест доказывал бы лишь то, что `iec2c` ругается
//! на любой вход.
//!
//! ⚠️ Полноту списка тест **не** доказывает: словаря всех имён MatIEC у нас нет.
//! Он доказывает, что каждая запись **обоснована**, и что проверка не
//! вырождена.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Путь к `iec2c` и каталогу его библиотеки (гейт ставит их в `~/.local`).
fn iec2c() -> Option<(PathBuf, PathBuf)> {
    let home = std::env::var("HOME").ok()?;
    let bin = PathBuf::from(&home).join(".local/bin/iec2c");
    let lib = PathBuf::from(&home).join(".local/share/matiec/lib");
    (bin.is_file() && lib.is_dir()).then_some((bin, lib))
}

/// Принимает ли `iec2c` файл, где `name` — имя переменной.
fn accepts(bin: &Path, lib: &Path, dir: &Path, name: &str) -> bool {
    let source = format!(
        "FUNCTION_BLOCK Probe\nVAR_OUTPUT\n    o : USINT;\nEND_VAR\nVAR\n    \
         {name} : USINT := 1;\nEND_VAR\n    o := {name};\nEND_FUNCTION_BLOCK\n"
    );
    let file = dir.join("probe.st");
    std::fs::write(&file, source).expect("запись пробы");
    let out = dir.join("out");
    std::fs::create_dir_all(&out).expect("каталог вывода");
    Command::new(bin)
        .arg("-I")
        .arg(lib)
        .arg("-T")
        .arg(&out)
        .arg(&file)
        .output()
        .expect("запуск iec2c")
        .status
        .success()
}

/// Принимает ли `iec2c` файл, где `name` — имя ПОЛЯ структуры (фича 0385).
///
/// ⚠️ Проба своя, а не `accepts`: замер показал, что поле и переменная
/// различаются — из 79 имён `IEC_RESERVED` поле принимает **52**, потому что
/// стандартные ФУНКЦИИ внутри `STRUCT` ни с чем не сталкиваются.
fn accepts_field(bin: &Path, lib: &Path, dir: &Path, name: &str) -> bool {
    let source = format!(
        "TYPE\n    ProbeRec :\n    STRUCT\n        {name} : USINT;\n        span : USINT;\n    \
         END_STRUCT;\nEND_TYPE\n\nFUNCTION_BLOCK Probe\nVAR_OUTPUT\n    o : USINT;\nEND_VAR\n\
         VAR\n    rec : ProbeRec;\nEND_VAR\n    o := rec.{name};\nEND_FUNCTION_BLOCK\n"
    );
    let file = dir.join("probe_field.st");
    std::fs::write(&file, source).expect("запись пробы");
    let out = dir.join("out_field");
    std::fs::create_dir_all(&out).expect("каталог вывода");
    Command::new(bin)
        .arg("-I")
        .arg(lib)
        .arg("-T")
        .arg(&out)
        .arg(&file)
        .output()
        .expect("запуск iec2c")
        .status
        .success()
}

/// Имена из списка `IEC_RESERVED` цели.
///
/// ⚠️ Список берётся **из исходника** цели, а не повторяется здесь: вторая
/// копия разошлась бы с первой (класс 0084/0193/0195).
fn reserved_names() -> Vec<String> {
    names_of("const IEC_RESERVED:")
}

/// Имена из списка `IEC_RESERVED_FIELD` цели (фича 0385).
fn reserved_field_names() -> Vec<String> {
    names_of("const IEC_RESERVED_FIELD:")
}

fn names_of(marker: &str) -> Vec<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/generator/st/st_reserved.rs");
    let text = std::fs::read_to_string(&path).expect("исходник st_reserved");
    let start = text.find(marker).expect("список найден");
    let end = text[start..].find("];").expect("конец списка") + start;
    let mut names = Vec::new();
    let mut rest = &text[start..end];
    while let Some(open) = rest.find('"') {
        let tail = &rest[open + 1..];
        let Some(close) = tail.find('"') else { break };
        names.push(tail[..close].to_string());
        rest = &tail[close + 1..];
    }
    names
}

/// Каждое имя списка `iec2c` действительно отвергает.
#[test]
fn every_reserved_name_is_rejected_by_iec2c() {
    let names = reserved_names();
    assert!(
        names.len() > 50,
        "список зарезервированных имён подозрительно мал: {}",
        names.len()
    );

    let Some((bin, lib)) = iec2c() else {
        eprintln!("[ПРОПУСК] every_reserved_name_is_rejected_by_iec2c: iec2c не найден");
        return;
    };
    let dir = std::env::temp_dir()
        .join(format!("takt_pid{}", std::process::id()))
        .join(format!(
            "takt_0342_{}",
            std::thread::current()
                .name()
                .unwrap_or("single")
                .replace(':', "_")
        ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог");

    // Контроль: обычное имя инструмент принимает. Без него тест проходил бы и в
    // мире, где `iec2c` отвергает вообще всё.
    assert!(
        accepts(&bin, &lib, &dir, "counter"),
        "контроль: обычное имя обязано приниматься — иначе проверка вырождена"
    );

    let extra: Vec<String> = names
        .iter()
        .filter(|name| accepts(&bin, &lib, &dir, name))
        .cloned()
        .collect();
    assert!(
        extra.is_empty(),
        "эти имена `iec2c` ПРИНИМАЕТ — значит цель отказывает на них зря: {extra:#?}"
    );
}

/// Каждое имя списка полей `iec2c` отвергает — и НЕ отвергает контрольное.
///
/// ⚠️ Список полей — **свой**, и это замер (фича 0385): из 79 имён
/// `IEC_RESERVED` поле структуры принимает 52. Запретить полю всё подряд
/// значило бы отнять валидную модель (урок фикса 0378-01), поэтому обоснование
/// каждой записи проверяется прогоном.
#[test]
fn every_reserved_field_name_is_rejected_by_iec2c() {
    let names = reserved_field_names();
    assert!(
        names.len() > 20,
        "список имён, запрещённых полю, подозрительно мал: {}",
        names.len()
    );

    let Some((bin, lib)) = iec2c() else {
        eprintln!("[ПРОПУСК] every_reserved_field_name_is_rejected_by_iec2c: iec2c не найден");
        return;
    };
    let dir = std::env::temp_dir()
        .join(format!("takt_pid{}", std::process::id()))
        .join(format!(
            "takt_0385_{}",
            std::thread::current()
                .name()
                .unwrap_or("single")
                .replace(':', "_")
        ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог");

    // Контроль: обычное имя поля инструмент принимает.
    assert!(
        accepts_field(&bin, &lib, &dir, "counter"),
        "контроль: обычное имя поля обязано приниматься — иначе проверка вырождена"
    );
    // Контроль обратный: имя ФУНКЦИИ IEC полем законно (в `IEC_RESERVED` оно
    // есть, а здесь его быть не должно) — иначе список читался бы как «тот же».
    assert!(
        accepts_field(&bin, &lib, &dir, "ln"),
        "контроль: `ln` полем принимается — список полей не равен IEC_RESERVED"
    );

    let extra: Vec<String> = names
        .iter()
        .filter(|name| accepts_field(&bin, &lib, &dir, name))
        .cloned()
        .collect();
    assert!(
        extra.is_empty(),
        "эти имена `iec2c` ПРИНИМАЕТ полем — цель отказывает на них зря: {extra:#?}"
    );
}

/// Ключевое слово ЯЗЫКА ST в имени переменной — отказ `ST-014` (фича 0511).
///
/// `var program: u8;` — имя из практики (выбор загружаемой программы у модели
/// процессора). Замер 2026-09-03: цель печатала его как есть, `iec2c` отвечал
/// «invalid variable(s) declaration» и «invalid test expression defined for ST
/// 'IF' statement» при НУЛЕВОМ коде возврата `taktc`. Прежний список знал
/// функции и типы IEC, но не структуру программы.
#[test]
fn st_keyword_in_variable_name_is_refused() {
    const SOURCE: &str = "var program: u8 := 1;\n\
                          out probe: u8 at 0x100;\n\
                          start Run { always { probe := program; } ref Run; }\n";
    let dir = std::env::temp_dir()
        .join(format!("takt_pid{}", std::process::id()))
        .join(format!(
            "takt_0511_{}",
            std::thread::current()
                .name()
                .unwrap_or("t")
                .replace(':', "_")
        ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог");
    let path = dir.to_str().expect("путь в UTF-8");
    let opts = takt_lang::generator::GenerateOptions::default();

    let err = takt_lang::compile_to_st("probe", SOURCE, path, &[], &opts)
        .expect_err("ключевое слово ST идентификатором быть не может");
    assert_eq!(err.code.as_deref(), Some("ST-014"), "{err:?}");

    // Контроль: прочие цели ту же модель переводят — отказ принадлежит цели,
    // а не языку.
    takt_lang::compile_to_c("probe", SOURCE, path, &[], &opts).expect("цель `c` переводит");
    takt_lang::compile_to_rust("probe", SOURCE, path, &[], &opts).expect("цель `rust` переводит");
    let _ = std::fs::remove_dir_all(&dir);
}

/// **Контроль:** имя, которое `iec2c` ПРИНИМАЕТ, в список не внесено.
///
/// Замер 2026-09-03 прогнал 44 ключевых слова ST: три из них (`single`,
/// `interval`, `priority`) инструмент принимает — отказ на них был бы ложным
/// (урок 0342), и цель обязана их переводить.
#[test]
fn names_accepted_by_iec2c_are_not_refused() {
    for name in ["single", "interval", "priority"] {
        let source = format!(
            "var {name}: u8 := 1;\nout probe: u8 at 0x100;\n\
             start Run {{ always {{ probe := {name}; }} ref Run; }}\n"
        );
        let dir = std::env::temp_dir()
            .join(format!("takt_pid{}", std::process::id()))
            .join(format!(
                "takt_0511_ok_{name}_{}",
                std::thread::current()
                    .name()
                    .unwrap_or("t")
                    .replace(':', "_")
            ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("каталог");
        takt_lang::compile_to_st(
            "probe",
            &source,
            dir.to_str().expect("путь в UTF-8"),
            &[],
            &takt_lang::generator::GenerateOptions::default(),
        )
        .unwrap_or_else(|e| panic!("`{name}` арбитр принимает — отказ был бы ложным: {e:?}"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
