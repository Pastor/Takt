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
//! `.takt` уже разобраны парсером, и печать плоского текста полагалась бы на то,
//! что приоритеты SV совпадают с приоритетами Takt. У SystemVerilog они совпадают
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
use crate::semantic::type_node::TypeNode;
use crate::semantic::{ConditionNode, ExpressionNode, FunctionDefinitionNode, VariableNode};
use std::collections::{BTreeMap, BTreeSet};

/// Строит диагностику `SV-002` — узел АСД не покрыт печатью.
pub(crate) fn sv002(what: &str) -> Diagnostic {
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
             extern fn языка Takt не описывает ни портов, ни тактов, ни \
             латентности, и вывести их нельзя. Используйте цель 'c'/'st'/'rust' \
             либо выразите логику на Takt",
            name
        ),
    )
    .with_code("SV-005")
}

/// Строит предупреждение `SV-009` — переменный делитель (фича 0064).
///
/// **Предупреждение, а не ошибка** (правило 1 ADR 0064): деление синтезируется
/// корректно, и отказывать в трансляции рабочего кода цель не вправе.
///
/// Текст называет **причину** (в RTL нет аппаратного делителя) и **следствие**
/// (длинный комбинационный путь → потолок частоты), но **без** конкретных LUT:
/// они технологичны (`synth_ice40` ≠ `synth_xilinx`) и устареют (правило 5 ADR,
/// action A-1). Про `*` умолчание обосновано замером (DSP, дешевле сложения),
/// про делитель-константу — тоже (17…106 LUT); срабатывает лишь переменный.
fn sv009() -> Diagnostic {
    Diagnostic::warning(
        Location::Codegen,
        "деление или остаток по переменному делителю: в синтезируемом RTL нет \
         аппаратного делителя, поэтому '/' и '%' разворачиваются в крупный \
         комбинационный блок (на порядок больше сложения) с длинным путём — это \
         снижает достижимый потолок тактовой частоты. Делитель-константа так не \
         стоит (сворачивается в сдвиги/умножение на обратное); если делитель по \
         существу постоянен, вынесите его в константу"
            .to_string(),
    )
    .with_code("SV-009")
}

/// Делитель — константа времени компиляции (числовой литерал)?
///
/// Правила 2–3 ADR 0064: константный делитель молчит (замер: `a / 2` → 17 LUT,
/// `a / 3` → 106 LUT — дёшево), переменный → `SV-009` (558 LUT, ноль DSP).
/// Проверка **синтаксическая** и **консервативная** (action A-2): `b := 3; a / b`
/// предупредит, хотя синтезатор свернул бы, — лишнее предупреждение честнее
/// пропущенного. Скобки снимаются: `a / (2)` — та же константа.
fn is_constant_divisor(node: &ExpressionNode) -> bool {
    match node {
        ExpressionNode::Number(_) => true,
        ExpressionNode::Parenthesis(inner) => is_constant_divisor(inner),
        _ => false,
    }
}

/// Дописывает `SV-009` в приёмник, если делитель не константа (фича 0064).
fn warn_variable_divisor(scope: &Scope, divisor: &ExpressionNode) {
    if !is_constant_divisor(divisor) {
        scope.warnings.borrow_mut().push(sv009());
    }
}

/// Контекст печати выражения.
pub(crate) struct Scope<'a> {
    /// Сигналы, имеющие регистровую пару `<имя>_next`.
    ///
    /// Это переменные модели и выходные порты: в `always_comb` **чтение** идёт
    /// из регистра (`name`), а **запись** — в комбинационную пару (`name_next`),
    /// которую `always_ff` защёлкивает по фронту. Разделение и делает такт
    /// тактом. Ключи — **имена сигналов** (уже с префиксом уровня), а не имена
    /// Takt.
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
    /// Приёмник предупреждений генератора (фича 0064).
    ///
    /// `print_expression` берётся по `&Scope`, но `SV-009` (переменный делитель)
    /// рождается именно здесь — в единственной точке трансляции всех выражений
    /// (тела, условия, функции). Интерьерная мутабельность позволяет дописать
    /// диагностику, не протаскивая `&mut` сквозь 32 места вызова печатника.
    /// Владелец ячейки — [`super::sv_fsm::Fsm`], доставку делает `generate_program`.
    pub(crate) warnings: &'a std::cell::RefCell<Vec<Diagnostic>>,
}

impl Scope<'_> {
    /// Имя сигнала для **чтения**: рабочая копия `_next`, если она есть.
    ///
    /// ⚠️ **Читается `_next`, а НЕ регистр — это не оптимизация, а семантика.**
    /// Тело состояния Takt императивно: `v := 1; w := v;` обязано дать `w = 1`
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

