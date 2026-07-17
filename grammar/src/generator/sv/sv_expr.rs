//! Выражения и условия в SystemVerilog (задача 0045-06).
//!
//! ## Две грамматики, а не одна
//!
//! `ConditionNode` (условие ребра `ref S: cond;`) и `ExpressionNode` (тело
//! блока) — **две разные грамматики с разной семантикой `=`**: в условии это
//! равенство, в выражении — присваивание (инвариант фичи 0019, ADR 0019 отверг
//! слияние). Поэтому здесь два печатающих пути, как и в цели `c`.
//!
//! ## Скобки: печатается ДЕРЕВО, а не текст
//!
//! Каждый бинарный узел заключается в скобки. Это не стиль: приоритеты исходного
//! `.lam` уже разобраны парсером, и печать плоского текста полагалась бы на то,
//! что приоритеты SV совпадают с приоритетами Lam. У SystemVerilog они совпадают
//! с C (`==` связывает сильнее `|`), но полагаться на это — ровно та ошибка,
//! которая в цели `rust` дала **молчащий** дефект: там `a == b | c` при `a=2,
//! b=2, c=1` даёт в C `1`, а в Rust `false` (`CLAUDE.md`, фича 0050). Цена
//! лишних скобок — ноль (Verilator на них не ругается, в отличие от `rustc`
//! с его `unused_parens`), цена доверия к совпадению — тихо неверная схема.
//!
//! ## `&`/`|` условий — ЛОГИЧЕСКИЕ, потому что так у эталона
//!
//! ADR оставлял открытым вопрос «`&&` или `&` — решать по типу операнда».
//! Вопрос закрыт чтением эталона: цель `c` печатает `ConditionNode::And` как
//! `&&`, а `Or` — как `||` **безусловно** (`c_expr.rs:656`), независимо от
//! ширины операнда. Сверка модели ведётся против C
//! (`conformance_c_tests.rs`), поэтому расхождение с ним и было бы дефектом:
//! на многобитном операнде `a & b` и `a && b` дают разное. Совпадение с C здесь
//! достигается даром — в SV `&&` над `logic [7:0]` тоже даёт один бит 0/1.
//!
//! ## Литералы печатаются БЕЗ явной ширины
//!
//! ADR требовал ширину у каждого литерала (`8'd7`), обосновывая это тем, что
//! иначе `verilator -Wall` даёт `WIDTHEXPAND`. **Проба 2026-07-16 это
//! опровергла:** модуль с голыми `7`, `0`, `1` в сравнениях и присваиваниях
//! `verilator --lint-only -Wall` принимает **чисто, код 0**. Десятичный литерал
//! в SV самоопределяем и подстраивается под контекст.
//!
//! Граница найдена той же пробой: `x_next = 300;` при `x : logic [7:0]` даёт
//! `%Warning-WIDTHTRUNC` — но это **дефект модели** (значение не влезает в
//! объявленный тип), а не форматирования, и краснеющий на нём гейт прав.
//!
//! Следствие крупное: печатнику **не нужен вывод типа в каждом узле**. У цели
//! `st` он нужен (`ST-011`: без типа операнда не построить имя функции
//! преобразования — `BYTE_TO_USINT(…)`), и это делает её трансляцию
//! типозависимой. Здесь — нет.

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::sv::sv_type::sv_enum_type_name;
use crate::parser::ast::Member;
use crate::semantic::{ConditionNode, ExpressionNode, FunctionDefinitionNode, VariableNode};
use std::collections::{BTreeMap, BTreeSet};

/// Строит диагностику `SV-002` — узел АСД не покрыт печатью.
fn sv002(what: &str) -> Diagnostic {
    Diagnostic::error(
        Location::Codegen,
        format!(
            "{} не транслируется в SystemVerilog целью 'sv'. Молчаливо \
             пропустить конструкцию нельзя: порождённый модуль вёл бы себя \
             иначе, чем модель",
            what
        ),
    )
    .with_code("SV-002")
}

