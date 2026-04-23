mod c;
mod indent;
mod plantuml;

use crate::diagnostics::Diagnostic;
use crate::semantic::ModelNode;

/// Пара (заголовочный файл, исходный файл) для генераторов, производящих несколько артефактов.
pub type Source = (Option<String>, Option<String>);

/// Поддерживаемые языки генерации кода.
#[derive(Debug)]
pub enum Language {
    /// Генерация C-кода.
    C,
    /// Генерация диаграммы состояний PlantUML.
    PlantUML,
}

/// Интерфейс генератора кода для языка Lam.
pub trait Generator {
    /// Генерирует код из семантического дерева модели и записывает результат в файл.
    fn generate(
        &self,
        model: &ModelNode,
        output_path: &str,
        guard_enable: bool,
    ) -> Result<(), Diagnostic>;
}

/// Запускает генератор кода для заданного языка.
///
/// Выбирает нужный генератор по значению [`Language`] и вызывает его.
pub fn generate(
    l: Language,
    model: &ModelNode,
    output_path: &str,
    guard_enable: bool,
) -> Result<(), Diagnostic> {
    match l {
        Language::C => {
            let generator = c::Generator {};
            generator.generate(model, output_path, guard_enable)
        }
        Language::PlantUML => {
            let generator = plantuml::Generator {};
            generator.generate(model, output_path, guard_enable)
        }
    }
}
