mod c;
mod indent;
mod plantuml;

use crate::diagnostics::Diagnostic;
use crate::semantic::ModelNode;

/// Поддерживаемые языки генерации кода.
///
/// Помечен `#[non_exhaustive]`: список целевых языков будет расширяться, и
/// добавление вариантов не должно ломать обратную совместимость (правило 11).
#[derive(Debug)]
#[non_exhaustive]
pub enum Language {
    /// Генерация C-кода.
    C,
    /// Генерация диаграммы состояний PlantUML.
    PlantUML,
}

/// Опции генерации кода.
///
/// Заменяет «голый» булев флаг `guard_enable` на именованную структуру опций —
/// вызов `generate(model, path, &options)` читается лучше, чем `generate(..., true)`.
/// Помечена `#[non_exhaustive]`: набор опций будет расширяться без слома обратной
/// совместимости (правило 11); конструирование — через [`GenerateOptions::new`]
/// либо [`Default`] с последующей правкой полей.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GenerateOptions {
    /// Генерировать guard-проверки в целевом коде.
    pub guard_enable: bool,
}

impl GenerateOptions {
    /// Создаёт опции с указанным режимом guard-проверок.
    pub fn new(guard_enable: bool) -> Self {
        Self { guard_enable }
    }
}

impl Default for GenerateOptions {
    /// По умолчанию guard-проверки включены.
    fn default() -> Self {
        Self { guard_enable: true }
    }
}

/// Интерфейс генератора кода для языка Lam.
pub trait Generator {
    /// Генерирует код из семантического дерева модели и записывает результат в файл.
    fn generate(
        &self,
        model: &ModelNode,
        output_path: &str,
        options: &GenerateOptions,
    ) -> Result<(), Diagnostic>;
}

/// Запускает генератор кода для заданного языка.
///
/// Выбирает нужный генератор по значению [`Language`] и вызывает его.
pub fn generate(
    l: Language,
    model: &ModelNode,
    output_path: &str,
    options: &GenerateOptions,
) -> Result<(), Diagnostic> {
    match l {
        Language::C => {
            let generator = c::Generator {};
            generator.generate(model, output_path, options)
        }
        Language::PlantUML => {
            let generator = plantuml::Generator {};
            generator.generate(model, output_path, options)
        }
    }
}
