use crate::{auth::AuthUser, error::{AppError, Result}, AppState};
use axum::{extract::{Path, Query, State}, response::Redirect, Json};
use serde_json::{json, Value};
use std::{collections::HashMap, env};
use uuid::Uuid;

struct Provider {
    id: &'static str,
    name: &'static str,
    authorize: String,
    token: String,
    userinfo: String,
    scope: &'static str,
    client_id: String,
    secret: String,
}

pub struct Connections {
    providers: Vec<Provider>,
    steam_key: Option<String>,
    public: String,
    http: reqwest::Client,
}

const STEAM_LOGIN: &str = "https://steamcommunity.com/openid/login";
const DEFS: [(&str, &str, &str, &str, &str, &str); 2] = [
    ("twitch", "Twitch", "https://id.twitch.tv/oauth2/authorize", "https://id.twitch.tv/oauth2/token", "https://api.twitch.tv/helix/users", ""),
    ("youtube", "YouTube", "https://accounts.google.com/o/oauth2/v2/auth", "https://oauth2.googleapis.com/token", "https://www.googleapis.com/youtube/v3/channels?part=snippet&mine=true", "https://www.googleapis.com/auth/youtube.readonly"),
];

fn var(name: &str) -> Option<String> {
    env::var(name).ok().filter(|v| !v.is_empty())
}

impl Connections {
    pub fn from_env() -> Option<Self> {
        let public = var("PUBLIC_URL")?.trim_end_matches('/').to_string();
        let mut providers = Vec::new();
        for (id, name, authorize, token, userinfo, scope) in DEFS {
            let key = id.to_uppercase();
            if let (Some(client_id), Some(secret)) = (var(&format!("CONNECT_{key}_ID")), var(&format!("CONNECT_{key}_SECRET"))) {
                providers.push(Provider { id, name, authorize: authorize.into(), token: token.into(), userinfo: userinfo.into(), scope, client_id, secret });
            }
        }
        if let Some(base) = var("CONNECT_MOCK_URL") {
            providers.push(Provider {
                id: "mock",
                name: "Mock",
                authorize: format!("{base}/oauth/authorize"),
                token: format!("{base}/oauth/token"),
                userinfo: format!("{base}/oauth/user"),
                scope: "profile",
                client_id: "mock-id".into(),
                secret: "mock-secret".into(),
            });
        }
        let http = reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).user_agent("Hearth/0.1").build().ok()?;
        Some(Self { providers, steam_key: var("STEAM_API_KEY"), public, http })
    }

    fn callback(&self, provider: &str) -> String {
        format!("{}/api/connections/{provider}/callback", self.public)
    }
}

fn config(st: &AppState) -> Result<&Connections> {
    st.connections.as_deref().ok_or_else(|| AppError::Unavailable("Account connections are not set up on this server".into()))
}

fn text(v: &Value) -> Option<String> {
    v.as_str().map(str::to_string).or_else(|| v.as_i64().map(|n| n.to_string()))
}

fn profile(provider: &str, v: &Value) -> Option<(String, String, Option<String>)> {
    Some(match provider {
        "twitch" => {
            let u = &v["data"][0];
            (text(&u["id"])?, text(&u["display_name"])?, text(&u["login"]).map(|l| format!("https://twitch.tv/{l}")))
        }
        "youtube" => {
            let c = &v["items"][0];
            let id = text(&c["id"])?;
            (id.clone(), text(&c["snippet"]["title"])?, Some(format!("https://youtube.com/channel/{id}")))
        }
        _ => (text(&v["id"])?, text(&v["name"])?, text(&v["url"])),
    })
}

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
    let mut available: Vec<Value> = Vec::new();
    if let Some(c) = st.connections.as_deref() {
        available.extend(c.providers.iter().map(|p| json!({ "id": p.id, "name": p.name })));
        if c.steam_key.is_some() {
            available.push(json!({ "id": "steam", "name": "Steam" }));
        }
    }
    if st.spotify.is_some() {
        available.push(json!({ "id": "spotify", "name": "Spotify" }));
    }
    Ok(Json(json!({ "available": available, "linked": of_user(&st, uid).await? })))
}

