//! Разбор объявления значения модели: `var`, порт, `const`, `parameter`.
//!
//! Вынесен из `semantic/tree.rs` фичей 0185: файл давно сверх лимита размера
//! (`scripts/check-module-size.sh`), а «построить узел объявления» —
//! самостоятельная ответственность, отделимая от обхода элементов модели.

use crate::diagnostics::{Diagnostic, Location};
use crate::parser::ast::{Identifier, VariableDefine};
use crate::semantic::type_node::{TypeNode, construct_type};
use crate::semantic::{
    ExpressionNode, ModelNode, ParameterNode, PortDirection, VariableNode, const_eval,
};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// Строит узел объявления и кладёт его в карту переменных модели.
///
/// Параметр (фича 0185) дополнительно попадает в `parameters` — в порядке
/// объявления: по нему строится ключ дедупликации специализаций
/// (`--parameters=specialize`), а детерминизм вывода (0048) требует, чтобы
/// порядок зависел только от входа.
pub(super) fn construct_declaration(
    def: &VariableDefine,
    model_node: Rc<RefCell<ModelNode>>,
    variables: &mut BTreeMap<String, VariableNode>,
    parameters: &mut Vec<ParameterNode>,
) -> Result<(), Diagnostic> {
    // Пока тип определяется только из явной аннотации.
    match def.clone() {
        VariableDefine::Variable {
            loc,
            typ,
            name,
            initializer,
        } => {
            let name = extract_name(name.clone(), loc)?;
            variables.insert(
                name.clone(),
                VariableNode::Simple {
                    upper: Some(Rc::downgrade(&model_node)),
                    loc,
                    name: name.clone(),
                    ty: construct_type(typ, Rc::clone(&model_node))?,
                    expr: initializer
                        .map(ExpressionNode::Unresolved)
                        .unwrap_or(ExpressionNode::None),
                },
            )
        }
        VariableDefine::Port {
            loc,
            typ,
            name,
            address,
            initializer,
            direction,
        } => {
            let name = extract_name(name.clone(), loc)?;
            let type_node = construct_type(typ, Rc::clone(&model_node))?;
            if type_node == TypeNode::Inference {
                return Err(
                    Diagnostic::error(loc, "Порт должен иметь конкретный тип".to_string())
                        .with_code("SE-023"),
                );
            }
            // Два независимых выражения (фича 0187): размещение `at <адрес>` и
            // начальное значение `:=`. Каждое необязательно — адрес может
            // прийти по имени порта (оператор `address`, внешняя карта), и
            // полноту проверяет слой адресов, а не объявление.
            let address_node = address
                .clone()
                .map(ExpressionNode::Unresolved)
                .unwrap_or(ExpressionNode::None);
            let init_node = initializer
                .clone()
                .map(ExpressionNode::Unresolved)
                .unwrap_or(ExpressionNode::None);
            variables.insert(
                name.clone(),
                VariableNode::Port {
                    upper: Some(Rc::downgrade(&model_node)),
                    loc,
                    name: name.clone(),
                    ty: type_node,
                    address: address_node,
                    init: init_node,
                    direction,
                },
            )
        }
        VariableDefine::Constant {
            loc,
            typ,
            name,
            initializer,
        } => {
            let name = extract_name(name.clone(), loc)?;
            variables.insert(
                name.clone(),
                VariableNode::Const {
                    upper: Some(Rc::downgrade(&model_node)),
                    loc,
                    name: name.clone(),
                    ty: construct_type(typ, Rc::clone(&model_node))?,
                    expr: ExpressionNode::Unresolved(initializer),
                },
            )
        }
        // Параметр модели (фича 0185). В дереве он **обычная
        // переменная** с начальным значением: в режиме генерации по
        // умолчанию (`--parameters=assign`) параметр и есть поле
        // экземпляра, поэтому потребитель, ничего не знающий о
        // параметрах, обращается с ним верно. Отличие хранится
        // отдельно — в `ModelNode::parameters` (имя, позиция, порядок).
        VariableDefine::Parameter {
            loc,
            typ,
            name,
            initializer,
        } => {
            let name = extract_name(name.clone(), loc)?;
            // Параметр верхнего уровня файла инстанцировать нечем:
            // анонимный корень в выражении реализации по имени не
            // появляется (ADR 0185, п. 2). Отказ здесь — вместо
            // объявления, которое молча вело бы себя как `var`.
            if model_node.borrow().upper.is_none() && model_node.borrow().name.is_none() {
                return Err(Diagnostic::error(
                    loc,
                    format!(
                        "Параметр '{name}' объявлен вне модели: \
                         верхний уровень файла инстанцировать нечем — \
                         перенесите объявление в модель либо замените на 'var'"
                    ),
                )
                .with_code("SE-075"));
            }
            parameters.push(ParameterNode {
                name: name.clone(),
                loc,
                // «Изменяемый», пока анализ изменяемости (0185-06) не сказал
                // иное: неразмеченный параметр обязан вести себя как переменная.
                mutated: true,
            });
            variables.insert(
                name.clone(),
                VariableNode::Simple {
                    upper: Some(Rc::downgrade(&model_node)),
                    loc,
                    name: name.clone(),
                    ty: construct_type(typ, Rc::clone(&model_node))?,
                    expr: ExpressionNode::Unresolved(initializer),
                },
            )
        }
    };
    Ok(())
}

