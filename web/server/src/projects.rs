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

use crate::access::{self, Level};
use crate::auth::User;
use crate::db;
use crate::error::ApiError;
use crate::grants;
use crate::limits;
use crate::retention;
use crate::routes::{AppState, current_user, optional_user};
use crate::showcase;
use crate::store::Store;

/// Метаданные проекта.
#[derive(Debug, Serialize)]
pub struct ProjectJson {
    pub id: String,
    pub name: String,
    pub description: String,
    pub visibility: String,
    /// Логин владельца — псевдоним, а не персональные данные (проработка §0).
    /// Нужен странице открытого проекта: «чей это образец».
    pub owner: String,
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

/// Запись списка проектов: сам проект и мой уровень доступа к нему.
///
/// ⚠️ Уровень назван **в каждой записи**: без него страница не знает, показать
/// ли кнопку сохранения, и решала бы это по владельцу — то есть завела бы
/// вторую копию правила (класс 0084).
#[derive(Debug, Serialize)]
pub struct ProjectListJson {
    #[serde(flatten)]
    pub project: ProjectJson,
    pub level: &'static str,
}

/// Проект вместе со списком файлов.
#[derive(Debug, Serialize)]
pub struct ProjectWithFilesJson {
    #[serde(flatten)]
    pub project: ProjectJson,
    pub files: Vec<FileEntryJson>,
    /// Мой уровень доступа: по нему страница решает, показывать ли сохранение.
    pub level: &'static str,
    /// Выданные права — **только владельцу**.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grants: Option<Vec<grants::GrantJson>>,
    /// Сколько раз проект скопировали себе (владельцу; иначе 0).
    pub forks: i64,
    /// Открытые копии: закрытая копия — уже чужой проект.
    pub open_forks: Vec<grants::ForkJson>,
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
    // Свои И выданные мне — одним запросом с уровнем. ⚠️ Открытые чужие сюда
    // НЕ попадают: «мои проекты» — это то, за что я отвечаю, а открытое чужое
    // живёт в витрине. Смешай их — и список станет каталогом сервиса.
    let rows = client
        .query(
            "SELECT p.id, p.name, p.description, p.visibility, u.login AS owner,
                    p.takt_lang, p.language_version, p.main_file, p.revision,
                    p.size_bytes, p.forked_from, p.created_at, p.updated_at,
                    g.level AS granted
             FROM projects p
             JOIN users u ON u.id = p.owner_id
             LEFT JOIN project_grants g ON g.project_id = p.id AND g.user_id = $1
             WHERE p.owner_id = $1 OR g.user_id = $1
             ORDER BY p.updated_at DESC",
            &[&user.id],
        )
        .await?;
    let projects: Vec<ProjectListJson> = rows
        .iter()
        .map(|row| {
            let project = project_of(row);
            let granted: Option<String> = row.get("granted");
            let level = access::effective(
                row.get::<_, String>("owner") == user.login,
                &project.visibility,
                granted.as_deref().and_then(Level::grantable),
            );
            ProjectListJson {
                project,
                level: level.as_str(),
            }
        })
        .collect();
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
            // ⚠️ `touched_at` ставится СРАЗУ: ноль означал бы «не обращались
            // никогда», и подметание свернуло бы проект в тот же час, когда
            // его завели (задача 09h).
            "INSERT INTO projects(id, owner_id, name, description, visibility,
                                  takt_lang, language_version, revision, size_bytes,
                                  created_at, updated_at, touched_at)
             VALUES ($1, $2, $3, $4, 'private', $5, $6, 0, 0, $7, $7, $7)",
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
    // У нового проекта файлов ещё нет — тело поиска пусто, и это законно.
    showcase::refresh(&transaction, &id, "").await?;
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
    let viewer = optional_user(&state, &headers).await?;
    let client = state.pool.get().await?;
    let (project, level) = resolve(&client, &state.store, &id, viewer.as_ref()).await?;
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
    // ⚠️ Права и копии показываются ТОЛЬКО владельцу: «кому я это открыл» —
    // его дело, а читателю список тех, кто ещё имеет доступ, не принадлежит.
    let (grants, forks, open_forks) = if level == Level::Owner {
        let (count, open) = grants::forks_of(&client, &id).await?;
        (Some(grants::of_project(&client, &id).await?), count, open)
    } else {
        (None, 0, Vec::new())
    };
    Ok(Json(ProjectWithFilesJson {
        project,
        files,
        level: level.as_str(),
        grants,
        forks,
        open_forks,
    })
    .into_response())
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
    let (_, level) = locked(&transaction, &state.store, &id, &user).await?;
    // Метаданные — только владелец: `edit` правит СОДЕРЖИМОЕ, а видимость,
    // права и версия модуля меняют, кому и чем проект открывается.
    require_level(level, Level::Owner)?;
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
    // ⚠️ Имя и описание попадают в поиск: без пересчёта проект искался бы по
    // позавчерашнему имени, и увидеть это глазом нельзя. Тело при этом
    // перечитывается с диска: поисковое значение — одно, и собирается оно
    // целиком (класс 0084).
    let owner: String = transaction
        .query_one("SELECT owner_id FROM projects WHERE id = $1", &[&id])
        .await?
        .get(0);
    let body = body_of(&*transaction, &id, &state.store, &owner).await?;
    showcase::refresh(&transaction, &id, &body).await?;
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
    let (_, level) = resolve(&client, &state.store, &id, Some(&user)).await?;
    require_level(level, Level::Owner)?;
    // Состав и права уходят каскадом (схема), копии — нет: у форка своя жизнь
    // (`forked_from` объявлен `ON DELETE SET NULL`).
    client
        .execute("DELETE FROM projects WHERE id = $1", &[&id])
        .await?;
    // ⚠️ Диск чистится ПОСЛЕ базы: обратный порядок при отказе базы оставил бы
    // проект в списке без единого файла. Осиротевший каталог хуже, но он виден
    // подметанию, а проект без исходников не виден никому.
    state
        .store
        .remove_project(&user.id, &id)
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn read_file(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, name)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let viewer = optional_user(&state, &headers).await?;
    let client = state.pool.get().await?;
    let (project, _) = resolve(&client, &state.store, &id, viewer.as_ref()).await?;
    let row = client
        .query_opt(
            // ⚠️ Столбцы КВАЛИФИЦИРОВАНЫ: `name` есть и у файла, и у проекта, и
            // без квалификации запрос двусмыслен — база отвечает отказом, а
            // ручка превращает его в `500` на совершенно обычном чтении.
            "SELECT f.name, f.kind, f.size_bytes, p.owner_id FROM project_files f
             JOIN projects p ON p.id = f.project_id
             WHERE f.project_id = $1 AND f.name = $2",
            &[&id, &name],
        )
        .await?;
    let Some(row) = row else {
        return Err(ApiError::NotFound);
    };
    let owner: String = row.get(3);
    // Текст живёт на ДИСКЕ (задача 09h): база ведёт состав, а не содержимое.
    let text = state
        .store
        .read(&owner, &id, &name)
        .map_err(ApiError::Internal)?;
    Ok(Json(FileJson {
        name: row.get(0),
        kind: row.get(1),
        text,
        size_bytes: row.get(2),
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
    let (revision, level) = locked(&transaction, &state.store, &id, &user).await?;
    require_level(level, Level::Edit)?;
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
            return Err(ApiError::Conflict {
                message: format!(
                    "файл '{name}' уже есть: правка требует ревизии (сейчас {revision})"
                ),
                seen: None,
                actual: revision,
            });
        }
        (Some(seen), _) if seen != revision => {
            return Err(ApiError::Conflict {
                message: format!("проект изменился: у вас ревизия {seen}, у проекта {revision}"),
                seen: Some(seen),
                actual: revision,
            });
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
    let owner: String = transaction
        .query_one("SELECT owner_id FROM projects WHERE id = $1", &[&id])
        .await?
        .get(0);
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
            "INSERT INTO project_files(project_id, name, kind, size_bytes)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (project_id, name) DO UPDATE
               SET kind = excluded.kind, size_bytes = excluded.size_bytes",
            &[&id, &name, &kind.as_str(), &size],
        )
        .await?;
    // ⚠️ Диск пишется ДО фиксации записи в базе: обратный порядок при отказе
    // диска оставил бы в составе файл, которого нет, — а состав читает страница.
    // Отказ диска здесь откатывает транзакцию, и база остаётся согласной.
    state
        .store
        .write(&owner, &id, &name, &request.text)
        .map_err(ApiError::Internal)?;
    let written = bump(&transaction, &id, &state.store, &owner).await?;
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
    let (_, level) = locked(&transaction, &state.store, &id, &user).await?;
    require_level(level, Level::Edit)?;
    let owner: String = transaction
        .query_one("SELECT owner_id FROM projects WHERE id = $1", &[&id])
        .await?
        .get(0);
    let affected = transaction
        .execute(
            "DELETE FROM project_files WHERE project_id = $1 AND name = $2",
            &[&id, &name],
        )
        .await?;
    if affected == 0 {
        return Err(ApiError::NotFound);
    }
    state
        .store
        .remove(&owner, &id, &name)
        .map_err(ApiError::Internal)?;
    // Активный файл, которого больше нет, забывается: иначе страница откроет
    // проект, показывая пустоту.
    transaction
        .execute(
            "UPDATE projects SET main_file = NULL WHERE id = $1 AND main_file = $2",
            &[&id, &name],
        )
        .await?;
    let written = bump(&transaction, &id, &state.store, &owner).await?;
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
    store: &Store,
    owner: &str,
) -> Result<WriteResponse, ApiError> {
    // Тексты файлов идут в поиск, и пересчёт стоит **в той же транзакции**:
    // отдельным запросом он разошёлся бы с содержимым при первом же откате.
    // ⚠️ Тело берётся с ДИСКА (задача 09h): базе его взять неоткуда.
    let body = body_of(transaction, id, store, owner).await?;
    showcase::refresh(transaction, id, &body).await?;
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

/// Читает ревизию и уровень доступа, взяв строку проекта под замок.
///
/// ⚠️ Право спрашивается **внутри той же транзакции**, что и запись: между
/// отдельными запросами помещается отзыв права, и запись прошла бы по уже
/// отобранному.
pub(crate) async fn locked(
    transaction: &tokio_postgres::Transaction<'_>,
    store: &Arc<Store>,
    id: &str,
    user: &User,
) -> Result<(i64, Level), ApiError> {
    let row = transaction
        .query_opt(
            "SELECT owner_id, visibility, revision, archived_at, touched_at
             FROM projects WHERE id = $1 FOR UPDATE",
            &[&id],
        )
        .await?;
    let Some(row) = row else {
        return Err(ApiError::NotFound);
    };
    let owner: String = row.get(0);
    let visibility: String = row.get(1);
    let revision: i64 = row.get(2);
    let granted = transaction
        .query_opt(
            "SELECT level FROM project_grants WHERE project_id = $1 AND user_id = $2",
            &[&id, &user.id],
        )
        .await?
        .map(|row| row.get::<_, String>(0));
    let level = access::effective(
        user.id == owner,
        &visibility,
        granted.as_deref().and_then(Level::grantable),
    );
    if level == Level::None {
        return Err(ApiError::NotFound);
    }
    // Запись — тоже обращение: счётчик сбрасывается, свёрнутый проект
    // разворачивается. Иначе запись легла бы поверх снятых с диска файлов.
    retention::touch(transaction, store, id, &owner, row.get(3), row.get(4))
        .await
        .map_err(ApiError::Internal)?;
    Ok((revision, level))
}

/// Собирает тело поиска — тексты файлов `*.takt` одной строкой.
///
/// ⚠️ Список имён берёт БАЗА, а тексты — диск: каталог знает про файлы, которых
/// нет в проекте (обрывок записи), а база — про состав.
///
/// ⚠️ Отказ чтения файла телом не считается ошибкой запроса: проект мог быть
/// свёрнут либо файл потерян, и терять из-за этого правку соседнего файла было
/// бы хуже, чем искать по неполному телу. Пропуск виден в журнале.
pub(crate) async fn body_of<C: tokio_postgres::GenericClient>(
    client: &C,
    id: &str,
    store: &Store,
    owner: &str,
) -> Result<String, ApiError> {
    let rows = client
        .query(
            "SELECT name FROM project_files WHERE project_id = $1 AND kind = 'takt' ORDER BY name",
            &[&id],
        )
        .await?;
    let mut body = String::new();
    for row in &rows {
        let name: String = row.get(0);
        match store.read(owner, id, &name) {
            Ok(text) => {
                body.push_str(&text);
                body.push('\n');
            }
            Err(error) => tracing::warn!(%error, project = %id, file = %name, "файл не прочитан"),
        }
    }
    Ok(body)
}

/// Читает проект и уровень доступа к нему спрашивающего.
///
/// ⚠️ Уровень считает ОДИН носитель [`access::effective`]; здесь только сбор
/// исходных данных. Ответь эта функция сама — правило стало бы жить в двух
/// местах, и расхождение проявилось бы не отказом, а чужой записью в чужой
/// проект.
///
/// ⚠️ Проект, недоступный вовсе, отвечает `404`, а не `403`: `403` означал бы
/// «он есть, но не для вас», то есть ручка стала бы оракулом существования.
pub(crate) async fn resolve(
    client: &deadpool_postgres::Client,
    store: &Arc<Store>,
    id: &str,
    viewer: Option<&User>,
) -> Result<(ProjectJson, Level), ApiError> {
    let row = client.query_opt(SELECT_PROJECT, &[&id]).await?;
    let Some(row) = row else {
        return Err(ApiError::NotFound);
    };
    let project = project_of(&row);
    let owner: String = row.get("owner_id");
    let level = level_of(client, id, &owner, &project.visibility, viewer).await?;
    if level == Level::None {
        return Err(ApiError::NotFound);
    }
    // ⚠️ Обращение на ЧТЕНИЕ тоже сбрасывает счётчик хранения (корректировка
    // заказчика): пропусти его — и проект, который читают каждый день,
    // однажды свернётся под руками читателя. Здесь же свёрнутый проект
    // разворачивается обратно.
    retention::touch(
        &***client,
        store,
        id,
        &owner,
        row.get("archived_at"),
        row.get("touched_at"),
    )
    .await
    .map_err(ApiError::Internal)?;
    Ok((project, level))
}

/// Считает уровень доступа, спросив выданное право.
async fn level_of(
    client: &deadpool_postgres::Client,
    id: &str,
    owner: &str,
    visibility: &str,
    viewer: Option<&User>,
) -> Result<Level, ApiError> {
    let Some(user) = viewer else {
        // Без входа выданного права быть не может: оно выдаётся человеку.
        return Ok(access::effective(false, visibility, None));
    };
    let granted = client
        .query_opt(
            "SELECT level FROM project_grants WHERE project_id = $1 AND user_id = $2",
            &[&id, &user.id],
        )
        .await?
        .map(|row| row.get::<_, String>(0));
    Ok(access::effective(
        user.id == owner,
        visibility,
        granted.as_deref().and_then(Level::grantable),
    ))
}

/// Требует уровня не ниже названного.
///
/// ⚠️ Здесь `403`, а не `404`: до этой точки доходит тот, кому проект **видно**,
/// и прятать его уже незачем — а «не найдено» вместо «нет права» отправило бы
/// человека искать опечатку в ссылке.
pub(crate) fn require_level(level: Level, needed: Level) -> Result<(), ApiError> {
    if level >= needed {
        Ok(())
    } else {
        Err(ApiError::Forbidden)
    }
}

pub(crate) const SELECT_PROJECT: &str = "SELECT p.id, p.name, p.description, p.visibility,
        u.login AS owner, p.takt_lang, p.language_version, p.main_file, p.revision,
        p.size_bytes, p.forked_from, p.created_at, p.updated_at, p.owner_id,
        p.touched_at, p.archived_at
    FROM projects p JOIN users u ON u.id = p.owner_id WHERE p.id = $1";

pub(crate) fn project_of(row: &tokio_postgres::Row) -> ProjectJson {
    ProjectJson {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        visibility: row.get("visibility"),
        owner: row.get("owner"),
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
