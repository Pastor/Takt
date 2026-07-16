// Порождено компилятором Lam (lamc) — цель: Rust (профиль no_std).
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

const MAX_COUNT: u8 = 10;
const MAX_TEMP: u8 = 100;

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

/// Функция 'increment' модели.
fn increment(n: u8) -> u8 {
    n + 1
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
            self.count = 0;
            self.temperature = 0;
            self.state = ComprehensiveControllerState::Idle;
        }
        match self.state {
            ComprehensiveControllerState::Cooling => {
                {
                    let mut i: u8 = 0;
                    while i < 3 {
                        if self.temperature > 0 {
                            self.temperature -= 1;
                        }
                        i += 1;
                    }
                }
                hal.log_count(self.count);
                if self.temperature <= 0 {
                    self.temperature = 0;
                    self.count = 0;
                    self.state = ComprehensiveControllerState::Done;
                } else if self.temperature > 0 {
                    self.count = 0;
                    self.temperature = 0;
                    self.state = ComprehensiveControllerState::Idle;
                }
            }
            ComprehensiveControllerState::Done => {
                self.state = ComprehensiveControllerState::End;
            }
            ComprehensiveControllerState::Heating => {
                while self.count < 3 {
                    self.count += 1;
                }
                if self.mode == Mode::Auto {
                    hal.log_temp(self.temperature);
                } else if self.mode == Mode::Manual {
                    hal.log_count(self.count);
                } else {
                    self.count = 0;
                }
                if self.temperature > MAX_TEMP {
                    self.state = ComprehensiveControllerState::Cooling;
                } else if self.count >= MAX_COUNT {
                    self.count = 0;
                    self.temperature = 0;
                    self.state = ComprehensiveControllerState::Idle;
                }
            }
            ComprehensiveControllerState::Idle => {
                let delta: u8 = 1;
                self.temperature += delta;
                hal.log_temp(self.temperature);
                if self.temperature > 10 {
                    self.count = increment(self.count);
                    let boost: u8 = 5;
                    self.temperature += boost;
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