/// Строит диагностику `SV-005` — `extern fn` в синтезируемом RTL невыразима.
fn sv005(name: &str, loc: Location) -> Diagnostic {
    Diagnostic::error(
        loc,
        format!(
            "внешняя функция '{}' (extern fn) целью 'sv' не поддерживается: в \
             синтезируемом RTL вызова внешнего кода не существует. DPI-C — \
             механизм симуляции, синтезатору он недоступен; аппаратный аналог \
             «внешней функции» — отдельный модуль со своим интерфейсом, а \
             extern fn языка Lam не описывает ни портов, ни тактов, ни \
             латентности, и вывести их нельзя. Используйте цель 'c'/'st'/'rust' \
             либо выразите логику на Lam",
            name
        ),
    )
    .with_code("SV-005")
}

/// Контекст печати выражения.
pub(crate) struct Scope<'a> {
    /// Сигналы, имеющие регистровую пару `<имя>_next`.
    ///
    /// Это переменные модели и выходные порты: в `always_comb` **чтение** идёт
    /// из регистра (`name`), а **запись** — в комбинационную пару (`name_next`),
    /// которую `always_ff` защёлкивает по фронту. Разделение и делает такт
    /// тактом. Ключи — **имена сигналов** (уже с префиксом уровня), а не имена
    /// Lam.
    pub(crate) registered: &'a BTreeSet<String>,
    /// Имя объемлющей функции, если печатается её тело.
    ///
    /// Нужно для `return`: в SystemVerilog функция возвращает значение
    /// **присваиванием собственному имени** (`f = t;`). Ключевое слово `return`
    /// синтаксически существует, но **yosys его не принимает** — проба
    /// 2026-07-16: `return t;` → `ERROR: syntax error, unexpected TOK_ID`, тогда
    /// как `verilator --lint-only -Wall` тот же модуль **принял чисто**.
    ///
    /// То есть это ещё один случай, где один Verilator пропустил бы
    /// несинтезируемую конструкцию, — ровно как с `real`. Форма выбрана не по
    /// вкусу, а по тому, что принимают **оба** инструмента.
    pub(crate) function: Option<&'a str>,
    /// Варианты перечислений модели: `имя перечисления → [(вариант, значение)]`.
    ///
    /// Нужны для **восстановления варианта по значению**. `command := Up` в АСД
    /// выглядит как `Assign(Variable, Number(2))`: узла варианта перечисления
    /// `ExpressionNode` не имеет вовсе (та же ловушка описана для цели `rust` в
    /// `CLAUDE.md`). А перечисления SystemVerilog **строго типизированы** —
    /// проба 2026-07-16: `command_next = 2;` даёт
    /// `%Error-ENUMVALUE: Implicit conversion to enum`.
    ///
    /// Приведение (`command_e'(2)`) оба инструмента принимают, но читать его
    /// инженеру хуже, чем `COMMAND_UP`, — а RTL читают. Поэтому значение
    /// восстанавливается в имя варианта, и приведение остаётся запасным путём
    /// для значения, которому варианта нет.
    pub(crate) enums: &'a BTreeMap<String, Vec<(String, i64)>>,
}

