// Порождено компилятором Takt (taktc) — цель: Rust (профиль no_std).
// Не редактировать вручную: файл перезаписывается при каждой генерации.
//
// Модуль не обращается к std и подключается как `mod`:
//
//     #[path = "elevator_mini.rs"]
//     pub mod elevator_mini;
//
// Атрибута #![no_std] здесь нет намеренно: он допустим только в корне
// крейта, а no_std — свойство крейта, не модуля. Совместимость с no_std
// проверяется гейтом (scripts/precheck.sh).

#![forbid(unsafe_code)]

/// Перечисление 'Command' модели.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    Up = 0,
    Down = 1,
    Stop = 2,
}

/// Порт ввода-вывода модели. Реализация — за трейтом [`Hal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InBitPort {
    CabinButtonDC,
    CabinButtonF1,
    CabinButtonF2,
    CabinButtonF3,
    CabinButtonF4,
    CabinButtonF5,
    CabinButtonF6,
    CabinButtonF7,
    CabinButtonF8,
    CabinButtonF9,
    FloorButtonF1,
    FloorButtonF2,
    FloorButtonF3,
    FloorButtonF4,
    FloorButtonF5,
    FloorButtonF6,
    FloorButtonF7,
    FloorButtonF8,
    FloorButtonF9,
    FloorSensorF1Bottom,
    FloorSensorF2Bottom,
    FloorSensorF3Bottom,
    FloorSensorF4Bottom,
    FloorSensorF5Bottom,
    FloorSensorF6Bottom,
    FloorSensorF7Bottom,
    FloorSensorF8Bottom,
    FloorSensorF9Bottom,
    ElevatorMotorSensorD,
    ElevatorMotorSensorU,
}

/// Порт ввода-вывода модели. Реализация — за трейтом [`Hal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutBitPort {
    DoorOpen,
    ElevatorMotorDown,
    ElevatorMotorStop,
    ElevatorMotorUp,
}

/// Аппаратный слой модели.
///
/// Заменяет пару указателей на функции и `void *userdata` цели `c`:
/// состояние слоя живёт в самом типе-реализации, поэтому привести
/// его не к тому типу или забыть проставить колбэк невозможно.
pub trait Hal {
    /// Читает входной порт `port`.
    fn read_bit(&mut self, port: InBitPort) -> bool;
    /// Пишет `value` в выходной порт `port`.
    fn write_bit(&mut self, port: OutBitPort, value: bool);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ElevatorMiniCabinState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    AtFloor,
    Idle,
    Moving,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'Cabin'.
pub struct ElevatorMiniCabin {
    state: ElevatorMiniCabinState,
}

impl ElevatorMiniCabin {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            state: ElevatorMiniCabinState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.state = ElevatorMiniCabinState::Init;
    }

