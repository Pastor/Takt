use crate::diagnostics::{Diagnostic, Location};
use crate::generator::c;
use crate::generator::c::c_map::CMap;
use crate::generator::c::{
    FUNCTION_PORT_READ_BIT, FUNCTION_PORT_READ_FLOAT, FUNCTION_PORT_READ_NUMERIC,
    FUNCTION_PORT_WRITE_BIT, FUNCTION_PORT_WRITE_FLOAT, FUNCTION_PORT_WRITE_NUMERIC, PortClass,
    typed_variable_or_diagnostic,
};
use crate::generator::indent::Printer;
use crate::semantic::minimap::{Element, Name, StateExtend};
use crate::semantic::naming::normalize_lowercase_snakecase;
use crate::semantic::{PortDirection, VariableNode};
use log::warn;

/// Словарь портов: `(класс порта, направление)` → список `(имя модели, имя порта)`.
type PortMap = std::collections::HashMap<(PortClass, PortDirection), Vec<(Name, String)>>;

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

/// Собирает все порты из всех моделей, сгруппированные по [`PortClass`] и [`PortDirection`].
///
/// Возвращает словарь `(PortClass, PortDirection) → Vec<(model_name, port_name)>`.
/// Порты отсортированы для детерминированной генерации.
fn collect_ports_by_class(map: &CMap) -> Result<PortMap, Diagnostic> {
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
    let mut result: HashMap<(PortClass, PortDirection), Vec<(Name, String)>> = HashMap::new();
    for element in &all_models {
        let model_name = element.name();
        let model = map.raw_model_at(model_name.clone())?;
        let model_borrowed = model.borrow();
        let mut ports: Vec<(Name, String, PortClass, PortDirection)> = model_borrowed
            .variables
            .values()
            .filter_map(|v| {
                if let VariableNode::Port {
                    name,
                    ty,
                    direction,
                    ..
                } = v
                {
                    Some((
                        model_name.clone(),
                        name.clone(),
                        PortClass::from_type(ty),
                        *direction,
                    ))
                } else {
                    None
                }
            })
            .collect();
        ports.sort_by(|(_, a, _, _), (_, b, _, _)| a.cmp(b));
        for (mname, pname, cls, dir) in ports {
            match dir {
                PortDirection::InOut => {
                    // двунаправленный порт появляется в обоих enum-ах
                    result
                        .entry((cls, PortDirection::In))
                        .or_default()
                        .push((mname.clone(), pname.clone()));
                    result
                        .entry((cls, PortDirection::Out))
                        .or_default()
                        .push((mname, pname));
                }
                _ => {
                    result.entry((cls, dir)).or_default().push((mname, pname));
                }
            }
        }
    }
    Ok(result)
}