impl Scope<'_> {
    /// Имя сигнала для **чтения**: рабочая копия `_next`, если она есть.
    ///
    /// ⚠️ **Читается `_next`, а НЕ регистр — это не оптимизация, а семантика.**
    /// Тело состояния Lam императивно: `v := 1; w := v;` обязано дать `w = 1`
    /// (так в симуляторе; в C — `write(V,1)` затем `read(V)` возвращает
    /// только что записанное). В `always_comb` рабочая копия — `v_next`: она
    /// инициализируется значением регистра умолчанием в начале блока, а затем
    /// накапливает записи такта. Чтение регистра `v` дало бы значение
    /// **предыдущего** такта, то есть `w = 0`, — молча иная модель.
    ///
    /// Ровно этот дефект был внесён и пойман при разработке (2026-07-16):
    /// вывод печатал `w_next = v;`. Симптомом послужил clippy
    /// (`only_used_in_recursion` на `scope`), то есть линтер нашёл семантическую
    /// ошибку раньше гейта — оба инструмента SV её пропускали: модуль валиден и
    /// синтезируем, просто считает не то.
    ///
    /// Единственное место, где читается сам регистр, — умолчания в начале
    /// `always_comb` (`v_next = v;`) и ветвь сброса; оба печатаются напрямую,
    /// минуя этот метод.
    pub(crate) fn read(&self, signal: &str) -> String {
        if self.registered.contains(signal) {
            format!("{}_next", signal)
        } else {
            signal.to_string()
        }
    }

    /// Печатает значение в позиции присваивания элементу типа `ty`.
    ///
    /// Обычные типы печатаются как есть; для перечисления число восстанавливается
    /// в имя варианта (см. поле [`enums`](Scope::enums)).
    pub(crate) fn coerce(
        &self,
        ty: &crate::semantic::type_node::TypeNode,
        value: &ExpressionNode,
    ) -> Result<String, Diagnostic> {
        let crate::semantic::type_node::TypeNode::Enum(enum_name) = ty else {
            return print_expression(value, self);
        };
        let printed = print_expression(value, self)?;
        let ExpressionNode::Number(n) = value else {
            // Значение уже имеет тип перечисления (переменная, вариант) —
            // приводить нечего.
            return Ok(printed);
        };
        if let Some(variants) = self.enums.get(enum_name)
            && let Some((variant, _)) = variants.iter().find(|(_, v)| v == n)
        {
            return Ok(sv_enum_variant_name(enum_name, variant));
        }
        // Варианта с таким значением нет. Приведение — единственный способ
        // напечатать это валидно; молча оставить число нельзя (ENUMVALUE).
        Ok(format!("{}'({})", sv_enum_type_name(enum_name), printed))
    }

    /// Имя сигнала для **записи**: комбинационная пара, если она есть.
    ///
    /// У локальной переменной функции и у константы пары нет: первая живёт
    /// внутри одного вычисления, вторая вообще не регистр.
    pub(crate) fn write(&self, signal: &str) -> String {
        if self.registered.contains(signal) {
            format!("{}_next", signal)
        } else {
            signal.to_string()
        }
    }
}

/// Имя варианта перечисления в SV: `Action`/`Idle` → `ACTION_IDLE`.
///
/// Префикс именем перечисления обязателен: метки `enum` в SystemVerilog живут в
/// **общем пространстве имён модуля**, а не внутри своего типа (в отличие от
/// Rust, где `Action::Idle` квалифицировано). Два перечисления модели с
/// одноимённым вариантом (`Idle` у `Action` и у `Mode`) без префикса дали бы
/// повторное объявление.
pub(crate) fn sv_enum_variant_name(enum_name: &str, variant: &str) -> String {
    format!(
        "{}_{}",
        sv_enum_type_name(enum_name)
            .trim_end_matches("_e")
            .to_uppercase(),
        crate::semantic::naming::normalize_lowercase_snakecase(variant.to_string()).to_uppercase()
    )
}

/// Возвращает имя вызываемой функции, отвергая невыразимые случаи.
///
/// # Ошибки
/// - [`SV-005`](sv005) — `extern fn`: в синтезируемом RTL вызова внешнего кода
///   не существует;
/// - [`SV-002`](sv002) — неразрешённое определение функции.
fn local_function_name(func: &FunctionDefinitionNode, loc: Location) -> Result<String, Diagnostic> {
    match func {
        FunctionDefinitionNode::Local { name, .. } => Ok(name.clone()),
        FunctionDefinitionNode::External { name, .. } => Err(sv005(name, loc)),
        // Встроенные (`min`/`max`/`abs`/`debug`) требуют каждая своего
        // разворачивания и разбираются отдельно (`print_builtin`); сюда попасть
        // не должны.
        FunctionDefinitionNode::Builtin(name, _, _) => Err(sv002(&format!(
            "встроенная функция '{}' в этой позиции",
            name
        ))),
        FunctionDefinitionNode::None | FunctionDefinitionNode::Unresolved(_) => {
            Err(sv002("неразрешённый вызов функции"))
        }
    }
}

/// Печатает вызов функции по уже напечатанным аргументам.
///
/// Общий хвост обоих печатающих путей (условие и выражение): грамматики разные,
/// а правила вызова — одни.
///
/// # Ошибки
/// [`SV-005`](sv005) на `extern fn`, [`SV-002`](sv002) на непереводимой
/// встроенной функции.
fn print_call(
    func: &FunctionDefinitionNode,
    args: &[String],
    loc: Location,
) -> Result<String, Diagnostic> {
    if let FunctionDefinitionNode::Builtin(name, _, _) = func {
        return print_builtin(name, args, loc);
    }
    Ok(format!(
        "{}({})",
        local_function_name(func, loc)?,
        args.join(", ")
    ))
}

