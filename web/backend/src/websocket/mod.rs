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
        #[serde(default)]
        reply_to_id: Option<Uuid>,
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
    #[serde(rename = "reaction.update")]
    ReactionUpdate {
        message_id: Uuid,
        user_id: Uuid,
        emoji: String,
        action: String,
    },
    #[serde(rename = "message.pinned")]
    MessagePinned {
        message_id: Uuid,
        is_pinned: bool,
    },
    #[serde(rename = "message.deleted")]
    MessageDeleted {
        message_id: Uuid,
    },
    #[serde(rename = "message.edited")]
    MessageEdited {
        message_id: Uuid,
        envelope: String,
    },
    #[serde(rename = "channel.message.new")]
    ChannelMessageNew {
        message: crate::models::ChannelMessageItem,
    },
    #[serde(rename = "webrtc.signal")]
    WebRtcSignal {
        sender_id: Uuid,
        channel_id: Option<Uuid>,
        signal_type: String,
        data: serde_json::Value,
    },
    #[serde(rename = "voice.state.update")]
    VoiceStateUpdate(crate::models::VoiceStateItem),
    #[serde(rename = "user.presence.update")]
    UserPresenceUpdate {
        user_id: Uuid,
        presence: String,
        status_text: Option<String>,
        status_emoji: Option<String>,
    },
    #[serde(rename = "group.message.new")]
    GroupMessageNew {
        message: crate::models::GroupMessageItem,
    },
    #[serde(rename = "group.reaction.update")]
    GroupReactionUpdate {
        group_id: Uuid,
        message_id: Uuid,
        user_id: Uuid,
        emoji: String,
        action: String,
    },
    #[serde(rename = "group.channel.new")]
    GroupChannelNew {
        group_id: Uuid,
        channel: crate::models::GroupChannelItem,
    },
    #[serde(rename = "group.channel.deleted")]
    GroupChannelDeleted {
        group_id: Uuid,
        channel_id: Uuid,
    },
    #[serde(rename = "group.member.joined")]
    GroupMemberJoined {
        group_id: Uuid,
        member: crate::models::GroupMemberItem,
    },
    #[serde(rename = "group.member.left")]
    GroupMemberLeft {
        group_id: Uuid,
        user_id: Uuid,
    },
    #[serde(rename = "group.updated")]
    GroupUpdated {
        group: crate::models::GroupItem,
    },
    #[serde(rename = "call.incoming")]
    CallIncoming {
        session: crate::models::CallSessionItem,
    },
    #[serde(rename = "call.accepted")]
    CallAccepted {
        call_id: Uuid,
        user_id: Uuid,
    },
    #[serde(rename = "call.rejected")]
    CallRejected {
        call_id: Uuid,
        user_id: Uuid,
        reason: String,
    },
    #[serde(rename = "call.ended")]
    CallEnded {
        call_id: Uuid,
        reason: String,
    },
    #[serde(rename = "call.signal")]
    CallSignal {
        call_id: Uuid,
        sender_id: Uuid,
        target_user_id: Option<Uuid>,
        signal_type: String,
        data: serde_json::Value,
    },
    #[serde(rename = "call.participant.joined")]
    CallParticipantJoined {
        call_id: Uuid,
        participant: crate::models::CallParticipantItem,
    },
    #[serde(rename = "call.participant.left")]
    CallParticipantLeft {
        call_id: Uuid,
        user_id: Uuid,
    },
    #[serde(rename = "call.state.update")]
    CallStateUpdate {
        call_id: Uuid,
        user_id: Uuid,
        muted: bool,
        camera_off: bool,
    },
    /// Partner read DMs up to `seq` (only sent when the reader shares receipts).
    #[serde(rename = "message.read")]
    MessageRead(serde_json::Value),
    /// A conversation's shared wallpaper changed.
    #[serde(rename = "wallpaper.updated")]
    WallpaperUpdated(serde_json::Value),
    /// New message in a user <-> bot conversation.
    #[serde(rename = "bot.message")]
    BotMessage(serde_json::Value),
    /// Someone asked to join a group that requires approval.
    #[serde(rename = "group.join_request")]
    GroupJoinRequest(serde_json::Value),
    /// Group management settings changed.
    #[serde(rename = "group.settings")]
    GroupSettings(serde_json::Value),
    /// Labels / pins / conversation preferences changed on another device.
    #[serde(rename = "prefs.updated")]
    PrefsUpdated(serde_json::Value),
    /// A friend's (or my own) avatar motion / profile details changed.
    #[serde(rename = "profile.updated")]
    ProfileUpdated(serde_json::Value),
    /// Sign-in from a device this account never used before.
    #[serde(rename = "security.alert")]
    SecurityAlert(serde_json::Value),
    /// Enterprise workspace state updated (config, tasks, HR, approvals, docs, meetings).
    #[serde(rename = "enterprise.event")]
    EnterpriseEvent(serde_json::Value),
    /// Threads-style social feed post / reply / delete update.
    #[serde(rename = "feed.updated")]
    FeedUpdated(serde_json::Value),
    /// New untrusted device requested login & sync approval on an online trusted device.
    #[serde(rename = "device.approval.request")]
    DeviceApprovalRequest(serde_json::Value),
    /// Pending device login request was approved or rejected.
    #[serde(rename = "device.approval.resolved")]
    DeviceApprovalResolved(serde_json::Value),
    /// A device was revoked and must immediately lock/wipe active session.
    #[serde(rename = "device.revoked")]
    DeviceRevoked(serde_json::Value),
    /// Two-way DM file recovery request arriving at peer/same-account device.
    #[serde(rename = "recovery.request.incoming")]
    RecoveryRequestIncoming(serde_json::Value),
    /// Status update for an active file recovery request.
    #[serde(rename = "recovery.request.updated")]
    RecoveryRequestUpdated(serde_json::Value),
    /// WebRTC DataChannel signaling relay scoped to a recovery request.
    #[serde(rename = "recovery.p2p.signal")]
    RecoveryP2pSignal(serde_json::Value),
    /// Authoritative remote cache wipe when sender deletes media for everyone.
    #[serde(rename = "media.delete_for_all")]
    MediaDeleteForAll(serde_json::Value),
    /// Realtime multi-device / server / group Plugin & Permission Sync event.
    #[serde(rename = "plugin.sync")]
    PluginSync(serde_json::Value),
}

