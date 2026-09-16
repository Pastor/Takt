//! Пределы хранилища проектов.
//!
//! # Откуда числа
//!
//! Не "на глаз", а замером корпуса:
//! крупнейший файл проекта - 21 506 Б, и предел файла втрое больше. Он же
//! совпадает с пределом черновика в браузере и с начальным буфером моста -
//! **один предел на всё**: второй разошёлся бы с первым молча.
//!
//! # Единица - Байты UTF-8
//!
//! Не символы: в `.takt` есть кириллица, и "64 тысячи символов" дали бы вдвое больший
//! файл. Считается то, что ляжет в базу.
//!
//! Превышение - **отказ с названным числом и фактом**, а не усечение. Усечённый
//! исходник выглядит целым и перестаёт компилироваться в месте, которого автор не
//! писал.

use std::collections::BTreeMap;

use crate::error::ApiError;

/// Наибольший размер одного файла.
pub const FILE_BYTES: usize = 64 * 1024;

/// Наибольшее число файлов в проекте.
pub const FILES_PER_PROJECT: i64 = 32;

/// Наибольший суммарный размер проекта.
pub const PROJECT_BYTES: i64 = 512 * 1024;

/// Наибольшее число проектов у одного владельца.
pub const PROJECTS_PER_USER: i64 = 100;

/// Наибольшая длина имени проекта и имени файла, символов.
pub const NAME_CHARS: usize = 64;

/// Наибольшая длина описания, символов.
pub const DESCRIPTION_CHARS: usize = 512;

/// Наибольшая длина строки ключей сборки, символов.
///
/// Предел нужен раньше разбора: строка уходит в модуль, а разбор чужого ввода без
/// границы - это работа, объём которой задаёт отправитель. Число взято с запасом от
/// самой длинной осмысленной строки ключей (замер m - 11 ключей вместе короче 200
/// символов).
pub const BUILD_ARGS_CHARS: usize = 512;

/// Строит отказ предела: и число, и факт.
///
/// Оба обязательны. "Слишком большой файл" не говорит, насколько ужиматься, а "предел
/// 65 536" не говорит, было ли превышение на байт или вдесятеро.
pub fn exceeded(
    what: &str,
    limit: impl std::fmt::Display,
    fact: impl std::fmt::Display,
) -> ApiError {
    ApiError::LimitExceeded {
        message: format!("{what}: предел {limit}, получено {fact}"),
    }
}

/// Судит объём владельца после записи.
///
/// Отказ - только когда запись **растит** объём и итог выше квоты. Владелец сверх
/// квоты (квоту уменьшили после того, как он её занял) вправе сокращать файлы и
/// удалять проекты: запрет любой записи оставил бы его без способа освободить место.
///
/// # Ошибки
/// Рост объёма выше квоты - отказ предела с квотой и итогом.
pub fn check_quota(before: i64, after: i64, quota: i64) -> Result<(), ApiError> {
    if after > before && after > quota {
        return Err(exceeded("объём данных владельца в байтах", quota, after));
    }
    Ok(())
}

/// Проверяет размер файла.
pub fn check_file(text: &str) -> Result<(), ApiError> {
    let size = text.len();
    if size > FILE_BYTES {
        return Err(exceeded("размер файла в байтах", FILE_BYTES, size));
    }
    Ok(())
}

/// Проверяет имя проекта.
pub fn check_project_name(name: &str) -> Result<(), ApiError> {
    let length = name.chars().count();
    if name.trim().is_empty() {
        return Err(ApiError::BadRequest("имя проекта: пустое".to_string()));
    }
    if length > NAME_CHARS {
        return Err(exceeded(
            "длина имени проекта в символах",
            NAME_CHARS,
            length,
        ));
    }
    Ok(())
}

/// Проверяет описание.
pub fn check_description(text: &str) -> Result<(), ApiError> {
    let length = text.chars().count();
    if length > DESCRIPTION_CHARS {
        return Err(exceeded(
            "длина описания в символах",
            DESCRIPTION_CHARS,
            length,
        ));
    }
    Ok(())
}

/// Проверяет длину строки ключей сборки.
pub fn check_build_args(text: &str) -> Result<(), ApiError> {
    let length = text.chars().count();
    if length > BUILD_ARGS_CHARS {
        return Err(exceeded(
            "длина строки ключей сборки в символах",
            BUILD_ARGS_CHARS,
            length,
        ));
    }
    Ok(())
}

