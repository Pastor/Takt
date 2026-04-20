use crate::diagnostics::{Diagnostic, Location};
use crate::generator::c;
use crate::generator::c::c_map::CMap;
use crate::generator::c::{
    FUNCTION_PORT_READ_BIT, FUNCTION_PORT_READ_FLOAT, FUNCTION_PORT_READ_NUMERIC,
    FUNCTION_PORT_WRITE_BIT, FUNCTION_PORT_WRITE_FLOAT, FUNCTION_PORT_WRITE_NUMERIC, PortClass,
    get_typed_variable,
};
use crate::generator::indent::Printer;
use crate::semantic::VariableNode;
use crate::semantic::minimap::{Element, Name, StateExtend};
use crate::semantic::naming::normalize_lowercase_snakecase;
use log::warn;

/// Генерирует поля структуры C для extend состояния.
/// Единичный Model → `{state}`, составной → делегирует в build_concat_item.
fn build_extend_header(
    printer: &mut Printer,
    state_name: &Name,
    extend: &StateExtend,
) -> Result<(), Diagnostic> {
    match extend {
        StateExtend::None => {}
        StateExtend::Model(model) => {
            printer
                .ident(&format!(
                    "{} {};",
                    model.unique_camelcase(),
                    state_name.local_lowercase_snakecase(),
                ))
                .nl();
        }
        StateExtend::Concatenation(items) => {
            for (idx, item) in items.iter().enumerate() {
                build_concat_item(printer, state_name, item, idx)?;
            }
            build_concat_state_enum(printer, state_name, items)?;
        }
        StateExtend::Parallel(items) => {
            printer.ident("struct {").nl().up();
            for (idx, item) in items.iter().enumerate() {
                build_parallel_item(printer, item, idx, &state_name.unique_lowercase_snakecase())?;
            }
            printer.ident("enum {").up().nl();
            printer
                .ident(&*(state_name.unique_uppercase_snakecase() + "_INIT,"))
                .nl();
            printer
                .ident(&*(state_name.unique_uppercase_snakecase() + "_TICK,"))
                .nl();
            printer
                .ident(&*(state_name.unique_uppercase_snakecase() + "_END"))
                .nl();
            printer.down().ident("} state;").nl();
            printer
                .down()
                .ident(&format!("}} {};", state_name.local_lowercase_snakecase()))
                .nl();
        }
    }
    Ok(())
}

/// Генерирует поле для элемента конкатенации.
/// Model → `{state}_{model}{idx}`, Parallel → struct `{state}_parallel{idx}`.
fn build_concat_item(
    printer: &mut Printer,
    state_name: &Name,
    extend: &StateExtend,
    idx: usize,
) -> Result<(), Diagnostic> {
    match extend {
        StateExtend::None => {}
        StateExtend::Model(model) => {
            printer
                .ident(&format!(
                    "{} {}_{}{};",
                    model.unique_camelcase(),
                    state_name.local_lowercase_snakecase(),
                    model.local_lowercase_snakecase(),
                    idx,
                ))
                .nl();
        }
        StateExtend::Parallel(items) => {
            let local_partial_name =
                format!("{}_parallel{}", state_name.local_lowercase_snakecase(), idx);
            let unique_partial_name = format!(
                "{}_parallel{}",
                state_name.unique_lowercase_snakecase(),
                idx
            );
            printer.ident("struct {").nl().up();
            for (inner_idx, item) in items.iter().enumerate() {
                build_parallel_item(printer, item, inner_idx, &unique_partial_name)?;
            }
            printer.ident("enum {").up().nl();
            printer
                .ident(&*(unique_partial_name.to_uppercase() + "_INIT,"))
                .nl();
            printer
                .ident(&*(unique_partial_name.to_uppercase() + "_TICK,"))
                .nl();
            printer
                .ident(&*(unique_partial_name.to_uppercase() + "_END"))
                .nl();
            printer.down().ident("} state;").nl();
            printer
                .down()
                .ident(&format!("}} {};", local_partial_name))
                .nl();
        }
        StateExtend::Concatenation(items) => {
            for (inner_idx, item) in items.iter().enumerate() {
                build_concat_item(printer, state_name, item, inner_idx)?;
            }
        }
    }
    Ok(())
}

