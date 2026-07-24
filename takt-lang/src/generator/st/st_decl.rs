//! Объявления переменных и типов `FUNCTION_BLOCK` для цели Structured Text.
//!
//! Задача 0041-02. Модуль печатает два пласта вывода:
//!
//! - **`TYPE … END_TYPE`** — объявления структур, общие для файла и печатаемые
//!   **до** первого использования (в IEC 61131-3 порядок объявлений значим).
//!   Пласт новый: ни цель `c` (там `struct` печатается по месту), ни `plantuml`
//!   (типы не печатаются вовсе) аналога не имеют.
//! - **`VAR_INPUT` / `VAR_OUTPUT` / `VAR_IN_OUT` / `VAR` / `VAR CONSTANT`** —
//!   секции объявлений внутри `FUNCTION_BLOCK`.
//!
//! ## Перечисления: константы вместо перечислимого типа
//!
//! Перечисление Lam не становится `TYPE … : (…); END_TYPE` — MatIEC отвергает
//! явные значения вариантов (проба П4, задача 0041-06). Действует откат Option C
//! ADR 0041: тип варианта считает [`get_st_type`], а сами варианты объявляются
//! именованными константами `<Перечисление>_<Вариант>` в секции `VAR CONSTANT`
//! **внутри** блока. Не `VAR_GLOBAL CONSTANT`, как предполагал ADR: `VAR_GLOBAL`
//! вне `CONFIGURATION` недопустим (проба П8), а цель `st` `CONFIGURATION` не
//! эмитит (проба П2).

use crate::diagnostics::Diagnostic;
use crate::generator::indent::Printer;
use crate::generator::st::st_reserved::check_st_name;
use crate::generator::st::st_type::get_st_type;
use crate::semantic::minimap::Name;
use crate::semantic::type_node::TypeNode;
use crate::semantic::unused::UsageSet;
use crate::semantic::{ExpressionNode, ModelNode, PortDirection, VariableNode};
use std::cell::RefCell;
use std::fmt::Write as _;
use std::rc::Rc;

/// Одно объявление вида `имя : ТИП := значение;` внутри секции `VAR…`.
struct Declaration {
    name: String,
    ty: String,
    init: Option<String>,
}

impl Declaration {
    /// Печатает объявление одной строкой.
    fn write(&self, p: &mut Printer) {
        let mut line = String::new();
        let _ = write!(line, "{} : {}", self.name, self.ty);
        if let Some(init) = &self.init {
            let _ = write!(line, " := {}", init);
        }
        line.push(';');
        p.ident(&line).nl();
    }
}

/// Печатает объявления структур файла как `TYPE … END_TYPE`.
///
/// Структуры собираются со **всех** моделей снимка и дедуплицируются по имени:
/// одна структура, видимая из нескольких моделей, объявляется однажды (R5.4).
/// Порядок — лексикографический: `structs` — `HashMap`, её обход
/// недетерминирован (та же первопричина, что у порядка `FUNCTION_BLOCK` в
/// `mod.rs`).
///
/// # Ошибки
/// Диагностика от [`get_st_type`], если тип поля не отображается в IEC.
pub(crate) fn emit_struct_types(
    p: &mut Printer,
    models: &[(Name, Rc<RefCell<ModelNode>>)],
) -> Result<bool, Diagnostic> {
    let mut declared: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for (_, model_rc) in models {
        let model = &*model_rc.borrow();
        let mut names: Vec<&String> = model.structs.keys().collect();
        names.sort();
        for name in names {
            if declared.iter().any(|(n, _)| n == name) {
                continue;
            }
            let node = &model.structs[name];
            let mut fields = Vec::new();
            for (field, ty) in &node.fields {
                fields.push((field.clone(), get_st_type(ty, model)?));
            }
            declared.push((name.clone(), fields));
        }
    }
    if declared.is_empty() {
        return Ok(false);
    }
    declared.sort_by(|a, b| a.0.cmp(&b.0));
    p.ident("TYPE").nl();
    p.up();
    for (name, fields) in &declared {
        p.ident(&format!("{} :", name)).nl();
        p.ident("STRUCT").nl();
        p.up();
        for (field, ty) in fields {
            p.ident(&format!("{} : {};", field, ty)).nl();
        }
        p.down();
        p.ident("END_STRUCT;").nl();
    }
    p.down();
    p.ident("END_TYPE").nl().nl();
    Ok(true)
}

