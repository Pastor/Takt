//! Заглушки генератора C: диагностика вместо тихого пропуска (фича 0028).
//!
//! # Что охраняется
//!
//! Непереводимое условие перехода **не должно** молча превращаться в
//! комментарий. Прежде `generate_state_transitions` печатала в порождаемый C
//! `//TODO: условный переход … не поддерживается` и **проглатывала** `Err`:
//! переход не генерировался, `taktc` печатал «Скомпилировано» и завершался с
//! кодом `0`, а порождённый C **собирался без замечаний** — комментарий сборку
//! не ломает. На выходе — **мёртвый автомат**, обнаруживаемый на объекте, а не
//! в CI. Для языка синтеза систем управления (правило 12) это худший режим
//! отказа.
//!
//! # Пара «пример / контрпример»
//!
//! Фикстуры отличаются **ровно одним** — типом порта, что отделяет проверяемое
//! поведение от всего прочего:
//! - `float_bit_access.lam` — `BitAccess` на `float`-порт: в C не транслируется
//!   (`CC-001`) → ожидается `CC-018`;
//! - `u8_bit_access.lam` — тот же автомат на `u8`: транслируется → переходы на
//!   месте, поведение **не изменилось**.
//!
//! Контрпример охраняет от переусердствования: ошибка не должна расползтись на
//! переводимые условия.
//!
//! # Ожидания захвачены зондом
//!
//! Строки сообщений и вывода **не угаданы** (`CLAUDE.md`): сперва зонд печатал
//! реальные значения, затем они зафиксированы здесь.

use std::path::{Path, PathBuf};
use takt_lang::diagnostics::Location;
use takt_lang::semantic::tree::construct_model;
use takt_lang::{GenerateOptions, compile_to_c};

const FIXTURE_ERR: &str = "tests/data/c_stubs/float_bit_access.lam";
const FIXTURE_OK: &str = "tests/data/c_stubs/u8_bit_access.lam";

