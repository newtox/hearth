use super::{clean_color, clean_name, users::announce};
use crate::{auth::AuthUser, error::{bad, AppError, Result}, models::Badge, AppState};
use axum::{extract::{Path, State}, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

pub const ICONS: [&str; 13] = ["crown", "wrench", "shield", "gem", "star", "heart", "bolt", "flame", "leaf", "gamepad", "broadcast", "mic", "calendar"];

async fn require_admin(st: &AppState, uid: Uuid) -> Result<()> {
    let admin: bool = sqlx::query_scalar("SELECT is_admin FROM users WHERE id = $1").bind(uid).fetch_one(&st.db).await?;
    if admin { Ok(()) } else { Err(AppError::Forbidden) }
}

pub async fn list(State(st): State<AppState>, AuthUser(_): AuthUser) -> Result<Json<Vec<Badge>>> {
    Ok(Json(sqlx::query_as("SELECT id, name, description, icon, color, translations FROM badges ORDER BY position, name").fetch_all(&st.db).await?))
}

const LANGS: [&str; 2] = ["de", "da"];

#[derive(Deserialize, Default, Clone)]
pub struct Text {
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
}

#[derive(Deserialize)]
pub struct BadgeBody {
    name: String,
    #[serde(default)]
    description: String,
    icon: String,
    color: Option<String>,
    #[serde(default)]
    translations: HashMap<String, Text>,
}

struct Clean {
    name: String,
    description: String,
    icon: String,
    color: String,
    translations: Value,
}

async fn deepl(texts: &[&str], lang: &str) -> Option<Vec<String>> {
    let key = std::env::var("DEEPL_API_KEY").ok().filter(|k| !k.trim().is_empty())?;
    let host = if key.trim().ends_with(":fx") { "https://api-free.deepl.com" } else { "https://api.deepl.com" };
    let res = reqwest::Client::new()
        .post(format!("{host}/v2/translate"))
        .header("Authorization", format!("DeepL-Auth-Key {}", key.trim()))
        .json(&json!({ "text": texts, "source_lang": "EN", "target_lang": lang.to_uppercase() }))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .ok()?;
    let body: Value = res.error_for_status().ok()?.json().await.ok()?;
    body["translations"].as_array().map(|list| list.iter().filter_map(|t| t["text"].as_str().map(str::to_string)).collect())
}

pub fn translation_available() -> bool {
    std::env::var("DEEPL_API_KEY").is_ok_and(|k| !k.trim().is_empty())
}

async fn clean(b: BadgeBody) -> Result<Clean> {
    let name = clean_name(&b.name, 32, "Name")?;
    let icon = b.icon.trim().to_string();
    let built_in = icon.strip_prefix("svg:").is_some_and(|name| ICONS.contains(&name));
    if !built_in && (icon.is_empty() || icon.len() > 16 || icon.chars().any(|c| c.is_ascii() || c.is_control())) {
        return bad("Unknown symbol");
    }
    let description = b.description.trim().to_string();
    if description.chars().count() > 120 {
        return bad("Description: at most 120 characters");
    }
    let color = clean_color(&b.color)?.unwrap_or_else(|| "#8b6cff".into());
    let mut out = serde_json::Map::new();
    for lang in LANGS {
        let given = b.translations.get(lang).cloned().unwrap_or_default();
        let mut t_name = given.name.trim().to_string();
        let mut t_desc = given.description.trim().to_string();
        if t_name.is_empty() || (t_desc.is_empty() && !description.is_empty()) {
            if let Some(found) = deepl(&[&name, &description], lang).await {
                if t_name.is_empty() {
                    t_name = found.first().cloned().unwrap_or_default();
                }
                if t_desc.is_empty() && !description.is_empty() {
                    t_desc = found.get(1).cloned().unwrap_or_default();
                }
            }
        }
        if t_name.chars().count() > 32 || t_desc.chars().count() > 120 {
            return bad("Translation too long");
        }
        if !t_name.is_empty() || !t_desc.is_empty() {
            out.insert(lang.into(), json!({ "name": t_name, "description": t_desc }));
        }
    }
    Ok(Clean { name, description, icon, color, translations: Value::Object(out) })
}

pub async fn create(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(b): Json<BadgeBody>) -> Result<Json<Badge>> {
    require_admin(&st, uid).await?;
    let c = clean(b).await?;
    Ok(Json(
        sqlx::query_as(
            "INSERT INTO badges (id, name, description, icon, color, translations, position) VALUES ($1, $2, $3, $4, $5, $6, (SELECT COALESCE(max(position), 0) + 1 FROM badges)) RETURNING id, name, description, icon, color, translations",
        )
        .bind(Uuid::now_v7())
        .bind(c.name)
        .bind(c.description)
        .bind(c.icon)
        .bind(c.color)
        .bind(c.translations)
        .fetch_one(&st.db)
        .await?,
    ))
}

pub async fn update(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(bid): Path<Uuid>, Json(b): Json<BadgeBody>) -> Result<Json<Badge>> {
    require_admin(&st, uid).await?;
    let c = clean(b).await?;
    let badge: Badge = sqlx::query_as("UPDATE badges SET name = $2, description = $3, icon = $4, color = $5, translations = $6 WHERE id = $1 RETURNING id, name, description, icon, color, translations")
        .bind(bid)
        .bind(c.name)
        .bind(c.description)
        .bind(c.icon)
        .bind(c.color)
        .bind(c.translations)
        .fetch_optional(&st.db)
        .await?
        .ok_or(AppError::NotFound)?;
    let holders: Vec<Uuid> = sqlx::query_scalar("SELECT user_id FROM user_badges WHERE badge_id = $1").bind(bid).fetch_all(&st.db).await?;
    for h in holders {
        let _ = announce(&st, h).await;
    }
    Ok(Json(badge))
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
