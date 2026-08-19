//! Структуры: пустое объявление, дубли полей, типы полей.
//!
//! Часть модуля `validate` (фича 0027: деление по логике).

use super::*;

/// Структура без полей — `SE-115` (фича 0284).
///
/// # Почему это правило языка, а не забота целей
///
/// Замер 2026-08-18 на `struct Empty { }`:
///
/// | Потребитель | Что происходило |
/// |---|---|
/// | эталон | исполнял, переменная получала `Empty{}` |
/// | `c` | `typedef struct Empty { } Empty;` — **расширение GNU**: C11 6.7.2.1 требует непустой список объявлений, `cc -pedantic` предупреждает `-Wgnu-empty-struct` |
/// | `st`, `st-at` | `Empty : STRUCT END_STRUCT;` — **`iec2c` отвергает**: «no structure element declared in structure type declaration» |
///
/// Контрольный замер отделяет причину от следствия: **непустая** структура
/// проходит и `cc -std=c11 -pedantic`, и `iec2c` — ломается именно пустота.
///
/// ⚠️ **Гейт проекта этого не видел по устройству**: он гоняет `cc` без
/// `-pedantic` (там пустая структура законна как расширение), а в корпусе
/// `examples/` пустых структур нет ни одной.
///
/// ⚠️ **Симметрично `SE-105`** (перечисление без вариантов, фича 0172): то же
/// «агрегатное объявление без элементов», тот же отказ **на объявлении**.
/// Причины разные — у перечисления выбирать нечего, у структуры вывод не
/// принимают чужие инструменты, — поэтому коды разные, а место одно: правило
/// языка, а не квантификатор грамматики. `Comma<StructField>` («ноль или
/// более») намеренно оставлен: смена квантификатора изменила бы язык молча, а
/// текст отказа стал бы внутренностью LR-разбора (урок 0172).
///
/// ⚠️ **Цели `rust` и `sv` в этот замер не входят:** они не переводят
/// структуры **вовсе** (`RS-014` на любой, висячая ссылка на необъявленный тип
/// у `sv`) — соседний класс, вынесенный кандидатом.
pub fn validate_empty_structs(model: Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let borrowed = model.borrow();
    // Накопление по объявлениям (правило 0151): две пустые структуры дают две
    // диагностики, иначе автор чинит по одной за прогон.
    borrowed
        .structs
        .values()
        .filter(|st| st.fields.is_empty())
        .map(|st| {
            Diagnostic::declaration_error(
                st.loc,
                format!(
                    "структура '{}' объявлена без полей: у структуры обязано быть \
                     хотя бы одно поле (добавьте поле либо удалите объявление). \
                     Пустая запись невалидна по стандарту C (6.7.2.1) и отвергается \
                     компилятором ST",
                    st.name
                ),
            )
            .with_code("SE-115")
        })
        .collect()
}

/// Ce17: проверяет, что в каждой структуре модели нет дублирующихся имён полей.
///
/// ## Правило Ce17
///
/// Каждое поле структурного типа должно иметь уникальное имя внутри одного
/// объявления `struct`. Повторное объявление поля с тем же именем — ошибка.
///
/// ## Примеры (Takt)
///
/// ```text
/// // Корректно
/// struct Point { x: bit, y: bit }
///
/// // Ce17: поле x объявлено дважды
/// struct Bad { x: bit, x: bit }
/// ```
///
/// # Возвращаемое значение
///
/// [`Diagnostic`] уровня `Error` с кодом Ce17 при первом нарушении,
/// `None` если дублирований нет.
pub fn check_duplicate_struct_fields(model: Rc<RefCell<ModelNode>>) -> Option<Diagnostic> {
    let structs: Vec<_> = model.borrow().structs.values().cloned().collect();

    for s in &structs {
        let mut seen: HashSet<&str> = HashSet::new();
        for (field_name, _) in &s.fields {
            if !seen.insert(field_name.as_str()) {
                return Some(
                    Diagnostic::error(
                        s.loc,
                        format!(
                            "структура '{}' содержит дублирующееся поле '{}'",
                            s.name, field_name
                        ),
                    )
                    .with_code("SE-040"),
                );
            }
        }
    }

    // Рекурсивная проверка вложенных моделей
    let nested: Vec<Rc<RefCell<ModelNode>>> =
        model.borrow().models.values().map(Rc::clone).collect();
    for nested_model in nested {
        if let Some(diag) = check_duplicate_struct_fields(nested_model) {
            return Some(diag);
        }
    }

    None
}

/// Ce18: проверяет, что типы полей всех структур разрешены и известны.
///
/// ## Правило Ce18
///
/// Каждое поле структуры должно иметь тип, который либо является
/// встроенным (`bit`, `bool`, массив), либо объявлен в области видимости
/// (псевдоним, перечисление или другая структура). Ссылка на неизвестный тип —
/// ошибка.
///
/// ## Примеры (Takt)
///
/// ```text
/// // Корректно
/// struct Vec2 { x: [bit;16], y: [bit;16] }
///
/// // Ce18: Ghost не объявлен
/// struct Bad { val: Ghost }
/// ```
///
/// # Возвращаемое значение
///
/// [`Diagnostic`] уровня `Error` с кодом Ce18 при первом нарушении,
/// `None` если все типы полей известны.
pub fn check_struct_field_types(model: Rc<RefCell<ModelNode>>) -> Option<Diagnostic> {
    let structs: Vec<_> = model.borrow().structs.values().cloned().collect();

    for s in &structs {
        for (field_name, field_ty) in &s.fields {
            if let TypeNode::Struct(type_name) = field_ty {
                // Проверяем, что структурный тип поля существует в области видимости
                if model.borrow().search_struct(type_name).is_none() {
                    return Some(
                        Diagnostic::error(
                            s.loc,
                            format!(
                                "поле '{}' структуры '{}' ссылается на неизвестный тип '{}'",
                                field_name, s.name, type_name
                            ),
                        )
                        .with_code("SE-041"),
                    );
                }
            }
        }
    }

    // Рекурсивная проверка вложенных моделей
    let nested: Vec<Rc<RefCell<ModelNode>>> =
        model.borrow().models.values().map(Rc::clone).collect();
    for nested_model in nested {
        if let Some(diag) = check_struct_field_types(nested_model) {
            return Some(diag);
        }
    }

    None
}
