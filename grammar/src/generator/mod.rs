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
    /// Режим `c-hal` (фича 0020-05): эмитить таблицу адресов портов и дефолтную
    /// реализацию HAL (`*(volatile T*)addr`). В обычном режиме `c` — `false`,
    /// вывод не меняется.
    pub hal: bool,
    /// Разрешённые адреса портов (`имя порта → адрес`) для режима [`hal`].
    ///
    /// Заполняется из [`resolve_addresses`](crate::address_map::resolve_addresses)
    /// (приоритет inline < `address` < внешняя карта). В обычном режиме пуста.
    pub address_map: std::collections::HashMap<String, crate::address_map::ResolvedAddress>,
}

impl GenerateOptions {
    /// Создаёт опции с указанным режимом guard-проверок (режим `c`, без HAL).
    pub fn new(guard_enable: bool) -> Self {
        Self {
            guard_enable,
            hal: false,
            address_map: std::collections::HashMap::new(),
        }
    }
}

impl Default for GenerateOptions {
    /// По умолчанию guard-проверки включены, режим HAL выключен.
    fn default() -> Self {
        Self {
            guard_enable: true,
            hal: false,
            address_map: std::collections::HashMap::new(),
        }
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
