use super::{audit, guild_audience, refresh, user_label};
use crate::{auth::AuthUser, error::{bad, AppError, Result}, perms, AppState};
use axum::{extract::{Path, State}, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct UpdateMember {
    nickname: Option<String>,
    roles: Option<Vec<Uuid>>,
    timeout_minutes: Option<i64>,
}

async fn outranks(st: &AppState, ctx: &perms::Ctx, gid: Uuid, target: Uuid) -> Result<()> {
    if ctx.owner {
        return Ok(());
    }
    if perms::top_position(&st.db, gid, target).await? >= ctx.top {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

pub async fn update(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((gid, target)): Path<(Uuid, Uuid)>, Json(b): Json<UpdateMember>) -> Result<Json<Value>> {
    let ctx = perms::ctx(&st.db, gid, uid).await?;
    perms::ctx(&st.db, gid, target).await?;
    if let Some(nick) = &b.nickname {
        if target == uid {
            if !ctx.has(perms::CHANGE_NICKNAME) && !ctx.has(perms::MANAGE_NICKNAMES) {
                return Err(AppError::Forbidden);
            }
        } else {
            ctx.require(perms::MANAGE_NICKNAMES)?;
            outranks(&st, &ctx, gid, target).await?;
        }
        let nick = nick.trim();
        if nick.chars().count() > 32 {
            return bad("Nickname: at most 32 characters");
        }
        sqlx::query("UPDATE members SET nickname = $1 WHERE guild_id = $2 AND user_id = $3")
            .bind(if nick.is_empty() { None } else { Some(nick) })
            .bind(gid)
            .bind(target)
            .execute(&st.db)
            .await?;
    }
    if let Some(wanted) = &b.roles {
        ctx.require(perms::MANAGE_ROLES)?;
        let all: Vec<(Uuid, i32, bool)> = sqlx::query_as("SELECT id, position, managed IS NOT NULL FROM roles WHERE guild_id = $1 AND NOT is_default").bind(gid).fetch_all(&st.db).await?;
        let current: Vec<Uuid> = sqlx::query_scalar("SELECT role_id FROM member_roles WHERE guild_id = $1 AND user_id = $2")
            .bind(gid)
            .bind(target)
            .fetch_all(&st.db)
            .await?;
        let mut tx = st.db.begin().await?;
        for (id, position, managed) in &all {
            let (has, wants) = (current.contains(id), wanted.contains(id));
            if has == wants || *managed {
                continue;
            }
            if *position >= ctx.top && !ctx.owner {
                return Err(AppError::Forbidden);
            }
            if wants {
                sqlx::query("INSERT INTO member_roles (guild_id, user_id, role_id) VALUES ($1, $2, $3)").bind(gid).bind(target).bind(id).execute(&mut *tx).await?;
            } else {
                sqlx::query("DELETE FROM member_roles WHERE guild_id = $1 AND user_id = $2 AND role_id = $3").bind(gid).bind(target).bind(id).execute(&mut *tx).await?;
            }
        }
        if wanted.iter().any(|w| !all.iter().any(|a| a.0 == *w)) {
            return bad("Unknown role");
        }
        tx.commit().await?;
        audit(&st, gid, uid, "member_roles", &user_label(&st, target).await, "").await;
    }
    if let Some(minutes) = b.timeout_minutes {
        ctx.require(perms::MODERATE_MEMBERS)?;
        let t = perms::ctx(&st.db, gid, target).await?;
        if target == uid || t.owner {
            return Err(AppError::Forbidden);
        }
        outranks(&st, &ctx, gid, target).await?;
        let minutes = minutes.clamp(0, 28 * 24 * 60);
        sqlx::query("UPDATE members SET timeout_until = CASE WHEN $1 > 0 THEN now() + make_interval(mins => $1) ELSE NULL END WHERE guild_id = $2 AND user_id = $3")
            .bind(minutes as i32)
            .bind(gid)
            .bind(target)
            .execute(&st.db)
            .await?;
        let action = if minutes > 0 { "timeout" } else { "timeout_end" };
        audit(&st, gid, uid, action, &user_label(&st, target).await, &if minutes > 0 { format!("{minutes} minutes") } else { String::new() }).await;
    }
    refresh(&st, gid).await?;
    Ok(Json(json!({})))
}

async fn remove(st: &AppState, gid: Uuid, target: Uuid) -> Result<()> {
    let users = guild_audience(st, gid).await?;
    let mut tx = st.db.begin().await?;
    sqlx::query("DELETE FROM overwrites WHERE target_id = $1 AND channel_id IN (SELECT id FROM channels WHERE guild_id = $2)")
        .bind(target)
        .bind(gid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM members WHERE guild_id = $1 AND user_id = $2").bind(gid).bind(target).execute(&mut *tx).await?;
    tx.commit().await?;
    crate::routes::voice::leave_all(st, target).await;
    st.hub.send(&[target], "GUILD_DELETE", json!({ "guild_id": gid }));
    let rest: Vec<Uuid> = users.into_iter().filter(|u| *u != target).collect();
    st.hub.send(&rest, "GUILD_REFRESH", json!({ "guild_id": gid }));
    Ok(())
}

pub async fn kick(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((gid, target)): Path<(Uuid, Uuid)>) -> Result<Json<Value>> {
    let ctx = perms::ctx(&st.db, gid, uid).await?;
    if target == uid {
        if ctx.owner {
            return bad("Transfer ownership first or delete the server");
        }
    } else {
        ctx.require(perms::KICK_MEMBERS)?;
        let t = perms::ctx(&st.db, gid, target).await?;
        if t.owner {
            return Err(AppError::Forbidden);
        }
        outranks(&st, &ctx, gid, target).await?;
        audit(&st, gid, uid, "kick", &user_label(&st, target).await, "").await;
    }
    remove(&st, gid, target).await?;
    Ok(Json(json!({})))
}

#[derive(Deserialize, Default)]
pub struct BanBody {
    #[serde(default)]
    reason: String,
}

pub async fn ban(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((gid, target)): Path<(Uuid, Uuid)>, body: Option<Json<BanBody>>) -> Result<Json<Value>> {
    let ctx = perms::ctx(&st.db, gid, uid).await?;
    ctx.require(perms::BAN_MEMBERS)?;
    if target == uid {
        return bad("You cannot ban yourself");
    }
    let owner: Uuid = sqlx::query_scalar("SELECT owner_id FROM guilds WHERE id = $1").bind(gid).fetch_one(&st.db).await?;
    if owner == target {
        return Err(AppError::Forbidden);
    }
    outranks(&st, &ctx, gid, target).await?;
    let reason: String = body.map(|b| b.0.reason).unwrap_or_default().chars().take(300).collect();
    sqlx::query("INSERT INTO bans (guild_id, user_id, reason) VALUES ($1, $2, $3) ON CONFLICT (guild_id, user_id) DO UPDATE SET reason = EXCLUDED.reason")
        .bind(gid)
        .bind(target)
        .bind(&reason)
        .execute(&st.db)
        .await?;
    audit(&st, gid, uid, "ban", &user_label(&st, target).await, &reason).await;
    remove(&st, gid, target).await?;
    Ok(Json(json!({})))
}

pub async fn unban(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((gid, target)): Path<(Uuid, Uuid)>) -> Result<Json<Value>> {
    perms::ctx(&st.db, gid, uid).await?.require(perms::BAN_MEMBERS)?;
    sqlx::query("DELETE FROM bans WHERE guild_id = $1 AND user_id = $2").bind(gid).bind(target).execute(&st.db).await?;
    audit(&st, gid, uid, "unban", &user_label(&st, target).await, "").await;
    Ok(Json(json!({})))
}

pub async fn bans(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(gid): Path<Uuid>) -> Result<Json<Value>> {
    perms::ctx(&st.db, gid, uid).await?.require(perms::BAN_MEMBERS)?;
    let rows: Vec<(Uuid, String, String, String)> = sqlx::query_as(
        "SELECT u.id, u.username, u.display_name, b.reason FROM bans b JOIN users u ON u.id = b.user_id WHERE b.guild_id = $1 ORDER BY b.created_at DESC",
    )
    .bind(gid)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|r| json!({ "id": r.0, "username": r.1, "display_name": r.2, "reason": r.3 })).collect::<Vec<_>>())))
}

