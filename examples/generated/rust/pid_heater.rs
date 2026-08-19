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

/// Структура 'PidState' модели.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PidState {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
    pub ts: f64,
    pub out_min: f64,
    pub out_max: f64,
    pub i_acc: f64,
    pub err_prev: f64,
    pub output: f64,
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
    fn write_f64(&mut self, port: OutF64Port, value: f64);
}

/// Функция 'pid_compute' модели.
fn pid_compute(p: PidState, sp: f64, pv: f64) -> PidState {
    let mut r: PidState = p;
    let err: f64 = sp - pv;
    let prop: f64 = p.kp * err;
    let i_new: f64 = p.i_acc + ((p.ki * err) * p.ts);
    let deriv: f64 = (err - p.err_prev) / p.ts;
    let raw: f64 = (prop + i_new) + (p.kd * deriv);
    if raw > p.out_max {
        r.output = p.out_max;
        if err <= 0.0 {
            r.i_acc = i_new;
        }
    } else {
        if raw < p.out_min {
            r.output = p.out_min;
            if err >= 0.0 {
                r.i_acc = i_new;
            }
        } else {
            r.output = raw;
            r.i_acc = i_new;
        }
    }
    r.err_prev = err;
    r
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
    err: f64,
    loop_pid: PidState,
    release: f64,
    setpoint: f64,
    state: PidHeaterHeaterState,
}

impl PidHeaterHeater {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            err: 0.0,
            loop_pid: PidState { kp: 0.5, ki: 0.0625, kd: 0.25, ts: 1.0, out_min: 0.0, out_max: 32.0, i_acc: 0.0, err_prev: 0.0, output: 0.0 },
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
        self.err = 0.0;
        self.loop_pid = PidState { kp: 0.5, ki: 0.0625, kd: 0.25, ts: 1.0, out_min: 0.0, out_max: 32.0, i_acc: 0.0, err_prev: 0.0, output: 0.0 };
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
                self.loop_pid = pid_compute(self.loop_pid, shared.target, shared.meas);
                shared.ctrl = self.loop_pid.output;
                self.err = shared.target - shared.meas;
                shared.meas = (shared.meas + (shared.gain * shared.ctrl)) - (shared.loss * (shared.meas - shared.ambient));
                hal.write_f64(OutF64Port::Temperature, shared.meas);
                if self.err <= 0.0 {
                    shared.ctrl = 0.0;
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
    Heater0,
    Heater1,
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
    pid_heater_heater0: PidHeaterHeater,
    pid_heater_heater1: PidHeaterHeater,
    /// Аппаратный слой. Заменяет `void *userdata` цели `c`.
    hal: H,
}

impl<H: Hal> PidHeater<H> {
    /// Создаёт модель поверх аппаратного слоя `hal`.
    ///
    /// В отличие от цели `c`, забыть проставить доступ к железу
    /// невозможно: без `hal` модель не конструируется.
    pub fn new(hal: H) -> Self {
        let mut this = Self {
            shared: PidHeaterShared {
                ambient: 18.0,
                ctrl: 0.0,
                gain: 0.5,
                loss: 0.05,
                meas: 25.0,
                target: 80.0,
            },
            state: PidHeaterState::Init,
            pid_heater_seq: PidHeaterPidHeaterSeq::Heater0,
            pid_heater_heater0: PidHeaterHeater::new(),
            pid_heater_heater1: {
                let mut instance = PidHeaterHeater::new();
                instance.setpoint = 55.0;
                instance.release = 52.0;
                instance
            },
            hal,
        };
        this.hal.write_f64(OutF64Port::Temperature, 0.0);
        this
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
        self.shared.meas = 25.0;
        self.shared.target = 80.0;
        self.state = PidHeaterState::Init;
        self.pid_heater_seq = PidHeaterPidHeaterSeq::Heater0;
        self.pid_heater_heater0.init();
        self.pid_heater_heater1.init();
        self.pid_heater_heater1.setpoint = 55.0;
        self.pid_heater_heater1.release = 52.0;
        self.hal.write_f64(OutF64Port::Temperature, 0.0);
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
                if self.pid_heater_seq == PidHeaterPidHeaterSeq::Heater0 {
                    self.pid_heater_heater0.tick(&mut self.shared, &mut self.hal);
                    if self.pid_heater_heater0.is_done() {
                        self.pid_heater_heater1.init();
                        self.pid_heater_seq = PidHeaterPidHeaterSeq::Heater1;
                    }
                } else if self.pid_heater_seq == PidHeaterPidHeaterSeq::Heater1 {
                    self.pid_heater_heater1.tick(&mut self.shared, &mut self.hal);
                    if self.pid_heater_heater1.is_done() {
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

