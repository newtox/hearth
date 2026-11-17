use crate::error::{AppError, Result};
use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

pub const VIEW_CHANNEL: i64 = 1 << 0;
pub const SEND_MESSAGES: i64 = 1 << 1;
pub const MANAGE_MESSAGES: i64 = 1 << 2;
pub const MANAGE_CHANNELS: i64 = 1 << 3;
pub const MANAGE_ROLES: i64 = 1 << 4;
pub const MANAGE_GUILD: i64 = 1 << 5;
pub const KICK_MEMBERS: i64 = 1 << 6;
pub const BAN_MEMBERS: i64 = 1 << 7;
pub const CREATE_INVITE: i64 = 1 << 8;
pub const CHANGE_NICKNAME: i64 = 1 << 9;
pub const MANAGE_NICKNAMES: i64 = 1 << 10;
pub const ADMINISTRATOR: i64 = 1 << 11;
pub const ATTACH_FILES: i64 = 1 << 12;
pub const ADD_REACTIONS: i64 = 1 << 13;
pub const MENTION_EVERYONE: i64 = 1 << 14;
pub const CONNECT: i64 = 1 << 15;
pub const SPEAK: i64 = 1 << 16;
pub const STREAM: i64 = 1 << 17;
pub const MODERATE_MEMBERS: i64 = 1 << 18;
pub const MANAGE_EMOJIS: i64 = 1 << 19;
pub const USE_EXTERNAL_EMOJIS: i64 = 1 << 20;
pub const PIN_MESSAGES: i64 = 1 << 21;
pub const ALL: i64 = (1 << 22) - 1;
pub const DEFAULT: i64 = USE_EXTERNAL_EMOJIS | VIEW_CHANNEL | SEND_MESSAGES | CREATE_INVITE | CHANGE_NICKNAME | ATTACH_FILES | ADD_REACTIONS | CONNECT | SPEAK | STREAM;
pub const DM: i64 = VIEW_CHANNEL | SEND_MESSAGES | ATTACH_FILES | ADD_REACTIONS | CONNECT | SPEAK | STREAM;

#[derive(Serialize, FromRow, Clone, Debug)]
pub struct Overwrite {
    pub channel_id: Uuid,
    pub target_id: Uuid,
    pub kind: String,
    pub allow: i64,
    pub deny: i64,
}

pub struct Ctx {
    pub user_id: Uuid,
    pub owner: bool,
    pub everyone: Uuid,
    pub roles: Vec<Uuid>,
    pub base: i64,
    pub top: i32,
    pub timed_out: bool,
}

impl Ctx {
    pub fn has(&self, p: i64) -> bool {
        self.base & p == p
    }

    pub fn require(&self, p: i64) -> Result<()> {
        if self.has(p) { Ok(()) } else { Err(AppError::Forbidden) }
    }

    pub fn in_channel(&self, ows: &[&Overwrite]) -> i64 {
        if self.base & ADMINISTRATOR != 0 {
            return ALL;
        }
        if self.timed_out {
            return self.overwritten(ows) & VIEW_CHANNEL;
        }
        self.overwritten(ows)
    }

    fn overwritten(&self, ows: &[&Overwrite]) -> i64 {
        let mut p = self.base;
        if let Some(o) = ows.iter().find(|o| o.target_id == self.everyone) {
            p = (p & !o.deny) | o.allow;
        }
        let (mut allow, mut deny) = (0, 0);
        for o in ows.iter().filter(|o| o.kind == "role" && o.target_id != self.everyone && self.roles.contains(&o.target_id)) {
            allow |= o.allow;
            deny |= o.deny;
        }
        p = (p & !deny) | allow;
        if let Some(o) = ows.iter().find(|o| o.kind == "member" && o.target_id == self.user_id) {
            p = (p & !o.deny) | o.allow;
        }
        p
    }
}

pub async fn ctx(db: &PgPool, guild_id: Uuid, user_id: Uuid) -> Result<Ctx> {
    let row: Option<(Uuid, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
        "SELECT g.owner_id, m.timeout_until FROM guilds g JOIN members m ON m.guild_id = g.id WHERE g.id = $1 AND m.user_id = $2",
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_optional(db)
    .await?;
    let (owner_id, timeout) = row.ok_or(AppError::NotFound)?;
    let rows: Vec<(Uuid, i64, i32, bool)> = sqlx::query_as(
        "SELECT id, permissions, position, is_default FROM roles WHERE guild_id = $1 AND (is_default OR id IN (SELECT role_id FROM member_roles WHERE guild_id = $1 AND user_id = $2))",
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_all(db)
    .await?;
    let owner = owner_id == user_id;
    let mut base = rows.iter().fold(0, |acc, r| acc | r.1);
    if owner || base & ADMINISTRATOR != 0 {
        base = ALL;
    }
    let timed_out = base != ALL && timeout.is_some_and(|t| t > chrono::Utc::now());
    Ok(Ctx {
        user_id,
        owner,
        everyone: rows.iter().find(|r| r.3).map(|r| r.0).unwrap_or_default(),
        roles: rows.iter().map(|r| r.0).collect(),
        base,
        top: if owner { i32::MAX } else { rows.iter().map(|r| r.2).max().unwrap_or(0) },
        timed_out,
    })
}

pub async fn top_position(db: &PgPool, guild_id: Uuid, user_id: Uuid) -> Result<i32> {
    match ctx(db, guild_id, user_id).await {
        Ok(c) => Ok(c.top),
        Err(AppError::NotFound) => Ok(-1),
        Err(e) => Err(e),
    }
}
