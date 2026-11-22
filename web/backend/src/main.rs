use axum::{
    Router,
    http::{HeaderValue, Method, StatusCode, header},
    middleware::{Next, from_fn},
    response::Response,
};
use std::{sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::{
    cors::{AllowHeaders, AllowOrigin, CorsLayer},
    services::{ServeDir, ServeFile},
    timeout::TimeoutLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use chatw::{
    attachments, audit,
    auth::{self, AppState, google::GoogleJwksCache},
    backup, calls, cloud, community,
    config::Config,
    admin, bots, business, convo, crypto, db, devices_sync, directory, drawings, enterprise, feed, friends, groups, library, messages, plugins, privacy, public_pages, servers,
    profile_ext, profile_plus, security, stickers, storage, typing, users, vico, webrtc,
    websocket::{self, WsHub},
};

async fn security_headers_middleware(req: axum::extract::Request, next: Next) -> Response {
    let is_plugin_sandbox = req.uri().path() == "/plugin-sandbox-frame";
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    if is_plugin_sandbox {
        headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("SAMEORIGIN"));
        headers.insert(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'none'; script-src 'unsafe-inline' 'unsafe-eval' 'wasm-unsafe-eval' blob:; worker-src blob:; connect-src 'none'; style-src 'unsafe-inline'; img-src data: https: blob:; font-src data:; frame-ancestors 'self' http://127.0.0.1:* http://localhost:* https://*;",
            ),
        );
    } else {
        headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
        headers.insert(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'self'; script-src 'self' 'wasm-unsafe-eval' 'unsafe-eval' blob:; worker-src 'self' blob:; frame-src 'self' blob: data:; style-src 'self' 'unsafe-inline'; connect-src 'self' ws: wss: http://ipc.localhost ipc:; img-src 'self' data: blob: https://*.googleusercontent.com https://lh3.googleusercontent.com; object-src 'none'; frame-ancestors 'none'; base-uri 'self';",
            ),
        );
    }
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        header::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(self), microphone=(self), display-capture=(self), geolocation=()"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate"),
    );

    response
}

