//! Диагностика: сбор ошибок компилятора и перевод их в форму LSP.
//!
//! Часть модуля `lsp` (фича 0027: деление по логике).

use super::*;

pub fn collect_diagnostics(source: &str) -> Vec<Diagnostic> {
    collect_diagnostics_at("", source, &[])
}

/// Диагностики документа, **зная его путь** (фича 0055).
///
/// Путь нужен по двум причинам:
///
/// 1. **Импорты разрешаются.** Прежде звался `construct_model(&ast, None, &[])`
///    — с пустыми путями поиска, поэтому `import "lib.lam";` в редакторе **всегда**
///    давал `SE-013` «файл не найден», даже когда файл лежит рядом. Каталог
///    документа — неявный путь поиска (общий для ядра и `lamc`).
/// 2. **Ошибку чужого файла есть к чему привязать.** Её координаты указывают в
///    текст, которого в открытом документе нет.
///
/// `search_paths` — дополнительные каталоги (как `-I` у `lamc`).
pub fn collect_diagnostics_at(
    path: &str,
    source: &str,
    search_paths: &[String],
) -> Vec<Diagnostic> {
    let mut lsp_diags = Vec::new();
    let mut files = crate::diagnostics::FileTable::new(path);

    // Шаг 1: Синтаксический анализ
    let (ast, _) = match crate::parse(source, 0) {
        Ok(result) => result,
        Err(errors) => {
            // Конвертируем ошибки парсера в LSP-диагностики
            for err in errors {
                lsp_diags.push(diagnostic_to_lsp(&err, source, &files));
            }
            return lsp_diags;
        }
    };

    // Шаг 2: Семантический анализ
    let model =
        match semantic::tree::construct_model_with_files(&ast, None, search_paths, &mut files) {
            Ok(m) => m,
            Err(err) => {
                lsp_diags.push(diagnostic_to_lsp(&err, source, &files));
                return lsp_diags;
            }
        };

    // Шаг 3: Дополнительные предупреждения
    let unused = crate::unused_variable_warnings(model.clone());
    for w in unused {
        lsp_diags.push(diagnostic_to_lsp(&w, source, &files));
    }

    let nondeterministic = crate::nondeterministic_transition_warnings(model.clone());
    for w in nondeterministic {
        lsp_diags.push(diagnostic_to_lsp(&w, source, &files));
    }

    let enum_errors = crate::enum_type_safety_errors(model);
    for e in enum_errors {
        lsp_diags.push(diagnostic_to_lsp(&e, source, &files));
    }

    lsp_diags
}

/// Конвертирует диагностику в LSP-диагностику, **различая свой файл и чужой**
/// (фича 0055).
///
/// Диагностика **своего** файла (`file_no == 0`) показывается на своём месте.
///
/// Диагностика **импортированного** файла своих координат в открытом документе
/// не имеет: смещения указывают в текст, которого здесь нет. Прежде `file_no`
/// молча отбрасывался, и подсветка ложилась по чужому смещению — то есть
/// **не туда**. Теперь такая ошибка:
///
/// - привязывается к строке `import`, через которую пришла (заметка
///   «импортирован в», её ставит проход 0 — см. `tree.rs::note_imported_here`);
/// - в тексте называет настоящее место: `в файле lib.lam:4:18: …`.
///
/// Так автор видит и **где** причина, и **что** в его файле к ней ведёт.
fn diagnostic_to_lsp(
    diag: &crate::diagnostics::Diagnostic,
    source: &str,
    files: &crate::diagnostics::FileTable,
) -> Diagnostic {
    use crate::diagnostics::Location;

    let own = matches!(diag.loc, Location::Source(0, _, _));
    if own {
        return grammar_diagnostic_to_lsp(diag, source);
    }

    // Путь чужого файла — из реестра; он же даёт `в файле X:строка:колонка`.
    let stamped = diag.clone().with_file_if_unset(files.path_of(&diag.loc));
    let where_ = crate::diagnostics::position_prefix(&stamped);

    // Якорь — заметка, чья позиция лежит В ЭТОМ документе (file_no == 0).
    // У цепочки `top → mid → deep` таких заметок ровно одна: `import` в top.
    let anchor = diag
        .notes
        .iter()
        .find_map(|n| match n.loc {
            Location::Source(0, start, end) => Some(offset_to_range(source, start, end)),
            _ => None,
        })
        .unwrap_or(Range {
            start: Position::new(0, 0),
            end: Position::new(0, 0),
        });

    let mut lsp = grammar_diagnostic_to_lsp(&stamped, source);
    lsp.range = anchor;
    lsp.message = format!("в файле {}{}", where_, diag.message);
    lsp
}

/// Конвертирует [`crate::diagnostics::Diagnostic`] в LSP [`Diagnostic`].
///
/// Диагностики с `Location::Source` получают точный диапазон в документе.
/// Диагностики без координат (`Location::Implicit`, `Location::Codegen` и т.д.)
/// получают нулевой диапазон `(0,0)-(0,0)`.
///
/// Вспомогательные заметки (`notes`) добавляются к тексту сообщения через
/// символ переноса строки, чтобы редактор мог отобразить их вместе с основным
/// сообщением без необходимости знать URI документа на этапе конвертации.
pub fn grammar_diagnostic_to_lsp(
    diag: &crate::diagnostics::Diagnostic,
    source: &str,
) -> Diagnostic {
    use crate::diagnostics::Level;

    let severity = match diag.level {
        Level::Error => Some(DiagnosticSeverity::ERROR),
        Level::Warning => Some(DiagnosticSeverity::WARNING),
        Level::Info => Some(DiagnosticSeverity::INFORMATION),
        Level::Debug => Some(DiagnosticSeverity::HINT),
    };

    // Конвертируем байтовое смещение в позицию строка:столбец
    let range = match diag.loc {
        crate::diagnostics::Location::Source(_, start, end) => offset_to_range(source, start, end),
        _ => Range {
            start: Position::new(0, 0),
            end: Position::new(0, 0),
        },
    };

    // Формируем полное сообщение: основной текст + заметки
    let message = if diag.notes.is_empty() {
        diag.message.clone()
    } else {
        let notes_text: String = diag
            .notes
            .iter()
            .map(|n| format!("\nЗаметка: {}", n.message))
            .collect();
        format!("{}{}", diag.message, notes_text)
    };

    Diagnostic {
        range,
        severity,
        message,
        source: Some("lam-lsp".to_string()),
        ..Default::default()
    }
}
