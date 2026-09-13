//! Словари языка для LSP: ключевые слова, встроенные типы, виды токенов.
//!
//! Часть модуля `lsp`.

use super::*;
use crate::diagnostics::lang::{Key, keys};

/// Типы семантических токенов (порядок важен - индекс используется как тип в легенде).
pub const SEMANTIC_TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,     // 0
    SemanticTokenType::VARIABLE,    // 1
    SemanticTokenType::FUNCTION,    // 2
    SemanticTokenType::TYPE,        // 3
    SemanticTokenType::ENUM_MEMBER, // 4
    SemanticTokenType::STRING,      // 5
    SemanticTokenType::NUMBER,      // 6
    SemanticTokenType::COMMENT,     // 7
    SemanticTokenType::OPERATOR,    // 8
    SemanticTokenType::CLASS,       // 9 (состояния и модели)
];

pub(super) const TT_KEYWORD: u32 = 0;

pub(super) const TT_VARIABLE: u32 = 1;

pub(super) const TT_FUNCTION: u32 = 2;

pub(super) const TT_TYPE: u32 = 3;

pub(super) const TT_ENUM_MEMBER: u32 = 4;

pub(super) const TT_STRING: u32 = 5;

pub(super) const TT_NUMBER: u32 = 6;

pub(super) const TT_COMMENT: u32 = 7;

pub(super) const TT_OPERATOR: u32 = 8;

pub(super) const TT_CLASS: u32 = 9;

/// Ключевые слова языка Takt для автодополнения.
pub(super) const TAKT_KEYWORDS: &[(&str, Key)] = &[
    ("model", keys::KW_MODEL),
    ("state", keys::KW_STATE),
    ("start", keys::KW_START),
    ("ref", keys::KW_REF),
    ("next", keys::KW_NEXT),
    ("enter", keys::KW_ENTER),
    ("exit", keys::KW_EXIT),
    ("always", keys::KW_ALWAYS),
    ("var", keys::KW_VAR),
    ("const", keys::KW_CONST),
    ("parameter", keys::KW_PARAMETER),
    ("type", keys::KW_TYPE),
    ("fn", keys::KW_FN),
    ("extern", keys::KW_EXTERN),
    ("in", keys::KW_IN),
    ("out", keys::KW_OUT),
    ("inout", keys::KW_INOUT),
    ("address", keys::KW_ADDRESS),
    ("at", keys::KW_AT),
    ("enum", keys::KW_ENUM),
    ("struct", keys::KW_STRUCT),
    ("cond", keys::KW_COND),
    ("invariant", keys::KW_INVARIANT),
    ("if", keys::KW_IF),
    ("else", keys::KW_ELSE),
    ("loop", keys::KW_LOOP),
    ("while", keys::KW_WHILE),
    ("for", keys::KW_FOR),
    ("match", keys::KW_MATCH),
    ("break", keys::KW_BREAK),
    ("continue", keys::KW_CONTINUE),
    ("return", keys::KW_RETURN),
    ("import", keys::KW_IMPORT),
    ("as", keys::KW_AS),
    ("from", keys::KW_FROM),
    ("formula", keys::KW_FORMULA),
    ("assembly", keys::KW_ASSEMBLY),
    ("true", keys::KW_TRUE),
    ("false", keys::KW_FALSE),
    // Конструкции времени.
    ("clock", keys::KW_CLOCK),
    ("after", keys::KW_AFTER),
    ("every", keys::KW_EVERY),
    // Операторы LTL и типы формул (: до неё в списке отсутствовали, хотя ключевыми
    // словами языка являются с самого начала верификации).
    ("X", keys::KW_LTL_NEXT),
    ("F", keys::KW_LTL_FINALLY),
    ("G", keys::KW_LTL_GLOBALLY),
    ("U", keys::KW_LTL_UNTIL),
    ("R", keys::KW_LTL_RELEASE),
    ("LTL", keys::KW_LTL),
    ("Guard", keys::KW_GUARD),
];

