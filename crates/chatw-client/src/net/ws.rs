use std::sync::Arc;
use futures_util::{SinkExt, StreamExt};
use reqwest::cookie::{CookieStore, Jar};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use chatw_core::{WsClientCommand, WsServerEvent};

pub struct WsManager {
    pub rx_events: mpsc::UnboundedReceiver<WsServerEvent>,
    pub tx_commands: mpsc::UnboundedSender<WsClientCommand>,
}

impl WsManager {
    pub fn start(
        ws_url: String,
        cookie_jar: Arc<Jar>,
        ctx: egui::Context,
    ) -> Self {
        let (tx_events, rx_events) = mpsc::unbounded_channel();
        let (tx_commands, mut rx_commands) = mpsc::unbounded_channel::<WsClientCommand>();

        tokio::spawn(async move {
            let mut retry_delay = std::time::Duration::from_millis(1000);

            loop {
                // Build HTTP request for WebSocket with cookies
                let dummy_url = "http://127.0.0.1:8082/api/ws".parse::<reqwest::Url>().unwrap();
                let cookie_header = cookie_jar.cookies(&dummy_url);

                let mut req = match ws_url.clone().into_client_request() {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Invalid WS URL: {e}");
                        break;
                    }
                };

                if let Some(cookie_val) = cookie_header {
                    if let Ok(hdr) = cookie_val.to_str() {
                        req.headers_mut().insert(
                            "Cookie",
                            hdr.parse().unwrap(),
                        );
                    }
                }

                match connect_async(req).await {
                    Ok((ws_stream, _)) => {
                        retry_delay = std::time::Duration::from_millis(1000);
                        let (mut write, mut read) = ws_stream.split();

                        loop {
                            tokio::select! {
                                Some(msg) = read.next() => {
                                    match msg {
                                        Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                                            if let Ok(event) = serde_json::from_str::<WsServerEvent>(&text) {
                                                let _ = tx_events.send(event);
                                                ctx.request_repaint();
                                            }
                                        }
                                        Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => {
                                            break;
                                        }
                                        Err(_) => {
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                                Some(cmd) = rx_commands.recv() => {
                                    if let Ok(json) = serde_json::to_string(&cmd) {
                                        let _ = write.send(tokio_tungstenite::tungstenite::Message::Text(json.into())).await;
                                    }
                                }
                                else => break,
                            }
                        }
                    }
                    Err(_) => {
                        tokio::time::sleep(retry_delay).await;
                        retry_delay = (retry_delay * 2).min(std::time::Duration::from_secs(10));
                    }
                }
            }
        });

        Self {
            rx_events,
            tx_commands,
        }
    }
}
