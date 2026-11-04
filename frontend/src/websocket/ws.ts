// WebSocket client with exponential backoff reconnect and typed dispatch

import { DESKTOP_BACKEND_ORIGIN, isBundledTauriOrigin } from '../api/client';

export type WsEvent =
  | {
      type: 'message.new';
      data: {
        id: string;
        seq: number;
        sender_id: string;
        recipient_id: string;
        envelope: string;
        created_at: string;
      };
    }
  | {
      type: 'friend.request';
      data: {
        id: string;
        sender_id: string;
        sender_username: string;
        created_at: string;
      };
    }
  | {
      type: 'friend.accepted';
      data: {
        user_id: string;
        username: string;
      };
    }
  | {
      type: 'typing.start';
      data: {
        user_id: string;
      };
    }
  | {
      type: 'typing.stop';
      data: {
        user_id: string;
      };
    };

type Listener = (event: WsEvent) => void;

class WebSocketClient {
  private ws: WebSocket | null = null;
  private listeners: Set<Listener> = new Set();
  private retryDelay = 1000;
  private maxRetryDelay = 10000;
  private reconnectTimer: any = null;
  private isExplicitlyClosed = false;

  public connect() {
    this.isExplicitlyClosed = false;
    if (this.ws && (this.ws.readyState === WebSocket.OPEN || this.ws.readyState === WebSocket.CONNECTING)) {
      return;
    }

    // From the bundled Tauri origin, connect directly to the backend server.
    // Otherwise use the current host (Vite proxy in dev, same-origin backend in prod/desktop).
    let wsUrl: string;
    if (isBundledTauriOrigin()) {
      wsUrl = DESKTOP_BACKEND_ORIGIN.replace(/^http/, 'ws') + '/api/ws';
    } else {
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      wsUrl = `${protocol}//${window.location.host}/api/ws`;
    }

    try {
      this.ws = new WebSocket(wsUrl);

      this.ws.onopen = () => {
        this.retryDelay = 1000;
      };

      this.ws.onmessage = (event) => {
        try {
          const parsed = JSON.parse(event.data) as WsEvent;
          for (const listener of this.listeners) {
            listener(parsed);
          }
        } catch {
          // ignore invalid frames
        }
      };

      this.ws.onclose = () => {
        this.scheduleReconnect();
      };

      this.ws.onerror = () => {
        if (this.ws) {
          this.ws.close();
        }
      };
    } catch {
      this.scheduleReconnect();
    }
  }

  private scheduleReconnect() {
    if (this.isExplicitlyClosed) return;
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);

    this.reconnectTimer = setTimeout(() => {
      this.retryDelay = Math.min(this.retryDelay * 1.5, this.maxRetryDelay);
      this.connect();
    }, this.retryDelay);
  }

  public subscribe(listener: Listener): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  public disconnect() {
    this.isExplicitlyClosed = true;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }
}

export const wsClient = new WebSocketClient();
