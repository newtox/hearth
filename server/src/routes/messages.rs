use super::{audit, channels::{access, viewers}, remove_upload, user_label};
use crate::{auth::AuthUser, error::{bad, AppError, Result}, models::{Attachment, MessageRow, MESSAGE_SELECT}, perms, AppState};
use axum::{extract::{Multipart, Path, Query, State}, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

async fn to_json(st: &AppState, rows: &[MessageRow]) -> Result<Vec<Value>> {
    let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let attachments: Vec<Attachment> = sqlx::query_as("SELECT id, message_id, filename, file, size, is_image FROM attachments WHERE message_id = ANY($1) ORDER BY id")
        .bind(&ids)
        .fetch_all(&st.db)
        .await?;
    let reactions: Vec<(Uuid, String, Vec<Uuid>)> = sqlx::query_as(
        "SELECT message_id, emoji, array_agg(user_id ORDER BY created_at) FROM reactions WHERE message_id = ANY($1) GROUP BY message_id, emoji ORDER BY min(created_at)",
    )
    .bind(&ids)
    .fetch_all(&st.db)
    .await?;
    Ok(rows
        .iter()
        .map(|r| {
            let mut v = r.json();
            v["attachments"] = json!(attachments.iter().filter(|a| a.message_id == Some(r.id)).collect::<Vec<_>>());
            v["reactions"] = json!(reactions.iter().filter(|x| x.0 == r.id).map(|x| json!({ "emoji": x.1, "users": x.2 })).collect::<Vec<_>>());
            v
        })
        .collect())
}

async fn fetch(st: &AppState, id: Uuid) -> Result<Value> {
    let row: MessageRow = sqlx::query_as(&format!("{MESSAGE_SELECT} WHERE m.id = $1")).bind(id).fetch_one(&st.db).await?;
    Ok(to_json(st, &[row]).await?.remove(0))
}

#[derive(Deserialize)]
pub struct ListQuery {
    before: Option<Uuid>,
    limit: Option<i64>,
}

pub async fn list(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(cid): Path<Uuid>, Query(q): Query<ListQuery>) -> Result<Json<Vec<Value>>> {
    access(&st, uid, cid, 0).await?;
    let mut rows: Vec<MessageRow> = sqlx::query_as(&format!(
        "{MESSAGE_SELECT} WHERE m.channel_id = $1 AND ($2::uuid IS NULL OR m.id < $2) ORDER BY m.id DESC LIMIT $3"
    ))
    .bind(cid)
    .bind(q.before)
    .bind(q.limit.unwrap_or(50).clamp(1, 100))
    .fetch_all(&st.db)
    .await?;
    rows.reverse();
    Ok(Json(to_json(&st, &rows).await?))
}

#[derive(Deserialize)]
pub struct MessageBody {
    #[serde(default)]
    content: String,
    #[serde(default)]
    attachment_ids: Vec<Uuid>,
}

pub async fn upload(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(cid): Path<Uuid>, multipart: Multipart) -> Result<Json<Value>> {
    access(&st, uid, cid, perms::SEND_MESSAGES | perms::ATTACH_FILES).await?;
    st.limits.check(format!("upload:{uid}"), 20, Duration::from_secs(60))?;
    let form = super::read_form(multipart).await?;
    let Some((filename, data)) = form.file else {
        return bad("No file was sent");
    };
    if data.is_empty() {
        return bad("The file is empty");
    }
    let filename: String = filename.chars().filter(|c| !c.is_control() && *c != '/' && *c != '\\').take(120).collect();
    let (file, is_image) = match super::image_ext(&data) {
        Some(ext) => (format!("{}.{ext}", Uuid::now_v7().simple()), true),
        None => (format!("files/{}", Uuid::now_v7().simple()), false),
    };
    tokio::fs::write(st.upload_dir.join(&file), &data).await?;
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO attachments (id, channel_id, uploader_id, filename, file, size, is_image) VALUES ($1, $2, $3, $4, $5, $6, $7)")
        .bind(id)
        .bind(cid)
        .bind(uid)
        .bind(&filename)
        .bind(&file)
        .bind(data.len() as i64)
        .bind(is_image)
        .execute(&st.db)
        .await?;
    Ok(Json(json!({ "id": id, "filename": filename, "file": file, "size": data.len(), "is_image": is_image })))
}

pub async fn create(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(cid): Path<Uuid>, Json(b): Json<MessageBody>) -> Result<Json<Value>> {
    let a = access(&st, uid, cid, perms::SEND_MESSAGES).await?;
    if a.channel.kind == "category" || a.channel.kind == "voice" {
        return bad("You cannot write in this channel");
    }
    if a.channel.kind == "dm" {
        let other: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM dm_recipients WHERE channel_id = $1 AND user_id <> $2").bind(cid).bind(uid).fetch_optional(&st.db).await?;
        if let Some(other) = other {
            if super::friends::blocked_between(&st, uid, other).await? {
                return bad("You cannot message this user");
            }
        }
    }
    st.limits.check(format!("msg:{uid}"), 10, Duration::from_secs(10))?;
    let content = b.content.trim();
    if content.chars().count() > 4000 || (content.is_empty() && b.attachment_ids.is_empty()) {
        return bad("Message: 1–4000 characters");
    }
    if b.attachment_ids.len() > 10 {
        return bad("At most 10 attachments per message");
    }
    let id = Uuid::now_v7();
    let mut tx = st.db.begin().await?;
    sqlx::query("INSERT INTO messages (id, channel_id, author_id, content) VALUES ($1, $2, $3, $4)")
        .bind(id)
        .bind(cid)
        .bind(uid)
        .bind(content)
        .execute(&mut *tx)
        .await?;
    if !b.attachment_ids.is_empty() {
        let linked = sqlx::query("UPDATE attachments SET message_id = $1 WHERE id = ANY($2) AND uploader_id = $3 AND channel_id = $4 AND message_id IS NULL")
            .bind(id)
            .bind(&b.attachment_ids)
            .bind(uid)
            .bind(cid)
            .execute(&mut *tx)
            .await?;
        if linked.rows_affected() as usize != b.attachment_ids.len() {
            return bad("An attachment is invalid");
        }
    }
    tx.commit().await?;
    let msg = fetch(&st, id).await?;
    st.hub.send(&viewers(&st, &a.channel).await?, "MESSAGE_CREATE", &msg);
    Ok(Json(msg))
}

pub async fn update(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((cid, mid)): Path<(Uuid, Uuid)>, Json(b): Json<MessageBody>) -> Result<Json<Value>> {
    let a = access(&st, uid, cid, perms::SEND_MESSAGES).await?;
    let content = b.content.trim();
    if content.is_empty() || content.chars().count() > 4000 {
        return bad("Message: 1–4000 characters");
    }
    let done = sqlx::query("UPDATE messages SET content = $1, edited_at = now() WHERE id = $2 AND channel_id = $3 AND author_id = $4")
        .bind(content)
        .bind(mid)
        .bind(cid)
        .bind(uid)
        .execute(&st.db)
        .await?;
    if done.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    let msg = fetch(&st, mid).await?;
    st.hub.send(&viewers(&st, &a.channel).await?, "MESSAGE_UPDATE", &msg);
    Ok(Json(msg))
}

pub async fn delete(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((cid, mid)): Path<(Uuid, Uuid)>) -> Result<Json<Value>> {
    let a = access(&st, uid, cid, 0).await?;
    let author: Uuid = sqlx::query_scalar("SELECT author_id FROM messages WHERE id = $1 AND channel_id = $2").bind(mid).bind(cid).fetch_one(&st.db).await?;
    let allowed = author == uid
        || match a.channel.guild_id {
            Some(_) => a.perms & perms::MANAGE_MESSAGES != 0,
            None => a.channel.kind == "group" && super::groups::group(&st, uid, cid).await?.may("delete"),
        };
    if !allowed {
        return Err(AppError::Forbidden);
    }
    let files: Vec<String> = sqlx::query_scalar("SELECT file FROM attachments WHERE message_id = $1").bind(mid).fetch_all(&st.db).await?;
    sqlx::query("DELETE FROM messages WHERE id = $1").bind(mid).execute(&st.db).await?;
    for f in files {
        remove_upload(&st, Some(f)).await;
    }
    if let (Some(gid), true) = (a.channel.guild_id, author != uid) {
        audit(&st, gid, uid, "message_delete", &user_label(&st, author).await, &a.channel.name).await;
    }
    st.hub.send(&viewers(&st, &a.channel).await?, "MESSAGE_DELETE", json!({ "id": mid, "channel_id": cid }));
    Ok(Json(json!({})))
}

#[derive(Deserialize)]
pub struct ReactBody {
    emoji: String,
    on: bool,
}

fn valid_emoji(e: &str) -> bool {
    if let Some(inner) = e.strip_prefix("<:").and_then(|x| x.strip_suffix('>')) {
        let mut parts = inner.split(':');
        let (Some(name), Some(file), None) = (parts.next(), parts.next(), parts.next()) else { return false };
        return (2..=32).contains(&name.len())
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && (5..=48).contains(&file.len())
            && file.chars().all(|c| c.is_ascii_alphanumeric() || c == '.');
    }
    !e.is_empty() && e.len() <= 32 && e.chars().all(|c| !c.is_ascii() && !c.is_control() && !c.is_alphabetic())
}

pub async fn react(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((cid, mid)): Path<(Uuid, Uuid)>, Json(b): Json<ReactBody>) -> Result<Json<Value>> {
    let a = access(&st, uid, cid, if b.on { perms::ADD_REACTIONS } else { 0 }).await?;
    st.limits.check(format!("react:{uid}"), 30, Duration::from_secs(10))?;
    if !valid_emoji(&b.emoji) {
        return bad("Invalid emoji");
    }
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM messages WHERE id = $1 AND channel_id = $2)").bind(mid).bind(cid).fetch_one(&st.db).await?;
    if !exists {
        return Err(AppError::NotFound);
    }
    if b.on {
        let distinct: i64 = sqlx::query_scalar("SELECT count(DISTINCT emoji) FROM reactions WHERE message_id = $1 AND emoji <> $2").bind(mid).bind(&b.emoji).fetch_one(&st.db).await?;
        if distinct >= 20 {
            return bad("At most 20 different reactions per message");
        }
        sqlx::query("INSERT INTO reactions (message_id, user_id, emoji) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING").bind(mid).bind(uid).bind(&b.emoji).execute(&st.db).await?;
    } else {
        sqlx::query("DELETE FROM reactions WHERE message_id = $1 AND user_id = $2 AND emoji = $3").bind(mid).bind(uid).bind(&b.emoji).execute(&st.db).await?;
    }
    let msg = fetch(&st, mid).await?;
    st.hub.send(&viewers(&st, &a.channel).await?, "MESSAGE_UPDATE", &msg);
    Ok(Json(json!({})))
}

pub async fn typing(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(cid): Path<Uuid>) -> Result<Json<Value>> {
    let a = access(&st, uid, cid, perms::SEND_MESSAGES).await?;
    st.hub.send(&viewers(&st, &a.channel).await?, "TYPING_START", json!({ "channel_id": cid, "user_id": uid }));
    Ok(Json(json!({})))
}