/// Извлекает имя элемента Takt из узла переменной.
///
/// Возвращает **имя Takt**, а не имя сигнала: отображение в сигнал делает
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
        // Длительность (фича 0134): эмиссия — задача этой цели; до неё явный
        // отказ, а не печать наносекунд обычным числом.
        ConditionNode::Duration(_) | ConditionNode::After(_) | ConditionNode::AfterTicks(_) => {
            Err(Diagnostic::error(
                Location::Codegen,
                "длительность целью 'sv' пока не поддерживается".to_string(),
            )
            .with_code("SV-015"))
        }
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
        ConditionNode::State(..) => Err(sv002("ссылка на состояние в условии")),
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
    // Q-путь бинарной операции (0061): `Some` ⇔ узел имеет тип q(m, n).
    let fixed_bin = |node: &ExpressionNode,
                     op: super::sv_fixed::FixedOp,
                     l: &ExpressionNode,
                     r: &ExpressionNode|
     -> Option<Result<String, Diagnostic>> {
        super::sv_fixed::fixed_format(node)
            .map(|(m, n)| super::sv_fixed::binary(op, l, r, scope, m, n))
    };
    match node {
        // Длительность (фича 0134): эмиссия — задача этой цели; до неё явный
        // отказ, а не печать наносекунд обычным числом.
        ExpressionNode::Duration(_) => Err(Diagnostic::error(
            Location::Codegen,
            "длительность целью 'sv' пока не поддерживается".to_string(),
        )
        .with_code("SV-015")),
        ExpressionNode::Number(n) => Ok(n.to_string()),
        ExpressionNode::Bool(v) => Ok(if *v { "1'b1" } else { "1'b0" }.to_string()),
        ExpressionNode::Parenthesis(inner) => Ok(format!("({})", print_expression(inner, scope)?)),
        ExpressionNode::Not(inner) => Ok(format!("(!{})", print_expression(inner, scope)?)),
        ExpressionNode::BitwiseNot(inner) => Ok(format!("(~{})", print_expression(inner, scope)?)),
        ExpressionNode::Negate(inner) => match super::sv_fixed::fixed_format(node) {
            Some((m, n)) => super::sv_fixed::negate(inner, scope, m, n),
            None => Ok(format!("(-{})", print_expression(inner, scope)?)),
        },
        ExpressionNode::UnaryPlus(inner) => Ok(format!("(+{})", print_expression(inner, scope)?)),
        // Над q(m, n) — масштабирующая Q-арифметика (0061); иначе прямая.
        ExpressionNode::Multiply(l, r) => fixed_bin(node, super::sv_fixed::FixedOp::Multiply, l, r)
            .unwrap_or_else(|| bin(l, "*", r)),
        // `/` и `%` синтезируются в аппаратный делитель — крупный и медленный
        // блок (фича 0064). Трансляция прямая (и для q(m, n) через `fixed_bin`,
        // action A-3), но переменный делитель порождает `SV-009`: автор `.takt`
        // цену потолка частоты из текста модели иначе не увидит. Константа —
        // молча (замер: дёшево).
        ExpressionNode::Divide(l, r) => {
            warn_variable_divisor(scope, r);
            fixed_bin(node, super::sv_fixed::FixedOp::Divide, l, r)
                .unwrap_or_else(|| bin(l, "/", r))
        }
        ExpressionNode::Modulo(l, r) => {
            warn_variable_divisor(scope, r);
            bin(l, "%", r)
        }
        ExpressionNode::Add(l, r) => {
            fixed_bin(node, super::sv_fixed::FixedOp::Add, l, r).unwrap_or_else(|| bin(l, "+", r))
        }
        ExpressionNode::Subtract(l, r) => fixed_bin(node, super::sv_fixed::FixedOp::Subtract, l, r)
            .unwrap_or_else(|| bin(l, "-", r)),
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
        // Здесь оно означало бы `x = (y = 1)`, чего Takt не строит.
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
        // Fixed-point (0061): масштабирующее приведение, когда источник либо цель
        // — q(m, n). Прочие `as` целью sv по-прежнему не транслируются.
        ExpressionNode::Cast(inner, ty) => {
            if matches!(ty, TypeNode::Fixed { .. })
                || super::sv_fixed::fixed_format(inner).is_some()
            {
                super::sv_fixed::cast(inner, ty, scope)
            } else {
                Err(sv002("приведение типа (`as`)"))
            }
        }
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
        let warnings = std::cell::RefCell::new(Vec::new());
        let scope = Scope {
            registered: &set,
            function: None,
            enums: &enums,
            warnings: &warnings,
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
        let warnings = std::cell::RefCell::new(Vec::new());
        let scope = Scope {
            registered: &set,
            function: None,
            enums: &enums,
            warnings: &warnings,
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
        let warnings = std::cell::RefCell::new(Vec::new());
        let scope = Scope {
            registered: &set,
            function: None,
            enums: &enums,
            warnings: &warnings,
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
        let warnings = std::cell::RefCell::new(Vec::new());
        let scope = Scope {
            registered: &set,
            function: None,
            enums: &enums,
            warnings: &warnings,
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
        let warnings = std::cell::RefCell::new(Vec::new());
        let scope = Scope {
            registered: &set,
            function: None,
            enums: &enums,
            warnings: &warnings,
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
        let warnings = std::cell::RefCell::new(Vec::new());
        let scope = Scope {
            registered: &set,
            function: None,
            enums: &enums,
            warnings: &warnings,
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
        let warnings = std::cell::RefCell::new(Vec::new());
        let scope = Scope {
            registered: &set,
            function: None,
            enums: &enums,
            warnings: &warnings,
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
        let warnings = std::cell::RefCell::new(Vec::new());
        let scope = Scope {
            registered: &set,
            function: None,
            enums: &enums,
            warnings: &warnings,
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
        let warnings = std::cell::RefCell::new(Vec::new());
        let scope = Scope {
            registered: &set,
            function: None,
            enums: &enums,
            warnings: &warnings,
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

    // --- Фича 0064: предупреждение `SV-009` о переменном делителе ---

    /// Строит `ExpressionNode::Variable` без владельца (сигнал = имя).
    fn var(name: &str) -> Box<ExpressionNode> {
        Box::new(ExpressionNode::Variable(std::rc::Rc::new(
            std::cell::RefCell::new(VariableNode::Simple {
                upper: None,
                loc: Location::Implicit,
                name: name.to_string(),
                ty: TypeNode::Integer {
                    bits: 8,
                    signed: false,
                },
                expr: ExpressionNode::None,
            }),
        )))
    }

    /// Печатает выражение и возвращает вместе с собранными предупреждениями.
    fn print_with_warnings(node: &ExpressionNode) -> (String, Vec<Diagnostic>) {
        let set = empty_scope();
        let enums = std::collections::BTreeMap::new();
        let warnings = std::cell::RefCell::new(Vec::new());
        let scope = Scope {
            registered: &set,
            function: None,
            enums: &enums,
            warnings: &warnings,
        };
        let out = print_expression(node, &scope).unwrap();
        (out, warnings.into_inner())
    }

    fn codes(warnings: &[Diagnostic]) -> Vec<&str> {
        warnings.iter().filter_map(|w| w.code.as_deref()).collect()
    }

    /// T1/A1: `a / b` (переменный делитель) → `SV-009`; трансляция успешна.
    #[test]
    fn variable_divide_warns_sv009() {
        let node = ExpressionNode::Divide(var("a"), var("b"));
        let (out, warnings) = print_with_warnings(&node);
        assert_eq!(out, "(a / b)", "трансляция обязана состояться");
        assert_eq!(codes(&warnings), ["SV-009"]);
    }

    /// T3/A3: `a % b` (переменный делитель) → `SV-009`.
    #[test]
    fn variable_modulo_warns_sv009() {
        let node = ExpressionNode::Modulo(var("a"), var("b"));
        let (out, warnings) = print_with_warnings(&node);
        assert_eq!(out, "(a % b)");
        assert_eq!(codes(&warnings), ["SV-009"]);
    }

    /// T2/A2: `a / 2` (константа — степень двойки) → **молчание** (замер: 17 LUT).
    #[test]
    fn constant_power_of_two_divide_is_silent() {
        let node = ExpressionNode::Divide(var("a"), Box::new(ExpressionNode::Number(2)));
        let (_, warnings) = print_with_warnings(&node);
        assert!(
            warnings.is_empty(),
            "константа-делитель обязана молчать: {warnings:?}"
        );
    }

    /// T4/A2: `a / 3` (константа — не степень двойки) → **молчание** (≈106 LUT).
    #[test]
    fn constant_non_power_of_two_divide_is_silent() {
        let node = ExpressionNode::Divide(var("a"), Box::new(ExpressionNode::Number(3)));
        let (_, warnings) = print_with_warnings(&node);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    /// T6/A3: `a % 4` (константа) → **молчание**.
    #[test]
    fn constant_modulo_is_silent() {
        let node = ExpressionNode::Modulo(var("a"), Box::new(ExpressionNode::Number(4)));
        let (_, warnings) = print_with_warnings(&node);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    /// Скобки вокруг константы не превращают её в переменную: `a / (2)` — молчит.
    #[test]
    fn parenthesised_constant_divisor_is_silent() {
        let inner = Box::new(ExpressionNode::Parenthesis(Box::new(
            ExpressionNode::Number(2),
        )));
        let node = ExpressionNode::Divide(var("a"), inner);
        let (_, warnings) = print_with_warnings(&node);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    /// T5/A4: `a * b` → **молчание** (замер: 3 DSP + 17 LUT — дешевле сложения).
    #[test]
    fn multiply_is_silent() {
        let node = ExpressionNode::Multiply(var("a"), var("b"));
        let (out, warnings) = print_with_warnings(&node);
        assert_eq!(out, "(a * b)");
        assert!(warnings.is_empty(), "умножение молчит (DSP): {warnings:?}");
    }

    /// T8/A5: текст называет причину (нет аппаратного делителя) и следствие
    /// (потолок частоты), **без** конкретных LUT (они технологичны, action A-1).
    #[test]
    fn sv009_text_names_cause_and_consequence_without_luts() {
        let msg = &sv009().message;
        assert!(msg.contains("аппаратного делителя"), "нет причины:\n{msg}");
        assert!(
            msg.contains("частот"),
            "нет следствия (потолок частоты):\n{msg}"
        );
        assert!(
            !msg.contains("LUT") && !msg.contains("558"),
            "текст не должен называть конкретные LUT (устареют):\n{msg}"
        );
    }
}
