//! Крипке с отслеживанием данных — **абстракция по формуле** (фича 0068,
//! ADR Option D, консервативное ядро).
//!
//! Чистая абстракция управления (0049) абстрагирует **все** данные: атом,
//! не являющийся именем состояния (`cond`, булев `var`), даёт
//! `Verdict::Unsupported`. Здесь атом-предикат получает **вердикт**: вершина
//! Крипке становится парой `(состояние, оценка отслеживаемых переменных)`.
//!
//! # Что «отслеживается»
//!
//! Только переменные, **встречающиеся в φ** (правило 1 ADR): переменные `cond`-
//! атомов и булевы `var`-атомы. Всё прочее — переменные вне φ, входные порты,
//! арифметика над ними — остаётся абстрагированным.
//!
//! # Как эволюционируют данные (консервативное ядро, решение заказчика)
//!
//! **Максимальная сверх-аппроксимация:** отслеживаемая переменная на каждом
//! такте вольна принять **любое** значение своего домена (переход по данным —
//! полностью недетерминированный). Разметка атомов при этом **точна**: в каждой
//! вершине предикат вычисляется по её оценке.
//!
//! Это **сохраняет главную гарантию 0049** (правило 3): реальный прогон
//! присутствует в абстракции с точными метками (полный недетерминизм включает
//! ход реальных данных), поэтому `Holds` надёжен. Цена честности — свойство над
//! меняющимися данными почти всегда даёт **`Violated` (возможно, ложный)**;
//! доказуемы лишь свойства, истинные при **всех** оценках (тавтологии над
//! доменом) и управляющие свойства. Прецизионное вычисление хода данных
//! (доказывающее `G (temp <= 100)`, когда логика ограничивает `temp`) — Option D
//! в полном объёме — сознательно **не** реализовано (тот же класс риска, что
//! стоил проекту года: неверный ход данных сломал бы `Holds`).
//!
//! # Потолок (правило 4)
//!
//! Размер `|состояний| × Π|домен|` считается **до** построения. Превышение
//! [`VERTEX_LIMIT`] → `Unsupported`, а не долгий счёт. `float`/`q` и прочие
//! неперечислимые типы → `Unsupported` (правило 5).

use super::kripke::Kripke;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{ConditionNode, ExpressionNode, ModelNode, VariableNode};
use std::collections::{BTreeMap, BTreeSet};

/// Потолок числа вершин Крипке с данными (правило 4 ADR).
///
/// Подобран так, чтобы заявленные примеры проходили, а взрыв — отсекался:
/// `comprehensive` + `u8` → 5 × 256 = 1280 (проходит); `stacker` + `bit` →
/// 19 × 2 = 38 (проходит); φ над **тремя** `u8` → 5 × 256³ ≈ 8.4 × 10⁷
/// (`Unsupported`). Значение — инженерное, не физическое: увеличивать можно, но
/// вместе с замером времени построения.
pub const VERTEX_LIMIT: u128 = 1_000_000;

