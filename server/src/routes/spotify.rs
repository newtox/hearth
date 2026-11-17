use super::stats::{broadcast, flush};
use crate::{auth::AuthUser, error::{AppError, Result}, models::Activity, AppState};
use axum::{extract::{Query, State}, response::Redirect, Json};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{env, time::Instant};
use uuid::Uuid;

pub struct Spotify {
    client_id: String,
    secret: String,
    redirect: String,
    accounts: String,
    api: String,
    poll_secs: u64,
    http: reqwest::Client,
}

impl Spotify {
    pub fn from_env() -> Option<Self> {
        let client_id = env::var("SPOTIFY_CLIENT_ID").ok().filter(|v| !v.is_empty())?;
        let secret = env::var("SPOTIFY_CLIENT_SECRET").ok().filter(|v| !v.is_empty())?;
        let public = env::var("PUBLIC_URL").ok().filter(|v| !v.is_empty())?;
        Some(Self {
            client_id,
            secret,
            redirect: format!("{}/api/spotify/callback", public.trim_end_matches('/')),
            accounts: env::var("SPOTIFY_ACCOUNTS_URL").unwrap_or_else(|_| "https://accounts.spotify.com".into()),
            api: env::var("SPOTIFY_API_URL").unwrap_or_else(|_| "https://api.spotify.com".into()),
            poll_secs: env::var("SPOTIFY_POLL_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(15),
            http: reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build().ok()?,
        })
    }
}

fn config(st: &AppState) -> Result<&Spotify> {
    st.spotify.as_deref().ok_or_else(|| AppError::Unavailable("Spotify is not set up on this server".into()))
}

#[derive(Deserialize)]
struct Token {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
}

async fn token(sp: &Spotify, form: &[(&str, &str)]) -> Option<Token> {
    let res = sp
        .http
        .post(format!("{}/api/token", sp.accounts))
        .basic_auth(&sp.client_id, Some(&sp.secret))
        .form(form)
        .send()
        .await
        .ok()?;
    if !res.status().is_success() {
        return None;
    }
    res.json().await.ok()
}

pub async fn login(State(st): State<AppState>, AuthUser(uid): AuthUser) -> Result<Json<Value>> {
    let sp = config(&st)?;
    let state = hex::encode(rand::random::<[u8; 16]>());
    sqlx::query("DELETE FROM spotify_states WHERE created_at < now() - interval '10 minutes'").execute(&st.db).await?;
    sqlx::query("INSERT INTO spotify_states (state, user_id) VALUES ($1, $2)").bind(&state).bind(uid).execute(&st.db).await?;
    let url = reqwest::Url::parse_with_params(
        &format!("{}/authorize", sp.accounts),
        &[
            ("response_type", "code"),
            ("client_id", sp.client_id.as_str()),
            ("scope", "user-read-currently-playing"),
            ("redirect_uri", sp.redirect.as_str()),
            ("state", state.as_str()),
        ],
    )
    .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(json!({ "url": url.to_string() })))
}

#[derive(Deserialize)]
pub struct Callback {
    code: Option<String>,
    state: Option<String>,
}

pub async fn callback(State(st): State<AppState>, Query(q): Query<Callback>) -> Redirect {
    let failed = Redirect::to("/?spotify=error");
    let (Some(sp), Some(code), Some(state)) = (st.spotify.as_deref(), q.code, q.state) else { return failed };
    let user: Option<Uuid> = sqlx::query_scalar("DELETE FROM spotify_states WHERE state = $1 AND created_at > now() - interval '10 minutes' RETURNING user_id")
        .bind(state)
        .fetch_optional(&st.db)
        .await
        .ok()
        .flatten();
    let Some(uid) = user else { return failed };
    let Some(t) = token(sp, &[("grant_type", "authorization_code"), ("code", &code), ("redirect_uri", &sp.redirect)]).await else { return failed };
    let Some(refresh) = t.refresh_token else { return failed };
    let saved = sqlx::query(
        "INSERT INTO spotify_links (user_id, refresh_token, access_token, expires_at) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (user_id) DO UPDATE SET refresh_token = EXCLUDED.refresh_token, access_token = EXCLUDED.access_token, expires_at = EXCLUDED.expires_at",
    )
    .bind(uid)
    .bind(refresh)
    .bind(&t.access_token)
    .bind(Utc::now() + Duration::seconds(t.expires_in))
    .execute(&st.db)
    .await;
    let access = t.access_token.clone();
    if saved.is_err() {
        return failed;
    }
    if let Some((id, name, url)) = me(sp, &access).await {
        let _ = super::connections::save(&st, uid, "spotify", &id, &name, url.as_deref()).await;
        let _ = super::users::announce(&st, uid).await;
    }
    Redirect::to("/?spotify=linked")
}

