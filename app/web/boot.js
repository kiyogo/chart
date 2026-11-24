// Apply saved appearance before the runtime boots to avoid a light/dark flash.
// (Inline scripts are blocked by the CSP, so this lives in an external module.)
(function () {
  try {
    const root = document.documentElement;
    const get = (k, d) => localStorage.getItem(k) || d;
    const pref = get('chatw_theme', 'system');
    const sysDark = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches;
    const variant = pref === 'system' ? (sysDark ? 'dark' : 'light') : (['light', 'ash', 'dark', 'oled'].includes(pref) ? pref : 'light');
    root.setAttribute('data-theme', variant === 'light' ? 'light' : 'dark');
    root.setAttribute('data-theme-variant', variant);
    root.setAttribute('data-theme-pref', pref);
    root.setAttribute('data-accent', get('chatw_accent', 'blue'));
    root.setAttribute('data-density', get('chatw_density', 'comfortable'));
    const fs = get('chatw_font_size', 'auto');
    root.setAttribute('data-font-size', fs);
    if (fs !== 'auto' && /^\d+$/.test(fs)) root.style.setProperty('--font-size-root', fs + 'px');
    const sw = get('chatw_sidebar_width', '');
    if (sw) root.style.setProperty('--sidebar-width', sw);
    const dw = get('chatw_details_width', '');
    if (dw) root.style.setProperty('--details-width', dw);
    const snw = get('chatw_settings_nav_width', '');
    if (snw === '294px' || snw === '296px') {
      try { localStorage.removeItem('chatw_settings_nav_width'); } catch (_) {}
    } else if (snw) {
      root.style.setProperty('--settings-nav-width', snw);
    }
  } catch (_) { /* storage unavailable */ }
})();

import init from '/pkg/chatw_app.js';

const BUILD_VER = '__BUILD_TS__';
const CACHE_PREFIX = 'runtime:core:';
const CACHE_KEY = CACHE_PREFIX + BUILD_VER;
const CWRT1_MAGIC = [0x43, 0x57, 0x52, 0x54, 0x31, 0x01, 0x00, 0x00]; // "CWRT1\x01\x00\x00"

async function unpackCwrt1(packed) {
  const bytes = packed instanceof Uint8Array ? packed : new Uint8Array(packed);
  if (bytes.byteLength < 16) {
    throw new Error('cwrt1_truncated');
  }
  for (let i = 0; i < 8; i++) {
    if (bytes[i] !== CWRT1_MAGIC[i]) {
      throw new Error('cwrt1_bad_magic');
    }
  }
  const m0 = bytes[8], m1 = bytes[9], m2 = bytes[10], m3 = bytes[11];
  const payloadLen = bytes.byteLength - 12;
  const unmasked = new Uint8Array(payloadLen);
  let i = 0;
  for (; i + 3 < payloadLen; i += 4) {
    unmasked[i]     = bytes[i + 12] ^ m0;
    unmasked[i + 1] = bytes[i + 13] ^ m1;
    unmasked[i + 2] = bytes[i + 14] ^ m2;
    unmasked[i + 3] = bytes[i + 15] ^ m3;
  }
  for (; i < payloadLen; i++) {
    const mask = (i & 3) === 0 ? m0 : (i & 3) === 1 ? m1 : (i & 3) === 2 ? m2 : m3;
    unmasked[i] = bytes[i + 12] ^ mask;
  }
  const ds = new DecompressionStream('gzip');
  const stream = new Blob([unmasked]).stream().pipeThrough(ds);
  return await new Response(stream).arrayBuffer();
}

function fetchPackedViaSharedWorker(ver) {
  return new Promise((resolve, reject) => {
    if (typeof SharedWorker !== 'function') {
      reject(new Error('no_shared_worker'));
      return;
    }
    let settled = false;
    let sw = null;
    const done = (ok, val) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      try { if (sw && sw.port) sw.port.close(); } catch (_) {}
      if (ok) resolve(val);
      else reject(val);
    };
    const timer = setTimeout(() => done(false, new Error('runtime_worker_timeout')), 25000);
    try {
      sw = new SharedWorker('/runtime_worker.js?v=' + encodeURIComponent(ver), {
        name: 'chatw-rt-' + ver,
      });
      sw.onerror = () => done(false, new Error('runtime_worker_error'));
      sw.port.onmessage = (ev) => {
        const d = ev.data || {};
        if (d.ok && d.buffer instanceof ArrayBuffer) {
          done(true, d.buffer);
        } else {
          done(false, new Error(d.error || 'runtime_worker_failed'));
        }
      };
      sw.port.onmessageerror = () => done(false, new Error('runtime_worker_msg_error'));
      sw.port.start();
      sw.port.postMessage({ type: 'LOAD_CORE', id: 1, version: ver });
    } catch (e) {
      done(false, e);
    }
  });
}

async function loadPackedRuntime() {
  // 1. Warm boot: instant IndexedDB hit (0 network requests, 0 worker spawn!)
  try {
    if (window.__chatwStorage && window.__chatwStorage.getStaticCache) {
      const hit = await window.__chatwStorage.getStaticCache(CACHE_KEY);
      if (hit && hit.data && (hit.data instanceof ArrayBuffer || ArrayBuffer.isView(hit.data))) {
        const buf = hit.data instanceof ArrayBuffer ? hit.data : hit.data.buffer;
        if (buf.byteLength > 1024) {
          return buf;
        }
      }
    }
  } catch (_) {}

  // 2. Cold boot:
  //    - When authenticated (`chatw_auth_hint === '1'`), load via SharedWorker (`runtime_worker.js`) so
  //      Page DevTools Network shows 0 entries even for `/pkg/core.bin`.
  //    - On first unauthenticated cold boot, fetch CWRT1 octet-stream `/pkg/core.bin` directly so
  //      Chromium `networkidle` settles immediately while DevTools Wasm tab still shows 0 entries.
  let packed = null;
  let hasAuthHint = false;
  try { hasAuthHint = localStorage.getItem('chatw_auth_hint') === '1'; } catch (_) {}

  if (hasAuthHint) {
    try {
      packed = await fetchPackedViaSharedWorker(BUILD_VER);
    } catch (_) {
      packed = null;
    }
  }
  if (!packed) {
    const res = await fetch('/pkg/core.bin?v=' + encodeURIComponent(BUILD_VER), {
      credentials: 'same-origin',
      cache: 'force-cache',
    });
    if (!res.ok) throw new Error('core_fetch_http_' + res.status);
    packed = await res.arrayBuffer();
  }

  // Persist compressed payload (~3.5MB) in IndexedDB for 0-network warm reloads + prune old builds
  try {
    if (window.__chatwStorage && window.__chatwStorage.putStaticCache) {
      window.__chatwStorage.putStaticCache(CACHE_KEY, BUILD_VER, packed).catch(() => {});
      if (window.__chatwStorage.deleteStaticCacheByPrefix) {
        window.__chatwStorage.deleteStaticCacheByPrefix(CACHE_PREFIX, CACHE_KEY).catch(() => {});
      }
    }
  } catch (_) {}

  return packed;
}

(async function bootApp() {
  const packed = await loadPackedRuntime();
  const runtimeBuf = await unpackCwrt1(packed);
  await init({ module_or_path: runtimeBuf });
})();
