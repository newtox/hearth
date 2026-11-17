use lettre::{message::header::ContentType, transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::env;

pub struct Mailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
}

fn var(name: &str) -> Option<String> {
    env::var(name).ok().filter(|v| !v.is_empty())
}

impl Mailer {
    pub fn from_env() -> Option<Self> {
        let host = var("SMTP_HOST")?;
        let from = var("SMTP_FROM")?;
        let port: u16 = var("SMTP_PORT").and_then(|p| p.parse().ok()).unwrap_or(587);
        let mode = var("SMTP_TLS").unwrap_or_else(|| if port == 465 { "tls".into() } else { "starttls".into() });
        let builder = match mode.as_str() {
            "tls" => AsyncSmtpTransport::<Tokio1Executor>::relay(&host).ok()?,
            "none" => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&host),
            _ => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host).ok()?,
        };
        let mut builder = builder.port(port);
        if let (Some(user), Some(password)) = (var("SMTP_USER"), var("SMTP_PASSWORD")) {
            builder = builder.credentials(Credentials::new(user, password));
        }
        Some(Self { transport: builder.build(), from })
    }

    pub async fn send(&self, to: &str, subject: &str, body: String) -> bool {
        let Ok(from) = self.from.parse() else { return false };
        let Ok(to) = to.parse() else { return false };
        let Ok(message) = Message::builder().from(from).to(to).subject(subject).header(ContentType::TEXT_PLAIN).body(body) else { return false };
        match self.transport.send(message).await {
            Ok(_) => true,
            Err(e) => {
                tracing::warn!("sending mail failed: {e}");
                false
            }
        }
    }
}
