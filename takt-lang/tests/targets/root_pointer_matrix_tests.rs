//! СПЛОШНОЙ сторож признака «нужен ли указатель на корень» (фича 0449).
//!
//! # Зачем перебор, а не отдельные входы
//!
//! Признак `c_needs` (фичи 0396, 0419, 0439) отвечает на вопрос, печатать ли
//! функции под-модели параметр `main`. Проверялся он **входами отдельных
//! фич** — то есть выборкой: каждая фича приносила свой случай и своего
//! сторожа. Здесь вопрос задаётся сплошь: каждый вид обращения к корню × каждая
//! форма реализации состояния, обе функции модели (`_init` и `_tick`).
//!
//! Первый же прогон нашёл дефект, которого выборка не видела: в профиле «часы»
//! выдержка `after Nms` сравнивает метку с `main->now_ms(…)` **в такте**, а
//! признак знал об этом только в `_init` — `cc` отвечал «use of undeclared
//! identifier 'main'» при нулевом коде возврата `taktc`.
//!
//! # Что именно проверяется
//!
//! 1. **Вывод собирается** `cc` флагами гейта цели — ловит ложное «указатель не
//!    нужен» (громкий класс: отказ чужого инструмента).
//! 2. **Признак точен**: там, где обращений нет, параметра в сигнатуре **тоже**
//!    нет — ловит ложное «нужен» (тихий класс: заглушка `(void)main;` делает
//!    вывод валидным, и `cc` молчит).
//!
//! ⚠️ Ожидания таблицы сняты **прогоном** (правило 30), а не выведены из кода
//! признака: сторож, повторяющий реализацию, доказывает лишь сам себя.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Вид обращения к корню — то, ради чего указатель и печатается.
#[derive(Clone, Copy, PartialEq)]
enum Touch {
    /// Обращений нет вовсе.
    None,
    /// Запись выходного порта: она идёт через HAL корня.
    PortWrite,
    /// Чтение переменной, объявленной в корне.
    SharedRead,
    /// Выходной порт с начальным значением: запись печатается в `_init`.
    PortInit,
    /// Инициализатор переменной читает объявление корня.
    VarInit,
    /// Порт пишет ФУНКЦИЯ модели — признак обязан быть транзитивным.
    Transitive,
    /// Профиль «часы» и выдержка `after Nms`: метка сравнивается с
    /// `main->now_ms(…)` и в такте, и при входе.
    ClockAfter,
    /// Вызов `extern fn`: печатается свободной функцией — контроль, что
    /// признак не срабатывает «на всякий случай».
    ExternCall,
}

impl Touch {
    fn name(self) -> &'static str {
        match self {
            Touch::None => "none",
            Touch::PortWrite => "port_write",
            Touch::SharedRead => "shared_read",
            Touch::PortInit => "port_init",
            Touch::VarInit => "var_init",
            Touch::Transitive => "transitive",
            Touch::ClockAfter => "clock_after",
            Touch::ExternCall => "extern_call",
        }
    }

    /// Объявления файла (корня), нужные этому виду.
    fn root_declarations(self) -> &'static str {
        match self {
            Touch::SharedRead | Touch::VarInit => "var shared: u8 := 3;\n\n",
            _ => "",
        }
    }

    /// Объявления модели, делающей обращение.
    fn declarations(self) -> &'static str {
        match self {
            Touch::PortWrite | Touch::Transitive => "    out a: u8;\n",
            Touch::PortInit => "    out a: u8 := 7;\n",
            Touch::VarInit => "    var seed: u8 := shared;\n",
            Touch::ExternCall => "    extern fn probe_value() -> u8;\n",
            _ => "",
        }
    }

    /// Функции модели, делающей обращение.
    fn functions(self) -> &'static str {
        match self {
            Touch::Transitive => {
                "    fn bump(v: u8) -> u8 {\n        a := v;\n        return v + 1;\n    }\n"
            }
            _ => "",
        }
    }

    /// Тело блока `always`.
    fn body(self) -> &'static str {
        match self {
            Touch::PortWrite => "            k := k + 1;\n            a := k;\n",
            Touch::SharedRead => "            k := k + shared;\n",
            Touch::VarInit => "            k := k + seed;\n",
            Touch::Transitive => "            k := bump(k);\n",
            Touch::ExternCall => "            k := probe_value();\n",
            _ => "            k := k + 1;\n",
        }
    }

    /// Переход из стартового состояния.
    fn transition(self) -> &'static str {
        match self {
            // Выдержка — единственный вид, которому нужен ход времени.
            Touch::ClockAfter => "        ref Done: after 5ms;\n",
            _ => "        next Done;\n",
        }
    }

    /// Ожидание: печатается ли `main` функциям `(_init, _tick)`.
    ///
    /// ⚠️ Снято прогоном 2026-08-31, а не выведено из кода признака.
    fn expects(self) -> (bool, bool) {
        match self {
            Touch::None | Touch::ExternCall => (false, false),
            Touch::PortWrite | Touch::SharedRead | Touch::Transitive => (false, true),
            Touch::PortInit | Touch::VarInit => (true, false),
            Touch::ClockAfter => (true, true),
        }
    }
}

