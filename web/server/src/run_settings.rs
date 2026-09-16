//! Настройки прогона в метаданных проекта: задержка и частота по сценариям,
//! наблюдаемые выходы по моделям.
//!
//! Ключ всех трёх - имя файла: переименование и удаление файла правят их одним
//! запросом, и настройка не переживает свой файл и не теряется при его переименовании.
//! Колонки пишет только сервер; негодный текст при чтении даёт пустой список - отказ
//! чтения проекта из-за показа прогона стоил бы дороже потерянной настройки.

use std::collections::BTreeMap;

use crate::error::ApiError;
use crate::limits;
use crate::limits::Kind;
use crate::projects::{self, PatchRequest};

/// Применяет настройки прогона из правки проекта: каждый список - целиком.
///
/// # Ошибки
/// Нарушен предел либо ключ - не файл нужного вида.
pub(crate) async fn patch(
    transaction: &tokio_postgres::Transaction<'_>,
    id: &str,
    request: &PatchRequest,
) -> Result<(), ApiError> {
    if let Some(delays) = &request.run_delays {
        // Задержка - свойство сценария: ключ, который сценарием не является, дал бы
        // запись, которую страница не покажет никогда.
        let delays = limits::check_run_delays(delays)?;
        for name in delays.keys() {
            projects::has_kind(
                transaction,
                id,
                name,
                Kind::Scenario,
                "сценарий задержки прогона",
            )
            .await?;
        }
        let stored =
            serde_json::to_string(&delays).map_err(|error| ApiError::Internal(error.into()))?;
        transaction
            .execute(
                "UPDATE projects SET run_delays = $1 WHERE id = $2",
                &[&stored, &id],
            )
            .await?;
    }
    if let Some(frequencies) = &request.run_frequencies {
        // Частота - свойство сценария по тому же правилу, что задержка.
        let frequencies = limits::check_run_frequencies(frequencies)?;
        for name in frequencies.keys() {
            projects::has_kind(
                transaction,
                id,
                name,
                Kind::Scenario,
                "сценарий частоты прогона",
            )
            .await?;
        }
        let stored = serde_json::to_string(&frequencies)
            .map_err(|error| ApiError::Internal(error.into()))?;
        transaction
            .execute(
                "UPDATE projects SET run_frequencies = $1 WHERE id = $2",
                &[&stored, &id],
            )
            .await?;
    }
    if let Some(watch) = &request.run_watch {
        // Набор наблюдения - свойство модели: порты у модели, и ключ-сценарий дал бы
        // список, который страница не применит никогда.
        let watch = limits::check_run_watch(watch)?;
        for name in watch.keys() {
            projects::has_kind(
                transaction,
                id,
                name,
                Kind::Takt,
                "модель наблюдения прогона",
            )
            .await?;
        }
        let stored =
            serde_json::to_string(&watch).map_err(|error| ApiError::Internal(error.into()))?;
        transaction
            .execute(
                "UPDATE projects SET run_watch = $1 WHERE id = $2",
                &[&stored, &id],
            )
            .await?;
    }
    Ok(())
}

/// Задержки прогона из колонки: объект JSON "сценарий - секунды".
///
/// Негодный текст даёт пустой список: колонку пишет только сервер, и отказ чтения
/// проекта из-за показа прогона стоил бы дороже потерянного темпа.
pub(crate) fn delays_of(stored: String) -> BTreeMap<String, f64> {
    serde_json::from_str(&stored).unwrap_or_default()
}

/// Частоты прогона из колонки: объект JSON "сценарий - герцы"; правило чтения то же,
/// что у [`delays_of`].
pub(crate) fn frequencies_of(stored: String) -> BTreeMap<String, u64> {
    serde_json::from_str(&stored).unwrap_or_default()
}

/// Переносит настройки прогона на новое имя файла: `$1` - проект, `$2` -
/// прежнее имя, `$3` - новое. Ключ обеих - имя файла, и без переноса переименованный
/// сценарий потерял бы свой темп. Колонка без ключа не трогается.
pub(crate) const RENAME_RUN_SETTINGS: &str = "UPDATE projects
    SET run_delays = CASE WHEN run_delays::jsonb ? $2::text
            THEN ((run_delays::jsonb - $2::text)
                || jsonb_build_object($3::text, run_delays::jsonb -> $2::text))::text
            ELSE run_delays END,
        run_frequencies = CASE WHEN run_frequencies::jsonb ? $2::text
            THEN ((run_frequencies::jsonb - $2::text)
                || jsonb_build_object($3::text, run_frequencies::jsonb -> $2::text))::text
            ELSE run_frequencies END,
        run_watch = CASE WHEN run_watch::jsonb ? $2::text
            THEN ((run_watch::jsonb - $2::text)
                || jsonb_build_object($3::text, run_watch::jsonb -> $2::text))::text
            ELSE run_watch END
    WHERE id = $1 AND (run_delays::jsonb ? $2::text OR run_frequencies::jsonb ? $2::text
        OR run_watch::jsonb ? $2::text)";

/// Забывает настройки прогона удалённого файла: `$1` - проект, `$2` - имя.
pub(crate) const FORGET_RUN_SETTINGS: &str = "UPDATE projects
    SET run_delays = (run_delays::jsonb - $2::text)::text,
        run_frequencies = (run_frequencies::jsonb - $2::text)::text,
        run_watch = (run_watch::jsonb - $2::text)::text
    WHERE id = $1 AND (run_delays::jsonb ? $2::text OR run_frequencies::jsonb ? $2::text
        OR run_watch::jsonb ? $2::text)";

/// Наблюдаемые выходы из колонки: объект JSON "модель - имена портов"; правило
/// чтения то же, что у [`delays_of`].
pub(crate) fn watch_of(stored: String) -> BTreeMap<String, Vec<String>> {
    serde_json::from_str(&stored).unwrap_or_default()
}
