use std::collections::{HashMap, VecDeque};
use std::io::ErrorKind;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Once, Weak, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use str0m::change::{SdpAnswer, SdpOffer, SdpPendingOffer};
use str0m::channel::{ChannelData, ChannelId};
use str0m::media::{Direction, KeyframeRequest, KeyframeRequestKind, MediaChanged, MediaData, MediaKind, Mid, Rid};
use str0m::net::{Protocol, Receive};
use str0m::{Candidate, Event, IceConnectionState, Input, Output, Rtc};
use tokio::sync::oneshot;
use tracing::{debug, info, warn};
use uuid::Uuid;

static INIT_STR0M_CRYPTO: Once = Once::new();

#[derive(Debug, Clone, Serialize)]
pub struct SfuJoinReply {
    pub sdp: String,
    #[serde(rename = "type")]
    pub kind: &'static str,
}

#[derive(Debug)]
enum SfuCommand {
    JoinPeer {
        call_id: Uuid,
        user_id: Uuid,
        offer_sdp: String,
        meta: HashMap<String, String>,
        reply_tx: oneshot::Sender<Result<SfuJoinReply, String>>,
    },
    LeavePeer {
        call_id: Uuid,
        user_id: Uuid,
    },
    EndCall {
        call_id: Uuid,
    },
}

#[derive(Clone)]
pub struct SfuHandle {
    tx: mpsc::Sender<SfuCommand>,
}

impl std::fmt::Debug for SfuHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SfuHandle").finish()
    }
}

impl SfuHandle {
    pub fn new() -> Self {
        INIT_STR0M_CRYPTO.call_once(|| {
            str0m::crypto::from_feature_flags().install_process_default();
        });

        let (tx, rx) = mpsc::channel::<SfuCommand>();
        let sockets = bind_sfu_udp_sockets();
        let addrs: Vec<SocketAddr> = sockets.iter().map(|s| s.local_addr).collect();
        info!(addrs = ?addrs, "KIYO SFU UDP sockets bound");

        let spawn_res = thread::Builder::new()
            .name("kiyo-sfu".to_string())
            .spawn(move || run_sfu_loop(sockets, rx));
        if let Err(e) = spawn_res {
            warn!(error = %e, "failed to spawn KIYO SFU thread");
        }

        Self { tx }
    }

    pub async fn join_peer(
        &self,
        call_id: Uuid,
        user_id: Uuid,
        offer_sdp: String,
        meta: HashMap<String, String>,
    ) -> Result<SfuJoinReply, String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(SfuCommand::JoinPeer {
                call_id,
                user_id,
                offer_sdp,
                meta,
                reply_tx,
            })
            .map_err(|_| "sfu_channel_closed".to_string())?;

        reply_rx
            .await
            .map_err(|_| "sfu_reply_dropped".to_string())?
    }

    pub fn leave_peer(&self, call_id: Uuid, user_id: Uuid) {
        let _ = self.tx.send(SfuCommand::LeavePeer { call_id, user_id });
    }

    pub fn end_call(&self, call_id: Uuid) {
        let _ = self.tx.send(SfuCommand::EndCall { call_id });
    }
}

impl Default for SfuHandle {
    fn default() -> Self {
        Self::new()
    }
}

struct BoundSocket {
    socket: UdpSocket,
    local_addr: SocketAddr,
}

fn detect_lan_ipv4() -> Option<Ipv4Addr> {
    if let Ok(explicit) = std::env::var("SFU_PUBLIC_IP") {
        if let Ok(ip) = explicit.trim().parse::<Ipv4Addr>() {
            return Some(ip);
        }
    }
    let probe = UdpSocket::bind("0.0.0.0:0").ok()?;
    if probe.connect("8.8.8.8:80").is_ok() {
        if let Ok(SocketAddr::V4(v4)) = probe.local_addr() {
            let ip = *v4.ip();
            if !ip.is_loopback() && !ip.is_unspecified() && !ip.is_link_local() {
                return Some(ip);
            }
        }
    }
    None
}

