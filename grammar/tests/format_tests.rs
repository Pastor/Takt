//! Тесты форматтера (фича 0024): идемпотентность и семантическая нейтральность.
//!
//! # Что здесь важнее отдельных примеров
//!
//! Форматтер печатает **всё** дерево, поэтому его нельзя проверить парой
//! фикстур: любой непокрытый узел молча испортил бы исходник пользователя.
//! Поэтому основная проверка — прогон по **всему корпусу** `.lam` репозитория
//! (`examples/`, `grammar/tests/data/`, `simulation/tests/data/`) с двумя
//! инвариантами:
//!
//! - **A1, идемпотентность:** `fmt(fmt(x)) == fmt(x)`;
//! - **A3, семантическая нейтральность:** `parse(fmt(x))` структурно равен
//!   `parse(x)` — форматтер меняет раскладку, но не смысл.
//!
//! Тест печатает **сводку покрытия**: сколько файлов корпуса форматируется, и
//! **падает при появлении нового непокрытого узла**. Так «не отформатировано» не
//! превращается в тихую норму.

use grammar::format::{FormatError, format_source};
use std::path::{Path, PathBuf};

/// Собирает все `.lam` репозитория.
fn corpus() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("корень репозитория")
        .to_path_buf();
    let mut files = Vec::new();
    for dir in ["examples", "grammar/tests/data", "simulation/tests/data"] {
        collect(&root.join(dir), &mut files);
    }
    files.sort();
    files
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "lam") {
            out.push(path);
        }
    }
}

/// Структурное сравнение АСД без учёта `Location`.
///
/// `Location` намеренно игнорируется: форматтер **обязан** сдвигать позиции —
/// в этом и состоит его работа. Сравнивается всё остальное.
fn ast_eq_ignoring_locations(
    a: &grammar::parser::ast::Model,
    b: &grammar::parser::ast::Model,
) -> bool {
    // `Debug`-представление содержит `Location`, поэтому вырезаем их текстом:
    // это дёшево и не требует отдельного обхода дерева.
    fn strip(model: &grammar::parser::ast::Model) -> String {
        let text = format!("{model:?}");
        let mut out = String::with_capacity(text.len());
        let mut rest = text.as_str();
        while let Some(i) = rest.find("Source(") {
            out.push_str(&rest[..i]);
            let Some(close) = rest[i..].find(')') else {
                break;
            };
            out.push_str("Source(…)");
            rest = &rest[i + close + 1..];
        }
        out.push_str(rest);
        out
    }
    strip(a) == strip(b)
}

