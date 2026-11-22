/* ChatW 197-Language Global i18n Runtime (Spec 064)
 * Canonical source language in all code is English ('en').
 * Lazy-loads /locales/_keys.json + compact indexed /locales/{code}.json packs,
 * enforces translate="no" / notranslate against browser auto-translate corruption,
 * applies Dioxus-safe WeakMap-tracked translations across all UI text nodes,
 * dynamic templates, CLDR dates, document.title, and attributes while strictly
 * protecting user-generated content.
 */
(function () {
  if (window.__chatwI18n) return;

  const RTL_LANGS = new Set(["ar", "ar-EG", "ur", "fa", "he", "ps", "ckb", "sd", "bal", "ug", "yi", "dv"]);
  const VALID_LANGS = new Set(["en", "vi", "zh-CN", "zh-TW", "yue", "ja", "ko", "es", "fr", "de", "pt-BR", "pt-PT", "ru", "ar", "ar-EG", "hi", "bn", "ur", "id", "ms", "fil", "th", "km", "lo", "my", "tr", "it", "nl", "pl", "uk", "ro", "cs", "sk", "hu", "el", "sv", "da", "no", "fi", "is", "fa", "he", "ps", "ku", "ckb", "sd", "bal", "ug", "yi", "dv", "ta", "te", "mr", "gu", "kn", "ml", "pa", "or", "as", "ne", "new", "si", "sa", "mai", "bho", "doi", "gom", "mni-Mtei", "lus", "sat", "kha", "bo", "dz", "ka", "hy", "az", "kk", "uz", "ky", "tk", "tg", "mn", "tt", "ba", "cv", "ce", "os", "av", "crh", "sah", "bua", "kv", "ab", "bg", "sr", "hr", "bs", "sl", "mk", "sq", "lt", "lv", "et", "be", "ga", "cy", "gd", "fo", "kl", "eu", "ca", "gl", "mt", "lb", "fy", "co", "br", "oc", "scn", "lmo", "vec", "fur", "lij", "eo", "la", "af", "sw", "am", "ti", "om", "so", "ha", "yo", "ig", "zu", "xh", "st", "sn", "ny", "rw", "rn", "lg", "mg", "bm", "ee", "ak", "ln", "nso", "ts", "tn", "ss", "ve", "nr", "kri", "wo", "ff", "sg", "kg", "lua", "bem", "tum", "ach", "luo", "ndc-ZW", "ber-Latn", "jv", "su", "ban", "min", "ace", "mak", "bbc", "btx", "bts", "ceb", "ilo", "pam", "pag", "hil", "bik", "hmn", "shn", "kac", "tet", "mi", "sm", "haw", "fj", "to", "ty", "tpi", "ht", "pap", "jam", "qu", "gn", "ay"]);

  const LANG_ALIASES = {
    tl: 'fil', 'tl-ph': 'fil', 'fil-ph': 'fil',
    jw: 'jv', 'jv-id': 'jv',
    nb: 'no', 'nb-no': 'no', nn: 'no', 'nn-no': 'no', 'no-no': 'no',
    zh: 'zh-CN', 'zh-hans': 'zh-CN', 'zh-sg': 'zh-CN',
    'zh-hant': 'zh-TW', 'zh-hk': 'zh-TW', 'zh-mo': 'zh-TW',
    pt: 'pt-BR',
    iw: 'he', 'he-il': 'he',
    in: 'id',
  };

  const ATTRS = ['placeholder', 'aria-label', 'data-tip', 'title'];

  const EN_MONTH_MAP = {
    January: 1, February: 2, March: 3, April: 4, May: 5, June: 6,
    July: 7, August: 8, September: 9, October: 10, November: 11, December: 12,
  };
  const EN_DATE_FULL_RE = /^(January|February|March|April|May|June|July|August|September|October|November|December)\s+(\d{1,2}),\s*(\d{4})$/;
  const EN_DATE_SHORT_RE = /^(January|February|March|April|May|June|July|August|September|October|November|December)\s+(\d{1,2})$/;

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

  let keysPromise = null;
  let keysData = null;           // { exact: [...], templates: [...] }
  let compiledTemplates = null;  // Array<{ idx, re, anchor, prefix }>
  let prefixBuckets = null;      // Map<firstWordLower, Array<entry>>
  let genericTemplates = null;   // Array<entry> (templates starting with {0})

  const packCache = new Map();   // code -> { exactMap: Map<en, tr>, templates: Array<trStr> }
  const inflight = new Map();    // code -> Promise

  const origText = new WeakMap();    // Text -> canonical English string
  const lastWritten = new WeakMap(); // Text -> last translated string written by us
  const origAttrs = new WeakMap();   // Element -> Map<attr, origEn>
  const lastAttrs = new WeakMap();   // Element -> Map<attr, written>
  const origDocTitle = document.title || 'ChatW - Private End-to-End Encrypted Messenger';

  let activeLang = 'en';
  let activePack = null;
  let observer = null;
  let muted = false;
  let version = 0;

  function normalizeLang(raw) {
    if (!raw || typeof raw !== 'string') return null;
    const trimmed = raw.trim();
    if (VALID_LANGS.has(trimmed)) return trimmed;
    const low = trimmed.toLowerCase();
    for (const c of VALID_LANGS) {
      if (c.toLowerCase() === low) return c;
    }
    if (LANG_ALIASES[low]) return LANG_ALIASES[low];
    const dash = low.indexOf('-');
    if (dash > 0) {
      const base = low.slice(0, dash);
      if (VALID_LANGS.has(base)) return base;
      if (LANG_ALIASES[base]) return LANG_ALIASES[base];
    }
    return null;
  }

  function detectInitialLang() {
    try {
      const direct = normalizeLang(localStorage.getItem('chatw_lang'));
      if (direct) return direct;
      for (let i = 0; i < localStorage.length; i++) {
        const k = localStorage.key(i);
        if (k && k.startsWith('chatw_prefs_')) {
          const parsed = JSON.parse(localStorage.getItem(k) || '{}');
          const found = normalizeLang(parsed && parsed.language);
          if (found) return found;
        }
      }
    } catch (_) {}
    return 'en';
  }

  function enforceNoBrowserTranslate() {
    const html = document.documentElement;
    if (html.getAttribute('translate') !== 'no') html.setAttribute('translate', 'no');
    if (!html.classList.contains('notranslate')) html.classList.add('notranslate');
    if (document.body) {
      if (document.body.getAttribute('translate') !== 'no') document.body.setAttribute('translate', 'no');
      if (!document.body.classList.contains('notranslate')) document.body.classList.add('notranslate');
    }
    const main = document.getElementById('main');
    if (main) {
      if (main.getAttribute('translate') !== 'no') main.setAttribute('translate', 'no');
      if (!main.classList.contains('notranslate')) main.classList.add('notranslate');
    }
    if (document.head && !document.head.querySelector('meta[name="google"][content="notranslate"]')) {
      const meta = document.createElement('meta');
      meta.name = 'google';
      meta.content = 'notranslate';
      document.head.appendChild(meta);
    }
  }

  function applyHtmlLangDir(code) {
    const html = document.documentElement;
    const dir = RTL_LANGS.has(code) ? 'rtl' : 'ltr';
    if (html.getAttribute('lang') !== code) html.setAttribute('lang', code);
    if (html.getAttribute('dir') !== dir) html.setAttribute('dir', dir);
    enforceNoBrowserTranslate();
  }

  function escapeRegex(s) {
    return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  }

  function compileTemplates(templates) {
    compiledTemplates = [];
    prefixBuckets = new Map();
    genericTemplates = [];
    for (let i = 0; i < templates.length; i++) {
      const tpl = templates[i];
      if (!tpl) continue;
      // Split by {0}, {1}, ...
      const parts = tpl.split(/\{\d+\}/g);
      const phMatches = tpl.match(/\{\d+\}/g);
      if (!phMatches || !phMatches.length) continue;
      let reStr = '^';
      for (let p = 0; p < parts.length; p++) {
        reStr += escapeRegex(parts[p]).replace(/\s+/g, '\\s+');
        if (p < phMatches.length) {
          // Non-greedy capture for intermediate slots, greedy for trailing slot
          reStr += p === phMatches.length - 1 && !parts[p + 1] ? '(.+)' : '(.+?)';
        }
      }
      reStr += '$';
      const phOrder = phMatches.map((m) => Number(m.slice(1, -1)));
      // Pick longest alphabetic static fragment as fast substring anchor
      let anchor = '';
      for (let p = 0; p < parts.length; p++) {
        const seg = parts[p].trim();
        if (seg.length > anchor.length && /[A-Za-z]/.test(seg)) anchor = seg;
      }
      let re;
      try {
        re = new RegExp(reStr);
      } catch (_) {
        continue;
      }
      const entry = { idx: i, re, phOrder, anchor };
      compiledTemplates.push(entry);
      const lead = parts[0].trim();
      const fw = lead.match(/^[A-Za-z]+/);
      if (fw) {
        const key = fw[0].toLowerCase();
        if (!prefixBuckets.has(key)) prefixBuckets.set(key, []);
        prefixBuckets.get(key).push(entry);
      } else {
        genericTemplates.push(entry);
      }
    }
  }

  function fetchKeys() {
    if (keysData) return Promise.resolve(keysData);
    if (keysPromise) return keysPromise;
    keysPromise = fetch('/locales/_keys.json', { cache: 'force-cache' })
      .then((res) => {
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        return res.json();
      })
      .then((data) => {
        keysData = {
          exact: (data && data.exact) || [],
          templates: (data && data.templates) || [],
        };
        compileTemplates(keysData.templates);
        return keysData;
      })
      .catch((err) => {
        keysPromise = null;
        console.warn('[chatw-i18n] failed to load _keys.json:', err);
        return null;
      });
    return keysPromise;
  }

  function fetchPack(code) {
    if (code === 'en') return Promise.resolve(null);
    if (packCache.has(code)) return Promise.resolve(packCache.get(code));
    if (inflight.has(code)) return inflight.get(code);

    const p = Promise.all([
      fetchKeys(),
      fetch(`/locales/${encodeURIComponent(code)}.json`, { cache: 'force-cache' }).then((res) => {
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        return res.json();
      }),
    ])
      .then(([keys, data]) => {
        if (!keys || !data) throw new Error('Invalid locale payload');
        const exactMap = new Map();
        const kEx = keys.exact;
        const vEx = data.exact || [];
        const len = Math.min(kEx.length, vEx.length);
        for (let i = 0; i < len; i++) {
          exactMap.set(kEx[i], vEx[i]);
        }
        const pack = {
          exactMap,
          templates: data.templates || [],
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

  function formatLocalizedDate(str, lang) {
    let m = str.match(EN_DATE_FULL_RE);
    if (m) {
      const mon = EN_MONTH_MAP[m[1]];
      const day = Number(m[2]);
      const year = Number(m[3]);
      if (lang === 'vi') return `${day} tháng ${mon} năm ${year}`;
      try {
        return new Intl.DateTimeFormat(lang, { year: 'numeric', month: 'long', day: 'numeric' }).format(
          new Date(year, mon - 1, day)
        );
      } catch (_) {
        return str;
      }
    }
    m = str.match(EN_DATE_SHORT_RE);
    if (m) {
      const mon = EN_MONTH_MAP[m[1]];
      const day = Number(m[2]);
      if (lang === 'vi') return `${day} tháng ${mon}`;
      try {
        return new Intl.DateTimeFormat(lang, { month: 'long', day: 'numeric' }).format(
          new Date(2026, mon - 1, day)
        );
      } catch (_) {
        return str;
      }
    }
    return null;
  }

  function applyTemplateCandidates(str, candidates, pack) {
    if (!candidates || !candidates.length) return null;
    for (let i = 0; i < candidates.length; i++) {
      const c = candidates[i];
      if (c.anchor && !str.includes(c.anchor)) continue;
      const outTpl = pack.templates[c.idx];
      if (!outTpl) continue;
      const m = str.match(c.re);
      if (m) {
        let res = outTpl;
        for (let g = 0; g < c.phOrder.length; g++) {
          const val = m[g + 1] != null ? m[g + 1] : '';
          res = res.replace(`{${c.phOrder[g]}}`, val);
        }
        return res;
      }
    }
    return null;
  }

  function translateCore(str, pack) {
    if (!pack || !str) return null;
    if (pack.exactMap.has(str)) {
      return pack.exactMap.get(str);
    }
    const dt = formatLocalizedDate(str, activeLang);
    if (dt != null) return dt;
    if (prefixBuckets) {
      const fw = str.match(/^[A-Za-z]+/);
      if (fw) {
        const hit = applyTemplateCandidates(str, prefixBuckets.get(fw[0].toLowerCase()), pack);
        if (hit != null) return hit;
      }
    }
    if (genericTemplates) {
      const hit = applyTemplateCandidates(str, genericTemplates, pack);
      if (hit != null) return hit;
    }
    return null;
  }

  function translateString(raw) {
    if (!raw || typeof raw !== 'string') return raw;
    if (activeLang === 'en' || !activePack) return raw;
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
    // Allow preview chat bubbles inside Appearance preview, skip real chat messages
    const bubble = el.closest('.message-bubble');
    if (bubble && !bubble.closest('.settings-preview-chat')) return true;
    // Allow "Note to self" system name in chat-item-name / header, skip arbitrary usernames
    const chatName = el.closest('.chat-item-name, .chat-header-title');
    if (chatName) {
      const txt = (origText.get(node) ?? node.nodeValue ?? '').trim();
      if (txt !== 'Note to self') return true;
    }
    // Skip arbitrary user message previews, translate system Note-to-self default subtitle & system preview labels
    const preview = el.closest('.chat-item-preview');
    if (preview) {
      const txt = (origText.get(node) ?? node.nodeValue ?? '').trim();
      if (
        txt !== 'Notes, files, and links synced across your devices' &&
        txt !== 'Voted' &&
        txt !== 'Sent a sticker'
      ) {
        return true;
      }
    }
    return false;
  }

  function processTextNode(node) {
    if (!node || node.nodeType !== 3) return;
    if (isTextNodeProtected(node)) return;

    const currentVal = node.nodeValue || '';
    if (!currentVal.trim()) return;

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

    if (activeLang === 'en' || !activePack) {
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

      if (activeLang === 'en' || !activePack) {
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

  function updateDocumentTitle() {
    if (activeLang === 'en' || !activePack) {
      if (document.title !== origDocTitle) document.title = origDocTitle;
      return;
    }
    const tr = translateCore(origDocTitle, activePack);
    if (tr && document.title !== tr) document.title = tr;
  }

  function refresh() {
    enforceNoBrowserTranslate();
    updateDocumentTitle();
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
    const next = normalizeLang(code) || 'en';
    const myVer = ++version;
    activeLang = next;
    applyHtmlLangDir(next);
    try {
      localStorage.setItem('chatw_lang', next);
    } catch (_) {}

    if (next === 'en') {
      activePack = null;
      refresh();
      return Promise.resolve('en');
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
    supportedLanguages: () => Array.from(VALID_LANGS),
  };

  const initCode = detectInitialLang();
  applyHtmlLangDir(initCode);
  const start = () => {
    enforceNoBrowserTranslate();
    ensureObserver();
    if (initCode !== 'en') {
      setLanguage(initCode);
    }
  };
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', start, { once: true });
  } else {
    start();
  }
})();
