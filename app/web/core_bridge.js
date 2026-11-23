// Main-Thread Controller for ChatW SharedWorker Core, Binary RPC Multiplexer,
// SW Direct MessagePort Bridge, and LRU Ref-Counted `blob:` URL Pool (Spec 071).
// Eliminates post-login Main-Thread Fetch/XHR & WebSocket connections completely.
(() => {
  if (window.__chatwCore) return;

  const TEXT_DEC = new TextDecoder('utf-8');
  const MAX_BLOB_URLS = 250;

  let port = null;
  let workerMode = 'none';
  let activeWsUrl = null;
  let nextCallId = 1;
  let swPortBridged = false;
  let simulatedOffline = false;

  const pendingCalls = new Map();
  const wsListeners = new Set();
  const tabSyncListeners = new Set();

  // LRU `blob:` URL cache so avatars/stickers/images never leak RAM or re-download
  /** @type {Map<string, { blobUrl: string, mime: string, size: number, lastUsed: number }>} */
  const blobPool = new Map();
  const blobInFlight = new Map();

  let lastStats = {
    mode: 'initializing',
    isSharedWorker: false,
    workerActive: false,
    connected: false,
    handshakeCompleted: false,
    connectedTabs: 1,
    binaryFramesReceived: 0,
    binaryFramesSent: 0,
    binaryEventsDecrypted: 0,
    rpcCallsCompleted: 0,
    mediaChunksTransferred: 0,
    plaintextFramesReceived: 0,
    reconnectCount: 0,
    lastPongAt: 0,
    lastError: null,
  };

  function defaultWsUrl() {
    const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
    return `${proto}//${location.host}/api/ws?proto=rpc-bin`;
  }

  function ensureWorkerPort() {
    if (port) return port;
    if (typeof SharedWorker !== 'undefined') {
      try {
        const sw = new SharedWorker('/core_worker.js', { name: 'chatw-core-v2' });
        port = sw.port;
        port.start();
        workerMode = 'shared';
        lastStats.workerActive = true;
        lastStats.isSharedWorker = true;
      } catch (_) {
        port = null;
      }
    }
    if (!port && typeof Worker !== 'undefined') {
      try {
        const w = new Worker('/core_worker.js');
        port = w;
        workerMode = 'dedicated';
        lastStats.workerActive = true;
        lastStats.isSharedWorker = false;
      } catch (err) {
        lastStats.lastError = String(err && err.message ? err.message : err);
        return null;
      }
    }
    if (!port) return null;

    port.onmessage = (ev) => {
      const d = ev.data;
      if (!d || typeof d !== 'object') return;
      if (d.type === 'ws_event' && typeof d.rawJson === 'string') {
        if (d.stats) lastStats = { ...lastStats, ...d.stats, workerActive: true };
        else lastStats.binaryEventsDecrypted += 1;
        for (const cb of wsListeners) {
          try { cb(d.rawJson); } catch (_) {}
        }
      } else if (d.type === 'ws_state' && d.stats) {
        const wasReady = lastStats.connected && lastStats.handshakeCompleted;
        lastStats = { ...lastStats, ...d.stats, workerActive: true };
        const isReady = lastStats.connected && lastStats.handshakeCompleted;
        if (isReady) {
          try { localStorage.setItem('chatw_auth_hint', '1'); } catch (_) {}
        }
        window.dispatchEvent(new CustomEvent('chatw:ws-state', { detail: { ...lastStats } }));
        if (!wasReady && isReady) {
          window.dispatchEvent(new CustomEvent('chatw:online-flush', { detail: { source: 'core_worker_connected' } }));
        }
      } else if (d.type === 'rpc_reply' && d.callId) {
        if (d.stats) lastStats = { ...lastStats, ...d.stats, workerActive: true };
        const entry = pendingCalls.get(d.callId);
        if (entry) {
          clearTimeout(entry.timer);
          pendingCalls.delete(d.callId);
          entry.resolve(d);
        }
      } else if (d.type === 'tab_sync') {
        window.dispatchEvent(new CustomEvent('chatw:tab-sync', { detail: { topic: d.topic, payload: d.payload } }));
        for (const cb of tabSyncListeners) {
          try { cb(d.topic, d.payload); } catch (_) {}
        }
      }
    };

    window.addEventListener('beforeunload', () => {
      try { port.postMessage({ type: 'tab_close' }); } catch (_) {}
    });

    return port;
  }

  function callWorker(msg, transfer = [], timeoutMs = 20000) {
    const p = ensureWorkerPort();
    if (!p) return Promise.resolve({ ok: false, error: 'worker_unavailable' });
    const callId = nextCallId++;
    return new Promise((resolve) => {
      const timer = setTimeout(() => {
        pendingCalls.delete(callId);
        resolve({ ok: false, error: 'worker_call_timeout' });
      }, timeoutMs);
      pendingCalls.set(callId, { resolve, timer });
      try {
        p.postMessage({ ...msg, callId }, transfer);
      } catch (err) {
        clearTimeout(timer);
        pendingCalls.delete(callId);
        resolve({ ok: false, error: String(err && err.message ? err.message : err) });
      }
    });
  }

  // Bridge a direct Transferable MessageChannel between SharedWorker and Service Worker
  // so SW `/stream/{grantId}` pulls media chunks straight from SharedWorker (0 server HTTP fetch!).
  async function bridgeServiceWorkerPort(force = false) {
    if (!('serviceWorker' in navigator)) return false;
    const ctrl = navigator.serviceWorker.controller;
    let authOk = force || Boolean(port);
    if (!authOk) {
      try { authOk = localStorage.getItem('chatw_auth_hint') === '1'; } catch (_) {}
    }
    if (!authOk) return false;
    const p = ensureWorkerPort();
    if (!ctrl || !p) return false;
    try {
      const ch = new MessageChannel();
      p.postMessage({ type: 'attach_sw_port' }, [ch.port1]);
      ctrl.postMessage({ type: 'ATTACH_WORKER_PORT' }, [ch.port2]);
      swPortBridged = true;
      return true;
    } catch (_) {
      return false;
    }
  }

  if ('serviceWorker' in navigator) {
    navigator.serviceWorker.addEventListener('controllerchange', () => {
      swPortBridged = false;
      if (port) bridgeServiceWorkerPort();
    });
    navigator.serviceWorker.ready.then(() => {
      if (port && !swPortBridged) bridgeServiceWorkerPort();
    }).catch(() => {});
  }

  // Eagerly spin up SharedWorker & connect Binary WS immediately on authenticated boot
  // (before WASM finishes loading, ensuring 0 Fetch/XHR on reload!), while avoiding
  // unauthenticated 401 WebSocket noise on the signed-out login screen.
  let hasAuthHint = false;
  try { hasAuthHint = localStorage.getItem('chatw_auth_hint') === '1'; } catch (_) {}
  if (hasAuthHint) {
    ensureWorkerPort();
    activeWsUrl = defaultWsUrl();
    if (port) {
      port.postMessage({ type: 'start', wsUrl: activeWsUrl });
    }
  }

  window.addEventListener('online', () => {
    if (port && !simulatedOffline) {
      port.postMessage({ type: 'reconnect_now' });
    }
    window.dispatchEvent(new CustomEvent('chatw:online-flush', { detail: { source: 'navigator_online' } }));
  });

  window.addEventListener('offline', () => {
    window.dispatchEvent(new CustomEvent('chatw:ws-state', { detail: { ...lastStats, connected: false } }));
  });

  function evictBlobPoolIfNeeded() {
    while (blobPool.size >= MAX_BLOB_URLS) {
      const oldestKey = blobPool.keys().next().value;
      const rec = blobPool.get(oldestKey);
      blobPool.delete(oldestKey);
      if (rec && rec.blobUrl) {
        try { URL.revokeObjectURL(rec.blobUrl); } catch (_) {}
      }
    }
  }

  function revokeAllBlobUrls() {
    for (const rec of blobPool.values()) {
      if (rec && rec.blobUrl) {
        try { URL.revokeObjectURL(rec.blobUrl); } catch (_) {}
      }
    }
    blobPool.clear();
  }

  function bytesToB64(u8) {
    let s = '';
    for (let i = 0; i < u8.length; i += 0x8000) {
      s += String.fromCharCode.apply(null, u8.subarray(i, i + 0x8000));
    }
    return btoa(s);
  }

  function b64ToBytes(b64) {
    const bin = atob(b64);
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
    return out;
  }

  const coreApi = {
    isSupported() {
      return Boolean(
        (typeof SharedWorker !== 'undefined' || typeof Worker !== 'undefined') &&
        typeof crypto !== 'undefined' &&
        crypto.subtle
      );
    },

    hasAuthHint() {
      try { return localStorage.getItem('chatw_auth_hint') === '1'; } catch (_) { return false; }
    },

    setAuthHint(flag) {
      try {
        if (flag) localStorage.setItem('chatw_auth_hint', '1');
        else localStorage.removeItem('chatw_auth_hint');
      } catch (_) {}
    },

    isConnected() {
      return Boolean(lastStats.connected && lastStats.handshakeCompleted && !simulatedOffline);
    },

    isSimulatedOffline() {
      return simulatedOffline;
    },

    async waitUntilReady(timeoutMs = 2500) {
      if (simulatedOffline) return false;
      if (this.isConnected()) return true;
      const p = ensureWorkerPort();
      if (!p) return false;
      p.postMessage({ type: 'start', wsUrl: activeWsUrl || defaultWsUrl() });
      return await new Promise((resolve) => {
        let settled = false;
        const done = (v) => {
          if (settled) return;
          settled = true;
          window.removeEventListener('chatw:ws-state', onState);
          clearTimeout(timer);
          resolve(v);
        };
        const onState = (e) => {
          if (e.detail && e.detail.connected && e.detail.handshakeCompleted) done(true);
        };
        window.addEventListener('chatw:ws-state', onState);
        const timer = setTimeout(() => done(this.isConnected()), timeoutMs);
      });
    },

    start(wsUrl) {
      const p = ensureWorkerPort();
      if (!p) return false;
      const base = String(wsUrl || defaultWsUrl()).replace(/proto=bin\b/, 'proto=rpc-bin');
      const sep = base.includes('?') ? '&' : '?';
      activeWsUrl = base.includes('proto=rpc-bin') ? base : `${base}${sep}proto=rpc-bin`;
      p.postMessage({ type: 'start', wsUrl: activeWsUrl });
      bridgeServiceWorkerPort();
      return true;
    },

    reconnectNow() {
      const p = ensureWorkerPort();
      if (!activeWsUrl) activeWsUrl = defaultWsUrl();
      if (p) p.postMessage({ type: 'reconnect_now', wsUrl: activeWsUrl });
    },

    async forceDropSocketForTest() {
      return await callWorker({ type: 'force_drop_for_test' }, [], 3000);
    },

    stop() {
      // Keep SharedWorker alive for other tabs; only detach listener on tab stop.
    },

    subscribe(fn) {
      if (typeof fn !== 'function') return () => {};
      wsListeners.add(fn);
      return () => wsListeners.delete(fn);
    },

    subscribeTabSync(fn) {
      if (typeof fn !== 'function') return () => {};
      tabSyncListeners.add(fn);
      return () => tabSyncListeners.delete(fn);
    },

    broadcastTabSync(topic, payload = {}) {
      const p = ensureWorkerPort();
      if (p) p.postMessage({ type: 'tab_sync', topic, payload });
    },

    bridgeServiceWorkerPort,

    async rpc(method, path, body = null, contentType = '', extraHeaders = {}) {
      if (simulatedOffline || navigator.onLine === false) {
        return { ok: false, status: 0, error: 'network_error', text: '' };
      }
      let transfer = [];
      let bodyToSend = null;
      if (body instanceof Uint8Array) {
        const copy = new Uint8Array(body);
        bodyToSend = copy.buffer;
        transfer = [copy.buffer];
      } else if (body instanceof ArrayBuffer) {
        const copy = body.slice(0);
        bodyToSend = copy;
        transfer = [copy];
      } else if (typeof body === 'string') {
        bodyToSend = body;
      }

      const res = await callWorker(
        {
          type: 'rpc_call',
          method: String(method || 'GET').toUpperCase(),
          path: String(path),
          contentType: contentType || (typeof body === 'string' ? 'application/json' : ''),
          headers: extraHeaders || {},
          body: bodyToSend,
        },
        transfer,
        18000,
      );
      if (!res.ok) {
        return { ok: false, status: 0, error: res.error || 'network_error', text: '' };
      }
      const u8 = new Uint8Array(res.bodyBuffer || 0);
      return {
        ok: true,
        status: res.status,
        contentType: res.contentType || '',
        bytes: u8,
        text: TEXT_DEC.decode(u8),
      };
    },

    async rpcFromRust(method, path, jsonBody, rawB64, rawContentType) {
      let body = null;
      let ct = '';
      if (rawB64) {
        body = b64ToBytes(rawB64);
        ct = rawContentType || 'application/octet-stream';
      } else if (jsonBody !== null && jsonBody !== undefined) {
        body = String(jsonBody);
        ct = 'application/json';
      }
      const res = await this.rpc(method, path, body, ct);
      if (!res.ok) {
        return { ok: false, status: 0, error: res.error || 'network_error', text: '' };
      }
      return { ok: true, status: res.status, text: res.text };
    },

    async downloadBytes(url, opts = {}) {
      if (simulatedOffline || navigator.onLine === false) {
        return { ok: false, error: 'network_error' };
      }
      const res = await callWorker(
        {
          type: 'download_media',
          url: String(url),
          keyB64: opts.keyB64 || '',
          nonceB64: opts.nonceB64 || '',
          mime: opts.mime || '',
          priority: opts.priority || 'HIGH',
          taskId: opts.taskId || '',
          scope: opts.scope || '',
        },
        [],
        25000,
      );
      if (!res.ok) return { ok: false, error: res.error || 'download_failed' };
      const u8 = new Uint8Array(res.bodyBuffer || 0);
      return { ok: true, status: res.status, contentType: res.contentType, bytes: u8, b64: bytesToB64(u8) };
    },

    // Returns a `blob:` URL backed by SharedWorker Priority DownloadManager + LRU pool.
    // Guarantees ZERO `/uploads/...` or `/api/attachments/...` URLs ever leak into DOM `<img>`.
    async downloadMediaBlobUrl(url, opts = {}) {
      if (!url || typeof url !== 'string') return null;
      if (url.startsWith('blob:') || url.startsWith('data:')) return url;
      const cacheKey = `${url}|${opts.keyB64 ? 'e2ee' : 'plain'}`;
      const hit = blobPool.get(cacheKey);
      if (hit) {
        blobPool.delete(cacheKey);
        hit.lastUsed = Date.now();
        blobPool.set(cacheKey, hit);
        return hit.blobUrl;
      }
      if (blobInFlight.has(cacheKey)) {
        return await blobInFlight.get(cacheKey);
      }

      const promise = (async () => {
        const res = await this.downloadBytes(url, opts);
        if (!res.ok || !res.bytes || res.bytes.byteLength === 0) return null;
        evictBlobPoolIfNeeded();
        const mime = (opts.mime && opts.mime !== 'application/octet-stream')
          ? opts.mime
          : (res.contentType || 'application/octet-stream');
        const blob = new Blob([res.bytes], { type: mime });
        const blobUrl = URL.createObjectURL(blob);
        blobPool.set(cacheKey, { blobUrl, mime, size: res.bytes.byteLength, lastUsed: Date.now() });
        return blobUrl;
      })().finally(() => {
        blobInFlight.delete(cacheKey);
      });

      blobInFlight.set(cacheKey, promise);
      return await promise;
    },

    abortDownload(taskId, scope) {
      const p = ensureWorkerPort();
      if (p) p.postMessage({ type: 'abort_download', taskId, scope });
    },

    async uploadPart(uploadId, partIndex, bytes) {
      const copy = new Uint8Array(bytes);
      return await callWorker(
        { type: 'upload_part', uploadId, partIndex, chunkBuffer: copy.buffer },
        [copy.buffer],
        25000,
      );
    },

    async uploadChunked(bytes, opts = {}) {
      const raw = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
      const chunkSize = Math.max(64 * 1024, Math.min(1024 * 1024, Number(opts.chunkSize || 256 * 1024)));
      const initBody = JSON.stringify({
        scope_type: opts.scopeType || 'dm',
        scope_id: opts.scopeId || null,
        file_name: opts.fileName || 'file.bin',
        mime_type: opts.mimeType || 'application/octet-stream',
        total_size: raw.byteLength,
        chunk_size: chunkSize,
        is_e2ee: opts.isE2ee !== false,
        preserve_original: opts.preserveOriginal !== false,
      });
      const initRes = await this.rpc('POST', '/api/storage/uploads/init', initBody, 'application/json');
      if (!initRes || !initRes.ok || initRes.status < 200 || initRes.status >= 300) {
        return { ok: false, status: initRes ? initRes.status : 0, error: 'upload_init_failed' };
      }
      const initData = JSON.parse(initRes.text || '{}');
      if (initData.dedup_instant && initData.download_url) {
        return { ok: true, status: 200, data: initData, partsUploaded: 0 };
      }
      const transferId = initData.transfer_id;
      const srvChunkSize = Number(initData.chunk_size || chunkSize);
      const totalChunks = Math.ceil(raw.byteLength / srvChunkSize);
      for (let idx = 0; idx < totalChunks; idx++) {
        const start = idx * srvChunkSize;
        const end = Math.min(raw.byteLength, start + srvChunkSize);
        const partRes = await this.uploadPart(transferId, idx, raw.subarray(start, end));
        if (!partRes || !partRes.ok || partRes.status < 200 || partRes.status >= 300) {
          return { ok: false, status: partRes ? partRes.status : 0, error: `upload_part_${idx}_failed` };
        }
      }
      const compRes = await this.rpc(
        'POST',
        `/api/storage/uploads/${encodeURIComponent(transferId)}/complete`,
        '{}',
        'application/json',
      );
      if (!compRes || !compRes.ok || compRes.status < 200 || compRes.status >= 300) {
        return { ok: false, status: compRes ? compRes.status : 0, error: 'upload_complete_failed' };
      }
      return {
        ok: true,
        status: compRes.status,
        data: JSON.parse(compRes.text || '{}'),
        partsUploaded: totalChunks,
      };
    },

    async deltaSync(dmPts = 0, groupPts = 0, channelPts = 0) {
      const res = await callWorker({ type: 'delta_sync', dmPts, groupPts, channelPts }, [], 15000);
      let data = null;
      if (res && typeof res.text === 'string' && res.text) {
        try { data = JSON.parse(res.text); } catch (_) {}
      }
      return { ...res, data };
    },

    async setSimulatedOffline(offline) {
      simulatedOffline = Boolean(offline);
      await callWorker({ type: 'set_simulated_offline', offline: simulatedOffline }, [], 3000);
      if (!simulatedOffline) {
        window.dispatchEvent(new CustomEvent('chatw:online-flush', { detail: { source: 'simulated_online' } }));
      }
    },

    async wipeSessionOnLogout() {
      try { localStorage.removeItem('chatw_auth_hint'); } catch (_) {}
      revokeAllBlobUrls();
      if (window.__chatwMediaStream && typeof window.__chatwMediaStream.revokeAll === 'function') {
        await window.__chatwMediaStream.revokeAll().catch(() => {});
      }
      if (port) {
        try { port.postMessage({ type: 'logout_wipe' }); } catch (_) {}
      }
      if (window.__chatwStorage && typeof window.__chatwStorage.clearSessionSecrets === 'function') {
        await window.__chatwStorage.clearSessionSecrets().catch(() => {});
      }
    },

    getBlobPoolSize() {
      return blobPool.size;
    },

    getStats() {
      if (port) {
        try { port.postMessage({ type: 'get_stats' }); } catch (_) {}
      }
      return {
        ...lastStats,
        workerMode,
        swPortBridged,
        blobPoolSize: blobPool.size,
      };
    },

    requestFlush() {
      window.dispatchEvent(new CustomEvent('chatw:online-flush', { detail: { source: 'manual' } }));
    },
  };

  window.__chatwCore = coreApi;
  // Backward-compatible alias for Spec 069 tooling & diagnostics
  window.__chatwNetBridge = coreApi;
})();
