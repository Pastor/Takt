//! Отображение типов Lam в типы SystemVerilog (задача 0045-03).
//!
//! ## Зачем это ядро фичи
//!
//! У `get_c_type` (`generator/c/mod.rs:150`) три зафиксированных дефекта
//! (фича 0029), и в RTL все три исчезают не «благодаря аккуратности», а
//! конструктивно — потому что аппаратура и есть та область, где ширина типа
//! означает ровно число проводов:
//!
//! | Lam | C (дефект 0029) | SystemVerilog |
//! |---|---|---|
//! | `bit` | `int` — 32 бита на один провод | `logic` — **один** триггер |
//! | `[u8; 4]` | `uint4_t` — типа не существует | `logic [7:0] name [0:3]` |
//! | `u8` | `uint8_t` | `logic [7:0]` — разрядность **буквальна** |
//! | `float` | `float` (f32) ≠ f64 симулятора | **ошибка `SV-003`** |
//!
//! `Rational` здесь не «отображается хуже, чем в Rust», а **не существует**: в
//! синтезируемом RTL плавающей точки нет. Молчаливо подставить `real` нельзя —
//! это дало бы модуль, который симулируется, но не синтезируется (и, что важно
//! для гейта, **Verilator принял бы его молча** — ловит только yosys).
//!
//! ## `Result`, а не `Option` — ключевое отличие от `get_c_type`
//!
//! На непокрытом узле — **ошибка**, а не молчание (R4). Прямое следствие уроков
//! 0025/0028/0029: в цели `c` неотображаемый тип возвращает `None`, и отличить
//! «отфильтровали как неиспользуемое» от «не смогли перевести» снаружи
//! невозможно.
//!
//! ## Ширина перечисления считается по ДИАПАЗОНУ ЗНАЧЕНИЙ, а не по числу вариантов
//!
//! ADR (таблица T11) предписывал `W = ⌈log₂(вариантов)⌉, минимум 1`. **Проба
//! 2026-07-16 это опровергла на реальном примере корпуса:** `enum Action { Idle
//! = 670, … }` (`elevator.lam:121`) — два варианта, то есть по формуле ADR
//! `logic [0:0]`, — а значение 670 требует десяти бит:
//!
//! ```text
//! %Error-ENUMITEMWIDTH: Enum value exceeds width of enum type (IEEE 1800-2023 6.19)
//!     typedef enum logic [0:0] { IDLE = 670, UP = 671 } action_e;
//! ```
//!
//! Это **тот же капкан**, в который уже попали два соседних бэкенда: ST
//! (`CLAUDE.md`: «Разрядность перечисления считается по диапазону вариантов, а
//! не берётся `USINT`, как предполагал ADR») и Rust (`#[repr(u8)]` не принял
//! `Idle = 670`). Здесь он учтён **до** написания кода.
//!
//! Формула [`enum_width`] — по диапазону — покрывает **оба** случая одной
//! ветвью: у перечисления состояний, где значения назначает сам генератор
//! (`0..n-1`), она даёт ровно `⌈log₂(n)⌉`, то есть совпадает с формулой ADR;
//! у пользовательского перечисления с явными значениями — верную ширину.
//! Отдельного правила для состояний заводить не требуется.

use crate::diagnostics::{Diagnostic, Location};
use crate::semantic::enum_facts;
use crate::semantic::naming::normalize_lowercase_snakecase;
use crate::semantic::type_node::TypeNode;

/// Строит диагностику `SV-002` — узел не покрыт отображением.
fn sv002(what: &str, ty: &TypeNode) -> Diagnostic {
    Diagnostic::error(
        Location::Codegen,
        format!(
            "{}: тип '{}' не отображается в SystemVerilog. Это внутренний либо \
             неразрешённый тип — в порождаемом RTL представления не имеет",
            what, ty
        ),
    )
    .with_code("SV-002")
}

