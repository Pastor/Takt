// Порождено компилятором Takt (taktc) — цель: Rust (профиль no_std).
// Не редактировать вручную: файл перезаписывается при каждой генерации.
//
// Модуль не обращается к std и подключается как `mod`:
//
//     #[path = "extend_complex.rs"]
//     pub mod extend_complex;
//
// Атрибута #![no_std] здесь нет намеренно: он допустим только в корне
// крейта, а no_std — свойство крейта, не модуля. Совместимость с no_std
// проверяется гейтом (scripts/precheck.sh).

#![forbid(unsafe_code)]

/// Структура 'Coord' модели.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Coord {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

/// Перечисление 'Point' модели.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Point {
    Global = 0,
    Local = 1,
}

/// Перечисление 'Constant' модели.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Constant {
    X = 0,
    Y = 1,
    Z = 2,
}

const EXTEND_COMPLEX_ENABLED: bool = true;

/// Порт ввода-вывода модели. Реализация — за трейтом [`Hal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InBitPort {
    Wait,
}

/// Порт ввода-вывода модели. Реализация — за трейтом [`Hal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutBitPort {
    Idle,
    Work,
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
    /// Внешняя функция модели (`extern fn` в исходнике .takt).
    fn has_flag(&mut self, v: bool) -> bool;
}