/// Форма, которой состояние обёртки реализовано.
#[derive(Clone, Copy, PartialEq)]
enum Shape {
    /// Обычное состояние: обращение делает сама обёртка.
    Plain,
    /// `= First` — одна модель.
    Single,
    /// `= First | Second` — параллель.
    Parallel,
    /// `= First + Second` — цепочка.
    Chain,
    /// `= (First + Second) | Third` — вложенная композиция.
    Nested,
}

impl Shape {
    fn name(self) -> &'static str {
        match self {
            Shape::Plain => "plain",
            Shape::Single => "single",
            Shape::Parallel => "parallel",
            Shape::Chain => "chain",
            Shape::Nested => "nested",
        }
    }

    fn implementation(self) -> &'static str {
        match self {
            Shape::Plain => "",
            Shape::Single => "First",
            Shape::Parallel => "First | Second",
            Shape::Chain => "First + Second",
            Shape::Nested => "(First + Second) | Third",
        }
    }
}

/// Модель-спутник без единого обращения к корню.
fn plain_child(name: &str) -> String {
    format!(
        "model {name} {{\n    var k: u8 := 0;\n    start Go {{\n        always {{\n            k := k + 1;\n        }}\n        next Done;\n    }}\n    state Done;\n}}\n\n"
    )
}

/// Модель, делающая обращение вида `touch`.
fn touching_model(name: &str, touch: Touch) -> String {
    format!(
        "model {name} {{\n    var k: u8 := 0;\n{decl}{funcs}    start Go {{\n        always {{\n{body}        }}\n{transition}    }}\n    state Done;\n}}\n\n",
        decl = touch.declarations(),
        funcs = touch.functions(),
        body = touch.body(),
        transition = touch.transition(),
    )
}

/// Собирает исходник: обращение живёт в `First` (либо в самой обёртке).
fn source(shape: Shape, touch: Touch) -> String {
    let mut text = String::new();
    text.push_str(touch.root_declarations());
    if shape == Shape::Plain {
        // Обращение делает сама обёртка: спутники не нужны.
        text.push_str(&touching_model("Wrap", touch));
    } else {
        text.push_str(&touching_model("First", touch));
        text.push_str(&plain_child("Second"));
        text.push_str(&plain_child("Third"));
        text.push_str(&format!(
            "model Wrap {{\n    start Only = {};\n}}\n\n",
            shape.implementation()
        ));
    }
    text.push_str("start Main = Wrap;\n");
    text
}

