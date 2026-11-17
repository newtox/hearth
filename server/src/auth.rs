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

pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

pub async fn user_from_token(db: &PgPool, token: &str) -> Option<Uuid> {
    sqlx::query_scalar("SELECT user_id FROM sessions WHERE token_hash = $1")
        .bind(hash_token(token))
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
}

pub fn bearer(parts: &Parts) -> Option<&str> {
    parts.headers.get(AUTHORIZATION)?.to_str().ok()?.strip_prefix("Bearer ")
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, st: &AppState) -> Result<Self> {
        let token = bearer(parts).ok_or(AppError::Unauthorized)?;
        user_from_token(&st.db, token).await.map(AuthUser).ok_or(AppError::Unauthorized)
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

pub async fn create_session(db: &PgPool, user_id: Uuid) -> Result<String> {
    let token = hex::encode(rand::random::<[u8; 32]>());
    sqlx::query("INSERT INTO sessions (token_hash, user_id) VALUES ($1, $2)")
        .bind(hash_token(&token))
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(token)
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

pub async fn register(State(st): State<AppState>, ClientIp(ip): ClientIp, Json(b): Json<Register>) -> Result<Json<Value>> {
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
    let token = create_session(&st.db, id).await?;
    Ok(Json(json!({ "token": token })))
}

#[derive(Deserialize)]
pub struct Login {
    login: String,
    password: String,
}

pub async fn login(State(st): State<AppState>, ClientIp(ip): ClientIp, Json(b): Json<Login>) -> Result<Json<Value>> {
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
    let token = create_session(&st.db, id).await?;
    Ok(Json(json!({ "token": token })))
}

pub async fn logout(State(st): State<AppState>, parts: Parts) -> Result<Json<Value>> {
    if let Some(token) = bearer(&parts) {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1").bind(hash_token(token)).execute(&st.db).await?;
    }
    Ok(Json(json!({})))
}