/// Строит диагностику `SV-003` — вещественного типа в синтезируемом RTL нет.
fn sv003(what: &str) -> Diagnostic {
    Diagnostic::error(
        Location::Codegen,
        format!(
            "{}: вещественный тип (float) не существует в синтезируемом RTL и \
             целью 'sv' не поддерживается. Тип 'real' языка SystemVerilog \
             пригоден только для симуляции — синтезатор его отвергает. \
             Используйте целочисленный тип либо цель 'c'/'rust', где float \
             отображается",
            what
        ),
    )
    .with_code("SV-003")
}

/// Строит диагностику `SV-004` — форма типа недопустима.
fn sv004(what: &str, why: &str) -> Diagnostic {
    Diagnostic::error(Location::Codegen, format!("{}: {}", what, why)).with_code("SV-004")
}

/// Объявление типа в SystemVerilog: часть до имени и распакованная размерность
/// после него.
///
/// Одной строкой тип не выражается: синтаксис объявления SV **разрывен вокруг
/// имени** — `logic [7:0] data [0:3];`, где `logic [7:0]` — упакованная часть
/// (ширина элемента), а `[0:3]` — распакованная (число элементов). Склеить их в
/// одну строку значило бы получить `logic [7:0] [0:3] data`, то есть **двумерный
/// упакованный вектор** — другой тип с другой семантикой.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SvType {
    /// Часть до имени: `logic`, `logic [7:0]`, `logic signed [15:0]`, `action_e`.
    pub(crate) prefix: String,
    /// Распакованная размерность после имени: пусто либо `[0:3]`.
    pub(crate) suffix: String,
}

impl SvType {
    /// Печатает объявление `<prefix> <name> <suffix>` — без завершающей `;`.
    ///
    /// Пробел перед распакованной размерностью — форма ADR (`<T> name [0:N-1]`)
    /// и общепринятый стиль SV: он отделяет её от имени, тогда как упакованная
    /// часть прижата к типу. Читателю видно, какая размерность к чему относится.
    pub(crate) fn declare(&self, name: &str) -> String {
        if self.suffix.is_empty() {
            format!("{} {}", self.prefix, name)
        } else {
            format!("{} {} {}", self.prefix, name, self.suffix)
        }
    }

    /// Скалярный тип без распакованной размерности.
    fn scalar(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            suffix: String::new(),
        }
    }
}

/// Имя типа перечисления в SV: `action` → `action_e`.
pub(crate) fn sv_enum_type_name(raw: &str) -> String {
    format!("{}_e", normalize_lowercase_snakecase(raw.to_string()))
}

/// Имя типа структуры в SV: `point` → `point_t`.
pub(crate) fn sv_struct_type_name(raw: &str) -> String {
    format!("{}_t", normalize_lowercase_snakecase(raw.to_string()))
}

