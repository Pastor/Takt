//! `textDocument/documentSymbol` — фича 0147.
//!
//! # Зачем этот слой существует
//!
//! Возможность **объявлена** клиенту в `ServerCapabilities`
//! (`document_symbol_provider`), то есть редактор её вызывает при каждом
//! открытии файла. Тестов на неё не было ни одного: сервер отвечал вслепую.
//! Замер покрытия (фича 0138, 2026-07-27) дал по `lsp/symbols.rs` **0 %** при
//! 183 строках; ноль перепроверен грепом отдельно — артефактом учёта он не был.
//!
//! # Что проверяется
//!
//! Три свойства ответа, и каждое ломается по-своему:
//!
//! 1. **состав** — какие объявления вообще становятся символами (и какие
//!    намеренно не становятся: `import`, `address`, `clock`, формулы);
//! 2. **вложенность** — модель → её элементы, состояние → именованные блоки,
//!    перечисление → варианты, структура → поля. Плоский список вместо дерева
//!    компилируется и выглядит «работающим», но панель структуры бесполезна;
//! 3. **диапазоны** — `selection_range` обязан покрывать **имя** (по нему
//!    редактор переходит из панели), а `range` — всё объявление и содержать
//!    `selection_range` внутри себя. Перепутанные диапазоны уводят курсор не
//!    туда, и никакой тест на «состав» этого не заметит.
//!
//! Ожидания сняты **зондом** с фактического вывода, а не выведены из чтения
//! кода: позиции, номера строк и виды символов угадывать нельзя.
//!
//! ⚠️ Тесты LSP живут под `#[cfg(feature = "lsp")]`: обычная `cargo test` их не
//! видит. Гоняет `cargo test --all-features` в `precheck.sh` (фича 0178).

#[cfg(feature = "lsp")]
mod symbols {
    use lsp_types::{DocumentSymbol, Position, Range, SymbolKind};
    use takt_lang::lsp::document_symbols;

    /// Файл со **всеми** видами объявлений верхнего уровня, какие модуль умеет
    /// показывать, плюс те, что он намеренно пропускает (`import`).
    const SRC: &str = r#"import { Thermostat } from "t.takt";
type Speed = u8;
enum Mode { Idle, Run }
struct Point { x: u8, y: u8 }
const LIMIT: u8 := 7;
out flag: bit := 0;
var speed: Speed := 0;
cond Fast = speed > 5;
invariant Safe = speed < 100;
fn bump(x: u8) -> u8 { return x + 1; }
always { speed := speed; }
model Inner {
    var t: u8 := 0;
    start S;
}
start Idle {
    enter { speed := 0; }
    always { speed := bump(speed); }
    invariant Sane = speed < 50;
    ref Done: Fast;
}
state Done;
"#;

