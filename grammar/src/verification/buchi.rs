use crate::verification::ltl::Ltl;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::Hash;
use std::rc::Rc;

fn to_nnf(formula: &Ltl) -> Rc<Ltl> {
    match formula {
        Ltl::True => Rc::new(Ltl::True),
        Ltl::False => Rc::new(Ltl::False),
        Ltl::Atom(a) => Rc::new(Ltl::Atom(a.clone())),
        Ltl::And(a, b) => Rc::new(Ltl::And(to_nnf(a), to_nnf(b))),
        Ltl::Or(a, b) => Rc::new(Ltl::Or(to_nnf(a), to_nnf(b))),
        Ltl::Implies(a, b) => to_nnf(&Ltl::Or(Rc::new(Ltl::Not(a.clone())), b.clone())),
        Ltl::Next(a) => Rc::new(Ltl::Next(to_nnf(a))),
        Ltl::Finally(a) => Rc::new(Ltl::Until(Rc::new(Ltl::True), to_nnf(a))),
        Ltl::Globally(a) => Rc::new(Ltl::Release(Rc::new(Ltl::False), to_nnf(a))),
        Ltl::Until(a, b) => Rc::new(Ltl::Until(to_nnf(a), to_nnf(b))),
        Ltl::Release(a, b) => Rc::new(Ltl::Release(to_nnf(a), to_nnf(b))),
        Ltl::Not(a) => match a.as_ref() {
            Ltl::True => Rc::new(Ltl::False),
            Ltl::False => Rc::new(Ltl::True),
            Ltl::Atom(x) => Rc::new(Ltl::Not(Rc::new(Ltl::Atom(x.clone())))),
            Ltl::Not(b) => to_nnf(b),
            Ltl::And(b, c) => Rc::new(Ltl::Or(
                to_nnf(&Ltl::Not(b.clone())),
                to_nnf(&Ltl::Not(c.clone())),
            )),
            Ltl::Or(b, c) => Rc::new(Ltl::And(
                to_nnf(&Ltl::Not(b.clone())),
                to_nnf(&Ltl::Not(c.clone())),
            )),
            Ltl::Implies(b, c) => to_nnf(&Ltl::Not(Rc::new(Ltl::Or(
                Rc::new(Ltl::Not(b.clone())),
                c.clone(),
            )))),
            Ltl::Next(b) => Rc::new(Ltl::Next(to_nnf(&Ltl::Not(b.clone())))),
            Ltl::Finally(b) => to_nnf(&Ltl::Not(Rc::new(Ltl::Until(
                Rc::new(Ltl::True),
                b.clone(),
            )))),
            Ltl::Globally(b) => to_nnf(&Ltl::Not(Rc::new(Ltl::Release(
                Rc::new(Ltl::False),
                b.clone(),
            )))),
            Ltl::Until(b, c) => Rc::new(Ltl::Release(
                to_nnf(&Ltl::Not(b.clone())),
                to_nnf(&Ltl::Not(c.clone())),
            )),
            Ltl::Release(b, c) => Rc::new(Ltl::Until(
                to_nnf(&Ltl::Not(b.clone())),
                to_nnf(&Ltl::Not(c.clone())),
            )),
        },
    }
}

// ============================================================================
// 4. Структуры данных для автомата
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
struct GbaNode {
    old: BTreeSet<Rc<Ltl>>,
    next: BTreeSet<Rc<Ltl>>,
}

#[derive(Debug)]
pub struct BuchiAutomaton {
    pub states: Vec<BTreeSet<Rc<Ltl>>>,
    pub initial_states: BTreeSet<usize>,
    pub transitions: BTreeMap<usize, BTreeSet<usize>>,
    pub accepting: BTreeSet<usize>,
}

impl BuchiAutomaton {
    pub fn print(&self) {
        println!("=== Автомат Бюхи (GPVW) ===");
        println!("Начальные состояния: {:?}", self.initial_states);
        println!("Принимающие состояния: {:?}", self.accepting);
        for i in 0..self.states.len() {
            print!("s{} : {{ ", i);
            for f in &self.states[i] {
                if matches!(f.as_ref(), Ltl::Atom(_) | Ltl::Not(_)) {
                    print!("{} ", f);
                }
            }
            println!("}}");
            if let Some(tos) = self.transitions.get(&i) {
                print!("  переходы -> ");
                for &to in tos {
                    print!("s{} ", to);
                }
                println!();
            }
        }
    }
}

// ============================================================================
// 5. Алгоритм GPVW (построение автомата Бюхи)
// ============================================================================

