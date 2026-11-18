// ChatW admin web: talks only to /api/admin/* (server enforces admin rights).
(function () {
  'use strict';
  const $ = (id) => document.getElementById(id);
  const api = async (path, opts = {}) => {
    const res = await fetch('/api/admin' + path, { credentials: 'same-origin', ...opts });
    let body = null;
    try { body = await res.json(); } catch (_) {}
    if (!res.ok) throw Object.assign(new Error((body && (body.error || body.code)) || res.status), { status: res.status });
    return body;
  };
  const el = (tag, props = {}, ...children) => {
    const n = document.createElement(tag);
    Object.entries(props).forEach(([k, v]) => { if (k === 'class') n.className = v; else if (k.startsWith('on')) n.addEventListener(k.slice(2), v); else n.setAttribute(k, v); });
    children.flat().forEach((c) => n.append(c instanceof Node ? c : document.createTextNode(c == null ? '' : String(c))));
    return n;
  };
  const toast = (msg) => { const t = $('toast'); t.textContent = msg; t.hidden = false; setTimeout(() => { t.hidden = true; }, 2400); };
  const fmt = (iso) => (iso ? new Date(iso).toLocaleString('vi-VN') : '');

  const STAT_LABELS = { users: 'Tài khoản', suspended: 'Bị khóa', active_sessions: 'Phiên hoạt động', dm_messages: 'Tin nhắn riêng (bản mã)', group_messages: 'Tin nhắn nhóm', groups: 'Nhóm', servers: 'Cộng đồng', bots: 'Bot', businesses: 'Trang Kinh doanh', pending_verifications: 'Chờ xác minh' };

  async function loadStats() {
    const s = await api('/stats');
    const box = $('stats'); box.replaceChildren();
    Object.entries(STAT_LABELS).forEach(([k, label]) => box.append(el('div', { class: 'stat' }, el('b', {}, s[k] ?? 0), el('span', {}, label))));
    const b = $('pending-badge'); b.hidden = !s.pending_verifications; b.textContent = s.pending_verifications;
  }

  async function loadVerify() {
    const items = await api('/verifications');
    const box = $('verify-list'); box.replaceChildren();
    if (!items.length) { box.append(el('div', { class: 'empty' }, 'Không có yêu cầu nào.')); return; }
    items.forEach((it) => {
      const decide = async (d) => {
        try { await api(`/verifications/${it.kind}/${it.id}/${d}`, { method: 'POST' }); toast(d === 'verify' ? 'Đã xác minh' : 'Đã từ chối'); loadVerify(); loadStats(); }
        catch (e) { toast('Lỗi: ' + e.message); }
      };
      box.append(el('div', { class: 'row' },
        el('span', { class: 'kind' }, it.kind),
        el('div', { class: 'main' }, el('div', { class: 'title' }, it.name), el('div', { class: 'sub' }, `Chủ sở hữu: ${it.owner || '?'} · ${fmt(it.created_at)}`), it.description ? el('div', { class: 'sub' }, it.description) : ''),
        el('button', { class: 'act', onclick: () => decide('verify') }, 'Xác minh'),
        el('button', { class: 'act secondary', onclick: () => decide('reject') }, 'Từ chối')));
    });
  }

  let userTimer = null;
  async function loadUsers() {
    const q = encodeURIComponent($('user-q').value.trim());
    const users = await api('/users?q=' + q);
    const box = $('user-list'); box.replaceChildren();
    if (!users.length) { box.append(el('div', { class: 'empty' }, 'Không có tài khoản phù hợp.')); return; }
    users.forEach((u) => {
      const act = async (a) => { try { await api(`/users/${u.id}/${a}`, { method: 'POST' }); toast(a === 'suspend' ? 'Đã khóa tài khoản' : 'Đã mở khóa'); loadUsers(); } catch (e) { toast('Lỗi: ' + e.message); } };
      box.append(el('div', { class: 'row' },
        el('div', { class: 'main' }, el('div', { class: 'title' }, (u.username || '(chưa đặt tên)') + (u.is_admin ? ' · admin' : '')), el('div', { class: 'sub' }, `Tạo ${fmt(u.created_at)} · ${u.sessions} phiên` + (u.suspended_at ? ` · bị khóa ${fmt(u.suspended_at)}` : ''))),
        u.is_admin ? '' : (u.suspended_at ? el('button', { class: 'act secondary', onclick: () => act('unsuspend') }, 'Mở khóa') : el('button', { class: 'act danger', onclick: () => act('suspend') }, 'Khóa tài khoản'))));
    });
  }

  async function loadAudit() {
    const rows = await api('/audit');
    const box = $('audit-list'); box.replaceChildren();
    rows.forEach((r) => box.append(el('div', { class: 'row' }, el('div', { class: 'main' }, el('div', { class: 'title' }, r.action), el('div', { class: 'sub' }, `${r.username || 'hệ thống'} · ${r.target_type || ''} ${r.target_id || ''} · ${fmt(r.created_at)}`)))));
  }

  const loaders = { overview: loadStats, verify: loadVerify, users: loadUsers, audit: loadAudit };
  function show(tab) {
    document.querySelectorAll('.tabs button').forEach((b) => b.classList.toggle('active', b.dataset.tab === tab));
    document.querySelectorAll('.tab-panel').forEach((p) => { p.hidden = p.id !== tab; });
    loaders[tab]().catch((e) => toast('Lỗi tải dữ liệu: ' + e.message));
  }

  async function boot() {
    try {
      const me = await api('/me');
      if (!me.is_admin) throw Object.assign(new Error('admin_only'), { status: 403 });
      $('who').textContent = 'Đăng nhập: ' + (me.username || me.id);
      document.querySelectorAll('.tabs button').forEach((b) => b.addEventListener('click', () => show(b.dataset.tab)));
      $('user-q').addEventListener('input', () => { clearTimeout(userTimer); userTimer = setTimeout(loadUsers, 250); });
      show('overview');
    } catch (e) {
      document.querySelectorAll('.tab-panel').forEach((p) => { p.hidden = true; });
      $('tabs').hidden = true;
      $('gate').hidden = false;
      if (e.status === 403) $('gate-text').textContent = 'Tài khoản hiện tại không có quyền quản trị. Quản trị viên được cấp qua biến môi trường ADMIN_USERNAMES.';
    }
  }
  boot();
})();
