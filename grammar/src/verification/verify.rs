//! Движок верификации: вердикт по LTL-свойству модели (фича 0049, задача
//! 0049-03).
//!
//! Конвейер (стандартный автоматный подход к model checking):
//!
//! ```text
//!   φ ──┬──> связывание атомов (атом = имя состояния?) ──> Unsupported
//!       │
//!       ├──> A = build_buchi(!φ)          (затравка 0010)
//!  model ──> K = build_kripke(model)      (0049-01, абстракция управления)
//!       │
//!       └──> P = K × A  (0049-02) ──> emptiness(P):
//!              пусто     ⟹ Holds     (ни один прогон не нарушает φ)
//!              непусто   ⟹ Violated  (лассо-контрпример)
//! ```
//!
//! Проверяется **отрицание**: язык `K × A_¬φ` содержит ровно те прогоны модели,
//! которые нарушают `φ`. Пустой язык — доказательство свойства на всех
//! (бесконечных) прогонах абстракции, а не проверка до какой-то глубины.
//!
//! # Что означает вердикт
//!
//! Абстракция управления (ADR 0049, Option A) даёт **over-approximation**:
//! прогонов у `K` не меньше, чем у реальной модели (условия рёбер
//! игнорируются). Отсюда несимметричность, которую обязан понимать
//! пользователь:
//!
//! - `Holds` — **надёжен**: свойство держится на всех прогонах абстракции,
//!   значит и на всех реальных.
//! - `Violated` — контрпример существует **в абстракции**; по данным он может
//!   быть недостижим (ложное срабатывание). Помечается «абстракция управления»
//!   (A4, правило 5 — честность результата).

use crate::semantic::ModelNode;
use crate::verification::buchi::build_buchi;
use crate::verification::check::emptiness;
use crate::verification::kripke::build_kripke;
use crate::verification::ltl::Ltl;
use crate::verification::product::product;
use std::collections::BTreeSet;
use std::rc::Rc;

/// Контрпример: бесконечный прогон модели, нарушающий свойство.
///
/// Лассо в **именах состояний FSM**: конечный префикс, затем цикл, повторяемый
/// бесконечно.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counterexample {
    /// Путь от стартового состояния до входа в цикл (без самого входа).
    pub prefix: Vec<String>,
    /// Цикл, повторяемый бесконечно.
    pub cycle: Vec<String>,
}

impl Counterexample {
    /// Человекочитаемая трасса: `Idle -> Fault -> [ Fault -> Retry ]*`.
    pub fn trace(&self) -> String {
        let mut out = String::new();
        if !self.prefix.is_empty() {
            out.push_str(&self.prefix.join(" -> "));
            out.push_str(" -> ");
        }
        out.push_str(&format!("[ {} ]*", self.cycle.join(" -> ")));
        out
    }
}

/// Вердикт проверки LTL-свойства модели.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Свойство держится на всех прогонах абстракции управления (надёжно).
    Holds,
    /// Свойство нарушено: найден контрпример-лассо.
    Violated(Counterexample),
    /// Свойство непроверяемо: перечисленные атомы не являются именами состояний
    /// модели (данные вне абстракции управления, ADR 0049 A3).
    ///
    /// Не молчаливое «ложно»: `G(temp <= 100)` не проверяется, и делать вид, что
    /// проверено, нельзя.
    Unsupported(Vec<String>),
    /// У модели нет стартового состояния — проверять нечего.
    ///
    /// Через `construct_model` такая модель не проходит (SE-011); ветка
    /// защитная: молчаливое «держится» на пустом графе было бы ложью.
    NoStartState,
}

/// Проверяет LTL-свойство `phi` на модели (model checking, ADR 0049 Option A).
///
/// Атом формулы — **имя состояния** FSM: `S` истинен, когда автомат находится в
/// состоянии `S`. Свойства над данными (`var`/порты/`cond`) в этой абстракции
/// невыразимы и дают [`Verdict::Unsupported`].
///
/// # Пример
///
/// ```
/// use grammar::parse;
/// use grammar::semantic::tree::construct_model;
/// use grammar::verification::ltl::Ltl;
/// use grammar::verification::verify::Verdict;
/// use std::rc::Rc;
///
/// let (ast, _) = parse("start Idle { ref Done; } state Done;", 0).unwrap();
/// let model = construct_model(&ast, None, &[]).unwrap();
///
/// // F Done — «состояние Done рано или поздно достигается».
/// let phi = Ltl::Finally(Rc::new(Ltl::Atom("Done".to_string())));
/// assert_eq!(grammar::verify_model(model, &phi), Verdict::Holds);
/// ```
pub fn verify_model(model: &ModelNode, phi: &Ltl) -> Verdict {
    run(model, phi, &mut None)
}

