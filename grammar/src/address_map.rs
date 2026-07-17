//! Внешняя карта адресов портов — отдельный `.ld`-подобный формат (фича 0020).
//!
//! Карта позволяет держать аппаратные адреса портов отдельно от модели и
//! переопределять их под платформу/ревизию (оверлей). Формат намеренно
//! **не** является фрагментом `.lam` — это простой построчный мини-язык в духе
//! линкер-скрипта:
//!
//! ```text
//! // адреса платформы STM32
//! BTN = 0x00200000;
//! LED = 0x00200004:3;   // адрес:бит
//! ```
//!
//! Грамматика (EBNF):
//!
//! ```ebnf
//! AddressMapFile = { Entry } ;
//! Entry          = Name, "=", Address, ";" ;
//! Name           = (letter | "_"), { letter | digit | "_" } ;
//! Address        = HexOrDec, [ ":", digit, { digit } ] ;
//! ```
//!
//! Комментарии — `//` до конца строки. Приоритет источников (внешняя карта
//! главнее inline/`address` в модели) и предупреждения об оверлее реализует
//! [`address_map_overlay_warnings`]; понижение адреса в целевой код — задача
//! 0020-05.

use crate::diagnostics::{Diagnostic, Location};
use crate::semantic::{ExpressionNode, ModelNode, VariableNode};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Одна запись внешней карты адресов: `NAME = 0xADDR[:bit];`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressMapEntry {
    /// Имя порта, которому назначается адрес.
    pub name: String,
    /// Числовой адрес.
    pub addr: i64,
    /// Битовая позиция (`0xADDR:bit`), если задана.
    pub bit: Option<i64>,
    /// Позиция записи в файле карты.
    pub loc: Location,
}

/// Сканер символов с байтовыми позициями (для `Location`).
struct Scanner {
    chars: Vec<(usize, char)>,
    /// Индекс текущего символа в [`chars`](Scanner::chars).
    pos: usize,
    /// Длина исходного текста в байтах (позиция конца).
    len: usize,
    file_no: u64,
}

impl Scanner {
    fn new(src: &str, file_no: u64) -> Self {
        Scanner {
            chars: src.char_indices().collect(),
            pos: 0,
            len: src.len(),
            file_no,
        }
    }

    /// Байтовое смещение текущего символа (или конец текста).
    fn offset(&self) -> usize {
        self.chars
            .get(self.pos)
            .map(|(o, _)| *o)
            .unwrap_or(self.len)
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).map(|(_, c)| *c)
    }

    fn peek2(&self) -> Option<char> {
        self.chars.get(self.pos + 1).map(|(_, c)| *c)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    /// Пропускает пробелы и строчные комментарии `//`.
    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                Some('/') if self.peek2() == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                _ => break,
            }
        }
    }

    fn loc(&self, start: usize, end: usize) -> Location {
        Location::Source(self.file_no, start, end)
    }
}