/// Строит Крипке с отслеживанием данных для формулы с атомами `atoms`, взяв
/// управляющий граф из уже построенной `control` (0049).
///
/// Вызывается **только** когда среди атомов есть не-имя-состояния (иначе работает
/// путь управления 0049 без изменений — критерий A7). Возвращает:
///
/// - `Ok(kripke)` — построена Крипке `(состояние × оценка)` с точной разметкой;
/// - `Err(atoms)` — отслеживание невозможно (атом не `cond`/булев `var`;
///   предикат вне поддержанного подмножества; порт/`float` в предикате; потолок
///   превышен; инициализатор не сворачивается). Имена — для `Verdict::Unsupported`.
pub fn build_data_kripke(
    model: &ModelNode,
    control: &Kripke,
    atoms: &BTreeSet<String>,
) -> Result<Kripke, Vec<String>> {
    // 1. Разбор атомов: имя состояния | предикат над данными | неизвестный.
    let mut data_atoms: Vec<DataAtom> = Vec::new();
    let mut unsupported: BTreeSet<String> = BTreeSet::new();
    for atom in atoms {
        if control.states.iter().any(|s| s == atom) {
            continue; // атом-имя состояния — размечается именем вершины
        }
        match classify_data_atom(model, atom) {
            Some(da) => data_atoms.push(da),
            None => {
                unsupported.insert(atom.clone());
            }
        }
    }
    // Атом, который не состояние и не поддержанный предикат, — честный отказ
    // (как чистый путь управления на атоме-переменной).
    if !unsupported.is_empty() {
        return Err(unsupported.into_iter().collect());
    }

    // 2. Сбор отслеживаемых переменных + валидация подмножества предикатов.
    let mut tracked: BTreeMap<String, TrackedDecl> = BTreeMap::new();
    for da in &data_atoms {
        if !collect_tracked(model, &da.expr, &mut tracked) {
            // Предикат вне подмножества (арифметика, функция, порт, битдоступ…).
            return Err(vec![da.name.clone()]);
        }
    }

    // 3. Домены из типов + потолок ДО построения (правила 4, 5).
    let mut total = control.states.len() as u128;
    let mut order: Vec<TrackedVar> = Vec::new();
    for (name, decl) in &tracked {
        let Some(size) = domain_size(&decl.ty, model) else {
            // float/q/массив/структура — домен не перечислим.
            return Err(data_atom_names(&data_atoms));
        };
        total = match total.checked_mul(size) {
            Some(t) if t <= VERTEX_LIMIT => t,
            _ => return Err(data_atom_names(&data_atoms)), // потолок/переполнение
        };
        let values = domain_values(&decl.ty, model);
        let Some(init) = fold_expr(&decl.init) else {
            return Err(data_atom_names(&data_atoms)); // инициализатор не сворачивается
        };
        let Some(init_idx) = values.iter().position(|v| *v == init) else {
            return Err(data_atom_names(&data_atoms)); // нач. значение вне домена
        };
        order.push(TrackedVar {
            name: name.clone(),
            values,
            init_idx,
        });
    }

    // 4. Перечисление оценок (одометр) и ТОЧНАЯ разметка предикатов по оценке.
    // Истинность предиката зависит только от оценки, не от состояния, — считаем
    // один раз на оценку.
    let num_val: usize = order
        .iter()
        .map(|v| v.values.len())
        .product::<usize>()
        .max(1);
    let mut data_true: Vec<BTreeSet<String>> = Vec::with_capacity(num_val);
    for j in 0..num_val {
        let valuation = valuation_at(&order, j);
        let mut here = BTreeSet::new();
        for da in &data_atoms {
            match eval_atom(&da.expr, &valuation) {
                Some(true) => {
                    here.insert(da.name.clone());
                }
                Some(false) => {}
                // Валидация п.2 гарантирует тотальность; None — страховка.
                None => return Err(vec![da.name.clone()]),
            }
        }
        data_true.push(here);
    }

    // 5. Полное произведение: вершина = состояние k × оценка j → id = k*num_val+j.
    let num_states = control.states.len();
    let total_vertices = num_states * num_val;
    let mut states: Vec<String> = Vec::with_capacity(total_vertices);
    let mut labels: Vec<BTreeSet<String>> = Vec::with_capacity(total_vertices);
    for state_name in &control.states {
        for dt in &data_true {
            states.push(state_name.clone());
            // Разметка: имя состояния (для атомов-состояний) + истинные предикаты.
            let mut lab = dt.clone();
            lab.insert(state_name.clone());
            labels.push(lab);
        }
    }

    // Переходы: управляющее ребро k→k2 крестится с ЛЮБОЙ оценкой j2 (правило 3:
    // ход данных полностью недетерминирован — сверх-аппроксимация).
    let mut transitions: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for k in 0..num_states {
        let succ_states = control.successors(k);
        for j in 0..num_val {
            let id = k * num_val + j;
            let mut succ = BTreeSet::new();
            for &k2 in succ_states {
                for j2 in 0..num_val {
                    succ.insert(k2 * num_val + j2);
                }
            }
            transitions.insert(id, succ);
        }
    }

    // Начальная вершина: (стартовое состояние, начальная оценка из инициализаторов).
    let init_val = order
        .iter()
        .enumerate()
        .fold(0usize, |acc, (i, v)| acc + v.init_idx * stride(&order, i));
    let initial = control.initial * num_val + init_val;

    Ok(Kripke {
        states,
        initial,
        transitions,
        labels,
    })
}

