// Порождено компилятором Lam (taktc) — цель: Rust (профиль no_std).
// Не редактировать вручную: файл перезаписывается при каждой генерации.
//
// Модуль не обращается к std и подключается как `mod`:
//
//     #[path = "elevator.rs"]
//     pub mod elevator;
//
// Атрибута #![no_std] здесь нет намеренно: он допустим только в корне
// крейта, а no_std — свойство крейта, не модуля. Совместимость с no_std
// проверяется гейтом (scripts/precheck.sh).

#![forbid(unsafe_code)]

/// Перечисление 'Action' модели.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Action {
    Idle = 670,
    Closing = 671,
}

/// Перечисление 'Floor' модели.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Floor {
    Bottom = 80,
    Top = 81,
}

/// Порт ввода-вывода модели. Реализация — за трейтом [`Hal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InU8Port {
    Sensors1,
    Sensors2,
    Sensors3,
    Sensors4,
    Sensors5,
    Sensors6,
    Sensors7,
    Sensors8,
    Sensors9,
    SensorsCab,
}

/// Аппаратный слой модели.
///
/// Заменяет пару указателей на функции и `void *userdata` цели `c`:
/// состояние слоя живёт в самом типе-реализации, поэтому привести
/// его не к тому типу или забыть проставить колбэк невозможно.
pub trait Hal {
    /// Читает входной порт `port`.
    fn read_u8(&mut self, port: InU8Port) -> u8;
    /// Внешняя функция модели (`extern fn` в исходнике .lam).
    fn door_close(&mut self);
    /// Внешняя функция модели (`extern fn` в исходнике .lam).
    fn door_open(&mut self);
    /// Внешняя функция модели (`extern fn` в исходнике .lam).
    fn motor_down(&mut self);
    /// Внешняя функция модели (`extern fn` в исходнике .lam).
    fn motor_stop(&mut self);
    /// Внешняя функция модели (`extern fn` в исходнике .lam).
    fn motor_up(&mut self);
    /// Внешняя функция модели (`extern fn` в исходнике .lam).
    fn read_floor_sensors(&mut self);
    /// Внешняя функция модели (`extern fn` в исходнике .lam).
    fn scan_cabin_buttons(&mut self);
    /// Внешняя функция модели (`extern fn` в исходнике .lam).
    fn scan_floor_buttons(&mut self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ElevatorEngineState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    DoorClosing,
    DoorOpening,
    Idle,
    MovingDown,
    MovingUp,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'Engine'.
pub struct ElevatorEngine {
    state: ElevatorEngineState,
}

impl ElevatorEngine {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            state: ElevatorEngineState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.state = ElevatorEngineState::Init;
    }

    /// Один такт автомата.
    fn tick<H: Hal>(&mut self, shared: &mut ElevatorShared, hal: &mut H) {
        if self.state == ElevatorEngineState::Init {
            hal.door_open();
            self.state = ElevatorEngineState::Idle;
        }
        match self.state {
            ElevatorEngineState::DoorClosing => {
                hal.door_close();
                if (((hal.read_u8(InU8Port::SensorsCab) >> 1) & 1) != 0) | (shared.current_floor == shared.target_floor) {
                    hal.door_open();
                    self.state = ElevatorEngineState::Idle;
                } else if (!(((hal.read_u8(InU8Port::SensorsCab) >> 1) & 1) != 0)) & (shared.current_floor < shared.target_floor) {
                    self.state = ElevatorEngineState::MovingUp;
                } else if (!(((hal.read_u8(InU8Port::SensorsCab) >> 1) & 1) != 0)) & (shared.current_floor > shared.target_floor) {
                    self.state = ElevatorEngineState::MovingDown;
                }
            }
            ElevatorEngineState::DoorOpening => {
                hal.door_open();
                self.state = ElevatorEngineState::Idle;
            }
            ElevatorEngineState::Idle => {
                hal.scan_floor_buttons();
                if (hal.read_u8(InU8Port::SensorsCab) & 1) != 0 {
                    hal.scan_cabin_buttons();
                }
                if (shared.has_call == 1) & (!(shared.current_floor == shared.target_floor)) {
                    self.state = ElevatorEngineState::DoorClosing;
                }
            }
            ElevatorEngineState::MovingDown => {
                hal.motor_down();
                hal.read_floor_sensors();
                hal.scan_floor_buttons();
                if (hal.read_u8(InU8Port::SensorsCab) & 1) != 0 {
                    hal.scan_cabin_buttons();
                }
                if shared.current_floor == shared.target_floor {
                    hal.motor_stop();
                    hal.door_open();
                    shared.has_call = 0;
                    self.state = ElevatorEngineState::DoorOpening;
                }
            }
            ElevatorEngineState::MovingUp => {
                hal.motor_up();
                hal.read_floor_sensors();
                hal.scan_floor_buttons();
                if (hal.read_u8(InU8Port::SensorsCab) & 1) != 0 {
                    hal.scan_cabin_buttons();
                }
                if shared.current_floor == shared.target_floor {
                    hal.motor_stop();
                    hal.door_open();
                    shared.has_call = 0;
                    self.state = ElevatorEngineState::DoorOpening;
                }
            }
            ElevatorEngineState::End => {}
            ElevatorEngineState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == ElevatorEngineState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ElevatorState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    End,
    Main,
    Middle,
}

/// Шаг последовательной композиции состояния 'Middle'.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ElevatorMiddleSeq {
    Engine0,
    Group1,
    Engine2,
    Engine3,
    Engine4,
}

/// Общие переменные модели 'elevator', разделяемые под-моделями.
struct ElevatorShared {
    current_floor: u8,
    has_call: u8,
    target_floor: u8,
}

/// Модель 'elevator'.
pub struct Elevator<H: Hal> {
    /// Общие с под-моделями переменные (фича 0059).
    shared: ElevatorShared,
    state: ElevatorState,
    /// Текущий шаг последовательной композиции состояния 'Middle'.
    middle_seq: ElevatorMiddleSeq,
    main: ElevatorEngine,
    middle_engine0: ElevatorEngine,
    middle_group1_engine0: ElevatorEngine,
    middle_group1_engine1: ElevatorEngine,
    middle_engine2: ElevatorEngine,
    middle_engine3: ElevatorEngine,
    middle_engine4: ElevatorEngine,
    /// Аппаратный слой. Заменяет `void *userdata` цели `c`.
    hal: H,
}

impl<H: Hal> Elevator<H> {
    /// Создаёт модель поверх аппаратного слоя `hal`.
    ///
    /// В отличие от цели `c`, забыть проставить доступ к железу
    /// невозможно: без `hal` модель не конструируется.
    pub fn new(hal: H) -> Self {
        Self {
            shared: ElevatorShared {
                current_floor: 1,
                has_call: 0,
                target_floor: 1,
            },
            state: ElevatorState::Init,
            middle_seq: ElevatorMiddleSeq::Engine0,
            main: ElevatorEngine::new(),
            middle_engine0: ElevatorEngine::new(),
            middle_group1_engine0: ElevatorEngine::new(),
            middle_group1_engine1: ElevatorEngine::new(),
            middle_engine2: ElevatorEngine::new(),
            middle_engine3: ElevatorEngine::new(),
            middle_engine4: ElevatorEngine::new(),
            hal,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    pub fn init(&mut self) {
        self.shared.current_floor = 1;
        self.shared.has_call = 0;
        self.shared.target_floor = 1;
        self.state = ElevatorState::Init;
        self.middle_seq = ElevatorMiddleSeq::Engine0;
        self.main.init();
        self.middle_engine0.init();
        self.middle_group1_engine0.init();
        self.middle_group1_engine1.init();
        self.middle_engine2.init();
        self.middle_engine3.init();
        self.middle_engine4.init();
    }

    /// Один такт автомата.
    ///
    /// Вход в стартовое состояние такта **не расходует** (контракт
    /// ADR 0033): его тело исполняется в этом же вызове.
    pub fn tick(&mut self) {
        if self.state == ElevatorState::Init {
            self.state = ElevatorState::Main;
        }
        match self.state {
            ElevatorState::End => {
            }
            ElevatorState::Main => {
                self.main.tick(&mut self.shared, &mut self.hal);
                if self.main.is_done() {
                    self.state = ElevatorState::Middle;
                }
            }
            ElevatorState::Middle => {
                if self.middle_seq == ElevatorMiddleSeq::Engine0 {
                    self.middle_engine0.tick(&mut self.shared, &mut self.hal);
                    if self.middle_engine0.is_done() {
                        self.middle_group1_engine0.init();
                        self.middle_group1_engine1.init();
                        self.middle_seq = ElevatorMiddleSeq::Group1;
                    }
                } else if self.middle_seq == ElevatorMiddleSeq::Group1 {
                    self.middle_group1_engine0.tick(&mut self.shared, &mut self.hal);
                    self.middle_group1_engine1.tick(&mut self.shared, &mut self.hal);
                    if self.middle_group1_engine0.is_done() && self.middle_group1_engine1.is_done() {
                        self.middle_engine2.init();
                        self.middle_seq = ElevatorMiddleSeq::Engine2;
                    }
                } else if self.middle_seq == ElevatorMiddleSeq::Engine2 {
                    self.middle_engine2.tick(&mut self.shared, &mut self.hal);
                    if self.middle_engine2.is_done() {
                        self.middle_engine3.init();
                        self.middle_seq = ElevatorMiddleSeq::Engine3;
                    }
                } else if self.middle_seq == ElevatorMiddleSeq::Engine3 {
                    self.middle_engine3.tick(&mut self.shared, &mut self.hal);
                    if self.middle_engine3.is_done() {
                        self.middle_engine4.init();
                        self.middle_seq = ElevatorMiddleSeq::Engine4;
                    }
                } else if self.middle_seq == ElevatorMiddleSeq::Engine4 {
                    self.middle_engine4.tick(&mut self.shared, &mut self.hal);
                    if self.middle_engine4.is_done() {
                        self.state = ElevatorState::End;
                    }
                }
            }
            ElevatorState::Init => {}
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
        self.state == ElevatorState::End
    }

}

