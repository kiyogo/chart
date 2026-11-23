/**
 * ChatW Custom Theme-Synchronized Pickers (Spec 046)
 * Replaces ugly OS/browser native popups for <select>, <input type="color">,
 * <input type="date">, and <input type="time"> with sleek, theme-synced UI
 * across both Web and Desktop while keeping all native DOM events intact.
 */
(function () {
  'use strict';
  if (window.__chatwCustomPickersInstalled) return;
  window.__chatwCustomPickersInstalled = true;

  let activePop = null;
  let activeAnchor = null;
  let activeSelect = null;

  function setNativeValue(el, val) {
    const proto = el.tagName === 'SELECT'
      ? window.HTMLSelectElement.prototype
      : window.HTMLInputElement.prototype;
    const desc = Object.getOwnPropertyDescriptor(proto, 'value');
    if (desc && desc.set) desc.set.call(el, val);
    else el.value = val;
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
  }

  function closePopover() {
    if (activeSelect && activeSelect.typeTimer) {
      clearTimeout(activeSelect.typeTimer);
    }
    activeSelect = null;
    if (activePop) {
      activePop.remove();
      activePop = null;
    }
    if (activeAnchor) {
      activeAnchor.classList.remove('cw-picker-open');
      if (activeAnchor.tagName === 'SELECT') {
        activeAnchor.setAttribute('aria-expanded', 'false');
      }
      activeAnchor = null;
    }
  }

  function createPopover(anchor, extraClass) {
    closePopover();
    const r = anchor.getBoundingClientRect();
    const dir = (document.documentElement.getAttribute('dir') || 'ltr').toLowerCase() === 'rtl' ? 'rtl' : 'ltr';
    const pop = document.createElement('div');
    pop.className = 'cw-picker-popover notranslate ' + extraClass;
    pop.setAttribute('role', 'dialog');
    pop.setAttribute('translate', 'no');
    pop.setAttribute('dir', dir);
    // Pre-position directly at anchor while invisible BEFORE attaching to DOM — zero (0,0) top-left flash!
    pop.style.visibility = 'hidden';
    pop.style.opacity = '0';
    pop.style.pointerEvents = 'none';
    pop.style.top = Math.round(r.bottom + 6) + 'px';
    pop.style.left = Math.round(r.left) + 'px';
    pop.addEventListener('mousedown', (e) => e.stopPropagation());
    document.body.appendChild(pop);
    activePop = pop;
    activeAnchor = anchor;
    anchor.classList.add('cw-picker-open');
    if (anchor.tagName === 'SELECT') {
      anchor.setAttribute('aria-expanded', 'true');
    }
    return pop;
  }

  function positionPopover(pop, anchor, matchWidth) {
    const r = anchor.getBoundingClientRect();
    if (matchWidth) {
      pop.style.minWidth = Math.max(180, Math.round(r.width)) + 'px';
      pop.style.maxWidth = Math.max(280, Math.round(r.width) + 72) + 'px';
    }
    const pr = pop.getBoundingClientRect();
    const gap = 6;
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const isRtl = (document.documentElement.getAttribute('dir') || '').toLowerCase() === 'rtl';
    let side = 'bottom';
    let top = r.bottom + gap;
    if (top + pr.height > vh - 12 && r.top - gap - pr.height >= 12) {
      top = r.top - gap - pr.height;
      side = 'top';
    } else if (top + pr.height > vh - 12) {
      top = Math.max(12, vh - 12 - pr.height);
    }
    let left = isRtl ? (r.right - pr.width) : r.left;
    if (left + pr.width > vw - 12) {
      left = Math.max(12, r.right - pr.width);
    }
    if (left < 12) {
      left = Math.max(12, Math.min(vw - 12 - pr.width, r.left));
    }
    pop.setAttribute('data-side', side);
    pop.style.top = Math.round(top) + 'px';
    pop.style.left = Math.round(left) + 'px';
    pop.style.visibility = 'visible';
    pop.style.opacity = '1';
    pop.style.pointerEvents = 'auto';
    pop.classList.add('cw-picker-ready');
  }

  // ==================== 1. CUSTOM SELECT DROPDOWN (W3C APG LISTBOX) ====================
  function openSelect(sel) {
    if (sel.disabled || sel.multiple) return;
    if (activeAnchor === sel) { closePopover(); return; }
    const pop = createPopover(sel, 'cw-select-menu');
    pop.setAttribute('role', 'listbox');
    const ariaLabel = sel.getAttribute('aria-label');
    if (ariaLabel) pop.setAttribute('aria-label', ariaLabel);

    const list = document.createElement('div');
    list.className = 'cw-select-list';
    const opts = Array.from(sel.options);
    const items = [];
    let activeIdx = -1;

    function applyHighlight(nextIdx, shouldScroll) {
      if (!items.length) return;
      if (activeSelect && activeSelect.highlightIdx >= 0 && items[activeSelect.highlightIdx]) {
        items[activeSelect.highlightIdx].btn.removeAttribute('data-highlighted');
      }
      if (nextIdx >= 0 && nextIdx < items.length && !items[nextIdx].opt.disabled) {
        if (activeSelect) activeSelect.highlightIdx = nextIdx;
        const btn = items[nextIdx].btn;
        btn.setAttribute('data-highlighted', 'true');
        if (shouldScroll) {
          const top = btn.offsetTop;
          const bottom = top + btn.offsetHeight;
          if (top < list.scrollTop) list.scrollTop = top;
          else if (bottom > list.scrollTop + list.clientHeight) list.scrollTop = bottom - list.clientHeight;
        }
      }
    }

    function commitOption(opt) {
      if (!opt || opt.disabled) return;
      const targetSel = sel;
      setNativeValue(targetSel, opt.value);
      closePopover();
      if (targetSel && typeof targetSel.focus === 'function') {
        targetSel.focus({ preventScroll: true });
      }
    }

    const isFontPicker = sel.getAttribute('data-font-picker') === 'true';
    if (isFontPicker) pop.setAttribute('data-font-menu', 'true');

    opts.forEach((opt, idx) => {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'cw-select-option' + (opt.selected ? ' selected' : '');
      btn.setAttribute('role', 'option');
      btn.setAttribute('aria-selected', opt.selected ? 'true' : 'false');
      if (isFontPicker) btn.setAttribute('data-font-preview', opt.value);
      if (opt.disabled) {
        btn.disabled = true;
        btn.setAttribute('aria-disabled', 'true');
      }

      const label = document.createElement('span');
      label.className = 'cw-select-option-label';
      const rawTxt = (opt.textContent || opt.label || opt.value || '').trim();
      const txt =
        window.__chatwI18n && typeof window.__chatwI18n.translateString === 'function'
          ? window.__chatwI18n.translateString(rawTxt)
          : rawTxt;
      label.textContent = txt;
      btn.appendChild(label);

      if (opt.selected) {
        const chk = document.createElement('span');
        chk.className = 'cw-select-check';
        chk.setAttribute('aria-hidden', 'true');
        chk.innerHTML = '<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>';
        btn.appendChild(chk);
        activeIdx = idx;
      }

      btn.addEventListener('pointerenter', () => {
        if (!opt.disabled) applyHighlight(idx, false);
      });
      btn.addEventListener('click', (e) => {
        e.preventDefault();
        e.stopPropagation();
        commitOption(opt);
      });

      items.push({ btn, opt, text: txt.toLowerCase() });
      list.appendChild(btn);
    });

    if (activeIdx < 0) {
      activeIdx = items.findIndex((it) => !it.opt.disabled);
    }

    activeSelect = {
      sel,
      pop,
      list,
      items,
      highlightIdx: activeIdx,
      typeBuf: '',
      typeTimer: null,
      applyHighlight,
      commitOption,
    };

    pop.appendChild(list);
    positionPopover(pop, sel, true);
    if (activeIdx >= 0 && items[activeIdx]) {
      items[activeIdx].btn.setAttribute('data-highlighted', 'true');
      const activeBtn = items[activeIdx].btn;
      list.scrollTop = Math.max(0, activeBtn.offsetTop - list.clientHeight / 2 + activeBtn.offsetHeight / 2);
    }
  }

  // ==================== 2. CUSTOM COLOR PICKER ====================
  function hexToRgb(hex) {
    const h = (hex || '#2764E7').replace('#', '').trim();
    if (!/^[0-9a-fA-F]{6}$/.test(h)) return [39, 100, 231];
    return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
  }
  function rgbToHex(r, g, b) {
    return '#' + [r, g, b].map((v) => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, '0')).join('').toUpperCase();
  }
  function rgbToHsv(r, g, b) {
    r /= 255; g /= 255; b /= 255;
    const max = Math.max(r, g, b), min = Math.min(r, g, b), d = max - min;
    let h = 0;
    const s = max === 0 ? 0 : d / max;
    const v = max;
    if (d !== 0) {
      if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) * 60;
      else if (max === g) h = ((b - r) / d + 2) * 60;
      else h = ((r - g) / d + 4) * 60;
    }
    return [h, s, v];
  }
  function hsvToRgb(h, s, v) {
    const c = v * s;
    const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
    const m = v - c;
    let r = 0, g = 0, b = 0;
    if (h < 60) { r = c; g = x; }
    else if (h < 120) { r = x; g = c; }
    else if (h < 180) { g = c; b = x; }
    else if (h < 240) { g = x; b = c; }
    else if (h < 300) { r = x; b = c; }
    else { r = c; b = x; }
    return [(r + m) * 255, (g + m) * 255, (b + m) * 255];
  }

  const QUICK_SWATCHES = [
    '#2764E7', '#6D4AE0', '#0EA5E9', '#138A5A',
    '#10B981', '#D9620F', '#F59E0B', '#E5484D',
    '#D6336C', '#EC4899', '#475569', '#111827',
  ];

  function openColorPicker(input) {
    if (input.disabled) return;
    if (activeAnchor === input) { closePopover(); return; }
    const pop = createPopover(input, 'cw-color-picker');

    let [h, s, v] = rgbToHsv(...hexToRgb(input.value || '#2764E7'));

    const svBox = document.createElement('div');
    svBox.className = 'cw-cp-sv';
    const svCursor = document.createElement('span');
    svCursor.className = 'cw-cp-cursor';
    svBox.appendChild(svCursor);

    const hueBar = document.createElement('div');
    hueBar.className = 'cw-cp-hue';
    const hueThumb = document.createElement('span');
    hueThumb.className = 'cw-cp-hue-thumb';
    hueBar.appendChild(hueThumb);

    const swatches = document.createElement('div');
    swatches.className = 'cw-cp-swatches';

    const foot = document.createElement('div');
    foot.className = 'cw-cp-foot';
    const preview = document.createElement('span');
    preview.className = 'cw-cp-preview';
    const hexInput = document.createElement('input');
    hexInput.type = 'text';
    hexInput.className = 'cw-cp-hex';
    hexInput.maxLength = 7;
    hexInput.setAttribute('aria-label', 'Color hex');
    const doneBtn = document.createElement('button');
    doneBtn.type = 'button';
    doneBtn.className = 'btn-primary cw-cp-done';
    doneBtn.textContent = 'Done';
    doneBtn.addEventListener('click', closePopover);
    foot.append(preview, hexInput, doneBtn);

    function syncUI(emit) {
      const hueRgb = hsvToRgb(h, 1, 1);
      svBox.style.backgroundColor = rgbToHex(...hueRgb);
      svCursor.style.left = (s * 100) + '%';
      svCursor.style.top = ((1 - v) * 100) + '%';
      hueThumb.style.left = ((h / 360) * 100) + '%';
      const hex = rgbToHex(...hsvToRgb(h, s, v));
      preview.style.backgroundColor = hex;
      if (document.activeElement !== hexInput) hexInput.value = hex;
      if (emit) setNativeValue(input, hex);
    }

    QUICK_SWATCHES.forEach((sw) => {
      const b = document.createElement('button');
      b.type = 'button';
      b.className = 'cw-cp-swatch';
      b.style.backgroundColor = sw;
      b.title = sw;
      b.addEventListener('click', () => {
        [h, s, v] = rgbToHsv(...hexToRgb(sw));
        syncUI(true);
      });
      swatches.appendChild(b);
    });

    function bindDrag(el, onMove) {
      el.addEventListener('pointerdown', (e) => {
        e.preventDefault();
        el.setPointerCapture(e.pointerId);
        onMove(e);
        const mm = (ev) => onMove(ev);
        const mu = () => {
          el.removeEventListener('pointermove', mm);
          el.removeEventListener('pointerup', mu);
        };
        el.addEventListener('pointermove', mm);
        el.addEventListener('pointerup', mu);
      });
    }

    bindDrag(svBox, (e) => {
      const r = svBox.getBoundingClientRect();
      s = Math.max(0, Math.min(1, (e.clientX - r.left) / r.width));
      v = Math.max(0, Math.min(1, 1 - (e.clientY - r.top) / r.height));
      syncUI(true);
    });

    bindDrag(hueBar, (e) => {
      const r = hueBar.getBoundingClientRect();
      h = Math.max(0, Math.min(359.9, ((e.clientX - r.left) / r.width) * 360));
      syncUI(true);
    });

    hexInput.addEventListener('input', () => {
      const val = hexInput.value.trim();
      if (/^#[0-9a-fA-F]{6}$/.test(val)) {
        [h, s, v] = rgbToHsv(...hexToRgb(val));
        syncUI(true);
      }
    });

    pop.append(svBox, hueBar, swatches, foot);
    syncUI(false);
    positionPopover(pop, input, false);
  }

  // ==================== 3. CUSTOM LOCALIZED DATE PICKER ====================
  function getLocalizedWeekdays() {
    const lang = document.documentElement.getAttribute('lang') || 'en';
    try {
      const fmt = new Intl.DateTimeFormat(lang, { weekday: 'short' });
      // 2024-01-01 is Monday
      return [0, 1, 2, 3, 4, 5, 6].map((d) => fmt.format(new Date(Date.UTC(2024, 0, 1 + d))));
    } catch (_) {
      return ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
    }
  }

  function formatMonthYear(year, month) {
    const lang = document.documentElement.getAttribute('lang') || 'en';
    try {
      return new Intl.DateTimeFormat(lang, { month: 'long', year: 'numeric' }).format(new Date(year, month, 1));
    } catch (_) {
      return `${year}-${String(month + 1).padStart(2, '0')}`;
    }
  }

  function openDatePicker(input) {
    if (input.disabled) return;
    if (activeAnchor === input) { closePopover(); return; }
    const pop = createPopover(input, 'cw-date-picker');

    const now = new Date();
    let selYear = null, selMonth = null, selDay = null;
    if (/^\d{4}-\d{2}-\d{2}$/.test(input.value)) {
      const parts = input.value.split('-').map(Number);
      selYear = parts[0]; selMonth = parts[1] - 1; selDay = parts[2];
    }
    let viewYear = selYear ?? now.getFullYear();
    let viewMonth = selMonth ?? now.getMonth();

    function pad2(n) { return String(n).padStart(2, '0'); }

    function render() {
      pop.innerHTML = '';
      const head = document.createElement('div');
      head.className = 'cw-dp-head';
      const prev = document.createElement('button');
      prev.type = 'button'; prev.className = 'icon-btn cw-dp-nav'; prev.textContent = '‹';
      prev.addEventListener('click', () => { viewMonth--; if (viewMonth < 0) { viewMonth = 11; viewYear--; } render(); });
      const title = document.createElement('span');
      title.className = 'cw-dp-title';
      title.textContent = formatMonthYear(viewYear, viewMonth);
      const next = document.createElement('button');
      next.type = 'button'; next.className = 'icon-btn cw-dp-nav'; next.textContent = '›';
      next.addEventListener('click', () => { viewMonth++; if (viewMonth > 11) { viewMonth = 0; viewYear++; } render(); });
      head.append(prev, title, next);

      const wk = document.createElement('div');
      wk.className = 'cw-dp-grid cw-dp-weekdays';
      getLocalizedWeekdays().forEach((d) => {
        const s = document.createElement('span');
        s.textContent = d;
        wk.appendChild(s);
      });

      const grid = document.createElement('div');
      grid.className = 'cw-dp-grid';
      const firstDow = (new Date(viewYear, viewMonth, 1).getDay() + 6) % 7; // Mon=0..Sun=6
      const daysInMonth = new Date(viewYear, viewMonth + 1, 0).getDate();

      for (let i = 0; i < firstDow; i++) {
        grid.appendChild(document.createElement('span'));
      }
      for (let d = 1; d <= daysInMonth; d++) {
        const btn = document.createElement('button');
        btn.type = 'button';
        btn.className = 'cw-dp-day';
        btn.textContent = String(d);
        const isToday = viewYear === now.getFullYear() && viewMonth === now.getMonth() && d === now.getDate();
        const isSel = viewYear === selYear && viewMonth === selMonth && d === selDay;
        if (isToday) btn.classList.add('today');
        if (isSel) btn.classList.add('selected');
        btn.addEventListener('click', () => {
          setNativeValue(input, `${viewYear}-${pad2(viewMonth + 1)}-${pad2(d)}`);
          closePopover();
        });
        grid.appendChild(btn);
      }

      const foot = document.createElement('div');
      foot.className = 'cw-dp-foot';
      const clearBtn = document.createElement('button');
      clearBtn.type = 'button'; clearBtn.className = 'btn-ghost'; clearBtn.textContent = 'Clear';
      clearBtn.addEventListener('click', () => { setNativeValue(input, ''); closePopover(); });
      const todayBtn = document.createElement('button');
      todayBtn.type = 'button'; todayBtn.className = 'btn-secondary'; todayBtn.textContent = 'Today';
      todayBtn.addEventListener('click', () => {
        setNativeValue(input, `${now.getFullYear()}-${pad2(now.getMonth() + 1)}-${pad2(now.getDate())}`);
        closePopover();
      });
      foot.append(clearBtn, todayBtn);

      pop.append(head, wk, grid, foot);
    }

    render();
    positionPopover(pop, input, false);
  }

  // ==================== 4. CUSTOM TIME PICKER ====================
  function openTimePicker(input) {
    if (input.disabled) return;
    if (activeAnchor === input) { closePopover(); return; }
    const pop = createPopover(input, 'cw-time-picker');

    const now = new Date();
    let curH = now.getHours(), curM = now.getMinutes();
    if (/^\d{2}:\d{2}/.test(input.value)) {
      const p = input.value.split(':').map(Number);
      curH = p[0]; curM = p[1];
    }
    const pad2 = (n) => String(n).padStart(2, '0');

    const cols = document.createElement('div');
    cols.className = 'cw-tp-cols';

    const hCol = document.createElement('div');
    hCol.className = 'cw-tp-col';
    const hHead = document.createElement('div');
    hHead.className = 'cw-tp-col-title'; hHead.textContent = 'Hour';
    const hList = document.createElement('div');
    hList.className = 'cw-tp-list';
    hCol.append(hHead, hList);

    const mCol = document.createElement('div');
    mCol.className = 'cw-tp-col';
    const mHead = document.createElement('div');
    mHead.className = 'cw-tp-col-title'; mHead.textContent = 'Minute';
    const mList = document.createElement('div');
    mList.className = 'cw-tp-list';
    mCol.append(mHead, mList);

    function emit() {
      setNativeValue(input, `${pad2(curH)}:${pad2(curM)}`);
    }

    let selHBtn = null, selMBtn = null;
    for (let h = 0; h < 24; h++) {
      const b = document.createElement('button');
      b.type = 'button';
      b.className = 'cw-tp-item' + (h === curH ? ' selected' : '');
      b.textContent = pad2(h);
      if (h === curH) selHBtn = b;
      b.addEventListener('click', () => {
        curH = h;
        hList.querySelectorAll('.selected').forEach((x) => x.classList.remove('selected'));
        b.classList.add('selected');
        emit();
      });
      hList.appendChild(b);
    }
    for (let m = 0; m < 60; m++) {
      const b = document.createElement('button');
      b.type = 'button';
      b.className = 'cw-tp-item' + (m === curM ? ' selected' : '');
      b.textContent = pad2(m);
      if (m === curM) selMBtn = b;
      b.addEventListener('click', () => {
        curM = m;
        mList.querySelectorAll('.selected').forEach((x) => x.classList.remove('selected'));
        b.classList.add('selected');
        emit();
      });
      mList.appendChild(b);
    }

    cols.append(hCol, mCol);
    const foot = document.createElement('div');
    foot.className = 'cw-dp-foot';
    const nowBtn = document.createElement('button');
    nowBtn.type = 'button'; nowBtn.className = 'btn-ghost'; nowBtn.textContent = 'Now';
    nowBtn.addEventListener('click', () => {
      const n = new Date();
      curH = n.getHours(); curM = n.getMinutes();
      emit(); closePopover();
    });
    const okBtn = document.createElement('button');
    okBtn.type = 'button'; okBtn.className = 'btn-primary'; okBtn.textContent = 'Done';
    okBtn.addEventListener('click', closePopover);
    foot.append(nowBtn, okBtn);

    pop.append(cols, foot);
    positionPopover(pop, input, false);
    if (selHBtn) hList.scrollTop = Math.max(0, selHBtn.offsetTop - hList.clientHeight / 2 + selHBtn.offsetHeight / 2);
    if (selMBtn) mList.scrollTop = Math.max(0, selMBtn.offsetTop - mList.clientHeight / 2 + selMBtn.offsetHeight / 2);
  }

  // ==================== GLOBAL INTERCEPTORS ====================
  document.addEventListener('mousedown', (e) => {
    if (e.button !== 0) return;
    if (activePop && activePop.contains(e.target)) return;

    const sel = e.target.closest ? e.target.closest('select') : null;
    if (sel && !sel.disabled && !sel.multiple) {
      e.preventDefault();
      sel.focus();
      openSelect(sel);
      return;
    }

    const inp = e.target.closest ? e.target.closest('input') : null;
    if (inp && !inp.disabled) {
      if (inp.type === 'color') {
        e.preventDefault();
        openColorPicker(inp);
        return;
      }
      if (inp.type === 'date') {
        e.preventDefault();
        openDatePicker(inp);
        return;
      }
      if (inp.type === 'time') {
        e.preventDefault();
        openTimePicker(inp);
        return;
      }
    }

    closePopover();
  }, true);

  // Also block native OS dialog on click for color/date/time inputs
  document.addEventListener('click', (e) => {
    if (!e.isTrusted) return;
    const inp = e.target.closest ? e.target.closest('input') : null;
    if (inp && !inp.disabled && (inp.type === 'color' || inp.type === 'date' || inp.type === 'time')) {
      e.preventDefault();
    }
  }, true);

  function findEnabledIndex(items, startIdx, step) {
    const n = items.length;
    if (!n) return -1;
    for (let i = 0; i < n; i++) {
      const idx = (startIdx + step * (i + 1) + n * 4) % n;
      if (!items[idx].opt.disabled) return idx;
    }
    return -1;
  }

  document.addEventListener('keydown', (e) => {
    if (activeSelect && activePop) {
      const st = activeSelect;
      if (e.key === 'Escape') {
        e.preventDefault();
        const a = st.sel;
        closePopover();
        if (a && typeof a.focus === 'function') a.focus({ preventScroll: true });
        return;
      }
      if (e.key === 'Tab') {
        closePopover();
        return;
      }
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        const next = findEnabledIndex(st.items, st.highlightIdx < 0 ? -1 : st.highlightIdx, 1);
        if (next >= 0) st.applyHighlight(next, true);
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        const prev = findEnabledIndex(st.items, st.highlightIdx < 0 ? 0 : st.highlightIdx, -1);
        if (prev >= 0) st.applyHighlight(prev, true);
        return;
      }
      if (e.key === 'Home') {
        e.preventDefault();
        const first = st.items.findIndex((it) => !it.opt.disabled);
        if (first >= 0) st.applyHighlight(first, true);
        return;
      }
      if (e.key === 'End') {
        e.preventDefault();
        for (let i = st.items.length - 1; i >= 0; i--) {
          if (!st.items[i].opt.disabled) { st.applyHighlight(i, true); break; }
        }
        return;
      }
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        const cur = st.items[st.highlightIdx];
        if (cur && !cur.opt.disabled) st.commitOption(cur.opt);
        return;
      }
      if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
        e.preventDefault();
        st.typeBuf += e.key.toLowerCase();
        if (st.typeTimer) clearTimeout(st.typeTimer);
        st.typeTimer = setTimeout(() => { if (activeSelect === st) st.typeBuf = ''; }, 500);
        const match = st.items.findIndex((it) => !it.opt.disabled && it.text.startsWith(st.typeBuf));
        if (match >= 0) st.applyHighlight(match, true);
        return;
      }
    }

    if (e.key === 'Escape' && activePop) {
      e.preventDefault();
      const a = activeAnchor;
      closePopover();
      if (a && typeof a.focus === 'function') a.focus({ preventScroll: true });
      return;
    }
    const t = e.target;
    if (t && t.tagName === 'SELECT' && !t.disabled && !t.multiple && (e.key === 'Enter' || e.key === ' ' || e.key === 'ArrowDown' || e.key === 'ArrowUp')) {
      e.preventDefault();
      openSelect(t);
    }
  }, true);

  window.addEventListener('resize', closePopover);
  window.addEventListener('scroll', (e) => {
    if (activePop && e.target instanceof Node && activePop.contains(e.target)) return;
    closePopover();
  }, true);
})();
