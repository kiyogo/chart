// ChatW Service Worker Media Proxy & Offline App Shell Cache (Spec 071 — Telegram Web K)
// 1. Intercepts `/stream/{grantId}` (and `/stream/media/{grantId}`), enforces strict 5-layer `403 Forbidden`.
// 2. Pulls media byte ranges directly from `SharedWorker` (`core_worker.js`) via direct Transferable `MessagePort`
//    (ZERO server HTTP `fetch()` calls!) and streams `ReadableStream` HTTP `206 Partial Content` with O(1)
//    E2EE `CWCH2` chunked AES-256-GCM seek + smart 1-2 chunk prefetch.
// 3. Offline App Shell & Static Asset Cache (`Cache-First` + `Stale-While-Revalidate`) for `/locales/*`,
//    `/stickers/*`, SVG/CSS/JS/WASM assets so repeat boots make 0 redundant Fetch/XHR requests.

const STREAM_PREFIXES = ['/stream/media/', '/stream/'];
const STATIC_CACHE_NAME = 'chatw-static-v5';
const DEFAULT_GRANT_TTL_MS = 15 * 60 * 1000;
const MAX_CHUNK_CACHE_ENTRIES = 64;
const MAX_STREAM_SPAN = 1024 * 1024;
const CWCH2_MAGIC = [0x43, 0x57, 0x43, 0x48, 0x32]; // "CWCH2"
const CWCH2_HEADER_LEN = 21;
const ALLOWED_DESTINATIONS = new Set(['video', 'audio', 'image', 'empty', '']);

/** @type {Map<string, any>} */
const grants = new Map();
/** @type {Map<string, Uint8Array>} */
const chunkLru = new Map();

// Direct Transferable MessagePort connected straight to SharedWorker (`core_worker.js`)
/** @type {MessagePort|null} */
let workerPort = null;
let nextSwReqId = 1;
const pendingWorkerReqs = new Map();

self.addEventListener('install', () => {
  self.skipWaiting();
});

self.addEventListener('activate', (event) => {
  event.waitUntil((async () => {
    const keys = await caches.keys().catch(() => []);
    await Promise.all(
      keys
        .filter((k) => k.startsWith('chatw-static-') && k !== STATIC_CACHE_NAME)
        .map((k) => caches.delete(k).catch(() => {})),
    );
    await self.clients.claim();
  })());
});

function attachWorkerPort(port) {
  workerPort = port;
  workerPort.onmessage = (ev) => {
    const d = ev.data;
    if (!d || typeof d !== 'object') return;
    if (d.type === 'SW_MEDIA_CHUNK_RES' && d.id) {
      const entry = pendingWorkerReqs.get(d.id);
      if (entry) {
        clearTimeout(entry.timer);
        pendingWorkerReqs.delete(d.id);
        entry.resolve(d);
      }
    }
  };
  if (typeof workerPort.start === 'function') workerPort.start();
}

function requestChunkFromSharedWorker(url, offset, limit, prefetchNext = false, totalKnown = 0) {
  if (!workerPort) {
    return Promise.resolve({ ok: false, error: 'no_worker_port' });
  }
  const id = nextSwReqId++;
  return new Promise((resolve) => {
    const timer = setTimeout(() => {
      pendingWorkerReqs.delete(id);
      resolve({ ok: false, error: 'worker_port_timeout' });
    }, 18000);
    pendingWorkerReqs.set(id, { resolve, timer });
    try {
      workerPort.postMessage({
        type: 'SW_MEDIA_CHUNK_REQ',
        id,
        url,
        offset,
        limit,
        priority: 'CRITICAL',
        prefetchNext,
        totalKnown,
      });
    } catch (err) {
      clearTimeout(timer);
      pendingWorkerReqs.delete(id);
      resolve({ ok: false, error: String(err && err.message ? err.message : err) });
    }
  });
}

