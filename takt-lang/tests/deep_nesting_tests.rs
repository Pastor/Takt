//! Сторож предела вложенности выражений и условий (фича 0129).
//!
//! ## Что здесь ловится
//!
//! Построение семантического дерева рекурсивно по структуре текста, поэтому
//! глубоко вложенное выражение съедало стек: инструменты падали с `SIGABRT`
//! (`has overflowed its stack`) **без диагностики**. Замер до правки: глубина
//! 100 проходила, 300 — роняла `taktc compile`, `taktc verify` и `takt-sim`.
//!
//! Теперь глубина ограничена (`validate::depth::MAX_NESTING_DEPTH`), и выход за
//! предел даёт `SE-062`.
//!
//! ## Почему тесты идут через публичный API, а не через внутренности
//!
//! Проверяется наблюдаемое поведение компилятора: пользователь видит либо
//! диагностику, либо падение. Тест на внутренний счётчик этого не докажет —
//! счётчик может работать, а точка входа остаться не прикрытой (ровно так и
//! случилось со сторожем операторов, см. отчёт фичи).

use takt_lang::semantic::tree::construct_model;

/// Предел вложенности, продублированный числом намеренно.
///
/// Константа `validate::depth::MAX_NESTING_DEPTH` живёт в `pub(crate)`-модуле и
/// интеграционному тесту недоступна, а делать её публичной значит расширять API
/// ради теста. Синхронность держит парный сторож `constant_matches_documented_limit`
/// в самом модуле: изменение константы красит его, и обе стороны правятся
/// осознанно.
const MAX_NESTING_DEPTH: usize = 32;

/// Модель с условием ребра из `depth` вложенных скобок.
fn model_with_nested_parens(depth: usize) -> String {
    format!(
        "var x: u8 := 0;\nstart S {{\n  ref S: {}x{} = 1;\n}}\n",
        "(".repeat(depth),
        ")".repeat(depth)
    )
}

/// Модель с присваиванием, где значение обёрнуто в `depth` скобок.
fn model_with_nested_expression(depth: usize) -> String {
    format!(
        "var x: u8 := 0;\nstart S {{\n  always {{ x := {}1{}; }}\n  ref S: x = 9;\n}}\n",
        "(".repeat(depth),
        ")".repeat(depth)
    )
}

fn build(src: &str) -> Result<(), String> {
    let (ast, _) = takt_lang::parse(src, 0).map_err(|e| format!("разбор: {e:?}"))?;
    construct_model(&ast, None, &[])
        .map(|_| ())
        .map_err(|d| format!("{}|{}", d.code.clone().unwrap_or_default(), d.message))
}

/// Глубина, заведомо укладывающаяся в предел.
///
/// Счётчик меряет глубину **узлов дерева**, а не число скобок: условие
/// `(((x))) = 1` добавляет к скобкам ещё и узел сравнения, поэтому «ровно
/// `MAX_NESTING_DEPTH` скобок» — уже перебор. Берём запас, чтобы тест проверял
/// правило, а не арифметику обёрток.
const WITHIN_LIMIT: usize = MAX_NESTING_DEPTH - 8;

#[test]
fn nesting_within_limit_is_accepted() {
    // Сторож направления: предел не должен «сползти» вниз от правок — обычные
    // выражения обязаны строиться как раньше.
    let err = build(&model_with_nested_parens(WITHIN_LIMIT));
    assert!(
        err.is_ok(),
        "глубина в пределах лимита обязана строиться: {err:?}"
    );
}

#[test]
fn nesting_beyond_limit_is_diagnosed_not_crash() {
    let err = build(&model_with_nested_parens(MAX_NESTING_DEPTH + 1))
        .expect_err("превышение предела обязано давать диагностику");
    assert!(
        err.starts_with("SE-062|"),
        "ожидался SE-062, получено: {err}"
    );
}

#[test]
fn deep_nesting_that_used_to_crash_is_diagnosed() {
    // Именно эта глубина роняла все три инструмента до фичи 0129.
    let err = build(&model_with_nested_parens(300)).expect_err("300 уровней — диагностика");
    assert!(
        err.starts_with("SE-062|"),
        "ожидался SE-062, получено: {err}"
    );
}

#[test]
fn deep_nesting_in_expression_is_diagnosed_too() {
    // Условие ребра и выражение в теле — разные точки рекурсии
    // (`resolve_condition` и `construct_expression`), прикрыть надо обе.
    let err = build(&model_with_nested_expression(300))
        .expect_err("глубокое выражение в теле — диагностика");
    assert!(
        err.starts_with("SE-062|"),
        "ожидался SE-062, получено: {err}"
    );
}

#[test]
fn diagnostic_names_the_limit() {
    // Сообщение обязано называть предел: без числа пользователь не поймёт, к
    // чему стремиться.
    let err = build(&model_with_nested_parens(300)).expect_err("диагностика");
    assert!(
        err.contains(&MAX_NESTING_DEPTH.to_string()),
        "сообщение обязано называть предел {MAX_NESTING_DEPTH}: {err}"
    );
}

#[test]
fn counter_does_not_leak_between_builds() {
    // Счётчик потоковый; если бы выход из рекурсии не учитывался, вторая сборка
    // в том же потоке упёрлась бы в остаток от первой. Строим подряд несколько
    // моделей у самого предела — все обязаны пройти.
    for _ in 0..3 {
        assert!(
            build(&model_with_nested_parens(WITHIN_LIMIT)).is_ok(),
            "остаток счётчика от предыдущей сборки"
        );
    }
}