/// Генерирует поле для элемента параллельного блока.
/// Model → `{model}{idx}`, вложенный Parallel → struct `parallel{idx}` с enum state.
fn build_parallel_item(
    printer: &mut Printer,
    extend: &StateExtend,
    idx: usize,
    unique_prefix: &str,
) -> Result<(), Diagnostic> {
    match extend {
        StateExtend::None => {}
        StateExtend::Model(model) => {
            printer
                .ident(&format!(
                    "{} {}{};",
                    model.unique_camelcase(),
                    model.local_lowercase_snakecase(),
                    idx,
                ))
                .nl();
        }
        StateExtend::Parallel(items) => {
            let nested_prefix = format!("{}_parallel{}", unique_prefix, idx);
            printer.ident("struct {").nl().up();
            for (inner_idx, item) in items.iter().enumerate() {
                build_parallel_item(printer, item, inner_idx, &nested_prefix)?;
            }
            printer.ident("enum {").up().nl();
            printer
                .ident(&*(nested_prefix.to_uppercase() + "_INIT,"))
                .nl();
            printer
                .ident(&*(nested_prefix.to_uppercase() + "_TICK,"))
                .nl();
            printer
                .ident(&*(nested_prefix.to_uppercase() + "_END"))
                .nl();
            printer.down().ident("} state;").nl();
            printer.down().ident(&format!("}} parallel{};", idx)).nl();
        }
        StateExtend::Concatenation(items) => {
            for (inner_idx, item) in items.iter().enumerate() {
                build_parallel_item(printer, item, inner_idx, unique_prefix)?;
            }
        }
    }
    Ok(())
}

/// Генерирует enum поля состояния для конкатенации.
/// Вариант INIT + по одному варианту на элемент: Model → {STATE}_{MODEL}{idx}, Parallel → {STATE}_PARALLEL{idx}.
fn build_concat_state_enum(
    printer: &mut Printer,
    state_name: &Name,
    items: &[StateExtend],
) -> Result<(), Diagnostic> {
    let prefix = state_name.unique_uppercase_snakecase();
    let mut variants: Vec<String> = vec![format!("{}_INIT", prefix)];
    for (idx, item) in items.iter().enumerate() {
        let variant = match item {
            StateExtend::None => continue,
            StateExtend::Model(model) => {
                format!(
                    "{}_{}{}",
                    prefix,
                    model.local_lowercase_snakecase().to_uppercase(),
                    idx,
                )
            }
            StateExtend::Parallel(_) | StateExtend::Concatenation(_) => {
                format!("{}_PARALLEL{}", prefix, idx)
            }
        };
        variants.push(variant);
    }
    variants.push(format!("{}_END", prefix));
    printer.ident("enum {").up().nl();
    let last = variants.len() - 1;
    for (i, variant) in variants.iter().enumerate() {
        if i < last {
            printer.ident(&format!("{},", variant)).nl();
        } else {
            printer.ident(variant).nl();
        }
    }
    printer
        .down()
        .ident(&format!(
            "}} {}_state;",
            state_name.local_lowercase_snakecase()
        ))
        .nl();
    Ok(())
}

/// Собирает все порты из всех моделей, сгруппированные по [`PortClass`].
///
/// Возвращает словарь `PortClass → Vec<(model_name, port_name)>`.
/// Порты отсортированы для детерминированной генерации.
fn collect_ports_by_class(
    map: &CMap,
) -> Result<std::collections::HashMap<PortClass, Vec<(Name, String)>>, Diagnostic> {
    use std::collections::HashMap;
    let mut all_models = map.using_models();
    all_models.insert(
        0,
        Element::Model {
            name: map.root_name().clone(),
            states: map.states().clone(),
            start: map.start().clone(),
        },
    );
    let mut result: HashMap<PortClass, Vec<(Name, String)>> = HashMap::new();
    for element in &all_models {
        let model_name = element.name();
        let model = map.raw_model_at(model_name.clone())?;
        let model_borrowed = model.borrow();
        let mut ports: Vec<(Name, String, PortClass)> = model_borrowed
            .variables
            .values()
            .filter_map(|v| {
                if let VariableNode::Port { name, ty, .. } = v {
                    Some((model_name.clone(), name.clone(), PortClass::from_type(ty)))
                } else {
                    None
                }
            })
            .collect();
        ports.sort_by(|(_, a, _), (_, b, _)| a.cmp(b));
        for (mname, pname, cls) in ports {
            result.entry(cls).or_default().push((mname, pname));
        }
    }
    Ok(result)
}

