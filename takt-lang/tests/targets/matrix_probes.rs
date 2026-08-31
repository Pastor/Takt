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

/// Форма ОБЪЯВЛЕНИЯ того, к чему идёт обращение (фича 0451).
///
/// Ось заведена потому, что тип меняет и путь печати, и границы целей:
/// массив и структура в порту **разворачиваются по листам** (фичи 0350, 0417),
/// перечисление в порту `rust` и `st-at` не размещают вовсе, а инициализатор
/// массива от другой переменной цель `c` не выражает.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Kind {
    /// Скаляр `u8`.
    Scalar,
    /// Массив `[u8; 3]`.
    Array,
    /// Структура из двух полей.
    Struct,
    /// Перечисление из двух вариантов.
    Enum,
}

/// Формы объявления — перебор идёт по ним целиком.
pub(crate) const KINDS: [Kind; 4] = [Kind::Scalar, Kind::Array, Kind::Struct, Kind::Enum];

impl Kind {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Kind::Scalar => "scalar",
            Kind::Array => "array",
            Kind::Struct => "struct",
            Kind::Enum => "enum",
        }
    }

    /// Имя типа в объявлении.
    fn type_name(self) -> &'static str {
        match self {
            Kind::Scalar => "u8",
            Kind::Array => "[u8; 3]",
            Kind::Struct => "Pair",
            Kind::Enum => "Mode",
        }
    }

    /// Объявление типа, если оно нужно.
    fn type_declaration(self) -> &'static str {
        match self {
            Kind::Struct => "struct Pair {\n    lo: u8,\n    hi: u8\n}\n\n",
            Kind::Enum => "enum Mode {\n    Idle = 1,\n    Work = 2\n}\n\n",
            _ => "",
        }
    }

    /// Значение для инициализатора.
    fn literal(self) -> &'static str {
        match self {
            Kind::Scalar => "7",
            Kind::Array => "{4, 5, 6}",
            Kind::Struct => "{4, 5}",
            Kind::Enum => "Work",
        }
    }

    /// Начальное значение объявления в корне.
    fn root_literal(self) -> &'static str {
        match self {
            Kind::Scalar => "3",
            Kind::Array => "{1, 2, 3}",
            Kind::Struct => "{1, 2}",
            Kind::Enum => "Idle",
        }
    }

    /// Оператор, читающий значение `name` и прибавляющий его к `k`.
    fn read_statement(self, name: &str) -> String {
        match self {
            Kind::Scalar => format!("            k := k + {name};\n"),
            Kind::Array => format!("            k := k + {name}[1];\n"),
            // ⚠️ Читаются ОБА поля: у цели `sv` структурный порт остаётся
            // одним сигналом (0390), и `verilator` под `-Wall` считает
            // непрочитанные биты ошибкой. Проба проверяет транспорт значения,
            // а не частичное использование (замер 0452 — тот класс вынесен
            // кандидатом).
            Kind::Struct => format!("            k := k + {name}.hi + {name}.lo;\n"),
            // У перечисления арифметики нет: читается СРАВНЕНИЕМ.
            Kind::Enum => {
                format!(
                    "            if {name} = Work {{\n                k := k + 2;\n            }}\n"
                )
            }
        }
    }

    /// Оператор, читающий ОДНУ часть значения (фича 0453).
    fn partial_read_statement(self, name: &str) -> String {
        match self {
            Kind::Struct => format!("            k := k + {name}.hi;\n"),
            Kind::Array => format!("            k := k + {name}[0];\n"),
            // У скаляра и перечисления частей нет: чтение полное.
            _ => self.read_statement(name),
        }
    }

    /// Оператор, пишущий в `name` значение, зависящее от такта.
    fn write_statement(self, name: &str) -> String {
        match self {
            Kind::Scalar => format!("            {name} := k;\n"),
            Kind::Array => format!("            {name}[1] := k;\n"),
            Kind::Struct => format!("            {name}.hi := k;\n"),
            Kind::Enum => format!("            {name} := Work;\n"),
        }
    }
}

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
    /// Чтение ВХОДНОГО порта: значение приходит извне через HAL корня.
    PortRead,
    /// Чтение ДВУНАПРАВЛЕННОГО порта: у цели `sv` это отдельный сигнал `_i`
    /// (фича 0428).
    InoutRead,
    /// Запись двунаправленного порта: сигналы `_o` и строб `_we`.
    InoutWrite,
    /// Инвариант УРОВНЯ МОДЕЛИ: `invariant Имя = условие;` — сахар над `cond`
    /// плюс охранная формула (фича 0044).
    InvariantModel,
    /// Инвариант в теле СОСТОЯНИЯ: то же обязательство, другое место
    /// объявления (одно из шести, фича 0203).
    InvariantState,
    /// Охранная формула краткой формой: `: условие;`.
    GuardFormula,
    /// Темпоральное свойство `: [LTL] φ;` — до целей оно не доезжает вовсе
    /// (предмет верификации), и это тоже часть таблицы.
    LtlFormula,
    /// Вызов функции из ПОДКЛЮЧЁННОГО файла (`import "…";`).
    ImportFunction,
    /// То же выборочным импортом: `import { twice } from "…";`.
    ImportSelective,
    /// Тип (структура) из подключённого файла.
    ImportType,
    /// Модель подключённого файла реализует состояние обёртки — выборочным
    /// импортом, потому что полный вносит только контейнер файла.
    ImportModel,
    /// Полный импорт + попытка взять ВЛОЖЕННУЮ модель: законный отказ
    /// (`SE-106`), и он часть таблицы.
    ImportNestedModel,
    /// ТРАНЗИТИВНЫЙ импорт: подключённый файл сам подключает третий.
    ImportTransitive,
    /// Модель с ПАРАМЕТРОМ, взятая с настройкой по умолчанию (фича 0185).
    ParameterDefault,
    /// То же с аргументом в месте инстанцирования: `M(portion := 7)`.
    ParameterArgument,
    /// Аргумент — константное ВЫРАЖЕНИЕ: `M(portion := BASE + 2)`.
    ParameterExpression,
    /// Адрес порта задан оператором `address`, а не inline (фича 0020).
    AddressOperator,
    /// Оператор `address` с ПОЗИЦИЕЙ БИТА: `0x…:3`.
    AddressBit,
    /// Адрес — константное ВЫРАЖЕНИЕ (арифметика вычисляется компилятором).
    AddressExpression,
    /// Адрес приходит ВНЕШНЕЙ КАРТОЙ (`--address-map`) и перекрывает inline —
    /// приоритет источников: inline < `address` < карта (фича 0020).
    AddressMap,
    /// Адрес опирается на `-D` определение среды вычислителя (фича 0042).
    AddressDefine,
    /// ТАКТОВАЯ выдержка `after Nt` — счёт тактов, а не миллисекунд.
    TimeAfterTicks,
    /// Периодический блок `every Nms`.
    TimeEvery,
    /// Выдержка от ПЕРЕМЕННОЙ типа `duration` — вычисляемая (фича 0183).
    TimeDurationVar,
    /// Вычисляемая выдержка выражением: `after (SETTLE + 500ms)`.
    TimeComputed,
    /// Модель ОБЪЯВЛЯЕТ такт: `clock 1kHz;` — флаг обязан совпасть (0134).
    TimeClockDeclared,
    /// ЧАСТИЧНОЕ чтение входного порта: у составного значения читается одна
    /// часть (фича 0453).
    ///
    /// ⚠️ Модель вправе так делать, а структурный порт цель `sv` печатает
    /// одним сигналом (0390) — непрочитанные биты `verilator` под `-Wall`
    /// считает ошибкой. Вид заведён именно ради этого случая.
    PortReadPartial,
}