/// Разворачивает встроенную функцию языка в выражение SystemVerilog.
///
/// Функции языка (`min`/`max`/`abs`) в SV не существуют — там они
/// **разворачиваются** в тернарный оператор, то есть в мультиплексор. Это не
/// обход, а прямое соответствие: в RTL выбор меньшего из двух и есть
/// мультиплексор со сравнителем.
///
/// # Ошибки
/// [`SV-002`](sv002) на `debug` и на неизвестной встроенной функции.
fn print_builtin(name: &str, args: &[String], _loc: Location) -> Result<String, Diagnostic> {
    match (name, args) {
        ("min", [a, b]) => Ok(format!("(({} < {}) ? {} : {})", a, b, a, b)),
        ("max", [a, b]) => Ok(format!("(({} > {}) ? {} : {})", a, b, a, b)),
        ("abs", [a]) => Ok(format!("(({} < 0) ? -{} : {})", a, a, a)),
        // Молчаливо отбросить нельзя: ровно эту тихую потерю закрыла фича 0035.
        ("debug", _) => Err(sv002(
            "встроенная функция 'debug': в синтезируемом RTL вывода текста не \
             существует — печатать некуда и нечем. Отладка RTL ведётся \
             осциллограммой сигналов, а не печатью; используйте цель \
             'c'/'rust', если нужен вывод",
        )),
        (other, _) => Err(sv002(&format!(
            "встроенная функция '{}' с таким числом аргументов",
            other
        ))),
    }
}

/// Извлекает имя элемента Lam из узла переменной.
///
/// Возвращает **имя Lam**, а не имя сигнала: отображение в сигнал делает
/// [`Scope`], который один знает про префиксы уплощения.
pub(crate) fn signal_of(var: &std::rc::Rc<std::cell::RefCell<VariableNode>>) -> Option<String> {
    match &*var.borrow() {
        // Переменная модели получает префикс ВЛАДЕЛЬЦА, а не «модели, которую
        // сейчас печатаем»: имя владельца берётся из самой переменной (`upper`),
        // поэтому одноимённые переменные разных под-моделей расходятся сами
        // собой, без карты имён и без риска, что карта отобразит не ту.
        VariableNode::Simple { upper, name, .. } => {
            let Some(owner) = upper.as_ref().and_then(|u| u.upgrade()) else {
                // Владельца нет — это не переменная модели, а локальная
                // переменная функции: у неё нет ни регистра, ни префикса.
                return Some(name.clone());
            };
            // ⚠️ Параметры и локальные переменные функции — ТОЖЕ `Simple` и
            // тоже ссылаются на модель. Отличить их можно ровно одним: они не
            // объявлены в самой модели. Без этой проверки `travel_time(to_stack)`
            // печатал бы `stacker_to_stack` — сигнал, которого не существует
            // (проба 2026-07-16: `Can't find definition of variable`).
            if !owner.borrow().variables.contains_key(name) {
                return Some(name.clone());
            }
            let model: crate::semantic::minimap::Name = owner.into();
            Some(format!("{}_{}", model.unique_lowercase_snakecase(), name))
        }
        // Порт — вывод кристалла, его имя задал автор и оно уникально по модулю
        // (`collect_ports` дедуплицирует). Константа — `localparam` уровня
        // модуля. Префикс им не нужен и только мешал бы читать.
        VariableNode::Port { name, .. } | VariableNode::Const { name, .. } => Some(name.clone()),
        VariableNode::Unresolved => None,
    }
}

/// Печатает доступ к члену (`x.0` → `x[0]`, `p.field` → `p.field`).
///
/// **Битового доступа как отдельной конструкции в SV нет и не нужно:** вектор
/// индексируется тем же `[]`, что и массив. Это заметно проще цели `st`, где
/// `x.0` разворачивается в маску `(USINT_TO_BYTE(x) AND 16#01) <> 16#00` —
/// MatIEC не знает ни `x.0`, ни `%X0` (`CLAUDE.md`, фича 0041).
fn print_member(base: &str, member: &Member) -> String {
    match member {
        Member::Number(index) => format!("{}[{}]", base, index),
        Member::Identifier(id) => format!("{}.{}", base, id.name),
    }
}