async fn me(sp: &Spotify, access: &str) -> Option<(String, String, Option<String>)> {
    let v: Value = sp.http.get(format!("{}/v1/me", sp.api)).bearer_auth(access).send().await.ok()?.json().await.ok()?;
    let id = v["id"].as_str()?.to_string();
    let name = v["display_name"].as_str().unwrap_or(&id).to_string();
    Some((id, name, v["external_urls"]["spotify"].as_str().map(str::to_string)))
}

async fn clear(st: &AppState, uid: Uuid) {
    if st.hub.activity.remove_if(&uid, |_, a| a.source == "spotify").is_some() {
        broadcast(st, uid).await;
    }
}

pub async fn unlink(State(st): State<AppState>, AuthUser(uid): AuthUser) -> Result<Json<Value>> {
    sqlx::query("DELETE FROM spotify_links WHERE user_id = $1").bind(uid).execute(&st.db).await?;
    sqlx::query("DELETE FROM connections WHERE user_id = $1 AND provider = 'spotify'").bind(uid).execute(&st.db).await?;
    super::users::announce(&st, uid).await?;
    clear(&st, uid).await;
    Ok(Json(json!({})))
}

async fn now_playing(st: &AppState, sp: &Spotify, uid: Uuid, mut access: String, refresh: String, expires: DateTime<Utc>) -> Option<(String, String)> {
    if expires < Utc::now() + Duration::seconds(60) {
        let t = token(sp, &[("grant_type", "refresh_token"), ("refresh_token", &refresh)]).await?;
        access = t.access_token;
        let _ = sqlx::query("UPDATE spotify_links SET access_token = $1, refresh_token = COALESCE($2, refresh_token), expires_at = $3 WHERE user_id = $4")
            .bind(&access)
            .bind(t.refresh_token)
            .bind(Utc::now() + Duration::seconds(t.expires_in))
            .bind(uid)
            .execute(&st.db)
            .await;
    }
    let res = sp.http.get(format!("{}/v1/me/player/currently-playing", sp.api)).bearer_auth(access).send().await.ok()?;
    if res.status() != reqwest::StatusCode::OK {
        return None;
    }
    let v: Value = res.json().await.ok()?;
    if v["is_playing"] != true {
        return None;
    }
    let track = v["item"]["name"].as_str()?.chars().take(64).collect::<String>();
    let artists: Vec<&str> = v["item"]["artists"].as_array().map(|a| a.iter().filter_map(|x| x["name"].as_str()).collect()).unwrap_or_default();
    Some((track, artists.join(", ").chars().take(64).collect()))
}

pub async fn poll(st: AppState) {
    let Some(sp) = st.spotify.clone() else { return };
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(sp.poll_secs)).await;
        let rows: Vec<(Uuid, String, String, DateTime<Utc>)> =
            sqlx::query_as("SELECT user_id, access_token, refresh_token, expires_at FROM spotify_links WHERE user_id = ANY($1)")
                .bind(st.hub.online_ids())
                .fetch_all(&st.db)
                .await
                .unwrap_or_default();
        for (uid, access, refresh, expires) in rows {
            let taken = st.hub.activity.get(&uid).is_some_and(|a| a.source != "spotify");
            if taken {
                continue;
            }
            match now_playing(&st, &sp, uid, access, refresh, expires).await {
                Some((track, artists)) => {
                    let same = st.hub.activity.get(&uid).is_some_and(|a| a.name == track && a.details == artists);
                    if !same && st.hub.is_online(&uid) {
                        flush(&st, uid).await;
                        st.hub.activity.insert(uid, Activity { kind: "listening".into(), name: track, details: artists, since: Utc::now(), tick: Instant::now(), source: "spotify" });
                        broadcast(&st, uid).await;
                    }
                }
                None => clear(&st, uid).await,
            }
        }
    }
}
