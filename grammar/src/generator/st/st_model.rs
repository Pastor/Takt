//! Отображение автомата модели в `CASE state OF` внутри `FUNCTION_BLOCK`.
//!
//! Задача 0041-03, **часть 1: простые модели** (композиция `M1 | M2` / `M1 + M2`
//! — часть 2). Подключает печатники задачи 0041-04: `st_expr` (условия),
//! `st_stmt` (тела блоков), `st_func` (функции).
//!
//! ## Изоморфизм с целью `c` — по зонду, а не по памяти
//!
//! Форма снята с **реального** вывода `lamc compile -t c` (2026-07-15) и
//! воспроизводится один в один:
//!
//! | Факт | Зонд C | ST |
//! |---|---|---|
//! | Ф3: `INIT` исполняет `enter` стартового и переходит в него | `case …_INIT: { enter; state = …_START; }` | ветвь `0:` |
//! | Ф4: `enter` **целевого** инлайнится в переход | `if (cond) { enter_target; state = …_T; }` | тело `IF` |
//! | `exit` **источника** — перед `enter` цели | `n := 2; n := 3;` (зонд `exit_probe`) | то же |
//! | Ф5: порядок `ref` = порядок проверки, первый выигрывает | цепочка `if … break;` | `IF … ELSIF …` |
//! | Ф8: состояние без исходящих `ref` исполняет `exit` и уходит в `END` | `case B: { n = 4; state = END; }` | то же |
//!
//! `always` исполняется **первым** в ветви — до проверок переходов (S8).
//!
//! ## Почему `state : USINT := 0`, а `INIT` — ноль
//!
//! Холодный старт ПЛК обнуляет `VAR`, поэтому отдельный `_init()` цели `c` не
//! нужен: нулевое состояние **само** является `INIT` (S3).
//!
//! ## Факты MatIEC, определившие форму
//!
//! - **Пустая ветвь `CASE` недопустима**: `1: ;` → `invalid statement in case
//!   element of ST 'CASE' statement`. Поэтому терминальная ветвь `END` печатает
//!   само-присваивание `state := <END>;` — семантически пустое, синтаксически
//!   обязательное.
//! - Перечислимых типов состояний нет (проба П4), поэтому номера состояний —
//!   числовые литералы, а не имена; читаемость держится на комментариях.

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::indent::Printer;
use crate::generator::st::st_expr::print_condition;
use crate::generator::st::st_map::StMap;
use crate::generator::st::st_stmt::{StmtOutput, print_statement};
use crate::semantic::minimap::{Element, Name, StateExtend};
use crate::semantic::type_node::TypeNode;
use crate::semantic::{ConditionNode, ModelNode, NamedCodeBlockDefinitionNode, StateNode};
use std::collections::HashMap;

/// Номер синтетического состояния `INIT`.
///
/// Ноль не случаен: холодный старт ПЛК обнуляет `VAR`, поэтому автомат сам
/// оказывается в `INIT` без отдельного вызова инициализации (S3).
const INIT_STATE: usize = 0;

/// Экземпляр под-`FUNCTION_BLOCK` внутри родительского FB.
///
/// Аналог поля `StackerCommandReceiver command_receiver0;` в структуре цели `c`
/// (Ф6). Числовой суффикс обязателен: одна и та же модель может входить в
/// композицию **несколько раз** (`elevator.lam:198` включает `Engine` пять раз).
#[derive(Debug)]
pub(crate) struct Instance {
    /// Имя переменной-экземпляра.
    pub name: String,
    /// Имя типа — `FUNCTION_BLOCK` под-модели.
    pub fb_type: String,
}

/// Результат печати тела: побочные эффекты операторов плюс экземпляры под-FB.
#[derive(Default, Debug)]
pub(crate) struct BodyOutput {
    /// Поднятые объявления и предупреждения печатника операторов.
    pub stmt: StmtOutput,
    /// Экземпляры под-FB, которые вызывающий обязан объявить в `VAR`.
    pub instances: Vec<Instance>,
}

/// Таблица номеров состояний модели: `INIT` = 0, состояния, `END` последним.
pub(crate) struct StateTable {
    /// Номер по уникальному имени состояния.
    numbers: HashMap<String, usize>,
    /// Состояния в порядке печати (номер = индекс + 1).
    ordered: Vec<Name>,
    /// Номер синтетического `END`.
    end: usize,
}

