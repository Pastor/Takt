//! Регистровый файл цели `sv-mmio` (фича 0062).
//!
//! ## Что делает цель
//!
//! Карта адресов ([0020](../../../../docs/features/0020-port-address-decl.md))
//! целью `sv` **не потребляется** — MMIO-адрес для RTL бессмыслен (ADR 0045,
//! вопрос 5, Option A). Но осмысленна другая трактовка: породить **регистровый
//! файл** — модуль с синхронным регистровым интерфейсом, где адрес порта задаёт
//! смещение регистра, а бит — позицию внутри слова. Это парная цель, как `c-hal`
//! парная к `c` (ADR 0062, Option B).
//!
//! ## Форма интерфейса — единственное, что здесь изобретается
//!
//! ADR оставил **протокол** шины (APB/AXI-Lite/Wishbone) заказчику: выбор
//! произволен, а цена ошибки высока (возражение ADR 0045). Поэтому интерфейс —
//! **шинно-агностичный** и синхронный: `reg_addr` (адрес), `reg_wdata` (данные
//! записи), `reg_wen` (строб записи), `reg_rdata` (данные чтения, комбинационные).
//! Адаптер под конкретный протокол — тонкий шим поверх, отдельная фича по
//! требованию (A-2 ADR). BFM тест-плану не нужен: интерфейс дёргается напрямую.
//!
//! ## Направление принадлежит биту, а не слову (правило 4 ADR)
//!
//! Одно слово может нести биты обоих направлений — это **факт корпуса**
//! (`extend_complex.lam`: `out :1`, `out :2`, `in :33`), а не гипотеза. Поэтому:
//!
//! - бит **`out`** — регистр автомата (защёлкнут в `always_ff` автомата),
//!   шина его только **читает**; запись шиной **игнорируется** (правило 5 —
//!   иначе конфликт драйверов);
//! - бит **`in`** — регистр, **записываемый шиной** и читаемый автоматом;
//!   чтение шиной возвращает записанное (правило 5).
//!
//! ## Ширина данных — по старшему занятому биту; предел — 64 (фикс 0020-01, 0098)
//!
//! Ширина слова данных ([`Mmio::data_width`]) — максимум `bit + width` по портам
//! модели, **не** фиксированные 64: лишние старшие биты `reg_wdata` повисли бы
//! как `UNUSEDSIGNAL` у `verilator -Wall` (глушить его `lint_off` правило проекта
//! запрещает). Жёсткий предел — 64 ([`MAX_REG_WIDTH`]): `SE-060` держит бит в
//! `[0, 63]`, то есть регистр не шире `uint64_t` (то же слово, что читает
//! дефолтный HAL стороны `c-hal`). Порт занимает срез `reg_*[bit +: width]`;
//! выход за 64 (`bit + width > 64`) — **отказ** (`SV-013`), а не догадка (R6).

use crate::address_map::ResolvedAddress;
use crate::diagnostics::{Diagnostic, Location};
use crate::generator::indent::Printer;
use crate::generator::sv::sv_fsm::Block;
use crate::generator::sv::sv_type::{SvType, enum_width, sv_type};
use crate::semantic::PortDirection;
use crate::semantic::type_node::TypeNode;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Жёсткий предел ширины регистра (бит).
///
/// 64 — предел `SE-060` (бит адреса в `[0, 63]`) и слово дефолтного HAL стороны
/// `c-hal` (`uint64_t`, фикс 0020-01). Порт с `bit + width > 64` не помещается в
/// регистр → [`SV-013`](sv013). **Реальная** ширина шины данных
/// ([`Mmio::data_width`]) — максимум `bit + width` по портам модели: шире, чем
/// нужно, шину не эмитим, иначе `verilator -Wall` даёт `UNUSEDSIGNAL` на старших
/// битах `reg_wdata` (глушить его `lint_off` правило проекта запрещает).
const MAX_REG_WIDTH: u32 = 64;

/// Имена, которые порождает сам регистровый интерфейс цели `sv-mmio`.
///
/// Совпадение пользовательского **неадресованного** порта/переменной с любым из
/// них дало бы два объявления одного идентификатора — [`SV-014`](sv014). (У
/// `sv` этих имён нет, поэтому в общий `RESERVED_NAMES` они не вынесены: там —
/// имена, которые цель порождает **всегда**.)
const REG_IFACE_NAMES: &[&str] = &["reg_addr", "reg_wdata", "reg_wen", "reg_rdata"];

