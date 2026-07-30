// Порождено компилятором Takt (taktc) — цель: Rust (профиль no_std).
// Не редактировать вручную: файл перезаписывается при каждой генерации.
//
// Модуль не обращается к std и подключается как `mod`:
//
//     #[path = "pid_heater.rs"]
//     pub mod pid_heater;
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

/// Порт ввода-вывода модели. Реализация — за трейтом [`Hal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutF64Port {
    Temperature,
}

/// Аппаратный слой модели.
///
/// Заменяет пару указателей на функции и `void *userdata` цели `c`:
/// состояние слоя живёт в самом типе-реализации, поэтому привести
/// его не к тому типу или забыть проставить колбэк невозможно.
pub trait Hal {
    /// Пишет `value` в выходной порт `port`.
    fn write_bit(&mut self, port: OutBitPort, value: bool);
    /// Пишет `value` в выходной порт `port`.
    fn write_f64(&mut self, port: OutF64Port, value: f64);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PidHeaterHeaterState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Done,
    Heating,
    Holding,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'Heater'.
pub struct PidHeaterHeater {
    release: f64,
    setpoint: f64,
    state: PidHeaterHeaterState,
}

impl PidHeaterHeater {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            release: 38.0,
            setpoint: 40.0,
            state: PidHeaterHeaterState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.release = 38.0;
        self.setpoint = 40.0;
        self.state = PidHeaterHeaterState::Init;
    }

