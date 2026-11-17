mod auth;
mod error;
mod gateway;
mod hub;
mod limit;
mod mail;
mod models;
#[allow(dead_code)]
mod perms;
mod routes;

use axum::{extract::DefaultBodyLimit, http::{header, HeaderValue}, response::Html, routing::{delete, get, patch, post, put}, Router};
use hub::Hub;
use routes::{account, badges, channels, connections, emojis, friends, groups, guilds, invites, members, messages, roles, spotify, stats, thumbs, users, voice};
use sqlx::postgres::PgPoolOptions;
use std::{env, net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use tower_http::{cors::CorsLayer, services::ServeDir, set_header::SetResponseHeaderLayer, trace::TraceLayer};

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub hub: Arc<Hub>,
    pub upload_dir: PathBuf,
    pub limits: Arc<limit::Limiter>,
    pub trust_proxy: bool,
    pub livekit: Option<Arc<voice::LiveKit>>,
    pub spotify: Option<Arc<spotify::Spotify>>,
    pub mailer: Option<Arc<mail::Mailer>>,
}

fn index_html(web_dir: &std::path::Path) -> String {
    let html = std::fs::read_to_string(web_dir.join("index.html")).unwrap_or_else(|_| "<!doctype html><title>Hearth</title>".into());
    let public = env::var("PUBLIC_URL").unwrap_or_default();
    html.replace("__PUBLIC_URL__", public.trim_end_matches('/'))
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt::init();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is missing");
    let bind = env::var("BIND").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let upload_dir = PathBuf::from(env::var("UPLOAD_DIR").unwrap_or_else(|_| "uploads".into()));
    let web_dir = PathBuf::from(env::var("WEB_DIR").unwrap_or_else(|_| "../web/dist".into()));
    std::fs::create_dir_all(upload_dir.join("files")).expect("cannot create upload directory");

    let db = PgPoolOptions::new().max_connections(20).connect(&db_url).await.expect("database unreachable");
    sqlx::migrate!("./migrations").run(&db).await.expect("migration failed");
    sqlx::query("UPDATE messages SET call_ended_at = now() WHERE kind = 'call' AND call_ended_at IS NULL").execute(&db).await.expect("closing calls failed");

    let livekit = match (env::var("LIVEKIT_URL"), env::var("LIVEKIT_API_KEY"), env::var("LIVEKIT_API_SECRET")) {
        (Ok(url), Ok(key), Ok(secret)) if !url.is_empty() => Some(Arc::new(voice::LiveKit { url, key, secret })),
        _ => None,
    };
    let state = AppState {
        db,
        hub: Arc::new(Hub::default()),
        upload_dir: upload_dir.clone(),
        limits: Arc::new(limit::Limiter::default()),
        trust_proxy: env::var("TRUST_PROXY").is_ok_and(|v| v == "1"),
        livekit,
        spotify: spotify::Spotify::from_env().map(Arc::new),
        mailer: mail::Mailer::from_env().map(Arc::new),
    };
    if state.spotify.is_some() {
        tokio::spawn(spotify::poll(state.clone()));
    }
    let ticker = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            ticker.limits.sweep();
            stats::flush_all(&ticker).await;
        }
    });

    let api = Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/forgot", post(account::forgot))
        .route("/auth/reset", post(account::reset))
        .route("/config", get(account::config))
        .route("/users/@me/account", patch(account::update))
        .route("/search", get(messages::search))
        .route("/channels/{cid}/pins", get(messages::pins))
        .route("/channels/{cid}/pins/{mid}", put(messages::pin).delete(messages::unpin))
        .route("/users/@me", get(users::me).patch(users::update_me))
        .route("/users/@me/avatar", post(users::set_avatar))
        .route("/users/@me/banner", post(users::set_banner))
        .route("/users/@me/channels", post(users::open_dm))
        .route("/users/{id}", get(users::get_user))
        .route("/users/{id}/badges/{bid}", put(badges::grant).delete(badges::revoke))
        .route("/badges", get(badges::list).post(badges::create))
        .route("/badges/{bid}", delete(badges::delete))
        .route("/users/@me/activity", post(stats::set_activity).delete(stats::clear_activity))
        .route("/voice/stream", post(stats::set_streaming))
        .route("/voice/watch", post(voice::watch))
        .route("/friends", get(friends::list).post(friends::add))
        .route("/friends/{uid}", put(friends::accept).delete(friends::remove))
        .route("/users/@me/groups", post(groups::create))
        .route("/blocks/{uid}", put(friends::block).delete(friends::unblock))
        .route("/channels/{cid}/group", patch(groups::update))
        .route("/channels/{cid}/group/icon", post(groups::set_icon))
        .route("/channels/{cid}/recipients/{uid}/role", put(groups::set_role))
        .route("/channels/{cid}/recipients/{uid}", put(groups::add).delete(groups::remove))
        .route("/connections", get(connections::list))
        .route("/spotify", get(spotify::login).delete(spotify::unlink))
        .route("/spotify/callback", get(spotify::callback))
        .route("/voice", delete(voice::leave))
        .route("/channels/{cid}/voice", post(voice::join))
        .route("/channels/{cid}/attachments", post(messages::upload).layer(DefaultBodyLimit::max(26 * 1024 * 1024)))
        .route("/channels/{cid}/messages/{mid}/reactions", post(messages::react))
        .route("/channels/{cid}/messages/{mid}/poll", post(messages::vote))
        .route("/guilds/{gid}/audit", get(guilds::audit_log))
        .route("/guilds/{gid}/emojis", post(emojis::create))
        .route("/guilds/{gid}/emojis/{eid}", delete(emojis::delete))
        .route("/guilds", post(guilds::create))
        .route("/guilds/{gid}", get(guilds::get).patch(guilds::update).delete(guilds::delete))
        .route("/guilds/{gid}/icon", post(guilds::set_icon))
        .route("/guilds/{gid}/channels", post(channels::create).put(channels::reorder))
        .route("/guilds/{gid}/roles", post(roles::create).put(roles::reorder))
        .route("/guilds/{gid}/roles/{rid}", patch(roles::update).delete(roles::delete))
        .route("/guilds/{gid}/members/{uid}", patch(members::update).delete(members::kick))
        .route("/guilds/{gid}/members/{uid}/summary", get(members::summary))
        .route("/guilds/{gid}/bans", get(members::bans))
        .route("/guilds/{gid}/bans/{uid}", put(members::ban).delete(members::unban))
        .route("/guilds/{gid}/invites", get(invites::list).post(invites::create))
        .route("/guilds/{gid}/invites/{code}", delete(invites::delete))
        .route("/invites/{code}", get(invites::preview).post(invites::join))
        .route("/channels/{cid}", patch(channels::update).delete(channels::delete))
        .route("/channels/{cid}/overwrites/{target}", put(channels::put_overwrite).delete(channels::delete_overwrite))
        .route("/channels/{cid}/messages", get(messages::list).post(messages::create))
        .route("/channels/{cid}/messages/{mid}", patch(messages::update).delete(messages::delete))
        .route("/channels/{cid}/typing", post(messages::typing));

    let index = Arc::new(index_html(&web_dir));
    let page = move || {
        let index = index.clone();
        async move { ([(header::CACHE_CONTROL, "no-cache")], Html(index.to_string())) }
    };
    let uploads = ServeDir::new(&upload_dir);
    let app = Router::new()
        .nest("/api", api.fallback(|| async { error::AppError::NotFound }))
        .route("/gateway", get(gateway::handler))
        .route("/thumbs/{size}/{name}", get(thumbs::thumb))
        .nest_service(
            "/uploads/files",
            tower::ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::overriding(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")))
                .layer(SetResponseHeaderLayer::overriding(header::CONTENT_TYPE, HeaderValue::from_static("application/octet-stream")))
                .layer(SetResponseHeaderLayer::overriding(header::CONTENT_DISPOSITION, HeaderValue::from_static("attachment")))
                .service(ServeDir::new(upload_dir.join("files"))),
        )
        .nest_service(
            "/uploads",
            tower::ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::overriding(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")))
                .layer(SetResponseHeaderLayer::overriding(header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=31536000, immutable")))
                .service(uploads),
        )
        .route("/index.html", get(page.clone()))
        .fallback_service(ServeDir::new(&web_dir).append_index_html_on_directories(false).fallback(get(page)))
        .layer(DefaultBodyLimit::max(8 * 1024 * 1024))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind).await.expect("port unavailable");
    tracing::info!("Hearth is running on http://{bind}");
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.expect("server crashed");
}
