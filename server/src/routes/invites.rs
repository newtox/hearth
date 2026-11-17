use super::{guilds::snapshot, refresh};
use crate::{auth::AuthUser, error::{bad, AppError, Result}, perms, AppState};
use axum::{extract::{Path, State}, Json};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Deserialize, Default)]
pub struct InviteBody {
    max_uses: Option<i32>,
    expires_in_hours: Option<i64>,
}

fn code() -> String {
    const CHARS: &[u8] = b"abcdefghjkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    rand::random::<[u8; 10]>().iter().map(|b| CHARS[*b as usize % CHARS.len()] as char).collect()
}

pub async fn create(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(gid): Path<Uuid>, body: Option<Json<InviteBody>>) -> Result<Json<Value>> {
    perms::ctx(&st.db, gid, uid).await?.require(perms::CREATE_INVITE)?;
    let b = body.map(|b| b.0).unwrap_or_default();
    let expires: Option<DateTime<Utc>> = b.expires_in_hours.filter(|h| *h > 0).map(|h| Utc::now() + Duration::hours(h.min(24 * 365)));
    let code = code();
    sqlx::query("INSERT INTO invites (code, guild_id, creator_id, max_uses, expires_at) VALUES ($1, $2, $3, $4, $5)")
        .bind(&code)
        .bind(gid)
        .bind(uid)
        .bind(b.max_uses.filter(|m| *m > 0))
        .bind(expires)
        .execute(&st.db)
        .await?;
    Ok(Json(json!({ "code": code, "max_uses": b.max_uses, "expires_at": expires, "uses": 0 })))
}

pub async fn list(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(gid): Path<Uuid>) -> Result<Json<Value>> {
    perms::ctx(&st.db, gid, uid).await?.require(perms::MANAGE_GUILD)?;
    let rows: Vec<(String, i32, Option<i32>, Option<DateTime<Utc>>, String)> = sqlx::query_as(
        "SELECT i.code, i.uses, i.max_uses, i.expires_at, u.username FROM invites i JOIN users u ON u.id = i.creator_id WHERE i.guild_id = $1 ORDER BY i.created_at DESC",
    )
    .bind(gid)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(|r| json!({ "code": r.0, "uses": r.1, "max_uses": r.2, "expires_at": r.3, "creator": r.4 }))
        .collect::<Vec<_>>())))
}

pub async fn delete(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((gid, code)): Path<(Uuid, String)>) -> Result<Json<Value>> {
    perms::ctx(&st.db, gid, uid).await?.require(perms::MANAGE_GUILD)?;
    sqlx::query("DELETE FROM invites WHERE code = $1 AND guild_id = $2").bind(code).bind(gid).execute(&st.db).await?;
    Ok(Json(json!({})))
}

async fn valid(st: &AppState, code: &str) -> Result<Uuid> {
    let row: Option<(Uuid, i32, Option<i32>, Option<DateTime<Utc>>)> = sqlx::query_as("SELECT guild_id, uses, max_uses, expires_at FROM invites WHERE code = $1")
        .bind(code)
        .fetch_optional(&st.db)
        .await?;
    let (gid, uses, max, expires) = row.ok_or(AppError::NotFound)?;
    if max.is_some_and(|m| uses >= m) || expires.is_some_and(|e| e < Utc::now()) {
        return Err(AppError::NotFound);
    }
    Ok(gid)
}

pub async fn preview(State(st): State<AppState>, AuthUser(_): AuthUser, Path(code): Path<String>) -> Result<Json<Value>> {
    let gid = valid(&st, &code).await?;
    let (name, icon, members): (String, Option<String>, i64) = sqlx::query_as(
        "SELECT name, icon, (SELECT count(*) FROM members WHERE guild_id = g.id) FROM guilds g WHERE id = $1",
    )
    .bind(gid)
    .fetch_one(&st.db)
    .await?;
    Ok(Json(json!({ "guild_id": gid, "name": name, "icon": icon, "members": members })))
}

pub async fn join(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(code): Path<String>) -> Result<Json<Value>> {
    let gid = valid(&st, &code).await?;
    let banned: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bans WHERE guild_id = $1 AND user_id = $2)").bind(gid).bind(uid).fetch_one(&st.db).await?;
    if banned {
        return bad("You are banned from this server");
    }
    let mut tx = st.db.begin().await?;
    let added = sqlx::query("INSERT INTO members (guild_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING").bind(gid).bind(uid).execute(&mut *tx).await?;
    if added.rows_affected() > 0 {
        let used = sqlx::query("UPDATE invites SET uses = uses + 1 WHERE code = $1 AND (max_uses IS NULL OR uses < max_uses)").bind(&code).execute(&mut *tx).await?;
        if used.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }
    }
    tx.commit().await?;
    refresh(&st, gid).await?;
    Ok(Json(snapshot(&st, gid, uid).await?))
}