/// Наибольшая задержка между тактами прогона, секунд.
///
/// Минута - уже не темп показа, а остановка: дольше автор ждать не станет, а
/// опечатка в тысячу секунд выглядела бы зависшим прогоном.
pub const RUN_DELAY_SECONDS: f64 = 60.0;

/// Наибольшее число сценариев с задержкой у проекта - по числу файлов.
pub const RUN_DELAYS: usize = FILES_PER_PROJECT as usize;

/// Проверяет задержки прогона и отдаёт их в хранимом виде.
///
/// Задержка - число секунд от нуля до [`RUN_DELAY_SECONDS`], дробное; хранится с
/// точностью до миллисекунды. Ноль означает "без задержки" и не хранится: запись о
/// нуле ничего не несёт, а список рос бы от каждого сценария, который открывали.
/// Что ключ - сценарий проекта, судит вызывающий: состав знает база.
pub fn check_run_delays(delays: &BTreeMap<String, f64>) -> Result<BTreeMap<String, f64>, ApiError> {
    let mut kept = BTreeMap::new();
    for (name, &seconds) in delays {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err(ApiError::BadRequest(format!(
                "задержка прогона у '{name}': число секунд от 0 до {RUN_DELAY_SECONDS}"
            )));
        }
        if seconds > RUN_DELAY_SECONDS {
            return Err(exceeded(
                &format!("задержка прогона у '{name}' в секундах"),
                RUN_DELAY_SECONDS,
                seconds,
            ));
        }
        let rounded = (seconds * 1000.0).round() / 1000.0;
        if rounded > 0.0 {
            kept.insert(name.clone(), rounded);
        }
    }
    if kept.len() > RUN_DELAYS {
        return Err(exceeded(
            "число сценариев с задержкой",
            RUN_DELAYS,
            kept.len(),
        ));
    }
    Ok(kept)
}

/// Наибольшая частота модельных часов прогона, Гц: такт не короче наносекунды.
pub const RUN_FREQUENCY_HZ: f64 = 1_000_000_000.0;

/// Проверяет частоты модельных часов прогона и отдаёт их в хранимом виде.
///
/// Частота - целое число герц от нуля до [`RUN_FREQUENCY_HZ`], как у объявления
/// `clock` модели: дробной частоты оно не знает. Приходит числом JSON, то есть
/// дробным, - иначе дробь и минус отвергал бы разбор запроса, не называя поля. Ноль
/// означает "частота из модели" и не хранится - по той же причине, что нулевая
/// задержка. Что ключ - сценарий проекта, судит вызывающий: состав знает база.
pub fn check_run_frequencies(
    frequencies: &BTreeMap<String, f64>,
) -> Result<BTreeMap<String, u64>, ApiError> {
    let mut kept = BTreeMap::new();
    for (name, &hz) in frequencies {
        if !hz.is_finite() || hz < 0.0 || hz.fract() != 0.0 {
            return Err(ApiError::BadRequest(format!(
                "частота прогона у '{name}': целое число герц от 0 до {RUN_FREQUENCY_HZ}"
            )));
        }
        if hz > RUN_FREQUENCY_HZ {
            return Err(exceeded(
                &format!("частота прогона у '{name}' в герцах"),
                RUN_FREQUENCY_HZ,
                hz,
            ));
        }
        if hz > 0.0 {
            kept.insert(name.clone(), hz as u64);
        }
    }
    if kept.len() > RUN_DELAYS {
        return Err(exceeded(
            "число сценариев с частотой",
            RUN_DELAYS,
            kept.len(),
        ));
    }
    Ok(kept)
}

/// Наибольшее число наблюдаемых выходов у одной модели.
pub const WATCH_PORTS: usize = 256;

/// Наибольшая длина имени порта в наборе наблюдения, символов: квалифицированное имя -
/// две части `Модель::порт`.
pub const WATCH_NAME_CHARS: usize = NAME_CHARS * 2 + 2;