// Unified chunk source: ALWAYS prefers SharedWorker direct MessagePort (0 server HTTP fetch!);
// falls back to authenticated fetch ONLY if workerPort hasn't bridged yet.
async function fetchUpstreamRange(grant, start, end, prefetchNext = false, totalKnown = 0) {
  const limit = end >= start ? (end - start + 1) : 0;
  if (workerPort) {
    const wRes = await requestChunkFromSharedWorker(grant.url, start, limit, prefetchNext, totalKnown);
    if (wRes && wRes.ok && (wRes.status === 200 || wRes.status === 206)) {
      return {
        ok: true,
        status: wRes.status,
        totalSize: wRes.totalSize || 0,
        contentType: wRes.contentType || grant.mime || 'application/octet-stream',
        bytes: new Uint8Array(wRes.buffer || 0),
        viaWorker: true,
      };
    }
  }
  const hdrs = new Headers();
  if (limit > 0) hdrs.set('Range', `bytes=${start}-${end}`);
  if (grant.streamToken) hdrs.set('X-ChatW-Stream-Token', grant.streamToken);
  const res = await fetch(grant.url, { credentials: 'include', headers: hdrs });
  const buf = new Uint8Array(await res.arrayBuffer());
  const cr = res.headers.get('content-range') || '';
  const m = cr.match(/\/(\d+)$/);
  const totalSize = m ? Number(m[1]) : (Number(res.headers.get('content-length')) || buf.byteLength);
  return {
    ok: res.ok || res.status === 206,
    status: res.status,
    totalSize,
    contentType: res.headers.get('content-type') || grant.mime || 'application/octet-stream',
    bytes: buf,
    viaWorker: false,
  };
}

