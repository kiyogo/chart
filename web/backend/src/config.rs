use std::{env, net::SocketAddr, path::PathBuf, time::Duration};

/// Runtime configuration, loaded once from environment variables.
#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub db_max_connections: u32,
    pub bind_addr: SocketAddr,
    /// Public origin of the app, e.g. `https://chat.example.com` (no trailing slash).
    pub public_origin: String,
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_url: String,
    pub session_secret: Vec<u8>,
    pub session_ttl: Duration,
    pub cookie_secure: bool,
    pub static_dir: Option<PathBuf>,
    pub uploads_dir: PathBuf,
    pub max_ws_connections: usize,
    pub trust_proxy: bool,
    /// Optional explicit extra allowed CORS origins (comma-separated). Empty by default.
    pub cors_allowed_origins: Vec<String>,
    /// False when Google credentials are placeholders, so the UI can avoid a dead-end redirect.
    pub google_configured: bool,
    pub kiyo_issuer: String,
    pub kiyo_client_id: String,
    pub kiyo_client_secret: String,
    pub kiyo_redirect_url: String,
    /// True when KIYO_ISSUER and KIYO_CLIENT_ID are configured.
    pub kiyo_configured: bool,
    /// Optional Valkey/Redis RESP URL for ephemeral presence, typing, locks, rate limits & WS Pub/Sub.
    pub valkey_url: Option<String>,
    /// Maximum encrypted chunk byte length stored inline in PostgreSQL (`pg_inline`) instead of Object Storage.
    /// Chunks strictly larger than this threshold go to MinIO/S3 or local CAS with ZERO duplication.
    pub small_chunk_inline_threshold_bytes: usize,
}

fn var(name: &str) -> Result<String, String> {
    env::var(name).map_err(|_| format!("missing environment variable {name}"))
}

fn var_or(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_owned())
}

fn parse<T: std::str::FromStr>(name: &str, default: &str) -> Result<T, String> {
    var_or(name, default)
        .parse()
        .map_err(|_| format!("invalid value for {name}"))
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let public_origin = var("PUBLIC_URL")?.trim_end_matches('/').to_owned();
        let cookie_secure = if public_origin.starts_with("https://") {
            true
        } else if public_origin.starts_with("http://") {
            false
        } else {
            return Err("PUBLIC_URL must start with http:// or https://".into());
        };
        if public_origin.matches('/').count() != 2 {
            return Err("PUBLIC_URL must be an origin without a path".into());
        }

        let session_secret = var("SESSION_SECRET")?.into_bytes();
        if session_secret.len() < 32 {
            return Err("SESSION_SECRET must be at least 32 characters".into());
        }

        let ttl_hours: u64 = parse("SESSION_TTL_HOURS", "168")?;
        if !(1..=24 * 90).contains(&ttl_hours) {
            return Err("SESSION_TTL_HOURS must be between 1 and 2160".into());
        }

        let google_redirect_url = env::var("GOOGLE_REDIRECT_URL")
            .unwrap_or_else(|_| format!("{public_origin}/api/auth/google/callback"));

        let static_dir = env::var("STATIC_DIR")
            .ok()
            .map(|s| {
                let p = PathBuf::from(&s);
                if p.exists() {
                    p
                } else if let Ok(cwd) = env::current_dir() {
                    let candidate1 = cwd.join(&s);
                    if candidate1.exists() { candidate1 } else { p }
                } else {
                    p
                }
            })
            .or_else(|| {
                let cwd = env::current_dir().ok()?;
                ["app/dist", "web/frontend/dist", "frontend/dist", "../frontend/dist"]
                    .into_iter()
                    .map(|p| cwd.join(p))
                    .find(|p| p.exists())
            })
            .filter(|p| p.exists());

        let uploads_dir = if let Ok(custom) = env::var("UPLOADS_DIR") {
            PathBuf::from(custom)
        } else if let Ok(cwd) = env::current_dir() {
            if cwd.join("web/backend").exists() {
                cwd.join("web/backend/uploads")
            } else if cwd.join("backend").exists() {
                cwd.join("backend/uploads")
            } else {
                cwd.join("uploads")
            }
        } else {
            PathBuf::from("uploads")
        };

        let cors_allowed_origins: Vec<String> = env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().trim_end_matches('/').to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let google_client_id = var("GOOGLE_CLIENT_ID")?;
        let google_configured = google_client_id.ends_with(".apps.googleusercontent.com")
            && !google_client_id.starts_with("dev-")
            && !google_client_id.starts_with("your-");

        let kiyo_issuer = var_or("KIYO_ISSUER", "").trim().trim_end_matches('/').to_owned();
        let kiyo_client_id = var_or("KIYO_CLIENT_ID", "").trim().to_owned();
        let kiyo_client_secret = var_or("KIYO_CLIENT_SECRET", "").trim().to_owned();
        let kiyo_redirect_url = env::var("KIYO_REDIRECT_URL")
            .ok()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("{public_origin}/api/auth/kiyo/callback"));
        let kiyo_configured = !kiyo_issuer.is_empty() && !kiyo_client_id.is_empty();

        let valkey_url = env::var("VALKEY_URL")
            .ok()
            .or_else(|| env::var("REDIS_URL").ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let small_chunk_inline_threshold_bytes: usize =
            parse("SMALL_CHUNK_INLINE_THRESHOLD_BYTES", "16384")?;

        Ok(Self {
            database_url: var("DATABASE_URL")?,
            db_max_connections: parse("DB_MAX_CONNECTIONS", "10")?,
            bind_addr: parse("BIND_ADDR", "127.0.0.1:8085")?,
            public_origin,
            google_client_id,
            google_client_secret: var("GOOGLE_CLIENT_SECRET")?,
            google_redirect_url,
            session_secret,
            session_ttl: Duration::from_secs(ttl_hours * 3600),
            cookie_secure,
            static_dir,
            uploads_dir,
            max_ws_connections: parse("MAX_WS_CONNECTIONS", "5000")?,
            trust_proxy: parse("TRUST_PROXY", "false")?,
            cors_allowed_origins,
            google_configured,
            kiyo_issuer,
            kiyo_client_id,
            kiyo_client_secret,
            kiyo_redirect_url,
            kiyo_configured,
            valkey_url,
            small_chunk_inline_threshold_bytes,
        })
    }

    pub fn ws_origin(&self) -> String {
        if let Some(rest) = self.public_origin.strip_prefix("https://") {
            format!("wss://{rest}")
        } else {
            format!("ws://{}", self.public_origin.trim_start_matches("http://"))
        }
    }
}
