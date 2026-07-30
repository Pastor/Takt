mod c;
mod indent;
mod plantuml;
mod rust;
mod st;
mod sv;

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
    /// Генерация Structured Text (IEC 61131-3) — язык ПЛК (фича 0041).
    ///
    /// Модель → `FUNCTION_BLOCK`, состояния → `CASE state OF` (ADR 0041,
    /// Option A). Потребление карты адресов (`AT %…`) включается флагом
    /// [`GenerateOptions::hal`] — тем же, что и для режима `c-hal`.
    ST,
    /// Генерация `no_std` Rust — прошивка микроконтроллера (фича 0050).
    ///
    /// Модель → `struct`, состояния → `enum` + `match`, порты → трейт `Hal`
    /// вместо пары указателей на функции и `void *userdata` цели `c`
    /// (ADR 0050, Option A по обеим развилкам). Вывод — один `.rs`-файл,
    /// подключаемый пользователем через `mod`; `Cargo.toml` генератор не
    /// порождает и им не владеет.
    ///
    /// Карта адресов ([`GenerateOptions::address_map`]) **не потребляется**:
    /// порты идут через HAL — это аналог режима `c`, а не `c-hal`.
    Rust,
    /// Генерация синтезируемого SystemVerilog (IEEE 1800) — FPGA/ASIC (фича 0045).
    ///
    /// Первая **аппаратная** цель: у `C`/`ST`/`Rust` такт — итерация цикла
    /// сканирования, здесь такт Takt ≡ **фронт тактового сигнала** `posedge clk`.
    /// Модель → `module`, состояния → `typedef enum` + `unique case`, порты
    /// `in`/`out` → `input`/`output logic` (ADR 0045).
    ///
    /// Сброс синхронный, активный низкий (`rst_n`); стартовое состояние стоит в
    /// ветви сброса, синтетического `INIT` нет — контракт
    /// [ADR 0033](../../../docs/adr/0033-init-tick-alignment.md) выполняется
    /// конструктивно, а не правкой.
    ///
    /// Карта адресов ([`GenerateOptions::address_map`]) **не потребляется**:
    /// MMIO-адрес для RTL бессмыслен — сигнал приходит на вывод кристалла, а не
    /// по адресу. Парная цель — [`SvMmio`](Language::SvMmio).
    SV,
    /// Генерация синтезируемого SystemVerilog с **регистровым файлом** — порты с
    /// адресом становятся битами регистров на шинно-агностичном интерфейсе (фича
    /// 0062).
    ///
    /// Парная к [`SV`](Language::SV), как `c-hal` парная к `c` (прецедент
    /// 0020-05). В отличие от `sv`, карта адресов
    /// ([`GenerateOptions::address_map`]) **потребляется**: порт **с** адресом →
    /// бит регистра (направление принадлежит биту, ADR 0062, правило 4); порт
    /// **без** адреса → порт модуля. Модуль получает синхронный регистровый
    /// интерфейс (`reg_addr`/`reg_wdata`/`reg_wen`/`reg_rdata`) **без протокола**:
    /// адаптеры APB/AXI-Lite/Wishbone — отдельные фичи по требованию (ADR 0062,
    /// Option B). Автомат, композиция и сброс — те же, что у `sv`.
    SvMmio,
}