fn is_name_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_name_cont(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Разбирает содержимое файла внешней карты адресов.
///
/// Возвращает список записей либо непустой список диагностик. Парсер устойчив к
/// ошибкам: при синтаксической ошибке в записи она пропускается до ближайшего
/// `;`, разбор продолжается — чтобы сообщить обо всех проблемах разом.
///
/// # Коды диагностик
///
/// - `AM-001` — ожидалось имя порта;
/// - `AM-002` — ожидался `=`;
/// - `AM-003` — ожидался адрес;
/// - `AM-004` — ожидался `;`;
/// - `AM-005` — некорректный литерал адреса;
/// - `AM-006` — повторная запись для одного порта.
pub fn parse_address_map(src: &str, file_no: u64) -> Result<Vec<AddressMapEntry>, Vec<Diagnostic>> {
    let mut sc = Scanner::new(src, file_no);
    let mut entries: Vec<AddressMapEntry> = Vec::new();
    let mut diags: Vec<Diagnostic> = Vec::new();
    let mut seen: HashMap<String, Location> = HashMap::new();

    loop {
        sc.skip_trivia();
        if sc.peek().is_none() {
            break;
        }
        match parse_entry(&mut sc) {
            Ok(entry) => {
                if let Some(prev) = seen.get(&entry.name) {
                    diags.push(
                        Diagnostic::error(
                            entry.loc,
                            format!(
                                "повторная запись адреса для порта '{}' во внешней карте",
                                entry.name
                            ),
                        )
                        .with_code("AM-006"),
                    );
                    let _ = prev;
                } else {
                    seen.insert(entry.name.clone(), entry.loc);
                    entries.push(entry);
                }
            }
            Err(diag) => {
                diags.push(diag);
                // Восстановление: пропускаем до ближайшего `;` включительно.
                while let Some(c) = sc.peek() {
                    sc.bump();
                    if c == ';' {
                        break;
                    }
                }
            }
        }
    }

    if diags.is_empty() {
        Ok(entries)
    } else {
        Err(diags)
    }
}

/// Разбирает одну запись `NAME = ADDRESS ;`.
fn parse_entry(sc: &mut Scanner) -> Result<AddressMapEntry, Diagnostic> {
    // Имя.
    let name_start = sc.offset();
    let Some(c0) = sc.peek() else {
        return Err(Diagnostic::error(
            sc.loc(name_start, name_start),
            "ожидалось имя порта".to_string(),
        )
        .with_code("AM-001"));
    };
    if !is_name_start(c0) {
        let end = sc.offset() + c0.len_utf8();
        return Err(
            Diagnostic::error(sc.loc(name_start, end), "ожидалось имя порта".to_string())
                .with_code("AM-001"),
        );
    }
    let mut name = String::new();
    while let Some(c) = sc.peek() {
        if is_name_cont(c) {
            name.push(c);
            sc.bump();
        } else {
            break;
        }
    }
    let name_end = sc.offset();

    // `=`.
    sc.skip_trivia();
    if sc.peek() != Some('=') {
        let at = sc.offset();
        return Err(Diagnostic::error(
            sc.loc(at, at),
            format!("ожидался '=' после имени порта '{}'", name),
        )
        .with_code("AM-002"));
    }
    sc.bump();

    // Адрес.
    sc.skip_trivia();
    let addr_start = sc.offset();
    let mut tok = String::new();
    while let Some(c) = sc.peek() {
        if c.is_alphanumeric() || c == ':' {
            tok.push(c);
            sc.bump();
        } else {
            break;
        }
    }
    let addr_end = sc.offset();
    if tok.is_empty() {
        return Err(Diagnostic::error(
            sc.loc(addr_start, addr_start),
            format!("ожидался адрес для порта '{}'", name),
        )
        .with_code("AM-003"));
    }
    let (addr, bit) = parse_address_token(&tok)
        .map_err(|msg| Diagnostic::error(sc.loc(addr_start, addr_end), msg).with_code("AM-005"))?;

    // `;`.
    sc.skip_trivia();
    if sc.peek() != Some(';') {
        let at = sc.offset();
        return Err(Diagnostic::error(
            sc.loc(at, at),
            format!("ожидался ';' после адреса порта '{}'", name),
        )
        .with_code("AM-004"));
    }
    sc.bump();

    Ok(AddressMapEntry {
        name,
        addr,
        bit,
        loc: sc.loc(name_start, name_end),
    })
}

/// Разбирает литерал адреса `0xADDR`, `123` или `0xADDR:bit`.
fn parse_address_token(tok: &str) -> Result<(i64, Option<i64>), String> {
    let (addr_part, bit_part) = match tok.split_once(':') {
        Some((a, b)) => (a, Some(b)),
        None => (tok, None),
    };
    let addr = parse_int(addr_part).ok_or_else(|| format!("некорректный адрес '{}'", addr_part))?;
    let bit = match bit_part {
        Some(b) => Some(parse_int(b).ok_or_else(|| format!("некорректный бит '{}'", b))?),
        None => None,
    };
    Ok((addr, bit))
}

/// Разбирает целое: hex (`0x…`/`0X…`) или десятичное.
fn parse_int(s: &str) -> Option<i64> {
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        i64::from_str_radix(hex, 16).ok()
    } else {
        s.parse::<i64>().ok()
    }
}

/// Фича 0020-03: предупреждения при наложении внешней карты на модель.
///
/// Внешняя карта — самый приоритетный источник адреса (оверлей). Функция
/// сверяет записи карты с портами модели и возвращает предупреждения:
///
/// - **SE-050** — запись карты переопределяет адрес порта, уже заданный в модели
///   (inline `:=` или оператором `address`); ожидаемое поведение оверлея, но
///   заметное.
/// - **SE-051** — запись карты для имени, которого нет среди портов модели.
///
/// Проверяются порты **переданной** модели (её `variables`); понижение адреса и
/// построение итогового `AddressMap` для генерации — задача 0020-05.
pub fn address_map_overlay_warnings(
    model: Rc<RefCell<ModelNode>>,
    entries: &[AddressMapEntry],
) -> Vec<Diagnostic> {
    let borrowed = model.borrow();
    let mut out = Vec::new();
    for e in entries {
        match borrowed.variables.get(&e.name) {
            Some(VariableNode::Port { expr, .. }) => {
                let has_inline = !matches!(expr, ExpressionNode::None);
                let has_operator = borrowed.address_defs.iter().any(|d| d.port == e.name);
                if has_inline || has_operator {
                    out.push(
                        Diagnostic::warning(
                            e.loc,
                            format!(
                                "внешняя карта переопределяет адрес порта '{}', заданный в модели",
                                e.name
                            ),
                        )
                        .with_code("SE-050"),
                    );
                }
            }
            _ => {
                out.push(
                    Diagnostic::warning(
                        e.loc,
                        format!(
                            "внешняя карта задаёт адрес для несуществующего порта '{}'",
                            e.name
                        ),
                    )
                    .with_code("SE-051"),
                );
            }
        }
    }
    out
}

