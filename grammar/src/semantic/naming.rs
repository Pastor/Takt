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
    for ch in name.chars() {
        if ch.is_alphabetic() && ch.is_uppercase() {
            if !result.is_empty() {
                result.push('_');
            }
            result.push(ch.to_ascii_lowercase());
        } else {
            result.push(ch);
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
}