/// Печатает условие ребра (`ref S: cond;`) в SystemVerilog.
///
/// # Ошибки
/// [`SV-002`](sv002) на непокрытом узле, [`SV-005`](sv005) на `extern fn`.
pub(crate) fn print_condition(node: &ConditionNode, scope: &Scope) -> Result<String, Diagnostic> {
    // Каждый бинарный узел — в скобках: печатается дерево, а не текст.
    let bin = |l: &ConditionNode, op: &str, r: &ConditionNode| -> Result<String, Diagnostic> {
        Ok(format!(
            "({} {} {})",
            print_condition(l, scope)?,
            op,
            print_condition(r, scope)?
        ))
    };
    match node {
        // Безусловное ребро: `if (1'b1)` не печатается — вызывающий код
        // (`sv_fsm`) обязан различать условный и безусловный переход, потому что
        // безусловный делает всё, что ниже него, недостижимым.
        ConditionNode::None => Ok("1'b1".to_string()),
        ConditionNode::Bool(v) => Ok(if *v { "1'b1" } else { "1'b0" }.to_string()),
        ConditionNode::Number(n) => Ok(n.to_string()),
        ConditionNode::Parenthesis(inner) => Ok(format!("({})", print_condition(inner, scope)?)),
        ConditionNode::Not(inner) => Ok(format!("(!{})", print_condition(inner, scope)?)),
        // Логические, а не побитовые: так у эталона (`c_expr.rs:656`).
        ConditionNode::And(l, r) => bin(l, "&&", r),
        ConditionNode::Or(l, r) => bin(l, "||", r),
        ConditionNode::Add(l, r) => bin(l, "+", r),
        ConditionNode::Subtract(l, r) => bin(l, "-", r),
        ConditionNode::Less(l, r) => bin(l, "<", r),
        ConditionNode::More(l, r) => bin(l, ">", r),
        ConditionNode::LessEqual(l, r) => bin(l, "<=", r),
        ConditionNode::MoreEqual(l, r) => bin(l, ">=", r),
        ConditionNode::Equal(l, r) => bin(l, "==", r),
        ConditionNode::NotEqual(l, r) => bin(l, "!=", r),
        ConditionNode::Variable(var, _) => signal_of(var)
            .map(|name| scope.read(&name))
            .ok_or_else(|| sv002("неразрешённая переменная в условии")),
        ConditionNode::ArraySubscript(var, index) => {
            let name = signal_of(var)
                .ok_or_else(|| sv002("неразрешённая переменная в индексации массива"))?;
            Ok(format!(
                "{}[{}]",
                scope.read(&name),
                print_condition(index, scope)?
            ))
        }
        ConditionNode::BitAccess(inner, member) => {
            Ok(print_member(&print_condition(inner, scope)?, member))
        }
        ConditionNode::EnumVariant(def, variant, _) => {
            Ok(sv_enum_variant_name(&def.borrow().name, variant))
        }
        ConditionNode::Function(func, args, loc) => {
            let printed: Result<Vec<String>, Diagnostic> =
                args.iter().map(|a| print_condition(a, scope)).collect();
            print_call(&func.borrow(), &printed?, *loc)
        }
        // Ветки `_` нет намеренно: `ConditionNode` объявлен в этом же крейте,
        // поэтому исчерпывающий разбор возможен — и обязан валить сборку при
        // добавлении варианта, а не проглатывать его молча (R4).
        ConditionNode::Unresolved(_) => Err(sv002("неразрешённое условие")),
        ConditionNode::Rational(_, _) => Err(sv002(
            "вещественный литерал: в синтезируемом RTL плавающей точки нет (см. SV-003)",
        )),
        ConditionNode::String(_) => Err(sv002("строковый литерал")),
        ConditionNode::Model(_, _) => Err(sv002("ссылка на модель в условии")),
        ConditionNode::State(_) => Err(sv002("ссылка на состояние в условии")),
    }
}

