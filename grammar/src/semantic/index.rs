//! Индекс семантических узлов для поиска по позиции в исходном тексте.
//!
//! Модуль реализует [`SemanticIndex`] — структуру данных, позволяющую LSP-сервису
//! за O(n) найти наиболее конкретный узел семантического дерева, покрывающий
//! заданное байтовое смещение в исходном тексте.
//!
//! ## Принцип работы
//!
//! 1. [`SemanticIndex::build`] обходит семантическое дерево [`ModelNode`] и
//!    собирает записи вида `(start_byte, end_byte, SemanticNodeRef)` из позиций
//!    всех объявлений: переменных, функций, состояний, типов, условий, перечислений
//!    и вложенных моделей.
//! 2. Записи сортируются по `start_byte`.
//! 3. [`SemanticIndex::node_at_offset`] перебирает записи и возвращает запись с
//!    наименьшим диапазоном, покрывающим заданное смещение («наиболее конкретный»
//!    или «внутренний» узел).
//!
//! ## Пример
//!
//! ```
//! use grammar::parse;
//! use grammar::semantic::tree::construct_model;
//! use grammar::semantic::index::SemanticIndex;
//!
//! let src = "var x: bit = false;";
//! let (ast, _) = parse(src, 0).unwrap();
//! let model = construct_model(&ast, None, &[]).unwrap();
//! let index = SemanticIndex::build(&model);
//!
//! // Смещение 4 — внутри имени переменной "x"
//! let node = index.node_at_offset(4);
//! assert!(node.is_some());
//! assert_eq!(node.unwrap().name, "x");
//! ```

use crate::diagnostics::Location;
use crate::semantic::{FunctionNode, ModelNode, StateNodeKind, VariableNode};
use std::cell::RefCell;
use std::rc::Rc;

/// Вид семантического узла.
///
/// Используется в [`SemanticNodeRef`] для указания категории найденного элемента,
/// что позволяет LSP-сервису формировать корректный ответ (hover, go-to-definition
/// и др.) без дополнительного поиска по имени.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticNodeKind {
    /// Обычная изменяемая переменная (`var`).
    Variable,
    /// Константа (`const`).
    Const,
    /// Порт ввода-вывода (`port`).
    Port,
    /// Локальная функция (`fn`).
    Function,
    /// Внешняя функция (`extern fn`).
    ExternFunction,
    /// Обычное состояние автомата (`state`).
    State,
    /// Начальное состояние автомата (`start`).
    StartState,
    /// Конечное состояние автомата (`end`).
    EndState,
    /// Псевдоним типа (`type`).
    TypeAlias,
    /// Именованное условие перехода (`cond`).
    Condition,
    /// Перечисление (`enum`).
    Enum,
    /// Именованная модель конечного автомата (`model`).
    Model,
}

/// Ссылка на узел семантического дерева с позицией в исходном тексте.
///
/// Содержит имя элемента, его вид и позицию объявления (`loc`).
/// Не хранит сам узел семантического дерева — для получения полных данных
/// узла используйте методы поиска [`ModelNode`]:
/// [`search_var`](ModelNode::search_var), [`search_func`](ModelNode::search_func),
/// [`search_state`](ModelNode::search_state) и т.д.
#[derive(Debug, Clone)]
pub struct SemanticNodeRef {
    /// Имя элемента (переменной, функции, состояния и т.д.).
    pub name: String,
    /// Вид узла.
    pub kind: SemanticNodeKind,
    /// Позиция объявления в исходном тексте.
    pub loc: Location,
    /// Модель, в которой объявлен элемент (для поиска в правильном контексте области видимости).
    pub model: Option<Rc<RefCell<ModelNode>>>,
}

/// Внутренняя запись индекса.
#[derive(Debug)]
struct IndexEntry {
    /// Начало диапазона (байтовое смещение, включительно).
    start: usize,
    /// Конец диапазона (байтовое смещение, включительно).
    end: usize,
    /// Ссылка на семантический узел.
    node_ref: SemanticNodeRef,
}

/// Индекс семантических узлов, упорядоченный по байтовому смещению.
///
/// Строится из корневого [`ModelNode`] методом [`build`](SemanticIndex::build).
/// Позволяет за O(n) найти наиболее конкретный узел, покрывающий заданное
/// байтовое смещение в исходном тексте.
pub struct SemanticIndex {
    /// Записи, отсортированные по полю `start` для предсказуемого обхода.
    entries: Vec<IndexEntry>,
}

impl std::fmt::Debug for SemanticIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SemanticIndex")
            .field("entries_count", &self.entries.len())
            .finish()
    }
}

