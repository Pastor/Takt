//! Общий конвейер: разбор → построение дерева → сбор диагностик.
//!
//! Вынесено из `lib.rs` (фича 0130): файл пришпилен реестром размеров, а логика
//! здесь самостоятельная — «как получить модель (или все ошибки) из текста».

use crate::diagnostics::{self, Diagnostic};
use crate::{parse, semantic};

/// Разбирает и строит модель, проставляя диагностике **путь её файла** (фича 0053).
///
/// Общий шаг всех целей. Заведён потому, что путь нужно разрешить там, где
/// [`FileTable`](diagnostics::FileTable) ещё жив: реестр — деталь компиляции и
/// наружу не выходит, а `Location` несёт лишь номер файла. Без этого диагностика
/// из импортированной библиотеки была неотличима от своей — `taktc` печатал обе
/// дословно одинаково.
pub(crate) fn parse_and_construct(
    filename: &str,
    source: &str,
    search_paths: &[String],
    specialize: bool,
) -> Result<std::rc::Rc<std::cell::RefCell<semantic::ModelNode>>, Diagnostic> {
    let mut files = diagnostics::FileTable::new(filename);

    // Корневой файл — номер 0 (его зарегистрировал `FileTable::new`).
    let (model_ast, _) = parse(source, 0).map_err(|ds| {
        let d = ds.into_iter().next().unwrap();
        stamp_file(d, &files)
    })?;

    semantic::tree::construct_model_with_files(
        &model_ast,
        None,
        search_paths,
        &mut files,
        specialize,
    )
    .map_err(|d| stamp_file(d, &files))
}

/// Разрешает номер файла диагностики в путь.
fn stamp_file(d: Diagnostic, files: &diagnostics::FileTable) -> Diagnostic {
    let path = files.path_of(&d.loc).map(str::to_string);
    d.with_file_if_unset(path.as_deref())
}

/// **Все** ошибки исходного текста за один прогон (фича 0130).
///
/// Тот же конвейер, что у `compile_to_*` (`parse` → построение дерева →
/// проверки), но результат — **список**, а не первая встреченная ошибка.
/// Пустой список означает, что модель строится.
///
/// # Зачем отдельный вход
///
/// `compile_to_*` возвращают `Result<_, Diagnostic>` — одну ошибку, и менять их
/// сигнатуры ради вывода значило бы править семь целей и 183 вызова
/// (`construct_model`). Вместо этого потребитель, которому нужны **все** ошибки,
/// зовёт эту функцию до компиляции; так делает и `taktc`, и языковой сервер —
/// и потому отвечают одинаково. Прежде расходились: сервер перечислял все ошибки
/// разбора, а CLI печатал первую, хотя оба зовут один [`parse`].
///
/// # Что уже накоплено, а что нет
///
/// - **Разбор** отдаёт все свои ошибки (лексер + парсер) — они и так собирались
///   в `Vec`, но терялись на стыке: вызывающий брал первую.
/// - **Построение дерева** остаётся терминальным: после ошибки дерево неполно, и
///   продолжение дало бы сообщения о следствиях, а не о причинах (решение
///   ADR 0130).
/// - **Проверки** (`validate`) высказываются все: они идут по готовому дереву и
///   независимы друг от друга.
///
/// # Почему вход знает про режим параметров
///
/// `specialize` — режим `--parameters=specialize` (фича 0185). Диагностика от
/// него **зависит** и зависеть обязана: параметр в позиции compile-time величины
/// (`after PARAM`, адрес порта) законен в `specialize` и отвергается в `assign`
/// (`SE-088`, R12) — это единственное место, где режим виден не в форме вывода, а
/// в множестве принимаемых программ. Прогон с жёстко зашитым `false` печатал бы
/// `SE-088` о программе, которую сам же компилятор в этом режиме собирает, — то
/// есть отказ на верном входе. Параметр продет **одной** сигнатурой, как и у
/// `parse_and_construct` (0185-05): два входа «с режимом» и «без» разошлись бы.
///
/// Список упорядочен по позиции в тексте и не содержит точных повторов
/// ([`diagnostics::normalize`]).
pub fn collect_compile_diagnostics(
    filename: &str,
    source: &str,
    search_paths: &[String],
    specialize: bool,
) -> Vec<Diagnostic> {
    let mut files = diagnostics::FileTable::new(filename);

    let model_ast = match parse(source, 0) {
        Ok((ast, _)) => ast,
        Err(ds) => {
            let stamped = ds.into_iter().map(|d| stamp_file(d, &files)).collect();
            return diagnostics::normalize(stamped);
        }
    };

    // Стадии построения и проверки разделены намеренно. Слитно (через
    // `construct_model_with_files`) получить всё нельзя — тот вход отдаёт первую
    // ошибку по контракту.
    //
    // ⚠️ С фичи 0152 стадии построения тоже отдают **список**: внутри стадий
    // 4–6 (тела блоков, функций, условия рёбер) диагностики накапливаются, между
    // стадиями — нет. Неполное дерево при этом наружу не выходит, поэтому здесь
    // на руках либо готовая модель, либо только диагностики.
    let model = match semantic::stages::construct_stages(
        &model_ast,
        None,
        search_paths,
        &mut files,
        specialize,
    ) {
        Ok(model) => model,
        Err(ds) => {
            let stamped = ds.into_iter().map(|d| stamp_file(d, &files)).collect();
            return diagnostics::normalize(stamped);
        }
    };

    let found = semantic::validate::validate_model_all(model)
        .into_iter()
        .map(|d| stamp_file(d, &files))
        .collect();
    diagnostics::normalize(found)
}