pub async fn summary(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((gid, target)): Path<(Uuid, Uuid)>) -> Result<Json<Value>> {
    let ctx = perms::ctx(&st.db, gid, uid).await?;
    let moderator = perms::ADMINISTRATOR | perms::MODERATE_MEMBERS | perms::KICK_MEMBERS | perms::BAN_MEMBERS | perms::MANAGE_ROLES | perms::MANAGE_NICKNAMES | perms::MANAGE_MESSAGES;
    if !ctx.owner && ctx.base & moderator == 0 {
        return Err(AppError::Forbidden);
    }
    perms::ctx(&st.db, gid, target).await?;
    let (messages, links, media): (i64, i64, i64) = sqlx::query_as(
        "SELECT count(*), count(*) FILTER (WHERE m.content ~* 'https?://'), count(*) FILTER (WHERE EXISTS (SELECT 1 FROM attachments a WHERE a.message_id = m.id)) \
         FROM messages m JOIN channels c ON c.id = m.channel_id WHERE c.guild_id = $1 AND m.author_id = $2",
    )
    .bind(gid)
    .bind(target)
    .fetch_one(&st.db)
    .await?;
    let audit: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE guild_id = $1 AND target = $2").bind(gid).bind(user_label(&st, target).await).fetch_one(&st.db).await?;
    let created: chrono::DateTime<chrono::Utc> = sqlx::query_scalar("SELECT created_at FROM users WHERE id = $1").bind(target).fetch_one(&st.db).await?;
    Ok(Json(json!({ "messages": messages, "links": links, "media": media, "audit": audit, "created_at": created })))
}
