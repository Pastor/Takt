//! Генератор матрицы проб: вид обращения к корню × форма реализации состояния.
//!
//! Носитель **общий** у двух сторожей (правило «одно правило — один носитель»):
//!
//! - [`root_pointer_matrix_tests`](super::root_pointer_matrix_tests) — точность
//!   признака «нужен ли указатель на корень» у цели `c` (фича 0449);
//! - [`target_matrix_tests`](super::target_matrix_tests) — та же матрица через
//!   **все восемь** целей и их инструменты (фича 0450).
//!
//! Разъехавшись, два генератора дали бы двум сторожам разные входы, и таблица
//! ожиданий одного перестала бы говорить о другом.
//!
//! ⚠️ Порты объявляются **с адресом**: он нужен целям `c-hal` и `st-at` (иначе
//! `SE-052`), а прочим безразличен — проверено прогоном (сигнатуры цели `c` от
//! адреса не меняются).

/// Вид обращения к корню — то, ради чего указатель и печатается.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Touch {
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

/// Все виды обращения — перебор идёт по ним целиком.
pub(crate) const TOUCHES: [Touch; 8] = [
    Touch::None,
    Touch::PortWrite,
    Touch::SharedRead,
    Touch::PortInit,
    Touch::VarInit,
    Touch::Transitive,
    Touch::ClockAfter,
    Touch::ExternCall,
];

impl Touch {
    pub(crate) fn name(self) -> &'static str {
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
            Touch::PortWrite | Touch::Transitive => "    out a: u8 at 0x40000100;\n",
            Touch::PortInit => "    out a: u8 at 0x40000100 := 7;\n",
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
}

/// Форма, которой состояние обёртки реализовано.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Shape {
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

/// Все формы реализации.
pub(crate) const SHAPES: [Shape; 5] = [
    Shape::Plain,
    Shape::Single,
    Shape::Parallel,
    Shape::Chain,
    Shape::Nested,
];

impl Shape {
    pub(crate) fn name(self) -> &'static str {
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

/// Исходник случая: обращение живёт в `First` (либо в самой обёртке).
pub(crate) fn source(shape: Shape, touch: Touch) -> String {
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