/// Дополнения к объявлениям, известные только вызывающему.
///
/// Секции `VAR` печатаются один раз, поэтому всё, что рождается при печати тела
/// (поднятые объявления, экземпляры под-FB) и всё, что синтезирует генератор
/// (`state`, `is_done`, `VAR_IN_OUT`), приходит сюда, а не печатается отдельно:
/// вторая секция `VAR` в одном POU недопустима.
#[derive(Default)]
pub(crate) struct Extras {
    /// Эмитить `state : USINT := 0;` — переменную автомата.
    pub state_var: bool,
    /// Эмитить `is_done : BOOL;` в `VAR_OUTPUT` — признак завершения (S11).
    pub is_done: bool,
    /// Переменные корня, разделяемые через `VAR_IN_OUT` (О1-в).
    pub shared: Vec<(String, TypeNode)>,
    /// Экземпляры под-FB: `(имя, тип)`.
    pub instances: Vec<(String, String)>,
    /// Объявления, поднятые из тела (`st_stmt`).
    pub hoisted: Vec<(String, TypeNode)>,
    /// Цель `st-at`: порты размещены глобально, поэтому блок видит их через
    /// `VAR_EXTERNAL`, а не объявляет своими входами/выходами.
    pub external_ports: bool,
}

/// Печатает все секции объявлений одного `FUNCTION_BLOCK`.
///
/// Возвращает `true`, если напечатана хотя бы одна секция. Это не косметика:
/// `iec2c` отвергает `FUNCTION_BLOCK` без объявлений
/// («no variable declarations and no body»), поэтому вызывающий обязан знать,
/// пуст ли блок.
///
/// # Фильтр неиспользуемых
///
/// Неиспользуемые переменные, порты и константы не объявляются — так же
/// поступает цель `c` (`c_header.rs:344`). Это **не** тихая потеря класса Д1b:
/// о неиспользуемом имени уже сообщает семантика (Ce13,
/// [`crate::unused_variable_warnings`]) — то есть диагностика есть, просто не
/// здесь. Потеря без диагностики была бы у **используемой** переменной; такой
/// исход исключён сигнатурой [`get_st_type`] (`Result`, а не `Option`).
///
/// # Ошибки
/// Диагностика от [`get_st_type`] на первом же неотображаемом типе; частичный
/// вывод не порождается.
pub(crate) fn emit_declarations(
    p: &mut Printer,
    model: &ModelNode,
    usage: &UsageSet,
    extras: &Extras,
) -> Result<bool, Diagnostic> {
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut in_outs = Vec::new();
    let mut externals = Vec::new();
    let mut locals = Vec::new();
    let mut constants = enum_constants(model)?;

    // Признак завершения — выход FB: по нему родитель узнаёт об окончании (S11).
    if extras.is_done {
        outputs.push(Declaration {
            name: "is_done".to_string(),
            ty: "BOOL".to_string(),
            init: None,
        });
    }
    // Переменная автомата. Ноль — это `INIT`: холодный старт ПЛК обнуляет `VAR`,
    // поэтому отдельная инициализация не нужна (S3).
    if extras.state_var {
        locals.push(Declaration {
            name: "state".to_string(),
            ty: "USINT".to_string(),
            init: Some("0".to_string()),
        });
    }
    for (name, ty) in &extras.shared {
        in_outs.push(Declaration {
            name: name.clone(),
            ty: get_st_type(ty, model)?,
            init: None,
        });
    }
    for (name, fb_type) in &extras.instances {
        locals.push(Declaration {
            name: name.clone(),
            ty: fb_type.clone(),
            init: None,
        });
    }
    for (name, ty) in &extras.hoisted {
        if locals.iter().any(|d| &d.name == name) {
            continue;
        }
        locals.push(Declaration {
            name: name.clone(),
            ty: get_st_type(ty, model)?,
            init: None,
        });
    }

    let mut names: Vec<&String> = model.variables.keys().collect();
    names.sort();
    for key in names {
        match &model.variables[key] {
            VariableNode::Unresolved => {}
            VariableNode::Simple {
                name,
                ty,
                expr,
                loc,
                ..
            } => {
                if !usage.variables.contains(name) {
                    continue;
                }
                // Проверка стоит ПОСЛЕ фильтра использования: неиспользуемую
                // переменную генератор не эмитит, `iec2c` её не увидит — значит и
                // ST-014 на неё срабатывать не должна (иначе `var action: Action`
                // из elevator.lam, объявленный, но не используемый, сломал бы
                // сборку). Столкновение проверяется на самом эмитируемом имени.
                check_st_name(name, *loc)?;
                // Разделяемая переменная уже объявлена в `VAR_IN_OUT`: повторное
                // объявление в `VAR` сделало бы у под-FB ДВЕ разных переменных с
                // одним именем — то есть тихо разорвало бы связь с корнем.
                if extras.shared.iter().any(|(n, _)| n == name) {
                    continue;
                }
                locals.push(declaration(name, ty, expr, model)?);
            }
            VariableNode::Port {
                name,
                ty,
                direction,
                loc,
                ..
            } => {
                if !usage.ports.contains(name) {
                    continue;
                }
                check_st_name(name, *loc)?;
                if extras.shared.iter().any(|(n, _)| n == name) {
                    continue;
                }
                let decl = declaration(name, ty, &ExpressionNode::None, model)?;
                // Цель `st-at`: порт — размещённая глобальная переменная
                // (`VAR_GLOBAL … AT %…` внутри `CONFIGURATION`), и блок видит её
                // через `VAR_EXTERNAL`. Цель `st` адрес не потребляет: порт
                // остаётся входом/выходом блока.
                if extras.external_ports {
                    externals.push(decl);
                    continue;
                }
                match direction {
                    PortDirection::In => inputs.push(decl),
                    PortDirection::Out => outputs.push(decl),
                    PortDirection::InOut => in_outs.push(decl),
                }
            }
            VariableNode::Const {
                name,
                ty,
                expr,
                loc,
                ..
            } => {
                if !usage.constants.contains(name) {
                    continue;
                }
                check_st_name(name, *loc)?;
                constants.push(declaration(name, ty, expr, model)?);
            }
        }
    }

    // Константы предков: FB в IEC замкнут и области видимости Lam не наследует.
    for (name, var) in inherited_constants(model, usage) {
        let VariableNode::Const { ty, expr, .. } = &var else {
            continue;
        };
        constants.push(declaration(&name, ty, expr, model)?);
    }

    let sections = [
        ("VAR_INPUT", inputs),
        ("VAR_OUTPUT", outputs),
        ("VAR_IN_OUT", in_outs),
        ("VAR_EXTERNAL", externals),
        ("VAR", locals),
        ("VAR CONSTANT", constants),
    ];
    let mut printed = false;
    for (keyword, decls) in sections {
        if decls.is_empty() {
            continue;
        }
        printed = true;
        p.ident(keyword).nl();
        p.up();
        for decl in &decls {
            decl.write(p);
        }
        p.down();
        p.ident("END_VAR").nl();
    }
    Ok(printed)
}

