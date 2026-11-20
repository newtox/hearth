use super::{channels::access, guild_audience};
use crate::{auth::AuthUser, error::{bad, AppError, Result}, perms, AppState};
use axum::{extract::{Path, State}, Json};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;
use uuid::Uuid;

pub struct LiveKit {
    pub url: String,
    pub key: String,
    pub secret: String,
}

fn token(lk: &LiveKit, user: Uuid, name: &str, room: Uuid, speak: bool, stream: bool) -> Result<String> {
    let now = chrono::Utc::now().timestamp();
    let mut sources = Vec::new();
    if speak {
        sources.push("microphone");
    }
    if stream {
        sources.extend(["camera", "screen_share", "screen_share_audio"]);
    }
    let claims = json!({
        "iss": lk.key,
        "sub": user,
        "name": name,
        "nbf": now - 10,
        "exp": now + 6 * 3600,
        "video": { "room": room, "roomJoin": true, "canSubscribe": true, "canPublish": speak || stream, "canPublishData": false, "canPublishSources": sources },
    });
    let head = URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#);
    let body = URL_SAFE_NO_PAD.encode(claims.to_string());
    let mut mac = Hmac::<Sha256>::new_from_slice(lk.secret.as_bytes()).map_err(|e| AppError::Internal(e.to_string()))?;
    mac.update(format!("{head}.{body}").as_bytes());
    Ok(format!("{head}.{body}.{}", URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())))
}

async fn announce(st: &AppState, guild_id: Option<Uuid>, channel: Uuid, user: Uuid, joined: bool) -> Result<()> {
    let audience = match guild_id {
        Some(g) => guild_audience(st, g).await?,
        None => super::groups::recipients(st, channel).await?,
    };
    st.hub.send(&audience, "VOICE_STATE", json!({ "guild_id": guild_id, "room": channel, "user_id": user, "channel_id": if joined { Some(channel) } else { None } }));
    Ok(())
}

pub fn watchers(st: &AppState, room: Uuid) -> Vec<(Uuid, Uuid)> {
    st.hub
        .watching
        .iter()
        .filter(|e| st.hub.voice.get(e.key()).is_some_and(|c| *c == room))
        .flat_map(|e| e.value().iter().map(|s| (*e.key(), *s)).collect::<Vec<_>>())
        .collect()
}

pub async fn announce_watchers(st: &AppState, room: Uuid) {
    let Ok(guild_id) = sqlx::query_scalar::<_, Option<Uuid>>("SELECT guild_id FROM channels WHERE id = $1").bind(room).fetch_one(&st.db).await else { return };
    let audience = match guild_id {
        Some(g) => guild_audience(st, g).await,
        None => super::groups::recipients(st, room).await,
    };
    if let Ok(audience) = audience {
        st.hub.send(&audience, "VOICE_WATCHERS", json!({ "room": room, "watchers": watchers(st, room) }));
    }
}

pub async fn stop_stream(st: &AppState, streamer: Uuid) {
    let Some(room) = st.hub.voice.get(&streamer).map(|c| *c) else { return };
    let mut changed = false;
    for mut e in st.hub.watching.iter_mut() {
        let before = e.len();
        e.retain(|s| *s != streamer);
        changed |= e.len() != before;
    }
    if changed {
        announce_watchers(st, room).await;
    }
}

pub async fn leave_all(st: &AppState, user: Uuid) {
    stop_stream(st, user).await;
    if st.hub.watching.remove(&user).is_some_and(|(_, v)| !v.is_empty()) {
        if let Some(room) = st.hub.voice.get(&user).map(|c| *c) {
            announce_watchers(st, room).await;
        }
    }
    super::stats::flush(st, user).await;
    st.hub.voice_tick.remove(&user);
    if st.hub.streaming.remove(&user).is_some() {
        super::stats::broadcast(st, user).await;
    }
    if let Some((_, channel)) = st.hub.voice.remove(&user) {
        if let Ok(guild_id) = sqlx::query_scalar::<_, Option<Uuid>>("SELECT guild_id FROM channels WHERE id = $1").bind(channel).fetch_one(&st.db).await {
            let _ = announce(st, guild_id, channel, user, false).await;
            if guild_id.is_none() {
                let _ = super::messages::call_left(st, channel).await;
            }
        }
    }
}

pub async fn join(State(st): State<AppState>, AuthUser(uid): AuthUser, Path(cid): Path<Uuid>) -> Result<Json<Value>> {
    let a = access(&st, uid, cid, perms::CONNECT).await?;
    let gid = a.channel.guild_id;
    if !matches!((gid, a.channel.kind.as_str()), (Some(_), "voice") | (None, "dm") | (None, "group")) {
        return bad("This is not a voice channel");
    }
    let Some(lk) = st.livekit.as_ref() else {
        return Err(AppError::Unavailable("Voice channels are not set up on this server".into()));
    };
    let name: String = sqlx::query_scalar("SELECT display_name FROM users WHERE id = $1").bind(uid).fetch_one(&st.db).await?;
    let jwt = token(lk, uid, &name, cid, a.perms & perms::SPEAK != 0, a.perms & perms::STREAM != 0)?;
    if st.hub.voice.get(&uid).is_none_or(|c| *c != cid) {
        leave_all(&st, uid).await;
        let first = !st.hub.voice.iter().any(|e| *e.value() == cid);
        st.hub.voice.insert(uid, cid);
        st.hub.voice_tick.insert(uid, std::time::Instant::now());
        announce(&st, gid, cid, uid, true).await?;
        if gid.is_none() {
            super::messages::call_joined(&st, cid, uid).await?;
        }
        if gid.is_none() && first {
            super::users::reopen_dm(&st, cid).await?;
            let others: Vec<Uuid> = sqlx::query_scalar("SELECT user_id FROM dm_recipients WHERE channel_id = $1 AND user_id <> $2 AND user_id NOT IN (SELECT user_id FROM blocks WHERE blocked_id = $2)")
                .bind(cid)
                .bind(uid)
                .fetch_all(&st.db)
                .await?;
            st.hub.send(&others, "CALL_RING", json!({ "channel_id": cid, "user_id": uid }));
        }
    }
    Ok(Json(json!({ "url": lk.url, "token": jwt, "can_speak": a.perms & perms::SPEAK != 0, "can_stream": a.perms & perms::STREAM != 0, "watchers": watchers(&st, cid) })))
}

pub async fn leave(State(st): State<AppState>, AuthUser(uid): AuthUser) -> Result<Json<Value>> {
    leave_all(&st, uid).await;
    Ok(Json(json!({})))
}

#[derive(serde::Deserialize)]
pub struct WatchBody {
    user_id: Uuid,
    on: bool,
}

pub async fn watch(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(b): Json<WatchBody>) -> Result<Json<Value>> {
    let Some(room) = st.hub.voice.get(&uid).map(|c| *c) else {
        return bad("Not connected");
    };
    if b.on && (b.user_id == uid || st.hub.voice.get(&b.user_id).is_none_or(|c| *c != room) || !st.hub.streaming.contains_key(&b.user_id)) {
        return bad("Nobody is streaming there");
    }
    {
        let mut list = st.hub.watching.entry(uid).or_default();
        list.retain(|s| *s != b.user_id);
        if b.on {
            list.push(b.user_id);
        }
    }
    announce_watchers(&st, room).await;
    Ok(Json(json!({})))
}
