// ChatW Sandboxed Plugin Runtime v2 (Web + Desktop WebKitGTK)
// - Executes JS/TS bundles and Rust-compiled WebAssembly (.wasm) inside isolated Blob Web Workers
// - Renders arbitrary custom plugin HTML/CSS/JS UI inside opaque-origin `sandbox="allow-scripts"` iframes
// - Enforces live per-installation permission grants, E2EE taint-tracked egress blocking, rate limits,
//   execution timeout kill-switch, Event Bus with deduplication, and dynamic UI extension registries.
(function () {
  if (window.ChatwPluginHost) return;

  const instances = new Map(); // instId -> runtime state
  const uiFrames = new Map();  // frameNonce -> { instId, iframe, slot, context }
  const seenDeliveries = new Set();
  const maxSeenDeliveries = 500;
  let globalAccount = { id: '', username: '', displayName: '' };
  let decryptedMessagesProvider = null; // (conversationId) => Array<{id, senderName, text, createdAt}>
  let composerInsertHandler = null;
  let toastHandler = null;

  function randNonce() {
    const arr = new Uint8Array(16);
    crypto.getRandomValues(arr);
    return Array.from(arr, (b) => b.toString(16).padStart(2, '0')).join('');
  }

  function getThemeTokens() {
    const cs = getComputedStyle(document.documentElement);
    const get = (k, fb) => (cs.getPropertyValue(k) || '').trim() || fb;
    return {
      theme: document.documentElement.getAttribute('data-theme') || 'light',
      font: get('--font-family', 'system-ui, sans-serif'),
      bg: get('--bg-chat', '#ffffff'),
      surface: get('--bg-surface', '#f8fafc'),
      surfaceHover: get('--bg-surface-hover', '#f1f5f9'),
      input: get('--bg-input', '#f1f5f9'),
      text: get('--text-primary', '#0f172a'),
      textSecondary: get('--text-secondary', '#475569'),
      accent: get('--accent', '#2563eb'),
      danger: get('--danger', '#ef4444'),
      radius: get('--radius-lg', '12px'),
    };
  }

  function notifyExtensionsChanged() {
    window.dispatchEvent(new CustomEvent('chatw-plugin-extensions-changed'));
  }

  function hasPermission(inst, perm, scopeId = null) {
    if (!inst || !inst.enabled) return false;
    const grants = inst.grants || [];
    const now = Date.now();
    for (const g of grants) {
      if (g.revoked_at) continue;
      if (g.expires_at && Date.parse(g.expires_at) <= now) continue;
      if (g.permission !== perm) continue;
      if (perm.startsWith('e2ee.')) {
        if (scopeId && g.scope_id && String(g.scope_id) === String(scopeId)) return true;
        continue;
      }
      if (g.scope_type === 'global' || !g.scope_id) return true;
      if (scopeId && String(g.scope_id) === String(scopeId)) return true;
    }
    return false;
  }

  function isChannelAllowed(inst, channelId) {
    if (!channelId) return true;
    const blocked = inst.blocked_channels || [];
    if (blocked.map(String).includes(String(channelId))) return false;
    const allowed = inst.allowed_channels || [];
    if (allowed.length > 0 && !allowed.map(String).includes(String(channelId))) return false;
    return true;
  }

  function addLog(inst, level, message, detail) {
    if (!inst) return;
    inst.logs.unshift({
      ts: new Date().toISOString(),
      level,
      message: String(message || ''),
      detail: detail ? String(detail).slice(0, 300) : '',
    });
    if (inst.logs.length > 80) inst.logs.length = 80;
    notifyExtensionsChanged();
  }

  async function apiJson(method, path, body) {
    const opts = {
      method,
      credentials: 'include',
      headers: { Accept: 'application/json' },
    };
    if (body !== undefined) {
      opts.headers['Content-Type'] = 'application/json';
      opts.body = JSON.stringify(body);
    }
    const resp = await fetch(path, opts);
    const txt = await resp.text();
    let parsed = null;
    try { parsed = txt ? JSON.parse(txt) : null; } catch (_) { parsed = txt; }
    if (!resp.ok) {
      const code = (parsed && parsed.error) || `http_${resp.status}`;
      const err = new Error(code);
      err.status = resp.status;
      err.code = code;
      throw err;
    }
    return parsed;
  }

  // ==========================================================================
  // SDK Bootstrap Script Injected Inside Isolated Workers & Sandboxed Iframes
  // ==========================================================================
  function buildSdkShim(nonce, mode) {
    const postLine =
      mode === 'worker'
        ? `self.postMessage({ __cwRpc: true, nonce: NONCE, id, method, args });`
        : `window.parent.postMessage({ __cwRpc: true, nonce: NONCE, id, method, args }, '*');`;
    return `
const NONCE = ${JSON.stringify(nonce)};
let _rpcSeq = 0;
const _pending = new Map();
const _cmdHandlers = new Map();
const _msgActionHandlers = new Map();
const _eventHandlers = new Map();
const _readyCbs = [];
let _isReady = false;

function _rpc(method, args) {
  return new Promise((resolve, reject) => {
    const id = 'r_' + (++_rpcSeq);
    const t = setTimeout(() => {
      _pending.delete(id);
      reject(new Error('RPC_TIMEOUT: ' + method));
    }, 10000);
    _pending.set(id, {
      resolve: (v) => { clearTimeout(t); resolve(v); },
      reject: (e) => { clearTimeout(t); reject(new Error(String(e || 'RPC_ERROR'))); }
    });
    ${postLine}
  });
}

function _handleHostMessage(msg) {
  if (!msg || msg.nonce !== NONCE) return;
  if (msg.__cwRpcRes) {
    const p = _pending.get(msg.id);
    if (!p) return;
    _pending.delete(msg.id);
    if (msg.error) p.reject(msg.error);
    else p.resolve(msg.result);
    return;
  }
  if (msg.__cwReady) {
    _isReady = true;
    while (_readyCbs.length) { try { _readyCbs.shift()(msg.context || {}); } catch (e) { console.error(e); } }
    return;
  }
  if (msg.__cwInvoke) {
    const invId = msg.invId;
    const reply = (ok, val) => {
      const payload = { __cwInvokeRes: true, nonce: NONCE, invId, ok, result: ok ? val : undefined, error: ok ? undefined : String(val) };
      ${mode === 'worker' ? 'self.postMessage(payload);' : "window.parent.postMessage(payload, '*');"}
    };
    (async () => {
      try {
        if (msg.kind === 'command') {
          const fn = _cmdHandlers.get(String(msg.trigger || '').toLowerCase());
          if (!fn) {
            if (typeof globalThis.__cwWasmInvoke === 'function') {
              return reply(true, await globalThis.__cwWasmInvoke({ kind: 'command', trigger: msg.trigger, args: msg.args, context: msg.context }));
            }
            return reply(true, null);
          }
          return reply(true, await fn({ trigger: msg.trigger, args: msg.args, ...(msg.context || {}) }));
        }
        if (msg.kind === 'message_action') {
          const fn = _msgActionHandlers.get(msg.actionId);
          if (fn) return reply(true, await fn(msg.message || {}));
          if (typeof globalThis.__cwWasmInvoke === 'function') {
            return reply(true, await globalThis.__cwWasmInvoke({ kind: 'message_action', actionId: msg.actionId, message: msg.message }));
          }
          return reply(true, null);
        }
        if (msg.kind === 'event') {
          const list = _eventHandlers.get(msg.eventName) || [];
          const wild = _eventHandlers.get('*') || [];
          for (const fn of [...list, ...wild]) {
            await fn(msg.payload || {}, msg.meta || {});
          }
          if (typeof globalThis.__cwWasmInvoke === 'function') {
            await globalThis.__cwWasmInvoke({ kind: 'event', eventName: msg.eventName, payload: msg.payload });
          }
          return reply(true, { handled: true });
        }
        if (msg.kind === 'wasm_render') {
          if (typeof globalThis.__cwWasmInvoke === 'function') {
            return reply(true, await globalThis.__cwWasmInvoke({ kind: 'render', context: msg.context || {} }));
          }
          return reply(true, null);
        }
        reply(true, null);
      } catch (err) {
        reply(false, (err && err.message) || String(err));
      }
    })();
  }
}

${mode === 'worker'
  ? `self.addEventListener('message', (e) => _handleHostMessage(e.data));`
  : `window.addEventListener('message', (e) => _handleHostMessage(e.data));`}

const chatw = {
  app: {
    info: () => _rpc('app.info', {}),
    themeTokens: () => _rpc('theme.getTokens', {}),
  },
  account: {
    current: () => _rpc('account.current', {}),
  },
  messages: {
    readDecrypted: (conversationId) => _rpc('messages.readDecrypted', { conversationId }),
    insertToComposer: (text) => _rpc('messages.insertToComposer', { text }),
    sendToChannel: (channelId, content) => _rpc('messages.sendToChannel', { channelId, content }),
    sendToGroup: (groupId, content) => _rpc('messages.sendToGroup', { groupId, content }),
  },
  chats: {
    active: () => _rpc('chats.active', {}),
  },
  conversations: {
    active: () => _rpc('chats.active', {}),
    readDecrypted: (conversationId) => _rpc('messages.readDecrypted', { conversationId }),
  },
  groups: {
    list: () => _rpc('groups.list', {}),
  },
  servers: {
    list: () => _rpc('servers.list', {}),
  },
  channels: {
    list: (serverId) => _rpc('channels.list', { serverId }),
  },
  members: {
    list: (serverId) => _rpc('members.list', { serverId }),
  },
  users: {
    current: () => _rpc('account.current', {}),
    listByServer: (serverId) => _rpc('members.list', { serverId }),
  },
  calls: {
    status: () => _rpc('calls.status', {}),
  },
  voice: {
    status: () => _rpc('calls.status', {}),
  },
  video: {
    status: () => _rpc('calls.status', {}),
  },
  modals: {
    open: (opts) => _rpc('ui.openDialog', opts || {}),
    close: () => _rpc('ui.closeDialog', {}),
  },
  menus: {
    registerMessageAction: (action) => _rpc('ui.registerMessageAction', action),
  },
  sidebar: {
    registerPage: (page) => _rpc('ui.registerPage', page),
  },
  toolbar: {
    registerComposerButton: (btn) => _rpc('ui.registerComposerButton', btn),
  },
  bots: {
    list: () => _rpc('bots.list', {}),
  },
  events: {
    on: (name, fn) => {
      if (!_eventHandlers.has(name)) _eventHandlers.set(name, []);
      _eventHandlers.get(name).push(fn);
      return _rpc('events.subscribe', { eventName: name });
    },
    off: (name, fn) => {
      const list = _eventHandlers.get(name);
      if (!list) return;
      const i = list.indexOf(fn);
      if (i >= 0) list.splice(i, 1);
    },
  },
  ui: {
    notify: (message, level) => _rpc('ui.notify', { message, level: level || 'info' }),
    openDialog: (opts) => _rpc('ui.openDialog', opts || {}),
    closeDialog: () => _rpc('ui.closeDialog', {}),
    registerMessageAction: (action) => _rpc('ui.registerMessageAction', action),
    registerComposerButton: (btn) => _rpc('ui.registerComposerButton', btn),
    registerPage: (page) => _rpc('ui.registerPage', page),
  },
  commands: {
    register: (trigger, description, handler) => {
      const t = String(trigger || '').trim().toLowerCase();
      if (typeof handler === 'function') _cmdHandlers.set(t, handler);
      return _rpc('commands.register', { trigger: t, description: description || '' });
    },
  },
  notifications: {
    send: (title, body) => _rpc('notifications.send', { title, body }),
  },
  storage: {
    get: (key) => _rpc('storage.get', { key }),
    set: (key, value, opts) => _rpc('storage.set', { key, value, ...(opts || {}) }),
    delete: (key) => _rpc('storage.delete', { key }),
    list: () => _rpc('storage.list', {}),
    setSecret: (key, value) => _rpc('storage.set', { key, value, is_secret: true, syncable: false }),
  },
  network: {
    fetch: (url, opts) => _rpc('network.fetch', { url, ...(opts || {}) }),
  },
  webhooks: {
    list: (scopeType, scopeId) => _rpc('webhooks.list', { scopeType, scopeId }),
    create: (payload) => _rpc('webhooks.create', payload),
  },
  settings: {
    get: () => _rpc('settings.get', {}),
    set: (patch) => _rpc('settings.set', { patch }),
  },
  permissions: {
    list: () => _rpc('permissions.list', {}),
    has: (permission, scopeId) => _rpc('permissions.has', { permission, scopeId }),
  },
  media: {},
  files: {},
  theme: {
    getTokens: () => _rpc('theme.getTokens', {}),
  },
  lifecycle: {
    onEnable: (fn) => chatw.events.on('onEnable', fn),
    onDisable: (fn) => chatw.events.on('onDisable', fn),
  },
  onCommand: (trigger, handler) => chatw.commands.register(trigger, '', handler),
  onMessageAction: (actionId, handler) => { _msgActionHandlers.set(actionId, handler); },
  onReady: (cb) => { if (_isReady) cb({}); else _readyCbs.push(cb); },
};
globalThis.chatw = chatw;
`;
  }

  // ==========================================================================
  // Worker Launcher (Supports JS/TS bundles AND Rust-compiled WebAssembly)
  // ==========================================================================
  function buildWorkerSource(inst, nonce) {
    const wasmB64 = (inst.wasm_base64 || '').trim();
    const maxPages = Math.min(4096, Math.max(16, Math.ceil(((inst.manifest?.resource_limits?.max_memory_mb) || 32) * 16)));
    const wasmLoader = wasmB64
      ? `
let _wasmInst = null;
async function _initWasm() {
  const b64 = ${JSON.stringify(wasmB64)};
  const raw = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
  const dec = new TextDecoder();
  const enc = new TextEncoder();
  const imports = {
    env: {
      chatw_host_log(ptr, len) {
        if (!_wasmInst || !_wasmInst.exports.memory) return;
        const msg = dec.decode(new Uint8Array(_wasmInst.exports.memory.buffer, ptr, len));
        chatw.ui.notify(msg, 'info');
      },
      chatw_host_emit(ptr, len) {
        if (!_wasmInst || !_wasmInst.exports.memory) return;
        const txt = dec.decode(new Uint8Array(_wasmInst.exports.memory.buffer, ptr, len));
        try {
          const obj = JSON.parse(txt);
          if (obj && obj.method) _rpc(obj.method, obj.args || {});
        } catch (_) {}
      }
    }
  };
  const { instance } = await WebAssembly.instantiate(raw, imports);
  _wasmInst = instance;
  if (_wasmInst.exports.memory && _wasmInst.exports.memory.buffer.byteLength > ${maxPages} * 65536) {
    throw new Error('WASM_MEMORY_LIMIT_EXCEEDED');
  }
  globalThis.__cwWasmInvoke = async (reqObj) => {
    if (!_wasmInst || typeof _wasmInst.exports.chatw_invoke !== 'function' || typeof _wasmInst.exports.chatw_alloc !== 'function') {
      return null;
    }
    const bytes = enc.encode(JSON.stringify(reqObj));
    const ptr = _wasmInst.exports.chatw_alloc(bytes.length);
    new Uint8Array(_wasmInst.exports.memory.buffer, ptr, bytes.length).set(bytes);
    const outPtr = _wasmInst.exports.chatw_invoke(ptr, bytes.length);
    if (!outPtr) return null;
    const view = new DataView(_wasmInst.exports.memory.buffer);
    const outLen = view.getUint32(outPtr, true);
    const outBytes = new Uint8Array(_wasmInst.exports.memory.buffer, outPtr + 4, outLen);
    const outText = dec.decode(outBytes);
    try { return JSON.parse(outText); } catch (_) { return { text: outText }; }
  };
}
`
      : `async function _initWasm() {}`;

    return `
"use strict";
${buildSdkShim(nonce, 'worker')}
${wasmLoader}
(async () => {
  try {
    await _initWasm();
    ${inst.bundle_code || ''}
    self.postMessage({ __cwWorkerBooted: true, nonce: NONCE });
  } catch (err) {
    self.postMessage({ __cwWorkerError: true, nonce: NONCE, error: (err && err.stack) || String(err) });
  }
})();
`;
  }

  // ==========================================================================
  // RPC Router with Live Permission Validation & E2EE Egress Taint Guard
  // ==========================================================================
  async function handleRpcCall(inst, method, args) {
    if (!inst.enabled) throw new Error('PLUGIN_DISABLED');
    const a = args || {};
    switch (method) {
      case 'app.info':
        return { platform: window.__TAURI__ || navigator.userAgent.includes('WebKitGTK') ? 'desktop' : 'web', sdkVersion: '1.0.0' };
      case 'theme.getTokens':
        return getThemeTokens();
      case 'account.current':
        if (!hasPermission(inst, 'profile.read')) throw new Error('PERMISSION_DENIED: profile.read');
        return { id: globalAccount.id, username: globalAccount.username, displayName: globalAccount.displayName };
      case 'permissions.list':
        return (inst.grants || []).filter((g) => !g.revoked_at).map((g) => ({
          permission: g.permission, scope_type: g.scope_type, scope_id: g.scope_id, device_only: g.device_only
        }));
      case 'permissions.has':
        return hasPermission(inst, a.permission, a.scopeId || null);
      case 'settings.get':
        return inst.settings || {};
      case 'settings.set': {
        const merged = { ...(inst.settings || {}), ...(a.patch || {}) };
        inst.settings = merged;
        await apiJson('PATCH', `/api/plugins/installations/${inst.id}`, { settings: merged });
        return merged;
      }
      case 'storage.get': {
        if (!hasPermission(inst, 'storage.local') && !hasPermission(inst, 'storage.sync')) {
          throw new Error('PERMISSION_DENIED: storage.local');
        }
        if (inst.storageCache && Object.prototype.hasOwnProperty.call(inst.storageCache, a.key)) {
          return inst.storageCache[a.key];
        }
        const res = await apiJson('GET', `/api/plugins/installations/${inst.id}/storage`);
        inst.storageCache = (res && res.data) || {};
        return inst.storageCache[a.key] ?? null;
      }
      case 'storage.list': {
        if (!hasPermission(inst, 'storage.local') && !hasPermission(inst, 'storage.sync')) {
          throw new Error('PERMISSION_DENIED: storage.local');
        }
        const res = await apiJson('GET', `/api/plugins/installations/${inst.id}/storage`);
        inst.storageCache = (res && res.data) || {};
        return inst.storageCache;
      }
      case 'storage.set': {
        if (!hasPermission(inst, 'storage.local') && !hasPermission(inst, 'storage.sync')) {
          throw new Error('PERMISSION_DENIED: storage.local');
        }
        const syncable = a.is_secret ? false : (a.syncable !== false && hasPermission(inst, 'storage.sync'));
        await apiJson('PUT', `/api/plugins/installations/${inst.id}/storage/${encodeURIComponent(String(a.key || ''))}`, {
          value: a.value,
          is_secret: !!a.is_secret,
          syncable,
        });
        if (!a.is_secret) {
          inst.storageCache = inst.storageCache || {};
          inst.storageCache[a.key] = a.value;
        }
        return { ok: true };
      }
      case 'storage.delete': {
        if (!hasPermission(inst, 'storage.local') && !hasPermission(inst, 'storage.sync')) {
          throw new Error('PERMISSION_DENIED: storage.local');
        }
        await apiJson('DELETE', `/api/plugins/installations/${inst.id}/storage/${encodeURIComponent(String(a.key || ''))}`);
        if (inst.storageCache) delete inst.storageCache[a.key];
        return { ok: true };
      }
      case 'messages.readDecrypted': {
        const cid = String(a.conversationId || '');
        if (!cid) throw new Error('CONVERSATION_ID_REQUIRED');
        if (!hasPermission(inst, 'e2ee.read_decrypted', cid)) {
          throw new Error('PERMISSION_DENIED: e2ee.read_decrypted');
        }
        // Server-side verification & immutable audit log entry
        await apiJson('POST', `/api/plugins/installations/${inst.id}/audit`, {
          action: 'e2ee_read',
          permission: 'e2ee.read_decrypted',
          conversation_id: cid,
          detail: { conversation_id: cid },
        });
        // Mark sandbox instance as E2EE-tainted so outbound network calls require explicit e2ee.egress_external!
        inst.e2eeTainted = true;
        addLog(inst, 'audit', `Đã đọc tin nhắn giải mã cho cuộc trò chuyện ${cid}`);
        if (typeof decryptedMessagesProvider === 'function') {
          return decryptedMessagesProvider(cid) || [];
        }
        return [];
      }
      case 'messages.insertToComposer': {
        if (!hasPermission(inst, 'ui.composer') && !hasPermission(inst, 'commands.register') && !hasPermission(inst, 'ui.message_action')) {
          throw new Error('PERMISSION_DENIED: ui.composer');
        }
        if (typeof composerInsertHandler === 'function') composerInsertHandler(String(a.text || ''));
        window.dispatchEvent(new CustomEvent('chatw-plugin-composer-insert', { detail: { text: String(a.text || '') } }));
        return { ok: true };
      }
      case 'messages.sendToChannel': {
        if (!hasPermission(inst, 'messages.write')) throw new Error('PERMISSION_DENIED: messages.write');
        if (!isChannelAllowed(inst, a.channelId)) throw new Error('CHANNEL_BLOCKED_BY_SCOPE');
        return await apiJson('POST', `/api/channels/${encodeURIComponent(a.channelId)}/messages`, { content: String(a.content || '') });
      }
      case 'messages.sendToGroup': {
        if (!hasPermission(inst, 'messages.write')) throw new Error('PERMISSION_DENIED: messages.write');
        return await apiJson('POST', `/api/groups/${encodeURIComponent(a.groupId)}/messages`, { content: String(a.content || '') });
      }
      case 'groups.list':
        if (!hasPermission(inst, 'groups.read')) throw new Error('PERMISSION_DENIED: groups.read');
        return await apiJson('GET', '/api/groups');
      case 'servers.list':
        if (!hasPermission(inst, 'servers.read')) throw new Error('PERMISSION_DENIED: servers.read');
        return await apiJson('GET', '/api/servers');
      case 'channels.list':
        if (!hasPermission(inst, 'channels.read')) throw new Error('PERMISSION_DENIED: channels.read');
        return await apiJson('GET', `/api/servers/${encodeURIComponent(a.serverId)}/channels`);
      case 'members.list':
        if (!hasPermission(inst, 'servers.read')) throw new Error('PERMISSION_DENIED: servers.read');
        return await apiJson('GET', `/api/servers/${encodeURIComponent(a.serverId)}/members`);
      case 'network.fetch': {
        if (!hasPermission(inst, 'network.outbound')) throw new Error('PERMISSION_DENIED: network.outbound');
        if (inst.e2eeTainted && !hasPermission(inst, 'e2ee.egress_external')) {
          throw new Error('E2EE_EGRESS_DENIED: plugin chỉ được xử lý E2EE cục bộ');
        }
        return await apiJson('POST', `/api/plugins/installations/${inst.id}/net/fetch`, {
          url: a.url,
          method: a.method || 'GET',
          headers: a.headers || {},
          body: a.body,
          inject_secret_key: a.inject_secret_key,
          inject_secret_header: a.inject_secret_header,
          inject_secret_prefix: a.inject_secret_prefix,
          e2ee_tainted: !!inst.e2eeTainted,
        });
      }
      case 'webhooks.list':
        if (!hasPermission(inst, 'webhooks.manage')) throw new Error('PERMISSION_DENIED: webhooks.manage');
        return await apiJson('GET', `/api/webhooks?scope_type=${encodeURIComponent(a.scopeType || '')}&scope_id=${encodeURIComponent(a.scopeId || '')}`);
      case 'webhooks.create':
        if (!hasPermission(inst, 'webhooks.manage')) throw new Error('PERMISSION_DENIED: webhooks.manage');
        return await apiJson('POST', '/api/webhooks', { ...a, installation_id: inst.id });
      case 'bots.list':
        return await apiJson('GET', `/api/plugins/installations/${inst.id}/bots`);
      case 'commands.register': {
        if (!hasPermission(inst, 'commands.register')) throw new Error('PERMISSION_DENIED: commands.register');
        const trig = String(a.trigger || '').trim().toLowerCase();
        if (!trig.startsWith('/')) throw new Error('INVALID_COMMAND_TRIGGER');
        if (!inst.commands.some((c) => c.trigger === trig)) {
          inst.commands.push({ trigger: trig, description: String(a.description || '') });
          notifyExtensionsChanged();
        }
        return { ok: true };
      }
      case 'ui.registerMessageAction': {
        if (!hasPermission(inst, 'ui.message_action')) throw new Error('PERMISSION_DENIED: ui.message_action');
        if (!inst.messageActions.some((x) => x.id === a.id)) {
          inst.messageActions.push({ id: String(a.id), label: String(a.label || a.id), icon: String(a.icon || '') });
          notifyExtensionsChanged();
        }
        return { ok: true };
      }
      case 'ui.registerComposerButton': {
        if (!hasPermission(inst, 'ui.composer')) throw new Error('PERMISSION_DENIED: ui.composer');
        if (!inst.composerButtons.some((x) => x.id === a.id)) {
          inst.composerButtons.push({ id: String(a.id), label: String(a.label || a.id), content: String(a.content || '') });
          notifyExtensionsChanged();
        }
        return { ok: true };
      }
      case 'ui.openDialog': {
        if (!hasPermission(inst, 'ui.dialog')) throw new Error('PERMISSION_DENIED: ui.dialog');
        window.dispatchEvent(new CustomEvent('chatw-plugin-open-dialog', { detail: { installationId: inst.id, title: a.title || inst.name, html: a.html || '' } }));
        return { ok: true };
      }
      case 'ui.closeDialog': {
        window.dispatchEvent(new CustomEvent('chatw-plugin-close-dialog'));
        return { ok: true };
      }
      case 'ui.notify':
      case 'notifications.send': {
        const text = a.message || (a.title ? `${a.title}: ${a.body || ''}` : '');
        if (typeof toastHandler === 'function') toastHandler(text);
        window.dispatchEvent(new CustomEvent('chatw-plugin-toast', { detail: { text, pluginName: inst.name } }));
        addLog(inst, 'info', text);
        return { ok: true };
      }
      case 'events.subscribe': {
        if (!inst.subscriptions.includes(a.eventName)) inst.subscriptions.push(a.eventName);
        return { ok: true };
      }
      default:
        throw new Error(`UNKNOWN_RPC_METHOD: ${method}`);
    }
  }

  // Global listener for sandboxed iframe postMessages
  window.addEventListener('message', async (e) => {
    const data = e.data;
    if (!data || !data.nonce) return;
    const entry = uiFrames.get(data.nonce);
    if (!entry || !entry.iframe || e.source !== entry.iframe.contentWindow) return;
    if (data.__cwSandboxShellReady) {
      try {
        entry.iframe.contentWindow.postMessage(
          {
            __cwMountDoc: true,
            nonce: data.nonce,
            cssVars: entry.cssVars,
            sdkCode: entry.sdkCode,
            html: entry.rawHtml,
            readyContext: entry.readyContext,
          },
          '*'
        );
      } catch (_) {}
      return;
    }
    if (!data.__cwRpc) return;
    const inst = instances.get(entry.instId);
    try {
      const result = await handleRpcCall(inst, data.method, data.args);
      entry.iframe.contentWindow.postMessage({ __cwRpcRes: true, nonce: data.nonce, id: data.id, result }, '*');
    } catch (err) {
      entry.iframe.contentWindow.postMessage({ __cwRpcRes: true, nonce: data.nonce, id: data.id, error: (err && err.message) || String(err) }, '*');
    }
  });

  function stopInstance(instId) {
    const inst = instances.get(instId);
    if (!inst) return;
    if (inst.worker) {
      try { inst.worker.terminate(); } catch (_) {}
      inst.worker = null;
    }
    if (inst.workerUrl) {
      try { URL.revokeObjectURL(inst.workerUrl); } catch (_) {}
      inst.workerUrl = null;
    }
    for (const [nonce, f] of uiFrames.entries()) {
      if (f.instId === instId) {
        if (f.iframe && f.iframe.parentNode) f.iframe.parentNode.removeChild(f.iframe);
        if (Array.isArray(f.blobUrls)) {
          for (const u of f.blobUrls) { try { URL.revokeObjectURL(u); } catch (_) {} }
        }
        uiFrames.delete(nonce);
      }
    }
    inst.status = 'stopped';
  }

  function startWorker(inst) {
    stopInstance(inst.id);
    if (!inst.enabled) return;
    if (!inst.bundle_code && !inst.wasm_base64) {
      inst.status = 'running';
      return;
    }
    const nonce = randNonce();
    inst.workerNonce = nonce;
    inst.pendingInvokes = new Map();
    const src = buildWorkerSource(inst, nonce);
    const blob = new Blob([src], { type: 'application/javascript' });
    const url = URL.createObjectURL(blob);
    inst.workerUrl = url;
    const worker = new Worker(url);
    inst.worker = worker;
    inst.status = 'starting';

    worker.onmessage = async (e) => {
      const data = e.data;
      if (!data || data.nonce !== nonce) return;
      if (data.__cwWorkerBooted) {
        inst.status = 'running';
        addLog(inst, 'info', `Khởi chạy sandbox (${inst.runtime}) thành công`);
        try {
          worker.postMessage({
            __cwReady: true,
            nonce,
            context: { installationId: inst.id, pluginKey: inst.plugin_key },
          });
        } catch (_) {}
        return;
      }
      if (data.__cwWorkerError) {
        inst.status = 'crashed';
        inst.lastError = data.error;
        addLog(inst, 'error', 'Lỗi khởi chạy Plugin', data.error);
        return;
      }
      if (data.__cwInvokeRes) {
        const p = inst.pendingInvokes.get(data.invId);
        if (!p) return;
        inst.pendingInvokes.delete(data.invId);
        if (data.ok) p.resolve(data.result);
        else p.reject(new Error(data.error || 'PLUGIN_INVOKE_ERROR'));
        return;
      }
      if (data.__cwRpc) {
        try {
          const result = await handleRpcCall(inst, data.method, data.args);
          worker.postMessage({ __cwRpcRes: true, nonce, id: data.id, result });
        } catch (err) {
          worker.postMessage({ __cwRpcRes: true, nonce, id: data.id, error: (err && err.message) || String(err) });
        }
      }
    };

    worker.onerror = (err) => {
      inst.status = 'crashed';
      inst.lastError = err.message || 'Worker runtime crash';
      addLog(inst, 'error', 'Sandbox bị lỗi và đã cách ly', inst.lastError);
      try { worker.terminate(); } catch (_) {}
      inst.worker = null;
    };
  }

  function invokeWorker(inst, payload) {
    return new Promise((resolve, reject) => {
      if (!inst || !inst.enabled || !inst.worker) return resolve(null);
      const invId = randNonce();
      const timeoutMs = Math.min(10000, Math.max(500, (inst.manifest?.resource_limits?.max_exec_ms) || 3000));
      const timer = setTimeout(() => {
        inst.pendingInvokes.delete(invId);
        // Kill runaway/infinite-loop plugin immediately!
        inst.status = 'timeout_killed';
        inst.lastError = `Vượt quá giới hạn thực thi (${timeoutMs}ms)`;
        addLog(inst, 'error', 'Đã dừng Plugin do treo hoặc quá thời gian xử lý', inst.lastError);
        try { inst.worker.terminate(); } catch (_) {}
        inst.worker = null;
        reject(new Error('PLUGIN_EXECUTION_TIMEOUT'));
      }, timeoutMs);
      inst.pendingInvokes.set(invId, {
        resolve: (v) => { clearTimeout(timer); resolve(v); },
        reject: (e) => { clearTimeout(timer); reject(e); },
      });
      inst.worker.postMessage({ __cwInvoke: true, nonce: inst.workerNonce, invId, ...payload });
    });
  }

  function loadInstallation(record) {
    const existing = instances.get(record.id);
    const sameRuntime =
      existing &&
      existing.enabled &&
      record.enabled &&
      existing.version_id === record.version_id &&
      existing.bundle_code === record.bundle_code &&
      existing.wasm_base64 === record.wasm_base64 &&
      ((!existing.bundle_code && !existing.wasm_base64) || existing.worker);

    if (sameRuntime) {
      // Live in-place sync of grants/settings/channels without tearing down active UI iframes or Worker!
      existing.grants = record.grants || [];
      existing.settings = record.settings || {};
      existing.allowed_channels = record.allowed_channels || [];
      existing.blocked_channels = record.blocked_channels || [];
      existing.ui_html = record.ui_html || existing.ui_html;
      existing.name = record.name || existing.name;
      existing.version = record.version || existing.version;
      notifyExtensionsChanged();
      return existing;
    }

    const uiContrib = (record.manifest && record.manifest.ui_contributions) || {};
    const inst = {
      ...record,
      logs: existing ? existing.logs : [],
      storageCache: existing ? existing.storageCache : null,
      e2eeTainted: false,
      status: record.enabled ? 'starting' : 'stopped',
      lastError: null,
      commands: Array.isArray(record.manifest?.commands)
        ? record.manifest.commands.map((c) => ({ trigger: String(c.trigger || '').toLowerCase(), description: String(c.description || '') }))
        : [],
      messageActions: Array.isArray(uiContrib.messageActions) ? [...uiContrib.messageActions] : [],
      composerButtons: Array.isArray(uiContrib.composerButtons) ? [...uiContrib.composerButtons] : [],
      subscriptions: Array.isArray(record.manifest?.event_subscriptions) ? [...record.manifest.event_subscriptions] : [],
    };
    instances.set(record.id, inst);
    if (record.enabled) {
      startWorker(inst);
    } else {
      stopInstance(record.id);
    }
    notifyExtensionsChanged();
    return inst;
  }

  function mountUi(instId, containerEl, customHtmlOverride) {
    const inst = instances.get(instId);
    if (!inst || !containerEl) return null;
    containerEl.innerHTML = '';
    const tokens = getThemeTokens();
    const nonce = randNonce();
    const cssVars = `:root{--cw-font:${tokens.font};--cw-bg:${tokens.bg};--cw-surface:${tokens.surface};--cw-input:${tokens.input};--cw-text:${tokens.text};--cw-text-secondary:${tokens.textSecondary};--cw-accent:${tokens.accent};--cw-danger:${tokens.danger};--cw-radius:${tokens.radius};}`;
    const rawHtml = customHtmlOverride || inst.ui_html || `<div style="padding:16px;font-family:var(--cw-font)"><h3>${inst.name}</h3><p>Plugin chạy nền (${inst.runtime}).</p></div>`;
    const sdkCode = buildSdkShim(nonce, 'iframe');
    const readyContext = { installationId: inst.id, pluginKey: inst.plugin_key, theme: tokens };

    const iframe = document.createElement('iframe');
    // Strict opaque-origin sandbox: allow-scripts ONLY (NO allow-same-origin, NO cookie/localStorage/DOM access!)
    iframe.setAttribute('sandbox', 'allow-scripts');
    iframe.setAttribute('referrerpolicy', 'no-referrer');
    iframe.className = 'plugin-sandbox-frame';
    iframe.style.cssText = 'width:100%;height:100%;min-height:380px;border:none;background:transparent;display:block;';

    const apiBase =
      window.location && (window.location.protocol === 'http:' || window.location.protocol === 'https:')
        ? ''
        : (window.CHATW_API_URL || 'http://127.0.0.1:8085');
    iframe.src = `${apiBase}/plugin-sandbox-frame#${nonce}`;

    uiFrames.set(nonce, { instId, iframe, cssVars, sdkCode, rawHtml, readyContext });
    containerEl.appendChild(iframe);
    return nonce;
  }

  window.ChatwPluginHost = {
    setAccount(acc) { globalAccount = acc || { id: '', username: '', displayName: '' }; },
    setDecryptedProvider(fn) { decryptedMessagesProvider = fn; },
    setComposerInsertHandler(fn) { composerInsertHandler = fn; },
    setToastHandler(fn) { toastHandler = fn; },

    async syncFromServer(deviceId) {
      const q = deviceId ? `?device_id=${encodeURIComponent(deviceId)}` : '';
      const snap = await apiJson('GET', `/api/plugins/sync${q}`);
      const list = (snap && snap.installations) || [];
      const activeIds = new Set(list.map((x) => x.id));
      for (const oldId of Array.from(instances.keys())) {
        if (!activeIds.has(oldId)) {
          stopInstance(oldId);
          instances.delete(oldId);
        }
      }
      for (const rec of list) {
        const inst = loadInstallation(rec);
        if (snap.storage && snap.storage[rec.id]) {
          inst.storageCache = snap.storage[rec.id];
        }
      }
      notifyExtensionsChanged();
      return snap;
    },

    syncInstallations(list) {
      const items = Array.isArray(list) ? list : [];
      const activeIds = new Set(items.map((x) => x.id));
      for (const oldId of Array.from(instances.keys())) {
        if (!activeIds.has(oldId)) {
          stopInstance(oldId);
          instances.delete(oldId);
        }
      }
      for (const rec of items) {
        loadInstallation(rec);
      }
      notifyExtensionsChanged();
      return items.length;
    },

    listStatuses() {
      return Array.from(instances.values())
        .filter((i) => i.enabled)
        .map((i) => ({
          id: i.id,
          plugin_key: i.plugin_key,
          name: i.name,
          runtime: i.runtime,
          enabled: i.enabled,
          status: i.status,
          lastError: i.lastError,
        }));
    },

    loadInstallation,
    stopInstallation(instId) { stopInstance(instId); instances.delete(instId); notifyExtensionsChanged(); },

    revokeGrantImmediately(instId, grantId) {
      const inst = instances.get(instId);
      if (!inst) return;
      inst.grants = (inst.grants || []).filter((g) => String(g.id) !== String(grantId));
      addLog(inst, 'audit', `Đã thu hồi quyền ngay lập tức (${grantId})`);
      invokeWorker(inst, { kind: 'event', eventName: 'onPermissionRevoked', payload: { grantId } }).catch(() => {});
      notifyExtensionsChanged();
    },

    mountUiBySelector(instId, selector, customHtml) {
      const el = document.querySelector(selector);
      if (!el) return null;
      return mountUi(instId, el, customHtml);
    },

    getMessageActions(channelId) {
      const out = [];
      for (const inst of instances.values()) {
        if (!inst.enabled || !hasPermission(inst, 'ui.message_action') || !isChannelAllowed(inst, channelId)) continue;
        for (const act of inst.messageActions || []) {
          out.push({ installationId: inst.id, pluginKey: inst.plugin_key, pluginName: inst.name, ...act });
        }
      }
      return out;
    },

    getComposerButtons(channelId) {
      const out = [];
      for (const inst of instances.values()) {
        if (!inst.enabled || !hasPermission(inst, 'ui.composer') || !isChannelAllowed(inst, channelId)) continue;
        for (const btn of inst.composerButtons || []) {
          out.push({ installationId: inst.id, pluginKey: inst.plugin_key, pluginName: inst.name, ...btn });
        }
      }
      return out;
    },

    getCommands(channelId) {
      const out = [];
      for (const inst of instances.values()) {
        if (!inst.enabled || !hasPermission(inst, 'commands.register') || !isChannelAllowed(inst, channelId)) continue;
        for (const cmd of inst.commands || []) {
          out.push({ installationId: inst.id, pluginKey: inst.plugin_key, pluginName: inst.name, ...cmd });
        }
      }
      return out;
    },

    getInstancesSummary() {
      return Array.from(instances.values()).map((i) => ({
        id: i.id,
        plugin_key: i.plugin_key,
        name: i.name,
        runtime: i.runtime,
        enabled: i.enabled,
        status: i.status,
        lastError: i.lastError,
        logs: i.logs.slice(0, 20),
      }));
    },

    async triggerMessageAction(installationId, actionId, message) {
      const inst = instances.get(installationId);
      if (!inst || !inst.enabled) return null;
      if (!hasPermission(inst, 'ui.message_action')) throw new Error('PERMISSION_DENIED');
      return await invokeWorker(inst, { kind: 'message_action', actionId, message });
    },

    async runCommand(rawOrTrigger, argsOrContext, maybeContext) {
      let trigger = '';
      let args = '';
      let context = {};
      if (typeof argsOrContext === 'string') {
        trigger = String(rawOrTrigger || '').trim().toLowerCase();
        args = argsOrContext.trim();
        context = maybeContext || {};
      } else {
        const trimmed = String(rawOrTrigger || '').trim();
        if (!trimmed.startsWith('/')) return null;
        const sp = trimmed.indexOf(' ');
        trigger = (sp >= 0 ? trimmed.slice(0, sp) : trimmed).toLowerCase();
        args = sp >= 0 ? trimmed.slice(sp + 1).trim() : '';
        context = argsOrContext || {};
      }
      if (!trigger.startsWith('/')) return null;
      for (const inst of instances.values()) {
        if (!inst.enabled || !hasPermission(inst, 'commands.register')) continue;
        if (!isChannelAllowed(inst, context && context.channelId)) continue;
        if ((inst.commands || []).some((c) => c.trigger === trigger)) {
          return await invokeWorker(inst, { kind: 'command', trigger, args, context });
        }
      }
      return null;
    },

    async emitEvent(eventName, payload, meta = {}) {
      const deliveryId = meta.deliveryId || `${eventName}:${payload?.id || ''}:${Date.now()}`;
      if (meta.deliveryId && seenDeliveries.has(deliveryId)) return;
      seenDeliveries.add(deliveryId);
      if (seenDeliveries.size > maxSeenDeliveries) {
        const first = seenDeliveries.values().next().value;
        seenDeliveries.delete(first);
      }
      for (const inst of instances.values()) {
        if (!inst.enabled || !inst.worker) continue;
        if (!isChannelAllowed(inst, meta.channelId)) continue;
        const subs = inst.subscriptions || [];
        if (subs.length > 0 && !subs.includes(eventName) && !subs.includes('*')) continue;
        // Enforce sensitive permission gating on event delivery
        if (eventName.startsWith('onMessage') && !hasPermission(inst, 'messages.read')) continue;
        if (eventName.startsWith('onChannel') && !hasPermission(inst, 'messages.read') && !hasPermission(inst, 'channels.read')) continue;
        if (eventName.startsWith('onGroup') && !hasPermission(inst, 'groups.read')) continue;
        if (eventName.startsWith('onCall') && !hasPermission(inst, 'calls.observe')) continue;
        invokeWorker(inst, { kind: 'event', eventName, payload, meta: { ...meta, deliveryId } }).catch((err) => {
          addLog(inst, 'error', `Lỗi xử lý sự kiện ${eventName}`, err.message);
        });
      }
    },

    async invokeWasmRender(installationId, context) {
      const inst = instances.get(installationId);
      if (!inst || !inst.enabled || !inst.worker) return null;
      return await invokeWorker(inst, { kind: 'wasm_render', context: context || {} });
    },
  };
})();
