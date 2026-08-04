//! Диагностика: сбор ошибок компилятора и перевод их в форму LSP.
//!
//! Часть модуля `lsp` (фича 0027: деление по логике).

use super::*;

/// Диагностики документа **без известного пути** — тонкая обёртка над
/// [`collect_diagnostics_at`] с пустым путём и без путей поиска импортов.
///
/// Без пути неявный путь импорта (каталог документа, фича 0055) не работает:
/// разрешится лишь то, что даёт исходный текст. Для полноценного разрешения
/// импортов зовите [`collect_diagnostics_at`].
pub fn collect_diagnostics(source: &str) -> Vec<Diagnostic> {
    collect_diagnostics_at("", source, &[])
}

/// Диагностики документа, **зная его путь** (фича 0055).
///
/// Путь нужен по двум причинам:
///
/// 1. **Импорты разрешаются.** Прежде звался `construct_model(&ast, None, &[])`
///    — с пустыми путями поиска, поэтому `import "lib.takt";` в редакторе **всегда**
///    давал `SE-013` «файл не найден», даже когда файл лежит рядом. Каталог
///    документа — неявный путь поиска (общий для ядра и `taktc`).
/// 2. **Ошибку чужого файла есть к чему привязать.** Её координаты указывают в
///    текст, которого в открытом документе нет.
///
/// `search_paths` — дополнительные каталоги (как `-I` у `taktc`).
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

    // Стиль (фича 0226): та же проверка, что у `taktc fmt` — одна реализация на
    // обоих потребителей. Считается **сразу после разбора**, потому что ей нужен
    // только АСД, и отдаётся во ВСЕХ ветвях возврата ниже (фича 0227).
    //
    // ⚠️ Это осознанное исключение из политики «при ошибках предупреждений не
    // показываем»: прочие предупреждения смотрят на построенную модель и на
    // сломанной могут быть ложными, а канон именования от смысла не зависит —
    // имя либо в каноне, либо нет. Прежде на файле с ошибкой `taktc fmt`
    // предупреждение печатал, а редактор молчал, хотя правят имена как раз в
    // файлах, которые ещё не собираются.
    let style: Vec<Diagnostic> = crate::style::naming_warnings(&ast)
        .iter()
        .map(|w| diagnostic_to_lsp(w, source, &files))
        .collect();

    // Шаг 2: Семантический анализ. Стадии построения терминальны, проверки —
    // накапливаются (фича 0130): редактор подчёркивает **все** нарушения, а не
    // первое. Прежде здесь стоял `construct_model_with_files`, отдающий одну
    // диагностику по контракту.
    // LSP работает в режиме по умолчанию (`assign`): флага генерации у
    // редактора нет, а диагностики режимов не расходятся.
    let model =
        match semantic::stages::construct_stages(&ast, None, search_paths, &mut files, false) {
            Ok(m) => m,
            Err(errs) => {
                // Фича 0152: стадии 4–6 накапливают, поэтому редактор
                // подчёркивает все ошибки тел сразу. `normalize` — порядок по
                // позиции и дедупликация: иначе наблюдаемым стал бы порядок
                // обхода `BTreeMap`.
                for err in crate::diagnostics::normalize(errs) {
                    lsp_diags.push(diagnostic_to_lsp(&err, source, &files));
                }
                lsp_diags.extend(style);
                return lsp_diags;
            }
        };

    let errors = semantic::validate::validate_model_all(model.clone());
    if !errors.is_empty() {
        // Предупреждения о СМЫСЛЕ при наличии ошибок не показываем: сперва
        // ошибки. Предупреждение о стиле — исключение (см. выше): оно смотрит на
        // текст, а не на модель, и от ошибок не портится.
        for err in crate::diagnostics::normalize(errors) {
            lsp_diags.push(diagnostic_to_lsp(&err, source, &files));
        }
        lsp_diags.extend(style);
        return lsp_diags;
    }

    // Шаг 3: Дополнительные предупреждения
    lsp_diags.extend(style);

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
/// - в тексте называет настоящее место: `в файле lib.takt:4:18: …`.
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
            Location::Source(0, start, end) => {
                Some(offset_to_range(source, start as usize, end as usize))
            }
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
        crate::diagnostics::Location::Source(_, start, end) => {
            offset_to_range(source, start as usize, end as usize)
        }
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
        // Код диагностики (фича 0130). Прежде он терялся: `taktc` печатал
        // `[SE-003]`, а редактор показывал тот же текст **без** кода — и найти
        // ошибку в справочнике кодов было не по чему. Поле в протоколе для этого
        // и предназначено.
        code: diag
            .code
            .as_ref()
            .map(|c| NumberOrString::String(c.clone())),
        message,
        source: Some("takt-lsp".to_string()),
        ..Default::default()
    }
}