/// Ширина вещественного типа в порождаемом C (фича 0029, ADR Option R-C).
///
/// Умолчание — [`W64`](FloatWidth::W64) (`double`): симулятор считает в f64
/// (`eval::Value::Real`), и без совпадения точности сверка модели с
/// синтезированным кодом по `float` недостижима. [`W32`](FloatWidth::W32)
/// (`float`) остаётся для платформ, где 8-байтное чтение недопустимо: цена
/// умолчания — ширина вещественного порта `c-hal` 4 → 8 байт.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FloatWidth {
    /// `float` — 4 байта (f32).
    W32,
    /// `double` — 8 байт (f64), совпадает с точностью симулятора.
    #[default]
    W64,
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
    /// Ширина вещественного типа в порождаемом C (фича 0029). Умолчание —
    /// [`FloatWidth::W64`]; CLI-флаг `--float-width=32|64`.
    pub float_width: FloatWidth,
    /// Глобальная точность `q(m, n)`, которой реализуется `float` (фича 0096).
    ///
    /// `None` (умолчание) — прежнее поведение: `float` нативен (`c`/`rust`/`st`)
    /// либо `SV-003` (`sv`). `Some((m, n))` (CLI-флаг `--float-as-q=m.n`) —
    /// глобальная точность подстановки `float → q(m, n)`: цель `sv` применяет её
    /// всегда (снимая `SV-003`), цели `c`/`rust`/`st` — только при
    /// [`float_embedded`](Self::float_embedded). Границы — правило 1 ADR 0061.
    pub float_as_q: Option<(u8, u8)>,
    /// Частота тактирования для профиля «такты» (фича 0134), в герцах.
    ///
    /// `None` (умолчание) — профиль **«часы»**: длительность меряется
    /// миллисекундами внешнего источника времени, частота не нужна.
    /// `Some(hz)` (CLI-флаг `--tick-hz`) — профиль **«такты»**: длительность
    /// пересчитывается в число тактов. Флаг **переопределяет** объявление
    /// `clock` в модели; приоритет разрешает
    /// [`duration::resolve_profile`](crate::semantic::duration::resolve_profile).
    ///
    /// ⚠️ Это **второе осознанное исключение** из принципа «CLI не меняет
    /// логику» (правило 13 ADR 0134): первое — `--float-as-q`/`--float-embedded`
    /// (действие A-7 фичи 0096). Исключение уже, чем у 0096: в профиле «часы»
    /// флага нет вовсе.
    pub tick_hz: Option<u64>,
    /// Режим `--parameters=specialize` (фича 0185): инстанцирования с
    /// аргументами заменяются копиями моделей с подставленными значениями —
    /// между стадиями 1 и 2 семантики (`semantic/specialize.rs`). Умолчание
    /// `false` — режим `assign`: модель одна, значения присваиваются полям
    /// экземпляров. ⚠️ Оба режима обязаны давать одинаковое поведение
    /// (потактовая сверка — сторож гибрида, ADR 0185 Option E).
    pub specialize: bool,
    /// Для целей `c`/`rust`/`st`: реализовать `float` целочисленным Q-путём
    /// (embedded без FPU) вместо нативного (фича 0096, CLI-флаг
    /// `--float-embedded`). Действует только вместе с [`float_as_q`](Self::float_as_q);
    /// на `sv` не влияет (там `float` всегда `q`).
    pub float_embedded: bool,
}

impl GenerateOptions {
    /// Создаёт опции с указанным режимом guard-проверок (режим `c`, без HAL).
    pub fn new(guard_enable: bool) -> Self {
        Self {
            guard_enable,
            hal: false,
            address_map: std::collections::HashMap::new(),
            float_width: FloatWidth::default(),
            float_as_q: None,
            float_embedded: false,
            tick_hz: None,
            specialize: false,
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
            float_width: FloatWidth::default(),
            float_as_q: None,
            float_embedded: false,
            tick_hz: None,
            specialize: false,
        }
    }
}

/// Интерфейс генератора кода для языка Takt.
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
        Language::ST => {
            let generator = st::Generator {};
            generator.generate(model, output_path, options)
        }
        Language::Rust => {
            let generator = rust::Generator {};
            generator.generate(model, output_path, options)
        }
        Language::SV => {
            let generator = sv::Generator { mmio: false };
            generator.generate(model, output_path, options)
        }
        Language::SvMmio => {
            let generator = sv::Generator { mmio: true };
            generator.generate(model, output_path, options)
        }
    }
}
