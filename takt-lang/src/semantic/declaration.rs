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
    untyped: &std::collections::BTreeSet<String>,
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
        // Ссылка ВПЕРЁД на переменную — ошибка (фича 0246). Проверка стоит до
        // общего «невычислимое оставляем как есть»: иначе она растворяется в
        // нём, и запись даёт РАЗНЫЕ значения у эталона (0) и в прошивке (1,
        // потому что цель `c` печатает присваивания в порядке объявления, а
        // поле к этому моменту обнулено). Пять потребителей отвечали
        // по-разному — замер ADR 0246.
        if let Some(diagnostic) = forward_reference(source, name, variables) {
            return Err(diagnostic);
        }
        let Ok(literal) = const_eval::fold_to_literal_in(source, model, &known) else {
            // Дробное объявление, инициализатор которого не свернулся, —
            // ошибка `SE-114` (фича 0300). Здесь молчание стоит дороже всего:
            // эталон оставляет ноль, а цель печатает выражение и считает его
            // сама, то есть прогон и прошивка расходятся БЕЗ единого слова.
            // Точную арифметику свёртка уже выполнила (`decimal.rs`); сюда
            // доходит лишь то, что требует ОКРУГЛЕНИЯ, а оно задано эталоном.
            if let Some(diagnostic) =
                unfoldable_fractional(source, name, declared_type(&variables[name]), loc)
            {
                return Err(diagnostic);
            }
            // Прочее невычислимое оставляем как есть: диагностику о нём (если
            // она нужна) поднимает разрешение выражения ниже по конвейеру.
            // Отвергать всякую невычислимую форму значило бы ломать входы,
            // которых нет в корпусе (решение заказчика: Option D расширяет
            // язык, а не ужесточает).
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
        // Вычисленное значение НОРМИРУЕТСЯ по типу объявления (фича 0207,
        // решение заказчика 2026-08-16). Литерал автора сюда не доходит — он
        // отсечён строкой выше, и его выход за границы по-прежнему `SE-089`.
        // Ширина ВЫВЕДЕННОГО типа берётся у РЕЗУЛЬТАТА, а не у операндов
        // (фича 0285). Тип выводится до свёртки, поэтому `const K := 1 + 255;`
        // получал ширину левого литерала (8 бит), а нормирование 0207
        // заворачивало вычисленные 256 в ноль — молча и одинаково у всех
        // девяти потребителей. Автор ширины не выбирал: её выбрал операнд.
        //
        // ⚠️ Явно объявленный тип НЕ трогается: `var u: u8 := 200 + 100;`
        // обязан остаться `44` — там ширину выбрал автор, и обёртка совпадает
        // с тем, что даёт то же выражение в теле (правило 0207/0127).
        //
        // ⚠️ Расширяем ТОЛЬКО когда значение не помещается в выведенный тип, а
        // не при каждой свёртке. Первая редакция переопределяла тип всегда — и
        // сломала вывод из сигнатуры функции: `fn get32() -> [bit;32]` с
        // `var val := get32();` давал `[bit;8]`, потому что вычислитель
        // сворачивает вызов в `0`. Там ширину выбрал не операнд, а объявленный
        // возвращаемый тип, и трогать её нельзя. Поймал тест Ce6, а не чтение.
        //
        // ⚠️ Расширяется только превышение ВЕРХНЕЙ границы. Значение ниже
        // нижней — домен правила 0207: `var u := ~0;` даёт `-1`, и беззнаковый
        // тип обязан завернуть его в `255`, а не «расшириться» до знакового.
        // Второй перебор, пойманный тестом нормирования, а не чтением.
        if untyped.contains(name)
            && let crate::parser::ast::Expression::Number(_, value) = &literal
            && exceeds_declared_upper(*value, declared_type(&folded[name]))
        {
            let widened = crate::semantic::type_inference::infer_int_type(*value);
            retype_declaration(folded.get_mut(name).expect("имя из этой же карты"), widened);
        }
        let literal = normalize_computed(literal, declared_type(&folded[name]));
        // Дробный результат, свёрнутый над `q(m, n)`, обязан быть ПОНИЖЕН в
        // целое q-представление (фича 0300). Понижение литералов (0096/0061)
        // идёт при выводе типов, то есть ДО свёртки, и `1.0 + 2.0` ему не
        // литерал — оно проходит мимо. Без этого шага цель `c` печатала
        // `model->s = 3.0;` в поле `int8_t`, то есть **3**, что в q(4, 4)
        // значит 0.1875, тогда как эталон давал 3.0. Понижение делает тот же
        // носитель, что и для написанного автором литерала: своя копия
        // разошлась бы с ним в округлении.
        let literal = lower_folded_fixed(literal, declared_type(&folded[name]))?;
        // Литерал обязан быть РАЗРЕШЁН здесь: свёртка идёт последней, и
        // разрешать `Unresolved` после неё уже некому — потребители получили бы
        // неразрешённый узел, а он для них «не константа» (то есть ноль).
        let resolved = resolve_declaration_expression(ExpressionNode::Unresolved(literal), model)?;
        let slot = folded.get_mut(name).expect("имя взято из этой же карты");
        set_initializer(slot, resolved, loc);
    }
    Ok(folded)
}

