use crate::{error::{AppError, Result}, AppState};
use axum::{extract::{ConnectInfo, FromRequestParts}, http::request::Parts};
use dashmap::DashMap;
use std::{net::SocketAddr, time::{Duration, Instant}};

#[derive(Default)]
pub struct Limiter {
    hits: DashMap<String, (u32, Instant)>,
}

impl Limiter {
    pub fn check(&self, key: String, max: u32, window: Duration) -> Result<()> {
        let now = Instant::now();
        let mut entry = self.hits.entry(key).or_insert((0, now));
        if now.duration_since(entry.1) > window {
            *entry = (0, now);
        }
        entry.0 += 1;
        if entry.0 > max { Err(AppError::TooMany) } else { Ok(()) }
    }

    pub fn sweep(&self) {
        let now = Instant::now();
        self.hits.retain(|_, v| now.duration_since(v.1) < Duration::from_secs(600));
    }
}

pub struct ClientIp(pub String);

impl FromRequestParts<AppState> for ClientIp {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, st: &AppState) -> Result<Self> {
        if st.trust_proxy {
            if let Some(ip) = parts.headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()).and_then(|v| v.split(',').next()) {
                return Ok(ClientIp(ip.trim().to_string()));
            }
        }
        let ip = parts.extensions.get::<ConnectInfo<SocketAddr>>().map(|c| c.0.ip().to_string()).unwrap_or_default();
        Ok(ClientIp(ip))
    }
}
