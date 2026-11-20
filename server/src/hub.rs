use dashmap::DashMap;
use std::time::Instant;
use serde::Serialize;
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use uuid::Uuid;

type Tx = mpsc::UnboundedSender<String>;

#[derive(Default)]
pub struct Hub {
    conns: DashMap<Uuid, Vec<(u64, Uuid, Tx)>>,
    pub voice: DashMap<Uuid, Uuid>,
    pub voice_tick: DashMap<Uuid, Instant>,
    pub calls: DashMap<Uuid, Uuid>,
    pub streaming: DashMap<Uuid, Instant>,
    pub watching: DashMap<Uuid, Vec<Uuid>>,
    pub status: DashMap<Uuid, String>,
    pub afk: DashMap<Uuid, ()>,
    next: AtomicU64,
}

impl Hub {
    pub fn add(&self, user: Uuid, session: Uuid) -> (u64, mpsc::UnboundedReceiver<String>, bool) {
        let (tx, rx) = mpsc::unbounded_channel();
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let mut entry = self.conns.entry(user).or_default();
        let first = entry.is_empty();
        entry.push((id, session, tx));
        (id, rx, first)
    }

    pub fn end_sessions(&self, user: Uuid, keep: Option<Uuid>, only: Option<Uuid>) {
        let text = json!({ "t": "INVALID_SESSION" }).to_string();
        if let Some(entry) = self.conns.get(&user) {
            for (_, session, tx) in entry.iter() {
                if Some(*session) != keep && only.is_none_or(|o| o == *session) {
                    let _ = tx.send(text.clone());
                }
            }
        }
    }

    pub fn remove(&self, user: Uuid, id: u64) -> bool {
        if let Some(mut entry) = self.conns.get_mut(&user) {
            entry.retain(|c| c.0 != id);
            if !entry.is_empty() {
                return false;
            }
        }
        self.conns.remove_if(&user, |_, v| v.is_empty());
        !self.conns.contains_key(&user)
    }

    pub fn is_online(&self, user: &Uuid) -> bool {
        self.conns.contains_key(user)
    }

    pub fn presence(&self, user: &Uuid) -> &'static str {
        if !self.is_online(user) {
            return "offline";
        }
        match self.status.get(user).as_deref().map(String::as_str) {
            Some("invisible") => "offline",
            Some("dnd") => "dnd",
            Some("idle") => "idle",
            _ if self.afk.contains_key(user) => "idle",
            _ => "online",
        }
    }

    pub fn visible(&self, user: &Uuid) -> bool {
        self.presence(user) != "offline"
    }

    pub fn online_ids(&self) -> Vec<Uuid> {
        self.conns.iter().map(|e| *e.key()).collect()
    }

    pub fn send(&self, users: &[Uuid], t: &str, d: impl Serialize) {
        let text = json!({ "t": t, "d": d }).to_string();
        for u in users {
            if let Some(entry) = self.conns.get(u) {
                for (_, _, tx) in entry.iter() {
                    let _ = tx.send(text.clone());
                }
            }
        }
    }
}
