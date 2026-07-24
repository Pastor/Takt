//! Отображение типов Takt ([`TypeNode`]) в типы IEC 61131-3.
//!
//! Задача 0041-02; нормативная таблица — [`analyze/0041-02`], решение — ADR 0041
//! (вопрос 2, Option B с откатом Option C для перечислений).
//!
//! ## Почему `Result`, а не `Option`
//!
//! Аналог для цели `c` — `get_c_type` ([`crate::generator::c::get_c_type`]) —
//! возвращает `Option<String>` и на неотображаемом типе отдаёт `None`. Вызывающий
//! волен молча пропустить переменную, и именно так возник дефект Д1b фичи 0029:
//! `var data: [u8; 4];` исчезает из порождённой структуры **без диагностики**.
//! Здесь сигнатура — `Result<String, Diagnostic>`: «тихого» исхода нет, отказ
//! обязан быть обработан вызывающим.
//!
//! Общего слоя с `get_c_type` намеренно **нет** (ADR 0041, вопрос 2): системы
//! типов C и IEC различны, а общий слой связал бы аддитивную 0041 с незакрытой
//! 0029.
//!
//! ## Факты, снятые пробой MatIEC `iec2c` (0041-06)
//!
//! Две нормы таблицы исправлены **по факту**, а не по ожиданию:
//!
//! - **Массивы не вкладываются.** `ARRAY [0..2] OF ARRAY [0..1] OF USINT` —
//!   `error: invalid item data type in array specification`. Принятая форма —
//!   многомерная: `ARRAY [0..2, 0..1] OF USINT` (проба ✅). Поэтому
//!   [`get_st_type`] **уплощает** вложенные [`TypeNode::Array`] в список
//!   размерностей, а не рекурсирует в текст типа.
//! - **Перечисления не отображаются напрямую.** `TYPE F : (A := 80); END_TYPE` —
//!   `error: ')' missing at the end of enumerated specification` (явные значения
//!   вариантов — 3-я редакция IEC, MatIEC её не знает). Действует откат Option C:
//!   целочисленный тип + именованные константы (их печатает `st_decl`).
//!
//! [`analyze/0041-02`]: ../../../../docs/analyze/0041-02-type-mapping.md

use crate::diagnostics::{Diagnostic, Location};
use crate::semantic::ModelNode;
use crate::semantic::enum_facts;
use crate::semantic::type_node::TypeNode;