function b64ToU8(b64) {
  const norm = String(b64 || '').replace(/-/g, '+').replace(/_/g, '/');
  const pad = norm.length % 4 === 0 ? '' : '='.repeat(4 - (norm.length % 4));
  const bin = atob(norm + pad);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

function forbiddenResponse(reason) {
  return new Response(`403 Forbidden: ${reason}`, {
    status: 403,
    statusText: 'Forbidden',
    headers: {
      'Content-Type': 'text/plain; charset=utf-8',
      'Cache-Control': 'no-store',
      'X-ChatW-Deny-Reason': reason,
    },
  });
}

function lruGet(key) {
  const val = chunkLru.get(key);
  if (!val) return null;
  chunkLru.delete(key);
  chunkLru.set(key, val);
  return val;
}

function lruSet(key, val) {
  if (chunkLru.has(key)) chunkLru.delete(key);
  chunkLru.set(key, val);
  while (chunkLru.size > MAX_CHUNK_CACHE_ENTRIES) {
    const oldest = chunkLru.keys().next().value;
    chunkLru.delete(oldest);
  }
}

function purgeGrantCache(grantId) {
  const prefix = `${grantId}:`;
  for (const k of Array.from(chunkLru.keys())) {
    if (k.startsWith(prefix)) chunkLru.delete(k);
  }
}

function parseByteRange(rangeHeader, totalSize) {
  if (!rangeHeader || !rangeHeader.startsWith('bytes=')) return null;
  const spec = rangeHeader.slice(6).split(',')[0].trim();
  const dash = spec.indexOf('-');
  if (dash < 0) return null;
  const startStr = spec.slice(0, dash).trim();
  const endStr = spec.slice(dash + 1).trim();
  if (startStr === '') {
    const suffix = parseInt(endStr, 10);
    if (!Number.isFinite(suffix) || suffix <= 0) return null;
    const start = Math.max(0, totalSize - suffix);
    return { start, end: Math.max(0, totalSize - 1) };
  }
  const start = parseInt(startStr, 10);
  if (!Number.isFinite(start) || start < 0 || start >= totalSize) return null;
  const end = endStr === ''
    ? Math.min(totalSize - 1, start + MAX_STREAM_SPAN - 1)
    : Math.min(totalSize - 1, parseInt(endStr, 10));
  if (!Number.isFinite(end) || end < start) return null;
  return { start, end };
}

function bytesToReadableStream(u8) {
  const STEP = 64 * 1024;
  let off = 0;
  return new ReadableStream({
    pull(controller) {
      if (off >= u8.byteLength) {
        controller.close();
        return;
      }
      const end = Math.min(u8.byteLength, off + STEP);
      controller.enqueue(u8.subarray(off, end));
      off = end;
    },
  });
}

async function ensureAesKey(grant) {
  if (grant.aesKey && grant.baseNonce) return;
  const keyBytes = b64ToU8(grant.keyB64);
  const nonceBytes = b64ToU8(grant.nonceB64);
  if (keyBytes.byteLength !== 32 || nonceBytes.byteLength !== 12) {
    throw new Error('invalid_e2ee_media_key');
  }
  grant.aesKey = await crypto.subtle.importKey('raw', keyBytes, { name: 'AES-GCM' }, false, ['decrypt']);
  grant.baseNonce = nonceBytes;
}

function isCwch2Header(u8) {
  if (u8.byteLength < CWCH2_HEADER_LEN) return false;
  for (let i = 0; i < 5; i++) {
    if (u8[i] !== CWCH2_MAGIC[i]) return false;
  }
  return true;
}

async function ensureCwch2Meta(grant) {
  if (grant.probeDone) return grant.cwch2Meta;
  const res = await fetchUpstreamRange(grant, 0, CWCH2_HEADER_LEN - 1, false, 0);
  if (!res.ok) throw new Error(`upstream_http_${res.status}`);
  grant.probeDone = true;
  grant.viaWorker = res.viaWorker;
  if (isCwch2Header(res.bytes)) {
    const dv = new DataView(res.bytes.buffer, res.bytes.byteOffset, res.bytes.byteLength);
    const chunkSize = dv.getUint32(5, false);
    const chunkCount = dv.getUint32(9, false);
    const totalPlainLen = Number(dv.getBigUint64(13, false));
    if (chunkSize >= 1024 && chunkCount >= 1 && totalPlainLen > 0) {
      grant.cwch2Meta = { chunkSize, chunkCount, totalPlainLen };
      return grant.cwch2Meta;
    }
  }
  grant.cwch2Meta = null;
  return null;
}

async function fetchAndDecryptCwch2Chunk(grant, grantId, meta, chunkIdx) {
  const cacheKey = `${grantId}:${chunkIdx}`;
  const hit = lruGet(cacheKey);
  if (hit) return hit;

  const { chunkSize, chunkCount, totalPlainLen } = meta;
  const encSlotSize = 4 + chunkSize + 16;
  const encStart = CWCH2_HEADER_LEN + chunkIdx * encSlotSize;
  const plainLen = chunkIdx === chunkCount - 1 ? totalPlainLen - chunkIdx * chunkSize : chunkSize;
  const expectedCtLen = plainLen + 16;
  const encEnd = encStart + 4 + expectedCtLen - 1;

  const res = await fetchUpstreamRange(grant, encStart, encEnd, false, 0);
  if (!res.ok) throw new Error(`chunk_fetch_http_${res.status}`);
  grant.viaWorker = res.viaWorker;
  const seg = res.bytes;
  if (seg.byteLength < 4 + expectedCtLen) throw new Error('truncated_cwch2_chunk');
  const ctLen = new DataView(seg.buffer, seg.byteOffset, 4).getUint32(0, false);
  if (ctLen !== expectedCtLen) throw new Error('cwch2_chunk_length_mismatch');

  const chunkNonce = new Uint8Array(grant.baseNonce);
  chunkNonce[8] ^= (chunkIdx >>> 24) & 0xff;
  chunkNonce[9] ^= (chunkIdx >>> 16) & 0xff;
  chunkNonce[10] ^= (chunkIdx >>> 8) & 0xff;
  chunkNonce[11] ^= chunkIdx & 0xff;

  const aad = new Uint8Array(10);
  aad.set(CWCH2_MAGIC, 0);
  new DataView(aad.buffer).setUint32(5, chunkIdx, false);
  aad[9] = chunkIdx === chunkCount - 1 ? 1 : 0;

  const plainBuf = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: chunkNonce, additionalData: aad },
    grant.aesKey,
    seg.subarray(4, 4 + ctLen),
  );
  const plainU8 = new Uint8Array(plainBuf);
  lruSet(cacheKey, plainU8);

  // Smart background prefetch of the next chunk (+1) for stutter-free video playback
  if (chunkIdx + 1 < chunkCount && !chunkLru.has(`${grantId}:${chunkIdx + 1}`)) {
    fetchAndDecryptCwch2Chunk(grant, grantId, meta, chunkIdx + 1).catch(() => {});
  }

  return plainU8;
}

