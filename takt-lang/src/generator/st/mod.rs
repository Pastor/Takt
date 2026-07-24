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

mod st_at;
mod st_decl;
mod st_expr;
mod st_fixed;
mod st_func;
mod st_map;
mod st_model;
mod st_reserved;
mod st_stmt;
mod st_type;

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::GenerateOptions;
use crate::generator::Generator as AsGenerator;
use crate::generator::indent::Printer;
use crate::semantic::minimap::{Element, Name};
use crate::semantic::naming::normalize_lowercase_snakecase;
use crate::semantic::{ModelNode, VariableNode};
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
            options.address_map.clone(),
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
    p.ident(" * Порождено компилятором Lam (taktc) — цель: Structured Text (IEC 61131-3).")
        .nl();
    p.ident(" * Не редактировать вручную: файл перезаписывается при каждой генерации.")
        .nl();
    p.ident(" *)").nl().nl();

    // Подмодели объявляются раньше корня: FUNCTION_BLOCK, используемый как тип
    // экземпляра, должен быть известен к моменту объявления экземпляра.
    //
    // Порядок фиксируется сортировкой по уникальному имени. Это не косметика:
    // `used_models()` отдаёт модели в порядке обхода `HashMap`, то есть **разном
    // от запуска к запуску** (пять прогонов `taktc -t st examples/stacker.lam`
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
    // Модель обязана быть объявлена РАНЬШЕ той, что заводит её экземпляр:
    // опережающие ссылки в ST — нестандартное расширение (`iec2c -p`).
    // Порядок топологический, а не по глубине вложенности: зависимость бывает и
    // между СОСЕДЯМИ (`E` содержит `F`, оба — подмодели корня), и одной лишь
    // «глубже — раньше» тут мало (поймано гейтом на `extend_complex`).
    submodels.sort_by(|a, b| a.unique().cmp(b.unique()));
    submodels = topological_order(map, submodels);

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
    // Функции — тоже раньше: опережающие ссылки в ST нестандартны (`iec2c -p`).
    let warnings = st_func::emit_functions(&mut p, &blocks)?;

    let root_name = map.root_name();
    for (name, model) in &blocks {
        let is_root = name.unique() == root_name.unique();
        emit_function_block(&mut p, map, name, &model.borrow(), is_root)?;
    }

    // Цель `st-at` порождает программу для ПЛК ЦЕЛИКОМ, а не библиотеку блоков:
    // размещённые порты живут в `VAR_GLOBAL`, а он вне `CONFIGURATION`
    // недопустим (проба П8). Цель `st` обёртки не требует (П2) — цели
    // асимметричны намеренно.
    if map.at_addresses() {
        emit_configuration(&mut p, map, &root_name, &blocks)?;
    }

    report(&warnings);
    // Q-хелпер LAM_Q_FLOORDIV (0061) вставляется перед первым POU по факту вызова.
    Ok(st_fixed::insert_helper(out))
}

/// Печатает `PROGRAM` и `CONFIGURATION` с размещёнными портами (цель `st-at`).
///
/// Форма проверена пробами П3 (обёртка) и П8 (`VAR_GLOBAL … AT %…` внутри
/// `CONFIGURATION`).
fn emit_configuration(
    p: &mut Printer,
    map: &StMap,
    root_name: &Name,
    blocks: &[(Name, Rc<RefCell<ModelNode>>)],
) -> Result<(), Diagnostic> {
    let fb = root_name.unique_camelcase();
    p.ident(&format!("PROGRAM {}Main", fb)).nl();
    p.ident("VAR").nl();
    p.up();
    p.ident(&format!("inst : {};", fb)).nl();
    p.down();
    p.ident("END_VAR").nl();
    p.up();
    p.ident("inst();").nl();
    p.down();
    p.ident("END_PROGRAM").nl().nl();

    // Размещённые порты собираются заранее: пустой `VAR_GLOBAL … END_VAR`
    // недопустим («no variable declared in global variable(s) declaration»), а
    // модель без портов — не ошибка (например, `comprehensive.lam`).
    let mut placed: Vec<String> = Vec::new();
    let mut warnings = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    // Порты собираются со ВСЕХ моделей, а не только с корня: в `elevator_mini`
    // они объявлены внутри под-моделей (`out ElevatorMotor_Up: bit;` в `Motor`).
    // Пропустить их значило бы оставить `VAR_EXTERNAL` без глобала — «the
    // external variable does not match with any global variable».
    for (model_name, model_rc) in blocks {
        let root = &*model_rc.borrow();
        let mut names: Vec<&String> = root.variables.keys().collect();
        names.sort();
        for name in names {
            let VariableNode::Port {
                name: pname,
                ty,
                direction,
                ..
            } = &root.variables[name]
            else {
                continue;
            };
            if !map.usage().ports.contains(pname) || seen.contains(pname) {
                continue;
            }
            // Порт без адреса при `st-at` — уже ошибка слоя 0020 (SE-052), сюда
            // он не доходит; но если карта пуста, размещать нечего.
            // Фича 0084: карта ключуется квалифицированно (модель+порт) — тот же
            // ключ, что строит продюсер `resolve_model`.
            let key = crate::address_map::qualified_port_key(model_name.unique(), pname);
            let Some(resolved) = map.address_of(&key) else {
                continue;
            };
            seen.push(pname.clone());
            let (location, comment, mut w) = st_at::location_of(pname, ty, *direction, resolved)?;
            warnings.append(&mut w);
            let ty_name = st_type::get_st_type(ty, root)?;
            placed.push(format!(
                "{} AT {} : {}; {}",
                pname, location, ty_name, comment
            ));
        }
    }
    // Порядок объявления глобалов на семантику не влияет, но должен быть
    // воспроизводимым: `variables` — HashMap.
    placed.sort();

    p.ident(&format!("CONFIGURATION {}Config", fb)).nl();
    p.up();
    if !placed.is_empty() {
        p.ident("VAR_GLOBAL").nl();
        p.up();
        for line in &placed {
            p.ident(line).nl();
        }
        p.down();
        p.ident("END_VAR").nl();
    }
    p.ident("RESOURCE Res0 ON PLC").nl();
    p.up();
    p.ident("TASK Tick(INTERVAL := T#100ms, PRIORITY := 0);")
        .nl();
    p.ident(&format!("PROGRAM Inst0 WITH Tick : {}Main;", fb))
        .nl();
    p.down();
    p.ident("END_RESOURCE").nl();
    p.down();
    p.ident("END_CONFIGURATION").nl().nl();
    report(&warnings);
    Ok(())
}