impl SemanticIndex {
    /// Строит индекс из корневого узла семантической модели.
    ///
    /// Рекурсивно обходит все вложенные модели, собирая позиции объявлений
    /// переменных, функций, состояний, псевдонимов типов, условий, перечислений
    /// и именованных моделей.
    ///
    /// Элементы с [`Location::Builtin`], [`Location::Implicit`] и
    /// [`Location::Codegen`] в индекс не включаются.
    ///
    /// # Пример
    ///
    /// ```
    /// use grammar::parse;
    /// use grammar::semantic::tree::construct_model;
    /// use grammar::semantic::index::SemanticIndex;
    ///
    /// let src = "var x: bit = false; start S;";
    /// let (ast, _) = parse(src, 0).unwrap();
    /// let model = construct_model(&ast, None, &[]).unwrap();
    /// let index = SemanticIndex::build(&model);
    /// assert!(index.len() >= 2); // минимум: переменная + состояние
    /// ```
    pub fn build(model: &Rc<RefCell<ModelNode>>) -> Self {
        let mut entries = Vec::new();
        collect_model_entries(model, &mut entries);
        // Сортируем по началу диапазона — позволяет прерывать перебор при start > offset
        entries.sort_by_key(|e| e.start);
        SemanticIndex { entries }
    }

    /// Возвращает наиболее конкретный семантический узел, покрывающий смещение `offset`.
    ///
    /// Среди всех записей, чей диапазон `[start, end]` содержит `offset`,
    /// выбирается та, у которой наименьший размер диапазона — т.е. наиболее
    /// специфичный (внутренний) узел.
    ///
    /// Возвращает `None`, если ни один узел не покрывает `offset`.
    ///
    /// # Пример
    ///
    /// ```
    /// use grammar::parse;
    /// use grammar::semantic::tree::construct_model;
    /// use grammar::semantic::index::{SemanticIndex, SemanticNodeKind};
    ///
    /// let src = "var counter: bit = false;";
    /// let (ast, _) = parse(src, 0).unwrap();
    /// let model = construct_model(&ast, None, &[]).unwrap();
    /// let index = SemanticIndex::build(&model);
    ///
    /// // Смещение 4 — на символе 'c' в "counter"
    /// let node = index.node_at_offset(4);
    /// assert!(node.is_some());
    /// let node = node.unwrap();
    /// assert_eq!(node.name, "counter");
    /// assert_eq!(node.kind, SemanticNodeKind::Variable);
    ///
    /// // Смещение за пределами всех объявлений
    /// let node_none = index.node_at_offset(99999);
    /// assert!(node_none.is_none());
    /// ```
    pub fn node_at_offset(&self, offset: usize) -> Option<&SemanticNodeRef> {
        let mut best: Option<&IndexEntry> = None;
        let mut best_size = usize::MAX;

        for entry in &self.entries {
            // Записи отсортированы по start: как только start > offset — дальше нет смысла
            if entry.start > offset {
                break;
            }
            // Проверяем, что offset попадает в диапазон [start, end] (включительно)
            if entry.end >= offset {
                let size = entry.end.saturating_sub(entry.start);
                if size < best_size {
                    best_size = size;
                    best = Some(entry);
                }
            }
        }

        best.map(|e| &e.node_ref)
    }