async fn plugin_sandbox_frame_html() -> ([(axum::http::header::HeaderName, &'static str); 1], &'static str) {
    (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        r#"<!doctype html>
<html lang="vi">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"></head>
<body>
<script>
(function () {
  const SHELL_NONCE = (location.hash || '').replace(/^#/, '');
  let mounted = false;
  let innerWorker = null;
  let innerWorkerUrl = null;
  window.addEventListener('message', function onMsg(ev) {
    const d = ev.data;
    if (!d || d.nonce !== SHELL_NONCE) return;
    if (!mounted && d.__cwSpawnWorker) {
      mounted = true;
      try {
        const blob = new Blob([String(d.workerSrc || '')], { type: 'application/javascript' });
        innerWorkerUrl = URL.createObjectURL(blob);
        innerWorker = new Worker(innerWorkerUrl);
        innerWorker.onmessage = function (we) {
          window.parent.postMessage(we.data, '*');
        };
        innerWorker.onerror = function (err) {
          window.parent.postMessage({
            __cwWorkerError: true,
            nonce: SHELL_NONCE,
            error: (err && err.message) || 'Opaque worker runtime crash'
          }, '*');
        };
      } catch (err) {
        window.parent.postMessage({
          __cwWorkerError: true,
          nonce: SHELL_NONCE,
          error: (err && err.message) || String(err)
        }, '*');
      }
      return;
    }
    if (d.__cwToWorker && innerWorker) {
      try { innerWorker.postMessage(d.payload); } catch (_) {}
      return;
    }
    if (d.__cwTerminateWorker) {
      if (innerWorker) { try { innerWorker.terminate(); } catch (_) {} innerWorker = null; }
      if (innerWorkerUrl) { try { URL.revokeObjectURL(innerWorkerUrl); } catch (_) {} innerWorkerUrl = null; }
      return;
    }
    if (mounted || !d.__cwMountDoc) return;
    mounted = true;
    try {
      if (d.cssVars) {
        const st = document.createElement('style');
        st.textContent = d.cssVars;
        document.head.appendChild(st);
      }
      if (d.sdkCode) {
        const sdk = document.createElement('script');
        sdk.textContent = d.sdkCode;
        document.head.appendChild(sdk);
      }
      const parser = new DOMParser();
      const parsed = parser.parseFromString(String(d.html || ''), 'text/html');
      parsed.head.querySelectorAll('style').forEach((s) => {
        const ns = document.createElement('style');
        ns.textContent = s.textContent || '';
        document.head.appendChild(ns);
      });
      const scripts = [];
      parsed.body.querySelectorAll('script').forEach((s) => {
        if (!s.src) scripts.push(s.textContent || '');
        s.remove();
      });
      document.body.innerHTML = parsed.body.innerHTML;
      for (const code of scripts) {
        const el = document.createElement('script');
        el.textContent = code;
        document.body.appendChild(el);
      }
      window.postMessage({ __cwReady: true, nonce: SHELL_NONCE, context: d.readyContext || {} }, '*');
    } catch (err) {
      document.body.textContent = 'Sandbox mount error: ' + (err && err.message ? err.message : String(err));
    }
  });
  window.parent.postMessage({ __cwSandboxShellReady: true, nonce: SHELL_NONCE }, '*');
})();
</script>
</body>
</html>"#,
    )
}

async fn robots_txt() -> ([(axum::http::header::HeaderName, &'static str); 1], &'static str) {
    (
        [(axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        "User-agent: *\nAllow: /\n\nSitemap: /sitemap.xml\n",
    )
}

async fn llms_txt() -> ([(axum::http::header::HeaderName, &'static str); 1], &'static str) {
    (
        [(axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        include_str!("../../frontend/public/llms.txt"),
    )
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
    let oauth_tickets = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    let storage_registry = Arc::new(storage::StorageRegistry::from_uploads_dir(&config.uploads_dir));
    let sfu = Arc::new(calls::sfu::SfuHandle::new());

    let state = AppState {
        config: Arc::new(config.clone()),
        db,
        http_client,
        jwks_cache,
        ws_hub,
        oauth_tickets,
        storage: storage_registry,
        sfu,
    };

    // 6. Build API router
    let api_router = Router::new()
        .nest("/auth", auth::routes())
        .nest("/users", users::routes())
        .nest("/keys", crypto::keys::routes().merge(devices_sync::key_backup_routes()))
        .nest("/devices", devices_sync::device_routes())
        .nest("/sync", devices_sync::sync_routes())
        .nest("/recovery", devices_sync::recovery_routes())
        .nest("/storage", storage::router::routes())
        .nest("/backup", backup::routes())
        .nest("/cloud", cloud::router::routes())
        .nest("/friends", friends::routes())
        .nest("/feed", feed::routes())
        .nest("/vico", vico::routes())
        .nest("/groups", groups::routes())
        .nest("/calls", calls::routes())
        .nest("/messages", messages::routes())
        .nest("/attachments", attachments::routes())
        .nest("/audit", audit::routes())
        .nest("/servers", servers::server_routes())
        .merge(enterprise::routes())
        .nest("/channels", servers::channel_routes())
        .nest("/stickers", stickers::routes())
        .nest("/media", library::routes())
        .nest("/privacy", privacy::routes())
        .nest("/plugins", plugins::plugin_routes())
        .nest("/webhooks", plugins::webhook_routes())
        .nest("/bots", bots::routes())
        .nest("/business", business::routes())
        .nest("/directory", directory::routes())
        .nest("/admin", admin::routes().nest("/plugins", plugins::admin_plugin_routes()))
        .nest("/bot", bots::api_routes())
        .nest("/conversations", convo::routes())
        .nest("/webrtc", webrtc::routes())
        .nest("/typing", typing::routes())
        .nest("/drawings", drawings::routes())
        .nest("/security", security::routes())
        .nest("/profile-plus", profile_plus::routes())
        .merge(community::routes())
        .merge(profile_ext::routes())
        .merge(websocket::routes())
        .with_state(state.clone());
    profile_ext::spawn_status_expiry(state.clone());
    plugins::webhooks::spawn_webhook_retry_worker(state.clone());
    if std::env::var("CHATW_SEED_SAMPLE_PLUGINS").as_deref() == Ok("1") {
        plugins::marketplace::seed_official_catalog(&state.db).await;
    }
    {
        let cloud_bg = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            interval.tick().await;
            loop {
                interval.tick().await;
                if let Err(e) = cloud::incremental::run_due_cloud_backup_policies(
                    &cloud_bg.db,
                    &cloud_bg.storage,
                    &cloud_bg.http_client,
                    &cloud_bg.config.session_secret,
                )
                .await
                {
                    tracing::debug!(error = ?e, "cloud scheduler tick");
                }
            }
        });
    }
    {
        let bg_state = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600));
            interval.tick().await;
            loop {
                interval.tick().await;
                let _ = storage::migration::cleanup_relays_safely(&bg_state.db, &bg_state.storage).await;
            }
        });
    }
    let public_pages = public_pages::routes().with_state(state.clone());
    admin::bootstrap_admins(&state.db).await;

    let _ = std::fs::create_dir_all(config.uploads_dir.join("avatars"));
    tracing::info!(dir = ?config.uploads_dir, "serving user uploaded files");

    let mut app = Router::new()
        .nest("/api", api_router)
        .route("/plugin-sandbox-frame", axum::routing::get(plugin_sandbox_frame_html))
        .route("/robots.txt", axum::routing::get(robots_txt))
        .route("/llms.txt", axum::routing::get(llms_txt))
        .merge(public_pages)
        .nest_service("/uploads/avatars", ServeDir::new(config.uploads_dir.join("avatars")))
        .nest("/uploads", attachments::protected_uploads_routes().with_state(state.clone()));

    // 7. Static file serving (if frontend dist exists)
    if let Some(ref static_dir) = config.static_dir
        && static_dir.exists()
    {
        let index_html = static_dir.join("index.html");
        // `.gz` siblings (e.g. the sticker catalogue) are served to clients that accept gzip.
        let serve_dir = ServeDir::new(static_dir).precompressed_gzip().not_found_service(ServeFile::new(index_html));
        app = app.fallback_service(serve_dir);
        tracing::info!(dir = ?static_dir, "serving static frontend assets");
    }

    // 7b. CORS layer for web development and same-origin requests
    let public_origin_for_cors = config.public_origin.clone();
    let cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers(AllowHeaders::list([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            header::ORIGIN,
            header::COOKIE,
            header::HeaderName::from_static("x-export-token"),
            header::HeaderName::from_static("x-backup-password"),
        ]))
        .allow_credentials(true)
        .allow_origin(AllowOrigin::predicate(move |origin: &HeaderValue, _| {
            let s = origin.to_str().unwrap_or("");
            auth::is_allowed_cors_origin(s, &public_origin_for_cors)
        }));

    let app = app
        .layer(from_fn(security_headers_middleware))
        .layer(cors)
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
