//! Ключевые слова ЦЕЛЕВЫХ языков — один носитель на весь проект (фича 0531,
//! задача 06).
//!
//! # Зачем список живёт отдельно от того, кто им судит
//!
//! Списки заведены генераторами ради отказа на столкновении имён: `SV-012`
//! («имя — ключевое слово SystemVerilog») и `RS-004` («идентификатор
//! непредставим в Rust»). Ровно те же слова нужны **подсветке вывода** в
//! онлайн-редакторе: вкладка цели красит порождённый код по правилам его
//! языка (требование R11 фичи 0531).
//!
//! Второй экземпляр списка разошёлся бы с первым молча — это класс 0084/0193,
//! и цена такого расхождения измерена на соседнем языке: список
//! зарезервированных имён IEC, собранный чтением стандарта, отстал на двадцать
//! имён (фича 0342), а прогон 2026-09-03 добавил ещё 33 (фича 0511). Поэтому
//! слова лежат здесь, а потребители — судья имён и подсветка — спрашивают их.
//!
//! ⚠️ Языка **Takt** здесь нет и быть не может: его лексика живёт в лексере
//! (`parser/lexer.rs`), а веб-редактор красит исходник токенами компилятора, а
//! не словарём (критерий 2 фичи 0531).
//!
//! ⚠️ Списка Structured Text здесь тоже нет, и это не пропуск. Предмет
//! `IEC_RESERVED` (`generator/st/st_reserved.rs`) иной: там имена, которые
//! `iec2c` отвергает **как идентификаторы пользователя**, — есть `abs`,
//! `left`, `concat` и нет `IF`, `VAR`, `TYPE`. Описание ST для подсветки — это
//! `book/st.sublime-syntax` (фича 0269), и с ним сверяется словарь моста.

