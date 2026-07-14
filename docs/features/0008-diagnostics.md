# Фича 0008: Семантические диагностики и предупреждения

- **Номер:** 0008
- **Статус:** ГОТОВО
- **Зависит от:** 0001
- **Крейт:** `grammar`

## Краткое описание

Набор диагностик анализа: неиспользуемые переменные, недетерминированные
переходы, недостижимые состояния, константные условия, «висячие» точки с запятой,
неизвестные именованные блоки, неявный bool, документирующие комментарии.

## Итог (что сделано)

- Ce13 — неиспользуемые переменные: `semantic/unused.rs`, API
  `unused_variable_warnings()` (порты/константы не предупреждаются).
- Ce14 — недетерминированные переходы: `check_nondeterministic_transitions`
  (`semantic/validate.rs`), API `nondeterministic_transition_warnings()`.
- SE-044 `StraySemicolon`, SE-045 неизвестный именованный блок, SE-046
  недостижимые состояния, SE-047 константные условия переходов (`validate.rs`, `lib.rs`).
- SE-048 висячая привязка адреса (`address` для несуществующего порта),
  SE-049 конфликт источников адреса (inline + `address`, либо дубликат `address`)
  — оператор `address` фичи 0020 (`check_port_addresses` в `validate.rs`).
- Неявный bool и документирующие комментарии (Ce12): `semantic/docs.rs`,
  `check_implicit_bool_conditions`.
- Фикстуры `unused_variable.lam`, `nondeterministic_warn.lam`, `implicit_bool_*.lam`,
  `doc_comments.lam`; контрпример `double_next.lam`.

> Ретроспективная карточка (правило 17). Источники: `STATUS.md` (задачи 14, 16, 17, 18),
> память проекта (FE3, FE4), `CHANGES.md`.
