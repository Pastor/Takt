//! Поиск и чтение файлов импорта для BuT.
//!
//! Основная функция [`read_import_file`] ищет `.but`-файл по списку директорий,
//! проверяет расширение и возвращает содержимое вместе с полным путём к файлу.

use crate::diagnostics::Diagnostic;
use crate::parser::ast::ImportPath;
use itertools::Itertools;
use std::fs::{exists, read_to_string};

/// Читает содержимое файла импорта, используя путь [`ImportPath`].
///
/// Перебирает `search_paths` в порядке объявления и возвращает первый найденный файл.
///
/// # Параметры
///
/// - `search_paths` — список директорий для поиска файлов (например, `["/src", "/lib"]`).
/// - `path` — путь импорта: строковый литерал (`"file.but"`) или
///   идентификаторный путь (`a::b::c` → `a/b/c.but`).
///
/// # Возвращает
///
/// Пару `(содержимое_файла, полный_путь)` при успехе.
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`], если:
/// - файл не найден ни в одной из директорий поиска;
/// - файл имеет расширение, отличное от `.but`;
/// - файл не удалось прочитать (нет прав, ошибка ввода-вывода и т.д.).
///
/// # Примеры
///
/// Использование через интеграционные тесты — см. `tests/semantic_tests.rs`:
/// ```text
/// import "model.but";            // Plain-импорт
/// import "engine.but" as Motor;  // GlobalSymbol-импорт с алиасом
/// ```
pub(crate) fn read_import_file(
    search_paths: &[String],
    path: &ImportPath,
) -> Result<(String, String), Diagnostic> {
    // Формируем список кандидатов: cross-product (search_paths × path)
    let files: Vec<String> = match path {
        // Строковый литерал: `import "file.but";`
        ImportPath::Filename(filename) => search_paths
            .iter()
            .map(|dir| format!("{}/{}", dir, filename.string))
            .collect(),

        // Идентификаторный путь: `import a::b::c;` → ищем `a/b/c.but`
        ImportPath::Path(id_path) => {
            let inner = id_path
                .identifiers
                .iter()
                .map(|i| i.name.clone())
                .join("/");
            search_paths
                .iter()
                .map(|dir| format!("{}/{}.but", dir, inner))
                .collect()
        }
    };

    // Оставляем только существующие файлы
    let found: Vec<String> = files
        .iter()
        .filter(|f| exists(f).is_ok_and(|r| r))
        .cloned()
        .collect();

    if found.is_empty() {
        return Err(format!(
            "Файл импорта не найден: {:?}. Пути поиска: {:?}",
            path, search_paths
        )
        .as_str()
        .into());
    }

    let filename = found.first().unwrap();

    // Проверяем, что файл имеет расширение .but
    if !filename.ends_with(".but") {
        return Err(
            format!("Недопустимое расширение файла импорта: «{}»", filename)
                .as_str()
                .into(),
        );
    }

    let content = read_to_string(filename).map_err(|e| {
        format!("Ошибка чтения файла импорта «{}»: {}", filename, e)
            .as_str()
            .into()
    })?;

    Ok((content, filename.clone()))
}