/// Все виды обращения — перебор идёт по ним целиком.
pub(crate) const TOUCHES: [Touch; 35] = [
    Touch::None,
    Touch::PortWrite,
    Touch::SharedRead,
    Touch::PortInit,
    Touch::VarInit,
    Touch::Transitive,
    Touch::ClockAfter,
    Touch::ExternCall,
    Touch::PortRead,
    Touch::InoutRead,
    Touch::InoutWrite,
    Touch::PortReadPartial,
    Touch::InvariantModel,
    Touch::InvariantState,
    Touch::GuardFormula,
    Touch::LtlFormula,
    Touch::ImportFunction,
    Touch::ImportSelective,
    Touch::ImportType,
    Touch::ImportModel,
    Touch::ImportNestedModel,
    Touch::ImportTransitive,
    Touch::ParameterDefault,
    Touch::ParameterArgument,
    Touch::ParameterExpression,
    Touch::AddressOperator,
    Touch::AddressBit,
    Touch::AddressExpression,
    Touch::AddressMap,
    Touch::AddressDefine,
    Touch::TimeAfterTicks,
    Touch::TimeEvery,
    Touch::TimeDurationVar,
    Touch::TimeComputed,
    Touch::TimeClockDeclared,
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
            Touch::PortRead => "port_read",
            Touch::InoutRead => "inout_read",
            Touch::InoutWrite => "inout_write",
            Touch::PortReadPartial => "port_read_partial",
            Touch::InvariantModel => "invariant_model",
            Touch::InvariantState => "invariant_state",
            Touch::GuardFormula => "guard_formula",
            Touch::LtlFormula => "ltl_formula",
            Touch::ImportFunction => "import_function",
            Touch::ImportSelective => "import_selective",
            Touch::ImportType => "import_type",
            Touch::ImportModel => "import_model",
            Touch::ImportNestedModel => "import_nested_model",
            Touch::ImportTransitive => "import_transitive",
            Touch::ParameterDefault => "parameter_default",
            Touch::ParameterArgument => "parameter_argument",
            Touch::ParameterExpression => "parameter_expression",
            Touch::AddressOperator => "address_operator",
            Touch::AddressBit => "address_bit",
            Touch::AddressExpression => "address_expression",
            Touch::AddressMap => "address_map",
            Touch::AddressDefine => "address_define",
            Touch::TimeAfterTicks => "time_after_ticks",
            Touch::TimeEvery => "time_every",
            Touch::TimeDurationVar => "time_duration_var",
            Touch::TimeComputed => "time_computed",
            Touch::TimeClockDeclared => "time_clock_declared",
        }
    }

    /// Значима ли для этого вида форма объявления.
    ///
    /// У обращений, которые не касаются объявленного значения (`none`,
    /// `clock_after`, `extern_call`), тип перебирать нечего — они идут
    /// однажды, со скаляром.
    pub(crate) fn varies_by_kind(self) -> bool {
        matches!(
            self,
            Touch::PortWrite
                | Touch::SharedRead
                | Touch::PortInit
                | Touch::VarInit
                | Touch::PortRead
                | Touch::InoutRead
                | Touch::InoutWrite
                | Touch::PortReadPartial
        )
    }

    /// Объявления файла (корня), нужные этому виду.
    fn root_declarations(self, kind: Kind) -> String {
        match self {
            Touch::SharedRead | Touch::VarInit => format!(
                "var shared: {} := {};\n\n",
                kind.type_name(),
                kind.root_literal()
            ),
            _ => String::new(),
        }
    }

    /// Объявления модели, делающей обращение.
    fn declarations(self, kind: Kind) -> String {
        match self {
            Touch::PortWrite | Touch::Transitive => {
                format!("    out a: {} at 0x40000100;\n", kind.type_name())
            }
            Touch::PortInit => format!(
                "    out a: {} at 0x40000100 := {};\n",
                kind.type_name(),
                kind.literal()
            ),
            Touch::VarInit => format!("    var seed: {} := shared;\n", kind.type_name()),
            Touch::PortRead | Touch::PortReadPartial => {
                format!("    in a: {} at 0x40000100;\n", kind.type_name())
            }
            // Адрес задаётся ОТДЕЛЬНО (оператором либо картой), поэтому у
            // объявления его нет. ⚠️ Оператор `address` действует в области
            // СВОЕГО объявления — он стоит рядом с портом (замер 0458).
            Touch::AddressOperator => "    out a: u8;\n    address a = 0x40000200;\n".to_string(),
            Touch::AddressBit => "    out a: bit;\n    address a = 0x40000004:3;\n".to_string(),
            Touch::AddressExpression => {
                "    out a: u8;\n    address a = 0x40000000 + 8;\n".to_string()
            }
            // Карта и определение приходят снаружи: объявление обычное.
            Touch::AddressMap => "    out a: u8 at 0x40000100;\n".to_string(),
            Touch::AddressDefine => "    out a: u8;\n    address a = BASE + 4;\n".to_string(),
            // Время: выдержка от переменной и от константного выражения — две
            // формы вычисляемой выдержки (фича 0183).
            Touch::TimeDurationVar => "    var hold: duration := 5ms;\n".to_string(),
            Touch::TimeComputed => "    const SETTLE: duration := 5ms;\n".to_string(),
            // Модель объявляет такт устройства: флаг `--tick-hz` обязан совпасть
            // (контракт 0134, `SE-069`/`SE-070`).
            Touch::TimeClockDeclared => "    clock 1kHz;\n".to_string(),
            Touch::InoutRead | Touch::InoutWrite => {
                format!("    inout a: {} at 0x40000100;\n", kind.type_name())
            }
            Touch::ExternCall => "    extern fn probe_value() -> u8;\n".to_string(),
            // Обязательства уровня модели: инвариант — сахар над `cond` плюс
            // охранная формула (0044), краткая форма — та же формула без имени.
            Touch::InvariantModel => "    invariant Bound = k < 200;\n".to_string(),
            Touch::GuardFormula => "    : k < 200;\n".to_string(),
            // Темпоральное свойство опирается на ИМЕНОВАННОЕ условие: атом
            // формулы обязан иметь имя (правило 0049).
            Touch::LtlFormula => "    cond Low = k < 200;\n    : [LTL] G Low;\n".to_string(),
            // Тип из подключённого файла — объявление обёртки.
            Touch::ImportType => "    var p: Pair := {1, 2};\n".to_string(),
            Touch::None
            | Touch::SharedRead
            | Touch::ClockAfter
            | Touch::InvariantState
            | Touch::ImportFunction
            | Touch::ImportSelective
            | Touch::ImportModel
            | Touch::ImportNestedModel
            | Touch::ImportTransitive
            // Виды с параметром строят свой исходник целиком (см. `source`).
            | Touch::ParameterDefault
            | Touch::ParameterArgument
            | Touch::ParameterExpression
            // Тактовая выдержка и периодический блок объявлений не требуют.
            | Touch::TimeAfterTicks
            | Touch::TimeEvery => String::new(),
        }
    }

    /// Функции модели, делающей обращение.
    fn functions(self, kind: Kind) -> String {
        match self {
            Touch::Transitive => format!(
                "    fn bump(v: u8) -> u8 {{\n{}        return v + 1;\n    }}\n",
                kind.write_statement("a")
                    .replace("            ", "        ")
                    .replace("k;", "v;")
            ),
            _ => String::new(),
        }
    }

    /// Тело блока `always`.
    fn body(self, kind: Kind) -> String {
        match self {
            Touch::PortWrite => format!("            k := k + 1;\n{}", kind.write_statement("a")),
            Touch::SharedRead => kind.read_statement("shared"),
            Touch::VarInit => kind.read_statement("seed"),
            Touch::Transitive => "            k := bump(k);\n".to_string(),
            Touch::PortRead | Touch::InoutRead => kind.read_statement("a"),
            // Читается ОДНА часть значения: у скаляра и перечисления это то же
            // самое, что полное чтение, а у массива и структуры — нет.
            Touch::PortReadPartial => kind.partial_read_statement("a"),
            Touch::ImportFunction | Touch::ImportSelective => {
                "            k := twice(k) + 1;\n".to_string()
            }
            Touch::ImportType => "            k := k + p.hi;\n".to_string(),
            Touch::ImportTransitive => "            k := mid_value();\n".to_string(),
            // Однобитному порту пишется бит, прочим — счётчик.
            Touch::AddressBit => "            k := k + 1;\n            a := 1;\n".to_string(),
            Touch::AddressOperator
            | Touch::AddressExpression
            | Touch::AddressMap
            | Touch::AddressDefine => "            k := k + 1;\n            a := k;\n".to_string(),
            Touch::InoutWrite => format!("            k := k + 1;\n{}", kind.write_statement("a")),
            Touch::ExternCall => "            k := probe_value();\n".to_string(),
            _ => "            k := k + 1;\n".to_string(),
        }
    }

    /// Переход из стартового состояния.
    fn transition(self) -> &'static str {
        match self {
            // Выдержка — вид, которому нужен ход времени.
            Touch::ClockAfter | Touch::TimeClockDeclared => "        ref Done: after 5ms;\n",
            Touch::TimeAfterTicks => "        ref Done: after 5t;\n",
            Touch::TimeDurationVar => "        ref Done: after hold;\n",
            Touch::TimeComputed => "        ref Done: after (SETTLE + 500ms);\n",
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

/// Модель, делающая обращение вида `touch` над значением формы `kind`.
fn touching_model(name: &str, touch: Touch, kind: Kind) -> String {
    // Инвариант СОСТОЯНИЯ объявляется внутри состояния — это другое из шести
    // мест объявления формулы (фича 0203).
    let in_state = match touch {
        Touch::InvariantState => "        invariant InState = k < 200;\n",
        // Периодический блок объявляется В СОСТОЯНИИ — ещё одно из мест, где
        // живёт время (фича 0134).
        Touch::TimeEvery => "        every 3ms {\n            k := k + 2;\n        }\n",
        _ => "",
    };
    format!(
        "model {name} {{\n    var k: u8 := 0;\n{decl}{funcs}    start Go {{\n{in_state}        always {{\n{body}        }}\n{transition}    }}\n    state Done;\n}}\n\n",
        decl = touch.declarations(kind),
        funcs = touch.functions(kind),
        body = touch.body(kind),
        transition = touch.transition(),
    )
}

/// Файл пробы: имя (без каталога) и содержимое.
pub(crate) struct ProbeFile {
    /// Имя файла — им же зовётся модель-контейнер (правило «имя корневой
    /// модели берётся из имени файла»).
    pub(crate) name: &'static str,
    /// Содержимое.
    pub(crate) text: String,
}

/// Подключаемые файлы случая: пусто у всех видов, кроме импортов (фича 0456).
///
/// ⚠️ Имена библиотек — `helper.takt` и `base.takt`: имя файла становится
/// именем модели-контейнера, и совпадение с именем пробы (`probe`) дало бы
/// столкновение, а не проверку импорта.
pub(crate) fn library_files(touch: Touch) -> Vec<ProbeFile> {
    let helper = |extra: &str| ProbeFile {
        name: "helper.takt",
        text: format!(
            "struct Pair {{\n    lo: u8,\n    hi: u8\n}}\n\nconst CAP: u8 := 9;\n\nfn twice(v: u8) -> u8 {{\n    return v * 2;\n}}\n\nmodel Engine {{\n    var h: u8 := 0;\n    start Work {{\n        always {{\n            h := h + 1;\n        }}\n        next Done;\n    }}\n    state Done;\n}}\n{extra}"
        ),
    };
    match touch {
        Touch::ImportFunction
        | Touch::ImportSelective
        | Touch::ImportType
        | Touch::ImportModel
        | Touch::ImportNestedModel => vec![helper("")],
        // Внешняя карта адресов лежит рядом с пробой и перекрывает inline.
        Touch::AddressMap => vec![ProbeFile {
            name: "plat.map",
            text: "a = 0x00200004;\n".to_string(),
        }],
        // Транзитивный импорт: подключённый файл сам подключает третий.
        Touch::ImportTransitive => vec![
            ProbeFile {
                name: "base.takt",
                text: "fn base_value() -> u8 {\n    return 3;\n}\n".to_string(),
            },
            ProbeFile {
                name: "mid.takt",
                text: "import \"base.takt\";\n\nfn mid_value() -> u8 {\n    return base_value() + 1;\n}\n"
                    .to_string(),
            },
        ],
        _ => Vec::new(),
    }
}

/// Аргумент инстанцирования для видов с параметром; `None` — вид не про них.
///
/// ⚠️ Форма реализации (`Shape`) к этим видам не применяется: модель-донор
/// **сама** и есть реализация состояния обёртки.
fn parameter_argument(touch: Touch) -> Option<&'static str> {
    match touch {
        Touch::ParameterDefault => Some(""),
        Touch::ParameterArgument => Some("(portion := 7)"),
        Touch::ParameterExpression => Some("(portion := BASE + 2)"),
        _ => None,
    }
}

