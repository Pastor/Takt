//! Смоук языка сообщений: под `en` ни одно сообщение корпуса не несёт кириллицы.
//!
//! Проверка каталогов (`check-messages.py`) доказывает, что перевод есть, а проверка
//! литералов доказывает, что места эмиссии зовут каталог. Этот тест смотрит на то, что видит
//! человек: прогоняет корпус через разбор, семантику, предупреждения, форматтер и
//! все цели и проверяет **напечатанные** тексты. Язык ставится на поток
//! (`lang::activate`), поэтому соседние тесты, идущие параллельно, его не видят.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use takt_lang::compile::{CompileInput, Target, compile_texts};
use takt_lang::diagnostics::{Diagnostic, lang};
use takt_lang::format::FormatError;

fn fixtures() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let mut files = Vec::new();
    collect(&root, &mut files);
    files.sort();
    files
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("каталог фикстур {} не читается: {e}", dir.display()));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "takt") {
            out.push(path);
        }
    }
}

fn has_cyrillic(text: &str) -> bool {
    text.chars().any(|c| matches!(c, 'А'..='я' | 'Ё' | 'ё'))
}

/// Собирает сообщения и префиксы их кодов; нарушителей называет с местом.
#[derive(Default)]
struct Probe {
    checked: usize,
    prefixes: BTreeSet<String>,
    offenders: Vec<String>,
}

impl Probe {
    fn see(&mut self, label: &str, diagnostic: &Diagnostic) {
        self.checked += 1;
        let code = diagnostic.code.as_deref().unwrap_or("?");
        if let Some((prefix, _)) = code.split_once('-') {
            self.prefixes.insert(prefix.to_string());
        }
        let notes = diagnostic.notes.iter().map(|n| n.message.as_str());
        for text in std::iter::once(diagnostic.message.as_str()).chain(notes) {
            if has_cyrillic(text) {
                self.offenders.push(format!("{label}: [{code}] {text}"));
            }
        }
    }
}

#[test]
fn no_cyrillic_in_messages_under_english() {
    lang::activate(lang::parse("en").expect("каталог en есть в дереве"));
    let search = vec![
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data/include")
            .display()
            .to_string(),
    ];
    let generate = takt_lang::GenerateOptions::new(true);
    let mut probe = Probe::default();
    let files = fixtures();

    for file in &files {
        let Ok(source) = std::fs::read_to_string(file) else {
            continue;
        };
        let name = file.display().to_string();
        for d in takt_lang::collect_compile_diagnostics(&name, &source, &search, false) {
            probe.see(&name, &d);
        }
        if let Ok((ast, _)) = takt_lang::parse(&source, 0)
            && let Ok(model) = takt_lang::semantic::tree::construct_model(&ast, None, &search)
        {
            for d in takt_lang::semantic::warnings::collect_model_warnings(&ast, &model) {
                probe.see(&name, &d);
            }
        }
        match takt_lang::format::format_source_with_warnings(&source) {
            Ok((_, style)) => style.iter().for_each(|d| probe.see(&name, d)),
            Err(FormatError::Parse(diagnostics)) => {
                diagnostics.iter().for_each(|d| probe.see(&name, d))
            }
            Err(FormatError::Unsupported(d)) => probe.see(&name, &d),
        }
        for target in Target::ALL {
            let input = CompileInput::new(&name, &source, &search, &generate);
            match compile_texts(target, &input) {
                Ok(output) => output.warnings.iter().for_each(|d| probe.see(&name, d)),
                Err(d) => probe.see(&name, &d),
            }
        }
    }

    // Коды, которых корпус не порождает: внешняя карта, `--define`, строка LTL.
    if let Err(diagnostics) = takt_lang::parse_address_map("= 0x10;", 0) {
        diagnostics
            .iter()
            .for_each(|d| probe.see("карта адресов", d));
    }
    if let Err(diagnostics) = takt_lang::parse_defines(&["BAD".to_string()]) {
        diagnostics.iter().for_each(|d| probe.see("--define", d));
    }
    if let Err(d) = takt_lang::parse_ltl_property("G Idle; start X { ref X; }") {
        probe.see("строка LTL", &d);
    }
    lang::reset();

    eprintln!(
        "── Язык en: проверено {} сообщений на {} фикстурах, префиксы {:?} ──",
        probe.checked,
        files.len(),
        probe.prefixes
    );
    for prefix in ["LE", "SY", "SE", "CS", "CC", "RS", "ST", "SV", "AM", "DF"] {
        assert!(
            probe.prefixes.contains(prefix),
            "префикс {prefix} не встретился — смоук его не проверил: {:?}",
            probe.prefixes
        );
    }
    assert!(
        probe.offenders.is_empty(),
        "кириллица в сообщениях под en:\n{}",
        probe.offenders.join("\n")
    );
}