/// Печатает выражение (тело блока) в SystemVerilog.
///
/// # Ошибки
/// [`SV-002`](sv002) на непокрытом узле, [`SV-005`](sv005) на `extern fn`.
pub(crate) fn print_expression(node: &ExpressionNode, scope: &Scope) -> Result<String, Diagnostic> {
    let bin = |l: &ExpressionNode, op: &str, r: &ExpressionNode| -> Result<String, Diagnostic> {
        Ok(format!(
            "({} {} {})",
            print_expression(l, scope)?,
            op,
            print_expression(r, scope)?
        ))
    };
    match node {
        ExpressionNode::Number(n) => Ok(n.to_string()),
        ExpressionNode::Bool(v) => Ok(if *v { "1'b1" } else { "1'b0" }.to_string()),
        ExpressionNode::Parenthesis(inner) => Ok(format!("({})", print_expression(inner, scope)?)),
        ExpressionNode::Not(inner) => Ok(format!("(!{})", print_expression(inner, scope)?)),
        ExpressionNode::BitwiseNot(inner) => Ok(format!("(~{})", print_expression(inner, scope)?)),
        ExpressionNode::Negate(inner) => Ok(format!("(-{})", print_expression(inner, scope)?)),
        ExpressionNode::UnaryPlus(inner) => Ok(format!("(+{})", print_expression(inner, scope)?)),
        ExpressionNode::Multiply(l, r) => bin(l, "*", r),
        // `/` и `%` синтезируются в аппаратный делитель — крупный и медленный
        // блок. Предупреждать о цене (кандидат `SV-009` задачи 0045-06) —
        // отдельное решение; здесь трансляция прямая.
        ExpressionNode::Divide(l, r) => bin(l, "/", r),
        ExpressionNode::Modulo(l, r) => bin(l, "%", r),
        ExpressionNode::Add(l, r) => bin(l, "+", r),
        ExpressionNode::Subtract(l, r) => bin(l, "-", r),
        ExpressionNode::ShiftLeft(l, r) => bin(l, "<<", r),
        ExpressionNode::ShiftRight(l, r) => bin(l, ">>", r),
        ExpressionNode::BitwiseAnd(l, r) => bin(l, "&", r),
        ExpressionNode::BitwiseOr(l, r) => bin(l, "|", r),
        ExpressionNode::BitwiseXor(l, r) => bin(l, "^", r),
        ExpressionNode::And(l, r) => bin(l, "&&", r),
        ExpressionNode::Or(l, r) => bin(l, "||", r),
        ExpressionNode::Less(l, r) => bin(l, "<", r),
        ExpressionNode::More(l, r) => bin(l, ">", r),
        ExpressionNode::LessEqual(l, r) => bin(l, "<=", r),
        ExpressionNode::MoreEqual(l, r) => bin(l, ">=", r),
        ExpressionNode::Equal(l, r) => bin(l, "==", r),
        ExpressionNode::NotEqual(l, r) => bin(l, "!=", r),
        ExpressionNode::ConditionalOperator(c, t, f) => Ok(format!(
            "({} ? {} : {})",
            print_expression(c, scope)?,
            print_expression(t, scope)?,
            print_expression(f, scope)?
        )),
        ExpressionNode::Variable(var) => signal_of(var)
            .map(|name| scope.read(&name))
            .ok_or_else(|| sv002("неразрешённая переменная")),
        ExpressionNode::ArraySubscript(var, index) => {
            let name = signal_of(var)
                .ok_or_else(|| sv002("неразрешённая переменная в индексации массива"))?;
            Ok(format!(
                "{}[{}]",
                scope.read(&name),
                print_expression(index, scope)?
            ))
        }
        ExpressionNode::BitAccess(inner, member) => {
            Ok(print_member(&print_expression(inner, scope)?, member))
        }
        ExpressionNode::Function(func, args) => {
            let printed: Result<Vec<String>, Diagnostic> =
                args.iter().map(|a| print_expression(a, scope)).collect();
            let f = func.borrow();
            let loc = f.loc();
            print_call(&f, &printed?, loc)
        }
        // Присваивание — оператор, а не выражение: печатается в `sv_stmt`.
        // Здесь оно означало бы `x = (y = 1)`, чего Lam не строит.
        ExpressionNode::Assign(_, _) => Err(sv002(
            "присваивание внутри выражения (в SystemVerilog присваивание — оператор)",
        )),
        ExpressionNode::Power(_, _) => Err(sv002(
            "возведение в степень: в синтезируемом RTL оператора `**` над \
             переменными не существует (синтезатор требует константу)",
        )),
        ExpressionNode::Rational(_, _) => Err(sv002(
            "вещественный литерал: в синтезируемом RTL плавающей точки нет (см. SV-003)",
        )),
        ExpressionNode::None => Err(sv002("пустое выражение")),
        ExpressionNode::Unresolved(_) => Err(sv002("неразрешённое выражение")),
        ExpressionNode::ArraySlice(_, _, _) => Err(sv002("срез массива")),
        ExpressionNode::CodeBlock(_, _) => Err(sv002("блок кода в выражении")),
        ExpressionNode::NamedFunctionBox(_, _) => Err(sv002("вызов с именованными аргументами")),
        ExpressionNode::String(_) => Err(sv002("строковый литерал")),
        ExpressionNode::Type(_) => Err(sv002("тип в позиции выражения")),
        ExpressionNode::Address(_, _) => Err(sv002(
            "адрес порта: для RTL адрес бессмыслен — сигнал приходит на вывод \
             кристалла, а не по адресу",
        )),
        ExpressionNode::Model(_) => Err(sv002("ссылка на модель в выражении")),
        ExpressionNode::Condition(_) => Err(sv002("именованное условие в позиции выражения")),
        ExpressionNode::List(_) => Err(sv002("список параметров в позиции выражения")),
        ExpressionNode::Array(_) => Err(sv002("литерал массива")),
        ExpressionNode::Initializer(_) => Err(sv002("инициализатор структуры")),
        ExpressionNode::Cast(_, _) => Err(sv002("приведение типа (`as`)")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Пустой контекст: регистровых пар нет.
    fn empty_scope() -> BTreeSet<String> {
        BTreeSet::new()
    }

    fn num(n: i64) -> Box<ConditionNode> {
        Box::new(ConditionNode::Number(n))
    }

    /// **Ключевой тест: печатается дерево, а не текст.**
    ///
    /// `a == b | c` — узел `Or(Equal(a, b), c)`. Скобки вокруг каждого бинарного
    /// узла делают структуру явной и не дают приоритетам целевого языка
    /// переопределить разбор. Именно отсутствие этой дисциплины дало в цели
    /// `rust` МОЛЧАЩИЙ дефект (`CLAUDE.md`, фича 0050).
    #[test]
    fn binary_nodes_are_fully_parenthesized() {
        let set = empty_scope();
        let enums = std::collections::BTreeMap::new();
        let scope = Scope {
            registered: &set,
            function: None,
            enums: &enums,
        };
        let node = ConditionNode::Or(Box::new(ConditionNode::Equal(num(2), num(2))), num(1));
        assert_eq!(print_condition(&node, &scope).unwrap(), "((2 == 2) || 1)");
    }

    /// `&`/`|` условий печатаются ЛОГИЧЕСКИМИ — как у эталона (цель `c`).
    ///
    /// Сверка модели ведётся против C, поэтому расхождение с ним было бы
    /// дефектом: на многобитном операнде `a & b` и `a && b` дают разное.
    #[test]
    fn condition_and_or_are_logical_like_c() {
        let set = empty_scope();
        let enums = std::collections::BTreeMap::new();
        let scope = Scope {
            registered: &set,
            function: None,
            enums: &enums,
        };
        let and = ConditionNode::And(num(1), num(0));
        let or = ConditionNode::Or(num(1), num(0));
        assert_eq!(print_condition(&and, &scope).unwrap(), "(1 && 0)");
        assert_eq!(print_condition(&or, &scope).unwrap(), "(1 || 0)");
    }

    /// Литералы печатаются БЕЗ явной ширины.
    ///
    /// Сторож против возврата к букве ADR: проба 2026-07-16 показала, что
    /// `verilator -Wall` принимает голые литералы чисто, а требование ширины
    /// было основано на несостоявшемся `WIDTHEXPAND`.
    #[test]
    fn literals_are_printed_unsized() {
        let set = empty_scope();
        let enums = std::collections::BTreeMap::new();
        let scope = Scope {
            registered: &set,
            function: None,
            enums: &enums,
        };
        assert_eq!(
            print_condition(&ConditionNode::Number(7), &scope).unwrap(),
            "7"
        );
        assert_eq!(
            print_expression(&ExpressionNode::Number(300), &scope).unwrap(),
            "300"
        );
    }

    /// Булев литерал печатается однобитным: `1'b1`/`1'b0`.
    #[test]
    fn bool_literals_are_one_bit() {
        let set = empty_scope();
        let enums = std::collections::BTreeMap::new();
        let scope = Scope {
            registered: &set,
            function: None,
            enums: &enums,
        };
        assert_eq!(
            print_condition(&ConditionNode::Bool(true), &scope).unwrap(),
            "1'b1"
        );
        assert_eq!(
            print_expression(&ExpressionNode::Bool(false), &scope).unwrap(),
            "1'b0"
        );
    }

    /// Запись в регистровый сигнал идёт в комбинационную пару `_next`.
    #[test]
    fn write_targets_next_signal() {
        let mut set = BTreeSet::new();
        set.insert("cmd_fork".to_string());
        let enums = std::collections::BTreeMap::new();
        let scope = Scope {
            registered: &set,
            function: None,
            enums: &enums,
        };
        assert_eq!(scope.write("cmd_fork"), "cmd_fork_next");
        // У локальной переменной функции пары нет.
        assert_eq!(scope.write("tmp"), "tmp");
    }

    /// **Уплощение:** префиксованный сигнал под-модели получает свою пару `_next`.
    ///
    /// У цели `c` две под-модели вправе иметь переменную `counter` — они лежат
    /// по своим структурам. В одном модуле SV они слиплись бы, поэтому имя
    /// сигнала строит `signal_of` по ВЛАДЕЛЬЦУ переменной, а `_next` — от уже
    /// готового имени сигнала.
    #[test]
    fn submodel_signal_gets_next_pair() {
        let mut set = BTreeSet::new();
        set.insert("cabin_counter".to_string());
        let enums = std::collections::BTreeMap::new();
        let scope = Scope {
            registered: &set,
            function: None,
            enums: &enums,
        };
        assert_eq!(scope.write("cabin_counter"), "cabin_counter_next");
    }

    /// Доступ к биту `x.0` → `x[0]`: в SV вектор индексируется как массив.
    ///
    /// Заметно проще цели `st`, где это разворачивается в маску: MatIEC не знает
    /// ни `x.0`, ни `%X0`.
    #[test]
    fn bit_access_is_plain_indexing() {
        let set = empty_scope();
        let enums = std::collections::BTreeMap::new();
        let scope = Scope {
            registered: &set,
            function: None,
            enums: &enums,
        };
        let node = ConditionNode::BitAccess(num(5), Member::Number(0));
        assert_eq!(print_condition(&node, &scope).unwrap(), "5[0]");
    }

    /// Вариант перечисления получает префикс имени перечисления.
    ///
    /// Метки `enum` в SV живут в общем пространстве имён модуля: два
    /// перечисления с вариантом `Idle` без префикса дали бы повторное
    /// объявление.
    #[test]
    fn enum_variant_is_prefixed_by_enum_name() {
        assert_eq!(sv_enum_variant_name("Action", "Idle"), "ACTION_IDLE");
        assert_eq!(sv_enum_variant_name("Mode", "Idle"), "MODE_IDLE");
    }

    /// **Контрпример:** вещественный литерал в условии → `SV-002`, а не молчание.
    #[test]
    fn rational_literal_is_sv002() {
        let set = empty_scope();
        let enums = std::collections::BTreeMap::new();
        let scope = Scope {
            registered: &set,
            function: None,
            enums: &enums,
        };
        let err = print_condition(&ConditionNode::Rational("1.5".to_string(), false), &scope)
            .unwrap_err();
        assert_eq!(err.code.as_deref(), Some("SV-002"));
    }

    /// **Контрпример:** непереводимые узлы выражения дают `SV-002`.
    #[test]
    fn untranslatable_expression_nodes_are_sv002() {
        let set = empty_scope();
        let enums = std::collections::BTreeMap::new();
        let scope = Scope {
            registered: &set,
            function: None,
            enums: &enums,
        };
        for node in [
            ExpressionNode::None,
            ExpressionNode::String(vec!["s".to_string()]),
            ExpressionNode::Array(vec![]),
            ExpressionNode::Address(0x100, 0),
        ] {
            let err = print_expression(&node, &scope).unwrap_err();
            assert_eq!(err.code.as_deref(), Some("SV-002"), "узел {:?}", node);
        }
    }
}
