//! Генератор синтезируемого SystemVerilog (IEEE 1800) из семантического дерева
//! Lam (фича 0045).
//!
//! Шестой целевой язык и **первый аппаратный**: C, ST и Rust — программные цели
//! (такт = итерация цикла сканирования), здесь такт Lam ≡ **фронт тактового
//! сигнала** `posedge clk`. Архитектурное решение — ADR 0045.
//!
//! ## Почему автоматная модель ложится на RTL лучше, чем на любую другую цель
//!
//! FSM — каноническая форма RTL, и часть дефектов программных целей здесь не
//! воспроизводится **конструктивно**:
//!
//! | Дефект цели `c` | Здесь |
//! |---|---|
//! | [0029](../../../../docs/features/0029-c-type-mapping.md): `bit` → `int` (32 бита на один провод) | `bit` → `logic` — один триггер, **идеальное** соответствие |
//! | [0033](../../../../docs/adr/0033-init-tick-alignment.md): синтетическое `INIT`-состояние стоит такта, и его требуется *убирать* правкой | `INIT` не существует: стартовое состояние — ветвь `if (!rst_n)`, **сдвиг = 0 на любой глубине даром** |
//!
//! Плата за аппаратную цель — три запрета (`SV-003` `float`, `SV-005`
//! `extern fn`, `SV-006` `inout`): в синтезируемом RTL плавающей точки, вызова
//! внешнего кода и двунаправленного провода без сигнала `oe` не существует.
//! Молчаливого пропуска нет ни в одном из случаев (R4).
//!
//! ## Форма вывода
//!
//! Один `.sv`-файл, **один `module` на корневую модель**: композиция `M1 | M2`
//! уплощается в общий `always_comb` (ADR, Option A′). Иерархия модулей SV дерево
//! моделей не повторяет — это плата за точную семантику `|`, где модель, идущая
//! позже, обязана видеть записи предыдущей **в том же такте**. Модуль-на-модель
//! дал бы комбинационную петлю (проверено: `verilator` → `UNOPTFLAT`).
//!
//! ## Состав модуля
//!
//! `sv_map` (снимок карты) · `sv_type` (типы, `SV-002`…`SV-004`) · `sv_module`
//! (модуль и порты, `SV-006`/`SV-007`) · `sv_fsm` (автомат и сброс, `SV-008`) ·
//! `sv_expr` (выражения и функции, `SV-005`).

mod sv_map;
mod sv_module;
mod sv_type;

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::GenerateOptions;
use crate::generator::Generator as AsGenerator;
use crate::generator::indent::Printer;
use crate::semantic::ModelNode;
use crate::semantic::minimap::{Element, Name};
use crate::semantic::naming::normalize_lowercase_snakecase;
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use sv_map::SvMap;

/// Размер одного уровня отступа в порождаемом SystemVerilog.
const INDENT: usize = 4;

/// Генератор SystemVerilog для модели Lam.
pub struct Generator {}

impl AsGenerator for Generator {
    fn generate(
        &self,
        model: &ModelNode,
        output_path: &str,
        options: &GenerateOptions,
    ) -> Result<(), Diagnostic> {
        let map = SvMap::new(
            &normalize_lowercase_snakecase(model.name().to_string()),
            model,
            options.guard_enable,
        )?;
        let program = generate_program(&map)?;
        let filename = map.get_filename();
        let _ = fs::create_dir(Path::new(output_path));
        fs::write(
            Path::new(output_path).join(filename.to_owned() + ".sv"),
            program,
        )
        .map_err(|e| {
            Diagnostic::error(Location::Codegen, format!("{:?}", e)).with_code("SV-001")
        })?;
        Ok(())
    }
}

