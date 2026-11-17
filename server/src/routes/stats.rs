use super::related_users;
use crate::{auth::AuthUser, error::{bad, Result}, models::{Activity, Badge}, AppState};
use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashSet, time::{Duration, Instant}};
use uuid::Uuid;

const TIER_COLORS: [&str; 10] = ["#9aa3ad", "#a3d977", "#3fca7a", "#2fd0c0", "#3fc3f5", "#4a7df8", "#a57bf7", "#f04fd0", "#f0485f", "#f0a530"];
const HOURS: [i64; 10] = [1, 5, 20, 75, 150, 300, 500, 1000, 2000, 5000];
const YEARS: [i64; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
const TRACKS: [(&str, &str, &str, [i64; 10]); 4] = [
    ("play", "Playtime", "svg:gamepad", HOURS),
    ("stream", "Stream time", "svg:broadcast", HOURS),
    ("voice", "Voice time", "svg:mic", HOURS),
    ("age", "Account age", "svg:calendar", YEARS),
];

fn lap(tick: &mut Instant) -> i64 {
    let secs = tick.elapsed().as_secs();
    *tick += Duration::from_secs(secs);
    secs as i64
}

pub async fn flush(st: &AppState, uid: Uuid) {
    let play = st.hub.activity.get_mut(&uid).map(|mut a| if a.kind == "playing" { lap(&mut a.tick) } else { 0 }).unwrap_or(0);
    let stream = st.hub.streaming.get_mut(&uid).map(|mut t| lap(&mut t)).unwrap_or(0);
    let voice = st.hub.voice_tick.get_mut(&uid).map(|mut t| lap(&mut t)).unwrap_or(0);
    if play + stream + voice == 0 {
        return;
    }
    let _ = sqlx::query(
        "INSERT INTO user_stats (user_id, play_seconds, stream_seconds, voice_seconds) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (user_id) DO UPDATE SET play_seconds = user_stats.play_seconds + EXCLUDED.play_seconds, \
         stream_seconds = user_stats.stream_seconds + EXCLUDED.stream_seconds, voice_seconds = user_stats.voice_seconds + EXCLUDED.voice_seconds",
    )
    .bind(uid)
    .bind(play)
    .bind(stream)
    .bind(voice)
    .execute(&st.db)
    .await;
}

pub async fn flush_all(st: &AppState) {
    let mut users: HashSet<Uuid> = st.hub.activity.iter().map(|e| *e.key()).collect();
    users.extend(st.hub.voice_tick.iter().map(|e| *e.key()));
    for u in users {
        flush(st, u).await;
    }
}

pub async fn tracked(st: &AppState, uid: Uuid) -> Result<(Vec<Badge>, Value)> {
    let (created, play, stream, voice): (chrono::DateTime<chrono::Utc>, i64, i64, i64) = sqlx::query_as(
        "SELECT u.created_at, COALESCE(s.play_seconds, 0), COALESCE(s.stream_seconds, 0), COALESCE(s.voice_seconds, 0) \
         FROM users u LEFT JOIN user_stats s ON s.user_id = u.id WHERE u.id = $1",
    )
    .bind(uid)
    .fetch_one(&st.db)
    .await?;
    let values = [play / 3600, stream / 3600, voice / 3600, (chrono::Utc::now() - created).num_days() / 365];
    let mut badges = Vec::new();
    let mut tracks = Vec::new();
    for (i, (track, label, icon, steps)) in TRACKS.iter().enumerate() {
        let tier = steps.iter().rposition(|s| values[i] >= *s);
        tracks.push(json!({ "track": track, "icon": icon, "value": values[i], "tier": tier, "steps": steps }));
        let Some(tier) = tier else { continue };
        badges.push(Badge {
            id: Uuid::from_u128(0x7000 + (i as u128) * 16 + tier as u128),
            name: format!("{label}: level {}", tier + 1),
            description: String::new(),
            icon: icon.to_string(),
            color: TIER_COLORS[tier].to_string(),
            track: Some(track.to_string()),
            tier: Some(tier as i32),
            hours: Some(values[i]),
        });
    }
    Ok((badges, json!({ "colors": TIER_COLORS, "tracks": tracks })))
}

pub async fn broadcast(st: &AppState, uid: Uuid) {
    if let Ok(users) = related_users(st, uid).await {
        let shown = st.hub.visible(&uid);
        let activity = if shown { st.hub.activity.get(&uid).map(|a| a.clone()) } else { None };
        st.hub.send(
            &users,
            "PRESENCE_UPDATE",
            json!({ "user_id": uid, "online": shown, "presence": st.hub.presence(&uid), "activity": activity, "streaming": shown && st.hub.streaming.contains_key(&uid) }),
        );
    }
}

#[derive(Deserialize)]
pub struct ActivityBody {
    kind: String,
    name: String,
    #[serde(default)]
    details: String,
}

pub async fn set_activity(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(b): Json<ActivityBody>) -> Result<Json<Value>> {
    st.limits.check(format!("activity:{uid}"), 30, Duration::from_secs(60))?;
    if !["playing", "listening", "watching", "custom"].contains(&b.kind.as_str()) {
        return bad("Unknown activity");
    }
    let name = b.name.trim();
    let details = b.details.trim();
    if name.is_empty() || name.chars().count() > 64 || details.chars().count() > 64 {
        return bad("Activity: 1–64 characters");
    }
    if !st.hub.is_online(&uid) {
        return bad("Not connected");
    }
    flush(&st, uid).await;
    let same = st.hub.activity.get(&uid).is_some_and(|a| a.kind == b.kind && a.name == name && a.source == "manual");
    if same {
        if let Some(mut a) = st.hub.activity.get_mut(&uid) {
            a.details = details.to_string();
        }
    } else {
        st.hub.activity.insert(uid, Activity { kind: b.kind, name: name.to_string(), details: details.to_string(), since: chrono::Utc::now(), tick: Instant::now(), source: "manual" });
    }
    broadcast(&st, uid).await;
    Ok(Json(json!({})))
}

pub async fn clear_activity(State(st): State<AppState>, AuthUser(uid): AuthUser) -> Result<Json<Value>> {
    flush(&st, uid).await;
    if st.hub.activity.remove(&uid).is_some() {
        broadcast(&st, uid).await;
    }
    Ok(Json(json!({})))
}

#[derive(Deserialize)]
pub struct StreamBody {
    on: bool,
}

pub async fn set_streaming(State(st): State<AppState>, AuthUser(uid): AuthUser, Json(b): Json<StreamBody>) -> Result<Json<Value>> {
    flush(&st, uid).await;
    let changed = if b.on && st.hub.voice.contains_key(&uid) {
        st.hub.streaming.insert(uid, Instant::now()).is_none()
    } else {
        super::voice::stop_stream(&st, uid).await;
        st.hub.streaming.remove(&uid).is_some()
    };
    if changed {
        broadcast(&st, uid).await;
    }
    Ok(Json(json!({})))
}
