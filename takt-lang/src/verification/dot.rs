//! Экспорт графов верификации в формат **Graphviz DOT** (фича 0124).
//!
//! Структуры Крипке (`kripke.rs`), автомат Бюхи (`buchi.rs`) и их произведение
//! (`product.rs`) — те же, что использует проверка, — печатаются графом переходов
//! для рендера `dot -Tsvg` и встраивания в документ (`book/`). Диаграмма = вывод
//! движка: правка верификации отражается в картинке (ADR 0124, драйвер 1).
//!
//! Соглашения нотации:
//! - **старт** — точка-источник (`shape=point`) со стрелкой в начальную вершину;
//! - **принимающее** состояние автомата/произведения — двойной кружок
//!   (`shape=doublecircle`);
//! - идентификатор узла — **индекс** вершины (в структуре Крипке по данным одно
//!   имя состояния делят несколько вершин — уникально только смещение), метка —
//!   человекочитаемая (имя состояния, для пути данных плюс набор истинных атомов).
//!
//! Порядок вывода детерминирован (обход по возрастанию индекса, множества —
//! `BTreeSet`), поэтому DOT воспроизводим побайтно (согласуется с гейтом 0048).

use crate::verification::buchi::BuchiAutomaton;
use crate::verification::kripke::Kripke;
use crate::verification::ltl::Ltl;
use crate::verification::product::Product;

/// Экранирует метку узла для DOT (кавычки и обратный слэш).
fn escape(label: &str) -> String {
    label.replace('\\', "\\\\").replace('"', "\\\"")
}

/// DOT структуры Крипке.
///
/// Вершины — состояния FSM; тотальность (самопетля у вершины без безусловного
/// выхода, `may_stutter`) видна ребром-петлёй. Для пути данных (0068, `labels`
/// непусто) метка вершины несёт набор истинных в ней атомов.
pub fn kripke_to_dot(kripke: &Kripke) -> String {
    let mut out = String::from("digraph Kripke {\n");
    out.push_str("  rankdir=LR;\n");
    out.push_str("  node [shape=circle, fontname=\"monospace\"];\n");
    out.push_str("  edge [fontname=\"monospace\"];\n");
    // Точка-источник → начальная вершина.
    out.push_str("  __start [shape=point, width=0.12];\n");
    out.push_str(&format!("  __start -> k{};\n", kripke.initial));

    for (k, name) in kripke.states.iter().enumerate() {
        // Компоненты экранируются по отдельности; разделитель `\n` — настоящий
        // перенос строки метки DOT и экранированию не подлежит.
        let label = if kripke.labels.is_empty() {
            escape(name)
        } else {
            // Путь данных: имя состояния + истинные атомы (кроме самого имени).
            let atoms: Vec<String> = kripke.labels[k]
                .iter()
                .filter(|a| a.as_str() != name)
                .map(|a| escape(a))
                .collect();
            if atoms.is_empty() {
                escape(name)
            } else {
                format!("{}\\n{{{}}}", escape(name), atoms.join(", "))
            }
        };
        out.push_str(&format!("  k{k} [label=\"{label}\"];\n"));
    }
    for (&from, tos) in &kripke.transitions {
        for &to in tos {
            out.push_str(&format!("  k{from} -> k{to};\n"));
        }
    }
    out.push_str("}\n");
    out
}

/// Литералы состояния автомата Бюхи — ограничения на текущую букву (как в
/// [`BuchiAutomaton::dump`]): только `Atom`/`Not`, прочие формулы состояния —
/// темпоральные обязательства, а не разметка узла.
fn buchi_state_label(i: usize, formulas: &std::collections::BTreeSet<std::rc::Rc<Ltl>>) -> String {
    let literals: Vec<String> = formulas
        .iter()
        .filter(|f| matches!(f.as_ref(), Ltl::Atom(_) | Ltl::Not(_)))
        .map(|f| escape(&f.to_string()))
        .collect();
    // Разделитель `\n` — настоящий перенос строки метки DOT (не экранируется).
    if literals.is_empty() {
        format!("s{i}")
    } else {
        format!("s{i}\\n{{{}}}", literals.join(", "))
    }
}

/// DOT автомата Бюхи для `¬φ`.
///
/// Начальные состояния получают стрелку из точки-источника, **принимающие** —
/// двойной кружок. Язык автомата — нарушающие свойство прогоны.
pub fn buchi_to_dot(automaton: &BuchiAutomaton) -> String {
    let mut out = String::from("digraph Buchi {\n");
    out.push_str("  rankdir=LR;\n");
    out.push_str("  node [shape=circle, fontname=\"monospace\"];\n");
    out.push_str("  edge [fontname=\"monospace\"];\n");

    for (i, formulas) in automaton.states.iter().enumerate() {
        let shape = if automaton.accepting.contains(&i) {
            "doublecircle"
        } else {
            "circle"
        };
        out.push_str(&format!(
            "  s{i} [shape={shape}, label=\"{}\"];\n",
            buchi_state_label(i, formulas)
        ));
    }
    // Точки-источники начальных состояний.
    for &init in &automaton.initial_states {
        out.push_str(&format!("  __start{init} [shape=point, width=0.12];\n"));
        out.push_str(&format!("  __start{init} -> s{init};\n"));
    }
    for (&from, tos) in &automaton.transitions {
        for &to in tos {
            out.push_str(&format!("  s{from} -> s{to};\n"));
        }
    }
    out.push_str("}\n");
    out
}

