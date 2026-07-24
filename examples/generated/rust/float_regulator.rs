// Порождено компилятором Takt (taktc) — цель: Rust (профиль no_std).
// Не редактировать вручную: файл перезаписывается при каждой генерации.
//
// Модуль не обращается к std и подключается как `mod`:
//
//     #[path = "float_regulator.rs"]
//     pub mod float_regulator;
//
// Атрибута #![no_std] здесь нет намеренно: он допустим только в корне
// крейта, а no_std — свойство крейта, не модуля. Совместимость с no_std
// проверяется гейтом (scripts/precheck.sh).

#![forbid(unsafe_code)]

/// Порт ввода-вывода модели. Реализация — за трейтом [`Hal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutBitPort {
    Ready,
}

/// Аппаратный слой модели.
///
/// Заменяет пару указателей на функции и `void *userdata` цели `c`:
/// состояние слоя живёт в самом типе-реализации, поэтому привести
/// его не к тому типу или забыть проставить колбэк невозможно.
pub trait Hal {
    /// Пишет `value` в выходной порт `port`.
    fn write_bit(&mut self, port: OutBitPort, value: bool);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FloatRegulatorFloatRegulatorState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Adjust,
    Done,
    Settled,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'FloatRegulator'.
pub struct FloatRegulatorFloatRegulator {
    half: f64,
    near: f64,
    setpoint: f64,
    value: f64,
    state: FloatRegulatorFloatRegulatorState,
}

impl FloatRegulatorFloatRegulator {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            half: 0.5,
            near: 9.5,
            setpoint: 10.0,
            value: 0.0,
            state: FloatRegulatorFloatRegulatorState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.half = 0.5;
        self.near = 9.5;
        self.setpoint = 10.0;
        self.value = 0.0;
        self.state = FloatRegulatorFloatRegulatorState::Init;
    }

    /// Один такт автомата.
    fn tick<H: Hal>(&mut self, hal: &mut H) {
        if self.state == FloatRegulatorFloatRegulatorState::Init {
            self.state = FloatRegulatorFloatRegulatorState::Adjust;
        }
        match self.state {
            FloatRegulatorFloatRegulatorState::Adjust => {
                self.value += (self.setpoint - self.value) * self.half;
                if self.value >= self.near {
                    self.state = FloatRegulatorFloatRegulatorState::Settled;
                }
            }
            FloatRegulatorFloatRegulatorState::Done => {
                hal.write_bit(OutBitPort::Ready, true);
                self.state = FloatRegulatorFloatRegulatorState::End;
            }
            FloatRegulatorFloatRegulatorState::Settled => {
                self.value = self.setpoint;
                self.state = FloatRegulatorFloatRegulatorState::Done;
            }
            FloatRegulatorFloatRegulatorState::End => {}
            FloatRegulatorFloatRegulatorState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == FloatRegulatorFloatRegulatorState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FloatRegulatorState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Main,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'float_regulator'.
pub struct FloatRegulator<H: Hal> {
    state: FloatRegulatorState,
    main: FloatRegulatorFloatRegulator,
    /// Аппаратный слой. Заменяет `void *userdata` цели `c`.
    hal: H,
}

impl<H: Hal> FloatRegulator<H> {
    /// Создаёт модель поверх аппаратного слоя `hal`.
    ///
    /// В отличие от цели `c`, забыть проставить доступ к железу
    /// невозможно: без `hal` модель не конструируется.
    pub fn new(hal: H) -> Self {
        Self {
            state: FloatRegulatorState::Init,
            main: FloatRegulatorFloatRegulator::new(),
            hal,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    pub fn init(&mut self) {
        self.state = FloatRegulatorState::Init;
        self.main.init();
    }

    /// Один такт автомата.
    ///
    /// Вход в стартовое состояние такта **не расходует** (контракт
    /// ADR 0033): его тело исполняется в этом же вызове.
    pub fn tick(&mut self) {
        if self.state == FloatRegulatorState::Init {
            self.state = FloatRegulatorState::Main;
        }
        match self.state {
            FloatRegulatorState::Main => {
                self.main.tick(&mut self.hal);
                if self.main.is_done() {
                    self.state = FloatRegulatorState::End;
                }
            }
            FloatRegulatorState::End => {}
            FloatRegulatorState::Init => {}
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
        self.state == FloatRegulatorState::End
    }

}

