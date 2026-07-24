// Порождено компилятором Takt (taktc) — цель: Rust (профиль no_std).
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
    ctrl: f64,
    deriv: f64,
    eps: f64,
    err: f64,
    err_prev: f64,
    i_acc: f64,
    imax: f64,
    kd: f64,
    ki: f64,
    kp: f64,
    kplant: f64,
    meas: f64,
    neg_imax: f64,
    target: f64,
    state: PidRegulatorPidState,
}

impl PidRegulatorPid {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            ctrl: 0.0,
            deriv: 0.0,
            eps: 0.125,
            err: 0.0,
            err_prev: 0.0,
            i_acc: 0.0,
            imax: 32.0,
            kd: 0.25,
            ki: 0.0625,
            kp: 0.5,
            kplant: 0.5,
            meas: 0.0,
            neg_imax: -32.0,
            target: 8.0,
            state: PidRegulatorPidState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.ctrl = 0.0;
        self.deriv = 0.0;
        self.eps = 0.125;
        self.err = 0.0;
        self.err_prev = 0.0;
        self.i_acc = 0.0;
        self.imax = 32.0;
        self.kd = 0.25;
        self.ki = 0.0625;
        self.kp = 0.5;
        self.kplant = 0.5;
        self.meas = 0.0;
        self.neg_imax = -32.0;
        self.target = 8.0;
        self.state = PidRegulatorPidState::Init;
    }

    /// Один такт автомата.
    fn tick<H: Hal>(&mut self, hal: &mut H) {
        if self.state == PidRegulatorPidState::Init {
            self.state = PidRegulatorPidState::Control;
        }
        match self.state {
            PidRegulatorPidState::Control => {
                self.err = self.target - self.meas;
                self.i_acc += self.err;
                if self.i_acc > self.imax {
                    self.i_acc = self.imax;
                }
                if self.i_acc < self.neg_imax {
                    self.i_acc = self.neg_imax;
                }
                self.deriv = self.err - self.err_prev;
                self.ctrl = ((self.kp * self.err) + (self.ki * self.i_acc)) + (self.kd * self.deriv);
                self.meas += self.kplant * self.ctrl;
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

