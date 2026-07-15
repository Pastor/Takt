//! Генератор Structured Text (IEC 61131-3) из семантического дерева Lam.
//!
//! Модуль транслирует семантическое дерево [`ModelNode`] в файл `.st` —
//! текстовый язык программируемых логических контроллеров (ПЛК), стандарт
//! IEC 61131-3. Фича 0041; архитектурное решение — ADR 0041 (Option A).
//!
//! ## Схема отображения
//!
//! - Модель Lam → `FUNCTION_BLOCK` (единственная конструкция IEC с сохраняемым
//!   между вызовами состоянием — прямой аналог `struct` + `_tick()` цели `c`).
//! - Состояния → `CASE state OF` по переменной состояния.
//! - Тело `FUNCTION_BLOCK` = один цикл сканирования ПЛК = один такт Lam.
//!
//! ## Цели
//!
//! - `st` — чистый IEC 61131-3, адреса портов не потребляются.
//! - `st-at` — плюс размещение портов по карте адресов (`AT %IX…`/`%QX…`),
//!   включается [`GenerateOptions::hal`]; реализуется задачей 0041-05.
//!
//! ## Состояние реализации
//!
//! Задача **0041-01** закрыла **каркас и диспетчеризацию**: цели, публичный API,
//! снимок карты и скелет `FUNCTION_BLOCK` на каждую модель. Задача **0041-02**
//! добавила отображение типов (`st_type.rs`) и секции объявлений
//! (`st_decl.rs`). Остаток:
//!
//! | Задача | Что добавляет |
//! |---|---|
//! | 0041-03 | `st_model.rs` — `CASE state OF`, переходы, композиция |
//! | 0041-04 | `st_expr.rs` — выражения, условия, операторы |
//! | 0041-05 | `AT %…` — потребление карты адресов (цель `st-at`) |
//!
//! ## Почему вывод пока не принимается `iec2c`
//!
//! Проба 0041-06 предполагала, что для валидности достаточно объявлений, и
//! ставила это критерием приёмки 0041-02. Проверка **опровергла** предположение:
//! `iec2c` требует от `FUNCTION_BLOCK` ещё и **тело** — блок с одними
//! объявлениями отвергается («no body defined in function block declaration»), и
//! комментарий за тело не считается. Тело — `CASE state OF` — предмет задачи
//! **0041-03**; до неё гейт закрыть нельзя. Заглушку-тело генератор намеренно
//! **не** эмитит: она уехала бы в ПЛК под видом логики.

mod st_decl;
mod st_expr;
mod st_map;
mod st_stmt;
mod st_type;

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::GenerateOptions;
use crate::generator::Generator as AsGenerator;
use crate::generator::indent::Printer;
use crate::semantic::ModelNode;
use crate::semantic::minimap::{Element, Name};
use crate::semantic::naming::normalize_lowercase_snakecase;
use crate::semantic::unused::UsageSet;
use st_map::StMap;
use std::cell::RefCell;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::rc::Rc;

/// Размер одного уровня отступа в порождаемом ST.
const INDENT: usize = 4;

/// Генератор Structured Text для модели Lam.
pub struct Generator {}

impl AsGenerator for Generator {
    fn generate(
        &self,
        model: &ModelNode,
        output_path: &str,
        options: &GenerateOptions,
    ) -> Result<(), Diagnostic> {
        let map = StMap::new(
            &normalize_lowercase_snakecase(model.name().to_string()),
            model,
            options.hal,
        )?;
        let program = generate_program(&map)?;
        let filename = map.get_filename();
        let _ = fs::create_dir(Path::new(output_path));
        fs::write(
            Path::new(output_path).join(filename.to_owned() + ".st"),
            program,
        )
        .map_err(|e| {
            Diagnostic::error(Location::Codegen, format!("{:?}", e)).with_code("ST-001")
        })?;
        Ok(())
    }
}