/// То же, что [`verify_model`], плюс текстовый дамп конвейера для отладки
/// (`lamc verify --trace`): структура Крипке, автомат `¬φ`, произведение.
pub fn verify_model_traced(model: &ModelNode, phi: &Ltl) -> (Verdict, String) {
    let mut trace = Some(String::new());
    let verdict = run(model, phi, &mut trace);
    (verdict, trace.unwrap_or_default())
}

fn run(model: &ModelNode, phi: &Ltl, trace: &mut Option<String>) -> Verdict {
    let Some(kripke) = build_kripke(model) else {
        return Verdict::NoStartState;
    };
    if let Some(t) = trace.as_mut() {
        t.push_str(&kripke.trace());
    }

    // R2/R7: связывание атомов до проверки. Атом, не являющийся именем
    // состояния, ложен во всех вершинах — проверка «прошла бы» с бессмысленным
    // вердиктом, поэтому отказываем громко.
    let mut atoms = BTreeSet::new();
    crate::semantic::ltl_check::collect_atoms(phi, &mut atoms);
    let unknown = kripke.unknown_atoms(&atoms);
    if !unknown.is_empty() {
        return Verdict::Unsupported(unknown);
    }

    // Автомат строится по ОТРИЦАНИЮ свойства: его язык — нарушающие прогоны.
    let automaton = build_buchi(&Ltl::Not(Rc::new(phi.clone())));
    if let Some(t) = trace.as_mut() {
        t.push_str(&format!(
            "=== Формула {} ; автомат строится по отрицанию !({}) — его язык суть \
             нарушающие прогоны ===\n",
            phi, phi
        ));
        t.push_str(&automaton.dump());
    }

    let prod = product(&kripke, &automaton);
    if let Some(t) = trace.as_mut() {
        t.push_str(&prod.trace(&kripke));
    }

    match emptiness(&prod) {
        None => Verdict::Holds,
        Some(lasso) => {
            let (prefix, cycle) = lasso.state_names(&kripke, &prod);
            Verdict::Violated(counterexample(prefix, cycle))
        }
    }
}

/// Приводит спроецированное лассо к читаемому виду.
///
/// Обе правки **сохраняют слово** `префикс · цикл^ω` — меняется лишь его
/// запись. Иначе бы это была ложь о прогоне, а не упрощение (правило 5).
///
/// 1. Цикл сжимается до минимального периода: `p^k` и `p` задают одно и то же
///    `p^ω`. Повторы возникают из-за того, что автомат `¬φ` проходит по вершине
///    Крипке несколько раз в разных своих состояниях — пользователю это не
///    видно и не нужно.
/// 2. Хвост префикса, совпадающий с циклом-самопетлёй, отбрасывается: при
///    периоде длины 1 верно `x · x^ω = x^ω`. Для периода длиннее это НЕ верно
///    (`A W · (W K)^ω ≠ A · (W K)^ω`), поэтому правка ограничена самопетлёй.
fn counterexample(prefix: Vec<String>, cycle: Vec<String>) -> Counterexample {
    let cycle = minimal_period(&cycle);
    // Последняя вершина префикса — вход в цикл; в тексте она уже печатается как
    // первая вершина цикла, дублировать её незачем.
    let mut prefix = prefix[..prefix.len().saturating_sub(1)].to_vec();
    if let [single] = cycle.as_slice() {
        while prefix.last() == Some(single) {
            prefix.pop();
        }
    }
    Counterexample { prefix, cycle }
}