/// Ключевые слова SystemVerilog (IEEE 1800-2017).
///
/// Непригодны как идентификаторы (отказ `SV-012` у цели `sv`) и
/// красятся ролью «ключевое слово» во вкладке цели `sv`.
///
/// ## Зачем список, которого нет ни в ADR, ни в плане задачи
///
/// Проба 2026-07-16 показала **реальную дыру**: Takt принимает `in fork: bit;` и
/// `out wire: bit;`, цель `c` их компилирует, — а `fork` и `wire` суть ключевые
/// слова SV, и вывод разваливается синтаксической ошибкой:
///
/// ```text
/// %Error: syntax error, unexpected fork
///     module kw (input logic clk, …, input logic fork, …);
/// ```
///
/// Это **тот же класс**, что ловушка цели `st` (`CLAUDE.md`: модель `Concat`
/// даёт `invalid function block name` — имя занято стандартной библиотекой IEC),
/// и он же — причина `RS-004` у цели `rust`. Отличие SV в том, что ключевых слов
/// у него около 250, и многие — обиходные имена автоматики: `fork`, `wire`,
/// `state` (его цель порождает сама — `RESERVED_NAMES` в `sv/sv_module.rs`),
/// `time`, `event`, `edge`, `cell`, `table`,
/// `force`, `release`, `disable`, `int`, `real`, `byte`.
///
/// Гейт эту дыру **не закрывает**: он проверяет корпус, а в корпусе таких имён
/// нет — красный SV увидел бы только пользователь, и увидел бы в виде ошибки
/// чужого инструмента. Отсюда диагностика `SV-012`.
pub static SV: &[&str] = &[
    "accept_on",
    "alias",
    "always",
    "always_comb",
    "always_ff",
    "always_latch",
    "and",
    "assert",
    "assign",
    "assume",
    "automatic",
    "before",
    "begin",
    "bind",
    "bins",
    "binsof",
    "bit",
    "break",
    "buf",
    "bufif0",
    "bufif1",
    "byte",
    "case",
    "casex",
    "casez",
    "cell",
    "chandle",
    "checker",
    "class",
    "clocking",
    "cmos",
    "config",
    "const",
    "constraint",
    "context",
    "continue",
    "cover",
    "covergroup",
    "coverpoint",
    "cross",
    "deassign",
    "default",
    "defparam",
    "design",
    "disable",
    "dist",
    "do",
    "edge",
    "else",
    "end",
    "endcase",
    "endchecker",
    "endclass",
    "endclocking",
    "endconfig",
    "endfunction",
    "endgenerate",
    "endgroup",
    "endinterface",
    "endmodule",
    "endpackage",
    "endprimitive",
    "endprogram",
    "endproperty",
    "endsequence",
    "endspecify",
    "endtable",
    "endtask",
    "enum",
    "event",
    "eventually",
    "expect",
    "export",
    "extends",
    "extern",
    "final",
    "first_match",
    "for",
    "force",
    "foreach",
    "forever",
    "fork",
    "forkjoin",
    "function",
    "generate",
    "genvar",
    "global",
    "highz0",
    "highz1",
    "if",
    "iff",
    "ifnone",
    "ignore_bins",
    "illegal_bins",
    "implements",
    "implies",
    "import",
    "incdir",
    "include",
    "initial",
    "inout",
    "input",
    "inside",
    "instance",
    "int",
    "integer",
    "interconnect",
    "interface",
    "intersect",
    "join",
    "join_any",
    "join_none",
    "large",
    "let",
    "liblist",
    "library",
    "local",
    "localparam",
    "logic",
    "longint",
    "macromodule",
    "matches",
    "medium",
    "modport",
    "module",
    "nand",
    "negedge",
    "nettype",
    "new",
    "nexttime",
    "nmos",
    "nor",
    "noshowcancelled",
    "not",
    "notif0",
    "notif1",
    "null",
    "or",
    "output",
    "package",
    "packed",
    "parameter",
    "pmos",
    "posedge",
    "primitive",
    "priority",
    "program",
    "property",
    "protected",
    "pull0",
    "pull1",
    "pulldown",
    "pullup",
    "pulsestyle_ondetect",
    "pulsestyle_onevent",
    "pure",
    "rand",
    "randc",
    "randcase",
    "randsequence",
    "rcmos",
    "real",
    "realtime",
    "ref",
    "reg",
    "reject_on",
    "release",
    "repeat",
    "restrict",
    "return",
    "rnmos",
    "rpmos",
    "rtran",
    "rtranif0",
    "rtranif1",
    "s_always",
    "s_eventually",
    "s_nexttime",
    "s_until",
    "s_until_with",
    "scalared",
    "sequence",
    "shortint",
    "shortreal",
    "showcancelled",
    "signed",
    "small",
    "soft",
    "solve",
    "specify",
    "specparam",
    "static",
    "string",
    "strong",
    "strong0",
    "strong1",
    "struct",
    "super",
    "supply0",
    "supply1",
    "sync_accept_on",
    "sync_reject_on",
    "table",
    "tagged",
    "task",
    "this",
    "throughout",
    "time",
    "timeprecision",
    "timeunit",
    "tran",
    "tranif0",
    "tranif1",
    "tri",
    "tri0",
    "tri1",
    "triand",
    "trior",
    "trireg",
    "type",
    "typedef",
    "union",
    "unique",
    "unique0",
    "unsigned",
    "until",
    "until_with",
    "untyped",
    "use",
    "uwire",
    "var",
    "vectored",
    "virtual",
    "void",
    "wait",
    "wait_order",
    "wand",
    "weak",
    "weak0",
    "weak1",
    "while",
    "wildcard",
    "wire",
    "with",
    "within",
    "wor",
    "xnor",
    "xor",
];

/// Ключевые слова Rust, которые **спасаются** raw-идентификатором (`r#type`).
///
/// Тот же список красит ключевые слова во вкладке цели `rust`.
///
/// Включая зарезервированные на будущее (`become`, `yield`, …): они не являются
/// ошибкой сегодня, но `r#` делает вывод устойчивым к смене редакции.
pub static RUST: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "do", "dyn",
    "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl", "in", "let", "loop",
    "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return", "static",
    "struct", "trait", "true", "try", "type", "typeof", "unsafe", "unsized", "use", "virtual",
    "where", "while", "yield",
];

/// Ключевые слова, которые raw-идентификатором **не спасаются**.
///
/// Проверено пробой 2026-07-16: `r#Self` отвергается отдельным правилом языка
/// («`Self` cannot be a raw identifier»); то же для `crate`, `self`, `super`.
/// Регистр приводится **до** проверки, поэтому исходные `Self` и `self` дают
/// одну и ту же диагностику — каждое в своём пространстве имён.
pub static RUST_NOT_RAW: &[&str] = &["Self", "crate", "self", "super"];
