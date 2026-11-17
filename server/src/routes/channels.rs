use super::{audit, clean_name, refresh};
use crate::{auth::AuthUser, error::{bad, AppError, Result}, models::{Channel, CHANNEL_COLS}, perms::{self, Overwrite}, AppState};
use axum::{extract::{Path, State}, Json};
use serde::{Deserialize, Deserializer};
use serde_json::{json, Value};
use uuid::Uuid;

pub struct Access {
    pub channel: Channel,
    pub perms: i64,
}

async fn overwrites(st: &AppState, cid: Uuid) -> Result<Vec<Overwrite>> {
    Ok(sqlx::query_as("SELECT channel_id, target_id, kind, allow, deny FROM overwrites WHERE channel_id = $1").bind(cid).fetch_all(&st.db).await?)
}

pub async fn access(st: &AppState, uid: Uuid, cid: Uuid, need: i64) -> Result<Access> {
    let channel: Channel = sqlx::query_as(&format!("SELECT {CHANNEL_COLS} FROM channels WHERE id = $1")).bind(cid).fetch_one(&st.db).await?;
    let p = match channel.guild_id {
        Some(gid) => {
            let ctx = perms::ctx(&st.db, gid, uid).await?;
            let ows = overwrites(st, channel.id).await?;
            ctx.in_channel(&ows.iter().collect::<Vec<_>>())
        }
        None => {
            let member: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM dm_recipients WHERE channel_id = $1 AND user_id = $2)")
                .bind(cid)
                .bind(uid)
                .fetch_one(&st.db)
                .await?;
            if member { perms::DM } else { 0 }
        }
    };
    if p & perms::VIEW_CHANNEL == 0 {
        return Err(AppError::NotFound);
    }
    if p & need != need {
        return Err(AppError::Forbidden);
    }
    Ok(Access { channel, perms: p })
}

pub async fn viewers(st: &AppState, channel: &Channel) -> Result<Vec<Uuid>> {
    let Some(gid) = channel.guild_id else {
        return Ok(sqlx::query_scalar("SELECT user_id FROM dm_recipients WHERE channel_id = $1").bind(channel.id).fetch_all(&st.db).await?);
    };
    let online: Vec<Uuid> = sqlx::query_scalar("SELECT user_id FROM members WHERE guild_id = $1 AND user_id = ANY($2)")
        .bind(gid)
        .bind(st.hub.online_ids())
        .fetch_all(&st.db)
        .await?;
    let ows = overwrites(st, channel.id).await?;
    let refs: Vec<&Overwrite> = ows.iter().collect();
    let mut out = Vec::with_capacity(online.len());
    for u in online {
        if let Ok(ctx) = perms::ctx(&st.db, gid, u).await {
            if ctx.in_channel(&refs) & perms::VIEW_CHANNEL != 0 {
                out.push(u);
            }
        }
    }
    Ok(out)
}

fn double<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Option<Option<Uuid>>, D::Error> {
    Option::<Uuid>::deserialize(d).map(Some)
}

#[derive(Deserialize)]
pub struct ChannelBody {
    name: Option<String>,
    kind: Option<String>,
    topic: Option<String>,
    #[serde(default, deserialize_with = "double")]
    parent_id: Option<Option<Uuid>>,
    position: Option<i32>,
}

async fn check_parent(st: &AppState, gid: Uuid, parent: Option<Uuid>, kind: &str) -> Result<()> {
    let Some(p) = parent else { return Ok(()) };
    if kind == "category" {
        return bad("Categories cannot be nested");
    }
    let ok: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM channels WHERE id = $1 AND guild_id = $2 AND kind = 'category')")
        .bind(p)
        .bind(gid)
        .fetch_one(&st.db)
        .await?;
    if ok { Ok(()) } else { bad("Unknown category") }
}