pub async fn start(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(provider): Path<String>) -> Result<Json<Value>> {
    let c = config(&st)?;
    let state = hex::encode(rand::random::<[u8; 16]>());
    let callback = c.callback(&provider);
    let url = if provider == "steam" && c.steam_key.is_some() {
        reqwest::Url::parse_with_params(
            STEAM_LOGIN,
            &[
                ("openid.ns", "http://specs.openid.net/auth/2.0"),
                ("openid.mode", "checkid_setup"),
                ("openid.return_to", &format!("{callback}?state={state}")),
                ("openid.realm", &c.public),
                ("openid.identity", "http://specs.openid.net/auth/2.0/identifier_select"),
                ("openid.claimed_id", "http://specs.openid.net/auth/2.0/identifier_select"),
            ],
        )
    } else {
        let p = c.providers.iter().find(|p| p.id == provider).ok_or(AppError::NotFound)?;
        reqwest::Url::parse_with_params(
            &p.authorize,
            &[("response_type", "code"), ("client_id", &p.client_id), ("scope", p.scope), ("redirect_uri", &callback), ("state", &state)],
        )
    }
    .map_err(|e| AppError::Internal(e.to_string()))?;
    sqlx::query("DELETE FROM oauth_states WHERE created_at < now() - interval '10 minutes'").execute(&st.db).await?;
    sqlx::query("INSERT INTO oauth_states (state, user_id, provider) VALUES ($1, $2, $3)").bind(&state).bind(uid).bind(&provider).execute(&st.db).await?;
    Ok(Json(json!({ "url": url.to_string() })))
}

async fn steam(c: &Connections, q: &HashMap<String, String>) -> Option<(String, String, Option<String>)> {
    let mut form: Vec<(&str, &str)> = q.iter().filter(|(k, _)| k.starts_with("openid.") && *k != "openid.mode").map(|(k, v)| (k.as_str(), v.as_str())).collect();
    form.push(("openid.mode", "check_authentication"));
    let body = c.http.post(STEAM_LOGIN).form(&form).send().await.ok()?.text().await.ok()?;
    if !body.lines().any(|l| l.trim() == "is_valid:true") {
        return None;
    }
    let id = q.get("openid.claimed_id")?.strip_prefix("https://steamcommunity.com/openid/id/")?.to_string();
    if id.is_empty() || !id.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let summary: Option<Value> = async {
        c.http
            .get("https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v2/")
            .query(&[("key", c.steam_key.as_deref()?), ("steamids", id.as_str())])
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()
    }
    .await;
    let name = summary.and_then(|v| text(&v["response"]["players"][0]["personaname"])).unwrap_or_else(|| id.clone());
    Some((id.clone(), name, Some(format!("https://steamcommunity.com/profiles/{id}"))))
}

async fn oauth(c: &Connections, p: &Provider, code: &str) -> Option<(String, String, Option<String>)> {
    let callback = c.callback(p.id);
    let form = [("grant_type", "authorization_code"), ("code", code), ("redirect_uri", callback.as_str()), ("client_id", &p.client_id), ("client_secret", &p.secret)];
    let token: Value = c.http.post(&p.token).header("Accept", "application/json").form(&form).send().await.ok()?.json().await.ok()?;
    let access = token["access_token"].as_str()?;
    let info: Value = c.http.get(&p.userinfo).bearer_auth(access).header("Client-Id", &p.client_id).header("Accept", "application/json").send().await.ok()?.json().await.ok()?;
    profile(p.id, &info)
}

pub async fn callback(State(st): State<AppState>, Path(provider): Path<String>, Query(q): Query<HashMap<String, String>>) -> Redirect {
    let failed = Redirect::to("/?connected=error");
    let (Some(c), Some(state)) = (st.connections.as_deref(), q.get("state")) else { return failed };
    let user: Option<Uuid> = sqlx::query_scalar("DELETE FROM oauth_states WHERE state = $1 AND provider = $2 AND created_at > now() - interval '10 minutes' RETURNING user_id")
        .bind(state)
        .bind(&provider)
        .fetch_optional(&st.db)
        .await
        .ok()
        .flatten();
    let Some(uid) = user else { return failed };
    let found = if provider == "steam" {
        steam(c, &q).await
    } else {
        match (c.providers.iter().find(|p| p.id == provider), q.get("code")) {
            (Some(p), Some(code)) => oauth(c, p, code).await,
            _ => None,
        }
    };
    let Some((account, name, url)) = found else { return failed };
    if save(&st, uid, &provider, &account, &name, url.as_deref()).await.is_err() {
        return failed;
    }
    super::users::announce(&st, uid).await.ok();
    Redirect::to("/?connected=ok")
}

pub async fn remove(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(provider): Path<String>) -> Result<Json<Value>> {
    sqlx::query("DELETE FROM connections WHERE user_id = $1 AND provider = $2").bind(uid).bind(provider).execute(&st.db).await?;
    super::users::announce(&st, uid).await?;
    Ok(Json(json!({})))
}
