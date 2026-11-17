use crate::{auth, error::Result, routes::{guilds::snapshot, stats, users::{dm_json, me_json}}, AppState};
use axum::{extract::{ws::{Message, WebSocket}, State, WebSocketUpgrade}, response::Response};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

pub async fn handler(ws: WebSocketUpgrade, State(st): State<AppState>) -> Response {
    ws.max_message_size(16 * 1024).on_upgrade(move |socket| run(socket, st))
}

async fn ready(st: &AppState, uid: Uuid) -> Result<Value> {
    let guild_ids: Vec<Uuid> = sqlx::query_scalar("SELECT guild_id FROM members WHERE user_id = $1 ORDER BY joined_at").bind(uid).fetch_all(&st.db).await?;
    let mut guilds = Vec::with_capacity(guild_ids.len());
    for g in guild_ids {
        guilds.push(snapshot(st, g, uid).await?);
    }
    let dm_ids: Vec<Uuid> = sqlx::query_scalar("SELECT channel_id FROM dm_recipients WHERE user_id = $1 ORDER BY channel_id DESC").bind(uid).fetch_all(&st.db).await?;
    let mut dms = Vec::with_capacity(dm_ids.len());
    for d in dm_ids {
        dms.push(dm_json(st, d).await?);
    }
    let friends = crate::routes::friends::list_json(st, uid).await?;
    Ok(json!({ "t": "READY", "d": { "user": me_json(st, uid).await?, "guilds": guilds, "dms": dms, "friends": friends } }))
}


async fn identify(socket: &mut WebSocket, st: &AppState) -> Option<Uuid> {
    let msg = tokio::time::timeout(Duration::from_secs(10), socket.next()).await.ok()??.ok()?;
    let Message::Text(text) = msg else { return None };
    let v: Value = serde_json::from_str(&text).ok()?;
    if v["op"] != "identify" {
        return None;
    }
    auth::user_from_token(&st.db, v["token"].as_str()?).await
}

async fn run(mut socket: WebSocket, st: AppState) {
    let Some(uid) = identify(&mut socket, &st).await else {
        let _ = socket.send(Message::Text(json!({ "t": "INVALID_SESSION" }).to_string().into())).await;
        return;
    };
    let status: String = sqlx::query_scalar("SELECT status FROM users WHERE id = $1").bind(uid).fetch_one(&st.db).await.unwrap_or_else(|_| "online".into());
    if status == "online" {
        st.hub.status.remove(&uid);
    } else {
        st.hub.status.insert(uid, status);
    }
    let (conn, mut rx, first) = st.hub.add(uid);
    let (mut sink, mut stream) = socket.split();
    let hello = match ready(&st, uid).await {
        Ok(v) => v.to_string(),
        Err(_) => {
            st.hub.remove(uid, conn);
            return;
        }
    };
    if sink.send(Message::Text(hello.into())).await.is_ok() {
        if first {
            st.hub.afk.remove(&uid);
            stats::broadcast(&st, uid).await;
        }
        loop {
            tokio::select! {
                out = rx.recv() => match out {
                    Some(text) => if sink.send(Message::Text(text.into())).await.is_err() { break },
                    None => break,
                },
                incoming = tokio::time::timeout(Duration::from_secs(90), stream.next()) => match incoming {
                    Ok(Some(Ok(Message::Text(t)))) => {
                        let v: Value = serde_json::from_str(&t).unwrap_or_default();
                        if v["op"] == "idle" {
                            let idle = v["idle"].as_bool().unwrap_or(false);
                            let changed = if idle { st.hub.afk.insert(uid, ()).is_none() } else { st.hub.afk.remove(&uid).is_some() };
                            if changed {
                                stats::broadcast(&st, uid).await;
                            }
                        } else if t.contains("ping") && sink.send(Message::Text(r#"{"t":"PONG"}"#.into())).await.is_err() { break }
                    }
                    Ok(Some(Ok(Message::Close(_)))) | Ok(Some(Err(_))) | Ok(None) | Err(_) => break,
                    Ok(Some(Ok(_))) => {}
                },
            }
        }
    }
    if st.hub.remove(uid, conn) {
        crate::routes::voice::leave_all(&st, uid).await;
        st.hub.activity.remove(&uid);
        st.hub.afk.remove(&uid);
        stats::broadcast(&st, uid).await;
    }
}