/// Ключевые слова языка, **намеренно** не предлагаемые автодополнением.
///
/// (`../../../docs/features/0178-editor-layer-language-sync.md#архитектура-adr`):
/// каждое ключевое слово лексера либо в [`TAKT_KEYWORDS`], либо здесь - с обоснованием.
/// Молча пропустить слово нельзя: тест `test_completion_covers_lexer_keywords`
/// покрывает обе стороны.
///
/// Рост этого списка - сигнал ревью: сюда легко "спрятать" пропуск.
///
/// Список **контрольный, а не рабочий**: путь автодополнения его не читает - исключённое
/// слово просто отсутствует в [`TAKT_KEYWORDS`]. Поэтому он под `cfg(test)`: в рабочей
/// сборке он был бы мёртвым кодом, а `-D warnings` мёртвый код не прощает.
#[cfg(test)]
const COMPLETION_EXCLUDED: &[(&str, &str)] = &[
    // `_` - подстановочный образец ветки `match`. По роли это знак препинания, а не
    // имя: дополнять нечего, и в списке имён он был бы шумом.
    (
        "_",
        "подстановочный образец match — знак препинания по роли, не имя",
    ),
];

/// Встроенные типы языка Takt: примитивные и целочисленные.
///
/// Используются для подсветки идентификаторов-типов (`TT_TYPE`) в semantic tokens, для
/// генерации элементов автодополнения с видом `TYPE` и для hover-подсказок.
pub(super) const TAKT_BUILTIN_TYPES: &[(&str, Key)] = &[
    ("bit", keys::TYPE_BIT),
    ("bool", keys::TYPE_BOOL),
    ("float", keys::TYPE_FLOAT),
    ("unit", keys::TYPE_UNIT),
    ("u8", keys::TYPE_U8),
    ("u16", keys::TYPE_U16),
    ("u32", keys::TYPE_U32),
    ("u64", keys::TYPE_U64),
    ("i8", keys::TYPE_I8),
    ("i16", keys::TYPE_I16),
    ("i32", keys::TYPE_I32),
    ("i64", keys::TYPE_I64),
    ("duration", keys::TYPE_DURATION),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::lexer::all_keywords;
    use std::collections::HashSet;

    /// Список автодополнения покрывает таблицу лексера.
    ///
    /// Каждое ключевое слово лексера обязано быть либо в [`TAKT_KEYWORDS`]
    /// (предлагается автодополнением), либо в [`COMPLETION_EXCLUDED`] (решено не
    /// предлагать, с обоснованием). Пропуск - испорченное автодополнение: список
    /// отстаёт от языка, и заметить это нечем.
    ///
    /// Проверяется **вложение**, а не равенство: в `TAKT_KEYWORDS` законно живут
    /// `enter`/`exit`/`always` - имена блоков, ключевыми словами лексера не являющиеся
    /// (лексер отдаёт их как `Identifier`). Тест на равенство краснел бы на верном
    /// списке.
    #[test]
    fn test_completion_covers_lexer_keywords() {
        let offered: HashSet<&str> = TAKT_KEYWORDS.iter().map(|(k, _)| *k).collect();
        let excluded: HashSet<&str> = COMPLETION_EXCLUDED.iter().map(|(k, _)| *k).collect();

        let uncovered: Vec<&str> = all_keywords()
            .filter(|k| !offered.contains(k) && !excluded.contains(k))
            .collect();

        assert!(
            uncovered.is_empty(),
            "ключевые слова языка не покрыты автодополнением и не внесены в \
             COMPLETION_EXCLUDED: {uncovered:?}\n\
             Добавьте их в TAKT_KEYWORDS с описанием либо в COMPLETION_EXCLUDED \
             с обоснованием (правило 2 ADR 0178)."
        );
    }

    /// Исключение обязано быть ключевым словом языка.
    ///
    /// Иначе список исключений копит мусор: слово, выведенное из языка, осталось бы
    /// "исключением" навсегда и маскировало бы своё исчезновение.
    #[test]
    fn test_exclusions_are_real_keywords() {
        let known: HashSet<&str> = all_keywords().collect();
        let stale: Vec<&str> = COMPLETION_EXCLUDED
            .iter()
            .map(|(k, _)| *k)
            .filter(|k| !known.contains(k))
            .collect();
        assert!(
            stale.is_empty(),
            "COMPLETION_EXCLUDED упоминает то, что ключевым словом языка не \
             является: {stale:?}"
        );
    }

    /// У каждой записи обоих списков есть непустое описание/обоснование.
    #[test]
    fn test_every_entry_is_documented() {
        for (word, key) in TAKT_KEYWORDS {
            assert!(!word.is_empty(), "пустое ключевое слово в списке");
            assert!(!crate::msg!(*key).is_empty(), "нет описания у `{word}`");
        }
        for (word, text) in COMPLETION_EXCLUDED {
            assert!(!word.is_empty(), "пустое ключевое слово в списке");
            assert!(!text.is_empty(), "нет обоснования у `{word}`");
        }
    }
}
