//! Порядок стадий построения семантического дерева (фича 0130).
//!
//! Вынесено из `tree.rs` (лимит размера) и заодно по смыслу: «какие стадии и в
//! каком порядке» — самостоятельное знание, а сами стадии живут рядом.

use crate::diagnostics::{Diagnostic, FileTable};
use crate::parser::ast::Model;
use crate::semantic::ModelNode;
pub(crate) mod body_stages;

use crate::semantic::tree::{
    construct_model_stage0, construct_model_stage1, construct_model_stage2, construct_model_stage3,
};
use body_stages::{construct_model_stage4, construct_model_stage5, construct_model_stage6};
use std::cell::RefCell;
use std::rc::Rc;

/// Строит дерево **без проверок** — стадии 0–6.
///
/// Нужен тому, кому нужны **все** диагностики проверок, а не первая:
/// `validate_model` терминален по контракту, а `validate_model_all` принимает
/// уже построенное дерево. Порядок стадий описан здесь **один раз**:
/// `construct_model_with_files` выражен через эту же функцию.
///
/// ⚠️ **Между стадиями** переход терминален (решение ADR 0130): выход каждой
/// стадии — предпосылка для следующей, и продолжение через предпосылку дало бы
/// сообщения о следствиях, а не о причинах.
///
/// ⚠️ **Внутри** стадий 4–6 диагностики **накапливаются** (фича 0152): их
/// элементы — соседи (именованные блоки, тела функций), от разрешённого
/// элемента не зависит другой, и проба показала, что вторая диагностика там —
/// самостоятельная причина. Стадии 0–3 строят предпосылки (имена, составные
/// состояния, переменные, `cond`) и остаются терминальными.
///
/// ⚠️ Наблюдаемую пользу дают стадии **4 и 5**. У стадии 6 накопление заведено
/// ради единообразия, но входа, который отказал бы именно в ней, замер 0152 не
/// нашёл: цель ребра разрешает стадия 0 (`SE-002`), а неразрешённое условие —
/// `validate` (`SE-025`).
///
/// ⚠️ Неполное дерево **не покидает эту функцию**: при непустом списке ошибок
/// наружу идёт `Err`, а дерево отбрасывается. Поэтому потребители — семь целей,
/// симулятор и верификация — частично построенной модели не видят никогда, и
/// признак неполноты не нужен: гарантия по построению сильнее проверки, которую
/// каждый потребитель обязан помнить.
/// `specialize == true` — режим `--parameters=specialize` (фича 0185): между
/// стадиями 1 и 2 инстанцирования с аргументами заменяются копиями моделей с
/// подставленными значениями. Точка выбрана не случайно: после стадии 1
/// аргументы уже вычислены, а тела ещё сырые — копия разрешит их на **свои**
/// переменные штатными стадиями 2–6 (см. шапку `semantic/specialize.rs`).
pub(crate) fn construct_stages(
    model: &Model,
    upper: Option<Rc<RefCell<ModelNode>>>,
    search_paths: &[String],
    files: &mut FileTable,
    specialize: bool,
) -> Result<Rc<RefCell<ModelNode>>, Vec<Diagnostic>> {
    // Стек путей файлов, чьи импорты сейчас обрабатываются.
    // Пустой на входе: текущая (корневая) единица компиляции не имеет пути.
    let mut import_stack: Vec<String> = Vec::new();
    construct_stages_within(
        model,
        upper,
        search_paths,
        &mut import_stack,
        files,
        specialize,
    )
}

/// То же, что [`construct_stages`], но со **стеком импорта вызывающего**
/// (фича 0296).
///
/// Нужна пути импорта: подключённый файл строится этими же стадиями, но обязан
/// видеть стек, иначе цикл `a.takt → b.takt → a.takt` не обнаружится. Отдельная
/// функция, а не параметр публичного входа, — чтобы корневой вызов не мог
/// случайно передать чужой стек.
///
/// ⚠️ **Второго перечисления стадий в проекте нет и быть не должно** (фича
/// 0296). Прежде путь импорта (`tree.rs::construct_model_impl`) нёс свою копию
/// последовательности, и та отстала на три прохода: `collect_clock`,
/// `specialize_instantiations`, `constify_parameters` не выполнялись для
/// подключённого файла вовсе. Цена измерена: `SE-067`, `SE-068` и контракт
/// `clock` через границу импорта **молчали**, а `--parameters=specialize`
/// порождал невалидный C при нулевом коде возврата. Сторож — греп-тест
/// `takt-lang/tests/semantic/stage_order_single_source_tests.rs`.
pub(crate) fn construct_stages_within(
    model: &Model,
    upper: Option<Rc<RefCell<ModelNode>>>,
    search_paths: &[String],
    import_stack: &mut Vec<String>,
    files: &mut FileTable,
    specialize: bool,
) -> Result<Rc<RefCell<ModelNode>>, Vec<Diagnostic>> {
    let ast = model;
    // Стадии 0–3 терминальны: одна диагностика оборачивается в список, чтобы
    // тип возврата был един для всех стадий.
    let one = |d: Diagnostic| vec![d];
    let model = construct_model_stage0(model, upper, search_paths, import_stack, files, specialize)
        .map_err(one)?;
    // Частота тактирования (фича 0134): собирается по АСД отдельным проходом —
    // она свойство единицы компиляции, а не отдельного элемента модели.
    crate::semantic::time_ast::collect_clock(ast, &model).map_err(one)?;
    let model = construct_model_stage1(model).map_err(one)?;
    if specialize {
        crate::semantic::specialize::specialize_instantiations(&model).map_err(one)?;
        // Вывод константности (задача 0185-06) — **после** специализации и до
        // стадии 2: специализация подставляет значения в объявления
        // (`VariableNode::Simple`), а этот проход заменяет объявление
        // константой. Обратный порядок сломал бы подстановку, а флип после
        // стадии 2 потребовал бы мутировать оба представления переменной
        // (засада 0096) и опоздал бы к `after PARAM`/адресу порта.
        crate::semantic::parameter_const::constify_parameters(&model);
    }
    let model = construct_model_stage2(model).map_err(one)?;
    let model = construct_model_stage3(model).map_err(one)?;
    // Функции (этап 5) разрешаются перед именованными блоками (этап 4): блоки
    // always/enter/exit находят уже разрешённые функции через `search_func`.
    //
    // ⚠️ Переход между стадиями 5 → 4 → 6 остаётся терминальным несмотря на то,
    // что каждая из них накапливает **внутри** себя: разрешённые функции —
    // предпосылка для тел блоков, а тела блоков — для условий рёбер. Слить их
    // накопления значило бы выпустить каскад, ради недопущения которого 0130 и
    // сделала стадии терминальными.
    let model = construct_model_stage5(model)?;
    let model = construct_model_stage4(model)?;
    construct_model_stage6(model)
}