/// Превышает ли вычисленное значение ВЕРХНЮЮ границу выведенного типа (0285).
///
/// Границы берутся у **единственного** их носителя
/// (`validate::literal_range::type_range`): своя копия разошлась бы с
/// проверкой `SE-089`, и расширение шло бы к одним границам, а отказ судил по
/// другим. Тип без границ (например `bit`) расширению не подлежит — его
/// значения судит своя проверка.
///
/// ⚠️ Нижняя граница НЕ проверяется намеренно: значение ниже неё — домен
/// правила 0207 (беззнаковое заворачивается `mod 2ⁿ`, знаковое остаётся
/// ошибкой), и «расширение» подменило бы там принятое решение.
fn exceeds_declared_upper(value: i128, ty: Option<&crate::semantic::type_node::TypeNode>) -> bool {
    match ty.and_then(crate::semantic::validate::literal_range::type_range) {
        Some((_, hi)) => value > hi,
        None => false,
    }
}

/// Задаёт тип объявления — для переменных, объявленных БЕЗ типа (фича 0285).
///
/// ⚠️ Меняются **оба** представления? Нет: здесь правится только запись в карте
/// объявлений. Ячейка тела (`Rc<RefCell<VariableNode>>`, засада 0096) к этому
/// моменту ещё не построена — тела разрешаются позже, — поэтому второго
/// представления не существует и синхронизировать нечего.
fn retype_declaration(var: &mut VariableNode, ty: crate::semantic::type_node::TypeNode) {
    match var {
        VariableNode::Simple { ty: slot, .. } | VariableNode::Const { ty: slot, .. } => *slot = ty,
        VariableNode::Port { .. } | VariableNode::Unresolved => {}
    }
}

/// Несвёрнутая дробная АРИФМЕТИКА в инициализаторе — `SE-114` (фича 0300).
///
/// # Почему именно дробное и почему ошибка
///
/// Замер ADR 0300: `var d: float := 1.0 / 3.0;` даёт `0.0` у эталона и
/// `model->d = 1.0 / 3.0;` (то есть `0.333…`) у цели `c` — молча. Общего
/// поведения у такой записи не существует: эталон вычисляет дробное **своими**
/// правилами округления, а свернуть их при компиляции значит завести второй
/// источник истины (довод фичи 0185, он же в `const_eval`).
///
/// Точная арифметика (`+`, `-`, `*` над десятичными литералами, в том числе
/// смешанная с целым) сюда не доходит — её свернул `const_eval::decimal`.
/// Остаётся то, что требует **округления**: прежде всего деление.
///
/// ⚠️ **Отказ узкий, и это замер, а не осторожность.** Отвергается только
/// **арифметика**: формы, которые эталон вычислять умеет, обязаны остаться
/// законными. Приведение `as` — ровно такая форма (фича 0205 научила ей второй
/// вычислитель), и первая, слишком широкая редакция проверки отвергла
/// `var v := 3 as q(4, 4);` — вход, на котором эталон и цель **уже согласны**
/// (48 у обоих). Поймали это сторожа 0205, а не чтение.
///
/// ⚠️ Целых это не касается: их свёртка (0192) точна всегда, а невычислимое
/// целое остаётся законным — сужать язык там повода нет.
///
/// ⚠️ **Граница:** невычислимая НЕарифметическая форма (вызов функции в
/// дробном инициализаторе) остаётся с прежним поведением — она вне объёма
/// фичи и вынесена кандидатом.
fn unfoldable_fractional(
    source: &crate::parser::ast::Expression,
    name: &str,
    ty: Option<&crate::semantic::type_node::TypeNode>,
    loc: Location,
) -> Option<Diagnostic> {
    use crate::semantic::type_node::TypeNode;
    if is_literal(source) {
        return None;
    }
    if !matches!(ty, Some(TypeNode::Rational | TypeNode::Fixed { .. })) {
        return None;
    }
    if !contains_arithmetic(source) {
        return None;
    }
    Some(
        Diagnostic::error(
            loc,
            format!(
                "инициализатор '{name}' — дробное выражение, которое компилятор не может \
                 вычислить точно: округление дробных задано эталоном симулятора, и \
                 посчитав здесь, компилятор дал бы значение, которого симулятор не \
                 вычислит (прогон показал бы ноль, а прошивка — своё число, и молча). \
                 Задайте готовый литерал — например 'var {name}: … := 0.333;' — либо \
                 вычисляйте в теле состояния: 'always {{ {name} := …; }}'"
            ),
        )
        .with_code("SE-114"),
    )
}

