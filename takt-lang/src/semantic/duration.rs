//! Профили времени и пересчёт длительности (фича 0134, подзадача 0134-02).
//!
//! **Единственное место арифметики времени в проекте** (требование R4 анализа,
//! правило 7 ADR 0134). Ни один генератор и ни один слой симулятора своей
//! арифметики длительности не заводит: разъехавшись, они дали бы разное число
//! тактов для одного текста — ровно тот класс дефекта, который проект ловит
//! потактовыми сверками. Прецеденты общего слоя: `enum_facts` (разрядность
//! перечислений), `bit_vector` (упаковка `[bit;N]`), `lower_float`.
//!
//! ## Два профиля (правило 3 ADR)
//!
//! - **«Часы»** — умолчание. Длительность сравнивается с показаниями внешнего
//!   источника времени; частота **не нужна**, и модель остаётся годной для
//!   потактовой отладки. Квант — **1 мс** (ограничение реализации источника).
//! - **«Такты»** — включается объявлением частоты (`clock 1kHz;` в модели либо
//!   флагом `--tick-hz`, флаг переопределяет). Длительность пересчитывается в
//!   число тактов; источник времени не нужен. Квант — период такта.
//!
//! Иных способов выбора профиля нет: **частота задана → такты, не задана → часы**.
//!
//! ## Непредставимое — ошибка, а не округление
//!
//! `500us` в профиле «часы» и `500ms` при 3 Гц (1.5 такта) дают **`SE-063`**.
//! Прецедент — `SE-058` для fixed-point: непредставимый литерал `q(8,8) := 0.001`
//! отвергается, а не округляется. Округление здесь означало бы выдержку, не
//! равную заявленной, — молча.

use crate::diagnostics::{Diagnostic, Location};

/// Квант профиля «часы» — миллисекунда, выраженная в наносекундах.
///
/// Это **ограничение реализации источника времени** (решение заказчика), а не
/// свойство языка: литерал `250us` разбирается лексером всегда, но представим
/// лишь в профиле «такты» с достаточной частотой.
pub const CLOCK_QUANTUM_NS: i64 = 1_000_000;

/// Наносекунд в секунде.
const NANOS_PER_SECOND: i64 = 1_000_000_000;

/// Профиль времени, выбранный для компиляции.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum TimeProfile {
    /// Внешний источник времени; длительность меряется миллисекундами.
    #[default]
    Clock,
    /// Счёт тактов с известной частотой.
    Ticks {
        /// Частота тактирования в герцах.
        hertz: u64,
    },
}

impl TimeProfile {
    /// Название профиля для сообщений (единая формулировка на весь проект).
    pub fn name(self) -> &'static str {
        match self {
            Self::Clock => "часы",
            Self::Ticks { .. } => "такты",
        }
    }

    /// Название единицы профиля в родительном падеже: «мс» либо «тактов».
    pub fn unit_name(self) -> &'static str {
        match self {
            Self::Clock => "мс",
            Self::Ticks { .. } => "тактов",
        }
    }
}

/// Причина отказа пересчёта.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum DurationError {
    /// Длительность не кратна кванту профиля (`SE-063`).
    NotRepresentable,
    /// Длительность не помещается в счётчик даже 64 бит (`SE-064`).
    TooLarge,
    /// Отрицательная длительность (в языке её нет; сторож от чужой ошибки).
    Negative,
}