/// Показывает предупреждения генератора (`ST-009`, `ST-010`).
///
/// Молчание здесь недопустимо: `ST-009` сообщает, что тело внешней функции
/// подменено заглушкой, а `ST-010` — что LTL-формула в ПЛК не поедет. И то и
/// другое пользователь обязан увидеть.
fn report(warnings: &[Diagnostic]) {
    for w in warnings {
        let code = w.code.as_deref().unwrap_or("ST");
        eprintln!("Предупреждение [{}]: {}", code, w.message);
    }
}

/// Упорядочивает подмодели так, чтобы каждая шла после тех, кого использует.
///
/// Алгоритм Кана по отношению «заводит экземпляр». Вход отсортирован
/// лексикографически, поэтому и результат воспроизводим (`used_models()` обходит
/// `HashMap`, то есть отдаёт модели в разном порядке от запуска к запуску).
///
/// Цикл в зависимостях невозможен: модель не может содержать саму себя. Но если
/// он вдруг возникнет, остаток дописывается как есть — тогда `iec2c` пожалуется
/// на опережающую ссылку, то есть **громко**, а не молча.
fn topological_order(map: &StMap, models: Vec<Name>) -> Vec<Name> {
    let deps: Vec<(Name, Vec<String>)> = models
        .iter()
        .map(|n| (n.clone(), map.instantiated_by(n)))
        .collect();
    let mut placed: Vec<Name> = Vec::new();
    let mut rest: Vec<(Name, Vec<String>)> = deps;

    while !rest.is_empty() {
        let ready: Vec<Name> = rest
            .iter()
            .filter(|(_, d)| d.iter().all(|dep| placed.iter().any(|p| p.unique() == dep)))
            .map(|(n, _)| n.clone())
            .collect();
        if ready.is_empty() {
            // Цикл: дописываем остаток, не теряя ни одной модели.
            placed.extend(rest.into_iter().map(|(n, _)| n));
            break;
        }
        for name in &ready {
            placed.push(name.clone());
        }
        rest.retain(|(n, _)| !ready.iter().any(|r| r.unique() == n.unique()));
    }
    placed
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
/// **Тело печатается дважды.** Сначала «вхолостую», в отдельный буфер: только
/// напечатав его, генератор узнаёт, что нужно объявить — поднятые из тела
/// переменные (`st_stmt`) и экземпляры под-FB (`st_model`). А объявления в POU
/// обязаны стоять **до** тела, и второй секции `VAR` быть не может. Поэтому
/// сперва буфер, затем шапка, затем готовый текст тела.
fn emit_function_block(
    p: &mut Printer,
    map: &StMap,
    name: &Name,
    model: &ModelNode,
    is_root: bool,
) -> Result<(), Diagnostic> {
    let element = if is_root {
        map.model()
    } else {
        map.element_of(name)
            .ok_or_else(|| root_missing(name.clone()))?
    };
    let Element::Model { states, .. } = &element else {
        return Err(root_missing(name.clone()));
    };
    let table = st_model::StateTable::build(states);

    // Переменные корня под-модель видит через `VAR_IN_OUT` (О1-в): указателей,
    // как `main->` в цели `c`, в переносимом ST нет.
    let shared = if is_root {
        Vec::new()
    } else {
        map.shared_variables(name)
    };

    let mut body = String::new();
    let out = {
        let mut bp = Printer::new(INDENT, &mut body);
        bp.up();
        st_model::emit_body(&mut bp, map, &element, model, &table)?
    };

    let extras = st_decl::Extras {
        state_var: true,
        is_done: true,
        external_ports: map.at_addresses(),
        shared,
        instances: out
            .instances
            .iter()
            .map(|i| (i.name.clone(), i.fb_type.clone()))
            .collect(),
        hoisted: out
            .stmt
            .hoisted
            .iter()
            .map(|h| (h.name.clone(), h.ty.clone()))
            .collect(),
    };

    // Имя FB совпадает с эмитируемым идентификатором. У корня оно берётся из
    // имени файла (проба 2: `concat.lam` → `FUNCTION_BLOCK Concat`, а `CONCAT` —
    // стандартная функция IEC), поэтому проверять надо именно эту строку.
    let fb_name = name.unique_camelcase();
    st_reserved::check_st_name(&fb_name, model.loc)?;
    let mut header = String::new();
    let _ = write!(header, "FUNCTION_BLOCK {}", fb_name);
    p.ident(&header).nl();
    st_decl::emit_declarations(p, model, map.usage(), &extras)?;
    p.print(&body);
    p.ident("END_FUNCTION_BLOCK").nl().nl();

    report(&out.stmt.warnings);
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
        StMap::new(name, &model, false, Default::default()).unwrap()
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
    /// `taktc -t st examples/stacker.lam` давали **четыре разных** файла. Для ST
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