/// Есть ли в выражении бинарная арифметика.
///
/// Обход идёт общим разбором [`ast::Expression::components`], поэтому новый узел
/// АСД сам собой попадает под спуск, а список арифметических форм остаётся
/// коротким и явным.
fn contains_arithmetic(expr: &crate::parser::ast::Expression) -> bool {
    use crate::parser::ast::Expression as E;
    if matches!(
        expr,
        E::Power(..)
            | E::Multiply(..)
            | E::Divide(..)
            | E::Modulo(..)
            | E::Add(..)
            | E::Subtract(..)
    ) {
        return true;
    }
    let (left, right) = expr.components();
    left.is_some_and(contains_arithmetic) || right.is_some_and(contains_arithmetic)
}

/// Понижает **свёрнутый** дробный литерал в целое q-представление (фича 0300).
///
/// Возвращает выражение как есть, если тип объявления не `q(m, n)` либо
/// литерал не дробный: понижать нечего.
///
/// ⚠️ Зовёт `lower_fixed_literal` — **тот же** носитель округления, которым
/// понижается литерал, написанный автором. Своя копия разошлась бы с ним, и
/// `var s: q(4,4) := 1.0 + 2.0;` дало бы не то, что `var s: q(4,4) := 3.0;`.
fn lower_folded_fixed(
    literal: crate::parser::ast::Expression,
    ty: Option<&crate::semantic::type_node::TypeNode>,
) -> Result<crate::parser::ast::Expression, Diagnostic> {
    use crate::parser::ast::Expression as E;
    use crate::semantic::type_node::TypeNode;
    let (Some(TypeNode::Fixed { m, n, .. }), E::Rational(loc, text, negative)) = (ty, &literal)
    else {
        return Ok(literal);
    };
    let node = ExpressionNode::Rational(text.clone(), *negative);
    match crate::semantic::type_node::type_fixed::lower_fixed_literal(&node, *m, *n, *loc)? {
        Some(repr) => Ok(E::Number(*loc, repr)),
        None => Ok(literal),
    }
}

/// Тип объявления, если он у него есть.
fn declared_type(var: &VariableNode) -> Option<&crate::semantic::type_node::TypeNode> {
    match var {
        VariableNode::Simple { ty, .. } | VariableNode::Const { ty, .. } => Some(ty),
        VariableNode::Port { .. } | VariableNode::Unresolved => None,
    }
}