fn collect_until(f: &Ltl, set: &mut BTreeSet<Rc<Ltl>>) {
    match f {
        Ltl::And(a, b) | Ltl::Or(a, b) | Ltl::Until(a, b) | Ltl::Release(a, b) => {
            if matches!(f, Ltl::Until(_, _)) {
                set.insert(Rc::new(f.clone()));
            }
            collect_until(a, set);
            collect_until(b, set);
        }
        Ltl::Next(a) | Ltl::Not(a) => {
            collect_until(a, set);
        }
        _ => {}
    }
}

fn expand(
    mut new: BTreeSet<Rc<Ltl>>,
    mut old: BTreeSet<Rc<Ltl>>,
    mut next: BTreeSet<Rc<Ltl>>,
    incoming: BTreeSet<Option<usize>>,
    nodes: &mut Vec<GbaNode>,
    node_map: &mut BTreeMap<(BTreeSet<Rc<Ltl>>, BTreeSet<Rc<Ltl>>), usize>,
    transitions: &mut BTreeSet<(usize, usize)>,
    initial_nodes: &mut BTreeSet<usize>,
) {
    if new.is_empty() {
        let key = (old.clone(), next.clone());
        let id = if let Some(&id) = node_map.get(&key) {
            id
        } else {
            let id = nodes.len();
            nodes.push(GbaNode { old, next });
            node_map.insert(key, id);

            let next_new = nodes[id].next.clone();
            let mut next_inc = BTreeSet::new();
            next_inc.insert(Some(id));
            expand(
                next_new,
                BTreeSet::new(),
                BTreeSet::new(),
                next_inc,
                nodes,
                node_map,
                transitions,
                initial_nodes,
            );
            id
        };

        for inc in incoming {
            if let Some(from_id) = inc {
                transitions.insert((from_id, id));
            } else {
                initial_nodes.insert(id);
            }
        }
        return;
    }

    let f = new.pop_first().unwrap();
    if f.as_ref() == &Ltl::True {
        expand(
            new,
            old,
            next,
            incoming,
            nodes,
            node_map,
            transitions,
            initial_nodes,
        );
    } else if f.as_ref() == &Ltl::False {
        return;
    } else {
        match f.as_ref() {
            Ltl::Atom(_) | Ltl::Not(_) => {
                let neg_f = match f.as_ref() {
                    Ltl::Atom(a) => Rc::new(Ltl::Not(Rc::new(Ltl::Atom(a.clone())))),
                    Ltl::Not(inner) => inner.clone(),
                    _ => unreachable!(),
                };
                if old.contains(&neg_f) {
                    return;
                }
                old.insert(f);
                expand(
                    new,
                    old,
                    next,
                    incoming,
                    nodes,
                    node_map,
                    transitions,
                    initial_nodes,
                );
            }
            Ltl::And(f1, f2) => {
                new.insert(f1.clone());
                new.insert(f2.clone());
                expand(
                    new,
                    old,
                    next,
                    incoming,
                    nodes,
                    node_map,
                    transitions,
                    initial_nodes,
                );
            }
            Ltl::Or(f1, f2) => {
                let mut new1 = new.clone();
                new1.insert(f1.clone());
                expand(
                    new1,
                    old.clone(),
                    next.clone(),
                    incoming.clone(),
                    nodes,
                    node_map,
                    transitions,
                    initial_nodes,
                );
                new.insert(f2.clone());
                expand(
                    new,
                    old,
                    next,
                    incoming,
                    nodes,
                    node_map,
                    transitions,
                    initial_nodes,
                );
            }
            Ltl::Next(f1) => {
                next.insert(f1.clone());
                expand(
                    new,
                    old,
                    next,
                    incoming,
                    nodes,
                    node_map,
                    transitions,
                    initial_nodes,
                );
            }
            Ltl::Until(f1, f2) => {
                let mut new1 = new.clone();
                new1.insert(f2.clone());
                expand(
                    new1,
                    old.clone(),
                    next.clone(),
                    incoming.clone(),
                    nodes,
                    node_map,
                    transitions,
                    initial_nodes,
                );
                new.insert(f1.clone());
                next.insert(f.clone());
                expand(
                    new,
                    old,
                    next,
                    incoming,
                    nodes,
                    node_map,
                    transitions,
                    initial_nodes,
                );
            }
            Ltl::Release(f1, f2) => {
                let mut new1 = new.clone();
                new1.insert(f1.clone());
                new1.insert(f2.clone());
                expand(
                    new1,
                    old.clone(),
                    next.clone(),
                    incoming.clone(),
                    nodes,
                    node_map,
                    transitions,
                    initial_nodes,
                );
                new.insert(f2.clone());
                next.insert(f.clone());
                expand(
                    new,
                    old,
                    next,
                    incoming,
                    nodes,
                    node_map,
                    transitions,
                    initial_nodes,
                );
            }
            _ => unreachable!("Formula not in NNF: {:?}", f),
        }
    }
}