/// Источник, из которого получен адрес порта (по возрастанию приоритета).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressSource {
    /// Inline-инициализатор объявления (`in P: T := <addr>;`).
    Inline,
    /// Оператор `address P = <addr>;`.
    Operator,
    /// Внешняя карта адресов (`--address-map`).
    External,
}

/// Разрешённый адрес порта: числовое значение, бит и источник-победитель.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAddress {
    /// Числовой адрес.
    pub addr: i64,
    /// Битовая позиция (`0xADDR:bit`), если задана.
    pub bit: Option<i64>,
    /// Источник, из которого взят адрес (после разрешения приоритета).
    pub source: AddressSource,
}

/// Результат разрешения адресов модели (фича 0020-05).
#[derive(Debug, Default)]
pub struct AddressResolution {
    /// Итоговая карта: имя порта → разрешённый адрес.
    pub map: HashMap<String, ResolvedAddress>,
    /// Диагностики: ошибки полноты (SE-052) и предупреждения оверлея
    /// (SE-050) / висячих записей карты (SE-051).
    pub diagnostics: Vec<Diagnostic>,
}

/// Среда символов адреса (фича 0042): имя → значение.
///
/// # Почему отдельная среда, а не общие константы языка
///
/// Символы приходят из `--define` — то есть **из командной строки**. Будь они
/// видны любому выражению, флаг сборки мог бы молча изменить логику автомата
/// (`-D LIMIT=5` переписал бы `ref Stop: cnt = LIMIT;`), и верифицированная
/// модель перестала бы быть самодостаточной. Поэтому среда видна **только**
/// вычислителю адреса ([`eval_addr_expr`]) — ADR 0042, решение B2.
///
/// Define **не является источником адреса** и в приоритет `inline < address <
/// внешняя карта` не входит (решение A2): он лишь снабжает значением выражение
/// того слоя, где записан.
#[derive(Debug, Default, Clone)]
pub struct AddressEnv {
    /// Символ → `(адрес, бит)`; значение разбирает [`parse_address_token`] —
    /// та же грамматика, что у записи внешней карты (решение C1).
    symbols: HashMap<String, (i64, Option<i64>)>,
    /// Имена, к которым обратилось хотя бы одно выражение адреса.
    ///
    /// Нужны для `DF-004`: define, которого никто не спросил, — почти всегда
    /// опечатка (симметрия с `SE-051` — висячей записью карты).
    used: RefCell<HashSet<String>>,
    /// Имена, где define перекрыл одноимённую `const` модели, и позиция `const`.
    ///
    /// Копится по ходу вычисления, а печатается вызывающим: `SE-053` обязана
    /// указывать на **объявление `const`** — то, что перекрыто.
    overridden: RefCell<Vec<(String, Location)>>,
}

impl AddressEnv {
    /// Собирает среду из разобранных пар `(имя, значение)`.
    pub fn new(symbols: HashMap<String, (i64, Option<i64>)>) -> Self {
        AddressEnv {
            symbols,
            used: RefCell::new(HashSet::new()),
            overridden: RefCell::new(Vec::new()),
        }
    }

    /// Значение символа; обращение запоминается (для `DF-004`).
    fn lookup(&self, name: &str) -> Option<(i64, Option<i64>)> {
        let found = self.symbols.get(name).copied();
        if found.is_some() {
            self.used.borrow_mut().insert(name.to_string());
        }
        found
    }

    /// Имена define'ов, к которым не обратилось ни одно выражение адреса.
    ///
    /// Порядок — алфавитный: диагностики обязаны быть детерминированными
    /// (фича 0048).
    pub fn unused(&self) -> Vec<String> {
        let used = self.used.borrow();
        let mut out: Vec<String> = self
            .symbols
            .keys()
            .filter(|n| !used.contains(*n))
            .cloned()
            .collect();
        out.sort();
        out
    }

    /// Пуста ли среда (define'ов не передавали).
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Запоминает, что define перекрыл `const` с этим именем.
    fn note_override(&self, name: &str, const_loc: Location) {
        let mut o = self.overridden.borrow_mut();
        if !o.iter().any(|(n, _)| n == name) {
            o.push((name.to_string(), const_loc));
        }
    }

    /// Перекрытия `const` define'ами: `(имя, позиция const)`, по алфавиту.
    fn overrides(&self) -> Vec<(String, Location)> {
        let mut out = self.overridden.borrow().clone();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }
}

