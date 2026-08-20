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

/// Имена из списка `IEC_RESERVED` цели.
///
/// ⚠️ Список берётся **из исходника** цели, а не повторяется здесь: вторая
/// копия разошлась бы с первой (класс 0084/0193/0195).
fn reserved_names() -> Vec<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/generator/st/st_reserved.rs");
    let text = std::fs::read_to_string(&path).expect("исходник st_reserved");
    let start = text.find("const IEC_RESERVED").expect("список найден");
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
    let dir = std::env::temp_dir().join(format!(
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
