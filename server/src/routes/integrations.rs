use super::{audit, connections::{self, Connections, Provider}, refresh};
use crate::{auth::AuthUser, error::{AppError, Result}, perms, AppState};
use axum::{extract::{Path, State}, Json};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::collections::HashSet;
use uuid::Uuid;

const EXPIRED: &str = "The connection has expired, connect the account again";
const UNREACHABLE: &str = "The platform could not be reached, trying again later";
const NO_ACCESS: &str = "YouTube has not enabled channel membership access for this channel";
const NOT_ENABLED: &str = "Channel memberships are not enabled on this YouTube channel";

#[derive(sqlx::FromRow)]
struct Row {
    guild_id: Uuid,
    provider: String,
    account_id: String,
    role_id: Uuid,
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Option<DateTime<Utc>>,
}

fn role_style(provider: &str) -> (&'static str, &'static str) {
    match provider {
        "twitch" => ("Twitch Subscriber", "#9146ff"),
        "youtube" => ("YouTube Member", "#ff0033"),
        _ => ("Supporter", "#8888aa"),
    }
}

async fn manager(st: &AppState, gid: Uuid, uid: Uuid) -> Result<()> {
    perms::ctx(&st.db, gid, uid).await?.require(perms::MANAGE_GUILD)
}

pub async fn list(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(gid): Path<Uuid>) -> Result<Json<Value>> {
    manager(&st, gid, uid).await?;
    let available: Vec<Value> = st.connections.as_deref().map(|c| c.syncable().map(|p| json!({ "id": p.id, "name": p.name })).collect()).unwrap_or_default();
    let rows: Vec<(Uuid, String, String, Uuid, Option<DateTime<Utc>>, Option<String>, i64)> = sqlx::query_as(
        "SELECT i.id, i.provider, i.account_name, i.role_id, i.synced_at, i.error, (SELECT count(*) FROM member_roles mr WHERE mr.role_id = i.role_id) \
         FROM integrations i WHERE i.guild_id = $1 ORDER BY i.provider",
    )
    .bind(gid)
    .fetch_all(&st.db)
    .await?;
    let linked: Vec<Value> = rows
        .into_iter()
        .map(|(id, provider, name, role_id, synced_at, error, members)| {
            json!({ "id": id, "provider": provider, "account_name": name, "role_id": role_id, "synced_at": synced_at, "error": error, "members": members })
        })
        .collect();
    Ok(Json(json!({ "available": available, "linked": linked })))
}

pub async fn start(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((gid, provider)): Path<(Uuid, String)>) -> Result<Json<Value>> {
    manager(&st, gid, uid).await?;
    let c = connections::config(&st)?;
    let p = c.provider(&provider).filter(|p| p.subs.is_some()).ok_or(AppError::NotFound)?;
    let state = hex::encode(rand::random::<[u8; 16]>());
    let url = c.authorize_url(p, &state, true)?;
    sqlx::query("DELETE FROM oauth_states WHERE created_at < now() - interval '10 minutes'").execute(&st.db).await?;
    sqlx::query("INSERT INTO oauth_states (state, user_id, provider, guild_id) VALUES ($1, $2, $3, $4)").bind(&state).bind(uid).bind(&provider).bind(gid).execute(&st.db).await?;
    Ok(Json(json!({ "url": url })))
}

pub async fn finish(st: &AppState, p: &Provider, code: &str, uid: Uuid, gid: Uuid) -> bool {
    match link(st, p, code, uid, gid).await {
        Ok(id) => {
            let st = st.clone();
            tokio::spawn(async move { sync(&st, id).await.ok() });
            true
        }
        Err(_) => false,
    }
}

