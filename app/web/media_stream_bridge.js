// Main-thread Service Worker registration, SharedWorker direct-port bridge & `/stream/{grantId}` grant manager (Spec 071)
(() => {
  let regPromise = null;

  function randomGrantId() {
    const bytes = new Uint8Array(16);
    crypto.getRandomValues(bytes);
    return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
  }

  async function registerSw() {
    if (!('serviceWorker' in navigator)) return null;
    if (regPromise) return regPromise;
    regPromise = navigator.serviceWorker
      .register('/sw.js', { scope: '/' })
      .then(async (reg) => {
        try { await navigator.serviceWorker.ready; } catch (_) {}
        if (window.__chatwCore && typeof window.__chatwCore.bridgeServiceWorkerPort === 'function') {
          window.__chatwCore.bridgeServiceWorkerPort();
        }
        return reg;
      })
      .catch(() => null);
    return regPromise;
  }

  function sendSwMessage(payload, timeoutMs = 2500) {
    return new Promise((resolve) => {
      const ctrl = navigator.serviceWorker && navigator.serviceWorker.controller;
      if (!ctrl) {
        resolve({ ok: false, error: 'no_sw_controller' });
        return;
      }
      const ch = new MessageChannel();
      const timer = setTimeout(() => {
        try { ch.port1.close(); } catch (_) {}
        resolve({ ok: false, error: 'sw_timeout' });
      }, timeoutMs);
      ch.port1.onmessage = (ev) => {
        clearTimeout(timer);
        resolve(ev.data || { ok: false });
      };
      try {
        ctrl.postMessage(payload, [ch.port2]);
      } catch (err) {
        clearTimeout(timer);
        resolve({ ok: false, error: String(err && err.message ? err.message : err) });
      }
    });
  }

  async function ensureController(timeoutMs = 2500) {
    if (!('serviceWorker' in navigator)) return false;
    await registerSw();
    if (navigator.serviceWorker.controller) {
      if (window.__chatwCore && typeof window.__chatwCore.bridgeServiceWorkerPort === 'function') {
        await window.__chatwCore.bridgeServiceWorkerPort();
      }
      return true;
    }
    return await new Promise((resolve) => {
      let done = false;
      const finish = async (v) => {
        if (done) return;
        done = true;
        navigator.serviceWorker.removeEventListener('controllerchange', onCtrl);
        clearTimeout(timer);
        if (v && window.__chatwCore && typeof window.__chatwCore.bridgeServiceWorkerPort === 'function') {
          await window.__chatwCore.bridgeServiceWorkerPort();
        }
        resolve(v);
      };
      const onCtrl = () => finish(Boolean(navigator.serviceWorker.controller));
      navigator.serviceWorker.addEventListener('controllerchange', onCtrl);
      const timer = setTimeout(() => finish(Boolean(navigator.serviceWorker.controller)), timeoutMs);
    });
  }

  if (typeof window !== 'undefined' && 'serviceWorker' in navigator) {
    registerSw();
  }

  window.__chatwMediaStream = {
    async ensureReady(timeoutMs = 2500) {
      return await ensureController(timeoutMs);
    },

    async registerStream({ url, keyB64 = '', nonceB64 = '', mime = 'application/octet-stream', fileName = 'media' }) {
      const ready = await ensureController(2000);
      if (!ready) return { ok: false, error: 'sw_not_controlling' };
      // When SharedWorker direct MessagePort is bridged, SW pulls chunks over encrypted CWBF2 Binary WS
      // with ZERO `/api/attachments/stream-token` HTTP fetch!
      const grantId = randomGrantId();
      const resp = await sendSwMessage({
        type: 'REGISTER_MEDIA_GRANT',
        grantId,
        url,
        keyB64,
        nonceB64,
        mime,
        fileName,
        streamToken: '',
      });
      return resp;
    },

    async revokeStream(grantId) {
      if (!grantId) return { ok: true };
      return await sendSwMessage({ type: 'REVOKE_MEDIA_GRANT', grantId }, 1000);
    },

    async revokeAll() {
      return await sendSwMessage({ type: 'REVOKE_ALL_GRANTS' }, 1000);
    },

    async getStatus() {
      const ready = await ensureController(1200);
      if (!ready) return { ok: false, controlling: false };
      const ping = await sendSwMessage({ type: 'PING_SW' }, 1500);
      return { ...ping, controlling: true };
    },
  };
})();
