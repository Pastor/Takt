//! Фикстуры `valid/` проверяются по своему ОБЕЩАНИЮ (фича 0288).
//!
//! # Что было
//!
//! Сторож фикстуры обычно выглядел так:
//!
//! ```ignore
//! /// Ce6: Файл ce6_type_from_func.takt разбирается без ошибок.
//! #[test]
//! fn example_ce6_type_from_func_valid() {
//!     build_file("…/ce6_type_from_func.takt").expect("должен разбираться");
//! }
//! ```
//!
//! — то есть **констатация разбора**, тогда как сама фикстура обещает
//! конкретное поведение («переменная получает тип возвращаемого значения
//! функции»). Такой сторож проходит всё время, пока обещанное **не работает**:
//! ровно это случилось с `ce6_type_inference_chain` (фича 0204 — вывод по
//! ссылке не работал, а тест был зелёным), и класс тот же, что у фикса 0010-01
//! (проверяли структуру автомата вместо языка).
//!
//! Замер 2026-08-20: из 57 тестов на фикстурах `valid/` **30** не имели ни
//! одной содержательной проверки.
//!
//! # Что здесь
//!
//! Сторожа тех фикстур, чьё обещание сформулировано **конкретно**: тип из
//! функции, локальная переменная блока, перечисление, доступ к элементу
//! массива, именованные блоки. Каждый тест проверяет **то, что обещано**, а не
//! факт разбора.
//!
//! ⚠️ Остальные констатации не переписаны молча: они перечислены в
//! `scripts/fixture-guard-baseline.txt` — реестр с ратчетом, который не имеет
//! права расти (гейт `scripts/check-fixture-guards.py`).

use std::cell::RefCell;
use std::rc::Rc;
use takt_lang::parse;
use takt_lang::semantic::tree::construct_model;

/// Строит дерево фикстуры.
fn build(path: &str) -> Rc<RefCell<takt_lang::semantic::ModelNode>> {
    let source = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let (ast, _) = parse(&source, 0).unwrap_or_else(|d| panic!("{path}: разбор: {d:?}"));
    construct_model(&ast, None, &[]).unwrap_or_else(|d| panic!("{path}: семантика: {d:?}"))
}

/// Тип объявления верхнего уровня — как его видит потребитель.
fn ty_of(model: &Rc<RefCell<takt_lang::semantic::ModelNode>>, name: &str) -> String {
    let model = model.borrow();
    let var = model
        .variables
        .get(name)
        .unwrap_or_else(|| panic!("объявление '{name}' не найдено"));
    format!("{:?}", var.ty())
}

/// **Ce6.** `var result := getbool();` получает тип **возвращаемого значения**
/// функции — это и обещает фикстура.
///
/// ⚠️ Прежний сторож проверял, что файл разбирается: он прошёл бы и в мире, где
/// вывод типа из вызова не работает вовсе.
#[test]
fn type_is_inferred_from_function_return() {
    let model = build("tests/data/semantic/valid/ce6_type_from_func.takt");
    assert_eq!(
        ty_of(&model, "result"),
        "Bool",
        "тип обязан прийти от возвращаемого значения `getbool()`"
    );
}

/// **С4.** Локальная переменная блока объявлена и **разрешена**: обещание
/// фикстуры — «доступна в последующих операторах того же блока».
///
/// Разрешённость проверяется отсутствием `Unresolved` в напечатанном дереве
/// тела: неразрешённое имя осталось бы там как `Unresolved(...)`.
#[test]
fn local_variable_of_block_is_resolved() {
    let model = build("tests/data/semantic/valid/local_var_in_block.takt");
    let printed = format!("{:?}", model.borrow().states);
    // Локальная переменная стала узлом объявления в теле блока.
    assert!(
        printed.contains("name: \"x\""),
        "локальная переменная блока в дереве не найдена:\n{printed}"
    );
    // Тело блока РАЗРЕШЕНО: `Always { body: Block([…]) }`, а не `Unresolved`.
    //
    // ⚠️ Проверять «в дереве нет Unresolved» вовсе нельзя: условия рёбер `ref`
    // по инварианту проекта остаются `Condition::Unresolved` (см. живой
    // контекст), и такой сторож падал бы на любой корректной модели.
    assert!(
        printed.contains("Always { upper: Some((Weak)), body: Block("),
        "тело блока `always` не разрешено:\n{printed}"
    );
}

/// **Ce4.** Фикстура перечислений: объявление `enum` доезжает до дерева со
/// **своими вариантами**.
///
/// ⚠️ Проверяется `enum_basic.takt`, а не `ce4_enum_basic.takt`: второй, вопреки
/// имени, перечислений не содержит вовсе — его комментарий обещает «enum
/// добавляется программно». Это находка фичи 0288: имя обещало одно, содержимое
/// другое.
#[test]
fn enum_fixture_carries_its_variants() {
    let model = build("tests/data/semantic/valid/enum_basic.takt");
    // Перечисление объявлено во вложенной модели `M` — искать его надо там же,
    // где оно написано, а не у корня.
    let printed = format!("{:?}", model.borrow().models);
    for variant in ["North", "South", "East", "West"] {
        assert!(
            printed.contains(variant),
            "вариант перечисления '{variant}' не доехал до дерева"
        );
    }
}

/// **Массивы.** Фикстура доступа к элементу обещает индексацию — значит в
/// дереве есть узел доступа, а не просто «файл разобрался».
#[test]
fn array_access_fixture_has_subscript() {
    let model = build("tests/data/semantic/valid/array_access.takt");
    // Индексация стоит в ИНИЦИАЛИЗАТОРАХ объявлений (`var second := bus[1];`),
    // а не в теле состояния: смотреть надо туда, где она написана.
    // ⚠️ База — массив НЕ битов: у упакованного `[bit;N]` индекс означает
    // разряд и сводится к `BitAccess` (фича 0533), поэтому обещание фикстуры
    // «здесь есть индексация элемента» держит именно `bus`.
    let printed = format!("{:?}", model.borrow().variables);
    assert!(
        printed.contains("ArraySubscript"),
        "фикстура доступа к массиву не дала узла индексации:\n{printed}"
    );
}

/// **Именованные блоки.** Фикстура обещает `enter`/`exit`/`always` — значит все
/// три доезжают до дерева состояния.
#[test]
fn named_blocks_fixture_carries_all_three() {
    let model = build("tests/data/semantic/valid/named_blocks.takt");
    let printed = format!("{:?}", model.borrow().states);
    // Имена блоков печатаются вариантами перечисления — `Enter`/`Exit`/`Always`.
    for block in ["Enter {", "Exit {", "Always {"] {
        assert!(
            printed.contains(block),
            "именованный блок '{block}' не доехал до дерева:\n{printed}"
        );
    }
}
