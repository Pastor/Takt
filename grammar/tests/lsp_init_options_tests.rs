//! Интеграционные тесты фичи 0072: пути поиска импортов LSP из
//! `initializationOptions.searchPaths` (аналог `-I` у `lamc`).
//!
//! Разбор самого `initializationOptions` покрыт юнит-тестами
//! `grammar::lsp::init_options` (в lib); здесь — сквозная проверка потребителей
//! ядра (`collect_diagnostics_at`, `goto_declaration_at`), которые зовёт
//! бинарник `lam_lsp.rs`. Импорт из общей библиотеки **вне** каталога документа
//! (`lsp72/lib/`) без путей не находится, с путями — находится (паритет с CLI).
//!
//! Вынесено в отдельный файл (не в `lsp_tests.rs`): тот сверх лимита размера
//! модуля (реестр долга) и расти не имеет права (CLAUDE.md, фича 0027).

#[cfg(feature = "lsp")]
mod lsp72_init_options {
    use lsp_types::Position;

    const DOC: &str = "tests/data/lsp72/proj/main.lam";
    const LIB: &str = "tests/data/lsp72/lib";

    fn doc_source() -> String {
        std::fs::read_to_string(DOC).unwrap_or_else(|e| panic!("{DOC}: {e}"))
    }

    /// Позиция курсора на `Shared` в `start Main = Shared;`.
    fn cursor_on_shared(source: &str) -> Position {
        let offset = source
            .find("= Shared")
            .expect("нет использования `= Shared`")
            + 2;
        let head = &source[..offset];
        let line = head.matches('\n').count() as u32;
        let col = head.rsplit('\n').next().map_or(0, |l| l.chars().count()) as u32;
        Position::new(line, col)
    }

    /// A4: без путей импорт из соседнего каталога не разрешается (только каталог
    /// документа, 0055) — как было до 0072.
    #[test]
    fn import_unresolved_without_search_paths() {
        let diags = grammar::lsp::collect_diagnostics_at(DOC, &doc_source(), &[]);
        assert!(
            diags.iter().any(|d| d.message.contains("не найден")),
            "без searchPaths импорт вне каталога документа обязан быть не найден: {diags:?}"
        );
    }

    /// A4: с путями (`searchPaths=[lib]`) импорт разрешается — диагностик нет.
    #[test]
    fn import_resolves_with_search_paths() {
        let paths = vec![LIB.to_string()];
        let diags = grammar::lsp::collect_diagnostics_at(DOC, &doc_source(), &paths);
        assert!(
            diags.is_empty(),
            "с searchPaths импорт обязан разрешаться без диагностик: {diags:?}"
        );
    }

    /// A5: переход к декларации ведёт в файл из `searchPaths`, а не «никуда».
    #[test]
    fn goto_opens_file_from_search_paths() {
        let source = doc_source();
        let pos = cursor_on_shared(&source);
        let paths = vec![LIB.to_string()];
        let loc = grammar::lsp::goto_declaration_at(DOC, &source, pos, &paths)
            .expect("переход на имени импортированной модели обязан находиться");
        assert!(
            loc.uri.ends_with("lsp72/lib/shared.lam"),
            "обязан открыться shared.lam из searchPaths, получено: {}",
            loc.uri
        );
    }

    /// Сторож паритета: без путей переход не находится (нечего открывать) —
    /// доказывает, что находку A5 даёт именно `searchPaths`, а не угадывание.
    #[test]
    fn goto_absent_without_search_paths() {
        let source = doc_source();
        let pos = cursor_on_shared(&source);
        assert!(
            grammar::lsp::goto_declaration_at(DOC, &source, pos, &[]).is_none(),
            "без searchPaths целевой файл недоступен — переходить некуда"
        );
    }
}
