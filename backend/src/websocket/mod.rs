use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, header},
    response::Response,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use crate::{
    auth::{
        AppState,
        session::{SESSION_COOKIE_NAME, authenticate_session, parse_cookie},
    },
    error::{AppError, AppResult},
};

const PING_INTERVAL: Duration = Duration::from_secs(30);
const CLIENT_CHANNEL_CAPACITY: usize = 128;

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum WsEvent {
    #[serde(rename = "message.new")]
    MessageNew {
        id: Uuid,
        seq: i64,
        sender_id: Uuid,
        recipient_id: Uuid,
        envelope: String, // Base64url encoded envelope
        created_at: chrono::DateTime<chrono::Utc>,
    },
    #[serde(rename = "friend.request")]
    FriendRequest {
        id: Uuid,
        sender_id: Uuid,
        sender_username: String,
        created_at: chrono::DateTime<chrono::Utc>,
    },
    #[serde(rename = "friend.accepted")]
    FriendAccepted { user_id: Uuid, username: String },
    #[serde(rename = "typing.start")]
    TypingStart { user_id: Uuid },
    #[serde(rename = "typing.stop")]
    TypingStop { user_id: Uuid },
}

pub struct WsHub {
    clients: RwLock<HashMap<Uuid, HashMap<u64, mpsc::Sender<WsEvent>>>>,
    next_id: AtomicU64,
    total_connections: AtomicUsize,
    max_connections: usize,
}

impl WsHub {
    pub fn new(max_connections: usize) -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            total_connections: AtomicUsize::new(0),
            max_connections,
        }
    }

    pub async fn register(&self, user_id: Uuid) -> Option<(u64, mpsc::Receiver<WsEvent>)> {
        if self.total_connections.load(Ordering::Relaxed) >= self.max_connections {
            return None;
        }

        let conn_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel(CLIENT_CHANNEL_CAPACITY);

        let mut clients = self.clients.write().await;
        clients.entry(user_id).or_default().insert(conn_id, tx);
        self.total_connections.fetch_add(1, Ordering::Relaxed);

        Some((conn_id, rx))
    }

    pub async fn unregister(&self, user_id: &Uuid, conn_id: u64) {
        let mut clients = self.clients.write().await;
        if let Some(user_conns) = clients.get_mut(user_id) {
            user_conns.remove(&conn_id);
            if user_conns.is_empty() {
                clients.remove(user_id);
            }
            self.total_connections.fetch_sub(1, Ordering::Relaxed);
        }
    }

    pub async fn send_to_user(&self, user_id: &Uuid, event: WsEvent) {
        let clients = self.clients.read().await;
        if let Some(user_conns) = clients.get(user_id) {
            for tx in user_conns.values() {
                // If channel is full or receiver is slow/closed, try_send drops or cleans up
                let _ = tx.try_send(event.clone());
            }
        }
    }
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/ws", get(ws_handler))
}

async fn ws_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> AppResult<Response> {
    // 1. Verify Origin header
    if let Some(origin) = headers.get(header::ORIGIN) {
        let origin_str = origin
            .to_str()
            .map_err(|_| AppError::Forbidden("invalid_origin"))?;
        if origin_str != state.config.public_origin {
            return Err(AppError::Forbidden("origin_mismatch"));
        }
    }

    // 2. Authenticate session from HttpOnly Cookie
    let session_token =
        parse_cookie(&headers, SESSION_COOKIE_NAME).ok_or(AppError::Unauthorized)?;

    let user = authenticate_session(&state.db, &state.config, session_token).await?;

    // 3. Register connection in hub
    let (conn_id, rx) = state
        .ws_hub
        .register(user.id)
        .await
        .ok_or(AppError::TooManyRequests)?;

    let hub = state.ws_hub.clone();
    let user_id = user.id;

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, user_id, conn_id, rx, hub)))
}

async fn handle_socket(
    socket: WebSocket,
    user_id: Uuid,
    conn_id: u64,
    mut rx: mpsc::Receiver<WsEvent>,
    hub: Arc<WsHub>,
) {
    let (mut sender, mut receiver) = socket.split();

    let mut ping_interval = tokio::time::interval(PING_INTERVAL);
    ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            // Heartbeat ping
            _ = ping_interval.tick() => {
                if sender.send(Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
            }

            // Outgoing event to client
            maybe_event = rx.recv() => {
                match maybe_event {
                    Some(event) => {
                        if let Ok(json) = serde_json::to_string(&event) {
                            let _ = sender.send(Message::Text(json.into())).await;
                        }
                    }
                    None => break,
                }
            }

            // Incoming message from client
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Pong(_))) => {
                        // Healthy heartbeat
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        break;
                    }
                    Some(Err(_)) => {
                        break;
                    }
                    // Reject client data messages - WebSocket is strictly server-to-client for events!
                    _ => {}
                }
            }
        }
    }

    hub.unregister(&user_id, conn_id).await;
}