/// Разбирает аргументы `--define NAME=VALUE` в среду символов (фича 0042).
///
/// Грамматика значения — **та же**, что у записи внешней карты
/// ([`parse_address_token`]): `0x…` / десятичное / необязательный `:bit`. Это
/// решение C1 ADR 0042: третий мини-язык не заводится, а арифметика остаётся
/// **в модели** (`address BTN = BASE + 4;`), где её видит ревьюер и форматтер,
/// а не в командной строке.
///
/// # Коды диагностик
///
/// - `DF-001` — нет `=`, пустое или некорректное имя (симметрия с `AM-001/002`);
/// - `DF-002` — некорректный литерал значения (симметрия с `AM-005`);
/// - `DF-003` — повторный `--define` для одного имени (симметрия с `AM-006`).
///
/// У аргумента командной строки нет позиции в файле, поэтому диагностики несут
/// [`Location::CommandLine`].
pub fn parse_defines(args: &[String]) -> Result<AddressEnv, Vec<Diagnostic>> {
    let mut symbols: HashMap<String, (i64, Option<i64>)> = HashMap::new();
    let mut diags: Vec<Diagnostic> = Vec::new();

    for arg in args {
        let Some((name, value)) = arg.split_once('=') else {
            diags.push(
                Diagnostic::error(
                    Location::CommandLine,
                    format!("--define '{}': ожидалось NAME=VALUE (нет '=')", arg),
                )
                .with_code("DF-001"),
            );
            continue;
        };
        if !is_valid_symbol(name) {
            diags.push(
                Diagnostic::error(
                    Location::CommandLine,
                    format!(
                        "--define '{}': некорректное имя символа '{}' \
                         (ожидается [A-Za-z_][A-Za-z0-9_]*)",
                        arg, name
                    ),
                )
                .with_code("DF-001"),
            );
            continue;
        }
        let parsed = match parse_address_token(value) {
            Ok(v) => v,
            Err(e) => {
                diags.push(
                    Diagnostic::error(Location::CommandLine, format!("--define '{}': {}", arg, e))
                        .with_code("DF-002"),
                );
                continue;
            }
        };
        // Повтор — ошибка, а не «побеждает последний»: молчаливое затирание
        // сделало бы адрес зависящим от порядка флагов (симметрия с `AM-006`).
        if symbols.contains_key(name) {
            diags.push(
                Diagnostic::error(
                    Location::CommandLine,
                    format!("--define: символ '{}' задан дважды", name),
                )
                .with_code("DF-003"),
            );
            continue;
        }
        symbols.insert(name.to_string(), parsed);
    }

    if diags.is_empty() {
        Ok(AddressEnv::new(symbols))
    } else {
        Err(diags)
    }
}

/// Имя символа: `[A-Za-z_][A-Za-z0-9_]*` — как имя порта в карте.
fn is_valid_symbol(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if is_name_start(c) => {}
        _ => return false,
    }
    chars.all(is_name_cont)
}

/// Диагностики выражений адреса для целей, которые адрес **не потребляют**.
///
/// # Зачем это нужно
///
/// `resolve_addresses` зовут только адрес-потребляющие цели (`c-hal`, `st-at`),
/// и для них `SE-054`/`SE-055` — ошибки. Но `address BTN = NOWHERE;` — опечатка
/// и при сборке целью `c`: адрес она не эмитит, однако молчать о заведомо
/// сломанной привязке — тот самый тихий пропуск, который фича и закрывает.
///
/// Поэтому здесь те же диагностики **понижены до предупреждений**: цель `c`
/// сегодня такой файл собирает успешно (rc=0), и делать это ошибкой значило бы
/// сломать существующие сборки (прецедент — `SE-052`, ADR 0042).
pub fn address_expr_warnings(model: Rc<RefCell<ModelNode>>, env: &AddressEnv) -> Vec<Diagnostic> {
    let resolution = resolve_addresses(model, &[], env);
    let mut out: Vec<Diagnostic> = resolution
        .diagnostics
        .into_iter()
        .filter(|d| matches!(d.code.as_deref(), Some("SE-054") | Some("SE-055")))
        .map(|d| Diagnostic {
            level: crate::diagnostics::Level::Warning,
            ..d
        })
        .collect();
    out.sort_by_key(|d| d.loc.start());
    out
}

/// Значение выражения адреса: число и необязательный бит (`0xADDR:bit`).
type AddrValue = (i64, Option<i64>);

/// `SE-054` — имя в выражении адреса не разрешается ни define'ом, ни `const`.
fn undefined_symbol(loc: Location, name: &str) -> Diagnostic {
    Diagnostic::error(
        loc,
        format!(
            "выражение адреса ссылается на неопределённый символ '{}' \
             (нет ни `--define {}=…`, ни `const {}` в модели)",
            name, name, name
        ),
    )
    .with_code("SE-054")
}

/// `SE-055` — выражение адреса не сворачивается в константу.
fn not_constant(loc: Location, reason: &str) -> Diagnostic {
    Diagnostic::error(
        loc,
        format!("выражение адреса не сворачивается в константу: {}", reason),
    )
    .with_code("SE-055")
}