/// Вид предиката-атома φ.
enum AtomExpr {
    /// Именованное условие (`cond Safe = temp <= 100;`).
    Cond(ConditionNode),
    /// Булев `var`/`const`-атом: истинен ⟺ значение переменной ≠ 0.
    BoolVar(String),
}

/// Предикат-атом φ: имя атома и его вид.
struct DataAtom {
    name: String,
    expr: AtomExpr,
}

/// Объявление отслеживаемой переменной: тип и инициализатор.
struct TrackedDecl {
    ty: TypeNode,
    init: ExpressionNode,
}

/// Отслеживаемая переменная с материализованным доменом и индексом нач. значения.
struct TrackedVar {
    name: String,
    values: Vec<i64>,
    init_idx: usize,
}

fn data_atom_names(atoms: &[DataAtom]) -> Vec<String> {
    atoms.iter().map(|a| a.name.clone()).collect()
}

/// Классифицирует не-состояние-атом: `cond` или булев `var`. Иначе `None`.
fn classify_data_atom(model: &ModelNode, atom: &str) -> Option<DataAtom> {
    if let Some(def) = model.conditions.get(atom) {
        return Some(DataAtom {
            name: atom.to_string(),
            expr: AtomExpr::Cond(def.value.clone()),
        });
    }
    // Булев `var`/`const`: атом истинен ⟺ значение ≠ 0.
    if let Some(var) = model.variables.get(atom) {
        let ty = match var {
            VariableNode::Simple { ty, .. } | VariableNode::Const { ty, .. } => Some(ty),
            _ => None,
        };
        if matches!(ty, Some(TypeNode::Bit | TypeNode::Bool)) {
            return Some(DataAtom {
                name: atom.to_string(),
                expr: AtomExpr::BoolVar(atom.to_string()),
            });
        }
    }
    None
}

/// Собирает отслеживаемые (`Simple`) переменные атома и проверяет, что он
/// **целиком** в поддержанном подмножестве. `false` → атом не поддержан.
fn collect_tracked(
    model: &ModelNode,
    expr: &AtomExpr,
    tracked: &mut BTreeMap<String, TrackedDecl>,
) -> bool {
    match expr {
        AtomExpr::BoolVar(name) => match model.variables.get(name) {
            Some(VariableNode::Simple { ty, expr, .. }) => {
                tracked.entry(name.clone()).or_insert_with(|| TrackedDecl {
                    ty: ty.clone(),
                    init: expr.clone(),
                });
                true
            }
            // Булева `const` — не отслеживается, но обязана сворачиваться.
            Some(VariableNode::Const { expr, .. }) => fold_expr(expr).is_some(),
            _ => false,
        },
        AtomExpr::Cond(cond) => collect_tracked_cond(cond, tracked),
    }
}

