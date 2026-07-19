//! Заголовок модуля и порты (задача 0045-04).
//!
//! ## Порт Lam здесь выражается напрямую — впервые из всех целей
//!
//! | Цель | `out cmd_fork: bit` |
//! |---|---|
//! | `c` | `(*main->write_bit)(STACKER_CMD_FORK, 1, main->userdata)` — косвенный вызов через таблицу колбэков |
//! | `c-hal` | `*(volatile uint8_t*)0x500 = 1` — запись по адресу |
//! | `st` | `VAR_OUTPUT cmd_fork : BOOL;` либо `VAR_GLOBAL … AT %QX…` |
//! | `rust` | `hal.write_bit(Port::CmdFork, true)` — метод трейта |
//! | **`sv`** | **`output logic cmd_fork`** — физический вывод кристалла |
//!
//! У всех программных целей между портом и миром стоит промежуточный слой,
//! потому что у процессора порта нет — есть адрес или вызов. У RTL порт **и
//! есть** порт.
//!
//! ## Служебные порты `clk`/`rst_n`/`en`: почему их нет в языке
//!
//! Такт Lam ≡ `posedge clk` (ADR, вопрос 1, Option A), поэтому модулю нужны
//! сигналы, которых в `.lam` не существует. Они добавляются **генератором**, а
//! имена для цели `sv` **зарезервированы**: коллизия → [`SV-007`]. Кроме
//! `clk`/`rst_n` это **`en`** (clock enable, фича 0063): вход с умолчанием
//! `1'b1`, гейтящий защёлкивание в `always_ff` (`end else if (en)`), но **не**
//! сброс (правило 3 ADR 0063). Неподключённый `en` тождествен `en=1`, поэтому
//! порт необязателен и существующие потребители его не замечают.
//!
//! Отвергнутая альтернатива (Option B — объявлять `clock`/`reset` в самом языке)
//! сломала бы аддитивность фичи и была бы **ложью для четырёх целей из пяти**:
//! у `c`/`st`/`rust` такта как сигнала не существует. Поэтому `clk`/`rst_n` —
//! имена **цели**, а не ключевые слова языка, и диагностика обязана это
//! объяснять: иначе автор решит, что сломан язык.
//!
//! ## Адрес порта (фича 0020) не потребляется
//!
//! `GenerateOptions::{hal, address_map}` здесь не читаются. MMIO-адрес для RTL
//! бессмыслен: у RTL-модуля процессора нет, сигнал приходит на вывод кристалла,
//! а не по адресу, — спрашивать адрес порта так же бессмысленно, как адрес ножки
//! микросхемы. Парной цели `sv-at` нет.

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::indent::Printer;
use crate::generator::sv::sv_map::SvMap;
use crate::generator::sv::sv_type::{SvType, sv_type};
use crate::semantic::minimap::Name;
use crate::semantic::{ModelNode, PortDirection, VariableNode};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

/// Имена, которые цель `sv` порождает сама и потому резервирует.
///
/// `clk`/`rst_n`/`is_done` — порты модуля; `state`/`state_next` — регистр
/// автомата и его комбинационная пара (задача 0045-05). Пользовательское имя,
/// совпавшее с любым из них, дало бы **два объявления одного идентификатора** —
/// то есть невалидный SV, причём в месте, не связанном с исходной строкой `.lam`.
///
/// ⚠️ ADR резервировал только `clk`/`rst_n`. Список расширен по **той же
/// причине**, по которой заведены те двое: генератор объявляет эти имена сам.
/// Цена расширения нулевая — в корпусе нет ни одного такого имени (проверено
/// `grep` 2026-07-16).
pub(crate) const RESERVED_NAMES: &[&str] =
    &["clk", "rst_n", "en", "is_done", "state", "state_next"];