impl StateTable {
    /// Строит таблицу номеров.
    ///
    /// Порядок — лексикографический по уникальному имени. Это не косметика:
    /// `states()` отдаёт имена в порядке обхода `HashMap`, то есть **разном от
    /// запуска к запуску**, а номер состояния — часть ABI порождённого ПЛК-кода.
    /// Та же причина, по которой отсортированы подмодели (0041-01).
    pub fn build(states: &[Name]) -> Self {
        let mut ordered: Vec<Name> = states.to_vec();
        ordered.sort_by(|a, b| a.unique().cmp(b.unique()));
        let mut numbers = HashMap::new();
        for (i, name) in ordered.iter().enumerate() {
            numbers.insert(name.unique().to_string(), i + 1);
        }
        let end = ordered.len() + 1;
        Self {
            numbers,
            ordered,
            end,
        }
    }

    /// Номер состояния по имени.
    fn number_of(&self, name: &str) -> Option<usize> {
        self.numbers.get(name).copied()
    }
}

/// Печатает тело `FUNCTION_BLOCK`: `CASE state OF` и признак завершения.
///
/// Возвращает побочные результаты печати тел (поднятые объявления,
/// предупреждения `ST-010`) — вызывающий обязан объявить поднятое в шапке POU.
///
/// # Ошибки
/// - `ST-011` — состояние с композицией (`= M1 | M2`): часть 2 задачи 0041-03.
/// - `ST-013` — переход ведёт в неизвестное состояние.
/// - Диагностики печатников выражений и операторов.
pub(crate) fn emit_body(
    p: &mut Printer,
    map: &StMap,
    element: &Element,
    model: &ModelNode,
    table: &StateTable,
) -> Result<BodyOutput, Diagnostic> {
    let Element::Model { start, .. } = element else {
        return Err(Diagnostic::error(
            Location::Codegen,
            "Тело автомата строится только для модели".to_string(),
        )
        .with_code("ST-012"));
    };
    let mut out = BodyOutput::default();

    p.ident("CASE state OF").nl();
    p.up();

    // Ветвь INIT: исполняет `enter` стартового состояния и переходит в него —
    // ровно как `case …_INIT` цели `c` (Ф3).
    let start_no = table.number_of(start.unique()).ok_or_else(|| {
        unknown_state(&format!(
            "стартовое состояние '{}' отсутствует в таблице номеров",
            start
        ))
    })?;
    p.ident(&format!("{}: (* INIT *)", INIT_STATE)).nl();
    p.up();
    let start_state = raw_state(model, start)?;
    emit_block(p, &start_state, "enter", model, &mut out.stmt)?;
    p.ident(&format!("state := {}; (* {} *)", start_no, start.local()))
        .nl();
    p.down();

    for name in &table.ordered {
        let number = table
            .number_of(name.unique())
            .ok_or_else(|| unknown_state(name.unique()))?;
        let state = raw_state(model, name)?;
        p.ident(&format!("{}: (* {} *)", number, name.local())).nl();
        p.up();
        emit_state(p, map, name, &state, model, table, &mut out)?;
        p.down();
    }

    // Терминальная ветвь. Само-присваивание — не описка: пустая ветвь `CASE`
    // синтаксически недопустима («invalid statement in case element»), а
    // семантически здесь ничего происходить не должно.
    p.ident(&format!("{}: (* END *)", table.end)).nl();
    p.up();
    p.ident(&format!("state := {};", table.end)).nl();
    p.down();

    p.down();
    p.ident("END_CASE;").nl();
    // Признак завершения — выход FB (S11); по нему родитель узнаёт об окончании.
    p.ident(&format!("is_done := state = {};", table.end)).nl();
    Ok(out)
}