/// Строит диагностику `SV-013` — срез порта не помещается в 64-битный регистр.
fn sv013(name: &str, bit: i64, width: u32) -> Diagnostic {
    Diagnostic::error(
        Location::Codegen,
        format!(
            "порт '{}' занимает биты [{}..{}] адреса, но регистр цели 'sv-mmio' \
             шириной 64 бита (слово дефолтного HAL): срез bit+width={} выходит за \
             границу. Ширину адресуемого слова язык Takt не выражает, поэтому \
             угадать её нельзя — сузьте тип порта или сместите бит так, чтобы \
             bit+width не превышало 64",
            name,
            bit,
            bit + i64::from(width) - 1,
            bit + i64::from(width)
        ),
    )
    .with_code("SV-013")
}

/// Строит диагностику `SV-014` — имя совпало с сигналом регистрового интерфейса.
fn sv014(name: &str) -> Diagnostic {
    Diagnostic::error(
        Location::Codegen,
        format!(
            "имя '{}' зарезервировано регистровым интерфейсом цели 'sv-mmio' \
             (reg_addr/reg_wdata/reg_wen/reg_rdata — сигналы шины, которых в .lam \
             нет). Это НЕ ключевое слово языка Takt: модель остаётся валидной для \
             целей 'c', 'c-hal', 'plantuml', 'st', 'rust' и 'sv'. Переименуйте \
             элемент в исходнике .lam, если модель нужна как регистровый файл",
            name
        ),
    )
    .with_code("SV-014")
}

/// Строит диагностику `SV-002` — тип порта не ложится на биты регистра.
fn sv002_width(name: &str, ty: &TypeNode) -> Diagnostic {
    Diagnostic::error(
        Location::Codegen,
        format!(
            "порт '{}' с адресом имеет тип '{}', ширина которого в битах не \
             определена (регистровый файл цели 'sv-mmio' раскладывает порт по \
             битам слова). Адресуйте порт скалярного типа (bit/целое/q) либо \
             используйте цель 'sv' без адресов",
            name, ty
        ),
    )
    .with_code("SV-002")
}

/// Адресованный порт: занимает срез `[bit +: width]` регистра по адресу `addr`.
pub(crate) struct MmioPort {
    /// Имя порта — как в исходнике `.lam` (совпадает с именем сигнала автомата).
    pub(crate) name: String,
    /// Тип порта (для объявления регистра автомата у `out`-битов).
    pub(crate) ty: TypeNode,
    /// Адрес регистра.
    addr: i64,
    /// Начальный бит внутри слова (умолчание 0, если `0xADDR` без `:bit`).
    bit: i64,
    /// Ширина порта в битах.
    width: u32,
    /// Направление: `in` — пишется шиной; `out` — читается шиной.
    direction: PortDirection,
}

/// Регистровый файл: адресованные порты и ширины шин.
pub(crate) struct Mmio {
    /// Адресованные порты, отсортированы по имени (детерминизм, фича 0048).
    ports: Vec<MmioPort>,
    /// Ширина `reg_addr` в битах — по **максимальному** адресу модели (R3 ADR).
    addr_width: u32,
    /// Ширина `reg_wdata`/`reg_rdata` в битах — максимум `bit + width` по портам
    /// (не [`MAX_REG_WIDTH`]: лишние старшие биты дали бы `UNUSEDSIGNAL`).
    data_width: u32,
}

