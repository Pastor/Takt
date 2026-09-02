//! Узлы оператора `match` — ветка и образец.
//!
//! Вынесены из `semantic/mod.rs` по границе ответственности (правило размера
//! модуля): там объявлены дерево и его корень, здесь — две самостоятельные
//! структуры, которыми пользуются понижение, обходы и все восемь печатников.

use crate::semantic::{ExpressionNode, StatementNode};

/// Семантическая ветка оператора `match`.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct MatchArmNode {
    /// Образцы (хотя бы один).
    pub patterns: Vec<MatchPatternNode>,
    /// Тело ветки.
    pub body: Box<StatementNode>,
}

/// Семантический образец ветки `match`.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum MatchPatternNode {
    /// Конкретное значение.
    Value(Box<ExpressionNode>),
    /// Подстановочный образец `_`.
    #[default]
    Wildcard,
}