/// Отображает тип Takt в имя типа IEC 61131-3.
///
/// `model` нужен для разрешения именованных типов (`enum`/`struct`): имя ищется
/// в модели и родительских (`search_enum`/`search_struct`).
///
/// # Ошибки
///
/// - `ST-002` — тип не имеет представления в IEC (`Inference`, `Unsupported`,
///   `Address`, `Unit`, `Builtin*`, нестандартная разрядность `Integer`).
/// - `ST-007` — массив нулевого размера (`ARRAY [0..-1]` невыразим).
/// - `ST-008` — `Enum`/`Struct` не разрешается в модели.
///
/// Кода `ST-003` («неизвестный вариант `TypeNode`») **нет**: разбор
/// исчерпывающий, и новый вариант ловит компилятор — см. комментарий в конце
/// `match`.
pub(crate) fn get_st_type(typ: &TypeNode, model: &ModelNode) -> Result<String, Diagnostic> {
    match typ {
        // T1, T2. В Takt `bit` и `bool` — разные узлы, в IEC оба суть `BOOL`.
        // Различие теряется безвредно: обратного преобразования нет, семантика
        // (1 бит) совпадает. Цель `c` здесь даёт `int` — дефект Д2 фичи 0029.
        TypeNode::Bit | TypeNode::Bool => Ok("BOOL".to_string()),
        // T3..T10.
        TypeNode::Integer { bits, signed } => integer_type(*bits, *signed),
        // Fixed-point q(m, n) (фича 0061): знаковое целое IEC, вмещающее W = m+n
        // бит (SINT/INT/DINT/LINT). Масштабирование при `*`/`/` — задача 0061-03
        // (сдвиг в ST через преобразования, `<<` над числами нет).
        TypeNode::Fixed { m, n } => {
            integer_type(crate::semantic::type_node::fixed_storage_bits(m + n), true)
        }
        // T11. LREAL — 64-битное вещественное, совпадает с f64 симулятора
        // (`simulation/src/eval/`). `REAL` (f32) повторил бы дефект Д3.
        TypeNode::Rational => Ok("LREAL".to_string()),
        // T12 / фича 0078. Бит-вектор `[bit;N]` — упакованный скаляр
        // `USINT/UINT/UDINT/ULINT` (round_up, N ≤ 64) либо массив слов
        // `ARRAY [0..count-1] OF ULINT` (N > 64). Иначе — настоящий массив.
        TypeNode::Array(_, _) => {
            if let Some(nbits) = crate::semantic::bit_vector::is_bit_vector(typ) {
                use crate::semantic::bit_vector::{self, BitVectorLayout};
                return match bit_vector::layout(nbits) {
                    BitVectorLayout::Scalar { width } => integer_type(width as u8, false),
                    BitVectorLayout::Words { count } => {
                        Ok(format!("ARRAY [0..{}] OF ULINT", count - 1))
                    }
                };
            }
            array_type(typ, model)
        }
        // T13. Откат Option C: перечислимый тип MatIEC не принимает (проба П4).
        TypeNode::Enum(name) => enum_type(name, model),
        // T14. Ссылка на объявление `TYPE … STRUCT … END_STRUCT; END_TYPE`,
        // которое печатает `st_decl`.
        TypeNode::Struct(name) => struct_type(name, model),
        // T16. Служебные узлы: типом переменной быть не могут. Цель `c` отдаёт
        // здесь `None` — тихий отказ (дефект Д4).
        TypeNode::Unit => Err(unmapped("unit", "пустой тип не является типом переменной")),
        TypeNode::Inference => Err(unmapped(
            "<не выведен>",
            "вывод типов не завершён — тип переменной неизвестен",
        )),
        TypeNode::Unsupported => Err(unmapped(
            "<неподдерживаемый>",
            "тип не поддержан семантикой",
        )),
        TypeNode::Address(addr, _) => Err(unmapped(
            &format!("0x{:X}", addr),
            "адресный литерал порта — внутренний тип, а не тип переменной",
        )),
        TypeNode::BuiltinString => Err(unmapped(
            "string",
            "строковый тип встроенных функций не транслируется в ST",
        )),
        TypeNode::BuiltinModel => Err(unmapped("<модель>", "внутренний тип встроенных функций")),
        TypeNode::BuiltinState => Err(unmapped("<состояние>", "внутренний тип встроенных функций")),
        TypeNode::BuiltinNumeric => Err(unmapped(
            "<числовой>",
            "внутренний тип встроенных функций: конкретная разрядность неизвестна",
        )),
        // Ветки `_` здесь НЕТ — и это проверенный факт, а не недосмотр.
        //
        // Анализ 0041-02 предполагал её вынужденной: `TypeNode` помечен
        // `#[non_exhaustive]` (`type_node.rs:495`), значит исчерпывающий разбор
        // якобы невозможен, и ветка `_` обязана вернуть `ST-003`. Предположение
        // **опроверг компилятор**: `#[non_exhaustive]` ограничивает только
        // **внешние** крейты, а `generator/st` живёт в том же крейте, что и
        // `semantic` — здесь атрибут не действует, и ветка `_` помечается
        // `unreachable_patterns`.
        //
        // Следствие сильнее, чем задумывалось: новый вариант `TypeNode` **завалит
        // сборку** этого разбора, а не всплывёт диагностикой `ST-003` в рантайме.
        // Компилятор — гарантия строже теста, поэтому `ST-003` не занят и остаётся
        // свободным. Оговорка анализа («новый вариант не завалит сборку») к
        // внутрикрейтовому коду не относится.
    }
}

