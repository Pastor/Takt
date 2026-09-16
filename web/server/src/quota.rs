//! Квота владельца: суммарный объём исходников всех его проектов.
//!
//! # Что считается
//!
//! Сумма `projects.size_bytes` по владельцу, свёрнутые проекты включительно: база ведёт
//! состав, и число совпадает с тем, что видит автор в списке проектов. Занятое на диске
//! не считается - оно зависело бы от того, свёрнут ли проект.
//!
//! # Чья квота
//!
//! Владельца проекта, а не того, кто пишет: данные лежат у владельца, и правка
//! редактора с выданным правом расходует его место.
//!
//! # Замок
//!
//! Сумма охватывает все проекты владельца, а транзакция записи запирает строку одного
//! проекта. Две вкладки, пишущие в два проекта разом, видели бы одну и ту же сумму и
//! прошли бы обе. Поэтому сверка идёт под замком по владельцу
//! (`pg_advisory_xact_lock`): он снимается фиксацией транзакции и чужих чтений строки
//! пользователя не держит. Замок берётся **после** замка строки проекта - в том же
//! порядке во всех путях записи.

use crate::error::ApiError;
use crate::limits;

/// Запирает сверку квоты владельца до конца транзакции.
///
/// # Ошибки
/// Отказ базы.
pub async fn lock(
    transaction: &tokio_postgres::Transaction<'_>,
    owner: &str,
) -> Result<(), ApiError> {
    transaction
        .execute(
            "SELECT pg_advisory_xact_lock(hashtextextended('quota:' || $1, 0))",
            &[&owner],
        )
        .await?;
    Ok(())
}

/// Занятый объём владельца, байты.
///
/// Приведение обязательно: `sum()` над `bigint` в PostgreSQL даёт `numeric`.
///
/// # Ошибки
/// Отказ базы.
pub async fn used(
    client: &impl tokio_postgres::GenericClient,
    owner: &str,
) -> Result<i64, ApiError> {
    Ok(client
        .query_one(
            "SELECT coalesce(sum(size_bytes), 0)::bigint FROM projects WHERE owner_id = $1",
            &[&owner],
        )
        .await?
        .get(0))
}

/// Запирает сверку, считает занятое и судит прирост.
///
/// `grow` - на сколько байт запись меняет объём владельца; отрицательный прирост
/// законен всегда.
///
/// # Ошибки
/// Запись выводит владельца за квоту либо отказ базы.
pub async fn check(
    transaction: &tokio_postgres::Transaction<'_>,
    owner: &str,
    grow: i64,
    quota: i64,
) -> Result<(), ApiError> {
    lock(transaction, owner).await?;
    let before = used(transaction, owner).await?;
    limits::check_quota(before, before + grow, quota)
}