/// Строит объявления констант-вариантов перечислений модели (откат Option C).
///
/// Имя константы — `<Перечисление>_<Вариант>`: пространство имён констант в
/// IEC 61131-3 плоское, а одноимённые варианты разных перечислений в Lam
/// допустимы.
fn enum_constants(model: &ModelNode) -> Result<Vec<Declaration>, Diagnostic> {
    let mut out = Vec::new();
    // Перечисления собираются с модели И ЕЁ ПРЕДКОВ. Причина: в Lam область
    // видимости вложенная (под-модель видит `enum Command` корня), а в
    // IEC 61131-3 `FUNCTION_BLOCK` — замкнутая единица: он видит только то, что
    // объявлено в нём самом. Гейт поймал это на `elevator_mini`: под-модель
    // `Motor` пишет `command = Command_Stop`, а константа жила лишь в корне →
    // «Variable not declared in this scope».
    let enums = visible_enums(model);
    let mut names: Vec<&String> = enums.keys().collect();
    names.sort();
    for enum_name in names {
        let node = &enums[enum_name];
        // Разрядность типа выбрана по фактическому диапазону вариантов
        // (`st_type::enum_type`), поэтому усечения значения здесь быть не может.
        let ty = get_st_type(&TypeNode::Enum(enum_name.clone()), model)?;
        for (variant, value) in &node.variants {
            out.push(Declaration {
                name: format!("{}_{}", enum_name, variant),
                ty: ty.clone(),
                init: Some(value.to_string()),
            });
        }
    }
    Ok(out)
}

