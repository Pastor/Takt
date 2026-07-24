// Порождено компилятором Lam (taktc) — цель: Rust (профиль no_std).
// Не редактировать вручную: файл перезаписывается при каждой генерации.
//
// Модуль не обращается к std и подключается как `mod`:
//
//     #[path = "stacker.rs"]
//     pub mod stacker;
//
// Атрибута #![no_std] здесь нет намеренно: он допустим только в корне
// крейта, а no_std — свойство крейта, не модуля. Совместимость с no_std
// проверяется гейтом (scripts/precheck.sh).

#![forbid(unsafe_code)]

const CHARGE_ROW: u8 = 0;
const CHARGE_SECTION: u8 = 0;
const CHARGE_STACK: u8 = 0;
const DROPOFF_ROW: u8 = 1;
const DROPOFF_SECTION: u8 = 1;
const DROPOFF_STACK: u8 = 11;
const PICKUP_ROW: u8 = 1;
const PICKUP_SECTION: u8 = 1;
const PICKUP_STACK: u8 = 0;

/// Порт ввода-вывода модели. Реализация — за трейтом [`Hal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InBitPort {
    SenseAtCharge,
    SenseBatteryLow,
    SenseLoaded,
    TaskType,
    TaskValid,
}

/// Порт ввода-вывода модели. Реализация — за трейтом [`Hal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InU8Port {
    PosRow,
    PosSection,
    PosStack,
    TaskRowNo,
    TaskSectionNo,
    TaskStackNo,
}

/// Порт ввода-вывода модели. Реализация — за трейтом [`Hal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutBitPort {
    CmdAck,
    CmdDone,
    CmdFork,
}

/// Порт ввода-вывода модели. Реализация — за трейтом [`Hal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutU8Port {
    CmdTargetRow,
    CmdTargetSection,
    CmdTargetStack,
}

/// Аппаратный слой модели.
///
/// Заменяет пару указателей на функции и `void *userdata` цели `c`:
/// состояние слоя живёт в самом типе-реализации, поэтому привести
/// его не к тому типу или забыть проставить колбэк невозможно.
pub trait Hal {
    /// Читает входной порт `port`.
    fn read_bit(&mut self, port: InBitPort) -> bool;
    /// Читает входной порт `port`.
    fn read_u8(&mut self, port: InU8Port) -> u8;
    /// Пишет `value` в выходной порт `port`.
    fn write_bit(&mut self, port: OutBitPort, value: bool);
    /// Пишет `value` в выходной порт `port`.
    fn write_u8(&mut self, port: OutU8Port, value: u8);
}

