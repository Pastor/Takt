// Порождено компилятором Lam (lamc) — цель: Rust (профиль no_std).
// Не редактировать вручную: файл перезаписывается при каждой генерации.
//
// Модуль не обращается к std и подключается как `mod`:
//
//     #[path = "pid_regulator.rs"]
//     pub mod pid_regulator;
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
enum PidRegulatorPidState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Control,
    Done,
    Settled,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'Pid'.
pub struct PidRegulatorPid {
    ctrl: i16,
    deriv: i16,
    eps: i16,
    err: i16,
    err_prev: i16,
    i_acc: i16,
    imax: i16,
    kd: i16,
    ki: i16,
    kp: i16,
    kplant: i16,
    meas: i16,
    neg_imax: i16,
    target: i16,
    state: PidRegulatorPidState,
}

impl PidRegulatorPid {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            ctrl: 0,
            deriv: 0,
            eps: 32,
            err: 0,
            err_prev: 0,
            i_acc: 0,
            imax: 8192,
            kd: 64,
            ki: 16,
            kp: 128,
            kplant: 128,
            meas: 0,
            neg_imax: -8192,
            target: 2048,
            state: PidRegulatorPidState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.ctrl = 0;
        self.deriv = 0;
        self.eps = 32;
        self.err = 0;
        self.err_prev = 0;
        self.i_acc = 0;
        self.imax = 8192;
        self.kd = 64;
        self.ki = 16;
        self.kp = 128;
        self.kplant = 128;
        self.meas = 0;
        self.neg_imax = -8192;
        self.target = 2048;
        self.state = PidRegulatorPidState::Init;
    }

    /// Один такт автомата.
    fn tick<H: Hal>(&mut self, hal: &mut H) {
        if self.state == PidRegulatorPidState::Init {
            self.state = PidRegulatorPidState::Control;
        }
        match self.state {
            PidRegulatorPidState::Control => {
                self.err = (self.target as i64 - self.meas as i64) as i16;
                self.i_acc = (self.i_acc as i64 + self.err as i64) as i16;
                if self.i_acc > self.imax {
                    self.i_acc = self.imax;
                }
                if self.i_acc < self.neg_imax {
                    self.i_acc = self.neg_imax;
                }
                self.deriv = (self.err as i64 - self.err_prev as i64) as i16;
                self.ctrl = ((((((self.kp as i128 * self.err as i128) >> 8) as i16) as i64 + (((self.ki as i128 * self.i_acc as i128) >> 8) as i16) as i64) as i16) as i64 + (((self.kd as i128 * self.deriv as i128) >> 8) as i16) as i64) as i16;
                self.meas = (self.meas as i64 + (((self.kplant as i128 * self.ctrl as i128) >> 8) as i16) as i64) as i16;
                self.err_prev = self.err;
                if self.err < self.eps {
                    self.state = PidRegulatorPidState::Settled;
                }
            }
            PidRegulatorPidState::Done => {
                hal.write_bit(OutBitPort::Ready, true);
                self.state = PidRegulatorPidState::End;
            }
            PidRegulatorPidState::Settled => {
                self.meas = self.target;
                self.state = PidRegulatorPidState::Done;
            }
            PidRegulatorPidState::End => {}
            PidRegulatorPidState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == PidRegulatorPidState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PidRegulatorState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Main,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'pid_regulator'.
pub struct PidRegulator<H: Hal> {
    state: PidRegulatorState,
    main: PidRegulatorPid,
    /// Аппаратный слой. Заменяет `void *userdata` цели `c`.
    hal: H,
}

impl<H: Hal> PidRegulator<H> {
    /// Создаёт модель поверх аппаратного слоя `hal`.
    ///
    /// В отличие от цели `c`, забыть проставить доступ к железу
    /// невозможно: без `hal` модель не конструируется.
    pub fn new(hal: H) -> Self {
        Self {
            state: PidRegulatorState::Init,
            main: PidRegulatorPid::new(),
            hal,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    pub fn init(&mut self) {
        self.state = PidRegulatorState::Init;
        self.main.init();
    }

    /// Один такт автомата.
    ///
    /// Вход в стартовое состояние такта **не расходует** (контракт
    /// ADR 0033): его тело исполняется в этом же вызове.
    pub fn tick(&mut self) {
        if self.state == PidRegulatorState::Init {
            self.state = PidRegulatorState::Main;
        }
        match self.state {
            PidRegulatorState::Main => {
                self.main.tick(&mut self.hal);
                if self.main.is_done() {
                    self.state = PidRegulatorState::End;
                }
            }
            PidRegulatorState::End => {}
            PidRegulatorState::Init => {}
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
        self.state == PidRegulatorState::End
    }

}