/// Генерирует тип-зависимые перечисления портов вида `{RootModel}_BitPort` и т. п.
///
/// Все порты одного типа объединяются в единый `typedef enum`.
/// Варианты именуются `{MODEL_UPPER}_{PORT_UPPER}` с последовательными значениями.
/// Определения помещаются в заголовочный файл до struct-определений.
fn generate_port_enums(printer: &mut Printer, map: &CMap) -> Result<(), Diagnostic> {
    let root_camelcase = map.root_name().unique_camelcase();
    let by_class = collect_ports_by_class(map)?;
    for cls in [PortClass::Bit, PortClass::Rational, PortClass::Numeric] {
        let Some(ports) = by_class.get(&cls) else {
            continue;
        };
        let type_name = cls.qualified_enum_name(&root_camelcase);
        printer.print("typedef enum {").nl();
        printer.up();
        for (idx, (model_name, port_name)) in ports.iter().enumerate() {
            let variant = format!(
                "{}_{}",
                model_name.unique_uppercase_snakecase(),
                normalize_lowercase_snakecase(port_name.clone()).to_uppercase()
            );
            printer.ident(&format!("{} = {},", variant, idx)).nl();
        }
        printer.down();
        printer.print(&format!("}} {};", type_name)).nl();
        printer.nl();
    }
    Ok(())
}

