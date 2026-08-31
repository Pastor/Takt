//! Экспорт корпуса матрицы для плагинов редакторов (фича 0465).
//!
//! # Зачем файлы, а не память
//!
//! Плагины живут **вне** Rust-дерева: IntelliJ собирается своим Gradle
//! (`./gradlew --offline test`, вне `precheck.sh`), Zed — самим редактором.
//! Подать им входы матрицы можно только файлами, и этот набор их пишет —
//! в `target/matrix-corpus/`, откуда их читает корпусный тест PSI
//! (`TaktPsiCorpusTest`), сверяющий round-trip байт-в-байт.
//!
//! ⚠️ Каталог **порождаемый**, а не хранимый: генератор матрицы меняется, и
//! committed-копия отстала бы молча. Оттого он и лежит в `target/`, который не
//! отслеживается.
//!
//! ⚠️ Набор ничего не проверяет у плагина — он лишь готовит вход. Проверка
//! round-trip принадлежит Gradle-тесту, и запускает её человек (правило 29:
//! плагин IntelliJ машиной проекта не защищён).

use std::path::PathBuf;

use super::matrix_probes::{case_name, cases, library_files, source};

/// Корень репозитория: подъём от каталога набора.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("у крейта есть родитель")
        .to_path_buf()
}

/// Пишет корпус матрицы в `target/matrix-corpus/`.
#[test]
fn matrix_corpus_is_exported_for_editor_plugins() {
    let dir = repo_root().join("target").join("matrix-corpus");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог корпуса");

    let all = cases();
    for (shape, touch, kind) in &all {
        let name = case_name(*shape, *touch, *kind);
        let case_dir = dir.join(&name);
        std::fs::create_dir_all(&case_dir).expect("каталог случая");
        std::fs::write(case_dir.join("probe.takt"), source(*shape, *touch, *kind))
            .expect("запись пробы");
        // Подключаемые файлы кладутся рядом: разбор плагина обязан пережить и
        // импортирующий файл, и библиотеку.
        for file in library_files(*touch) {
            std::fs::write(case_dir.join(file.name), file.text).expect("запись библиотеки");
        }
    }

    let written = std::fs::read_dir(&dir).expect("каталог читается").count();
    assert_eq!(
        written,
        all.len(),
        "корпус матрицы выгружен не полностью: {written} из {}",
        all.len()
    );
}
