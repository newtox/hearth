use crate::{auth::AuthUser, error::{bad, AppError, Result}, AppState};
use axum::{extract::{Path, State}, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Serialize, Deserialize, FromRow)]
pub struct NotifyPref {
    #[serde(default)]
    pub target_id: Uuid,
    pub level: Option<String>,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub suppress_everyone: bool,
    #[serde(default)]
    pub suppress_roles: bool,
}

pub async fn list(st: &AppState, uid: Uuid) -> Result<Vec<NotifyPref>> {
    Ok(sqlx::query_as("SELECT target_id, level, muted, suppress_everyone, suppress_roles FROM notify_prefs WHERE user_id = $1")
        .bind(uid)
        .fetch_all(&st.db)
        .await?)
}

pub async fn set(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(target): Path<Uuid>, Json(mut b): Json<NotifyPref>) -> Result<Json<Value>> {
    if let Some(level) = &b.level {
        if !["all", "mentions", "none"].contains(&level.as_str()) {
            return bad("Unknown notification level");
        }
    }
    let known: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM members WHERE guild_id = $1 AND user_id = $2) \
         OR EXISTS(SELECT 1 FROM channels c JOIN members m ON m.guild_id = c.guild_id WHERE c.id = $1 AND m.user_id = $2) \
         OR EXISTS(SELECT 1 FROM dm_recipients WHERE channel_id = $1 AND user_id = $2)",
    )
    .bind(target)
    .bind(uid)
    .fetch_one(&st.db)
    .await?;
    if !known {
        return Err(AppError::NotFound);
    }
    b.target_id = target;
    if b.level.is_none() && !b.muted && !b.suppress_everyone && !b.suppress_roles {
        sqlx::query("DELETE FROM notify_prefs WHERE user_id = $1 AND target_id = $2").bind(uid).bind(target).execute(&st.db).await?;
    } else {
        sqlx::query(
            "INSERT INTO notify_prefs (user_id, target_id, level, muted, suppress_everyone, suppress_roles) VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (user_id, target_id) DO UPDATE SET level = $3, muted = $4, suppress_everyone = $5, suppress_roles = $6",
        )
        .bind(uid)
        .bind(target)
        .bind(&b.level)
        .bind(b.muted)
        .bind(b.suppress_everyone)
        .bind(b.suppress_roles)
        .execute(&st.db)
        .await?;
    }
    st.hub.send(&[uid], "NOTIFY_UPDATE", &b);
    Ok(Json(json!(b)))
}
