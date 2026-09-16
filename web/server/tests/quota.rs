//! Проверки квоты владельца.
//!
//! Политика та же, что у прочих наборов: нет базы - проверки не выполняются и говорят
//! об этом словами.
//!
//! Квота стенда здесь мала - несколько сотен байт: предмет проверки - правило, а не
//! величина, и файл в десяток килобайт на каждую пробу только замедлил бы набор.

mod common;

use axum::http::StatusCode;
use common::{Stand, skipped};

/// Квота стенда проверок, байты.
const QUOTA: i64 = 1000;

/// Поднимает стенд с малой квотой.
async fn stand(tag: &str) -> Option<Stand> {
    Stand::open_with(tag, |config| config.user_bytes = QUOTA).await
}

/// Заводит человека и возвращает его access-токен.
async fn person(stand: &Stand, login: &str) -> String {
    let (status, body) = stand
        .post(
            "/api/register",
            serde_json::json!({"login": login, "password": "пароль-пароль"}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["access_token"].as_str().expect("токен").to_string()
}

/// Создаёт пустой проект.
async fn project(stand: &Stand, token: &str, name: &str) -> String {
    let (status, body) = stand
        .post_as("/api/projects", token, serde_json::json!({"name": name}))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["id"].as_str().expect("идентификатор").to_string()
}

/// Текущая ревизия проекта.
async fn revision(stand: &Stand, token: &str, id: &str) -> i64 {
    let (status, body) = stand.get_as(&format!("/api/projects/{id}"), token).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["revision"].as_i64().expect("ревизия")
}

/// Пишет файл из `size` байт: новый - без ревизии, существующий - с текущей.
async fn write(
    stand: &Stand,
    token: &str,
    id: &str,
    name: &str,
    size: usize,
) -> (StatusCode, serde_json::Value) {
    let (_, body) = stand.get_as(&format!("/api/projects/{id}"), token).await;
    let exists = body["files"]
        .as_array()
        .is_some_and(|files| files.iter().any(|file| file["name"] == name));
    let mut request = serde_json::json!({"text": "x".repeat(size)});
    if exists {
        request["revision"] = revision(stand, token, id).await.into();
    }
    stand
        .put_as(&format!("/api/projects/{id}/files/{name}"), token, request)
        .await
}

/// Занятое и квота из `me`.
async fn usage(stand: &Stand, token: &str) -> (i64, i64) {
    let (status, me) = stand.get_as("/api/me", token).await;
    assert_eq!(status, StatusCode::OK, "{me}");
    (
        me["used_bytes"].as_i64().expect("занято"),
        me["quota_bytes"].as_i64().expect("квота"),
    )
}

/// Отказ предела называет квоту и итог.
fn assert_quota_refusal(status: StatusCode, body: &serde_json::Value, after: i64) {
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE, "{body}");
    assert_eq!(body["error"], "limit_exceeded", "{body}");
    let text = body["message"].as_str().expect("текст");
    assert!(
        text.contains(&QUOTA.to_string()) && text.contains(&after.to_string()),
        "{text}"
    );
}

#[tokio::test]
async fn a_file_write_is_judged_by_the_quota_of_all_projects() {
    let Some(stand) = stand("q_write").await else {
        return skipped("квота записи файла");
    };
    let ivan = person(&stand, "ivan").await;
    let first = project(&stand, &ivan, "Первый").await;
    let second = project(&stand, &ivan, "Второй").await;
    assert_eq!(usage(&stand, &ivan).await, (0, QUOTA), "пустой владелец");

    let (status, body) = write(&stand, &ivan, &first, "a.takt", 600).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // Второй проект сам по себе мал, но вместе с первым выходит за квоту.
    let (status, body) = write(&stand, &ivan, &second, "b.takt", 401).await;
    assert_quota_refusal(status, &body, 1001);
    let (status, body) = write(&stand, &ivan, &second, "b.takt", 400).await;
    assert_eq!(status, StatusCode::OK, "ровно квота: {body}");
    assert_eq!(usage(&stand, &ivan).await, (1000, QUOTA));

    // Замена файла считается разностью: тот же размер проходит, больший - нет.
    let (status, body) = write(&stand, &ivan, &first, "a.takt", 600).await;
    assert_eq!(status, StatusCode::OK, "замена тем же объёмом: {body}");
    let (status, body) = write(&stand, &ivan, &first, "a.takt", 601).await;
    assert_quota_refusal(status, &body, 1001);

    // Квота одна на человека: сосед пишет свободно.
    let petr = person(&stand, "petr").await;
    let his = project(&stand, &petr, "Чужой").await;
    let (status, body) = write(&stand, &petr, &his, "c.takt", 900).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    stand.drop_schema().await;
}

#[tokio::test]
async fn beyond_the_quota_only_growth_is_refused() {
    let Some(stand) = stand("q_beyond").await else {
        return skipped("владелец сверх квоты");
    };
    let ivan = person(&stand, "ivan").await;
    let id = project(&stand, &ivan, "Большой").await;
    let other = project(&stand, &ivan, "Лишний").await;
    let (status, body) = write(&stand, &ivan, &id, "a.takt", 700).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = write(&stand, &ivan, &other, "b.takt", 300).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // Владелец за квотой: так бывает, когда квоту стенда уменьшили.
    stand
        .execute(
            "UPDATE projects SET size_bytes = size_bytes + 500 WHERE id = $1",
            &[&id],
        )
        .await;
    assert_eq!(usage(&stand, &ivan).await, (1500, QUOTA));

    let (status, body) = write(&stand, &ivan, &id, "a.takt", 701).await;
    assert_quota_refusal(status, &body, 1501);
    let (status, body) = write(&stand, &ivan, &id, "a.takt", 100).await;
    assert_eq!(status, StatusCode::OK, "сокращение: {body}");
    let (status, _) = stand
        .delete_as(&format!("/api/projects/{other}"), &ivan)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "удаление");
    let (status, body) = stand.get_as(&format!("/api/projects/{id}"), &ivan).await;
    assert_eq!(status, StatusCode::OK, "чтение: {body}");
    stand.drop_schema().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_writes_at_once_do_not_share_the_same_room() {
    let Some(stand) = stand("q_race").await else {
        return skipped("параллельные записи");
    };
    let ivan = person(&stand, "ivan").await;
    // Каждая запись в пределе, обе вместе - нет. Без замка по владельцу обе видели бы
    // одну сумму и прошли бы обе.
    for round in 0..8 {
        let first = project(&stand, &ivan, &format!("Левый {round}")).await;
        let second = project(&stand, &ivan, &format!("Правый {round}")).await;
        let left = format!("/api/projects/{first}/files/a.takt");
        let right = format!("/api/projects/{second}/files/b.takt");
        let (one, two) = tokio::join!(
            stand.put_as(&left, &ivan, serde_json::json!({"text": "x".repeat(600)})),
            stand.put_as(&right, &ivan, serde_json::json!({"text": "x".repeat(600)})),
        );
        let passed = [one.0, two.0]
            .iter()
            .filter(|status| **status == StatusCode::OK)
            .count();
        assert_eq!(passed, 1, "раунд {round}: {one:?} {two:?}");
        assert!(usage(&stand, &ivan).await.0 <= QUOTA, "раунд {round}");
        for id in [first, second] {
            stand.delete_as(&format!("/api/projects/{id}"), &ivan).await;
        }
    }
    stand.drop_schema().await;
}

#[tokio::test]
async fn an_edit_by_a_grantee_spends_the_owner_quota() {
    let Some(stand) = stand("q_grantee").await else {
        return skipped("правка по праву");
    };
    let ivan = person(&stand, "ivan").await;
    let vera = person(&stand, "vera").await;
    let id = project(&stand, &ivan, "Общий").await;
    let mine = project(&stand, &ivan, "Мой").await;
    let (status, body) = write(&stand, &ivan, &mine, "a.takt", 800).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = stand
        .put_as(
            &format!("/api/projects/{id}/grants/vera"),
            &ivan,
            serde_json::json!({"level": "edit"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = write(&stand, &vera, &id, "b.takt", 201).await;
    assert_quota_refusal(status, &body, 1001);
    let (status, body) = write(&stand, &vera, &id, "b.takt", 200).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(usage(&stand, &vera).await.0, 0, "редактор не тратит своего");
    assert_eq!(usage(&stand, &ivan).await.0, 1000);
    stand.drop_schema().await;
}

#[tokio::test]
async fn a_fork_and_an_import_are_judged_by_the_taker_quota() {
    let Some(stand) = stand("q_fork").await else {
        return skipped("копия и загрузка архива");
    };
    let ivan = person(&stand, "ivan").await;
    let vera = person(&stand, "vera").await;
    let id = project(&stand, &ivan, "Образец").await;
    let (status, body) = write(&stand, &ivan, &id, "model.takt", 400).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, _) = stand
        .patch_as(
            &format!("/api/projects/{id}"),
            &ivan,
            serde_json::json!({"visibility": "public"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, archive) = stand
        .bytes(&format!("/api/projects/{id}/archive"), Some(&ivan))
        .await;
    assert_eq!(status, StatusCode::OK);

    let own = project(&stand, &vera, "Свой").await;
    let (status, body) = write(&stand, &vera, &own, "a.takt", 300).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // 300 своих + 400 копии в пределе, ещё 400 - уже нет.
    let fork = format!("/api/projects/{id}/fork");
    let (status, body) = stand.post_as(&fork, &vera, serde_json::json!({})).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let (status, body) = stand.post_as(&fork, &vera, serde_json::json!({})).await;
    assert_quota_refusal(status, &body, 1100);
    let (status, body) = stand.upload("/api/projects/import", &vera, &archive).await;
    assert_quota_refusal(status, &body, 1100);
    assert_eq!(usage(&stand, &vera).await.0, 700, "отказ ничего не занял");

    // Освободив место, загрузку тот же архив проходит.
    let (status, _) = stand
        .delete_as(&format!("/api/projects/{own}"), &vera)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, body) = stand.upload("/api/projects/import", &vera, &archive).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(usage(&stand, &vera).await.0, 800);
    stand.drop_schema().await;
}