/// Печатает содержимое ветви одного состояния.
fn emit_state(
    p: &mut Printer,
    map: &StMap,
    name: &Name,
    state: &StateNode,
    model: &ModelNode,
    table: &StateTable,
    out: &mut BodyOutput,
) -> Result<(), Diagnostic> {
    // `always` — первым в ветви, до проверок переходов (S8, Ф5).
    emit_block(p, state, "always", model, &mut out.stmt)?;

    // Состояние с реализацией (`= Модель`, `= M1 | M2`) — композиция.
    if let Some(Element::StateExtend { extend, next, .. }) = map.element_of(name)
        && !matches!(extend, StateExtend::None)
    {
        return emit_composition(p, map, name, &extend, &next, table, out);
    }

    let references = match state {
        StateNode::Simple { references, .. } | StateNode::Implement { references, .. } => {
            references.clone()
        }
        StateNode::Unresolved => Vec::new(),
    };

    if references.is_empty() {
        // Состояние без исходящих переходов терминально: исполняет `exit` и
        // уходит в `END` — как `case B` в зонде `exit_probe` (Ф8).
        emit_block(p, state, "exit", model, &mut out.stmt)?;
        p.ident(&format!("state := {}; (* END *)", table.end)).nl();
        return Ok(());
    }

    // Порядок `ref` = порядок проверки, первый сработавший выигрывает (Ф5):
    // цепочка `if … break;` цели `c` — это `IF … ELSIF …` в ST.
    let mut printed_if = false;
    for reference in &references {
        let target = table.number_of_local(&reference.name).ok_or_else(|| {
            unknown_state(&format!(
                "переход ведёт в состояние '{}', которого нет в модели",
                reference.name
            ))
        })?;
        // Безусловный переход (`ref T;` без условия) приходит как
        // `ConditionNode::None`: проверять нечего — переход печатается как есть,
        // и цепочка на нём заканчивается.
        if matches!(reference.cond, ConditionNode::None) {
            if printed_if {
                p.ident("ELSE").nl();
                p.up();
                emit_transition(p, state, &reference.name, target, model, &mut out.stmt)?;
                p.down();
                p.ident("END_IF;").nl();
            } else {
                emit_transition(p, state, &reference.name, target, model, &mut out.stmt)?;
            }
            return Ok(());
        }
        let guard = print_condition(&reference.cond, model)?;
        p.ident(&format!(
            "{} {} THEN",
            if printed_if { "ELSIF" } else { "IF" },
            guard
        ))
        .nl();
        p.up();
        emit_transition(p, state, &reference.name, target, model, &mut out.stmt)?;
        p.down();
        printed_if = true;
    }
    if printed_if {
        p.ident("END_IF;").nl();
    }
    Ok(())
}

/// Печатает ветвь состояния-композиции: вызовы под-FB и завершение по `is_done`.
///
/// Форма изоморфна цели `c` (Ф6, `stacker.c:414-439`): под-модели вызываются
/// **последовательно в одном такте** родителя, в порядке объявления, а
/// композиция завершается по **конъюнкции** их `is_done`. Настоящей
/// конкурентности нет — чередование детерминировано, что и нужно скан-циклу ПЛК.
fn emit_composition(
    p: &mut Printer,
    map: &StMap,
    state_name: &Name,
    extend: &StateExtend,
    next: &Name,
    table: &StateTable,
    out: &mut BodyOutput,
) -> Result<(), Diagnostic> {
    // Последовательная композиция идёт своим путём: ей нужен счётчик шагов.
    if let StateExtend::Concatenation(steps) = extend {
        return emit_concatenation(p, map, steps, state_name, next, table, out);
    }

    let mut group = Vec::new();
    collect_models(extend, &mut group)?;
    // Переменные корня под-FB видит через `VAR_IN_OUT`: в ST указателей нет, а
    // `main->lift_request` цели `c` выразить нечем (О1-в, проба П7).
    let done_terms = vec![emit_group(p, map, &group, "", out)];

    let target = if next.unique().is_empty() {
        table.end
    } else {
        table.number_of_local(next.local()).unwrap_or(table.end)
    };
    p.ident(&format!("IF {} THEN", done_terms.join(" AND ")))
        .nl();
    p.up();
    p.ident(&format!("state := {}; (* {} *)", target, next.local()))
        .nl();
    p.down();
    p.ident("END_IF;").nl();
    Ok(())
}