/// Ключевые слова SystemVerilog (IEEE 1800-2017), непригодные как идентификаторы.
///
/// ## Зачем список, которого нет ни в ADR, ни в плане задачи
///
/// Проба 2026-07-16 показала **реальную дыру**: Lam принимает `in fork: bit;` и
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
/// `state` (см. [`RESERVED_NAMES`]), `time`, `event`, `edge`, `cell`, `table`,
/// `force`, `release`, `disable`, `int`, `real`, `byte`.
///
/// Гейт эту дыру **не закрывает**: он проверяет корпус, а в корпусе таких имён
/// нет — красный SV увидел бы только пользователь, и увидел бы в виде ошибки
/// чужого инструмента. Отсюда диагностика [`SV-012`].
const SV_KEYWORDS: &[&str] = &[
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

/// Строит диагностику `SV-006` — `inout` невыразим.
fn sv006(name: &str, loc: Location) -> Diagnostic {
    Diagnostic::error(
        loc,
        format!(
            "порт '{}': направление 'inout' целью 'sv' не поддерживается. \
             Двунаправленный порт в SystemVerilog — это трёхстабильная шина, \
             которой требуется сигнал разрешения выхода 'oe' \
             (assign pad = oe ? val : 1'bz;). Язык Lam сигнала 'oe' не выражает, \
             и вывести его из модели нельзя: модель не сообщает, когда порт \
             ведёт линию, а когда слушает её. Разделите порт на входной и \
             выходной либо используйте цель 'c'/'st'/'rust'",
            name
        ),
    )
    .with_code("SV-006")
}

/// Строит диагностику `SV-007` — коллизия с именем, которое порождает цель.
fn sv007(name: &str, loc: Location) -> Diagnostic {
    Diagnostic::error(
        loc,
        format!(
            "имя '{}' зарезервировано целью 'sv': генератор объявляет его сам \
             (clk/rst_n — служебные порты такта и сброса, en — clock enable, \
             is_done — выход терминальности, state/state_next — регистр \
             автомата). Это НЕ \
             ключевое слово языка Lam: модель остаётся полностью валидной для \
             целей 'c', 'c-hal', 'plantuml', 'st' и 'rust'. Переименуйте элемент \
             в исходнике .lam, если модель нужна в аппаратуре",
            name
        ),
    )
    .with_code("SV-007")
}

/// Строит диагностику `SV-012` — имя совпало с ключевым словом SystemVerilog.
fn sv012(name: &str, loc: Location) -> Diagnostic {
    Diagnostic::error(
        loc,
        format!(
            "имя '{}' является ключевым словом SystemVerilog и идентификатором \
             быть не может. Это НЕ ключевое слово языка Lam: модель остаётся \
             валидной для целей 'c', 'c-hal', 'plantuml', 'st' и 'rust'. \
             Переименуйте элемент в исходнике .lam, если модель нужна в \
             аппаратуре",
            name
        ),
    )
    .with_code("SV-012")
}

/// Проверяет, что имя пригодно как идентификатор SystemVerilog.
///
/// # Ошибки
/// - [`SV-007`](sv007) — имя порождает сама цель (`clk`, `state`, …);
/// - [`SV-012`](sv012) — имя является ключевым словом SystemVerilog.
pub(crate) fn check_sv_name(name: &str, loc: Location) -> Result<(), Diagnostic> {
    if RESERVED_NAMES.contains(&name) {
        return Err(sv007(name, loc));
    }
    if SV_KEYWORDS.contains(&name) {
        return Err(sv012(name, loc));
    }
    Ok(())
}

/// Порт модуля, подготовленный к эмиссии.
pub(crate) struct SvPort {
    /// Имя порта — **как в исходнике `.lam`**, без нормализации.
    ///
    /// Решение ADR («порты: имя из `.lam` без изменений»). Приведение регистра
    /// завело бы класс коллизий (`FloorSensor` и `floor_sensor` слиплись бы в
    /// одно имя — ср. `RS-005` цели `rust`) ради чистой косметики; к тому же
    /// имя порта читает инженер, подключающий модуль, и оно обязано совпадать с
    /// именем в спецификации автоматики.
    pub(crate) name: String,
    /// Тип порта в SystemVerilog.
    pub(crate) ty: SvType,
}

/// Порты модуля, разложенные по направлениям.
#[derive(Default)]
pub(crate) struct SvPorts {
    /// Входные порты (`in` → `input logic`).
    pub(crate) inputs: Vec<SvPort>,
    /// Выходные порты (`out` → `output logic`).
    pub(crate) outputs: Vec<SvPort>,
}

/// Собирает порты **всех** моделей файла в единый набор.
///
/// Порты берутся со всех моделей, а не только с корня: в `elevator_mini.lam`
/// они объявлены внутри под-моделей (`out ElevatorMotor_Up: bit;` в `Motor`), а
/// модуль SV — **один на корневую модель** (композиция уплощается, ADR
/// Option A′), поэтому его порты суть объединение портов уровней.
///
/// **Фильтр по [`UsageSet`](crate::semantic::unused::UsageSet) обязателен, а не
/// оптимизация.** Неиспользуемый порт в SV — не просто лишняя строка, а лишний
/// вывод кристалла; и, что решает дело, `verilator --lint-only -Wall` даёт на
/// него `UNUSEDSIGNAL`, то есть без фильтра гейт краснел бы на легальных
/// моделях (открытый вопрос 7 задачи 0045-04 — решён пробой в пользу фильтра).
///
/// # Ошибки
/// [`SV-006`](sv006) на `inout`, [`SV-007`](sv007)/[`SV-012`](sv012) на
/// непригодном имени, `SV-002`…`SV-004` на непереводимом типе.
pub(crate) fn collect_ports(
    map: &SvMap,
    blocks: &[(Name, Rc<RefCell<ModelNode>>)],
) -> Result<SvPorts, Diagnostic> {
    let mut ports = SvPorts::default();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (_, model_rc) in blocks {
        let model = model_rc.borrow();
        for var in model.variables.values() {
            let VariableNode::Port {
                name,
                ty,
                direction,
                loc,
                ..
            } = var
            else {
                continue;
            };
            // `inout` проверяется ДО фильтра использования: молча пропустить
            // невыразимое направление лишь потому, что порт нигде не читается,
            // значило бы отложить отказ до момента, когда его начнут читать.
            if matches!(direction, PortDirection::InOut) {
                return Err(sv006(name, *loc));
            }
            if !map.usage().ports.contains(name) || !seen.insert(name.clone()) {
                continue;
            }
            check_sv_name(name, *loc)?;
            let port = SvPort {
                name: name.clone(),
                ty: sv_type(ty, &format!("порт '{}'", name))?,
            };
            match direction {
                PortDirection::In => ports.inputs.push(port),
                PortDirection::Out => ports.outputs.push(port),
                // Отсечено выше; ветка `_` не заводится намеренно — добавление
                // направления обязано валить сборку, а не проваливаться молча.
                PortDirection::InOut => unreachable!("отсечено проверкой выше"),
            }
        }
    }
    Ok(ports)
}

/// Печатает заголовок модуля со списком портов.
///
/// Порядок — служебные, затем `in`, затем `out`, затем `is_done` (форма ADR).
/// Внутри направления порядок задан `BTreeMap` карты модели, то есть
/// детерминирован даром (фича 0048).
pub(crate) fn emit_module_header(p: &mut Printer, module: &str, ports: &SvPorts) {
    p.ident(&format!("module {} (", module)).nl();
    p.up();
    p.ident("input  logic clk,   // служебный порт цели sv: в .lam его нет")
        .nl();
    p.ident("input  logic rst_n, // служебный порт цели sv: сброс, активный низкий")
        .nl();
    // Умолчание `1'b1` (IEEE 1800 §23.2.2.4): неподключённый `en` тождествен `en=1`,
    // поэтому существующие потребители не обязаны его подключать (фича 0063).
    p.ident("input  logic en = 1'b1, // служебный порт цели sv: clock enable; НЕ обязателен (умолчание 1)")
        .nl();
    for port in &ports.inputs {
        p.ident(&format!("input  {},", port.ty.declare(&port.name)))
            .nl();
    }
    for port in &ports.outputs {
        p.ident(&format!("output {},", port.ty.declare(&port.name)))
            .nl();
    }
    // Последним и без запятой: терминальность модели наблюдаема снаружи.
    p.ident("output logic is_done").nl();
    p.down();
    p.ident(");").nl();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc() -> Location {
        Location::Codegen
    }

    /// Служебные имена цели отвергаются с `SV-007`.
    #[test]
    fn service_names_are_sv007() {
        for name in ["clk", "rst_n", "en", "is_done", "state", "state_next"] {
            let err = check_sv_name(name, loc()).unwrap_err();
            assert_eq!(err.code.as_deref(), Some("SV-007"), "имя {}", name);
        }
    }

    /// **Диагностика обязана объяснять, что язык не сломан.**
    ///
    /// `clk` — имя ЦЕЛИ, а не ключевое слово Lam: та же модель компилируется в
    /// `c`/`st`/`rust`. Без этого пояснения автор решит, что сломан язык.
    #[test]
    fn sv007_explains_that_language_is_intact() {
        let err = check_sv_name("clk", loc()).unwrap_err();
        assert!(
            err.message.contains("НЕ \nключевое слово")
                || err.message.contains("НЕ ключевое слово"),
            "SV-007 обязана отрицать, что это ключевое слово Lam: {}",
            err.message
        );
        assert!(
            err.message.contains("'c'") && err.message.contains("'rust'"),
            "SV-007 обязана назвать цели, где модель по-прежнему валидна: {}",
            err.message
        );
    }

    /// **Контрпример из пробы 2026-07-16:** `fork` и `wire` — ключевые слова SV.
    ///
    /// Lam принимает `in fork: bit;`, цель `c` компилируется, а SV разваливается
    /// синтаксической ошибкой. Ни ADR, ни план задачи этого класса не
    /// предусматривали.
    #[test]
    fn sv_keywords_are_sv012() {
        for name in [
            "fork", "wire", "reg", "always", "begin", "end", "time", "edge",
        ] {
            let err = check_sv_name(name, loc()).unwrap_err();
            assert_eq!(err.code.as_deref(), Some("SV-012"), "имя {}", name);
        }
    }

    /// Обиходные имена автоматики ключевыми словами не являются.
    ///
    /// Сторож против переусердствования: список не должен отвергать нормальные
    /// имена корпуса.
    #[test]
    fn ordinary_names_are_accepted() {
        for name in [
            "cmd_fork",
            "lift_request",
            "task_valid",
            "ElevatorMotor_Up",
            "sense_loaded",
            "current_floor",
        ] {
            assert!(
                check_sv_name(name, loc()).is_ok(),
                "имя {} отвергнуто",
                name
            );
        }
    }

    /// Заголовок модуля несёт служебные порты, которых в `.lam` нет.
    #[test]
    fn header_carries_service_ports() {
        let mut out = String::new();
        let mut p = Printer::new(4, &mut out);
        emit_module_header(&mut p, "stacker", &SvPorts::default());
        assert!(out.contains("input  logic clk,"), "нет clk:\n{out}");
        assert!(out.contains("input  logic rst_n,"), "нет rst_n:\n{out}");
        assert!(out.contains("output logic is_done"), "нет is_done:\n{out}");
        assert!(out.contains("module stacker ("), "нет имени модуля:\n{out}");
    }

    /// **0063 (A1):** заголовок несёт вход `en` с умолчанием `1'b1`.
    ///
    /// Умолчание (IEEE 1800 §23.2.2.4) делает порт необязательным: неподключённый
    /// `en` тождествен `en=1`, поэтому существующая сверка (`en` не подключает)
    /// остаётся зелёной (A3).
    #[test]
    fn header_carries_clock_enable_with_default() {
        let mut out = String::new();
        let mut p = Printer::new(4, &mut out);
        emit_module_header(&mut p, "stacker", &SvPorts::default());
        assert!(
            out.contains("input  logic en = 1'b1,"),
            "нет en с умолчанием 1'b1:\n{out}"
        );
    }

    /// **T19:** `in` → `input logic`, `out` → `output logic`; порядок — вход, выход.
    #[test]
    fn header_emits_directions_in_order() {
        let mut out = String::new();
        let mut p = Printer::new(4, &mut out);
        let ports = SvPorts {
            inputs: vec![SvPort {
                name: "lift_request".to_string(),
                ty: sv_type(&crate::semantic::type_node::TypeNode::Bit, "тест").unwrap(),
            }],
            outputs: vec![SvPort {
                name: "cmd_fork".to_string(),
                ty: sv_type(&crate::semantic::type_node::TypeNode::Bit, "тест").unwrap(),
            }],
        };
        emit_module_header(&mut p, "stacker", &ports);
        assert!(
            out.contains("input  logic lift_request,"),
            "нет входного порта:\n{out}"
        );
        assert!(
            out.contains("output logic cmd_fork,"),
            "нет выходного порта:\n{out}"
        );
        let inp = out.find("lift_request").unwrap();
        let outp = out.find("cmd_fork").unwrap();
        assert!(inp < outp, "входы обязаны идти раньше выходов:\n{out}");
        // Ни колбэков, ни volatile, ни AT % — порт здесь и есть порт.
        assert!(!out.contains("volatile") && !out.contains("AT %") && !out.contains("write_bit"));
    }

    /// Многобитный порт несёт упакованную ширину.
    #[test]
    fn header_emits_vector_port_width() {
        let mut out = String::new();
        let mut p = Printer::new(4, &mut out);
        let ports = SvPorts {
            inputs: vec![SvPort {
                name: "task_stack_no".to_string(),
                ty: sv_type(
                    &crate::semantic::type_node::TypeNode::Integer {
                        bits: 8,
                        signed: false,
                    },
                    "тест",
                )
                .unwrap(),
            }],
            outputs: Vec::new(),
        };
        emit_module_header(&mut p, "stacker", &ports);
        assert!(
            out.contains("input  logic [7:0] task_stack_no,"),
            "нет ширины порта:\n{out}"
        );
    }
}
