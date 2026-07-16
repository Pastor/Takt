//! Проверка порождённого Rust: модель `comprehensive` (`examples/comprehensive.lam`).
//!
//! Модель владеет аппаратным слоем (`Comprehensive::new(hal)`), поле `hal`
//! приватно и геттера нет — поэтому трасса собирается в общий `Rc<RefCell<_>>`,
//! а модели отдаётся дескриптор.
//!
//! ⚠️ ЗДЕСЬ ЗАКРЕПЛЕНО РЕАЛЬНОЕ ПОВЕДЕНИЕ, А НЕ ЗАЯВЛЕННОЕ. Пример дефектен
//! (фича 0030): сценарий `Idle → Heating → Cooling → Done` **недостижим по его
//! же логике** — из `Heating` выход требует `count >= MAX_COUNT` (10) либо
//! `temperature > MAX_TEMP` (100), а тело поднимает `count` лишь до 3 и
//! температуру не трогает. Автомат навсегда остаётся в `Heating`. Проверка
//! закрепляет это: на верной трансляции дефектного примера она обязана
//! проходить. Когда 0030 исправит пример, этот `main` упадёт — в этом и смысл.

use std::cell::RefCell;
use std::rc::Rc;

use lam_generated::comprehensive::{Comprehensive, Hal};

/// Записанные вызовы внешних функций модели.
#[derive(Default)]
struct Trace {
    temp: Vec<u8>,
    count: Vec<u8>,
}

/// Дескриптор трассы, отдаваемый модели.
struct Probe(Rc<RefCell<Trace>>);

impl Hal for Probe {
    fn log_count(&mut self, n: u8) {
        self.0.borrow_mut().count.push(n);
    }

    fn log_temp(&mut self, value: u8) {
        self.0.borrow_mut().temp.push(value);
    }
}

const TICKS: usize = 60;

fn main() {
    let trace = Rc::new(RefCell::new(Trace::default()));
    let mut m = Comprehensive::new(Probe(Rc::clone(&trace)));

    m.init();
    for _ in 0..TICKS {
        m.tick();
    }

    let t = trace.borrow();

    // `Idle` греет на 1 за такт и логирует температуру — такты 1..=11.
    assert_eq!(
        t.temp[..11],
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        "Idle обязан греть на 1 за такт и логировать температуру"
    );
    // На 11-м такте `temperature > 10` → `count := increment(count)`,
    // `temperature += 5` (16) и переход в `Heating`.
    assert!(
        t.temp[11..].iter().all(|&v| v == 16),
        "в Heating температура не меняется: 11 + boost 5 = 16 навсегда, получено {:?}",
        &t.temp[11..]
    );
    assert_eq!(t.temp.len(), TICKS, "log_temp обязан звать каждый такт");
    // `mode` — `Auto`, поэтому ветка `log_count` в `Heating` недостижима,
    // а `Cooling` (единственный другой её вызов) недостижим по дефекту 0030.
    assert!(t.count.is_empty(), "при mode = Auto log_count не зовётся");
    // Дефект 0030: `Done` недостижим, автомат вечно в `Heating`.
    assert!(
        !m.is_done(),
        "пример дефектен (фича 0030): Done недостижим — если автомат завершился, \
         значит пример исправлен и проверку надо переписать"
    );

    println!("comprehensive: OK ({TICKS} тактов, Heating не выпускает — дефект 0030 на месте)");
}