/// Печатает шаг: вызовы моделей группы и условие её завершения.
///
/// Возвращает выражение «группа завершена» (конъюнкция `is_done`).
fn emit_group(
    p: &mut Printer,
    map: &StMap,
    group: &[Name],
    prefix: &str,
    out: &mut BodyOutput,
) -> String {
    let mut done_terms = Vec::new();
    for model_name in group {
        // Числовой суффикс — по образцу цели `c` (`start_a0`, `start_b1`): одна и
        // та же модель может входить в композицию несколько раз.
        let index = out.instances.len();
        let inst = format!(
            "{}{}{}",
            prefix,
            model_name.local_lowercase_snakecase(),
            index
        );
        out.instances.push(Instance {
            name: inst.clone(),
            fb_type: model_name.unique_camelcase(),
        });
        let args: Vec<String> = map
            .shared_variables(model_name)
            .into_iter()
            .map(|(n, _)| format!("{} := {}", n, n))
            .collect();
        p.ident(&format!("{}({});", inst, args.join(", "))).nl();
        done_terms.push(format!("{}.is_done", inst));
    }
    done_terms.join(" AND ")
}

/// Печатает последовательную композицию (`M1 + M2`) как вложенный `CASE` по
/// собственному счётчику шагов.
///
/// Форма — из зонда цели `c` (`extend_complex.h`): у конкатенации там **свой**
/// `enum` шагов (`…_START_A0`, `…_START_B1`, `…_START_PARALLEL2`, `…_START_E3`),
/// отдельный от состояния модели. В ST это переменная-счётчик и вложенный `CASE`
/// (форма проверена пробой ✅).
///
/// Шаг завершается по `is_done` своей группы; параллельная группа внутри
/// конкатенации (`A + (C | D) + E`) — по конъюнкции, как обычная параллель.
fn emit_concatenation(
    p: &mut Printer,
    map: &StMap,
    steps: &[StateExtend],
    state_name: &Name,
    next: &Name,
    table: &StateTable,
    out: &mut BodyOutput,
) -> Result<(), Diagnostic> {
    let counter = format!("{}_step", state_name.local_lowercase_snakecase());
    out.stmt
        .hoisted
        .push(crate::generator::st::st_stmt::Hoisted {
            name: counter.clone(),
            ty: TypeNode::Integer {
                bits: 8,
                signed: false,
            },
        });
    let prefix = format!("{}_", state_name.local_lowercase_snakecase());

    p.ident(&format!("CASE {} OF", counter)).nl();
    p.up();
    for (i, step) in steps.iter().enumerate() {
        let mut group = Vec::new();
        collect_models(step, &mut group)?;
        p.ident(&format!("{}:", i)).nl();
        p.up();
        let done = emit_group(p, map, &group, &prefix, out);
        p.ident(&format!("IF {} THEN", done)).nl();
        p.up();
        p.ident(&format!("{} := {};", counter, i + 1)).nl();
        p.down();
        p.ident("END_IF;").nl();
        p.down();
    }
    // Последний шаг: конкатенация пройдена — уходим по `next`.
    let target = if next.unique().is_empty() {
        table.end
    } else {
        table.number_of_local(next.local()).unwrap_or(table.end)
    };
    p.ident(&format!("{}: (* конкатенация завершена *)", steps.len()))
        .nl();
    p.up();
    p.ident(&format!("state := {}; (* {} *)", target, next.local()))
        .nl();
    p.down();
    p.down();
    p.ident("END_CASE;").nl();
    Ok(())
}

/// Собирает модели композиции в порядке объявления.
///
/// # Ошибки
/// `ST-011` — `Concatenation` (`M1 + M2`): последовательная композиция требует
/// собственного счётчика шагов (в цели `c` — вложенный `enum state`) и
/// реализуется частью 3.
fn collect_models(extend: &StateExtend, out: &mut Vec<Name>) -> Result<(), Diagnostic> {
    match extend {
        StateExtend::None => Ok(()),
        StateExtend::Model(name) => {
            out.push(name.clone());
            Ok(())
        }
        StateExtend::Parallel(steps) => {
            for step in steps {
                collect_models(step, out)?;
            }
            Ok(())
        }
        // Конкатенацию печатает `emit_concatenation` — у неё свой счётчик шагов.
        // Сюда она попадает только вложенной в параллель (`(A + B) | C`), а такой
        // вложенности нужен ещё один уровень счётчика: пока — громкий отказ, а не
        // печать шагов как параллельных (это молча изменило бы семантику).
        StateExtend::Concatenation(_) => Err(Diagnostic::error(
            Location::Codegen,
            "Конкатенация внутри параллельной композиции (`(A + B) | C`) требует \
             вложенного счётчика шагов. Напечатать её шаги как параллельные значило \
             бы молча изменить семантику модели"
                .to_string(),
        )
        .with_code("ST-011")),
    }
}

