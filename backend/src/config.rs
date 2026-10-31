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

        let static_dir = env::var("STATIC_DIR").ok().map(|s| {
            let p = PathBuf::from(&s);
            if p.exists() {
                p
            } else if let Ok(cwd) = env::current_dir() {
                let candidate1 = cwd.join(&s);
                if candidate1.exists() {
                    candidate1
                } else {
                    let candidate2 = cwd.join("frontend/dist");
                    if candidate2.exists() {
                        candidate2
                    } else {
                        let candidate3 = cwd.join("../frontend/dist");
                        if candidate3.exists() {
                            candidate3
                        } else {
                            p
                        }
                    }
                }
            } else {
                p
            }
        });

        let uploads_dir = if let Ok(custom) = env::var("UPLOADS_DIR") {
            PathBuf::from(custom)
        } else if let Ok(cwd) = env::current_dir() {
            if cwd.join("backend").exists() {
                cwd.join("backend").join("uploads")
            } else {
                cwd.join("uploads")
            }
        } else {
            PathBuf::from("uploads")
        };

        Ok(Self {
            database_url: var("DATABASE_URL")?,
            db_max_connections: parse("DB_MAX_CONNECTIONS", "10")?,
            bind_addr: parse("BIND_ADDR", "127.0.0.1:8080")?,
            public_origin,
            google_client_id: var("GOOGLE_CLIENT_ID")?,
            google_client_secret: var("GOOGLE_CLIENT_SECRET")?,
            google_redirect_url,
            session_secret,
            session_ttl: Duration::from_secs(ttl_hours * 3600),
            cookie_secure,
            static_dir,
            uploads_dir,
            max_ws_connections: parse("MAX_WS_CONNECTIONS", "5000")?,
            trust_proxy: parse("TRUST_PROXY", "false")?,
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