/// Порождает C во временный каталог, возвращая результат как есть.
fn compile(
    fixture: &str,
    name: &str,
    out: &Path,
) -> Result<(), takt_lang::diagnostics::Diagnostic> {
    let source = std::fs::read_to_string(fixture).expect("фикстура читается");
    compile_to_c(
        name,
        &source,
        out.to_str().expect("путь в UTF-8"),
        &[],
        &GenerateOptions::default(),
    )
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lam_c_stubs_{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог сборки");
    dir
}

/// **T1.** Непереводимое условие → ошибка, а не «успех» с мёртвым автоматом.
#[test]
fn float_bit_access_is_error() {
    let dir = temp_dir("t1");
    let diag = compile(FIXTURE_ERR, "float_bit_access", &dir)
        .expect_err("непереводимое условие обязано давать ошибку, а не Ok");
    assert_eq!(diag.code.as_deref(), Some("CC-018"));
}

/// **T3.** Сообщение называет **целевое состояние** — иначе в модели с десятком
/// переходов непонятно, какой именно не транслировался.
///
/// # Почему не проверяется конкретное имя
///
/// Тест-план ожидал дословно `Work`. **Ожидание неисполнимо:** в фикстуре два
/// непереводимых перехода (`→ Work` и `→ End`), а какой из них всплывёт первым,
/// зависит от порядка обхода `states` — это `HashMap` (`semantic/mod.rs`),
/// поэтому порядок меняется от прогона к прогону. Зонд дал `Work`, первый же
/// прогон теста — `End`. Дословная проверка была бы «мигающим» тестом.
///
/// Проверяется то, что от сообщения действительно требуется: оно называет
/// **какое-то** из целевых состояний фикстуры, а не молчит о месте. Это
/// проявление известной недетерминированности генерации (кандидат в
/// `FEATURES.md`), а не дефект фичи 0028.
#[test]
fn cc018_message_names_target_state() {
    let dir = temp_dir("t3");
    let diag = compile(FIXTURE_ERR, "float_bit_access", &dir).expect_err("ожидалась ошибка");
    assert!(
        diag.message.contains("Work") || diag.message.contains("End"),
        "сообщение обязано называть целевое состояние перехода: {}",
        diag.message
    );
}

/// **T4.** Причина сохранена: и код исходной диагностики, и её текст.
///
/// Код `CC-001` доходит **заметкой** — схлопывать его в строку сообщения
/// нельзя, иначе теряется машинно-читаемая причина.
#[test]
fn cc018_keeps_cause() {
    let dir = temp_dir("t4");
    let diag = compile(FIXTURE_ERR, "float_bit_access", &dir).expect_err("ожидалась ошибка");
    assert!(
        diag.message.contains("BitAccess на float-порт"),
        "текст причины обязан дойти до пользователя: {}",
        diag.message
    );
    assert!(
        diag.notes.iter().any(|n| n.message.contains("CC-001")),
        "код исходной причины обязан дойти заметкой: {:?}",
        diag.notes
    );
}

/// **T5.** Позиция указывает на `ref` в **исходнике**, а не на `Codegen`.
///
/// Без этого пользователю негде искать причину: `Location::Codegen` не
/// указывает ни на одну строку его файла.
#[test]
fn cc018_points_to_source() {
    let dir = temp_dir("t5");
    let diag = compile(FIXTURE_ERR, "float_bit_access", &dir).expect_err("ожидалась ошибка");
    assert!(
        matches!(diag.loc, Location::Source(..)),
        "ошибка обязана указывать на позицию ref в исходнике, получено {:?}",
        diag.loc
    );
}

/// **T6.** При ошибке артефакты не остаются на диске.
///
/// Частичный C здесь неотличим от полного и потому опаснее его отсутствия.
#[test]
fn cc018_writes_no_artifacts() {
    let dir = temp_dir("t6");
    let _ = compile(FIXTURE_ERR, "float_bit_access", &dir).expect_err("ожидалась ошибка");
    assert!(
        !dir.join("float_bit_access.c").exists() && !dir.join("float_bit_access.h").exists(),
        "при ошибке .c/.h не должны создаваться: {:?}",
        std::fs::read_dir(&dir).map(|d| d.count())
    );
}

/// **T7, T8.** Контрпример: переводимое условие компилируется, переходы на
/// месте, заглушек в выводе нет.
#[test]
fn u8_bit_access_generates_transitions() {
    let dir = temp_dir("t7");
    compile(FIXTURE_OK, "u8_bit_access", &dir)
        .expect("переводимое условие обязано компилироваться");
    let source = std::fs::read_to_string(dir.join("u8_bit_access.c")).expect(".c порождён");
    // Строка захвачена зондом, не угадана.
    assert!(
        source.contains(
            "if ((((*model->read_numeric)(U8_BIT_ACCESS_FLAGS, model->userdata) >> 0) & 1u) == 1) {"
        ),
        "переход по биту 0 обязан быть на месте:\n{source}"
    );
    assert!(
        !source.contains("TODO") && !source.contains("FIXME"),
        "заглушек в выводе быть не должно:\n{source}"
    );
}

/// **T10.** Цель `c-hal` даёт **тот же** отказ: обе цели делят
/// `generate_state_transitions`, и расхождение между ними — отдельный класс
/// дефектов (урок закрытой 0026).
#[test]
fn cc018_same_for_c_hal() {
    let dir = temp_dir("t10");
    let source = std::fs::read_to_string(FIXTURE_ERR).expect("фикстура читается");
    let mut options = GenerateOptions::default();
    options.hal = true;
    let (ast, _) = takt_lang::parse(&source, 0).expect("разбор");
    let model = construct_model(&ast, None, &[]).expect("семантика");
    model.borrow_mut().name = Some("FloatBitAccess".to_string());
    let diag = takt_lang::generator::generate(
        takt_lang::generator::Language::C,
        &model.borrow(),
        dir.to_str().expect("путь в UTF-8"),
        &options,
    )
    .expect_err("цель c-hal обязана отказать так же, как c");
    assert_eq!(diag.code.as_deref(), Some("CC-018"));
}

/// **T16 (A7).** Защита корпуса от возврата заглушек.
///
/// Проверка **грепом по порождённому**, а не сверкой с эталонными строками:
/// вывод генератора недетерминирован (`HashMap` в `semantic/mod.rs`), поэтому
/// побайтовые эталоны здесь невозможны в принципе.
#[test]
fn corpus_generates_no_stubs() {
    let dir = temp_dir("corpus");
    let mut checked = 0;
    for entry in std::fs::read_dir("../examples").expect("каталог examples") {
        let path = entry.expect("запись каталога").path();
        if path.extension().and_then(|e| e.to_str()) != Some("lam") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("имя файла")
            .to_string();
        let source = std::fs::read_to_string(&path).expect("исходник читается");
        let out = dir.join(&name);
        compile_to_c(
            &name,
            &source,
            out.to_str().expect("путь в UTF-8"),
            &["../examples/include".to_string()],
            &GenerateOptions::default(),
        )
        .unwrap_or_else(|d| panic!("{name}: корпус обязан компилироваться: {d:?}"));
        let c = std::fs::read_to_string(out.join(format!("{name}.c"))).expect(".c порождён");
        assert!(
            !c.contains("TODO") && !c.contains("FIXME"),
            "{name}: в порождённом C появилась заглушка"
        );
        checked += 1;
    }
    assert!(
        checked >= 5,
        "ожидалось ≥5 примеров корпуса, найдено {checked}"
    );
}
