//! Архив проекта: выгрузка и загрузка (фича 0531, задача 09g).
//!
//! # Круговой рейс, а не выгрузка
//!
//! Корректировка заказчика 2026-09-04: архив несёт **файл метаданных**, и
//! главная цель — **загрузка проекта из архива**. Значит, архив — не снимок для
//! человека, а форма обмена, и требования к нему другие:
//!
//! - **метаданные обязательны**: без них загрузка не восстановит ни имени, ни
//!   активного файла, ни **версии модуля** (решение A5) — проект открылся бы
//!   чужим компилятором, и вывод целей молча поехал бы;
//! - **версия формата обязательна**: читатель, встретивший незнакомую,
//!   отказывает словами — половина восстановленного проекта хуже отказа;
//! - **исходники и вывод разделены** (`src/` и `generated/`): вывод
//!   воспроизводим (0048) и в проекте не хранится, поэтому загрузка его
//!   **игнорирует**. Свали их рядом — и в проекте появились бы файлы, которых
//!   компилятор не писал.
//!
//! ⚠️ Видимость, права, число копий и владелец в архив **не идут**: это
//! свойства места, а не проекта. Восстановив их у себя, автор получил бы чужие
//! права на своей стороне.
//!
//! # Пределы
//!
//! Те же, что у ручек (файл 64 КиБ, файлов 32, проект 512 КиБ, проектов 100), и
//! проверяет их **сервер**: архив приходит извне, и доверять ему нельзя. Имя
//! файла судится тем же правилом, что при записи (0195).

use std::io::{Cursor, Read as _, Write as _};

use serde::{Deserialize, Serialize};

use crate::db;
use crate::error::ApiError;
use crate::limits;

/// Имя файла метаданных внутри архива.
pub const MANIFEST: &str = "takt-project.json";

/// Каталог исходников внутри архива.
pub const SOURCES: &str = "src/";

/// Каталог порождённого вывода внутри архива.
pub const GENERATED: &str = "generated/";

/// Версия формата архива.
///
/// ⚠️ Растёт вместе с формой записи. Читатель, встретивший бо́льшую версию,
/// **отказывает**: разобрать наполовину значит отдать автору проект, про
/// который он думает, что тот целый.
pub const FORMAT: u32 = 1;

/// Метаданные проекта в архиве.
#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    /// Версия формата архива.
    pub format: u32,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Версия модуля, которой открывается проект (решение A5).
    pub takt_lang: String,
    #[serde(default)]
    pub language_version: String,
    /// Активный файл; `null` — не назначен.
    #[serde(default)]
    pub main_file: Option<String>,
    /// Состав исходников: имя и вид.
    #[serde(default)]
    pub files: Vec<ManifestFile>,
    /// Когда выгружен, Unix-секунды.
    #[serde(default)]
    pub exported_at: i64,
    /// Какой целью собран `generated/`; `null` — вывода в архиве нет.
    #[serde(default)]
    pub generated_target: Option<String>,
}

/// Запись состава.
#[derive(Debug, Serialize, Deserialize)]
pub struct ManifestFile {
    pub name: String,
    pub kind: String,
}

/// Один файл проекта для укладки в архив.
#[derive(Debug)]
pub struct SourceFile {
    pub name: String,
    pub kind: String,
    pub text: String,
}

/// Что положить в архив.
pub struct Export {
    pub manifest: Manifest,
    pub sources: Vec<SourceFile>,
    /// Вывод цели: пары «имя, текст». Пусто — выгрузка без генерации.
    pub generated: Vec<(String, String)>,
    /// Отказ цели с причиной; `None` — цель не звали либо она не отказала.
    ///
    /// ⚠️ Отказ цели — **нормальный ответ**, а не ошибка сервиса (вопрос
    /// задачи закрыт так): он записывается в архив словами, потому что молча
    /// пропущенный вывод неотличим от «цель ничего не печатает».
    pub refusal: Option<String>,
}

/// Складывает архив.
///
/// # Ошибки
/// Отказ записи в память (практически недостижим) либо неразбираемые
/// метаданные.
pub fn pack(export: &Export) -> anyhow::Result<Vec<u8>> {
    let mut buffer = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buffer);
        let options: zip::write::FileOptions<'_, ()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file(MANIFEST, options)?;
        zip.write_all(serde_json::to_string_pretty(&export.manifest)?.as_bytes())?;
        for file in &export.sources {
            zip.start_file(format!("{SOURCES}{}", file.name), options)?;
            zip.write_all(file.text.as_bytes())?;
        }
        for (name, text) in &export.generated {
            zip.start_file(format!("{GENERATED}{name}"), options)?;
            zip.write_all(text.as_bytes())?;
        }
        if let Some(reason) = &export.refusal {
            zip.start_file(format!("{GENERATED}REFUSAL.txt"), options)?;
            zip.write_all(reason.as_bytes())?;
        }
        zip.finish()?;
    }
    Ok(buffer.into_inner())
}