/// Генерирует тип-зависимые перечисления портов с разделением по направлению.
///
/// Для каждой комбинации `(PortClass, PortDirection)` генерируется отдельный `typedef enum`:
/// `{Root}_In_BitPort`, `{Root}_Out_BitPort`, `{Root}_In_NumericPort` и т. п.
/// Варианты именуются `{MODEL_UPPER}_{PORT_UPPER}` с последовательными значениями.
/// Определения помещаются в заголовочный файл до struct-определений.
fn generate_port_enums(printer: &mut Printer, map: &CMap) -> Result<(), Diagnostic> {
    let root_camelcase = map.root_name().unique_camelcase();
    let by_class = collect_ports_by_class(map)?;
    for cls in [PortClass::Bit, PortClass::Rational, PortClass::Numeric] {
        for dir in [PortDirection::In, PortDirection::Out] {
            let Some(ports) = by_class.get(&(cls, dir)) else {
                continue;
            };
            let type_name = cls.qualified_enum_name_with_dir(&root_camelcase, dir);
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
                // 0029-01: прежде отказ отображения давал CC-009 «Variable not
                // found» — ошибку не по адресу: переменная найдена, невыразим
                // её тип. Теперь причина доходит до пользователя (CC-014/015).
                let tv = typed_variable_or_diagnostic(
                    &ty,
                    &name,
                    &*model.borrow(),
                    map.float_width(),
                    &format!("переменная '{}'", name),
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
        let has_in_bit = by_class.contains_key(&(PortClass::Bit, PortDirection::In));
        let has_out_bit = by_class.contains_key(&(PortClass::Bit, PortDirection::Out));
        let has_in_rational = by_class.contains_key(&(PortClass::Rational, PortDirection::In));
        let has_out_rational = by_class.contains_key(&(PortClass::Rational, PortDirection::Out));
        let has_in_numeric = by_class.contains_key(&(PortClass::Numeric, PortDirection::In));
        let has_out_numeric = by_class.contains_key(&(PortClass::Numeric, PortDirection::Out));
        let has_any = has_in_bit
            || has_out_bit
            || has_in_rational
            || has_out_rational
            || has_in_numeric
            || has_out_numeric;
        if has_any {
            printer
                .ident("/// NOTICE: Функции портов ввода вывода")
                .nl();
            printer.ident("void  *userdata;").nl();
            if has_out_bit {
                let bit_out = PortClass::Bit
                    .qualified_enum_name_with_dir(&root_camelcase, PortDirection::Out);
                printer
                    .ident(&format!(
                        "void  (*{write_bit})({bit_out} port, bool val, void *userdata);",
                        write_bit = FUNCTION_PORT_WRITE_BIT
                    ))
                    .nl();
            }
            if has_in_bit {
                let bit_in =
                    PortClass::Bit.qualified_enum_name_with_dir(&root_camelcase, PortDirection::In);
                printer
                    .ident(&format!(
                        "bool  (*{read_bit} )({bit_in} port, void *userdata);",
                        read_bit = FUNCTION_PORT_READ_BIT
                    ))
                    .nl();
            }
            if has_out_rational {
                let rat_out = PortClass::Rational
                    .qualified_enum_name_with_dir(&root_camelcase, PortDirection::Out);
                printer
                    .ident(&format!(
                        "void  (*{write_float})({rat_out} port, float val, void *userdata);",
                        write_float = FUNCTION_PORT_WRITE_FLOAT
                    ))
                    .nl();
            }
            if has_in_rational {
                let rat_in = PortClass::Rational
                    .qualified_enum_name_with_dir(&root_camelcase, PortDirection::In);
                printer
                    .ident(&format!(
                        "float (*{read_float} )({rat_in} port, void *userdata);",
                        read_float = FUNCTION_PORT_READ_FLOAT
                    ))
                    .nl();
            }
            if has_out_numeric {
                let num_out = PortClass::Numeric
                    .qualified_enum_name_with_dir(&root_camelcase, PortDirection::Out);
                printer
                    .ident(&format!(
                        "void    (*{write_numeric})({num_out} port, int64_t val, void *userdata);",
                        write_numeric = FUNCTION_PORT_WRITE_NUMERIC
                    ))
                    .nl();
            }
            if has_in_numeric {
                let num_in = PortClass::Numeric
                    .qualified_enum_name_with_dir(&root_camelcase, PortDirection::In);
                printer
                    .ident(&format!(
                        "int64_t (*{read_numeric} )({num_in} port, void *userdata);",
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

pub fn generate_header(
    filename: &str,
    map: &CMap,
    options: &crate::generator::GenerateOptions,
) -> Result<String, Diagnostic> {
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
    // раньше их полного определения (важно при взаимных ссылках).
    //
    // Typedef корня эмитится БЕЗУСЛОВНО — от наличия под-моделей он не зависит.
    // Структура печатается тегом (`struct {Root} { … };`), а прототипы объявлены
    // через голое имя (`void {Root}_init({Root} *main);`), поэтому без typedef
    // голое имя типом не становится и порождённый C НЕ КОМПИЛИРУЕТСЯ:
    // «must use 'struct' tag to refer to type».
    //
    // Прежде эмиссия была разветвлена на три случая, и ветка «под-моделей нет,
    // цель `c`» не печатала ничего — отсюда дефект. Ветку `c-hal` (правка
    // 0020-05) с ней слили в один путь: расхождение целей и было причиной того,
    // что `c-hal` работал, а `c` — нет. Единый путь не даёт этому воспроизвестись.
    if !sorted_models.is_empty() {
        printer.print("/* Forward declarations */").nl();
        for element in &sorted_models {
            let Element::Model { name, .. } = element else {
                continue;
            };
            let s = name.unique_camelcase();
            printer.print(&format!("typedef struct {0} {0};", s)).nl();
        }
    }
    let root_struct = map.root_name().unique_camelcase();
    printer
        .print(&format!("typedef struct {0} {0};", root_struct))
        .nl();
    printer.nl();

    // Генерируем typedef struct для пользовательских структур
    if let Some(model_rc) = map.root_model_node() {
        let model = model_rc.borrow();
        let mut structs: Vec<_> = model.structs.values().collect();
        structs.sort_by_key(|s| &s.name);
        if !structs.is_empty() {
            for s in structs {
                printer
                    .print(&format!("typedef struct {} {{", s.name))
                    .nl()
                    .up();
                for (field_name, field_ty) in &s.fields {
                    // 0029-01: было `/* unsupported */ имя` — поле без типа,
                    // то есть НЕВАЛИДНЫЙ C, выданный молча и с кодом 0.
                    let c_decl = c::typed_variable_or_diagnostic(
                        field_ty,
                        field_name,
                        &*model,
                        map.float_width(),
                        &format!("поле '{}' структуры '{}'", field_name, s.name),
                    )?;
                    printer.ident(&format!("{};", c_decl)).nl();
                }
                printer.down().print(&format!("}} {};", s.name)).nl();
            }
            printer.nl();
        }
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

    // Фича 0020-05: в режиме `c-hal` — таблица адресов портов и дефолтный HAL.
    if options.hal {
        generate_hal(&mut printer, map, options)?;
    }

    printer.print("#endif").nl();
    Ok(header)
}

/// Ширина разыменования (в байтах) по C-типу порта из [`get_c_type`].
///
/// **Перечисление исчерпывающее** (0029-02, R9). Прежде здесь стояло `_ => 4`,
/// и всё неузнанное читалось четырьмя байтами **молча** — тот же класс дефекта,
/// против которого заведена вся фича 0029. Пробы показали, что ветка достижима
/// и даёт неверный результат: битовый порт читался 4 байтами (через `Bit` →
/// `int`), а порт структурного типа `Point` (2 байта) читается 4 байтами и
/// сегодня. На железе это доступ за пределы регистра.
///
/// `None` — ширина неизвестна: вызывающий обязан дать `CC-016`, а не выбрать
/// число за пользователя.
fn width_from_ctype(ct: &str) -> Option<u8> {
    match ct {
        "uint8_t" | "int8_t" | "bool" => Some(1),
        "uint16_t" | "int16_t" => Some(2),
        "uint32_t" | "int32_t" | "float" => Some(4),
        "uint64_t" | "int64_t" | "double" => Some(8),
        _ => None,
    }
}

/// C-тип порта `(model, name)` через [`get_c_type`] (для ширины доступа).
fn port_ctype(map: &CMap, model_name: &Name, port_name: &str) -> Option<String> {
    let model = map.raw_model_at(model_name.clone()).ok()?;
    let borrowed = model.borrow();
    let VariableNode::Port { ty, .. } = borrowed.variables.get(port_name)? else {
        return None;
    };
    c::get_c_type(ty, &borrowed, map.float_width())
}

/// Фича 0020-05: эмитит таблицу адресов портов и дефолтную реализацию HAL.
///
/// Для каждой пары `(класс, направление)` генерируется `static const`-таблица
/// `{Enum}__ADDR[]`, индексируемая enum-вариантами порта, и дефолтные
/// `read_*`/`write_*` через `*(volatile T*)addr`. Помощник `bind_default_hal`
/// связывает указатели структуры с этими функциями (только для присутствующих
/// классов/направлений).
fn generate_hal(
    printer: &mut Printer,
    map: &CMap,
    options: &crate::generator::GenerateOptions,
) -> Result<(), Diagnostic> {
    let root = map.root_name().unique_camelcase();
    let by_class = collect_ports_by_class(map)?;
    let addr_map = &options.address_map;
    let has = |c: PortClass, d: PortDirection| by_class.contains_key(&(c, d));

    printer.nl();
    printer
        .print("/* 0020: карта адресов портов и дефолтный HAL */")
        .nl();
    printer
        .print(&format!(
            "typedef struct {{ uintptr_t addr; int8_t bit; uint8_t width; }} {}_PortBinding;",
            root
        ))
        .nl();
    printer.nl();

    // Таблицы адресов для всех присутствующих (класс, направление).
    for cls in [PortClass::Bit, PortClass::Rational, PortClass::Numeric] {
        for dir in [PortDirection::In, PortDirection::Out] {
            let Some(ports) = by_class.get(&(cls, dir)) else {
                continue;
            };
            let enum_type = cls.qualified_enum_name_with_dir(&root, dir);
            printer
                .print(&format!(
                    "static const {root}_PortBinding {enum_type}__ADDR[] = {{"
                ))
                .nl();
            printer.up();
            for (model_name, port_name) in ports {
                let variant = format!(
                    "{}_{}",
                    model_name.unique_uppercase_snakecase(),
                    normalize_lowercase_snakecase(port_name.clone()).to_uppercase()
                );
                let resolved = addr_map.get(port_name);
                let addr = resolved.map(|r| r.addr).unwrap_or(0);
                let bit = resolved.and_then(|r| r.bit).unwrap_or(-1);
                // 0029-02: было `.unwrap_or(4)` поверх `_ => 4` — два молчаливых
                // умолчания подряд. Ширина доступа к MMIO угадыванию не подлежит.
                let ct = port_ctype(map, model_name, port_name).ok_or_else(|| {
                    Diagnostic::error(
                        Location::Codegen,
                        format!(
                            "порт '{}' модели '{}': тип не представим в C — \
                             ширина доступа к регистру неизвестна",
                            port_name, model_name
                        ),
                    )
                    .with_code("CC-015")
                })?;
                let width = width_from_ctype(&ct).ok_or_else(|| {
                    Diagnostic::error(
                        Location::Codegen,
                        format!(
                            "порт '{}' модели '{}': ширина доступа к регистру \
                             неизвестна для типа C '{}'",
                            port_name, model_name, ct
                        ),
                    )
                    .with_code("CC-016")
                })?;
                printer
                    .ident(&format!(
                        "[{variant}] = {{ (uintptr_t)0x{addr:X}u, {bit}, {width} }},",
                        addr = addr as u64,
                    ))
                    .nl();
            }
            printer.down();
            printer.print("};").nl();
        }
    }
    printer.nl();

    // Дефолтные функции чтения/записи (только для присутствующих классов).
    if has(PortClass::Bit, PortDirection::In) {
        let e = PortClass::Bit.qualified_enum_name_with_dir(&root, PortDirection::In);
        printer
            .print(&format!(
                "static bool {root}_default_{f}({e} p, void *userdata) {{ (void)userdata; \
             {root}_PortBinding b = {e}__ADDR[p]; \
             return ((*(volatile uint8_t*)b.addr) >> (b.bit < 0 ? 0 : b.bit)) & 1u; }}",
                f = FUNCTION_PORT_READ_BIT,
            ))
            .nl();
    }
    if has(PortClass::Bit, PortDirection::Out) {
        let e = PortClass::Bit.qualified_enum_name_with_dir(&root, PortDirection::Out);
        printer.print(&format!(
            "static void {root}_default_{f}({e} p, bool val, void *userdata) {{ (void)userdata; \
             {root}_PortBinding b = {e}__ADDR[p]; \
             volatile uint8_t *reg = (volatile uint8_t*)b.addr; \
             uint8_t mask = (uint8_t)(1u << (b.bit < 0 ? 0 : b.bit)); \
             if (val) *reg |= mask; else *reg &= (uint8_t)~mask; }}",
            f = FUNCTION_PORT_WRITE_BIT,
        )).nl();
    }
    if has(PortClass::Rational, PortDirection::In) {
        let e = PortClass::Rational.qualified_enum_name_with_dir(&root, PortDirection::In);
        printer
            .print(&format!(
                "static float {root}_default_{f}({e} p, void *userdata) {{ (void)userdata; \
             {root}_PortBinding b = {e}__ADDR[p]; return *(volatile float*)b.addr; }}",
                f = FUNCTION_PORT_READ_FLOAT,
            ))
            .nl();
    }
    if has(PortClass::Rational, PortDirection::Out) {
        let e = PortClass::Rational.qualified_enum_name_with_dir(&root, PortDirection::Out);
        printer.print(&format!(
            "static void {root}_default_{f}({e} p, float val, void *userdata) {{ (void)userdata; \
             {root}_PortBinding b = {e}__ADDR[p]; *(volatile float*)b.addr = val; }}",
            f = FUNCTION_PORT_WRITE_FLOAT,
        )).nl();
    }
    if has(PortClass::Numeric, PortDirection::In) {
        let e = PortClass::Numeric.qualified_enum_name_with_dir(&root, PortDirection::In);
        printer
            .print(&format!(
                "static int64_t {root}_default_{f}({e} p, void *userdata) {{ (void)userdata; \
             {root}_PortBinding b = {e}__ADDR[p]; switch (b.width) {{ \
             case 1: return (int64_t)*(volatile uint8_t*)b.addr; \
             case 2: return (int64_t)*(volatile uint16_t*)b.addr; \
             case 8: return (int64_t)*(volatile uint64_t*)b.addr; \
             default: return (int64_t)*(volatile uint32_t*)b.addr; }} }}",
                f = FUNCTION_PORT_READ_NUMERIC,
            ))
            .nl();
    }
    if has(PortClass::Numeric, PortDirection::Out) {
        let e = PortClass::Numeric.qualified_enum_name_with_dir(&root, PortDirection::Out);
        printer.print(&format!(
            "static void {root}_default_{f}({e} p, int64_t val, void *userdata) {{ (void)userdata; \
             {root}_PortBinding b = {e}__ADDR[p]; switch (b.width) {{ \
             case 1: *(volatile uint8_t*)b.addr = (uint8_t)val; break; \
             case 2: *(volatile uint16_t*)b.addr = (uint16_t)val; break; \
             case 8: *(volatile uint64_t*)b.addr = (uint64_t)val; break; \
             default: *(volatile uint32_t*)b.addr = (uint32_t)val; break; }} }}",
            f = FUNCTION_PORT_WRITE_NUMERIC,
        )).nl();
    }
    printer.nl();

    // Помощник связывания дефолтного HAL со структурой модели.
    printer
        .print(&format!(
            "static void {root}_bind_default_hal({root} *m) {{"
        ))
        .nl();
    printer.up();
    let bindings: [(bool, &str); 6] = [
        (
            has(PortClass::Bit, PortDirection::In),
            FUNCTION_PORT_READ_BIT,
        ),
        (
            has(PortClass::Bit, PortDirection::Out),
            FUNCTION_PORT_WRITE_BIT,
        ),
        (
            has(PortClass::Rational, PortDirection::In),
            FUNCTION_PORT_READ_FLOAT,
        ),
        (
            has(PortClass::Rational, PortDirection::Out),
            FUNCTION_PORT_WRITE_FLOAT,
        ),
        (
            has(PortClass::Numeric, PortDirection::In),
            FUNCTION_PORT_READ_NUMERIC,
        ),
        (
            has(PortClass::Numeric, PortDirection::Out),
            FUNCTION_PORT_WRITE_NUMERIC,
        ),
    ];
    for (present, field) in bindings {
        if present {
            printer
                .ident(&format!("m->{field} = {root}_default_{field};"))
                .nl();
        }
    }
    printer.down();
    printer.print("}").nl();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::c::c_map::CMap;
    use crate::{parse, semantic};

    /// Строит `.h` для исходника с именем корневой модели.
    fn generate_h_content(src: &str, name: &str) -> String {
        let (model_ast, _) = parse(src, 0).unwrap();
        let model = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model.borrow_mut().name = Some(name.to_string());
        let model = model.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        generate_header(
            map.get_filename(),
            &map,
            &crate::generator::GenerateOptions::default(),
        )
        .unwrap()
    }

    /// Строит `.h` цели `c-hal` с разрешёнными адресами.
    ///
    /// Адреса разрешаются тем же путём, что и `lamc -t c-hal`
    /// (`resolve_addresses`), — таблица `*__ADDR` иначе не эмитится.
    fn generate_hal_h(src: &str, name: &str, float_width: crate::generator::FloatWidth) -> String {
        let (model_ast, _) = parse(src, 0).unwrap();
        let model = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model.borrow_mut().name = Some(name.to_string());
        let resolution = crate::address_map::resolve_addresses(std::rc::Rc::clone(&model), &[]);
        let model = model.borrow();
        let map = CMap::new(model.name(), &*model, true)
            .unwrap()
            .with_float_width(float_width);
        let mut options = crate::generator::GenerateOptions::default();
        options.hal = true;
        options.address_map = resolution.map;
        options.float_width = float_width;
        generate_header(map.get_filename(), &map, &options).unwrap()
    }

    /// Исходник цели `c-hal` с портами трёх ширин.
    const HAL_SRC: &str = r#"
in temperature: float;
in sensor: bit;
in level: u16;

address temperature = 0x1000;
address sensor = 0x2000;
address level = 0x3000;

var reading: float := 0.0;

start Idle {
    always {
        reading := temperature + 1.0;
    }
}
"#;

    /// **T10 (0029-02/03).** Ширина доступа к MMIO по типу порта.
    ///
    /// Значения **захвачены зондом** (`lamc -t c-hal`), а не угаданы. Ловит два
    /// исправления фичи 0029 сразу:
    /// - битовый порт — **1** байт (было 4: `Bit` → `int` → `_ => 4`); чтение
    ///   4 байтами из однобайтового регистра — доступ за его пределы;
    /// - вещественный порт — **8** байт (было 4: `Rational` → `float`); это
    ///   ожидаемая цена умолчания `--float-width=64`, решение заказчика.
    #[test]
    fn test_hal_port_width_follows_c_type() {
        let header = generate_hal_h(HAL_SRC, "Hal", crate::generator::FloatWidth::W64);
        assert!(
            header.contains("[HAL_SENSOR] = { (uintptr_t)0x2000u, -1, 1 },"),
            "битовый порт обязан читаться 1 байтом:\n{header}"
        );
        assert!(
            header.contains("[HAL_TEMPERATURE] = { (uintptr_t)0x1000u, -1, 8 },"),
            "вещественный порт при умолчании W64 — 8 байт:\n{header}"
        );
        assert!(
            header.contains("[HAL_LEVEL] = { (uintptr_t)0x3000u, -1, 2 },"),
            "u16 — 2 байта, без изменений:\n{header}"
        );
    }

    /// **T11 (0029-03).** `--float-width=32` возвращает вещественному порту 4
    /// байта — для платформ, где 8-байтное чтение недопустимо.
    #[test]
    fn test_hal_float_port_width_is_4_with_float_width_32() {
        let header = generate_hal_h(HAL_SRC, "Hal", crate::generator::FloatWidth::W32);
        assert!(
            header.contains("[HAL_TEMPERATURE] = { (uintptr_t)0x1000u, -1, 4 },"),
            "при W32 вещественный порт — 4 байта:\n{header}"
        );
        // Прочие ширины от флага не зависят.
        assert!(
            header.contains("[HAL_SENSOR] = { (uintptr_t)0x2000u, -1, 1 },"),
            "битовый порт от --float-width не зависит:\n{header}"
        );
    }

    /// **R9 (0029-02).** Таблица ширин исчерпывающая: неузнанный тип даёт
    /// `None`, а не молчаливые 4 байта.
    ///
    /// Достижимость проверена зондом: порт структурного типа `Point` (2 байта)
    /// получал `width 4` — доступ за пределы регистра, выданный молча.
    #[test]
    fn test_width_from_ctype_has_no_silent_default() {
        assert_eq!(width_from_ctype("uint8_t"), Some(1));
        assert_eq!(width_from_ctype("bool"), Some(1));
        assert_eq!(width_from_ctype("uint16_t"), Some(2));
        assert_eq!(width_from_ctype("float"), Some(4));
        assert_eq!(width_from_ctype("uint32_t"), Some(4));
        assert_eq!(width_from_ctype("double"), Some(8));
        assert_eq!(width_from_ctype("uint64_t"), Some(8));
        assert_eq!(
            width_from_ctype("Point"),
            None,
            "структурный тип: ширина неизвестна → CC-016, а не 4 байта молча"
        );
        assert_eq!(
            width_from_ctype("int"),
            None,
            "`int` больше не порождается get_c_type; узнавать его нечего"
        );
    }

    /// **T1.** Одиночная модель БЕЗ под-моделей получает typedef корня.
    ///
    /// Дефект фичи 0026: typedef эмитился только при наличии под-моделей либо в
    /// цели `c-hal`, а ветка «под-моделей нет, цель `c`» не печатала ничего.
    /// Структура печатается ТЕГОМ (`struct Demo { … };`), а прототипы — через
    /// голое имя (`void Demo_init(Demo *main);`), поэтому без typedef имя типом
    /// не становится.
    #[test]
    fn test_single_model_without_submodels_gets_root_typedef() {
        let header = generate_h_content(
            "var n: u8 := 0;\nstart S { always { n := n + 1; } }",
            "Demo",
        );
        assert_eq!(
            header.matches("typedef struct Demo Demo;").count(),
            1,
            "typedef корня обязан быть ровно один:\n{header}"
        );
    }

    /// **T1.** Модель С под-моделями typedef корня не теряет и не дублирует.
    ///
    /// Сторож слияния веток: раньше эту ветку правка не трогала, и она обязана
    /// остаться прежней.
    #[test]
    fn test_model_with_submodels_still_has_exactly_one_root_typedef() {
        let header = generate_h_content(
            "model A { start Q { } }\nvar n: u8 := 0;\nstart Entry = A;",
            "Demo",
        );
        assert_eq!(
            header.matches("typedef struct Demo Demo;").count(),
            1,
            "typedef корня обязан быть ровно один:\n{header}"
        );
        assert!(
            header.contains("typedef struct DemoA DemoA;"),
            "forward-декларация под-модели потеряна:\n{header}"
        );
    }

    /// **T3.** Заголовок одиночной модели самодостаточен: голое имя — тип.
    #[test]
    fn test_root_name_is_usable_as_type_in_prototypes() {
        let header = generate_h_content(
            "var n: u8 := 0;\nstart S { always { n := n + 1; } }",
            "Demo",
        );
        let typedef = header
            .find("typedef struct Demo Demo;")
            .expect("нет typedef корня");
        let proto = header.find("Demo_init(Demo *").expect("нет прототипа init");
        assert!(
            typedef < proto,
            "typedef обязан предшествовать прототипу, иначе имя ещё не тип:\n{header}"
        );
    }

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
var p: Priority := Low;
start Main { always { p := High; } }
        "#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model.borrow_mut().name = Some("Test".to_string());
        let model = model.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        let header = generate_header(
            map.get_filename(),
            &map,
            &crate::generator::GenerateOptions::default(),
        )
        .unwrap();
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
var lv: Levels := Low;
start Main { always { lv := High; } }
        "#;
        let (model_ast2, _) = parse(src2, 0).unwrap();
        let model2 = semantic::tree::construct_model(&model_ast2, None, &[]).unwrap();
        model2.borrow_mut().name = Some("Test2".to_string());
        let model2 = model2.borrow();
        let map2 = CMap::new(model2.name(), &*model2, true).unwrap();
        let header2 = generate_header(
            map2.get_filename(),
            &map2,
            &crate::generator::GenerateOptions::default(),
        )
        .unwrap();
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
        let header = generate_header(
            map.get_filename(),
            &map,
            &crate::generator::GenerateOptions::default(),
        )
        .unwrap();
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
        let header = generate_header(
            map.get_filename(),
            &map,
            &crate::generator::GenerateOptions::default(),
        )
        .unwrap();
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
        let header = generate_header(
            map.get_filename(),
            &map,
            &crate::generator::GenerateOptions::default(),
        )
        .unwrap();
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

    /// Struct с полями генерирует `typedef struct Name { ... } Name;` в заголовке.
    #[test]
    fn struct_typedef_in_header() {
        let src = r#"
struct Point { x: [bit;16], y: [bit;16] }
var pos: Point := 0;
start Main { always { pos.x := 1; } }
        "#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model.borrow_mut().name = Some("Test".to_string());
        let model = model.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        let header = generate_header(
            map.get_filename(),
            &map,
            &crate::generator::GenerateOptions::default(),
        )
        .unwrap();
        assert!(
            header.contains("typedef struct Point"),
            "ожидался typedef struct Point:\n{header}"
        );
        assert!(
            header.contains("} Point;"),
            "ожидалось закрытие }} Point;:\n{header}"
        );
    }
}
