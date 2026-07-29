// Порождено компилятором Takt (taktc) — цель: Rust (профиль no_std).
// Не редактировать вручную: файл перезаписывается при каждой генерации.
//
// Модуль не обращается к std и подключается как `mod`:
//
//     #[path = "batch_cycle.rs"]
//     pub mod batch_cycle;
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
enum BatchCycleDoseState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Fill,
    Full,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'Dose'.
pub struct BatchCycleDose {
    dosed: u8,
    state: BatchCycleDoseState,
}

impl BatchCycleDose {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            dosed: 0,
            state: BatchCycleDoseState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.dosed = 0;
        self.state = BatchCycleDoseState::Init;
    }

    /// Один такт автомата.
    fn tick(&mut self, shared: &mut BatchCycleShared) {
        if self.state == BatchCycleDoseState::Init {
            self.state = BatchCycleDoseState::Fill;
        }
        match self.state {
            BatchCycleDoseState::Fill => {
                shared.stage = 1;
                self.dosed = self.dosed.wrapping_add(1);
                if self.dosed >= 3 {
                    self.state = BatchCycleDoseState::Full;
                }
            }
            BatchCycleDoseState::Full => {
                self.state = BatchCycleDoseState::End;
            }
            BatchCycleDoseState::End => {}
            BatchCycleDoseState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == BatchCycleDoseState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BatchCycleDrainState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Dry,
    Empty,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'Drain'.
pub struct BatchCycleDrain {
    drained: u8,
    state: BatchCycleDrainState,
}

impl BatchCycleDrain {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            drained: 0,
            state: BatchCycleDrainState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.drained = 0;
        self.state = BatchCycleDrainState::Init;
    }

    /// Один такт автомата.
    fn tick(&mut self, shared: &mut BatchCycleShared) {
        if self.state == BatchCycleDrainState::Init {
            self.state = BatchCycleDrainState::Empty;
        }
        match self.state {
            BatchCycleDrainState::Dry => {
                self.state = BatchCycleDrainState::End;
            }
            BatchCycleDrainState::Empty => {
                shared.stage = 3;
                self.drained = self.drained.wrapping_add(1);
                if self.drained >= 2 {
                    self.state = BatchCycleDrainState::Dry;
                }
            }
            BatchCycleDrainState::End => {}
            BatchCycleDrainState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == BatchCycleDrainState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BatchCycleMixState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Blended,
    Stir,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'Mix'.
pub struct BatchCycleMix {
    stirred: u8,
    state: BatchCycleMixState,
}

impl BatchCycleMix {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            stirred: 0,
            state: BatchCycleMixState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.stirred = 0;
        self.state = BatchCycleMixState::Init;
    }

    /// Один такт автомата.
    fn tick(&mut self, shared: &mut BatchCycleShared) {
        if self.state == BatchCycleMixState::Init {
            self.state = BatchCycleMixState::Stir;
        }
        match self.state {
            BatchCycleMixState::Blended => {
                self.state = BatchCycleMixState::End;
            }
            BatchCycleMixState::Stir => {
                shared.stage = 2;
                self.stirred = self.stirred.wrapping_add(1);
                if self.stirred >= 2 {
                    self.state = BatchCycleMixState::Blended;
                }
            }
            BatchCycleMixState::End => {}
            BatchCycleMixState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == BatchCycleMixState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BatchCycleState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Cycle,
    Done,
    /// Автомат завершён (`is_done`).
    End,
}

/// Шаг последовательной композиции состояния 'Cycle'.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BatchCycleCycleSeq {
    Dose0,
    Mix1,
    Drain2,
}

/// Общие переменные модели 'batch_cycle', разделяемые под-моделями.
struct BatchCycleShared {
    stage: u8,
}

/// Модель 'batch_cycle'.
pub struct BatchCycle<H: Hal> {
    /// Общие с под-моделями переменные (фича 0059).
    shared: BatchCycleShared,
    state: BatchCycleState,
    /// Текущий шаг последовательной композиции состояния 'Cycle'.
    cycle_seq: BatchCycleCycleSeq,
    cycle_dose0: BatchCycleDose,
    cycle_mix1: BatchCycleMix,
    cycle_drain2: BatchCycleDrain,
    /// Аппаратный слой. Заменяет `void *userdata` цели `c`.
    hal: H,
}

impl<H: Hal> BatchCycle<H> {
    /// Создаёт модель поверх аппаратного слоя `hal`.
    ///
    /// В отличие от цели `c`, забыть проставить доступ к железу
    /// невозможно: без `hal` модель не конструируется.
    pub fn new(hal: H) -> Self {
        Self {
            shared: BatchCycleShared {
                stage: 0,
            },
            state: BatchCycleState::Init,
            cycle_seq: BatchCycleCycleSeq::Dose0,
            cycle_dose0: BatchCycleDose::new(),
            cycle_mix1: BatchCycleMix::new(),
            cycle_drain2: BatchCycleDrain::new(),
            hal,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    pub fn init(&mut self) {
        self.shared.stage = 0;
        self.state = BatchCycleState::Init;
        self.cycle_seq = BatchCycleCycleSeq::Dose0;
        self.cycle_dose0.init();
        self.cycle_mix1.init();
        self.cycle_drain2.init();
    }

    /// Один такт автомата.
    ///
    /// Вход в стартовое состояние такта **не расходует** (контракт
    /// ADR 0033): его тело исполняется в этом же вызове.
    pub fn tick(&mut self) {
        if self.state == BatchCycleState::Init {
            self.state = BatchCycleState::Cycle;
        }
        match self.state {
            BatchCycleState::Cycle => {
                if self.cycle_seq == BatchCycleCycleSeq::Dose0 {
                    self.cycle_dose0.tick(&mut self.shared);
                    if self.cycle_dose0.is_done() {
                        self.cycle_mix1.init();
                        self.cycle_seq = BatchCycleCycleSeq::Mix1;
                    }
                } else if self.cycle_seq == BatchCycleCycleSeq::Mix1 {
                    self.cycle_mix1.tick(&mut self.shared);
                    if self.cycle_mix1.is_done() {
                        self.cycle_drain2.init();
                        self.cycle_seq = BatchCycleCycleSeq::Drain2;
                    }
                } else if self.cycle_seq == BatchCycleCycleSeq::Drain2 {
                    self.cycle_drain2.tick(&mut self.shared);
                    if self.cycle_drain2.is_done() {
                        self.state = BatchCycleState::Done;
                    }
                }
            }
            BatchCycleState::Done => {
                self.hal.write_bit(OutBitPort::Ready, true);
                self.state = BatchCycleState::End;
            }
            BatchCycleState::End => {}
            BatchCycleState::Init => {}
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
        self.state == BatchCycleState::End
    }

}

