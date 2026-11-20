// Client-side IndexedDB + OPFS + Cache API Storage Engine for ChatW (Spec 022)
// Handles persistent chunked/object caching, SHA-256 integrity, QuotaExceededError resilience,
// and safe cache-vs-source eviction protecting unique unbacked `local_only` DM media.
(() => {
  const DB_NAME = 'chatw_storage_v1';
  const DB_VERSION = 1;
  const OPFS_DIR = 'chatw_cas';
  const CACHE_NAME = 'chatw_media_cache_v1';

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

  async function evictSafeCacheIfNeeded(incomingBytes) {
    const limitMb = getConfiguredLimitMb();
    const rows = await idbGetAll('objects');
    const totalUsed = rows.reduce((acc, r) => acc + (r.byte_size || 0), 0);
    if (limitMb === 0) return { evicted: 0, freed: 0 }; // 0 = Unlimited
    const limitBytes = limitMb * 1024 * 1024;
    if (totalUsed + incomingBytes <= limitBytes) return { evicted: 0, freed: 0 };

    // LRU sort on evictable entries only (server-durable Group/Channel or replicated copies).
    const candidates = rows.filter(isSafeToEvict).sort((a, b) => (a.last_accessed_at || 0) - (b.last_accessed_at || 0));
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
          sha256_plain: meta.sha256_plain || shaHex,
          availability_state: meta.availability_state || 'local_only',
          storage_tier: storedInOpfs ? 'opfs' : 'indexeddb',
          opfs_key: storedInOpfs ? opfsKey : null,
          blob: storedInOpfs ? null : bytes,
          thumb_b64: meta.thumb_b64 || null,
          updated_at: now,
          last_accessed_at: now,
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
      // Touch LRU timestamp
      try {
        meta.last_accessed_at = Date.now();
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
      await idbTx('sync_cursors', 'readwrite', (s) => s.put({ id: 'current', ...cursors, updated_at: Date.now() }));
      return true;
    },

    async loadCursors() {
      return (await idbTx('sync_cursors', 'readonly', (s) => s.get('current'))) || { id: 'current', dm_seq: 0, group_seq: 0, channel_seq: 0 };
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
  };
})();
