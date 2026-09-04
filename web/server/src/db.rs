//! Хранилище: пул соединений и схема (фича 0531, задача 09a).
//!
//! # PostgreSQL, а не SQLite
//!
//! Решение заказчика 2026-09-04 отменяет выбор проработки. Вместе с SQLite
//! уходит и названная там граница «один писатель»: у PostgreSQL запись
//! параллельна, и очередь на соединении была бы выдуманной — отсюда пул, а не
//! одно соединение под мьютексом.
//!
//! Полнотекстовый поиск (задача `09c`) переезжает с FTS5 на `tsvector`
//! средствами самой базы; словарь `russian` разбирает кириллицу без
//! дополнительных расширений.
//!
//! # Схема — одна функция, а не каталог миграций
//!
//! Версия лежит в таблице `schema_version`, а сама схема описана **одним
//! местом**: пока выпуска не было, «миграция» с первой версии на вторую — это
//! лишний носитель того же знания (класс 0084). Появится выложенный стенд с
//! данными — появится и шаг перехода, и он будет виден в номере версии.
//!
//! # Чего в схеме нет и почему
//!
//! Ни почты, ни имени, ни адреса, ни `User-Agent`, ни часового пояса, ни
//! `last_seen_at`. Решение заказчика 2026-09-04: восстановление пароля —
//! **сброс администратором**, поэтому почта не нужна вовсе, а обещание A6 «не
//! хранить адрес» держится тем, что хранить его негде. Сторож на это стоит
//! отдельным тестом: колонка, заведённая «на будущее», — это персональные
//! данные, которых никто не собирался собирать.

use deadpool_postgres::{Config as PoolConfig, Pool, Runtime};
use tokio_postgres::NoTls;

/// Версия схемы. Растёт вместе с изменением таблиц.
pub const SCHEMA_VERSION: i64 = 1;

/// Заводит пул соединений по строке подключения.
///
/// # Ошибки
/// Строка подключения не разбирается либо пул не создаётся.
pub fn pool(url: &str) -> anyhow::Result<Pool> {
    let mut config = PoolConfig::new();
    config.url = Some(url.to_string());
    let pool = config.create_pool(Some(Runtime::Tokio1), NoTls)?;
    Ok(pool)
}

/// Приводит схему к [`SCHEMA_VERSION`].
///
/// # Ошибки
/// База чужой версии — отказ с обоими номерами: молчаливый переход между
/// версиями означал бы порчу данных стенда.
pub async fn prepare(client: &tokio_postgres::Client) -> anyhow::Result<()> {
    client
        .batch_execute("CREATE TABLE IF NOT EXISTS schema_version (version BIGINT NOT NULL)")
        .await?;
    let rows = client
        .query("SELECT version FROM schema_version", &[])
        .await?;
    match rows.first().map(|row| row.get::<_, i64>(0)) {
        Some(version) if version == SCHEMA_VERSION => return Ok(()),
        Some(version) => {
            anyhow::bail!("база версии {version}, а сервер знает {SCHEMA_VERSION}");
        }
        None => {}
    }
    client.batch_execute(SCHEMA).await?;
    client
        .execute(
            "INSERT INTO schema_version(version) VALUES ($1)",
            &[&SCHEMA_VERSION],
        )
        .await?;
    Ok(())
}

/// Схема целиком.
///
/// ⚠️ Таблицы проектов заводятся **здесь и сразу**, хотя ручки к ним появятся
/// задачей `09b`: схема — одно место, и дописывать её частями значило бы
/// заводить переход между версиями до первого выпуска.
pub const SCHEMA: &str = r#"
CREATE TABLE users (
    id         TEXT PRIMARY KEY,
    -- ⚠️ `CITEXT` в проекте не заводится (расширение ставится отдельно и на
    -- стенде его может не быть): единственность без учёта регистра держит
    -- индекс по `lower(login)`. `Ivan` и `ivan` — один человек, иначе вход
    -- зависел бы от регистра, а два владельца получили бы неразличимые на
    -- глаз имена.
    login      TEXT NOT NULL,
    pass_hash  TEXT NOT NULL,
    role       TEXT NOT NULL DEFAULT 'user' CHECK (role IN ('user', 'admin')),
    created_at BIGINT NOT NULL
);
CREATE UNIQUE INDEX users_login_lower ON users (lower(login));

CREATE TABLE refresh_tokens (
    id         BIGSERIAL PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Хранится ОТПЕЧАТОК, а не токен: утечка базы не даёт войти.
    token_hash TEXT NOT NULL UNIQUE,
    -- Семейство от одного входа: повторное предъявление гасит его целиком.
    family     TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    expires_at BIGINT NOT NULL,
    revoked_at BIGINT
);
CREATE INDEX refresh_tokens_family ON refresh_tokens(family);

CREATE TABLE projects (
    id               TEXT PRIMARY KEY,
    owner_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name             TEXT NOT NULL,
    description      TEXT NOT NULL DEFAULT '',
    visibility       TEXT NOT NULL CHECK (visibility IN ('private', 'link', 'public')),
    takt_lang        TEXT NOT NULL,
    language_version TEXT NOT NULL,
    main_file        TEXT,
    revision         BIGINT NOT NULL DEFAULT 0,
    size_bytes       BIGINT NOT NULL DEFAULT 0,
    forked_from      TEXT REFERENCES projects(id) ON DELETE SET NULL,
    created_at       BIGINT NOT NULL,
    updated_at       BIGINT NOT NULL
);
CREATE INDEX projects_owner ON projects(owner_id);
CREATE INDEX projects_public ON projects(visibility, updated_at);

CREATE TABLE project_files (
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    kind       TEXT NOT NULL CHECK (kind IN ('takt', 'scenario')),
    text       TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    PRIMARY KEY (project_id, name)
);

CREATE TABLE project_grants (
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    level      TEXT NOT NULL CHECK (level IN ('view', 'fork', 'edit')),
    granted_at BIGINT NOT NULL,
    PRIMARY KEY (project_id, user_id)
);

-- Поиск по открытым проектам (задача 09c). Колонка считается базой, а не
-- кодом: вторая точка вычисления разошлась бы с первой при первой же правке
-- запроса. Словарь `russian` разбирает кириллицу; `simple` оставлен именам,
-- которые склонять нечего.
ALTER TABLE projects ADD COLUMN search tsvector
    GENERATED ALWAYS AS (
        setweight(to_tsvector('russian', coalesce(name, '')), 'A') ||
        setweight(to_tsvector('russian', coalesce(description, '')), 'B')
    ) STORED;
CREATE INDEX projects_search ON projects USING gin(search);
"#;

/// Текущее время в Unix-секундах.
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}