fn bind_sfu_udp_sockets() -> Vec<BoundSocket> {
    let mut out = Vec::new();
    if let Ok(loopback) = UdpSocket::bind("127.0.0.1:0") {
        if let Ok(local_addr) = loopback.local_addr() {
            let _ = loopback.set_nonblocking(true);
            out.push(BoundSocket {
                socket: loopback,
                local_addr,
            });
        }
    }
    if let Some(lan_ip) = detect_lan_ipv4() {
        if let Ok(lan_sock) = UdpSocket::bind(SocketAddr::new(IpAddr::V4(lan_ip), 0)) {
            if let Ok(local_addr) = lan_sock.local_addr() {
                let _ = lan_sock.set_nonblocking(true);
                out.push(BoundSocket {
                    socket: lan_sock,
                    local_addr,
                });
            }
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ClientId(u64);

#[derive(Debug)]
struct TrackIn {
    call_id: Uuid,
    origin: ClientId,
    origin_user_id: Uuid,
    mid: Mid,
    kind: MediaKind,
    tag: String, // "audio" | "cam" | "screen" | "screen_audio"
}

#[derive(Debug)]
struct TrackInEntry {
    id: Arc<TrackIn>,
    last_keyframe_request: Option<Instant>,
}

#[derive(Debug)]
struct TrackOut {
    track_in: Weak<TrackIn>,
    origin_user_id: Uuid,
    tag: String,
    state: TrackOutState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrackOutState {
    ToOpen,
    Negotiating(Mid),
    Open(Mid),
    ToStop(Mid),
    NegotiatingStop(Mid),
}

impl TrackOut {
    fn mid(&self) -> Option<Mid> {
        match self.state {
            TrackOutState::ToOpen => None,
            TrackOutState::Negotiating(m)
            | TrackOutState::Open(m)
            | TrackOutState::ToStop(m)
            | TrackOutState::NegotiatingStop(m) => Some(m),
        }
    }
}

#[derive(Deserialize)]
struct ClientDataMsg {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    offer_id: Option<u64>,
    #[serde(default)]
    sdp: String,
    #[serde(default)]
    meta: HashMap<String, String>,
    #[serde(default)]
    mid: Option<String>,
}

#[derive(Serialize)]
struct SfuOfferMetaItem {
    user_id: Uuid,
    tag: String,
}

#[derive(Serialize)]
struct SfuSignalOut<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    offer_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sdp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<HashMap<String, SfuOfferMetaItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tag: Option<String>,
}

#[derive(Debug)]
enum Propagated {
    Noop,
    Timeout(Instant),
    TrackOpen(Uuid, ClientId, Weak<TrackIn>),
    TrackClose(Uuid, ClientId, Uuid, String),
    MediaData(Uuid, ClientId, MediaData),
    KeyframeRequest(Uuid, ClientId, KeyframeRequest, ClientId, Mid),
}

struct Client {
    id: ClientId,
    call_id: Uuid,
    user_id: Uuid,
    rtc: Rtc,
    pending: Option<SdpPendingOffer>,
    pending_offer_id: Option<u64>,
    next_offer_id: u64,
    cid: Option<ChannelId>,
    mid_tags: HashMap<String, String>,
    tracks_in: Vec<TrackInEntry>,
    tracks_out: Vec<TrackOut>,
    chosen_rid: Option<Rid>,
}

impl Client {
    fn new(call_id: Uuid, user_id: Uuid, rtc: Rtc, mid_tags: HashMap<String, String>) -> Self {
        static ID_COUNTER: AtomicU64 = AtomicU64::new(1);
        let id = ClientId(ID_COUNTER.fetch_add(1, Ordering::SeqCst));
        Self {
            id,
            call_id,
            user_id,
            rtc,
            pending: None,
            pending_offer_id: None,
            next_offer_id: 1,
            cid: None,
            mid_tags,
            tracks_in: Vec::new(),
            tracks_out: Vec::new(),
            chosen_rid: None,
        }
    }

    fn accepts(&self, input: &Input) -> bool {
        self.rtc.accepts(input)
    }

    fn handle_input(&mut self, input: Input) {
        if !self.rtc.is_alive() {
            return;
        }
        if let Err(e) = self.rtc.handle_input(input) {
            warn!(client = self.id.0, user = %self.user_id, error = ?e, "SFU client handle_input error");
            self.rtc.disconnect();
        }
    }

    fn send_channel_json<T: Serialize>(&mut self, payload: &T) {
        let Some(cid) = self.cid else { return };
        let Some(mut ch) = self.rtc.channel(cid) else { return };
        if let Ok(json) = serde_json::to_string(payload) {
            if let Err(e) = ch.write(false, json.as_bytes()) {
                warn!(client = self.id.0, error = ?e, "SFU DataChannel write failed");
            }
        }
    }

    fn poll_output(&mut self, sockets: &[BoundSocket]) -> Propagated {
        if !self.rtc.is_alive() {
            return Propagated::Noop;
        }

        if self.negotiate_if_needed() {
            return Propagated::Noop;
        }

        match self.rtc.poll_output() {
            Ok(output) => self.handle_output(output, sockets),
            Err(e) => {
                warn!(client = self.id.0, error = ?e, "SFU poll_output failed");
                self.rtc.disconnect();
                Propagated::Noop
            }
        }
    }

    fn handle_output(&mut self, output: Output, sockets: &[BoundSocket]) -> Propagated {
        match output {
            Output::Transmit(t) => {
                let sock = sockets
                    .iter()
                    .find(|s| s.local_addr == t.source)
                    .or_else(|| {
                        if t.destination.ip().is_loopback() {
                            sockets.iter().find(|s| s.local_addr.ip().is_loopback())
                        } else {
                            sockets.iter().find(|s| !s.local_addr.ip().is_loopback())
                        }
                    })
                    .or_else(|| sockets.first());
                if let Some(s) = sock {
                    if let Err(e) = s.socket.send_to(&t.contents, t.destination) {
                        if e.kind() != ErrorKind::WouldBlock {
                            debug!(error = %e, dest = %t.destination, "SFU UDP send_to error");
                        }
                    }
                }
                Propagated::Noop
            }
            Output::Timeout(t) => Propagated::Timeout(t),
            Output::Event(ev) => match ev {
                Event::IceConnectionStateChange(IceConnectionState::Disconnected) => {
                    self.rtc.disconnect();
                    Propagated::Noop
                }
                Event::MediaAdded(e) => {
                    self.handle_media_added(e.mid, e.kind, e.direction)
                }
                Event::MediaChanged(e) => self.handle_media_changed(e),
                Event::MediaData(data) => self.handle_media_data_in(data),
                Event::KeyframeRequest(req) => self.handle_incoming_keyframe_req(req),
                Event::ChannelOpen(cid, _label) => {
                    self.cid = Some(cid);
                    Propagated::Noop
                }
                Event::ChannelData(data) => self.handle_channel_data(data),
                _ => Propagated::Noop,
            },
        }
    }

    fn resolve_tag(&self, mid: Mid, kind: MediaKind) -> String {
        if let Some(t) = self.mid_tags.get(&*mid) {
            return t.clone();
        }
        match kind {
            MediaKind::Audio => "audio".to_string(),
            MediaKind::Video => "cam".to_string(),
        }
    }

    fn handle_media_added(&mut self, mid: Mid, kind: MediaKind, dir: Direction) -> Propagated {
        if dir == Direction::Inactive || dir == Direction::SendOnly {
            return Propagated::Noop;
        }
        let tag = self.resolve_tag(mid, kind);
        if self.tracks_in.iter().any(|t| t.id.mid == mid) {
            return Propagated::Noop;
        }
        let entry = TrackInEntry {
            id: Arc::new(TrackIn {
                call_id: self.call_id,
                origin: self.id,
                origin_user_id: self.user_id,
                mid,
                kind,
                tag,
            }),
            last_keyframe_request: None,
        };
        let weak = Arc::downgrade(&entry.id);
        self.tracks_in.push(entry);
        Propagated::TrackOpen(self.call_id, self.id, weak)
    }

    fn handle_media_changed(&mut self, e: MediaChanged) -> Propagated {
        let receiving_from_peer = matches!(e.direction, Direction::RecvOnly | Direction::SendRecv);
        if !receiving_from_peer {
            if let Some(pos) = self.tracks_in.iter().position(|t| t.id.mid == e.mid) {
                let removed = self.tracks_in.remove(pos);
                return Propagated::TrackClose(
                    self.call_id,
                    self.id,
                    removed.id.origin_user_id,
                    removed.id.tag.clone(),
                );
            }
        }
        Propagated::Noop
    }

    fn handle_media_data_in(&mut self, data: MediaData) -> Propagated {
        if !data.contiguous {
            self.request_keyframe_throttled(data.mid, data.rid, KeyframeRequestKind::Pli);
        }
        Propagated::MediaData(self.call_id, self.id, data)
    }

    fn request_keyframe_throttled(
        &mut self,
        mid: Mid,
        rid: Option<Rid>,
        kind: KeyframeRequestKind,
    ) {
        let Some(mut writer) = self.rtc.writer(mid) else {
            return;
        };
        let Some(track_entry) = self.tracks_in.iter_mut().find(|t| t.id.mid == mid) else {
            return;
        };
        if track_entry
            .last_keyframe_request
            .map(|t| t.elapsed() < Duration::from_millis(650))
            .unwrap_or(false)
        {
            return;
        }
        let _ = writer.request_keyframe(rid, kind);
        track_entry.last_keyframe_request = Some(Instant::now());
    }

    fn handle_incoming_keyframe_req(&self, mut req: KeyframeRequest) -> Propagated {
        let Some(track_out) = self.tracks_out.iter().find(|t| t.mid() == Some(req.mid)) else {
            return Propagated::Noop;
        };
        let Some(track_in) = track_out.track_in.upgrade() else {
            return Propagated::Noop;
        };
        req.rid = self.chosen_rid;
        Propagated::KeyframeRequest(self.call_id, self.id, req, track_in.origin, track_in.mid)
    }

    fn negotiate_if_needed(&mut self) -> bool {
        if self.cid.is_none() || self.pending.is_some() {
            return false;
        }

        for track in &mut self.tracks_out {
            if let TrackOutState::Open(m) = track.state {
                if track.track_in.upgrade().is_none() {
                    track.state = TrackOutState::ToStop(m);
                }
            } else if let TrackOutState::ToOpen = track.state {
                // Drop before opening if publisher already stopped
            }
        }
        self.tracks_out.retain(|t| {
            !matches!(t.state, TrackOutState::ToOpen) || t.track_in.upgrade().is_some()
        });

        let mut change = self.rtc.sdp_api();
        let mut offer_meta: HashMap<String, SfuOfferMetaItem> = HashMap::new();

        for track in &mut self.tracks_out {
            match track.state {
                TrackOutState::ToOpen => {
                    if let Some(track_in) = track.track_in.upgrade() {
                        let stream_id =
                            format!("sfu_{}_{}", track_in.origin_user_id, track_in.tag);
                        let mid = change.add_media(
                            track_in.kind,
                            Direction::SendOnly,
                            Some(stream_id),
                            None,
                            None,
                        );
                        offer_meta.insert(
                            mid.to_string(),
                            SfuOfferMetaItem {
                                user_id: track_in.origin_user_id,
                                tag: track_in.tag.clone(),
                            },
                        );
                        track.state = TrackOutState::Negotiating(mid);
                    }
                }
                TrackOutState::ToStop(mid) => {
                    change.stop_media(mid);
                    track.state = TrackOutState::NegotiatingStop(mid);
                }
                _ => {}
            }
        }

        if !change.has_changes() {
            return false;
        }

        let Some((offer, pending)) = change.apply() else {
            return false;
        };

        let offer_id = self.next_offer_id;
        self.next_offer_id += 1;

        self.send_channel_json(&SfuSignalOut {
            kind: "offer",
            offer_id: Some(offer_id),
            sdp: Some(offer.to_sdp_string()),
            meta: if offer_meta.is_empty() {
                None
            } else {
                Some(offer_meta)
            },
            user_id: None,
            tag: None,
        });

        self.pending = Some(pending);
        self.pending_offer_id = Some(offer_id);
        true
    }

    fn handle_channel_data(&mut self, d: ChannelData) -> Propagated {
        let Ok(msg) = serde_json::from_slice::<ClientDataMsg>(&d.data) else {
            return Propagated::Noop;
        };

        match msg.kind.as_str() {
            "offer" => {
                if self.pending.is_some() {
                    // SFU is impolite on glare: client will roll back, answer our offer, then retry.
                    self.send_channel_json(&SfuSignalOut {
                        kind: "retry_offer",
                        offer_id: msg.offer_id,
                        sdp: None,
                        meta: None,
                        user_id: None,
                        tag: None,
                    });
                    return Propagated::Noop;
                }
                for (k, v) in msg.meta {
                    self.mid_tags.insert(k, v);
                }
                if let Ok(offer) = SdpOffer::from_sdp_string(&msg.sdp) {
                    self.handle_offer(offer, msg.offer_id);
                }
                Propagated::Noop
            }
            "answer" => {
                if msg.offer_id.is_some() && msg.offer_id != self.pending_offer_id {
                    debug!(client = self.id.0, got = ?msg.offer_id, expected = ?self.pending_offer_id, "Ignoring stale SFU answer");
                    return Propagated::Noop;
                }
                if let Ok(answer) = SdpAnswer::from_sdp_string(&msg.sdp) {
                    self.handle_answer(answer)
                } else {
                    Propagated::Noop
                }
            }
            "track_stop" => {
                if let Some(mid_str) = msg.mid {
                    if let Some(pos) = self
                        .tracks_in
                        .iter()
                        .position(|t| &*t.id.mid == mid_str)
                    {
                        let removed = self.tracks_in.remove(pos);
                        return Propagated::TrackClose(
                            self.call_id,
                            self.id,
                            removed.id.origin_user_id,
                            removed.id.tag.clone(),
                        );
                    }
                }
                Propagated::Noop
            }
            _ => Propagated::Noop,
        }
    }

    fn handle_offer(&mut self, offer: SdpOffer, client_offer_id: Option<u64>) {
        let answer = match self.rtc.sdp_api().accept_offer(offer) {
            Ok(a) => a,
            Err(e) => {
                warn!(client = self.id.0, error = ?e, "SFU accept_offer failed");
                return;
            }
        };

        self.send_channel_json(&SfuSignalOut {
            kind: "answer",
            offer_id: client_offer_id,
            sdp: Some(answer.to_sdp_string()),
            meta: None,
            user_id: None,
            tag: None,
        });
    }

    fn handle_answer(&mut self, answer: SdpAnswer) -> Propagated {
        self.pending_offer_id = None;
        let mut first_keyframe: Option<(ClientId, Mid)> = None;
        if let Some(pending) = self.pending.take() {
            if let Err(e) = self.rtc.sdp_api().accept_answer(pending, answer) {
                warn!(client = self.id.0, error = ?e, "SFU accept_answer failed");
                for track in &mut self.tracks_out {
                    if let TrackOutState::Negotiating(_) = track.state {
                        track.state = TrackOutState::ToOpen;
                    } else if let TrackOutState::NegotiatingStop(m) = track.state {
                        track.state = TrackOutState::ToStop(m);
                    }
                }
                return Propagated::Noop;
            }

            for track in &mut self.tracks_out {
                if let TrackOutState::Negotiating(m) = track.state {
                    track.state = TrackOutState::Open(m);
                    if let Some(ti) = track.track_in.upgrade() {
                        if ti.kind == MediaKind::Video && first_keyframe.is_none() {
                            first_keyframe = Some((ti.origin, ti.mid));
                        }
                    }
                }
            }

            self.tracks_out
                .retain(|t| !matches!(t.state, TrackOutState::NegotiatingStop(_)));
        }

        if let Some((origin, mid_in)) = first_keyframe {
            return Propagated::KeyframeRequest(
                self.call_id,
                self.id,
                KeyframeRequest {
                    mid: mid_in,
                    rid: None,
                    kind: KeyframeRequestKind::Pli,
                },
                origin,
                mid_in,
            );
        }
        Propagated::Noop
    }

    fn handle_track_open(&mut self, track_in: Weak<TrackIn>) {
        let Some(upgraded) = track_in.upgrade() else {
            return;
        };
        if upgraded.call_id != self.call_id || upgraded.origin_user_id == self.user_id {
            return;
        }
        // Avoid duplicate subscription to same upstream (origin, mid)
        let already = self.tracks_out.iter().any(|t| {
            t.track_in
                .upgrade()
                .map(|x| x.origin == upgraded.origin && x.mid == upgraded.mid)
                .unwrap_or(false)
        });
        if already {
            return;
        }
        self.tracks_out.push(TrackOut {
            origin_user_id: upgraded.origin_user_id,
            tag: upgraded.tag.clone(),
            track_in,
            state: TrackOutState::ToOpen,
        });
    }

    fn handle_track_close_notify(&mut self, origin_user_id: Uuid, tag: &str) {
        for track in &mut self.tracks_out {
            if let TrackOutState::Open(m) = track.state {
                if track.track_in.upgrade().is_none()
                    || (track.origin_user_id == origin_user_id && track.tag == tag)
                {
                    track.state = TrackOutState::ToStop(m);
                }
            }
        }
        self.send_channel_json(&SfuSignalOut {
            kind: "track_stopped",
            offer_id: None,
            sdp: None,
            meta: None,
            user_id: Some(origin_user_id),
            tag: Some(tag.to_string()),
        });
    }

    fn handle_media_data_out(&mut self, call_id: Uuid, origin: ClientId, data: &MediaData) {
        if self.call_id != call_id {
            return;
        }
        let Some(mid) = self
            .tracks_out
            .iter()
            .find(|o| {
                o.track_in
                    .upgrade()
                    .filter(|i| i.origin == origin && i.mid == data.mid)
                    .is_some()
            })
            .and_then(|o| o.mid())
        else {
            return;
        };

        if data.rid.is_some() && data.rid != Some("h".into()) {
            return;
        }
        if self.chosen_rid != data.rid {
            self.chosen_rid = data.rid;
        }

        let Some(writer) = self.rtc.writer(mid) else {
            return;
        };
        let Some(pt) = writer.match_params(data.params) else {
            return;
        };
        if let Err(e) = writer.write(pt, data.network_time, data.time, data.data.clone()) {
            debug!(client = self.id.0, error = ?e, "SFU writer.write failed");
        }
    }

    fn handle_keyframe_request(&mut self, req: KeyframeRequest, mid_in: Mid) {
        if !self.tracks_in.iter().any(|i| i.id.mid == mid_in) {
            return;
        }
        if let Some(mut writer) = self.rtc.writer(mid_in) {
            let _ = writer.request_keyframe(req.rid, req.kind);
        }
    }
}

fn run_sfu_loop(sockets: Vec<BoundSocket>, rx: mpsc::Receiver<SfuCommand>) {
    let mut clients: Vec<Client> = Vec::new();
    let mut to_propagate: VecDeque<Propagated> = VecDeque::new();
    let mut buf = vec![0u8; 4096];

    loop {
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                SfuCommand::JoinPeer {
                    call_id,
                    user_id,
                    offer_sdp,
                    meta,
                    reply_tx,
                } => {
                    // Disconnect previous connection for same user in same call cleanly
                    let old_tags: Vec<(Uuid, String)> = clients
                        .iter()
                        .filter(|c| c.call_id == call_id && c.user_id == user_id)
                        .flat_map(|c| {
                            c.tracks_in
                                .iter()
                                .map(|t| (t.id.origin_user_id, t.id.tag.clone()))
                        })
                        .collect();
                    clients.retain(|c| !(c.call_id == call_id && c.user_id == user_id));
                    for (uid, tag) in old_tags {
                        for c in clients.iter_mut().filter(|c| c.call_id == call_id) {
                            c.handle_track_close_notify(uid, &tag);
                        }
                    }

                    let now = Instant::now();
                    let mut rtc = Rtc::builder().build(now);
                    for sock in &sockets {
                        if let Ok(cand) = Candidate::host(sock.local_addr, "udp") {
                            let _ = rtc.add_local_candidate(cand);
                        }
                    }

                    let res = SdpOffer::from_sdp_string(&offer_sdp)
                        .map_err(|e| format!("invalid_sdp: {e:?}"))
                        .and_then(|offer| {
                            rtc.sdp_api()
                                .accept_offer(offer)
                                .map_err(|e| format!("accept_offer_failed: {e:?}"))
                        });

                    match res {
                        Ok(answer) => {
                            let sdp = answer.to_sdp_string();
                            let mut client = Client::new(call_id, user_id, rtc, meta);
                            // Subscribe to all existing active tracks in the same call room
                            for existing in clients.iter().filter(|c| c.call_id == call_id && c.user_id != user_id) {
                                for track in &existing.tracks_in {
                                    client.handle_track_open(Arc::downgrade(&track.id));
                                }
                            }
                            clients.push(client);
                            let _ = reply_tx.send(Ok(SfuJoinReply {
                                sdp,
                                kind: "answer",
                            }));
                        }
                        Err(e) => {
                            let _ = reply_tx.send(Err(e));
                        }
                    }
                }
                SfuCommand::LeavePeer { call_id, user_id } => {
                    let old_tags: Vec<(Uuid, String)> = clients
                        .iter()
                        .filter(|c| c.call_id == call_id && c.user_id == user_id)
                        .flat_map(|c| {
                            c.tracks_in
                                .iter()
                                .map(|t| (t.id.origin_user_id, t.id.tag.clone()))
                        })
                        .collect();
                    clients.retain(|c| !(c.call_id == call_id && c.user_id == user_id));
                    for (uid, tag) in old_tags {
                        for c in clients.iter_mut().filter(|c| c.call_id == call_id) {
                            c.handle_track_close_notify(uid, &tag);
                        }
                    }
                }
                SfuCommand::EndCall { call_id } => {
                    clients.retain(|c| c.call_id != call_id);
                }
            }
        }

        // Remove dead clients & notify subscribers of closed tracks
        let mut dead_tags: Vec<(Uuid, Uuid, String)> = Vec::new();
        clients.retain(|c| {
            if c.rtc.is_alive() {
                true
            } else {
                for t in &c.tracks_in {
                    dead_tags.push((c.call_id, t.id.origin_user_id, t.id.tag.clone()));
                }
                false
            }
        });
        for (cid, uid, tag) in dead_tags {
            for c in clients.iter_mut().filter(|c| c.call_id == cid) {
                c.handle_track_close_notify(uid, &tag);
            }
        }

        // Poll client outputs until Timeout
        let mut timeout = Instant::now() + Duration::from_millis(8);
        for client in clients.iter_mut() {
            let t = poll_until_timeout(client, &mut to_propagate, &sockets);
            timeout = timeout.min(t);
        }

        if !to_propagate.is_empty() {
            while let Some(p) = to_propagate.pop_front() {
                propagate(&p, &mut clients);
            }
            continue;
        }

        // Drain all ready UDP packets across sockets
        let mut got_packet = false;
        let recv_now = Instant::now();
        for bound in &sockets {
            loop {
                buf.resize(4096, 0);
                match bound.socket.recv_from(&mut buf) {
                    Ok((n, source)) => {
                        got_packet = true;
                        buf.truncate(n);
                        if let Ok(contents) = buf.as_slice().try_into() {
                            let input = Input::Receive(
                                recv_now,
                                Receive {
                                    proto: Protocol::Udp,
                                    source,
                                    destination: bound.local_addr,
                                    contents,
                                },
                            );
                            if let Some(client) = clients.iter_mut().find(|c| c.accepts(&input)) {
                                client.handle_input(input);
                            }
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                        break;
                    }
                    Err(e) => {
                        warn!(error = %e, "SFU UDP recv_from error");
                        break;
                    }
                }
            }
        }

        let now = Instant::now();
        for client in &mut clients {
            client.handle_input(Input::Timeout(now));
        }

        if !got_packet {
            let wait = timeout
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(5))
                .max(Duration::from_millis(1));
            thread::sleep(wait);
        }
    }
}

fn poll_until_timeout(
    client: &mut Client,
    queue: &mut VecDeque<Propagated>,
    sockets: &[BoundSocket],
) -> Instant {
    for _ in 0..256 {
        if !client.rtc.is_alive() {
            return Instant::now();
        }
        let prop = client.poll_output(sockets);
        if let Propagated::Timeout(t) = prop {
            return t;
        }
        if !matches!(prop, Propagated::Noop) {
            queue.push_back(prop);
        }
    }
    Instant::now() + Duration::from_millis(1)
}

fn propagate(p: &Propagated, clients: &mut [Client]) {
    match p {
        Propagated::TrackOpen(call_id, origin, weak) => {
            for c in clients.iter_mut() {
                if c.call_id == *call_id && c.id != *origin {
                    c.handle_track_open(weak.clone());
                }
            }
        }
        Propagated::TrackClose(call_id, origin, origin_user_id, tag) => {
            for c in clients.iter_mut() {
                if c.call_id == *call_id && c.id != *origin {
                    c.handle_track_close_notify(*origin_user_id, tag);
                }
            }
        }
        Propagated::MediaData(call_id, origin, data) => {
            for c in clients.iter_mut() {
                if c.call_id == *call_id && c.id != *origin {
                    c.handle_media_data_out(*call_id, *origin, data);
                }
            }
        }
        Propagated::KeyframeRequest(call_id, _requester, req, origin, mid_in) => {
            for c in clients.iter_mut() {
                if c.call_id == *call_id && c.id == *origin {
                    c.handle_keyframe_request(*req, *mid_in);
                }
            }
        }
        Propagated::Noop | Propagated::Timeout(_) => {}
    }
}