pub async fn create(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(gid): Path<Uuid>, Json(b): Json<ChannelBody>) -> Result<Json<Channel>> {
    perms::ctx(&st.db, gid, uid).await?.require(perms::MANAGE_CHANNELS)?;
    let kind = b.kind.as_deref().unwrap_or("text");
    if kind != "text" && kind != "category" && kind != "voice" {
        return bad("Unknown channel type");
    }
    let name = clean_name(b.name.as_deref().unwrap_or(""), 60, "Name")?;
    let parent = b.parent_id.flatten();
    check_parent(&st, gid, parent, kind).await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM channels WHERE guild_id = $1").bind(gid).fetch_one(&st.db).await?;
    if count >= 200 {
        return bad("At most 200 channels per server");
    }
    let channel: Channel = sqlx::query_as(&format!(
        "INSERT INTO channels (id, guild_id, kind, name, topic, parent_id, position) \
         VALUES ($1, $2, $3, $4, $5, $6, (SELECT COALESCE(max(position), 0) + 1 FROM channels WHERE guild_id = $2)) RETURNING {CHANNEL_COLS}"
    ))
    .bind(Uuid::now_v7())
    .bind(gid)
    .bind(kind)
    .bind(name)
    .bind(b.topic.unwrap_or_default().trim())
    .bind(parent)
    .fetch_one(&st.db)
    .await?;
    audit(&st, gid, uid, "channel_create", &channel.name, "").await;
    refresh(&st, gid).await?;
    Ok(Json(channel))
}

async fn guild_channel(st: &AppState, uid: Uuid, cid: Uuid) -> Result<(Channel, Uuid, perms::Ctx)> {
    let a = access(st, uid, cid, perms::MANAGE_CHANNELS).await?;
    let gid = a.channel.guild_id.ok_or(AppError::Forbidden)?;
    let ctx = perms::ctx(&st.db, gid, uid).await?;
    Ok((a.channel, gid, ctx))
}

pub async fn update(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(cid): Path<Uuid>, Json(b): Json<ChannelBody>) -> Result<Json<Channel>> {
    let (ch, gid, _) = guild_channel(&st, uid, cid).await?;
    let name = match &b.name { Some(n) => clean_name(n, 60, "Name")?, None => ch.name.clone() };
    let topic = b.topic.as_deref().map(str::trim).unwrap_or(&ch.topic).to_string();
    if topic.chars().count() > 300 {
        return bad("Topic: at most 300 characters");
    }
    let parent = b.parent_id.unwrap_or(ch.parent_id);
    check_parent(&st, gid, parent, &ch.kind).await?;
    let channel: Channel = sqlx::query_as(&format!(
        "UPDATE channels SET name = $1, topic = $2, parent_id = $3, position = $4 WHERE id = $5 RETURNING {CHANNEL_COLS}"
    ))
    .bind(name)
    .bind(topic)
    .bind(parent)
    .bind(b.position.unwrap_or(ch.position))
    .bind(cid)
    .fetch_one(&st.db)
    .await?;
    audit(&st, gid, uid, "channel_update", &channel.name, "").await;
    refresh(&st, gid).await?;
    Ok(Json(channel))
}

pub async fn delete(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(cid): Path<Uuid>) -> Result<Json<Value>> {
    let (ch, gid, _) = guild_channel(&st, uid, cid).await?;
    let files: Vec<String> = sqlx::query_scalar("SELECT file FROM attachments WHERE channel_id = $1").bind(cid).fetch_all(&st.db).await?;
    sqlx::query("DELETE FROM channels WHERE id = $1").bind(cid).execute(&st.db).await?;
    for f in files {
        super::remove_upload(&st, Some(f)).await;
    }
    st.hub.voice.retain(|_, c| *c != cid);
    audit(&st, gid, uid, "channel_delete", &ch.name, "").await;
    refresh(&st, gid).await?;
    Ok(Json(json!({})))
}

#[derive(Deserialize)]
pub struct OverwriteBody {
    kind: String,
    allow: i64,
    deny: i64,
}

