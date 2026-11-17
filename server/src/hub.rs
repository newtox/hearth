use crate::models::Activity;
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
    conns: DashMap<Uuid, Vec<(u64, Tx)>>,
    pub voice: DashMap<Uuid, Uuid>,
    pub voice_tick: DashMap<Uuid, Instant>,
    pub calls: DashMap<Uuid, Uuid>,
    pub streaming: DashMap<Uuid, Instant>,
    pub watching: DashMap<Uuid, Vec<Uuid>>,
    pub activity: DashMap<Uuid, Activity>,
    pub status: DashMap<Uuid, String>,
    pub afk: DashMap<Uuid, ()>,
    next: AtomicU64,
}

impl Hub {
    pub fn add(&self, user: Uuid) -> (u64, mpsc::UnboundedReceiver<String>, bool) {
        let (tx, rx) = mpsc::unbounded_channel();
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let mut entry = self.conns.entry(user).or_default();
        let first = entry.is_empty();
        entry.push((id, tx));
        (id, rx, first)
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
                for (_, tx) in entry.iter() {
                    let _ = tx.send(text.clone());
                }
            }
        }
    }
}
