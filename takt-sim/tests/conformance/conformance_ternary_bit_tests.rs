//! Потактовая сверка тернарника в битовом выходе цели Rust.

use std::process::Command;

const SOURCE: &str = "
model T {
    var n: u32 := 7;
    out late: bit := 0;
    start Idle {
        always {
            n := n - 1;
            late := (n > 5) ? 1 : 0;
        }
        ref Idle: false;
    }
}
start Main = T;
";

/// Оба исхода условия дают ту же трассу, что и эталон.
#[test]
fn ternary_bit_port_matches_simulator() {
    let (ast, _) = takt_lang::parse(SOURCE, 0).expect("разбор");
    let model = takt_lang::semantic::tree::construct_model(&ast, None, &[]).expect("семантика");
    let mut unit = takt_sim::build_unit(model).expect("сборка эталона");
    let reference: Vec<_> = (0..3)
        .map(|_| {
            let tick = unit.tick();
            assert!(!matches!(tick, takt_sim::TickResult::Failed(_)), "{tick:?}");
            match unit.variable("late") {
                Some(takt_sim::Value::Boolean(value)) => i128::from(value),
                Some(takt_sim::Value::Number(value)) => value,
                other => panic!("неожиданное значение порта late: {other:?}"),
            }
        })
        .collect();
    assert_eq!(reference, [1, 0, 0]);

    let dir = tempfile::tempdir().expect("каталог генерации");
    takt_lang::compile_to_rust(
        "ternary_bit",
        SOURCE,
        dir.path().to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect("порождение Rust");

    std::fs::write(
        dir.path().join("driver.rs"),
        r#"
mod ternary_bit;
use ternary_bit::{Hal, OutBitPort, TernaryBit};
use std::{cell::Cell, rc::Rc};

struct Probe(Rc<Cell<bool>>);

impl Hal for Probe {
    fn write_bit(&mut self, port: OutBitPort, value: bool) {
        match port {
            OutBitPort::Late => self.0.set(value),
        }
    }
}

fn main() {
    let late = Rc::new(Cell::new(false));
    let mut model = TernaryBit::new(Probe(Rc::clone(&late)));
    model.init();
    for _ in 0..3 {
        model.tick();
        println!("{}", u8::from(late.get()));
    }
    model.reset();
    assert!(!model.is_done());
}
"#,
    )
    .expect("запись драйвера");

    for tool in ["rustc", "clippy-driver"] {
        let build = Command::new(tool)
            .current_dir(dir.path())
            .args([
                "--edition",
                "2021",
                "-D",
                "warnings",
                "driver.rs",
                "-o",
                "driver",
            ])
            .output()
            .unwrap_or_else(|err| panic!("запуск {tool}: {err}"));
        assert!(
            build.status.success(),
            "{tool}: {}",
            String::from_utf8_lossy(&build.stderr)
        );
    }
    let run = Command::new(dir.path().join("driver"))
        .output()
        .expect("запуск драйвера");
    assert!(run.status.success(), "драйвер завершился с ошибкой");
    let generated: Vec<i128> = std::str::from_utf8(&run.stdout)
        .expect("вывод в UTF-8")
        .lines()
        .map(|line| line.parse().expect("бинарное значение"))
        .collect();
    assert_eq!(generated, reference);
}
