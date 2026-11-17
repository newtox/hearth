use super::{audit, clean_name, guild_audience, refresh, remove_upload, save_image};
use crate::{auth::AuthUser, error::{bad, AppError, Result}, models::{Channel, Emoji, Guild, Member, Role, CHANNEL_COLS}, perms::{self, Overwrite}, AppState};
use axum::{extract::{Multipart, Path, State}, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

pub async fn snapshot(st: &AppState, guild_id: Uuid, user_id: Uuid) -> Result<Value> {
    let ctx = perms::ctx(&st.db, guild_id, user_id).await?;
    let guild: Guild = sqlx::query_as("SELECT id, name, description, icon, owner_id, created_at FROM guilds WHERE id = $1")
        .bind(guild_id)
        .fetch_one(&st.db)
        .await?;
    let roles: Vec<Role> = sqlx::query_as("SELECT * FROM roles WHERE guild_id = $1 ORDER BY position DESC, id").bind(guild_id).fetch_all(&st.db).await?;
    let channels: Vec<Channel> = sqlx::query_as(&format!("SELECT {CHANNEL_COLS} FROM channels WHERE guild_id = $1 ORDER BY position, id"))
        .bind(guild_id)
        .fetch_all(&st.db)
        .await?;
    let overwrites: Vec<Overwrite> = sqlx::query_as(
        "SELECT o.channel_id, o.target_id, o.kind, o.allow, o.deny FROM overwrites o JOIN channels c ON c.id = o.channel_id WHERE c.guild_id = $1",
    )
    .bind(guild_id)
    .fetch_all(&st.db)
    .await?;
    let channels: Vec<Value> = channels
        .iter()
        .filter_map(|c| {
            let ows: Vec<&Overwrite> = overwrites.iter().filter(|o| o.channel_id == c.id).collect();
            let p = ctx.in_channel(&ows);
            if p & perms::VIEW_CHANNEL == 0 {
                return None;
            }
            let mut v = serde_json::to_value(c).ok()?;
            v["overwrites"] = json!(ows);
            v["permissions"] = json!(p);
            Some(v)
        })
        .collect();
    let mut members: Vec<Member> = sqlx::query_as(
        "SELECT u.id, u.username, u.display_name, u.avatar, m.nickname, m.joined_at, m.timeout_until, \
         COALESCE(array_agg(mr.role_id) FILTER (WHERE mr.role_id IS NOT NULL), '{}') AS roles \
         FROM members m JOIN users u ON u.id = m.user_id \
         LEFT JOIN member_roles mr ON mr.guild_id = m.guild_id AND mr.user_id = m.user_id \
         WHERE m.guild_id = $1 GROUP BY u.id, m.nickname, m.joined_at, m.timeout_until ORDER BY u.username",
    )
    .bind(guild_id)
    .fetch_all(&st.db)
    .await?;
    for m in &mut members {
        m.online = st.hub.is_online(&m.id);
        m.activity = st.hub.activity.get(&m.id).map(|a| a.clone());
        m.streaming = st.hub.streaming.contains_key(&m.id);
    }
    let mut v = serde_json::to_value(guild).map_err(|e| AppError::Internal(e.to_string()))?;
    v["roles"] = json!(roles);
    v["channels"] = json!(channels);
    v["members"] = json!(members);
    let emojis: Vec<Emoji> = sqlx::query_as("SELECT id, kind, name, file FROM emojis WHERE guild_id = $1 ORDER BY name").bind(guild_id).fetch_all(&st.db).await?;
    let voice: Vec<Value> = st
        .hub
        .voice
        .iter()
        .filter(|e| channels.iter().any(|c| c["id"] == json!(e.value())))
        .map(|e| json!({ "user_id": e.key(), "channel_id": e.value() }))
        .collect();
    v["emojis"] = json!(emojis);
    v["voice_states"] = json!(voice);
    v["permissions"] = json!(ctx.base);
    Ok(v)
}

#[derive(Deserialize)]
pub struct CreateGuild {
    name: String,
    category: Option<String>,
    channel: Option<String>,
}

pub async fn create(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(b): Json<CreateGuild>) -> Result<Json<Value>> {
    let name = clean_name(&b.name, 60, "Name")?;
    let category = clean_name(b.category.as_deref().unwrap_or("General"), 60, "Name")?;
    let channel = clean_name(b.channel.as_deref().unwrap_or("general"), 60, "Name")?;
    let owned: i64 = sqlx::query_scalar("SELECT count(*) FROM guilds WHERE owner_id = $1").bind(uid).fetch_one(&st.db).await?;
    if owned >= 50 {
        return bad("You already own 50 servers");
    }
    let (gid, cat, chan) = (Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());
    let mut tx = st.db.begin().await?;
    sqlx::query("INSERT INTO guilds (id, name, owner_id) VALUES ($1, $2, $3)").bind(gid).bind(&name).bind(uid).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO roles (id, guild_id, name, permissions, position, is_default) VALUES ($1, $2, '@everyone', $3, 0, true)")
        .bind(Uuid::now_v7())
        .bind(gid)
        .bind(perms::DEFAULT)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO members (guild_id, user_id) VALUES ($1, $2)").bind(gid).bind(uid).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO channels (id, guild_id, kind, name, position) VALUES ($1, $2, 'category', $3, 0)")
        .bind(cat)
        .bind(gid)
        .bind(category)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO channels (id, guild_id, kind, name, parent_id, position) VALUES ($1, $2, 'text', $4, $3, 1)")
        .bind(chan)
        .bind(gid)
        .bind(cat)
        .bind(channel)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    refresh(&st, gid).await?;
    Ok(Json(snapshot(&st, gid, uid).await?))
}