/// Строит диагностику `ST-002` — тип без представления в IEC 61131-3.
fn unmapped(shown: &str, why: &str) -> Diagnostic {
    Diagnostic::error(
        Location::Codegen,
        format!(
            "Тип '{}' не имеет представления в IEC 61131-3: {}",
            shown, why
        ),
    )
    .with_code("ST-002")
}

/// T3..T10: целые Takt → целые IEC.
///
/// ⚠ `INT` в IEC — **16-битный** знаковый, а не 32-битный, как в C.
fn integer_type(bits: u8, signed: bool) -> Result<String, Diagnostic> {
    let name = match (bits, signed) {
        (8, false) => "USINT",
        (16, false) => "UINT",
        (32, false) => "UDINT",
        (64, false) => "ULINT",
        (8, true) => "SINT",
        (16, true) => "INT",
        (32, true) => "DINT",
        (64, true) => "LINT",
        _ => {
            return Err(unmapped(
                &format!("{}{}", if signed { "i" } else { "u" }, bits),
                "IEC 61131-3 знает только разрядности 8, 16, 32 и 64",
            ));
        }
    };
    Ok(name.to_string())
}

/// T12: `Array(N, T)` → `ARRAY [0..N-1] OF <T>`; вложенные массивы уплощаются в
/// многомерный `ARRAY [0..N-1, 0..M-1] OF <T>`.
///
/// Уплощение — не стилистика: `iec2c` **отвергает** `ARRAY OF ARRAY`
/// (`invalid item data type in array specification`), но принимает многомерную
/// форму. Порядок размерностей — от внешней к внутренней, как в индексации Takt:
/// `[[u8; 2]; 3]` = `Array(3, Array(2, u8))` → `ARRAY [0..2, 0..1] OF USINT`.
fn array_type(typ: &TypeNode, model: &ModelNode) -> Result<String, Diagnostic> {
    let mut dims: Vec<String> = Vec::new();
    let mut current = typ;
    while let TypeNode::Array(size, elem) = current {
        if *size == 0 {
            return Err(Diagnostic::error(
                Location::Codegen,
                format!(
                    "Массив нулевого размера ('{}') невыразим в IEC 61131-3: \
                     диапазон 'ARRAY [0..-1]' пуст",
                    typ
                ),
            )
            .with_code("ST-007"));
        }
        dims.push(format!("0..{}", size - 1));
        current = elem;
    }
    // Базовый тип разбирается тем же отображением: массив структур или
    // перечислений обязан пройти проверку разрешимости имени.
    let base = get_st_type(current, model)?;
    Ok(format!("ARRAY [{}] OF {}", dims.join(", "), base))
}

/// T13 (откат Option C): `Enum(name)` → целочисленный тип, вмещающий все варианты.
///
/// **Почему не перечислимый `TYPE`.** Проба П4 (0041-06): MatIEC отвергает
/// `TYPE F : (A := 80); END_TYPE` — явные значения вариантов появились в 3-й
/// редакции IEC 61131-3. Перечисления Takt значения **имеют**
/// (`enum Floor { Bottom = 80, Top }`), поэтому прямое отображение непригодно.
///
/// **Почему разрядность считается, а не берётся `USINT`.** ADR (Option C)
/// предполагал плоский `USINT`, но это предположение не выдерживает корпуса:
/// `enum Action { Idle = 670, Closing }` (`examples/elevator.lam:121`) в `USINT`
/// **не помещается** — 670 > 255. Плоский `USINT` дал бы тихое усечение, то есть
/// ровно тот класс дефекта («тихий пропуск»), против которого написана фича.
/// Разрядность выбирается по фактическому диапазону вариантов — так же, как это
/// делает цель `c` (`c/mod.rs:167-190`).
fn enum_type(name: &str, model: &ModelNode) -> Result<String, Diagnostic> {
    let node = model
        .search_enum(name)
        .ok_or_else(|| unresolved("Перечисление", name))?;
    // Знак и ширина — из общего факта (фича 0060): цель лишь отображает его в имя
    // типа IEC через `integer_type`. Свой каскад извлечения диапазона удалён
    // (в `generator/` не остаётся ни одного — ADR 0060, правило 5).
    match enum_facts(&node.variants) {
        Some(f) => integer_type(f.machine_bits() as u8, f.signed),
        // Пустое перечисление — поведение сохраняется сегодняшним (`USINT`).
        None => integer_type(8, false),
    }
}

