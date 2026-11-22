/* ChatW 33-Language i18n Runtime (Spec 062)
 * Lazy-loads /locales/{code}.json packs, applies Dioxus-safe WeakMap-tracked translations
 * across all UI text nodes, dynamic templates, and attributes (placeholder, aria-label,
 * data-tip, title) while strictly excluding user-generated content.
 */
(function () {
  if (window.__chatwI18n) return;

  const RTL_LANGS = new Set(['ar', 'fa', 'he', 'ur']);
  const VALID_LANGS = new Set([
    'vi', 'en', 'zh-CN', 'zh-TW', 'ja', 'ko', 'th', 'id', 'ms', 'fil', 'km', 'lo',
    'hi', 'bn', 'ar', 'fa', 'he', 'ur', 'tr', 'ru', 'uk', 'pl', 'cs', 'de',
    'fr', 'es', 'pt-BR', 'it', 'nl', 'sv', 'ro', 'hu', 'el',
  ]);

  const ATTRS = ['placeholder', 'aria-label', 'data-tip', 'title'];

  // Template matchers ordered most-specific first (e.g. full date before day-month)
  const TEMPLATE_PATTERNS = [
    { key: 'Độ trong suốt khung trắng: {0}%', re: /^Độ trong suốt khung trắng:\s*(\d+)%$/ },
    { key: 'Mức độ làm mờ kính: {0}px', re: /^Mức độ làm mờ kính:\s*(\d+)px$/ },
    { key: 'Độ tối phủ lên ảnh: {0}%', re: /^Độ tối phủ lên ảnh:\s*(\d+)%$/ },
    { key: 'Làm mờ hình nền: {0}px', re: /^Làm mờ hình nền:\s*(\d+)px$/ },
    { key: 'Phiên bản đồng bộ: {0}', re: /^Phiên bản đồng bộ:\s*(\d+)$/ },
    { key: '{0} tháng {1} năm {2}', re: /^(\d{1,2})\s+tháng\s+(\d{1,2})\s+năm\s+(\d{4})$/ },
    { key: '{0} tháng {1}', re: /^(\d{1,2})\s+tháng\s+(\d{1,2})$/ },
    { key: '{0} lượt bình chọn', re: /^(\d+)\s+lượt bình chọn$/ },
    { key: 'Nhắn tin tới {0}', re: /^Nhắn tin tới\s+(.+)$/ },
    { key: '{0} đang soạn tin', re: /^(.+)\s+đang soạn tin$/ },
    { key: 'Chỉ mình bạn · {0} thiết bị', re: /^Chỉ mình bạn\s*·\s*(\d+)\s+thiết bị$/ },
    { key: 'Bước {0} / {1}', re: /^Bước\s+(\d+)\s*\/\s*(\d+)$/ },
    { key: '{0} phút', re: /^(\d+)\s+phút$/ },
    { key: '{0} giờ', re: /^(\d+)\s+giờ$/ },
    { key: '{0} ngày', re: /^(\d+)\s+ngày$/ },
    { key: 'Kết quả xúc xắc của {0}: {1}', re: /^Kết quả xúc xắc của\s+(.+):\s*(\d+)$/ },
  ];

  // Containers holding user-generated content or native language names — NEVER translate text inside
  const PROTECTED_SELECTOR = [
    '.message-text',
    '.settings-profile-name',
    '.settings-profile-sub',
    '.account-name',
    '.member-name',
    '.result-name',
    '.channel-row-name',
    '.sidebar-server-name',
    '.lang-tile',
    '.fingerprint',
    '.plugin-frame',
    '[data-no-i18n]',
    'code',
    'pre',
    'script',
    'style',
    'svg',
  ].join(', ');

  const packCache = new Map(); // code -> { exact: {}, templates: {} }
  const inflight = new Map();  // code -> Promise

  const origText = new WeakMap();    // Text -> original VI string
  const lastWritten = new WeakMap(); // Text -> last translated string written by us
  const origAttrs = new WeakMap();   // Element -> Map<attr, orig>
  const lastAttrs = new WeakMap();   // Element -> Map<attr, written>

  let activeLang = 'vi';
  let activePack = null;
  let observer = null;
  let muted = false;
  let version = 0;

  function detectInitialLang() {
    try {
      const direct = localStorage.getItem('chatw_lang');
      if (direct && VALID_LANGS.has(direct)) return direct;
      for (let i = 0; i < localStorage.length; i++) {
        const k = localStorage.key(i);
        if (k && k.startsWith('chatw_prefs_')) {
          const parsed = JSON.parse(localStorage.getItem(k) || '{}');
          if (parsed && parsed.language && VALID_LANGS.has(parsed.language)) {
            return parsed.language;
          }
        }
      }
    } catch (_) {}
    return 'vi';
  }

  function applyHtmlLangDir(code) {
    const html = document.documentElement;
    const dir = RTL_LANGS.has(code) ? 'rtl' : 'ltr';
    if (html.getAttribute('lang') !== code) html.setAttribute('lang', code);
    if (html.getAttribute('dir') !== dir) html.setAttribute('dir', dir);
  }

  function fetchPack(code) {
    if (code === 'vi') return Promise.resolve(null);
    if (packCache.has(code)) return Promise.resolve(packCache.get(code));
    if (inflight.has(code)) return inflight.get(code);

    const p = fetch(`/locales/${encodeURIComponent(code)}.json`, { cache: 'force-cache' })
      .then((res) => {
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        return res.json();
      })
      .then((data) => {
        const pack = {
          exact: (data && data.exact) || {},
          templates: (data && data.templates) || {},
        };
        packCache.set(code, pack);
        inflight.delete(code);
        return pack;
      })
      .catch((err) => {
        inflight.delete(code);
        console.warn('[chatw-i18n] failed to load locale:', code, err);
        return null;
      });

    inflight.set(code, p);
    return p;
  }

  function translateCore(str, pack) {
    if (!pack || !str) return null;
    if (Object.prototype.hasOwnProperty.call(pack.exact, str)) {
      return pack.exact[str];
    }
    const tpls = pack.templates;
    for (let i = 0; i < TEMPLATE_PATTERNS.length; i++) {
      const tp = TEMPLATE_PATTERNS[i];
      const outTpl = tpls[tp.key];
      if (!outTpl) continue;
      const m = str.match(tp.re);
      if (m) {
        let res = outTpl;
        for (let g = 1; g < m.length; g++) {
          res = res.replace(`{${g - 1}}`, m[g]);
        }
        return res;
      }
    }
    return null;
  }

  function translateString(raw) {
    if (!raw || typeof raw !== 'string') return raw;
    if (activeLang === 'vi' || !activePack) return raw;
    const m = raw.match(/^(\s*)([\s\S]*?)(\s*)$/);
    if (!m || !m[2]) return raw;
    const tr = translateCore(m[2], activePack);
    return tr != null ? `${m[1]}${tr}${m[3]}` : raw;
  }

  function isTextNodeProtected(node) {
    const el = node.parentElement;
    if (!el) return true;
    if (el.tagName === 'TEXTAREA' || el.tagName === 'INPUT') return true;
    if (el.closest(PROTECTED_SELECTOR)) return true;
    // Allow preview chat bubbles inside Appearance preview, but skip real chat messages
    const bubble = el.closest('.message-bubble');
    if (bubble && !bubble.closest('.settings-preview-chat')) return true;
    // Allow "Chính mình" system name in chat-item-name, skip arbitrary usernames
    const chatName = el.closest('.chat-item-name, .chat-header-title');
    if (chatName) {
      const txt = (origText.get(node) ?? node.nodeValue ?? '').trim();
      if (txt !== 'Chính mình') return true;
    }
    // Skip arbitrary user message previews, but translate the static default Note-to-self subtitle
    const preview = el.closest('.chat-item-preview');
    if (preview) {
      const txt = (origText.get(node) ?? node.nodeValue ?? '').trim();
      if (txt !== 'Ghi chú, tệp và liên kết giữa các thiết bị của bạn') return true;
    }
    return false;
  }

  function processTextNode(node) {
    if (!node || node.nodeType !== 3) return;
    if (isTextNodeProtected(node)) return;

    const currentVal = node.nodeValue || '';
    if (!currentVal.trim()) return;

    // Detect if Dioxus updated this text node to a brand-new source string
    const prevWritten = lastWritten.get(node);
    let src;
    if (prevWritten !== undefined && currentVal === prevWritten) {
      src = origText.get(node);
    } else {
      src = currentVal;
      origText.set(node, src);
      lastWritten.delete(node);
    }
    if (!src) return;

    if (activeLang === 'vi' || !activePack) {
      if (currentVal !== src) {
        muted = true;
        node.nodeValue = src;
        muted = false;
      }
      lastWritten.delete(node);
      return;
    }

    const m = src.match(/^(\s*)([\s\S]*?)(\s*)$/);
    if (!m || !m[2]) return;
    const tr = translateCore(m[2], activePack);
    if (tr == null) return;

    const nextVal = `${m[1]}${tr}${m[3]}`;
    if (currentVal !== nextVal) {
      lastWritten.set(node, nextVal);
      muted = true;
      node.nodeValue = nextVal;
      muted = false;
    } else {
      lastWritten.set(node, nextVal);
    }
  }

  function processElementAttrs(el) {
    if (!el || el.nodeType !== 1) return;
    if (el.classList && (el.classList.contains('lang-tile') || el.hasAttribute('data-no-i18n'))) return;

    let oMap = origAttrs.get(el);
    let lMap = lastAttrs.get(el);

    for (let i = 0; i < ATTRS.length; i++) {
      const attr = ATTRS[i];
      if (!el.hasAttribute(attr)) continue;
      const cur = el.getAttribute(attr) || '';
      if (!cur.trim()) continue;

      const prevW = lMap ? lMap.get(attr) : undefined;
      let src;
      if (prevW !== undefined && cur === prevW && oMap && oMap.has(attr)) {
        src = oMap.get(attr);
      } else {
        src = cur;
        if (!oMap) {
          oMap = new Map();
          origAttrs.set(el, oMap);
        }
        oMap.set(attr, src);
        if (lMap) lMap.delete(attr);
      }

      if (activeLang === 'vi' || !activePack) {
        if (cur !== src) {
          muted = true;
          el.setAttribute(attr, src);
          muted = false;
        }
        if (lMap) lMap.delete(attr);
        continue;
      }

      const tr = translateCore(src.trim(), activePack);
      if (tr == null) continue;
      if (!lMap) {
        lMap = new Map();
        lastAttrs.set(el, lMap);
      }
      lMap.set(attr, tr);
      if (cur !== tr) {
        muted = true;
        el.setAttribute(attr, tr);
        muted = false;
      }
    }
  }

  function walkSubtree(root) {
    if (!root) return;
    if (root.nodeType === 3) {
      processTextNode(root);
      return;
    }
    if (root.nodeType !== 1) return;
    processElementAttrs(root);

    const walker = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT, null);
    let cur = walker.nextNode();
    while (cur) {
      if (cur.nodeType === 3) {
        processTextNode(cur);
      } else if (cur.nodeType === 1) {
        processElementAttrs(cur);
      }
      cur = walker.nextNode();
    }
  }

  function refresh() {
    const main = document.getElementById('main') || document.body;
    if (!main) return;
    walkSubtree(main);
  }

  function ensureObserver() {
    if (observer || !document.body) return;
    const target = document.getElementById('main') || document.body;
    observer = new MutationObserver((records) => {
      if (muted) return;
      for (let i = 0; i < records.length; i++) {
        const rec = records[i];
        if (rec.type === 'characterData') {
          processTextNode(rec.target);
        } else if (rec.type === 'attributes') {
          processElementAttrs(rec.target);
        } else if (rec.type === 'childList') {
          for (let j = 0; j < rec.addedNodes.length; j++) {
            walkSubtree(rec.addedNodes[j]);
          }
        }
      }
    });
    observer.observe(target, {
      subtree: true,
      childList: true,
      characterData: true,
      attributes: true,
      attributeFilter: ATTRS,
    });
  }

  function setLanguage(code) {
    const next = VALID_LANGS.has(code) ? code : 'vi';
    const myVer = ++version;
    activeLang = next;
    applyHtmlLangDir(next);
    try {
      localStorage.setItem('chatw_lang', next);
    } catch (_) {}

    if (next === 'vi') {
      activePack = null;
      refresh();
      return Promise.resolve('vi');
    }

    return fetchPack(next).then((pack) => {
      if (myVer !== version) return activeLang;
      activePack = pack;
      ensureObserver();
      refresh();
      return next;
    });
  }

  window.__chatwI18n = {
    setLanguage,
    getLanguage: () => activeLang,
    translateString,
    refresh,
    loadedLocales: () => Array.from(packCache.keys()),
    supportedCount: () => VALID_LANGS.size,
  };

  // Boot immediately from persisted language so reload renders directly in the chosen language
  const initCode = detectInitialLang();
  applyHtmlLangDir(initCode);
  const start = () => {
    ensureObserver();
    if (initCode !== 'vi') {
      setLanguage(initCode);
    }
  };
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', start, { once: true });
  } else {
    start();
  }
})();
