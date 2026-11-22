// ChatW Dedicated Network & Crypto Web Worker (Spec 069)
// Executes WebSocket I/O, P-256 ECDH + HKDF-SHA256 handshake, and CWBF1 AES-256-GCM
// binary frame decryption completely off the Main Thread so the UI stays at 60fps.
(() => {
  const CWBF_MAGIC_0 = 0x43; // 'C'
  const CWBF_MAGIC_1 = 0x57; // 'W'
  const CWBF_VERSION = 1;
  const CWBF_HEADER_LEN = 28;
  const OP_HELLO = 0x01;
  const OP_HELLO_ACK = 0x02;
  const OP_EVENT = 0x03;
  const OP_PING = 0x05;
  const OP_PONG = 0x06;
  const HKDF_INFO = new TextEncoder().encode('chatw-ws-bin-v1');
  const TEXT_DECODER = new TextDecoder('utf-8');

  let ws = null;
  let targetUrl = '';
  let running = false;
  let sessionKey = null;
  let handshakeCompleted = false;
  let retryDelayMs = 800;
  let reconnectTimer = null;
  let pingTimer = null;
  let pongTimeout = null;
  let clientSeq = 1;

  const stats = {
    mode: 'worker-binary-ecdh',
    connected: false,
    handshakeCompleted: false,
    binaryFramesReceived: 0,
    binaryEventsDecrypted: 0,
    plaintextFramesReceived: 0,
    reconnectCount: 0,
    lastPongAt: 0,
    lastError: null,
  };

  function postState() {
    self.postMessage({
      type: 'ws_state',
      stats: { ...stats },
    });
  }

  function buildPlainFrame(op, seq, payloadBytes) {
    const payload = payloadBytes ? new Uint8Array(payloadBytes) : new Uint8Array(0);
    const out = new Uint8Array(CWBF_HEADER_LEN + payload.byteLength);
    const dv = new DataView(out.buffer);
    out[0] = CWBF_MAGIC_0;
    out[1] = CWBF_MAGIC_1;
    out[2] = CWBF_VERSION;
    out[3] = op;
    dv.setBigUint64(4, BigInt(seq), false);
    // bytes 12..24 remain 0 for plaintext handshake/control frames
    dv.setUint32(24, payload.byteLength, false);
    out.set(payload, CWBF_HEADER_LEN);
    return out.buffer;
  }

  function parseFrame(buf) {
    if (!(buf instanceof ArrayBuffer) || buf.byteLength < CWBF_HEADER_LEN) return null;
    const u8 = new Uint8Array(buf);
    if (u8[0] !== CWBF_MAGIC_0 || u8[1] !== CWBF_MAGIC_1 || u8[2] !== CWBF_VERSION) return null;
    const dv = new DataView(buf);
    const op = u8[3];
    const seq = Number(dv.getBigUint64(4, false));
    const nonce = u8.subarray(12, 24);
    const payloadLen = dv.getUint32(24, false);
    if (u8.byteLength !== CWBF_HEADER_LEN + payloadLen) return null;
    return {
      op,
      seq,
      header: u8.subarray(0, CWBF_HEADER_LEN),
      nonce,
      payload: u8.subarray(CWBF_HEADER_LEN),
    };
  }

  function clearTimers() {
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
    if (pingTimer) {
      clearInterval(pingTimer);
      pingTimer = null;
    }
    if (pongTimeout) {
      clearTimeout(pongTimeout);
      pongTimeout = null;
    }
  }

  function startHeartbeat() {
    if (pingTimer) clearInterval(pingTimer);
    pingTimer = setInterval(() => {
      if (!ws || ws.readyState !== WebSocket.OPEN || !handshakeCompleted) return;
      const tsBuf = new ArrayBuffer(8);
      new DataView(tsBuf).setBigUint64(0, BigInt(Date.now()), false);
      try {
        ws.send(buildPlainFrame(OP_PING, clientSeq++, tsBuf));
      } catch (_) {
        return;
      }
      if (pongTimeout) clearTimeout(pongTimeout);
      pongTimeout = setTimeout(() => {
        if (ws && ws.readyState === WebSocket.OPEN) {
          try { ws.close(); } catch (_) {}
        }
      }, 10000);
    }, 20000);
  }

  async function handleHello(payload, activeWs) {
    if (payload.byteLength !== 81 || payload[0] !== 0x04) {
      stats.lastError = 'invalid_server_hello';
      postState();
      try { activeWs.close(); } catch (_) {}
      return;
    }
    const serverPubRaw = payload.slice(0, 65);
    const salt = payload.slice(65, 81);

    try {
      const serverPubKey = await crypto.subtle.importKey(
        'raw',
        serverPubRaw,
        { name: 'ECDH', namedCurve: 'P-256' },
        false,
        [],
      );
      const clientKp = await crypto.subtle.generateKey(
        { name: 'ECDH', namedCurve: 'P-256' },
        false,
        ['deriveBits'],
      );
      const clientPubRaw = new Uint8Array(await crypto.subtle.exportKey('raw', clientKp.publicKey));
      const sharedBits = await crypto.subtle.deriveBits(
        { name: 'ECDH', public: serverPubKey },
        clientKp.privateKey,
        256,
      );
      const hkdfBase = await crypto.subtle.importKey('raw', sharedBits, 'HKDF', false, ['deriveKey']);
      const derivedKey = await crypto.subtle.deriveKey(
        {
          name: 'HKDF',
          hash: 'SHA-256',
          salt,
          info: HKDF_INFO,
        },
        hkdfBase,
        { name: 'AES-GCM', length: 256 },
        false,
        ['encrypt', 'decrypt'],
      );

      if (ws !== activeWs || activeWs.readyState !== WebSocket.OPEN) return;

      sessionKey = derivedKey;
      activeWs.send(buildPlainFrame(OP_HELLO_ACK, 0, clientPubRaw));
      handshakeCompleted = true;
      retryDelayMs = 800;
      stats.connected = true;
      stats.handshakeCompleted = true;
      stats.lastError = null;
      startHeartbeat();
      postState();
    } catch (err) {
      stats.lastError = String(err && err.message ? err.message : err);
      postState();
      try { activeWs.close(); } catch (_) {}
    }
  }

  async function handleEncryptedEvent(frame) {
    if (!sessionKey || !handshakeCompleted) return;
    try {
      const plainBuf = await crypto.subtle.decrypt(
        {
          name: 'AES-GCM',
          iv: frame.nonce,
          additionalData: frame.header,
        },
        sessionKey,
        frame.payload,
      );
      const rawJson = TEXT_DECODER.decode(plainBuf);
      stats.binaryEventsDecrypted += 1;
      self.postMessage({
        type: 'ws_event',
        rawJson,
        stats: { ...stats },
      });
    } catch (err) {
      stats.lastError = 'frame_decrypt_failed';
      postState();
    }
  }

  function scheduleReconnect() {
    if (!running) return;
    clearTimers();
    const jitter = Math.floor(Math.random() * 250);
    const wait = Math.min(10000, Math.round(retryDelayMs) + jitter);
    retryDelayMs = Math.min(10000, retryDelayMs * 1.5);
    reconnectTimer = setTimeout(() => {
      if (running) {
        stats.reconnectCount += 1;
        connect();
      }
    }, wait);
  }

  function connect() {
    if (!running || !targetUrl) return;
    clearTimers();
    sessionKey = null;
    handshakeCompleted = false;
    stats.connected = false;
    stats.handshakeCompleted = false;

    let socket;
    try {
      socket = new WebSocket(targetUrl);
      socket.binaryType = 'arraybuffer';
    } catch (err) {
      stats.lastError = String(err && err.message ? err.message : err);
      postState();
      scheduleReconnect();
      return;
    }

    ws = socket;

    socket.onmessage = (ev) => {
      if (ws !== socket) return;
      if (ev.data instanceof ArrayBuffer) {
        stats.binaryFramesReceived += 1;
        const frame = parseFrame(ev.data);
        if (!frame) return;
        if (frame.op === OP_HELLO) {
          handleHello(frame.payload, socket);
        } else if (frame.op === OP_EVENT) {
          handleEncryptedEvent(frame);
        } else if (frame.op === OP_PONG) {
          if (pongTimeout) {
            clearTimeout(pongTimeout);
            pongTimeout = null;
          }
          stats.lastPongAt = Date.now();
        }
      } else if (typeof ev.data === 'string') {
        // Fallback compatibility if a proxy ever downgrades a frame
        stats.plaintextFramesReceived += 1;
        self.postMessage({ type: 'ws_event', rawJson: ev.data });
      }
    };

    socket.onclose = () => {
      if (ws !== socket) return;
      ws = null;
      sessionKey = null;
      handshakeCompleted = false;
      stats.connected = false;
      stats.handshakeCompleted = false;
      postState();
      scheduleReconnect();
    };

    socket.onerror = () => {
      if (ws !== socket) return;
      try { socket.close(); } catch (_) {}
    };
  }

  self.onmessage = (ev) => {
    const msg = ev.data;
    if (!msg || typeof msg !== 'object') return;
    if (msg.type === 'start') {
      const rawUrl = String(msg.wsUrl || '');
      if (!rawUrl) return;
      const sep = rawUrl.includes('?') ? '&' : '?';
      targetUrl = rawUrl.includes('proto=bin') ? rawUrl : `${rawUrl}${sep}proto=bin`;
      running = true;
      retryDelayMs = 800;
      if (ws) {
        try { ws.close(); } catch (_) {}
        ws = null;
      }
      connect();
    } else if (msg.type === 'stop') {
      running = false;
      clearTimers();
      if (ws) {
        const prev = ws;
        ws = null;
        try { prev.close(); } catch (_) {}
      }
      stats.connected = false;
      stats.handshakeCompleted = false;
      postState();
    } else if (msg.type === 'reconnect_now') {
      if (!running || !targetUrl) return;
      retryDelayMs = 800;
      if (ws && ws.readyState === WebSocket.OPEN && handshakeCompleted) {
        return;
      }
      if (ws) {
        try { ws.close(); } catch (_) {}
        ws = null;
      }
      connect();
    } else if (msg.type === 'get_stats') {
      postState();
    }
  };
})();
