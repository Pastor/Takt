//! Срок хранения проекта и его свёртка (фича 0531, задача 09h).
//!
//! # Правило
//!
//! Корректировка заказчика 2026-09-04: у проектов есть **срок хранения**. Не
//! обращались дольше срока — проект сворачивается в архив; обращение на чтение
//! **или** запись сбрасывает счётчик. Свёрнутый проект разворачивается **первым
//! же обращением** (решение заказчика): для автора архивация невидима, кроме
//! того, что первое открытие медленнее.
//!
//! # Почему отметка обновляется не на каждое чтение
//!
//! Счётчик меряется днями, а чтений у открытой вкладки — десятки в минуту.
//! Отметка обновляется, только если она старше [`GRANULARITY`]: иначе каждое
//! чтение становилось бы записью в базу, а точность от этого не выросла бы ни
//! на секунду.
//!
//! # Подметание
//!
//! Обход стоит **и командой, и по времени**: команда нужна тому, кто ставит её
//! в `cron`, а обход по времени — тому, у кого `cron` нет (своя машина, проба
//! стенда). Оба зовут одну функцию: второй проход разошёлся бы с первым.

use std::sync::Arc;

use tokio_postgres::GenericClient;

use crate::db;
use crate::store::Store;

/// Насколько грубо ведётся отметка обращения, секунд.
pub const GRANULARITY: i64 = 3600;

/// Отмечает обращение и разворачивает свёрнутый проект.
///
/// Зовётся из **обоих** путей — чтения и записи: правило заказчика говорит про
/// оба, и пропуск одного означал бы, что активно правимый проект однажды
/// свернётся под руками автора.
///
/// # Ошибки
/// Отказ базы либо диска.
pub async fn touch<C: GenericClient>(
    client: &C,
    store: &Arc<Store>,
    id: &str,
    owner: &str,
    archived_at: Option<i64>,
    touched_at: i64,
) -> anyhow::Result<()> {
    let now = db::now();
    if archived_at.is_some() {
        // ⚠️ Сначала диск, потом база: обратный порядок при обрыве оставляет
        // проект «развёрнутым» по записи и свёрнутым на диске — то есть без
        // исходников.
        store.unpack(owner, id)?;
        client
            .execute(
                "UPDATE projects SET archived_at = NULL, touched_at = $2 WHERE id = $1",
                &[&id, &now],
            )
            .await?;
        return Ok(());
    }
    if now - touched_at >= GRANULARITY {
        client
            .execute(
                "UPDATE projects SET touched_at = $2 WHERE id = $1",
                &[&id, &now],
            )
            .await?;
    }
    Ok(())
}

/// Сворачивает проекты, к которым не обращались дольше срока.
///
/// Возвращает, сколько свёрнуто.
///
/// # Ошибки
/// Отказ базы. ⚠️ Отказ ДИСКА на одном проекте обход не останавливает: иначе
/// один испорченный проект оставлял бы несвёрнутыми все следующие, и заметить
/// это было бы нечем.
pub async fn sweep(
    client: &deadpool_postgres::Client,
    store: &Arc<Store>,
    retention_secs: i64,
) -> anyhow::Result<usize> {
    let border = db::now() - retention_secs;
    let rows = client
        .query(
            "SELECT id, owner_id FROM projects
             WHERE archived_at IS NULL AND touched_at < $1
             ORDER BY touched_at",
            &[&border],
        )
        .await?;
    let mut packed = 0;
    for row in &rows {
        let id: String = row.get(0);
        let owner: String = row.get(1);
        let names: Vec<String> = client
            .query(
                "SELECT name FROM project_files WHERE project_id = $1 ORDER BY name",
                &[&id],
            )
            .await?
            .iter()
            .map(|row| row.get(0))
            .collect();
        if let Err(error) = store.pack(&owner, &id, &names) {
            tracing::warn!(%error, project = %id, "проект не сворачивается");
            continue;
        }
        client
            .execute(
                "UPDATE projects SET archived_at = $2 WHERE id = $1",
                &[&id, &db::now()],
            )
            .await?;
        packed += 1;
    }
    Ok(packed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_granularity_is_coarser_than_a_click_and_finer_than_a_day() {
        // ⚠️ Смысл числа: отметка обновляется реже, чем читают, и чаще, чем
        // считается срок. Сделай её нулевой — каждое чтение станет записью;
        // сделай её сутками — проект, открытый вчера вечером, свернётся сегодня.
        assert!(
            GRANULARITY >= 60,
            "чаще минуты — это запись на каждое чтение"
        );
        assert!(
            GRANULARITY <= 24 * 3600,
            "реже суток — счётчик врёт на день"
        );
    }
}
