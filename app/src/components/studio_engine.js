// ChatW Studio engine: layers × frames of canvases, drawing tools, history,
// onion skin, playback, motion tweens and export helpers. Loaded once with
// `include_str!` and driven from Rust through `window.ChatwStudio`.
(function () {
  if (window.ChatwStudio) return;
  const MAX_LAYERS = 12, MAX_FRAMES = 60, MAX_HISTORY = 40;
  const BLENDS = { normal: 'source-over', multiply: 'multiply', screen: 'screen', overlay: 'overlay', darken: 'darken', lighten: 'lighten' };
  const EASE = {
    linear: (t) => t,
    'ease-in': (t) => t * t,
    'ease-out': (t) => 1 - (1 - t) * (1 - t),
    'ease-in-out': (t) => (t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2),
    bounce: (t) => {
      const n = 7.5625, d = 2.75;
      if (t < 1 / d) return n * t * t;
      if (t < 2 / d) return n * (t -= 1.5 / d) * t + 0.75;
      if (t < 2.5 / d) return n * (t -= 2.25 / d) * t + 0.9375;
      return n * (t -= 2.625 / d) * t + 0.984375;
    },
    elastic: (t) => (t === 0 || t === 1 ? t : Math.pow(2, -10 * t) * Math.sin((t * 10 - 0.75) * (2 * Math.PI) / 3) + 1),
  };

  const S = {
    doc: null, frame: 0, layer: 0,
    tool: 'brush', color: '#2764E7', size: 8, opacity: 1, fill: false, tolerance: 24,
    text: 'ChatW', font: 32, sides: 5, star: true,
    zoom: 1, grid: false, gridSize: 16, snap: false, onion: true,
    undo: [], redo: [], playing: false, timer: null, recent: [],
    view: null, overlay: null, notify: null, draftKey: null, draftTimer: null, draftState: '',
    drag: null,
  };

  const mk = (w, h) => { const c = document.createElement('canvas'); c.width = w; c.height = h; c.__used = false; return c; };
  const cell = (f, l) => {
    const fr = S.doc.frames[f];
    if (!fr.cells[l]) fr.cells[l] = mk(S.doc.w, S.doc.h);
    return fr.cells[l];
  };
  const clone = (c) => { const n = mk(c.width, c.height); n.getContext('2d').drawImage(c, 0, 0); n.__used = c.__used; return n; };
  const cloneDoc = () => ({
    w: S.doc.w, h: S.doc.h, fps: S.doc.fps, loop: S.doc.loop,
    layers: S.doc.layers.map((l) => ({ ...l })),
    frames: S.doc.frames.map((f) => ({ cells: f.cells.map((c) => (c ? clone(c) : null)) })),
    frame: S.frame, layer: S.layer,
  });
  const restoreDoc = (d) => {
    S.doc = { w: d.w, h: d.h, fps: d.fps, loop: d.loop, layers: d.layers.map((l) => ({ ...l })), frames: d.frames.map((f) => ({ cells: f.cells.slice() })) };
    S.frame = Math.min(d.frame, S.doc.frames.length - 1);
    S.layer = Math.min(d.layer, S.doc.layers.length - 1);
  };

  // ---------- history ----------
  const pushHistory = (entry) => { S.undo.push(entry); if (S.undo.length > MAX_HISTORY) S.undo.shift(); S.redo = []; };
  const snapCell = (f, l) => pushHistory({ t: 'cell', f, l, c: clone(cell(f, l)) });
  const snapDoc = () => pushHistory({ t: 'doc', d: cloneDoc() });
  const swap = (from, to) => {
    const e = from.pop();
    if (!e) return;
    if (e.t === 'cell') {
      if (!S.doc.frames[e.f] || e.l >= S.doc.layers.length) return;
      to.push({ t: 'cell', f: e.f, l: e.l, c: clone(cell(e.f, e.l)) });
      S.doc.frames[e.f].cells[e.l] = e.c;
      S.frame = e.f; S.layer = e.l;
    } else {
      to.push({ t: 'doc', d: cloneDoc() });
      restoreDoc(e.d);
    }
    changed();
  };

  // ---------- rendering ----------
  const composite = (ctx, f, opts) => {
    const o = opts || {};
    S.doc.layers.forEach((L, i) => {
      if (!L.visible && !o.all) return;
      const c = S.doc.frames[f].cells[i];
      if (!c) return;
      ctx.save();
      ctx.globalAlpha = L.opacity * (o.alpha == null ? 1 : o.alpha);
      ctx.globalCompositeOperation = BLENDS[L.blend] || 'source-over';
      ctx.drawImage(c, 0, 0);
      if (o.liveStroke && i === S.layer && S.drag && S.drag.stroke) {
        ctx.globalAlpha = S.drag.eraser ? 1 : S.opacity * L.opacity;
        ctx.globalCompositeOperation = S.drag.eraser ? 'destination-out' : 'source-over';
        // Eraser preview: punch through only this layer's own pixels.
        if (!S.drag.eraser) ctx.drawImage(S.drag.stroke, 0, 0);
      }
      ctx.restore();
    });
  };
  const frameCanvas = (f) => { const c = mk(S.doc.w, S.doc.h); composite(c.getContext('2d'), f); return c; };
  const render = () => {
    if (!S.view || !S.doc) return;
    const ctx = S.view.getContext('2d');
    ctx.clearRect(0, 0, S.doc.w, S.doc.h);
    if (S.onion && !S.playing) {
      if (S.frame > 0) composite(ctx, S.frame - 1, { alpha: 0.28 });
      if (S.frame < S.doc.frames.length - 1) composite(ctx, S.frame + 1, { alpha: 0.12 });
    }
    composite(ctx, S.frame, { liveStroke: true });
    drawOverlay();
  };
  const drawOverlay = (preview) => {
    if (!S.overlay) return;
    const ctx = S.overlay.getContext('2d');
    ctx.clearRect(0, 0, S.doc.w, S.doc.h);
    if (S.grid) {
      ctx.save();
      ctx.strokeStyle = 'rgba(100,116,139,0.35)';
      ctx.lineWidth = 1;
      ctx.beginPath();
      for (let x = S.gridSize; x < S.doc.w; x += S.gridSize) { ctx.moveTo(x + 0.5, 0); ctx.lineTo(x + 0.5, S.doc.h); }
      for (let y = S.gridSize; y < S.doc.h; y += S.gridSize) { ctx.moveTo(0, y + 0.5); ctx.lineTo(S.doc.w, y + 0.5); }
      ctx.stroke();
      ctx.restore();
    }
    if (preview) preview(ctx);
  };
  const changed = (light) => {
    render();
    if (!light) { scheduleDraft(); emit(); }
  };
  const state = () => ({
    w: S.doc.w, h: S.doc.h, fps: S.doc.fps, loop: S.doc.loop, frame: S.frame, layer: S.layer, frames: S.doc.frames.length,
    layers: S.doc.layers.map((l) => ({ name: l.name, visible: l.visible, opacity: l.opacity, blend: l.blend })),
    tool: S.tool, color: S.color, size: S.size, opacity: S.opacity, fill: S.fill, tolerance: S.tolerance, text: S.text, font: S.font,
    sides: S.sides, star: S.star, zoom: S.zoom, grid: S.grid, snap: S.snap, onion: S.onion, playing: S.playing,
    undo: S.undo.length, redo: S.redo.length, recent: S.recent.slice(), draft: S.draftState,
  });
  const emit = () => { if (S.notify) try { S.notify(state()); } catch (e) { /* channel closed */ } };

  // ---------- drafts ----------
  const scheduleDraft = () => {
    if (!S.draftKey) return;
    clearTimeout(S.draftTimer);
    S.draftTimer = setTimeout(() => {
      try { localStorage.setItem(S.draftKey, toProject()); S.draftState = 'saved'; }
      catch (e) { S.draftState = 'too_large'; }
      emit();
    }, 2500);
  };

  // ---------- geometry ----------
  const pos = (e) => {
    const r = S.overlay.getBoundingClientRect();
    let x = ((e.clientX - r.left) * S.doc.w) / r.width, y = ((e.clientY - r.top) * S.doc.h) / r.height;
    if (S.snap && S.grid && ['line', 'rect', 'ellipse', 'polygon', 'text'].includes(S.tool)) {
      x = Math.round(x / S.gridSize) * S.gridSize; y = Math.round(y / S.gridSize) * S.gridSize;
    }
    return { x, y };
  };
  const shape = (ctx, a, b) => {
    ctx.save();
    ctx.strokeStyle = S.color; ctx.fillStyle = S.color; ctx.lineWidth = S.size; ctx.lineCap = 'round'; ctx.lineJoin = 'round';
    ctx.beginPath();
    if (S.tool === 'line') { ctx.moveTo(a.x, a.y); ctx.lineTo(b.x, b.y); ctx.stroke(); ctx.restore(); return; }
    if (S.tool === 'rect') ctx.rect(Math.min(a.x, b.x), Math.min(a.y, b.y), Math.abs(b.x - a.x), Math.abs(b.y - a.y));
    if (S.tool === 'ellipse') ctx.ellipse((a.x + b.x) / 2, (a.y + b.y) / 2, Math.abs(b.x - a.x) / 2 || 0.5, Math.abs(b.y - a.y) / 2 || 0.5, 0, 0, Math.PI * 2);
    if (S.tool === 'polygon') {
      const r = Math.hypot(b.x - a.x, b.y - a.y), n = Math.max(3, Math.min(12, S.sides)), pts = S.star ? n * 2 : n;
      const start = Math.atan2(b.y - a.y, b.x - a.x);
      for (let i = 0; i < pts; i++) {
        const ang = start + (i * Math.PI * 2) / pts, rr = S.star && i % 2 ? r * 0.45 : r;
        ctx.lineTo(a.x + Math.cos(ang) * rr, a.y + Math.sin(ang) * rr);
      }
      ctx.closePath();
    }
    if (S.fill) ctx.fill(); else ctx.stroke();
    ctx.restore();
  };
  const segment = (ctx, a, b) => {
    if (S.tool === 'pencil') {
      // Hard pixels (pixel art): square dabs along the line.
      const n = Math.max(1, Math.ceil(Math.hypot(b.x - a.x, b.y - a.y)));
      const s = Math.max(1, Math.round(S.size / 4));
      for (let i = 0; i <= n; i++) {
        const x = Math.floor(a.x + ((b.x - a.x) * i) / n - s / 2), y = Math.floor(a.y + ((b.y - a.y) * i) / n - s / 2);
        ctx.fillRect(x, y, s, s);
      }
      return;
    }
    ctx.beginPath(); ctx.moveTo(a.x, a.y); ctx.lineTo(b.x, b.y); ctx.stroke();
  };
  const hexToRgba = (hex) => [parseInt(hex.slice(1, 3), 16), parseInt(hex.slice(3, 5), 16), parseInt(hex.slice(5, 7), 16), Math.round(255 * S.opacity)];
  const flood = (c, x, y) => {
    const ctx = c.getContext('2d'), W = c.width, H = c.height;
    x = Math.floor(x); y = Math.floor(y);
    if (x < 0 || y < 0 || x >= W || y >= H) return;
    const img = ctx.getImageData(0, 0, W, H), d = img.data, i0 = (y * W + x) * 4;
    const t = [d[i0], d[i0 + 1], d[i0 + 2], d[i0 + 3]], fill = hexToRgba(S.color), tol = S.tolerance;
    if (t.every((v, k) => v === fill[k])) return;
    const same = (i) => Math.abs(d[i] - t[0]) <= tol && Math.abs(d[i + 1] - t[1]) <= tol && Math.abs(d[i + 2] - t[2]) <= tol && Math.abs(d[i + 3] - t[3]) <= tol;
    const seen = new Uint8Array(W * H), stack = [x, y];
    while (stack.length) {
      const py = stack.pop(), px = stack.pop();
      let lx = px;
      while (lx >= 0 && !seen[py * W + lx] && same((py * W + lx) * 4)) lx--;
      lx++;
      let up = false, down = false;
      for (let cx = lx; cx < W && !seen[py * W + cx] && same((py * W + cx) * 4); cx++) {
        const i = (py * W + cx) * 4;
        seen[py * W + cx] = 1; d[i] = fill[0]; d[i + 1] = fill[1]; d[i + 2] = fill[2]; d[i + 3] = fill[3];
        if (py > 0) { const ok = !seen[(py - 1) * W + cx] && same(((py - 1) * W + cx) * 4); if (ok && !up) { stack.push(cx, py - 1); up = true; } else if (!ok) up = false; }
        if (py < H - 1) { const ok = !seen[(py + 1) * W + cx] && same(((py + 1) * W + cx) * 4); if (ok && !down) { stack.push(cx, py + 1); down = true; } else if (!ok) down = false; }
      }
    }
    ctx.putImageData(img, 0, 0);
    c.__used = true;
  };
  const rememberColor = () => {
    S.recent = [S.color].concat(S.recent.filter((c) => c !== S.color)).slice(0, 10);
  };

  // ---------- pointer ----------
  const down = (e) => {
    if (!S.doc || S.playing) return;
    e.preventDefault();
    S.overlay.setPointerCapture(e.pointerId);
    const p = pos(e), L = S.doc.layers[S.layer];
    if (S.tool === 'picker') {
      const ctx = frameCanvas(S.frame).getContext('2d');
      const px = ctx.getImageData(Math.floor(p.x), Math.floor(p.y), 1, 1).data;
      if (px[3] > 0) { S.color = '#' + [px[0], px[1], px[2]].map((v) => v.toString(16).padStart(2, '0')).join('').toUpperCase(); rememberColor(); emit(); }
      return;
    }
    if (!L.visible) { L.visible = true; }
    const target = cell(S.frame, S.layer);
    if (S.tool === 'fill') { snapCell(S.frame, S.layer); flood(target, p.x, p.y); rememberColor(); changed(); return; }
    if (S.tool === 'text') {
      snapCell(S.frame, S.layer);
      const ctx = target.getContext('2d');
      ctx.save(); ctx.globalAlpha = S.opacity; ctx.fillStyle = S.color; ctx.textBaseline = 'top';
      ctx.font = `600 ${S.font}px system-ui, -apple-system, "Segoe UI", sans-serif`;
      String(S.text || '').split('\n').forEach((line, i) => ctx.fillText(line, p.x, p.y + i * S.font * 1.2));
      ctx.restore(); target.__used = true; rememberColor(); changed(); return;
    }
    if (S.tool === 'move') { snapCell(S.frame, S.layer); S.drag = { kind: 'move', start: p, src: clone(target) }; return; }
    if (['line', 'rect', 'ellipse', 'polygon'].includes(S.tool)) { S.drag = { kind: 'shape', start: p }; return; }
    // brush / pencil / eraser: draw into a stroke buffer, commit with opacity on release.
    snapCell(S.frame, S.layer);
    const stroke = mk(S.doc.w, S.doc.h), sctx = stroke.getContext('2d');
    sctx.lineCap = 'round'; sctx.lineJoin = 'round'; sctx.lineWidth = S.size;
    sctx.strokeStyle = S.color; sctx.fillStyle = S.color;
    const eraser = S.tool === 'eraser';
    S.drag = { kind: 'stroke', last: p, stroke, eraser };
    if (S.tool === 'pencil') segment(sctx, p, p);
    else { sctx.beginPath(); sctx.arc(p.x, p.y, S.size / 2, 0, Math.PI * 2); sctx.fill(); }
    if (eraser) eraseWith(stroke);
    changed(true);
  };
  const eraseWith = (stroke) => {
    // The eraser works directly on the layer so the preview is exact.
    const c = cell(S.frame, S.layer), ctx = c.getContext('2d');
    ctx.save(); ctx.globalCompositeOperation = 'destination-out'; ctx.globalAlpha = S.opacity; ctx.drawImage(stroke, 0, 0); ctx.restore();
    stroke.getContext('2d').clearRect(0, 0, stroke.width, stroke.height);
  };
  const move = (e) => {
    if (!S.drag) return;
    const p = pos(e);
    if (S.drag.kind === 'stroke') {
      const sctx = S.drag.stroke.getContext('2d');
      segment(sctx, S.drag.last, p);
      S.drag.last = p;
      if (S.drag.eraser) eraseWith(S.drag.stroke);
      changed(true);
    } else if (S.drag.kind === 'shape') {
      const a = S.drag.start;
      drawOverlay((ctx) => shape(ctx, a, p));
    } else if (S.drag.kind === 'move') {
      const c = cell(S.frame, S.layer), ctx = c.getContext('2d');
      ctx.clearRect(0, 0, c.width, c.height);
      ctx.drawImage(S.drag.src, Math.round(p.x - S.drag.start.x), Math.round(p.y - S.drag.start.y));
      changed(true);
    }
  };
  const up = (e) => {
    if (!S.drag) return;
    const d = S.drag, p = pos(e);
    S.drag = null;
    const c = cell(S.frame, S.layer), ctx = c.getContext('2d');
    if (d.kind === 'stroke' && !d.eraser) {
      ctx.save(); ctx.globalAlpha = S.opacity; ctx.drawImage(d.stroke, 0, 0); ctx.restore();
      c.__used = true; rememberColor();
    } else if (d.kind === 'shape') {
      snapCell(S.frame, S.layer);
      ctx.save(); ctx.globalAlpha = S.opacity; shape(ctx, d.start, p); ctx.restore();
      c.__used = true; rememberColor();
    } else if (d.kind === 'move') {
      c.__used = true;
    }
    changed();
  };

  // ---------- documents ----------
  const blankDoc = (w, h) => ({ w, h, fps: 8, loop: true, layers: [{ name: 'Background', visible: true, opacity: 1, blend: 'normal' }], frames: [{ cells: [null] }] });
  const usedCell = (c) => {
    if (!c || !c.__used) return null;
    return c.toDataURL('image/png');
  };
  const toProject = () => JSON.stringify({
    v: 1, w: S.doc.w, h: S.doc.h, fps: S.doc.fps, loop: S.doc.loop,
    layers: S.doc.layers.map((l) => ({ name: l.name, visible: l.visible, opacity: l.opacity, blend: l.blend })),
    frames: S.doc.frames.map((f) => ({ cells: S.doc.layers.map((_, i) => usedCell(f.cells[i])) })),
  });
  const loadImage = (src) => new Promise((ok, bad) => { const im = new Image(); im.onload = () => ok(im); im.onerror = bad; im.src = src; });
  const fromProject = async (json) => {
    const p = typeof json === 'string' ? JSON.parse(json) : json;
    if (!p || p.v !== 1 || !(p.w >= 16 && p.w <= 1024 && p.h >= 16 && p.h <= 1024)) throw new Error('invalid_project');
    if (!Array.isArray(p.layers) || !Array.isArray(p.frames) || !p.layers.length || p.layers.length > MAX_LAYERS || !p.frames.length || p.frames.length > MAX_FRAMES) throw new Error('invalid_project');
    const doc = {
      w: p.w, h: p.h, fps: Math.min(30, Math.max(1, p.fps || 8)), loop: p.loop !== false,
      layers: p.layers.map((l, i) => ({ name: String(l.name || `Layer ${i + 1}`).slice(0, 40), visible: l.visible !== false, opacity: Math.min(1, Math.max(0, Number(l.opacity ?? 1))), blend: BLENDS[l.blend] ? l.blend : 'normal' })),
      frames: [],
    };
    for (const f of p.frames) {
      const cells = [];
      for (let i = 0; i < doc.layers.length; i++) {
        const src = f.cells && f.cells[i];
        if (typeof src === 'string' && src.startsWith('data:image/png;base64,')) {
          const im = await loadImage(src), c = mk(doc.w, doc.h);
          c.getContext('2d').drawImage(im, 0, 0); c.__used = true; cells.push(c);
        } else cells.push(null);
      }
      doc.frames.push({ cells });
    }
    return doc;
  };

  // ---------- animation ----------
  const tick = () => {
    S.frame += 1;
    if (S.frame >= S.doc.frames.length) {
      if (S.doc.loop) S.frame = 0; else { S.frame = S.doc.frames.length - 1; pause(); return; }
    }
    render(); emit();
  };
  const play = () => { if (S.playing || S.doc.frames.length < 2) return; S.playing = true; S.timer = setInterval(tick, 1000 / S.doc.fps); changed(); };
  const pause = () => { S.playing = false; clearInterval(S.timer); S.timer = null; changed(); };
  const PRESETS = {
    bounce: (t, w, h) => ({ y: -Math.abs(Math.sin(Math.PI * 2 * t)) * h * 0.18 }),
    spin: (t) => ({ rot: 360 * t }),
    fadein: (t) => ({ alpha: t }),
    fadeout: (t) => ({ alpha: 1 - t }),
    slidein: (t, w) => ({ x: -w * (1 - EASE['ease-out'](t)) }),
    pulse: (t) => ({ scale: 1 + 0.15 * Math.sin(Math.PI * 2 * t) }),
    shake: (t, w) => ({ x: Math.sin(Math.PI * 6 * t) * w * 0.05 }),
    zoomin: (t) => ({ scale: Math.max(0.05, EASE['ease-out'](t)) }),
    float: (t, w, h) => ({ y: Math.sin(Math.PI * 2 * t) * h * 0.06 }),
    wobble: (t) => ({ rot: Math.sin(Math.PI * 4 * t) * 12 }),
  };
  const tween = (opt) => {
    const n = Math.max(2, Math.min(MAX_FRAMES, opt.frames | 0));
    if (S.frame + n > MAX_FRAMES) throw new Error('too_many_frames');
    snapDoc();
    const src = clone(cell(S.frame, S.layer)), w = S.doc.w, h = S.doc.h, base = S.frame;
    const ease = EASE[opt.easing] || EASE.linear;
    const lerp = (a, b, t) => a + (b - a) * t;
    for (let k = 0; k < n; k++) {
      const f = base + k;
      if (!S.doc.frames[f]) {
        // New frames keep the other layers of the starting frame (backgrounds stay put).
        S.doc.frames.splice(f, 0, { cells: S.doc.layers.map((_, i) => (i === S.layer ? null : (S.doc.frames[base].cells[i] ? clone(S.doc.frames[base].cells[i]) : null))) });
      }
      const t = n === 1 ? 1 : k / (n - 1), e = ease(t);
      let tr;
      if (opt.preset && PRESETS[opt.preset]) tr = Object.assign({ x: 0, y: 0, scale: 1, rot: 0, alpha: 1 }, PRESETS[opt.preset](t, w, h));
      else {
        const a = opt.from || {}, b = opt.to || {};
        tr = { x: lerp(a.x || 0, b.x || 0, e), y: lerp(a.y || 0, b.y || 0, e), scale: lerp(a.scale ?? 1, b.scale ?? 1, e), rot: lerp(a.rot || 0, b.rot || 0, e), alpha: lerp(a.alpha ?? 1, b.alpha ?? 1, e) };
      }
      const c = cell(f, S.layer), ctx = c.getContext('2d');
      ctx.clearRect(0, 0, w, h);
      ctx.save();
      ctx.globalAlpha = Math.max(0, Math.min(1, tr.alpha));
      ctx.translate(w / 2 + tr.x, h / 2 + tr.y);
      ctx.rotate((tr.rot * Math.PI) / 180);
      ctx.scale(tr.scale, tr.scale);
      ctx.drawImage(src, -w / 2, -h / 2);
      ctx.restore();
      c.__used = true;
    }
    changed();
  };

  // ---------- public API ----------
  const API = {
    init(viewId, overlayId, w, h, draftKey) {
      S.view = document.getElementById(viewId);
      S.overlay = document.getElementById(overlayId);
      // w = h = 0 only re-binds the canvases (after a resize or a project load).
      if (draftKey) S.draftKey = draftKey;
      if (w > 0 && h > 0 && (!S.doc || S.doc.w !== w || S.doc.h !== h)) { S.doc = blankDoc(w, h); S.frame = 0; S.layer = 0; S.undo = []; S.redo = []; }
      if (!S.doc) S.doc = blankDoc(320, 320);
      if (S.overlay && !S.overlay.__cwBound) {
        S.overlay.__cwBound = true;
        S.overlay.addEventListener('pointerdown', down);
        S.overlay.addEventListener('pointermove', move);
        S.overlay.addEventListener('pointerup', up);
        S.overlay.addEventListener('pointercancel', up);
      }
      render(); emit();
      return state();
    },
    listen(fn) { S.notify = fn; emit(); },
    state,
    newDoc(w, h) { pause(); S.doc = blankDoc(w, h); S.frame = 0; S.layer = 0; S.undo = []; S.redo = []; changed(); },
    set(key, value) {
      const allowed = ['tool', 'color', 'size', 'opacity', 'fill', 'tolerance', 'text', 'font', 'sides', 'star', 'zoom', 'grid', 'snap', 'onion', 'gridSize'];
      if (!allowed.includes(key)) return;
      S[key] = value;
      if (key === 'color') rememberColor();
      changed(key !== 'grid' && key !== 'onion');
      emit();
    },
    setDoc(key, value) {
      if (key === 'fps') { S.doc.fps = Math.max(1, Math.min(30, value | 0)); if (S.playing) { pause(); play(); } }
      if (key === 'loop') S.doc.loop = !!value;
      changed();
    },
    undo() { swap(S.undo, S.redo); },
    redo() { swap(S.redo, S.undo); },
    clearLayer() { snapCell(S.frame, S.layer); const c = cell(S.frame, S.layer); c.getContext('2d').clearRect(0, 0, c.width, c.height); c.__used = false; changed(); },
    // layers
    addLayer() {
      if (S.doc.layers.length >= MAX_LAYERS) return 'too_many_layers';
      snapDoc();
      const at = S.layer + 1;
      S.doc.layers.splice(at, 0, { name: `Layer ${S.doc.layers.length + 1}`, visible: true, opacity: 1, blend: 'normal' });
      S.doc.frames.forEach((f) => f.cells.splice(at, 0, null));
      S.layer = at; changed(); return null;
    },
    duplicateLayer() {
      if (S.doc.layers.length >= MAX_LAYERS) return 'too_many_layers';
      snapDoc();
      const at = S.layer + 1, src = S.doc.layers[S.layer];
      S.doc.layers.splice(at, 0, { ...src, name: (src.name + ' copy').slice(0, 40) });
      S.doc.frames.forEach((f) => f.cells.splice(at, 0, f.cells[S.layer] ? clone(f.cells[S.layer]) : null));
      S.layer = at; changed(); return null;
    },
    deleteLayer() {
      if (S.doc.layers.length <= 1) return 'last_layer';
      snapDoc();
      S.doc.layers.splice(S.layer, 1);
      S.doc.frames.forEach((f) => f.cells.splice(S.layer, 1));
      S.layer = Math.max(0, S.layer - 1); changed(); return null;
    },
    moveLayer(dir) {
      const to = S.layer + dir;
      if (to < 0 || to >= S.doc.layers.length) return;
      snapDoc();
      const sw = (a) => { const t = a[S.layer]; a[S.layer] = a[to]; a[to] = t; };
      sw(S.doc.layers); S.doc.frames.forEach((f) => sw(f.cells));
      S.layer = to; changed();
    },
    selectLayer(i) { if (i >= 0 && i < S.doc.layers.length) { S.layer = i; changed(); } },
    layerProp(i, key, value) {
      const L = S.doc.layers[i]; if (!L) return;
      if (key === 'visible') L.visible = !!value;
      if (key === 'opacity') L.opacity = Math.max(0, Math.min(1, Number(value)));
      if (key === 'blend' && BLENDS[value]) L.blend = value;
      if (key === 'name') L.name = String(value).slice(0, 40) || L.name;
      changed();
    },
    mergeDown() {
      if (S.layer === 0) return;
      snapDoc();
      const top = S.doc.layers[S.layer];
      S.doc.frames.forEach((f, fi) => {
        const a = cell(fi, S.layer - 1), b = f.cells[S.layer];
        if (b) { const ctx = a.getContext('2d'); ctx.save(); ctx.globalAlpha = top.opacity; ctx.globalCompositeOperation = BLENDS[top.blend]; ctx.drawImage(b, 0, 0); ctx.restore(); a.__used = a.__used || b.__used; }
        f.cells.splice(S.layer, 1);
      });
      S.doc.layers.splice(S.layer, 1); S.layer -= 1; changed();
    },
    // frames
    addFrame() {
      if (S.doc.frames.length >= MAX_FRAMES) return 'too_many_frames';
      snapDoc(); S.doc.frames.splice(S.frame + 1, 0, { cells: S.doc.layers.map(() => null) }); S.frame += 1; changed(); return null;
    },
    duplicateFrame() {
      if (S.doc.frames.length >= MAX_FRAMES) return 'too_many_frames';
      snapDoc(); S.doc.frames.splice(S.frame + 1, 0, { cells: S.doc.frames[S.frame].cells.map((c) => (c ? clone(c) : null)) }); S.frame += 1; changed(); return null;
    },
    deleteFrame() {
      if (S.doc.frames.length <= 1) return 'last_frame';
      snapDoc(); S.doc.frames.splice(S.frame, 1); S.frame = Math.min(S.frame, S.doc.frames.length - 1); changed(); return null;
    },
    moveFrame(dir) {
      const to = S.frame + dir;
      if (to < 0 || to >= S.doc.frames.length) return;
      snapDoc(); const fr = S.doc.frames; const t = fr[S.frame]; fr[S.frame] = fr[to]; fr[to] = t; S.frame = to; changed();
    },
    selectFrame(i) { if (i >= 0 && i < S.doc.frames.length) { if (S.playing) pause(); S.frame = i; changed(); } },
    play, pause,
    tween(opt) { try { tween(opt); return null; } catch (e) { return e.message; } },
    // thumbnails for the timeline / layer list
    thumbs() {
      document.querySelectorAll('canvas[data-studio-thumb]').forEach((el) => {
        const i = Number(el.dataset.studioThumb); if (!S.doc.frames[i]) return;
        const ctx = el.getContext('2d'); ctx.clearRect(0, 0, el.width, el.height);
        ctx.drawImage(frameCanvas(i), 0, 0, el.width, el.height);
      });
      document.querySelectorAll('canvas[data-studio-layer]').forEach((el) => {
        const i = Number(el.dataset.studioLayer), c = S.doc.frames[S.frame].cells[i];
        const ctx = el.getContext('2d'); ctx.clearRect(0, 0, el.width, el.height);
        if (c) ctx.drawImage(c, 0, 0, el.width, el.height);
      });
    },
    // import / export
    async importImage(dataUrl) {
      if (S.doc.layers.length >= MAX_LAYERS) return 'too_many_layers';
      const im = await loadImage(dataUrl);
      API.addLayer();
      const c = cell(S.frame, S.layer), ctx = c.getContext('2d');
      const k = Math.min(S.doc.w / im.width, S.doc.h / im.height, 1);
      const w = im.width * k, h = im.height * k;
      ctx.drawImage(im, (S.doc.w - w) / 2, (S.doc.h - h) / 2, w, h);
      c.__used = true; S.doc.layers[S.layer].name = 'Image'; changed(); return null;
    },
    async readFile(inputId) {
      const input = document.getElementById(inputId);
      if (!input || !input.files || !input.files[0]) return 'no_file';
      const f = input.files[0]; input.value = '';
      if (f.size > 8 * 1024 * 1024) return 'too_large';
      if (f.name.endsWith('.json') || f.type === 'application/json') {
        try { const doc = await fromProject(await f.text()); pause(); S.doc = doc; S.frame = 0; S.layer = 0; S.undo = []; S.redo = []; changed(); return 'project'; }
        catch (e) { return 'invalid_project'; }
      }
      if (!/^image\/(png|jpeg|webp|gif|avif)$/.test(f.type)) return 'unsupported';
      const url = await new Promise((ok) => { const r = new FileReader(); r.onload = () => ok(r.result); r.readAsDataURL(f); });
      return (await API.importImage(url)) || 'image';
    },
    async load(json) { const doc = await fromProject(json); pause(); S.doc = doc; S.frame = 0; S.layer = 0; S.undo = []; S.redo = []; changed(); return state(); },
    draft() { try { return S.draftKey ? localStorage.getItem(S.draftKey) : null; } catch (e) { return null; } },
    dropDraft() { try { if (S.draftKey) localStorage.removeItem(S.draftKey); } catch (e) { /* ignore */ } S.draftState = ''; emit(); },
    project: toProject,
    png(frame) { return frameCanvas(frame == null ? S.frame : frame).toDataURL('image/png'); },
    thumbnail() {
      const k = Math.min(1, 160 / Math.max(S.doc.w, S.doc.h));
      const c = mk(Math.max(1, Math.round(S.doc.w * k)), Math.max(1, Math.round(S.doc.h * k)));
      c.getContext('2d').drawImage(frameCanvas(0), 0, 0, c.width, c.height);
      return c.toDataURL('image/png');
    },
    // Raw RGBA of every frame, one base64 string per frame (for the GIF encoder).
    rgba(i) {
      const d = frameCanvas(i).getContext('2d').getImageData(0, 0, S.doc.w, S.doc.h).data;
      let bin = '';
      for (let k = 0; k < d.length; k += 0x8000) bin += String.fromCharCode.apply(null, d.subarray(k, k + 0x8000));
      return btoa(bin);
    },
    download(name, b64, mime) {
      const bin = atob(b64), u8 = new Uint8Array(bin.length);
      for (let i = 0; i < bin.length; i++) u8[i] = bin.charCodeAt(i);
      const url = URL.createObjectURL(new Blob([u8], { type: mime }));
      const a = document.createElement('a'); a.href = url; a.download = name; document.body.appendChild(a); a.click(); a.remove();
      setTimeout(() => URL.revokeObjectURL(url), 4000);
    },
    key(e) {
      // Keyboard shortcuts while the studio has focus (not while typing in a field).
      const tag = (e.target && e.target.tagName) || '';
      if (['INPUT', 'TEXTAREA', 'SELECT'].includes(tag)) return false;
      const k = e.key.toLowerCase(), mod = e.ctrlKey || e.metaKey;
      if (mod && k === 'z' && !e.shiftKey) { API.undo(); return true; }
      if ((mod && k === 'y') || (mod && k === 'z' && e.shiftKey)) { API.redo(); return true; }
      if (mod) return false;
      const tools = { b: 'brush', p: 'pencil', e: 'eraser', g: 'fill', l: 'line', r: 'rect', o: 'ellipse', s: 'polygon', t: 'text', i: 'picker', v: 'move' };
      if (tools[k]) { API.set('tool', tools[k]); return true; }
      if (k === '[') { API.set('size', Math.max(1, S.size - 1)); return true; }
      if (k === ']') { API.set('size', Math.min(128, S.size + 1)); return true; }
      if (k === ',') { API.selectFrame(Math.max(0, S.frame - 1)); return true; }
      if (k === '.') { API.selectFrame(Math.min(S.doc.frames.length - 1, S.frame + 1)); return true; }
      if (k === ' ') { S.playing ? pause() : play(); return true; }
      return false;
    },
  };
  window.ChatwStudio = API;
})();