/// Пересчитывает длительность (наносекунды) в единицы профиля.
///
/// Профиль «часы» → миллисекунды; профиль «такты» → число тактов. Дробный
/// результат — **отказ** (`NotRepresentable`), а не округление: правило 6 ADR.
///
/// # Примеры
///
/// ```
/// use takt_lang::semantic::duration::{units, TimeProfile};
///
/// // 3 с в профиле «часы» — 3000 мс.
/// assert_eq!(units(3_000_000_000, TimeProfile::Clock), Ok(3_000));
/// // 3 с при 1 кГц — 3000 тактов.
/// assert_eq!(units(3_000_000_000, TimeProfile::Ticks { hertz: 1_000 }), Ok(3_000));
/// // 500 мс при 3 Гц — 1.5 такта: непредставимо.
/// assert!(units(500_000_000, TimeProfile::Ticks { hertz: 3 }).is_err());
/// ```
pub fn units(nanos: i64, profile: TimeProfile) -> Result<u64, DurationError> {
    if nanos < 0 {
        return Err(DurationError::Negative);
    }
    let quantum = match profile {
        TimeProfile::Clock => CLOCK_QUANTUM_NS,
        TimeProfile::Ticks { hertz } => {
            if hertz == 0 {
                return Err(DurationError::NotRepresentable);
            }
            // Период такта в наносекундах. Частота, не делящая секунду нацело
            // (например 3 Гц), даёт дробный период — тогда кратность проверяется
            // по произведению, а не по периоду: см. ветку ниже.
            let hertz = i64::try_from(hertz).map_err(|_| DurationError::TooLarge)?;
            if NANOS_PER_SECOND % hertz != 0 {
                // Точная проверка без потери: nanos·hertz должно делиться на 10⁹.
                let product = nanos.checked_mul(hertz).ok_or(DurationError::TooLarge)?;
                if product % NANOS_PER_SECOND != 0 {
                    return Err(DurationError::NotRepresentable);
                }
                return u64::try_from(product / NANOS_PER_SECOND)
                    .map_err(|_| DurationError::TooLarge);
            }
            NANOS_PER_SECOND / hertz
        }
    };
    if nanos % quantum != 0 {
        return Err(DurationError::NotRepresentable);
    }
    u64::try_from(nanos / quantum).map_err(|_| DurationError::TooLarge)
}

/// Наименьшая разрядность счётчика (8/16/32/64), вмещающая `value`.
///
/// `None` — не вмещается даже в 64 бита (`SE-064`). Разрядность выбирает
/// **компилятор** (требование T7 отчёта `DIFF.md`): счётчик, объявленный узко,
/// молча зациклил бы выдержку.
///
/// ⚠️ Сравнение «истекло ли» ведётся **разностью** беззнаковых счётчиков
/// (`now - t0 >= D`), поэтому обёртка не ломает выдержки короче полупериода
/// счётчика — обёртка беззнакового нормирована ADR 0127.
pub fn counter_bits(value: u64) -> Option<u8> {
    match value {
        v if v <= u64::from(u8::MAX) => Some(8),
        v if v <= u64::from(u16::MAX) => Some(16),
        v if v <= u64::from(u32::MAX) => Some(32),
        _ => Some(64),
    }
}

/// Пересчитывает длительность и превращает отказ в диагностику.
///
/// `what` описывает место (`константа 'DWELL'`) и попадает в сообщение: пачка
/// диагностик без указания предмета бесполезна (урок 0130).
pub fn units_or_diagnostic(
    nanos: i64,
    profile: TimeProfile,
    loc: Location,
    what: &str,
) -> Result<u64, Diagnostic> {
    units(nanos, profile).map_err(|error| match error {
        DurationError::NotRepresentable => {
            let detail = match profile {
                TimeProfile::Clock => {
                    "квант профиля «часы» — 1 мс; выразите длительность целым числом миллисекунд"
                        .to_string()
                }
                TimeProfile::Ticks { hertz } => format!(
                    "при частоте {hertz} Гц длительность не кратна периоду такта; \
                     выберите кратную длительность или другую частоту"
                ),
            };
            Diagnostic::error(
                loc,
                format!(
                    "{what}: длительность {nanos} нс непредставима в профиле «{}» — {detail}",
                    profile.name()
                ),
            )
            .with_code("SE-063")
        }
        DurationError::TooLarge => Diagnostic::error(
            loc,
            format!(
                "{what}: длительность {nanos} нс не помещается в счётчик времени \
                 (профиль «{}», максимум — 64 бита)",
                profile.name()
            ),
        )
        .with_code("SE-064"),
        DurationError::Negative => Diagnostic::error(
            loc,
            format!("{what}: отрицательная длительность ({nanos} нс) не имеет смысла"),
        )
        .with_code("SE-063"),
    })
}