/// Проверяет набор наблюдаемых выходов и отдаёт его в хранимом виде.
///
/// Ключ - файл модели, значение - имена портов. Повтор имени снимается с сохранением
/// порядка, пустой список не хранится - по той же причине, что нулевая задержка. Что
/// ключ - модель проекта, судит вызывающий: состав знает база. Порты модели сервер
/// не знает - их знает модуль, - и лишнее имя страница отбрасывает при показе.
pub fn check_run_watch(
    watch: &BTreeMap<String, Vec<String>>,
) -> Result<BTreeMap<String, Vec<String>>, ApiError> {
    if watch.len() > FILES_PER_PROJECT as usize {
        return Err(exceeded(
            "число моделей с набором наблюдения",
            FILES_PER_PROJECT,
            watch.len(),
        ));
    }
    let mut kept = BTreeMap::new();
    for (model, names) in watch {
        let mut unique: Vec<String> = Vec::new();
        for name in names {
            let length = name.chars().count();
            let form = name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':');
            if length == 0 || !form {
                return Err(ApiError::BadRequest(format!(
                    "набор наблюдения '{model}': имя порта '{name}' - латиница, цифры, '_' и '::'"
                )));
            }
            if length > WATCH_NAME_CHARS {
                return Err(exceeded(
                    &format!("длина имени порта в наборе '{model}' в символах"),
                    WATCH_NAME_CHARS,
                    length,
                ));
            }
            if !unique.contains(name) {
                unique.push(name.clone());
            }
        }
        if unique.len() > WATCH_PORTS {
            return Err(exceeded(
                &format!("число наблюдаемых выходов модели '{model}'"),
                WATCH_PORTS,
                unique.len(),
            ));
        }
        if !unique.is_empty() {
            kept.insert(model.clone(), unique);
        }
    }
    Ok(kept)
}

/// Род файла проекта - тип крейта проекта: правило одно у сервера и командной
/// строки.
pub use takt_project::Kind;