struct WsClientConn {
    session_id: Option<Uuid>,
    device_id: Option<Uuid>,
    tx: mpsc::Sender<WsEvent>,
}

pub struct WsHub {
    clients: RwLock<HashMap<Uuid, HashMap<u64, WsClientConn>>>,
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

    pub async fn register(
        &self,
        user_id: Uuid,
        session_id: Option<Uuid>,
        device_id: Option<Uuid>,
    ) -> Option<(u64, mpsc::Receiver<WsEvent>)> {
        if self.total_connections.load(Ordering::Relaxed) >= self.max_connections {
            return None;
        }

        let conn_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel(CLIENT_CHANNEL_CAPACITY);

        let mut clients = self.clients.write().await;
        clients.entry(user_id).or_default().insert(
            conn_id,
            WsClientConn {
                session_id,
                device_id,
                tx,
            },
        );
        self.total_connections.fetch_add(1, Ordering::Relaxed);

        Some((conn_id, rx))
    }

    pub async fn unregister(&self, user_id: &Uuid, conn_id: u64) {
        let mut clients = self.clients.write().await;
        if let Some(user_conns) = clients.get_mut(user_id)
            && user_conns.remove(&conn_id).is_some()
        {
            if user_conns.is_empty() {
                clients.remove(user_id);
            }
            self.total_connections.fetch_sub(1, Ordering::Relaxed);
        }
    }

    pub async fn disconnect_device(&self, user_id: &Uuid, target_device_id: Uuid) {
        let mut clients = self.clients.write().await;
        if let Some(user_conns) = clients.get_mut(user_id) {
            let before = user_conns.len();
            user_conns.retain(|_, c| c.device_id != Some(target_device_id));
            let removed = before.saturating_sub(user_conns.len());
            if removed > 0 {
                self.total_connections.fetch_sub(removed, Ordering::Relaxed);
            }
            if user_conns.is_empty() {
                clients.remove(user_id);
            }
        }
    }

    pub async fn disconnect_session(&self, user_id: &Uuid, target_session_id: Uuid) {
        let mut clients = self.clients.write().await;
        if let Some(user_conns) = clients.get_mut(user_id) {
            let before = user_conns.len();
            user_conns.retain(|_, c| c.session_id != Some(target_session_id));
            let removed = before.saturating_sub(user_conns.len());
            if removed > 0 {
                self.total_connections.fetch_sub(removed, Ordering::Relaxed);
            }
            if user_conns.is_empty() {
                clients.remove(user_id);
            }
        }
    }

    pub async fn send_to_user(&self, user_id: &Uuid, event: WsEvent) {
        let clients = self.clients.read().await;
        if let Some(user_conns) = clients.get(user_id) {
            for c in user_conns.values() {
                let _ = c.tx.try_send(event.clone());
            }
        }
    }

    pub async fn send_to_users(&self, user_ids: &[Uuid], event: WsEvent) {
        let clients = self.clients.read().await;
        for uid in user_ids {
            if let Some(user_conns) = clients.get(uid) {
                for c in user_conns.values() {
                    let _ = c.tx.try_send(event.clone());
                }
            }
        }
    }

    pub async fn online_user_ids(&self) -> std::collections::HashSet<Uuid> {
        let clients = self.clients.read().await;
        clients.keys().copied().collect()
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
        .register(user.id, user.session_id, user.device_id)
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
