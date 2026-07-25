// Порождено компилятором Takt (taktc) — цель: Rust (профиль no_std).
// Не редактировать вручную: файл перезаписывается при каждой генерации.
//
// Модуль не обращается к std и подключается как `mod`:
//
//     #[path = "lift.rs"]
//     pub mod lift;
//
// Атрибута #![no_std] здесь нет намеренно: он допустим только в корне
// крейта, а no_std — свойство крейта, не модуля. Совместимость с no_std
// проверяется гейтом (scripts/precheck.sh).

#![forbid(unsafe_code)]

const DWELL_TICKS: u8 = 3;

/// Порт ввода-вывода модели. Реализация — за трейтом [`Hal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InU8Port {
    AtFloor,
    Call,
}

/// Порт ввода-вывода модели. Реализация — за трейтом [`Hal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutBitPort {
    Brake,
    DoorsOpen,
    MotorDown,
    MotorUp,
}

/// Порт ввода-вывода модели. Реализация — за трейтом [`Hal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutU8Port {
    Display,
}

/// Аппаратный слой модели.
///
/// Заменяет пару указателей на функции и `void *userdata` цели `c`:
/// состояние слоя живёт в самом типе-реализации, поэтому привести
/// его не к тому типу или забыть проставить колбэк невозможно.
pub trait Hal {
    /// Читает входной порт `port`.
    fn read_u8(&mut self, port: InU8Port) -> u8;
    /// Пишет `value` в выходной порт `port`.
    fn write_bit(&mut self, port: OutBitPort, value: bool);
    /// Пишет `value` в выходной порт `port`.
    fn write_u8(&mut self, port: OutU8Port, value: u8);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiftState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Boarding,
    GoingDown,
    GoingUp,
    Leaving,
    Stopping,
    Waiting,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'lift'.
pub struct Lift<H: Hal> {
    doors: bool,
    dwell: u8,
    moving: bool,
    state: LiftState,
    /// Аппаратный слой. Заменяет `void *userdata` цели `c`.
    hal: H,
}

impl<H: Hal> Lift<H> {
    /// Создаёт модель поверх аппаратного слоя `hal`.
    ///
    /// В отличие от цели `c`, забыть проставить доступ к железу
    /// невозможно: без `hal` модель не конструируется.
    pub fn new(hal: H) -> Self {
        Self {
            doors: false,
            dwell: 0,
            moving: false,
            state: LiftState::Init,
            hal,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    pub fn init(&mut self) {
        self.doors = false;
        self.dwell = 0;
        self.moving = false;
        self.state = LiftState::Init;
    }

    /// Один такт автомата.
    ///
    /// Вход в стартовое состояние такта **не расходует** (контракт
    /// ADR 0033): его тело исполняется в этом же вызове.
    pub fn tick(&mut self) {
        assert!((self.moving == 0) | (self.doors == 0), "нарушен инвариант 'SafeMove'");
        if self.state == LiftState::Init {
            self.moving = false;
            self.hal.write_bit(OutBitPort::MotorUp, false);
            self.hal.write_bit(OutBitPort::MotorDown, false);
            self.hal.write_bit(OutBitPort::Brake, true);
            self.doors = false;
            self.hal.write_bit(OutBitPort::DoorsOpen, false);
            self.state = LiftState::Waiting;
        }
        match self.state {
            LiftState::Boarding => {
                self.dwell += 1;
                if self.dwell >= DWELL_TICKS {
                    self.doors = false;
                    self.hal.write_bit(OutBitPort::DoorsOpen, false);
                    self.state = LiftState::Leaving;
                }
            }
            LiftState::GoingDown => {
                self.hal.write_u8(OutU8Port::Display, self.hal.read_u8(InU8Port::AtFloor));
                if self.hal.read_u8(InU8Port::AtFloor) <= self.hal.read_u8(InU8Port::Call) {
                    self.moving = false;
                    self.hal.write_bit(OutBitPort::MotorUp, false);
                    self.hal.write_bit(OutBitPort::MotorDown, false);
                    self.hal.write_bit(OutBitPort::Brake, true);
                    self.state = LiftState::Stopping;
                }
            }
            LiftState::GoingUp => {
                self.hal.write_u8(OutU8Port::Display, self.hal.read_u8(InU8Port::AtFloor));
                if self.hal.read_u8(InU8Port::AtFloor) >= self.hal.read_u8(InU8Port::Call) {
                    self.moving = false;
                    self.hal.write_bit(OutBitPort::MotorUp, false);
                    self.hal.write_bit(OutBitPort::MotorDown, false);
                    self.hal.write_bit(OutBitPort::Brake, true);
                    self.state = LiftState::Stopping;
                }
            }
            LiftState::Leaving => {
                self.moving = false;
                self.hal.write_bit(OutBitPort::MotorUp, false);
                self.hal.write_bit(OutBitPort::MotorDown, false);
                self.hal.write_bit(OutBitPort::Brake, true);
                self.doors = false;
                self.hal.write_bit(OutBitPort::DoorsOpen, false);
                self.state = LiftState::Waiting;
            }
            LiftState::Stopping => {
                self.doors = true;
                self.hal.write_bit(OutBitPort::DoorsOpen, true);
                self.dwell = 0;
                self.state = LiftState::Boarding;
            }
            LiftState::Waiting => {
                self.hal.write_u8(OutU8Port::Display, self.hal.read_u8(InU8Port::AtFloor));
                if self.hal.read_u8(InU8Port::Call) == self.hal.read_u8(InU8Port::AtFloor) {
                    self.doors = true;
                    self.hal.write_bit(OutBitPort::DoorsOpen, true);
                    self.dwell = 0;
                    self.state = LiftState::Boarding;
                } else if self.hal.read_u8(InU8Port::Call) > self.hal.read_u8(InU8Port::AtFloor) {
                    self.moving = true;
                    self.hal.write_bit(OutBitPort::Brake, false);
                    self.hal.write_bit(OutBitPort::MotorUp, true);
                    self.state = LiftState::GoingUp;
                } else if (self.hal.read_u8(InU8Port::Call) > 0) & (self.hal.read_u8(InU8Port::Call) < self.hal.read_u8(InU8Port::AtFloor)) {
                    self.moving = true;
                    self.hal.write_bit(OutBitPort::Brake, false);
                    self.hal.write_bit(OutBitPort::MotorDown, true);
                    self.state = LiftState::GoingDown;
                }
            }
            LiftState::End => {}
            LiftState::Init => {}
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
        self.state == LiftState::End
    }

}