async function handleCwch2Stream(request, grant, grantId, meta) {
  const total = meta.totalPlainLen;
  const rangeHdr = request.headers.get('Range');
  let start = 0;
  let end = Math.min(total - 1, MAX_STREAM_SPAN - 1);
  const isPartial = Boolean(rangeHdr);

  if (rangeHdr) {
    const parsed = parseByteRange(rangeHdr, total);
    if (!parsed) {
      return new Response(null, { status: 416, headers: { 'Content-Range': `bytes */${total}` } });
    }
    start = parsed.start;
    end = Math.min(parsed.end, start + MAX_STREAM_SPAN - 1);
  } else {
    end = total - 1;
  }

  const firstChunk = Math.floor(start / meta.chunkSize);
  const lastChunk = Math.floor(end / meta.chunkSize);
  const out = new Uint8Array(end - start + 1);
  let writeOff = 0;

  for (let idx = firstChunk; idx <= lastChunk; idx++) {
    const plainChunk = await fetchAndDecryptCwch2Chunk(grant, grantId, meta, idx);
    const chunkBase = idx * meta.chunkSize;
    const sliceStart = Math.max(0, start - chunkBase);
    const sliceEnd = Math.min(plainChunk.byteLength, end - chunkBase + 1);
    const view = plainChunk.subarray(sliceStart, sliceEnd);
    out.set(view, writeOff);
    writeOff += view.byteLength;
  }

  const headers = {
    'Content-Type': grant.mime || 'application/octet-stream',
    'Accept-Ranges': 'bytes',
    'Content-Length': String(out.byteLength),
    'Cache-Control': 'no-store',
    'X-ChatW-Stream-Mode': 'sw-cwch2-206',
    'X-ChatW-Stream-Transport': grant.viaWorker ? 'shared-worker-bin' : 'fallback',
  };
  if (isPartial) headers['Content-Range'] = `bytes ${start}-${end}/${total}`;
  return new Response(bytesToReadableStream(out), {
    status: isPartial ? 206 : 200,
    headers,
  });
}

async function handleLegacySingleE2eeStream(request, grant) {
  if (!grant.singlePlainBuf) {
    const res = await fetchUpstreamRange(grant, 0, -1, false, 0);
    if (!res.ok) throw new Error(`legacy_e2ee_http_${res.status}`);
    const pt = await crypto.subtle.decrypt({ name: 'AES-GCM', iv: grant.baseNonce }, grant.aesKey, res.bytes);
    grant.singlePlainBuf = new Uint8Array(pt);
    grant.viaWorker = res.viaWorker;
  }
  const plain = grant.singlePlainBuf;
  const total = plain.byteLength;
  const rangeHdr = request.headers.get('Range');
  if (rangeHdr) {
    const parsed = parseByteRange(rangeHdr, total);
    if (!parsed) return new Response(null, { status: 416, headers: { 'Content-Range': `bytes */${total}` } });
    const slice = plain.subarray(parsed.start, parsed.end + 1);
    return new Response(bytesToReadableStream(slice), {
      status: 206,
      headers: {
        'Content-Type': grant.mime || 'application/octet-stream',
        'Accept-Ranges': 'bytes',
        'Content-Range': `bytes ${parsed.start}-${parsed.end}/${total}`,
        'Content-Length': String(slice.byteLength),
        'Cache-Control': 'no-store',
        'X-ChatW-Stream-Mode': 'sw-e2ee-single-206',
        'X-ChatW-Stream-Transport': grant.viaWorker ? 'shared-worker-bin' : 'fallback',
      },
    });
  }
  return new Response(bytesToReadableStream(plain), {
    status: 200,
    headers: {
      'Content-Type': grant.mime || 'application/octet-stream',
      'Accept-Ranges': 'bytes',
      'Content-Length': String(total),
      'Cache-Control': 'no-store',
      'X-ChatW-Stream-Mode': 'sw-e2ee-single-200',
    },
  });
}