/// Разбирает объявление **внутри блока оператора**: имя, тип, инициализатор.
///
/// Отличается от [`construct_declaration`] тем, что локальное объявление не
/// становится членом модели: узел строит вызывающий
/// ([`StatementNode::Variable`](crate::semantic::StatementNode::Variable)).
pub(super) fn local_declaration(
    def: &VariableDefine,
    loc: Location,
    model: Rc<RefCell<ModelNode>>,
) -> Result<(String, TypeNode, Option<crate::parser::ast::Expression>), Diagnostic> {
    let named =
        |name: &Option<Identifier>| name.as_ref().map(|i| i.name.clone()).unwrap_or_default();
    match def {
        VariableDefine::Variable {
            name,
            typ,
            initializer,
            ..
        }
        | VariableDefine::Port {
            name,
            typ,
            initializer,
            ..
        } => Ok((
            named(name),
            construct_type(typ.clone(), model)?,
            initializer.clone(),
        )),
        VariableDefine::Constant {
            name,
            typ,
            initializer,
            ..
        } => Ok((
            named(name),
            construct_type(typ.clone(), model)?,
            Some(initializer.clone()),
        )),
        // Параметр в теле блока грамматикой не порождается
        // (`LocalVariableDefine` слова `parameter` не знает), но ветвь обязана
        // быть: расширив грамматику, разработчик получит здесь явный отказ, а
        // не молчаливое превращение параметра в локальную переменную (0185).
        VariableDefine::Parameter { name, .. } => Err(Diagnostic::error(
            loc,
            format!(
                "Параметр '{}' объявлен внутри блока: параметр задаётся в месте \
                 инстанцирования модели, поэтому объявляется только на уровне модели",
                named(name)
            ),
        )
        .with_code("SE-075")),
    }
}

/// Имя объявления либо отказ: безымянное объявление разбором не отсеивается.
fn extract_name(id: Option<Identifier>, loc: Location) -> Result<String, Diagnostic> {
    match id {
        Some(id) => Ok(id.name.clone()),
        None => {
            Err(Diagnostic::error(loc, "Идентификатор не задан".to_string()).with_code("SE-021"))
        }
    }
}

/// Разрешает выражение объявления, если оно ещё «сырое» (`Unresolved`).
///
/// Вынесено сюда (фича 0187): у порта таких выражений **два** — размещение и
/// начальное значение, — и повтор `match` для каждого раздул бы `tree.rs`, уже
/// стоящий в реестре размера.
pub(crate) fn resolve_declaration_expression(
    expr: ExpressionNode,
    model: &Rc<RefCell<ModelNode>>,
) -> Result<ExpressionNode, Diagnostic> {
    match expr {
        ExpressionNode::Unresolved(raw) => {
            crate::semantic::expression::construct_expression(raw, vec![], Rc::clone(model))
        }
        other => Ok(other),
    }
}