    /// Символ верхнего уровня по имени.
    fn top<'a>(syms: &'a [DocumentSymbol], name: &str) -> &'a DocumentSymbol {
        syms.iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("нет символа '{name}'; есть: {:?}", names(syms)))
    }

    fn names(syms: &[DocumentSymbol]) -> Vec<&str> {
        syms.iter().map(|s| s.name.as_str()).collect()
    }

    fn children(sym: &DocumentSymbol) -> &[DocumentSymbol] {
        sym.children.as_deref().unwrap_or(&[])
    }

    /// Текст, покрытый диапазоном (в координатах строк/столбцов).
    fn slice(source: &str, range: Range) -> String {
        let line_of = |p: Position| {
            source
                .lines()
                .nth(p.line as usize)
                .unwrap_or_else(|| panic!("нет строки {}", p.line))
        };
        if range.start.line == range.end.line {
            let line: Vec<char> = line_of(range.start).chars().collect();
            return line[range.start.character as usize..range.end.character as usize]
                .iter()
                .collect();
        }
        // Многострочный диапазон: для проверок ниже достаточно первой строки.
        let line: Vec<char> = line_of(range.start).chars().collect();
        line[range.start.character as usize..].iter().collect()
    }

    fn contains(outer: Range, inner: Range) -> bool {
        (outer.start.line, outer.start.character) <= (inner.start.line, inner.start.character)
            && (inner.end.line, inner.end.character) <= (outer.end.line, outer.end.character)
    }

    // ── Состав ───────────────────────────────────────────────────────────────

    /// Каждое именованное объявление верхнего уровня становится символом —
    /// и получает **свой** вид.
    ///
    /// Вид (`SymbolKind`) — не косметика: по нему редактор рисует значок и
    /// группирует панель. Перепутанные виды дают «работающий» список, в котором
    /// порт неотличим от переменной.
    #[test]
    fn every_top_level_declaration_becomes_a_symbol() {
        let syms = document_symbols(SRC);
        let expected: &[(&str, SymbolKind)] = &[
            ("Speed", SymbolKind::TYPE_PARAMETER),
            ("Mode", SymbolKind::ENUM),
            ("Point", SymbolKind::STRUCT),
            ("LIMIT", SymbolKind::CONSTANT),
            ("flag", SymbolKind::PROPERTY),
            ("speed", SymbolKind::VARIABLE),
            ("Fast", SymbolKind::CONSTANT),
            ("Safe", SymbolKind::CONSTANT),
            ("bump", SymbolKind::FUNCTION),
            ("always", SymbolKind::EVENT),
            ("Inner", SymbolKind::MODULE),
            ("Idle", SymbolKind::CLASS),
            ("Done", SymbolKind::CLASS),
        ];
        for (name, kind) in expected {
            let sym = top(&syms, name);
            assert_eq!(
                sym.kind, *kind,
                "символ '{name}': вид определяет значок и группировку в панели \
                 структуры — перепутанный вид даёт «работающий» бесполезный список"
            );
        }
        assert_eq!(
            syms.len(),
            expected.len(),
            "состав символов верхнего уровня изменился: {:?}",
            names(&syms)
        );
    }

    /// `import` символом **не** становится — и это замысел.
    ///
    /// Он не объявляет имени в этом файле: показать его в панели структуры
    /// значило бы предложить переход к объявлению, которого здесь нет.
    #[test]
    fn import_is_not_a_symbol() {
        let syms = document_symbols(SRC);
        assert!(
            !names(&syms).contains(&"Thermostat"),
            "импортированное имя не объявлено в этом файле и символом быть не \
             должно: {:?}",
            names(&syms)
        );
    }

    // ── Вложенность ──────────────────────────────────────────────────────────

    /// Под-модель отдаёт свои элементы **детьми**, а не в общий плоский список.
    #[test]
    fn nested_model_owns_its_elements() {
        let syms = document_symbols(SRC);
        let inner = top(&syms, "Inner");
        assert_eq!(
            names(children(inner)),
            vec!["t", "S"],
            "элементы под-модели обязаны быть её детьми"
        );
        assert!(
            !names(&syms).contains(&"t"),
            "элемент под-модели не должен дублироваться на верхнем уровне: {:?}",
            names(&syms)
        );
    }

    /// Состояние отдаёт детьми свои именованные блоки.
    #[test]
    fn state_owns_its_named_blocks() {
        let syms = document_symbols(SRC);
        let idle = top(&syms, "Idle");
        assert_eq!(
            names(children(idle)),
            vec!["enter", "always"],
            "именованные блоки состояния обязаны быть его детьми"
        );
        for block in children(idle) {
            assert_eq!(block.kind, SymbolKind::EVENT, "блок — событие");
        }
    }

    /// Перечисление отдаёт детьми свои варианты, структура — свои поля.
    #[test]
    fn enum_and_struct_own_their_members() {
        let syms = document_symbols(SRC);
        assert_eq!(names(children(top(&syms, "Mode"))), vec!["Idle", "Run"]);
        assert_eq!(names(children(top(&syms, "Point"))), vec!["x", "y"]);
        assert!(
            children(top(&syms, "Mode"))
                .iter()
                .all(|v| v.kind == SymbolKind::ENUM_MEMBER)
        );
        assert!(
            children(top(&syms, "Point"))
                .iter()
                .all(|f| f.kind == SymbolKind::FIELD)
        );
    }

    /// Состояние `Idle` и вариант `Mode::Idle` — **разные** символы.
    ///
    /// Одноимённость законна (разные пространства), и плоский поиск по имени
    /// склеил бы их. Проверяется, что вариант лежит внутри перечисления, а
    /// состояние — на верхнем уровне.
    #[test]
    fn same_name_in_different_scopes_stays_separate() {
        let syms = document_symbols(SRC);
        assert_eq!(top(&syms, "Idle").kind, SymbolKind::CLASS);
        let variant = children(top(&syms, "Mode"))
            .iter()
            .find(|v| v.name == "Idle")
            .expect("вариант Mode::Idle");
        assert_eq!(variant.kind, SymbolKind::ENUM_MEMBER);
        assert_ne!(
            top(&syms, "Idle").range,
            variant.range,
            "одноимённые символы из разных областей обязаны различаться диапазоном"
        );
    }

    // ── Диапазоны ────────────────────────────────────────────────────────────

    /// `selection_range` покрывает **имя**, `range` — всё объявление и
    /// содержит `selection_range`.
    ///
    /// По `selection_range` редактор переходит из панели структуры: сдвиг
    /// уводит курсор не туда, и ни один тест на состав этого не заметит.
    #[test]
    fn selection_range_covers_the_name() {
        let syms = document_symbols(SRC);
        for name in [
            "Speed", "Mode", "Point", "LIMIT", "flag", "speed", "Fast", "Safe", "bump", "Inner",
            "Idle", "Done",
        ] {
            let sym = top(&syms, name);
            assert_eq!(
                slice(SRC, sym.selection_range),
                name,
                "'{name}': selection_range обязан покрывать имя — по нему \
                 редактор переходит из панели структуры"
            );
            assert!(
                contains(sym.range, sym.selection_range),
                "'{name}': range обязан содержать selection_range"
            );
        }
    }

    /// Тот же контракт — у детей.
    #[test]
    fn selection_range_covers_the_name_for_children() {
        let syms = document_symbols(SRC);
        for (parent, child) in [
            ("Point", "x"),
            ("Point", "y"),
            ("Inner", "t"),
            ("Inner", "S"),
        ] {
            let sym = children(top(&syms, parent))
                .iter()
                .find(|c| c.name == child)
                .unwrap_or_else(|| panic!("нет '{parent}::{child}'"));
            assert_eq!(
                slice(SRC, sym.selection_range),
                child,
                "'{parent}::{child}': selection_range обязан покрывать имя"
            );
            assert!(contains(sym.range, sym.selection_range));
        }
    }

    /// `range` объявления начинается с его ключевого слова.
    ///
    /// Пиннинг границы: `range` — всё объявление, а не только имя. Иначе
    /// сворачивание блока в редакторе схлопнуло бы не то.
    #[test]
    fn range_starts_at_the_declaration_keyword() {
        let syms = document_symbols(SRC);
        for (name, head) in [
            ("Speed", "type"),
            ("Mode", "enum"),
            ("Point", "struct"),
            ("LIMIT", "const"),
            ("flag", "out"),
            ("speed", "var"),
            ("Fast", "cond"),
            ("Safe", "invariant"),
            ("bump", "fn"),
            ("Inner", "model"),
            ("Idle", "start"),
            ("Done", "state"),
        ] {
            let text = slice(SRC, top(&syms, name).range);
            assert!(
                text.starts_with(head),
                "'{name}': range обязан начинаться с '{head}', а начинается с {text:?}"
            );
        }
    }

    // ── Границы и отказы ─────────────────────────────────────────────────────

    /// **Контрпример:** неразбираемый файл даёт пустой список, а не панику.
    ///
    /// Сервер языка получает файл в каждом промежуточном состоянии набора —
    /// то есть чаще неразбираемый, чем разбираемый. Паника здесь роняет сервер
    /// у пользователя под руками.
    #[test]
    fn broken_source_yields_empty_list_not_panic() {
        for src in [
            "",
            "model {{{",
            "struct { x: u8 }",
            "model M { start",
            "enum { A, B }",
            "фыва",
        ] {
            let syms = document_symbols(src);
            assert!(
                syms.is_empty(),
                "неразбираемый вход {src:?} обязан давать пустой список: {:?}",
                names(&syms)
            );
        }
    }

    /// Пустая, но валидная модель даёт пустой список без ошибок.
    #[test]
    fn empty_model_yields_no_symbols() {
        assert!(document_symbols("").is_empty());
    }

    /// ⚠️ **Известный пробел, зафиксированный сознательно.**
    ///
    /// Инвариант **модели** (`Safe`) символом становится, а инвариант
    /// **состояния** (`Sane`) — нет: дети состояния собираются только из
    /// именованных блоков. Тест пришпиливает текущее поведение, чтобы его
    /// изменение было **осознанным**, а не побочным. Разбор — находка в
    /// `FEATURES.md`.
    #[test]
    fn state_level_invariant_is_not_a_symbol_yet() {
        let syms = document_symbols(SRC);
        let idle = top(&syms, "Idle");
        assert!(
            !names(children(idle)).contains(&"Sane"),
            "поведение изменилось: инвариант состояния стал символом. Это может \
             быть улучшением — но тогда снимите этот тест осознанно и обновите \
             находку в FEATURES.md.\nдети Idle: {:?}",
            names(children(idle))
        );
        assert_eq!(
            top(&syms, "Safe").kind,
            SymbolKind::CONSTANT,
            "инвариант МОДЕЛИ символом остаётся — асимметрия и есть пробел"
        );
    }

    /// Возможность объявлена клиенту — иначе редактор её не вызовет.
    #[test]
    fn capability_is_advertised() {
        let caps = takt_lang::lsp::server_capabilities();
        assert!(
            caps.document_symbol_provider.is_some(),
            "без объявления в ServerCapabilities реализация мертва: редактор \
             просто не пришлёт запрос"
        );
    }
}
