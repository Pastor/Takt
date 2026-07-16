# Задача 0044-01: Грамматика и АСД конструкции `invariant`

> Фича: [../features/0044-sim-assert-invariant.md](../features/0044-sim-assert-invariant.md) · ADR: [../adr/0044-sim-assert-invariant.md](../adr/0044-sim-assert-invariant.md) · анализ: [../analyze/0044-sim-assert-invariant.md](../analyze/0044-sim-assert-invariant.md)

## Что было

*Реальное состояние кода на 2026-07-15 (ветка `v2`), проверено grep/чтением.*

- **Слова `invariant` в языке нет.** `KEYWORDS` (`grammar/src/parser/lexer.rs:460–503`)
  содержит 41 слово, включая `cond`, `address`, `formula`, `LTL`, `Guard`;
  `invariant` и `assert` — отсутствуют. `grep -rniw "invariant\|assert"` по
  **158** файлам `.lam` корпуса — **ноль** совпадений (база обоснования правила 11).
- **Ближайший родственник — `ConditionDefine`:** `cond Имя = Условие;`
  (`grammar/src/grammar.lalrpop`, правило `ConditionDefine`; АСД —
  `ast::ConditionDefine`, `parser/ast.rs:917+`; элемент модели —
  `ModelElement::Condition(Box<ConditionDefine>)`, `ast.rs:260`).
- **Позиции элементов уже перегружены:** `ModelElement` (`grammar.lalrpop:39–54`)
  — 15 альтернатив, включая `NamedBlockCodeDefine`, начинающийся с
  **произвольного идентификатора** (`grammar.lalrpop:266–274`); `StateElement`
  (`grammar.lalrpop:74–81`) — 5 альтернатив.
- **Прецедент жёсткого слова:** `"address" => Token::Address` (`lexer.rs:484`),
  правило `AddressDefine`, узел `ModelElement::Address` — фича 0020.
- **Прецедент мягкого слова (запасной путь):** `X`, `F`, `G`, `U`, `R`, `LTL`,
  `Guard` — в `KEYWORDS` (`lexer.rs:497–503`), но принимаются как идентификаторы
  правилом `Identifier` (`grammar.lalrpop:92–99`).

## Что сделано

> **Выполнено** (2026-07-16).

По ADR 0044 (Option C) — **одно** новое жёсткое ключевое слово; `assert` **не**
вводится.

1. **Лексер** (`parser/lexer.rs`): вариант `Token::Invariant` в перечислении
   `Token`; запись `"invariant" => Token::Invariant` в `KEYWORDS`.
2. **АСД** (`parser/ast.rs`): структура `InvariantDefine { loc, name: Option<Identifier>, name_loc, value: Condition }`
   — по образцу `ConditionDefine`. Правая часть — **`Condition`**, не
   `Expression` (R2, инвариант ADR 0019: `=` в условии есть равенство).
   Варианты `ModelElement::Invariant(Box<InvariantDefine>)` и
   `StateElement::Invariant(Box<InvariantDefine>)`.
3. **Грамматика** (`grammar.lalrpop`): правило
   `InvariantDefine: Box<InvariantDefine> = { <l:@L> "invariant" <name:IdentifierOrError> "=" <c:Condition> ";" <r:@R> => … }`;
   подключение в `ModelElement` и `StateElement`. Связка — `"="`
   (`Token::Assign`), **не** `":="` (`Token::ColonAssign`) — инвариант фичи 0021.
4. **В блоке кода `invariant` не подключается** (R4) — там уже есть `: c;`.

**Статус по функциональности (правило 11):**

- `grammar` — основная работа.
- `simulation` — н/п (АСД-слой не потребляется симулятором напрямую).
- Обратная совместимость — **слом намеренный и обоснованный** (анализ, ось 1):
  `var invariant: u8;` перестаёт разбираться; цена измерена = 0 на корпусе.

## Проверки

> **Планируется (разработка не начата).** Ожидаемые значения — гипотезы,
> подлежащие подтверждению зондом (`CLAUDE.md`: сперва зонд, затем assertions).

- **T1, T2** — `invariant` разбирается как элемент модели и состояния;
  фикстуры `grammar/tests/data/semantic/valid/invariant_model.lam`,
  `invariant_state.lam`.
- **T3** — контрпример: `invariant` в блоке → ошибка разбора.
- **T4** — контрпример: `invariant P := c;` → ошибка разбора.
- **T5** — `var assert: u8 := 1;` по-прежнему валиден; `grep -w '"assert"'
  grammar/src/parser/lexer.rs` → ноль.
- **T7** — контрпример: `var invariant: u8;` → ошибка разбора (ожидаемый слом).
- **Риск LR(1):** сборка `lalrpop` обязана пройти **без** новых конфликтов.
  Жёсткое слово снимает риск by construction (в отличие от мягкого — см.
  «Что было»); проверяется первой же сборкой.

```sh
cargo build --bin lamc
cargo test -- --test-threads=1
```

Соответствие анализу: **R1, R2, R3, R4** → критерии **A1, A2, A3**.
