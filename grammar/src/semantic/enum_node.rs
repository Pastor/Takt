//! Семантический узел перечисления (`enum`) языка BuT.
//!
//! Модуль реализует [`EnumDefinitionNode`] — структуру данных, описывающую
//! именованное перечисление с вариантами и опциональными числовыми значениями.
//!
//! Перечисления создаются в ходе семантического анализа при обработке
//! объявлений `enum` в исходном тексте.

use crate::diagnostics::Location;
use crate::semantic::ModelNode;
use std::cell::RefCell;
use std::rc::Weak;

/// Семантический узел перечисления (Ce4).
///
/// Описывает именованное перечисление: набор вариантов с опциональными числовыми
/// значениями. Если значение не задано, предполагается автоинкремент от 0.
///
/// # Пример BuT (концептуально, поддержка на семантическом уровне)
///
/// ```text
/// // enum Color { Red = 0, Green = 1, Blue = 2 }
/// ```
///
/// Фактически перечисления сейчас не имеют синтаксиса в грамматике BuT,
/// поэтому `EnumNode` создаётся программно через API.
#[derive(Default, Debug, Clone)]
pub struct EnumDefinitionNode {
    /// Слабая ссылка на родительскую модель (для разрешения имён при генерации кода).
    pub upper: Option<Weak<RefCell<ModelNode>>>,
    /// Имя перечисления.
    pub name: String,
    /// Варианты перечисления: `(имя_варианта, числовое_значение)`.
    /// Если значение не задано при создании, оно равно индексу варианта.
    pub variants: Vec<(String, i64)>,
    /// Позиция объявления перечисления в исходном тексте.
    pub loc: Location,
}

impl EnumDefinitionNode {
    /// Возвращает имя перечисления.
    #[allow(dead_code)]
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}

impl PartialEq for EnumDefinitionNode {
    fn eq(&self, other: &Self) -> bool {
        // loc игнорируется: не является частью семантической идентичности перечисления
        self.name == other.name && self.variants == other.variants
    }
}

impl Eq for EnumDefinitionNode {}

impl EnumDefinitionNode {
    /// Создаёт новый `EnumNode` с именем и именованными вариантами.
    ///
    /// Если значение варианта задано явно через `(имя, Some(значение))`,
    /// используется это значение. Иначе вариант получает порядковый индекс.
    ///
    /// # Пример
    ///
    /// ```
    /// use grammar::semantic::EnumDefinitionNode;
    ///
    /// let e = EnumDefinitionNode::new("Color", &[("Red", None), ("Green", None), ("Blue", Some(5))]);
    /// assert_eq!(e.variants[0], ("Red".to_string(), 0));
    /// assert_eq!(e.variants[1], ("Green".to_string(), 1));
    /// assert_eq!(e.variants[2], ("Blue".to_string(), 5));
    /// ```
    pub fn new(name: &str, variants: &[(&str, Option<i64>)]) -> Self {
        let resolved: Vec<(String, i64)> = variants
            .iter()
            .enumerate()
            .map(|(i, (vname, val))| (vname.to_string(), val.unwrap_or(i as i64)))
            .collect();
        EnumDefinitionNode {
            upper: None,
            name: name.to_string(),
            variants: resolved,
            loc: Location::Implicit,
        }
    }

    /// Ищет числовое значение варианта по имени.
    ///
    /// Возвращает `Some(значение)`, если вариант найден.
    ///
    /// # Пример
    ///
    /// ```
    /// use grammar::semantic::EnumDefinitionNode;
    ///
    /// let e = EnumDefinitionNode::new("Color", &[("Red", None), ("Green", None)]);
    /// assert_eq!(e.find_variant("Red"), Some(0));
    /// assert_eq!(e.find_variant("Blue"), None);
    /// ```
    pub fn find_variant(&self, variant_name: &str) -> Option<i64> {
        self.variants
            .iter()
            .find(|(name, _)| name == variant_name)
            .map(|(_, val)| *val)
    }

    /// Возвращает `true`, если вариант с данным именем существует.
    pub fn has_variant(&self, variant_name: &str) -> bool {
        self.find_variant(variant_name).is_some()
    }
}
