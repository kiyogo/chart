//! Spec 165 (Section 36): Valkey (RESP-compatible) Ephemeral Coordination Layer
//!
//! Responsibilities (strictly ephemeral & bounded TTL — NEVER sole store of messages!):
//! - Online presence (`chatw:presence:{user_id}` with TTL)
//! - Typing indicators (`chatw:typing:{scope}:{user_id}` with 8s TTL)
//! - Distributed rate limiting (atomic Lua fixed-window counter with in-memory fallback)
//! - Atomic Lua distributed locks / leases (`SET NX PX` + compare-and-renew / compare-and-delete)
//! - Task deduplication (`SET NX EX`)
//! - Non-sensitive server metadata cache (strict TTL, invalidated on commit)
//! - Cross-instance WebSocket Pub/Sub event distribution (`chatw:ws:events`, with `instance_id` echo guard)
//!
//! Resilience Guarantee:
//! - Hard per-command timeout (`250ms`) + `ConnectionManagerConfig` zero-retry fast-fail +
//!   lightweight `400ms` circuit breaker so a Valkey outage or restart NEVER stalls HTTP message sends!
//! - Automatically reconnects as soon as Valkey returns (both `ConnectionManager` and Pub/Sub loop).
//! - Zero messages are ever lost because PostgreSQL remains the sole durable authority (including
//!   `ws_event_log` for ordered HTTP reconnect gap recovery).

use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures_util::StreamExt;
use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use redis::{AsyncCommands, Client, Script};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::websocket::{WsEvent, WsHub};

pub const VALKEY_WS_PUBSUB_CHANNEL: &str = "chatw:ws:events:v1";
const PRESENCE_TTL_SECS: u64 = 90;
const TYPING_TTL_SECS: u64 = 8;
const CMD_TIMEOUT: Duration = Duration::from_millis(250);
const CIRCUIT_COOLDOWN_MS: u64 = 400;

