// Порождено компилятором Takt (taktc) — цель: Rust (профиль no_std).
// Не редактировать вручную: файл перезаписывается при каждой генерации.
//
// Модуль не обращается к std и подключается как `mod`:
//
//     #[path = "pid_law.rs"]
//     pub mod pid_law;
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
enum PidLawPidState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Control,
    Done,
    Settled,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'Pid'.
pub struct PidLawPid {
    deriv: f64,
    eps: f64,
    err: f64,
    err_prev: f64,
    i_acc: f64,
    imax: f64,
    kd: f64,
    ki: f64,
    kp: f64,
    neg_imax: f64,
    state: PidLawPidState,
}

impl PidLawPid {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            deriv: 0.0,
            eps: 0.5,
            err: 0.0,
            err_prev: 0.0,
            i_acc: 0.0,
            imax: 32.0,
            kd: 0.25,
            ki: 0.0625,
            kp: 0.5,
            neg_imax: -32.0,
            state: PidLawPidState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.deriv = 0.0;
        self.eps = 0.5;
        self.err = 0.0;
        self.err_prev = 0.0;
        self.i_acc = 0.0;
        self.imax = 32.0;
        self.kd = 0.25;
        self.ki = 0.0625;
        self.kp = 0.5;
        self.neg_imax = -32.0;
        self.state = PidLawPidState::Init;
    }

    /// Один такт автомата.
    fn tick<H: Hal>(&mut self, shared: &mut PidLawShared, hal: &mut H) {
        if self.state == PidLawPidState::Init {
            self.neg_imax = 0.0 - self.imax;
            self.state = PidLawPidState::Control;
        }
        match self.state {
            PidLawPidState::Control => {
                self.err = shared.target - shared.meas;
                self.i_acc += self.err;
                if self.i_acc > self.imax {
                    self.i_acc = self.imax;
                }
                if self.i_acc < self.neg_imax {
                    self.i_acc = self.neg_imax;
                }
                self.deriv = self.err - self.err_prev;
                shared.ctrl = ((self.kp * self.err) + (self.ki * self.i_acc)) + (self.kd * self.deriv);
                self.err_prev = self.err;
                if self.err < self.eps {
                    shared.ctrl = 0.0;
                    hal.write_bit(OutBitPort::Ready, true);
                    self.state = PidLawPidState::Settled;
                }
            }
            PidLawPidState::Done => {
                self.state = PidLawPidState::End;
            }
            PidLawPidState::Settled => {
                self.state = PidLawPidState::Done;
            }
            PidLawPidState::End => {}
            PidLawPidState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == PidLawPidState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PidLawState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Main,
    /// Автомат завершён (`is_done`).
    End,
}

/// Общие переменные модели 'pid_law', разделяемые под-моделями.
struct PidLawShared {
    ctrl: f64,
    meas: f64,
    target: f64,
}

/// Модель 'pid_law'.
pub struct PidLaw<H: Hal> {
    /// Общие с под-моделями переменные (фича 0059).
    shared: PidLawShared,
    state: PidLawState,
    main: PidLawPid,
    /// Аппаратный слой. Заменяет `void *userdata` цели `c`.
    hal: H,
}

impl<H: Hal> PidLaw<H> {
    /// Создаёт модель поверх аппаратного слоя `hal`.
    ///
    /// В отличие от цели `c`, забыть проставить доступ к железу
    /// невозможно: без `hal` модель не конструируется.
    pub fn new(hal: H) -> Self {
        Self {
            shared: PidLawShared {
                ctrl: 0.0,
                meas: 0.0,
                target: 40.0,
            },
            state: PidLawState::Init,
            main: PidLawPid::new(),
            hal,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    pub fn init(&mut self) {
        self.shared.ctrl = 0.0;
        self.shared.meas = 0.0;
        self.shared.target = 40.0;
        self.state = PidLawState::Init;
        self.main.init();
    }

    /// Один такт автомата.
    ///
    /// Вход в стартовое состояние такта **не расходует** (контракт
    /// ADR 0033): его тело исполняется в этом же вызове.
    pub fn tick(&mut self) {
        if self.state == PidLawState::Init {
            self.state = PidLawState::Main;
        }
        match self.state {
            PidLawState::Main => {
                self.main.tick(&mut self.shared, &mut self.hal);
                if self.main.is_done() {
                    self.state = PidLawState::End;
                }
            }
            PidLawState::End => {}
            PidLawState::Init => {}
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
        self.state == PidLawState::End
    }

}

