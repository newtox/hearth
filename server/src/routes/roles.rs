use super::{audit, clean_color, clean_name, refresh};
use crate::{auth::AuthUser, error::{bad, AppError, Result}, models::Role, perms, AppState};
use axum::{extract::{Path, State}, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct RoleBody {
    name: Option<String>,
    color: Option<String>,
    permissions: Option<i64>,
    hoist: Option<bool>,
}

fn check_grant(ctx: &perms::Ctx, old: i64, new: i64) -> Result<()> {
    if new & !perms::ALL != 0 || new < 0 {
        return bad("Unknown permission");
    }
    if (old ^ new) & !ctx.base != 0 {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

pub async fn create(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(gid): Path<Uuid>, Json(b): Json<RoleBody>) -> Result<Json<Role>> {
    let ctx = perms::ctx(&st.db, gid, uid).await?;
    ctx.require(perms::MANAGE_ROLES)?;
    let name = clean_name(b.name.as_deref().unwrap_or("New role"), 40, "Name")?;
    let color = clean_color(&b.color)?;
    let permissions = b.permissions.unwrap_or(0);
    check_grant(&ctx, 0, permissions)?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM roles WHERE guild_id = $1").bind(gid).fetch_one(&st.db).await?;
    if count >= 100 {
        return bad("At most 100 roles per server");
    }
    let mut tx = st.db.begin().await?;
    sqlx::query("UPDATE roles SET position = position + 1 WHERE guild_id = $1 AND NOT is_default").bind(gid).execute(&mut *tx).await?;
    let role: Role = sqlx::query_as(
        "INSERT INTO roles (id, guild_id, name, color, permissions, position, hoist) VALUES ($1, $2, $3, $4, $5, 1, $6) RETURNING *",
    )
    .bind(Uuid::now_v7())
    .bind(gid)
    .bind(name)
    .bind(color)
    .bind(permissions)
    .bind(b.hoist.unwrap_or(false))
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    audit(&st, gid, uid, "role_create", &role.name, "").await;
    refresh(&st, gid).await?;
    Ok(Json(role))
}

async fn editable(st: &AppState, ctx: &perms::Ctx, gid: Uuid, rid: Uuid) -> Result<Role> {
    ctx.require(perms::MANAGE_ROLES)?;
    let role: Role = sqlx::query_as("SELECT * FROM roles WHERE id = $1 AND guild_id = $2").bind(rid).bind(gid).fetch_one(&st.db).await?;
    if role.position >= ctx.top && !ctx.owner {
        return Err(AppError::Forbidden);
    }
    Ok(role)
}

pub async fn update(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((gid, rid)): Path<(Uuid, Uuid)>, Json(b): Json<RoleBody>) -> Result<Json<Role>> {
    let ctx = perms::ctx(&st.db, gid, uid).await?;
    let role = editable(&st, &ctx, gid, rid).await?;
    let permissions = b.permissions.unwrap_or(role.permissions);
    check_grant(&ctx, role.permissions, permissions)?;
    let (name, color, hoist) = if role.is_default {
        (role.name.clone(), None, false)
    } else {
        (
            match &b.name { Some(n) => clean_name(n, 40, "Name")?, None => role.name.clone() },
            if b.color.is_some() { clean_color(&b.color)? } else { role.color.clone() },
            b.hoist.unwrap_or(role.hoist),
        )
    };
    let role: Role = sqlx::query_as("UPDATE roles SET name = $1, color = $2, permissions = $3, hoist = $4 WHERE id = $5 RETURNING *")
        .bind(name)
        .bind(color)
        .bind(permissions)
        .bind(hoist)
        .bind(rid)
        .fetch_one(&st.db)
        .await?;
    audit(&st, gid, uid, "role_update", &role.name, "").await;
    refresh(&st, gid).await?;
    Ok(Json(role))
}

pub async fn delete(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((gid, rid)): Path<(Uuid, Uuid)>) -> Result<Json<Value>> {
    let ctx = perms::ctx(&st.db, gid, uid).await?;
    let role = editable(&st, &ctx, gid, rid).await?;
    if role.is_default {
        return bad("The default role cannot be deleted");
    }
    let mut tx = st.db.begin().await?;
    sqlx::query("DELETE FROM overwrites WHERE target_id = $1").bind(rid).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM roles WHERE id = $1").bind(rid).execute(&mut *tx).await?;
    sqlx::query("UPDATE roles SET position = position - 1 WHERE guild_id = $1 AND position > $2").bind(gid).bind(role.position).execute(&mut *tx).await?;
    tx.commit().await?;
    audit(&st, gid, uid, "role_delete", &role.name, "").await;
    refresh(&st, gid).await?;
    Ok(Json(json!({})))
}

pub async fn reorder(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(gid): Path<Uuid>, Json(order): Json<Vec<Uuid>>) -> Result<Json<Value>> {
    let ctx = perms::ctx(&st.db, gid, uid).await?;
    ctx.require(perms::MANAGE_ROLES)?;
    let current: Vec<(Uuid, i32)> = sqlx::query_as("SELECT id, position FROM roles WHERE guild_id = $1 AND NOT is_default").bind(gid).fetch_all(&st.db).await?;
    let mut sorted = order.clone();
    sorted.sort();
    sorted.dedup();
    if sorted.len() != current.len() || !current.iter().all(|(id, _)| order.contains(id)) {
        return bad("The role list is incomplete");
    }
    let n = order.len() as i32;
    let mut tx = st.db.begin().await?;
    for (i, id) in order.iter().enumerate() {
        let new = n - i as i32;
        let old = current.iter().find(|c| c.0 == *id).map(|c| c.1).unwrap_or(0);
        if !ctx.owner && ((old >= ctx.top && new != old) || (old < ctx.top && new >= ctx.top)) {
            return Err(AppError::Forbidden);
        }
        sqlx::query("UPDATE roles SET position = $1 WHERE id = $2").bind(new).bind(id).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    refresh(&st, gid).await?;
    Ok(Json(json!({})))
}