/// Что прочитано из архива.
#[derive(Debug)]
pub struct Import {
    pub manifest: Manifest,
    pub sources: Vec<SourceFile>,
}

/// Разбирает архив и судит его пределами хранилища.
///
/// # Ошибки
/// Не архив, нет метаданных, чужая версия формата, нарушен предел, негодное имя
/// файла.
pub fn unpack(bytes: &[u8]) -> Result<Import, ApiError> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| ApiError::BadRequest(format!("это не архив: {error}")))?;

    let manifest: Manifest = {
        let mut entry = zip.by_name(MANIFEST).map_err(|_| {
            ApiError::BadRequest(format!(
                "в архиве нет '{MANIFEST}': без метаданных проект не восстановить"
            ))
        })?;
        let mut text = String::new();
        entry
            .read_to_string(&mut text)
            .map_err(|error| ApiError::BadRequest(format!("'{MANIFEST}' не читается: {error}")))?;
        serde_json::from_str(&text).map_err(|error| {
            ApiError::BadRequest(format!("'{MANIFEST}' не разбирается: {error}"))
        })?
    };
    if manifest.format > FORMAT {
        // ⚠️ Отказ, а не «прочитаем что сможем»: половина восстановленного
        // проекта хуже отказа — автор будет думать, что он целый.
        return Err(ApiError::BadRequest(format!(
            "архив версии формата {}, а сервис знает {FORMAT}",
            manifest.format
        )));
    }
    limits::check_project_name(&manifest.name)?;
    limits::check_description(&manifest.description)?;

    let mut sources = Vec::new();
    let mut total: i64 = 0;
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|error| ApiError::BadRequest(format!("архив повреждён: {error}")))?;
        let path = entry.name().to_string();
        // Вывод целей ИГНОРИРУЕТСЯ: он воспроизводим и в проекте не хранится.
        // Прими его загрузка за исходник — в проекте появились бы файлы,
        // которых компилятор не писал.
        let Some(name) = path.strip_prefix(SOURCES) else {
            continue;
        };
        if name.is_empty() || name.ends_with('/') {
            continue;
        }
        let kind = limits::check_file_name(name)?;
        let mut text = String::new();
        entry.read_to_string(&mut text).map_err(|error| {
            ApiError::BadRequest(format!("файл '{name}' не читается как текст: {error}"))
        })?;
        limits::check_file(&text)?;
        total += text.len() as i64;
        if sources.len() as i64 >= limits::FILES_PER_PROJECT {
            return Err(limits::exceeded(
                "число файлов в проекте",
                limits::FILES_PER_PROJECT,
                sources.len() as i64 + 1,
            ));
        }
        if total > limits::PROJECT_BYTES {
            return Err(limits::exceeded(
                "размер проекта в байтах",
                limits::PROJECT_BYTES,
                total,
            ));
        }
        sources.push(SourceFile {
            name: name.to_string(),
            kind: kind.as_str().to_string(),
            text,
        });
    }
    if sources.is_empty() {
        return Err(ApiError::BadRequest(format!(
            "в архиве нет исходников: их место — каталог '{SOURCES}'"
        )));
    }
    // Имена внутри архива могут повторяться — форма это допускает, а проект
    // нет: `PRIMARY KEY (project_id, name)` принял бы последний молча.
    let mut seen = std::collections::BTreeSet::new();
    for file in &sources {
        if !seen.insert(file.name.clone()) {
            return Err(ApiError::BadRequest(format!(
                "файл '{}' в архиве дважды",
                file.name
            )));
        }
    }
    Ok(Import { manifest, sources })
}

/// Собирает метаданные выгрузки.
pub fn manifest_of(
    name: &str,
    description: &str,
    takt_lang: &str,
    language_version: &str,
    main_file: Option<String>,
    files: &[SourceFile],
    generated_target: Option<String>,
) -> Manifest {
    Manifest {
        format: FORMAT,
        name: name.to_string(),
        description: description.to_string(),
        takt_lang: takt_lang.to_string(),
        language_version: language_version.to_string(),
        main_file,
        files: files
            .iter()
            .map(|file| ManifestFile {
                name: file.name.clone(),
                kind: file.kind.clone(),
            })
            .collect(),
        exported_at: db::now(),
        generated_target,
    }
}