fn generate_model_header(
    printer: &mut Printer,
    map: &CMap,
    name: Name,
    states: Vec<Name>,
    num: Option<usize>,
    main: bool,
) -> Result<usize, Diagnostic> {
    let num = num.unwrap_or(0);
    let model = map.raw_model_at(name.clone())?;
    printer
        .ident(format!("// NOTICE: Определение констант для модели {}", name).as_str())
        .nl();
    let struct_name = name.unique_camelcase();
    printer.print(format!("/* Model {} */", name).as_str()).nl();
    printer
        .print(format!("struct {} {{", struct_name).as_str())
        .nl();
    printer.up();
    printer
        .ident("// NOTICE: Определение переменных модели")
        .nl();
    for var in model.borrow().variables.clone().into_values() {
        match var {
            VariableNode::Unresolved => {}
            VariableNode::Simple { name, ty, .. } => {
                // Пропускаем переменные, которые нигде не используются
                if !map.usage().variables.contains(&name) {
                    continue;
                }
                let tv = get_typed_variable(&ty, Some(name.clone()), &*model.borrow()).ok_or_else(
                    || {
                        Diagnostic::error(Location::Codegen, format!("Variable {} not found", name))
                            .with_code("CC-009")
                    },
                )?;
                printer.ident(&tv).print(";").nl();
            }
            VariableNode::Port { .. } => {}
            VariableNode::Const { .. } => {}
        }
    }
    // Генерируем enum состояний модели
    let end_constant = name.unique_uppercase_snakecase() + "_END";
    printer.ident("enum {").up().nl();
    printer.ident(&*(name.unique_uppercase_snakecase() + "_INIT"));
    let mut end_already_generated = false;
    for state_name in states.clone() {
        if map.state_at(state_name.clone()).is_none() {
            continue;
        }
        printer.print(",").nl();
        let constant = state_name.unique_uppercase_snakecase();
        if constant == end_constant {
            end_already_generated = true;
        }
        printer.ident(&constant);
    }
    // Добавляем _END только если ни одно состояние не сгенерировало ту же константу
    if !end_already_generated {
        printer.print(",").nl();
        printer.ident(&end_constant);
    }
    printer.down().nl().ident("} state;").nl();
    // Генерируем поля extend-состояний
    let mut is_extend = false;
    for state_name in states {
        let Some(state) = map.state_at(state_name.clone()) else {
            warn!("State {} not used", state_name);
            continue;
        };
        if state.is_state()
            && let Element::StateExtend { extend, .. } = state
        {
            if !is_extend {
                printer.ident("// NOTICE: Определение extend").nl();
                is_extend = true;
            }
            build_extend_header(printer, &state_name, &extend)?;
        }
    }
    if main {
        let root_camelcase = map.root_name().unique_camelcase();
        let by_class = collect_ports_by_class(map)?;
        let has_bit = by_class.contains_key(&PortClass::Bit);
        let has_rational = by_class.contains_key(&PortClass::Rational);
        let has_numeric = by_class.contains_key(&PortClass::Numeric);
        if has_bit || has_rational || has_numeric {
            printer
                .ident("/// NOTICE: Функции портов ввода вывода")
                .nl();
            printer.ident("void  *userdata;").nl();
            if has_bit {
                let bit_type = PortClass::Bit.qualified_enum_name(&root_camelcase);
                printer
                    .ident(&format!(
                        "void  (*{write_bit})({bit_type} port, bool val, void *userdata);",
                        write_bit = FUNCTION_PORT_WRITE_BIT
                    ))
                    .nl();
                printer
                    .ident(&format!(
                        "bool  (*{read_bit} )({bit_type} port, void *userdata);",
                        read_bit = FUNCTION_PORT_READ_BIT
                    ))
                    .nl();
            }
            if has_rational {
                let rat_type = PortClass::Rational.qualified_enum_name(&root_camelcase);
                printer
                    .ident(&format!(
                        "void  (*{write_float})({rat_type} port, float val, void *userdata);",
                        write_float = FUNCTION_PORT_WRITE_FLOAT
                    ))
                    .nl();
                printer
                    .ident(&format!(
                        "float (*{read_float} )({rat_type} port, void *userdata);",
                        read_float = FUNCTION_PORT_READ_FLOAT
                    ))
                    .nl();
            }
            if has_numeric {
                let num_type = PortClass::Numeric.qualified_enum_name(&root_camelcase);
                printer
                    .ident(&format!(
                        "void    (*{write_numeric})({num_type} port, int64_t val, void *userdata);",
                        write_numeric = FUNCTION_PORT_WRITE_NUMERIC
                    ))
                    .nl();
                printer
                    .ident(&format!(
                        "int64_t (*{read_numeric} )({num_type} port, void *userdata);",
                        read_numeric = FUNCTION_PORT_READ_NUMERIC
                    ))
                    .nl();
            }
        }
    }

    printer.down();
    // Корректное закрытие typedef struct: } TypeName;
    printer.print("};".to_string().as_str()).nl();
    printer.nl();
    Ok(num)
}