/// Дополнительные ключи CLI, которых требует вид обращения (фича 0458).
pub(crate) fn extra_flags(touch: Touch) -> Vec<String> {
    match touch {
        // ⚠️ Карта передаётся ПУТЁМ ОТ РАБОЧЕГО КАТАЛОГА процесса, а не от
        // каталога пробы (в отличие от `import`, который ищется рядом с
        // импортёром, — правило 0055). Путь подставляет вызывающий: он один
        // знает каталог случая.
        Touch::AddressMap => vec!["--address-map".to_string(), "{dir}/plat.map".to_string()],
        // Среда вычислителя адреса: имя видно только ему (фича 0042).
        Touch::AddressDefine => vec!["-DBASE=0x40000000".to_string()],
        // ⚠️ Модель объявила такт устройства — флаг ОБЯЗАН совпасть, иначе
        // `SE-069`/`SE-070` (контракт 0134). Это часть случая, а не настройка.
        Touch::TimeClockDeclared => vec!["--tick-hz=1000".to_string()],
        _ => Vec::new(),
    }
}

/// Строка подключения для вида обращения.
fn import_line(touch: Touch) -> &'static str {
    match touch {
        Touch::ImportFunction | Touch::ImportType | Touch::ImportNestedModel => {
            "import \"helper.takt\";\n\n"
        }
        Touch::ImportSelective => "import { twice } from \"helper.takt\";\n\n",
        Touch::ImportModel => "import { Engine } from \"helper.takt\";\n\n",
        Touch::ImportTransitive => "import \"mid.takt\";\n\n",
        _ => "",
    }
}

