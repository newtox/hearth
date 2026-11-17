use super::{audit, channels::{access, viewers}, remove_upload, user_label};
use crate::{auth::AuthUser, error::{bad, AppError, Result}, models::{Attachment, MessageRow, MESSAGE_SELECT}, perms, AppState};
use axum::{extract::{Multipart, Path, Query, State}, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

fn emoji_files(content: &str) -> Vec<&str> {
    content
        .split('<')
        .skip(1)
        .filter_map(|part| {
            let mut fields = part.split('>').next()?.split(':');
            let kind = fields.next()?;
            let (_, file, None) = (fields.next()?, fields.next()?, fields.next()) else { return None };
            (kind.is_empty() || kind == "sticker").then_some(file)
        })
        .collect()
}

async fn check_emojis(st: &AppState, a: &super::channels::Access, content: &str) -> Result<()> {
    let Some(gid) = a.channel.guild_id else { return Ok(()) };
    if a.perms & perms::USE_EXTERNAL_EMOJIS != 0 {
        return Ok(());
    }
    let used = emoji_files(content);
    if used.is_empty() {
        return Ok(());
    }
    let own: Vec<String> = sqlx::query_scalar("SELECT file FROM emojis WHERE guild_id = $1").bind(gid).fetch_all(&st.db).await?;
    if used.iter().all(|f| own.iter().any(|o| o == f)) { Ok(()) } else { bad("You cannot use emojis from other servers here") }
}

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
    let polls: Vec<(Uuid, String, bool, Option<chrono::DateTime<chrono::Utc>>)> =
        sqlx::query_as("SELECT message_id, question, multi, expires_at FROM polls WHERE message_id = ANY($1)").bind(&ids).fetch_all(&st.db).await?;
    let options: Vec<(Uuid, Uuid, String, Vec<Uuid>)> = if polls.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as(
            "SELECT o.id, o.message_id, o.text, COALESCE(array_agg(v.user_id) FILTER (WHERE v.user_id IS NOT NULL), '{}') \
             FROM poll_options o LEFT JOIN poll_votes v ON v.option_id = o.id WHERE o.message_id = ANY($1) GROUP BY o.id ORDER BY o.position",
        )
        .bind(&ids)
        .fetch_all(&st.db)
        .await?
    };
    let reply_ids: Vec<Uuid> = rows.iter().filter_map(|r| r.reply_to).collect();
    let replied: Vec<MessageRow> = if reply_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as(&format!("{MESSAGE_SELECT} WHERE m.id = ANY($1)")).bind(&reply_ids).fetch_all(&st.db).await?
    };
    let with_files: Vec<Uuid> = if reply_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_scalar("SELECT DISTINCT message_id FROM attachments WHERE message_id = ANY($1)").bind(&reply_ids).fetch_all(&st.db).await?
    };
    Ok(rows
        .iter()
        .map(|r| {
            let mut v = r.json();
            if let Some(rid) = r.reply_to {
                v["reply"] = match replied.iter().find(|x| x.id == rid) {
                    Some(x) => json!({
                        "id": x.id,
                        "content": x.content.chars().take(200).collect::<String>(),
                        "attachments": with_files.contains(&x.id),
                        "author": { "id": x.author_id, "username": x.username, "display_name": x.display_name, "avatar": x.avatar },
                    }),
                    None => json!({ "id": rid, "deleted": true }),
                };
            }
            if let Some(p) = polls.iter().find(|p| p.0 == r.id) {
                let opts: Vec<Value> = options.iter().filter(|o| o.1 == r.id).map(|o| json!({ "id": o.0, "text": o.2, "voters": o.3 })).collect();
                v["poll"] = json!({ "question": p.1, "multi": p.2, "expires_at": p.3, "options": opts });
            }
            v["attachments"] = json!(attachments.iter().filter(|a| a.message_id == Some(r.id)).collect::<Vec<_>>());
            v["reactions"] = json!(reactions.iter().filter(|x| x.0 == r.id).map(|x| json!({ "emoji": x.1, "users": x.2 })).collect::<Vec<_>>());
            v
        })
        .collect())
}

pub async fn call_joined(st: &AppState, room: Uuid, user: Uuid) -> Result<()> {
    let existing = st.hub.calls.get(&room).map(|m| *m);
    let id = match existing {
        Some(id) => {
            sqlx::query("UPDATE messages SET call_participants = array_append(call_participants, $2) WHERE id = $1 AND NOT ($2 = ANY(call_participants))").bind(id).bind(user).execute(&st.db).await?;
            id
        }
        None => {
            let id = Uuid::now_v7();
            sqlx::query("INSERT INTO messages (id, channel_id, author_id, content, kind, call_participants) VALUES ($1, $2, $3, '', 'call', ARRAY[$3])").bind(id).bind(room).bind(user).execute(&st.db).await?;
            st.hub.calls.insert(room, id);
            id
        }
    };
    let msg = fetch(st, id).await?;
    st.hub.send(&super::groups::recipients(st, room).await?, if existing.is_some() { "MESSAGE_UPDATE" } else { "MESSAGE_CREATE" }, &msg);
    Ok(())
}

