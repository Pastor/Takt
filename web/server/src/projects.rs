//! Проекты и их файлы (фича 0531, задача 09b).
//!
//! # Что такое проект
//!
//! Именованное хранилище **исходников** одного владельца: файлы `*.takt` и
//! сценарии `*.json`. Вывода целей, трасс, кадров и диагностик здесь нет и не
//! будет: всё это воспроизводится модулем версии проекта (генерация
//! детерминирована — 0048, раскладка сеяна — задача 01) и стоило бы на порядок
//! больше исходника (200 КиБ вывода против 17 КиБ модели).
//!
//! # Ревизия
//!
//! Счётчик обнаружения конфликта, **не журнал**: истории правок проект не
//! хранит. Запись файла шлёт ревизию, которую видела; разошлась — `409`, и
//! выбор («перечитать» либо «перезаписать») делает автор, а не сервер.
//!
//! # Размер
//!
//! `projects.size_bytes` — сумма размеров файлов, и она пересчитывается **в
//! той же транзакции**, что запись. Считать её отдельным запросом значило бы
//! иметь два ответа на один вопрос: параллельная запись развела бы их молча.
//!
//! ⚠️ Чужой проект отвечает **`404`**, а не `403`: иначе ручка становится
//! оракулом существования — по ответам перечисляются чужие проекты.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::auth::User;
use crate::db;
use crate::error::ApiError;
use crate::limits;
use crate::routes::{AppState, current_user};

/// Метаданные проекта.
#[derive(Debug, Serialize)]
pub struct ProjectJson {
    pub id: String,
    pub name: String,
    pub description: String,
    pub visibility: String,
    /// Версия модуля, которой открывается проект (решение A5).
    pub takt_lang: String,
    pub language_version: String,
    pub main_file: Option<String>,
    pub revision: i64,
    pub size_bytes: i64,
    pub forked_from: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Файл в списке: без текста — список бывает длинным.
#[derive(Debug, Serialize)]
pub struct FileEntryJson {
    pub name: String,
    pub kind: String,
    pub size_bytes: i64,
}

/// Проект вместе со списком файлов.
#[derive(Debug, Serialize)]
pub struct ProjectWithFilesJson {
    #[serde(flatten)]
    pub project: ProjectJson,
    pub files: Vec<FileEntryJson>,
}

/// Файл целиком.
#[derive(Debug, Serialize)]
pub struct FileJson {
    pub name: String,
    pub kind: String,
    pub text: String,
    pub size_bytes: i64,
    /// Ревизия ПРОЕКТА на момент чтения: её и шлют обратно при записи.
    pub revision: i64,
}

/// Запрос создания проекта.
#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Версия модуля; пусто — последняя, которую знает сервер.
    #[serde(default)]
    pub takt_lang: Option<String>,
}

/// Запрос правки метаданных. Отсутствующее поле не трогается.
#[derive(Debug, Deserialize)]
pub struct PatchRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub visibility: Option<String>,
    #[serde(default)]
    pub main_file: Option<String>,
    /// Подъём версии модуля — **явное действие владельца** (решение A5):
    /// после него вывод целей может измениться.
    #[serde(default)]
    pub takt_lang: Option<String>,
}

/// Запрос записи файла.
#[derive(Debug, Deserialize)]
pub struct PutFileRequest {
    pub text: String,
    /// Ревизия, которую видел автор. Пусто — только у нового файла.
    #[serde(default)]
    pub revision: Option<i64>,
}

/// Ответ записи: новая ревизия и новый размер проекта.
#[derive(Debug, Serialize)]
pub struct WriteResponse {
    pub revision: i64,
    pub size_bytes: i64,
}

/// Маршруты проектов.
///
/// ⚠️ Состояние НЕ подставляется здесь: его ставит внешний роутер один раз на
/// всё дерево (`routes::router`). Подставь его дважды — и вложение перестанет
/// собираться, потому что типы состояния разойдутся.
pub fn router() -> Router<Arc<AppState>> {
    // ⚠️ Пути полные, а роутер ПРИМЕШИВАЕТСЯ, а не вкладывается: вложение с
    // внутренним маршрутом `/` даёт адрес с хвостовой косой чертой, и
    // `/api/projects` отвечал бы `404`. Поймано тестом списка маршрутов.
    Router::new()
        .route("/projects", get(list).post(create))
        .route("/projects/{id}", get(read).patch(patch).delete(remove))
        .route(
            "/projects/{id}/files/{name}",
            get(read_file).put(write_file).delete(remove_file),
        )
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let user = current_user(&state, &headers).await?;
    let client = state.pool.get().await?;
    let rows = client
        .query(
            "SELECT id, name, description, visibility, takt_lang, language_version,
                    main_file, revision, size_bytes, forked_from, created_at, updated_at
             FROM projects WHERE owner_id = $1 ORDER BY updated_at DESC",
            &[&user.id],
        )
        .await?;
    let projects: Vec<ProjectJson> = rows.iter().map(project_of).collect();
    Ok(Json(projects).into_response())
}

