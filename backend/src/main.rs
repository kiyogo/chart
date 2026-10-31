use axum::{
    Router,
    http::{HeaderValue, StatusCode, header},
    middleware::{Next, from_fn},
    response::Response,
};
use std::{sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::{
    services::{ServeDir, ServeFile},
    timeout::TimeoutLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use chatw::{
    auth::{self, AppState, google::GoogleJwksCache},
    config::Config,
    crypto, db, friends, messages, typing, users,
    websocket::{self, WsHub},
};

async fn security_headers_middleware(req: axum::extract::Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        header::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self' ws: wss:; img-src 'self' data: https://*.googleusercontent.com https://lh3.googleusercontent.com; object-src 'none'; frame-ancestors 'none'; base-uri 'self';",
        ),
    );

    response
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install ring crypto provider for rustls
    let _ = rustls::crypto::ring::default_provider().install_default();
    // 1. Initialize structured logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "chatw=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load .env if present
    let _ = dotenvy::dotenv();

    // 2. Load config
    let config = Config::from_env().map_err(|e| {
        tracing::error!(error = %e, "configuration error");
        e
    })?;
    tracing::info!(bind = %config.bind_addr, "starting chatw backend");

    // 3. Connect to PostgreSQL
    let db = db::connect(&config.database_url, config.db_max_connections).await?;
    tracing::info!("connected to postgresql pool");

    // 4. Run database migrations
    db::run_migrations(&db).await?;
    tracing::info!("database migrations applied successfully");

    // 5. Initialize shared state
    let ws_hub = Arc::new(WsHub::new(config.max_ws_connections));
    let jwks_cache = Arc::new(tokio::sync::RwLock::new(GoogleJwksCache::new()));
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let state = AppState {
        config: Arc::new(config.clone()),
        db,
        http_client,
        jwks_cache,
        ws_hub,
    };

    // 6. Build API router
    let api_router = Router::new()
        .nest("/auth", auth::routes())
        .nest("/users", users::routes())
        .nest("/keys", crypto::keys::routes())
        .nest("/friends", friends::routes())
        .nest("/messages", messages::routes())
        .nest("/typing", typing::routes())
        .merge(websocket::routes())
        .with_state(state);

    let _ = std::fs::create_dir_all(config.uploads_dir.join("avatars"));
    tracing::info!(dir = ?config.uploads_dir, "serving user uploaded files");

    let mut app = Router::new()
        .nest("/api", api_router)
        .nest_service("/uploads", ServeDir::new(&config.uploads_dir));

    // 7. Static file serving (if frontend dist exists)
    if let Some(ref static_dir) = config.static_dir
        && static_dir.exists()
    {
        let index_html = static_dir.join("index.html");
        let serve_dir = ServeDir::new(static_dir).not_found_service(ServeFile::new(index_html));
        app = app.fallback_service(serve_dir);
        tracing::info!(dir = ?static_dir, "serving static frontend assets");
    }

    let app = app
        .layer(from_fn(security_headers_middleware))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(15),
        ));

    // 8. Bind server
    let listener = TcpListener::bind(config.bind_addr).await?;
    tracing::info!("listening on http://{}", config.bind_addr);

    // 9. Graceful shutdown handler
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("server shutdown complete");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("received Ctrl+C, shutting down gracefully");
        },
        _ = terminate => {
            tracing::info!("received SIGTERM, shutting down gracefully");
        },
    }
}