// ─── Тесты ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{IdentifierPath, ImportPath, StringLiteral};
    use crate::diagnostics::Location;
    use std::fs;

    // ─── Вспомогательные функции ──────────────────────────────────────────────

    /// Создаёт временный `.but`-файл с заданным содержимым.
    /// Возвращает (директория, полный_путь_к_файлу).
    fn make_tmp_but(name: &str, content: &str) -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        fs::write(&path, content).unwrap();
        (dir, path.to_string_lossy().into_owned())
    }

    /// Строит `ImportPath::Filename` из строки.
    fn filename_path(s: &str) -> ImportPath {
        ImportPath::Filename(StringLiteral {
            string: s.to_string(),
            unicode: false,
            loc: Location::default(),
        })
    }

    // ─── Позитивные тесты ─────────────────────────────────────────────────────

    /// Файл найден по строковому литералу: содержимое и путь возвращаются корректно.
    #[test]
    fn filename_found_returns_content_and_path() {
        let (dir, _) = make_tmp_but("model.but", "start S;");
        let search = vec![dir.path().to_string_lossy().into_owned()];
        let path = filename_path("model.but");

        let (content, fullpath) = read_import_file(&search, &path).unwrap();
        assert_eq!(content, "start S;");
        assert!(fullpath.ends_with("model.but"));
    }

    /// Первый файл в нескольких путях поиска возвращается (порядок поиска).
    #[test]
    fn first_search_path_wins() {
        let dir1 = tempfile::tempdir().unwrap();
        let dir2 = tempfile::tempdir().unwrap();
        fs::write(dir1.path().join("x.but"), "start A;").unwrap();
        fs::write(dir2.path().join("x.but"), "start B;").unwrap();

        let search = vec![
            dir1.path().to_string_lossy().into_owned(),
            dir2.path().to_string_lossy().into_owned(),
        ];
        let (content, _) = read_import_file(&search, &filename_path("x.but")).unwrap();
        assert_eq!(content, "start A;", "должна вернуться версия из первого пути");
    }

    /// Идентификаторный путь `a::b` ищет файл `a/b.but`.
    #[test]
    fn identifier_path_appends_but_extension() {
        use crate::parser::ast::Identifier;

        let dir = tempfile::tempdir().unwrap();
        let subdir = dir.path().join("a");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("b.but"), "start S;").unwrap();

        let search = vec![dir.path().to_string_lossy().into_owned()];
        let path = ImportPath::Path(IdentifierPath {
            identifiers: vec![
                Identifier { name: "a".to_string(), loc: Location::default() },
                Identifier { name: "b".to_string(), loc: Location::default() },
            ],
            loc: Location::default(),
        });

        let (content, fullpath) = read_import_file(&search, &path).unwrap();
        assert_eq!(content, "start S;");
        assert!(
            fullpath.ends_with("a/b.but") || fullpath.ends_with("a\\b.but"),
            "путь должен заканчиваться на a/b.but"
        );
    }

    // ─── Негативные тесты (контр-примеры) ────────────────────────────────────

    /// Файл не найден → ошибка с упоминанием путей поиска.
    #[test]
    fn missing_file_returns_error() {
        let err = read_import_file(
            &["/nonexistent_dir_xyz".to_string()],
            &filename_path("ghost.but"),
        )
        .unwrap_err();
        assert!(
            err.message.contains("не найден"),
            "сообщение ошибки должно содержать «не найден»: {}",
            err.message
        );
    }

    /// Пустой список путей поиска → ошибка.
    #[test]
    fn empty_search_paths_returns_error() {
        let err = read_import_file(&[], &filename_path("any.but")).unwrap_err();
        assert!(err.message.contains("не найден"));
    }

    /// Файл существует, но не имеет расширения `.but` → ошибка.
    #[test]
    fn non_but_extension_is_error() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("model.txt"), "start S;").unwrap();
        let search = vec![dir.path().to_string_lossy().into_owned()];

        let err = read_import_file(&search, &filename_path("model.txt")).unwrap_err();
        assert!(
            err.message.contains("Недопустимое"),
            "ошибка должна сообщать о недопустимом расширении: {}",
            err.message
        );
    }

    /// Файл существует, но не имеет расширения вовсе → ошибка.
    #[test]
    fn file_without_extension_is_error() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("model"), "start S;").unwrap();
        let search = vec![dir.path().to_string_lossy().into_owned()];

        let err = read_import_file(&search, &filename_path("model")).unwrap_err();
        assert!(err.message.contains("Недопустимое"));
    }

    /// Ошибка чтения файла (нет прав) возвращает осмысленный диагностик.
    /// Этот тест запускается только на Unix, где можно снять права на чтение.
    #[cfg(unix)]
    #[test]
    fn unreadable_file_returns_io_error() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("secret.but");
        fs::write(&p, "start S;").unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o000)).unwrap();

        let search = vec![dir.path().to_string_lossy().into_owned()];
        let err = read_import_file(&search, &filename_path("secret.but")).unwrap_err();
        // Восстанавливаем права, чтобы tempdir мог удалить файл
        fs::set_permissions(&p, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(
            !err.message.is_empty(),
            "сообщение об ошибке не должно быть пустым"
        );
    }
}