/// Собирает перечисления, видимые модели: её собственные плюс предков.
fn visible_enums(
    model: &ModelNode,
) -> std::collections::HashMap<String, crate::semantic::EnumDefinitionNode> {
    let mut out = std::collections::HashMap::new();
    // Свои — в первую очередь: ближняя область видимости перекрывает дальнюю.
    for (k, v) in &model.enums {
        out.insert(k.clone(), v.clone());
    }
    let mut current = model.upper.as_ref().and_then(|w| w.upgrade());
    while let Some(parent_rc) = current {
        let parent = parent_rc.borrow();
        for (k, v) in &parent.enums {
            out.entry(k.clone()).or_insert_with(|| v.clone());
        }
        current = parent.upper.as_ref().and_then(|w| w.upgrade());
    }
    out
}

/// Собирает константы, видимые модели, но объявленные у предков.
///
/// В Lam под-модель видит `const CHARGE_STACK` корня; в IEC — нет. Константа
/// неизменна, поэтому дешевле продублировать её в `VAR CONSTANT` каждого FB,
/// чем плести через `VAR_IN_OUT`.
fn inherited_constants(model: &ModelNode, usage: &UsageSet) -> Vec<(String, VariableNode)> {
    let mut out: Vec<(String, VariableNode)> = Vec::new();
    let mut current = model.upper.as_ref().and_then(|w| w.upgrade());
    while let Some(parent_rc) = current {
        let parent = parent_rc.borrow();
        let mut names: Vec<&String> = parent.variables.keys().collect();
        names.sort();
        for name in names {
            let var = &parent.variables[name];
            if !matches!(var, VariableNode::Const { .. }) {
                continue;
            }
            if !usage.constants.contains(name) {
                continue;
            }
            if model.variables.contains_key(name) || out.iter().any(|(n, _)| n == name) {
                continue;
            }
            out.push((name.clone(), var.clone()));
        }
        current = parent.upper.as_ref().and_then(|w| w.upgrade());
    }
    out
}

/// Строит одно объявление: имя, тип IEC и — если он литерал — инициализатор.
fn declaration(
    name: &str,
    ty: &TypeNode,
    expr: &ExpressionNode,
    model: &ModelNode,
) -> Result<Declaration, Diagnostic> {
    Ok(Declaration {
        name: name.to_string(),
        ty: get_st_type(ty, model)?,
        init: literal_init(expr, ty),
    })
}