/// Печатает переход: `exit` источника, `enter` цели, смена состояния.
///
/// Порядок снят зондом (`exit_probe`), а не предположен: `exit` источника
/// исполняется **перед** `enter` цели, и оба — в такте перехода (Ф4).
fn emit_transition(
    p: &mut Printer,
    source: &StateNode,
    target_name: &str,
    target_number: usize,
    model: &ModelNode,
    out: &mut StmtOutput,
) -> Result<(), Diagnostic> {
    emit_block(p, source, "exit", model, out)?;
    if let Some(target) = model.states.get(target_name) {
        emit_block(p, target, "enter", model, out)?;
    }
    p.ident(&format!(
        "state := {}; (* {} *)",
        target_number, target_name
    ))
    .nl();
    Ok(())
}

/// Печатает тело именованного блока (`enter`/`exit`/`always`) состояния.
fn emit_block(
    p: &mut Printer,
    state: &StateNode,
    kind: &str,
    model: &ModelNode,
    out: &mut StmtOutput,
) -> Result<(), Diagnostic> {
    let blocks = match state {
        StateNode::Simple { named_blocks, .. } | StateNode::Implement { named_blocks, .. } => {
            named_blocks
        }
        StateNode::Unresolved => return Ok(()),
    };
    for block in blocks {
        let body = match (kind, block) {
            ("enter", NamedCodeBlockDefinitionNode::Enter { body, .. })
            | ("exit", NamedCodeBlockDefinitionNode::Exit { body, .. })
            | ("always", NamedCodeBlockDefinitionNode::Always { body, .. }) => body,
            _ => continue,
        };
        print_statement(body, model, p, out, None)?;
    }
    Ok(())
}

/// Возвращает семантический узел состояния по имени карты.
fn raw_state(model: &ModelNode, name: &Name) -> Result<StateNode, Diagnostic> {
    model
        .states
        .get(name.local())
        .cloned()
        .ok_or_else(|| unknown_state(name.local()))
}

impl StateTable {
    /// Номер состояния по **локальному** имени (как его пишет `ref`).
    fn number_of_local(&self, local: &str) -> Option<usize> {
        self.ordered
            .iter()
            .find(|n| n.local() == local)
            .and_then(|n| self.number_of(n.unique()))
    }
}