/// Имя файла архива: по нему его узнают в каталоге загрузок.
///
/// ⚠️ Строится из ИМЕНИ проекта, но чистится: имя бывает кириллическим и с
/// пробелами, а заголовок `Content-Disposition` их не переносит.
pub fn file_name(project: &str) -> String {
    let mut out = String::new();
    for ch in project.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "takt-project.zip".to_string()
    } else {
        format!("{trimmed}.zip")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Export {
        Export {
            manifest: manifest_of(
                "Термореле",
                "проба",
                "0.58.0",
                "0.17.0",
                Some("model.takt".to_string()),
                &[SourceFile {
                    name: "model.takt".into(),
                    kind: "takt".into(),
                    text: "model A {}".into(),
                }],
                Some("c".to_string()),
            ),
            sources: vec![SourceFile {
                name: "model.takt".into(),
                kind: "takt".into(),
                text: "model A {}".into(),
            }],
            generated: vec![("playground.h".into(), "#ifndef X".into())],
            refusal: None,
        }
    }

    #[test]
    fn the_archive_makes_a_round_trip() {
        // ⚠️ Круговой рейс — и есть предмет задачи: «архив собрался» не
        // доказывает ничего, пока он не прочитан обратно.
        let bytes = pack(&sample()).expect("архив");
        let back = unpack(&bytes).expect("разбор");
        assert_eq!(back.manifest.name, "Термореле");
        assert_eq!(
            back.manifest.takt_lang, "0.58.0",
            "версия модуля пережила рейс"
        );
        assert_eq!(back.manifest.main_file.as_deref(), Some("model.takt"));
        assert_eq!(back.sources.len(), 1, "вывод цели исходником не считается");
        assert_eq!(back.sources[0].name, "model.takt");
        assert_eq!(back.sources[0].text, "model A {}");
    }

    #[test]
    fn an_archive_without_metadata_is_refused() {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buffer);
            let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
            zip.start_file("src/model.takt", options).expect("файл");
            zip.write_all(b"model A {}").expect("запись");
            zip.finish().expect("конец");
        }
        let error = unpack(&buffer.into_inner()).expect_err("должен отказать");
        let (status, code) = error.status_and_code();
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(code, "bad_request");
        assert!(error.to_string().contains(MANIFEST), "{error}");
    }

    #[test]
    fn a_future_format_is_refused_by_words() {
        let mut export = sample();
        export.manifest.format = FORMAT + 1;
        let bytes = pack(&export).expect("архив");
        let error = unpack(&bytes).expect_err("должен отказать");
        // Оба числа названы: автор обязан понять, что обновлять — архив или
        // сервис.
        assert!(
            error.to_string().contains(&(FORMAT + 1).to_string()),
            "{error}"
        );
        assert!(error.to_string().contains(&FORMAT.to_string()), "{error}");
    }

    #[test]
    fn the_limits_of_the_storage_apply_to_what_came_from_outside() {
        // ⚠️ Архив приходит извне, и доверять ему нельзя: пределы те же, что у
        // ручек, и проверяет их сервер.
        let mut export = sample();
        export.sources[0].text = "x".repeat(limits::FILE_BYTES + 1);
        let bytes = pack(&export).expect("архив");
        let error = unpack(&bytes).expect_err("должен отказать");
        let (status, _) = error.status_and_code();
        assert_eq!(status, axum::http::StatusCode::PAYLOAD_TOO_LARGE);

        // Имя файла судится как имя модели (0195).
        let mut export = sample();
        export.sources[0].name = "модель.takt".to_string();
        let bytes = pack(&export).expect("архив");
        assert!(unpack(&bytes).is_err(), "кириллица в имени файла");
    }

    #[test]
    fn the_same_name_twice_does_not_get_into_an_archive() {
        // Форма zip повторы допускает, а проект — нет: `PRIMARY KEY` принял бы
        // последний молча. ⚠️ Замер: повтор отвергают ОБА конца — и запись
        // (проверяется здесь), и чтение. Проверка в `unpack` оставлена
        // защитой в глубину и названа: архив приходит извне, и полагаться на
        // чужую библиотеку в вопросе целостности данных нельзя.
        let mut export = sample();
        export.sources.push(SourceFile {
            name: "model.takt".into(),
            kind: "takt".into(),
            text: "model B {}".into(),
        });
        let error = pack(&export).expect_err("повтор не должен записаться");
        assert!(error.to_string().contains("Duplicate"), "{error}");
    }

    #[test]
    fn the_file_name_survives_a_cyrillic_project_name() {
        // Заголовок `Content-Disposition` не переносит ни кириллицы, ни
        // пробелов, а имя проекта бывает и тем и другим.
        assert_eq!(file_name("counter"), "counter.zip");
        assert_eq!(file_name("Термореле 2"), "2.zip");
        assert_eq!(file_name("Термореле"), "takt-project.zip");
        assert_eq!(file_name(""), "takt-project.zip");
    }
}