async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateRequest>,
) -> Result<Response, ApiError> {
    let user = current_user(&state, &headers).await?;
    limits::check_project_name(&request.name)?;
    limits::check_description(&request.description)?;
    let mut client = state.pool.get().await?;
    // ⚠️ Счёт и вставка — в ОДНОЙ транзакции: между отдельными запросами
    // помещается вторая вкладка того же владельца, и предел обходится вдвоём
    // с самим собой.
    let transaction = client.transaction().await?;
    let count: i64 = transaction
        .query_one(
            "SELECT count(*) FROM projects WHERE owner_id = $1",
            &[&user.id],
        )
        .await?
        .get(0);
    if count >= limits::PROJECTS_PER_USER {
        return Err(limits::exceeded(
            "число проектов у владельца",
            limits::PROJECTS_PER_USER,
            count + 1,
        ));
    }
    let id = new_id();
    let now = db::now();
    let takt_lang = request
        .takt_lang
        .unwrap_or_else(|| state.module_version.clone());
    transaction
        .execute(
            "INSERT INTO projects(id, owner_id, name, description, visibility,
                                  takt_lang, language_version, revision, size_bytes,
                                  created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'private', $5, $6, 0, 0, $7, $7)",
            &[
                &id,
                &user.id,
                &request.name,
                &request.description,
                &takt_lang,
                &state.language_version,
                &now,
            ],
        )
        .await?;
    let row = transaction.query_one(SELECT_PROJECT, &[&id]).await?;
    let project = project_of(&row);
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(project)).into_response())
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let user = current_user(&state, &headers).await?;
    let client = state.pool.get().await?;
    let project = owned(&client, &id, &user).await?;
    let rows = client
        .query(
            "SELECT name, kind, size_bytes FROM project_files
             WHERE project_id = $1 ORDER BY name",
            &[&id],
        )
        .await?;
    let files = rows
        .iter()
        .map(|row| FileEntryJson {
            name: row.get(0),
            kind: row.get(1),
            size_bytes: row.get(2),
        })
        .collect();
    Ok(Json(ProjectWithFilesJson { project, files }).into_response())
}

async fn patch(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<PatchRequest>,
) -> Result<Response, ApiError> {
    let user = current_user(&state, &headers).await?;
    let mut client = state.pool.get().await?;
    let transaction = client.transaction().await?;
    let row = transaction
        .query_opt(
            "SELECT owner_id FROM projects WHERE id = $1 FOR UPDATE",
            &[&id],
        )
        .await?;
    let Some(row) = row else {
        return Err(ApiError::NotFound);
    };
    if row.get::<_, String>(0) != user.id {
        return Err(ApiError::NotFound);
    }
    if let Some(name) = &request.name {
        limits::check_project_name(name)?;
        transaction
            .execute("UPDATE projects SET name = $1 WHERE id = $2", &[name, &id])
            .await?;
    }
    if let Some(description) = &request.description {
        limits::check_description(description)?;
        transaction
            .execute(
                "UPDATE projects SET description = $1 WHERE id = $2",
                &[description, &id],
            )
            .await?;
    }
    if let Some(visibility) = &request.visibility {
        // ⚠️ Поле ставится здесь, а ДЕЙСТВУЕТ с задачи 09c: до неё чужой
        // проект недоступен при любом значении. Разрешить его сейчас честнее,
        // чем отвергать: колонка есть, и правило её значений — в схеме.
        if !["private", "link", "public"].contains(&visibility.as_str()) {
            return Err(ApiError::BadRequest(
                "видимость: 'private', 'link' либо 'public'".to_string(),
            ));
        }
        transaction
            .execute(
                "UPDATE projects SET visibility = $1 WHERE id = $2",
                &[visibility, &id],
            )
            .await?;
    }
    if let Some(main_file) = &request.main_file {
        // Активным можно назначить только тот файл, который есть: иначе
        // страница откроет проект, показывая пустоту.
        let exists: i64 = transaction
            .query_one(
                "SELECT count(*) FROM project_files WHERE project_id = $1 AND name = $2",
                &[&id, main_file],
            )
            .await?
            .get(0);
        if exists == 0 {
            return Err(ApiError::BadRequest(format!(
                "активный файл '{main_file}': в проекте такого нет"
            )));
        }
        transaction
            .execute(
                "UPDATE projects SET main_file = $1 WHERE id = $2",
                &[main_file, &id],
            )
            .await?;
    }
    if let Some(takt_lang) = &request.takt_lang {
        transaction
            .execute(
                "UPDATE projects SET takt_lang = $1 WHERE id = $2",
                &[takt_lang, &id],
            )
            .await?;
    }
    transaction
        .execute(
            "UPDATE projects SET updated_at = $1 WHERE id = $2",
            &[&db::now(), &id],
        )
        .await?;
    let row = transaction.query_one(SELECT_PROJECT, &[&id]).await?;
    let project = project_of(&row);
    transaction.commit().await?;
    Ok(Json(project).into_response())
}

