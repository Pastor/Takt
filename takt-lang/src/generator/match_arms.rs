//! Пустая ветвь `match` — общий признак целей (0509).
//!
//! Автор вправе написать ветвь без операторов: `_ => {}` («прочие значения
//! ничего не делают») — запись из практики, она стоит в примерах документа.
//! Эталон и цели `c`/`sv` её принимают, а инструменты двух других — нет:
//!
//! | Цель | Ответ инструмента до 0509 |
//! |---|---|
//! | `rust` | `clippy`: «this `else` branch is empty» — отказ под `-D warnings` |
//! | `st`, `st-at` | `iec2c`: «no statement defined after 'ELSE'/'THEN'» |
//!
//! Код возврата `taktc` — **нулевой**. Пустого оператора в IEC 61131-3 нет
//! вовсе (`;` MatIEC отвергает, урок 0473), поэтому у цели `st` пустая ветвь
//! **опускается**, а не наполняется заглушкой.
//!
//! ⚠️ Опустить ветвь можно, только если её образцы **не повторяются ниже**:
//! `match` берёт ПЕРВОЕ совпадение, и при дубле пустая ветвь его поглощает.
//! Такой вход и без того невалиден у цели `c` (`cc`: «duplicate case value»),
//! но менять на нём автомат молча нельзя — признак ниже это стережёт.

use crate::semantic::{MatchArmNode, MatchPatternNode};

/// Повторяется ли хотя бы один образец ветви `index` в последующих ветвях.
///
/// Сравниваются УЗЛЫ образцов: форма записи (`0x1` против `1`) до генерации не
/// доживает — значение свёрнуто семантикой (0192).
pub(crate) fn pattern_repeats_below(arms: &[MatchArmNode], index: usize) -> bool {
    let Some(arm) = arms.get(index) else {
        return false;
    };
    arms.iter().skip(index + 1).any(|later| {
        later.patterns.iter().any(|lp| {
            arm.patterns
                .iter()
                .any(|p| matches!((p, lp), (MatchPatternNode::Value(a), MatchPatternNode::Value(b)) if a == b))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::pattern_repeats_below;
    use crate::semantic::{ExpressionNode, MatchArmNode, MatchPatternNode, StatementNode};

    fn arm(value: i128) -> MatchArmNode {
        MatchArmNode {
            patterns: vec![MatchPatternNode::Value(Box::new(ExpressionNode::Number(
                value,
            )))],
            body: Box::new(StatementNode::Block(Vec::new())),
        }
    }

    #[test]
    fn distinct_patterns_do_not_repeat() {
        let arms = vec![arm(1), arm(2)];
        assert!(!pattern_repeats_below(&arms, 0));
    }

    /// Дубль ниже — ветвь опускать нельзя: `match` берёт первое совпадение.
    #[test]
    fn duplicate_below_is_seen() {
        let arms = vec![arm(1), arm(1)];
        assert!(pattern_repeats_below(&arms, 0));
    }

    /// Дубль ВЫШЕ не мешает: та ветвь и так поглотит значение.
    #[test]
    fn duplicate_above_is_not_seen() {
        let arms = vec![arm(1), arm(1)];
        assert!(!pattern_repeats_below(&arms, 1));
    }
}
