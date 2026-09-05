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

/// Функция 'pid_init' модели.
fn pid_init(kp: f64, ki: f64, kd: f64, ts: f64, lo: f64, hi: f64) -> PidState {
    let mut p: PidState = PidState { kp: 0.0, ki: 0.0, kd: 0.0, ts: 1.0, out_min: 0.0, out_max: 0.0, i_acc: 0.0, err_prev: 0.0, output: 0.0 };
    p.kp = kp;
    p.ki = ki;
    p.kd = kd;
    p.ts = ts;
    p.out_min = lo;
    p.out_max = hi;
    p
}

/// Функция 'pid_reset' модели.
fn pid_reset(p: PidState) -> PidState {
    let mut r: PidState = p;
    r.i_acc = 0.0;
    r.err_prev = 0.0;
    r.output = 0.0;
    r
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PidLawState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Run,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'pid_law'.
pub struct PidLaw {
    ctrl: f64,
    hold: bool,
    loop_pid: PidState,
    meas: f64,
    target: f64,
    state: PidLawState,
}

impl PidLaw {
    /// Создаёт модель в начальном состоянии.
    pub fn new() -> Self {
        Self {
            ctrl: 0.0,
            hold: false,
            loop_pid: PidState { kp: 3.0, ki: 0.75, kd: 1.5, ts: 0.1, out_min: 0.0, out_max: 100.0, i_acc: 0.0, err_prev: 0.0, output: 0.0 },
            meas: 25.0,
            target: 80.0,
            state: PidLawState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    pub fn init(&mut self) {
        self.ctrl = 0.0;
        self.hold = false;
        self.loop_pid = PidState { kp: 3.0, ki: 0.75, kd: 1.5, ts: 0.1, out_min: 0.0, out_max: 100.0, i_acc: 0.0, err_prev: 0.0, output: 0.0 };
        self.meas = 25.0;
        self.target = 80.0;
        self.state = PidLawState::Init;
    }

    /// Один такт автомата.
    ///
    /// Вход в стартовое состояние такта **не расходует** (контракт
    /// ADR 0033): его тело исполняется в этом же вызове.
    pub fn tick(&mut self) {
        if self.state == PidLawState::Init {
            self.loop_pid = pid_init(3.0, 0.75, 1.5, 0.1, 0.0, 100.0);
            self.state = PidLawState::Run;
        }
        match self.state {
            PidLawState::Run => {
                self.loop_pid = pid_compute(self.loop_pid, self.target, self.meas);
                self.ctrl = self.loop_pid.output;
                if self.hold {
                    self.loop_pid = pid_reset(self.loop_pid);
                    self.ctrl = 0.0;
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

/// Модель в начальном состоянии — синоним [`new`](Self::new).
impl Default for PidLaw {
    fn default() -> Self {
        Self::new()
    }
}