/// Обход условия: сбор `Simple`-переменных + проверка поддержанности подмножества.
fn collect_tracked_cond(cond: &ConditionNode, tracked: &mut BTreeMap<String, TrackedDecl>) -> bool {
    match cond {
        ConditionNode::Bool(_) | ConditionNode::Number(_) | ConditionNode::EnumVariant(..) => true,
        ConditionNode::Parenthesis(c) | ConditionNode::Not(c) => collect_tracked_cond(c, tracked),
        ConditionNode::And(l, r)
        | ConditionNode::Or(l, r)
        | ConditionNode::Less(l, r)
        | ConditionNode::More(l, r)
        | ConditionNode::LessEqual(l, r)
        | ConditionNode::MoreEqual(l, r)
        | ConditionNode::Equal(l, r)
        | ConditionNode::NotEqual(l, r) => {
            collect_tracked_cond(l, tracked) && collect_tracked_cond(r, tracked)
        }
        ConditionNode::Variable(rc, _) => {
            let borrowed = rc.borrow();
            match &*borrowed {
                VariableNode::Simple { name, ty, expr, .. } => {
                    if !is_trackable_type(ty) {
                        return false;
                    }
                    tracked.entry(name.clone()).or_insert_with(|| TrackedDecl {
                        ty: ty.clone(),
                        init: expr.clone(),
                    });
                    true
                }
                // Константа — не отслеживается, но обязана сворачиваться.
                VariableNode::Const { expr, .. } => fold_expr(expr).is_some(),
                // Порт — среда, не данные состояния.
                _ => false,
            }
        }
        // Арифметика, функции, битдоступ, срезы, Unresolved — вне подмножества.
        _ => false,
    }
}

/// Перечислим ли тип для отслеживания (домен конечен и практически мал).
fn is_trackable_type(ty: &TypeNode) -> bool {
    matches!(
        ty,
        TypeNode::Bit | TypeNode::Bool | TypeNode::Integer { .. } | TypeNode::Enum(_)
    )
}

/// Размер домена типа (число значений); `None` — не перечислим (правило 5).
fn domain_size(ty: &TypeNode, model: &ModelNode) -> Option<u128> {
    match ty {
        TypeNode::Bit | TypeNode::Bool => Some(2),
        TypeNode::Integer { bits, .. } => Some(1u128 << *bits),
        TypeNode::Enum(name) => model
            .enums
            .get(name)
            .map(|e| distinct_variant_values(e).len() as u128),
        _ => None,
    }
}

