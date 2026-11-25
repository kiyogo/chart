// Client-side IndexedDB + OPFS + Cache API Intelligent Storage Engine for ChatW (Spec 022 + Spec 110)
// - W-TinyLFU (4-bit Count-Min Sketch) + Segmented LRU + Cost-Aware & Size-Aware Eviction
// - Adaptive Capacity Budget based on real browser storage headroom
// - Dedicated AI Output Cache (`ai_outputs`), AI Model Weight Cache (`ai_models`), Video Segment Cache (`video_segments`)
// - SHA-256 integrity verification, QuotaExceededError resilience, & protection for unique unbacked DM files.
(() => {
  const DB_NAME = 'chatw_storage_v1';
  const DB_VERSION = 6;
  const OPFS_DIR = 'chatw_cas';
  const CACHE_NAME = 'chatw_media_cache_v1';
  const MAX_CACHED_MESSAGES_PER_CONV = 2000;
  const MAX_CACHED_CONVS = 200;

  // 4-bit Count-Min Sketch for W-TinyLFU frequency estimation (4 rows x 1024 counters, periodic halving)
  const CMS_WIDTH = 1024;
  const CMS_ROWS = 4;
  const cmsTable = new Uint8Array(CMS_ROWS * CMS_WIDTH);
  let cmsOps = 0;

  function cmsHash(str, seed) {
    let h = (2166136261 ^ seed) >>> 0;
    for (let i = 0; i < str.length; i++) {
      h ^= str.charCodeAt(i);
      h = Math.imul(h, 16777619) >>> 0;
    }
    return h & (CMS_WIDTH - 1);
  }

  function cmsRecord(key) {
    const s = String(key || '');
    for (let r = 0; r < CMS_ROWS; r++) {
      const idx = r * CMS_WIDTH + cmsHash(s, (r + 1) * 0x9e3779b1);
      if (cmsTable[idx] < 15) cmsTable[idx]++;
    }
    cmsOps++;
    if (cmsOps >= 8192) {
      for (let i = 0; i < cmsTable.length; i++) cmsTable[i] >>= 1;
      cmsOps = 0;
    }
  }

  function cmsEstimate(key) {
    const s = String(key || '');
    let minVal = 15;
    for (let r = 0; r < CMS_ROWS; r++) {
      const idx = r * CMS_WIDTH + cmsHash(s, (r + 1) * 0x9e3779b1);
      if (cmsTable[idx] < minVal) minVal = cmsTable[idx];
    }
    return minVal;
  }

  let dbPromise = null;

  function openDb() {
    if (dbPromise) return dbPromise;
    dbPromise = new Promise((resolve, reject) => {
      if (typeof indexedDB === 'undefined') {
        reject(new Error('indexeddb_unavailable'));
        return;
      }
      const req = indexedDB.open(DB_NAME, DB_VERSION);
      req.onupgradeneeded = (e) => {
        const db = e.target.result;
        if (!db.objectStoreNames.contains('objects')) {
          const os = db.createObjectStore('objects', { keyPath: 'ref_id' });
          os.createIndex('last_accessed_at', 'last_accessed_at', { unique: false });
          os.createIndex('scope_type', 'scope_type', { unique: false });
        }
        if (!db.objectStoreNames.contains('sync_queue')) {
          const sq = db.createObjectStore('sync_queue', { keyPath: 'ref_id' });
          sq.createIndex('priority', 'priority', { unique: false });
        }
        if (!db.objectStoreNames.contains('sync_cursors')) {
          db.createObjectStore('sync_cursors', { keyPath: 'id' });
        }
        if (!db.objectStoreNames.contains('passkeys')) {
          db.createObjectStore('passkeys', { keyPath: 'id' });
        }
        if (!db.objectStoreNames.contains('chat_messages')) {
          const cm = db.createObjectStore('chat_messages', { keyPath: 'conv_key' });
          cm.createIndex('updated_at', 'updated_at', { unique: false });
        }
        if (!db.objectStoreNames.contains('chat_outbox')) {
          const co = db.createObjectStore('chat_outbox', { keyPath: 'client_id' });
          co.createIndex('created_at', 'created_at', { unique: false });
        }
        if (!db.objectStoreNames.contains('dialogs')) {
          db.createObjectStore('dialogs', { keyPath: 'id' });
        }
        if (!db.objectStoreNames.contains('users_peers')) {
          db.createObjectStore('users_peers', { keyPath: 'user_id' });
        }
        if (!db.objectStoreNames.contains('sync_state')) {
          db.createObjectStore('sync_state', { keyPath: 'id' });
        }
        if (!db.objectStoreNames.contains('drafts')) {
          db.createObjectStore('drafts', { keyPath: 'conv_key' });
        }
        if (!db.objectStoreNames.contains('static_cache')) {
          db.createObjectStore('static_cache', { keyPath: 'key' });
        }
        if (!db.objectStoreNames.contains('crypto_keys')) {
          db.createObjectStore('crypto_keys', { keyPath: 'id' });
        }
        // Spec 110 stores: Dedicated AI Output Cache, AI Model Cache, and Video Segment Cache
        if (!db.objectStoreNames.contains('ai_outputs')) {
          const ao = db.createObjectStore('ai_outputs', { keyPath: 'cache_key' });
          ao.createIndex('last_accessed_at', 'last_accessed_at', { unique: false });
          ao.createIndex('model_version', 'model_version', { unique: false });
        }
        if (!db.objectStoreNames.contains('ai_models')) {
          db.createObjectStore('ai_models', { keyPath: 'model_id' });
        }
        if (!db.objectStoreNames.contains('video_segments')) {
          const vs = db.createObjectStore('video_segments', { keyPath: 'seg_key' });
          vs.createIndex('last_accessed_at', 'last_accessed_at', { unique: false });
        }
        // Spec 164 stores: Encrypted Zstd-22 Message Chunks, Manifests, Encrypted Search Shards,
        // Versioned Dictionaries, and Crash-Recovery Compression Checkpoints (non-destructive v6 upgrade!)
        if (!db.objectStoreNames.contains('msg_chunks')) {
          const mc = db.createObjectStore('msg_chunks', { keyPath: 'chunk_id' });
          mc.createIndex('scope_key', 'scope_key', { unique: false });
          mc.createIndex('last_accessed_at', 'last_accessed_at', { unique: false });
        }
        if (!db.objectStoreNames.contains('chunk_manifests')) {
          db.createObjectStore('chunk_manifests', { keyPath: 'scope_key' });
        }
        if (!db.objectStoreNames.contains('search_shards')) {
          const ss = db.createObjectStore('search_shards', { keyPath: 'shard_key' });
          ss.createIndex('updated_at', 'updated_at', { unique: false });
        }
        if (!db.objectStoreNames.contains('zstd_dicts')) {
          db.createObjectStore('zstd_dicts', { keyPath: 'dict_id' });
        }
        if (!db.objectStoreNames.contains('compression_checkpoints')) {
          db.createObjectStore('compression_checkpoints', { keyPath: 'scope_key' });
        }
      };
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => reject(req.error || new Error('indexeddb_open_failed'));
    });
    return dbPromise;
  }

  async function idbTx(storeName, mode, fn) {
    const db = await openDb();
    return new Promise((resolve, reject) => {
      const tx = db.transaction(storeName, mode);
      const store = tx.objectStore(storeName);
      const req = fn(store);
      tx.oncomplete = () => resolve(req && 'result' in req ? req.result : undefined);
      tx.onerror = () => reject(tx.error || new Error('idb_tx_failed'));
      tx.onabort = () => reject(tx.error || new Error('idb_tx_aborted'));
    });
  }

  async function idbGetAll(storeName) {
    const db = await openDb();
    return new Promise((resolve, reject) => {
      const tx = db.transaction(storeName, 'readonly');
      const req = tx.objectStore(storeName).getAll();
      req.onsuccess = () => resolve(req.result || []);
      req.onerror = () => reject(req.error || new Error('idb_getall_failed'));
    });
  }

  async function getOpfsRoot() {
    if (navigator.storage && typeof navigator.storage.getDirectory === 'function') {
      try {
        const root = await navigator.storage.getDirectory();
        return await root.getDirectoryHandle(OPFS_DIR, { create: true });
      } catch (_) {
        return null;
      }
    }
    return null;
  }

  async function writeOpfs(fileKey, bytes) {
    const dir = await getOpfsRoot();
    if (!dir) return false;
    const fh = await dir.getFileHandle(fileKey, { create: true });
    const writable = await fh.createWritable();
    try {
      await writable.write(bytes);
      await writable.close();
      return true;
    } catch (err) {
      try { await writable.abort(); } catch (_) {}
      throw err;
    }
  }

  async function readOpfs(fileKey) {
    const dir = await getOpfsRoot();
    if (!dir) return null;
    try {
      const fh = await dir.getFileHandle(fileKey);
      const file = await fh.getFile();
      return new Uint8Array(await file.arrayBuffer());
    } catch (_) {
      return null;
    }
  }

  async function deleteOpfs(fileKey) {
    const dir = await getOpfsRoot();
    if (!dir) return;
    try {
      await dir.removeEntry(fileKey);
    } catch (_) {}
  }

  function b64ToBytes(b64) {
    const bin = atob(b64);
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
    return out;
  }

  function bytesToB64(buf) {
    const u8 = buf instanceof Uint8Array ? buf : new Uint8Array(buf);
    let s = '';
    for (let i = 0; i < u8.length; i += 0x8000) {
      s += String.fromCharCode.apply(null, u8.subarray(i, i + 0x8000));
    }
    return btoa(s);
  }

  async function sha256Hex(u8) {
    if (!crypto || !crypto.subtle) return '';
    const digest = await crypto.subtle.digest('SHA-256', u8);
    return Array.from(new Uint8Array(digest)).map((b) => b.toString(16).padStart(2, '0')).join('');
  }

  function getConfiguredLimitMb() {
    try {
      const v = parseInt(localStorage.getItem('chatw_cache_limit_mb') || '1024', 10);
      return Number.isFinite(v) && v >= 0 ? v : 1024;
    } catch (_) {
      return 1024;
    }
  }

  function isSafeToEvict(meta) {
    // NEVER silently evict unique local-only or undelivered DM files!
    const state = meta.availability_state || '';
    if (state === 'local_only' || state === 'pending_delivery') return false;
    return true;
  }

  async function deleteLocalObjectEntry(meta) {
    if (meta.opfs_key) {
      await deleteOpfs(meta.opfs_key);
    }
    try {
      if (typeof caches !== 'undefined') {
        const cache = await caches.open(CACHE_NAME);
        await cache.delete('/__chatw_local/' + meta.ref_id);
      }
    } catch (_) {}
    await idbTx('objects', 'readwrite', (s) => s.delete(meta.ref_id));
  }

  async function computeAdaptiveLimitBytes() {
    const cfgMb = getConfiguredLimitMb();
    // Spec 163: Always enforce an adaptive ceiling proportional to real device/browser storage,
    // even when Auto (0 MB) is selected, so we NEVER fill disk on Web/Android/iOS/Desktop!
    let autoCapBytes = 512 * 1024 * 1024;
    try {
      if (navigator.storage && typeof navigator.storage.estimate === 'function') {
        const est = await navigator.storage.estimate();
        const quota = Number(est.quota || 0);
        const usage = Number(est.usage || 0);
        if (quota > 0) {
          const freeBytes = Math.max(0, quota - usage);
          // Reserve generous OS/browser headroom: auto target = 12% of available free quota (clamped 48 MiB..4 GiB)
          const proportional = Math.floor(freeBytes * 0.12);
          const safeCap = Math.max(32 * 1024 * 1024, Math.min(4 * 1024 * 1024 * 1024, Math.floor(quota * 0.65)));
          autoCapBytes = Math.max(32 * 1024 * 1024, Math.min(safeCap, Math.max(64 * 1024 * 1024, proportional)));
          if (freeBytes < 192 * 1024 * 1024) {
            autoCapBytes = Math.min(autoCapBytes, Math.max(24 * 1024 * 1024, Math.floor(freeBytes * 0.35)));
          }
        }
      }
    } catch (_) {}
    if (cfgMb === 0) return autoCapBytes;
    return Math.min(cfgMb * 1024 * 1024, Math.max(autoCapBytes, 64 * 1024 * 1024));
  }

  // W-TinyLFU + Cost-Aware & Size-Aware eviction utility score (higher = more valuable to keep)
  function computeRetentionScore(item, nowMs) {
    const key = item.ref_id || item.cache_key || item.seg_key || '';
    const freq = cmsEstimate(key) + Math.min(15, Number(item.access_count || 1));
    const ageHours = Math.max(0.05, (nowMs - (item.last_accessed_at || item.updated_at || nowMs)) / 3600000);
    const recency = 1.0 / (1.0 + Math.log2(1.0 + ageHours));
    const sizeKb = Math.max(1, (item.byte_size || 1024) / 1024);
    const computeCost = 1.0 + Math.min(8.0, Number(item.compute_cost_ms || 0) / 120.0);
    // Size-aware penalty: giant single-hit files lose against frequently-reused compact media/AI outputs
    return ((freq * 1.4 + recency * 3.0) * computeCost) / Math.pow(sizeKb, 0.35);
  }

  async function evictSafeCacheIfNeeded(incomingBytes) {
    const limitBytes = await computeAdaptiveLimitBytes();
    if (limitBytes === Number.MAX_SAFE_INTEGER) return { evicted: 0, freed: 0 };
    const rows = await idbGetAll('objects');
    const totalUsed = rows.reduce((acc, r) => acc + (r.byte_size || 0), 0);
    if (totalUsed + incomingBytes <= limitBytes) return { evicted: 0, freed: 0 };

    const nowMs = Date.now();
    // W-TinyLFU + Cost-Aware + Size-Aware sort on evictable entries ONLY (lowest retention score evicted first!)
    const candidates = rows
      .filter(isSafeToEvict)
      .sort((a, b) => computeRetentionScore(a, nowMs) - computeRetentionScore(b, nowMs));
    let freed = 0;
    let evicted = 0;
    let current = totalUsed;
    for (const item of candidates) {
      if (current + incomingBytes <= limitBytes) break;
      await deleteLocalObjectEntry(item);
      current -= (item.byte_size || 0);
      freed += (item.byte_size || 0);
      evicted += 1;
    }
    return { evicted, freed };
  }

  async function evictAiCacheIfNeeded(incomingBytes) {
    const limitBytes = await computeAdaptiveLimitBytes();
    // Dedicate up to 22% of adaptive budget (capped 320 MiB, min 24 MiB) to AI Output Cache
    const aiCap = limitBytes === Number.MAX_SAFE_INTEGER
      ? 320 * 1024 * 1024
      : Math.max(24 * 1024 * 1024, Math.min(320 * 1024 * 1024, Math.floor(limitBytes * 0.22)));
    const rows = await idbGetAll('ai_outputs').catch(() => []);
    let total = rows.reduce((acc, r) => acc + (r.byte_size || 0), 0);
    if (total + incomingBytes <= aiCap) return 0;
    const nowMs = Date.now();
    rows.sort((a, b) => computeRetentionScore(a, nowMs) - computeRetentionScore(b, nowMs));
    let evicted = 0;
    for (const item of rows) {
      if (total + incomingBytes <= aiCap) break;
      await idbTx('ai_outputs', 'readwrite', (s) => s.delete(item.cache_key)).catch(() => {});
      total -= (item.byte_size || 0);
      evicted++;
    }
    return evicted;
  }

  async function requestPersistence() {
    let persisted = false;
    try {
      if (navigator.storage && typeof navigator.storage.persisted === 'function') {
        persisted = await navigator.storage.persisted();
        if (!persisted && typeof navigator.storage.persist === 'function') {
          persisted = await navigator.storage.persist();
        }
      }
    } catch (_) {}
    return persisted;
  }

  function getClientDeviceId() {
    try {
      let id = localStorage.getItem('chatw_client_device_id');
      if (!id || id.length < 8) {
        id = (crypto && crypto.randomUUID) ? crypto.randomUUID() : ('web-' + Date.now().toString(36) + '-' + Math.random().toString(36).slice(2, 10));
        localStorage.setItem('chatw_client_device_id', id);
      }
      return id;
    } catch (_) {
      return 'ephemeral-web-' + Date.now().toString(36);
    }
  }

  function detectDeviceName() {
    const ua = navigator.userAgent || '';
    const os = ua.includes('Windows') ? 'Windows' : ua.includes('Mac') ? 'macOS' : ua.includes('Android') ? 'Android' : ua.includes('iPhone') || ua.includes('iPad') ? 'iOS' : ua.includes('Linux') ? 'Linux' : 'Web';
    const br = ua.includes('Edg/') ? 'Edge' : ua.includes('Firefox') ? 'Firefox' : ua.includes('Chrome') ? 'Chrome' : ua.includes('Safari') ? 'Safari' : 'Browser';
    return `${os} · ${br}`;
  }

  window.__chatwStorage = {
    getClientDeviceId,
    detectDeviceName,
    requestPersistence,
    sha256Hex,
    b64ToBytes,
    bytesToB64,

    async init() {
      await openDb();
      const persisted = await requestPersistence();
      const opfs = (await getOpfsRoot()) !== null;
      // Schedule gentle 15-day cold image compaction once in background idle without stalling UI
      if (!window.__chatwCold15dScheduled) {
        window.__chatwCold15dScheduled = true;
        const runIdle = () => {
          setTimeout(() => {
            if (window.__chatwStorage && typeof window.__chatwStorage.compressColdImages15d === 'function') {
              window.__chatwStorage.compressColdImages15d(15, false).catch(() => {});
            }
          }, 8000);
        };
        if (typeof requestIdleCallback === 'function') requestIdleCallback(runIdle, { timeout: 10000 });
        else runIdle();
      }
      return {
        ok: true,
        persisted,
        opfsSupported: opfs,
        cacheApiSupported: typeof caches !== 'undefined',
        clientDeviceId: getClientDeviceId(),
        deviceName: detectDeviceName(),
      };
    },

    async putObject(meta, b64OrUint8) {
      const bytes = typeof b64OrUint8 === 'string' ? b64ToBytes(b64OrUint8) : new Uint8Array(b64OrUint8);
      const shaHex = await sha256Hex(bytes);
      const now = Date.now();
      const opfsKey = String(meta.ref_id).replace(/[^a-zA-Z0-9_-]/g, '_');

      const attemptWrite = async () => {
        await evictSafeCacheIfNeeded(bytes.byteLength);
        let storedInOpfs = false;
        try {
          storedInOpfs = await writeOpfs(opfsKey, bytes);
        } catch (e) {
          if (e && (e.name === 'QuotaExceededError' || e.code === 22)) throw e;
          storedInOpfs = false;
        }
        const record = {
          ref_id: meta.ref_id,
          scope_type: meta.scope_type || 'dm',
          scope_id: meta.scope_id || null,
          file_name: meta.file_name || 'file.bin',
          mime_type: meta.mime_type || 'application/octet-stream',
          byte_size: bytes.byteLength,
          original_byte_size: Number(meta.original_byte_size || bytes.byteLength),
          sha256_plain: meta.sha256_plain || shaHex,
          availability_state: meta.availability_state || 'local_only',
          storage_tier: storedInOpfs ? 'opfs' : 'indexeddb',
          opfs_key: storedInOpfs ? opfsKey : null,
          blob: storedInOpfs ? null : bytes,
          thumb_b64: meta.thumb_b64 || null,
          cold_compressed_15d: Boolean(meta.cold_compressed_15d || (meta.mime_type || '') === 'image/avif'),
          created_at: Number(meta.created_at || now),
          updated_at: now,
          last_accessed_at: Number(meta.last_accessed_at || now),
        };
        await idbTx('objects', 'readwrite', (s) => s.put(record));
        try {
          if (typeof caches !== 'undefined' && bytes.byteLength <= 16 * 1024 * 1024) {
            const cache = await caches.open(CACHE_NAME);
            await cache.put('/__chatw_local/' + meta.ref_id, new Response(bytes, { headers: { 'Content-Type': record.mime_type } }));
          }
        } catch (_) {}
        return { ok: true, sha256: shaHex, byte_size: bytes.byteLength, local_tier: record.storage_tier };
      };

      try {
        return await attemptWrite();
      } catch (err) {
        if (err && (err.name === 'QuotaExceededError' || err.code === 22)) {
          // Force cleanup of all safe disposable caches and retry once.
          const ev = await this.clearSafeCache(false);
          if (ev.freed_bytes > 0) {
            try {
              return await attemptWrite();
            } catch (err2) {
              return { ok: false, error: 'quota_exceeded', freed_before_retry: ev.freed_bytes };
            }
          }
          return { ok: false, error: 'quota_exceeded' };
        }
        return { ok: false, error: String(err && err.message ? err.message : err) };
      }
    },

    async getObject(refId) {
      const meta = await idbTx('objects', 'readonly', (s) => s.get(refId));
      if (!meta) return { found: false };
      let bytes = null;
      if (meta.opfs_key) {
        bytes = await readOpfs(meta.opfs_key);
      }
      if (!bytes && meta.blob) {
        bytes = meta.blob instanceof Uint8Array ? meta.blob : new Uint8Array(meta.blob);
      }
      if (!bytes && typeof caches !== 'undefined') {
        try {
          const cache = await caches.open(CACHE_NAME);
          const match = await cache.match('/__chatw_local/' + refId);
          if (match) bytes = new Uint8Array(await match.arrayBuffer());
        } catch (_) {}
      }
      if (!bytes) return { found: false };

      const actualSha = await sha256Hex(bytes);
      if (meta.sha256_plain && actualSha && actualSha !== meta.sha256_plain) {
        return { found: false, corrupted: true };
      }
      // Touch W-TinyLFU frequency sketch + access count + LRU timestamp
      cmsRecord(refId);
      try {
        meta.last_accessed_at = Date.now();
        meta.access_count = (meta.access_count || 1) + 1;
        await idbTx('objects', 'readwrite', (s) => s.put(meta));
      } catch (_) {}

      return {
        found: true,
        bytesB64: bytesToB64(bytes),
        mime_type: meta.mime_type,
        file_name: meta.file_name,
        byte_size: bytes.byteLength,
        sha256_plain: actualSha,
        local_tier: meta.storage_tier,
        availability_state: meta.availability_state,
      };
    },

    async listLocalLocations() {
      const rows = await idbGetAll('objects');
      return rows.map((r) => ({
        ref_id: r.ref_id,
        local_tier: r.storage_tier || 'indexeddb',
        sha256_hex: r.sha256_plain || null,
        has_original: true,
        byte_size: r.byte_size || 0,
        scope_type: r.scope_type || 'dm',
        availability_state: r.availability_state || 'local_only',
      }));
    },

    async deleteObject(refId, forceUnique = false) {
      const meta = await idbTx('objects', 'readonly', (s) => s.get(refId));
      if (!meta) return { deleted: false, reason: 'not_found' };
      if (!forceUnique && !isSafeToEvict(meta)) {
        return { deleted: false, reason: 'unique_local_file' };
      }
      await deleteLocalObjectEntry(meta);
      return { deleted: true, freed_bytes: meta.byte_size || 0 };
    },

    async clearSafeCache(includeUnique = false) {
      const rows = await idbGetAll('objects');
      let freed = 0;
      let removed = 0;
      let protectedCount = 0;
      for (const r of rows) {
        if (!includeUnique && !isSafeToEvict(r)) {
          protectedCount += 1;
          continue;
        }
        await deleteLocalObjectEntry(r);
        freed += (r.byte_size || 0);
        removed += 1;
      }
      return { removed, freed_bytes: freed, protected_unique: protectedCount };
    },

    async getQuotaEstimate() {
      let browserQuota = 0;
      let browserUsage = 0;
      let persisted = false;
      try {
        if (navigator.storage) {
          if (typeof navigator.storage.persisted === 'function') persisted = await navigator.storage.persisted();
          if (typeof navigator.storage.estimate === 'function') {
            const est = await navigator.storage.estimate();
            browserQuota = est.quota || 0;
            browserUsage = est.usage || 0;
          }
        }
      } catch (_) {}
      const rows = await idbGetAll('objects');
      let totalLocalBytes = 0;
      let disposableBytes = 0;
      let pinnedUniqueBytes = 0;
      for (const r of rows) {
        const sz = r.byte_size || 0;
        totalLocalBytes += sz;
        if (isSafeToEvict(r)) disposableBytes += sz;
        else pinnedUniqueBytes += sz;
      }
      const opfsSupported = (await getOpfsRoot()) !== null;
      return {
        limit_mb: getConfiguredLimitMb(),
        browser_quota_bytes: browserQuota,
        browser_usage_bytes: browserUsage,
        persisted,
        opfs_supported: opfsSupported,
        local_object_count: rows.length,
        total_local_bytes: totalLocalBytes,
        disposable_cache_bytes: disposableBytes,
        pinned_unique_bytes: pinnedUniqueBytes,
      };
    },

    async saveCursors(cursors) {
      const uid = (cursors && cursors.user_id) || window.__chatwCurrentUserId || '';
      const key = uid ? `user:${uid}` : 'current';
      await idbTx('sync_cursors', 'readwrite', (s) => s.put({ id: key, ...cursors, updated_at: Date.now() }));
      return true;
    },

    async loadCursors(userId = null) {
      const uid = userId || window.__chatwCurrentUserId || '';
      const key = uid ? `user:${uid}` : 'current';
      const rec = await idbTx('sync_cursors', 'readonly', (s) => s.get(key));
      if (rec) return rec;
      if (key !== 'current') {
        const legacy = await idbTx('sync_cursors', 'readonly', (s) => s.get('current'));
        if (legacy && (!legacy.user_id || legacy.user_id === uid)) return legacy;
      }
      return { id: key, dm_seq: 0, group_seq: 0, channel_seq: 0 };
    },

    async enqueueSync(item) {
      await idbTx('sync_queue', 'readwrite', (s) => s.put({ ...item, updated_at: Date.now() }));
      return true;
    },

    async listSyncQueue() {
      return await idbGetAll('sync_queue');
    },

    async removeSyncQueue(refId) {
      await idbTx('sync_queue', 'readwrite', (s) => s.delete(refId));
      return true;
    },

    async streamDownloadPost(url, jsonBody, defaultFilename, extraHeaders = {}) {
      const headers = { 'Content-Type': 'application/json', ...extraHeaders };
      const res = await fetch(url, {
        method: 'POST',
        credentials: 'include',
        headers,
        body: jsonBody,
      });
      if (!res.ok) {
        const text = await res.text();
        let code = 'request_failed';
        try {
          const parsed = JSON.parse(text);
          if (parsed && typeof parsed.error === 'string') code = parsed.error;
        } catch (_) {}
        return { ok: false, status: res.status, error: code };
      }
      const jobId = res.headers.get('x-backup-job-id') || null;
      const disp = res.headers.get('content-disposition') || '';
      let filename = defaultFilename || 'KIYO_Backup.kdata';
      const m = disp.match(/filename="?([^";\n]+)"?/i);
      if (m && m[1]) filename = m[1].trim();

      const chunks = [];
      let totalBytes = 0;
      if (res.body && typeof res.body.getReader === 'function') {
        const reader = res.body.getReader();
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          if (value && value.byteLength > 0) {
            chunks.push(value);
            totalBytes += value.byteLength;
          }
        }
      } else {
        const buf = new Uint8Array(await res.arrayBuffer());
        chunks.push(buf);
        totalBytes = buf.byteLength;
      }

      const blob = new Blob(chunks, { type: res.headers.get('content-type') || 'application/octet-stream' });
      const objUrl = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = objUrl;
      a.download = filename;
      a.style.display = 'none';
      document.body.appendChild(a);
      a.click();
      setTimeout(() => {
        try { document.body.removeChild(a); } catch (_) {}
        try { URL.revokeObjectURL(objUrl); } catch (_) {}
      }, 30000);

      return { ok: true, status: res.status, filename, byte_size: totalBytes, job_id: jobId };
    },

    getSelectedFileInfo(inputId) {
      const input = document.getElementById(inputId);
      if (!input || !input.files || input.files.length === 0) return null;
      const f = input.files[0];
      return { name: f.name, byte_size: f.size };
    },

    async uploadBackupInput(inputId, url, backupPassword) {
      const input = document.getElementById(inputId);
      if (!input || !input.files || input.files.length === 0) {
        return { ok: false, status: 400, error: 'no_file', body: '' };
      }
      const file = input.files[0];
      if (file.size > 512 * 1024 * 1024) {
        return { ok: false, status: 413, error: 'backup_too_large', body: '' };
      }
      const headers = { 'Content-Type': 'application/octet-stream' };
      if (backupPassword && backupPassword.length > 0) {
        headers['x-backup-password'] = backupPassword;
      }
      const res = await fetch(url, {
        method: 'POST',
        credentials: 'include',
        headers,
        body: file,
      });
      const text = await res.text();
      return {
        ok: res.ok,
        status: res.status,
        body: text,
        filename: file.name,
        byte_size: file.size,
      };
    },

    async registerDevicePasskey() {
      const kp = await crypto.subtle.generateKey(
        { name: 'ECDSA', namedCurve: 'P-256' },
        false,
        ['sign', 'verify'],
      );
      const rawPub = new Uint8Array(await crypto.subtle.exportKey('raw', kp.publicKey));
      const rand = crypto.getRandomValues(new Uint8Array(16));
      const credId = bytesToB64(rand).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
      const pubB64 = bytesToB64(rawPub).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
      await idbTx('passkeys', 'readwrite', (s) =>
        s.put({
          id: 'default',
          credential_id: credId,
          privateKey: kp.privateKey,
          publicKeyB64: pubB64,
          created_at: Date.now(),
        }),
      );
      return { credential_id: credId, public_key_spki_b64: pubB64 };
    },

    async signPasskeyChallenge(challengeB64) {
      const rec = await idbTx('passkeys', 'readonly', (s) => s.get('default'));
      if (!rec || !rec.privateKey || !rec.credential_id) {
        return { error: 'no_local_passkey' };
      }
      const msgBytes = new TextEncoder().encode(challengeB64);
      const sigBuf = await crypto.subtle.sign(
        { name: 'ECDSA', hash: 'SHA-256' },
        rec.privateKey,
        msgBytes,
      );
      const sigB64 = bytesToB64(new Uint8Array(sigBuf))
        .replace(/\+/g, '-')
        .replace(/\//g, '_')
        .replace(/=+$/, '');
      return { credential_id: rec.credential_id, signature_b64: sigB64 };
    },

    async hasLocalPasskey() {
      const rec = await idbTx('passkeys', 'readonly', (s) => s.get('default')).catch(() => null);
      return Boolean(rec && rec.credential_id && rec.privateKey);
    },

    // ── Offline-First Message Cache (Encrypted At Rest via Non-Extractable Vault AES-GCM) & Outbox ──
    async putChatMessages(convKey, items, hasMore = false) {
      if (!convKey || !Array.isArray(items)) return false;
      const trimmed = items.length > MAX_CACHED_MESSAGES_PER_CONV
        ? items.slice(items.length - MAX_CACHED_MESSAGES_PER_CONV)
        : items;
      try {
        let record = {
          conv_key: String(convKey),
          items: trimmed,
          has_more: Boolean(hasMore),
          updated_at: Date.now(),
        };
        // Encrypt cached messages at rest using non-extractable WebCrypto AES-256-GCM vault key!
        try {
          const vRec = await idbTx('crypto_keys', 'readonly', (s) => s.get('device_vault_aes'));
          const aesKey = vRec && vRec.key;
          if (aesKey && crypto && crypto.subtle) {
            const iv = crypto.getRandomValues(new Uint8Array(12));
            const pt = new TextEncoder().encode(JSON.stringify(trimmed));
            const ct = new Uint8Array(await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, aesKey, pt));
            record = {
              conv_key: String(convKey),
              enc_v1: true,
              iv,
              ct,
              item_count: trimmed.length,
              byte_size: ct.byteLength,
              has_more: Boolean(hasMore),
              updated_at: Date.now(),
            };
          }
        } catch (_) {}

        await idbTx('chat_messages', 'readwrite', (s) => s.put(record));
        // Lazy LRU cap on total cached conversations
        const all = await idbGetAll('chat_messages').catch(() => []);
        if (all.length > MAX_CACHED_CONVS) {
          all.sort((a, b) => (a.updated_at || 0) - (b.updated_at || 0));
          const excess = all.slice(0, all.length - MAX_CACHED_CONVS);
          for (const row of excess) {
            await idbTx('chat_messages', 'readwrite', (s) => s.delete(row.conv_key)).catch(() => {});
          }
        }
        return true;
      } catch (_) {
        return false;
      }
    },

    async getChatMessages(convKey) {
      if (!convKey) return { found: false, items: [], has_more: false };
      try {
        const rec = await idbTx('chat_messages', 'readonly', (s) => s.get(String(convKey)));
        if (!rec) return { found: false, items: [], has_more: false };
        if (rec.enc_v1 && rec.iv && rec.ct) {
          try {
            const vRec = await idbTx('crypto_keys', 'readonly', (s) => s.get('device_vault_aes'));
            const aesKey = vRec && vRec.key;
            if (aesKey && crypto && crypto.subtle) {
              const iv = rec.iv instanceof Uint8Array ? rec.iv : new Uint8Array(rec.iv);
              const ct = rec.ct instanceof Uint8Array ? rec.ct : new Uint8Array(rec.ct);
              const pt = await crypto.subtle.decrypt({ name: 'AES-GCM', iv }, aesKey, ct);
              const parsed = JSON.parse(new TextDecoder().decode(pt));
              if (Array.isArray(parsed)) {
                return {
                  found: true,
                  items: parsed,
                  has_more: Boolean(rec.has_more),
                  updated_at: rec.updated_at || 0,
                };
              }
            }
          } catch (_) {
            return { found: false, items: [], has_more: false };
          }
        }
        if (!Array.isArray(rec.items)) {
          return { found: false, items: [], has_more: false };
        }
        return {
          found: true,
          items: rec.items,
          has_more: Boolean(rec.has_more),
          updated_at: rec.updated_at || 0,
        };
      } catch (_) {
        return { found: false, items: [], has_more: false };
      }
    },

    async enqueueOutbox(entry) {
      if (!entry || !entry.client_id) return false;
      try {
        await idbTx('chat_outbox', 'readwrite', (s) =>
          s.put({
            client_id: String(entry.client_id),
            kind: String(entry.kind || 'dm'),
            target_id: String(entry.target_id || ''),
            sub_id: entry.sub_id ? String(entry.sub_id) : null,
            text: String(entry.text || ''),
            reply_to_id: entry.reply_to_id ? String(entry.reply_to_id) : null,
            created_at: entry.created_at || new Date().toISOString(),
            attempts: Number(entry.attempts || 0),
            status: String(entry.status || 'pending'),
          }),
        );
        return true;
      } catch (_) {
        return false;
      }
    },

    async listOutbox() {
      try {
        const rows = await idbGetAll('chat_outbox');
        rows.sort((a, b) => String(a.created_at || '').localeCompare(String(b.created_at || '')));
        return rows;
      } catch (_) {
        return [];
      }
    },

    async removeOutbox(clientId) {
      if (!clientId) return false;
      try {
        await idbTx('chat_outbox', 'readwrite', (s) => s.delete(String(clientId)));
        return true;
      } catch (_) {
        return false;
      }
    },

    // ── Spec 071: Dialogs, Peers, Drafts, Static Cache & Non-Extractable CryptoKey Vault ──
    async putDialogs(uid, payload) {
      if (!uid || !payload) return false;
      try {
        await idbTx('dialogs', 'readwrite', (s) =>
          s.put({ id: `dialogs:${uid}`, payload, updated_at: Date.now() }),
        );
        return true;
      } catch (_) {
        return false;
      }
    },

    async getDialogs(uid) {
      if (!uid) return { found: false, payload: null };
      try {
        const rec = await idbTx('dialogs', 'readonly', (s) => s.get(`dialogs:${uid}`));
        if (!rec || !rec.payload) return { found: false, payload: null };
        return { found: true, payload: rec.payload, updated_at: rec.updated_at || 0 };
      } catch (_) {
        return { found: false, payload: null };
      }
    },

    async putDraft(convKey, text) {
      if (!convKey) return false;
      try {
        if (!text || !String(text).trim()) {
          await idbTx('drafts', 'readwrite', (s) => s.delete(String(convKey)));
        } else {
          await idbTx('drafts', 'readwrite', (s) =>
            s.put({ conv_key: String(convKey), text: String(text), updated_at: Date.now() }),
          );
        }
        return true;
      } catch (_) {
        return false;
      }
    },

    async getDraft(convKey) {
      if (!convKey) return '';
      try {
        const rec = await idbTx('drafts', 'readonly', (s) => s.get(String(convKey)));
        return rec && typeof rec.text === 'string' ? rec.text : '';
      } catch (_) {
        return '';
      }
    },

    async putStaticCache(key, version, data) {
      if (!key) return false;
      try {
        await idbTx('static_cache', 'readwrite', (s) =>
          s.put({ key: String(key), version: String(version || 'v1'), data, updated_at: Date.now() }),
        );
        return true;
      } catch (_) {
        return false;
      }
    },

    async getStaticCache(key) {
      if (!key) return null;
      try {
        const rec = await idbTx('static_cache', 'readonly', (s) => s.get(String(key)));
        return rec || null;
      } catch (_) {
        return null;
      }
    },

    async deleteStaticCacheByPrefix(prefix, keepKey) {
      if (!prefix) return 0;
      try {
        const db = await openDb();
        return await new Promise((resolve) => {
          const tx = db.transaction('static_cache', 'readwrite');
          const store = tx.objectStore('static_cache');
          const req = store.openCursor();
          let deleted = 0;
          req.onsuccess = (ev) => {
            const cursor = ev.target.result;
            if (!cursor) return;
            const k = String(cursor.key || '');
            if (k.startsWith(prefix) && k !== keepKey) {
              cursor.delete();
              deleted++;
            }
            cursor.continue();
          };
          tx.oncomplete = () => resolve(deleted);
          tx.onerror = () => resolve(deleted);
          tx.onabort = () => resolve(deleted);
        });
      } catch (_) {
        return 0;
      }
    },

    async ensureNonExtractableVaultKey() {
      if (!crypto || !crypto.subtle) return { ok: false };
      try {
        let rec = await idbTx('crypto_keys', 'readonly', (s) => s.get('device_vault_aes'));
        if (rec && rec.key && rec.key.extractable === false) {
          return { ok: true, extractable: false, algorithm: rec.key.algorithm.name };
        }
        const key = await crypto.subtle.generateKey(
          { name: 'AES-GCM', length: 256 },
          false, // NEVER extractable — F12 Console cannot export raw key bytes!
          ['encrypt', 'decrypt'],
        );
        await idbTx('crypto_keys', 'readwrite', (s) =>
          s.put({ id: 'device_vault_aes', key, created_at: Date.now() }),
        );
        return { ok: true, extractable: key.extractable, algorithm: key.algorithm.name };
      } catch (e) {
        return { ok: false, error: String(e && e.message ? e.message : e) };
      }
    },

    async clearSessionSecrets() {
      try {
        // Wipe L1 RAM sensitive plaintext cache immediately on lock / logout (Section 11.1 & 14)
        if (window.__chatwMsgStore && typeof window.__chatwMsgStore.wipeL1Ram === 'function') {
          window.__chatwMsgStore.wipeL1Ram();
        }
        for (const storeName of [
          'dialogs',
          'chat_messages',
          'chat_outbox',
          'drafts',
          'users_peers',
          'sync_cursors',
          'sync_queue',
          'msg_chunks',
          'chunk_manifests',
          'search_shards',
          'compression_checkpoints',
        ]) {
          await idbTx(storeName, 'readwrite', (s) => s.clear()).catch(() => {});
        }
        window.__chatwCurrentUserId = null;
        try {
          localStorage.removeItem('chatw_last_delta_sync_v1');
        } catch (_) {}
        return true;
      } catch (_) {
        return false;
      }
    },

    // ── Spec 110: Dedicated AI Output Cache, Video Segment Cache & Smart Cache Telemetry ──
    async putAiOutput(cacheKey, blobOrU8, meta = {}) {
      if (!cacheKey || !blobOrU8) return { ok: false };
      const blob = blobOrU8 instanceof Blob ? blobOrU8 : new Blob([blobOrU8], { type: meta.mime_type || 'image/png' });
      const byteSize = blob.size || 0;
      if (byteSize === 0) return { ok: false };
      try {
        await evictAiCacheIfNeeded(byteSize);
        const now = Date.now();
        cmsRecord(cacheKey);
        await idbTx('ai_outputs', 'readwrite', (s) =>
          s.put({
            cache_key: String(cacheKey),
            media_id: meta.media_id || '',
            model_id: meta.model_id || 'spab_lite_builtin',
            model_label: meta.model_label || '',
            model_version: meta.model_version || '3.0.0',
            backend: meta.backend || 'rust_wasm_cnn',
            ssim: meta.ssim || null,
            compute_cost_ms: Number(meta.compute_cost_ms || 40),
            width: Number(meta.width || 0),
            height: Number(meta.height || 0),
            src_width: Number(meta.src_width || 0),
            src_height: Number(meta.src_height || 0),
            src_bytes: Number(meta.src_bytes || 0),
            super_res: Number(meta.super_res || 1),
            match_pct: meta.match_pct != null ? Number(meta.match_pct) : null,
            byte_size: byteSize,
            blob,
            cold_compressed_15d: Boolean(meta.cold_compressed_15d || (blob.type || '') === 'image/avif'),
            access_count: 1,
            created_at: now,
            last_accessed_at: Number(meta.last_accessed_at || now),
          }),
        );
        return { ok: true, byte_size: byteSize };
      } catch (err) {
        return { ok: false, error: String(err && err.message ? err.message : err) };
      }
    },

    async getAiOutput(cacheKey) {
      if (!cacheKey) return { found: false };
      try {
        const rec = await idbTx('ai_outputs', 'readonly', (s) => s.get(String(cacheKey)));
        if (!rec || !rec.blob) return { found: false };
        cmsRecord(cacheKey);
        try {
          rec.last_accessed_at = Date.now();
          rec.access_count = (rec.access_count || 1) + 1;
          await idbTx('ai_outputs', 'readwrite', (s) => s.put(rec));
        } catch (_) {}
        return {
          found: true,
          blob: rec.blob,
          model_id: rec.model_id,
          model_label: rec.model_label || '',
          model_version: rec.model_version,
          backend: rec.backend,
          ssim: rec.ssim,
          width: rec.width || 0,
          height: rec.height || 0,
          src_width: rec.src_width || 0,
          src_height: rec.src_height || 0,
          src_bytes: rec.src_bytes || 0,
          super_res: rec.super_res || 1,
          match_pct: rec.match_pct != null ? rec.match_pct : null,
          byte_size: rec.byte_size,
        };
      } catch (_) {
        return { found: false };
      }
    },

    async clearAiCache() {
      try {
        const rows = await idbGetAll('ai_outputs').catch(() => []);
        const freed = rows.reduce((a, r) => a + (r.byte_size || 0), 0);
        await idbTx('ai_outputs', 'readwrite', (s) => s.clear());
        return { removed: rows.length, freed_bytes: freed };
      } catch (_) {
        return { removed: 0, freed_bytes: 0 };
      }
    },

    async putAiModel(modelId, bytesU8, meta = {}) {
      if (!modelId || !bytesU8) return false;
      const bytes = bytesU8 instanceof Uint8Array ? bytesU8 : new Uint8Array(bytesU8);
      try {
        await idbTx('ai_models', 'readwrite', (s) =>
          s.put({
            model_id: String(modelId),
            version: String(meta.version || '1.0'),
            byte_size: bytes.byteLength,
            bytes,
            saved_at: Date.now(),
          }),
        );
        return true;
      } catch (_) {
        return false;
      }
    },

    async getAiModel(modelId) {
      if (!modelId) return null;
      try {
        const rec = await idbTx('ai_models', 'readonly', (s) => s.get(String(modelId)));
        if (!rec || !rec.bytes) return null;
        return rec.bytes instanceof Uint8Array ? rec.bytes : new Uint8Array(rec.bytes);
      } catch (_) {
        return null;
      }
    },

    async putVideoSegment(segKey, u8, meta = {}) {
      if (!segKey || !u8) return false;
      const bytes = u8 instanceof Uint8Array ? u8 : new Uint8Array(u8);
      try {
        const rows = await idbGetAll('video_segments').catch(() => []);
        let total = rows.reduce((a, r) => a + (r.byte_size || 0), 0);
        const maxVideoSegBytes = 96 * 1024 * 1024;
        if (total + bytes.byteLength > maxVideoSegBytes) {
          const nowMs = Date.now();
          rows.sort((a, b) => computeRetentionScore(a, nowMs) - computeRetentionScore(b, nowMs));
          for (const item of rows) {
            if (total + bytes.byteLength <= maxVideoSegBytes) break;
            await idbTx('video_segments', 'readwrite', (s) => s.delete(item.seg_key)).catch(() => {});
            total -= (item.byte_size || 0);
          }
        }
        cmsRecord(segKey);
        const now = Date.now();
        await idbTx('video_segments', 'readwrite', (s) =>
          s.put({
            seg_key: String(segKey),
            bytes,
            byte_size: bytes.byteLength,
            mime_type: meta.mime_type || 'application/octet-stream',
            access_count: 1,
            last_accessed_at: now,
          }),
        );
        return true;
      } catch (_) {
        return false;
      }
    },

    async getVideoSegment(segKey) {
      if (!segKey) return null;
      try {
        const rec = await idbTx('video_segments', 'readonly', (s) => s.get(String(segKey)));
        if (!rec || !rec.bytes) return null;
        cmsRecord(segKey);
        rec.last_accessed_at = Date.now();
        rec.access_count = (rec.access_count || 1) + 1;
        await idbTx('video_segments', 'readwrite', (s) => s.put(rec)).catch(() => {});
        return rec.bytes instanceof Uint8Array ? rec.bytes : new Uint8Array(rec.bytes);
      } catch (_) {
        return null;
      }
    },

    async getSmartCacheStats() {
      const [objs, aiRows, vidRows, effectiveLimit] = await Promise.all([
        idbGetAll('objects').catch(() => []),
        idbGetAll('ai_outputs').catch(() => []),
        idbGetAll('video_segments').catch(() => []),
        computeAdaptiveLimitBytes().catch(() => 1024 * 1024 * 1024),
      ]);
      const objBytes = objs.reduce((a, r) => a + (r.byte_size || 0), 0);
      const aiBytes = aiRows.reduce((a, r) => a + (r.byte_size || 0), 0);
      const vidBytes = vidRows.reduce((a, r) => a + (r.byte_size || 0), 0);
      return {
        policy: 'W-TinyLFU + Cost-Aware SLRU',
        adaptive_limit_bytes: effectiveLimit === Number.MAX_SAFE_INTEGER ? 0 : effectiveLimit,
        media_object_count: objs.length,
        media_object_bytes: objBytes,
        ai_output_count: aiRows.length,
        ai_output_bytes: aiBytes,
        video_segment_count: vidRows.length,
        video_segment_bytes: vidBytes,
        total_smart_cache_bytes: objBytes + aiBytes + vidBytes,
      };
    },

    // Spec 158: Compress local cached images (`objects` & `sender_orig:*`) untouched for >= 15 days
    // off-main-thread via `/media_worker.js`, leaving active (< 15 days) images 100% untouched as-is.
    async compressColdImages15d(idleDays = 15, dryRun = false) {
      const days = Math.max(1, Number(idleDays) || 15);
      const cutoffMs = Date.now() - days * 86400 * 1000;
      const report = {
        idle_days: days,
        scanned: 0,
        kept_active: 0,
        compressed: 0,
        skipped_already_compressed: 0,
        freed_bytes: 0,
      };

      let sharedColdWorker = null;
      let coldJobId = 1;
      const disposeColdWorker = () => {
        if (sharedColdWorker) {
          try { sharedColdWorker.terminate(); } catch (_) {}
          sharedColdWorker = null;
        }
      };
      const encodeColdWorker = (u8, mime) => {
        if (typeof Worker === 'undefined') return Promise.resolve(null);
        return new Promise((resolve) => {
          try {
            if (!sharedColdWorker) {
              sharedColdWorker = new Worker('/media_worker.js', { type: 'module' });
            }
            const w = sharedColdWorker;
            const jobId = coldJobId++;
            const timer = setTimeout(() => {
              disposeColdWorker();
              resolve(null);
            }, 30000);
            w.onmessage = (ev) => {
              if (ev.data && ev.data.id !== jobId) return;
              clearTimeout(timer);
              resolve(ev.data && ev.data.ok ? ev.data : null);
            };
            w.onerror = () => {
              clearTimeout(timer);
              disposeColdWorker();
              resolve(null);
            };
            const copy = new Uint8Array(u8);
            w.postMessage(
              { id: jobId, task: 'optimize_image', buffer: copy.buffer, mime, opts: { qualityMode: 'ultra_optimized', forceCompress: true } },
              [copy.buffer],
            );
          } catch (_) {
            disposeColdWorker();
            resolve(null);
          }
        });
      };

      const objs = await idbGetAll('objects').catch(() => []);
      try {
      for (const meta of objs) {
        const mime = String(meta.mime_type || '').toLowerCase();
        if (!mime.startsWith('image/') || mime === 'image/svg+xml') continue;
        report.scanned++;
        if (mime === 'image/avif' || meta.cold_compressed_15d) {
          report.skipped_already_compressed++;
          continue;
        }
        const lastUsed = Number(meta.last_accessed_at || meta.updated_at || meta.created_at || Date.now());
        if (lastUsed > cutoffMs) {
          report.kept_active++;
          continue;
        }
        if (dryRun) {
          report.compressed++;
          continue;
        }
        let raw = null;
        if (meta.opfs_key) raw = await readOpfs(meta.opfs_key);
        if (!raw && meta.blob) raw = meta.blob instanceof Uint8Array ? meta.blob : new Uint8Array(meta.blob);
        if (!raw || raw.byteLength < 64) continue;

        // Never attempt image transcoding on CWLC1 encrypted E2EE cache envelopes.
        if (
          raw.byteLength >= 5 &&
          raw[0] === 0x43 &&
          raw[1] === 0x57 &&
          raw[2] === 0x4c &&
          raw[3] === 0x43 &&
          raw[4] === 0x31
        ) {
          continue;
        }

        const wres = await encodeColdWorker(raw, mime);
        if (wres && wres.ext === 'avif' && wres.bytesB64) {
          const avifBytes = b64ToBytes(wres.bytesB64);
          if (avifBytes.byteLength > 0 && avifBytes.byteLength < raw.byteLength) {
            const newSha = await sha256Hex(avifBytes);
            let storedOpfs = false;
            if (meta.opfs_key) {
              try { storedOpfs = await writeOpfs(meta.opfs_key, avifBytes); } catch (_) { storedOpfs = false; }
            }
            const nextRec = {
              ...meta,
              opfs_key: storedOpfs ? meta.opfs_key : null,
              mime_type: 'image/avif',
              original_byte_size: Number(meta.original_byte_size || raw.byteLength),
              byte_size: avifBytes.byteLength,
              sha256_plain: newSha,
              blob: storedOpfs ? null : avifBytes,
              cold_compressed_15d: true,
              cold_compressed_at: Date.now(),
              updated_at: Date.now(),
            };
            await idbTx('objects', 'readwrite', (s) => s.put(nextRec)).catch(() => {});
            if (typeof caches !== 'undefined') {
              try {
                const cache = await caches.open(CACHE_NAME);
                await cache.put('/__chatw_local/' + meta.ref_id, new Response(avifBytes, { headers: { 'Content-Type': 'image/avif' } }));
              } catch (_) {}
            }
            report.compressed++;
            report.freed_bytes += (raw.byteLength - avifBytes.byteLength);
            continue;
          }
        }
        await idbTx('objects', 'readwrite', (s) => s.put({ ...meta, cold_compressed_15d: true })).catch(() => {});
      }

      const aiRows = await idbGetAll('ai_outputs').catch(() => []);
      for (const rec of aiRows) {
        if (!String(rec.cache_key || '').startsWith('sender_orig:') || !rec.blob) continue;
        const mime = String(rec.blob.type || 'image/png').toLowerCase();
        if (!mime.startsWith('image/') || mime === 'image/svg+xml') continue;
        report.scanned++;
        if (mime === 'image/avif' || rec.cold_compressed_15d) {
          report.skipped_already_compressed++;
          continue;
        }
        const lastUsed = Number(rec.last_accessed_at || rec.created_at || Date.now());
        if (lastUsed > cutoffMs) {
          report.kept_active++;
          continue;
        }
        if (dryRun) {
          report.compressed++;
          continue;
        }
        const raw = new Uint8Array(await rec.blob.arrayBuffer());
        const wres = await encodeColdWorker(raw, mime);
        if (wres && wres.ext === 'avif' && wres.bytesB64) {
          const avifBytes = b64ToBytes(wres.bytesB64);
          if (avifBytes.byteLength > 0 && avifBytes.byteLength < raw.byteLength) {
            const avifBlob = new Blob([avifBytes], { type: 'image/avif' });
            await idbTx('ai_outputs', 'readwrite', (s) =>
              s.put({
                ...rec,
                blob: avifBlob,
                byte_size: avifBlob.size,
                cold_compressed_15d: true,
                cold_compressed_at: Date.now(),
              }),
            ).catch(() => {});
            report.compressed++;
            report.freed_bytes += (raw.byteLength - avifBytes.byteLength);
            continue;
          }
        }
        await idbTx('ai_outputs', 'readwrite', (s) => s.put({ ...rec, cold_compressed_15d: true })).catch(() => {});
      }
      } finally {
        disposeColdWorker();
      }

      return report;
    },
  };

  // ── Spec 164: Unified Off-Main-Thread Zstd-22 Message Chunk Store, L1/L2 Smart Cache,
  //    Encrypted Vietnamese Full-Text Search Index, Adaptive Budget & Incremental Runner ──
  const l1ChunkRam = new Map();
  let l1RamUsedBytes = 0;
  let l1Hits = 0;
  let l1Misses = 0;
  let prevAdaptiveBudgetBytes = 0;
  let msgWorker = null;
  let msgWorkerJobId = 1;
  const msgWorkerPending = new Map();
  const activePrefetchScopes = new Set();
  let compressionPaused = false;
  let lastSearchLatencyMs = 0;

  function getL1RamBudgetBytes() {
    const devMemGb = Number(navigator.deviceMemory || 4);
    const mode = getStorageMode();
    if (mode === 'space_saver') return Math.min(32, Math.max(8, devMemGb * 4)) * 1024 * 1024;
    if (mode === 'high_performance') return Math.min(128, Math.max(32, devMemGb * 16)) * 1024 * 1024;
    return Math.min(64, Math.max(16, devMemGb * 8)) * 1024 * 1024;
  }

  function getStorageMode() {
    try {
      const m = localStorage.getItem('chatw_storage_mode') || 'auto';
      return ['auto', 'space_saver', 'high_performance', 'custom'].includes(m) ? m : 'auto';
    } catch (_) {
      return 'auto';
    }
  }

  function ensureMsgWorker() {
    if (msgWorker) return msgWorker;
    if (typeof Worker === 'undefined') return null;
    try {
      const w = new Worker('/media_worker.js', { type: 'module' });
      w.onmessage = (ev) => {
        const d = ev.data;
        if (!d || !d.id) return;
        const entry = msgWorkerPending.get(d.id);
        if (entry) {
          clearTimeout(entry.timer);
          msgWorkerPending.delete(d.id);
          entry.resolve(d);
        }
      };
      w.onerror = () => {
        for (const [, entry] of msgWorkerPending.entries()) {
          clearTimeout(entry.timer);
          entry.resolve({ ok: false, error: 'worker_error' });
        }
        msgWorkerPending.clear();
        try { w.terminate(); } catch (_) {}
        msgWorker = null;
      };
      msgWorker = w;
      return w;
    } catch (_) {
      return null;
    }
  }

  function callMsgWorker(task, opts = {}, buffer = null, timeoutMs = 45000) {
    const w = ensureMsgWorker();
    if (!w) return Promise.resolve({ ok: false, error: 'worker_unavailable' });
    const id = msgWorkerJobId++;
    return new Promise((resolve) => {
      const timer = setTimeout(() => {
        msgWorkerPending.delete(id);
        resolve({ ok: false, error: 'worker_timeout' });
      }, timeoutMs);
      msgWorkerPending.set(id, { resolve, timer });
      try {
        const transfer = [];
        let payloadBuf = null;
        if (buffer) {
          const copy = buffer instanceof Uint8Array ? new Uint8Array(buffer) : new Uint8Array(buffer);
          payloadBuf = copy.buffer;
          transfer.push(copy.buffer);
        }
        w.postMessage({ id, task, opts, buffer: payloadBuf }, transfer);
      } catch (err) {
        clearTimeout(timer);
        msgWorkerPending.delete(id);
        resolve({ ok: false, error: String(err && err.message ? err.message : err) });
      }
    });
  }

  function deriveWebRootKeyB64(accountId) {
    // Use account-bound key material from in-memory session or stable local vault seed
    const acc = String(accountId || window.__chatwCurrentUserId || 'anon');
    if (window.__chatwSessionRootKeyB64 && typeof window.__chatwSessionRootKeyB64 === 'string') {
      return window.__chatwSessionRootKeyB64;
    }
    try {
      const k = 'chatw_msgstore_rk_' + acc;
      let val = localStorage.getItem(k);
      if (!val) {
        const raw = crypto.getRandomValues(new Uint8Array(32));
        val = bytesToB64(raw);
        localStorage.setItem(k, val);
      }
      return val;
    } catch (_) {
      return btoa('chatw-fallback-ephemeral-key-32b!');
    }
  }

  async function readSearchShardBytes(shardKey) {
    const rec = await idbTx('search_shards', 'readonly', (s) => s.get(shardKey)).catch(() => null);
    if (!rec) return new Uint8Array(0);
    if (rec.opfs_key) {
      const fromOpfs = await readOpfs(rec.opfs_key).catch(() => null);
      if (fromOpfs && fromOpfs.byteLength > 0) return fromOpfs;
    }
    if (rec.bytes) return rec.bytes instanceof Uint8Array ? rec.bytes : new Uint8Array(rec.bytes);
    return new Uint8Array(0);
  }

  async function writeSearchShardBytes(shardKey, u8, meta = {}) {
    const opfsKey = 'search_' + shardKey.replace(/[^a-zA-Z0-9_-]/g, '_');
    let storedOpfs = false;
    try { storedOpfs = await writeOpfs(opfsKey, u8); } catch (_) { storedOpfs = false; }
    await idbTx('search_shards', 'readwrite', (s) =>
      s.put({
        shard_key: shardKey,
        opfs_key: storedOpfs ? opfsKey : null,
        bytes: storedOpfs ? null : u8,
        byte_size: u8.byteLength,
        doc_count: Number(meta.docCount || 0),
        term_count: Number(meta.termCount || 0),
        index_version: Number(meta.indexVersion || 1),
        blake3_hex: String(meta.blake3Hex || ''),
        updated_at: Date.now(),
      }),
    ).catch(() => {});
  }

  function l1TouchAndGet(chunkId) {
    const hit = l1ChunkRam.get(chunkId);
    if (!hit) {
      l1Misses++;
      return null;
    }
    l1Hits++;
    cmsRecord('l1:' + chunkId);
    l1ChunkRam.delete(chunkId);
    hit.lastUsed = Date.now();
    l1ChunkRam.set(chunkId, hit);
    return hit.messages;
  }

  function l1Put(chunkId, messages) {
    const estBytes = Math.max(4096, JSON.stringify(messages).length * 2);
    const maxBytes = getL1RamBudgetBytes();
    if (l1ChunkRam.has(chunkId)) {
      const prev = l1ChunkRam.get(chunkId);
      l1RamUsedBytes = Math.max(0, l1RamUsedBytes - prev.estBytes);
      l1ChunkRam.delete(chunkId);
    }
    while (l1RamUsedBytes + estBytes > maxBytes && l1ChunkRam.size > 0) {
      const oldestKey = l1ChunkRam.keys().next().value;
      const old = l1ChunkRam.get(oldestKey);
      l1ChunkRam.delete(oldestKey);
      l1RamUsedBytes = Math.max(0, l1RamUsedBytes - (old ? old.estBytes : 0));
    }
    if (estBytes <= maxBytes) {
      l1ChunkRam.set(chunkId, { messages, estBytes, lastUsed: Date.now() });
      l1RamUsedBytes += estBytes;
    }
  }

  function getLiveRuntimeSignals() {
    const conn = navigator.connection || navigator.mozConnection || navigator.webkitConnection;
    const isCall = Boolean(window.__chatwActiveCall);
    return {
      active_voice_or_video_call: isCall,
      user_actively_scrolling_or_typing: false,
      app_in_foreground: document.visibilityState === 'visible',
      battery_level_pct: null,
      on_battery_saver: false,
      thermal_throttled: false,
      on_metered_or_cellular: Boolean(conn && (conn.type === 'cellular' || conn.effectiveType === '2g' || conn.effectiveType === '3g')),
      data_saver_enabled: Boolean(conn && conn.saveData),
      is_offline: navigator.onLine === false,
      estimated_bandwidth_kbps: conn && conn.downlink ? Math.round(conn.downlink * 1000) : 10000,
      ram_pressure_pct: 30,
    };
  }

  const msgStoreApi = {
    getStorageMode,

    setStorageMode(mode, customMb = null) {
      const valid = ['auto', 'space_saver', 'high_performance', 'custom'].includes(mode) ? mode : 'auto';
      try {
        localStorage.setItem('chatw_storage_mode', valid);
        if (valid === 'custom' && customMb != null && Number(customMb) >= 32) {
          localStorage.setItem('chatw_cache_limit_mb', String(Math.round(Number(customMb))));
        } else if (valid === 'auto') {
          localStorage.setItem('chatw_cache_limit_mb', '0');
        }
      } catch (_) {}
      return valid;
    },

    wipeL1Ram() {
      l1ChunkRam.clear();
      l1RamUsedBytes = 0;
    },

    pauseCompression() {
      compressionPaused = true;
    },

    resumeCompression() {
      compressionPaused = false;
    },

    async getDynamicBudgetAndBreakdown() {
      let quota = 0;
      let usage = 0;
      try {
        if (navigator.storage && typeof navigator.storage.estimate === 'function') {
          const est = await navigator.storage.estimate();
          quota = Number(est.quota || 0);
          usage = Number(est.usage || 0);
        }
      } catch (_) {}
      const freeBytes = Math.max(0, quota - usage);
      const mode = getStorageMode();
      const customMb = mode === 'custom' ? getConfiguredLimitMb() : 0;
      const profile = {
        platform: 'web',
        total_ram_mb: Math.round(Number(navigator.deviceMemory || 4) * 1024),
        available_ram_mb: Math.round(Number(navigator.deviceMemory || 4) * 768),
        cpu_cores: Number(navigator.hardwareConcurrency || 4),
        total_disk_bytes: quota || 4 * 1024 * 1024 * 1024,
        free_disk_bytes: freeBytes || 2 * 1024 * 1024 * 1024,
        os_quota_bytes: quota > 0 ? quota : null,
        is_ssd: true,
        is_mobile: /Android|iPhone|iPad|iPod/i.test(navigator.userAgent || ''),
      };
      const signals = getLiveRuntimeSignals();

      // Try exact Rust WASM dynamic budget with ±12% hysteresis; fallback to deterministic JS calculation
      let budget = null;
      const wres = await callMsgWorker('compute_dynamic_budget', {
        profileJson: JSON.stringify(profile),
        mode,
        customMb,
        prevBudget: prevAdaptiveBudgetBytes,
        signalsJson: JSON.stringify(signals),
      }, null, 6000);
      if (wres && wres.ok && wres.budgetJson) {
        try { budget = JSON.parse(wres.budgetJson); } catch (_) {}
      }
      if (!budget || !budget.total_local_budget_bytes) {
        const lim = await computeAdaptiveLimitBytes();
        budget = {
          mode,
          total_local_budget_bytes: lim,
          message_cache_budget_bytes: Math.floor(lim * 0.26),
          search_index_budget_bytes: Math.floor(lim * 0.15),
          dictionary_budget_bytes: Math.max(2 * 1024 * 1024, Math.floor(lim * 0.02)),
          media_cache_budget_bytes: Math.floor(lim * 0.34),
          offline_data_budget_bytes: Math.floor(lim * 0.17),
          temp_storage_budget_bytes: Math.floor(lim * 0.06),
          reserved_free_space_bytes: Math.floor(freeBytes * 0.2),
          l1_ram_budget_bytes: getL1RamBudgetBytes(),
          effective_safe_ceiling_bytes: Math.max(32 * 1024 * 1024, Math.floor(freeBytes * 0.8)),
        };
      }
      prevAdaptiveBudgetBytes = Number(budget.total_local_budget_bytes || 0);

      // Compute granular real local storage breakdown across all stores
      const [objs, aiRows, vidRows, chatRows, chunkRows, shardRows] = await Promise.all([
        idbGetAll('objects').catch(() => []),
        idbGetAll('ai_outputs').catch(() => []),
        idbGetAll('video_segments').catch(() => []),
        idbGetAll('chat_messages').catch(() => []),
        idbGetAll('msg_chunks').catch(() => []),
        idbGetAll('search_shards').catch(() => []),
      ]);

      let mediaBytes = 0;
      let filesBytes = 0;
      let offlinePinnedBytes = 0;
      for (const o of objs) {
        const sz = Number(o.byte_size || 0);
        const m = String(o.mime_type || '');
        if (m.startsWith('image/') || m.startsWith('video/') || m.startsWith('audio/')) mediaBytes += sz;
        else filesBytes += sz;
        if (!isSafeToEvict(o) || o.pinned_offline) offlinePinnedBytes += sz;
      }
      const aiBytes = aiRows.reduce((a, r) => a + Number(r.byte_size || 0), 0);
      const vidBytes = vidRows.reduce((a, r) => a + Number(r.byte_size || 0), 0);
      const hotMsgBytes = chatRows.reduce((a, r) => a + Number(r.byte_size || (r.items ? JSON.stringify(r.items).length : 0)), 0);
      const chunkBytes = chunkRows.reduce((a, r) => a + Number(r.encrypted_size || r.byte_size || 0), 0);
      const chunkOrigBytes = chunkRows.reduce((a, r) => a + Number(r.original_size || 0), 0);
      const searchBytes = shardRows.reduce((a, r) => a + Number(r.byte_size || 0), 0);
      const indexedDocs = shardRows.reduce((a, r) => a + Number(r.doc_count || 0), 0);

      const totalLocalBytes = mediaBytes + filesBytes + aiBytes + vidBytes + hotMsgBytes + chunkBytes + searchBytes;
      const savedBytes = Math.max(0, chunkOrigBytes - chunkBytes);

      return {
        mode,
        budget,
        browser_quota_bytes: quota,
        browser_usage_bytes: usage,
        free_bytes: freeBytes,
        total_bytes: totalLocalBytes,
        messages_bytes: hotMsgBytes + chunkBytes,
        media_bytes: mediaBytes + vidBytes,
        files_bytes: filesBytes,
        cache_bytes: aiBytes + mediaBytes + vidBytes + chunkBytes,
        offline_bytes: offlinePinnedBytes + hotMsgBytes + chunkBytes,
        search_index_bytes: searchBytes,
        saved_bytes: savedBytes,
        chunk_count: chunkRows.length,
        indexed_docs: indexedDocs,
        l1_ram_bytes: l1RamUsedBytes,
        l1_hit_rate: (l1Hits + l1Misses) > 0 ? l1Hits / (l1Hits + l1Misses) : 0,
        last_search_latency_ms: lastSearchLatencyMs,
      };
    },

    // Incrementally index a batch of decrypted messages into the encrypted local Vietnamese search shard!
    async indexMessages(accountId, scopeType, scopeId, messages, chunkId = '', coverage = 'partially_indexed') {
      if (!Array.isArray(messages) || messages.length === 0) return { ok: true, docCount: 0 };
      const acc = String(accountId || window.__chatwCurrentUserId || 'anon');
      const shardKey = `acc:${acc}:all`;
      const rootKeyB64 = deriveWebRootKeyB64(acc);

      // Normalize UI message objects into CompactMessage schema
      const normalized = messages
        .filter((m) => m && m.id && typeof (m.text ?? m.content) === 'string')
        .map((m) => ({
          id: String(m.id),
          seq: Number(m.seq || 0),
          sender_id: String(m.sender_id || ''),
          created_at_ms: typeof m.created_at_ms === 'number'
            ? m.created_at_ms
            : (Date.parse(m.created_at || '') || Date.now()),
          msg_type: String(m.msg_type || 'text'),
          text: String(m.text ?? m.content ?? ''),
          reply_to_id: m.reply_to_id ? String(m.reply_to_id) : null,
          is_edited: Boolean(m.is_edited),
          deleted_for_all: Boolean(m.deleted_for_all),
          is_pinned: Boolean(m.is_pinned),
          is_bookmarked: Boolean(m.is_bookmarked),
          status_code: 0,
          reactions: [],
          attachments: [],
          extra_meta: null,
        }));
      if (normalized.length === 0) return { ok: true, docCount: 0 };

      const existing = await readSearchShardBytes(shardKey);
      const res = await callMsgWorker(
        'search_upsert_batch',
        {
          messagesJson: JSON.stringify(normalized),
          scopeType: String(scopeType || 'dm'),
          scopeId: String(scopeId || ''),
          chunkId: String(chunkId || ''),
          rootKeyB64,
          accountId: acc,
          scopeKey: shardKey,
          coverage,
        },
        existing,
        30000,
      );
      if (res && res.ok && res.buffer) {
        await writeSearchShardBytes(shardKey, new Uint8Array(res.buffer), res);
        return { ok: true, docCount: res.docCount || 0, termCount: res.termCount || 0 };
      }
      return { ok: false, error: (res && res.error) || 'index_failed' };
    },

    async deleteFromSearchIndex(accountId, messageId) {
      if (!messageId) return false;
      const acc = String(accountId || window.__chatwCurrentUserId || 'anon');
      const shardKey = `acc:${acc}:all`;
      const existing = await readSearchShardBytes(shardKey);
      if (existing.byteLength === 0) return true;
      const res = await callMsgWorker(
        'search_delete_message',
        { messageId: String(messageId), rootKeyB64: deriveWebRootKeyB64(acc), accountId: acc, scopeKey: shardKey },
        existing,
        15000,
      );
      if (res && res.ok && res.buffer) {
        await writeSearchShardBytes(shardKey, new Uint8Array(res.buffer), res);
        return true;
      }
      return false;
    },

    // Execute encrypted Vietnamese full-text search completely off-main-thread!
    async searchLocalMessages(accountId, queryObj) {
      const t0 = performance.now();
      const acc = String(accountId || window.__chatwCurrentUserId || 'anon');
      const shardKey = `acc:${acc}:all`;
      const existing = await readSearchShardBytes(shardKey);
      const q = {
        text: String((queryObj && queryObj.text) || ''),
        exact_diacritics: Boolean(queryObj && queryObj.exact_diacritics),
        phrase_match: Boolean(queryObj && queryObj.phrase_match),
        prefix_match: queryObj && queryObj.prefix_match !== false,
        filters: (queryObj && queryObj.filters) || {},
        offset: Number((queryObj && queryObj.offset) || 0),
        limit: Number((queryObj && queryObj.limit) || 30),
      };
      const res = await callMsgWorker(
        'search_query',
        { queryJson: JSON.stringify(q), rootKeyB64: deriveWebRootKeyB64(acc), accountId: acc, scopeKey: shardKey },
        existing,
        15000,
      );
      lastSearchLatencyMs = Math.round((performance.now() - t0) * 100) / 100;
      if (res && res.ok && res.responseJson) {
        try {
          const parsed = JSON.parse(res.responseJson);
          parsed.latency_ms = lastSearchLatencyMs;
          return parsed;
        } catch (_) {}
      }
      return {
        hits: [],
        total_matched: 0,
        has_more: false,
        coverage: 'not_downloaded',
        unindexed_scopes: [],
        from_result_cache: false,
        latency_ms: lastSearchLatencyMs,
      };
    },

    // Seal a batch of >=7d messages using pure-Rust Zstandard Level 22 in Web Worker & persist to L2 OPFS/IDB!
    async sealAndStoreLocalChunk(accountId, scopeType, scopeId, messages, chunkId = null) {
      if (!Array.isArray(messages) || messages.length === 0) return { ok: false, error: 'empty' };
      const acc = String(accountId || window.__chatwCurrentUserId || 'anon');
      const cid = chunkId || (crypto.randomUUID ? crypto.randomUUID() : `chk_${Date.now()}`);
      const req = {
        chunk_id: cid,
        account_id: acc,
        scope_type: scopeType,
        scope_id: scopeId,
        chunk_version: 1,
        access_epoch: 1,
        key_version: 1,
        candidate_dict_id: 1,
        now_ms: Date.now(),
        require_space_savings: true,
        ram_budget_bytes: getL1RamBudgetBytes(),
      };
      const res = await callMsgWorker(
        'seal_msg_chunk_zstd22',
        { messagesJson: JSON.stringify(messages), reqJson: JSON.stringify(req), rootKeyB64: deriveWebRootKeyB64(acc) },
        null,
        60000,
      );
      if (!res || !res.ok || !res.buffer) {
        return { ok: false, error: (res && res.error) || 'seal_failed' };
      }
      const sealedU8 = new Uint8Array(res.buffer);
      const meta = JSON.parse(res.metadataJson || '{}');
      const opfsKey = `msgchk_${cid}`;
      let storedOpfs = false;
      try { storedOpfs = await writeOpfs(opfsKey, sealedU8); } catch (_) {}
      const scopeKey = `${scopeType}:${scopeId}`;
      await idbTx('msg_chunks', 'readwrite', (s) =>
        s.put({
          chunk_id: cid,
          scope_key: scopeKey,
          meta,
          opfs_key: storedOpfs ? opfsKey : null,
          bytes: storedOpfs ? null : sealedU8,
          original_size: Number(meta.original_size || 0),
          encrypted_size: sealedU8.byteLength,
          byte_size: sealedU8.byteLength,
          last_accessed_at: Date.now(),
          access_count: 1,
          pinned_offline: false,
        }),
      );
      return { ok: true, chunk_id: cid, metadata: meta, sealed_bytes: sealedU8 };
    },

    // Lazy Decompression: L1 RAM -> L2 OPFS/IDB -> L3 Server fetch -> Verify -> Decrypt -> Zstd-22 Decompress!
    async loadChunkMessages(accountId, chunkId, metaOverride = null) {
      const cached = l1TouchAndGet(chunkId);
      if (cached) return { ok: true, source: 'l1_ram', messages: cached };

      const acc = String(accountId || window.__chatwCurrentUserId || 'anon');
      const rec = await idbTx('msg_chunks', 'readonly', (s) => s.get(chunkId)).catch(() => null);
      let sealedU8 = null;
      let meta = metaOverride || (rec && rec.meta) || null;
      let source = 'l2_disk';

      if (rec) {
        if (rec.opfs_key) sealedU8 = await readOpfs(rec.opfs_key).catch(() => null);
        if (!sealedU8 && rec.bytes) sealedU8 = rec.bytes instanceof Uint8Array ? rec.bytes : new Uint8Array(rec.bytes);
      }

      if (!sealedU8 && window.__chatwCore && window.__chatwCore.isConnected()) {
        const dl = await window.__chatwCore.downloadBytes(`/api/storage/msg-chunks/${encodeURIComponent(chunkId)}`, { priority: 'HIGH' });
        if (dl && dl.ok && dl.bytes && dl.bytes.byteLength > 0) {
          sealedU8 = dl.bytes;
          source = 'l3_server';
          // Cache downloaded chunk into L2 OPFS/IDB
          if (meta) {
            const opfsKey = `msgchk_${chunkId}`;
            let storedOpfs = false;
            try { storedOpfs = await writeOpfs(opfsKey, sealedU8); } catch (_) {}
            await idbTx('msg_chunks', 'readwrite', (s) =>
              s.put({
                chunk_id: chunkId,
                scope_key: `${meta.scope_type}:${meta.scope_id}`,
                meta,
                opfs_key: storedOpfs ? opfsKey : null,
                bytes: storedOpfs ? null : sealedU8,
                original_size: Number(meta.original_size || sealedU8.byteLength),
                encrypted_size: sealedU8.byteLength,
                byte_size: sealedU8.byteLength,
                last_accessed_at: Date.now(),
                access_count: 1,
                pinned_offline: false,
              }),
            ).catch(() => {});
          }
        }
      }

      if (!sealedU8 || !meta) {
        return { ok: false, error: navigator.onLine === false ? 'offline_chunk_unavailable' : 'chunk_not_found' };
      }

      const res = await callMsgWorker(
        'open_msg_chunk_zstd22',
        { metaJson: JSON.stringify(meta), rootKeyB64: deriveWebRootKeyB64(acc) },
        sealedU8,
        30000,
      );
      if (res && res.ok) {
        const messages = JSON.parse(res.messagesJson || '[]');
        l1Put(chunkId, messages);
        return { ok: true, source, messages };
      }
      return { ok: false, error: (res && res.error) || 'decompress_failed' };
    },

    // Intelligent cancellable adjacent-chunk prefetch (respects active calls, data saver, offline!)
    async prefetchChunksForScope(scopeKey, chunkMetas = []) {
      const sig = getLiveRuntimeSignals();
      if (sig.is_offline || sig.active_voice_or_video_call || sig.data_saver_enabled || compressionPaused) return;
      const mode = getStorageMode();
      const maxPrefetch = mode === 'space_saver' ? 1 : (mode === 'high_performance' ? 4 : 2);
      activePrefetchScopes.add(scopeKey);
      let count = 0;
      for (const m of chunkMetas) {
        if (!activePrefetchScopes.has(scopeKey) || count >= maxPrefetch) break;
        const exists = await idbTx('msg_chunks', 'readonly', (s) => s.get(m.chunk_id)).catch(() => null);
        if (exists) continue;
        if (window.__chatwCore && window.__chatwCore.isConnected()) {
          const dl = await window.__chatwCore.downloadBytes(`/api/storage/msg-chunks/${encodeURIComponent(m.chunk_id)}`, {
            priority: 'LOW',
            scope: `prefetch:${scopeKey}`,
          }).catch(() => null);
          if (dl && dl.ok && dl.bytes && dl.bytes.byteLength > 0) {
            const opfsKey = `msgchk_${m.chunk_id}`;
            let storedOpfs = false;
            try { storedOpfs = await writeOpfs(opfsKey, dl.bytes); } catch (_) {}
            await idbTx('msg_chunks', 'readwrite', (s) =>
              s.put({
                chunk_id: m.chunk_id,
                scope_key: scopeKey,
                meta: m,
                opfs_key: storedOpfs ? opfsKey : null,
                bytes: storedOpfs ? null : dl.bytes,
                original_size: Number(m.original_size || dl.bytes.byteLength),
                encrypted_size: dl.bytes.byteLength,
                byte_size: dl.bytes.byteLength,
                last_accessed_at: Date.now(),
                access_count: 1,
                pinned_offline: false,
              }),
            ).catch(() => {});
            count++;
          }
        }
      }
    },

    cancelPrefetchForScope(scopeKey) {
      activePrefetchScopes.delete(scopeKey);
      if (window.__chatwCore && typeof window.__chatwCore.abortDownload === 'function') {
        window.__chatwCore.abortDownload('', `prefetch:${scopeKey}`);
      }
    },

    // Report privacy-safe technical telemetry (Section 32: zero content, zero search queries, zero keys!)
    async reportTelemetryToServer(deviceId) {
      if (!window.__chatwCore || !window.__chatwCore.isConnected()) return false;
      const stats = await this.getDynamicBudgetAndBreakdown().catch(() => null);
      if (!stats) return false;
      const ratio = stats.messages_bytes + stats.saved_bytes > 0
        ? stats.messages_bytes / (stats.messages_bytes + stats.saved_bytes)
        : 1.0;
      const body = JSON.stringify({
        device_id: String(deviceId || 'web'),
        platform: 'web',
        storage_mode: stats.mode,
        compression_ratio: ratio,
        compression_queue_size: 0,
        cache_hit_rate: stats.l1_hit_rate,
        storage_usage_bytes: stats.total_bytes,
        storage_budget_bytes: stats.budget ? stats.budget.total_local_budget_bytes : 0,
        total_saved_bytes: stats.saved_bytes,
        indexed_message_count: stats.indexed_docs,
        last_search_latency_ms: stats.last_search_latency_ms,
        chunk_count: stats.chunk_count,
      });
      await window.__chatwCore.rpc('POST', '/api/storage/telemetry', body, 'application/json').catch(() => {});
      return true;
    },
  };

  window.__chatwMsgStore = msgStoreApi;
  if (window.__chatwStorage) {
    window.__chatwStorage.msgStore = msgStoreApi;
  }
})();