/// Строит текст ST-программы из снимка модели.
///
/// Эмитится заголовок файла, общие объявления `TYPE … END_TYPE` и по одному
/// `FUNCTION_BLOCK … END_FUNCTION_BLOCK` на корневую модель и на каждую
/// используемую подмодель — с секциями объявлений (0041-02). Тело блоков
/// наполняют задачи 0041-03…0041-05.
fn generate_program(map: &StMap) -> Result<String, Diagnostic> {
    let Element::Model { .. } = map.model() else {
        return Err(Diagnostic::error(
            Location::Codegen,
            "Корневой элемент карты не является моделью".to_string(),
        )
        .with_code("ST-012"));
    };

    let mut out = String::new();
    let mut p = Printer::new(INDENT, &mut out);

    p.ident("(*").nl();
    p.ident(" * Порождено компилятором Lam (lamc) — цель: Structured Text (IEC 61131-3).")
        .nl();
    p.ident(" * Не редактировать вручную: файл перезаписывается при каждой генерации.")
        .nl();
    p.ident(" *)").nl().nl();

    // Подмодели объявляются раньше корня: FUNCTION_BLOCK, используемый как тип
    // экземпляра, должен быть известен к моменту объявления экземпляра.
    //
    // Порядок фиксируется сортировкой по уникальному имени. Это не косметика:
    // `used_models()` отдаёт модели в порядке обхода `HashMap`, то есть **разном
    // от запуска к запуску** (пять прогонов `lamc -t st examples/stacker.lam`
    // дали четыре разных файла). В IEC 61131-3 порядок объявлений **значим** —
    // тип экземпляра обязан быть объявлен раньше использования, — поэтому
    // случайный порядок здесь дороже, чем в C: он делает вывод то валидным, то
    // нет. Сортировка даёт воспроизводимую сборку и устойчивый вход для
    // проверки MatIEC (задача 0041-06).
    //
    // Первопричина — `HashMap` в семантическом слое (`semantic/mod.rs`), она
    // общая для целей `c`/`plantuml` и чинится отдельным кандидатом
    // «Генерация C недетерминирована» (`FEATURES.md`); здесь снимается только
    // следствие в своём бэкенде.
    let mut submodels: Vec<_> = map
        .using_models()
        .into_iter()
        .filter_map(|element| match element {
            Element::Model { name, .. } => Some(name),
            _ => None,
        })
        .collect();
    submodels.sort_by(|a, b| a.unique().cmp(b.unique()));

    // Блоки строятся в порядке «подмодели → корень»; корень идёт последним по
    // той же причине, что и сортировка выше.
    let mut blocks: Vec<(Name, Rc<RefCell<ModelNode>>)> = Vec::new();
    for name in submodels {
        let model = map.raw_model_at(name.clone())?;
        blocks.push((name, model));
    }
    let root = map
        .root_model_node()
        .ok_or_else(|| root_missing(map.root_name()))?;
    blocks.push((map.root_name(), root));

    // Объявления структур — общие для файла и печатаются раньше всех блоков:
    // в IEC 61131-3 тип обязан быть известен к моменту использования.
    st_decl::emit_struct_types(&mut p, &blocks)?;

    for (name, model) in &blocks {
        emit_function_block(
            &mut p,
            &name.unique_camelcase(),
            &model.borrow(),
            map.usage(),
        )?;
    }

    Ok(out)
}

/// Строит диагностику `ST-012` — снимок карты не содержит корневой модели.
fn root_missing(name: Name) -> Diagnostic {
    Diagnostic::error(
        Location::Codegen,
        format!("Корневая модель '{}' отсутствует в снимке карты", name),
    )
    .with_code("ST-012")
}