/// Отображает тип Lam в объявление SystemVerilog.
///
/// `what` — что именно объявляется (`переменная 'x'`, `порт 'p'`), чтобы
/// диагностика указывала на место, а не на абстрактный тип.
///
/// # Ошибки
///
/// - [`SV-002`](sv002) — узел не покрыт (внутренний либо неразрешённый тип);
/// - [`SV-003`](sv003) — `Rational`: в синтезируемом RTL FP не существует;
/// - [`SV-004`](sv004) — форма типа недопустима (`Array(0, _)`, `Integer{0}`).
pub(crate) fn sv_type(ty: &TypeNode, what: &str) -> Result<SvType, Diagnostic> {
    match ty {
        // Идеальное соответствие: один провод / один триггер. В цели `c` — `int`,
        // то есть 32 бита на бит (дефект 0029, Д2).
        TypeNode::Bit | TypeNode::Bool => Ok(SvType::scalar("logic")),
        // Отдельного `real` нет и быть не может: см. шапку модуля.
        TypeNode::Rational => Err(sv003(what)),
        TypeNode::Integer { bits, signed } => {
            if *bits == 0 {
                return Err(sv004(
                    what,
                    "нулевая разрядность целого: `logic [-1:0]` не является \
                     допустимым диапазоном",
                ));
            }
            // Разрядность буквальна и произвольна: в RTL машинного слова нет,
            // `logic [11:0]` так же нормален, как `logic [7:0]`. Округления до
            // 8/16/32/64, как в C, здесь не требуется.
            //
            // `signed` обязателен, а не косметика: он меняет семантику сравнений
            // и арифметического сдвига вправо.
            let sign = if *signed { "signed " } else { "" };
            Ok(SvType::scalar(format!("logic {}[{}:0]", sign, bits - 1)))
        }
        // Настоящий (распакованный) массив. В цели `c` — `uint{N}_t`, где N —
        // ЧИСЛО ЭЛЕМЕНТОВ, то есть несуществующий тип (дефект 0029, Д1).
        TypeNode::Array(n, elem) => {
            if *n == 0 {
                return Err(sv004(
                    what,
                    "массив нулевого размера: `[0:-1]` не является допустимым \
                     диапазоном",
                ));
            }
            let inner = sv_type(elem, what)?;
            // Размерности накапливаются СЛЕВА НАПРАВО: `[u8; 2]` элементов
            // `[u8; 4]` даёт `logic [7:0] a [0:1][0:3]` — внешняя размерность
            // первая, как и в исходнике.
            Ok(SvType {
                prefix: inner.prefix,
                suffix: format!("[0:{}]{}", n - 1, inner.suffix),
            })
        }
        // Имена вариантов видны в осциллограмме — для отладки RTL это не
        // косметика: иначе на волнах числа вместо имён состояний.
        TypeNode::Enum(name) => Ok(SvType::scalar(sv_enum_type_name(name))),
        TypeNode::Struct(name) => Ok(SvType::scalar(sv_struct_type_name(name))),
        // Пустой тип осмыслен ровно в одной позиции — `function automatic void`.
        TypeNode::Unit => Ok(SvType::scalar("void")),
        // Ниже — то, что представления не имеет. Ветки `_` нет намеренно:
        // `TypeNode` помечен `#[non_exhaustive]`, но ВНУТРИ крейта-объявителя
        // атрибут не действует, поэтому исчерпывающий разбор здесь возможен — и
        // обязателен. Добавление варианта в `TypeNode` обязано валить сборку, а
        // не тихо проваливаться в чужую ветку (ADR 0025).
        //
        // ⚠️ План задачи требовал ветку `_ =>`, возвращающую `SV-002`, полагая
        // `#[non_exhaustive]` действующим здесь. Это неверно: атрибут не
        // действует внутри объявляющего крейта, и ветка `_` была бы не сторожем,
        // а его отключением — она проглотила бы новый вариант молча.
        TypeNode::Address(_, _)
        | TypeNode::Inference
        | TypeNode::Unsupported
        | TypeNode::BuiltinString
        | TypeNode::BuiltinModel
        | TypeNode::BuiltinState
        | TypeNode::BuiltinNumeric => Err(sv002(what, ty)),
    }
}

