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

// Печатник автомата подключит часть 2 (композиция + шапка POU): пока его никто
// не вызывает. Разрешение снимается вместе с появлением вызывающего — та же
// причина и тот же приём, что в `st_expr.rs`/`st_stmt.rs`/`st_func.rs`.
#![allow(dead_code)]

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::indent::Printer;
use crate::generator::st::st_expr::print_condition;
use crate::generator::st::st_stmt::{StmtOutput, print_statement};
use crate::semantic::minimap::{Element, Name};
use crate::semantic::{ModelNode, NamedCodeBlockDefinitionNode, StateNode};
use std::collections::HashMap;

/// Номер синтетического состояния `INIT`.
///
/// Ноль не случаен: холодный старт ПЛК обнуляет `VAR`, поэтому автомат сам
/// оказывается в `INIT` без отдельного вызова инициализации (S3).
const INIT_STATE: usize = 0;

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
    element: &Element,
    model: &ModelNode,
    table: &StateTable,
) -> Result<StmtOutput, Diagnostic> {
    let Element::Model { start, .. } = element else {
        return Err(Diagnostic::error(
            Location::Codegen,
            "Тело автомата строится только для модели".to_string(),
        )
        .with_code("ST-012"));
    };
    let mut out = StmtOutput::default();

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
    emit_block(p, &start_state, "enter", model, &mut out)?;
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
        emit_state(p, &state, model, table, &mut out)?;
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
    state: &StateNode,
    model: &ModelNode,
    table: &StateTable,
    out: &mut StmtOutput,
) -> Result<(), Diagnostic> {
    // Состояние с реализацией (`state X = Model`, `= M1 | M2`) — композиция:
    // предмет части 2. Отказ обязателен и не косметичен: без него ветвь
    // напечаталась бы БЕЗ вызова под-модели, то есть автомат молча потерял бы
    // всю вложенную логику — тихое расхождение, худший класс дефекта (0025).
    if let StateNode::Implement { name, .. } = state {
        return Err(Diagnostic::error(
            Location::Codegen,
            format!(
                "Состояние '{}' реализовано композицией моделей: её трансляция — \
                 часть 2 задачи 0041-03 (экземпляры под-FB, VAR_IN_OUT, конъюнкция \
                 is_done). Напечатать ветвь без вызова под-модели значило бы молча \
                 потерять её логику",
                name
            ),
        )
        .with_code("ST-011"));
    }

    // `always` — первым в ветви, до проверок переходов (S8, Ф5).
    emit_block(p, state, "always", model, out)?;

    let references = match state {
        StateNode::Simple { references, .. } | StateNode::Implement { references, .. } => {
            references.clone()
        }
        StateNode::Unresolved => Vec::new(),
    };

    if references.is_empty() {
        // Состояние без исходящих переходов терминально: исполняет `exit` и
        // уходит в `END` — как `case B` в зонде `exit_probe` (Ф8).
        emit_block(p, state, "exit", model, out)?;
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
        let guard = print_condition(&reference.cond, model)?;
        if guard.is_empty() {
            // Безусловный переход: цепочку прерывать нечем — печатаем как есть.
            emit_transition(p, state, &reference.name, target, model, out)?;
            return Ok(());
        }
        p.ident(&format!(
            "{} {} THEN",
            if printed_if { "ELSIF" } else { "IF" },
            guard
        ))
        .nl();
        p.up();
        emit_transition(p, state, &reference.name, target, model, out)?;
        p.down();
        printed_if = true;
    }
    if printed_if {
        p.ident("END_IF;").nl();
    }
    Ok(())
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
    use crate::semantic::minimap::Map;
    use crate::semantic::tree::construct_model;
    use std::rc::Rc;

    /// Печатает тело автомата корневой модели исходника.
    fn body_of(src: &str) -> String {
        let (ast, _) = crate::parse(src, 0).unwrap();
        let rc = construct_model(&ast, None, &[]).unwrap();
        rc.borrow_mut().name = Some("Root".to_string());
        let map = Map::create(Rc::clone(&rc)).unwrap();
        let element = map.model();
        let Element::Model { states, .. } = &element else {
            panic!("корень не модель");
        };
        let table = StateTable::build(states);
        let model = rc.borrow();
        let mut text = String::new();
        let mut p = Printer::new(4, &mut text);
        emit_body(&mut p, &element, &model, &table).expect("тело должно печататься");
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

    /// Состояние-композиция отвергается, а не печатается без вызова под-модели.
    ///
    /// Ключевой сторож части 1: напечатать ветвь `= Controller` без вызова
    /// под-FB значило бы молча потерять всю вложенную логику. Отказ громкий до
    /// тех пор, пока часть 2 не научится композиции.
    #[test]
    fn test_composed_state_is_refused_not_silently_emitted_without_submodel() {
        let (ast, _) = crate::parse(
            "model M { start Q { } }\nvar n: u8 := 0;\nstart Entry = M;",
            0,
        )
        .unwrap();
        let rc = construct_model(&ast, None, &[]).unwrap();
        rc.borrow_mut().name = Some("Root".to_string());
        let map = Map::create(Rc::clone(&rc)).unwrap();
        let element = map.model();
        let Element::Model { states, .. } = &element else {
            panic!("корень не модель");
        };
        let table = StateTable::build(states);
        let model = rc.borrow();
        let mut text = String::new();
        let mut p = Printer::new(4, &mut text);
        let err = emit_body(&mut p, &element, &model, &table)
            .expect_err("композиция обязана отвергаться до части 2");
        assert_eq!(err.code.as_deref(), Some("ST-011"));
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