async fn link(st: &AppState, p: &Provider, code: &str, uid: Uuid, gid: Uuid) -> Result<Uuid> {
    manager(st, gid, uid).await?;
    let c = connections::config(st)?;
    let failed = || AppError::Bad(EXPIRED.into());
    let token = connections::exchange(c, p, code).await.ok_or_else(failed)?;
    let access = token["access_token"].as_str().ok_or_else(failed)?.to_string();
    let (account, name, _) = connections::whoami(c, p, &access).await.ok_or_else(failed)?;
    let refresh_token = token["refresh_token"].as_str().map(str::to_string);
    let expires = token["expires_in"].as_i64().map(|s| Utc::now() + chrono::Duration::seconds(s));
    let existing: Option<Uuid> = sqlx::query_scalar("SELECT id FROM integrations WHERE guild_id = $1 AND provider = $2").bind(gid).bind(p.id).fetch_optional(&st.db).await?;
    if let Some(id) = existing {
        sqlx::query(
            "UPDATE integrations SET account_id = $1, account_name = $2, access_token = $3, refresh_token = COALESCE($4, refresh_token), expires_at = $5, error = NULL WHERE id = $6",
        )
        .bind(&account)
        .bind(&name)
        .bind(&access)
        .bind(&refresh_token)
        .bind(expires)
        .bind(id)
        .execute(&st.db)
        .await?;
        return Ok(id);
    }
    let (role_name, color) = role_style(p.id);
    let (id, role_id) = (Uuid::now_v7(), Uuid::now_v7());
    let mut tx = st.db.begin().await?;
    sqlx::query("UPDATE roles SET position = position + 1 WHERE guild_id = $1 AND NOT is_default").bind(gid).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO roles (id, guild_id, name, color, permissions, position, managed) VALUES ($1, $2, $3, $4, 0, 1, $5)")
        .bind(role_id)
        .bind(gid)
        .bind(role_name)
        .bind(color)
        .bind(p.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO integrations (id, guild_id, provider, account_id, account_name, role_id, access_token, refresh_token, expires_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(id)
    .bind(gid)
    .bind(p.id)
    .bind(&account)
    .bind(&name)
    .bind(role_id)
    .bind(&access)
    .bind(&refresh_token)
    .bind(expires)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    audit(st, gid, uid, "integration_create", p.name, "").await;
    refresh(st, gid).await?;
    Ok(id)
}

async fn access(st: &AppState, c: &Connections, p: &Provider, id: Uuid, row: &Row) -> std::result::Result<String, &'static str> {
    if row.expires_at.is_none_or(|e| e > Utc::now() + chrono::Duration::seconds(60)) {
        return Ok(row.access_token.clone());
    }
    let refresh_token = row.refresh_token.as_deref().ok_or(EXPIRED)?;
    let form = [("grant_type", "refresh_token"), ("refresh_token", refresh_token), ("client_id", &p.client_id), ("client_secret", &p.secret)];
    let res = c.http.post(&p.token).header("Accept", "application/json").form(&form).send().await.map_err(|_| UNREACHABLE)?;
    if res.status().is_server_error() {
        return Err(UNREACHABLE);
    }
    let t: Value = res.json().await.map_err(|_| EXPIRED)?;
    let fresh = t["access_token"].as_str().ok_or(EXPIRED)?.to_string();
    let expires = t["expires_in"].as_i64().map(|s| Utc::now() + chrono::Duration::seconds(s));
    sqlx::query("UPDATE integrations SET access_token = $1, refresh_token = COALESCE($2, refresh_token), expires_at = $3 WHERE id = $4")
        .bind(&fresh)
        .bind(t["refresh_token"].as_str())
        .bind(expires)
        .bind(id)
        .execute(&st.db)
        .await
        .map_err(|_| UNREACHABLE)?;
    Ok(fresh)
}

async fn supporters(c: &Connections, p: &Provider, account: &str, token: &str) -> std::result::Result<HashSet<String>, &'static str> {
    let url = p.subs.as_deref().ok_or(UNREACHABLE)?;
    let youtube = p.id == "youtube";
    let mut ids = HashSet::new();
    let mut cursor: Option<String> = None;
    for _ in 0..200 {
        let mut req = c.http.get(url).bearer_auth(token).header("Client-Id", &p.client_id);
        req = if youtube {
            req.query(&[("part", "snippet"), ("mode", "all_current"), ("maxResults", "1000")])
        } else {
            req.query(&[("broadcaster_id", account), ("first", "100")])
        };
        if let Some(next) = &cursor {
            req = req.query(&[(if youtube { "pageToken" } else { "after" }, next.as_str())]);
        }
        let res = req.send().await.map_err(|_| UNREACHABLE)?;
        match res.status().as_u16() {
            401 => return Err(EXPIRED),
            403 if youtube => return Err(NO_ACCESS),
            403 => return Err(EXPIRED),
            400 if youtube => return Err(NOT_ENABLED),
            s if s >= 400 => return Err(UNREACHABLE),
            _ => {}
        }
        let v: Value = res.json().await.map_err(|_| UNREACHABLE)?;
        if youtube {
            ids.extend(v["items"].as_array().into_iter().flatten().filter_map(|it| connections::text(&it["snippet"]["memberDetails"]["channelId"])));
            cursor = connections::text(&v["nextPageToken"]);
        } else {
            ids.extend(v["data"].as_array().into_iter().flatten().filter_map(|it| connections::text(&it["user_id"])));
            cursor = connections::text(&v["pagination"]["cursor"]);
        }
        if cursor.as_deref().is_none_or(str::is_empty) {
            break;
        }
    }
    Ok(ids)
}