async fn remove(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let user = current_user(&state, &headers).await?;
    let client = state.pool.get().await?;
    owned(&client, &id, &user).await?;
    // Файлы и права уходят каскадом (схема), копии — нет: у форка своя жизнь
    // (`forked_from` объявлен `ON DELETE SET NULL`).
    client
        .execute("DELETE FROM projects WHERE id = $1", &[&id])
        .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn read_file(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, name)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let user = current_user(&state, &headers).await?;
    let client = state.pool.get().await?;
    let project = owned(&client, &id, &user).await?;
    let row = client
        .query_opt(
            "SELECT name, kind, text, size_bytes FROM project_files
             WHERE project_id = $1 AND name = $2",
            &[&id, &name],
        )
        .await?;
    let Some(row) = row else {
        return Err(ApiError::NotFound);
    };
    Ok(Json(FileJson {
        name: row.get(0),
        kind: row.get(1),
        text: row.get(2),
        size_bytes: row.get(3),
        revision: project.revision,
    })
    .into_response())
}

async fn write_file(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, name)): Path<(String, String)>,
    Json(request): Json<PutFileRequest>,
) -> Result<Response, ApiError> {
    let user = current_user(&state, &headers).await?;
    let kind = limits::check_file_name(&name)?;
    limits::check_file(&request.text)?;

    let mut client = state.pool.get().await?;
    // Вся запись — ОДНА транзакция со строкой проекта под замком: ревизия,
    // предел числа файлов, предел размера и пересчёт суммы обязаны видеть одно
    // и то же состояние. Порознь их обходит вторая вкладка того же владельца.
    let transaction = client.transaction().await?;
    let row = transaction
        .query_opt(
            "SELECT owner_id, revision FROM projects WHERE id = $1 FOR UPDATE",
            &[&id],
        )
        .await?;
    let Some(row) = row else {
        return Err(ApiError::NotFound);
    };
    if row.get::<_, String>(0) != user.id {
        return Err(ApiError::NotFound);
    }
    let revision: i64 = row.get(1);
    let existing: Option<i64> = transaction
        .query_opt(
            "SELECT size_bytes FROM project_files WHERE project_id = $1 AND name = $2",
            &[&id, &name],
        )
        .await?
        .map(|row| row.get(0));

    match (request.revision, existing) {
        // Правка существующего файла обязана назвать ревизию, которую видела.
        (None, Some(_)) => {
            return Err(ApiError::Conflict(format!(
                "файл '{name}' уже есть: правка требует ревизии (сейчас {revision})"
            )));
        }
        (Some(seen), _) if seen != revision => {
            return Err(ApiError::Conflict(format!(
                "проект изменился: у вас ревизия {seen}, у проекта {revision}"
            )));
        }
        _ => {}
    }

    if existing.is_none() {
        let count: i64 = transaction
            .query_one(
                "SELECT count(*) FROM project_files WHERE project_id = $1",
                &[&id],
            )
            .await?
            .get(0);
        if count >= limits::FILES_PER_PROJECT {
            return Err(limits::exceeded(
                "число файлов в проекте",
                limits::FILES_PER_PROJECT,
                count + 1,
            ));
        }
    }

    let size = request.text.len() as i64;
    // ⚠️ Приведение обязательно: `sum()` над `bigint` в PostgreSQL даёт
    // `numeric`, и чтение его как `i64` кончается отказом разбора столбца.
    let others: i64 = transaction
        .query_one(
            "SELECT coalesce(sum(size_bytes), 0)::bigint FROM project_files
             WHERE project_id = $1 AND name <> $2",
            &[&id, &name],
        )
        .await?
        .get(0);
    if others + size > limits::PROJECT_BYTES {
        return Err(limits::exceeded(
            "размер проекта в байтах",
            limits::PROJECT_BYTES,
            others + size,
        ));
    }

    transaction
        .execute(
            "INSERT INTO project_files(project_id, name, kind, text, size_bytes)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (project_id, name) DO UPDATE
               SET kind = excluded.kind, text = excluded.text, size_bytes = excluded.size_bytes",
            &[&id, &name, &kind.as_str(), &request.text, &size],
        )
        .await?;
    let written = bump(&transaction, &id).await?;
    transaction.commit().await?;
    Ok(Json(written).into_response())
}