/// Минимальный период последовательности: `[F, F]` → `[F]`, `[W, K]` → `[W, K]`.
fn minimal_period(cycle: &[String]) -> Vec<String> {
    for period in 1..=cycle.len() {
        if cycle.len() % period != 0 {
            continue;
        }
        if cycle
            .iter()
            .enumerate()
            .all(|(i, name)| name == &cycle[i % period])
        {
            return cycle[..period].to_vec();
        }
    }
    cycle.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use crate::semantic::tree::construct_model;

    fn verdict(src: &str, phi: &Ltl) -> Verdict {
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let v = verify_model(&model.borrow(), phi);
        v
    }

    fn atom(name: &str) -> Rc<Ltl> {
        Rc::new(Ltl::Atom(name.to_string()))
    }

    // ─── Известные теоремы (ADR 0049, критерий A7) ───────────────────────────
    //
    // Тесты — на ВЕРДИКТ, а не на «автомат непуст» (капкан 0025: зелёные тесты
    // на структуру при неверной семантике).

    /// `F Done` держится, если Done достижимо и неизбежно.
    #[test]
    fn finally_holds_when_state_is_inevitable() {
        assert_eq!(
            verdict(
                "start A { ref Done; } state Done;",
                &Ltl::Finally(atom("Done"))
            ),
            Verdict::Holds
        );
    }

    /// `F Done` нарушается, если из старта есть прогон, минующий Done навсегда.
    #[test]
    fn finally_violated_when_state_is_avoidable() {
        // A может крутиться в цикле A -> B -> A, не заходя в Done.
        let v = verdict(
            "start A { ref B; ref Done; } state B { ref A; } state Done;",
            &Ltl::Finally(atom("Done")),
        );
        let Verdict::Violated(cex) = v else {
            panic!("прогон A -> B -> A -> ... минует Done: ожидалось нарушение, получено {v:?}");
        };
        assert!(
            !cex.cycle.contains(&"Done".to_string()),
            "цикл контрпримера обязан обходить Done: {}",
            cex.trace()
        );
    }

    /// `F Done` нарушается, если Done недостижимо (критерий A3).
    #[test]
    fn finally_violated_when_state_is_unreachable() {
        // Done не достижимо из A: единственный прогон — самопетля A.
        let v = verdict(
            "start A { ref A; } state Done;",
            &Ltl::Finally(atom("Done")),
        );
        let Verdict::Violated(cex) = v else {
            panic!("Done недостижимо: ожидалось нарушение, получено {v:?}");
        };
        assert_eq!(cex.trace(), "[ A ]*");
    }

    /// `G A` держится на модели, которая всегда в A.
    #[test]
    fn globally_holds_on_single_state_model() {
        assert_eq!(
            verdict("start A { ref A; }", &Ltl::Globally(atom("A"))),
            Verdict::Holds
        );
    }

    /// `G A` нарушается, как только достижимо любое другое состояние.
    #[test]
    fn globally_violated_when_another_state_is_reachable() {
        let v = verdict("start A { ref B; } state B;", &Ltl::Globally(atom("A")));
        let Verdict::Violated(cex) = v else {
            panic!("B достижимо: ожидалось нарушение, получено {v:?}");
        };
        assert!(
            cex.trace().contains('B'),
            "контрпример обязан показывать выход в B: {}",
            cex.trace()
        );
    }

    // ─── Управляющие свойства (критерий A2) ──────────────────────────────────

    /// `G(Fault -> F Idle)` держится: из Fault единственный путь ведёт в Idle.
    #[test]
    fn response_property_holds_when_fault_always_leads_to_idle() {
        let phi = Ltl::Globally(Rc::new(Ltl::Implies(
            atom("Fault"),
            Rc::new(Ltl::Finally(atom("Idle"))),
        )));
        assert_eq!(
            verdict(
                "start Idle { ref Fault; ref Idle; } state Fault { ref Idle; }",
                &phi
            ),
            Verdict::Holds
        );
    }

    /// `G(Fault -> F Idle)` нарушается: Fault умеет зациклиться на себе.
    #[test]
    fn response_property_violated_when_fault_can_loop_forever() {
        let phi = Ltl::Globally(Rc::new(Ltl::Implies(
            atom("Fault"),
            Rc::new(Ltl::Finally(atom("Idle"))),
        )));
        let v = verdict(
            "start Idle { ref Fault; } state Fault { ref Fault; ref Idle; }",
            &phi,
        );
        let Verdict::Violated(cex) = v else {
            panic!("Fault может крутиться вечно: ожидалось нарушение, получено {v:?}");
        };
        // Цикл — вечное залипание в Fault. Длина цикла в тактах не
        // фиксируется: она зависит от числа шагов автомата `!φ` на витке
        // (здесь — два состояния произведения, оба над вершиной Fault).
        assert!(
            cex.cycle.iter().all(|s| s == "Fault"),
            "контрпример — вечное залипание в Fault, без возврата в Idle: {}",
            cex.trace()
        );
        assert_eq!(
            cex.prefix.first().map(String::as_str),
            Some("Idle"),
            "прогон начинается со стартового состояния: {}",
            cex.trace()
        );
    }

    /// Тупик получает самопетлю, поэтому `G F Done` на конечном состоянии
    /// держится (A2: конечный прогон продолжается «стоянием» в Done).
    #[test]
    fn terminal_state_stutters_forever() {
        let phi = Ltl::Globally(Rc::new(Ltl::Finally(atom("Done"))));
        assert_eq!(
            verdict("start A { ref Done; } state Done;", &phi),
            Verdict::Holds
        );
    }

    // ─── Честная граница (критерий A4) ───────────────────────────────────────

    /// Атом-переменная не проверяется молча — Unsupported с именем атома.
    #[test]
    fn data_atom_is_unsupported_not_false() {
        let v = verdict(
            "var temp: u8 := 0; start A { ref A; }",
            &Ltl::Globally(atom("temp")),
        );
        assert_eq!(v, Verdict::Unsupported(vec!["temp".to_string()]));
    }

    /// Неизвестный атом (опечатка в имени состояния) — тоже Unsupported.
    #[test]
    fn unknown_atom_is_unsupported() {
        let v = verdict("start A { ref A; }", &Ltl::Finally(atom("Dane")));
        assert_eq!(v, Verdict::Unsupported(vec!["Dane".to_string()]));
    }

    /// Смешанная формула: известный атом не спасает — неизвестный перечисляется.
    #[test]
    fn unsupported_lists_only_unknown_atoms() {
        let phi = Ltl::And(atom("A"), atom("temp"));
        let v = verdict("var temp: u8 := 0; start A { ref A; }", &phi);
        assert_eq!(v, Verdict::Unsupported(vec!["temp".to_string()]));
    }

    // ─── Прочее ──────────────────────────────────────────────────────────────

    /// Тавтология без атомов проверяется (атомов нет — связывать нечего).
    #[test]
    fn atomless_formula_is_verifiable() {
        assert_eq!(
            verdict("start A { ref A; }", &Ltl::Globally(Rc::new(Ltl::True))),
            Verdict::Holds
        );
    }

    /// Детерминизм вердикта (критерий A6, гейт 0048): 10 прогонов — один ответ.
    #[test]
    fn verdict_is_deterministic() {
        let src = "start Idle { ref Fault; } state Fault { ref Fault; ref Idle; }";
        let phi = Ltl::Globally(Rc::new(Ltl::Implies(
            atom("Fault"),
            Rc::new(Ltl::Finally(atom("Idle"))),
        )));
        let first = verdict(src, &phi);
        for _ in 0..10 {
            assert_eq!(verdict(src, &phi), first);
        }
    }

    // ─── Читаемость контрпримера (запись слова, а не само слово) ─────────────

    /// Повтор вершины на витке сжимается: `[F -> F]*` и `[F]*` — одно слово.
    #[test]
    fn repeated_cycle_is_compressed_to_its_period() {
        let cex = counterexample(
            vec!["Idle".to_string(), "Fault".to_string()],
            vec!["Fault".to_string(), "Fault".to_string()],
        );
        assert_eq!(cex.trace(), "Idle -> [ Fault ]*");
    }

    /// Цикл без периода не трогается: `A W · (W K)^ω ≠ A · (W K)^ω`.
    #[test]
    fn non_periodic_cycle_is_left_intact() {
        let cex = counterexample(
            vec!["Start".to_string(), "Work".to_string()],
            vec!["Work".to_string(), "Wait".to_string()],
        );
        assert_eq!(cex.trace(), "Start -> [ Work -> Wait ]*");
    }

    /// Период вычисляется по всей длине витка, а не по соседним повторам.
    #[test]
    fn minimal_period_of_repeated_pair() {
        let names = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            minimal_period(&names(&["W", "K", "W", "K"])),
            names(&["W", "K"])
        );
        assert_eq!(
            minimal_period(&names(&["W", "K", "W"])),
            names(&["W", "K", "W"])
        );
        assert_eq!(minimal_period(&names(&["F"])), names(&["F"]));
    }

    /// Трасса отладки содержит все три звена конвейера.
    #[test]
    fn trace_dumps_the_whole_pipeline() {
        let (ast, _) = parse("start A { ref Done; } state Done;", 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let (verdict, trace) = verify_model_traced(&model.borrow(), &Ltl::Finally(atom("Done")));
        assert_eq!(verdict, Verdict::Holds);
        assert!(trace.contains("Структура Крипке"), "{trace}");
        assert!(trace.contains("Автомат Бюхи"), "{trace}");
        assert!(trace.contains("Произведение"), "{trace}");
    }
}