/// Проверяет имя файла и определяет его род.
///
/// Правило живёт в крейте проекта (`takt_project::check_file_name`): имя файла
/// становится именем корневой модели, и алфавит у него узкий. Здесь - только
/// перевод отказа в ответ сервиса.
pub fn check_file_name(name: &str) -> Result<Kind, ApiError> {
    takt_project::check_file_name(name).map_err(ApiError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_limit_is_counted_in_bytes_not_characters() {
        // "64 тысячи символов" дали бы вдвое больший файл: в `.takt` есть кириллица, и
        // считать надо то, что ляжет в базу.
        let cyrillic = "я".repeat(FILE_BYTES / 2);
        assert_eq!(cyrillic.chars().count(), FILE_BYTES / 2);
        assert_eq!(cyrillic.len(), FILE_BYTES);
        assert!(check_file(&cyrillic).is_ok(), "ровно предел — можно");
        assert!(
            check_file(&format!("{cyrillic}я")).is_err(),
            "на два байта больше"
        );
    }

    #[test]
    fn refusal_names_both_the_limit_and_the_fact() {
        // Без числа предела автор не знает, насколько ужиматься; без факта - было ли
        // превышение на байт или вдесятеро.
        let error = check_file(&"x".repeat(FILE_BYTES + 5)).expect_err("предел");
        let text = error.to_string();
        assert!(text.contains(&FILE_BYTES.to_string()), "{text}");
        assert!(text.contains(&(FILE_BYTES + 5).to_string()), "{text}");
    }

    #[test]
    fn a_bad_file_name_is_a_bad_request_and_a_long_one_is_a_limit() {
        // Правило имени проверяется в крейте проекта; здесь - что его отказы
        // доезжают до ответа своим кодом.
        assert_eq!(
            check_file_name("heater.takt-ui").expect("годно"),
            Kind::Layout
        );
        let (status, _) = check_file_name("модель.takt")
            .expect_err("кириллица")
            .status_and_code();
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        let long = format!("{}.takt", "x".repeat(NAME_CHARS));
        let (status, _) = check_file_name(&long)
            .expect_err("длинное")
            .status_and_code();
        assert_eq!(status, axum::http::StatusCode::PAYLOAD_TOO_LARGE);
    }

    /// Расширения проекта в кавычках - признак второго списка родов.
    const EXTENSIONS: [&str; 5] = [
        "\".takt\"",
        "\".takt-ui\"",
        "\".takt-map\"",
        "\".json\"",
        "\".md\"",
    ];

    /// Строки кода файла (без тестов и комментариев), где стоит расширение проекта.
    fn extension_lines(source: &str) -> Vec<String> {
        let code = source.split("#[cfg(test)]").next().unwrap_or_default();
        code.lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .filter(|line| EXTENSIONS.iter().any(|ext| line.contains(ext)))
            .map(|line| line.trim().to_string())
            .collect()
    }

    #[test]
    fn the_server_has_no_second_list_of_extensions() {
        // Род файла по расширению знает крейт проекта; список у сервера разошёлся бы
        // с ним молча - так уже было с родом, которого не знала страница.
        assert_eq!(
            extension_lines("fn kind(n: &str) { n.strip_suffix(\".takt-map\") }"),
            ["fn kind(n: &str) { n.strip_suffix(\".takt-map\") }"],
            "контроль ловит второй список"
        );
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut found = Vec::new();
        let mut read = 0;
        for entry in std::fs::read_dir(&dir).expect("каталог исходников") {
            let path = entry.expect("запись").path();
            if path.extension().is_some_and(|e| e == "rs") {
                read += 1;
                let text = std::fs::read_to_string(&path).expect("исходник");
                for line in extension_lines(&text) {
                    found.push(format!("{}: {line}", path.display()));
                }
            }
        }
        assert!(read >= 10, "выборка пуста: прочитано {read} файлов");
        assert!(
            found.is_empty(),
            "список расширений у сервера:\n{}",
            found.join("\n")
        );
    }

    #[test]
    fn watch_is_deduplicated_and_empty_lists_are_dropped() {
        let watch = BTreeMap::from([
            (
                "pump.takt".to_string(),
                vec!["speed".into(), "Pump::on".into(), "speed".into()],
            ),
            ("idle.takt".to_string(), Vec::new()),
        ]);
        let kept = check_run_watch(&watch).expect("годно");
        assert_eq!(
            kept,
            BTreeMap::from([(
                "pump.takt".to_string(),
                vec!["speed".to_string(), "Pump::on".to_string()]
            )])
        );
        let bad = BTreeMap::from([("pump.takt".to_string(), vec!["sp eed".to_string()])]);
        assert!(check_run_watch(&bad).is_err(), "пробел в имени");
        let long = BTreeMap::from([(
            "pump.takt".to_string(),
            vec!["x".repeat(WATCH_NAME_CHARS + 1)],
        )]);
        let (status, _) = check_run_watch(&long)
            .expect_err("длинное")
            .status_and_code();
        assert_eq!(status, axum::http::StatusCode::PAYLOAD_TOO_LARGE);
        let many = BTreeMap::from([(
            "pump.takt".to_string(),
            (0..=WATCH_PORTS).map(|i| format!("p{i}")).collect(),
        )]);
        assert!(check_run_watch(&many).is_err(), "сверх предела портов");
    }

    #[test]
    fn quota_refuses_only_growth_beyond_it() {
        assert!(check_quota(0, 100, 100).is_ok(), "ровно квота");
        let text = check_quota(0, 101, 100).expect_err("сверх").to_string();
        assert!(text.contains("100") && text.contains("101"), "{text}");
        assert!(
            check_quota(150, 120, 100).is_ok(),
            "сверх квоты, но сокращение"
        );
        assert!(
            check_quota(150, 150, 100).is_ok(),
            "сверх квоты, объём прежний"
        );
        assert!(check_quota(150, 151, 100).is_err(), "сверх квоты и рост");
    }

    #[test]
    fn project_name_and_description_are_measured_in_characters() {
        // Имя показывается человеку, а не хранится в порождённом коде: считать его
        // байтами значило бы дать кириллическому имени вдвое меньше места.
        assert!(check_project_name(&"я".repeat(NAME_CHARS)).is_ok());
        assert!(check_project_name(&"я".repeat(NAME_CHARS + 1)).is_err());
        assert!(check_project_name("   ").is_err(), "пустое имя");
        assert!(check_description(&"я".repeat(DESCRIPTION_CHARS)).is_ok());
        assert!(check_description(&"я".repeat(DESCRIPTION_CHARS + 1)).is_err());
    }

    #[test]
    fn limits_agree_with_each_other() {
        // Предел проекта обязан вмещать хотя бы несколько файлов предела: иначе один
        // законный файл делает проект невозможным.
        assert!(PROJECT_BYTES >= FILE_BYTES as i64 * 4, "проект тесен файлу");
        assert!(
            FILES_PER_PROJECT * FILE_BYTES as i64 > PROJECT_BYTES,
            "предел числа файлов недостижим — он ничего не ограничивает"
        );
    }
}