async function handlePlainRangeProxy(request, grant) {
  const rangeHdr = request.headers.get('Range');
  let start = 0;
  let end = 512 * 1024 - 1;
  const hasRange = Boolean(rangeHdr);
  if (rangeHdr && rangeHdr.startsWith('bytes=')) {
    const spec = rangeHdr.slice(6).split(',')[0].trim();
    const [s, e] = spec.split('-');
    start = Math.max(0, parseInt(s, 10) || 0);
    end = e && e.trim() !== '' ? Math.max(start, parseInt(e, 10)) : start + 512 * 1024 - 1;
  } else {
    end = -1;
  }
  const res = await fetchUpstreamRange(grant, start, end, hasRange, grant.knownTotal || 0);
  if (res.totalSize > 0) grant.knownTotal = res.totalSize;
  const total = res.totalSize || res.bytes.byteLength;
  const actualEnd = start + Math.max(0, res.bytes.byteLength - 1);
  const hdrs = {
    'Content-Type': res.contentType || grant.mime || 'application/octet-stream',
    'Accept-Ranges': 'bytes',
    'Content-Length': String(res.bytes.byteLength),
    'Cache-Control': 'no-store',
    'X-ChatW-Stream-Mode': 'sw-range-proxy',
    'X-ChatW-Stream-Transport': res.viaWorker ? 'shared-worker-bin' : 'fallback',
  };
  if (hasRange && res.status === 206) {
    hdrs['Content-Range'] = `bytes ${start}-${actualEnd}/${total}`;
  }
  return new Response(bytesToReadableStream(res.bytes), {
    status: hasRange && res.status === 206 ? 206 : res.status,
    headers: hdrs,
  });
}

function extractStreamGrantId(pathname) {
  for (const prefix of STREAM_PREFIXES) {
    if (pathname.startsWith(prefix)) {
      return pathname.slice(prefix.length).split('/')[0];
    }
  }
  return null;
}

function isCacheableStaticAsset(pathname) {
  return (
    pathname.startsWith('/locales/') ||
    pathname.startsWith('/stickers/') ||
    pathname.startsWith('/wallpapers/') ||
    pathname.endsWith('.svg') ||
    pathname.endsWith('.woff2')
  );
}

self.addEventListener('message', (event) => {
  const msg = event.data;
  const replyPort = event.ports && event.ports[0];
  const reply = (payload) => {
    if (replyPort) {
      try { replyPort.postMessage(payload); } catch (_) {}
    }
  };
  if (!msg || typeof msg !== 'object') return;

  if (msg.type === 'ATTACH_WORKER_PORT' && event.ports && event.ports[0]) {
    attachWorkerPort(event.ports[0]);
    return;
  }

  if (msg.type === 'PING_SW') {
    reply({
      ok: true,
      version: 'chatw-sw-v2',
      activeGrants: grants.size,
      workerPortReady: Boolean(workerPort),
    });
    return;
  }

  if (msg.type === 'REGISTER_MEDIA_GRANT') {
    const ownerClientId = event.source && event.source.id ? String(event.source.id) : '';
    const grantId = String(msg.grantId || '');
    const url = String(msg.url || '');
    if (!ownerClientId || !grantId || !url || (!url.startsWith('/api/attachments/') && !url.startsWith('/uploads/media/'))) {
      reply({ ok: false, error: 'invalid_grant_params' });
      return;
    }
    const ttlMs = Math.min(30 * 60 * 1000, Math.max(10000, Number(msg.ttlMs) || DEFAULT_GRANT_TTL_MS));
    grants.set(grantId, {
      ownerClientId,
      url,
      keyB64: msg.keyB64 || '',
      nonceB64: msg.nonceB64 || '',
      mime: msg.mime || 'application/octet-stream',
      fileName: msg.fileName || 'media',
      streamToken: msg.streamToken || '',
      expiresAt: Date.now() + ttlMs,
      probeDone: false,
      cwch2Meta: null,
      aesKey: null,
      baseNonce: null,
      singlePlainBuf: null,
      viaWorker: false,
      knownTotal: 0,
    });
    const now = Date.now();
    for (const [gid, g] of grants.entries()) {
      if (g.expiresAt < now) {
        grants.delete(gid);
        purgeGrantCache(gid);
      }
    }
    reply({
      ok: true,
      grantId,
      ownerClientId,
      streamUrl: `/stream/${grantId}`,
      workerPortReady: Boolean(workerPort),
    });
    return;
  }

  if (msg.type === 'REVOKE_MEDIA_GRANT') {
    const grantId = String(msg.grantId || '');
    if (grantId) {
      grants.delete(grantId);
      purgeGrantCache(grantId);
    }
    reply({ ok: true });
    return;
  }

  if (msg.type === 'REVOKE_ALL_GRANTS') {
    grants.clear();
    chunkLru.clear();
    reply({ ok: true });
  }
});