pub async fn call_left(st: &AppState, room: Uuid) -> Result<()> {
    if st.hub.voice.iter().any(|e| *e.value() == room) {
        return Ok(());
    }
    let Some((_, id)) = st.hub.calls.remove(&room) else { return Ok(()) };
    sqlx::query("UPDATE messages SET call_ended_at = now() WHERE id = $1").bind(id).execute(&st.db).await?;
    if let Ok(msg) = fetch(st, id).await {
        st.hub.send(&super::groups::recipients(st, room).await?, "MESSAGE_UPDATE", &msg);
    }
    Ok(())
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
    poll: Option<PollBody>,
    reply_to: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct PollBody {
    question: String,
    options: Vec<String>,
    #[serde(default)]
    multi: bool,
    hours: Option<i64>,
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
    if content.chars().count() > 4000 || (content.is_empty() && b.attachment_ids.is_empty() && b.poll.is_none()) {
        return bad("Message: 1–4000 characters");
    }
    let poll = match &b.poll {
        Some(p) => {
            let question = p.question.trim();
            let options: Vec<&str> = p.options.iter().map(|o| o.trim()).filter(|o| !o.is_empty()).collect();
            if question.is_empty() || question.chars().count() > 200 || !(2..=10).contains(&options.len()) || options.iter().any(|o| o.chars().count() > 80) {
                return bad("A poll needs a question and 2 to 10 options");
            }
            Some((question, options, p.multi, p.hours.filter(|h| *h > 0).map(|h| h.min(24 * 30))))
        }
        None => None,
    };
    check_emojis(&st, &a, content).await?;
    if b.attachment_ids.len() > 10 {
        return bad("At most 10 attachments per message");
    }
    if let Some(rid) = b.reply_to {
        let same: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM messages WHERE id = $1 AND channel_id = $2)").bind(rid).bind(cid).fetch_one(&st.db).await?;
        if !same {
            return bad("The message you are replying to no longer exists");
        }
    }
    let id = Uuid::now_v7();
    let mut tx = st.db.begin().await?;
    sqlx::query("INSERT INTO messages (id, channel_id, author_id, content, reply_to) VALUES ($1, $2, $3, $4, $5)")
        .bind(id)
        .bind(cid)
        .bind(uid)
        .bind(content)
        .bind(b.reply_to)
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
    if let Some((question, options, multi, hours)) = poll {
        sqlx::query("INSERT INTO polls (message_id, question, multi, expires_at) VALUES ($1, $2, $3, CASE WHEN $4::int IS NULL THEN NULL ELSE now() + make_interval(hours => $4) END)")
            .bind(id)
            .bind(question)
            .bind(multi)
            .bind(hours.map(|h| h as i32))
            .execute(&mut *tx)
            .await?;
        for (i, text) in options.iter().enumerate() {
            sqlx::query("INSERT INTO poll_options (id, message_id, position, text) VALUES ($1, $2, $3, $4)").bind(Uuid::now_v7()).bind(id).bind(i as i32).bind(text).execute(&mut *tx).await?;
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
    check_emojis(&st, &a, content).await?;
    let done = sqlx::query("UPDATE messages SET content = $1, edited_at = now() WHERE id = $2 AND channel_id = $3 AND author_id = $4 AND kind = 'default'")
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
    if b.on {
        check_emojis(&st, &a, &b.emoji).await?;
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

#[derive(Deserialize)]
pub struct VoteBody {
    option_ids: Vec<Uuid>,
}

pub async fn vote(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((cid, mid)): Path<(Uuid, Uuid)>, Json(b): Json<VoteBody>) -> Result<Json<Value>> {
    let a = access(&st, uid, cid, 0).await?;
    st.limits.check(format!("react:{uid}"), 30, Duration::from_secs(10))?;
    let poll: Option<(bool, bool)> = sqlx::query_as(
        "SELECT p.multi, (p.expires_at IS NOT NULL AND p.expires_at < now()) FROM polls p JOIN messages m ON m.id = p.message_id WHERE p.message_id = $1 AND m.channel_id = $2",
    )
    .bind(mid)
    .bind(cid)
    .fetch_optional(&st.db)
    .await?;
    let (multi, closed) = poll.ok_or(AppError::NotFound)?;
    if closed {
        return bad("This poll has ended");
    }
    let mut wanted = b.option_ids;
    wanted.sort();
    wanted.dedup();
    if !multi && wanted.len() > 1 {
        return bad("This poll allows only one answer");
    }
    let valid: i64 = sqlx::query_scalar("SELECT count(*) FROM poll_options WHERE message_id = $1 AND id = ANY($2)").bind(mid).bind(&wanted).fetch_one(&st.db).await?;
    if valid as usize != wanted.len() {
        return bad("Unknown option");
    }
    let mut tx = st.db.begin().await?;
    sqlx::query("DELETE FROM poll_votes WHERE message_id = $1 AND user_id = $2").bind(mid).bind(uid).execute(&mut *tx).await?;
    for o in &wanted {
        sqlx::query("INSERT INTO poll_votes (option_id, user_id, message_id) VALUES ($1, $2, $3)").bind(o).bind(uid).bind(mid).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    let msg = fetch(&st, mid).await?;
    st.hub.send(&viewers(&st, &a.channel).await?, "MESSAGE_UPDATE", &msg);
    Ok(Json(json!({})))
}

async fn may_pin(st: &AppState, uid: Uuid, a: &super::channels::Access) -> Result<()> {
    let allowed = match a.channel.guild_id {
        Some(_) => a.perms & perms::PIN_MESSAGES != 0,
        None => a.channel.kind != "group" || super::groups::group(st, uid, a.channel.id).await?.may("pin"),
    };
    if allowed { Ok(()) } else { Err(AppError::Forbidden) }
}

pub async fn pins(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(cid): Path<Uuid>) -> Result<Json<Vec<Value>>> {
    access(&st, uid, cid, 0).await?;
    let rows: Vec<MessageRow> = sqlx::query_as(&format!("{MESSAGE_SELECT} WHERE m.channel_id = $1 AND m.pinned_at IS NOT NULL ORDER BY m.pinned_at DESC"))
        .bind(cid)
        .fetch_all(&st.db)
        .await?;
    Ok(Json(to_json(&st, &rows).await?))
}

async fn set_pin(st: &AppState, uid: Uuid, cid: Uuid, mid: Uuid, pinned: bool) -> Result<Json<Value>> {
    let a = access(st, uid, cid, 0).await?;
    may_pin(st, uid, &a).await?;
    if pinned {
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM messages WHERE channel_id = $1 AND pinned_at IS NOT NULL").bind(cid).fetch_one(&st.db).await?;
        if count >= 50 {
            return bad("At most 50 pinned messages per channel");
        }
    }
    let done = sqlx::query(if pinned { "UPDATE messages SET pinned_at = COALESCE(pinned_at, now()) WHERE id = $1 AND channel_id = $2" } else { "UPDATE messages SET pinned_at = NULL WHERE id = $1 AND channel_id = $2" })
        .bind(mid)
        .bind(cid)
        .execute(&st.db)
        .await?;
    if done.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    let msg = fetch(st, mid).await?;
    st.hub.send(&viewers(st, &a.channel).await?, "MESSAGE_UPDATE", &msg);
    if let Some(gid) = a.channel.guild_id {
        audit(st, gid, uid, if pinned { "message_pin" } else { "message_unpin" }, &a.channel.name, "").await;
    }
    Ok(Json(msg))
}

pub async fn pin(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((cid, mid)): Path<(Uuid, Uuid)>) -> Result<Json<Value>> {
    set_pin(&st, uid, cid, mid, true).await
}

pub async fn unpin(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((cid, mid)): Path<(Uuid, Uuid)>) -> Result<Json<Value>> {
    set_pin(&st, uid, cid, mid, false).await
}

#[derive(Deserialize)]
pub struct SearchQuery {
    q: Option<String>,
    guild_id: Option<Uuid>,
    channel_id: Option<Uuid>,
    author: Option<String>,
    mentions: Option<String>,
    has: Option<String>,
    before: Option<chrono::DateTime<chrono::Utc>>,
    after: Option<chrono::DateTime<chrono::Utc>>,
    pinned: Option<bool>,
    offset: Option<i64>,
}

const HAS: [&str; 7] = ["link", "file", "image", "video", "sound", "sticker", "poll"];
const VIDEO: &str = "\\.(mp4|webm|mov|mkv|avi)$";
const SOUND: &str = "\\.(mp3|ogg|oga|opus|wav|flac|m4a|aac)$";

struct Filters {
    ids: Vec<Uuid>,
    pattern: Option<String>,
    author: Option<String>,
    mention: Option<String>,
    has: Vec<String>,
    before: Option<chrono::DateTime<chrono::Utc>>,
    after: Option<chrono::DateTime<chrono::Utc>>,
    pinned: bool,
}

fn name(raw: Option<&str>) -> Option<String> {
    raw.map(|a| a.trim().trim_start_matches('@').to_lowercase()).filter(|a| !a.is_empty())
}

fn push_filters(qb: &mut sqlx::QueryBuilder<'_, sqlx::Postgres>, f: &Filters) {
    qb.push(" WHERE m.channel_id = ANY(").push_bind(f.ids.clone()).push(")");
    if let Some(p) = &f.pattern {
        qb.push(" AND (m.content ILIKE ").push_bind(p.clone()).push(" OR EXISTS (SELECT 1 FROM polls pl WHERE pl.message_id = m.id AND pl.question ILIKE ").push_bind(p.clone()).push("))");
    }
    if let Some(a) = &f.author {
        qb.push(" AND u.username = ").push_bind(a.clone());
    }
    if let Some(n) = &f.mention {
        let escaped = n.replace('.', "\\.");
        qb.push(" AND m.content ~* ").push_bind(format!("(^|[^[:alnum:]_@])@{escaped}([^[:alnum:]_.]|$)"));
    }
    for kind in &f.has {
        qb.push(match kind.as_str() {
            "link" => " AND m.content ~* 'https?://'",
            "file" => " AND EXISTS (SELECT 1 FROM attachments a WHERE a.message_id = m.id)",
            "image" => " AND EXISTS (SELECT 1 FROM attachments a WHERE a.message_id = m.id AND a.is_image)",
            "sticker" => " AND m.content LIKE '<sticker:%'",
            "poll" => " AND EXISTS (SELECT 1 FROM polls pl WHERE pl.message_id = m.id)",
            _ => "",
        });
        if kind == "video" || kind == "sound" {
            qb.push(" AND EXISTS (SELECT 1 FROM attachments a WHERE a.message_id = m.id AND a.filename ~* ")
                .push_bind(if kind == "video" { VIDEO } else { SOUND })
                .push(")");
        }
    }
    if let Some(b) = f.before {
        qb.push(" AND m.created_at < ").push_bind(b);
    }
    if let Some(a) = f.after {
        qb.push(" AND m.created_at >= ").push_bind(a);
    }
    if f.pinned {
        qb.push(" AND m.pinned_at IS NOT NULL");
    }
}

pub async fn search(State(st): State<AppState>, AuthUser(uid): AuthUser, Query(q): Query<SearchQuery>) -> Result<Json<Value>> {
    st.limits.check(format!("search:{uid}"), 30, Duration::from_secs(60))?;
    let text = q.q.as_deref().unwrap_or("").trim().to_string();
    let author = name(q.author.as_deref());
    let mention = name(q.mentions.as_deref());
    let has: Vec<String> = q.has.as_deref().unwrap_or("").split(',').map(|h| h.trim().to_lowercase()).filter(|h| HAS.contains(&h.as_str())).collect();
    let pinned = q.pinned.unwrap_or(false);
    let filtered = author.is_some() || mention.is_some() || !has.is_empty() || q.before.is_some() || q.after.is_some() || pinned;
    if text.chars().count() > 200 || (text.chars().count() < 2 && !filtered) {
        return bad("Search: at least 2 characters");
    }
    let channels: Vec<(Uuid, String)> = match (q.channel_id, q.guild_id) {
        (Some(cid), _) => {
            let a = access(&st, uid, cid, 0).await?;
            vec![(cid, a.channel.name)]
        }
        (None, Some(gid)) => super::channels::visible(&st, uid, gid).await?,
        (None, None) => return bad("Search: pick a server or channel"),
    };
    let filters = Filters {
        ids: channels.iter().map(|c| c.0).collect(),
        pattern: (!text.is_empty()).then(|| format!("%{}%", text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"))),
        author,
        mention,
        has,
        before: q.before,
        after: q.after,
        pinned,
    };
    let offset = q.offset.unwrap_or(0).clamp(0, 10_000);
    let mut count = sqlx::QueryBuilder::new("SELECT count(*) FROM messages m JOIN users u ON u.id = m.author_id");
    push_filters(&mut count, &filters);
    let total: i64 = count.build_query_scalar().fetch_one(&st.db).await?;
    let mut select = sqlx::QueryBuilder::new(MESSAGE_SELECT);
    push_filters(&mut select, &filters);
    select.push(" ORDER BY m.id DESC LIMIT 25 OFFSET ").push_bind(offset);
    let rows: Vec<MessageRow> = select.build_query_as().fetch_all(&st.db).await?;
    let mut results = to_json(&st, &rows).await?;
    for v in &mut results {
        let cid = v["channel_id"].as_str().and_then(|s| s.parse::<Uuid>().ok());
        if let Some(name) = cid.and_then(|cid| channels.iter().find(|c| c.0 == cid)).map(|c| c.1.clone()) {
            v["channel_name"] = json!(name);
        }
    }
    Ok(Json(json!({ "total": total, "results": results })))
}
