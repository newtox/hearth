use crate::{
    auth::{bearer, create_session, hash_password, hash_token, verify_password, AuthUser},
    error::{bad, AppError, Result},
    limit::ClientIp,
    AppState,
};
use axum::{extract::State, http::request::Parts, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct AccountBody {
    password: String,
    username: Option<String>,
    email: Option<String>,
    new_password: Option<String>,
}

pub async fn update(State(st): State<AppState>, AuthUser(uid): AuthUser, parts: Parts, Json(b): Json<AccountBody>) -> Result<Json<Value>> {
    st.limits.check(format!("account:{uid}"), 10, Duration::from_secs(600))?;
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1").bind(uid).fetch_one(&st.db).await?;
    if !verify_password(b.password, hash).await {
        return bad("Wrong password");
    }
    if let Some(name) = &b.username {
        let name = name.trim().to_lowercase();
        if !(2..=32).contains(&name.len()) || !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.') {
            return bad("Username: 2–32 characters, only a–z, 0–9, dot and underscore");
        }
        sqlx::query("UPDATE users SET username = $1 WHERE id = $2").bind(&name).bind(uid).execute(&st.db).await?;
    }
    if let Some(email) = &b.email {
        let email = email.trim().to_lowercase();
        if email.len() > 254 || !email.contains('@') || email.starts_with('@') || email.ends_with('@') {
            return bad("Invalid email address");
        }
        sqlx::query("UPDATE users SET email = $1 WHERE id = $2").bind(&email).bind(uid).execute(&st.db).await?;
    }
    if let Some(new) = b.new_password {
        if new.len() < 8 || new.len() > 128 {
            return bad("Password: 8–128 characters");
        }
        let hash = hash_password(new).await?;
        sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2").bind(hash).bind(uid).execute(&st.db).await?;
        let keep = bearer(&parts).map(hash_token).unwrap_or_default();
        sqlx::query("DELETE FROM sessions WHERE user_id = $1 AND token_hash <> $2").bind(uid).bind(keep).execute(&st.db).await?;
    }
    super::users::announce(&st, uid).await?;
    Ok(Json(super::users::me_json(&st, uid).await?))
}

pub async fn config(State(st): State<AppState>) -> Json<Value> {
    Json(json!({ "password_reset": st.mailer.is_some(), "voice": st.livekit.is_some() }))
}

#[derive(Deserialize)]
pub struct ForgotBody {
    login: String,
    lang: Option<String>,
}

fn reset_mail(lang: &str, name: &str, link: &str) -> (&'static str, String) {
    match lang {
        "de" => (
            "Passwort zurücksetzen",
            format!("Hallo {name},\n\nüber diesen Link setzt du ein neues Passwort. Er gilt eine Stunde lang:\n\n{link}\n\nWenn du das nicht angefordert hast, kannst du diese Mail ignorieren. Dein Passwort bleibt dann unverändert.\n"),
        ),
        "da" => (
            "Nulstil adgangskode",
            format!("Hej {name},\n\nvia dette link vælger du en ny adgangskode. Det gælder i en time:\n\n{link}\n\nHvis du ikke har bedt om det, kan du ignorere denne mail. Din adgangskode forbliver uændret.\n"),
        ),
        _ => (
            "Reset your password",
            format!("Hi {name},\n\nuse this link to choose a new password. It is valid for one hour:\n\n{link}\n\nIf you did not ask for this, you can ignore this email and your password stays the same.\n"),
        ),
    }
}

pub async fn forgot(State(st): State<AppState>, ClientIp(ip): ClientIp, Json(b): Json<ForgotBody>) -> Result<Json<Value>> {
    let Some(mailer) = st.mailer.clone() else {
        return Err(AppError::Unavailable("Password reset by email is not set up on this server".into()));
    };
    st.limits.check(format!("forgot:{ip}"), 5, Duration::from_secs(3600))?;
    let login = b.login.trim().to_lowercase();
    let user: Option<(Uuid, String, String)> = sqlx::query_as("SELECT id, email, display_name FROM users WHERE username = $1 OR email = $1").bind(&login).fetch_optional(&st.db).await?;
    if let Some((uid, email, name)) = user {
        let token = hex::encode(rand::random::<[u8; 32]>());
        sqlx::query("DELETE FROM password_resets WHERE user_id = $1 OR expires_at < now()").bind(uid).execute(&st.db).await?;
        sqlx::query("INSERT INTO password_resets (token_hash, user_id, expires_at) VALUES ($1, $2, now() + interval '1 hour')")
            .bind(hash_token(&token))
            .bind(uid)
            .execute(&st.db)
            .await?;
        let base = std::env::var("PUBLIC_URL").unwrap_or_default();
        let link = format!("{}/#/reset/{token}", base.trim_end_matches('/'));
        let (subject, body) = reset_mail(b.lang.as_deref().unwrap_or("en"), &name, &link);
        tokio::spawn(async move {
            mailer.send(&email, subject, body).await;
        });
    }
    Ok(Json(json!({})))
}

#[derive(Deserialize)]
pub struct ResetBody {
    token: String,
    password: String,
}

pub async fn reset(State(st): State<AppState>, ClientIp(ip): ClientIp, Json(b): Json<ResetBody>) -> Result<Json<Value>> {
    st.limits.check(format!("reset:{ip}"), 10, Duration::from_secs(3600))?;
    if b.password.len() < 8 || b.password.len() > 128 {
        return bad("Password: 8–128 characters");
    }
    let uid: Option<Uuid> = sqlx::query_scalar("DELETE FROM password_resets WHERE token_hash = $1 AND expires_at > now() RETURNING user_id")
        .bind(hash_token(b.token.trim()))
        .fetch_optional(&st.db)
        .await?;
    let Some(uid) = uid else { return bad("This link is invalid or has expired") };
    let hash = hash_password(b.password).await?;
    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2").bind(hash).bind(uid).execute(&st.db).await?;
    sqlx::query("DELETE FROM sessions WHERE user_id = $1").bind(uid).execute(&st.db).await?;
    let token = create_session(&st.db, uid).await?;
    Ok(Json(json!({ "token": token })))
}