self.addEventListener('fetch', (event) => {
  const reqUrl = new URL(event.request.url);
  if (reqUrl.origin !== self.location.origin) return;
  // Never intercept runtime loader binary so SW never emits a ⚙ fetch entry (Spec 077)
  if (reqUrl.pathname === '/pkg/core.bin' || reqUrl.pathname.endsWith('.wasm')) return;

  // 1. Protected Media Streaming Proxy (`/stream/{grantId}` & `/stream/media/{grantId}`)
  const grantId = extractStreamGrantId(reqUrl.pathname);
  if (grantId !== null) {
    event.respondWith((async () => {
      // Layer 1: Block direct tab navigation / document / iframe / object opens
      if (
        event.request.mode === 'navigate' ||
        event.request.destination === 'document' ||
        event.request.destination === 'iframe' ||
        event.request.destination === 'object'
      ) {
        return forbiddenResponse('direct_navigation_denied');
      }

      // Layer 2: Restrict request destination strictly to media/subresource
      if (!ALLOWED_DESTINATIONS.has(event.request.destination || '')) {
        return forbiddenResponse('disallowed_destination');
      }

      // Layer 3: Validate active unexpired 128-bit grant
      const grant = grants.get(grantId);
      if (!grant || Date.now() > grant.expiresAt) {
        if (grant) {
          grants.delete(grantId);
          purgeGrantCache(grantId);
        }
        return forbiddenResponse('grant_missing_or_expired');
      }

      // Layer 4: Strict per-tab clientId binding + live same-origin verification
      const reqClientId = event.clientId || '';
      if (!reqClientId || reqClientId !== grant.ownerClientId) {
        return forbiddenResponse('foreign_client_tab_denied');
      }
      const client = await self.clients.get(reqClientId);
      if (!client || new URL(client.url).origin !== self.location.origin) {
        return forbiddenResponse('unverified_client_origin');
      }

      try {
        if (!grant.keyB64) {
          return await handlePlainRangeProxy(event.request, grant);
        }
        await ensureAesKey(grant);
        const cwch2Meta = await ensureCwch2Meta(grant);
        if (cwch2Meta) {
          return await handleCwch2Stream(event.request, grant, grantId, cwch2Meta);
        }
        return await handleLegacySingleE2eeStream(event.request, grant);
      } catch (err) {
        return new Response(`502 Media stream error: ${err && err.message ? err.message : err}`, {
          status: 502,
          headers: { 'Content-Type': 'text/plain; charset=utf-8', 'Cache-Control': 'no-store' },
        });
      }
    })());
    return;
  }

  // 2. App Shell & Static Assets Offline Cache (Pure Cache-First — 0 duplicate background re-fetches!)
  if (event.request.method === 'GET' && isCacheableStaticAsset(reqUrl.pathname)) {
    event.respondWith((async () => {
      try {
        const cache = await caches.open(STATIC_CACHE_NAME);
        const cached = await cache.match(event.request);
        if (cached) {
          return cached;
        }
        const net = await fetch(event.request);
        if (net && net.ok) {
          cache.put(event.request, net.clone());
        }
        return net;
      } catch (_) {
        return fetch(event.request);
      }
    })());
  }
});
