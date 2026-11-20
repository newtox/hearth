use crate::{
    auth::{check_second_factor, hash_token, verify_password, AuthSession, AuthUser},
    error::{bad, Result},
    totp, AppState,
};
use axum::{extract::{Path, State}, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

#[derive(Serialize, sqlx::FromRow)]
pub struct SessionInfo {
    id: Uuid,
    created_at: DateTime<Utc>,
    last_seen: DateTime<Utc>,
    user_agent: String,
    client: String,
    ip: String,
    #[sqlx(skip)]
    current: bool,
}

pub async fn list_sessions(State(st): State<AppState>, auth: AuthSession) -> Result<Json<Vec<SessionInfo>>> {
    let mut list: Vec<SessionInfo> = sqlx::query_as("SELECT id, created_at, last_seen, user_agent, client, ip FROM sessions WHERE user_id = $1 ORDER BY last_seen DESC")
        .bind(auth.user)
        .fetch_all(&st.db)
        .await?;
    for s in &mut list {
        s.current = s.id == auth.session;
    }
    list.sort_by_key(|s| !s.current);
    Ok(Json(list))
}

pub async fn end_session(State(st): State<AppState>, auth: AuthSession, Path(id): Path<Uuid>) -> Result<Json<Value>> {
    if id == auth.session {
        return bad("Use sign out to end this session");
    }
    let done = sqlx::query("DELETE FROM sessions WHERE id = $1 AND user_id = $2").bind(id).bind(auth.user).execute(&st.db).await?;
    if done.rows_affected() == 0 {
        return bad("This session no longer exists");
    }
    st.hub.end_sessions(auth.user, None, Some(id));
    Ok(Json(json!({})))
}

pub async fn end_other_sessions(State(st): State<AppState>, auth: AuthSession) -> Result<Json<Value>> {
    sqlx::query("DELETE FROM sessions WHERE user_id = $1 AND id <> $2").bind(auth.user).bind(auth.session).execute(&st.db).await?;
    st.hub.end_sessions(auth.user, Some(auth.session), None);
    Ok(Json(json!({})))
}

async fn require_password(st: &AppState, uid: Uuid, password: String) -> Result<()> {
    st.limits.check(format!("security:{uid}"), 10, Duration::from_secs(600))?;
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1").bind(uid).fetch_one(&st.db).await?;
    if !verify_password(password, hash).await {
        return bad("Wrong password");
    }
    Ok(())
}

async fn new_recovery_codes(st: &AppState, uid: Uuid) -> Result<Vec<String>> {
    const CHARS: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";
    sqlx::query("DELETE FROM recovery_codes WHERE user_id = $1").bind(uid).execute(&st.db).await?;
    let mut codes = Vec::with_capacity(10);
    for _ in 0..10 {
        let raw: String = (0..10).map(|_| CHARS[rand::random_range(0..CHARS.len())] as char).collect();
        sqlx::query("INSERT INTO recovery_codes (user_id, code_hash) VALUES ($1, $2)").bind(uid).bind(hash_token(&raw)).execute(&st.db).await?;
        codes.push(format!("{}-{}", &raw[..5], &raw[5..]));
    }
    Ok(codes)
}

#[derive(Deserialize)]
pub struct PasswordBody {
    password: String,
}

pub async fn two_factor_setup(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(b): Json<PasswordBody>) -> Result<Json<Value>> {
    require_password(&st, uid, b.password).await?;
    let (active, username): (bool, String) = sqlx::query_as("SELECT totp_secret IS NOT NULL, username FROM users WHERE id = $1").bind(uid).fetch_one(&st.db).await?;
    if active {
        return bad("Two-factor authentication is already on");
    }
    let secret = totp::new_secret();
    sqlx::query("UPDATE users SET totp_pending = $1 WHERE id = $2").bind(&secret).bind(uid).execute(&st.db).await?;
    let issuer = std::env::var("PUBLIC_URL")
        .ok()
        .and_then(|u| u.split("://").nth(1).map(|h| h.trim_end_matches('/').to_string()))
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "Hearth".into());
    Ok(Json(json!({ "secret": secret, "uri": totp::uri(&secret, &username, &issuer) })))
}

#[derive(Deserialize)]
pub struct CodeBody {
    code: String,
}

pub async fn two_factor_enable(State(st): State<AppState>, auth: AuthSession, Json(b): Json<CodeBody>) -> Result<Json<Value>> {
    let uid = auth.user;
    st.limits.check(format!("security:{uid}"), 10, Duration::from_secs(600))?;
    let pending: Option<String> = sqlx::query_scalar("SELECT totp_pending FROM users WHERE id = $1").bind(uid).fetch_one(&st.db).await?;
    let Some(secret) = pending else { return bad("Start the setup again") };
    let Some(step) = totp::matching_step(&secret, &b.code, 0) else { return bad("Wrong code") };
    sqlx::query("UPDATE users SET totp_secret = totp_pending, totp_pending = NULL, totp_last_step = $1 WHERE id = $2").bind(step).bind(uid).execute(&st.db).await?;
    let codes = new_recovery_codes(&st, uid).await?;
    sqlx::query("DELETE FROM sessions WHERE user_id = $1 AND id <> $2").bind(uid).bind(auth.session).execute(&st.db).await?;
    st.hub.end_sessions(uid, Some(auth.session), None);
    Ok(Json(json!({ "codes": codes })))
}

#[derive(Deserialize)]
pub struct ConfirmBody {
    password: String,
    code: String,
}

pub async fn two_factor_disable(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(b): Json<ConfirmBody>) -> Result<Json<Value>> {
    require_password(&st, uid, b.password).await?;
    if !check_second_factor(&st, uid, &b.code).await? {
        return bad("Wrong code");
    }
    sqlx::query("UPDATE users SET totp_secret = NULL, totp_pending = NULL, totp_last_step = 0 WHERE id = $1").bind(uid).execute(&st.db).await?;
    sqlx::query("DELETE FROM recovery_codes WHERE user_id = $1").bind(uid).execute(&st.db).await?;
    Ok(Json(json!({})))
}

pub async fn recovery_codes(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(b): Json<ConfirmBody>) -> Result<Json<Value>> {
    require_password(&st, uid, b.password).await?;
    if !check_second_factor(&st, uid, &b.code).await? {
        return bad("Wrong code");
    }
    Ok(Json(json!({ "codes": new_recovery_codes(&st, uid).await? })))
}

pub async fn recovery_count(State(st): State<AppState>, AuthUser(uid): AuthUser) -> Result<Json<Value>> {
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM recovery_codes WHERE user_id = $1").bind(uid).fetch_one(&st.db).await?;
    Ok(Json(json!({ "remaining": n })))
}
