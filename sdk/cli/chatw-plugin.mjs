#!/usr/bin/env node
/**
 * ChatW Plugin Developer CLI
 * Commands:
 *   init <dir> <publisher.slug>   Scaffold a new JS/TS or Rust-WASM plugin project
 *   validate <plugin-dir>         Validate manifest.json, permissions, entry files & WASM header
 *   hash <plugin-dir>             Compute deterministic SHA-256 package hash (matches ChatW backend)
 *   pack <plugin-dir> [out.json]  Bundle manifest + bundle_code + ui_html + wasm_base64 + sha256 into publish JSON
 */
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

const WASM_MAGIC = Buffer.from([0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);
const PLUGIN_KEY_RE = /^[a-z0-9][a-z0-9_-]{1,31}\.[a-z0-9][a-z0-9_-]{1,47}$/;
const SEMVER_RE = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;

function canonicalizeManifest(m) {
  const rl = m.resource_limits || {};
  return {
    plugin_key: String(m.plugin_key || ''),
    name: String(m.name || ''),
    version: String(m.version || ''),
    publisher: String(m.publisher || ''),
    entry_point: String(m.entry_point || 'index.js'),
    runtime: String(m.runtime || 'js'),
    sdk_version: String(m.sdk_version || '1.0.0'),
    platforms: Array.isArray(m.platforms) ? m.platforms : [],
    permissions: Array.isArray(m.permissions) ? m.permissions : [],
    dependencies: Array.isArray(m.dependencies) ? m.dependencies : [],
    ui_contributions: m.ui_contributions === undefined ? null : m.ui_contributions,
    commands: Array.isArray(m.commands) ? m.commands : [],
    event_subscriptions: Array.isArray(m.event_subscriptions) ? m.event_subscriptions : [],
    settings_schema: Array.isArray(m.settings_schema) ? m.settings_schema : [],
    resource_limits: {
      max_memory_mb: Number(rl.max_memory_mb ?? 32),
      max_exec_ms: Number(rl.max_exec_ms ?? 3000),
      max_storage_kb: Number(rl.max_storage_kb ?? 512),
      max_requests_per_min: Number(rl.max_requests_per_min ?? 30),
    },
    allowed_domains: Array.isArray(m.allowed_domains) ? m.allowed_domains : [],
    allow_custom_logo: m.allow_custom_logo === undefined ? null : Boolean(m.allow_custom_logo),
  };
}

function readBundleFiles(dir) {
  const manifestPath = path.join(dir, 'manifest.json');
  if (!fs.existsSync(manifestPath)) throw new Error(`Missing manifest.json in ${dir}`);
  const rawMeta = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  const manifest = canonicalizeManifest(rawMeta);

  const readOpt = (rel) => {
    if (!rel) return '';
    if (rel.includes('..') || path.isAbsolute(rel)) throw new Error(`Invalid relative path: ${rel}`);
    const full = path.join(dir, rel);
    if (!fs.existsSync(full)) throw new Error(`Referenced file missing: ${rel}`);
    return fs.readFileSync(full, 'utf8');
  };

  const jsRel = rawMeta.entry_js || (fs.existsSync(path.join(dir, 'worker.js')) ? 'worker.js' : '');
  const uiRel = rawMeta.entry_ui || (fs.existsSync(path.join(dir, 'ui.html')) ? 'ui.html' : '');
  const wasmRel = rawMeta.entry_wasm || (fs.existsSync(path.join(dir, 'plugin.wasm')) ? 'plugin.wasm' : '');

  const bundle_code = readOpt(jsRel);
  const ui_html = readOpt(uiRel);
  let wasm_base64 = '';
  if (wasmRel) {
    if (wasmRel.includes('..') || path.isAbsolute(wasmRel)) throw new Error(`Invalid wasm path: ${wasmRel}`);
    const full = path.join(dir, wasmRel);
    if (!fs.existsSync(full)) throw new Error(`Referenced wasm missing: ${wasmRel}`);
    const buf = fs.readFileSync(full);
    if (buf.length < 8 || !buf.subarray(0, 8).equals(WASM_MAGIC)) {
      throw new Error(`Invalid WASM magic header in ${wasmRel}`);
    }
    wasm_base64 = buf.toString('base64');
  }

  return { rawMeta, manifest, bundle_code, ui_html, wasm_base64 };
}

function computePackageSha256(manifest, bundle_code, ui_html, wasm_base64) {
  const h = crypto.createHash('sha256');
  h.update(JSON.stringify(manifest));
  h.update('\n---BUNDLE---\n');
  h.update(bundle_code || '');
  h.update('\n---UI---\n');
  h.update(ui_html || '');
  h.update('\n---WASM---\n');
  h.update(wasm_base64 || '');
  return h.digest('hex');
}

function validateBundle(dir) {
  const data = readBundleFiles(dir);
  const { manifest, bundle_code, ui_html, wasm_base64 } = data;
  if (!PLUGIN_KEY_RE.test(manifest.plugin_key || '')) {
    throw new Error(`Invalid plugin_key "${manifest.plugin_key}" (must be publisher.slug)`);
  }
  if (!SEMVER_RE.test(manifest.version || '')) {
    throw new Error(`Invalid semver "${manifest.version}"`);
  }
  if (!['js', 'wasm', 'hybrid', 'declarative'].includes(manifest.runtime)) {
    throw new Error(`Invalid runtime "${manifest.runtime}"`);
  }
  if (manifest.runtime === 'wasm' && !wasm_base64) {
    throw new Error('runtime="wasm" requires a compiled .wasm entry');
  }
  if (!bundle_code && !ui_html && !wasm_base64) {
    throw new Error('Plugin bundle has no executable worker.js, ui.html, or plugin.wasm');
  }
  const sha256 = computePackageSha256(manifest, bundle_code, ui_html, wasm_base64);
  return { ...data, sha256 };
}

function cmdInit(dir, pluginKey) {
  if (!PLUGIN_KEY_RE.test(pluginKey || '')) {
    throw new Error('Usage: chatw-plugin init <dir> <publisher.slug>');
  }
  fs.mkdirSync(dir, { recursive: true });
  const [pub, slug] = pluginKey.split('.');
  const manifest = {
    plugin_key: pluginKey,
    name: slug,
    version: '1.0.0',
    publisher: pub,
    entry_point: 'worker.js',
    runtime: 'js',
    sdk_version: '1.0.0',
    platforms: ['web', 'desktop'],
    permissions: ['storage.local', 'commands.register', 'ui.page'],
    ui_contributions: { page: true, sidebar: true },
    commands: [{ trigger: `/${slug}`, description: 'Chạy lệnh plugin' }],
    event_subscriptions: [],
    settings_schema: [],
    summary: 'Plugin tùy chỉnh phát triển bằng ChatW SDK',
    description: 'Plugin độc lập chạy trong sandbox cách ly.',
    category: 'productivity',
    entry_js: 'worker.js',
    entry_ui: 'ui.html',
  };
  fs.writeFileSync(path.join(dir, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
  fs.writeFileSync(
    path.join(dir, 'worker.js'),
    `chatw.commands.register('/${slug}', 'Chạy lệnh mẫu', async (ctx) => {\n  return { text: 'Xin chào từ ${pluginKey}: ' + (ctx.args || '') };\n});\n`
  );
  fs.writeFileSync(
    path.join(dir, 'ui.html'),
    `<div style="padding:16px"><h3>${pluginKey}</h3><p>Giao diện tùy chỉnh chạy trong Sandbox.</p></div>\n`
  );
  console.log(`Initialized plugin at ${dir} (${pluginKey})`);
}

const [, , cmd, arg1, arg2] = process.argv;
try {
  if (cmd === 'init') {
    cmdInit(arg1, arg2);
  } else if (cmd === 'validate') {
    const res = validateBundle(arg1 || '.');
    console.log(JSON.stringify({ ok: true, plugin_key: res.manifest.plugin_key, version: res.manifest.version, runtime: res.manifest.runtime, sha256: res.sha256 }, null, 2));
  } else if (cmd === 'hash') {
    const res = validateBundle(arg1 || '.');
    console.log(res.sha256);
  } else if (cmd === 'pack') {
    const res = validateBundle(arg1 || '.');
    const payload = {
      manifest: res.manifest,
      summary: res.rawMeta.summary || res.manifest.name,
      description: res.rawMeta.description || res.manifest.name,
      category: res.rawMeta.category || 'general',
      is_public: res.rawMeta.is_public !== false,
      changelog: res.rawMeta.changelog || `Phát hành phiên bản ${res.manifest.version}`,
      bundle_code: res.bundle_code,
      ui_html: res.ui_html,
      wasm_base64: res.wasm_base64,
      sha256: res.sha256,
    };
    const outPath = arg2 || path.join(arg1 || '.', `${res.manifest.plugin_key}-${res.manifest.version}.chatw-plugin.json`);
    fs.writeFileSync(outPath, JSON.stringify(payload, null, 2) + '\n');
    console.log(`Packed ${res.manifest.plugin_key}@${res.manifest.version} -> ${outPath} (sha256=${res.sha256})`);
  } else {
    console.error('Usage: chatw-plugin <init|validate|hash|pack> ...');
    process.exit(1);
  }
} catch (err) {
  console.error('Error:', err.message);
  process.exit(1);
}