/// Функция 'is_collected' модели.
fn is_collected(c: Constant, v: u8, x: u8) -> bool {
    (((c == Constant::X) && (v == 1)) || ((c == Constant::Y) && (v == 2))) || (!(((x >> 2) & 1) != 0))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtendComplexAState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Start,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'A'.
pub struct ExtendComplexA {
    state: ExtendComplexAState,
}

impl ExtendComplexA {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            state: ExtendComplexAState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.state = ExtendComplexAState::Init;
    }

    /// Один такт автомата.
    fn tick(&mut self) {
        if self.state == ExtendComplexAState::Init {
            self.state = ExtendComplexAState::Start;
        }
        match self.state {
            ExtendComplexAState::Start => {
                self.state = ExtendComplexAState::End;
            }
            ExtendComplexAState::End => {}
            ExtendComplexAState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == ExtendComplexAState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtendComplexBState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Done,
    Start,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'B'.
pub struct ExtendComplexB {
    state: ExtendComplexBState,
}

impl ExtendComplexB {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            state: ExtendComplexBState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.state = ExtendComplexBState::Init;
    }

    /// Один такт автомата.
    fn tick<H: Hal>(&mut self, hal: &mut H) {
        if self.state == ExtendComplexBState::Init {
            self.state = ExtendComplexBState::Start;
        }
        match self.state {
            ExtendComplexBState::Done => {
                self.state = ExtendComplexBState::End;
            }
            ExtendComplexBState::Start => {
                if hal.read_bit(InBitPort::Wait) {
                    hal.write_bit(OutBitPort::Work, false);
                    hal.write_bit(OutBitPort::Idle, true);
                } else {
                    hal.write_bit(OutBitPort::Idle, false);
                    hal.write_bit(OutBitPort::Work, true);
                }
                self.state = ExtendComplexBState::Done;
            }
            ExtendComplexBState::End => {}
            ExtendComplexBState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == ExtendComplexBState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtendComplexCState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    End,
    Start,
}

/// Модель 'C'.
pub struct ExtendComplexC {
    state: ExtendComplexCState,
    start_c10: ExtendComplexCC1,
    start_c21: ExtendComplexCC2,
}

impl ExtendComplexC {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            state: ExtendComplexCState::Init,
            start_c10: ExtendComplexCC1::new(),
            start_c21: ExtendComplexCC2::new(),
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.state = ExtendComplexCState::Init;
        self.start_c10.init();
        self.start_c21.init();
    }

    /// Один такт автомата.
    fn tick<H: Hal>(&mut self, shared: &mut ExtendComplexShared, hal: &mut H) {
        if self.state == ExtendComplexCState::Init {
            self.state = ExtendComplexCState::Start;
        }
        match self.state {
            ExtendComplexCState::End => {
            }
            ExtendComplexCState::Start => {
                self.start_c10.tick(shared, &mut *hal);
                self.start_c21.tick();
                if self.start_c10.is_done() && self.start_c21.is_done() {
                    self.state = ExtendComplexCState::End;
                }
            }
            ExtendComplexCState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == ExtendComplexCState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtendComplexCC1State {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    End,
    Start,
}

/// Модель 'C1'.
pub struct ExtendComplexCC1 {
    state: ExtendComplexCC1State,
}

impl ExtendComplexCC1 {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            state: ExtendComplexCC1State::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.state = ExtendComplexCC1State::Init;
    }

    /// Один такт автомата.
    fn tick<H: Hal>(&mut self, shared: &mut ExtendComplexShared, hal: &mut H) {
        if self.state == ExtendComplexCC1State::Init {
            self.state = ExtendComplexCC1State::Start;
        }
        match self.state {
            ExtendComplexCC1State::End => {
            }
            ExtendComplexCC1State::Start => {
                if (is_collected(Constant::Y, shared.y, shared.x) & is_collected(Constant::X, shared.x, shared.x)) & hal.has_flag(EXTEND_COMPLEX_ENABLED) {
                    self.state = ExtendComplexCC1State::End;
                }
            }
            ExtendComplexCC1State::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == ExtendComplexCC1State::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtendComplexCC2State {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Start,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'C2'.
pub struct ExtendComplexCC2 {
    state: ExtendComplexCC2State,
}

impl ExtendComplexCC2 {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            state: ExtendComplexCC2State::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.state = ExtendComplexCC2State::Init;
    }

    /// Один такт автомата.
    fn tick(&mut self) {
        if self.state == ExtendComplexCC2State::Init {
            self.state = ExtendComplexCC2State::Start;
        }
        match self.state {
            ExtendComplexCC2State::Start => {
                self.state = ExtendComplexCC2State::End;
            }
            ExtendComplexCC2State::End => {}
            ExtendComplexCC2State::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == ExtendComplexCC2State::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtendComplexDState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Start,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'D'.
pub struct ExtendComplexD {
    state: ExtendComplexDState,
}

impl ExtendComplexD {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            state: ExtendComplexDState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.state = ExtendComplexDState::Init;
    }

    /// Один такт автомата.
    fn tick(&mut self) {
        if self.state == ExtendComplexDState::Init {
            self.state = ExtendComplexDState::Start;
        }
        match self.state {
            ExtendComplexDState::Start => {
                self.state = ExtendComplexDState::End;
            }
            ExtendComplexDState::End => {}
            ExtendComplexDState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == ExtendComplexDState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtendComplexEState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    End,
    Start,
}

/// Шаг последовательной композиции состояния 'Start'.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtendComplexEStartSeq {
    D0,
    F1,
}

/// Модель 'E'.
pub struct ExtendComplexE {
    state: ExtendComplexEState,
    /// Текущий шаг последовательной композиции состояния 'Start'.
    start_seq: ExtendComplexEStartSeq,
    start_d0: ExtendComplexD,
    start_f1: ExtendComplexF,
}

impl ExtendComplexE {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            state: ExtendComplexEState::Init,
            start_seq: ExtendComplexEStartSeq::D0,
            start_d0: ExtendComplexD::new(),
            start_f1: ExtendComplexF::new(),
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.state = ExtendComplexEState::Init;
        self.start_seq = ExtendComplexEStartSeq::D0;
        self.start_d0.init();
        self.start_f1.init();
    }

    /// Один такт автомата.
    fn tick(&mut self) {
        if self.state == ExtendComplexEState::Init {
            self.state = ExtendComplexEState::Start;
        }
        match self.state {
            ExtendComplexEState::End => {
            }
            ExtendComplexEState::Start => {
                if self.start_seq == ExtendComplexEStartSeq::D0 {
                    self.start_d0.tick();
                    if self.start_d0.is_done() {
                        self.start_f1.init();
                        self.start_seq = ExtendComplexEStartSeq::F1;
                    }
                } else if self.start_seq == ExtendComplexEStartSeq::F1 {
                    self.start_f1.tick();
                    if self.start_f1.is_done() {
                        self.state = ExtendComplexEState::End;
                    }
                }
            }
            ExtendComplexEState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == ExtendComplexEState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtendComplexFState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Start,
    /// Автомат завершён (`is_done`).
    End,
}

/// Модель 'F'.
pub struct ExtendComplexF {
    state: ExtendComplexFState,
}

impl ExtendComplexF {
    /// Создаёт модель в начальном состоянии.
    fn new() -> Self {
        Self {
            state: ExtendComplexFState::Init,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    fn init(&mut self) {
        self.state = ExtendComplexFState::Init;
    }

    /// Один такт автомата.
    fn tick(&mut self) {
        if self.state == ExtendComplexFState::Init {
            self.state = ExtendComplexFState::Start;
        }
        match self.state {
            ExtendComplexFState::Start => {
                self.state = ExtendComplexFState::End;
            }
            ExtendComplexFState::End => {}
            ExtendComplexFState::Init => {}
        }
    }

    /// Завершён ли автомат модели.
    fn is_done(&self) -> bool {
        self.state == ExtendComplexFState::End
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtendComplexState {
    /// Модель создана, но стартовое состояние ещё не занято.
    Init,
    Next,
    Start,
    /// Автомат завершён (`is_done`).
    End,
}

/// Шаг последовательной композиции состояния 'Start'.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtendComplexStartSeq {
    A0,
    B1,
    Group2,
    E3,
}

/// Общие переменные модели 'extend_complex', разделяемые под-моделями.
struct ExtendComplexShared {
    x: u8,
    y: u8,
}

/// Модель 'extend_complex'.
pub struct ExtendComplex<H: Hal> {
    /// Общие с под-моделями переменные (фича 0059).
    shared: ExtendComplexShared,
    state: ExtendComplexState,
    /// Текущий шаг последовательной композиции состояния 'Start'.
    start_seq: ExtendComplexStartSeq,
    next: ExtendComplexF,
    start_a0: ExtendComplexA,
    start_b1: ExtendComplexB,
    start_group2_c0: ExtendComplexC,
    start_group2_d1: ExtendComplexD,
    start_e3: ExtendComplexE,
    /// Аппаратный слой. Заменяет `void *userdata` цели `c`.
    hal: H,
}

impl<H: Hal> ExtendComplex<H> {
    /// Создаёт модель поверх аппаратного слоя `hal`.
    ///
    /// В отличие от цели `c`, забыть проставить доступ к железу
    /// невозможно: без `hal` модель не конструируется.
    pub fn new(hal: H) -> Self {
        Self {
            shared: ExtendComplexShared {
                x: 1,
                y: 2,
            },
            state: ExtendComplexState::Init,
            start_seq: ExtendComplexStartSeq::A0,
            next: ExtendComplexF::new(),
            start_a0: ExtendComplexA::new(),
            start_b1: ExtendComplexB::new(),
            start_group2_c0: ExtendComplexC::new(),
            start_group2_d1: ExtendComplexD::new(),
            start_e3: ExtendComplexE::new(),
            hal,
        }
    }

    /// Возвращает модель в начальное состояние.
    ///
    /// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход
    /// в стартовое состояние — это поведение, и оно живёт в `tick`.
    pub fn init(&mut self) {
        self.shared.x = 1;
        self.shared.y = 2;
        self.state = ExtendComplexState::Init;
        self.start_seq = ExtendComplexStartSeq::A0;
        self.next.init();
        self.start_a0.init();
        self.start_b1.init();
        self.start_group2_c0.init();
        self.start_group2_d1.init();
        self.start_e3.init();
    }

    /// Один такт автомата.
    ///
    /// Вход в стартовое состояние такта **не расходует** (контракт
    /// ADR 0033): его тело исполняется в этом же вызове.
    pub fn tick(&mut self) {
        if self.state == ExtendComplexState::Init {
            self.state = ExtendComplexState::Start;
        }
        match self.state {
            ExtendComplexState::Next => {
                self.next.tick();
                if self.next.is_done() {
                    self.state = ExtendComplexState::End;
                }
            }
            ExtendComplexState::Start => {
                assert!(self.start_e3.state != ExtendComplexEState::End);
                assert!(self.shared.x > 0);
                if self.start_seq == ExtendComplexStartSeq::A0 {
                    self.start_a0.tick();
                    if self.start_a0.is_done() {
                        self.start_b1.init();
                        self.start_seq = ExtendComplexStartSeq::B1;
                    }
                } else if self.start_seq == ExtendComplexStartSeq::B1 {
                    self.start_b1.tick(&mut self.hal);
                    if self.start_b1.is_done() {
                        self.start_group2_c0.init();
                        self.start_group2_d1.init();
                        self.start_seq = ExtendComplexStartSeq::Group2;
                    }
                } else if self.start_seq == ExtendComplexStartSeq::Group2 {
                    self.start_group2_c0.tick(&mut self.shared, &mut self.hal);
                    self.start_group2_d1.tick();
                    if self.start_group2_c0.is_done() && self.start_group2_d1.is_done() {
                        self.start_e3.init();
                        self.start_seq = ExtendComplexStartSeq::E3;
                    }
                } else if self.start_seq == ExtendComplexStartSeq::E3 {
                    self.start_e3.tick();
                    if self.start_e3.is_done() {
                        self.state = ExtendComplexState::Next;
                    }
                }
            }
            ExtendComplexState::End => {}
            ExtendComplexState::Init => {}
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
        self.state == ExtendComplexState::End
    }

}

