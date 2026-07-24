// Порождено компилятором Takt (taktc) — цель: Rust (профиль no_std).
// Не редактировать вручную: файл перезаписывается при каждой генерации.
//
// Модуль не обращается к std и подключается как `mod`:
//
//     #[path = "comprehensive.rs"]
//     pub mod comprehensive;
//
// Атрибута #![no_std] здесь нет намеренно: он допустим только в корне
// крейта, а no_std — свойство крейта, не модуля. Совместимость с no_std
// проверяется гейтом (scripts/precheck.sh).

#![forbid(unsafe_code)]

/// Перечисление 'Mode' модели.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Mode {
    Auto = 0,
    Manual = 1,
    Emergency = 2,
}

const COOL_STEP: u8 = 3;
const HEAT_STEP: u8 = 8;
const MAX_COUNT: u8 = 3;
const MAX_TEMP: u8 = 100;
const WARMUP_TEMP: u8 = 10;

/// Аппаратный слой модели.
///
/// Заменяет пару указателей на функции и `void *userdata` цели `c`:
/// состояние слоя живёт в самом типе-реализации, поэтому привести
/// его не к тому типу или забыть проставить колбэк невозможно.
pub trait Hal {
    /// Внешняя функция модели (`extern fn` в исходнике .lam).
    fn log_count(&mut self, n: u8);
    /// Внешняя функция модели (`extern fn` в исходнике .lam).
    fn log_temp(&mut self, value: u8);
}

/// Функция 'clamp_temp' модели.
fn clamp_temp(value: u8) -> u8 {
    if value > MAX_TEMP {
        return MAX_TEMP;
    }
    value
}

/// Функция 'increment' модели.
fn increment(n: u8) -> u8 {
    n + 1
}

/// Функция 'steps_to_limit' модели.
fn steps_to_limit(value: u8) -> u8 {
    let mut remaining: u8 = 0;
    let mut v: u8 = value;
    while v < MAX_TEMP {
        v += HEAT_STEP;
        remaining += 1;
    }
    remaining
}

/// Функция 'steps_to_zero' модели.
fn steps_to_zero(value: u8) -> u8 {
    let mut remaining: u8 = 0;
    let mut v: u8 = value;
    while v > 0 {
        if v > COOL_STEP {
            v -= COOL_STEP;
        } else {
            v = 0;
        }
        remaining += 1;
    }
    remaining
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComprehensiveControllerState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Cooling,
    Done,
    Heating,
    Idle,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'Controller'.
pub struct ComprehensiveController {
    count: u8,
    mode: Mode,
    temperature: u8,
    state: ComprehensiveControllerState,
}

impl ComprehensiveController {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            count: 0,
            mode: Mode::Auto,
            temperature: 0,
            state: ComprehensiveControllerState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.count = 0;
        self.mode = Mode::Auto;
        self.temperature = 0;
        self.state = ComprehensiveControllerState::Init;
    }

    /// Один такт автомата.
    fn tick<H: Hal>(&mut self, hal: &mut H) {
        if self.state == ComprehensiveControllerState::Init {
            self.temperature = 0;
            self.state = ComprehensiveControllerState::Idle;
        }
        match self.state {
            ComprehensiveControllerState::Cooling => {
                {
                    let mut i: u8 = 0;
                    while i < COOL_STEP {
                        if self.temperature > 0 {
                            self.temperature -= 1;
                        }
                        i += 1;
                    }
                }
                hal.log_count(steps_to_zero(self.temperature));
                if (self.temperature == 0) & (self.count >= MAX_COUNT) {
                    self.temperature = 0;
                    self.count = 0;
                    self.state = ComprehensiveControllerState::Done;
                } else if (self.temperature == 0) & (!(self.count >= MAX_COUNT)) {
                    self.temperature = 0;
                    self.state = ComprehensiveControllerState::Idle;
                }
            }
            ComprehensiveControllerState::Done => {
                self.state = ComprehensiveControllerState::End;
            }
            ComprehensiveControllerState::Heating => {
                self.temperature = clamp_temp(self.temperature + HEAT_STEP);
                if self.mode == Mode::Auto {
                    hal.log_count(steps_to_limit(self.temperature));
                } else if self.mode == Mode::Manual {
                    hal.log_temp(self.temperature);
                } else {
                    self.mode = Mode::Auto;
                }
                if self.temperature >= MAX_TEMP {
                    self.state = ComprehensiveControllerState::Cooling;
                }
            }
            ComprehensiveControllerState::Idle => {
                let delta: u8 = 1;
                self.temperature += delta;
                hal.log_temp(self.temperature);
                if self.temperature > WARMUP_TEMP {
                    self.count = increment(self.count);
                    self.state = ComprehensiveControllerState::Heating;
                }
            }
            ComprehensiveControllerState::End => {}
            ComprehensiveControllerState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == ComprehensiveControllerState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComprehensiveState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Entry,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'comprehensive'.
pub struct Comprehensive<H: Hal> {
    state: ComprehensiveState,
    entry: ComprehensiveController,
    /// Аппаратный слой. Заменяет `void *userdata` цели `c`.
    hal: H,
}

impl<H: Hal> Comprehensive<H> {
    /// Создаёт модель поверх аппаратного слоя `hal`.
    ///
    /// В отличие от цели `c`, забыть проставить доступ к железу
    /// невозможно: без `hal` модель не конструируется.
    pub fn new(hal: H) -> Self {
        Self {
            state: ComprehensiveState::Init,
            entry: ComprehensiveController::new(),
            hal,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    pub fn init(&mut self) {
        self.state = ComprehensiveState::Init;
        self.entry.init();
    }

    /// Один такт автомата.
    ///
    /// Вход в стартовое состояние такта **не расходует** (контракт
    /// ADR 0033): его тело исполняется в этом же вызове.
    pub fn tick(&mut self) {
        if self.state == ComprehensiveState::Init {
            self.state = ComprehensiveState::Entry;
        }
        match self.state {
            ComprehensiveState::Entry => {
                self.entry.tick(&mut self.hal);
                if self.entry.is_done() {
                    self.state = ComprehensiveState::End;
                }
            }
            ComprehensiveState::End => {}
            ComprehensiveState::Init => {}
        }
    }

    /// Сбрасывает модель. Паритет с `_reset` цели `c`.
    ///
    /// Сброс доходит до вложенных моделей через `init`.
    pub fn reset(&mut self) {
        self.init();
    }

    /// Завершён ли автомат модели.
    pub fn is_done(&self) -> bool {
        self.state == ComprehensiveState::End
    }

}

