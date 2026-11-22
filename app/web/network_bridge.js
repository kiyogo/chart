// Main-thread bridge for ChatW's Dedicated Network & Crypto Web Worker (Spec 069)
// Keeps WebSocket + binary AES-256-GCM decryption off the UI thread and drives online outbox flushing.
(() => {
  let worker = null;
  let activeUrl = null;
  let refCount = 0;
  const listeners = new Set();
  let lastStats = {
    mode: 'worker-binary-ecdh',
    workerActive: false,
    connected: false,
    handshakeCompleted: false,
    binaryFramesReceived: 0,
    binaryEventsDecrypted: 0,
    plaintextFramesReceived: 0,
    reconnectCount: 0,
    lastPongAt: 0,
    lastError: null,
  };

  function ensureWorker() {
    if (worker) return worker;
    if (typeof Worker === 'undefined') return null;
    try {
      worker = new Worker('/network_worker.js');
      lastStats.workerActive = true;
      worker.onmessage = (ev) => {
        const d = ev.data;
        if (!d || typeof d !== 'object') return;
        if (d.type === 'ws_event' && typeof d.rawJson === 'string') {
          if (d.stats) {
            lastStats = { ...lastStats, ...d.stats, workerActive: true };
          } else {
            lastStats.binaryEventsDecrypted += 1;
          }
          for (const cb of listeners) {
            try { cb(d.rawJson); } catch (_) {}
          }
        } else if (d.type === 'ws_state' && d.stats) {
          const wasReady = lastStats.connected && lastStats.handshakeCompleted;
          lastStats = { ...lastStats, ...d.stats, workerActive: true };
          const isReady = lastStats.connected && lastStats.handshakeCompleted;
          window.dispatchEvent(new CustomEvent('chatw:ws-state', { detail: { ...lastStats } }));
          if (!wasReady && isReady) {
            window.dispatchEvent(new CustomEvent('chatw:online-flush', { detail: { source: 'ws_reconnected' } }));
          }
        }
      };
      worker.onerror = (err) => {
        lastStats.workerActive = false;
        lastStats.lastError = (err && err.message) || 'worker_error';
      };
      return worker;
    } catch (err) {
      lastStats.workerActive = false;
      lastStats.lastError = String(err && err.message ? err.message : err);
      return null;
    }
  }

  window.addEventListener('online', () => {
    if (worker && activeUrl) {
      worker.postMessage({ type: 'reconnect_now' });
    }
    window.dispatchEvent(new CustomEvent('chatw:online-flush', { detail: { source: 'navigator_online' } }));
  });

  window.__chatwNetBridge = {
    isSupported() {
      return typeof Worker !== 'undefined' && typeof crypto !== 'undefined' && Boolean(crypto.subtle);
    },

    start(wsUrl) {
      const w = ensureWorker();
      if (!w) return false;
      refCount += 1;
      if (activeUrl !== wsUrl || !lastStats.connected) {
        activeUrl = wsUrl;
        w.postMessage({ type: 'start', wsUrl });
      }
      return true;
    },

    stop() {
      refCount = Math.max(0, refCount - 1);
      if (refCount === 0 && worker) {
        worker.postMessage({ type: 'stop' });
        activeUrl = null;
      }
    },

    subscribe(fn) {
      if (typeof fn !== 'function') return () => {};
      listeners.add(fn);
      return () => {
        listeners.delete(fn);
      };
    },

    getStats() {
      if (worker) {
        try { worker.postMessage({ type: 'get_stats' }); } catch (_) {}
      }
      return { ...lastStats };
    },

    requestFlush() {
      window.dispatchEvent(new CustomEvent('chatw:online-flush', { detail: { source: 'manual' } }));
    },
  };
})();