/// Применяет бинарную операцию к вычисленным операндам.
///
/// Единственное место арифметики адреса — им пользуются **оба** матчера (по
/// `ExpressionNode` и по сырому АСД). Разъехавшись, они дали бы разный адрес для
/// одного и того же текста в зависимости от того, каким путём выражение попало в
/// разрешение (inline против оператора `address`).
fn apply_binary(
    op: &str,
    left: AddrValue,
    right: AddrValue,
    loc: Location,
) -> Result<AddrValue, Diagnostic> {
    // Бит — свойство записи адреса (`0x1000:3`), а не число: арифметика над ним
    // бессмысленна, и молча его терять нельзя.
    if left.1.is_some() || right.1.is_some() {
        return Err(not_constant(
            loc,
            "операнд формы `адрес:бит` не участвует в арифметике",
        ));
    }
    let (a, b) = (left.0, right.0);
    let value = match op {
        "+" => a.wrapping_add(b),
        "-" => a.wrapping_sub(b),
        "*" => a.wrapping_mul(b),
        "/" => {
            if b == 0 {
                return Err(not_constant(loc, "деление на ноль"));
            }
            a.wrapping_div(b)
        }
        "%" => {
            if b == 0 {
                return Err(not_constant(loc, "остаток от деления на ноль"));
            }
            a.wrapping_rem(b)
        }
        "<<" | ">>" => {
            // Сдвиг на отрицательное или ≥ 64 в Rust — паника; у адреса это
            // всегда опечатка, поэтому диагностика, а не молчание.
            if !(0..64).contains(&b) {
                return Err(not_constant(loc, "сдвиг допустим только на 0..63 бит"));
            }
            if op == "<<" { a << b } else { a >> b }
        }
        "&" => a & b,
        "|" => a | b,
        "^" => a ^ b,
        _ => {
            return Err(not_constant(
                loc,
                "операция не поддержана в выражении адреса",
            ));
        }
    };
    Ok((value, None))
}

/// Вычисляет **сырое АСД**-выражение адреса — путь оператора `address`.
///
/// Значение оператора хранится как `ExpressionNode::Unresolved(ast::Expression)`
/// и семантикой не понижается (`tree.rs`): ни один из 7 проходов к
/// `address_defs` не обращается. Поэтому здесь разбирается АСД напрямую.
fn eval_ast_addr(
    expr: &crate::parser::ast::Expression,
    scope: &Rc<RefCell<ModelNode>>,
    env: &AddressEnv,
    seen: &mut Vec<String>,
) -> Result<AddrValue, Diagnostic> {
    use crate::parser::ast::Expression as E;
    // Бинарная операция разворачивается явно: замыкание заимствовало бы `seen`
    // изменяемо на всё тело match.
    macro_rules! bin {
        ($op:literal, $l:expr, $r:expr, $loc:expr) => {{
            let left = eval_ast_addr($l, scope, env, seen)?;
            let right = eval_ast_addr($r, scope, env, seen)?;
            apply_binary($op, left, right, $loc)
        }};
    }
    match expr {
        E::Number(_, n) => Ok((*n, None)),
        E::Address(_, a, b) => Ok((*a, Some(*b))),
        E::Variable(id) => resolve_symbol(&id.name, id.loc, scope, env, seen),
        E::Parenthesis(_, inner) => eval_ast_addr(inner, scope, env, seen),
        E::UnaryPlus(_, inner) => eval_ast_addr(inner, scope, env, seen),
        E::Negate(loc, inner) => {
            let (v, bit) = eval_ast_addr(inner, scope, env, seen)?;
            unary_bitless(v.wrapping_neg(), bit, *loc)
        }
        E::BitwiseNot(loc, inner) => {
            let (v, bit) = eval_ast_addr(inner, scope, env, seen)?;
            unary_bitless(!v, bit, *loc)
        }
        E::Add(loc, l, r) => bin!("+", l, r, *loc),
        E::Subtract(loc, l, r) => bin!("-", l, r, *loc),
        E::Multiply(loc, l, r) => bin!("*", l, r, *loc),
        E::Divide(loc, l, r) => bin!("/", l, r, *loc),
        E::Modulo(loc, l, r) => bin!("%", l, r, *loc),
        E::ShiftLeft(loc, l, r) => bin!("<<", l, r, *loc),
        E::ShiftRight(loc, l, r) => bin!(">>", l, r, *loc),
        E::BitwiseAnd(loc, l, r) => bin!("&", l, r, *loc),
        E::BitwiseOr(loc, l, r) => bin!("|", l, r, *loc),
        E::BitwiseXor(loc, l, r) => bin!("^", l, r, *loc),
        other => Err(not_constant(
            other.loc(),
            "поддержаны только целочисленные литералы, символы и операции \
             `+ - * / % << >> & | ^ ~`",
        )),
    }
}

/// Унарная операция: бит в ней бессмыслен — см. [`apply_binary`].
fn unary_bitless(value: i64, bit: Option<i64>, loc: Location) -> Result<AddrValue, Diagnostic> {
    if bit.is_some() {
        return Err(not_constant(
            loc,
            "операнд формы `адрес:бит` не участвует в арифметике",
        ));
    }
    Ok((value, None))
}

