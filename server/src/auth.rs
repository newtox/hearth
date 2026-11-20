use crate::{error::{bad, AppError, Result}, limit::ClientIp, AppState};
use std::time::Duration;
use argon2::{password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString}, Argon2};
use axum::{extract::{FromRequestParts, State}, http::{header::AUTHORIZATION, request::Parts}, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

pub struct AuthUser(pub Uuid);

pub struct AuthSession {
    pub user: Uuid,
    pub session: Uuid,
}

pub struct Device {
    pub user_agent: String,
    pub client: String,
    pub ip: String,
}

impl FromRequestParts<AppState> for Device {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, st: &AppState) -> Result<Self> {
        let ClientIp(ip) = ClientIp::from_request_parts(parts, st).await?;
        let header = |name: &str| parts.headers.get(name).and_then(|v| v.to_str().ok()).unwrap_or_default().to_string();
        let user_agent: String = header("user-agent").chars().take(300).collect();
        let client = match header("x-hearth-client").as_str() {
            c @ ("web" | "desktop" | "android") => c.to_string(),
            _ => String::new(),
        };
        Ok(Device { user_agent, client, ip })
    }
}

pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

pub async fn session_from_token(db: &PgPool, token: &str, device: Option<&Device>) -> Option<(Uuid, Uuid)> {
    let hash = hash_token(token);
    let row: Option<(Uuid, Uuid, bool)> = sqlx::query_as("SELECT user_id, id, last_seen < now() - interval '2 minutes' OR (user_agent = '' AND $2) FROM sessions WHERE token_hash = $1")
        .bind(&hash)
        .bind(device.is_some_and(|d| !d.user_agent.is_empty()))
        .fetch_optional(db)
        .await
        .ok()
        .flatten();
    let (user, session, stale) = row?;
    if stale {
        let (agent, client, ip) = device.map(|d| (d.user_agent.as_str(), d.client.as_str(), d.ip.as_str())).unwrap_or_default();
        let _ = sqlx::query(
            "UPDATE sessions SET last_seen = now(), user_agent = CASE WHEN $2 <> '' THEN $2 ELSE user_agent END, client = CASE WHEN $3 <> '' THEN $3 ELSE client END, ip = CASE WHEN $4 <> '' THEN $4 ELSE ip END WHERE token_hash = $1",
        )
        .bind(&hash)
        .bind(agent)
        .bind(client)
        .bind(ip)
        .execute(db)
        .await;
    }
    Some((user, session))
}

pub fn bearer(parts: &Parts) -> Option<&str> {
    parts.headers.get(AUTHORIZATION)?.to_str().ok()?.strip_prefix("Bearer ")
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, st: &AppState) -> Result<Self> {
        let token = bearer(parts).ok_or(AppError::Unauthorized)?.to_string();
        let device = Device::from_request_parts(parts, st).await.ok();
        session_from_token(&st.db, &token, device.as_ref()).await.map(|(user, _)| AuthUser(user)).ok_or(AppError::Unauthorized)
    }
}

impl FromRequestParts<AppState> for AuthSession {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, st: &AppState) -> Result<Self> {
        let token = bearer(parts).ok_or(AppError::Unauthorized)?.to_string();
        let device = Device::from_request_parts(parts, st).await.ok();
        let (user, session) = session_from_token(&st.db, &token, device.as_ref()).await.ok_or(AppError::Unauthorized)?;
        Ok(AuthSession { user, session })
    }
}

pub async fn hash_password(password: String) -> Result<String> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::encode_b64(&rand::random::<[u8; 16]>()).map_err(|e| AppError::Internal(e.to_string()))?;
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| AppError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

pub async fn verify_password(password: String, hash: String) -> bool {
    tokio::task::spawn_blocking(move || {
        PasswordHash::new(&hash).map(|h| Argon2::default().verify_password(password.as_bytes(), &h).is_ok()).unwrap_or(false)
    })
    .await
    .unwrap_or(false)
}

pub async fn create_session(db: &PgPool, user_id: Uuid, device: &Device) -> Result<String> {
    let token = hex::encode(rand::random::<[u8; 32]>());
    sqlx::query("INSERT INTO sessions (token_hash, user_id, user_agent, client, ip) VALUES ($1, $2, $3, $4, $5)")
        .bind(hash_token(&token))
        .bind(user_id)
        .bind(&device.user_agent)
        .bind(&device.client)
        .bind(&device.ip)
        .execute(db)
        .await?;
    Ok(token)
}

pub async fn sign_in(st: &AppState, user_id: Uuid, device: &Device) -> Result<Json<Value>> {
    let two_factor: bool = sqlx::query_scalar("SELECT totp_secret IS NOT NULL FROM users WHERE id = $1").bind(user_id).fetch_one(&st.db).await?;
    if !two_factor {
        let token = create_session(&st.db, user_id, device).await?;
        return Ok(Json(json!({ "token": token })));
    }
    let ticket = hex::encode(rand::random::<[u8; 32]>());
    sqlx::query("DELETE FROM login_tickets WHERE expires_at < now()").execute(&st.db).await?;
    sqlx::query("INSERT INTO login_tickets (ticket_hash, user_id, expires_at) VALUES ($1, $2, now() + interval '10 minutes')")
        .bind(hash_token(&ticket))
        .bind(user_id)
        .execute(&st.db)
        .await?;
    Ok(Json(json!({ "two_factor": true, "ticket": ticket })))
}

