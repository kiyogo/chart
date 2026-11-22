// ChatW Protected Media Streaming Service Worker (Spec 069)
// Intercepts `/stream/media/{grantId}`, enforces strict 4-layer `403 Forbidden`
// anti-direct-access & per-tab binding, and serves HTTP `206 Partial Content`
// Range streaming for both non-E2EE media and O(1) random-access E2EE `CWCH2` chunks.

const STREAM_PREFIX = '/stream/media/';
const DEFAULT_GRANT_TTL_MS = 15 * 60 * 1000; // 15 minutes
const MAX_CHUNK_CACHE_ENTRIES = 48; // ~24 MiB bounded RAM LRU cache (512 KiB chunks)
const MAX_CWCH2_RANGE_SPAN = 1024 * 1024; // Serve up to 1 MiB per video Range slice for snappy seek
const CWCH2_MAGIC = [0x43, 0x57, 0x43, 0x48, 0x32]; // "CWCH2"
const CWCH2_HEADER_LEN = 21;
const ALLOWED_DESTINATIONS = new Set(['video', 'audio', 'image', 'empty', '']);

/** @type {Map<string, any>} */
const grants = new Map();
/** @type {Map<string, Uint8Array>} */
const chunkLru = new Map();

self.addEventListener('install', () => {
  self.skipWaiting();
});

self.addEventListener('activate', (event) => {
  event.waitUntil(self.clients.claim());
});

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
  let end = endStr === ''
    ? Math.min(totalSize - 1, start + MAX_CWCH2_RANGE_SPAN - 1)
    : Math.min(totalSize - 1, parseInt(endStr, 10));
  if (!Number.isFinite(end) || end < start) return null;
  return { start, end };
}

function buildUpstreamHeaders(grant, rangeStr) {
  const headers = new Headers();
  if (rangeStr) headers.set('Range', rangeStr);
  if (grant.streamToken) headers.set('X-ChatW-Stream-Token', grant.streamToken);
  return headers;
}

async function ensureAesKey(grant) {
  if (grant.aesKey && grant.baseNonce) return;
  const keyBytes = b64ToU8(grant.keyB64);
  const nonceBytes = b64ToU8(grant.nonceB64);
  if (keyBytes.byteLength !== 32 || nonceBytes.byteLength !== 12) {
    throw new Error('invalid_e2ee_media_key');
  }
  grant.aesKey = await crypto.subtle.importKey(
    'raw',
    keyBytes,
    { name: 'AES-GCM' },
    false,
    ['decrypt'],
  );
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
  const res = await fetch(grant.url, {
    credentials: 'include',
    headers: buildUpstreamHeaders(grant, `bytes=0-${CWCH2_HEADER_LEN - 1}`),
  });
  if (!res.ok && res.status !== 206) {
    throw new Error(`upstream_http_${res.status}`);
  }
  const buf = new Uint8Array(await res.arrayBuffer());
  grant.probeDone = true;
  if (isCwch2Header(buf)) {
    const dv = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
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
  const plainLen = chunkIdx === chunkCount - 1
    ? totalPlainLen - chunkIdx * chunkSize
    : chunkSize;
  const expectedCtLen = plainLen + 16;
  const encEnd = encStart + 4 + expectedCtLen - 1;

  const res = await fetch(grant.url, {
    credentials: 'include',
    headers: buildUpstreamHeaders(grant, `bytes=${encStart}-${encEnd}`),
  });
  if (!res.ok && res.status !== 206) {
    throw new Error(`chunk_fetch_http_${res.status}`);
  }
  const seg = new Uint8Array(await res.arrayBuffer());
  if (seg.byteLength < 4 + expectedCtLen) {
    throw new Error('truncated_cwch2_chunk');
  }
  const ctLen = new DataView(seg.buffer, seg.byteOffset, 4).getUint32(0, false);
  if (ctLen !== expectedCtLen) {
    throw new Error('cwch2_chunk_length_mismatch');
  }

  // Exact match with app/src/crypto/blob.rs:
  // nonce[8..12] ^= chunk_index.to_be_bytes()
  const chunkNonce = new Uint8Array(grant.baseNonce);
  chunkNonce[8] ^= (chunkIdx >>> 24) & 0xff;
  chunkNonce[9] ^= (chunkIdx >>> 16) & 0xff;
  chunkNonce[10] ^= (chunkIdx >>> 8) & 0xff;
  chunkNonce[11] ^= chunkIdx & 0xff;

  // AAD = b"CWCH2" (5B) || chunk_index BE (4B) || [is_final] (1B)
  const aad = new Uint8Array(10);
  aad.set(CWCH2_MAGIC, 0);
  const aadDv = new DataView(aad.buffer);
  aadDv.setUint32(5, chunkIdx, false);
  aad[9] = chunkIdx === chunkCount - 1 ? 1 : 0;

  const plainBuf = await crypto.subtle.decrypt(
    {
      name: 'AES-GCM',
      iv: chunkNonce,
      additionalData: aad,
    },
    grant.aesKey,
    seg.subarray(4, 4 + ctLen),
  );
  const plainU8 = new Uint8Array(plainBuf);
  lruSet(cacheKey, plainU8);
  return plainU8;
}

async function handleCwch2Stream(request, grant, grantId, meta) {
  const total = meta.totalPlainLen;
  const rangeHdr = request.headers.get('Range');
  let start = 0;
  let end = Math.min(total - 1, MAX_CWCH2_RANGE_SPAN - 1);
  let isPartial = Boolean(rangeHdr);

  if (rangeHdr) {
    const parsed = parseByteRange(rangeHdr, total);
    if (!parsed) {
      return new Response(null, {
        status: 416,
        headers: { 'Content-Range': `bytes */${total}` },
      });
    }
    start = parsed.start;
    end = Math.min(parsed.end, start + MAX_CWCH2_RANGE_SPAN - 1);
  } else if (total > MAX_CWCH2_RANGE_SPAN) {
    // Full non-range read: cap at full length up to 16 MiB
    end = total - 1;
  }

  const firstChunk = Math.floor(start / meta.chunkSize);
  const lastChunk = Math.floor(end / meta.chunkSize);
  const outLen = end - start + 1;
  const out = new Uint8Array(outLen);
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
  };
  if (isPartial) {
    headers['Content-Range'] = `bytes ${start}-${end}/${total}`;
  }
  return new Response(out, {
    status: isPartial ? 206 : 200,
    headers,
  });
}

