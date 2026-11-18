use futures_channel::mpsc::{self, UnboundedReceiver};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::{models::WsEvent, platform};

pub struct WsHandle {
    stop: Arc<AtomicBool>,
}

impl WsHandle {
    pub fn disconnect(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Drop for WsHandle {
    fn drop(&mut self) {
        self.disconnect();
    }
}

pub fn connect_ws() -> (WsHandle, UnboundedReceiver<WsEvent>) {
    let (tx, rx) = mpsc::unbounded();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();

    dioxus::prelude::spawn(async move {
        platform::run_ws_loop(tx, stop_clone).await;
    });

    (WsHandle { stop }, rx)
}