pub async fn get(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(gid): Path<Uuid>) -> Result<Json<Value>> {
    Ok(Json(snapshot(&st, gid, uid).await?))
}

#[derive(Deserialize)]
pub struct UpdateGuild {
    name: Option<String>,
    description: Option<String>,
    owner_id: Option<Uuid>,
}

pub async fn update(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(gid): Path<Uuid>, Json(b): Json<UpdateGuild>) -> Result<Json<Value>> {
    let ctx = perms::ctx(&st.db, gid, uid).await?;
    ctx.require(perms::MANAGE_GUILD)?;
    if let Some(n) = &b.name {
        let n = clean_name(n, 60, "Name")?;
        sqlx::query("UPDATE guilds SET name = $1 WHERE id = $2").bind(n).bind(gid).execute(&st.db).await?;
    }
    if let Some(d) = &b.description {
        if d.chars().count() > 300 {
            return bad("Description: at most 300 characters");
        }
        sqlx::query("UPDATE guilds SET description = $1 WHERE id = $2").bind(d.trim()).bind(gid).execute(&st.db).await?;
    }
    if let Some(new_owner) = b.owner_id {
        if !ctx.owner {
            return Err(AppError::Forbidden);
        }
        perms::ctx(&st.db, gid, new_owner).await?;
        sqlx::query("UPDATE guilds SET owner_id = $1 WHERE id = $2").bind(new_owner).bind(gid).execute(&st.db).await?;
    }
    audit(&st, gid, uid, "server_update", "", "").await;
    refresh(&st, gid).await?;
    Ok(Json(snapshot(&st, gid, uid).await?))
}

pub async fn audit_log(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(gid): Path<Uuid>) -> Result<Json<Value>> {
    perms::ctx(&st.db, gid, uid).await?.require(perms::MANAGE_GUILD)?;
    let rows: Vec<(Uuid, String, String, String, String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT a.id, u.username, a.action, a.target, a.detail, a.created_at FROM audit_log a JOIN users u ON u.id = a.actor_id WHERE a.guild_id = $1 ORDER BY a.id DESC LIMIT 200",
    )
    .bind(gid)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(|r| json!({ "id": r.0, "actor": r.1, "action": r.2, "target": r.3, "detail": r.4, "created_at": r.5 }))
        .collect::<Vec<_>>())))
}

pub async fn set_icon(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(gid): Path<Uuid>, multipart: Multipart) -> Result<Json<Value>> {
    perms::ctx(&st.db, gid, uid).await?.require(perms::MANAGE_GUILD)?;
    let name = save_image(&st, multipart).await?;
    let old: Option<String> = sqlx::query_scalar("SELECT icon FROM guilds WHERE id = $1").bind(gid).fetch_one(&st.db).await?;
    sqlx::query("UPDATE guilds SET icon = $1 WHERE id = $2").bind(&name).bind(gid).execute(&st.db).await?;
    remove_upload(&st, old).await;
    refresh(&st, gid).await?;
    Ok(Json(json!({ "icon": name })))
}

pub async fn delete(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(gid): Path<Uuid>) -> Result<Json<Value>> {
    if !perms::ctx(&st.db, gid, uid).await?.owner {
        return Err(AppError::Forbidden);
    }
    let users = guild_audience(&st, gid).await?;
    let icon: Option<String> = sqlx::query_scalar("SELECT icon FROM guilds WHERE id = $1").bind(gid).fetch_one(&st.db).await?;
    sqlx::query("DELETE FROM guilds WHERE id = $1").bind(gid).execute(&st.db).await?;
    remove_upload(&st, icon).await;
    st.hub.send(&users, "GUILD_DELETE", json!({ "guild_id": gid }));
    Ok(Json(json!({})))
}
