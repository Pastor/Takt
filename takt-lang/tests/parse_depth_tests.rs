//! Сторожа глубины разбора (фича 0156).
//!
//! ## Что здесь ловится
//!
//! Дерево АСД строится **до** семантики, и над ним работают рекурсии, которых
//! сторож `SE-062` (фича 0129) не покрывает: печать форматтера, производные
//! `Clone` и `Drop`. Замер стадии архитектуры (ADR 0156): самый низкий потолок —
//! печать, ≈ 200 уровней в потоке 2 МиБ; `Clone` — ≈ 561; `Drop` — ≈ 18 750.
//!
//! ## Почему тесты идут в потоке с 2 МиБ стека
//!
//! Главный поток CLI имеет 8 МиБ, и в нём потолки вчетверо выше — тест в главном
//! потоке доказал бы меньше, чем нужно. 2 МиБ — консервативная оценка рабочего
//! потока сервера (столько же по умолчанию у потока `cargo test`), то есть самое
//! стеснённое из поддерживаемых окружений.

/// Стек потока, в котором гоняются пробы глубины.
const STACK_2MIB: usize = 2 * 1024 * 1024;

/// Модель с выражением из `depth` вложенных скобок.
fn source_with_parens(depth: usize) -> String {
    format!(
        "var x: u8 := 0;\nstart S {{\n  always {{ x := {}1{}; }}\n  ref S: x = 9;\n}}\n",
        "(".repeat(depth),
        ")".repeat(depth)
    )
}

/// Запускает работу в потоке с 2 МиБ стека и возвращает её результат.
///
/// Падение потока (переполнение стека) убивает **процесс**, а не поток, поэтому
/// «тест не прошёл» здесь выглядит как оборванный прогон — это и есть искомый
/// сигнал: инструмент не имеет права падать без диагностики.
fn in_small_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(STACK_2MIB)
        .spawn(work)
        .expect("поток пробы")
        .join()
        .expect("поток пробы завершился аварийно")
}

#[test]
fn deep_parentheses_do_not_crash_the_parser() {
    // Действие грамматики `"(" Expression ")"` клонировало поддерево на каждом
    // уровне (фича 0156, задача 01), поэтому разбор скобок падал уже на глубине
    // ≈ 561 в потоке 2 МиБ — **до** всякой проверки и без диагностики. После
    // перехода на перемещение значения потолок разбора поднялся до потолка
    // `Drop` (≈ 18 750), и эта глубина проходит с запасом.
    //
    // Тест намеренно не требует `Ok`: с введением предела (задача 04) тот же
    // ввод отвергается диагностикой. Проверяется, что вызов **завершается**, а
    // не роняет процесс.
    let outcome = in_small_stack(|| {
        let src = source_with_parens(5_000);
        takt_lang::parse(&src, 0).is_ok()
    });
    // Значение важно лишь как признак того, что вызов вернулся.
    let _ = outcome;
}

/// Модель с цепочкой из `count` слагаемых — глубокое дерево при нулевом балансе
/// скобок.
fn source_with_chain(count: usize) -> String {
    format!(
        "var x: u8 := 0;\nstart S {{\n  always {{ x := {}; }}\n  ref S: x = 9;\n}}\n",
        vec!["1"; count].join("+")
    )
}

/// Код первой диагностики отказа разбора.
fn parse_error_code(src: &str) -> String {
    let diagnostics = takt_lang::parse(src, 0).expect_err("глубокий ввод обязан отвергаться");
    diagnostics
        .first()
        .and_then(|d| d.code.clone())
        .unwrap_or_default()
}

#[test]
fn deep_parentheses_are_rejected_with_a_diagnostic() {
    let code = in_small_stack(|| parse_error_code(&source_with_parens(3_000)));
    assert_eq!(code, "SY-005", "ожидался предел разбора");
}

#[test]
fn deep_operator_chain_is_rejected_too() {
    // Баланс скобок здесь нулевой: счёт идёт по узлам дерева, а не по скобкам.
    let code = in_small_stack(|| parse_error_code(&source_with_chain(3_000)));
    assert_eq!(code, "SY-005", "цепочка без скобок обязана ловиться так же");
}

#[test]
fn rejection_message_carries_position_and_limit() {
    // Диагностика без координат и без числа бесполезна: пользователь не узнает
    // ни где, ни к чему стремиться (урок `SE-062`, фича 0129).
    let (loc_is_source, names_limit) = in_small_stack(|| {
        let diagnostics = takt_lang::parse(&source_with_parens(3_000), 0).expect_err("отказ");
        let first = diagnostics.first().expect("диагностика").clone();
        (
            matches!(first.loc, takt_lang::diagnostics::Location::Source(..)),
            first.message.contains("96"),
        )
    });
    assert!(loc_is_source, "диагностика обязана нести позицию в файле");
    assert!(names_limit, "сообщение обязано называть предел");
}

#[test]
fn formatting_deep_source_fails_instead_of_crashing() {
    // Самый низкий потолок из всех потребителей — печать форматтера (≈ 200
    // уровней в потоке 2 МиБ). Именно этот путь семантику не зовёт, поэтому до
    // фичи 0156 `taktc fmt` и форматирование в редакторе падали без диагностики.
    let reported = in_small_stack(|| {
        let error = takt_lang::format::format_source(&source_with_parens(3_000))
            .expect_err("глубокий ввод обязан отвергаться форматтером");
        format!("{error:?}")
    });
    assert!(
        reported.contains("SY-005"),
        "форматтер обязан сообщить о пределе разбора: {reported}"
    );
}

#[test]
fn rejecting_a_tree_deeper_than_drop_survives() {
    // Утилизация отвергнутого дерева (задача 0156-03). Потолок производного
    // `Drop` — ≈ 18 750 уровней в потоке 2 МиБ; без разбора дерева на плоские
    // узлы отказ на таком вводе ронял бы процесс ровно в момент возврата
    // диагностики.
    let code = in_small_stack(|| parse_error_code(&source_with_chain(200_000)));
    assert_eq!(code, "SY-005", "глубочайший ввод обязан отвергаться");
}

#[cfg(feature = "lsp")]
#[test]
fn lsp_reports_the_limit_to_the_editor() {
    // Правило 29: новая диагностика обязана доходить до редактора. Путь тот же,
    // что у действующих `SY-001…004` (`lsp/diagnostics.rs` → `crate::parse`), но
    // проверяется прогоном, а не рассуждением.
    let count =
        in_small_stack(|| takt_lang::lsp::collect_diagnostics(&source_with_parens(3_000)).len());
    assert_eq!(count, 1, "редактор обязан получить ровно одну диагностику");
}

#[test]
fn moderate_parentheses_still_parse_into_a_tree() {
    // Сторож направления: правка действия грамматики не должна менять разбор
    // обычных скобок — умеренная вложенность обязана давать дерево, а не отказ.
    let parsed = in_small_stack(|| {
        let src = source_with_parens(8);
        takt_lang::parse(&src, 0).is_ok()
    });
    assert!(parsed, "восемь уровней скобок обязаны разбираться");
}