async function handleLegacySingleE2eeStream(request, grant) {
  if (!grant.singlePlainBuf) {
    const res = await fetch(grant.url, {
      credentials: 'include',
      headers: buildUpstreamHeaders(grant, null),
    });
    if (!res.ok) throw new Error(`legacy_e2ee_http_${res.status}`);
    const ct = new Uint8Array(await res.arrayBuffer());
    const pt = await crypto.subtle.decrypt(
      { name: 'AES-GCM', iv: grant.baseNonce },
      grant.aesKey,
      ct,
    );
    grant.singlePlainBuf = new Uint8Array(pt);
  }

  const plain = grant.singlePlainBuf;
  const total = plain.byteLength;
  const rangeHdr = request.headers.get('Range');
  if (rangeHdr) {
    const parsed = parseByteRange(rangeHdr, total);
    if (!parsed) {
      return new Response(null, {
        status: 416,
        headers: { 'Content-Range': `bytes */${total}` },
      });
    }
    const slice = plain.subarray(parsed.start, parsed.end + 1);
    return new Response(slice, {
      status: 206,
      headers: {
        'Content-Type': grant.mime || 'application/octet-stream',
        'Accept-Ranges': 'bytes',
        'Content-Range': `bytes ${parsed.start}-${parsed.end}/${total}`,
        'Content-Length': String(slice.byteLength),
        'Cache-Control': 'no-store',
        'X-ChatW-Stream-Mode': 'sw-e2ee-single-206',
      },
    });
  }

  return new Response(plain, {
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
  const upstream = await fetch(grant.url, {
    credentials: 'include',
    headers: buildUpstreamHeaders(grant, rangeHdr),
  });
  const hdrs = new Headers();
  for (const h of ['content-type', 'content-range', 'content-length', 'accept-ranges']) {
    const v = upstream.headers.get(h);
    if (v) hdrs.set(h, v);
  }
  if (!hdrs.has('accept-ranges')) hdrs.set('accept-ranges', 'bytes');
  hdrs.set('cache-control', 'no-store');
  hdrs.set('x-chatw-stream-mode', 'sw-range-proxy');
  return new Response(upstream.body, {
    status: upstream.status,
    statusText: upstream.statusText,
    headers: hdrs,
  });
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

  if (msg.type === 'PING_SW') {
    reply({ ok: true, version: 'chatw-sw-v1', activeGrants: grants.size });
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
    });
    // Prune expired grants lazily
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
      streamUrl: `${STREAM_PREFIX}${grantId}`,
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
  }
});

self.addEventListener('fetch', (event) => {
  const reqUrl = new URL(event.request.url);
  if (reqUrl.origin !== self.location.origin) return;
  if (!reqUrl.pathname.startsWith(STREAM_PREFIX)) return;

  event.respondWith((async () => {
    // Layer 1: Block direct browser tab navigation / document / iframe opens
    if (
      event.request.mode === 'navigate' ||
      event.request.destination === 'document' ||
      event.request.destination === 'iframe'
    ) {
      return forbiddenResponse('direct_navigation_denied');
    }

    // Layer 2: Restrict request destination to media/subresource only
    if (!ALLOWED_DESTINATIONS.has(event.request.destination || '')) {
      return forbiddenResponse('disallowed_destination');
    }

    // Layer 3: Validate active unexpired grant
    const grantId = reqUrl.pathname.slice(STREAM_PREFIX.length).split('/')[0];
    const grant = grants.get(grantId);
    if (!grant || Date.now() > grant.expiresAt) {
      if (grant) {
        grants.delete(grantId);
        purgeGrantCache(grantId);
      }
      return forbiddenResponse('grant_missing_or_expired');
    }

    // Layer 4: Strict per-tab clientId ownership verification
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
});