/// Разрешает имя в выражении адреса: **сначала** define, **затем** `const` модели.
///
/// Порядок — решение D2 ADR 0042: платформенный слой главнее модели (прямая
/// симметрия с оверлеем внешней карты `SE-050`). Предупреждение о перекрытии
/// (`SE-053`) выдаёт вызывающий: здесь нет позиции объявления `const`.
fn resolve_symbol(
    name: &str,
    loc: Location,
    scope: &Rc<RefCell<ModelNode>>,
    env: &AddressEnv,
    seen: &mut Vec<String>,
) -> Result<AddrValue, Diagnostic> {
    if let Some(v) = env.lookup(name) {
        // Define побеждает `const` (решение D2), но перекрытие обязано быть
        // ЗАМЕТНЫМ — прямая симметрия с оверлеем внешней карты (`SE-050`):
        // платформенный слой главнее модели, однако молча её не подменяет.
        if let Some(VariableNode::Const { loc: const_loc, .. }) = scope.borrow().search_var(name) {
            env.note_override(name, const_loc);
        }
        return Ok(v);
    }
    // Цикл `const A := B; const B := A;` — иначе вычислитель зациклится.
    if seen.iter().any(|s| s == name) {
        return Err(not_constant(
            loc,
            &format!("циклическая ссылка через '{}'", name),
        ));
    }
    let Some(var) = scope.borrow().search_var(name) else {
        return Err(undefined_symbol(loc, name));
    };
    let VariableNode::Const { expr, .. } = &var else {
        // Переменная или порт: их значение известно только в рантайме.
        return Err(not_constant(
            loc,
            &format!(
                "'{}' — не константа (адрес обязан быть известен при сборке)",
                name
            ),
        ));
    };
    seen.push(name.to_string());
    let value = eval_addr_value(expr, scope, env, seen);
    seen.pop();
    value
}

/// Вычисляет выражение адреса, уже понижённое семантикой, — путь inline.
fn eval_addr_value(
    expr: &ExpressionNode,
    scope: &Rc<RefCell<ModelNode>>,
    env: &AddressEnv,
    seen: &mut Vec<String>,
) -> Result<AddrValue, Diagnostic> {
    macro_rules! bin {
        ($op:literal, $l:expr, $r:expr) => {{
            let left = eval_addr_value($l, scope, env, seen)?;
            let right = eval_addr_value($r, scope, env, seen)?;
            apply_binary($op, left, right, Location::Implicit)
        }};
    }
    match expr {
        ExpressionNode::Number(n) => Ok((*n, None)),
        ExpressionNode::Address(a, b) => Ok((*a, Some(*b))),
        // Сырой АСД: значение оператора `address` семантикой не понижается.
        ExpressionNode::Unresolved(ast_expr) => eval_ast_addr(ast_expr, scope, env, seen),
        // Имя, разрешённое семантикой в объявление (путь inline).
        ExpressionNode::Variable(var_rc) => {
            let borrowed = var_rc.borrow();
            let name = borrowed.name().to_string();
            let loc = borrowed.loc();
            drop(borrowed);
            resolve_symbol(&name, loc, scope, env, seen)
        }
        ExpressionNode::Parenthesis(inner) => eval_addr_value(inner, scope, env, seen),
        ExpressionNode::UnaryPlus(inner) => eval_addr_value(inner, scope, env, seen),
        ExpressionNode::Negate(inner) => {
            let (v, bit) = eval_addr_value(inner, scope, env, seen)?;
            unary_bitless(v.wrapping_neg(), bit, Location::Implicit)
        }
        ExpressionNode::BitwiseNot(inner) => {
            let (v, bit) = eval_addr_value(inner, scope, env, seen)?;
            unary_bitless(!v, bit, Location::Implicit)
        }
        ExpressionNode::Add(l, r) => bin!("+", l, r),
        ExpressionNode::Subtract(l, r) => bin!("-", l, r),
        ExpressionNode::Multiply(l, r) => bin!("*", l, r),
        ExpressionNode::Divide(l, r) => bin!("/", l, r),
        ExpressionNode::Modulo(l, r) => bin!("%", l, r),
        ExpressionNode::ShiftLeft(l, r) => bin!("<<", l, r),
        ExpressionNode::ShiftRight(l, r) => bin!(">>", l, r),
        ExpressionNode::BitwiseAnd(l, r) => bin!("&", l, r),
        ExpressionNode::BitwiseOr(l, r) => bin!("|", l, r),
        ExpressionNode::BitwiseXor(l, r) => bin!("^", l, r),
        _ => Err(not_constant(
            Location::Implicit,
            "поддержаны только целочисленные литералы, символы и операции \
             `+ - * / % << >> & | ^ ~`",
        )),
    }
}

