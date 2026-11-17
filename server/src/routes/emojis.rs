use super::{audit, read_form, refresh_parts, remove_upload, store_image};
use crate::{auth::AuthUser, error::{bad, Result}, models::Emoji, perms, AppState};
use axum::{extract::{Multipart, Path, State}, Json};
use serde_json::{json, Value};
use uuid::Uuid;

pub async fn create(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(gid): Path<Uuid>, multipart: Multipart) -> Result<Json<Emoji>> {
    perms::ctx(&st.db, gid, uid).await?.require(perms::MANAGE_EMOJIS)?;
    let form = read_form(multipart).await?;
    let kind = form.fields.get("kind").map(String::as_str).unwrap_or("emoji");
    let (limit, max_bytes) = match kind {
        "emoji" => (100, 1024 * 1024),
        "sticker" => (50, 2 * 1024 * 1024),
        _ => return bad("Unknown type"),
    };
    let name = form.fields.get("name").map(|n| n.trim().to_lowercase()).unwrap_or_default();
    if !(2..=32).contains(&name.len()) || !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
        return bad("Name: 2–32 characters, only a–z, 0–9 and underscore");
    }
    let Some((_, data)) = form.file else {
        return bad("No file was sent");
    };
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM emojis WHERE guild_id = $1 AND kind = $2").bind(gid).bind(kind).fetch_one(&st.db).await?;
    if count >= limit {
        return bad("This server has reached its limit");
    }
    let taken: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM emojis WHERE guild_id = $1 AND kind = $2 AND name = $3)")
        .bind(gid)
        .bind(kind)
        .bind(&name)
        .fetch_one(&st.db)
        .await?;
    if taken {
        return bad("That name is already taken");
    }
    let file = store_image(&st, &data, max_bytes).await?;
    let emoji: Emoji = sqlx::query_as("INSERT INTO emojis (id, guild_id, kind, name, file, creator_id) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id, kind, name, file")
        .bind(Uuid::now_v7())
        .bind(gid)
        .bind(kind)
        .bind(&name)
        .bind(file)
        .bind(uid)
        .fetch_one(&st.db)
        .await?;
    audit(&st, gid, uid, if kind == "emoji" { "emoji_create" } else { "sticker_create" }, &name, "").await;
    refresh_parts(&st, gid, &["emojis"]).await?;
    Ok(Json(emoji))
}

pub async fn delete(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((gid, eid)): Path<(Uuid, Uuid)>) -> Result<Json<Value>> {
    perms::ctx(&st.db, gid, uid).await?.require(perms::MANAGE_EMOJIS)?;
    let (name, file): (String, String) = sqlx::query_as("DELETE FROM emojis WHERE id = $1 AND guild_id = $2 RETURNING name, file").bind(eid).bind(gid).fetch_one(&st.db).await?;
    audit(&st, gid, uid, "emoji_delete", &name, "").await;
    refresh_parts(&st, gid, &["emojis"]).await?;
    remove_upload(&st, Some(file)).await;
    Ok(Json(json!({})))
}