/// Уникальный по тесту каталог (инвариант 0190/0429).
fn work_dir(tag: &str) -> PathBuf {
    let thread = std::thread::current()
        .name()
        .unwrap_or("main")
        .replace(':', "_");
    let dir = std::env::temp_dir()
        .join(format!("takt_pid{}", std::process::id()))
        .join(format!("takt_0449_{tag}_{thread}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог теста");
    dir
}

fn cc_available() -> bool {
    Command::new("cc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Компилирует случай целью `c`; отдаёт текст порождённого файла.
fn compile(dir: &Path, source: &str) -> String {
    let input = dir.join("probe.takt");
    std::fs::write(&input, source).expect("запись пробы");
    let out = Command::new(env!("CARGO_BIN_EXE_taktc"))
        .arg("compile")
        .args(["-t", "c"])
        .arg(&input)
        .arg("-o")
        .arg(dir.join("out"))
        .output()
        .expect("запуск taktc compile");
    assert!(
        out.status.success(),
        "цель `c` обязана перевести вход:\n{}\n--- исходник ---\n{source}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::read_to_string(dir.join("out").join("probe.c")).expect("порождённый файл читается")
}

/// Собирает порождённый файл флагами гейта цели.
fn build(dir: &Path) -> Result<(), String> {
    let cc = Command::new("cc")
        .args([
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Wno-unused-parameter",
            "-Werror",
            "-c",
        ])
        .arg(dir.join("out").join("probe.c"))
        .arg("-I")
        .arg(dir.join("out"))
        .arg("-o")
        .arg(dir.join("probe.o"))
        .output()
        .expect("запуск cc");
    if cc.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&cc.stderr).into_owned())
    }
}

/// Есть ли у функции параметр-указатель на корень.
///
/// Ищется **прототип**: он печатается первым и в единственном экземпляре.
fn has_root_parameter(text: &str, function: &str) -> bool {
    let needle = format!("static void {function}(");
    let line = text
        .lines()
        .find(|l| l.contains(&needle))
        .unwrap_or_else(|| panic!("в выводе нет прототипа '{function}':\n{text}"));
    line.contains("Probe *main")
}

/// Сплошной перебор: каждый вид обращения × каждая форма реализации.
#[test]
fn root_pointer_is_exact_for_every_shape_and_touch() {
    if !cc_available() {
        eprintln!("cc недоступен — сплошной сторож пропущен");
        return;
    }
    const TOUCHES: [Touch; 8] = [
        Touch::None,
        Touch::PortWrite,
        Touch::SharedRead,
        Touch::PortInit,
        Touch::VarInit,
        Touch::Transitive,
        Touch::ClockAfter,
        Touch::ExternCall,
    ];
    const SHAPES: [Shape; 5] = [
        Shape::Plain,
        Shape::Single,
        Shape::Parallel,
        Shape::Chain,
        Shape::Nested,
    ];

    let mut failures: Vec<String> = Vec::new();
    for shape in SHAPES {
        for touch in TOUCHES {
            let tag = format!("{}_{}", shape.name(), touch.name());
            let dir = work_dir(&tag);
            let text = compile(&dir, &source(shape, touch));
            if let Err(err) = build(&dir) {
                failures.push(format!("{tag}: cc отверг вывод:\n{err}"));
                continue;
            }
            let (init_expected, tick_expected) = touch.expects();
            let init_actual = has_root_parameter(&text, "ProbeWrap_init");
            let tick_actual = has_root_parameter(&text, "ProbeWrap_tick");
            if init_actual != init_expected {
                failures.push(format!(
                    "{tag}: `_init` {} указатель, ожидалось «{}»",
                    if init_actual {
                        "получил"
                    } else {
                        "не получил"
                    },
                    if init_expected {
                        "получит"
                    } else {
                        "не получит"
                    }
                ));
            }
            if tick_actual != tick_expected {
                failures.push(format!(
                    "{tag}: `_tick` {} указатель, ожидалось «{}»",
                    if tick_actual {
                        "получил"
                    } else {
                        "не получил"
                    },
                    if tick_expected {
                        "получит"
                    } else {
                        "не получит"
                    }
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "признак «нужен ли указатель на корень» разошёлся с ожиданием в {} случаях из {}:\n{}",
        failures.len(),
        SHAPES.len() * TOUCHES.len(),
        failures.join("\n")
    );
}