/// Вычисляет выражение адреса в константу `(addr, bit)`.
///
/// # Чем отличается от прежнего понижения
///
/// Раньше здесь стояла `lower_addr_expr`, принимавшая **только** литералы:
///
/// ```text
/// _ => None,   // символ, арифметика, всё прочее — «адреса нет», без диагностики
/// ```
///
/// То есть `address BTN = BTN_ADDR;` и `address BTN = 0x100000 + 4;` **молча
/// теряли адрес**, а пользователь получал `SE-052` «порт не имеет адреса» —
/// диагностику, называющую следствие вместо причины. Тот же класс «тихого
/// пропуска», что дал восемь дефектов в фиче 0025.
///
/// Теперь неудача **всегда** названа: `SE-054` (неопределённый символ) либо
/// `SE-055` (не сворачивается в константу).
///
/// # Возврат
///
/// - `Ok(None)` — выражения адреса нет вовсе (порт без инициализатора и без
///   оператора `address`); полноту проверяет вызывающий (`SE-052`).
/// - `Ok(Some(v))` — вычислено.
/// - `Err(d)` — `SE-054`/`SE-055`.
fn eval_addr_expr(
    expr: &ExpressionNode,
    scope: &Rc<RefCell<ModelNode>>,
    env: &AddressEnv,
) -> Result<Option<AddrValue>, Diagnostic> {
    if matches!(expr, ExpressionNode::None) {
        return Ok(None);
    }
    let mut seen = Vec::new();
    eval_addr_value(expr, scope, env, &mut seen).map(Some)
}

/// Разрешает адреса всех портов модели с учётом приоритета источников
/// (inline < `address` < внешняя карта) — фича 0020-05.
///
/// Строит консолидированную карту `имя порта → ResolvedAddress` и собирает
/// диагностики для адрес-потребляющего режима (`c-hal`):
///
/// - **SE-052** — используемый (достижимый кодогенерацией) порт без адреса ни из
///   одного источника (ошибка полноты);
/// - **SE-050** — внешняя карта переопределяет адрес, заданный в модели
///   (предупреждение оверлея);
/// - **SE-051** — запись внешней карты для несуществующего порта.
///
/// Конфликт inline + `address` внутри модели уже исключён семантикой (SE-049 на
/// этапе `construct_model`), поэтому здесь не проверяется.
pub fn resolve_addresses(
    model: Rc<RefCell<ModelNode>>,
    external: &[AddressMapEntry],
    env: &AddressEnv,
) -> AddressResolution {
    let usage = crate::semantic::unused::compute_usage(Rc::clone(&model));
    let external_by_name: HashMap<&str, &AddressMapEntry> =
        external.iter().map(|e| (e.name.as_str(), e)).collect();

    let mut result = AddressResolution::default();
    resolve_model(&model, &usage.ports, &external_by_name, env, &mut result);

    // SE-053: define перекрыл `const` модели. Позиция — у объявления `const`:
    // показать надо то, что перекрыто (симметрия с `SE-050`).
    for (name, const_loc) in env.overrides() {
        result.diagnostics.push(
            Diagnostic::warning(
                const_loc,
                format!(
                    "--define '{}' перекрывает одноимённую `const` модели в выражении адреса; \
                     в логике автомата `const` сохраняет своё значение",
                    name
                ),
            )
            .with_code("SE-053"),
        );
    }

    // DF-004: define, которого не спросило ни одно выражение адреса, — почти
    // всегда опечатка в имени (симметрия с `SE-051`).
    for name in env.unused() {
        result.diagnostics.push(
            Diagnostic::warning(
                Location::CommandLine,
                format!(
                    "--define '{}' не использован: ни одно выражение адреса к нему не обращается",
                    name
                ),
            )
            .with_code("DF-004"),
        );
    }

    // SE-051: записи карты без соответствующего порта.
    for e in external {
        if !result.map.contains_key(&e.name) {
            result.diagnostics.push(
                Diagnostic::warning(
                    e.loc,
                    format!(
                        "внешняя карта задаёт адрес для несуществующего порта '{}'",
                        e.name
                    ),
                )
                .with_code("SE-051"),
            );
        }
    }
    // Порядок детерминирован (фича 0048). Диагностики без файловой позиции
    // (`DF-004` — у аргумента CLI её нет) идут последними: привязать их к месту
    // в тексте не к чему, а `Location::start()` на них паникует.
    result.diagnostics.sort_by_key(|d| match d.loc {
        Location::Source(_, start, _) => (0usize, start),
        _ => (1usize, 0),
    });
    result
}