pub fn build_buchi(phi: &Ltl) -> BuchiAutomaton {
    let nnf_phi = to_nnf(phi);
    let mut nodes = Vec::new();
    let mut node_map = BTreeMap::new();
    let mut transitions = BTreeSet::new();
    let mut initial_nodes = BTreeSet::new();

    let mut new = BTreeSet::new();
    new.insert(nnf_phi.clone());
    let mut incoming = BTreeSet::new();
    incoming.insert(None);

    expand(
        new,
        BTreeSet::new(),
        BTreeSet::new(),
        incoming,
        &mut nodes,
        &mut node_map,
        &mut transitions,
        &mut initial_nodes,
    );

    let mut until_formulas = BTreeSet::new();
    collect_until(&nnf_phi, &mut until_formulas);
    let until_list: Vec<_> = until_formulas.into_iter().collect();
    let k = until_list.len();

    let mut ba_states = Vec::new();
    let mut ba_initial = BTreeSet::new();
    let mut ba_transitions: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    let mut ba_accepting = BTreeSet::new();

    let n_nodes = nodes.len();
    if n_nodes == 0 {
        return BuchiAutomaton {
            states: vec![],
            initial_states: BTreeSet::new(),
            transitions: BTreeMap::new(),
            accepting: BTreeSet::new(),
        };
    }

    for i in 0..n_nodes {
        for _ in 0..=k {
            ba_states.push(nodes[i].old.clone());
        }
    }

    for &init_node in &initial_nodes {
        ba_initial.insert(init_node * (k + 1));
    }

    for &(u, v) in &transitions {
        for i in 0..=k {
            let next_i = if let Some(f_until) = until_list.get(i) {
                let chi = match f_until.as_ref() {
                    Ltl::Until(_, c) => c,
                    _ => unreachable!(),
                };
                if nodes[u].old.contains(chi) || !nodes[u].old.contains(f_until) {
                    i + 1
                } else {
                    i
                }
            } else {
                0
            };
            ba_transitions
                .entry(u * (k + 1) + i)
                .or_default()
                .insert(v * (k + 1) + next_i);
        }
    }

    for i in 0..n_nodes {
        ba_accepting.insert(i * (k + 1) + k);
    }

    if k == 0 {
        for i in 0..n_nodes {
            ba_accepting.insert(i);
        }
    }

    BuchiAutomaton {
        states: ba_states,
        initial_states: ba_initial,
        transitions: ba_transitions,
        accepting: ba_accepting,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verification::ltl::parse_ltl;

    #[test]
    fn test_parse_ltl() {
        assert_eq!(
            parse_ltl("false & G true"),
            Ltl::And(
                Rc::new(Ltl::False),
                Rc::new(Ltl::Globally(Rc::new(Ltl::True)))
            )
        );
    }

    #[test]
    fn test_x_p() {
        let formula = parse_ltl("X p");
        let automaton = build_buchi(&formula);
        automaton.print();
        assert!(!automaton.states.is_empty());
    }

    #[test]
    fn test_f_p() {
        let formula = parse_ltl("F p");
        let automaton = build_buchi(&formula);
        automaton.print();
        assert!(!automaton.accepting.is_empty());
    }

    #[test]
    fn test_g_p() {
        let formula = parse_ltl("G p");
        let automaton = build_buchi(&formula);
        automaton.print();
    }

    #[test]
    fn test_complex() {
        let formula = parse_ltl("G (p -> F q)");
        let automaton = build_buchi(&formula);
        automaton.print();
    }

    #[test]
    fn test_until() {
        let formula = parse_ltl("p U q");
        let automaton = build_buchi(&formula);
        automaton.print();
        assert!(!automaton.states.is_empty());
    }

    #[test]
    fn test_release() {
        let formula = parse_ltl("p R q");
        let automaton = build_buchi(&formula);
        automaton.print();
        assert!(!automaton.states.is_empty());
    }

    #[test]
    fn test_gf() {
        let formula = parse_ltl("G F p");
        let automaton = build_buchi(&formula);
        automaton.print();
        assert!(!automaton.accepting.is_empty());
    }

    #[test]
    fn test_fg() {
        let formula = parse_ltl("F G p");
        let automaton = build_buchi(&formula);
        automaton.print();
    }

    #[test]
    fn test_nested_until() {
        let formula = parse_ltl("p U (q U r)");
        let automaton = build_buchi(&formula);
        automaton.print();
    }
}
