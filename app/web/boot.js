// Apply saved appearance before the WASM bundle boots to avoid a light/dark flash.
// (Inline scripts are blocked by the CSP, so this lives in an external module.)
(function () {
  try {
    const root = document.documentElement;
    const get = (k, d) => localStorage.getItem(k) || d;
    const pref = get('chatw_theme', 'system');
    const dark = pref === 'dark' || (pref === 'system' && window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches);
    root.setAttribute('data-theme', dark ? 'dark' : 'light');
    root.setAttribute('data-theme-pref', pref);
    root.setAttribute('data-accent', get('chatw_accent', 'blue'));
    root.setAttribute('data-density', get('chatw_density', 'comfortable'));
    const fs = get('chatw_font_size', 'auto');
    root.setAttribute('data-font-size', fs);
    if (fs !== 'auto' && /^\d+$/.test(fs)) root.style.setProperty('--font-size-root', fs + 'px');
  } catch (_) { /* storage unavailable */ }
})();

import init from '/pkg/chatw_app.js';
init();