/// DOT произведения `K × A_¬φ`.
///
/// Узел — пара `(состояние FSM, состояние автомата)`; **принимающие** пары —
/// двойной кружок, начальные — стрелка из точки-источника. Непустой цикл через
/// принимающую пару = контрпример (проверяется [`super::check::emptiness`]).
pub fn product_to_dot(product: &Product, kripke: &Kripke) -> String {
    let mut out = String::from("digraph Product {\n");
    out.push_str("  rankdir=LR;\n");
    out.push_str("  node [shape=circle, fontname=\"monospace\"];\n");
    out.push_str("  edge [fontname=\"monospace\"];\n");

    for (s, &(k, q)) in product.states.iter().enumerate() {
        let shape = if product.accepting.contains(&s) {
            "doublecircle"
        } else {
            "circle"
        };
        let label = format!("{},q{q}", escape(&kripke.states[k]));
        out.push_str(&format!("  p{s} [shape={shape}, label=\"{label}\"];\n"));
    }
    for &init in &product.initial {
        out.push_str(&format!("  __start{init} [shape=point, width=0.12];\n"));
        out.push_str(&format!("  __start{init} -> p{init};\n"));
    }
    for (&from, tos) in &product.transitions {
        for &to in tos {
            out.push_str(&format!("  p{from} -> p{to};\n"));
        }
    }
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::tree::construct_model;
    use crate::verification::verify::{build_control_kripke, build_graphs};
    use crate::{parse, parse_ltl_property};

    /// Строит модель из исходника (для тестов эмиттеров).
    fn model(src: &str) -> std::rc::Rc<std::cell::RefCell<crate::semantic::ModelNode>> {
        let (ast, _) = parse(src, 0).expect("разбор");
        construct_model(&ast, None, &[]).expect("семантика")
    }

    const DISPENSER: &str = "\
        var level: u8 := 0;\n\
        start Idle { next Filling; }\n\
        state Filling { always { level := level + 20; } ref Ready: level >= 100; }\n\
        state Ready { next Idle; }";

    #[test]
    fn kripke_dot_marks_start_and_all_states() {
        let m = model(DISPENSER);
        let k = build_control_kripke(&m.borrow()).unwrap();
        let dot = kripke_to_dot(&k);
        assert!(dot.starts_with("digraph Kripke {"), "заголовок: {dot}");
        // Точка-источник ведёт в начальную вершину.
        assert!(
            dot.contains(&format!("__start -> k{};\n", k.initial)),
            "старт: {dot}"
        );
        // Каждое состояние — узел с меткой-именем.
        for name in &k.states {
            assert!(dot.contains(&format!("label=\"{name}\"")), "узел {name}: {dot}");
        }
        assert!(dot.trim_end().ends_with('}'));
    }

    #[test]
    fn buchi_dot_accepting_is_doublecircle() {
        let m = model(DISPENSER);
        let phi = parse_ltl_property("F Filling").unwrap();
        let g = build_graphs(&m.borrow(), &phi).unwrap();
        let dot = buchi_to_dot(&g.automaton);
        assert!(dot.starts_with("digraph Buchi {"));
        // Хотя бы одно принимающее состояние — двойной кружок.
        assert!(!g.automaton.accepting.is_empty(), "автомат ¬(F Filling) принимает");
        for &acc in &g.automaton.accepting {
            assert!(
                dot.contains(&format!("s{acc} [shape=doublecircle")),
                "принимающее s{acc}: {dot}"
            );
        }
        // Начальные состояния получают точку-источник.
        for &init in &g.automaton.initial_states {
            assert!(dot.contains(&format!("__start{init} -> s{init};")), "нач s{init}");
        }
    }

    #[test]
    fn product_dot_labels_are_state_automaton_pairs() {
        let m = model(DISPENSER);
        let phi = parse_ltl_property("F Ready").unwrap();
        let g = build_graphs(&m.borrow(), &phi).unwrap();
        let dot = product_to_dot(&g.product, &g.kripke);
        assert!(dot.starts_with("digraph Product {"));
        // Метка узла — пара (имя состояния, qN).
        for (s, &(k, q)) in g.product.states.iter().enumerate() {
            let name = &g.kripke.states[k];
            assert!(
                dot.contains(&format!("p{s} [shape=")) && dot.contains(&format!("{name},q{q}")),
                "пара p{s}=({name},q{q}): {dot}"
            );
        }
    }

    #[test]
    fn data_path_kripke_label_carries_atoms() {
        // Предикат над отслеживаемым bit-var (0068) → метка несёт атом.
        let src = "\
            var flag: bit := 0;\n\
            cond On = flag = 1;\n\
            start A { always { flag := 1; } ref B: flag = 1; }\n\
            state B;";
        let m = model(src);
        let phi = parse_ltl_property("F On").unwrap();
        let g = build_graphs(&m.borrow(), &phi).unwrap();
        let dot = kripke_to_dot(&g.kripke);
        // Есть вершина в состоянии A, где предикат On истинен → метка "A\n{On}".
        assert!(dot.contains("A\\n{On}"), "метка данных: {dot}");
    }

    #[test]
    fn labels_use_real_newline_not_escaped_backslash() {
        // Разделитель метки — `\n` (одиночный слэш = перенос строки DOT), а не
        // `\\n` (был бы литеральный текст) — регресс двойного экранирования.
        let src = "var flag: bit := 0;\n cond On = flag = 1;\n \
                   start A { always { flag := 1; } ref B: flag = 1; }\n state B;";
        let m = model(src);
        let phi = parse_ltl_property("F On").unwrap();
        let g = build_graphs(&m.borrow(), &phi).unwrap();
        let dot = kripke_to_dot(&g.kripke);
        assert!(!dot.contains("\\\\n"), "двойное экранирование \\n: {dot}");
    }

    #[test]
    fn escape_quotes_in_label() {
        assert_eq!(escape("a\"b"), "a\\\"b");
        assert_eq!(escape("a\\b"), "a\\\\b");
    }
}
