use super::{channels::access, friends::{are_friends, blocked_between}, remove_upload, save_image, users::dm_json, voice};
use crate::{auth::AuthUser, error::{bad, AppError, Result}, AppState};
use axum::{extract::{Multipart, Path, State}, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

const MAX_MEMBERS: usize = 10;
pub const MEMBER: i16 = 0;
pub const MODERATOR: i16 = 1;
pub const ADMIN: i16 = 2;
const RULES: [(&str, i16); 5] = [("invite", MEMBER), ("edit", MODERATOR), ("kick", MODERATOR), ("delete", MODERATOR), ("pin", MODERATOR)];

pub fn settings(raw: &Value) -> Value {
    let mut out = json!({});
    for (key, default) in RULES {
        out[key] = json!(raw[key].as_i64().map(|v| v.clamp(0, 2) as i16).unwrap_or(default));
    }
    out
}

pub struct Group {
    pub role: i16,
    pub owner: Option<Uuid>,
    rules: Value,
}

impl Group {
    pub fn may(&self, rule: &str) -> bool {
        self.role as i64 >= self.rules[rule].as_i64().unwrap_or(2)
    }

    fn require(&self, rule: &str) -> Result<()> {
        if self.may(rule) { Ok(()) } else { Err(AppError::Forbidden) }
    }
}

pub async fn recipients(st: &AppState, cid: Uuid) -> Result<Vec<Uuid>> {
    Ok(sqlx::query_scalar("SELECT user_id FROM dm_recipients WHERE channel_id = $1").bind(cid).fetch_all(&st.db).await?)
}

pub async fn group(st: &AppState, uid: Uuid, cid: Uuid) -> Result<Group> {
    let a = access(st, uid, cid, 0).await?;
    if a.channel.kind != "group" {
        return bad("This is not a group chat");
    }
    let (owner, raw, role): (Option<Uuid>, Value, i16) = sqlx::query_as(
        "SELECT c.owner_id, c.settings, r.role FROM channels c JOIN dm_recipients r ON r.channel_id = c.id WHERE c.id = $1 AND r.user_id = $2",
    )
    .bind(cid)
    .bind(uid)
    .fetch_one(&st.db)
    .await?;
    Ok(Group { role, owner, rules: settings(&raw) })
}

async fn role_of(st: &AppState, cid: Uuid, uid: Uuid) -> Result<i16> {
    let role: Option<i16> = sqlx::query_scalar("SELECT role FROM dm_recipients WHERE channel_id = $1 AND user_id = $2").bind(cid).bind(uid).fetch_optional(&st.db).await?;
    role.ok_or(AppError::NotFound)
}

fn clean(name: &str) -> Result<String> {
    let name = name.trim();
    if name.chars().count() > 60 {
        return bad("Name: at most 60 characters");
    }
    Ok(name.to_string())
}

async fn updated(st: &AppState, cid: Uuid) -> Result<Value> {
    let dm = dm_json(st, cid).await?;
    st.hub.send(&recipients(st, cid).await?, "DM_UPDATE", &dm);
    Ok(dm)
}

#[derive(Deserialize)]
pub struct CreateBody {
    user_ids: Vec<Uuid>,
    name: Option<String>,
}

pub async fn create(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(b): Json<CreateBody>) -> Result<Json<Value>> {
    let mut others = b.user_ids;
    others.sort();
    others.dedup();
    others.retain(|u| *u != uid);
    if others.is_empty() || others.len() >= MAX_MEMBERS {
        return bad("A group has 2 to 10 members");
    }
    for u in &others {
        if !are_friends(&st, uid, *u).await? {
            return bad("You can only add friends to a group");
        }
    }
    let name = clean(b.name.as_deref().unwrap_or(""))?;
    let id = Uuid::now_v7();
    let mut tx = st.db.begin().await?;
    sqlx::query("INSERT INTO channels (id, kind, name, owner_id) VALUES ($1, 'group', $2, $3)").bind(id).bind(name).bind(uid).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO dm_recipients (channel_id, user_id, role) VALUES ($1, $2, $3)").bind(id).bind(uid).bind(ADMIN).execute(&mut *tx).await?;
    for u in &others {
        sqlx::query("INSERT INTO dm_recipients (channel_id, user_id) VALUES ($1, $2)").bind(id).bind(u).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    others.push(uid);
    let dm = dm_json(&st, id).await?;
    st.hub.send(&others, "DM_CREATE", &dm);
    Ok(Json(dm))
}

#[derive(Deserialize)]
pub struct UpdateBody {
    name: Option<String>,
    settings: Option<Value>,
}

pub async fn update(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(cid): Path<Uuid>, Json(b): Json<UpdateBody>) -> Result<Json<Value>> {
    let g = group(&st, uid, cid).await?;
    if let Some(name) = &b.name {
        g.require("edit")?;
        sqlx::query("UPDATE channels SET name = $1 WHERE id = $2").bind(clean(name)?).bind(cid).execute(&st.db).await?;
    }
    if let Some(raw) = &b.settings {
        if g.role < ADMIN {
            return Err(AppError::Forbidden);
        }
        sqlx::query("UPDATE channels SET settings = $1 WHERE id = $2").bind(settings(raw)).bind(cid).execute(&st.db).await?;
    }
    Ok(Json(updated(&st, cid).await?))
}

pub async fn set_icon(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(cid): Path<Uuid>, multipart: Multipart) -> Result<Json<Value>> {
    group(&st, uid, cid).await?.require("edit")?;
    let name = save_image(&st, multipart).await?;
    let old: Option<String> = sqlx::query_scalar("SELECT icon FROM channels WHERE id = $1").bind(cid).fetch_one(&st.db).await?;
    sqlx::query("UPDATE channels SET icon = $1 WHERE id = $2").bind(&name).bind(cid).execute(&st.db).await?;
    remove_upload(&st, old).await;
    Ok(Json(updated(&st, cid).await?))
}

pub async fn clear_icon(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(cid): Path<Uuid>) -> Result<Json<Value>> {
    group(&st, uid, cid).await?.require("edit")?;
    let old: Option<String> = sqlx::query_scalar("SELECT icon FROM channels WHERE id = $1").bind(cid).fetch_one(&st.db).await?;
    sqlx::query("UPDATE channels SET icon = NULL WHERE id = $1").bind(cid).execute(&st.db).await?;
    remove_upload(&st, old).await;
    Ok(Json(updated(&st, cid).await?))
}

pub async fn add(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((cid, target)): Path<(Uuid, Uuid)>) -> Result<Json<Value>> {
    group(&st, uid, cid).await?.require("invite")?;
    let members = recipients(&st, cid).await?;
    if members.contains(&target) {
        return bad("Already in this group");
    }
    if members.len() >= MAX_MEMBERS {
        return bad("A group has 2 to 10 members");
    }
    if !are_friends(&st, uid, target).await? || blocked_between(&st, uid, target).await? {
        return bad("You can only add friends to a group");
    }
    sqlx::query("INSERT INTO dm_recipients (channel_id, user_id) VALUES ($1, $2)").bind(cid).bind(target).execute(&st.db).await?;
    let dm = dm_json(&st, cid).await?;
    st.hub.send(&[target], "DM_CREATE", &dm);
    st.hub.send(&members, "DM_UPDATE", &dm);
    Ok(Json(dm))
}

#[derive(Deserialize)]
pub struct RoleBody {
    role: i16,
}

pub async fn set_role(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((cid, target)): Path<(Uuid, Uuid)>, Json(b): Json<RoleBody>) -> Result<Json<Value>> {
    let g = group(&st, uid, cid).await?;
    if g.role < ADMIN || !(MEMBER..=ADMIN).contains(&b.role) {
        return Err(AppError::Forbidden);
    }
    role_of(&st, cid, target).await?;
    if g.owner == Some(target) && target != uid {
        return Err(AppError::Forbidden);
    }
    if g.owner == Some(uid) && target == uid && b.role < ADMIN {
        return bad("The owner stays administrator");
    }
    sqlx::query("UPDATE dm_recipients SET role = $1 WHERE channel_id = $2 AND user_id = $3").bind(b.role).bind(cid).bind(target).execute(&st.db).await?;
    let admins: i64 = sqlx::query_scalar("SELECT count(*) FROM dm_recipients WHERE channel_id = $1 AND role = $2").bind(cid).bind(ADMIN).fetch_one(&st.db).await?;
    if admins == 0 {
        sqlx::query("UPDATE dm_recipients SET role = $1 WHERE channel_id = $2 AND user_id = $3").bind(ADMIN).bind(cid).bind(target).execute(&st.db).await?;
        return bad("A group needs at least one administrator");
    }
    Ok(Json(updated(&st, cid).await?))
}

pub async fn remove(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((cid, target)): Path<(Uuid, Uuid)>) -> Result<Json<Value>> {
    let g = group(&st, uid, cid).await?;
    if target != uid {
        g.require("kick")?;
        if g.owner == Some(target) || (g.owner != Some(uid) && role_of(&st, cid, target).await? >= g.role) {
            return Err(AppError::Forbidden);
        }
    }
    if st.hub.voice.get(&target).is_some_and(|c| *c == cid) {
        voice::leave_all(&st, target).await;
    }
    let done = sqlx::query("DELETE FROM dm_recipients WHERE channel_id = $1 AND user_id = $2").bind(cid).bind(target).execute(&st.db).await?;
    if done.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    st.hub.send(&[target], "DM_DELETE", json!({ "id": cid }));
    let rest: Vec<(Uuid, i16)> = sqlx::query_as("SELECT user_id, role FROM dm_recipients WHERE channel_id = $1 ORDER BY role DESC, user_id").bind(cid).fetch_all(&st.db).await?;
    let Some(top) = rest.first() else {
        let mut files: Vec<String> = sqlx::query_scalar("SELECT file FROM attachments WHERE channel_id = $1").bind(cid).fetch_all(&st.db).await?;
        let icon: Option<String> = sqlx::query_scalar("DELETE FROM channels WHERE id = $1 RETURNING icon").bind(cid).fetch_one(&st.db).await?;
        files.extend(icon);
        for f in files {
            remove_upload(&st, Some(f)).await;
        }
        return Ok(Json(json!({})));
    };
    if top.1 < ADMIN {
        sqlx::query("UPDATE dm_recipients SET role = $1 WHERE channel_id = $2 AND user_id = $3").bind(ADMIN).bind(cid).bind(top.0).execute(&st.db).await?;
    }
    if g.owner == Some(target) {
        sqlx::query("UPDATE channels SET owner_id = $1 WHERE id = $2").bind(top.0).bind(cid).execute(&st.db).await?;
    }
    updated(&st, cid).await?;
    Ok(Json(json!({})))
}