/// Материализует значения домена (только после прохождения потолка).
fn domain_values(ty: &TypeNode, model: &ModelNode) -> Vec<i64> {
    match ty {
        TypeNode::Bit | TypeNode::Bool => vec![0, 1],
        TypeNode::Integer { bits, signed } => {
            let n = 1i128 << *bits;
            if *signed {
                let half = n / 2;
                (-half..half).map(|v| v as i64).collect()
            } else {
                (0..n).map(|v| v as i64).collect()
            }
        }
        TypeNode::Enum(name) => model
            .enums
            .get(name)
            .map(distinct_variant_values)
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Различные значения вариантов перечисления, по возрастанию (детерминизм 0048).
fn distinct_variant_values(e: &crate::semantic::EnumDefinitionNode) -> Vec<i64> {
    let set: BTreeSet<i64> = e.variants.iter().map(|(_, v)| *v).collect();
    set.into_iter().collect()
}

/// Оценка `j` как отображение имя → значение (одометр по [`TrackedVar`]).
fn valuation_at(order: &[TrackedVar], j: usize) -> BTreeMap<String, i64> {
    let mut out = BTreeMap::new();
    for (i, v) in order.iter().enumerate() {
        let idx = (j / stride(order, i)) % v.values.len();
        out.insert(v.name.clone(), v.values[idx]);
    }
    out
}

/// Шаг одометра для переменной `i` (произведение размеров правее неё).
fn stride(order: &[TrackedVar], i: usize) -> usize {
    order[i + 1..]
        .iter()
        .map(|v| v.values.len())
        .product::<usize>()
        .max(1)
}

/// Свёртка выражения-инициализатора/константы в целое. `None` — не константа.
fn fold_expr(expr: &ExpressionNode) -> Option<i64> {
    match expr {
        ExpressionNode::Number(n) => Some(*n),
        ExpressionNode::Bool(b) => Some(*b as i64),
        ExpressionNode::Parenthesis(inner) | ExpressionNode::UnaryPlus(inner) => fold_expr(inner),
        ExpressionNode::Negate(inner) => fold_expr(inner).map(|v| v.wrapping_neg()),
        _ => None,
    }
}

/// Булева оценка атома по конкретной оценке переменных.
fn eval_atom(expr: &AtomExpr, val: &BTreeMap<String, i64>) -> Option<bool> {
    match expr {
        AtomExpr::BoolVar(name) => val.get(name).map(|v| *v != 0),
        AtomExpr::Cond(cond) => eval_bool(cond, val),
    }
}

/// Булева оценка условия по конкретной оценке переменных. `None` — предикат вне
/// поддержанного подмножества (страховка; валидация п.2 это исключает).
fn eval_bool(cond: &ConditionNode, val: &BTreeMap<String, i64>) -> Option<bool> {
    match cond {
        ConditionNode::Bool(b) => Some(*b),
        ConditionNode::Not(c) => eval_bool(c, val).map(|x| !x),
        ConditionNode::Parenthesis(c) => eval_bool(c, val),
        // `&`/`|` над булевыми операндами (0/1) — побитовое совпадает с логическим.
        ConditionNode::And(l, r) => Some(eval_bool(l, val)? && eval_bool(r, val)?),
        ConditionNode::Or(l, r) => Some(eval_bool(l, val)? || eval_bool(r, val)?),
        ConditionNode::Less(l, r) => Some(eval_num(l, val)? < eval_num(r, val)?),
        ConditionNode::More(l, r) => Some(eval_num(l, val)? > eval_num(r, val)?),
        ConditionNode::LessEqual(l, r) => Some(eval_num(l, val)? <= eval_num(r, val)?),
        ConditionNode::MoreEqual(l, r) => Some(eval_num(l, val)? >= eval_num(r, val)?),
        ConditionNode::Equal(l, r) => Some(eval_num(l, val)? == eval_num(r, val)?),
        ConditionNode::NotEqual(l, r) => Some(eval_num(l, val)? != eval_num(r, val)?),
        // Булева переменная как самостоятельный операнд: != 0.
        ConditionNode::Variable(rc, _) => {
            let borrowed = rc.borrow();
            match &*borrowed {
                VariableNode::Simple { name, .. } => val.get(name).map(|v| *v != 0),
                VariableNode::Const { expr, .. } => fold_expr(expr).map(|v| v != 0),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Числовая оценка терма по оценке переменных. `None` — вне подмножества.
fn eval_num(cond: &ConditionNode, val: &BTreeMap<String, i64>) -> Option<i64> {
    match cond {
        ConditionNode::Number(n) => Some(*n),
        ConditionNode::Bool(b) => Some(*b as i64),
        ConditionNode::EnumVariant(_, _, v) => Some(*v),
        ConditionNode::Parenthesis(c) => eval_num(c, val),
        ConditionNode::Variable(rc, _) => {
            let borrowed = rc.borrow();
            match &*borrowed {
                VariableNode::Simple { name, .. } => val.get(name).copied(),
                VariableNode::Const { expr, .. } => fold_expr(expr),
                _ => None,
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::ltl_check::collect_atoms;
    use crate::semantic::tree::construct_model;
    use crate::verification::verify::{Verdict, verify_model};
    use crate::{parse, parse_ltl_property};

    /// Строит Крипке данных для φ над моделью `src` (или `Err`, если не вышло).
    fn data_kripke_of(src: &str, phi_src: &str) -> Result<Kripke, Vec<String>> {
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let m = model.borrow();
        let control = super::super::kripke::build_kripke(&m).unwrap();
        let phi = parse_ltl_property(phi_src).unwrap();
        let mut atoms = BTreeSet::new();
        collect_atoms(&phi, &mut atoms);
        build_data_kripke(&m, &control, &atoms)
    }

    fn verdict(src: &str, phi_src: &str) -> Verdict {
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let phi = parse_ltl_property(phi_src).unwrap();
        verify_model(&model.borrow(), &phi)
    }

    /// A1: предикат над `u8` даёт **вершины** (не Unsupported); размер — |состояний| × 256.
    #[test]
    fn u8_predicate_yields_state_times_256_vertices() {
        // 2 состояния, один отслеживаемый `u8` → 2 × 256 = 512 вершин.
        let src = "var temp: u8 := 0; \
                   cond Hot = temp >= 100; \
                   start A { ref B; } state B { ref A; }";
        let k = data_kripke_of(src, "G !Hot").expect("предикат над u8 отслеживается");
        assert_eq!(k.states.len(), 2 * 256, "вершин = состояний × домен(u8)");
        assert!(!k.labels.is_empty(), "путь данных: разметка непуста");
    }

    /// A1: свойство над данными получает **вердикт**, а не `Unsupported`.
    #[test]
    fn data_property_gets_a_verdict_not_unsupported() {
        let src = "var temp: u8 := 0; \
                   cond Hot = temp >= 100; \
                   start A { ref B; } state B { ref A; }";
        let v = verdict(src, "G !Hot");
        assert!(
            !matches!(v, Verdict::Unsupported(_)),
            "предикат над данными обязан дать вердикт, получено {v:?}"
        );
    }

    /// A4 (ГЛАВНАЯ): гарантия «держится» сохранена. Тавтология над доменом `u8`
    /// (`temp <= 255` истинно при **всех** 256 значениях) → `Holds`, несмотря на
    /// полный недетерминизм данных. Тест **на вердикт**, а не на форму.
    #[test]
    fn tautology_over_domain_holds() {
        let src = "var temp: u8 := 0; \
                   cond InRange = temp <= 255; \
                   start A { ref A; }";
        assert_eq!(
            verdict(src, "G InRange"),
            Verdict::Holds,
            "temp<=255 истинно при всех значениях u8 — свойство держится"
        );
    }

    /// A4: булева переменная, которую никто не меняет? — нет, данные роумят.
    /// Но `G (Safe | !Safe)` — тавтология — держится (сохранность вердикта).
    #[test]
    fn excluded_middle_holds() {
        let src = "var temp: u8 := 5; \
                   cond Hi = temp >= 200; \
                   start A { ref A; }";
        assert_eq!(verdict(src, "G (Hi | !Hi)"), Verdict::Holds);
    }

    /// A4: свойство, ложное при некоторой достижимой оценке, → `Violated`
    /// (возможно, ложное — цена сверх-аппроксимации). Здесь `temp` роумит по
    /// всему домену, значит `temp >= 200` достижимо.
    #[test]
    fn reachable_false_valuation_violates() {
        let src = "var temp: u8 := 0; \
                   cond Hi = temp >= 200; \
                   start A { ref A; }";
        let v = verdict(src, "G !Hi");
        assert!(
            matches!(v, Verdict::Violated(_)),
            "temp вольна достичь 200 (недетерминизм данных) → нарушение, получено {v:?}"
        );
    }

    /// A5 (направление ошибки): переход/значение из абстрагированного источника
    /// даёт **лишний** прогон, а не пропущенный. Модель, где `temp` меняется
    /// только по входу (мы его не отслеживаем), обязана допускать оценку, при
    /// которой предикат ложен → `Violated`. Мутация правила 3 («невычислимое —
    /// ложно/заморожено») сделала бы `temp` неизменным (=0) и дала бы ложный
    /// `Holds` — то есть тест валился бы при мутации.
    #[test]
    fn abstracted_source_yields_extra_run_not_missed() {
        // temp инициализирована 0; при «заморозке» temp=0 всегда, `!Hot` держится
        // ложно. При верном направлении ошибки temp роумит → Hot достижим → Violated.
        let src = "in sensor: u8; \
                   var temp: u8 := 0; \
                   cond Hot = temp >= 100; \
                   start A { ref A; }";
        let v = verdict(src, "G !Hot");
        assert!(
            matches!(v, Verdict::Violated(_)),
            "лишний прогон (temp достигает 100) обязан быть, получено {v:?}"
        );
    }

    /// A2/потолок: φ над тремя `u8` (2 × 256³ ≈ 3.3 × 10⁷ > потолка) → `Unsupported`
    /// **сразу** (потолок считается до построения).
    #[test]
    fn three_u8_exceeds_ceiling_unsupported() {
        let src = "var a: u8 := 0; var b: u8 := 0; var c: u8 := 0; \
                   cond P = a <= b & b <= c; \
                   start A { ref A; }";
        let v = verdict(src, "G P");
        assert!(
            matches!(v, Verdict::Unsupported(_)),
            "три u8 превышают потолок — ожидался Unsupported, получено {v:?}"
        );
    }

    /// A3: `float` в предикате → `Unsupported` (домен не перечислим, правило 5).
    #[test]
    fn float_predicate_is_unsupported() {
        let src = "var x: float := 0.0; \
                   cond P = x <= 1.0; \
                   start A { ref A; }";
        let v = verdict(src, "G P");
        assert!(
            matches!(v, Verdict::Unsupported(_)),
            "float не перечислим — ожидался Unsupported, получено {v:?}"
        );
    }

    /// A6: `bit`-переменная-атом → домен 2; |состояний| × 2 вершин.
    #[test]
    fn bit_variable_atom_domain_is_two() {
        // 3 состояния, один `bit` → 3 × 2 = 6 вершин.
        let src = "var flag: bit := false; \
                   start A { ref B; } state B { ref C; } state C { ref A; }";
        let k = data_kripke_of(src, "G flag").expect("булев var-атом отслеживается");
        assert_eq!(k.states.len(), 3 * 2, "вершин = состояний × домен(bit)");
    }

    /// Неизвестный атом (не состояние, не cond, не булев var) → `Unsupported`
    /// с его именем — как в чистом пути управления.
    #[test]
    fn unknown_atom_is_unsupported() {
        let src = "start A { ref A; }";
        let v = verdict(src, "G nosuch");
        assert!(matches!(v, Verdict::Unsupported(names) if names == vec!["nosuch".to_string()]));
    }

    /// Начальная вершина несёт оценку инициализаторов: `temp := 7` → в стартовой
    /// вершине `temp = 7`, поэтому `Hot7` (temp = 7) там истинно.
    #[test]
    fn initial_vertex_carries_initializer_valuation() {
        let src = "var temp: u8 := 7; \
                   cond Is7 = temp = 7; \
                   start A { ref A; }";
        let k = data_kripke_of(src, "G Is7").expect("отслеживается");
        assert!(
            k.labels[k.initial].contains("Is7"),
            "в стартовой вершине temp=7, значит Is7 истинно: {:?}",
            k.labels[k.initial]
        );
    }

    /// A1 на корпусном примере: `cond Overheated = temperature >= MAX_TEMP`
    /// (u8 + const) над моделью `Controller` даёт **вершины** = состояний × 256,
    /// а не `Unsupported`. Реально у `Controller` **4** состояния → 1024 вершины
    /// (ADR прикидывал 5 × 256 = 1280 — оценка, а не замер).
    #[test]
    fn comprehensive_controller_u8_predicate_is_tracked() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/comprehensive.takt"
        );
        let src = std::fs::read_to_string(path).expect("examples/comprehensive.takt");
        let (ast, _) = parse(&src, 0).unwrap();
        let root = construct_model(&ast, None, &[]).unwrap();
        let controller = root.borrow().models.get("Controller").unwrap().clone();
        let c = controller.borrow();
        let control = super::super::kripke::build_kripke(&c).unwrap();
        let n_states = control.states.len();
        let phi = parse_ltl_property("G !Overheated").unwrap();
        let mut atoms = BTreeSet::new();
        collect_atoms(&phi, &mut atoms);
        let k = build_data_kripke(&c, &control, &atoms)
            .expect("предикат над u8 корпусной модели отслеживается");
        assert_eq!(
            k.states.len(),
            n_states * 256,
            "вершин = состояний Controller ({n_states}) × домен(u8)"
        );
    }
}
