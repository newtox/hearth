use crate::{auth::AuthUser, error::Result, AppState};
use axum::{extract::State, Json};
use serde_json::{json, Value};
use uuid::Uuid;

pub async fn save(st: &AppState, uid: Uuid, provider: &str, account: &str, name: &str, url: Option<&str>) -> Result<()> {
    sqlx::query(
        "INSERT INTO connections (user_id, provider, account_id, name, url) VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (user_id, provider) DO UPDATE SET account_id = EXCLUDED.account_id, name = EXCLUDED.name, url = EXCLUDED.url",
    )
    .bind(uid)
    .bind(provider)
    .bind(account)
    .bind(name.chars().take(64).collect::<String>())
    .bind(url)
    .execute(&st.db)
    .await?;
    Ok(())
}

pub async fn of_user(st: &AppState, uid: Uuid) -> Result<Value> {
    let rows: Vec<(String, String, Option<String>)> = sqlx::query_as("SELECT provider, name, url FROM connections WHERE user_id = $1 ORDER BY provider").bind(uid).fetch_all(&st.db).await?;
    Ok(json!(rows.into_iter().map(|(provider, name, url)| json!({ "provider": provider, "name": name, "url": url })).collect::<Vec<_>>()))
}

pub async fn list(State(st): State<AppState>, AuthUser(uid): AuthUser) -> Result<Json<Value>> {
    let available: Vec<Value> = if st.spotify.is_some() { vec![json!({ "id": "spotify", "name": "Spotify" })] } else { Vec::new() };
    Ok(Json(json!({ "available": available, "linked": of_user(&st, uid).await? })))
}