impl Mmio {
    /// Строит регистровый файл из разрешённой карты адресов.
    ///
    /// `address_map` уже прошёл `resolve_addresses` (приоритет источников +
    /// `SE-060` на бит вне `[0, 63]`), поэтому здесь остаются лишь проверки,
    /// специфичные регистровому файлу: ширина среза (`SV-013`), тип
    /// (`SV-002`), коллизия имени с интерфейсом (`SV-014`) и невыразимое
    /// направление `inout`.
    ///
    /// # Ошибки
    /// [`SV-013`](sv013), [`SV-014`](sv014), [`SV-002`](sv002_width),
    /// `SV-006` (через [`sv_type`] на несовместимом типе — не наступает для
    /// скаляров).
    pub(crate) fn build(
        blocks: &[Block],
        address_map: &HashMap<String, ResolvedAddress>,
    ) -> Result<Self, Diagnostic> {
        // Перечисления собираются со всех уровней: ширина enum-порта считается по
        // диапазону его значений (как в `sv_type::enum_width`).
        let mut enums: BTreeMap<String, Vec<(String, i64)>> = BTreeMap::new();
        for (_, model_rc) in blocks {
            for def in model_rc.borrow().enums.values() {
                enums
                    .entry(def.name.clone())
                    .or_insert_with(|| def.variants.clone());
            }
        }

        // Коллизия неадресованного сигнала с именем интерфейса. Проверяются
        // ВСЕ порты/переменные/константы модели: адресованный порт `reg_addr`
        // стал бы битом регистра, но неадресованный — портом модуля рядом с
        // сигналом интерфейса.
        for (_, model_rc) in blocks {
            for name in model_rc.borrow().variables.keys() {
                if REG_IFACE_NAMES.contains(&name.as_str()) {
                    return Err(sv014(name));
                }
            }
        }

        let mut ports: Vec<MmioPort> = Vec::new();
        for resolved in address_map.values() {
            // Фича 0084: ключ карты квалифицирован моделью; имя регистра
            // (пользовательское) — голое `resolved.name`, не ключ.
            let name = &resolved.name;
            let what = format!("порт '{}'", name);
            let width = bit_width(&resolved.ty, &enums, &what)
                .ok_or_else(|| sv002_width(name, &resolved.ty))?;
            let bit = resolved.bit.unwrap_or(0);
            // Бит уже в [0, 63] (SE-060). Здесь — верхняя граница среза.
            if bit + i64::from(width) > i64::from(MAX_REG_WIDTH) {
                return Err(sv013(name, bit, width));
            }
            if matches!(resolved.direction, PortDirection::InOut) {
                return Err(Diagnostic::error(
                    Location::Codegen,
                    format!(
                        "порт '{}': направление 'inout' целью 'sv-mmio' не \
                         поддерживается — направление принадлежит биту регистра \
                         (бит либо пишется шиной, либо читается ею), а inout не \
                         выражает, когда бит ведёт линию. Разделите порт на \
                         входной и выходной",
                        name
                    ),
                )
                .with_code("SV-006"));
            }
            ports.push(MmioPort {
                name: name.clone(),
                ty: resolved.ty.clone(),
                addr: resolved.addr,
                bit,
                width,
                direction: resolved.direction,
            });
        }
        // Детерминизм эмиссии (фича 0048): порядок задаётся именем, а не обходом
        // `HashMap`. Группировка по адресу в эмиттерах — через `BTreeMap`.
        ports.sort_by(|a, b| a.name.cmp(&b.name));

        let max_addr = ports.iter().map(|p| p.addr).max().unwrap_or(0);
        let addr_width = address_bits(max_addr);
        // Ширина данных — по самому старшему занятому биту (min 1). Шире не надо:
        // старшие биты `reg_wdata` иначе повисли бы как `UNUSEDSIGNAL`.
        let data_width = ports
            .iter()
            .map(|p| (p.bit + i64::from(p.width)) as u32)
            .max()
            .unwrap_or(1)
            .max(1);

        Ok(Self {
            ports,
            addr_width,
            data_width,
        })
    }

    /// Имена адресованных портов — их `collect_ports` исключает из портов модуля.
    pub(crate) fn addressed_names(&self) -> BTreeSet<String> {
        self.ports.iter().map(|p| p.name.clone()).collect()
    }

    /// Адресованные `out`-порты: становятся внутренними регистрами автомата.
    pub(crate) fn outputs(&self) -> impl Iterator<Item = &MmioPort> {
        self.ports
            .iter()
            .filter(|p| matches!(p.direction, PortDirection::Out))
    }

    /// Есть ли хоть один адресованный порт (иначе интерфейс не эмитится).
    fn is_empty(&self) -> bool {
        self.ports.is_empty()
    }