pub async fn sync(st: &AppState, id: Uuid) -> Result<()> {
    let Some(c) = st.connections.as_deref() else { return Ok(()) };
    let row: Option<Row> = sqlx::query_as("SELECT guild_id, provider, account_id, role_id, access_token, refresh_token, expires_at FROM integrations WHERE id = $1")
        .bind(id)
        .fetch_optional(&st.db)
        .await?;
    let Some(row) = row else { return Ok(()) };
    let Some(p) = c.provider(&row.provider) else { return Ok(()) };
    let found = match access(st, c, p, id, &row).await {
        Ok(token) => supporters(c, p, &row.account_id, &token).await,
        Err(e) => Err(e),
    };
    let ids = match found {
        Ok(ids) => ids,
        Err(e) => {
            sqlx::query("UPDATE integrations SET error = $1, synced_at = now() WHERE id = $2").bind(e).bind(id).execute(&st.db).await?;
            return Ok(());
        }
    };
    let linked: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT m.user_id, c.account_id FROM members m JOIN connections c ON c.user_id = m.user_id AND c.provider = $2 WHERE m.guild_id = $1",
    )
    .bind(row.guild_id)
    .bind(&row.provider)
    .fetch_all(&st.db)
    .await?;
    let want: HashSet<Uuid> = linked.into_iter().filter(|(_, account)| ids.contains(account)).map(|(u, _)| u).collect();
    let have: HashSet<Uuid> = sqlx::query_scalar("SELECT user_id FROM member_roles WHERE role_id = $1").bind(row.role_id).fetch_all(&st.db).await?.into_iter().collect();
    let mut changed = false;
    for u in want.difference(&have) {
        sqlx::query("INSERT INTO member_roles (guild_id, user_id, role_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING").bind(row.guild_id).bind(u).bind(row.role_id).execute(&st.db).await?;
        changed = true;
    }
    for u in have.difference(&want) {
        sqlx::query("DELETE FROM member_roles WHERE role_id = $1 AND user_id = $2").bind(row.role_id).bind(u).execute(&st.db).await?;
        changed = true;
    }
    sqlx::query("UPDATE integrations SET error = NULL, synced_at = now() WHERE id = $1").bind(id).execute(&st.db).await?;
    if changed {
        refresh(st, row.guild_id).await?;
    }
    Ok(())
}

fn sync_where(st: &AppState, sql: &'static str, uid: Uuid, key: String) {
    let st = st.clone();
    tokio::spawn(async move {
        let ids: Vec<Uuid> = sqlx::query_scalar(sql).bind(uid).bind(&key).fetch_all(&st.db).await.unwrap_or_default();
        for id in ids {
            sync(&st, id).await.ok();
        }
    });
}

pub fn sync_for_user(st: &AppState, uid: Uuid, provider: &str) {
    sync_where(st, "SELECT i.id FROM integrations i JOIN members m ON m.guild_id = i.guild_id WHERE m.user_id = $1 AND i.provider = $2", uid, provider.to_string());
}

pub fn sync_member(st: &AppState, uid: Uuid, gid: Uuid) {
    sync_where(
        st,
        "SELECT i.id FROM integrations i JOIN connections c ON c.provider = i.provider WHERE c.user_id = $1 AND i.guild_id::text = $2",
        uid,
        gid.to_string(),
    );
}

pub async fn poll(st: AppState) {
    let secs = std::env::var("INTEGRATION_SYNC_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(600u64).max(5);
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
        let ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM integrations").fetch_all(&st.db).await.unwrap_or_default();
        for id in ids {
            sync(&st, id).await.ok();
        }
    }
}

async fn owned(st: &AppState, gid: Uuid, iid: &str) -> Result<(Uuid, Uuid, String)> {
    let iid: Uuid = iid.parse().map_err(|_| AppError::NotFound)?;
    let (role_id, provider): (Uuid, String) =
        sqlx::query_as("SELECT role_id, provider FROM integrations WHERE id = $1 AND guild_id = $2").bind(iid).bind(gid).fetch_one(&st.db).await?;
    Ok((iid, role_id, provider))
}

pub async fn sync_now(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((gid, iid)): Path<(Uuid, String)>) -> Result<Json<Value>> {
    manager(&st, gid, uid).await?;
    let (iid, _, _) = owned(&st, gid, &iid).await?;
    st.limits.check(format!("isync:{gid}"), 5, std::time::Duration::from_secs(60))?;
    sync(&st, iid).await?;
    Ok(Json(json!({})))
}

pub async fn remove(State(st): State<AppState>, AuthUser(uid): AuthUser, Path((gid, iid)): Path<(Uuid, String)>) -> Result<Json<Value>> {
    manager(&st, gid, uid).await?;
    let (_, role_id, provider) = owned(&st, gid, &iid).await?;
    let position: i32 = sqlx::query_scalar("SELECT position FROM roles WHERE id = $1").bind(role_id).fetch_one(&st.db).await?;
    let mut tx = st.db.begin().await?;
    sqlx::query("DELETE FROM overwrites WHERE target_id = $1").bind(role_id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM roles WHERE id = $1").bind(role_id).execute(&mut *tx).await?;
    sqlx::query("UPDATE roles SET position = position - 1 WHERE guild_id = $1 AND position > $2").bind(gid).bind(position).execute(&mut *tx).await?;
    tx.commit().await?;
    let label = st.connections.as_deref().and_then(|c| c.provider(&provider)).map_or(provider.clone(), |p| p.name.to_string());
    audit(&st, gid, uid, "integration_delete", &label, "").await;
    refresh(&st, gid).await?;
    Ok(Json(json!({})))
}
