// Порождено компилятором Takt (taktc) — цель: Rust (профиль no_std).
// Не редактировать вручную: файл перезаписывается при каждой генерации.
//
// Модуль не обращается к std и подключается как `mod`:
//
//     #[path = "regulator.rs"]
//     pub mod regulator;
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
enum RegulatorRegulatorState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Adjust,
    Done,
    Settled,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'Regulator'.
pub struct RegulatorRegulator {
    half: i16,
    near: i16,
    setpoint: i16,
    value: i16,
    state: RegulatorRegulatorState,
}

impl RegulatorRegulator {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            half: 128,
            near: 2432,
            setpoint: 2560,
            value: 0,
            state: RegulatorRegulatorState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.half = 128;
        self.near = 2432;
        self.setpoint = 2560;
        self.value = 0;
        self.state = RegulatorRegulatorState::Init;
    }

    /// Один такт автомата.
    fn tick<H: Hal>(&mut self, hal: &mut H) {
        if self.state == RegulatorRegulatorState::Init {
            self.state = RegulatorRegulatorState::Adjust;
        }
        match self.state {
            RegulatorRegulatorState::Adjust => {
                self.value = (self.value as i64 + (((((self.setpoint as i64 - self.value as i64) as i16) as i128 * self.half as i128) >> 8) as i16) as i64) as i16;
                if self.value >= self.near {
                    self.state = RegulatorRegulatorState::Settled;
                }
            }
            RegulatorRegulatorState::Done => {
                hal.write_bit(OutBitPort::Ready, true);
                self.state = RegulatorRegulatorState::End;
            }
            RegulatorRegulatorState::Settled => {
                self.value = self.setpoint;
                self.state = RegulatorRegulatorState::Done;
            }
            RegulatorRegulatorState::End => {}
            RegulatorRegulatorState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == RegulatorRegulatorState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RegulatorState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Main,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'regulator'.
pub struct Regulator<H: Hal> {
    state: RegulatorState,
    main: RegulatorRegulator,
    /// Аппаратный слой. Заменяет `void *userdata` цели `c`.
    hal: H,
}

impl<H: Hal> Regulator<H> {
    /// Создаёт модель поверх аппаратного слоя `hal`.
    ///
    /// В отличие от цели `c`, забыть проставить доступ к железу
    /// невозможно: без `hal` модель не конструируется.
    pub fn new(hal: H) -> Self {
        Self {
            state: RegulatorState::Init,
            main: RegulatorRegulator::new(),
            hal,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    pub fn init(&mut self) {
        self.state = RegulatorState::Init;
        self.main.init();
    }

    /// Один такт автомата.
    ///
    /// Вход в стартовое состояние такта **не расходует** (контракт
    /// ADR 0033): его тело исполняется в этом же вызове.
    pub fn tick(&mut self) {
        if self.state == RegulatorState::Init {
            self.state = RegulatorState::Main;
        }
        match self.state {
            RegulatorState::Main => {
                self.main.tick(&mut self.hal);
                if self.main.is_done() {
                    self.state = RegulatorState::End;
                }
            }
            RegulatorState::End => {}
            RegulatorState::Init => {}
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
        self.state == RegulatorState::End
    }

}