pub fn generate_header(filename: &str, map: &CMap) -> Result<String, Diagnostic> {
    let mut header = String::new();
    let mut printer = Printer::new(4, &mut header);
    // Исправлено: заменяем '.' (не '\.') для корректного C-идентификатора #ifndef guard
    let id = normalize_lowercase_snakecase(filename.to_string())
        .replace(".", "_")
        .to_uppercase()
        + "_H__";
    printer.print("#ifndef ").print(&id).nl();
    printer.print("#define ").print(&id).nl();
    printer.print("#include <stdint.h>").nl();
    printer.print("#include <stdbool.h>").nl();
    printer.nl();

    // Топологически сортируем зависимые модели — зависимости идут первыми
    let sorted_models = c::topological_sort_models(map, map.using_models());

    // Forward declarations всех структур: позволяют компилятору C знать о типах
    // раньше их полного определения (важно при взаимных ссылках)
    if !sorted_models.is_empty() {
        printer.print("/* Forward declarations */").nl();
        for element in &sorted_models {
            let Element::Model { name, .. } = element else {
                continue;
            };
            let s = name.unique_camelcase();
            printer.print(&format!("typedef struct {0} {0};", s)).nl();
        }
        let root_struct = map.root_name().unique_camelcase();
        printer
            .print(&format!("typedef struct {0} {0};", root_struct))
            .nl();
        printer.nl();
    }

    // Генерируем enum типы для портов — до struct, чтобы можно было использовать в сигнатурах
    generate_port_enums(&mut printer, map)?;

    let mut num = 0;
    for element in sorted_models {
        let Element::Model { name, states, .. } = element else {
            continue;
        };
        generate_model_header(&mut printer, map, name, states, Some(num), false)?;
        num += 1;
    }
    generate_model_header(
        &mut printer,
        map,
        map.root_name(),
        map.states(),
        Some(num),
        true,
    )?;
    let struct_name = map.root_name().unique_camelcase();
    printer
        .print("void ")
        .print(&struct_name)
        .print("_init(")
        .print(&struct_name)
        .print(" *main);")
        .nl();
    printer
        .print("void ")
        .print(&struct_name)
        .print("_tick(")
        .print(&struct_name)
        .print(" *main);")
        .nl();
    printer
        .print("void ")
        .print(&struct_name)
        .print("_reset(")
        .print(&struct_name)
        .print(" *main);")
        .nl();
    printer
        .print("bool ")
        .print(&struct_name)
        .print("_is_done(const ")
        .print(&struct_name)
        .print(" *main);")
        .nl();
    printer.print("#endif").nl();
    Ok(header)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::c::c_map::CMap;
    use crate::{parse, semantic};

    /// Enum выбирает минимальный беззнаковый тип по максимальному значению варианта.
    ///
    /// Граница: u8::MAX = 255.
    /// - max ≤ 255  → uint8_t
    /// - 255 < max ≤ 65535  → uint16_t
    /// - max > 65535 → uint32_t (и далее uint64_t)
    #[test]
    fn enum_type_sized_by_maximum_variant() {
        // High=300 > u8::MAX(255) → uint16_t
        // p используется в always, чтобы попасть в UsageSet.
        let src = r#"
enum Priority { Low = 0, Medium = 5, High = 300 }
var p: Priority = Low;
start Main { always { p = High; } }
        "#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model.borrow_mut().name = Some("Test".to_string());
        let model = model.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        let header = generate_header(map.get_filename(), &map).unwrap();
        assert!(
            header.contains("uint16_t"),
            "ожидался uint16_t для max=300, получено:\n{header}"
        );
        assert!(
            !header.contains("uint8_t p"),
            "uint8_t слишком мал для max=300:\n{header}"
        );

        // High=200 ≤ u8::MAX(255) → uint8_t (ранее ошибочно давал uint16_t)
        // lv используется в always, чтобы попасть в UsageSet.
        let src2 = r#"
enum Levels { Low = 0, High = 200 }
var lv: Levels = Low;
start Main { always { lv = High; } }
        "#;
        let (model_ast2, _) = parse(src2, 0).unwrap();
        let model2 = semantic::tree::construct_model(&model_ast2, None, &[]).unwrap();
        model2.borrow_mut().name = Some("Test2".to_string());
        let model2 = model2.borrow();
        let map2 = CMap::new(model2.name(), &*model2, true).unwrap();
        let header2 = generate_header(map2.get_filename(), &map2).unwrap();
        assert!(
            header2.contains("uint8_t lv"),
            "ожидался uint8_t для max=200 (≤ u8::MAX=255), получено:\n{header2}"
        );
    }

    #[test]
    fn test_parallel_extend_numbering_starts_from_zero() {
        // state Par = Eng | Eng: внутри struct поля eng0, eng1 (по имени модели),
        // сам struct: `} par;` (по имени состояния, без номера).
        let src = r#"
model Eng { start S; }
start Main = Eng {
    next Par;
}
state Par = Eng | Eng;
"#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model.borrow_mut().name = Some("Test".to_string());
        let model = model.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        let header = generate_header(map.get_filename(), &map).unwrap();
        // Внутри параллельного блока поля нумеруются с 0 по имени модели
        assert!(
            header.contains("eng0;"),
            "первый элемент parallel должен быть eng0:\n{header}"
        );
        assert!(
            header.contains("eng1;"),
            "второй элемент parallel должен быть eng1:\n{header}"
        );
        // Сам struct закрывается именем состояния без номера
        assert!(
            header.contains("} par;"),
            "struct parallel должен закрываться как `}} par;`:\n{header}"
        );
        // Поля eng0/eng1 должны быть ПЕРЕД `} state;` внутри parallel struct
        // Ищем последний `} state;` до `} par;`, чтобы взять state именно parallel struct
        let pos_par_close = header.find("} par;").expect("} par; не найден");
        let pos_state = header[..pos_par_close]
            .rfind("} state;")
            .expect("} state; не найден до } par;");
        let pos_eng0 = header[..pos_par_close]
            .rfind("eng0;")
            .expect("eng0 не найден до } par;");
        assert!(
            pos_eng0 < pos_state,
            "поля должны быть перед state enum:\n{header}"
        );
    }

    #[test]
    fn test_concatenation_with_parallel_no_gaps() {
        // state Mid = Eng + (Eng | Eng) + Eng:
        // idx 0: mid_eng0, idx 1: struct{ eng0, eng1 } mid_parallel1, idx 2: mid_eng2.
        let src = r#"
model Eng { start S; }
start Main = Eng {
    next Mid;
}
state Mid = Eng + (Eng | Eng) + Eng;
"#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model.borrow_mut().name = Some("Test".to_string());
        let model = model.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        let header = generate_header(map.get_filename(), &map).unwrap();
        // Первый элемент конкатенации: {state}_{model}{idx}
        assert!(
            header.contains("mid_eng0;"),
            "первый элемент конкатенации должен быть mid_eng0:\n{header}"
        );
        // Параллельный блок под индексом 1
        assert!(
            header.contains("} mid_parallel1;"),
            "параллельный блок должен закрываться как `}} mid_parallel1;`:\n{header}"
        );
        // Последний элемент конкатенации под индексом 2
        assert!(
            header.contains("mid_eng2;"),
            "третий элемент конкатенации должен быть mid_eng2:\n{header}"
        );
        // Внутри parallel struct поля должны быть перед state enum
        // Ищем последний `} state;` до `} mid_parallel1;`
        let pos_par1_close = header
            .find("} mid_parallel1;")
            .expect("} mid_parallel1; не найден");
        let pos_state = header[..pos_par1_close]
            .rfind("} state;")
            .expect("} state; не найден до } mid_parallel1;");
        let pos_eng0 = header[..pos_par1_close]
            .rfind("eng0;")
            .expect("eng0 не найден до } mid_parallel1;");
        assert!(
            pos_eng0 < pos_state,
            "поля parallel должны быть перед state enum:\n{header}"
        );
        // concat state enum должен присутствовать
        assert!(
            header.contains("mid_state;"),
            "concat state enum mid_state должен быть сгенерирован:\n{header}"
        );
    }

    #[test]
    fn test_concatenation_generates_state_enum() {
        // state Mid = Eng + (Eng | Eng) + Eng:
        // ожидается mid_state с вариантами INIT, ENG0, PARALLEL1, ENG2.
        let src = r#"
model Eng { start S; }
start Main = Eng {
    next Mid;
}
state Mid = Eng + (Eng | Eng) + Eng;
"#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model.borrow_mut().name = Some("Test".to_string());
        let model = model.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        let header = generate_header(map.get_filename(), &map).unwrap();
        // enum поле состояния конкатенации
        assert!(
            header.contains("mid_state;"),
            "ожидался mid_state enum:\n{header}"
        );
        // вариант INIT
        assert!(
            header.contains("TEST_MID_INIT"),
            "ожидался TEST_MID_INIT:\n{header}"
        );
        // вариант для первого Model-элемента (idx=0)
        assert!(
            header.contains("TEST_MID_ENG0"),
            "ожидался TEST_MID_ENG0:\n{header}"
        );
        // вариант для Parallel-элемента (idx=1)
        assert!(
            header.contains("TEST_MID_PARALLEL1"),
            "ожидался TEST_MID_PARALLEL1:\n{header}"
        );
        // вариант для последнего Model-элемента (idx=2)
        assert!(
            header.contains("TEST_MID_ENG2"),
            "ожидался TEST_MID_ENG2:\n{header}"
        );
    }
}