#[test]
fn a1_a3_corpus_report() {
    let files = corpus();
    assert!(!files.is_empty(), "корпус .lam не найден");

    let mut formatted = 0usize;
    let mut unsupported: Vec<(String, String)> = Vec::new();
    let mut idempotency_failures: Vec<String> = Vec::new();
    let mut semantic_failures: Vec<String> = Vec::new();
    let mut parse_failures = 0usize;

    for path in &files {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();
        let Ok(source) = std::fs::read_to_string(path) else {
            continue;
        };
        // Невалидные фикстуры (`data/semantic/invalid/`) могут не разбираться —
        // это не дефект форматтера.
        if grammar::parse(&source, 0).is_err() {
            parse_failures += 1;
            continue;
        }

        match format_source(&source) {
            Err(FormatError::Unsupported(node)) => unsupported.push((name, node)),
            Err(FormatError::Parse(_)) => parse_failures += 1,
            Ok(once) => {
                formatted += 1;
                // A1: идемпотентность.
                match format_source(&once) {
                    Ok(twice) if twice == once => {}
                    Ok(_) => idempotency_failures.push(name.clone()),
                    Err(e) => idempotency_failures.push(format!("{name}: повторный прогон: {e}")),
                }
                // A3: семантическая нейтральность.
                match (grammar::parse(&source, 0), grammar::parse(&once, 0)) {
                    (Ok((before, _)), Ok((after, _))) => {
                        if !ast_eq_ignoring_locations(&before, &after) {
                            semantic_failures.push(name.clone());
                        }
                    }
                    _ => semantic_failures.push(format!("{name}: результат не разбирается")),
                }
            }
        }
    }

    // Сводка покрытия — печатается всегда: это метрика прогресса задачи 0024-01.
    let total = files.len();
    eprintln!("\n── Форматтер: покрытие корпуса ──");
    eprintln!("  всего .lam:            {total}");
    eprintln!("  отформатировано:       {formatted}");
    eprintln!("  не разбирается:        {parse_failures} (невалидные фикстуры — норма)");
    eprintln!("  узел не поддержан:     {}", unsupported.len());
    let mut kinds: Vec<&str> = unsupported.iter().map(|(_, k)| k.as_str()).collect();
    kinds.sort_unstable();
    kinds.dedup();
    for kind in kinds {
        let count = unsupported.iter().filter(|(_, k)| k == kind).count();
        eprintln!("      {count:3} × {kind}");
    }

    assert!(
        idempotency_failures.is_empty(),
        "A1 (идемпотентность) нарушена: {idempotency_failures:?}"
    );
    assert!(
        semantic_failures.is_empty(),
        "A3 (семантическая нейтральность) нарушена — форматтер изменил смысл: {semantic_failures:?}"
    );
    // Гейт: единственными причинами «не отформатировано» могут быть ИЗВЕСТНЫЕ и
    // задокументированные пробелы. Любой НОВЫЙ непокрытый узел валит тест — это
    // и есть защита от тихой потери куска исходника.
    //
    // Комментарии блокируют 136 из 158 файлов корпуса: печать комментариев
    // (задача 0024-02) — не деталь, а условие проверяемости ядра. Пока она не
    // сделана, A1/A3 на корпусе проверить нельзя; см. документ задачи 0024-01.
    const KNOWN_GAPS: &[&str] = &["комментарии (печать — задача 0024-02)", "InlineFormula"];
    let unexpected: Vec<&(String, String)> = unsupported
        .iter()
        .filter(|(_, kind)| !KNOWN_GAPS.contains(&kind.as_str()))
        .collect();
    assert!(
        unexpected.is_empty(),
        "появился НЕизвестный непокрытый узел (тихая потеря исходника недопустима): {unexpected:?}"
    );
}

#[test]
fn a3_semantics_preserved_on_operators() {
    // Инвариант фичи 0021: `=` — сравнение, `:=` — присваивание. Подмена одного
    // другим при печати изменила бы смысл программы молча.
    let source = "var x: u8 := 1;\nstate A {\n    ref B: x = 1;\n}\n";
    let out = format_source(source).unwrap();
    assert!(out.contains("x: u8 := 1"), "присваивание: {out}");
    assert!(out.contains("ref B: x = 1"), "сравнение в условии: {out}");
}

#[test]
fn idempotent_on_messy_input() {
    let messy = "start    S ;\n\n\n\nstate   A{ref B:x>1;}";
    let once = format_source(messy).unwrap();
    let twice = format_source(&once).unwrap();
    assert_eq!(once, twice, "fmt(fmt(x)) должно равняться fmt(x)");
}

#[test]
fn a2_comments_of_all_three_kinds_survive() {
    // R2/A2: ни один комментарий не теряется. Три вида: `//`, `///`, `/* */`,
    // в ведущей и хвостовой позиции.
    let source = r#"/// документация модели
// обычный ведущий
var x: u8 := 0; // хвостовой
/* блочный */
start S;
"#;
    let out = format_source(source).unwrap();
    for expected in [
        "/// документация модели",
        "// обычный ведущий",
        "// хвостовой",
        "/* блочный */",
    ] {
        assert!(
            out.contains(expected),
            "потерян комментарий {expected:?}:\n{out}"
        );
    }
    // Хвостовой обязан остаться на строке своего узла, а не уехать отдельно.
    assert!(
        out.contains("var x: u8 := 0; // хвостовой"),
        "хвостовой комментарий оторвался от узла:\n{out}"
    );
}

#[test]
fn a2_comments_are_idempotent() {
    // Комментарии — главный риск идемпотентности: при повторном прогоне они не
    // должны ни дублироваться, ни смещаться.
    let source = "// ведущий\nvar x: u8 := 0; // хвостовой\nstart S;\n";
    let once = format_source(source).unwrap();
    let twice = format_source(&once).unwrap();
    assert_eq!(once, twice);
    assert_eq!(
        once.matches("// ведущий").count(),
        1,
        "дубль комментария:\n{once}"
    );
}
