//! Порядок стадий построения семантического дерева (фича 0130).
//!
//! Вынесено из `tree.rs` (лимит размера) и заодно по смыслу: «какие стадии и в
//! каком порядке» — самостоятельное знание, а сами стадии живут рядом.

use crate::diagnostics::{Diagnostic, FileTable};
use crate::parser::ast::Model;
use crate::semantic::ModelNode;
use crate::semantic::tree::{
    construct_model_stage0, construct_model_stage1, construct_model_stage2, construct_model_stage3,
    construct_model_stage4, construct_model_stage5, construct_model_stage6,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Строит дерево **без проверок** — стадии 0–6.
///
/// Нужен тому, кому нужны **все** диагностики проверок, а не первая:
/// `validate_model` терминален по контракту, а `validate_model_all` принимает
/// уже построенное дерево. Порядок стадий описан здесь **один раз**:
/// `construct_model_with_files` выражен через эту же функцию.
///
/// ⚠️ Стадии остаются **терминальными** (решение ADR 0130): после ошибки
/// построения дерево неполно, и продолжение дало бы сообщения о следствиях, а не
/// о причинах.
pub(crate) fn construct_stages(
    model: &Model,
    upper: Option<Rc<RefCell<ModelNode>>>,
    search_paths: &[String],
    files: &mut FileTable,
) -> Result<Rc<RefCell<ModelNode>>, Diagnostic> {
    // Стек путей файлов, чьи импорты сейчас обрабатываются.
    // Пустой на входе: текущая (корневая) единица компиляции не имеет пути.
    let mut import_stack: Vec<String> = Vec::new();
    let model = construct_model_stage0(model, upper, search_paths, &mut import_stack, files)?;
    let model = construct_model_stage1(model)?;
    let model = construct_model_stage2(model)?;
    let model = construct_model_stage3(model)?;
    // Функции (этап 5) разрешаются перед именованными блоками (этап 4): блоки
    // always/enter/exit находят уже разрешённые функции через `search_func`.
    let model = construct_model_stage5(model)?;
    let model = construct_model_stage4(model)?;
    construct_model_stage6(model)
}