/// Считает ширину и знаковость перечисления **по диапазону его значений**.
///
/// Возвращает `(ширина в битах, знаковое ли)`.
///
/// Формула ADR (`⌈log₂(вариантов)⌉`) **неверна** и опровергнута пробой: см.
/// шапку модуля. Здесь ширина считается по значениям, что покрывает и
/// перечисление состояний (значения `0..n-1` назначает генератор → формула
/// вырождается в `⌈log₂(n)⌉`), и пользовательское с явными значениями.
///
/// # Ошибки
/// [`SV-004`](sv004), если вариантов нет: ширина перечисления не определена.
pub(crate) fn enum_width(
    variants: &[(String, i64)],
    what: &str,
) -> Result<(u32, bool), Diagnostic> {
    // Аппаратная ширина точна, поэтому `sv` берёт `min_bits` факта НАПРЯМУЮ (без
    // округления до машинной, в отличие от `c`/`st`/`rust`). Тонкости, которые
    // раньше жили здесь, теперь в факте (фича 0060): знаковое — минимум 2 бита
    // (однобитного знакового не бывает), `max == 0` → 1 (ширины 0 не бывает).
    // Пустое перечисление → `SV-004` (сегодняшнее поведение цели).
    match enum_facts(variants) {
        Some(f) => Ok((f.min_bits, f.signed)),
        None => Err(sv004(
            what,
            "перечисление без вариантов: ширина типа не определена",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(bits: u8) -> TypeNode {
        TypeNode::Integer {
            bits,
            signed: false,
        }
    }

    /// **T1/A7:** `bit` → `logic`, а **не** `int`.
    ///
    /// Сторож против дефекта 0029 (Д2): в C один провод занимает 32 бита.
    #[test]
    fn bit_maps_to_logic_not_int() {
        let ty = sv_type(&TypeNode::Bit, "тест").unwrap();
        assert_eq!(ty.prefix, "logic");
        assert_eq!(ty.suffix, "");
        assert_ne!(
            ty.prefix, "int",
            "повторён дефект 0029: 32 бита на один провод"
        );
    }

    /// **T2:** `bool` → `logic`. Для RTL `bit` и `bool` — один провод.
    #[test]
    fn bool_maps_to_logic() {
        assert_eq!(sv_type(&TypeNode::Bool, "тест").unwrap().prefix, "logic");
    }

    /// **T3–T6:** разрядность целого буквальна.
    #[test]
    fn unsigned_integers_map_literally() {
        for (bits, expected) in [
            (8u8, "logic [7:0]"),
            (16, "logic [15:0]"),
            (32, "logic [31:0]"),
            (64, "logic [63:0]"),
        ] {
            assert_eq!(sv_type(&u(bits), "тест").unwrap().prefix, expected);
        }
    }

    /// **T7:** знаковое целое получает `signed` — это не косметика.
    ///
    /// `signed` меняет семантику сравнений и арифметического сдвига вправо;
    /// без него `-1 < 0` на `logic [7:0]` было бы ложным.
    #[test]
    fn signed_integers_carry_signed_keyword() {
        let ty = sv_type(
            &TypeNode::Integer {
                bits: 16,
                signed: true,
            },
            "тест",
        )
        .unwrap();
        assert_eq!(ty.prefix, "logic signed [15:0]");
    }

    /// **T8/T15:** разрядность, не кратная машинному слову, отображается даром.
    ///
    /// ⚠️ Проверка T15 тест-плана (`var x: u12;` → `logic [11:0] x;`) в виде
    /// исходника `.lam` **невыполнима**: проба 2026-07-16 показала, что `u12`
    /// языком не принимается вовсе (`SE-034: Локальный тип 'u12' не найден`) —
    /// конструктор типов строит только 8/16/32/64. Утверждение ADR «произвольная
    /// разрядность бесплатна» верно применительно к **отображению**, что этот
    /// тест и проверяет напрямую на `TypeNode`; появится ли `u12` в языке —
    /// вопрос к языку, а не к цели.
    #[test]
    fn arbitrary_width_integer_maps_without_rounding() {
        assert_eq!(sv_type(&u(12), "тест").unwrap().prefix, "logic [11:0]");
        assert_eq!(sv_type(&u(1), "тест").unwrap().prefix, "logic [0:0]");
        assert_eq!(sv_type(&u(3), "тест").unwrap().prefix, "logic [2:0]");
    }

    /// **T14/A6:** `[u8; 4]` → `logic [7:0] data [0:3]`, а **не** `uint4_t`.
    ///
    /// Сторож против дефекта 0029 (Д1): в C `Array(size, elem)` даёт
    /// `uint{size}_t`, где `size` — число элементов, то есть несуществующий тип.
    #[test]
    fn array_maps_to_unpacked_array_not_uint4_t() {
        let ty = sv_type(&TypeNode::Array(4, Box::new(u(8))), "тест").unwrap();
        assert_eq!(ty.prefix, "logic [7:0]");
        assert_eq!(ty.suffix, "[0:3]");
        assert_eq!(ty.declare("data"), "logic [7:0] data [0:3]");
        assert!(
            !ty.declare("data").contains("uint4_t"),
            "повторён дефект 0029"
        );
    }

    /// Вложенный массив даёт многомерную распакованную форму.
    ///
    /// В ST это невозможно (`ARRAY OF ARRAY` отвергается MatIEC — нужна
    /// многомерная форма `ARRAY [0..2, 0..1] OF T`); в SV — бесплатно.
    /// Размерности идут снаружи внутрь, как в исходнике.
    #[test]
    fn nested_array_maps_to_multidimensional_unpacked() {
        let inner = TypeNode::Array(4, Box::new(u(8)));
        let outer = TypeNode::Array(2, Box::new(inner));
        let ty = sv_type(&outer, "тест").unwrap();
        assert_eq!(ty.declare("a"), "logic [7:0] a [0:1][0:3]");
    }

    /// Массив бит даёт распакованный массив однобитных `logic`.
    #[test]
    fn array_of_bit_maps_to_unpacked_logic() {
        let ty = sv_type(&TypeNode::Array(8, Box::new(TypeNode::Bit)), "тест").unwrap();
        assert_eq!(ty.declare("flags"), "logic flags [0:7]");
    }

    /// **T11:** перечисление отображается в именованный тип с суффиксом `_e`.
    #[test]
    fn enum_maps_to_named_type() {
        let ty = sv_type(&TypeNode::Enum("Action".to_string()), "тест").unwrap();
        assert_eq!(ty.prefix, "action_e");
    }

    /// **T12:** структура отображается в именованный тип с суффиксом `_t`.
    #[test]
    fn struct_maps_to_named_type() {
        let ty = sv_type(&TypeNode::Struct("Point".to_string()), "тест").unwrap();
        assert_eq!(ty.prefix, "point_t");
    }

    /// **T13:** пустой тип — `void` (осмыслен только как тип возврата функции).
    #[test]
    fn unit_maps_to_void() {
        assert_eq!(sv_type(&TypeNode::Unit, "тест").unwrap().prefix, "void");
    }

    /// **T9/A5: контрпример.** `float` → `SV-003`, а не `real`.
    ///
    /// Молчаливая подстановка `real` дала бы модуль, который **симулируется, но
    /// не синтезируется**, — и `verilator --lint-only` пропустил бы это молча
    /// (проба ADR). То есть цена ошибки здесь — необнаруживаемый линтером
    /// дефект.
    #[test]
    fn rational_is_sv003_not_real() {
        let err = sv_type(&TypeNode::Rational, "переменная 'x'").unwrap_err();
        assert_eq!(err.code.as_deref(), Some("SV-003"));
        assert!(
            err.message.contains("переменная 'x'"),
            "диагностика должна указывать на место: {}",
            err.message
        );
    }

    /// **T16/A5: контрпример.** Непокрытый узел даёт `SV-002`, а не молчание.
    ///
    /// В цели `c` `get_c_type` возвращает здесь `None`, и «не смогли перевести»
    /// снаружи неотличимо от «отфильтровали как неиспользуемое» (наследие
    /// ADR 0028).
    #[test]
    fn uncovered_nodes_are_sv002() {
        for ty in [
            TypeNode::Inference,
            TypeNode::Unsupported,
            TypeNode::Address(0x10, None),
            TypeNode::BuiltinString,
            TypeNode::BuiltinModel,
            TypeNode::BuiltinState,
            TypeNode::BuiltinNumeric,
        ] {
            let err = sv_type(&ty, "переменная 'x'").unwrap_err();
            assert_eq!(err.code.as_deref(), Some("SV-002"), "тип {:?}", ty);
        }
    }

    /// **A5: контрпример.** Массив нулевого размера → `SV-004`.
    #[test]
    fn zero_sized_array_is_sv004() {
        let err = sv_type(&TypeNode::Array(0, Box::new(u(8))), "переменная 'x'").unwrap_err();
        assert_eq!(err.code.as_deref(), Some("SV-004"));
    }

    /// **A5: контрпример.** Целое нулевой разрядности → `SV-004`.
    #[test]
    fn zero_width_integer_is_sv004() {
        let err = sv_type(&u(0), "переменная 'x'").unwrap_err();
        assert_eq!(err.code.as_deref(), Some("SV-004"));
    }

    /// Ошибка типа элемента всплывает наружу из массива, а не теряется.
    #[test]
    fn array_of_float_propagates_sv003() {
        let err = sv_type(
            &TypeNode::Array(4, Box::new(TypeNode::Rational)),
            "переменная 'x'",
        )
        .unwrap_err();
        assert_eq!(err.code.as_deref(), Some("SV-003"));
    }

    /// **Ключевой тест.** Ширина перечисления — по ДИАПАЗОНУ ЗНАЧЕНИЙ.
    ///
    /// Реальный случай корпуса — `elevator.lam:121` (`Idle = 670`). По формуле
    /// ADR (`⌈log₂(вариантов)⌉`) два варианта дали бы `logic [0:0]`, и проба
    /// 2026-07-16 показала, что Verilator отвергает это с `%Error-ENUMITEMWIDTH:
    /// Enum value exceeds width of enum type`. Тот же капкан ранее поймал
    /// бэкенды ST и Rust.
    #[test]
    fn enum_width_670_needs_ten_bits_not_one() {
        let variants = vec![("Idle".to_string(), 670), ("Up".to_string(), 671)];
        let (width, signed) = enum_width(&variants, "тест").unwrap();
        assert_eq!(
            width, 10,
            "ширина обязана считаться по диапазону значений, а не по числу вариантов"
        );
        assert!(!signed);
    }

    /// Для перечисления состояний формула совпадает с формулой ADR.
    ///
    /// Значения `0..n-1` назначает сам генератор, поэтому «по диапазону» здесь
    /// вырождается в `⌈log₂(n)⌉` — отдельной ветви для состояний не требуется.
    #[test]
    fn enum_width_of_sequential_states_matches_log2() {
        let seq = |n: i64| -> u32 {
            let variants: Vec<(String, i64)> = (0..n).map(|i| (format!("S{}", i), i)).collect();
            enum_width(&variants, "тест").unwrap().0
        };
        assert_eq!(seq(1), 1, "одно состояние — ширины 0 в SV не бывает");
        assert_eq!(seq(2), 1);
        assert_eq!(seq(3), 2);
        assert_eq!(seq(4), 2);
        assert_eq!(seq(5), 3);
        assert_eq!(seq(8), 3);
        assert_eq!(seq(9), 4);
    }

    /// Границы беззнаковой ширины.
    #[test]
    fn enum_width_unsigned_boundaries() {
        let at = |v: i64| enum_width(&[("V".to_string(), v)], "тест").unwrap();
        assert_eq!(at(0), (1, false));
        assert_eq!(at(1), (1, false));
        assert_eq!(at(2), (2, false));
        assert_eq!(at(255), (8, false));
        assert_eq!(at(256), (9, false));
    }

    /// Отрицательный вариант даёт знаковое перечисление нужной ширины.
    #[test]
    fn enum_width_negative_is_signed() {
        assert_eq!(
            enum_width(&[("V".to_string(), -1)], "тест").unwrap(),
            (2, true)
        );
        assert_eq!(
            enum_width(&[("A".to_string(), -128), ("B".to_string(), 127)], "тест").unwrap(),
            (8, true)
        );
        assert_eq!(
            enum_width(&[("A".to_string(), -129)], "тест").unwrap(),
            (9, true)
        );
    }

    /// **Контрпример.** Перечисление без вариантов → `SV-004`.
    #[test]
    fn empty_enum_is_sv004() {
        let err = enum_width(&[], "перечисление 'E'").unwrap_err();
        assert_eq!(err.code.as_deref(), Some("SV-004"));
    }
}
