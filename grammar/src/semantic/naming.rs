/// Нормализует имя файла или идентификатора в CamelCase.
///
/// Преобразует `my_model`, `mein-leib`, `Mein_Leib` → `MyModel`, `MeinLeib`.
/// Небуквенно-цифровые символы (`_`, `-`, `#` и т.д.) используются как разделители слов.
pub fn normalize_model_name(name: &str) -> String {
    let mut result = String::new();
    let mut upper = true;
    for ch in name.chars() {
        if ch.is_alphabetic() && upper {
            result.push(ch.to_ascii_uppercase());
        } else if !ch.is_alphanumeric() {
            upper = true;
            continue;
        } else {
            result.push(ch);
        }
        upper = false;
    }
    result
}

pub fn normalize_lowercase_snakecase(name: String) -> String {
    let mut result = String::new();
    let mut prev_was_lower = false;
    for ch in name.chars() {
        if ch.is_alphabetic() && ch.is_uppercase() {
            if prev_was_lower {
                result.push('_');
            }
            result.push(ch.to_ascii_lowercase());
            prev_was_lower = false;
        } else {
            result.push(ch);
            prev_was_lower = ch.is_alphabetic() && ch.is_lowercase();
        }
    }
    result
}

#[cfg(test)]
mod tests {
    const NAMES: &[(&str, &str)] = &[
        ("mein_leib", "MeinLeib"),
        ("mein-leib", "MeinLeib"),
        ("Mein_Leib", "MeinLeib"),
        ("mein_Leib", "MeinLeib"),
        ("Mein#Leib", "MeinLeib"),
    ];

    #[test]
    fn normalize_model_name() {
        use super::normalize_model_name;
        for (name, expected) in NAMES {
            let normalized = normalize_model_name(name);
            assert_eq!(&normalized, expected);
        }
    }

    // ── Дополнительные тесты нормализации имён ────────────────────────────────

    /// Пустая строка остаётся пустой.
    #[test]
    fn normalize_model_name_empty() {
        use super::normalize_model_name;
        assert_eq!(normalize_model_name(""), "");
    }

    /// Строка из одних цифр не изменяется.
    #[test]
    fn normalize_model_name_digits_only() {
        use super::normalize_model_name;
        assert_eq!(normalize_model_name("123"), "123");
    }

    /// Одно слово: первая буква становится заглавной.
    #[test]
    fn normalize_model_name_single_word() {
        use super::normalize_model_name;
        assert_eq!(normalize_model_name("hello"), "Hello");
    }

    // ── Тесты normalize_lowercase_snakecase ───────────────────────────────────

    /// CamelCase → snake_case: граница нижний→верхний регистр.
    #[test]
    fn snakecase_camel_case() {
        use super::normalize_lowercase_snakecase;
        assert_eq!(normalize_lowercase_snakecase("MyModel".to_string()), "my_model");
        assert_eq!(normalize_lowercase_snakecase("ThisIsMyModel".to_string()), "this_is_my_model");
        assert_eq!(normalize_lowercase_snakecase("isReady".to_string()), "is_ready");
    }

    /// ALL_CAPS-имя (уже разделено `_`) не разбивается на символы.
    #[test]
    fn snakecase_all_caps_with_underscore() {
        use super::normalize_lowercase_snakecase;
        assert_eq!(normalize_lowercase_snakecase("IS_EMPTY".to_string()), "is_empty");
        assert_eq!(normalize_lowercase_snakecase("AT_FLOOR".to_string()), "at_floor");
    }

    /// Сплошные заглавные без разделителя не разбиваются посимвольно.
    #[test]
    fn snakecase_solid_all_caps() {
        use super::normalize_lowercase_snakecase;
        assert_eq!(normalize_lowercase_snakecase("MATRIX".to_string()), "matrix");
        assert_eq!(normalize_lowercase_snakecase("NUMB".to_string()), "numb");
    }

    /// Цифры не вызывают вставку `_`.
    #[test]
    fn snakecase_with_digits() {
        use super::normalize_lowercase_snakecase;
        assert_eq!(normalize_lowercase_snakecase("B1".to_string()), "b1");
        assert_eq!(normalize_lowercase_snakecase("sensors1".to_string()), "sensors1");
    }

    /// Пустая строка остаётся пустой.
    #[test]
    fn snakecase_empty() {
        use super::normalize_lowercase_snakecase;
        assert_eq!(normalize_lowercase_snakecase(String::new()), "");
    }
}