/// Нормирует **вычисленное** значение по типу объявления (фича 0207).
///
/// # Почему это правило языка, а не удобство свёртки
///
/// Одна и та же запись отвечала по-разному в зависимости от места: в теле
/// `u := ~0;` даёт `255` у эталона и у целей `c`/`rust`, а в объявлении
/// `var u: u8 := ~0;` отвергалось `SE-089` — свёртка (0192) считала `-1` в
/// `i128`, а проверка диапазона (0157) отвергала результат. Нормирование
/// приводит объявление к тому же ответу, что даёт тело: беззнаковое —
/// **обёртка `mod 2ⁿ`** (правило ADR 0127), знаковое — по-прежнему ошибка, то
/// есть значение остаётся как есть и его отвергает `SE-089`.
///
/// ⚠️ **Литерал автора сюда не попадает** (вызывающий отсекает его раньше):
/// `var u: u8 := 300;` обязан остаться ошибкой — автор написал число, которое
/// не помещается, и молча заменить его на `44` значило бы потерять диагностику
/// 0157.
///
/// ⚠️ Границы берутся у **единственного** их носителя —
/// `validate::literal_range::type_range`: своя копия разошлась бы с проверкой, и
/// свёртка нормировала бы к одним границам, а `SE-089` судил по другим.
fn normalize_computed(
    literal: crate::parser::ast::Expression,
    ty: Option<&crate::semantic::type_node::TypeNode>,
) -> crate::parser::ast::Expression {
    use crate::parser::ast::Expression as E;
    let (E::Number(loc, value), Some(ty)) = (&literal, ty) else {
        return literal;
    };
    let Some((lo, hi)) = crate::semantic::validate::literal_range::type_range(ty) else {
        return literal;
    };
    // Беззнаковый тип узнаём по нижней границе: маска — сама верхняя граница
    // (`2ⁿ - 1`), поэтому ширину пересчитывать не нужно.
    if lo == 0 && (*value < lo || *value > hi) {
        return E::Number(*loc, *value & hi);
    }
    literal
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

/// Ссылка вперёд: имя переменной, объявленной НИЖЕ по тексту (фича 0246).
///
/// Возвращает диагностику `SE-109`, если инициализатор `source` упоминает
/// переменную (`var`) той же карты объявлений, чьё объявление стоит после
/// объявления `owner`.
///
/// # Что проверяется точно, а что оставлено законным
///
/// Правило 0192 — «имя в инициализаторе значит начальное значение и ссылается
/// только НАЗАД по тексту» — до этой фичи не проверялось ничем.
///
/// ⚠️ **Константы исключены намеренно:** у них ссылка вперёд разрешается
/// проходами до неподвижной точки (фича 0204) и даёт согласованный результат у
/// эталона и целей — проверено пробой. Запрет сломал бы работающие входы.
///
/// ⚠️ **Прочие невычислимые формы (порт, вызов функции, обращение к полю)
/// остаются законными:** фича не ужесточает язык, а исполняет уже принятое
/// правило. Отвергать всё невычислимое значило бы ломать входы, которых нет в
/// корпусе, — тот же довод, по которому 0192 выбрала расширение, а не запрет.
fn forward_reference(
    source: &crate::parser::ast::Expression,
    owner: &str,
    variables: &BTreeMap<String, VariableNode>,
) -> Option<Diagnostic> {
    let after = declaration_position(&variables[owner]);
    let mut names = Vec::new();
    collect_identifiers(source, &mut names);
    for (name, loc) in names {
        let Some(other) = variables.get(&name) else {
            continue;
        };
        // Только переменные: константа вперёд законна (см. заголовок функции).
        if !matches!(other, VariableNode::Simple { .. }) {
            continue;
        }
        if declaration_position(other) <= after {
            continue;
        }
        return Some(
            Diagnostic::error(
                loc,
                format!(
                    "переменная '{name}' объявлена ниже: в инициализаторе имя значит \
                     НАЧАЛЬНОЕ значение и ссылается только назад по тексту. \
                     Переставьте объявления либо возьмите константу"
                ),
            )
            .with_code("SE-109"),
        );
    }
    None
}

/// Собирает идентификаторы выражения вместе с их позициями (фича 0246).
///
/// Разбор намеренно **не** исчерпывающий по `Expression`: интересны только
/// имена, а формы, их не содержащие, к делу не относятся. Пропущенная форма
/// даёт прежнее поведение (молчание), а не ложный отказ.
fn collect_identifiers(expr: &crate::parser::ast::Expression, out: &mut Vec<(String, Location)>) {
    use crate::parser::ast::Expression;
    match expr {
        Expression::Variable(id) => out.push((id.name.clone(), id.loc)),
        Expression::Parenthesis(_, inner)
        | Expression::Not(_, inner)
        | Expression::BitwiseNot(_, inner)
        | Expression::UnaryPlus(_, inner)
        | Expression::Negate(_, inner)
        | Expression::Cast(_, inner, _) => collect_identifiers(inner, out),
        Expression::Power(_, l, r)
        | Expression::Multiply(_, l, r)
        | Expression::Divide(_, l, r)
        | Expression::Modulo(_, l, r)
        | Expression::Add(_, l, r)
        | Expression::Subtract(_, l, r)
        | Expression::ShiftLeft(_, l, r)
        | Expression::ShiftRight(_, l, r)
        | Expression::BitwiseAnd(_, l, r)
        | Expression::BitwiseXor(_, l, r)
        | Expression::BitwiseOr(_, l, r) => {
            collect_identifiers(l, out);
            collect_identifiers(r, out);
        }
        _ => {}
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

    // Имена, у которых тип НЕ объявлен, снимаются ДО вывода типов: после него
    // `Inference` заменён выведенным, и отличить «автор выбрал ширину» от
    // «ширину выбрал литерал» уже нечем (фича 0285).
    let untyped: std::collections::BTreeSet<String> = resolved
        .iter()
        .filter(|(_, var)| {
            matches!(
                declared_type(var),
                Some(crate::semantic::type_node::TypeNode::Inference)
            )
        })
        .map(|(name, _)| name.clone())
        .collect();

    let inferred =
        crate::semantic::type_inference::type_inference(&mut resolved, Rc::clone(model))?;
    fold_variable_initializers(&inferred, &raw, model, &untyped)
}
