//! Проверки набора наблюдаемых выходов прогона в метаданных проекта.
//!
//! Политика та же, что у прочих наборов: нет базы - проверки не выполняются и говорят
//! об этом словами.

mod common;

use axum::http::StatusCode;
use common::{Stand, skipped};
use serde_json::json;

/// Заводит человека и возвращает его access-токен.
async fn person(stand: &Stand, login: &str) -> String {
    let (status, body) = stand
        .post(
            "/api/register",
            json!({"login": login, "password": "пароль-пароль"}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["access_token"].as_str().expect("токен").to_string()
}

#[tokio::test]
async fn the_watch_is_kept_per_model_and_follows_its_file() {
    let Some(stand) = Stand::open("w_watch").await else {
        return skipped("набор наблюдения прогона");
    };
    let token = person(&stand, "ivan").await;
    let (status, body) = stand
        .post_as("/api/projects", &token, json!({"name": "Насос"}))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let id = body["id"].as_str().expect("id").to_string();
    for (name, text) in [("pump.takt", "start Run {}"), ("run.json", "[]")] {
        let (status, body) = stand
            .put_as(
                &format!("/api/projects/{id}/files/{name}"),
                &token,
                json!({"text": text}),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{name}: {body}");
    }
    let path = format!("/api/projects/{id}");

    // Ключ - только модель проекта, имена - в форме имени порта.
    for watch in [
        json!({"run.json": ["speed"]}),
        json!({"нет-такого.takt": ["speed"]}),
        json!({"pump.takt": ["sp eed"]}),
    ] {
        let (status, body) = stand
            .patch_as(&path, &token, json!({ "run_watch": watch }))
            .await;
        assert!(status.is_client_error(), "{watch}: {status} {body}");
    }

    // Контроль: годный набор принимается, повтор снят, порядок сохранён.
    let (status, patched) = stand
        .patch_as(
            &path,
            &token,
            json!({"run_watch": {"pump.takt": ["speed", "Pump::on", "speed"]}}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{patched}");
    assert_eq!(
        patched["run_watch"],
        json!({"pump.takt": ["speed", "Pump::on"]})
    );

    // Другие настройки прогона набор не трогает.
    let (_, patched) = stand
        .patch_as(&path, &token, json!({"run_delays": {"run.json": 0.5}}))
        .await;
    assert_eq!(
        patched["run_watch"],
        json!({"pump.takt": ["speed", "Pump::on"]}),
        "{patched}"
    );

    // Копия несёт набор: это часть показа модели автором.
    let (_, _) = stand
        .patch_as(&path, &token, json!({"visibility": "public"}))
        .await;
    let petr = person(&stand, "petr").await;
    let (status, copy) = stand
        .post_as(&format!("{path}/fork"), &petr, json!({}))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{copy}");
    assert_eq!(
        copy["run_watch"],
        json!({"pump.takt": ["speed", "Pump::on"]})
    );

    // Переименованная модель уносит набор, удалённая - забывает.
    let (status, body) = stand
        .post_as(
            &format!("{path}/files/pump.takt/rename"),
            &token,
            json!({"to": "motor.takt"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (_, read) = stand.get_as(&path, &token).await;
    assert_eq!(
        read["run_watch"],
        json!({"motor.takt": ["speed", "Pump::on"]}),
        "{read}"
    );
    stand
        .delete_as(&format!("{path}/files/motor.takt"), &token)
        .await;
    let (_, read) = stand.get_as(&path, &token).await;
    assert_eq!(read["run_watch"], json!({}), "{read}");

    // Пустой список не хранится.
    stand
        .put_as(
            &format!("{path}/files/pump.takt"),
            &token,
            json!({"text": "start Run {}"}),
        )
        .await;
    let (_, patched) = stand
        .patch_as(&path, &token, json!({"run_watch": {"pump.takt": []}}))
        .await;
    assert_eq!(patched["run_watch"], json!({}), "{patched}");

    stand.drop_schema().await;
}

#[tokio::test]
async fn the_watch_survives_the_archive_round_trip() {
    let Some(stand) = Stand::open("w_archive").await else {
        return skipped("набор наблюдения в архиве");
    };
    let token = person(&stand, "ivan").await;
    let (_, body) = stand
        .post_as("/api/projects", &token, json!({"name": "Насос"}))
        .await;
    let id = body["id"].as_str().expect("id").to_string();
    stand
        .put_as(
            &format!("/api/projects/{id}/files/pump.takt"),
            &token,
            json!({"text": "start Run {}"}),
        )
        .await;
    let (status, _) = stand
        .patch_as(
            &format!("/api/projects/{id}"),
            &token,
            json!({"run_watch": {"pump.takt": ["speed"]}}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, archive) = stand
        .bytes(&format!("/api/projects/{id}/archive"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, loaded) = stand.upload("/api/projects/import", &token, &archive).await;
    assert_eq!(status, StatusCode::CREATED, "{loaded}");
    assert_eq!(loaded["run_watch"], json!({"pump.takt": ["speed"]}));
    stand.drop_schema().await;
}