/// Исходник случая: обращение живёт в `First` (либо в самой обёртке).
pub(crate) fn source(shape: Shape, touch: Touch, kind: Kind) -> String {
    let mut text = String::new();
    text.push_str(import_line(touch));
    text.push_str(kind.type_declaration());
    text.push_str(&touch.root_declarations(kind));
    // Модель с параметром: донор объявляет `parameter`, обёртка берёт его
    // реализацией — с настройкой по умолчанию либо с аргументом (фича 0185).
    if let Some(argument) = parameter_argument(touch) {
        text.push_str("const BASE: u8 := 4;\n\n");
        text.push_str(
            "model Feeder {\n    parameter portion: u8 := 3;\n    var k: u8 := 0;\n    out led: u8 at 0x40000100;\n    start Go {\n        always {\n            k := k + portion;\n            led := k;\n        }\n        next Done;\n    }\n    state Done;\n}\n\n",
        );
        text.push_str(&format!(
            "model Wrap {{\n    start Only = Feeder{argument};\n}}\n\nstart Main = Wrap;\n"
        ));
        return text;
    }
    // Модель подключённого файла реализует состояние обёртки — формы
    // реализации к ней не применяются: она сама и есть реализация.
    if matches!(touch, Touch::ImportModel | Touch::ImportNestedModel) {
        let implementation = if touch == Touch::ImportModel {
            "Engine"
        } else {
            // Полный импорт вносит только контейнер файла; вложенная модель
            // снаружи не видна — законный отказ, часть таблицы.
            "Helper"
        };
        text.push_str(&format!(
            "model Wrap {{\n    start Only = {implementation};\n}}\n\n"
        ));
        text.push_str("start Main = Wrap;\n");
        return text;
    }
    if shape == Shape::Plain {
        // Обращение делает сама обёртка: спутники не нужны.
        text.push_str(&touching_model("Wrap", touch, kind));
    } else {
        text.push_str(&touching_model("First", touch, kind));
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

/// Все случаи перебора: форма реализации × вид обращения × форма объявления.
///
/// ⚠️ Форма объявления перебирается только у тех видов, которые её касаются
/// (`varies_by_kind`): у `none`, `clock_after` и `extern_call` объявленного
/// значения нет вовсе, и четыре одинаковых прогона были бы просто платой за
/// время.
pub(crate) fn cases() -> Vec<(Shape, Touch, Kind)> {
    let mut out = Vec::new();
    for shape in SHAPES {
        for touch in TOUCHES {
            if touch.varies_by_kind() {
                for kind in KINDS {
                    out.push((shape, touch, kind));
                }
            } else {
                out.push((shape, touch, Kind::Scalar));
            }
        }
    }
    out
}

/// Имя случая — оно же тег каталога и строка отчёта.
pub(crate) fn case_name(shape: Shape, touch: Touch, kind: Kind) -> String {
    if touch.varies_by_kind() {
        format!("{}_{}_{}", shape.name(), touch.name(), kind.name())
    } else {
        format!("{}_{}", shape.name(), touch.name())
    }
}