/// Разрешает «сырые» выражения объявления переменной (`Unresolved` → дерево).
///
/// Вынесено из `tree.rs` (фича 0187): у порта выражений **два** — размещение и
/// начальное значение, — и разбор каждого на месте раздул бы файл, стоящий в
/// реестре размера. Логика прежняя: узел без «сырых» выражений возвращается как
/// есть.
pub(crate) fn resolve_variable_expressions(
    var: VariableNode,
    model: &Rc<RefCell<ModelNode>>,
) -> Result<VariableNode, Diagnostic> {
    Ok(match var {
        VariableNode::Simple {
            upper,
            loc,
            name,
            ty,
            expr,
        } => VariableNode::Simple {
            upper,
            loc,
            name,
            ty,
            expr: resolve_declaration_expression(expr, model)?,
        },
        VariableNode::Const {
            upper,
            loc,
            name,
            ty,
            expr,
        } => VariableNode::Const {
            upper,
            loc,
            name,
            ty,
            expr: resolve_declaration_expression(expr, model)?,
        },
        VariableNode::Port {
            upper,
            loc,
            name,
            ty,
            address,
            init,
            direction,
        } => VariableNode::Port {
            upper,
            loc,
            address: resolve_declaration_expression(address, model)?,
            init: resolve_port_init(init, &name, direction, loc, model)?,
            name,
            ty,
            direction,
        },
        VariableNode::Unresolved => VariableNode::Unresolved,
    })
}

/// Разрешает **начальное значение** порта, сворачивая его в литерал
/// (фича 0187, задача 03).
///
/// # Почему литерал, а не выражение
///
/// Значение выставляется **до первого такта**, и выставляют его шесть разных
/// потребителей: `_init` цели `c`, `new()`/`init()` цели `rust`, ветвь сброса
/// `sv`, инициализатор объявления `st`, старт порта в симуляторе. Выражение
/// печатается **в контексте владельца**, а места эмиссии у целей разные: у
/// цели `rust` под-модель конструируется без доступа к HAL, поэтому значения
/// портов всего дерева выставляет корень — и имя, законное в под-модели, там
/// уже не разрешается. Свёртка снимает вопрос целиком: за границей семантики
/// выражения не существует, разойтись целям не по чему (тот же приём, что у
/// константной выдержки `after`, ADR 0143).
///
/// # Что принимается
///
/// Всё, что вычисляет [`const_eval`]: литералы, константы модели (в том числе
/// цепочкой) и арифметика над ними. Прочее — **`SE-094`** с названной причиной:
/// молчаливая потеря значения здесь дороже отказа.
///
/// ⚠️ У **входного** порта значение не сворачивается: его там не бывает вовсе
/// (`SE-092`), и свёртка перехватила бы диагностику, подменив её жалобой на
/// невычислимость.
fn resolve_port_init(
    init: ExpressionNode,
    name: &str,
    direction: PortDirection,
    loc: Location,
    model: &Rc<RefCell<ModelNode>>,
) -> Result<ExpressionNode, Diagnostic> {
    if direction == PortDirection::In {
        return resolve_declaration_expression(init, model);
    }
    let ExpressionNode::Unresolved(raw) = &init else {
        return Ok(init);
    };
    let literal = const_eval::fold_to_literal(raw, model).map_err(|cause| {
        Diagnostic::error(
            loc,
            format!(
                "начальное значение порта '{name}' выставляется до первого такта, \
                 поэтому обязано быть известно при компиляции: {}",
                cause.message
            ),
        )
        .with_code("SE-094")
    })?;
    resolve_declaration_expression(ExpressionNode::Unresolved(literal), model)
}