/// T14: `Struct(name)` → ссылка на объявленный `TYPE`; объявление печатает `st_decl`.
fn struct_type(name: &str, model: &ModelNode) -> Result<String, Diagnostic> {
    if model.search_struct(name).is_none() {
        return Err(unresolved("Структура", name));
    }
    Ok(name.to_string())
}

/// Строит диагностику `ST-008` — именованный тип не разрешается в модели.
fn unresolved(kind: &str, name: &str) -> Diagnostic {
    Diagnostic::error(
        Location::Codegen,
        format!(
            "{} '{}' не найдена в модели: объявление типа для ST построить нельзя",
            kind, name
        ),
    )
    .with_code("ST-008")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::tree::construct_model;
    use crate::semantic::{EnumDefinitionNode, StructDefinitionNode};

    /// Пустая модель — для типов, не требующих разрешения имён.
    ///
    /// `ModelNode` не реализует `Clone`, поэтому модель живёт в `Rc<RefCell<…>>`
    /// и заимствуется по месту.
    fn empty_model() -> std::rc::Rc<std::cell::RefCell<ModelNode>> {
        let (ast, _) = crate::parse("start S;", 0).unwrap();
        construct_model(&ast, None, &[]).unwrap()
    }

    fn st_of(typ: &TypeNode) -> String {
        get_st_type(typ, &empty_model().borrow()).expect("тип должен отображаться")
    }

    fn code_of(typ: &TypeNode, model: &ModelNode) -> String {
        get_st_type(typ, model)
            .expect_err("ожидалась диагностика")
            .code
            .expect("диагностика обязана иметь код")
    }

    /// T1: `bit` → `BOOL`, а не `int` (дефект Д2 цели `c`).
    #[test]
    fn test_get_st_type_bit_is_bool_not_int() {
        assert_eq!(st_of(&TypeNode::Bit), "BOOL");
    }

    /// T2: `bool` → `BOOL`.
    #[test]
    fn test_get_st_type_bool() {
        assert_eq!(st_of(&TypeNode::Bool), "BOOL");
    }

    /// T3..T10: целые. `INT` в IEC — 16-битный знаковый, это не опечатка.
    #[test]
    fn test_get_st_type_integers_follow_iec_widths() {
        let cases = [
            (8, false, "USINT"),
            (16, false, "UINT"),
            (32, false, "UDINT"),
            (64, false, "ULINT"),
            (8, true, "SINT"),
            (16, true, "INT"),
            (32, true, "DINT"),
            (64, true, "LINT"),
        ];
        for (bits, signed, expected) in cases {
            assert_eq!(
                st_of(&TypeNode::Integer { bits, signed }),
                expected,
                "разрядность {bits}, знаковый={signed}"
            );
        }
    }

    /// Нестандартная разрядность — ошибка, а не молчаливая подстановка.
    #[test]
    fn test_get_st_type_odd_integer_width_is_error() {
        assert_eq!(
            code_of(
                &TypeNode::Integer {
                    bits: 24,
                    signed: false
                },
                &empty_model().borrow()
            ),
            "ST-002"
        );
    }

    /// T11: `float` → `LREAL` (f64 — как симулятор), **не** `REAL` (дефект Д3).
    #[test]
    fn test_get_st_type_rational_is_lreal_not_real() {
        assert_eq!(st_of(&TypeNode::Rational), "LREAL");
    }

    /// T12: массив — настоящий `ARRAY`, а не `uint{N}_t` (дефект Д1 цели `c`).
    ///
    /// Тот же вход, на котором цель `c` теряет переменную: `[u8; 4]` → `uint4_t`.
    #[test]
    fn test_get_st_type_array_uses_element_count_as_range() {
        assert_eq!(
            st_of(&TypeNode::Array(
                4,
                Box::new(TypeNode::Integer {
                    bits: 8,
                    signed: false
                })
            )),
            "ARRAY [0..3] OF USINT"
        );
    }

    /// T12: вложенный массив уплощается в многомерный.
    ///
    /// Форма продиктована фактом, а не вкусом: `iec2c` отвергает
    /// `ARRAY [0..2] OF ARRAY [0..1] OF USINT` («invalid item data type in array
    /// specification»), но принимает `ARRAY [0..2, 0..1] OF USINT`. Порядок
    /// размерностей — внешняя первой, как в индексации Takt.
    #[test]
    fn test_get_st_type_nested_array_is_flattened_to_multidim() {
        let inner = TypeNode::Array(
            2,
            Box::new(TypeNode::Integer {
                bits: 8,
                signed: false,
            }),
        );
        assert_eq!(
            st_of(&TypeNode::Array(3, Box::new(inner))),
            "ARRAY [0..2, 0..1] OF USINT"
        );
    }

    /// Массив нулевого размера невыразим → `ST-007`, а не пустой диапазон.
    #[test]
    fn test_get_st_type_zero_sized_array_is_st007() {
        assert_eq!(
            code_of(
                &TypeNode::Array(0, Box::new(TypeNode::Bit)),
                &empty_model().borrow()
            ),
            "ST-007"
        );
    }

    /// T13: перечисление — целое, вмещающее варианты (откат Option C).
    ///
    /// Значения сняты зондом с `examples/elevator.lam:117`: `Floor { Bottom = 80,
    /// Top }` даёт варианты `[("Bottom", 80), ("Top", 81)]` — `Top` наследует 81.
    #[test]
    fn test_get_st_type_enum_fits_in_usint() {
        let rc = empty_model();
        rc.borrow_mut().enums.insert(
            "Floor".to_string(),
            EnumDefinitionNode::new("Floor", &[("Bottom", Some(80)), ("Top", None)]),
        );
        assert_eq!(
            get_st_type(&TypeNode::Enum("Floor".to_string()), &rc.borrow()).unwrap(),
            "USINT"
        );
    }

    /// Перечисление шире байта получает более широкий тип, а не усекается.
    ///
    /// Вход не гипотетический: `enum Action { Idle = 670, Closing }` —
    /// `examples/elevator.lam:121`. Плоский `USINT` (как предполагал ADR) усёк бы
    /// 670 молча.
    #[test]
    fn test_get_st_type_enum_wider_than_byte_is_widened_not_truncated() {
        let rc = empty_model();
        rc.borrow_mut().enums.insert(
            "Action".to_string(),
            EnumDefinitionNode::new("Action", &[("Idle", Some(670)), ("Closing", None)]),
        );
        assert_eq!(
            get_st_type(&TypeNode::Enum("Action".to_string()), &rc.borrow()).unwrap(),
            "UINT",
            "670 не помещается в USINT — тип обязан расшириться"
        );
    }

    /// Отрицательные варианты требуют знакового типа.
    #[test]
    fn test_get_st_type_enum_with_negative_variant_is_signed() {
        let rc = empty_model();
        rc.borrow_mut().enums.insert(
            "Dir".to_string(),
            EnumDefinitionNode::new("Dir", &[("Down", Some(-1)), ("Up", Some(1))]),
        );
        assert_eq!(
            get_st_type(&TypeNode::Enum("Dir".to_string()), &rc.borrow()).unwrap(),
            "SINT"
        );
    }

    /// Неразрешимое перечисление → `ST-008`, а не тихий пропуск.
    #[test]
    fn test_get_st_type_unresolved_enum_is_st008() {
        assert_eq!(
            code_of(
                &TypeNode::Enum("Missing".to_string()),
                &empty_model().borrow()
            ),
            "ST-008"
        );
    }

    /// T14: структура → ссылка на имя объявленного `TYPE`.
    #[test]
    fn test_get_st_type_struct_refers_to_declared_type() {
        let rc = empty_model();
        rc.borrow_mut().structs.insert(
            "Point".to_string(),
            StructDefinitionNode::new("Point", &[("x", TypeNode::Bit)]),
        );
        assert_eq!(
            get_st_type(&TypeNode::Struct("Point".to_string()), &rc.borrow()).unwrap(),
            "Point"
        );
    }

    /// Неразрешимая структура → `ST-008`.
    #[test]
    fn test_get_st_type_unresolved_struct_is_st008() {
        assert_eq!(
            code_of(
                &TypeNode::Struct("Missing".to_string()),
                &empty_model().borrow()
            ),
            "ST-008"
        );
    }

    /// T16: служебные типы → `ST-002` (ошибка), а не `None` (дефект Д4 цели `c`).
    #[test]
    fn test_get_st_type_unmappable_types_are_st002() {
        let cases = [
            TypeNode::Inference,
            TypeNode::Unsupported,
            TypeNode::Unit,
            TypeNode::Address(0x100, Some(0)),
            TypeNode::BuiltinString,
            TypeNode::BuiltinModel,
            TypeNode::BuiltinState,
            TypeNode::BuiltinNumeric,
        ];
        let rc = empty_model();
        for typ in cases {
            assert_eq!(code_of(&typ, &rc.borrow()), "ST-002", "тип {:?}", typ);
        }
    }

    /// Полнота разбора: каждый вариант `TypeNode` даёт либо тип, либо ошибку с
    /// кодом — но никогда не `None`-подобный тихий исход (R4.3).
    ///
    /// `TypeNode` помечен `#[non_exhaustive]`, поэтому исчерпывающий `match` в
    /// `get_st_type` невозможен и компилятор о новом варианте не предупредит.
    /// Этот тест — замена такому предупреждению: список ведётся вручную и его
    /// расхождение с перечислением ловится ревью (см. оговорку в анализе 0041-02).
    #[test]
    fn test_get_st_type_covers_all_variants() {
        let rc = empty_model();
        rc.borrow_mut().enums.insert(
            "E".to_string(),
            EnumDefinitionNode::new("E", &[("A", None)]),
        );
        rc.borrow_mut().structs.insert(
            "S".to_string(),
            StructDefinitionNode::new("S", &[("f", TypeNode::Bit)]),
        );
        let all = [
            TypeNode::Inference,
            TypeNode::Address(0, None),
            TypeNode::Bit,
            TypeNode::Bool,
            TypeNode::Rational,
            TypeNode::Array(1, Box::new(TypeNode::Bit)),
            TypeNode::Enum("E".to_string()),
            TypeNode::Unsupported,
            TypeNode::Unit,
            TypeNode::BuiltinString,
            TypeNode::BuiltinModel,
            TypeNode::BuiltinState,
            TypeNode::BuiltinNumeric,
            TypeNode::Struct("S".to_string()),
            TypeNode::Integer {
                bits: 8,
                signed: false,
            },
        ];
        assert_eq!(
            all.len(),
            15,
            "список вариантов TypeNode разошёлся с тестом"
        );
        for typ in all {
            match get_st_type(&typ, &rc.borrow()) {
                Ok(name) => assert!(!name.is_empty(), "пустое имя типа для {:?}", typ),
                Err(d) => assert!(
                    d.code.is_some(),
                    "отказ без кода диагностики для {:?} — это тихий пропуск",
                    typ
                ),
            }
        }
    }
}