/// Печатает один `FUNCTION_BLOCK`: заголовок, секции объявлений, тело.
///
/// Тело (`CASE state OF`) — задача 0041-03; сейчас на его месте комментарий.
/// Из-за этого `iec2c` блок пока отвергает: IEC 61131-3 требует тела, и
/// комментарий за него не считается (см. заметку в шапке модуля).
fn emit_function_block(
    p: &mut Printer,
    name: &str,
    model: &ModelNode,
    usage: &UsageSet,
) -> Result<(), Diagnostic> {
    let mut header = String::new();
    let _ = write!(header, "FUNCTION_BLOCK {}", name);
    p.ident(&header).nl();
    st_decl::emit_declarations(p, model, usage)?;
    p.up();
    p.ident("(* Тело: задачи 0041-03…0041-05 (CASE state OF, выражения, адреса). *)")
        .nl();
    p.down();
    p.ident("END_FUNCTION_BLOCK").nl().nl();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::tree::construct_model;

    /// Строит снимок ST-карты из исходника Lam (по образцу `plantuml::tests::make_map`).
    fn make_map(src: &str, name: &str) -> StMap {
        let (ast, _) = crate::parse(src, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some(name.to_string());
        let model = model_rc.borrow();
        StMap::new(name, &model, false).unwrap()
    }

    fn program_of(src: &str, name: &str) -> String {
        generate_program(&make_map(src, name)).unwrap()
    }

    /// Корневая модель должна порождать `FUNCTION_BLOCK` — несущую конструкцию ST.
    #[test]
    fn test_generate_program_root_model_emits_function_block() {
        let st = program_of("start S;", "Root");
        assert!(
            st.contains("FUNCTION_BLOCK Root"),
            "отсутствует FUNCTION_BLOCK корневой модели:\n{st}"
        );
        assert!(
            st.contains("END_FUNCTION_BLOCK"),
            "отсутствует END_FUNCTION_BLOCK:\n{st}"
        );
    }

    /// Комментарий в IEC 61131-3 — `(* … *)`; C-формы недопустимы.
    ///
    /// Проверка не косметическая: `//` и `/* */` — синтаксическая ошибка для
    /// компилятора ST, то есть порождённый файл не приняла бы ни одна среда ПЛК.
    #[test]
    fn test_generate_program_uses_iec_comments_not_c_style() {
        let st = program_of("start S;", "Root");
        assert!(st.starts_with("(*"), "ожидался IEC-комментарий:\n{st}");
        assert!(!st.contains("/*"), "C-комментарий недопустим в ST:\n{st}");
        assert!(!st.contains("//"), "C-комментарий недопустим в ST:\n{st}");
    }

    /// Каждый открытый `FUNCTION_BLOCK` должен быть закрыт.
    #[test]
    fn test_generate_program_every_function_block_is_closed() {
        let src = "model A { start S; } start E = A;";
        let st = program_of(src, "Root");
        assert_eq!(
            st.matches("FUNCTION_BLOCK ").count(),
            st.matches("END_FUNCTION_BLOCK").count(),
            "число открытий и закрытий FUNCTION_BLOCK должно совпадать:\n{st}"
        );
    }

    /// Подмодель должна порождать собственный `FUNCTION_BLOCK`.
    ///
    /// Имя — **уникальное** (с путём родителей), а не локальное: подмодель `A`
    /// модели `Root` даёт `RootA`. Так же именует цель `c`
    /// (`STACKER_LIFT_CONTROLLER`), и того же требует ADR 0041
    /// (`FUNCTION_BLOCK StackerLiftController`). Причина — в IEC 61131-3
    /// пространство имён `FUNCTION_BLOCK` **плоское**: одноимённые подмодели
    /// разных родителей столкнулись бы.
    #[test]
    fn test_generate_program_submodel_emits_own_function_block() {
        let src = "model A { start S; } start E = A;";
        let st = program_of(src, "Root");
        assert!(
            st.contains("FUNCTION_BLOCK RootA"),
            "отсутствует FUNCTION_BLOCK подмодели:\n{st}"
        );
    }

    /// Подмодель объявляется раньше корня: в IEC 61131-3 тип экземпляра должен
    /// быть известен к моменту объявления экземпляра.
    #[test]
    fn test_generate_program_submodel_declared_before_root() {
        let src = "model A { start S; } start E = A;";
        let st = program_of(src, "Root");
        let sub = st.find("FUNCTION_BLOCK RootA").expect("нет подмодели");
        let root = st.find("FUNCTION_BLOCK Root\n").expect("нет корня");
        assert!(
            sub < root,
            "подмодель должна объявляться раньше корня:\n{st}"
        );
    }

    /// Вывод ST должен быть **воспроизводимым**: одна модель — один и тот же файл.
    ///
    /// Сторож против регресса недетерминизма. `used_models()` отдаёт модели в
    /// порядке обхода `HashMap`, поэтому без явной сортировки пять прогонов
    /// `lamc -t st examples/stacker.lam` давали **четыре разных** файла. Для ST
    /// это не косметика: порядок объявлений в IEC 61131-3 значим.
    ///
    /// Тест строит карту заново на каждой итерации — иначе он проверял бы кэш,
    /// а не обход.
    #[test]
    fn test_generate_program_output_is_deterministic() {
        let src = "model A { start S; } model B { start T; } model C { start U; } \
                   start E = A | B | C;";
        let first = program_of(src, "Root");
        for i in 1..8 {
            assert_eq!(
                first,
                program_of(src, "Root"),
                "прогон {i} дал другой вывод — вернулся недетерминизм порядка"
            );
        }
    }

    /// Подмодели печатаются в устойчивом (лексикографическом) порядке.
    #[test]
    fn test_generate_program_submodels_in_stable_order() {
        let src = "model B { start T; } model A { start S; } start E = A | B;";
        let st = program_of(src, "Root");
        let a = st.find("FUNCTION_BLOCK RootA").expect("нет блока A");
        let b = st.find("FUNCTION_BLOCK RootB").expect("нет блока B");
        assert!(
            a < b,
            "порядок подмоделей должен быть устойчивым, а не зависеть от обхода HashMap:\n{st}"
        );
    }

    /// Параллельная композиция даёт по `FUNCTION_BLOCK` на каждую подмодель.
    #[test]
    fn test_generate_program_parallel_composition_emits_block_per_submodel() {
        let src = "model A { start S; } model B { start T; } start E = A | B;";
        let st = program_of(src, "Root");
        assert!(st.contains("FUNCTION_BLOCK RootA"), "нет блока A:\n{st}");
        assert!(st.contains("FUNCTION_BLOCK RootB"), "нет блока B:\n{st}");
        assert_eq!(
            st.matches("END_FUNCTION_BLOCK").count(),
            3,
            "ожидались блоки A, B и корня:\n{st}"
        );
    }
}