/// Сворачивает инициализаторы `var`/`const` в литералы — **в порядке текста**
/// (фича 0192, ADR Option D).
///
/// # Зачем
///
/// До фичи одно объявление давало **пять разных** результатов: `var a: u8 :=
/// 1 + 2;` — эталон `0` (вычислитель начальных значений относил арифметику к
/// «не константа», а семантика свёртку не делала), цели `c`/`rust` — `3`, цель
/// `st` теряла инициализатор молча, цель `sv` отказывала. После свёртки в
/// дереве стоит литерал: за границей семантики выражения не существует, и
/// расходиться потребителям **не по чему** — тот же приём, что у начального
/// значения порта (0187) и константной выдержки `after` (0143).
///
/// # Порядок и ссылки на имена
///
/// Имя в позиции инициализатора означает **начальное значение** переменной, а
/// не значение в такте, и ссылаться можно только **назад по тексту**. Поэтому
/// объявления сортируются по позиции в исходнике: `variables` — `BTreeMap`, её
/// обход **алфавитный**, и порядок объявлений в нём не сохранён.
///
/// ⚠️ Послабление «имя переменной вычислимо» живёт **здесь**, в наполнении
/// [`const_eval::Locals`], а не в `resolve_name` общего вычислителя: тот же
/// вычислитель обслуживает выдержку `after` (0143), параметры моделей (0185) и
/// порты (0187), где «значение переменной известно только в такте» — верное
/// правило.
///
/// # Ошибки
///
/// `SE-083` от вычислителя — с названной причиной (ссылка вперёд, цикл, порт,
/// невычислимая форма). Молчаливый ноль здесь дороже отказа: именно он и был
/// дефектом.
pub(crate) fn fold_variable_initializers(
    variables: &BTreeMap<String, VariableNode>,
    raw: &BTreeMap<String, crate::parser::ast::Expression>,
    model: &Rc<RefCell<ModelNode>>,
) -> Result<BTreeMap<String, VariableNode>, Diagnostic> {
    let mut order: Vec<&String> = variables.keys().collect();
    order.sort_by_key(|name| declaration_position(&variables[*name]));

    let mut known = const_eval::Locals::default();
    let mut folded = variables.clone();
    for name in order {
        let loc = match &variables[name] {
            VariableNode::Simple { loc, .. } | VariableNode::Const { loc, .. } => *loc,
            // Порт сворачивается своим путём (0187): у него другое правило и
            // другая диагностика. Прочее значений не несёт.
            VariableNode::Port { .. } | VariableNode::Unresolved => continue,
        };
        let Some(source) = raw.get(name) else {
            continue;
        };
        let Ok(literal) = const_eval::fold_to_literal_in(source, model, &known) else {
            // Невычислимое оставляем как есть: диагностику о нём (если она
            // нужна) поднимает разрешение выражения ниже по конвейеру. Молча
            // подменять значение нулём — то, ради устранения чего фича и
            // заведена, а вот отвергать всякую невычислимую форму значило бы
            // ломать входы, которых нет в корпусе (решение заказчика: Option D
            // расширяет язык, а не ужесточает).
            continue;
        };
        // Значение запоминается ВСЕГДА — в том числе у литерала: на него могут
        // сослаться объявления ниже (`var base := 5; var probe := base + 1;`).
        // ⚠️ Пропустив этот шаг для литералов, ссылку сломаешь молча: проба
        // давала `probe = 0`.
        if let Ok(value) = const_eval::eval(&literal, model, &mut const_eval::Budget::new()) {
            known.declare(name, value);
        }
        // А вот ПОДМЕНЯТЬ литерал нечем и вредно: дробный литерал к этому
        // моменту уже понижен в q-представление (фича 0096), и подстановка
        // «свёрнутого» `0.0` обратно отменила бы понижение. Проба: сверка
        // Q-арифметики с целью `c` показала `model->acc = 0.0;` вместо целого
        // repr — то есть сверка это поймала, а не рассуждение.
        if is_literal(source) {
            continue;
        }
        // Литерал обязан быть РАЗРЕШЁН здесь: свёртка идёт последней, и
        // разрешать `Unresolved` после неё уже некому — потребители получили бы
        // неразрешённый узел, а он для них «не константа» (то есть ноль).
        let resolved = resolve_declaration_expression(ExpressionNode::Unresolved(literal), model)?;
        let slot = folded.get_mut(name).expect("имя взято из этой же карты");
        set_initializer(slot, resolved, loc);
    }
    Ok(folded)
}

