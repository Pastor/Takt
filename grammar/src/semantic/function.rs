//! Построение семантических узлов функций языка BuT.
//!
//! Основная функция [`construct_function`] преобразует [`FunctionNode::Unresolved`]
//! (необработанное AST-определение) в:
//! - [`FunctionNode::Local`] — локальная функция с телом;
//! - [`FunctionNode::External`] — внешняя функция (`extern fn`).
//!
//! Уже разрешённые функции ([`FunctionNode::Local`], [`FunctionNode::External`],
//! [`FunctionNode::Builtin`]) возвращаются без изменений.

use crate::diagnostics::Diagnostic;
use crate::parser::ast;
use crate::semantic::statement::resolve_statement;
use crate::semantic::type_::construct_type;
use crate::semantic::{FunctionNode, ModelNode, Statement, TypeNode};
use std::cell::RefCell;
use std::rc::Rc;

/// Строит разрешённый семантический узел функции из [`FunctionNode`].
///
/// Обрабатывает только `Unresolved`-вариант; остальные возвращаются без изменений.
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`], если:
/// - функция без имени;
/// - функция с таким именем уже объявлена в текущей модели;
/// - параметр не имеет объявления типа;
/// - локальная функция объявлена без тела.
pub fn construct_function(
    func: FunctionNode,
    model: Rc<RefCell<ModelNode>>,
) -> Result<FunctionNode, Diagnostic> {
    if let FunctionNode::Unresolved(def) = func {
        let name = def
            .clone()
            .name
            .ok_or_else(|| "При определении функция должна иметь имя".into())?
            .name
            .clone();
        if model.borrow().functions.contains_key(&name) {
            Err(format!("Функция с именем '{}' уже определена", name)
                .as_str()
                .into())
        } else {
            let mut params = Vec::new();
            for (_, param) in def.params.iter() {
                if let Some(param) = param {
                    // В грамматике тип параметра хранится как `ast::Expression`.
                    // Возможные варианты:
                    //   - `Expression::Type(_, typ)`  — явный тип (редко используется).
                    //   - `Expression::Variable(id)`  — идентификатор типа (`bit`, `u8`, …).
                    //   - `Expression::Array(_, items)` — массивный тип (не поддерживается как параметр).
                    let param_type = match param.clone().ty {
                        ast::Expression::Type(_, typ) => {
                            construct_type(Some(typ), model.clone())?
                        }
                        ast::Expression::Variable(id) => {
                            // Преобразуем идентификатор в псевдоним типа и разрешаем.
                            construct_type(Some(ast::Type::Alias(id)), model.clone())?
                        }
                        _ => {
                            return Err("Параметр функции должен иметь тип".into());
                        }
                    };
                    let param_name = param
                        .clone()
                        .name
                        .map(|t| t.name.clone())
                        .unwrap_or_default();
                    params.push((param_name, param_type));
                }
            }
            let rett = match def.return_type {
                Some(t) => construct_type(Some(t), model.clone()).map_err(|e| e)?,
                None => TypeNode::Unit,
            };
            if def.external {
                Ok(FunctionNode::External {
                    upper: Some(Rc::downgrade(&model)),
                    name: name.clone(),
                    params,
                    ret: rett,
                })
            } else {
                let statement = if let Some(body) = def.body {
                    resolve_statement(&Statement::Unresolved(body), model.clone())?
                } else {
                    return Err("Локальная функция должна иметь тело".into());
                };
                Ok(FunctionNode::Local {
                    upper: Some(Rc::downgrade(&model)),
                    name: name.clone(),
                    params,
                    ret: rett,
                    body: statement,
                })
            }
        }
    } else if let FunctionNode::None = func {
        Err("Функция не определена".into())
    } else {
        Ok(func)
    }
}

// ── Тесты ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::parse;
    use crate::semantic::tree::construct_model;
    use crate::semantic::{FunctionNode, TypeNode};

    /// Строит модель и возвращает корневой ModelNode.
    fn build(src: &str) -> Result<crate::semantic::ModelNode, crate::diagnostics::Diagnostic> {
        let (ast, _) = parse(src, 0).expect("ошибка разбора");
        construct_model(&ast, None, &[]).map(|m| m.take())
    }

    // ── Внешние функции ───────────────────────────────────────────────────────

    /// `extern fn foo(x: bit);` разрешается в `FunctionNode::External`.
    ///
    /// # Пример (BuT)
    /// ```but
    /// extern fn send(data: bit);
    /// ```
    #[test]
    fn extern_fn_no_params_resolves_to_external() {
        let node = build("extern fn foo(x: bit);").unwrap();
        match node.functions.get("foo").expect("функция foo не найдена") {
            FunctionNode::External {
                name, params, ret, ..
            } => {
                assert_eq!(name, "foo");
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].0, "x");
                assert_eq!(params[0].1, TypeNode::Bit);
                assert_eq!(*ret, TypeNode::Unit);
            }
            other => panic!("ожидался External, получен {:?}", other),
        }
    }

    /// `extern fn foo() -> bit;` — внешняя функция с возвращаемым типом.
    #[test]
    fn extern_fn_with_return_type() {
        let node = build("extern fn status() -> bit;").unwrap();
        match node.functions.get("status").expect("status не найдена") {
            FunctionNode::External { ret, .. } => assert_eq!(*ret, TypeNode::Bit),
            other => panic!("ожидался External, получен {:?}", other),
        }
    }

    /// `extern fn` без параметров — пустой список параметров.
    #[test]
    fn extern_fn_no_return_type_defaults_to_unit() {
        let node = build("extern fn noop();").unwrap();
        match node.functions.get("noop").expect("noop не найдена") {
            FunctionNode::External { params, ret, .. } => {
                assert!(params.is_empty(), "параметров быть не должно");
                assert_eq!(*ret, TypeNode::Unit);
            }
            other => panic!("ожидался External, получен {:?}", other),
        }
    }

    // ── Локальные функции ─────────────────────────────────────────────────────

    /// `fn id(x: bit) -> bit { return true; }` разрешается в `FunctionNode::Local`.
    ///
    /// Параметры функции (`x`) сейчас не добавляются в область видимости модели,
    /// поэтому тело использует литерал `true` вместо параметра `x`.
    ///
    /// # Пример (BuT)
    /// ```but
    /// fn ready() -> bit {
    ///     return true;
    /// }
    /// ```
    #[test]
    fn local_fn_resolves_to_local() {
        let node = build("fn id(x: bit) -> bit { return true; }").unwrap();
        match node.functions.get("id").expect("id не найдена") {
            FunctionNode::Local {
                name, params, ret, ..
            } => {
                assert_eq!(name, "id");
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].0, "x");
                assert_eq!(params[0].1, TypeNode::Bit);
                assert_eq!(*ret, TypeNode::Bit);
            }
            other => panic!("ожидался Local, получен {:?}", other),
        }
    }

    /// `fn noop() { }` — функция без параметров и без возвращаемого типа.
    #[test]
    fn local_fn_no_params_no_return() {
        let node = build("fn noop() { }").unwrap();
        match node.functions.get("noop").expect("noop не найдена") {
            FunctionNode::Local { params, ret, .. } => {
                assert!(params.is_empty());
                assert_eq!(*ret, TypeNode::Unit);
            }
            other => panic!("ожидался Local, получен {:?}", other),
        }
    }

    /// `fn add(a: bit, b: bit) -> bit { return true; }` — несколько параметров.
    ///
    /// Тело использует литерал, т.к. параметры не в области видимости модели.
    #[test]
    fn local_fn_multiple_params() {
        let node = build("fn add(a: bit, b: bit) -> bit { return true; }").unwrap();
        match node.functions.get("add").expect("add не найдена") {
            FunctionNode::Local { params, .. } => {
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].0, "a");
                assert_eq!(params[0].1, TypeNode::Bit);
                assert_eq!(params[1].0, "b");
                assert_eq!(params[1].1, TypeNode::Bit);
            }
            other => panic!("ожидался Local, получен {:?}", other),
        }
    }

    /// Функция с псевдонимом типа в параметре: `type u8 = [bit;8]; extern fn foo(x: u8);`.
    #[test]
    fn extern_fn_alias_param_resolves() {
        let node = build("type u8 = [bit;8]; extern fn foo(x: u8);").unwrap();
        match node.functions.get("foo").expect("foo не найдена") {
            FunctionNode::External { params, .. } => {
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].1, TypeNode::Array(8, Box::new(TypeNode::Bit)));
            }
            other => panic!("ожидался External, получен {:?}", other),
        }
    }

    // ── Контрпримеры ──────────────────────────────────────────────────────────

    /// Контрпример: нельзя объявить функцию без имени.
    ///
    /// Примечание: такой синтаксис не проходит парсер, поэтому проверяем
    /// напрямую через `construct_function` с вручную созданным узлом.
    #[test]
    fn function_none_node_is_error() {
        use super::*;
        use std::cell::RefCell;
        use std::rc::Rc;
        let model = Rc::new(RefCell::new(ModelNode::default()));
        let result = construct_function(FunctionNode::None, model);
        assert!(result.is_err(), "FunctionNode::None должен давать ошибку");
    }

    /// Контрпример: `FunctionNode::Local` уже разрешён — возвращается без изменений.
    #[test]
    fn already_resolved_local_passthrough() {
        use super::*;
        use crate::semantic::Statement;
        use std::cell::RefCell;
        use std::rc::Rc;
        let local = FunctionNode::Local {
            upper: None,
            name: "f".into(),
            params: vec![],
            ret: TypeNode::Unit,
            body: Statement::None,
        };
        let model = Rc::new(RefCell::new(ModelNode::default()));
        let result = construct_function(local.clone(), model).unwrap();
        assert_eq!(result, local);
    }
}
