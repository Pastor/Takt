//! Печать условий перехода, включая `S(Модель) = Состояние`.
//!
//! Часть модуля `c_expr` (фича 0027: деление по логике).

use super::*;

/// Модель, о **текущем состоянии** которой идёт речь в левой части сравнения.
///
/// Две записи означают одно и то же и дают один и тот же C:
/// - `S(Модель)` — встроенная функция языка: текущее состояние модели
///   (`semantic/builtin.rs`, `Builtin("S", [model: BuiltinModel]) -> BuiltinState`);
/// - `Модель` — та же величина в краткой форме.
///
/// Краткая форма поддерживалась генератором с самого начала, а `S(…)` —
/// **нет**: она приходит сюда как `Function`, не как `Model`, и упиралась в
/// `CC-003` «Ссылки на модели и состояния не поддерживаются». Дефект был
/// **невиден**, потому что вызывающий проглатывал ошибку (фича 0028).
///
/// **Скобки здесь не разворачиваются — и не нужно.** Скобочные формы
/// (`(S(Ping)) = End`, `S((Ping)) = End`, `S(Ping) = (End)`) канонизирует
/// СЕМАНТИКА до генератора: `resolve_condition` снимает прозрачные скобки
/// паттерна `S(Модель)` в единой воронке разбора (фича 0074). Сюда условие
/// приходит уже каноничным `Function(S, [Model])` — распознавание совпадает без
/// разворота. Прежде эти формы отвергались `SE-025` (обёртка `Parenthesis`
/// ломала сопоставление) — 0074 сделала скобки прозрачными.
fn state_of_model(cond: &ConditionNode) -> Option<&Rc<RefCell<ModelNode>>> {
    match cond {
        ConditionNode::Model(model, _) => Some(model),
        ConditionNode::Function(fun, args, _) => {
            if !matches!(&*fun.borrow(), FunctionDefinitionNode::Builtin("S", ..)) {
                return None;
            }
            // Арность встроенной `S` — ровно один параметр (`model`), проверена
            // семантикой; аргумент обязан быть моделью.
            match args.first().map(|a| a.as_ref())? {
                ConditionNode::Model(model, _) => Some(model),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Печатает сравнение текущего состояния модели с её состоянием:
/// `<путь к модели>.state == {MODEL}_{STATE}`.
///
/// Общая реализация для `=` и `!=` (`op`): ветки различались **только**
/// оператором и были дословными копиями по сорок строк каждая.
///
/// # Почему имя состояния берётся строкой
///
/// Правая часть (`End` в `S(Ping) = End`) приходит **неразрешённой**: `End` —
/// состояние модели-аргумента, а не той, где записано условие, поэтому
/// семантика разрешить его не может и не должна (`CLAUDE.md`: проход
/// `resolve_state_references` запрещён — он ломает ровно эту конструкцию).
/// Разрешение выполняется здесь, **в области видимости модели-аргумента**:
/// имя ищется среди её состояний.
fn generate_state_comparison(
    model: &Rc<RefCell<ModelNode>>,
    right: &ConditionNode,
    op: &str,
    map: &CMap,
    owner: &Element,
) -> Result<String, Diagnostic> {
    let eq_name = match right {
        ConditionNode::Variable(v, ..) => v.borrow().name().to_string(),
        // Неразрешённое имя — штатный случай (см. заголовок функции).
        ConditionNode::Unresolved(Condition::Variable(id)) => id.name.clone(),
        // Имя состояния, случайно совпавшее с состоянием объемлющей модели:
        // семантика разрешила его в ЧУЖОЙ области видимости. Берём только имя —
        // искать всё равно в модели-аргументе.
        ConditionNode::State(state, _) => state.borrow().name().to_string(),
        other => {
            return Err(Diagnostic::error(
                Location::Codegen,
                format!("Выражение {:?} не разыменовано", other),
            )
            .with_code("CC-013"));
        }
    };

    let model_name = Name::from(model.clone());
    let using_models = map.using_models();
    let element = using_models
        .iter()
        .find(|m| m.name().eq(&model_name))
        .ok_or_else(|| {
            Diagnostic::error(
                Location::Codegen,
                format!("Модель {} не найдена", model_name),
            )
            .with_code("CC-012")
        })?;

    let Element::Model { states, .. } = element else {
        return Err(Diagnostic::error(
            Location::Codegen,
            format!("Элемент {} не является моделью", model_name),
        )
        .with_code("CC-006"));
    };

    let state = states
        .iter()
        .find(|s| s.local() == eq_name)
        .ok_or_else(|| {
            Diagnostic::error(
                Location::Codegen,
                format!("Состояние {} не найдено в модели {}", eq_name, model_name),
            )
            .with_code("CC-011")
        })?;

    let is_same_model = model_name.eq(&owner.name());
    let is_root_model = model.borrow().upper.is_none();
    let is_root_owner = owner.name().eq(&map.root_name());

    // Поле целевой модели лежит в структуре её РОДИТЕЛЯ, поэтому база пути
    // зависит от родства с владельцем условия. Прежде база была `model->`
    // БЕЗУСЛОВНО — то есть предполагалось, что владелец и есть родитель. Для
    // модели-СЕСТРЫ это давало `model->entry.ping0.state` внутри
    // `SrefPong_tick`: `cc` → «no member named 'entry' in 'struct SrefPong'»
    // (проба). Дефект не проявлялся, потому что единственный вход в эту ветку —
    // `S(Модель) = Состояние`, а он до генератора не доходил вовсе.
    let parent_is_owner = model
        .borrow()
        .upper
        .as_ref()
        .and_then(|w| w.upgrade())
        .map(|p| Name::from(p).eq(&owner.name()))
        .unwrap_or(false);

    let path = if is_same_model {
        // Своё же состояние.
        "model->state".to_string()
    } else if is_root_model && !is_root_owner {
        "main->state".to_string()
    } else if parent_is_owner {
        // Поле своей структуры — самый короткий путь.
        let field = field_name_in_parent(model).unwrap_or_else(|| {
            normalize_lowercase_snakecase(model.borrow().name.clone().unwrap_or_default())
        });
        format!("model->{}.state", field)
    } else {
        // Прямого пути у владельца нет (модель-сестра, в т.ч. вложенная), но
        // корень доступен всегда и владеет всем по значению — адресуем цепочкой
        // от него.
        //
        // ⚠ Достижимость ветки `CC-019` НЕ УСТАНОВЛЕНА: единственный найденный
        // способ получить `None` — модель, не встроенную ни в одно состояние, —
        // отсекается раньше кодом `CC-012` («Модель не найдена»), потому что
        // такой модели нет в `using_models` (проба). Диагностика оставлена как
        // отказ по умолчанию: альтернатива — вернуть заведомо неверный путь
        // молча, то есть ровно тот класс дефекта, против которого заведена 0028.
        let chain = path_from_root(model).ok_or_else(|| {
            Diagnostic::error(
                Location::Codegen,
                format!(
                    "состояние модели '{}' недостижимо из '{}': модель не \
                     встроена ни в одно состояние родителя",
                    model_name,
                    owner.name()
                ),
            )
            .with_code("CC-019")
        })?;
        format!("main->{}.state", chain)
    };

    let state_const = format!(
        "{}_{}",
        model_name.unique_uppercase_snakecase(),
        normalize_lowercase_snakecase(state.local().to_string()).to_uppercase()
    );

    Ok(format!("{} {} {}", path, op, state_const))
}

/// Преобразует [`ConditionNode`] в строку C-выражения.
///
/// Используется при генерации условий переходов для простых состояний.
/// Возвращает пустую строку для безусловных переходов (`ConditionNode::None`).
pub(in crate::generator::c) fn generate_condition_expr(
    cond: &ConditionNode,
    map: &CMap,
    owner: &Element,
) -> Result<String, Diagnostic> {
    match cond {
        ConditionNode::None | ConditionNode::Unresolved(_) => Ok(String::new()),
        ConditionNode::Bool(b) => Ok(if *b { "true" } else { "false" }.to_string()),
        // Длительность (фича 0134): эмиссия — задача этой цели; до неё явный
        // отказ, а не печать наносекунд обычным числом.
        // Выдержка `after` (фича 0134): сравнение по РАЗНОСТИ единиц профиля.
        // Счётчик `_dwell` обнуляется при входе в состояние и увеличивается в
        // конце такта, поэтому его значение равно числу тактов, прошедших с
        // входа, — ровно то, что меряет эталон модельным временем.
        ConditionNode::After(nanos) => after_condition(*nanos, map, owner),
        // Вычисляемая выдержка (фича 0183): значение известно лишь в такте,
        // поэтому сравнивается не число, а напечатанное выражение — в
        // МИЛЛИСЕКУНДАХ (представление `duration` в целях).
        ConditionNode::AfterExpr(inner) => {
            let expr = generate_condition_expr(inner, map, owner)?;
            after_dynamic_condition(&expr, map, owner)
        }
        // Выдержка в тактах частоты НЕ требует: счётчик и так считает такты.
        ConditionNode::AfterTicks(ticks) => Ok(format!("{} >= {}", dwell_access(), ticks)),
        // Литерал длительности вне `after` — сравнение со значением типа
        // `duration` (фича 0183). Печатается **миллисекундами**, как и значение;
        // пересчёт зовёт общий слой (правило 7 ADR 0134).
        ConditionNode::Duration(nanos) => Ok(crate::semantic::duration::value_millis(
            *nanos,
            Location::Codegen,
            "литерал длительности в условии",
        )?
        .to_string()),
        ConditionNode::Number(n) => Ok(n.to_string()),
        ConditionNode::Rational(s, neg) => {
            if *neg {
                Ok(format!("-{}", s))
            } else {
                Ok(s.clone())
            }
        }
        ConditionNode::String(parts) => Ok(format!("\"{}\"", parts.join(""))),
        ConditionNode::Not(inner) => Ok(format!(
            "!({})",
            generate_condition_expr(inner, map, owner)?
        )),
        ConditionNode::Parenthesis(inner) => {
            Ok(format!("({})", generate_condition_expr(inner, map, owner)?))
        }
        ConditionNode::Add(l, r) => Ok(format!(
            "{} + {}",
            generate_condition_expr(l, map, owner)?,
            generate_condition_expr(r, map, owner)?
        )),
        ConditionNode::Subtract(l, r) => Ok(format!(
            "{} - {}",
            generate_condition_expr(l, map, owner)?,
            generate_condition_expr(r, map, owner)?
        )),
        ConditionNode::And(l, r) => Ok(format!(
            "{} && {}",
            generate_condition_expr(l, map, owner)?,
            generate_condition_expr(r, map, owner)?
        )),
        ConditionNode::Or(l, r) => Ok(format!(
            "{} || {}",
            generate_condition_expr(l, map, owner)?,
            generate_condition_expr(r, map, owner)?
        )),
        ConditionNode::Less(l, r) => Ok(format!(
            "{} < {}",
            generate_condition_expr(l, map, owner)?,
            generate_condition_expr(r, map, owner)?
        )),
        ConditionNode::More(l, r) => Ok(format!(
            "{} > {}",
            generate_condition_expr(l, map, owner)?,
            generate_condition_expr(r, map, owner)?
        )),
        ConditionNode::LessEqual(l, r) => Ok(format!(
            "{} <= {}",
            generate_condition_expr(l, map, owner)?,
            generate_condition_expr(r, map, owner)?
        )),
        ConditionNode::MoreEqual(l, r) => Ok(format!(
            "{} >= {}",
            generate_condition_expr(l, map, owner)?,
            generate_condition_expr(r, map, owner)?
        )),
        ConditionNode::Equal(l, r) => {
            if let Some(model) = state_of_model(l) {
                generate_state_comparison(model, r, "==", map, owner)
            } else {
                Ok(format!(
                    "{} == {}",
                    generate_condition_expr(l, map, owner)?,
                    generate_condition_expr(r, map, owner)?
                ))
            }
        }
        ConditionNode::NotEqual(l, r) => {
            if let Some(model) = state_of_model(l) {
                generate_state_comparison(model, r, "!=", map, owner)
            } else {
                Ok(format!(
                    "{} != {}",
                    generate_condition_expr(l, map, owner)?,
                    generate_condition_expr(r, map, owner)?
                ))
            }
        }
        ConditionNode::Variable(var_rc, _) => {
            let var = var_rc.borrow();
            if let VariableNode::Simple { upper, .. } = &*var
                && let Some(s) =
                    resolve_simple_var_in_context(var.name(), upper, &[], owner, map, true)
            {
                return Ok(s);
            }
            resolve_variable_c_expr(&var, &[], map, owner, true)
        }
        ConditionNode::EnumVariant(_, _, value) => Ok(value.to_string()),
        ConditionNode::ArraySubscript(var_rc, idx) => {
            let idx_str = generate_condition_expr(idx, map, owner)?;
            let var = var_rc.borrow();
            if let VariableNode::Simple { upper, .. } = &*var
                && let Some(s) =
                    resolve_simple_var_in_context(var.name(), upper, &[], owner, map, true)
            {
                return Ok(format!("{}[{}]", s, idx_str));
            }
            let base = resolve_variable_c_expr(&var, &[], map, owner, true)?;
            Ok(format!("{}[{}]", base, idx_str))
        }
        ConditionNode::BitAccess(inner, member) => {
            match member {
                Member::Identifier(id) => {
                    // Доступ к полю структуры: inner.field
                    let inner_str = generate_condition_expr(inner, map, owner)?;
                    Ok(format!("{}.{}", inner_str, id.name))
                }
                Member::Number(n) => {
                    // Битовый доступ: проверяем, является ли inner портовой переменной
                    if let ConditionNode::Variable(var_rc, _) = inner.as_ref() {
                        let var = var_rc.borrow();
                        if let VariableNode::Port {
                            name, ty, upper, ..
                        } = &*var
                        {
                            let model_name =
                                if let Some(rc) = upper.as_ref().and_then(|w| w.upgrade()) {
                                    Name::from(rc)
                                } else {
                                    return Err(
                                        "Неразрешённый owner порта при BitAccess в условии".into(),
                                    );
                                };
                            let cls = PortClass::from_type(ty);
                            let variant = format!(
                                "{}_{}",
                                model_name.unique_uppercase_snakecase(),
                                normalize_lowercase_snakecase(name.clone()).to_uppercase()
                            );
                            // В условиях всегда has_model=true; ptr зависит от owner
                            let ptr = if owner.name().eq(&map.root_name()) {
                                "model"
                            } else {
                                "main"
                            };
                            return match cls {
                                PortClass::Bit => Ok(format!(
                                    "(*{ptr}->{read_bit})({variant}, {ptr}->userdata)",
                                    read_bit = FUNCTION_PORT_READ_BIT
                                )),
                                PortClass::Numeric => Ok(format!(
                                    "(((*{ptr}->{read_numeric})({variant}, {ptr}->userdata) >> {n}) & 1u)",
                                    read_numeric = FUNCTION_PORT_READ_NUMERIC
                                )),
                                PortClass::Rational => Err(Diagnostic::error(
                                    Location::Codegen,
                                    "BitAccess на float-порт не поддерживается в условии"
                                        .to_string(),
                                )
                                .with_code("CC-001")),
                            };
                        }
                    }
                    // Обычная переменная/выражение: ((inner >> N) & 1u)
                    let inner_str = generate_condition_expr(inner, map, owner)?;
                    Ok(format!("(({} >> {}) & 1u)", inner_str, n))
                }
            }
        }
        ConditionNode::Function(fun_rc, args, _) => {
            let fun = fun_rc.borrow();
            // Пропускаем неразрешённые и пустые функции — они не могут быть сгенерированы
            if !matches!(
                *fun,
                FunctionDefinitionNode::Local { .. }
                    | FunctionDefinitionNode::External { .. }
                    | FunctionDefinitionNode::Builtin { .. }
            ) {
                return Err(Diagnostic::error(
                    Location::Codegen,
                    "Неразрешённая функция в условии перехода".to_string(),
                )
                .with_code("CC-002"));
            }
            let fn_name = get_function_name(&fun);
            let args_strs: Result<Vec<_>, _> = args
                .iter()
                .map(|a| generate_condition_expr(a, map, owner))
                .collect();
            let args_strs = args_strs?;
            // Локальная функция в C принимает main/model как первый аргумент
            if matches!(*fun, FunctionDefinitionNode::Local { .. }) {
                let first_arg = if owner.name().eq(&map.root_name()) {
                    "model"
                } else {
                    "main"
                };
                let mut all_args = vec![first_arg.to_string()];
                all_args.extend(args_strs);
                Ok(format!("{}({})", fn_name, all_args.join(", ")))
            } else {
                Ok(format!("{}({})", fn_name, args_strs.join(", ")))
            }
        }
        ConditionNode::Model(_, _) | ConditionNode::State(_, _) => Err(Diagnostic::error(
            Location::Codegen,
            "Ссылки на модели и состояния не поддерживаются в условиях переходов".to_string(),
        )
        .with_code("CC-003")),
    }
}

/// Имя поля-счётчика времени, проведённого в текущем состоянии (фича 0134).
///
/// Имя начинается с `takt_`, чтобы не столкнуться с полем автора: имена Takt
/// нормализуются в snake_case без этого префикса.
pub(in crate::generator::c) const DWELL_FIELD: &str = "takt_dwell";

/// Имя поля-метки времени входа в состояние — профиль «часы» (фича 0134-04b).
///
/// Хранит `now_ms()` момента входа; выдержка сравнивается **разностью**
/// `(uintN)(now - t0) >= D_MS` (обёртка беззнакового нормирована ADR 0127).
pub(in crate::generator::c) const ENTRY_MS_FIELD: &str = "takt_entry_ms";

/// Имя поля «состояние на конец предыдущего такта» (фича 0134).
///
/// Нужно, чтобы вход в состояние определялся **одним** сравнением в конце
/// такта, а не десятью правками рядом с присваиваниями `model->state`.
pub(in crate::generator::c) const PREV_STATE_FIELD: &str = "takt_prev_state";

/// Печатает условие выдержки `after` в обоих профилях времени (фича 0134).
///
/// - «такты»: счётчик `takt_dwell >= D_TICKS` (инкремент в конце такта).
/// - «часы»: метка входа `takt_entry_ms` и внешний источник `now_ms`; сравнение
///   **разностью** `(uintN)(now - t0) >= D_MS` — беззнаковая обёртка нормирована
///   ADR 0127, поэтому `t0 + D <= now` (переполнение) не даёт молча неверного
///   результата. Ширина `N` — общая с объявлением поля (см. `c_time`).
fn after_condition(nanos: i64, map: &CMap, owner: &Element) -> Result<String, Diagnostic> {
    let profile = map.time_profile();
    let units = crate::semantic::duration::units_or_diagnostic(
        nanos,
        profile,
        Location::Codegen,
        "выдержка 'after'",
    )?;
    match profile {
        crate::semantic::duration::TimeProfile::Ticks { .. } => {
            Ok(format!("{} >= {}", dwell_access(), units))
        }
        crate::semantic::duration::TimeProfile::Clock => {
            let bits = crate::generator::c::c_time::clock_marker_bits(map)?;
            // HAL (`now_ms`/`userdata`) — на КОРНЕВОЙ структуре: `model` в корне,
            // `main` в под-модели (как порты). Метка `takt_entry_ms` — на self.
            let hal = if owner.name().eq(&map.root_name()) {
                "model"
            } else {
                "main"
            };
            let now = format!(
                "{hal}->{}({hal}->userdata)",
                crate::generator::c::FUNCTION_TIME_NOW_MS
            );
            Ok(format!(
                "(uint{bits}_t)((uint{bits}_t){now} - model->{ENTRY_MS_FIELD}) >= {units}"
            ))
        }
    }
}

/// Условие **вычисляемой** выдержки (фича 0183).
///
/// `expr` — уже напечатанное выражение в миллисекундах.
///
/// Профиль «часы» сравнивает миллисекунды напрямую; профиль «такты» переводит
/// миллисекунды в такты множителем `hertz / 1000`, который обязан быть целым
/// (иначе `SE-073`: округление молча изменило бы выдержку).
fn after_dynamic_condition(expr: &str, map: &CMap, owner: &Element) -> Result<String, Diagnostic> {
    let profile = map.time_profile();
    match crate::semantic::duration::ticks_per_milli(profile, Location::Codegen)? {
        Some(1) => Ok(format!("{} >= {expr}", dwell_access())),
        Some(multiplier) => Ok(format!("{} >= ({expr}) * {multiplier}", dwell_access())),
        None => {
            let bits = crate::generator::c::c_time::clock_marker_bits(map)?;
            let hal = if owner.name().eq(&map.root_name()) {
                "model"
            } else {
                "main"
            };
            let now = format!(
                "{hal}->{}({hal}->userdata)",
                crate::generator::c::FUNCTION_TIME_NOW_MS
            );
            Ok(format!(
                "(uint{bits}_t)((uint{bits}_t){now} - model->{ENTRY_MS_FIELD}) >= ({expr})"
            ))
        }
    }
}

/// Доступ к счётчику времени состояния изнутри такта своей модели.
///
/// Условие ребра печатается при генерации такта **своей** модели, поэтому путь —
/// всегда `model->…` (как у `model->state` для собственного состояния).
pub(in crate::generator::c) fn dwell_access() -> String {
    format!("model->{DWELL_FIELD}")
}