/// Строит диагностику `ST-013` — переход в неизвестное состояние.
fn unknown_state(what: &str) -> Diagnostic {
    Diagnostic::error(Location::Codegen, format!("Автомат ST: {}", what)).with_code("ST-013")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::tree::construct_model;

    /// Печатает тело автомата корневой модели исходника.
    fn body_of(src: &str) -> String {
        let (ast, _) = crate::parse(src, 0).unwrap();
        let rc = construct_model(&ast, None, &[]).unwrap();
        rc.borrow_mut().name = Some("Root".to_string());
        let map = StMap::new("root", &rc.borrow(), false, Default::default()).unwrap();
        let element = map.model();
        let Element::Model { states, .. } = &element else {
            panic!("корень не модель");
        };
        let table = StateTable::build(states);
        let model = rc.borrow();
        let mut text = String::new();
        let mut p = Printer::new(4, &mut text);
        emit_body(&mut p, &map, &element, &model, &table).expect("тело должно печататься");
        text
    }

    /// Автомат превращается в `CASE state OF … END_CASE;`.
    #[test]
    fn test_emits_case_state_of() {
        let st = body_of("var n: u8 := 0;\nstart A { always { n := n + 1; } }");
        assert!(st.contains("CASE state OF"), "нет CASE:\n{st}");
        assert!(st.contains("END_CASE;"), "CASE обязан закрываться:\n{st}");
    }

    /// Ветвь `INIT` — нулевая, исполняет `enter` стартового и переходит в него.
    ///
    /// Сверка с зондом C (Ф3): `case …_INIT: { enter; state = …_START; }`.
    /// Ноль не случаен: холодный старт ПЛК обнуляет `VAR` (S3).
    #[test]
    fn test_init_branch_is_zero_runs_enter_then_transitions() {
        let st = body_of("var n: u8 := 0;\nstart A { enter { n := 1; } }");
        let init = st.find("0: (* INIT *)").expect("нет ветви INIT");
        let enter = st.find("n := 1;").expect("нет enter стартового");
        let go = st.find("state := 1;").expect("нет перехода в стартовое");
        assert!(
            init < enter && enter < go,
            "порядок INIT→enter→переход:\n{st}"
        );
    }

    /// `always` исполняется первым в ветви — до проверок переходов (S8).
    #[test]
    fn test_always_runs_before_transition_checks() {
        let st = body_of(
            "var n: u8 := 0;\ncond Go = n = 1;\n\
             start A { always { n := n + 1; } ref B: Go; }\nstate B {}",
        );
        let always = st.find("n := n + 1;").expect("нет always");
        let check = st.find("IF n = 1 THEN").expect("нет проверки перехода");
        assert!(always < check, "always обязан идти до проверок:\n{st}");
    }

    /// `exit` источника исполняется **перед** `enter` цели — по зонду, не по догадке.
    ///
    /// Зонд C (`exit_probe`): `if (model->n == 1) { model->n = 2; model->n = 3; … }`
    /// — где `n := 2` это `exit` источника, а `n := 3` — `enter` цели.
    #[test]
    fn test_exit_of_source_runs_before_enter_of_target() {
        let st = body_of(
            "var n: u8 := 0;\ncond Go = n = 1;\n\
             start A { exit { n := 2; } ref B: Go; }\n\
             state B { enter { n := 3; } }",
        );
        let exit = st.find("n := 2;").expect("нет exit источника");
        let enter = st.find("n := 3;").expect("нет enter цели");
        assert!(
            exit < enter,
            "exit источника обязан идти до enter цели:\n{st}"
        );
    }

    /// Несколько `ref` → цепочка `IF/ELSIF`: порядок объявления = порядок проверки.
    ///
    /// Сверка с зондом C (Ф5, `stacker.c:82-101`): три независимых `if` с `break`,
    /// первый сработавший выигрывает — это и есть `ELSIF`-цепочка.
    #[test]
    fn test_multiple_refs_become_if_elsif_chain_in_source_order() {
        let st = body_of(
            "var n: u8 := 0;\ncond G1 = n = 1;\ncond G2 = n = 2;\n\
             start A { ref B: G1; ref C: G2; }\nstate B {}\nstate C {}",
        );
        let first = st.find("IF n = 1 THEN").expect("нет первого перехода");
        let second = st.find("ELSIF n = 2 THEN").expect("нет второго перехода");
        assert!(first < second, "порядок ref обязан сохраняться:\n{st}");
        assert!(st.contains("END_IF;"), "цепочка обязана закрываться:\n{st}");
    }

    /// Состояние без исходящих `ref` терминально: `exit` и уход в `END` (Ф8).
    #[test]
    fn test_state_without_refs_runs_exit_and_goes_to_end() {
        let st = body_of(
            "var n: u8 := 0;\ncond Go = n = 1;\n\
             start A { ref B: Go; }\nstate B { exit { n := 4; } }",
        );
        let b_branch = st.find("(* B *)").expect("нет ветви B");
        let tail = &st[b_branch..];
        assert!(
            tail.contains("n := 4;"),
            "exit терминального не исполнен:\n{st}"
        );
        assert!(tail.contains("(* END *)"), "нет ухода в END:\n{st}");
    }

    /// Терминальная ветвь `END` печатает само-присваивание, а не пустоту.
    ///
    /// Не косметика: пустая ветвь `CASE` синтаксически недопустима — `iec2c`
    /// отвечает «invalid statement in case element of ST 'CASE' statement».
    #[test]
    fn test_end_branch_is_not_empty_because_iec_forbids_it() {
        let st = body_of("var n: u8 := 0;\nstart A { always { n := n + 1; } }");
        // Ищем именно МЕТКУ ветви (`2: (* END *)`), а не комментарий `(* END *)`
        // у перехода: первое вхождение подстроки — как раз переход.
        let label = st
            .lines()
            .position(|l| l.trim_start().starts_with("2: (* END *)"))
            .expect("нет метки ветви END");
        let body = st.lines().nth(label + 1).unwrap_or("");
        assert!(
            body.trim_start().starts_with("state :="),
            "ветвь END обязана содержать оператор, иначе iec2c её отвергнет:\n{st}"
        );
    }

    /// Признак завершения — выход FB: по нему родитель узнаёт об окончании (S11).
    #[test]
    fn test_is_done_reflects_end_state() {
        let st = body_of("var n: u8 := 0;\nstart A { always { n := n + 1; } }");
        assert!(
            st.contains("is_done := state = "),
            "нет признака завершения:\n{st}"
        );
    }

    /// Одиночная под-модель (`= Controller`) → экземпляр под-FB и вызов.
    #[test]
    fn test_single_submodel_becomes_instance_call() {
        let st = body_of("model M { start Q { } }\nvar n: u8 := 0;\nstart Entry = M;");
        assert!(st.contains("m0("), "нет вызова экземпляра под-FB:\n{st}");
        assert!(
            st.contains("IF m0.is_done THEN"),
            "завершение композиции — по is_done под-FB:\n{st}"
        );
    }

    /// Параллельная композиция: вызовы последовательны, завершение — конъюнкция.
    ///
    /// Сверка с зондом C (Ф6, `stacker.c:414-439`): под-модели вызываются
    /// последовательно в ОДНОМ такте родителя, в порядке объявления; завершение —
    /// конъюнкция `is_done`. Настоящей конкурентности нет.
    #[test]
    fn test_parallel_composition_calls_sequentially_and_joins_by_conjunction() {
        let st = body_of(
            "model A { start Q { } }\nmodel B { start R { } }\n\
             var n: u8 := 0;\nstart Main = A | B;",
        );
        let a = st.find("a0(").expect("нет вызова A");
        let b = st.find("b1(").expect("нет вызова B");
        assert!(
            a < b,
            "порядок вызовов обязан совпадать с порядком объявления:\n{st}"
        );
        assert!(
            st.contains("IF a0.is_done AND b1.is_done THEN"),
            "завершение — конъюнкция is_done:\n{st}"
        );
    }

    /// Экземпляры нумеруются: одна модель может входить в композицию несколько раз.
    ///
    /// Вход не гипотетический: `elevator.lam:198` включает `Engine` пять раз.
    #[test]
    fn test_repeated_model_gets_distinct_instances() {
        let st = body_of("model A { start Q { } }\nvar n: u8 := 0;\nstart Main = A | A;");
        assert!(st.contains("a0("), "нет первого экземпляра:\n{st}");
        assert!(
            st.contains("a1("),
            "повторная модель обязана получить свой экземпляр:\n{st}"
        );
    }

    /// Последовательная композиция (`M1 + M2`) — вложенный `CASE` по счётчику шагов.
    ///
    /// Форма из зонда цели `c` (`extend_complex.h`): у конкатенации там свой
    /// `enum` шагов, отдельный от состояния модели. Шаг сменяется по `is_done`
    /// своей группы — то есть модели идут ПОСЛЕДОВАТЕЛЬНО, а не параллельно.
    #[test]
    fn test_concatenation_becomes_nested_case_over_step_counter() {
        let st = body_of(
            "model A { start Q { } }\nmodel B { start R { } }\n\
             var n: u8 := 0;\nstart Main = A + B;",
        );
        assert!(
            st.contains("CASE main_step OF"),
            "нет счётчика шагов:\n{st}"
        );
        let a = st.find("main_a0(").expect("нет шага A");
        let b = st.find("main_b1(").expect("нет шага B");
        assert!(a < b, "шаги обязаны идти в порядке объявления:\n{st}");
        assert!(
            st.contains("main_step := 1;"),
            "шаг обязан сменяться по is_done группы:\n{st}"
        );
    }

    /// Нумерация состояний устойчива между запусками.
    ///
    /// Сторож против недетерминизма: `states()` обходит `HashMap`, а номер
    /// состояния — часть ABI порождённого ПЛК-кода.
    #[test]
    fn test_state_numbering_is_deterministic() {
        let src = "var n: u8 := 0;\ncond G = n = 1;\n\
                   start A { ref B: G; }\nstate B { ref C: G; }\nstate C {}";
        let first = body_of(src);
        for i in 1..6 {
            assert_eq!(first, body_of(src), "прогон {i} дал другую нумерацию");
        }
    }
}