    /// Один такт автомата.
    fn tick<H: Hal>(&mut self, shared: &mut ElevatorMiniShared, hal: &mut H) {
        if self.state == ElevatorMiniCabinState::Init {
            shared.command = Command::Stop;
            self.state = ElevatorMiniCabinState::Idle;
        }
        match self.state {
            ElevatorMiniCabinState::AtFloor => {
                hal.write_bit(OutBitPort::DoorOpen, true);
                if hal.read_bit(InBitPort::CabinButtonDC) {
                    shared.command = Command::Stop;
                    self.state = ElevatorMiniCabinState::Idle;
                }
            }
            ElevatorMiniCabinState::Idle => {
                if hal.read_bit(InBitPort::FloorSensorF1Bottom) {
                    shared.current_floor = 1;
                }
                if hal.read_bit(InBitPort::FloorSensorF2Bottom) {
                    shared.current_floor = 2;
                }
                if hal.read_bit(InBitPort::FloorSensorF3Bottom) {
                    shared.current_floor = 3;
                }
                if hal.read_bit(InBitPort::FloorSensorF4Bottom) {
                    shared.current_floor = 4;
                }
                if hal.read_bit(InBitPort::FloorSensorF5Bottom) {
                    shared.current_floor = 5;
                }
                if hal.read_bit(InBitPort::FloorSensorF6Bottom) {
                    shared.current_floor = 6;
                }
                if hal.read_bit(InBitPort::FloorSensorF7Bottom) {
                    shared.current_floor = 7;
                }
                if hal.read_bit(InBitPort::FloorSensorF8Bottom) {
                    shared.current_floor = 8;
                }
                if hal.read_bit(InBitPort::FloorSensorF9Bottom) {
                    shared.current_floor = 9;
                }
                if hal.read_bit(InBitPort::CabinButtonF1) {
                    shared.target_floor = 1;
                }
                if hal.read_bit(InBitPort::CabinButtonF2) {
                    shared.target_floor = 2;
                }
                if hal.read_bit(InBitPort::CabinButtonF3) {
                    shared.target_floor = 3;
                }
                if hal.read_bit(InBitPort::CabinButtonF4) {
                    shared.target_floor = 4;
                }
                if hal.read_bit(InBitPort::CabinButtonF5) {
                    shared.target_floor = 5;
                }
                if hal.read_bit(InBitPort::CabinButtonF6) {
                    shared.target_floor = 6;
                }
                if hal.read_bit(InBitPort::CabinButtonF7) {
                    shared.target_floor = 7;
                }
                if hal.read_bit(InBitPort::CabinButtonF8) {
                    shared.target_floor = 8;
                }
                if hal.read_bit(InBitPort::CabinButtonF9) {
                    shared.target_floor = 9;
                }
                if hal.read_bit(InBitPort::FloorButtonF1) {
                    shared.target_floor = 1;
                }
                if hal.read_bit(InBitPort::FloorButtonF2) {
                    shared.target_floor = 2;
                }
                if hal.read_bit(InBitPort::FloorButtonF3) {
                    shared.target_floor = 3;
                }
                if hal.read_bit(InBitPort::FloorButtonF4) {
                    shared.target_floor = 4;
                }
                if hal.read_bit(InBitPort::FloorButtonF5) {
                    shared.target_floor = 5;
                }
                if hal.read_bit(InBitPort::FloorButtonF6) {
                    shared.target_floor = 6;
                }
                if hal.read_bit(InBitPort::FloorButtonF7) {
                    shared.target_floor = 7;
                }
                if hal.read_bit(InBitPort::FloorButtonF8) {
                    shared.target_floor = 8;
                }
                if hal.read_bit(InBitPort::FloorButtonF9) {
                    shared.target_floor = 9;
                }
                hal.write_bit(OutBitPort::DoorOpen, true);
                if shared.target_floor != 0 {
                    self.state = ElevatorMiniCabinState::Moving;
                }
            }
            ElevatorMiniCabinState::Moving => {
                if hal.read_bit(InBitPort::FloorSensorF1Bottom) {
                    shared.current_floor = 1;
                }
                if hal.read_bit(InBitPort::FloorSensorF2Bottom) {
                    shared.current_floor = 2;
                }
                if hal.read_bit(InBitPort::FloorSensorF3Bottom) {
                    shared.current_floor = 3;
                }
                if hal.read_bit(InBitPort::FloorSensorF4Bottom) {
                    shared.current_floor = 4;
                }
                if hal.read_bit(InBitPort::FloorSensorF5Bottom) {
                    shared.current_floor = 5;
                }
                if hal.read_bit(InBitPort::FloorSensorF6Bottom) {
                    shared.current_floor = 6;
                }
                if hal.read_bit(InBitPort::FloorSensorF7Bottom) {
                    shared.current_floor = 7;
                }
                if hal.read_bit(InBitPort::FloorSensorF8Bottom) {
                    shared.current_floor = 8;
                }
                if hal.read_bit(InBitPort::FloorSensorF9Bottom) {
                    shared.current_floor = 9;
                }
                if shared.target_floor > shared.current_floor {
                    shared.command = Command::Up;
                }
                if shared.target_floor < shared.current_floor {
                    shared.command = Command::Down;
                }
                if shared.target_floor == shared.current_floor {
                    shared.command = Command::Stop;
                    shared.target_floor = 0;
                    self.state = ElevatorMiniCabinState::AtFloor;
                }
            }
            ElevatorMiniCabinState::End => {}
            ElevatorMiniCabinState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == ElevatorMiniCabinState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ElevatorMiniMotorState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Down,
    Idle,
    Stop,
    Up,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'Motor'.
pub struct ElevatorMiniMotor {
    state: ElevatorMiniMotorState,
}

impl ElevatorMiniMotor {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            state: ElevatorMiniMotorState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.state = ElevatorMiniMotorState::Init;
    }

    /// Один такт автомата.
    fn tick<H: Hal>(&mut self, shared: &mut ElevatorMiniShared, hal: &mut H) {
        if self.state == ElevatorMiniMotorState::Init {
            hal.write_bit(OutBitPort::ElevatorMotorStop, true);
            self.state = ElevatorMiniMotorState::Idle;
        }
        match self.state {
            ElevatorMiniMotorState::Down => {
                hal.write_bit(OutBitPort::ElevatorMotorDown, true);
                if (shared.command == Command::Stop) | hal.read_bit(InBitPort::ElevatorMotorSensorD) {
                    self.state = ElevatorMiniMotorState::Stop;
                }
            }
            ElevatorMiniMotorState::Idle => {
                if shared.command == Command::Up {
                    self.state = ElevatorMiniMotorState::Up;
                } else if shared.command == Command::Down {
                    self.state = ElevatorMiniMotorState::Down;
                } else if shared.command == Command::Stop {
                    self.state = ElevatorMiniMotorState::Stop;
                }
            }
            ElevatorMiniMotorState::Stop => {
                hal.write_bit(OutBitPort::ElevatorMotorStop, true);
                hal.write_bit(OutBitPort::ElevatorMotorStop, true);
                self.state = ElevatorMiniMotorState::Idle;
            }
            ElevatorMiniMotorState::Up => {
                hal.write_bit(OutBitPort::ElevatorMotorUp, true);
                if (shared.command == Command::Stop) | hal.read_bit(InBitPort::ElevatorMotorSensorU) {
                    self.state = ElevatorMiniMotorState::Stop;
                }
            }
            ElevatorMiniMotorState::End => {}
            ElevatorMiniMotorState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == ElevatorMiniMotorState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ElevatorMiniState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Main,
    /// Автомат завершён (`is_done`).
    End,
}

/// Общие переменные модели 'elevator_mini', разделяемые под-моделями.
struct ElevatorMiniShared {
    command: Command,
    current_floor: u8,
    target_floor: u8,
}

/// Модель 'elevator_mini'.
pub struct ElevatorMini<H: Hal> {
    /// Общие с под-моделями переменные (фича 0059).
    shared: ElevatorMiniShared,
    state: ElevatorMiniState,
    main_cabin0: ElevatorMiniCabin,
    main_motor1: ElevatorMiniMotor,
    /// Аппаратный слой. Заменяет `void *userdata` цели `c`.
    hal: H,
}

impl<H: Hal> ElevatorMini<H> {
    /// Создаёт модель поверх аппаратного слоя `hal`.
    ///
    /// В отличие от цели `c`, забыть проставить доступ к железу
    /// невозможно: без `hal` модель не конструируется.
    pub fn new(hal: H) -> Self {
        Self {
            shared: ElevatorMiniShared {
                command: Command::Stop,
                current_floor: 1,
                target_floor: 0,
            },
            state: ElevatorMiniState::Init,
            main_cabin0: ElevatorMiniCabin::new(),
            main_motor1: ElevatorMiniMotor::new(),
            hal,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    pub fn init(&mut self) {
        self.shared.command = Command::Stop;
        self.shared.current_floor = 1;
        self.shared.target_floor = 0;
        self.state = ElevatorMiniState::Init;
        self.main_cabin0.init();
        self.main_motor1.init();
    }

    /// Один такт автомата.
    ///
    /// Вход в стартовое состояние такта **не расходует** (контракт
    /// ADR 0033): его тело исполняется в этом же вызове.
    pub fn tick(&mut self) {
        if self.state == ElevatorMiniState::Init {
            self.state = ElevatorMiniState::Main;
        }
        match self.state {
            ElevatorMiniState::Main => {
                self.main_cabin0.tick(&mut self.shared, &mut self.hal);
                self.main_motor1.tick(&mut self.shared, &mut self.hal);
                if self.main_cabin0.is_done() && self.main_motor1.is_done() {
                    self.state = ElevatorMiniState::End;
                }
            }
            ElevatorMiniState::End => {}
            ElevatorMiniState::Init => {}
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
        self.state == ElevatorMiniState::End
    }

}