/// Выбирает профиль: флаг сборки переопределяет объявление в модели.
///
/// Приоритет — **`clock` в модели < `--tick-hz`** (правило 3 ADR). Выражен
/// **одной** функцией намеренно: у карты адресов (фича 0020) приоритет
/// источников размазался по слоям, и это стоило отдельной проработки.
pub fn resolve_profile(model_clock_hz: Option<u64>, flag_tick_hz: Option<u64>) -> TimeProfile {
    match flag_tick_hz.or(model_clock_hz) {
        Some(hertz) => TimeProfile::Ticks { hertz },
        None => TimeProfile::Clock,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_profile_counts_milliseconds() {
        assert_eq!(units(3_000_000_000, TimeProfile::Clock), Ok(3_000));
        assert_eq!(units(500_000_000, TimeProfile::Clock), Ok(500));
        assert_eq!(units(0, TimeProfile::Clock), Ok(0));
    }

    #[test]
    fn sub_millisecond_is_refused_in_clock_profile() {
        // `500us` — мельче кванта источника времени.
        assert_eq!(
            units(500_000, TimeProfile::Clock),
            Err(DurationError::NotRepresentable)
        );
        assert_eq!(
            units(40, TimeProfile::Clock),
            Err(DurationError::NotRepresentable)
        );
    }

    #[test]
    fn tick_profile_counts_ticks() {
        let khz = TimeProfile::Ticks { hertz: 1_000 };
        assert_eq!(units(3_000_000_000, khz), Ok(3_000));
        assert_eq!(units(1_000_000, khz), Ok(1));
        // 1 МГц: микросекунды представимы, в отличие от профиля «часы».
        let mhz = TimeProfile::Ticks { hertz: 1_000_000 };
        assert_eq!(units(500_000, mhz), Ok(500));
    }

    #[test]
    fn non_integral_tick_count_is_refused() {
        // Пример из ADR: `500ms` при 3 Гц = 1.5 такта.
        assert_eq!(
            units(500_000_000, TimeProfile::Ticks { hertz: 3 }),
            Err(DurationError::NotRepresentable)
        );
        // Та же частота, но кратная длительность — принимается.
        assert_eq!(units(1_000_000_000, TimeProfile::Ticks { hertz: 3 }), Ok(3));
        // 1 такт при 3 Гц (⅓ с) записать точно нельзя — и это отказ.
        assert_eq!(
            units(333_333_333, TimeProfile::Ticks { hertz: 3 }),
            Err(DurationError::NotRepresentable)
        );
    }

    #[test]
    fn counter_width_is_the_narrowest_that_fits() {
        assert_eq!(counter_bits(255), Some(8));
        assert_eq!(counter_bits(256), Some(16));
        assert_eq!(counter_bits(65_535), Some(16));
        assert_eq!(counter_bits(65_536), Some(32));
        assert_eq!(counter_bits(u64::from(u32::MAX) + 1), Some(64));
    }

    #[test]
    fn profile_priority_is_flag_over_model() {
        // Флаг переопределяет объявление (правило 3 ADR).
        assert_eq!(resolve_profile(None, None), TimeProfile::Clock);
        assert_eq!(
            resolve_profile(Some(1_000), None),
            TimeProfile::Ticks { hertz: 1_000 }
        );
        assert_eq!(
            resolve_profile(Some(1_000), Some(8_000_000)),
            TimeProfile::Ticks { hertz: 8_000_000 }
        );
        assert_eq!(
            resolve_profile(None, Some(50)),
            TimeProfile::Ticks { hertz: 50 }
        );
    }

    #[test]
    fn diagnostic_names_the_place_and_the_reason() {
        let diagnostic = units_or_diagnostic(
            500_000,
            TimeProfile::Clock,
            Location::Implicit,
            "константа 'DWELL'",
        )
        .expect_err("500us в профиле «часы» непредставима");
        assert_eq!(diagnostic.code.as_deref(), Some("SE-063"));
        assert!(
            diagnostic.message.contains("константа 'DWELL'"),
            "{diagnostic:?}"
        );
        assert!(diagnostic.message.contains("1 мс"), "{diagnostic:?}");
    }
}
