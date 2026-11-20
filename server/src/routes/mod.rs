pub mod account;
pub mod badges;
pub mod channels;
pub mod emojis;
pub mod friends;
pub mod groups;
pub mod guilds;
pub mod invites;
pub mod members;
pub mod messages;
pub mod roles;
pub mod thumbs;
pub mod stats;
pub mod security;
pub mod users;
pub mod voice;

use crate::{error::{bad, AppError, Result}, AppState};
use axum::{body::Bytes, extract::Multipart};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

pub async fn guild_audience(st: &AppState, guild_id: Uuid) -> Result<Vec<Uuid>> {
    Ok(sqlx::query_scalar("SELECT user_id FROM members WHERE guild_id = $1").bind(guild_id).fetch_all(&st.db).await?)
}

pub async fn related_users(st: &AppState, user_id: Uuid) -> Result<Vec<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT m2.user_id FROM members m1 JOIN members m2 USING (guild_id) WHERE m1.user_id = $1 \
         UNION SELECT d2.user_id FROM dm_recipients d1 JOIN dm_recipients d2 USING (channel_id) WHERE d1.user_id = $1 \
         UNION SELECT CASE WHEN user_id = $1 THEN friend_id ELSE user_id END FROM friendships WHERE status = 'accepted' AND (user_id = $1 OR friend_id = $1) \
         UNION SELECT $1",
    )
    .bind(user_id)
    .fetch_all(&st.db)
    .await?)
}

pub async fn refresh_parts(st: &AppState, guild_id: Uuid, parts: &[&str]) -> Result<()> {
    let users = guild_audience(st, guild_id).await?;
    st.hub.send(&users, "GUILD_REFRESH", json!({ "guild_id": guild_id, "parts": parts }));
    Ok(())
}

pub fn clean_name(s: &str, max: usize, what: &str) -> Result<String> {
    let s = s.trim();
    if s.is_empty() || s.chars().count() > max {
        return Err(AppError::Bad(format!("{what}: 1–{max} characters")));
    }
    Ok(s.to_string())
}

pub fn clean_color(s: &Option<String>) -> Result<Option<String>> {
    match s.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(c) if c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|x| x.is_ascii_hexdigit()) => Ok(Some(c.to_lowercase())),
        _ => bad("Colour must be in the format #rrggbb"),
    }
}

pub async fn audit(st: &AppState, guild_id: Uuid, actor: Uuid, action: &str, target: &str, detail: &str) {
    let _ = sqlx::query("INSERT INTO audit_log (id, guild_id, actor_id, action, target, detail) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(Uuid::now_v7())
        .bind(guild_id)
        .bind(actor)
        .bind(action)
        .bind(target)
        .bind(detail)
        .execute(&st.db)
        .await;
}

pub async fn user_label(st: &AppState, id: Uuid) -> String {
    sqlx::query_scalar("SELECT username FROM users WHERE id = $1").bind(id).fetch_optional(&st.db).await.ok().flatten().unwrap_or_default()
}

pub fn image_ext(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("png")
    } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if data.starts_with(b"GIF8") {
        Some("gif")
    } else if data.len() > 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}

pub struct Form {
    pub fields: HashMap<String, String>,
    pub file: Option<(String, Bytes)>,
}

pub async fn read_form(mut multipart: Multipart) -> Result<Form> {
    let mut form = Form { fields: HashMap::new(), file: None };
    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::Bad(e.to_string()))? {
        let name = field.name().unwrap_or_default().to_string();
        if name == "file" {
            let filename = field.file_name().unwrap_or("file").to_string();
            let data = field.bytes().await.map_err(|_| AppError::Bad("The file is too large".into()))?;
            form.file = Some((filename, data));
        } else {
            let text = field.text().await.map_err(|e| AppError::Bad(e.to_string()))?;
            form.fields.insert(name, text);
        }
    }
    Ok(form)
}

pub async fn store_image(st: &AppState, data: &[u8], max: usize) -> Result<String> {
    if data.len() > max {
        return Err(AppError::Bad(format!("The image is too large (max. {} MB)", max / 1024 / 1024)));
    }
    let Some(ext) = image_ext(data) else {
        return bad("Only PNG, JPEG, GIF or WebP");
    };
    let name = format!("{}.{ext}", Uuid::now_v7().simple());
    tokio::fs::write(st.upload_dir.join(&name), data).await?;
    Ok(name)
}

pub async fn save_image(st: &AppState, multipart: Multipart) -> Result<String> {
    let form = read_form(multipart).await?;
    let Some((_, data)) = form.file else {
        return bad("No file was sent");
    };
    let crop = form.fields.get("crop").zip(form.fields.get("size")).and_then(|(r, o)| thumbs::Crop::parse(r, o));
    if let (Some(crop), Some("gif")) = (crop, image_ext(&data)) {
        if data.len() > 8 * 1024 * 1024 {
            return Err(AppError::Bad("The image is too large (max. 8 MB)".into()));
        }
        let cut = tokio::task::spawn_blocking(move || thumbs::crop_gif(&data, &crop)).await.map_err(|e| AppError::Internal(e.to_string()))?;
        let Some(cut) = cut else {
            return bad("The image could not be read");
        };
        return store_image(st, &cut, 8 * 1024 * 1024).await;
    }
    store_image(st, &data, 8 * 1024 * 1024).await
}

pub async fn remove_upload(st: &AppState, name: Option<String>) {
    if let Some(n) = name {
        let base = n.strip_prefix("files/").unwrap_or(&n);
        if !base.is_empty() && base.chars().all(|c| c.is_ascii_alphanumeric() || c == '.') && !base.starts_with('.') {
            let _ = tokio::fs::remove_file(st.upload_dir.join(&n)).await;
            for size in ["32", "64", "128", "256"] {
                for suffix in ["gif", "png"] {
                    let _ = tokio::fs::remove_file(st.upload_dir.join("thumbs").join(size).join(format!("{base}.{suffix}"))).await;
                }
            }
        }
    }
}