/// Литерал ли выражение: сворачивать такое нечего.
///
/// ⚠️ Дробный литерал к моменту свёртки уже понижен в q-представление
/// (фича 0096), поэтому «свёртка» вернула бы его в дробный вид и отменила
/// понижение — сверка Q-арифметики с целью `c` это ловит.
fn is_literal(expr: &crate::parser::ast::Expression) -> bool {
    use crate::parser::ast::Expression as E;
    matches!(
        expr,
        E::Number(..) | E::Bool(..) | E::Rational(..) | E::Duration(..) | E::String(..)
    )
}

/// Позиция объявления в исходнике — ключ сортировки «по тексту».
///
/// Синтезированные узлы (без позиции) идут последними: ссылаться на них
/// инициализатору всё равно нечем.
fn declaration_position(var: &VariableNode) -> (u32, u32) {
    let loc = match var {
        VariableNode::Simple { loc, .. }
        | VariableNode::Const { loc, .. }
        | VariableNode::Port { loc, .. } => *loc,
        VariableNode::Unresolved => Location::Codegen,
    };
    match loc {
        Location::Source(file, start, _) => (file, start),
        _ => (u32::MAX, u32::MAX),
    }
}

/// Заменяет инициализатор узла, сохраняя всё остальное.
fn set_initializer(var: &mut VariableNode, init: ExpressionNode, _loc: Location) {
    match var {
        VariableNode::Simple { expr, .. } | VariableNode::Const { expr, .. } => *expr = init,
        VariableNode::Port { .. } | VariableNode::Unresolved => {}
    }
}

/// Готовит переменные модели: разрешение «сырых» выражений → вывод типов →
/// свёртка инициализаторов в литералы (фича 0192).
///
/// # Порядок обязателен, и он не очевиден
///
/// - **свёртка работает с сырым АСД**, поэтому исходные выражения запоминаются
///   до разрешения (разрешение их заменяет);
/// - **свёртка идёт последней**, уже после вывода типов. Проба показала почему:
///   `var b: bit := false; var a := b;` — если свернуть раньше, вывод типов
///   увидит булев литерал и даст `a` тип `bool` вместо `bit`. Значение при этом
///   верное, а тип — нет.
///
/// Вынесено из `tree.rs` (фича 0192): тот файл стоит в реестре размера
/// (`scripts/module-size-baseline.txt`) и расти не имеет права, а «подготовить
/// объявления» — ответственность этого модуля.
///
/// # Ошибки
///
/// Пробрасывает диагностику построения выражения (имя в инициализаторе не
/// найдено в области видимости), вывода типов и свёртки начального значения
/// порта (`SE-094`).
pub(crate) fn prepare_variables(
    variables: &BTreeMap<String, VariableNode>,
    model: &Rc<RefCell<ModelNode>>,
) -> Result<BTreeMap<String, VariableNode>, Diagnostic> {
    let raw: BTreeMap<String, crate::parser::ast::Expression> = variables
        .iter()
        .filter_map(|(name, var)| match var {
            VariableNode::Simple { expr, .. } | VariableNode::Const { expr, .. } => match expr {
                ExpressionNode::Unresolved(source) => Some((name.clone(), source.clone())),
                _ => None,
            },
            VariableNode::Port { .. } | VariableNode::Unresolved => None,
        })
        .collect();

    let mut resolved = BTreeMap::new();
    for (name, var) in variables {
        resolved.insert(
            name.clone(),
            resolve_variable_expressions(var.clone(), model)?,
        );
    }

    let inferred =
        crate::semantic::type_inference::type_inference(&mut resolved, Rc::clone(model))?;
    fold_variable_initializers(&inferred, &raw, model)
}
