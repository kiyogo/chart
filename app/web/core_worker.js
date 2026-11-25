// ChatW Unified SharedWorker Core Engine (Spec 071 — Telegram Web K `tweb` Architecture)
// Auto-detects SharedWorkerGlobalScope (multi-tab shared socket) vs DedicatedWorkerGlobalScope fallback.
// Owns:
//  1. Single CWBF2 Encrypted Binary WebSocket (/api/ws?proto=rpc-bin): ECDH P-256 + HKDF-SHA256 + AES-256-GCM,
//     34B header AAD, monotonic seq anti-replay, Zlib deflate (>512B), 16B random padding.
//  2. Multiplexed Binary RPC (`OP_RPC_CALL`/`OP_RPC_RES`) eliminating 100% of Main-Thread Fetch/XHR.
//  3. 4-Lane Priority DownloadManager (`CRITICAL > HIGH > NORMAL > LOW`, cap 6, Abort, E2EE decrypt, prefetch).
//  4. Direct Service Worker MessagePort bridge (zero-copy Transferable chunks -> SW 206 streaming, 0 server fetch).
//  5. Resumable Chunked Binary Upload (`OP_UPLOAD_PART`/`OP_UPLOAD_ACK`).
//  6. PTS Delta Sync (`OP_DELTA_SYNC_REQ`/`OP_DELTA_SYNC_RES`) & <5ms Multi-Tab Sync.
(() => {
  const CWBF_MAGIC_0 = 0x43; // 'C'
  const CWBF_MAGIC_1 = 0x57; // 'W'
  const CWBF2_VERSION = 2;
  const CWBF2_HEADER_LEN = 34;

  const OP_HELLO = 0x01;
  const OP_HELLO_ACK = 0x02;
  const OP_EVENT = 0x03;
  const OP_PING = 0x05;
  const OP_PONG = 0x06;
  const OP_RPC_CALL = 0x10;
  const OP_RPC_RES = 0x11;
  const OP_MEDIA_CHUNK_REQ = 0x12;
  const OP_MEDIA_CHUNK_RES = 0x13;
  const OP_UPLOAD_PART = 0x14;
  const OP_UPLOAD_ACK = 0x15;
  const OP_DELTA_SYNC_REQ = 0x16;
  const OP_DELTA_SYNC_RES = 0x17;

  const FLAG_COMPRESSED = 0x01;
  const FLAG_PADDED = 0x02;

  const HKDF_INFO = new TextEncoder().encode('chatw-ws-bin-v1');
  const TEXT_ENC = new TextEncoder();
  const TEXT_DEC = new TextDecoder('utf-8');
  const CWCH2_MAGIC = [0x43, 0x57, 0x43, 0x48, 0x32]; // "CWCH2"
  const CWCH2_HEADER_LEN = 21;
  const COMPRESS_THRESHOLD = 512;
  const METHOD_CODES = { GET: 0, POST: 1, PUT: 2, DELETE: 3, PATCH: 4 };

  const IS_SHARED = typeof SharedWorkerGlobalScope !== 'undefined' && self instanceof SharedWorkerGlobalScope;

  let ws = null;
  let targetUrl = '';
  let running = false;
  let simulatedOffline = false;
  let sessionKey = null;
  let handshakeCompleted = false;
  let clientSeq = 1;
  let serverHighestSeq = 0;
  let serverReplayMask = 0n;
  let nextReqId = 1;
  let retryDelayMs = 600;
  let reconnectTimer = null;
  let pingTimer = null;
  let pongTimeout = null;
  let nextTabId = 1;
  let authFailedCloseCount = 0;
  let everAuthenticated = false;

  /** @type {Map<number, MessagePort>} */
  const tabs = new Map();
  /** @type {Map<number, { resolve: Function, reject: Function, timer: any, op: number }>} */
  const pendingRpcs = new Map();
  /** @type {Array<{ op: number, reqId: number, payload: Uint8Array, compress: boolean, resolve: Function, reject: Function, timeoutMs: number }>} */
  const preHandshakeQueue = [];
  /** @type {Set<MessagePort>} */
  const swPorts = new Set();

  // Bounded RAM LRU cache for media chunks & full small assets (avatars/stickers/images)
  const MAX_MEDIA_CACHE_ENTRIES = 96;
  /** @type {Map<string, { bytes: Uint8Array, totalSize: number, contentType: string, status: number }>} */
  const mediaCache = new Map();
  const inFlightChunkReqs = new Map();

  const stats = {
    mode: IS_SHARED ? 'shared-worker-cwbf2-rpc' : 'dedicated-worker-cwbf2-rpc',
    isSharedWorker: IS_SHARED,
    connected: false,
    handshakeCompleted: false,
    connectedTabs: 0,
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

  function lruGet(key) {
    const v = mediaCache.get(key);
    if (!v) return null;
    mediaCache.delete(key);
    mediaCache.set(key, v);
    return v;
  }

  function lruSet(key, val) {
    if (mediaCache.has(key)) mediaCache.delete(key);
    mediaCache.set(key, val);
    while (mediaCache.size > MAX_MEDIA_CACHE_ENTRIES) {
      const oldest = mediaCache.keys().next().value;
      mediaCache.delete(oldest);
    }
  }

  function postToTab(port, msg, transfer = []) {
    try {
      port.postMessage(msg, transfer);
    } catch (_) {}
  }

  function broadcastAll(msg) {
    stats.connectedTabs = tabs.size;
    for (const port of tabs.values()) {
      postToTab(port, msg);
    }
  }

  function broadcastState() {
    stats.connectedTabs = tabs.size;
    broadcastAll({ type: 'ws_state', stats: { ...stats } });
  }

  function b64ToU8(b64) {
    const norm = String(b64 || '').replace(/-/g, '+').replace(/_/g, '/');
    const pad = norm.length % 4 === 0 ? '' : '='.repeat(4 - (norm.length % 4));
    const bin = atob(norm + pad);
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
    return out;
  }

  // ── Zlib Deflate / Inflate via Native Web Streams (matches Rust flate2::ZlibEncoder) ──
  async function zlibCompress(u8) {
    if (typeof CompressionStream === 'undefined') return null;
    try {
      const cs = new CompressionStream('deflate');
      const writer = cs.writable.getWriter();
      writer.write(u8);
      writer.close();
      const buf = await new Response(cs.readable).arrayBuffer();
      return new Uint8Array(buf);
    } catch (_) {
      return null;
    }
  }

  async function zlibDecompress(u8) {
    if (typeof DecompressionStream === 'undefined') {
      throw new Error('decompression_stream_unavailable');
    }
    const ds = new DecompressionStream('deflate');
    const writer = ds.writable.getWriter();
    writer.write(u8);
    writer.close();
    const buf = await new Response(ds.readable).arrayBuffer();
    return new Uint8Array(buf);
  }

  // ── 16-Byte Aligned Random Padding (Anti-Traffic-Analysis) ──────────────────
  function pad16(u8) {
    const rem = u8.byteLength % 16;
    const basePad = rem === 0 ? 16 : 16 - rem;
    const extraByte = crypto.getRandomValues(new Uint8Array(1))[0];
    const extra = u8.byteLength < 256 && (extraByte & 1) === 1 ? 16 : 0;
    const padLen = basePad + extra; // 1..=32
    const out = new Uint8Array(u8.byteLength + padLen);
    out.set(u8, 0);
    if (padLen > 1) {
      out.set(crypto.getRandomValues(new Uint8Array(padLen - 1)), u8.byteLength);
    }
    out[out.byteLength - 1] = padLen;
    return out;
  }

  function unpad16(u8) {
    if (u8.byteLength === 0 || u8.byteLength % 16 !== 0) {
      throw new Error('invalid_padded_length');
    }
    const padLen = u8[u8.byteLength - 1];
    if (padLen === 0 || padLen > 32 || padLen > u8.byteLength) {
      throw new Error('invalid_pad_byte');
    }
    return u8.subarray(0, u8.byteLength - padLen);
  }

  function buildHeader(op, flags, seq, reqId, nonce, payloadLen) {
    const hdr = new Uint8Array(CWBF2_HEADER_LEN);
    const dv = new DataView(hdr.buffer);
    hdr[0] = CWBF_MAGIC_0;
    hdr[1] = CWBF_MAGIC_1;
    hdr[2] = CWBF2_VERSION;
    hdr[3] = op;
    hdr[4] = flags;
    hdr[5] = 0;
    dv.setBigUint64(6, BigInt(seq), false);
    dv.setUint32(14, reqId >>> 0, false);
    if (nonce) hdr.set(nonce, 18);
    dv.setUint32(30, payloadLen >>> 0, false);
    return hdr;
  }

  function buildPlainFrame(op, seq, reqId, payload) {
    const body = payload ? new Uint8Array(payload) : new Uint8Array(0);
    const hdr = buildHeader(op, 0, seq, reqId, null, body.byteLength);
    const out = new Uint8Array(CWBF2_HEADER_LEN + body.byteLength);
    out.set(hdr, 0);
    out.set(body, CWBF2_HEADER_LEN);
    return out.buffer;
  }

  function parseHeader(buf) {
    if (!(buf instanceof ArrayBuffer) || buf.byteLength < CWBF2_HEADER_LEN) return null;
    const u8 = new Uint8Array(buf);
    if (u8[0] !== CWBF_MAGIC_0 || u8[1] !== CWBF_MAGIC_1 || u8[2] !== CWBF2_VERSION) return null;
    const dv = new DataView(buf);
    const op = u8[3];
    const flags = u8[4];
    const seq = Number(dv.getBigUint64(6, false));
    const reqId = dv.getUint32(14, false);
    const nonce = u8.subarray(18, 30);
    const len = dv.getUint32(30, false);
    if (u8.byteLength !== CWBF2_HEADER_LEN + len) return null;
    return {
      op,
      flags,
      seq,
      reqId,
      header: u8.subarray(0, CWBF2_HEADER_LEN),
      nonce,
      payload: u8.subarray(CWBF2_HEADER_LEN),
    };
  }

  async function encodeEncryptedFrame(op, reqId, rawPayload, allowCompress) {
    let flags = FLAG_PADDED;
    let body = rawPayload;
    if (allowCompress && rawPayload.byteLength > COMPRESS_THRESHOLD) {
      const comp = await zlibCompress(rawPayload);
      if (comp && comp.byteLength < rawPayload.byteLength) {
        flags |= FLAG_COMPRESSED;
        body = comp;
      }
    }
    const padded = pad16(body);
    const nonce = crypto.getRandomValues(new Uint8Array(12));
    const seq = clientSeq++;
    const ctLen = padded.byteLength + 16;
    const hdr = buildHeader(op, flags, seq, reqId, nonce, ctLen);
    const ctBuf = await crypto.subtle.encrypt(
      { name: 'AES-GCM', iv: nonce, additionalData: hdr },
      sessionKey,
      padded,
    );
    const out = new Uint8Array(CWBF2_HEADER_LEN + ctBuf.byteLength);
    out.set(hdr, 0);
    out.set(new Uint8Array(ctBuf), CWBF2_HEADER_LEN);
    return out.buffer;
  }

  async function decodeEncryptedPayload(frame) {
    const plainBuf = await crypto.subtle.decrypt(
      { name: 'AES-GCM', iv: frame.nonce, additionalData: frame.header },
      sessionKey,
      frame.payload,
    );
    let data = new Uint8Array(plainBuf);
    if ((frame.flags & FLAG_PADDED) !== 0) {
      data = unpad16(data);
    }
    if ((frame.flags & FLAG_COMPRESSED) !== 0) {
      data = await zlibDecompress(data);
    }
    return data;
  }

  // ── Serialized Outgoing Encrypt/Send Queue (preserves strict monotonic seq order!) ──
  // CRITICAL: Never let a rejected send poison `sendChain` for subsequent RPCs!
  let sendChain = Promise.resolve();
  function sendEncrypted(op, reqId, payload, allowCompress) {
    const task = sendChain.then(async () => {
      if (!ws || ws.readyState !== WebSocket.OPEN || !handshakeCompleted || !sessionKey || simulatedOffline) {
        throw new Error('ws_not_ready');
      }
      const buf = await encodeEncryptedFrame(op, reqId, payload, allowCompress);
      if (!ws || ws.readyState !== WebSocket.OPEN || simulatedOffline) {
        throw new Error('ws_closed');
      }
      ws.send(buf);
      stats.binaryFramesSent += 1;
    });
    sendChain = task.catch(() => {});
    return task;
  }

  function sendRpcRaw(op, payload, allowCompress = true, timeoutMs = 15000) {
    if (simulatedOffline) {
      return Promise.reject(new Error('offline'));
    }
    const reqId = (nextReqId++) >>> 0 || (nextReqId = 2);
    return new Promise((resolve, reject) => {
      const dispatch = () => {
        const timer = setTimeout(() => {
          pendingRpcs.delete(reqId);
          reject(new Error('rpc_timeout'));
        }, timeoutMs);
        pendingRpcs.set(reqId, { resolve, reject, timer, op });
        sendEncrypted(op, reqId, payload, allowCompress).catch((err) => {
          const entry = pendingRpcs.get(reqId);
          if (entry) {
            clearTimeout(entry.timer);
            pendingRpcs.delete(reqId);
            entry.reject(err);
          }
        });
      };

      if (ws && ws.readyState === WebSocket.OPEN && handshakeCompleted && sessionKey) {
        dispatch();
      } else if (running && !simulatedOffline && (everAuthenticated || authFailedCloseCount < 4)) {
        if (!ws || ws.readyState === WebSocket.CLOSED) {
          connect();
        }
        const qTimer = setTimeout(() => {
          const idx = preHandshakeQueue.findIndex((q) => q.reqId === reqId);
          if (idx >= 0) preHandshakeQueue.splice(idx, 1);
          reject(new Error('ws_handshake_timeout'));
        }, Math.min(timeoutMs, 3500));
        preHandshakeQueue.push({ op, reqId, payload, allowCompress, resolve, reject, timeoutMs, qTimer });
      } else {
        reject(new Error('ws_offline'));
      }
    });
  }

  function flushPreHandshakeQueue() {
    while (preHandshakeQueue.length > 0) {
      const item = preHandshakeQueue.shift();
      clearTimeout(item.qTimer);
      const timer = setTimeout(() => {
        pendingRpcs.delete(item.reqId);
        item.reject(new Error('rpc_timeout'));
      }, item.timeoutMs);
      pendingRpcs.set(item.reqId, { resolve: item.resolve, reject: item.reject, timer, op: item.op });
      sendEncrypted(item.op, item.reqId, item.payload, item.allowCompress).catch((err) => {
        const e = pendingRpcs.get(item.reqId);
        if (e) {
          clearTimeout(e.timer);
          pendingRpcs.delete(item.reqId);
          e.reject(err);
        }
      });
    }
  }

  function rejectAllPending(reason) {
    while (preHandshakeQueue.length > 0) {
      const item = preHandshakeQueue.shift();
      clearTimeout(item.qTimer);
      item.reject(new Error(reason));
    }
    for (const [rid, entry] of pendingRpcs.entries()) {
      clearTimeout(entry.timer);
      entry.reject(new Error(reason));
      pendingRpcs.delete(rid);
    }
  }

  // ── Binary Payload Encoders/Decoders ────────────────────────────────────────
  function encodeRpcCall(method, path, contentType, headersObj, bodyBytes) {
    const mCode = METHOD_CODES[String(method || 'GET').toUpperCase()] ?? 0;
    const pBytes = TEXT_ENC.encode(String(path || '/'));
    const ctBytes = TEXT_ENC.encode(String(contentType || ''));
    const hdrEntries = Object.entries(headersObj || {}).slice(0, 16);
    const encHdrs = hdrEntries.map(([k, v]) => [TEXT_ENC.encode(String(k)), TEXT_ENC.encode(String(v))]);
    let hdrByteLen = 0;
    for (const [kb, vb] of encHdrs) hdrByteLen += 4 + kb.byteLength + vb.byteLength;
    const bBytes = bodyBytes instanceof Uint8Array ? bodyBytes : new Uint8Array(bodyBytes || 0);

    const out = new Uint8Array(1 + 2 + pBytes.byteLength + 2 + ctBytes.byteLength + 1 + hdrByteLen + bBytes.byteLength);
    const dv = new DataView(out.buffer);
    let pos = 0;
    out[pos++] = mCode;
    dv.setUint16(pos, pBytes.byteLength, false); pos += 2;
    out.set(pBytes, pos); pos += pBytes.byteLength;
    dv.setUint16(pos, ctBytes.byteLength, false); pos += 2;
    out.set(ctBytes, pos); pos += ctBytes.byteLength;
    out[pos++] = encHdrs.length;
    for (const [kb, vb] of encHdrs) {
      dv.setUint16(pos, kb.byteLength, false); pos += 2;
      out.set(kb, pos); pos += kb.byteLength;
      dv.setUint16(pos, vb.byteLength, false); pos += 2;
      out.set(vb, pos); pos += vb.byteLength;
    }
    out.set(bBytes, pos);
    return out;
  }

  function decodeRpcRes(u8) {
    if (u8.byteLength < 4) throw new Error('short_rpc_res');
    const dv = new DataView(u8.buffer, u8.byteOffset, u8.byteLength);
    const status = dv.getUint16(0, false);
    const ctLen = dv.getUint16(2, false);
    const contentType = TEXT_DEC.decode(u8.subarray(4, 4 + ctLen));
    const body = u8.slice(4 + ctLen);
    return { status, contentType, body };
  }

  function encodeMediaChunkReq(path, offset, limit) {
    const pBytes = TEXT_ENC.encode(String(path));
    const out = new Uint8Array(2 + pBytes.byteLength + 12);
    const dv = new DataView(out.buffer);
    dv.setUint16(0, pBytes.byteLength, false);
    out.set(pBytes, 2);
    dv.setBigUint64(2 + pBytes.byteLength, BigInt(Math.max(0, offset)), false);
    dv.setUint32(10 + pBytes.byteLength, Math.max(0, limit) >>> 0, false);
    return out;
  }

  function decodeMediaChunkRes(u8) {
    if (u8.byteLength < 12) throw new Error('short_media_res');
    const dv = new DataView(u8.buffer, u8.byteOffset, u8.byteLength);
    const status = dv.getUint16(0, false);
    const totalSize = Number(dv.getBigUint64(2, false));
    const ctLen = dv.getUint16(10, false);
    const contentType = TEXT_DEC.decode(u8.subarray(12, 12 + ctLen));
    const bytes = u8.slice(12 + ctLen);
    return { status, totalSize, contentType, bytes };
  }

  // ── 4-Lane Priority DownloadManager (CRITICAL=0 > HIGH=1 > NORMAL=2 > LOW=3) ──
  const MAX_DOWNLOAD_CONCURRENCY = 6;
  const PRIORITY_MAP = { CRITICAL: 0, HIGH: 1, NORMAL: 2, LOW: 3 };
  let activeDownloads = 0;
  /** @type {Array<{ taskId: string, scope: string, priority: number, aborted: boolean, run: Function, resolve: Function, reject: Function }>} */
  const dlQueue = [];
  const activeAbortSet = new Set();

  function enqueueDownload(taskId, scope, priorityLabel, runFn) {
    const prio = PRIORITY_MAP[String(priorityLabel || 'NORMAL').toUpperCase()] ?? 2;
    return new Promise((resolve, reject) => {
      dlQueue.push({ taskId: taskId || '', scope: scope || '', priority: prio, aborted: false, run: runFn, resolve, reject });
      dlQueue.sort((a, b) => a.priority - b.priority);
      pumpDownloadQueue();
    });
  }

  function abortDownloads({ taskId, scope }) {
    if (taskId) activeAbortSet.add(taskId);
    for (let i = dlQueue.length - 1; i >= 0; i--) {
      const item = dlQueue[i];
      if ((taskId && item.taskId === taskId) || (scope && item.scope === scope)) {
        item.aborted = true;
        dlQueue.splice(i, 1);
        item.reject(new Error('aborted'));
      }
    }
  }

  function pumpDownloadQueue() {
    while (activeDownloads < MAX_DOWNLOAD_CONCURRENCY && dlQueue.length > 0) {
      const job = dlQueue.shift();
      if (job.aborted || (job.taskId && activeAbortSet.has(job.taskId))) {
        activeAbortSet.delete(job.taskId);
        job.reject(new Error('aborted'));
        continue;
      }
      activeDownloads += 1;
      Promise.resolve()
        .then(() => job.run())
        .then((res) => {
          if (job.taskId && activeAbortSet.has(job.taskId)) {
            activeAbortSet.delete(job.taskId);
            job.reject(new Error('aborted'));
          } else {
            job.resolve(res);
          }
        })
        .catch((err) => job.reject(err))
        .finally(() => {
          activeDownloads = Math.max(0, activeDownloads - 1);
          pumpDownloadQueue();
        });
    }
  }

  async function fetchMediaRangeViaWs(url, offset, limit, priority = 'NORMAL', taskId = '', scope = '') {
    const cacheKey = `${url}|${offset}|${limit}`;
    const hit = lruGet(cacheKey);
    if (hit) return hit;

    // Deduplicate identical concurrent chunk fetches
    if (inFlightChunkReqs.has(cacheKey)) {
      return await inFlightChunkReqs.get(cacheKey);
    }

    const promise = enqueueDownload(taskId, scope, priority, async () => {
      const rawRes = await sendRpcRaw(OP_MEDIA_CHUNK_REQ, encodeMediaChunkReq(url, offset, limit), false, 20000);
      const parsed = decodeMediaChunkRes(rawRes);
      stats.mediaChunksTransferred += 1;
      if (parsed.status === 200 || parsed.status === 206) {
        lruSet(cacheKey, parsed);
      }
      return parsed;
    }).finally(() => {
      inFlightChunkReqs.delete(cacheKey);
    });

    inFlightChunkReqs.set(cacheKey, promise);
    return await promise;
  }

  const CWCH3_MAGIC = new TextEncoder().encode('CWCH3');
  const CWCH3_NONCE_PREFIX = new TextEncoder().encode('chatw-ch3-n!');
  const CWCH3_HEADER_LEN = 34;

  async function decryptE2eeBlobInWorker(rawBytes, keyB64, nonceB64) {
    const keyBytes = b64ToU8(keyB64);
    const baseNonce = b64ToU8(nonceB64);
    const aesKey = await crypto.subtle.importKey('raw', keyBytes, { name: 'AES-GCM' }, false, ['decrypt']);

    // Check CWCH3 HKDF-nonce chunked format (>= 512 KiB E2EE payloads)
    let isCwch3 = rawBytes.byteLength >= CWCH3_HEADER_LEN;
    for (let i = 0; isCwch3 && i < 5; i++) {
      if (rawBytes[i] !== CWCH3_MAGIC[i]) isCwch3 = false;
    }
    if (isCwch3 && rawBytes[5] === 3) {
      const dv = new DataView(rawBytes.buffer, rawBytes.byteOffset, rawBytes.byteLength);
      const chunkCount = dv.getUint32(10, false);
      const totalPlain = Number(dv.getBigUint64(14, false));
      const mediaTag = rawBytes.subarray(22, 30);
      const variantTag = rawBytes.subarray(30, 34);
      const hkdfKey = await crypto.subtle.importKey('raw', keyBytes, 'HKDF', false, ['deriveBits']);
      const out = new Uint8Array(totalPlain);
      let pos = CWCH3_HEADER_LEN;
      let writeOff = 0;
      for (let idx = 0; idx < chunkCount; idx++) {
        const ctLen = dv.getUint32(pos, false);
        pos += 4;
        const ct = rawBytes.subarray(pos, pos + ctLen);
        pos += ctLen;
        const info = new Uint8Array(28);
        info.set(CWCH3_NONCE_PREFIX, 0);
        info.set(mediaTag, 12);
        info.set(variantTag, 20);
        new DataView(info.buffer).setUint32(24, idx, false);
        const cNonce = new Uint8Array(
          await crypto.subtle.deriveBits({ name: 'HKDF', hash: 'SHA-256', salt: baseNonce, info }, hkdfKey, 96),
        );
        const aad = new Uint8Array(27);
        aad.set(CWCH3_MAGIC, 0);
        aad[5] = 3;
        aad.set(mediaTag, 6);
        aad.set(variantTag, 14);
        const aadDv = new DataView(aad.buffer);
        aadDv.setUint32(18, idx, false);
        aadDv.setUint32(22, chunkCount, false);
        aad[26] = idx === chunkCount - 1 ? 1 : 0;
        const pt = new Uint8Array(
          await crypto.subtle.decrypt({ name: 'AES-GCM', iv: cNonce, additionalData: aad }, aesKey, ct),
        );
        out.set(pt, writeOff);
        writeOff += pt.byteLength;
      }
      return out;
    }

    // Check CWCH2 chunked format
    let isCwch2 = rawBytes.byteLength >= CWCH2_HEADER_LEN;
    for (let i = 0; isCwch2 && i < 5; i++) {
      if (rawBytes[i] !== CWCH2_MAGIC[i]) isCwch2 = false;
    }
    if (isCwch2) {
      const dv = new DataView(rawBytes.buffer, rawBytes.byteOffset, rawBytes.byteLength);
      const chunkCount = dv.getUint32(9, false);
      const totalPlain = Number(dv.getBigUint64(13, false));
      const out = new Uint8Array(totalPlain);
      let pos = CWCH2_HEADER_LEN;
      let writeOff = 0;
      for (let idx = 0; idx < chunkCount; idx++) {
        const ctLen = dv.getUint32(pos, false);
        pos += 4;
        const ct = rawBytes.subarray(pos, pos + ctLen);
        pos += ctLen;
        const cNonce = new Uint8Array(baseNonce);
        cNonce[8] ^= (idx >>> 24) & 0xff;
        cNonce[9] ^= (idx >>> 16) & 0xff;
        cNonce[10] ^= (idx >>> 8) & 0xff;
        cNonce[11] ^= idx & 0xff;
        const aad = new Uint8Array(10);
        aad.set(CWCH2_MAGIC, 0);
        new DataView(aad.buffer).setUint32(5, idx, false);
        aad[9] = idx === chunkCount - 1 ? 1 : 0;
        const pt = new Uint8Array(await crypto.subtle.decrypt({ name: 'AES-GCM', iv: cNonce, additionalData: aad }, aesKey, ct));
        out.set(pt, writeOff);
        writeOff += pt.byteLength;
      }
      return out;
    }

    const pt = await crypto.subtle.decrypt({ name: 'AES-GCM', iv: baseNonce }, aesKey, rawBytes);
    return new Uint8Array(pt);
  }

  async function downloadFullMediaInWorker({ url, keyB64 = '', nonceB64 = '', mime = '', priority = 'HIGH', taskId = '', scope = '' }) {
    const fullKey = `full|${url}|${keyB64 ? 'e2ee' : 'plain'}`;
    const hit = lruGet(fullKey);
    if (hit) return hit;

    // Fetch via Binary WS (512 KiB chunks for large objects, or single shot for small assets/avatars)
    const first = await fetchMediaRangeViaWs(url, 0, 512 * 1024, priority, taskId, scope);
    if (first.status !== 200 && first.status !== 206) {
      throw new Error(`media_http_${first.status}`);
    }
    let assembled = first.bytes;
    const total = first.totalSize || first.bytes.byteLength;
    if (first.status === 206 && assembled.byteLength < total) {
      const buf = new Uint8Array(total);
      buf.set(assembled, 0);
      let off = assembled.byteLength;
      while (off < total) {
        const next = await fetchMediaRangeViaWs(url, off, Math.min(512 * 1024, total - off), priority, taskId, scope);
        if (next.status !== 206 && next.status !== 200) throw new Error(`media_chunk_http_${next.status}`);
        buf.set(next.bytes, off);
        off += next.bytes.byteLength;
        if (next.bytes.byteLength === 0) break;
      }
      assembled = buf;
    }

    const plainBytes = keyB64 ? await decryptE2eeBlobInWorker(assembled, keyB64, nonceB64) : assembled;
    const finalMime = (mime && mime !== 'application/octet-stream') ? mime : (first.contentType || 'application/octet-stream');
    const result = { status: 200, totalSize: plainBytes.byteLength, contentType: finalMime, bytes: plainBytes };
    if (plainBytes.byteLength <= 4 * 1024 * 1024) {
      lruSet(fullKey, result);
    }
    return result;
  }

  // ── WebSocket Lifecycle & CWBF2 Handshake ───────────────────────────────────
  function clearTimers() {
    if (reconnectTimer) { clearTimeout(reconnectTimer); reconnectTimer = null; }
    if (pingTimer) { clearInterval(pingTimer); pingTimer = null; }
    if (pongTimeout) { clearTimeout(pongTimeout); pongTimeout = null; }
  }

  function startHeartbeat() {
    if (pingTimer) clearInterval(pingTimer);
    pingTimer = setInterval(() => {
      if (!ws || ws.readyState !== WebSocket.OPEN || !handshakeCompleted || simulatedOffline) return;
      const tsBuf = new ArrayBuffer(8);
      new DataView(tsBuf).setBigUint64(0, BigInt(Date.now()), false);
      try {
        ws.send(buildPlainFrame(OP_PING, 0, 0, tsBuf));
      } catch (_) { return; }
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
      broadcastState();
      try { activeWs.close(); } catch (_) {}
      return;
    }
    const serverPubRaw = payload.slice(0, 65);
    const salt = payload.slice(65, 81);
    try {
      const serverPubKey = await crypto.subtle.importKey('raw', serverPubRaw, { name: 'ECDH', namedCurve: 'P-256' }, false, []);
      const clientKp = await crypto.subtle.generateKey({ name: 'ECDH', namedCurve: 'P-256' }, false, ['deriveBits']);
      const clientPubRaw = new Uint8Array(await crypto.subtle.exportKey('raw', clientKp.publicKey));
      const sharedBits = await crypto.subtle.deriveBits({ name: 'ECDH', public: serverPubKey }, clientKp.privateKey, 256);
      const hkdfBase = await crypto.subtle.importKey('raw', sharedBits, 'HKDF', false, ['deriveKey']);
      const derivedKey = await crypto.subtle.deriveKey(
        { name: 'HKDF', hash: 'SHA-256', salt, info: HKDF_INFO },
        hkdfBase,
        { name: 'AES-GCM', length: 256 },
        false,
        ['encrypt', 'decrypt'],
      );
      if (ws !== activeWs || activeWs.readyState !== WebSocket.OPEN) return;

      sessionKey = derivedKey;
      clientSeq = 1;
      serverHighestSeq = 0;
      serverReplayMask = 0n;
      sendChain = Promise.resolve();
      authFailedCloseCount = 0;
      everAuthenticated = true;
      activeWs.send(buildPlainFrame(OP_HELLO_ACK, 0, 0, clientPubRaw));
      handshakeCompleted = true;
      retryDelayMs = 500;
      stats.connected = true;
      stats.handshakeCompleted = true;
      stats.lastError = null;
      startHeartbeat();
      broadcastState();
      flushPreHandshakeQueue();
    } catch (err) {
      stats.lastError = String(err && err.message ? err.message : err);
      broadcastState();
      try { activeWs.close(); } catch (_) {}
    }
  }

  function scheduleReconnect() {
    if (!running || simulatedOffline) return;
    clearTimers();
    const jitter = Math.floor(Math.random() * 200);
    const wait = Math.min(5000, Math.round(retryDelayMs) + jitter);
    retryDelayMs = Math.min(5000, retryDelayMs * 1.5);
    reconnectTimer = setTimeout(() => {
      if (running && !simulatedOffline) {
        stats.reconnectCount += 1;
        connect();
      }
    }, wait);
  }

  function connect() {
    if (!running || !targetUrl || simulatedOffline) return;
    if (ws && (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)) return;
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
      broadcastState();
      scheduleReconnect();
      return;
    }

    ws = socket;

    socket.onmessage = async (ev) => {
      if (ws !== socket) return;
      if (ev.data instanceof ArrayBuffer) {
        stats.binaryFramesReceived += 1;
        const frame = parseHeader(ev.data);
        if (!frame) return;
        if (frame.op === OP_HELLO) {
          await handleHello(frame.payload, socket);
        } else if (frame.op === OP_PONG) {
          if (pongTimeout) { clearTimeout(pongTimeout); pongTimeout = null; }
          stats.lastPongAt = Date.now();
        } else if (sessionKey && handshakeCompleted) {
          // Enforce monotonic sliding-window anti-replay on server-sent CWBF2 frame.seq
          const fseq = frame.seq;
          if (!Number.isSafeInteger(fseq) || fseq <= 0) {
            stats.lastError = 'invalid_frame_seq';
            return;
          }
          if (fseq > serverHighestSeq) {
            const shift = BigInt(fseq - serverHighestSeq);
            serverReplayMask = shift >= 64n ? 1n : ((serverReplayMask << shift) | 1n) & ((1n << 64n) - 1n);
            serverHighestSeq = fseq;
          } else {
            const delta = BigInt(serverHighestSeq - fseq);
            if (delta >= 64n || ((serverReplayMask >> delta) & 1n) !== 0n) {
              stats.lastError = 'replayed_frame_rejected';
              return;
            }
            serverReplayMask |= 1n << delta;
          }
          try {
            const plain = await decodeEncryptedPayload(frame);
            if (frame.op === OP_EVENT) {
              const rawJson = TEXT_DEC.decode(plain);
              stats.binaryEventsDecrypted += 1;
              broadcastAll({ type: 'ws_event', rawJson, stats: { ...stats } });
            } else if (
              frame.op === OP_RPC_RES ||
              frame.op === OP_MEDIA_CHUNK_RES ||
              frame.op === OP_UPLOAD_ACK ||
              frame.op === OP_DELTA_SYNC_RES
            ) {
              const entry = pendingRpcs.get(frame.reqId);
              if (entry) {
                clearTimeout(entry.timer);
                pendingRpcs.delete(frame.reqId);
                entry.resolve(plain);
              }
            }
          } catch (_) {
            stats.lastError = 'frame_decrypt_failed';
            broadcastState();
          }
        }
      } else if (typeof ev.data === 'string') {
        stats.plaintextFramesReceived += 1;
        // Once CWBF2 encryption is established, never process unexpected plaintext WS events.
        if (handshakeCompleted || sessionKey) {
          stats.lastError = 'unexpected_plaintext_frame_dropped';
          return;
        }
        broadcastAll({ type: 'ws_event', rawJson: ev.data, stats: { ...stats } });
      }
    };

    socket.onclose = () => {
      if (ws !== socket) return;
      const wasHandshaked = handshakeCompleted;
      ws = null;
      sessionKey = null;
      serverHighestSeq = 0;
      serverReplayMask = 0n;
      handshakeCompleted = false;
      stats.connected = false;
      stats.handshakeCompleted = false;
      if (!wasHandshaked) authFailedCloseCount += 1;
      rejectAllPending('ws_disconnected');
      broadcastState();
      if (everAuthenticated || authFailedCloseCount < 4) {
        scheduleReconnect();
      }
    };

    socket.onerror = () => {
      if (ws !== socket) return;
      try { socket.close(); } catch (_) {}
    };
  }

  // ── Direct Service Worker MessagePort Handler (Zero-Copy Transferable Chunks) ──
  function attachSwPort(swPort) {
    swPorts.add(swPort);
    swPort.onmessage = async (ev) => {
      const m = ev.data;
      if (!m || typeof m !== 'object') return;
      if (m.type === 'SW_MEDIA_CHUNK_REQ') {
        const { id, url, offset = 0, limit = 512 * 1024, priority = 'CRITICAL', prefetchNext = false, totalKnown = 0 } = m;
        try {
          const res = await fetchMediaRangeViaWs(url, offset, limit, priority, `sw:${id}`, 'sw-stream');
          // Clone slice so RAM LRU cache keeps its own copy while Transferable moves zero-copy to SW
          const copy = new Uint8Array(res.bytes);
          swPort.postMessage(
            {
              type: 'SW_MEDIA_CHUNK_RES',
              id,
              ok: true,
              status: res.status,
              totalSize: res.totalSize,
              contentType: res.contentType,
              buffer: copy.buffer,
            },
            [copy.buffer],
          );
          // Smart background prefetch of next 1-2 chunks at LOW priority for smooth video playback!
          const total = res.totalSize || totalKnown;
          if (prefetchNext && total > 0 && limit > 0) {
            const nextOff1 = offset + limit;
            if (nextOff1 < total) {
              fetchMediaRangeViaWs(url, nextOff1, Math.min(limit, total - nextOff1), 'LOW', '', 'prefetch').catch(() => {});
            }
          }
        } catch (err) {
          swPort.postMessage({
            type: 'SW_MEDIA_CHUNK_RES',
            id,
            ok: false,
            error: String(err && err.message ? err.message : err),
          });
        }
      }
    };
    if (typeof swPort.start === 'function') swPort.start();
  }

  // ── Tab MessagePort Protocol Handler ────────────────────────────────────────
  function attachTabPort(port) {
    const tabId = nextTabId++;
    tabs.set(tabId, port);
    stats.connectedTabs = tabs.size;

    port.onmessage = async (ev) => {
      const msg = ev.data;
      if (!msg || typeof msg !== 'object') return;

      if (msg.type === 'attach_sw_port' && ev.ports && ev.ports[0]) {
        attachSwPort(ev.ports[0]);
        postToTab(port, { type: 'sw_port_attached', ok: true });
        return;
      }

      if (msg.type === 'start') {
        const rawUrl = String(msg.wsUrl || '');
        if (!rawUrl) return;
        const sep = rawUrl.includes('?') ? '&' : '?';
        const nextUrl = rawUrl.includes('proto=rpc-bin') ? rawUrl : `${rawUrl}${sep}proto=rpc-bin`;
        running = true;
        authFailedCloseCount = 0;
        if (targetUrl !== nextUrl || !ws || ws.readyState === WebSocket.CLOSED) {
          targetUrl = nextUrl;
          retryDelayMs = 600;
          connect();
        }
        postToTab(port, { type: 'ws_state', stats: { ...stats } });
        return;
      }

      if (msg.type === 'reconnect_now') {
        if (simulatedOffline) return;
        if (msg.wsUrl) {
          const rawUrl = String(msg.wsUrl);
          const sep = rawUrl.includes('?') ? '&' : '?';
          targetUrl = rawUrl.includes('proto=rpc-bin') ? rawUrl : `${rawUrl}${sep}proto=rpc-bin`;
        }
        if (!targetUrl) return;
        running = true;
        authFailedCloseCount = 0;
        retryDelayMs = 500;
        if (ws && ws.readyState === WebSocket.OPEN && handshakeCompleted) {
          postToTab(port, { type: 'ws_state', stats: { ...stats } });
          return;
        }
        if (ws) { try { ws.close(); } catch (_) {} ws = null; }
        connect();
        return;
      }

      if (msg.type === 'force_drop_for_test') {
        if (ws) {
          try { ws.close(); } catch (_) {}
        }
        postToTab(port, { type: 'rpc_reply', callId: msg.callId, ok: true });
        return;
      }

      if (msg.type === 'set_simulated_offline') {
        simulatedOffline = Boolean(msg.offline);
        if (simulatedOffline) {
          clearTimers();
          if (ws) { try { ws.close(); } catch (_) {} ws = null; }
          sessionKey = null;
          handshakeCompleted = false;
          stats.connected = false;
          stats.handshakeCompleted = false;
          rejectAllPending('offline');
          broadcastState();
        } else if (running && targetUrl) {
          authFailedCloseCount = 0;
          retryDelayMs = 600;
          connect();
        }
        postToTab(port, { type: 'rpc_reply', callId: msg.callId, ok: true });
        return;
      }

      if (msg.type === 'get_stats') {
        postToTab(port, { type: 'ws_state', stats: { ...stats } });
        return;
      }

      if (msg.type === 'rpc_call') {
        const { callId, method, path, contentType, headers, body } = msg;
        try {
          const bodyU8 = body instanceof ArrayBuffer
            ? new Uint8Array(body)
            : (typeof body === 'string' ? TEXT_ENC.encode(body) : new Uint8Array(0));
          const reqPayload = encodeRpcCall(method, path, contentType || '', headers || {}, bodyU8);
          const timeoutMs = Number(msg.timeoutMs) > 0 ? Number(msg.timeoutMs) : 18000;
          const rawRes = await sendRpcRaw(OP_RPC_CALL, reqPayload, true, timeoutMs);
          const parsed = decodeRpcRes(rawRes);
          stats.rpcCallsCompleted += 1;
          const outCopy = new Uint8Array(parsed.body);
          postToTab(
            port,
            {
              type: 'rpc_reply',
              callId,
              ok: true,
              status: parsed.status,
              contentType: parsed.contentType,
              bodyBuffer: outCopy.buffer,
              stats: { ...stats },
            },
            [outCopy.buffer],
          );
        } catch (err) {
          postToTab(port, {
            type: 'rpc_reply',
            callId,
            ok: false,
            error: String(err && err.message ? err.message : err),
          });
        }
        return;
      }

      if (msg.type === 'download_media') {
        const { callId, url, keyB64, nonceB64, mime, priority, taskId, scope } = msg;
        try {
          const res = await downloadFullMediaInWorker({ url, keyB64, nonceB64, mime, priority, taskId, scope });
          const copy = new Uint8Array(res.bytes);
          postToTab(
            port,
            {
              type: 'rpc_reply',
              callId,
              ok: true,
              status: res.status,
              contentType: res.contentType,
              bodyBuffer: copy.buffer,
              stats: { ...stats },
            },
            [copy.buffer],
          );
        } catch (err) {
          postToTab(port, {
            type: 'rpc_reply',
            callId,
            ok: false,
            error: String(err && err.message ? err.message : err),
          });
        }
        return;
      }

      if (msg.type === 'abort_download') {
        abortDownloads({ taskId: msg.taskId, scope: msg.scope });
        return;
      }

      if (msg.type === 'upload_part') {
        const { callId, uploadId, partIndex, chunkBuffer } = msg;
        try {
          const idBytes = TEXT_ENC.encode(String(uploadId || ''));
          const chunk = new Uint8Array(chunkBuffer || 0);
          const payload = new Uint8Array(2 + idBytes.byteLength + 4 + chunk.byteLength);
          const dv = new DataView(payload.buffer);
          dv.setUint16(0, idBytes.byteLength, false);
          payload.set(idBytes, 2);
          dv.setUint32(2 + idBytes.byteLength, partIndex >>> 0, false);
          payload.set(chunk, 6 + idBytes.byteLength);

          const rawAck = await sendRpcRaw(OP_UPLOAD_PART, payload, false, 25000);
          const ackDv = new DataView(rawAck.buffer, rawAck.byteOffset, rawAck.byteLength);
          const status = ackDv.getUint16(0, false);
          const ackPart = ackDv.getUint32(2, false);
          const ackText = TEXT_DEC.decode(rawAck.subarray(6));
          postToTab(port, { type: 'rpc_reply', callId, ok: status >= 200 && status < 300, status, partIndex: ackPart, text: ackText });
        } catch (err) {
          postToTab(port, { type: 'rpc_reply', callId, ok: false, error: String(err && err.message ? err.message : err) });
        }
        return;
      }

      if (msg.type === 'delta_sync') {
        const { callId, dmPts = 0, groupPts = 0, channelPts = 0 } = msg;
        try {
          const buf = new Uint8Array(24);
          const dv = new DataView(buf.buffer);
          dv.setBigInt64(0, BigInt(dmPts), false);
          dv.setBigInt64(8, BigInt(groupPts), false);
          dv.setBigInt64(16, BigInt(channelPts), false);
          const raw = await sendRpcRaw(OP_DELTA_SYNC_REQ, buf, true, 15000);
          const status = new DataView(raw.buffer, raw.byteOffset, 2).getUint16(0, false);
          const text = TEXT_DEC.decode(raw.subarray(2));
          postToTab(port, { type: 'rpc_reply', callId, ok: status >= 200 && status < 300, status, text });
        } catch (err) {
          postToTab(port, { type: 'rpc_reply', callId, ok: false, error: String(err && err.message ? err.message : err) });
        }
        return;
      }

      if (msg.type === 'tab_sync') {
        // Multi-tab instant state sync (<5ms) to all OTHER tabs connected to this SharedWorker
        for (const [otherId, otherPort] of tabs.entries()) {
          if (otherId !== tabId) {
            postToTab(otherPort, { type: 'tab_sync', topic: msg.topic, payload: msg.payload });
          }
        }
        return;
      }

      if (msg.type === 'logout_wipe') {
        running = false;
        everAuthenticated = false;
        sendChain = Promise.resolve();
        clearTimers();
        if (ws) { try { ws.close(); } catch (_) {} ws = null; }
        sessionKey = null;
        handshakeCompleted = false;
        stats.connected = false;
        stats.handshakeCompleted = false;
        mediaCache.clear();
        rejectAllPending('logged_out');
        for (const [otherId, otherPort] of tabs.entries()) {
          if (otherId !== tabId) {
            postToTab(otherPort, { type: 'tab_sync', topic: 'logout', payload: {} });
          }
        }
        broadcastState();
        return;
      }

      if (msg.type === 'tab_close') {
        tabs.delete(tabId);
        stats.connectedTabs = tabs.size;
        return;
      }
    };

    if (typeof port.start === 'function') port.start();
    postToTab(port, { type: 'ws_state', stats: { ...stats } });
  }

  if (IS_SHARED) {
    self.onconnect = (ev) => {
      const port = ev.ports && ev.ports[0];
      if (port) attachTabPort(port);
    };
  } else {
    // Dedicated Worker fallback mode: wrap `self` with a MessagePort-compatible shim
    attachTabPort({
      postMessage: (m, t) => self.postMessage(m, t || []),
      set onmessage(fn) { self.onmessage = fn; },
      start() {},
    });
  }
})();