    /// Группировка портов по адресу (адреса — по возрастанию, детерминизм).
    fn by_address(&self) -> BTreeMap<i64, Vec<&MmioPort>> {
        let mut groups: BTreeMap<i64, Vec<&MmioPort>> = BTreeMap::new();
        for port in &self.ports {
            groups.entry(port.addr).or_default().push(port);
        }
        groups
    }
}

/// Тип `out`-порта в объявлении регистра автомата (для `sv_fsm::Fsm::build`).
pub(crate) fn port_sv_type(port: &MmioPort) -> Result<SvType, Diagnostic> {
    sv_type(&port.ty, &format!("порт '{}'", port.name))
}

/// Имя сигнала `out`-порта — совпадает с именем в `.lam` (как у портов модуля).
pub(crate) fn port_signal_name(port: &MmioPort) -> &str {
    &port.name
}

/// Ширина типа в битах для среза регистра.
///
/// Возвращает `None`, если ширина не определена (массив, структура, `float`,
/// неразрешённый тип) — такой порт битом регистра быть не может.
fn bit_width(
    ty: &TypeNode,
    enums: &BTreeMap<String, Vec<(String, i64)>>,
    what: &str,
) -> Option<u32> {
    match ty {
        TypeNode::Bit | TypeNode::Bool => Some(1),
        TypeNode::Integer { bits, .. } if *bits > 0 => Some(u32::from(*bits)),
        TypeNode::Fixed { m, n } => Some(u32::from(*m) + u32::from(*n)),
        TypeNode::Enum(name) => {
            let variants = enums.get(name)?;
            enum_width(variants, what).ok().map(|(w, _)| w)
        }
        _ => None,
    }
}

/// Число бит, нужное для представления адреса `max_addr` (минимум 1).
fn address_bits(max_addr: i64) -> u32 {
    let m = max_addr.max(0) as u64;
    if m == 0 { 1 } else { 64 - m.leading_zeros() }
}

/// Литерал адреса в формате SV: `<addr_width>'h<hex>`.
fn addr_literal(addr: i64, addr_width: u32) -> String {
    format!("{}'h{:x}", addr_width, addr)
}

/// Печатает строки регистрового интерфейса в заголовок модуля (после `en`).
///
/// Вызывается из [`sv_module::emit_module_header`](super::sv_module::emit_module_header);
/// при отсутствии адресованных портов не печатает ничего (модуль вырождается в
/// обычный `sv`).
pub(crate) fn emit_reg_iface_lines(p: &mut Printer, mmio: &Mmio) {
    if mmio.is_empty() {
        return;
    }
    let aw = mmio.addr_width;
    let dw = mmio.data_width;
    p.ident(&format!(
        "input  logic [{}:0] reg_addr,   // регистровый интерфейс цели sv-mmio (фича 0062): адрес",
        aw - 1
    ))
    .nl();
    p.ident(&format!(
        "input  logic [{}:0] reg_wdata,  // данные записи (бит out игнорирует запись)",
        dw - 1
    ))
    .nl();
    p.ident("input  logic reg_wen,        // строб записи").nl();
    p.ident(&format!(
        "output logic [{}:0] reg_rdata,  // данные чтения (комбинационные)",
        dw - 1
    ))
    .nl();
}

