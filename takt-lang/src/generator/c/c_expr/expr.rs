//! Печать выражений.
//!
//! Часть модуля `c_expr` (фича 0027: деление по логике).

use super::*;

/// Генерирует C-выражение из семантического узла с учётом приоритета операторов.
///
/// Скобки добавляются автоматически только там, где это необходимо для
/// сохранения семантики: если `expr_precedence(expr) < min_prec`.
///
/// Используйте `min_prec = 0` для выражений верхнего уровня.
pub(in crate::generator::c) fn generate_expr(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    expr: &ExpressionNode,
    min_prec: u8,
    has_model: bool,
) -> Result<(), Diagnostic> {
    let my_prec = expr_precedence(expr);
    let wrap = my_prec < min_prec;
    if wrap {
        printer.print("(");
    }
    match expr {
        // Длительность (фича 0183) печатается **миллисекундами** — единицей
        // представления значения в целях. Пересчёт зовёт общий слой: своей
        // арифметики времени генератор не заводит (правило 7 ADR 0134).
        ExpressionNode::Duration(nanos) => {
            let millis = crate::semantic::duration::value_millis(
                *nanos,
                crate::diagnostics::Location::Codegen,
                "литерал длительности",
            )?;
            printer.print(&millis.to_string());
        }
        // Выражения нет вовсе: полезной нагрузки у ветви тоже нет, позицию
        // взять негде — отказ остаётся безликим. Это предмет фичи 0212
        // («диагностика цели `c` без кода»), а не забывчивость.
        ExpressionNode::None => {
            return Err("Неразрешённое выражение".into());
        }
        // ⚠️ Неразрешённое выражение отделено от отсутствующего (фича 0236):
        // узел несёт АСД, а значит и позицию, и отказ обязан её нести.
        ExpressionNode::Unresolved(raw) => {
            return Err(crate::generator::c::c_unresolved::refuse(
                raw.loc(),
                crate::generator::c::c_unresolved::UnresolvedNode::Expression,
            ));
        }

        // ── Литералы ──────────────────────────────────────────────────────────
        ExpressionNode::Number(n) => {
            printer.print(&crate::generator::c::c_literal::c_int_literal(*n));
        }
        ExpressionNode::Bool(value) => {
            printer.print(if *value { "true" } else { "false" });
        }
        ExpressionNode::String(v) => {
            printer.print("\"").print(&v.join("")).print("\"");
        }
        ExpressionNode::Rational(s, neg) => {
            if *neg {
                printer.print("-");
            }
            printer.print(s);
        }

        // ── Унарные операторы ──────────────────────────────────────────────────
        // min_prec=14 для операнда: бинарные выражения (prec≤13) будут обёрнуты;
        // также исключает двусмысленные `--x` и `++x` (унарный + унарный).
        ExpressionNode::Not(e) => {
            printer.print("!");
            generate_expr(printer, map, owner, params, e, 14, has_model)?;
        }
        ExpressionNode::BitwiseNot(e) => {
            printer.print("~");
            generate_expr(printer, map, owner, params, e, 14, has_model)?;
        }
        ExpressionNode::UnaryPlus(e) => {
            printer.print("+");
            generate_expr(printer, map, owner, params, e, 14, has_model)?;
        }
        ExpressionNode::Negate(e) => {
            // Унарный минус над q(m, n): −repr с wraparound к W (правило 3 ADR).
            if let Some((m, n, sat)) = super::fixed::fixed_of(map, owner, expr) {
                super::fixed::negate(printer, map, owner, params, e, m, n, sat, has_model)?;
            } else {
                printer.print("-");
                generate_expr(printer, map, owner, params, e, 14, has_model)?;
            }
        }

        // ── Степень → pow() ────────────────────────────────────────────────────
        ExpressionNode::Power(l, r) => {
            printer.print("pow((double)(");
            generate_expr(printer, map, owner, params.clone(), l, 0, has_model)?;
            printer.print("), (double)(");
            generate_expr(printer, map, owner, params, r, 0, has_model)?;
            printer.print("))");
        }

        // ── Бинарные арифметические ────────────────────────────────────────────
        // Левый операнд: допускается тот же приоритет (левоассоциативность).
        // Правый операнд: требует более высокого приоритета (wrap при равном).
        ExpressionNode::Multiply(l, r) => {
            if let Some((m, n, sat)) = super::fixed::fixed_of(map, owner, expr) {
                super::fixed::binary(
                    printer,
                    map,
                    owner,
                    params,
                    super::fixed::FixedOp::Multiply,
                    l,
                    r,
                    m,
                    n,
                    sat,
                    has_model,
                )?;
            } else {
                generate_expr(printer, map, owner, params.clone(), l, 12, has_model)?;
                printer.print(" * ");
                generate_expr(printer, map, owner, params, r, 13, has_model)?;
            }
        }
        ExpressionNode::Divide(l, r) => {
            if let Some((m, n, sat)) = super::fixed::fixed_of(map, owner, expr) {
                super::fixed::binary(
                    printer,
                    map,
                    owner,
                    params,
                    super::fixed::FixedOp::Divide,
                    l,
                    r,
                    m,
                    n,
                    sat,
                    has_model,
                )?;
            } else {
                generate_expr(printer, map, owner, params.clone(), l, 12, has_model)?;
                printer.print(" / ");
                generate_expr(printer, map, owner, params, r, 13, has_model)?;
            }
        }
        ExpressionNode::Modulo(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 12, has_model)?;
            printer.print(" % ");
            generate_expr(printer, map, owner, params, r, 13, has_model)?;
        }
        ExpressionNode::Add(l, r) => {
            if let Some((m, n, sat)) = super::fixed::fixed_of(map, owner, expr) {
                super::fixed::binary(
                    printer,
                    map,
                    owner,
                    params,
                    super::fixed::FixedOp::Add,
                    l,
                    r,
                    m,
                    n,
                    sat,
                    has_model,
                )?;
            } else {
                generate_expr(printer, map, owner, params.clone(), l, 11, has_model)?;
                printer.print(" + ");
                generate_expr(printer, map, owner, params, r, 12, has_model)?;
            }
        }
        ExpressionNode::Subtract(l, r) => {
            if let Some((m, n, sat)) = super::fixed::fixed_of(map, owner, expr) {
                super::fixed::binary(
                    printer,
                    map,
                    owner,
                    params,
                    super::fixed::FixedOp::Subtract,
                    l,
                    r,
                    m,
                    n,
                    sat,
                    has_model,
                )?;
            } else {
                generate_expr(printer, map, owner, params.clone(), l, 11, has_model)?;
                printer.print(" - ");
                generate_expr(printer, map, owner, params, r, 12, has_model)?;
            }
        }

        // ── Битовые сдвиги ────────────────────────────────────────────────────
        ExpressionNode::ShiftLeft(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 10, has_model)?;
            printer.print(" << ");
            generate_expr(printer, map, owner, params, r, 11, has_model)?;
        }
        ExpressionNode::ShiftRight(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 10, has_model)?;
            printer.print(" >> ");
            generate_expr(printer, map, owner, params, r, 11, has_model)?;
        }

        // ── Побитовые операторы ────────────────────────────────────────────────
        ExpressionNode::BitwiseAnd(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 7, has_model)?;
            printer.print(" & ");
            generate_expr(printer, map, owner, params, r, 8, has_model)?;
        }
        ExpressionNode::BitwiseXor(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 6, has_model)?;
            printer.print(" ^ ");
            generate_expr(printer, map, owner, params, r, 7, has_model)?;
        }
        ExpressionNode::BitwiseOr(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 5, has_model)?;
            printer.print(" | ");
            generate_expr(printer, map, owner, params, r, 6, has_model)?;
        }

        // ── Сравнение ─────────────────────────────────────────────────────────
        ExpressionNode::Less(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 9, has_model)?;
            printer.print(" < ");
            generate_expr(printer, map, owner, params, r, 10, has_model)?;
        }
        ExpressionNode::More(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 9, has_model)?;
            printer.print(" > ");
            generate_expr(printer, map, owner, params, r, 10, has_model)?;
        }
        ExpressionNode::LessEqual(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 9, has_model)?;
            printer.print(" <= ");
            generate_expr(printer, map, owner, params, r, 10, has_model)?;
        }
        ExpressionNode::MoreEqual(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 9, has_model)?;
            printer.print(" >= ");
            generate_expr(printer, map, owner, params, r, 10, has_model)?;
        }
        ExpressionNode::Equal(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 8, has_model)?;
            printer.print(" == ");
            generate_expr(printer, map, owner, params, r, 9, has_model)?;
        }
        ExpressionNode::NotEqual(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 8, has_model)?;
            printer.print(" != ");
            generate_expr(printer, map, owner, params, r, 9, has_model)?;
        }

        // ── Логические ────────────────────────────────────────────────────────
        ExpressionNode::And(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 4, has_model)?;
            printer.print(" && ");
            generate_expr(printer, map, owner, params, r, 5, has_model)?;
        }
        ExpressionNode::Or(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 3, has_model)?;
            printer.print(" || ");
            generate_expr(printer, map, owner, params, r, 4, has_model)?;
        }

        // ── Специальные ───────────────────────────────────────────────────────
        // Явные скобки из исходного кода — всегда генерируем как есть.
        ExpressionNode::Parenthesis(e) => {
            printer.print("(");
            generate_expr(printer, map, owner, params, e, 0, has_model)?;
            printer.print(")");
        }

        // Тернарный оператор: условие обёртывается при prec ≤ ||, чтобы
        // присваивание или вложенный тернарный в условии был явно выделен.
        ExpressionNode::ConditionalOperator(cond, then_, else_) => {
            generate_expr(printer, map, owner, params.clone(), cond, 4, has_model)?;
            printer.print(" ? ");
            generate_expr(printer, map, owner, params.clone(), then_, 0, has_model)?;
            printer.print(" : ");
            generate_expr(printer, map, owner, params, else_, 0, has_model)?;
        }

        ExpressionNode::Assign(l, r) => {
            // Запись по анонимному адресу (фича 0189): поле уже слова пишется
            // чтением-изменением-записью, целое слово — прямым присваиванием.
            if let ExpressionNode::AnonPort(access) = l.as_ref() {
                if !map.hal() {
                    return Err(crate::generator::c::c_anon::refuse_plain_c());
                }
                let mut rhs_str = String::new();
                {
                    let mut tmp = Printer::new(4, &mut rhs_str);
                    generate_expr(&mut tmp, map, owner, params, r, 0, has_model)?;
                }
                printer.print(&crate::generator::c::c_anon::write(access, &rhs_str));
                return Ok(());
            }
            // Запись в порт → write_bit / write_float
            if let ExpressionNode::Variable(var_rc) = l.as_ref() {
                let var = var_rc.borrow();
                if let VariableNode::Port {
                    name, ty, upper, ..
                } = &*var
                {
                    let model_name =
                        if let Some(model_rc) = upper.as_ref().and_then(|w| w.upgrade()) {
                            Name::from(model_rc)
                        } else {
                            return Err("Неразрешённый owner порта при записи".into());
                        };
                    let cls = PortClass::from_type(ty);
                    let variant =
                        crate::generator::c::c_names::port_enum_variant(&model_name, name);
                    let mut rhs_str = String::new();
                    {
                        let mut tmp = Printer::new(4, &mut rhs_str);
                        generate_expr(&mut tmp, map, owner, params, r, 0, has_model)?;
                    }
                    let ptr = if has_model && !owner.name().eq(&map.root_name()) {
                        "main"
                    } else {
                        "model"
                    };
                    match cls {
                        PortClass::Rational => {
                            printer.print(&format!(
                                "(*{ptr}->{write_float})({variant}, {rhs_str}, {ptr}->userdata)",
                                write_float = FUNCTION_PORT_WRITE_FLOAT
                            ));
                        }
                        PortClass::Numeric => {
                            printer.print(&format!(
                                "(*{ptr}->{write_numeric})({variant}, {rhs_str}, {ptr}->userdata)",
                                write_numeric = FUNCTION_PORT_WRITE_NUMERIC
                            ));
                        }
                        PortClass::Bit => {
                            printer.print(&format!(
                                "(*{ptr}->{write_bit})({variant}, {rhs_str}, {ptr}->userdata)",
                                write_bit = FUNCTION_PORT_WRITE_BIT
                            ));
                        }
                    }
                    return Ok(());
                }
            }
            // BitAccess как lvalue: inner.N = val
            if let ExpressionNode::BitAccess(inner_expr, Member::Number(n)) = l.as_ref() {
                // Порт.бит = val → write_bit(PORT, N, val, userdata)
                if let ExpressionNode::Variable(var_rc) = inner_expr.as_ref() {
                    let var = var_rc.borrow();
                    if let VariableNode::Port {
                        name, ty, upper, ..
                    } = &*var
                    {
                        let model_name = if let Some(rc) = upper.as_ref().and_then(|w| w.upgrade())
                        {
                            Name::from(rc)
                        } else {
                            return Err("Неразрешённый owner порта при BitAccess записи".into());
                        };
                        let cls = PortClass::from_type(ty);
                        if cls == PortClass::Rational {
                            return Err(Diagnostic::error(
                                Location::Codegen,
                                "BitAccess на float-порт не поддерживается при записи".to_string(),
                            )
                            .with_code("CC-001"));
                        }
                        let variant =
                            crate::generator::c::c_names::port_enum_variant(&model_name, name);
                        let mut rhs_str = String::new();
                        {
                            let mut tmp = Printer::new(4, &mut rhs_str);
                            generate_expr(&mut tmp, map, owner, params, r, 0, has_model)?;
                        }
                        let ptr = if has_model && !owner.name().eq(&map.root_name()) {
                            "main"
                        } else {
                            "model"
                        };
                        match cls {
                            PortClass::Bit => {
                                printer.print(&format!(
                                    "(*{ptr}->{write_bit})({variant}, {rhs_str}, {ptr}->userdata)",
                                    write_bit = FUNCTION_PORT_WRITE_BIT
                                ));
                            }
                            PortClass::Numeric => {
                                printer.print(&format!(
                                    "(*{ptr}->{write_numeric})({variant}, \
                                    ((*{ptr}->{read_numeric})({variant}, {ptr}->userdata) \
                                    & ~(1LL << {n})) | (({rhs_str} & 1LL) << {n}), {ptr}->userdata)",
                                    write_numeric = FUNCTION_PORT_WRITE_NUMERIC,
                                    read_numeric = FUNCTION_PORT_READ_NUMERIC,
                                ));
                            }
                            PortClass::Rational => unreachable!(),
                        }
                        return Ok(());
                    }
                }
                // Обычная переменная.бит = val
                // x = (x & ~(1u << N)) | ((val & 1u) << N)
                let mut lhs_str = String::new();
                {
                    let mut tmp = Printer::new(4, &mut lhs_str);
                    generate_expr(
                        &mut tmp,
                        map,
                        owner,
                        params.clone(),
                        inner_expr,
                        0,
                        has_model,
                    )?;
                }
                let mut rhs_str = String::new();
                {
                    let mut tmp = Printer::new(4, &mut rhs_str);
                    generate_expr(&mut tmp, map, owner, params, r, 0, has_model)?;
                }
                printer.print(&format!(
                    "{0} = ({0} & ~(1u << {1})) | (({2} & 1u) << {1})",
                    lhs_str, n, rhs_str
                ));
                return Ok(());
            }
            // Обычное присваивание (право-ассоциативно: тот же prec не оборачивается)
            generate_expr(printer, map, owner, params.clone(), l, 1, has_model)?;
            printer.print(" = ");
            generate_expr(printer, map, owner, params, r, 1, has_model)?;
        }

        ExpressionNode::ArraySubscript(var_rc, idx) => {
            let idx_str = {
                let mut buf = String::new();
                let mut p = Printer::new(0, &mut buf);
                generate_expr(&mut p, map, owner, params.clone(), idx, 0, has_model)?;
                buf
            };
            let var = var_rc.borrow();
            let var_expr = if let VariableNode::Simple { upper, .. } = &*var {
                resolve_simple_var_in_context(var.name(), upper, &params, owner, map, has_model)
                    .map_or_else(
                        || resolve_variable_c_expr(&*var, &params, map, owner, has_model),
                        Ok,
                    )?
            } else {
                resolve_variable_c_expr(&*var, &params, map, owner, has_model)?
            };
            printer.print(&format!("{}[{}]", var_expr, idx_str));
        }

        ExpressionNode::Variable(var_rc) => {
            let var = var_rc.borrow();
            let var_expr = if let VariableNode::Simple { upper, loc, .. } = &*var {
                // Локальные переменные (loc == Implicit) доступны по имени напрямую,
                // а не через model->name, даже если они принадлежат той же модели.
                if matches!(loc, crate::diagnostics::Location::Implicit) {
                    normalize_lowercase_snakecase(var.name().to_string())
                } else {
                    resolve_simple_var_in_context(var.name(), upper, &params, owner, map, has_model)
                        .map_or_else(
                            || resolve_variable_c_expr(&*var, &params, map, owner, has_model),
                            Ok,
                        )?
                }
            } else {
                resolve_variable_c_expr(&*var, &params, map, owner, has_model)?
            };
            printer.print(&var_expr);
        }

        ExpressionNode::Condition(cond_rc) => {
            let cond = cond_rc.borrow();
            let cond_str = condition_macro_name(&*cond);
            printer.print(&cond_str);
        }

        ExpressionNode::Function(fun_rc, args) => {
            let fun = fun_rc.borrow();
            generate_function_call(printer, map, owner, params, &*fun, args, has_model)?;
        }

        ExpressionNode::Initializer(elems) => {
            printer.print("{");
            for (i, elem) in elems.iter().enumerate() {
                if i > 0 {
                    printer.print(", ");
                }
                generate_expr(printer, map, owner, params.clone(), elem, 0, has_model)?;
            }
            printer.print("}");
        }

        ExpressionNode::Array(elems) => {
            printer.print("{");
            for (i, elem) in elems.iter().enumerate() {
                if i > 0 {
                    printer.print(", ");
                }
                generate_expr(printer, map, owner, params.clone(), elem, 0, has_model)?;
            }
            printer.print("}");
        }

        ExpressionNode::Cast(expr, typ) => {
            let model = map.raw_model_at(owner.name())?;
            let model = &*model.borrow();
            // 0029-01: было `unwrap_or_else(|| "int")` — невыразимый тип приведения
            // молча превращался в `(int)`, то есть приведение к ДРУГОМУ типу,
            // принятое C-компилятором без замечаний.
            let type_c = c_type_or_diagnostic(typ, model, map.float_width(), "приведение типа")?;
            // Fixed-point (0061): масштабирующее приведение, когда источник либо
            // цель — q(m, n). Сдвиги не используются (ловушка C11, UB `<<`).
            if matches!(typ, TypeNode::Fixed { .. })
                || super::fixed::fixed_of(map, owner, expr).is_some()
            {
                super::fixed::cast(printer, map, owner, params, expr, typ, &type_c, has_model)?;
            } else {
                // Приводимое выражение оборачивается при prec < UNARY (13),
                // то есть при наличии бинарных операторов: (int)(a + b).
                printer.print("(").print(&type_c).print(")");
                generate_expr(printer, map, owner, params, expr, 13, has_model)?;
            }
        }

        // ── Неподдерживаемые ──────────────────────────────────────────────────
        ExpressionNode::ArraySlice(_, _, _) => {
            return Err("ArraySlice не поддерживается в C генераторе".into());
        }
        ExpressionNode::BitAccess(inner, member) => {
            match member {
                Member::Identifier(id) => {
                    // Доступ к полю структуры: inner.field — используем максимальный приоритет
                    generate_expr(printer, map, owner, params, inner, 15, has_model)?;
                    printer.print(&format!(".{}", id.name));
                }
                Member::Number(n) => {
                    // Битовый доступ к порту: (*main->read_bit)(PORT_X, N, main->userdata)
                    if let ExpressionNode::Variable(var_rc) = inner.as_ref() {
                        let var = var_rc.borrow();
                        if let VariableNode::Port {
                            name, ty, upper, ..
                        } = &*var
                        {
                            let model_name =
                                if let Some(rc) = upper.as_ref().and_then(|w| w.upgrade()) {
                                    Name::from(rc)
                                } else {
                                    return Err("Неразрешённый owner порта при BitAccess".into());
                                };
                            let cls = PortClass::from_type(ty);
                            let variant =
                                crate::generator::c::c_names::port_enum_variant(&model_name, name);
                            let ptr = if has_model && !owner.name().eq(&map.root_name()) {
                                "main"
                            } else {
                                "model"
                            };
                            match cls {
                                PortClass::Bit => {
                                    printer.print(&format!(
                                        "(*{ptr}->{read_bit})({variant}, {ptr}->userdata)",
                                        read_bit = FUNCTION_PORT_READ_BIT
                                    ));
                                }
                                PortClass::Numeric => {
                                    printer.print(&format!(
                                        "(((*{ptr}->{read_numeric})({variant}, {ptr}->userdata) >> {n}) & 1u)",
                                        read_numeric = FUNCTION_PORT_READ_NUMERIC
                                    ));
                                }
                                PortClass::Rational => {
                                    return Err(Diagnostic::error(
                                        Location::Codegen,
                                        "BitAccess на float-порт не поддерживается".to_string(),
                                    )
                                    .with_code("CC-001"));
                                }
                            }
                            return Ok(());
                        }
                    }
                    // Обычная переменная/выражение: ((inner >> N) & 1u)
                    printer.print("((");
                    generate_expr(printer, map, owner, params, inner, 0, has_model)?;
                    printer.print(&format!(" >> {}) & 1u)", n));
                }
            }
        }
        ExpressionNode::CodeBlock(_, _) => {
            return Err("CodeBlock не поддерживается как выражение в C генераторе".into());
        }
        ExpressionNode::NamedFunctionBox(_, _) => {
            return Err("NamedFunctionBox не поддерживается в C генераторе".into());
        }
        ExpressionNode::List(_) => {
            return Err("List не поддерживается в C генераторе".into());
        }
        ExpressionNode::Type(_) => {
            return Err("Type не поддерживается как выражение в C генераторе".into());
        }
        ExpressionNode::Address(_, _) => {
            return Err("Address не поддерживается как выражение в C генераторе".into());
        }
        // Анонимное обращение к ячейке (фича 0189): печатает только `c-hal` —
        // цель `c` адресов не знает по устройству (ADR 0020).
        ExpressionNode::AnonPort(access) => {
            if !map.hal() {
                return Err(crate::generator::c::c_anon::refuse_plain_c());
            }
            printer.print(&crate::generator::c::c_anon::read(access));
        }
        ExpressionNode::Model(_) => {
            return Err("Model не поддерживается как выражение в C генераторе".into());
        }
    }
    if wrap {
        printer.print(")");
    }
    Ok(())
}
