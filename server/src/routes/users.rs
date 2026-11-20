use super::{clean_color, clean_name, related_users, remove_upload, save_image};
use crate::{auth::AuthUser, error::{bad, AppError, Result}, models::{PublicUser, USER_COLS}, AppState};
use axum::{extract::{Multipart, Path, State}, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

pub fn decorate(st: &AppState, u: &mut PublicUser) {
    let shown = st.hub.visible(&u.id);
    u.online = shown;
    u.presence = st.hub.presence(&u.id).to_string();
    u.streaming = shown && st.hub.streaming.contains_key(&u.id);
}

pub async fn public_user(st: &AppState, id: Uuid) -> Result<PublicUser> {
    let mut u: PublicUser = sqlx::query_as(&format!("SELECT {USER_COLS} FROM users WHERE id = $1")).bind(id).fetch_one(&st.db).await?;
    decorate(st, &mut u);
    let (badges, progress) = super::stats::tracked(st, id).await?;
    u.badges.0.extend(badges);
    u.progress = Some(progress);
    Ok(u)
}

pub async fn me_json(st: &AppState, id: Uuid) -> Result<Value> {
    let user = public_user(st, id).await?;
    let email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1").bind(id).fetch_one(&st.db).await?;
    let mut v = serde_json::to_value(user).map_err(|e| AppError::Internal(e.to_string()))?;
    v["email"] = json!(email);
    let verified: bool = sqlx::query_scalar("SELECT email_verified FROM users WHERE id = $1").bind(id).fetch_one(&st.db).await?;
    v["email_verified"] = json!(verified);
    let status: String = sqlx::query_scalar("SELECT status FROM users WHERE id = $1").bind(id).fetch_one(&st.db).await?;
    v["status"] = json!(status);
    v["voice"] = json!(st.livekit.is_some());
    let two_factor: bool = sqlx::query_scalar("SELECT totp_secret IS NOT NULL FROM users WHERE id = $1").bind(id).fetch_one(&st.db).await?;
    v["two_factor"] = json!(two_factor);
    Ok(v)
}

pub async fn announce(st: &AppState, id: Uuid) -> Result<()> {
    let user = public_user(st, id).await?;
    st.hub.send(&related_users(st, id).await?, "USER_UPDATE", &user);
    Ok(())
}

pub async fn me(State(st): State<AppState>, AuthUser(uid): AuthUser) -> Result<Json<Value>> {
    Ok(Json(me_json(&st, uid).await?))
}

#[derive(Deserialize)]
pub struct UpdateMe {
    display_name: Option<String>,
    bio: Option<String>,
    accent_color: Option<String>,
    status: Option<String>,
}

pub async fn update_me(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(b): Json<UpdateMe>) -> Result<Json<Value>> {
    if let Some(n) = &b.display_name {
        let n = clean_name(n, 32, "Display name")?;
        sqlx::query("UPDATE users SET display_name = $1 WHERE id = $2").bind(n).bind(uid).execute(&st.db).await?;
    }
    if let Some(bio) = &b.bio {
        if bio.chars().count() > 500 {
            return bad("Description: at most 500 characters");
        }
        sqlx::query("UPDATE users SET bio = $1 WHERE id = $2").bind(bio.trim()).bind(uid).execute(&st.db).await?;
    }
    if b.accent_color.is_some() {
        let c = clean_color(&b.accent_color)?;
        sqlx::query("UPDATE users SET accent_color = $1 WHERE id = $2").bind(c).bind(uid).execute(&st.db).await?;
    }
    if let Some(status) = &b.status {
        if !["online", "idle", "dnd", "invisible"].contains(&status.as_str()) {
            return bad("Unknown status");
        }
        sqlx::query("UPDATE users SET status = $1 WHERE id = $2").bind(status).bind(uid).execute(&st.db).await?;
        if status == "online" {
            st.hub.status.remove(&uid);
        } else {
            st.hub.status.insert(uid, status.clone());
        }
        super::stats::broadcast(&st, uid).await;
    }
    announce(&st, uid).await?;
    Ok(Json(me_json(&st, uid).await?))
}

async fn set_image(st: &AppState, uid: Uuid, column: &str, multipart: Multipart) -> Result<Json<Value>> {
    let name = save_image(st, multipart).await?;
    let old: Option<String> = sqlx::query_scalar(&format!("SELECT {column} FROM users WHERE id = $1")).bind(uid).fetch_one(&st.db).await?;
    sqlx::query(&format!("UPDATE users SET {column} = $1 WHERE id = $2")).bind(&name).bind(uid).execute(&st.db).await?;
    remove_upload(st, old).await;
    announce(st, uid).await?;
    Ok(Json(me_json(st, uid).await?))
}

pub async fn set_avatar(State(st): State<AppState>, AuthUser(uid): AuthUser, multipart: Multipart) -> Result<Json<Value>> {
    set_image(&st, uid, "avatar", multipart).await
}

pub async fn set_banner(State(st): State<AppState>, AuthUser(uid): AuthUser, multipart: Multipart) -> Result<Json<Value>> {
    set_image(&st, uid, "banner", multipart).await
}

async fn clear_image(st: &AppState, uid: Uuid, column: &str) -> Result<Json<Value>> {
    let old: Option<String> = sqlx::query_scalar(&format!("SELECT {column} FROM users WHERE id = $1")).bind(uid).fetch_one(&st.db).await?;
    sqlx::query(&format!("UPDATE users SET {column} = NULL WHERE id = $1")).bind(uid).execute(&st.db).await?;
    remove_upload(st, old).await;
    announce(st, uid).await?;
    Ok(Json(me_json(st, uid).await?))
}

pub async fn clear_avatar(State(st): State<AppState>, AuthUser(uid): AuthUser) -> Result<Json<Value>> {
    clear_image(&st, uid, "avatar").await
}

pub async fn clear_banner(State(st): State<AppState>, AuthUser(uid): AuthUser) -> Result<Json<Value>> {
    clear_image(&st, uid, "banner").await
}

pub async fn get_user(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(id): Path<Uuid>) -> Result<Json<PublicUser>> {
    let mut user = public_user(&st, id).await?;
    if id != uid {
        user.note = sqlx::query_scalar("SELECT note FROM user_notes WHERE owner_id = $1 AND target_id = $2").bind(uid).bind(id).fetch_optional(&st.db).await?;
    }
    Ok(Json(user))
}

#[derive(Deserialize)]
pub struct NoteBody {
    note: String,
}

pub async fn set_note(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(id): Path<Uuid>, Json(b): Json<NoteBody>) -> Result<Json<Value>> {
    let note = b.note.trim();
    if note.chars().count() > 256 {
        return bad("Note: at most 256 characters");
    }
    if id == uid {
        return bad("You cannot add a note to yourself");
    }
    if note.is_empty() {
        sqlx::query("DELETE FROM user_notes WHERE owner_id = $1 AND target_id = $2").bind(uid).bind(id).execute(&st.db).await?;
    } else {
        sqlx::query("INSERT INTO user_notes (owner_id, target_id, note) VALUES ($1, $2, $3) ON CONFLICT (owner_id, target_id) DO UPDATE SET note = $3, updated_at = now()")
            .bind(uid)
            .bind(id)
            .bind(note)
            .execute(&st.db)
            .await?;
    }
    Ok(Json(json!({ "note": note })))
}

pub async fn close_dm(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(cid): Path<Uuid>) -> Result<Json<Value>> {
    let done = sqlx::query("UPDATE dm_recipients r SET hidden = true FROM channels c WHERE c.id = r.channel_id AND c.kind = 'dm' AND r.channel_id = $1 AND r.user_id = $2")
        .bind(cid)
        .bind(uid)
        .execute(&st.db)
        .await?;
    if done.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    st.hub.send(&[uid], "DM_DELETE", json!({ "id": cid }));
    Ok(Json(json!({})))
}

pub async fn reopen_dm(st: &AppState, cid: Uuid) -> Result<()> {
    let back: Vec<Uuid> = sqlx::query_scalar("UPDATE dm_recipients SET hidden = false WHERE channel_id = $1 AND hidden RETURNING user_id").bind(cid).fetch_all(&st.db).await?;
    if !back.is_empty() {
        let dm = dm_json(st, cid).await?;
        st.hub.send(&back, "DM_CREATE", &dm);
    }
    Ok(())
}

pub async fn dm_json(st: &AppState, channel_id: Uuid) -> Result<Value> {
    let mut recipients: Vec<PublicUser> = sqlx::query_as(&format!(
        "SELECT {USER_COLS} FROM users WHERE id IN (SELECT user_id FROM dm_recipients WHERE channel_id = $1)"
    ))
    .bind(channel_id)
    .fetch_all(&st.db)
    .await?;
    for r in &mut recipients {
        decorate(st, r);
    }
    let (kind, name, owner, icon, settings): (String, String, Option<Uuid>, Option<String>, Value) =
        sqlx::query_as("SELECT kind, name, owner_id, icon, settings FROM channels WHERE id = $1").bind(channel_id).fetch_one(&st.db).await?;
    let roles: Vec<(Uuid, i16)> = sqlx::query_as("SELECT user_id, role FROM dm_recipients WHERE channel_id = $1").bind(channel_id).fetch_all(&st.db).await?;
    let roles: serde_json::Map<String, Value> = roles.into_iter().map(|(u, r)| (u.to_string(), json!(r))).collect();
    let voice: Vec<Uuid> = st.hub.voice.iter().filter(|e| *e.value() == channel_id).map(|e| *e.key()).collect();
    Ok(json!({ "id": channel_id, "kind": kind, "name": name, "owner_id": owner, "icon": icon, "settings": super::groups::settings(&settings), "roles": roles, "recipients": recipients, "voice": voice }))
}

#[derive(Deserialize)]
pub struct OpenDm {
    user_id: Uuid,
}

pub async fn open_dm(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(b): Json<OpenDm>) -> Result<Json<Value>> {
    crate::routes::account::require_verified(&st, uid).await?;
    if b.user_id == uid {
        return bad("You cannot message yourself");
    }
    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT a.channel_id FROM dm_recipients a JOIN dm_recipients b ON b.channel_id = a.channel_id JOIN channels c ON c.id = a.channel_id WHERE a.user_id = $1 AND b.user_id = $2 AND c.kind = 'dm'",
    )
    .bind(uid)
    .bind(b.user_id)
    .fetch_optional(&st.db)
    .await?;
    if let Some(id) = existing {
        sqlx::query("UPDATE dm_recipients SET hidden = false WHERE channel_id = $1 AND user_id = $2").bind(id).bind(uid).execute(&st.db).await?;
        return Ok(Json(dm_json(&st, id).await?));
    }
    let shared: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM members a JOIN members b USING (guild_id) WHERE a.user_id = $1 AND b.user_id = $2)",
    )
    .bind(uid)
    .bind(b.user_id)
    .fetch_one(&st.db)
    .await?;
    if (!shared && !super::friends::are_friends(&st, uid, b.user_id).await?) || super::friends::blocked_between(&st, uid, b.user_id).await? {
        return Err(AppError::Forbidden);
    }
    let id = Uuid::now_v7();
    let mut tx = st.db.begin().await?;
    sqlx::query("INSERT INTO channels (id, kind) VALUES ($1, 'dm')").bind(id).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO dm_recipients (channel_id, user_id) VALUES ($1, $2), ($1, $3)")
        .bind(id)
        .bind(uid)
        .bind(b.user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let dm = dm_json(&st, id).await?;
    st.hub.send(&[uid, b.user_id], "DM_CREATE", &dm);
    Ok(Json(dm))
}