pub async fn put_overwrite(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((cid, target)): Path<(Uuid, Uuid)>, Json(b): Json<OverwriteBody>) -> Result<Json<Value>> {
    let (_, gid, ctx) = guild_channel(&st, uid, cid).await?;
    ctx.require(perms::MANAGE_ROLES)?;
    if (b.allow | b.deny) & !perms::ALL != 0 || b.allow < 0 || b.deny < 0 || b.allow & b.deny != 0 {
        return bad("Invalid permissions");
    }
    let old: Option<(i64, i64)> = sqlx::query_as("SELECT allow, deny FROM overwrites WHERE channel_id = $1 AND target_id = $2")
        .bind(cid)
        .bind(target)
        .fetch_optional(&st.db)
        .await?;
    let (old_allow, old_deny) = old.unwrap_or((0, 0));
    if ((old_allow ^ b.allow) | (old_deny ^ b.deny)) & !ctx.base != 0 {
        return Err(AppError::Forbidden);
    }
    let valid: bool = match b.kind.as_str() {
        "role" => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM roles WHERE id = $1 AND guild_id = $2)").bind(target).bind(gid).fetch_one(&st.db).await?,
        "member" => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM members WHERE user_id = $1 AND guild_id = $2)").bind(target).bind(gid).fetch_one(&st.db).await?,
        _ => false,
    };
    if !valid {
        return bad("Unknown target");
    }
    sqlx::query(
        "INSERT INTO overwrites (channel_id, target_id, kind, allow, deny) VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (channel_id, target_id) DO UPDATE SET allow = EXCLUDED.allow, deny = EXCLUDED.deny",
    )
    .bind(cid)
    .bind(target)
    .bind(&b.kind)
    .bind(b.allow)
    .bind(b.deny)
    .execute(&st.db)
    .await?;
    audit(&st, gid, uid, "channel_rules", "", "").await;
    refresh(&st, gid).await?;
    Ok(Json(json!({})))
}

pub async fn delete_overwrite(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((cid, target)): Path<(Uuid, Uuid)>) -> Result<Json<Value>> {
    let (_, gid, ctx) = guild_channel(&st, uid, cid).await?;
    ctx.require(perms::MANAGE_ROLES)?;
    let old: Option<(i64, i64)> = sqlx::query_as("SELECT allow, deny FROM overwrites WHERE channel_id = $1 AND target_id = $2")
        .bind(cid)
        .bind(target)
        .fetch_optional(&st.db)
        .await?;
    if let Some((a, d)) = old {
        if (a | d) & !ctx.base != 0 {
            return Err(AppError::Forbidden);
        }
    }
    sqlx::query("DELETE FROM overwrites WHERE channel_id = $1 AND target_id = $2").bind(cid).bind(target).execute(&st.db).await?;
    refresh(&st, gid).await?;
    Ok(Json(json!({})))
}

pub async fn visible(st: &AppState, uid: Uuid, gid: Uuid) -> Result<Vec<(Uuid, String)>> {
    let ctx = perms::ctx(&st.db, gid, uid).await?;
    let channels: Vec<Channel> = sqlx::query_as(&format!("SELECT {CHANNEL_COLS} FROM channels WHERE guild_id = $1 AND kind = 'text'")).bind(gid).fetch_all(&st.db).await?;
    let overwrites: Vec<Overwrite> = sqlx::query_as(
        "SELECT o.channel_id, o.target_id, o.kind, o.allow, o.deny FROM overwrites o JOIN channels c ON c.id = o.channel_id WHERE c.guild_id = $1",
    )
    .bind(gid)
    .fetch_all(&st.db)
    .await?;
    Ok(channels
        .into_iter()
        .filter(|c| {
            let ows: Vec<&Overwrite> = overwrites.iter().filter(|o| o.channel_id == c.id).collect();
            ctx.in_channel(&ows) & perms::VIEW_CHANNEL != 0
        })
        .map(|c| (c.id, c.name))
        .collect())
}