/// Печатает регистровый файл: объявление входных регистров, их защёлкивание
/// шиной и комбинационное чтение.
///
/// Выходные адресованные порты — уже регистры автомата (объявлены и защёлкнуты
/// `sv_fsm`), поэтому здесь только читаются мультиплексором `reg_rdata`.
pub(crate) fn emit_register_file(p: &mut Printer, mmio: &Mmio) {
    if mmio.is_empty() {
        return;
    }
    let aw = mmio.addr_width;
    let inputs: Vec<&MmioPort> = mmio
        .ports
        .iter()
        .filter(|p| matches!(p.direction, PortDirection::In))
        .collect();

    // Объявление входных регистров: их пишет шина, а не автомат, поэтому у них
    // нет комбинационной пары `_next` (автомат читает их как значение регистра,
    // ровно как раньше читал входной порт модуля).
    if !inputs.is_empty() {
        p.ident("// Входные регистры sv-mmio: их значение приходит от шины (reg_wen),")
            .nl();
        p.ident("// а не от автомата, поэтому пары _next у них нет.")
            .nl();
        for port in &inputs {
            let ty = sv_type(&port.ty, "").unwrap_or(SvType {
                prefix: "logic".to_string(),
                suffix: String::new(),
            });
            p.ident(&format!("{};", ty.declare(&port.name))).nl();
        }
        p.nl();

        // Защёлкивание входов шиной. Отдельный always_ff: каждый входной регистр
        // имеет ровно один драйвер (эту шину); регистры автомата — свой always_ff.
        p.ident("// Запись входных регистров шиной. Сброс — в 0; при reg_wen адрес")
            .nl();
        p.ident("// декодируется в регистр (запись в бит out сюда не попадает — R5).")
            .nl();
        p.ident("always_ff @(posedge clk) begin").nl();
        p.up();
        p.ident("if (!rst_n) begin").nl();
        p.up();
        for port in &inputs {
            p.ident(&format!("{} <= '0;", port.name)).nl();
        }
        p.down();
        p.ident("end else if (reg_wen) begin").nl();
        p.up();
        p.ident("unique case (reg_addr)").nl();
        p.up();
        // Группировка по адресу: у слова со смешанными направлениями пишутся
        // только in-биты (out игнорирует запись, R5).
        let mut in_groups: BTreeMap<i64, Vec<&MmioPort>> = BTreeMap::new();
        for port in &inputs {
            in_groups.entry(port.addr).or_default().push(port);
        }
        for (addr, group) in &in_groups {
            p.ident(&format!("{}: begin", addr_literal(*addr, aw))).nl();
            p.up();
            for port in group {
                p.ident(&format!(
                    "{} <= reg_wdata[{} +: {}];",
                    port.name, port.bit, port.width
                ))
                .nl();
            }
            p.down();
            p.ident("end").nl();
        }
        p.ident("default: ; // адрес без in-порта — запись игнорируется")
            .nl();
        p.down();
        p.ident("endcase").nl();
        p.down();
        p.ident("end").nl();
        p.down();
        p.ident("end").nl().nl();
    }

    // Чтение регистров шиной (комбинационное): собирает слово из всех портов по
    // адресу — и out (регистр автомата), и in (чтение возвращает записанное, R5).
    p.ident("// Чтение регистров шиной (комбинационное). Слово собирается из всех")
        .nl();
    p.ident("// портов адреса; чтение бита in возвращает записанное шиной (R5).")
        .nl();
    p.ident("always_comb begin").nl();
    p.up();
    p.ident("reg_rdata = '0;").nl();
    p.ident("unique case (reg_addr)").nl();
    p.up();
    for (addr, group) in &mmio.by_address() {
        p.ident(&format!("{}: begin", addr_literal(*addr, aw))).nl();
        p.up();
        for port in group {
            p.ident(&format!(
                "reg_rdata[{} +: {}] = {};",
                port.bit, port.width, port.name
            ))
            .nl();
        }
        p.down();
        p.ident("end").nl();
    }
    p.ident("default: reg_rdata = '0;").nl();
    p.down();
    p.ident("endcase").nl();
    p.down();
    p.ident("end").nl().nl();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_bits_counts_significant_bits() {
        assert_eq!(address_bits(0), 1);
        assert_eq!(address_bits(1), 1);
        assert_eq!(address_bits(0x601), 11); // stacker: 1537 → 11 бит
        assert_eq!(address_bits(0x10000000), 29); // elevator
    }

    #[test]
    fn addr_literal_is_sized_hex() {
        assert_eq!(addr_literal(0x100, 11), "11'h100");
    }

    #[test]
    fn bit_width_of_scalars() {
        let enums = BTreeMap::new();
        assert_eq!(bit_width(&TypeNode::Bit, &enums, ""), Some(1));
        assert_eq!(
            bit_width(
                &TypeNode::Integer {
                    bits: 8,
                    signed: false
                },
                &enums,
                ""
            ),
            Some(8)
        );
        assert_eq!(
            bit_width(&TypeNode::Fixed { m: 8, n: 8 }, &enums, ""),
            Some(16)
        );
        // Массив/структура/float — ширина не определена.
        assert_eq!(bit_width(&TypeNode::Rational, &enums, ""), None);
    }
}
