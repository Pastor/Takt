//! `textDocument/references` — поиск всех использований символа (фича 0131).
//!
//! Работает на слое использований (`semantic::usages`), а не на
//! `SemanticIndex`: индекс не видит вхождений в телах `enter`/`always` и в телах
//! функций — позиции теряются при семантическом понижении.
//!
//! Часть модуля `lsp` (фича 0027: деление по логике).

use super::*;
use crate::semantic::usages::{self, UsageKind};

/// Все вхождения символа, стоящего под курсором, — в **открытом документе**.
///
/// `include_declaration` — включать ли само объявление (клиент передаёт это в
/// `context.includeDeclaration`).
///
/// ## Границы
///
/// Ищутся вхождения только в тексте документа: рабочая область сервером не
/// индексируется. Для символа, объявленного в импортированном файле, ответ
/// покажет его вхождения **здесь** — это правда, просто не вся: другие
/// файлы-потребители серверу не видны.
///
/// Возвращает `None`, если текст не разбирается или под курсором нет имени.
pub fn references_at(
    source: &str,
    position: Position,
    include_declaration: bool,
) -> Option<Vec<Range>> {
    let (ast, _) = crate::parse(source, 0).ok()?;
    let table = usages::collect_usages(&ast);
    let offset = position_to_offset(source, position)?;
    let symbol = table.usage_at(offset)?.symbol;

    let ranges = table
        .occurrences_of(symbol)
        .into_iter()
        .filter(|u| include_declaration || u.kind != UsageKind::Declaration)
        .map(|u| offset_to_range(source, u.start as usize, u.end as usize))
        .collect();
    Some(ranges)
}