/// Рекурсивный обход дерева моделей для разрешения адресов портов.
fn resolve_model(
    model: &Rc<RefCell<ModelNode>>,
    used_ports: &std::collections::HashSet<String>,
    external_by_name: &HashMap<&str, &AddressMapEntry>,
    env: &AddressEnv,
    out: &mut AddressResolution,
) {
    let borrowed = model.borrow();
    for var in borrowed.variables.values() {
        let VariableNode::Port {
            expr, loc, name, ..
        } = var
        else {
            continue;
        };

        // Выражения вычисляются ДО выбора слоя-победителя, но в цепочку
        // приоритета (инвариант 0020) это ничего не добавляет.
        //
        // `failed` отличает «источника адреса нет» от «источник есть, но его
        // выражение сломано». Без этого различия рядом с точной причиной
        // (`SE-054`/`SE-055`) вылезал бы и `SE-052` «нет адреса» — то есть
        // пользователь снова получал бы диагностику о следствии, ради ухода от
        // которой фича и делается.
        let mut failed = false;
        let mut eval = |expr: &ExpressionNode, out: &mut AddressResolution| match eval_addr_expr(
            expr, model, env,
        ) {
            Ok(v) => v,
            Err(d) => {
                out.diagnostics.push(d);
                failed = true;
                None
            }
        };
        let inline = eval(expr, out);
        let operator = match borrowed.address_defs.iter().find(|d| &d.port == name) {
            Some(d) => eval(&d.value, out),
            None => None,
        };
        let external = external_by_name.get(name.as_str());

        // Приоритет: внешняя карта > оператор > inline.
        let resolved = if let Some(e) = external {
            // Оверлей поверх адреса модели — предупреждение SE-050.
            if inline.is_some() || operator.is_some() {
                out.diagnostics.push(
                    Diagnostic::warning(
                        e.loc,
                        format!(
                            "внешняя карта переопределяет адрес порта '{}', заданный в модели",
                            name
                        ),
                    )
                    .with_code("SE-050"),
                );
            }
            Some(ResolvedAddress {
                addr: e.addr,
                bit: e.bit,
                source: AddressSource::External,
            })
        } else if let Some((addr, bit)) = operator {
            Some(ResolvedAddress {
                addr,
                bit,
                source: AddressSource::Operator,
            })
        } else {
            inline.map(|(addr, bit)| ResolvedAddress {
                addr,
                bit,
                source: AddressSource::Inline,
            })
        };

        match resolved {
            Some(r) => {
                out.map.insert(name.clone(), r);
            }
            None => {
                // SE-052: используемый порт без адреса ни из одного источника.
                // Если источник был, но его выражение не вычислилось, причина
                // уже названа (`SE-054`/`SE-055`) — второй диагностики не надо.
                if used_ports.contains(name) && !failed {
                    out.diagnostics.push(
                        Diagnostic::error(
                            *loc,
                            format!(
                                "порт '{}' используется в кодогенерации, но не имеет адреса \
                                 (ни inline, ни оператором `address`, ни во внешней карте)",
                                name
                            ),
                        )
                        .with_code("SE-052"),
                    );
                }
            }
        }
    }
    let nested: Vec<Rc<RefCell<ModelNode>>> = borrowed.models.values().map(Rc::clone).collect();
    drop(borrowed);
    for nested_model in nested {
        resolve_model(&nested_model, used_ports, external_by_name, env, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_decimal_and_bit_addressed_entries() {
        let entries = parse_address_map(
            "// карта\nBTN = 0x00200000;\nLED = 0x00200004:3;\nCNT = 42;\n",
            0,
        )
        .expect("карта должна разобраться");
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].name, "BTN");
        assert_eq!(entries[0].addr, 0x0020_0000);
        assert_eq!(entries[0].bit, None);
        assert_eq!(entries[1].name, "LED");
        assert_eq!(entries[1].addr, 0x0020_0004);
        assert_eq!(entries[1].bit, Some(3));
        assert_eq!(entries[2].addr, 42);
    }

    #[test]
    fn empty_and_comment_only_map_is_ok() {
        assert!(parse_address_map("", 0).unwrap().is_empty());
        assert!(
            parse_address_map("// только комментарий\n", 0)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn missing_equals_is_am002() {
        let err = parse_address_map("BTN 0x200000;", 0).unwrap_err();
        assert_eq!(err[0].code.as_deref(), Some("AM-002"));
    }

    #[test]
    fn missing_semicolon_is_am004() {
        let err = parse_address_map("BTN = 0x200000", 0).unwrap_err();
        assert_eq!(err[0].code.as_deref(), Some("AM-004"));
    }

    #[test]
    fn bad_address_literal_is_am005() {
        let err = parse_address_map("BTN = 0xZZ;", 0).unwrap_err();
        assert_eq!(err[0].code.as_deref(), Some("AM-005"));
    }

    #[test]
    fn duplicate_entry_is_am006() {
        let err = parse_address_map("BTN = 0x1; BTN = 0x2;", 0).unwrap_err();
        assert_eq!(err[0].code.as_deref(), Some("AM-006"));
    }

    #[test]
    fn recovers_and_reports_all_errors() {
        // Три битые записи → три диагностики (восстановление до `;`).
        let err = parse_address_map("A 1; B = ; C = 0xZZ;", 0).unwrap_err();
        assert_eq!(err.len(), 3, "должны сообщаться все три ошибки: {:?}", err);
    }
}