    /// Один такт автомата.
    fn tick<H: Hal>(&mut self, shared: &mut PidHeaterShared, hal: &mut H) {
        if self.state == PidHeaterHeaterState::Init {
            shared.target = self.setpoint;
            self.state = PidHeaterHeaterState::Heating;
        }
        match self.state {
            PidHeaterHeaterState::Done => {
                self.state = PidHeaterHeaterState::End;
            }
            PidHeaterHeaterState::Heating => {
                shared.meas = (shared.meas + (shared.gain * shared.ctrl)) - (shared.loss * (shared.meas - shared.ambient));
                hal.write_f64(OutF64Port::Temperature, shared.meas);
                if shared.ctrl <= 0.0 {
                    self.state = PidHeaterHeaterState::Holding;
                }
            }
            PidHeaterHeaterState::Holding => {
                shared.meas -= shared.loss * (shared.meas - shared.ambient);
                hal.write_f64(OutF64Port::Temperature, shared.meas);
                if shared.meas <= self.release {
                    self.state = PidHeaterHeaterState::Done;
                }
            }
            PidHeaterHeaterState::End => {}
            PidHeaterHeaterState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == PidHeaterHeaterState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PidHeaterPidState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Control,
    Done,
    Settled,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'Pid'.
pub struct PidHeaterPid {
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
    state: PidHeaterPidState,
}

impl PidHeaterPid {
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
            state: PidHeaterPidState::Init,
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
        self.state = PidHeaterPidState::Init;
    }

    /// Один такт автомата.
    fn tick<H: Hal>(&mut self, shared: &mut PidHeaterShared, hal: &mut H) {
        if self.state == PidHeaterPidState::Init {
            self.neg_imax = 0.0 - self.imax;
            self.state = PidHeaterPidState::Control;
        }
        match self.state {
            PidHeaterPidState::Control => {
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
                    self.state = PidHeaterPidState::Settled;
                }
            }
            PidHeaterPidState::Done => {
                self.state = PidHeaterPidState::End;
            }
            PidHeaterPidState::Settled => {
                self.state = PidHeaterPidState::Done;
            }
            PidHeaterPidState::End => {}
            PidHeaterPidState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == PidHeaterPidState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PidHeaterState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Finished,
    PidHeater,
    /// Автомат завершён (`is_done`).
    End,
}

/// Шаг последовательной композиции состояния 'PidHeater'.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PidHeaterPidHeaterSeq {
    Group0,
    Group1,
}

/// Общие переменные модели 'pid_heater', разделяемые под-моделями.
struct PidHeaterShared {
    ambient: f64,
    ctrl: f64,
    gain: f64,
    loss: f64,
    meas: f64,
    target: f64,
}

/// Модель 'pid_heater'.
pub struct PidHeater<H: Hal> {
    /// Общие с под-моделями переменные (фича 0059).
    shared: PidHeaterShared,
    state: PidHeaterState,
    /// Текущий шаг последовательной композиции состояния 'PidHeater'.
    pid_heater_seq: PidHeaterPidHeaterSeq,
    pid_heater_group0_pid0: PidHeaterPid,
    pid_heater_group0_heater1: PidHeaterHeater,
    pid_heater_group1_pid0: PidHeaterPid,
    pid_heater_group1_heater1: PidHeaterHeater,
    /// Аппаратный слой. Заменяет `void *userdata` цели `c`.
    hal: H,
}

impl<H: Hal> PidHeater<H> {
    /// Создаёт модель поверх аппаратного слоя `hal`.
    ///
    /// В отличие от цели `c`, забыть проставить доступ к железу
    /// невозможно: без `hal` модель не конструируется.
    pub fn new(hal: H) -> Self {
        Self {
            shared: PidHeaterShared {
                ambient: 18.0,
                ctrl: 0.0,
                gain: 0.5,
                loss: 0.05,
                meas: 0.0,
                target: 40.0,
            },
            state: PidHeaterState::Init,
            pid_heater_seq: PidHeaterPidHeaterSeq::Group0,
            pid_heater_group0_pid0: PidHeaterPid::new(),
            pid_heater_group0_heater1: PidHeaterHeater::new(),
            pid_heater_group1_pid0: {
                let mut instance = PidHeaterPid::new();
                instance.kp = 0.25;
                instance.ki = 0.125;
                instance
            },
            pid_heater_group1_heater1: {
                let mut instance = PidHeaterHeater::new();
                instance.setpoint = 55.0;
                instance.release = 52.0;
                instance
            },
            hal,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    pub fn init(&mut self) {
        self.shared.ambient = 18.0;
        self.shared.ctrl = 0.0;
        self.shared.gain = 0.5;
        self.shared.loss = 0.05;
        self.shared.meas = 0.0;
        self.shared.target = 40.0;
        self.state = PidHeaterState::Init;
        self.pid_heater_seq = PidHeaterPidHeaterSeq::Group0;
        self.pid_heater_group0_pid0.init();
        self.pid_heater_group0_heater1.init();
        self.pid_heater_group1_pid0.init();
        self.pid_heater_group1_pid0.kp = 0.25;
        self.pid_heater_group1_pid0.ki = 0.125;
        self.pid_heater_group1_heater1.init();
        self.pid_heater_group1_heater1.setpoint = 55.0;
        self.pid_heater_group1_heater1.release = 52.0;
    }

    /// Один такт автомата.
    ///
    /// Вход в стартовое состояние такта **не расходует** (контракт
    /// ADR 0033): его тело исполняется в этом же вызове.
    pub fn tick(&mut self) {
        if self.state == PidHeaterState::Init {
            self.state = PidHeaterState::PidHeater;
        }
        match self.state {
            PidHeaterState::Finished => {
                self.state = PidHeaterState::End;
            }
            PidHeaterState::PidHeater => {
                if self.pid_heater_seq == PidHeaterPidHeaterSeq::Group0 {
                    self.pid_heater_group0_pid0.tick(&mut self.shared, &mut self.hal);
                    self.pid_heater_group0_heater1.tick(&mut self.shared, &mut self.hal);
                    if self.pid_heater_group0_pid0.is_done() && self.pid_heater_group0_heater1.is_done() {
                        self.pid_heater_group1_pid0.init();
                        self.pid_heater_group1_heater1.init();
                        self.pid_heater_seq = PidHeaterPidHeaterSeq::Group1;
                    }
                } else if self.pid_heater_seq == PidHeaterPidHeaterSeq::Group1 {
                    self.pid_heater_group1_pid0.tick(&mut self.shared, &mut self.hal);
                    self.pid_heater_group1_heater1.tick(&mut self.shared, &mut self.hal);
                    if self.pid_heater_group1_pid0.is_done() && self.pid_heater_group1_heater1.is_done() {
                        self.state = PidHeaterState::Finished;
                    }
                }
            }
            PidHeaterState::End => {}
            PidHeaterState::Init => {}
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
        self.state == PidHeaterState::End
    }

}

