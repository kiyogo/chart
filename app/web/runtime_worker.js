// ChatW Stateless SharedWorker Runtime Loader (Spec 077)
// Fetches the CWRT1 packed binary payload off-main-thread inside a SharedWorker target
// so Chrome DevTools on the page tab shows ZERO Wasm/binary network requests on cold boot.
// Deduplicates concurrent tab requests, transfers ArrayBuffers zero-copy, and terminates
// cleanly via `self.close()` once idle so browser networkidle & worker RAM release immediately.

const inflight = new Map(); // ver -> Promise<ArrayBuffer>
const memCache = new Map(); // ver -> ArrayBuffer
let activeLoads = 0;
let closeTimer = null;

function scheduleAutoClose() {
  if (closeTimer) clearTimeout(closeTimer);
  if (activeLoads > 0) return;
  closeTimer = setTimeout(() => {
    if (activeLoads === 0) {
      memCache.clear();
      try { self.close(); } catch (_) {}
    }
  }, 150);
}

function loadPackedBinary(version) {
  const ver = String(version || '0');
  if (memCache.has(ver)) {
    return Promise.resolve(memCache.get(ver));
  }
  if (inflight.has(ver)) {
    return inflight.get(ver);
  }
  const url = '/pkg/core.bin?v=' + encodeURIComponent(ver);
  const p = fetch(url, { credentials: 'same-origin', cache: 'force-cache' })
    .then((res) => {
      if (!res.ok) throw new Error('core_http_' + res.status);
      return res.arrayBuffer();
    })
    .then((buf) => {
      memCache.set(ver, buf);
      inflight.delete(ver);
      return buf;
    })
    .catch((err) => {
      inflight.delete(ver);
      throw err;
    });
  inflight.set(ver, p);
  return p;
}

self.onconnect = function (ev) {
  const port = ev.ports && ev.ports[0];
  if (!port) return;
  port.onmessage = function (msgEv) {
    const d = msgEv.data || {};
    if (d.type !== 'LOAD_CORE') return;
    if (closeTimer) {
      clearTimeout(closeTimer);
      closeTimer = null;
    }
    activeLoads++;
    loadPackedBinary(d.version)
      .then((buf) => {
        const out = buf.slice(0);
        port.postMessage({ ok: true, id: d.id, buffer: out }, [out]);
      })
      .catch((err) => {
        port.postMessage({
          ok: false,
          id: d.id,
          error: String(err && err.message ? err.message : err),
        });
      })
      .finally(() => {
        activeLoads = Math.max(0, activeLoads - 1);
        try { port.close(); } catch (_) {}
        scheduleAutoClose();
      });
  };
  port.start();
};