/// Собирает текст модуля SystemVerilog из снимка модели.
///
/// Каркас (задача 0045-01): порождает минимальный **синтезируемый** модуль —
/// объявление, служебные порты `clk`/`rst_n`, регистр состояния, `always_comb`
/// вместе с `always_ff` и выход `is_done`. Это не «пустая заглушка», а
/// вырожденный случай целевой формы: он проходит оба гейта (0045-02), которые
/// тем самым ставятся **сразу**, а не после накопления кода. Содержательное
/// наполнение — задачи 0045-03…0045-06.
fn generate_program(map: &SvMap) -> Result<String, Diagnostic> {
    let Element::Model { .. } = map.model() else {
        return Err(Diagnostic::error(
            Location::Codegen,
            "Корневой элемент карты не является моделью".to_string(),
        )
        .with_code("SV-010"));
    };
    let root_name = map.root_name();
    let root = map.root_model_node().ok_or_else(|| {
        Diagnostic::error(
            Location::Codegen,
            format!("Корневая модель '{}' отсутствует в снимке карты", root_name),
        )
        .with_code("SV-010")
    })?;

    // Модуль ОДИН на корневую модель (ADR, Option A′): композиция уплощается,
    // поэтому порты собираются со всех уровней — в `elevator_mini.lam` они
    // объявлены внутри под-моделей. Порядок под-моделей задан `BTreeMap` карты
    // (фича 0048) — детерминизм достаётся даром.
    let mut blocks: Vec<(Name, Rc<RefCell<ModelNode>>)> = Vec::new();
    let mut submodels: Vec<Name> = map
        .using_models()
        .into_iter()
        .filter_map(|element| match element {
            Element::Model { name, .. } => Some(name),
            _ => None,
        })
        .collect();
    submodels.sort_by(|a, b| a.unique().cmp(b.unique()));
    for name in submodels {
        let model = map.raw_model_at(name.clone())?;
        blocks.push((name, model));
    }
    blocks.push((root_name.clone(), root));

    // ⚠️ Порты СОБИРАЮТСЯ, но пока не печатаются, и это не забывчивость.
    // Объявленный порт, которого никто не читает, даёт `UNUSEDSIGNAL` от
    // `verilator -Wall` — то есть красный гейт. Читатель порта живёт в теле
    // автомата (условия рёбер, блоки `always`), а тело — предмет задач 0045-05
    // и 0045-06. Значит, задача 0045-04 в одиночку зелёной быть не может **в
    // принципе**: порты и их использование обязаны появиться в выводе одним
    // шагом. Сбор здесь уже боевой (диагностики `SV-006`/`SV-007`/`SV-012`
    // работают), печать включается вместе с автоматом.
    let _ports = sv_module::collect_ports(map, &blocks)?;
    let module = normalize_lowercase_snakecase(root_name.unique().replace(':', "_"));

    let mut out = String::new();
    let mut p = Printer::new(INDENT, &mut out);

    p.ident("// Порождено компилятором Lam (lamc) — цель: SystemVerilog (IEEE 1800).")
        .nl();
    p.ident("// Не редактировать вручную: файл перезаписывается при каждой генерации.")
        .nl();
    p.ident("//").nl();
    p.ident("// Такт модели Lam ≡ фронт clk (posedge). Сброс синхронный, активный низкий:")
        .nl();
    p.ident("// ветвь if (!rst_n) несёт стартовое состояние — синтетического INIT нет,")
        .nl();
    p.ident("// поэтому тело стартового состояния исполняется на такте 1 (контракт 0033).")
        .nl();
    p.nl();

    sv_module::emit_module_header(&mut p, &module, &sv_module::SvPorts::default());

    p.up();
    p.ident("typedef enum logic [0:0] {").nl();
    p.up();
    p.ident("ST_START = 1'd0,").nl();
    p.ident("ST_END   = 1'd1").nl();
    p.down();
    p.ident("} state_e;").nl().nl();
    p.ident("state_e state, state_next;").nl().nl();

    p.ident("always_comb begin").nl();
    p.up();
    // Умолчание обязательно: неполное присваивание даёт защёлку, а
    // `verilator -Wall` — LATCH. Это не стиль, а условие гейта.
    p.ident("state_next = state;").nl();
    p.down();
    p.ident("end").nl().nl();

    p.ident("always_ff @(posedge clk) begin").nl();
    p.up();
    p.ident("if (!rst_n) begin").nl();
    p.up();
    p.ident("state <= ST_START;").nl();
    p.down();
    p.ident("end else begin").nl();
    p.up();
    p.ident("state <= state_next;").nl();
    p.down();
    p.ident("end").nl();
    p.down();
    p.ident("end").nl().nl();

    p.ident("assign is_done = (state == ST_END);").nl();
    p.down();
    p.ident("endmodule").nl();

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::tree::construct_model;

    /// Строит снимок карты из исходника Lam (по образцу `rust::tests::make_map`).
    fn make_map(src: &str, name: &str) -> SvMap {
        let (ast, _) = crate::parse(src, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some(name.to_string());
        let model = model_rc.borrow();
        SvMap::new(name, &model, true).unwrap()
    }

    fn program_of(src: &str, name: &str) -> String {
        generate_program(&make_map(src, name)).unwrap()
    }

    /// Корневая модель порождает `module` с именем в `snake_case`.
    #[test]
    fn root_model_emits_module() {
        let sv = program_of("start S;", "Root");
        assert!(sv.contains("module root ("), "нет module корня:\n{sv}");
        assert!(sv.contains("endmodule"), "нет endmodule:\n{sv}");
    }

    /// Служебные порты `clk`/`rst_n` эмитятся всегда: в `.lam` их нет.
    #[test]
    fn service_ports_are_emitted() {
        let sv = program_of("start S;", "Root");
        assert!(sv.contains("input  logic clk"), "нет порта clk:\n{sv}");
        assert!(sv.contains("input  logic rst_n"), "нет порта rst_n:\n{sv}");
    }

    /// Автомат — two-process: `always_comb` (тело) + `always_ff` (латч и сброс).
    #[test]
    fn fsm_is_two_process() {
        let sv = program_of("start S;", "Root");
        assert!(sv.contains("always_comb begin"), "нет always_comb:\n{sv}");
        assert!(
            sv.contains("always_ff @(posedge clk) begin"),
            "нет always_ff по фронту clk:\n{sv}"
        );
    }

    /// **Контракт ADR 0033:** стартовое состояние стоит в ветви сброса, а
    /// синтетического `INIT` нет вовсе.
    ///
    /// У целей `c`/`rust` вход в стартовое состояние требуется *убирать*
    /// правкой; здесь его просто нет — регистр состояния физически не может
    /// «побыть в INIT».
    #[test]
    fn reset_branch_carries_start_state_without_init() {
        let sv = program_of("start S;", "Root");
        assert!(
            sv.contains("if (!rst_n) begin"),
            "нет ветви синхронного сброса:\n{sv}"
        );
        // Сравнение идёт по СТРОКАМ кода, а не подстрокой: шапка модуля сама
        // объясняет, почему INIT-состояния нет, и упоминает его текстом.
        // Подстрочная проверка ловила бы этот комментарий — что она и сделала
        // при первом прогоне.
        let offender = sv
            .lines()
            .map(str::trim)
            .find(|line| !line.starts_with("//") && line.contains("INIT"));
        assert!(
            offender.is_none(),
            "синтетического INIT в цели sv быть не должно (контракт 0033), найдено: {:?}",
            offender
        );
    }

    /// Сброс **синхронный**: в списке чувствительности только `posedge clk`.
    #[test]
    fn reset_is_synchronous() {
        let sv = program_of("start S;", "Root");
        assert!(
            !sv.contains("negedge rst_n"),
            "сброс обязан быть синхронным (ADR 0045, Option A):\n{sv}"
        );
    }

    /// В `always_comb` — только блокирующие присваивания.
    ///
    /// Смешение даёт другую семантику: `v := 1; w := v;` в Lam даёт `w = 1`,
    /// а с неблокирующими `w` получил бы **старое** `v`.
    #[test]
    fn comb_block_has_default_assignment() {
        let sv = program_of("start S;", "Root");
        assert!(
            sv.contains("state_next = state;"),
            "нет умолчания в always_comb — неполное присваивание даёт защёлку (LATCH):\n{sv}"
        );
    }

    /// Вывод **воспроизводим**: одна модель — один и тот же текст (фича 0048).
    #[test]
    fn output_is_deterministic() {
        let src = "model A { start S; } model B { start T; } start E = A | B;";
        let first = program_of(src, "Root");
        for i in 1..8 {
            assert_eq!(
                first,
                program_of(src, "Root"),
                "прогон {i} дал другой вывод — вернулся недетерминизм порядка"
            );
        }
    }
}