async fn remove_file(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, name)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let user = current_user(&state, &headers).await?;
    let mut client = state.pool.get().await?;
    let transaction = client.transaction().await?;
    let row = transaction
        .query_opt(
            "SELECT owner_id FROM projects WHERE id = $1 FOR UPDATE",
            &[&id],
        )
        .await?;
    let Some(row) = row else {
        return Err(ApiError::NotFound);
    };
    if row.get::<_, String>(0) != user.id {
        return Err(ApiError::NotFound);
    }
    let affected = transaction
        .execute(
            "DELETE FROM project_files WHERE project_id = $1 AND name = $2",
            &[&id, &name],
        )
        .await?;
    if affected == 0 {
        return Err(ApiError::NotFound);
    }
    // Активный файл, которого больше нет, забывается: иначе страница откроет
    // проект, показывая пустоту.
    transaction
        .execute(
            "UPDATE projects SET main_file = NULL WHERE id = $1 AND main_file = $2",
            &[&id, &name],
        )
        .await?;
    let written = bump(&transaction, &id).await?;
    transaction.commit().await?;
    Ok(Json(written).into_response())
}

/// Пересчитывает размер, поднимает ревизию и отмечает правку.
///
/// ⚠️ Размер считается **суммой по файлам**, а не приращением: приращение
/// расходится с истиной на первой же неудачной попытке, и разойдётся молча.
async fn bump(
    transaction: &tokio_postgres::Transaction<'_>,
    id: &str,
) -> Result<WriteResponse, ApiError> {
    let row = transaction
        .query_one(
            "UPDATE projects SET
                 revision = revision + 1,
                 updated_at = $2,
                 size_bytes = (SELECT coalesce(sum(size_bytes), 0)::bigint
                               FROM project_files WHERE project_id = $1)
             WHERE id = $1
             RETURNING revision, size_bytes",
            &[&id, &db::now()],
        )
        .await?;
    Ok(WriteResponse {
        revision: row.get(0),
        size_bytes: row.get(1),
    })
}

/// Читает проект и убеждается, что он принадлежит спрашивающему.
///
/// ⚠️ Чужой проект отвечает `404`, а не `403`: иначе ручка перечисляет чужие
/// проекты по ответам.
async fn owned(
    client: &deadpool_postgres::Client,
    id: &str,
    user: &User,
) -> Result<ProjectJson, ApiError> {
    let row = client.query_opt(SELECT_PROJECT, &[&id]).await?;
    let Some(row) = row else {
        return Err(ApiError::NotFound);
    };
    let owner: String = row.get("owner_id");
    if owner != user.id {
        return Err(ApiError::NotFound);
    }
    Ok(project_of(&row))
}

const SELECT_PROJECT: &str = "SELECT id, name, description, visibility, takt_lang,
        language_version, main_file, revision, size_bytes, forked_from,
        created_at, updated_at, owner_id
    FROM projects WHERE id = $1";

fn project_of(row: &tokio_postgres::Row) -> ProjectJson {
    ProjectJson {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        visibility: row.get("visibility"),
        takt_lang: row.get("takt_lang"),
        language_version: row.get("language_version"),
        main_file: row.get("main_file"),
        revision: row.get("revision"),
        size_bytes: row.get("size_bytes"),
        forked_from: row.get("forked_from"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

/// Идентификатор проекта: 16 случайных байт, base64url.
///
/// ⚠️ Не последовательный номер: при видимости `link` идентификатор И ЕСТЬ
/// секрет (задача 09c), а номера перечисляются перебором.
pub fn new_id() -> String {
    use base64::Engine as _;
    use rand::RngCore as _;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifier_is_random_and_url_safe() {
        let first = new_id();
        assert_ne!(first, new_id(), "идентификаторы не повторяются");
        assert_eq!(first.len(), 22, "16 байт в base64url без набивки");
        assert!(
            first
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "идентификатор попадает в адрес: {first}"
        );
    }
}
