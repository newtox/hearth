use super::users::decorate;
use crate::{auth::AuthUser, error::{bad, AppError, Result}, models::{PublicUser, USER_COLS}, AppState};
use axum::{extract::{Path, State}, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

pub async fn are_friends(st: &AppState, a: Uuid, b: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM friendships WHERE status = 'accepted' AND ((user_id = $1 AND friend_id = $2) OR (user_id = $2 AND friend_id = $1)))",
    )
    .bind(a)
    .bind(b)
    .fetch_one(&st.db)
    .await?)
}

pub async fn blocked_between(st: &AppState, a: Uuid, b: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM blocks WHERE (user_id = $1 AND blocked_id = $2) OR (user_id = $2 AND blocked_id = $1))")
        .bind(a)
        .bind(b)
        .fetch_one(&st.db)
        .await?)
}

async fn users(st: &AppState, filter: &str, uid: Uuid) -> Result<Vec<PublicUser>> {
    let mut list: Vec<PublicUser> = sqlx::query_as(&format!("SELECT {USER_COLS} FROM users WHERE id IN ({filter}) ORDER BY username")).bind(uid).fetch_all(&st.db).await?;
    for u in &mut list {
        decorate(st, u);
    }
    Ok(list)
}

pub async fn list_json(st: &AppState, uid: Uuid) -> Result<Value> {
    Ok(json!({
        "friends": users(st, "SELECT CASE WHEN user_id = $1 THEN friend_id ELSE user_id END FROM friendships WHERE status = 'accepted' AND (user_id = $1 OR friend_id = $1)", uid).await?,
        "incoming": users(st, "SELECT user_id FROM friendships WHERE friend_id = $1 AND status = 'pending'", uid).await?,
        "outgoing": users(st, "SELECT friend_id FROM friendships WHERE user_id = $1 AND status = 'pending'", uid).await?,
        "blocked": users(st, "SELECT blocked_id FROM blocks WHERE user_id = $1", uid).await?,
    }))
}

fn notify(st: &AppState, a: Uuid, b: Uuid) {
    st.hub.send(&[a, b], "FRIENDS_UPDATE", json!({}));
}

pub async fn list(State(st): State<AppState>, AuthUser(uid): AuthUser) -> Result<Json<Value>> {
    Ok(Json(list_json(&st, uid).await?))
}

#[derive(Deserialize)]
pub struct AddBody {
    username: Option<String>,
    user_id: Option<Uuid>,
}

pub async fn add(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(b): Json<AddBody>) -> Result<Json<Value>> {
    crate::routes::account::require_verified(&st, uid).await?;
    st.limits.check(format!("friend:{uid}"), 20, Duration::from_secs(60))?;
    let target: Option<Uuid> = match (b.user_id, b.username) {
        (Some(id), _) => sqlx::query_scalar("SELECT id FROM users WHERE id = $1").bind(id).fetch_optional(&st.db).await?,
        (None, Some(name)) => sqlx::query_scalar("SELECT id FROM users WHERE lower(username) = $1").bind(name.trim().trim_start_matches('@').split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()).fetch_optional(&st.db).await?,
        _ => None,
    };
    let Some(target) = target else {
        return bad("No user with that name");
    };
    if target == uid {
        return bad("You cannot add yourself");
    }
    if blocked_between(&st, uid, target).await? {
        return bad("You cannot add this user");
    }
    let existing: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT user_id, status FROM friendships WHERE (user_id = $1 AND friend_id = $2) OR (user_id = $2 AND friend_id = $1)",
    )
    .bind(uid)
    .bind(target)
    .fetch_optional(&st.db)
    .await?;
    match existing {
        Some((_, status)) if status == "accepted" => return bad("You are already friends"),
        Some((from, _)) if from == uid => return bad("Request already sent"),
        Some(_) => {
            sqlx::query("UPDATE friendships SET status = 'accepted' WHERE user_id = $1 AND friend_id = $2").bind(target).bind(uid).execute(&st.db).await?;
        }
        None => {
            sqlx::query("INSERT INTO friendships (user_id, friend_id, status) VALUES ($1, $2, 'pending')").bind(uid).bind(target).execute(&st.db).await?;
        }
    }
    notify(&st, uid, target);
    Ok(Json(json!({})))
}

pub async fn accept(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(target): Path<Uuid>) -> Result<Json<Value>> {
    let done = sqlx::query("UPDATE friendships SET status = 'accepted' WHERE user_id = $1 AND friend_id = $2 AND status = 'pending'")
        .bind(target)
        .bind(uid)
        .execute(&st.db)
        .await?;
    if done.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    notify(&st, uid, target);
    Ok(Json(json!({})))
}

pub async fn remove(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(target): Path<Uuid>) -> Result<Json<Value>> {
    sqlx::query("DELETE FROM friendships WHERE (user_id = $1 AND friend_id = $2) OR (user_id = $2 AND friend_id = $1)")
        .bind(uid)
        .bind(target)
        .execute(&st.db)
        .await?;
    notify(&st, uid, target);
    Ok(Json(json!({})))
}

pub async fn block(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(target): Path<Uuid>) -> Result<Json<Value>> {
    if target == uid {
        return bad("You cannot block yourself");
    }
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)").bind(target).fetch_one(&st.db).await?;
    if !exists {
        return Err(AppError::NotFound);
    }
    sqlx::query("INSERT INTO blocks (user_id, blocked_id) VALUES ($1, $2) ON CONFLICT DO NOTHING").bind(uid).bind(target).execute(&st.db).await?;
    let ended = sqlx::query("DELETE FROM friendships WHERE (user_id = $1 AND friend_id = $2) OR (user_id = $2 AND friend_id = $1)")
        .bind(uid)
        .bind(target)
        .execute(&st.db)
        .await?;
    if ended.rows_affected() > 0 {
        st.hub.send(&[target], "FRIENDS_UPDATE", json!({}));
    }
    st.hub.send(&[uid], "FRIENDS_UPDATE", json!({}));
    Ok(Json(json!({})))
}

pub async fn unblock(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(target): Path<Uuid>) -> Result<Json<Value>> {
    sqlx::query("DELETE FROM blocks WHERE user_id = $1 AND blocked_id = $2").bind(uid).bind(target).execute(&st.db).await?;
    st.hub.send(&[uid], "FRIENDS_UPDATE", json!({}));
    Ok(Json(json!({})))
}