pub async fn check_second_factor(st: &AppState, user_id: Uuid, code: &str) -> Result<bool> {
    let (secret, last): (Option<String>, i64) = sqlx::query_as("SELECT totp_secret, totp_last_step FROM users WHERE id = $1").bind(user_id).fetch_one(&st.db).await?;
    let Some(secret) = secret else { return Ok(false) };
    if let Some(step) = crate::totp::matching_step(&secret, code, last) {
        sqlx::query("UPDATE users SET totp_last_step = $1 WHERE id = $2").bind(step).bind(user_id).execute(&st.db).await?;
        return Ok(true);
    }
    let normalized: String = code.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
    if normalized.len() != 10 {
        return Ok(false);
    }
    let used = sqlx::query("DELETE FROM recovery_codes WHERE user_id = $1 AND code_hash = $2")
        .bind(user_id)
        .bind(hash_token(&normalized))
        .execute(&st.db)
        .await?;
    Ok(used.rows_affected() == 1)
}

#[derive(Deserialize)]
pub struct SecondFactor {
    ticket: String,
    code: String,
}

pub async fn login_second_factor(State(st): State<AppState>, device: Device, Json(b): Json<SecondFactor>) -> Result<Json<Value>> {
    st.limits.check(format!("login2fa:{}", device.ip), 20, Duration::from_secs(300))?;
    let hash = hash_token(b.ticket.trim());
    let row: Option<(Uuid, i32)> = sqlx::query_as("UPDATE login_tickets SET attempts = attempts + 1 WHERE ticket_hash = $1 AND expires_at > now() RETURNING user_id, attempts")
        .bind(&hash)
        .fetch_optional(&st.db)
        .await?;
    let Some((user_id, attempts)) = row else { return bad("This sign-in has expired, please start again") };
    if attempts > 5 {
        sqlx::query("DELETE FROM login_tickets WHERE ticket_hash = $1").bind(&hash).execute(&st.db).await?;
        return bad("This sign-in has expired, please start again");
    }
    if !check_second_factor(&st, user_id, &b.code).await? {
        return bad("Wrong code");
    }
    sqlx::query("DELETE FROM login_tickets WHERE ticket_hash = $1").bind(&hash).execute(&st.db).await?;
    let token = create_session(&st.db, user_id, &device).await?;
    Ok(Json(json!({ "token": token })))
}

pub fn clean_username(raw: &str) -> Option<String> {
    let name = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let ok = (2..=32).contains(&name.chars().count()) && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '.' | '_' | '-'));
    ok.then_some(name)
}

#[derive(Deserialize)]
pub struct Register {
    username: String,
    email: String,
    password: String,
    lang: Option<String>,
}

pub async fn register(State(st): State<AppState>, device: Device, Json(b): Json<Register>) -> Result<Json<Value>> {
    let ip = device.ip.clone();
    st.limits.check(format!("register:{ip}"), 20, Duration::from_secs(3600))?;
    let Some(username) = clean_username(&b.username) else {
        return bad("Username: 2–32 characters, only letters, digits, spaces, dot, dash and underscore");
    };
    let email = b.email.trim().to_lowercase();
    if email.len() > 254 || !email.contains('@') || email.starts_with('@') || email.ends_with('@') {
        return bad("Invalid email address");
    }
    if b.password.len() < 8 || b.password.len() > 128 {
        return bad("Password: 8–128 characters");
    }
    let taken: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE lower(username) = lower($1) OR email = $2)")
        .bind(&username)
        .bind(&email)
        .fetch_one(&st.db)
        .await?;
    if taken {
        return Err(AppError::Conflict("Username or email is already taken".into()));
    }
    let hash = hash_password(b.password).await?;
    let id = Uuid::now_v7();
    let first: bool = sqlx::query_scalar(
        "INSERT INTO users (id, username, email, password_hash, display_name, is_admin) VALUES ($1, $2, $3, $4, $2, NOT EXISTS(SELECT 1 FROM users)) RETURNING is_admin",
    )
    .bind(id)
    .bind(&username)
    .bind(&email)
    .bind(hash)
    .fetch_one(&st.db)
    .await?;
    if st.mailer.is_none() {
        sqlx::query("UPDATE users SET email_verified = true WHERE id = $1").bind(id).execute(&st.db).await?;
    } else {
        crate::routes::account::send_verification(&st, id, b.lang.as_deref().unwrap_or("en")).await?;
    }
    if first {
        sqlx::query("INSERT INTO user_badges (user_id, badge_id) SELECT $1, id FROM badges WHERE id = '00000000-0000-7000-8000-000000000002'").bind(id).execute(&st.db).await?;
    }
    let token = create_session(&st.db, id, &device).await?;
    Ok(Json(json!({ "token": token })))
}

#[derive(Deserialize)]
pub struct Login {
    login: String,
    password: String,
}

pub async fn login(State(st): State<AppState>, device: Device, Json(b): Json<Login>) -> Result<Json<Value>> {
    let ip = device.ip.clone();
    st.limits.check(format!("login:{ip}"), 10, Duration::from_secs(60))?;
    let login = b.login.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    let row: Option<(Uuid, String)> = sqlx::query_as("SELECT id, password_hash FROM users WHERE lower(username) = $1 OR email = $1")
        .bind(&login)
        .fetch_optional(&st.db)
        .await?;
    let (id, hash) = match row {
        Some(r) => r,
        None => {
            let _ = hash_password(b.password).await;
            return bad("Wrong username or password");
        }
    };
    if !verify_password(b.password, hash).await {
        return bad("Wrong username or password");
    }
    sign_in(&st, id, &device).await
}

pub async fn logout(State(st): State<AppState>, parts: Parts) -> Result<Json<Value>> {
    if let Some(token) = bearer(&parts) {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1").bind(hash_token(token)).execute(&st.db).await?;
    }
    Ok(Json(json!({})))
}
