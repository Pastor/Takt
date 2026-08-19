//! Заглушка неиспользуемого параметра порождённых функций (фича 0260).
//!
//! Функции цели `c` следуют единому протоколу вызова: под-модель получает
//! указатель на корень (`main`), пользовательская функция — указатель на
//! состояние (`model`), помощник HAL — `m`. Протокол единообразен намеренно:
//! печать вызовов, предобъявлений и определений идёт из четырёх модулей, и
//! сигнатура, зависящая от тела, потребовала бы согласовать их все (разбор — ADR
//! 0260, Option A).
//!
//! Но тело **не всегда** пользуется параметром — и тогда `cc -Wall -Wextra`
//! говорит `-Wunused-parameter`. Замер 2026-08-19: корпус даёт **49** таких
//! предупреждений у цели `c` и **45** у `c-hal`; в гейтах класс был отключён
//! флагом `-Wno-unused-parameter`, то есть не сторожил ничего, а в сборке
//! пользователя — шумел.
//!
//! Решение: там, где тело параметром не пользуется, первой строкой печатается
//! `(void)параметр;` — идиома C, гасящая предупреждение и честно говорящая
//! читателю «протокол требует, тело не пользуется».
//!
//! ## Почему признак смотрит на ТЕКСТ тела
//!
//! Вопрос задаётся уже напечатанному телу, а не семантике. Это снимает
//! транзитивность: тело родителя содержит `Child_tick(&model->c, main)`, то есть
//! упоминает `main`, — значит параметр используется, и заглушка не нужна.
//! Обходить дерево снизу вверх и знать обо **всех** каналах (переменные корня,
//! порты, состояние соседа, источник времени, `userdata`) не требуется.
//!
//! ⚠️ **`assert` использованием не считается.** Часть функций сегодня спасает от
//! предупреждения строка `assert(0 != main);`, но под `-DNDEBUG` она исчезает —
//! а прошивки собирают именно так. Замер: корпус даёт **53** предупреждения под
//! `-DNDEBUG` против 49 без него. Признак, доверившийся `assert`, дал бы гейт,
//! зелёный в отладке и красный в релизе пользователя.
//!
//! ⚠️ **Границы идентификатора обязательны:** `domain` содержит `main`,
//! `model_state` содержит `model`. Поиск подстрокой заглушил бы заглушку там,
//! где она нужна.

/// Печатается ли `(void)<param>;` перед телом функции.
///
/// `body` — уже напечатанное тело (без сигнатуры). Возвращает `true`, если
/// параметр телом **не** используется.
pub(in crate::generator::c) fn is_unused(body: &str, param: &str) -> bool {
    !body
        .lines()
        // Строка `assert(…)` исчезает под `-DNDEBUG` — упоминание в ней
        // использованием не является (см. шапку модуля).
        .filter(|line| !line.trim_start().starts_with("assert("))
        .any(|line| mentions(line, param))
}

/// Строка-заглушка для параметра.
pub(in crate::generator::c) fn unused_guard(param: &str) -> String {
    format!("(void){param};")
}

/// Встречается ли `ident` в строке **как отдельный идентификатор**.
fn mentions(line: &str, ident: &str) -> bool {
    let bytes = line.as_bytes();
    let mut from = 0;
    while let Some(pos) = line[from..].find(ident) {
        let start = from + pos;
        let end = start + ident.len();
        let before_ok = start == 0 || !is_ident_byte(bytes[start - 1]);
        let after_ok = end == bytes.len() || !is_ident_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        from = end;
    }
    false
}

/// Байт, который может входить в идентификатор C.
fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_use_is_seen() {
        assert!(!is_unused("    model->state = 1;\n", "model"));
        assert!(!is_unused("    Child_tick(&model->c, main);\n", "main"));
    }

    #[test]
    fn absent_parameter_is_unused() {
        assert!(is_unused("    return model->state == END;\n", "main"));
        assert!(is_unused("", "model"));
    }

    /// ⚠️ Границы идентификатора: `domain` — не `main`, `model_state` — не `model`.
    #[test]
    fn substring_is_not_a_use() {
        assert!(is_unused("    uint8_t domain = 0;\n", "main"));
        assert!(is_unused("    x = MAIN_STATE;\n", "main"));
        assert!(is_unused("    int model_state = 0;\n", "model"));
    }

    /// ⚠️ `assert` исчезает под `-DNDEBUG` — использованием не считается.
    #[test]
    fn assert_is_not_a_use() {
        assert!(is_unused("    assert(0 != main);\n    return 1;\n", "main"));
        assert!(!is_unused(
            "    assert(0 != main);\n    main->x = 1;\n",
            "main"
        ));
    }

    #[test]
    fn guard_text() {
        assert_eq!(unused_guard("main"), "(void)main;");
    }
}