    /// Возвращает количество записей в индексе.
    ///
    /// Используется преимущественно в тестах для проверки корректности построения.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Возвращает `true`, если индекс не содержит ни одной записи.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Рекурсивно собирает записи из модели и всех вложенных моделей.
fn collect_model_entries(model: &Rc<RefCell<ModelNode>>, entries: &mut Vec<IndexEntry>) {
    let borrowed = model.borrow();

    // Сама модель (только именованные; корневая анонимная модель пропускается)
    if let Some(ref name) = borrowed.name {
        if let Location::Source(_, start, end) = borrowed.loc {
            entries.push(IndexEntry {
                start,
                end,
                node_ref: SemanticNodeRef {
                    name: name.clone(),
                    kind: SemanticNodeKind::Model,
                    loc: borrowed.loc,
                    model: Some(model.clone()),
                },
            });
        }
    }

    // Переменные (var, port, const)
    for (name, var) in &borrowed.variables {
        let (loc, kind) = match var {
            VariableNode::Simple { loc, .. } => (*loc, SemanticNodeKind::Variable),
            VariableNode::Port { loc, .. } => (*loc, SemanticNodeKind::Port),
            VariableNode::Const { loc, .. } => (*loc, SemanticNodeKind::Const),
            VariableNode::Unresolved => continue,
        };
        if let Location::Source(_, start, end) = loc {
            entries.push(IndexEntry {
                start,
                end,
                node_ref: SemanticNodeRef {
                    name: name.clone(),
                    kind,
                    loc,
                    model: Some(model.clone()),
                },
            });
        }
    }

    // Функции (fn, extern fn; встроенные не индексируются — нет позиции в коде)
    for (name, func) in &borrowed.functions {
        let (loc, kind) = match func {
            FunctionNode::Local { loc, .. } => (*loc, SemanticNodeKind::Function),
            FunctionNode::External { loc, .. } => (*loc, SemanticNodeKind::ExternFunction),
            _ => continue,
        };
        if let Location::Source(_, start, end) = loc {
            entries.push(IndexEntry {
                start,
                end,
                node_ref: SemanticNodeRef {
                    name: name.clone(),
                    kind,
                    loc,
                    model: Some(model.clone()),
                },
            });
        }
    }

    // Состояния (state / start / end)
    for (name, state) in &borrowed.states {
        let loc = state.loc();
        let kind = match state {
            crate::semantic::StateNode::Simple { kind, .. }
            | crate::semantic::StateNode::Implement { kind, .. } => match kind {
                StateNodeKind::Start => SemanticNodeKind::StartState,
                StateNodeKind::End => SemanticNodeKind::EndState,
                StateNodeKind::Simple => SemanticNodeKind::State,
            },
            crate::semantic::StateNode::Unresolved => continue,
        };
        if let Location::Source(_, start, end) = loc {
            entries.push(IndexEntry {
                start,
                end,
                node_ref: SemanticNodeRef {
                    name: name.clone(),
                    kind,
                    loc,
                    model: Some(model.clone()),
                },
            });
        }
    }

    // Псевдонимы типов: позиции хранятся в type_locs (отдельно от types).
    // Перечисления (enum) тоже добавляются в type_locs при построении модели,
    // поэтому пропускаем записи, для которых уже есть соответствующий EnumNode.
    for (name, loc) in &borrowed.type_locs {
        if borrowed.enums.contains_key(name.as_str()) {
            // Этот идентификатор — перечисление, оно будет добавлено в секции enums
            continue;
        }
        if let Location::Source(_, start, end) = loc {
            entries.push(IndexEntry {
                start: *start,
                end: *end,
                node_ref: SemanticNodeRef {
                    name: name.clone(),
                    kind: SemanticNodeKind::TypeAlias,
                    loc: *loc,
                    model: Some(model.clone()),
                },
            });
        }
    }

    // Именованные условия переходов (cond)
    for (name, cond) in &borrowed.conditions {
        if let Location::Source(_, start, end) = cond.loc {
            entries.push(IndexEntry {
                start,
                end,
                node_ref: SemanticNodeRef {
                    name: name.clone(),
                    kind: SemanticNodeKind::Condition,
                    loc: cond.loc,
                    model: Some(model.clone()),
                },
            });
        }
    }

    // Перечисления (enum)
    for (name, enum_node) in &borrowed.enums {
        if let Location::Source(_, start, end) = enum_node.loc {
            entries.push(IndexEntry {
                start,
                end,
                node_ref: SemanticNodeRef {
                    name: name.clone(),
                    kind: SemanticNodeKind::Enum,
                    loc: enum_node.loc,
                    model: Some(model.clone()),
                },
            });
        }
    }

    for nb in &borrowed.named_blocks {
        //TODO: Реализовать
    }

    // Рекурсивно обходим вложенные именованные модели
    for nested in borrowed.models.values() {
        collect_model_entries(nested, entries);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use crate::semantic::tree::construct_model;

    /// Вспомогательная функция: парсит источник и строит индекс.
    fn build_index(src: &str) -> SemanticIndex {
        let (ast, _) = parse(src, 0).expect("ошибка парсинга");
        let model = construct_model(&ast, None, &[]).expect("ошибка семантики");
        SemanticIndex::build(&model)
    }

    // ── Юнит-тесты SemanticIndex ──────────────────────────────────────────────

    /// Пустая программа → пустой индекс.
    #[test]
    fn empty_source_gives_empty_index() {
        let index = build_index("");
        assert!(index.is_empty());
    }

    /// Одна переменная индексируется корректно.
    #[test]
    fn single_variable_is_indexed() {
        let src = "var x: bit = false;";
        let index = build_index(src);
        // Индекс должен содержать хотя бы запись о переменной
        assert!(!index.is_empty());
    }

    /// Поиск по смещению внутри объявления переменной возвращает её имя.
    #[test]
    fn node_at_offset_finds_variable() {
        //           0123456789...
        let src = "var x: bit = false;";
        let index = build_index(src);
        // Смещение 4 — символ 'x'
        let node = index.node_at_offset(4);
        assert!(node.is_some(), "должен найти переменную");
        let node = node.unwrap();
        assert_eq!(node.name, "x");
        assert_eq!(node.kind, SemanticNodeKind::Variable);
    }

    /// Поиск за пределами всех объявлений возвращает None.
    #[test]
    fn node_at_offset_out_of_range_returns_none() {
        let src = "var x: bit = false;";
        let index = build_index(src);
        // Смещение 99999 — далеко за концом файла
        assert!(index.node_at_offset(99999).is_none());
    }

    /// Состояние индексируется и находится по смещению.
    #[test]
    fn state_is_indexed() {
        //           012345678901234
        let src = "start Init;";
        let index = build_index(src);
        let node = index.node_at_offset(6);
        assert!(node.is_some(), "должен найти состояние");
        let node = node.unwrap();
        assert_eq!(node.name, "Init");
        assert_eq!(node.kind, SemanticNodeKind::StartState);
    }

    /// Функция индексируется и находится по смещению.
    #[test]
    fn function_is_indexed() {
        // Параметры функции не доступны в выражениях тела — используем литерал
        let src = "fn add(a: bit) -> bit { return true; }";
        let index = build_index(src);
        // Смещение 3 — символ 'a' в "add"
        let node = index.node_at_offset(3);
        assert!(node.is_some(), "должен найти функцию");
        let node = node.unwrap();
        assert_eq!(node.name, "add");
        assert_eq!(node.kind, SemanticNodeKind::Function);
    }

    /// Псевдоним типа индексируется.
    #[test]
    fn type_alias_is_indexed() {
        let src = "type Byte = [bit;8];";
        let index = build_index(src);
        // Смещение 5 — символ 'B' в "Byte"
        let node = index.node_at_offset(5);
        assert!(node.is_some(), "должен найти псевдоним типа");
        let node = node.unwrap();
        assert_eq!(node.name, "Byte");
        assert_eq!(node.kind, SemanticNodeKind::TypeAlias);
    }

    /// Именованное условие индексируется.
    #[test]
    fn condition_is_indexed() {
        let src = "cond IsReady = true;";
        let index = build_index(src);
        // Смещение 5 — символ 'I' в "IsReady"
        let node = index.node_at_offset(5);
        assert!(node.is_some(), "должен найти условие");
        let node = node.unwrap();
        assert_eq!(node.name, "IsReady");
        assert_eq!(node.kind, SemanticNodeKind::Condition);
    }

    /// Перечисление индексируется.
    #[test]
    fn enum_is_indexed() {
        // Перечисление объявляется внутри модели; варианты разделяются запятыми
        let src = "model M { enum Color { Red, Green, Blue } start S; }";
        let index = build_index(src);
        // Смещение 16 — символ 'C' в "Color"
        let node = index.node_at_offset(16);
        assert!(node.is_some(), "должен найти перечисление");
        let node = node.unwrap();
        assert_eq!(node.name, "Color");
        assert_eq!(node.kind, SemanticNodeKind::Enum);
    }

    /// Именованная модель индексируется.
    #[test]
    fn model_is_indexed() {
        let src = "model Blinker { start On; state Off; }";
        let index = build_index(src);
        // Смещение 6 — символ 'B' в "Blinker"
        let node = index.node_at_offset(6);
        assert!(node.is_some(), "должен найти модель");
        let node = node.unwrap();
        assert_eq!(node.name, "Blinker");
        assert_eq!(node.kind, SemanticNodeKind::Model);
    }

    /// Несколько элементов: поиск возвращает наиболее конкретный.
    #[test]
    fn multiple_elements_most_specific_returned() {
        //           0         1         2         3
        //           0123456789012345678901234567890123456789
        let src = "var alpha: bit = false; start Beta;";
        let index = build_index(src);
        // Смещение 4 — 'a' в "alpha"
        let node = index.node_at_offset(4).expect("должен найти элемент");
        assert_eq!(node.name, "alpha");
        // Смещение 30 — 'B' в "Beta"
        let node2 = index.node_at_offset(30).expect("должен найти состояние");
        assert_eq!(node2.name, "Beta");
    }

    /// len() возвращает корректное количество записей.
    #[test]
    fn len_matches_element_count() {
        // 1 переменная + 1 состояние = минимум 2
        let src = "var x: bit = false; start S;";
        let index = build_index(src);
        assert!(index.len() >= 2);
    }

    /// Константа индексируется с правильным видом.
    #[test]
    fn const_kind_is_indexed() {
        let src = "const LIMIT: bit = true;";
        let index = build_index(src);
        let node = index.node_at_offset(6);
        assert!(node.is_some(), "должен найти константу");
        let node = node.unwrap();
        assert_eq!(node.name, "LIMIT");
        assert_eq!(node.kind, SemanticNodeKind::Const);
    }

    /// Внешняя функция индексируется с видом ExternFunction.
    #[test]
    fn extern_function_kind_is_indexed() {
        let src = "extern fn send(data: bit);";
        let index = build_index(src);
        let node = index.node_at_offset(10);
        assert!(node.is_some(), "должен найти extern fn");
        let node = node.unwrap();
        assert_eq!(node.name, "send");
        assert_eq!(node.kind, SemanticNodeKind::ExternFunction);
    }
}
