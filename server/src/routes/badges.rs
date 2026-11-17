use super::{clean_color, clean_name, users::announce};
use crate::{auth::AuthUser, error::{bad, AppError, Result}, models::Badge, AppState};
use axum::{extract::{Path, State}, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

pub const ICONS: [&str; 12] = ["crown", "shield", "gem", "star", "heart", "bolt", "flame", "leaf", "gamepad", "broadcast", "mic", "calendar"];

async fn require_admin(st: &AppState, uid: Uuid) -> Result<()> {
    let admin: bool = sqlx::query_scalar("SELECT is_admin FROM users WHERE id = $1").bind(uid).fetch_one(&st.db).await?;
    if admin { Ok(()) } else { Err(AppError::Forbidden) }
}

pub async fn list(State(st): State<AppState>, AuthUser(_): AuthUser) -> Result<Json<Vec<Badge>>> {
    Ok(Json(sqlx::query_as("SELECT id, name, description, icon, color FROM badges ORDER BY position, name").fetch_all(&st.db).await?))
}

#[derive(Deserialize)]
pub struct BadgeBody {
    name: String,
    #[serde(default)]
    description: String,
    icon: String,
    color: Option<String>,
}

pub async fn create(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(b): Json<BadgeBody>) -> Result<Json<Badge>> {
    require_admin(&st, uid).await?;
    let name = clean_name(&b.name, 32, "Name")?;
    let icon = b.icon.trim();
    let built_in = icon.strip_prefix("svg:").is_some_and(|name| ICONS.contains(&name));
    if !built_in && (icon.is_empty() || icon.len() > 16 || icon.chars().any(|c| c.is_ascii() || c.is_control())) {
        return bad("Unknown symbol");
    }
    if b.description.chars().count() > 120 {
        return bad("Description: at most 120 characters");
    }
    let color = clean_color(&b.color)?.unwrap_or_else(|| "#8b6cff".into());
    Ok(Json(
        sqlx::query_as(
            "INSERT INTO badges (id, name, description, icon, color, position) VALUES ($1, $2, $3, $4, $5, (SELECT COALESCE(max(position), 0) + 1 FROM badges)) RETURNING id, name, description, icon, color",
        )
        .bind(Uuid::now_v7())
        .bind(name)
        .bind(b.description.trim())
        .bind(icon)
        .bind(color)
        .fetch_one(&st.db)
        .await?,
    ))
}

pub async fn delete(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(bid): Path<Uuid>) -> Result<Json<Value>> {
    require_admin(&st, uid).await?;
    sqlx::query("DELETE FROM badges WHERE id = $1").bind(bid).execute(&st.db).await?;
    Ok(Json(json!({})))
}

pub async fn grant(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((target, bid)): Path<(Uuid, Uuid)>) -> Result<Json<Value>> {
    require_admin(&st, uid).await?;
    sqlx::query("INSERT INTO user_badges (user_id, badge_id) VALUES ($1, $2) ON CONFLICT DO NOTHING").bind(target).bind(bid).execute(&st.db).await?;
    announce(&st, target).await?;
    Ok(Json(json!({})))
}

pub async fn revoke(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((target, bid)): Path<(Uuid, Uuid)>) -> Result<Json<Value>> {
    require_admin(&st, uid).await?;
    sqlx::query("DELETE FROM user_badges WHERE user_id = $1 AND badge_id = $2").bind(target).bind(bid).execute(&st.db).await?;
    announce(&st, target).await?;
    Ok(Json(json!({})))
}