/// Функция 'travel_time' модели.
fn travel_time<H: Hal>(to_stack: u8, to_row: u8, to_section: u8, hal: &mut H) -> u8 {
    let ds: u8 = if hal.read_u8(InU8Port::PosStack) > to_stack { hal.read_u8(InU8Port::PosStack) - to_stack } else { to_stack - hal.read_u8(InU8Port::PosStack) };
    let dr: u8 = if hal.read_u8(InU8Port::PosRow) > to_row { hal.read_u8(InU8Port::PosRow) - to_row } else { to_row - hal.read_u8(InU8Port::PosRow) };
    let dy: u8 = if hal.read_u8(InU8Port::PosSection) > to_section { hal.read_u8(InU8Port::PosSection) - to_section } else { to_section - hal.read_u8(InU8Port::PosSection) };
    let mut t: u8 = ds;
    if dr > t {
        t = dr;
    }
    if dy > t {
        t = dy;
    }
    t
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StackerCommandReceiverState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    AcceptingTask,
    TaskActive,
    WaitingForTask,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'CommandReceiver'.
pub struct StackerCommandReceiver {
    state: StackerCommandReceiverState,
}

impl StackerCommandReceiver {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            state: StackerCommandReceiverState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.state = StackerCommandReceiverState::Init;
    }

    /// Один такт автомата.
    fn tick<H: Hal>(&mut self, shared: &mut StackerShared, hal: &mut H) {
        if self.state == StackerCommandReceiverState::Init {
            hal.write_bit(OutBitPort::CmdAck, false);
            self.state = StackerCommandReceiverState::WaitingForTask;
        }
        match self.state {
            StackerCommandReceiverState::AcceptingTask => {
                hal.write_bit(OutBitPort::CmdAck, false);
                self.state = StackerCommandReceiverState::TaskActive;
            }
            StackerCommandReceiverState::TaskActive => {
                if !shared.busy {
                    hal.write_bit(OutBitPort::CmdAck, false);
                    self.state = StackerCommandReceiverState::WaitingForTask;
                }
            }
            StackerCommandReceiverState::WaitingForTask => {
                if (hal.read_bit(InBitPort::TaskValid) & (!shared.busy)) & (!hal.read_bit(InBitPort::SenseBatteryLow)) {
                    shared.tgt_stack = hal.read_u8(InU8Port::TaskStackNo);
                    shared.tgt_row = hal.read_u8(InU8Port::TaskRowNo);
                    shared.tgt_section = hal.read_u8(InU8Port::TaskSectionNo);
                    shared.tgt_type = hal.read_bit(InBitPort::TaskType);
                    shared.eta = travel_time(hal.read_u8(InU8Port::TaskStackNo), hal.read_u8(InU8Port::TaskRowNo), hal.read_u8(InU8Port::TaskSectionNo), &mut *hal);
                    shared.busy = true;
                    hal.write_bit(OutBitPort::CmdAck, true);
                    self.state = StackerCommandReceiverState::AcceptingTask;
                }
            }
            StackerCommandReceiverState::End => {}
            StackerCommandReceiverState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == StackerCommandReceiverState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StackerLiftControllerState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    LiftDone,
    LiftIdle,
    LiftOperating,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'LiftController'.
pub struct StackerLiftController {
    state: StackerLiftControllerState,
}

impl StackerLiftController {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            state: StackerLiftControllerState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.state = StackerLiftControllerState::Init;
    }

    /// Один такт автомата.
    fn tick<H: Hal>(&mut self, shared: &mut StackerShared, hal: &mut H) {
        if self.state == StackerLiftControllerState::Init {
            hal.write_bit(OutBitPort::CmdFork, false);
            self.state = StackerLiftControllerState::LiftIdle;
        }
        match self.state {
            StackerLiftControllerState::LiftDone => {
                if !shared.lift_request {
                    hal.write_bit(OutBitPort::CmdFork, false);
                    self.state = StackerLiftControllerState::LiftIdle;
                }
            }
            StackerLiftControllerState::LiftIdle => {
                if shared.lift_request {
                    hal.write_bit(OutBitPort::CmdFork, true);
                    self.state = StackerLiftControllerState::LiftOperating;
                }
            }
            StackerLiftControllerState::LiftOperating => {
                if ((shared.lift_request & (!shared.lift_op)) & hal.read_bit(InBitPort::SenseLoaded)) || ((shared.lift_request & shared.lift_op) & (!hal.read_bit(InBitPort::SenseLoaded))) {
                    hal.write_bit(OutBitPort::CmdFork, false);
                    shared.lift_done = true;
                    self.state = StackerLiftControllerState::LiftDone;
                } else if !shared.lift_request {
                    hal.write_bit(OutBitPort::CmdFork, false);
                    self.state = StackerLiftControllerState::LiftIdle;
                }
            }
            StackerLiftControllerState::End => {}
            StackerLiftControllerState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == StackerLiftControllerState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StackerMovementControllerState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    DispatchMove,
    EmergencyCharge,
    MovementIdle,
    MovingToCell,
    MovingToDropoff,
    MovingToPickup,
    MovingToStorage,
    TaskCompleting,
    WaitingForkAtCell,
    WaitingForkAtDropoff,
    WaitingForkAtPickup,
    WaitingForkAtStorage,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'MovementController'.
pub struct StackerMovementController {
    state: StackerMovementControllerState,
}

impl StackerMovementController {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            state: StackerMovementControllerState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.state = StackerMovementControllerState::Init;
    }

    /// Один такт автомата.
    fn tick<H: Hal>(&mut self, shared: &mut StackerShared, hal: &mut H) {
        if self.state == StackerMovementControllerState::Init {
            hal.write_u8(OutU8Port::CmdTargetStack, CHARGE_STACK);
            hal.write_u8(OutU8Port::CmdTargetRow, CHARGE_ROW);
            hal.write_u8(OutU8Port::CmdTargetSection, CHARGE_SECTION);
            hal.write_bit(OutBitPort::CmdDone, false);
            self.state = StackerMovementControllerState::MovementIdle;
        }
        match self.state {
            StackerMovementControllerState::DispatchMove => {
                if !shared.tgt_type {
                    hal.write_u8(OutU8Port::CmdTargetStack, PICKUP_STACK);
                    hal.write_u8(OutU8Port::CmdTargetRow, PICKUP_ROW);
                    hal.write_u8(OutU8Port::CmdTargetSection, PICKUP_SECTION);
                    self.state = StackerMovementControllerState::MovingToPickup;
                } else if shared.tgt_type {
                    shared.lift_request = false;
                    shared.lift_done = false;
                    hal.write_u8(OutU8Port::CmdTargetStack, shared.tgt_stack);
                    hal.write_u8(OutU8Port::CmdTargetRow, shared.tgt_row);
                    hal.write_u8(OutU8Port::CmdTargetSection, shared.tgt_section);
                    self.state = StackerMovementControllerState::MovingToStorage;
                }
            }
            StackerMovementControllerState::EmergencyCharge => {
                if hal.read_bit(InBitPort::SenseAtCharge) {
                    hal.write_u8(OutU8Port::CmdTargetStack, CHARGE_STACK);
                    hal.write_u8(OutU8Port::CmdTargetRow, CHARGE_ROW);
                    hal.write_u8(OutU8Port::CmdTargetSection, CHARGE_SECTION);
                    hal.write_bit(OutBitPort::CmdDone, false);
                    self.state = StackerMovementControllerState::MovementIdle;
                }
            }
            StackerMovementControllerState::MovementIdle => {
                if shared.busy & (!hal.read_bit(InBitPort::SenseBatteryLow)) {
                    self.state = StackerMovementControllerState::DispatchMove;
                }
            }
            StackerMovementControllerState::MovingToCell => {
                if ((hal.read_u8(InU8Port::PosStack) == shared.tgt_stack) & (hal.read_u8(InU8Port::PosRow) == shared.tgt_row)) & (hal.read_u8(InU8Port::PosSection) == shared.tgt_section) {
                    shared.lift_request = true;
                    shared.lift_op = true;
                    self.state = StackerMovementControllerState::WaitingForkAtCell;
                } else if hal.read_bit(InBitPort::SenseBatteryLow) {
                    hal.write_u8(OutU8Port::CmdTargetStack, CHARGE_STACK);
                    hal.write_u8(OutU8Port::CmdTargetRow, CHARGE_ROW);
                    hal.write_u8(OutU8Port::CmdTargetSection, CHARGE_SECTION);
                    shared.lift_request = false;
                    shared.lift_done = false;
                    hal.write_bit(OutBitPort::CmdAck, false);
                    hal.write_bit(OutBitPort::CmdDone, false);
                    shared.busy = false;
                    self.state = StackerMovementControllerState::EmergencyCharge;
                }
            }
            StackerMovementControllerState::MovingToDropoff => {
                if ((hal.read_u8(InU8Port::PosStack) == DROPOFF_STACK) & (hal.read_u8(InU8Port::PosRow) == DROPOFF_ROW)) & (hal.read_u8(InU8Port::PosSection) == DROPOFF_SECTION) {
                    shared.lift_request = true;
                    shared.lift_op = true;
                    self.state = StackerMovementControllerState::WaitingForkAtDropoff;
                } else if hal.read_bit(InBitPort::SenseBatteryLow) {
                    hal.write_u8(OutU8Port::CmdTargetStack, CHARGE_STACK);
                    hal.write_u8(OutU8Port::CmdTargetRow, CHARGE_ROW);
                    hal.write_u8(OutU8Port::CmdTargetSection, CHARGE_SECTION);
                    shared.lift_request = false;
                    shared.lift_done = false;
                    hal.write_bit(OutBitPort::CmdAck, false);
                    hal.write_bit(OutBitPort::CmdDone, false);
                    shared.busy = false;
                    self.state = StackerMovementControllerState::EmergencyCharge;
                }
            }
            StackerMovementControllerState::MovingToPickup => {
                if ((hal.read_u8(InU8Port::PosStack) == PICKUP_STACK) & (hal.read_u8(InU8Port::PosRow) == PICKUP_ROW)) & (hal.read_u8(InU8Port::PosSection) == PICKUP_SECTION) {
                    shared.lift_request = true;
                    shared.lift_op = false;
                    self.state = StackerMovementControllerState::WaitingForkAtPickup;
                } else if hal.read_bit(InBitPort::SenseBatteryLow) {
                    hal.write_u8(OutU8Port::CmdTargetStack, CHARGE_STACK);
                    hal.write_u8(OutU8Port::CmdTargetRow, CHARGE_ROW);
                    hal.write_u8(OutU8Port::CmdTargetSection, CHARGE_SECTION);
                    shared.lift_request = false;
                    shared.lift_done = false;
                    hal.write_bit(OutBitPort::CmdAck, false);
                    hal.write_bit(OutBitPort::CmdDone, false);
                    shared.busy = false;
                    self.state = StackerMovementControllerState::EmergencyCharge;
                }
            }
            StackerMovementControllerState::MovingToStorage => {
                if ((hal.read_u8(InU8Port::PosStack) == shared.tgt_stack) & (hal.read_u8(InU8Port::PosRow) == shared.tgt_row)) & (hal.read_u8(InU8Port::PosSection) == shared.tgt_section) {
                    shared.lift_request = true;
                    shared.lift_op = false;
                    self.state = StackerMovementControllerState::WaitingForkAtStorage;
                } else if hal.read_bit(InBitPort::SenseBatteryLow) {
                    hal.write_u8(OutU8Port::CmdTargetStack, CHARGE_STACK);
                    hal.write_u8(OutU8Port::CmdTargetRow, CHARGE_ROW);
                    hal.write_u8(OutU8Port::CmdTargetSection, CHARGE_SECTION);
                    shared.lift_request = false;
                    shared.lift_done = false;
                    hal.write_bit(OutBitPort::CmdAck, false);
                    hal.write_bit(OutBitPort::CmdDone, false);
                    shared.busy = false;
                    self.state = StackerMovementControllerState::EmergencyCharge;
                }
            }
            StackerMovementControllerState::TaskCompleting => {
                hal.write_bit(OutBitPort::CmdDone, false);
                hal.write_u8(OutU8Port::CmdTargetStack, CHARGE_STACK);
                hal.write_u8(OutU8Port::CmdTargetRow, CHARGE_ROW);
                hal.write_u8(OutU8Port::CmdTargetSection, CHARGE_SECTION);
                hal.write_bit(OutBitPort::CmdDone, false);
                self.state = StackerMovementControllerState::MovementIdle;
            }
            StackerMovementControllerState::WaitingForkAtCell => {
                if shared.lift_done {
                    shared.lift_request = false;
                    shared.lift_done = false;
                    shared.busy = false;
                    hal.write_bit(OutBitPort::CmdDone, true);
                    self.state = StackerMovementControllerState::TaskCompleting;
                }
            }
            StackerMovementControllerState::WaitingForkAtDropoff => {
                if shared.lift_done {
                    shared.lift_request = false;
                    shared.lift_done = false;
                    shared.busy = false;
                    hal.write_bit(OutBitPort::CmdDone, true);
                    self.state = StackerMovementControllerState::TaskCompleting;
                }
            }
            StackerMovementControllerState::WaitingForkAtPickup => {
                if shared.lift_done {
                    shared.lift_request = false;
                    shared.lift_done = false;
                    hal.write_u8(OutU8Port::CmdTargetStack, shared.tgt_stack);
                    hal.write_u8(OutU8Port::CmdTargetRow, shared.tgt_row);
                    hal.write_u8(OutU8Port::CmdTargetSection, shared.tgt_section);
                    self.state = StackerMovementControllerState::MovingToCell;
                }
            }
            StackerMovementControllerState::WaitingForkAtStorage => {
                if shared.lift_done {
                    shared.lift_request = false;
                    shared.lift_done = false;
                    hal.write_u8(OutU8Port::CmdTargetStack, DROPOFF_STACK);
                    hal.write_u8(OutU8Port::CmdTargetRow, DROPOFF_ROW);
                    hal.write_u8(OutU8Port::CmdTargetSection, DROPOFF_SECTION);
                    self.state = StackerMovementControllerState::MovingToDropoff;
                }
            }
            StackerMovementControllerState::End => {}
            StackerMovementControllerState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == StackerMovementControllerState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StackerState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Stacker,
    /// Автомат завершён (`is_done`).
    End,
}

/// Общие переменные модели 'stacker', разделяемые под-моделями.
struct StackerShared {
    busy: bool,
    eta: u8,
    lift_done: bool,
    lift_op: bool,
    lift_request: bool,
    tgt_row: u8,
    tgt_section: u8,
    tgt_stack: u8,
    tgt_type: bool,
}

/// Модель 'stacker'.
pub struct Stacker<H: Hal> {
    /// Общие с под-моделями переменные (фича 0059).
    shared: StackerShared,
    state: StackerState,
    stacker_command_receiver0: StackerCommandReceiver,
    stacker_movement_controller1: StackerMovementController,
    stacker_lift_controller2: StackerLiftController,
    /// Аппаратный слой. Заменяет `void *userdata` цели `c`.
    hal: H,
}

impl<H: Hal> Stacker<H> {
    /// Создаёт модель поверх аппаратного слоя `hal`.
    ///
    /// В отличие от цели `c`, забыть проставить доступ к железу
    /// невозможно: без `hal` модель не конструируется.
    pub fn new(hal: H) -> Self {
        Self {
            shared: StackerShared {
                busy: false,
                eta: 0,
                lift_done: false,
                lift_op: false,
                lift_request: false,
                tgt_row: 0,
                tgt_section: 0,
                tgt_stack: 0,
                tgt_type: false,
            },
            state: StackerState::Init,
            stacker_command_receiver0: StackerCommandReceiver::new(),
            stacker_movement_controller1: StackerMovementController::new(),
            stacker_lift_controller2: StackerLiftController::new(),
            hal,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    pub fn init(&mut self) {
        self.shared.busy = false;
        self.shared.eta = 0;
        self.shared.lift_done = false;
        self.shared.lift_op = false;
        self.shared.lift_request = false;
        self.shared.tgt_row = 0;
        self.shared.tgt_section = 0;
        self.shared.tgt_stack = 0;
        self.shared.tgt_type = false;
        self.state = StackerState::Init;
        self.stacker_command_receiver0.init();
        self.stacker_movement_controller1.init();
        self.stacker_lift_controller2.init();
    }

    /// Один такт автомата.
    ///
    /// Вход в стартовое состояние такта **не расходует** (контракт
    /// ADR 0033): его тело исполняется в этом же вызове.
    pub fn tick(&mut self) {
        if self.state == StackerState::Init {
            self.state = StackerState::Stacker;
        }
        match self.state {
            StackerState::Stacker => {
                self.stacker_command_receiver0.tick(&mut self.shared, &mut self.hal);
                self.stacker_movement_controller1.tick(&mut self.shared, &mut self.hal);
                self.stacker_lift_controller2.tick(&mut self.shared, &mut self.hal);
                if self.stacker_command_receiver0.is_done() && self.stacker_movement_controller1.is_done() && self.stacker_lift_controller2.is_done() {
                    self.state = StackerState::End;
                }
            }
            StackerState::End => {}
            StackerState::Init => {}
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
        self.state == StackerState::End
    }

}

