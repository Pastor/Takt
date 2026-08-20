use crate::diagnostics::{Diagnostic, Location};
use crate::generator::FloatWidth;
use crate::semantic::minimap::{Element, Map, Name};
use crate::semantic::naming::normalize_camelcase_name;
use crate::semantic::unused::UsageSet;
use crate::semantic::{ModelNode, StateNode};
use std::cell::RefCell;
use std::rc::Rc;

pub struct CMap {
    filename: String,
    map: Map,
    /// Множество используемых имён модели (для фильтрации неиспользуемых элементов).
    usage: UsageSet,
    /// Флаг включения генерации проверок Guard-формул.
    guard_enable: bool,
    /// Ширина вещественного типа (фича 0029): `double` при `W64`, `float` при `W32`.
    float_width: FloatWidth,
    /// Профиль времени (фича 0134): в какие единицы пересчитывается длительность.
    time_profile: crate::semantic::duration::TimeProfile,
    /// Режим `c-hal` (фича 0020-05): цель знает адреса и вправе обращаться к
    /// памяти напрямую.
    ///
    /// Нужен печати анонимного обращения `#0x…` (фича 0189): у цели `c` доступ
    /// идёт **только** через колбэки HAL, поэтому обращение по адресу она
    /// отвергает (`CC-021`), а `c-hal` печатает `*(volatile uintN_t*)`.
    hal: bool,
    /// Предупреждения цели, накопленные за проход (фича 0314).
    ///
    /// Канал у цели `c` существует с фичи 0168 (`Generator::generate` возвращает
    /// `Vec<Diagnostic>`), но говорить по нему было нечем. Первое, что по нему
    /// поехало, — `CC-024`: вызов `debug(…)` цель **выбрасывает**, и молчать
    /// об этом нельзя (соседние `st` и `rust` на том же входе отказывают).
    ///
    /// ⚠️ `RefCell`, потому что печатники получают карту по `&self`: заводить
    /// `&mut` через полтора десятка сигнатур ради накопителя — цена выше
    /// пользы. Генерация однопоточна по построению.
    warnings: RefCell<Vec<Diagnostic>>,
}

impl CMap {
    pub(crate) fn raw_model_at(&self, name: Name) -> Result<Rc<RefCell<ModelNode>>, Diagnostic> {
        self.map
            .model_at(Some(name.unique().to_string()))
            .ok_or_else(|| {
                Diagnostic::error(
                    Location::Codegen,
                    format!("Model with name '{}' not found", name),
                )
                .with_code("CC-004")
            })
    }

    pub(crate) fn raw_state_at(&self, name: Name) -> Result<Rc<RefCell<StateNode>>, Diagnostic> {
        self.map
            .state_at(Some(name.unique().to_string()))
            .ok_or_else(|| {
                Diagnostic::error(
                    Location::Codegen,
                    format!("State with name '{}' not found", name),
                )
                .with_code("CC-005")
            })
    }
}

impl CMap {
    /// Запоминает предупреждение цели (фича 0314).
    pub(crate) fn warn(&self, diagnostic: Diagnostic) {
        self.warnings.borrow_mut().push(diagnostic);
    }

    /// Забирает накопленные предупреждения — зовёт генератор в конце прохода.
    pub(crate) fn take_warnings(&self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.warnings.borrow_mut())
    }

    pub fn new(filename: &str, model: &ModelNode, guard_enable: bool) -> Result<Self, Diagnostic> {
        let model_rc = Rc::new(RefCell::new(model.copy(None, None)));
        let usage = crate::semantic::unused::compute_usage(Rc::clone(&model_rc));
        Ok(Self {
            filename: filename.to_string(),
            map: Map::create(model_rc)?,
            usage,
            guard_enable,
            float_width: FloatWidth::default(),
            time_profile: crate::semantic::duration::TimeProfile::default(),
            hal: false,
            warnings: RefCell::new(Vec::new()),
        })
    }

    /// Включает режим `c-hal` (фича 0020-05, потребитель — фича 0189).
    ///
    /// Отдельным методом по той же причине, что профиль времени и ширина
    /// вещественного: умолчание — цель `c`, и существующие вызовы `new` его не
    /// повторяют. Умолчание при этом **безопасное**: не зная адресов, цель
    /// отказывает, а не печатает доступ наугад.
    pub fn with_hal(mut self, hal: bool) -> Self {
        self.hal = hal;
        self
    }

    /// Знает ли цель адреса (режим `c-hal`).
    pub(crate) fn hal(&self) -> bool {
        self.hal
    }

    /// Задаёт профиль времени (фича 0134): «часы» либо «такты».
    ///
    /// Отдельным методом по той же причине, что и ширина вещественного:
    /// умолчание — профиль «часы», и существующие вызовы `new` его не повторяют.
    pub fn with_time_profile(mut self, profile: crate::semantic::duration::TimeProfile) -> Self {
        self.time_profile = profile;
        self
    }

    /// Профиль времени, выбранный для этой сборки (фича 0134).
    pub(crate) fn time_profile(&self) -> crate::semantic::duration::TimeProfile {
        self.time_profile
    }

    /// Задаёт ширину вещественного типа (фича 0029).
    ///
    /// Отдельным методом, а не параметром `new`: умолчание `W64` — поведение по
    /// умолчанию генератора, и 26 существующих вызовов `new` (в основном тесты)
    /// не должны повторять его дословно.
    pub fn with_float_width(mut self, float_width: FloatWidth) -> Self {
        self.float_width = float_width;
        self
    }

    /// Ширина вещественного типа в порождаемом C.
    pub(crate) fn float_width(&self) -> FloatWidth {
        self.float_width
    }

    pub fn guard_enable(&self) -> bool {
        self.guard_enable
    }

    /// Возвращает ссылку на множество используемых имён модели.
    pub fn usage(&self) -> &UsageSet {
        &self.usage
    }

    pub fn get_filename(&self) -> &str {
        &self.filename
    }

    /// Возвращает имя корневой структуры в PascalCase (например, `ElevatorEngine`).
    #[allow(dead_code)]
    pub fn get_struct_name(&self) -> String {
        let name = self
            .map
            .model_at(None)
            .unwrap()
            .borrow()
            .name
            .clone()
            .unwrap();
        normalize_camelcase_name(&name)
    }

    pub fn using_models(&self) -> Vec<Element> {
        self.map.used_models()
    }

    /// Возвращает элемент корневой модели из карты (если есть).
    #[allow(dead_code)]
    pub fn own_model(&self) -> Option<Element> {
        self.map.own()
    }

    /// Возвращает имя стартового состояния корневой модели.
    #[allow(dead_code)]
    pub fn start(&self) -> Name {
        let Element::Model { start, .. } = self.map.model().clone() else {
            unreachable!()
        };
        start
    }

    pub(crate) fn model(&self) -> Element {
        self.map.model()
    }

    pub fn state_at(&self, name: Name) -> Option<Element> {
        if let Some(element) = self.map.element_at(name)
            && element.is_state()
        {
            Some(element)
        } else {
            None
        }
    }

    pub fn root_name(&self) -> Name {
        self.map.root_name()
    }

    pub fn states(&self) -> Vec<Name> {
        self.map.states()
    }

    /// Возвращает корневую модель (только для чтения, для генерации типов).
    pub(crate) fn root_model_node(&self) -> Option<Rc<RefCell<crate::semantic::ModelNode>>> {
        self.map.model_at(None)
    }
}