/// Возвращает инициализатор, если выражение — литерал, а тип — скалярный.
///
/// Переводятся только литералы: трансляция произвольных выражений — задача
/// 0041-04. Пропуск нелитерального инициализатора **безопасен**: переменная
/// объявляется без него и получает нулевое значение по умолчанию IEC, а не
/// исчезает. Полную форму (включая вычислимые инициализаторы) даёт 0041-04.
///
/// **Составные типы инициализатор не получают.** Lam разрешает скалярный `0` для
/// массива (`var data: [u8; 4] := 0;` — так объявлены переменные корпуса), но в
/// IEC это ошибка: `iec2c` на `ARRAY [0..3] OF USINT := 0` отвечает «invalid
/// initial value in array specification with initialization». Агрегатная форма
/// (`:= [0, 0, 0, 0]`) — задача 0041-04 вместе с остальными выражениями; до неё
/// массив объявляется без инициализатора и обнуляется правилами IEC по
/// умолчанию, что совпадает с намерением `:= 0`.
pub(crate) fn literal_init(expr: &ExpressionNode, ty: &TypeNode) -> Option<String> {
    if matches!(ty, TypeNode::Array(_, _) | TypeNode::Struct(_)) {
        return None;
    }
    match expr {
        // `bit`/`bool` в IEC — `BOOL`: числовой литерал 0/1 ему не присвоить,
        // нужны `FALSE`/`TRUE`.
        ExpressionNode::Number(n) if matches!(ty, TypeNode::Bit | TypeNode::Bool) => {
            Some(if *n == 0 { "FALSE" } else { "TRUE" }.to_string())
        }
        // Булев литерал (`const ENABLED: bool := true;`). Без этой ветви
        // константа теряла инициализатор и `iec2c` отвергал объявление:
        // `VAR CONSTANT` без значения — «invalid specification in variable
        // declaration».
        ExpressionNode::Bool(b) => Some(if *b { "TRUE" } else { "FALSE" }.to_string()),
        ExpressionNode::Number(n) => Some(n.to_string()),
        ExpressionNode::Rational(text, negative) => {
            Some(format!("{}{}", if *negative { "-" } else { "" }, text))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::tree::construct_model;

    /// Печатает секции объявлений корневой модели исходника.
    fn declarations_of(src: &str) -> String {
        let (ast, _) = crate::parse(src, 0).unwrap();
        let rc = construct_model(&ast, None, &[]).unwrap();
        let usage = crate::semantic::unused::compute_usage(std::rc::Rc::clone(&rc));
        let model = rc.borrow();
        let mut out = String::new();
        let mut p = Printer::new(4, &mut out);
        emit_declarations(&mut p, &model, &usage, &Extras::default())
            .expect("объявления должны печататься");
        out
    }

    /// Используемая переменная-массив объявляется настоящим `ARRAY`.
    ///
    /// **Прямой контрпример дефекту Д1b фичи 0029**: на этом же входе цель `c`
    /// даёт `uint4_t` (несуществующий тип) — размерность теряется целиком.
    #[test]
    fn test_emit_declarations_array_variable_is_declared_not_lost() {
        let src = "var data: [u8; 4] := 0;\ncond C = data[0] = 1;\nstart S { ref Done: C; }\nstate Done {}";
        let st = declarations_of(src);
        assert!(
            st.contains("data : ARRAY [0..3] OF USINT"),
            "переменная-массив обязана быть объявлена:\n{st}"
        );
    }

    /// Скалярный инициализатор массива не переносится в ST.
    ///
    /// В Lam `var data: [u8; 4] := 0;` — обычная форма (так объявлен весь
    /// корпус), но `iec2c` отвергает `ARRAY [0..3] OF USINT := 0` («invalid
    /// initial value in array specification with initialization»). Сторож против
    /// возврата: без него вывод невалиден, а тест на присутствие `data` этого не
    /// ловит.
    #[test]
    fn test_emit_declarations_array_gets_no_scalar_initializer() {
        let src = "var data: [u8; 4] := 0;\ncond C = data[0] = 1;\nstart S { ref Done: C; }\nstate Done {}";
        let st = declarations_of(src);
        assert!(
            st.contains("data : ARRAY [0..3] OF USINT;"),
            "у массива не должно быть скалярного инициализатора:\n{st}"
        );
    }

    /// Входные и выходные порты попадают в разные секции.
    #[test]
    fn test_emit_declarations_ports_split_by_direction() {
        let src = "in btn: bit := 0x100:0;\nout lamp: bit := 0x200:0;\nstart S { always { lamp := btn; } }";
        let st = declarations_of(src);
        let inputs = st.find("VAR_INPUT").expect("нет VAR_INPUT");
        let outputs = st.find("VAR_OUTPUT").expect("нет VAR_OUTPUT");
        assert!(
            st[inputs..outputs].contains("btn : BOOL;"),
            "btn не входной:\n{st}"
        );
        assert!(
            st[outputs..].contains("lamp : BOOL;"),
            "lamp не выходной:\n{st}"
        );
    }

    /// Каждая открытая секция закрыта `END_VAR`.
    #[test]
    fn test_emit_declarations_every_section_is_closed() {
        let src = "in btn: bit := 0x100:0;\nout lamp: bit := 0x200:0;\nvar n: u8 := 0;\nstart S { always { lamp := btn; n := n + 1; } }";
        let st = declarations_of(src);
        assert_eq!(
            st.matches("END_VAR").count(),
            3,
            "ожидались VAR_INPUT, VAR_OUTPUT и VAR:\n{st}"
        );
    }

    /// Варианты перечисления становятся именованными константами.
    ///
    /// Значения — из зонда по `examples/elevator.lam:117`: `Floor { Bottom = 80,
    /// Top }` даёт `[("Bottom", 80), ("Top", 81)]`.
    #[test]
    fn test_emit_declarations_enum_variants_become_named_constants() {
        let src =
            "enum Floor { Bottom = 80, Top }\nvar f: u8 := 0;\nstart S { always { f := f + 1; } }";
        let st = declarations_of(src);
        assert!(st.contains("VAR CONSTANT"), "нет секции констант:\n{st}");
        assert!(
            st.contains("Floor_Bottom : USINT := 80;"),
            "нет константы Bottom:\n{st}"
        );
        assert!(
            st.contains("Floor_Top : USINT := 81;"),
            "Top обязан наследовать 81:\n{st}"
        );
    }

    /// Перечисление ПРЕДКА объявляется в под-модели: FB в IEC замкнут.
    ///
    /// Сторож против регресса, который поймал гейт, а юнит-тесты — нет:
    /// `elevator_mini` пишет в под-модели `command = Command_Stop`, а
    /// `enum Command` объявлен в корне. В Lam область видимости вложенная, в
    /// IEC 61131-3 — нет: `FUNCTION_BLOCK` видит только объявленное в нём самом.
    #[test]
    fn test_enum_of_ancestor_is_declared_in_submodel_block() {
        let src = "enum Command { Up, Stop }\n\
                   model Motor {\n\
                     var c: u8 := 0;\n\
                     start S { always { c := Stop; } }\n\
                   }\n\
                   start Main = Motor;";
        let (ast, _) = crate::parse(src, 0).unwrap();
        let rc = construct_model(&ast, None, &[]).unwrap();
        let usage = crate::semantic::unused::compute_usage(std::rc::Rc::clone(&rc));
        let sub = rc.borrow().models.get("Motor").cloned();
        let Some(sub) = sub else {
            panic!("под-модель Motor не найдена");
        };
        let model = sub.borrow();
        let mut out = String::new();
        let mut p = Printer::new(4, &mut out);
        emit_declarations(&mut p, &model, &usage, &Extras::default()).unwrap();
        assert!(
            out.contains("Command_Stop"),
            "перечисление корня обязано объявляться в под-модели:\n{out}"
        );
    }

    /// Перечисление шире байта не усекается — тип константы расширяется.
    ///
    /// Вход из `examples/elevator.lam:121`: `Action { Idle = 670, Closing }`.
    #[test]
    fn test_emit_declarations_wide_enum_constant_is_not_truncated() {
        let src = "enum Action { Idle = 670, Closing }\nvar a: u8 := 0;\nstart S { always { a := a + 1; } }";
        let st = declarations_of(src);
        assert!(
            st.contains("Action_Idle : UINT := 670;"),
            "670 не помещается в USINT — константа обязана быть шире:\n{st}"
        );
    }

    /// Литеральный инициализатор `bit`-переменной — `FALSE`/`TRUE`, не 0/1:
    /// числовой литерал в IEC несовместим с `BOOL`.
    #[test]
    fn test_emit_declarations_bool_initializer_is_keyword_not_number() {
        let src = "var flag: bit := 1;\nstart S { always { flag := flag; } }";
        let st = declarations_of(src);
        assert!(
            st.contains("flag : BOOL := TRUE;"),
            "инициализатор BOOL обязан быть TRUE/FALSE:\n{st}"
        );
    }

    /// Неиспользуемая переменная не объявляется — как в цели `c`.
    ///
    /// Это не дефект Д1b: о неиспользуемом имени сообщает семантика (Ce13),
    /// диагностика есть. Тест закрепляет намеренность поведения.
    #[test]
    fn test_emit_declarations_unused_variable_is_filtered_like_c_target() {
        let src =
            "var used: u8 := 0;\nvar unused: u8 := 0;\nstart S { always { used := used + 1; } }";
        let st = declarations_of(src);
        assert!(
            st.contains("used : USINT"),
            "используемая обязана быть:\n{st}"
        );
        assert!(
            !st.contains("unused :"),
            "неиспользуемая фильтруется (паритет с целью c):\n{st}"
        );
    }

    /// Модель без объявлений сообщает об этом вызывающему.
    ///
    /// `iec2c` отвергает `FUNCTION_BLOCK` без объявлений и тела, поэтому пустота
    /// обязана быть видна снаружи, а не «пустой строкой».
    #[test]
    fn test_emit_declarations_reports_empty_model() {
        let (ast, _) = crate::parse("start S;", 0).unwrap();
        let rc = construct_model(&ast, None, &[]).unwrap();
        let usage = crate::semantic::unused::compute_usage(std::rc::Rc::clone(&rc));
        let model = rc.borrow();
        let mut out = String::new();
        let mut p = Printer::new(4, &mut out);
        let printed = emit_declarations(&mut p, &model, &usage, &Extras::default()).unwrap();
        assert!(!printed, "модель без переменных не имеет секций");
        assert!(
            out.is_empty(),
            "пустая модель не должна печатать секции:\n{out}"
        );
    }

    /// Неотображаемый тип **используемой** переменной обязан завалить генерацию,
    /// а не убрать переменную из вывода (R4.3, контрпример дефекту Д1b).
    ///
    /// Тип портится после разбора: исходника, дающего `Unsupported` у
    /// используемой переменной, в языке нет — узел служебный.
    #[test]
    fn test_emit_declarations_unmappable_type_is_error_not_silent_skip() {
        let (ast, _) = crate::parse(
            "var bad: u8 := 0;\nstart S { always { bad := bad + 1; } }",
            0,
        )
        .unwrap();
        let rc = construct_model(&ast, None, &[]).unwrap();
        let usage = crate::semantic::unused::compute_usage(std::rc::Rc::clone(&rc));
        assert!(
            usage.variables.contains("bad"),
            "переменная обязана считаться используемой — иначе тест проверял бы фильтр"
        );
        if let Some(VariableNode::Simple { ty, .. }) = rc.borrow_mut().variables.get_mut("bad") {
            *ty = TypeNode::Unsupported;
        }
        let model = rc.borrow();
        let mut out = String::new();
        let mut p = Printer::new(4, &mut out);
        let err = emit_declarations(&mut p, &model, &usage, &Extras::default())
            .expect_err("ожидалась диагностика");
        assert_eq!(err.code.as_deref(), Some("ST-002"));
    }
}
