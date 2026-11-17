use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

pub const USER_COLS: &str = "id, username, display_name, avatar, banner, bio, accent_color, created_at, is_admin, \
    (SELECT COALESCE(json_agg(json_build_object('id', b.id, 'name', b.name, 'description', b.description, 'icon', b.icon, 'color', b.color, 'granted_at', ub.granted_at) ORDER BY b.position), '[]'::json) \
     FROM user_badges ub JOIN badges b ON b.id = ub.badge_id WHERE ub.user_id = users.id) AS badges";

#[derive(Serialize, Deserialize, FromRow, Clone)]
pub struct Badge {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[sqlx(default)]
    pub track: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[sqlx(default)]
    pub tier: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[sqlx(default)]
    pub hours: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[sqlx(default)]
    pub granted_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Clone)]
pub struct Activity {
    pub kind: String,
    pub name: String,
    pub details: String,
    pub since: DateTime<Utc>,
    #[serde(skip)]
    pub tick: std::time::Instant,
    #[serde(skip)]
    pub source: &'static str,
}

#[derive(Serialize, FromRow, Clone)]
pub struct PublicUser {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar: Option<String>,
    pub banner: Option<String>,
    pub bio: String,
    pub accent_color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub is_admin: bool,
    pub badges: sqlx::types::Json<Vec<Badge>>,
    #[sqlx(default)]
    pub online: bool,
    #[sqlx(skip)]
    pub presence: String,
    #[sqlx(skip)]
    pub activity: Option<Activity>,
    #[sqlx(default)]
    pub streaming: bool,
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<serde_json::Value>,
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connections: Option<serde_json::Value>,
}

#[derive(Serialize, FromRow)]
pub struct Guild {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub icon: Option<String>,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, FromRow, Clone)]
pub struct Role {
    pub id: Uuid,
    pub guild_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub permissions: i64,
    pub position: i32,
    pub hoist: bool,
    pub is_default: bool,
}

#[derive(Serialize, FromRow, Clone)]
pub struct Channel {
    pub id: Uuid,
    pub guild_id: Option<Uuid>,
    pub kind: String,
    pub name: String,
    pub topic: String,
    pub parent_id: Option<Uuid>,
    pub position: i32,
}

pub const CHANNEL_COLS: &str = "id, guild_id, kind, name, topic, parent_id, position";

#[derive(Serialize, FromRow)]
pub struct Member {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar: Option<String>,
    pub nickname: Option<String>,
    pub joined_at: DateTime<Utc>,
    pub timeout_until: Option<DateTime<Utc>>,
    pub roles: Vec<Uuid>,
    #[sqlx(default)]
    pub online: bool,
    #[sqlx(skip)]
    pub presence: String,
    #[sqlx(skip)]
    pub activity: Option<Activity>,
    #[sqlx(default)]
    pub streaming: bool,
}

#[derive(FromRow)]
pub struct MessageRow {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub author_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar: Option<String>,
    pub reply_to: Option<Uuid>,
    pub pinned_at: Option<DateTime<Utc>>,
    pub kind: String,
    pub call_ended_at: Option<DateTime<Utc>>,
    pub call_participants: Vec<Uuid>,
}

pub const MESSAGE_SELECT: &str = "SELECT m.id, m.channel_id, m.content, m.created_at, m.edited_at, u.id AS author_id, u.username, u.display_name, u.avatar, m.reply_to, m.pinned_at, m.kind, m.call_ended_at, m.call_participants FROM messages m JOIN users u ON u.id = m.author_id";

#[derive(Serialize, FromRow)]
pub struct Attachment {
    pub id: Uuid,
    #[serde(skip)]
    pub message_id: Option<Uuid>,
    pub filename: String,
    pub file: String,
    pub size: i64,
    pub is_image: bool,
}

#[derive(Serialize, FromRow)]
pub struct Emoji {
    pub id: Uuid,
    pub kind: String,
    pub name: String,
    pub file: String,
}

impl MessageRow {
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "channel_id": self.channel_id,
            "content": self.content,
            "created_at": self.created_at,
            "edited_at": self.edited_at,
            "author": { "id": self.author_id, "username": self.username, "display_name": self.display_name, "avatar": self.avatar },
            "reply_to": self.reply_to,
            "pinned": self.pinned_at.is_some(),
            "kind": self.kind,
            "call": if self.kind == "call" { serde_json::json!({ "ended_at": self.call_ended_at, "participants": self.call_participants }) } else { serde_json::Value::Null },
        })
    }
}