fn unix_ms_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn fast_fail_cm_config() -> ConnectionManagerConfig {
    ConnectionManagerConfig::new()
        .set_number_of_retries(0)
        .set_connection_timeout(Duration::from_millis(300))
        .set_response_timeout(Duration::from_millis(250))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValkeyHealthSnapshot {
    pub configured: bool,
    pub connected: bool,
    pub instance_id: String,
    pub pubsub_published: u64,
    pub pubsub_received: u64,
    pub fallback_ops: u64,
    pub used_memory_bytes: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PubSubWsEnvelope {
    origin_instance: Uuid,
    user_ids: Vec<Uuid>,
    event: WsEvent,
}

pub struct ValkeyCoordinator {
    instance_id: Uuid,
    url: Option<String>,
    client: Option<Client>,
    manager: RwLock<Option<ConnectionManager>>,
    connected: AtomicBool,
    circuit_open_until_ms: AtomicU64,
    pubsub_published: AtomicU64,
    pubsub_received: AtomicU64,
    fallback_ops: AtomicU64,
    /// In-memory fallback rate limiter used automatically when Valkey is disabled or temporarily down.
    local_rate_limits: Mutex<HashMap<String, Vec<Instant>>>,
}

impl ValkeyCoordinator {
    /// Disabled coordinator (no Valkey configured — uses local fallback and PostgreSQL authority).
    pub fn disabled() -> Self {
        Self {
            instance_id: Uuid::new_v4(),
            url: None,
            client: None,
            manager: RwLock::new(None),
            connected: AtomicBool::new(false),
            circuit_open_until_ms: AtomicU64::new(0),
            pubsub_published: AtomicU64::new(0),
            pubsub_received: AtomicU64::new(0),
            fallback_ops: AtomicU64::new(0),
            local_rate_limits: Mutex::new(HashMap::new()),
        }
    }

    /// Attempt to connect to Valkey/Redis at `url`. Never panics or aborts startup if unreachable:
    /// if Valkey is temporarily unavailable, logs a warning and degrades cleanly to fallback mode
    /// while automatically attempting reconnect once the circuit cooldown elapses.
    pub async fn connect(url: Option<String>) -> Self {
        let Some(raw_url) = url.filter(|s| !s.trim().is_empty()) else {
            return Self::disabled();
        };
        let Ok(client) = Client::open(raw_url.as_str()) else {
            tracing::warn!("invalid VALKEY_URL format; running in PostgreSQL-only fallback mode");
            return Self::disabled();
        };

        let mgr = match tokio::time::timeout(
            Duration::from_secs(2),
            ConnectionManager::new_with_config(client.clone(), fast_fail_cm_config()),
        )
        .await
        {
            Ok(Ok(m)) => {
                tracing::info!("connected to Valkey coordinator");
                Some(m)
            }
            Ok(Err(err)) => {
                tracing::warn!(error = %err, "Valkey unavailable at startup; will auto-fallback & reconnect");
                None
            }
            Err(_) => {
                tracing::warn!("Valkey startup connection timed out; will auto-fallback & reconnect");
                None
            }
        };

        let is_up = mgr.is_some();
        Self {
            instance_id: Uuid::new_v4(),
            url: Some(raw_url),
            client: Some(client),
            manager: RwLock::new(mgr),
            connected: AtomicBool::new(is_up),
            circuit_open_until_ms: AtomicU64::new(0),
            pubsub_published: AtomicU64::new(0),
            pubsub_received: AtomicU64::new(0),
            fallback_ops: AtomicU64::new(0),
            local_rate_limits: Mutex::new(HashMap::new()),
        }
    }

    pub fn instance_id(&self) -> Uuid {
        self.instance_id
    }

    pub fn is_configured(&self) -> bool {
        self.url.is_some()
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    async fn get_conn(&self) -> Option<ConnectionManager> {
        let client = self.client.as_ref()?;
        let now = unix_ms_now();
        if now < self.circuit_open_until_ms.load(Ordering::Relaxed) {
            self.fallback_ops.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        {
            let guard = self.manager.read().await;
            if let Some(m) = guard.as_ref() {
                return Some(m.clone());
            }
        }
        // Lazy fast reconnect attempt once circuit cooldown has elapsed
        match tokio::time::timeout(
            Duration::from_millis(350),
            ConnectionManager::new_with_config(client.clone(), fast_fail_cm_config()),
        )
        .await
        {
            Ok(Ok(m)) => {
                let mut guard = self.manager.write().await;
                *guard = Some(m.clone());
                self.connected.store(true, Ordering::Relaxed);
                self.circuit_open_until_ms.store(0, Ordering::Relaxed);
                Some(m)
            }
            _ => {
                self.record_error().await;
                None
            }
        }
    }

    async fn record_error(&self) {
        self.connected.store(false, Ordering::Relaxed);
        self.circuit_open_until_ms
            .store(unix_ms_now().saturating_add(CIRCUIT_COOLDOWN_MS), Ordering::Relaxed);
        self.fallback_ops.fetch_add(1, Ordering::Relaxed);
        // Drop dead manager so next probe cleanly establishes a fresh TCP connection!
        let mut guard = self.manager.write().await;
        *guard = None;
    }

    // ── 1. Online Presence (TTL) ─────────────────────────────────────────────────────────────────

    pub async fn set_user_online(&self, user_id: Uuid, device_id: Option<Uuid>) {
        let Some(mut conn) = self.get_conn().await else { return; };
        let key = format!("chatw:presence:{user_id}");
        let val = device_id
            .map(|d| format!("{}:{d}", self.instance_id))
            .unwrap_or_else(|| self.instance_id.to_string());
        match tokio::time::timeout(CMD_TIMEOUT, conn.set_ex::<_, _, ()>(&key, val, PRESENCE_TTL_SECS)).await {
            Ok(Ok(())) => self.connected.store(true, Ordering::Relaxed),
            _ => self.record_error().await,
        }
    }

    pub async fn clear_user_online(&self, user_id: Uuid) {
        let Some(mut conn) = self.get_conn().await else { return; };
        let key = format!("chatw:presence:{user_id}");
        match tokio::time::timeout(CMD_TIMEOUT, conn.del::<_, i64>(&key)).await {
            Ok(Ok(_)) => self.connected.store(true, Ordering::Relaxed),
            _ => self.record_error().await,
        }
    }

    pub async fn is_user_online(&self, user_id: Uuid) -> Option<bool> {
        let mut conn = self.get_conn().await?;
        let key = format!("chatw:presence:{user_id}");
        match tokio::time::timeout(CMD_TIMEOUT, conn.exists::<_, bool>(&key)).await {
            Ok(Ok(v)) => {
                self.connected.store(true, Ordering::Relaxed);
                Some(v)
            }
            _ => {
                self.record_error().await;
                None
            }
        }
    }

    // ── 2. Typing Indicator (8s TTL) ─────────────────────────────────────────────────────────────

    pub async fn set_typing(&self, user_id: Uuid, target_id: Uuid, is_typing: bool) {
        let Some(mut conn) = self.get_conn().await else { return; };
        let key = format!("chatw:typing:{target_id}:{user_id}");
        let res = if is_typing {
            tokio::time::timeout(CMD_TIMEOUT, conn.set_ex::<_, _, ()>(&key, "1", TYPING_TTL_SECS)).await
        } else {
            tokio::time::timeout(CMD_TIMEOUT, async {
                let _: i64 = conn.del(&key).await?;
                Ok(())
            })
            .await
        };
        match res {
            Ok(Ok(())) => self.connected.store(true, Ordering::Relaxed),
            _ => self.record_error().await,
        }
    }

    pub async fn is_typing(&self, user_id: Uuid, target_id: Uuid) -> Option<bool> {
        let mut conn = self.get_conn().await?;
        let key = format!("chatw:typing:{target_id}:{user_id}");
        match tokio::time::timeout(CMD_TIMEOUT, conn.exists::<_, bool>(&key)).await {
            Ok(Ok(v)) => {
                self.connected.store(true, Ordering::Relaxed);
                Some(v)
            }
            _ => {
                self.record_error().await;
                None
            }
        }
    }

    // ── 3. Distributed Rate Limiter (Atomic Lua + In-Memory Fallback) ────────────────────────────

    pub async fn check_rate_limit(
        &self,
        keys: &[String],
        max_attempts: usize,
        window: Duration,
    ) -> Result<(), ()> {
        let window_secs = window.as_secs().max(1);
        if let Some(mut conn) = self.get_conn().await {
            let script = Script::new(
                r#"
                local cur = redis.call('INCR', KEYS[1])
                if cur == 1 then
                    redis.call('EXPIRE', KEYS[1], tonumber(ARGV[1]))
                end
                return cur
                "#,
            );
            let mut valkey_ok = true;
            for k in keys {
                let rkey = format!("chatw:rl:{k}");
                match tokio::time::timeout(
                    CMD_TIMEOUT,
                    script.key(&rkey).arg(window_secs).invoke_async::<i64>(&mut conn),
                )
                .await
                {
                    Ok(Ok(count)) => {
                        self.connected.store(true, Ordering::Relaxed);
                        if count as usize > max_attempts {
                            return Err(());
                        }
                    }
                    _ => {
                        self.record_error().await;
                        valkey_ok = false;
                        break;
                    }
                }
            }
            if valkey_ok {
                return Ok(());
            }
        }

        // Local in-memory fallback when Valkey is disabled or down
        let mut map = self
            .local_rate_limits
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        if map.len() > 4096 {
            map.retain(|_, ts| {
                ts.retain(|t| now.duration_since(*t) < window);
                !ts.is_empty()
            });
        }
        for key in keys {
            let entries = map.entry(key.clone()).or_default();
            entries.retain(|t| now.duration_since(*t) < window);
            if entries.len() >= max_attempts {
                return Err(());
            }
        }
        for key in keys {
            map.entry(key.clone()).or_default().push(now);
        }
        Ok(())
    }

    // ── 4. Atomic Lua Distributed Lock / Lease Coordination ──────────────────────────────────────

    pub async fn try_acquire_lock(&self, lock_key: &str, token: &str, ttl_secs: u64) -> Option<bool> {
        let mut conn = self.get_conn().await?;
        let rkey = format!("chatw:lock:{lock_key}");
        let ttl_ms = ttl_secs.clamp(5, 600) * 1000;
        let script = Script::new(
            r#"
            local cur = redis.call('GET', KEYS[1])
            if not cur or cur == ARGV[1] then
                redis.call('SET', KEYS[1], ARGV[1], 'PX', tonumber(ARGV[2]))
                return 1
            end
            return 0
            "#,
        );
        match tokio::time::timeout(
            CMD_TIMEOUT,
            script
                .key(&rkey)
                .arg(token)
                .arg(ttl_ms)
                .invoke_async::<i64>(&mut conn),
        )
        .await
        {
            Ok(Ok(v)) => {
                self.connected.store(true, Ordering::Relaxed);
                Some(v == 1)
            }
            _ => {
                self.record_error().await;
                None
            }
        }
    }

    pub async fn renew_lock(&self, lock_key: &str, token: &str, ttl_secs: u64) -> Option<bool> {
        let mut conn = self.get_conn().await?;
        let rkey = format!("chatw:lock:{lock_key}");
        let ttl_ms = ttl_secs.clamp(5, 600) * 1000;
        let script = Script::new(
            r#"
            if redis.call('GET', KEYS[1]) == ARGV[1] then
                redis.call('PEXPIRE', KEYS[1], tonumber(ARGV[2]))
                return 1
            end
            return 0
            "#,
        );
        match tokio::time::timeout(
            CMD_TIMEOUT,
            script
                .key(&rkey)
                .arg(token)
                .arg(ttl_ms)
                .invoke_async::<i64>(&mut conn),
        )
        .await
        {
            Ok(Ok(v)) => {
                self.connected.store(true, Ordering::Relaxed);
                Some(v == 1)
            }
            _ => {
                self.record_error().await;
                None
            }
        }
    }

    pub async fn release_lock(&self, lock_key: &str, token: &str) {
        let Some(mut conn) = self.get_conn().await else { return; };
        let rkey = format!("chatw:lock:{lock_key}");
        let script = Script::new(
            r#"
            if redis.call('GET', KEYS[1]) == ARGV[1] then
                return redis.call('DEL', KEYS[1])
            end
            return 0
            "#,
        );
        match tokio::time::timeout(
            CMD_TIMEOUT,
            script.key(&rkey).arg(token).invoke_async::<i64>(&mut conn),
        )
        .await
        {
            Ok(Ok(_)) => self.connected.store(true, Ordering::Relaxed),
            _ => self.record_error().await,
        }
    }

    // ── 5. Task Deduplication (`SET NX EX`) ──────────────────────────────────────────────────────

    pub async fn claim_task_once(&self, task_key: &str, ttl_secs: u64) -> bool {
        let Some(mut conn) = self.get_conn().await else {
            return true;
        };
        let rkey = format!("chatw:dedup:{task_key}");
        let script = Script::new(
            r#"
            if redis.call('SET', KEYS[1], '1', 'NX', 'EX', tonumber(ARGV[1])) then
                return 1
            end
            return 0
            "#,
        );
        match tokio::time::timeout(
            CMD_TIMEOUT,
            script
                .key(&rkey)
                .arg(ttl_secs.clamp(5, 86_400))
                .invoke_async::<i64>(&mut conn),
        )
        .await
        {
            Ok(Ok(v)) => {
                self.connected.store(true, Ordering::Relaxed);
                v == 1
            }
            _ => {
                self.record_error().await;
                true
            }
        }
    }

    // ── 6. Non-Sensitive Metadata Cache (Strict TTL) ─────────────────────────────────────────────

    pub async fn cache_get_str(&self, key: &str) -> Option<String> {
        let mut conn = self.get_conn().await?;
        let rkey = format!("chatw:meta:{key}");
        match tokio::time::timeout(CMD_TIMEOUT, conn.get::<_, Option<String>>(&rkey)).await {
            Ok(Ok(v)) => {
                self.connected.store(true, Ordering::Relaxed);
                v
            }
            _ => {
                self.record_error().await;
                None
            }
        }
    }

    pub async fn cache_set_str(&self, key: &str, val: &str, ttl_secs: u64) {
        let Some(mut conn) = self.get_conn().await else { return; };
        let rkey = format!("chatw:meta:{key}");
        match tokio::time::timeout(
            CMD_TIMEOUT,
            conn.set_ex::<_, _, ()>(&rkey, val, ttl_secs.clamp(5, 3600)),
        )
        .await
        {
            Ok(Ok(())) => self.connected.store(true, Ordering::Relaxed),
            _ => self.record_error().await,
        }
    }

    pub async fn cache_del(&self, key: &str) {
        let Some(mut conn) = self.get_conn().await else { return; };
        let rkey = format!("chatw:meta:{key}");
        match tokio::time::timeout(CMD_TIMEOUT, conn.del::<_, i64>(&rkey)).await {
            Ok(Ok(_)) => self.connected.store(true, Ordering::Relaxed),
            _ => self.record_error().await,
        }
    }

    // ── 7. Multi-Node WebSocket Pub/Sub Fan-Out ──────────────────────────────────────────────────

    pub async fn publish_ws_event(&self, user_ids: &[Uuid], event: &WsEvent) {
        if user_ids.is_empty() || !self.is_configured() {
            return;
        }
        let Some(mut conn) = self.get_conn().await else { return; };
        let env = PubSubWsEnvelope {
            origin_instance: self.instance_id,
            user_ids: user_ids.to_vec(),
            event: event.clone(),
        };
        let Ok(payload) = serde_json::to_string(&env) else {
            return;
        };
        match tokio::time::timeout(
            CMD_TIMEOUT,
            conn.publish::<_, _, i64>(VALKEY_WS_PUBSUB_CHANNEL, payload),
        )
        .await
        {
            Ok(Ok(_)) => {
                self.connected.store(true, Ordering::Relaxed);
                self.pubsub_published.fetch_add(1, Ordering::Relaxed);
            }
            _ => {
                self.record_error().await;
            }
        }
    }

    /// Spawn a resilient background Pub/Sub subscriber that delivers remote-node `WsEvent`s
    /// into this node's `WsHub`. Automatically reconnects with capped backoff after Valkey restart!
    pub fn spawn_ws_pubsub_listener(self: &Arc<Self>, ws_hub: Arc<WsHub>) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let this = Arc::clone(self);
        tokio::spawn(async move {
            let mut backoff_ms: u64 = 250;
            loop {
                match tokio::time::timeout(Duration::from_secs(2), client.get_async_pubsub()).await {
                    Ok(Ok(mut pubsub)) => {
                        if tokio::time::timeout(
                            Duration::from_secs(2),
                            pubsub.subscribe(VALKEY_WS_PUBSUB_CHANNEL),
                        )
                        .await
                        .is_ok_and(|r| r.is_ok())
                        {
                            this.connected.store(true, Ordering::Relaxed);
                            backoff_ms = 250;
                            let mut stream = pubsub.on_message();
                            while let Some(msg) = stream.next().await {
                                if let Ok(payload) = msg.get_payload::<String>()
                                    && let Ok(env) = serde_json::from_str::<PubSubWsEnvelope>(&payload)
                                {
                                    // Skip echo from our own instance (already delivered locally!)
                                    if env.origin_instance == this.instance_id {
                                        continue;
                                    }
                                    this.pubsub_received.fetch_add(1, Ordering::Relaxed);
                                    ws_hub.deliver_local_users(&env.user_ids, env.event).await;
                                }
                            }
                        }
                        this.connected.store(false, Ordering::Relaxed);
                    }
                    _ => {
                        this.connected.store(false, Ordering::Relaxed);
                    }
                }
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                backoff_ms = (backoff_ms * 2).min(5000);
            }
        });
    }

    // ── 8. Diagnostics & Memory Telemetry ────────────────────────────────────────────────────────

    pub async fn health_snapshot(&self) -> ValkeyHealthSnapshot {
        let mut used_mem = None;
        if let Some(mut conn) = self.get_conn().await {
            match tokio::time::timeout(
                CMD_TIMEOUT,
                redis::cmd("INFO").arg("memory").query_async::<String>(&mut conn),
            )
            .await
            {
                Ok(Ok(info)) => {
                    self.connected.store(true, Ordering::Relaxed);
                    for line in info.lines() {
                        if let Some(rest) = line.strip_prefix("used_memory:")
                            && let Ok(bytes) = rest.trim().parse::<u64>()
                        {
                            used_mem = Some(bytes);
                            break;
                        }
                    }
                }
                _ => {
                    self.record_error().await;
                }
            }
        }
        ValkeyHealthSnapshot {
            configured: self.is_configured(),
            connected: self.is_connected(),
            instance_id: self.instance_id.to_string(),
            pubsub_published: self.pubsub_published.load(Ordering::Relaxed),
            pubsub_received: self.pubsub_received.load(Ordering::Relaxed),
            fallback_ops: self.fallback_ops.load(Ordering::Relaxed),
            used_memory_bytes: used_mem,
        }
    }
}
