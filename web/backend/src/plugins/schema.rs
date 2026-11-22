//! Official ChatW Plugin Permission Catalog, Manifest Schema & Discord Embed Validators.

use base64::{Engine, engine::general_purpose::{STANDARD as B64, URL_SAFE_NO_PAD}};
use p256::ecdsa::{
    Signature as EcdsaSignature, SigningKey as EcdsaSigningKey, VerifyingKey as EcdsaVerifyingKey,
    signature::{Signer, Verifier},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize)]
pub struct PermissionDef {
    pub id: &'static str,
    pub label: &'static str,
    pub category: &'static str,
    pub sensitive: bool,
    pub e2ee: bool,
    pub device_only_default: bool,
    pub description: &'static str,
}

/// Authoritative permission schema enforced by both Backend and Client Sandbox Host.
pub const PERMISSION_CATALOG: &[PermissionDef] = &[
    // UI & Commands (Standard)
    PermissionDef { id: "ui.page", label: "Create your own page", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Displays the Plugin's own interface in the main content area." },
    PermissionDef { id: "ui.sidebar", label: "Open in the left column", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Displays the Plugin's compact interface in parallel with the chat frame." },
    PermissionDef { id: "ui.dialog", label: "Open the dialog box", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Displays an interactive dialog window in the sandbox." },
    PermissionDef { id: "ui.message_action", label: "Add buttons to messages", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Add action buttons or context menu items on messages." },
    PermissionDef { id: "ui.composer", label: "Expand the message editor", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Add quick action buttons or expansion panels next to the input box." },
    PermissionDef { id: "ui.server_tab", label: "Server administration page", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Add a management tab or dashboard in the server." },
    PermissionDef { id: "ui.group_tab", label: "Group expansion page", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Added utility interface in chat group." },
    PermissionDef { id: "ui.profile_tab", label: "Profile extension card", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Show information tabs or cards in the user profile." },
    PermissionDef { id: "ui.call_panel", label: "In-call convenience", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Add a control button or backend interface during the call." },
    PermissionDef { id: "commands.register", label: "Register shortcut commands", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Register /command commands in the message editor." },
    PermissionDef { id: "theme.apply", label: "Customize ChatW interface", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Allows applying restoreable color palettes, fonts, and theme CSS." },
    PermissionDef { id: "storage.local", label: "Private local storage", category: "storage", sensitive: false, e2ee: false, device_only_default: false, description: "Stores the Plugin's private key-value data on the device." },
    PermissionDef { id: "storage.sync", label: "Synchronize Plugin data", category: "storage", sensitive: false, e2ee: false, device_only_default: false, description: "Sync configuration and non-sensitive data between devices." },
    PermissionDef { id: "events.lifecycle", label: "Plugin lifecycle events", category: "events", sensitive: false, e2ee: false, device_only_default: false, description: "Receive notifications when Plugins are enabled, disabled, updated or settings changed." },
    PermissionDef { id: "notifications.send", label: "Send notification", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Show in-app notifications to users." },
    PermissionDef { id: "clipboard.write", label: "Copy to clipboard", category: "device", sensitive: false, e2ee: false, device_only_default: true, description: "Copy text when users interact with the Plugin." },

    // Sensitive Data & Scope Management (Require Platform Approval + User Consent)
    PermissionDef { id: "profile.read", label: "Read profile information", category: "account", sensitive: true, e2ee: false, device_only_default: false, description: "Read your display name, public identifier, and account status." },
    PermissionDef { id: "contacts.read", label: "Read your friends list", category: "account", sensitive: true, e2ee: false, device_only_default: false, description: "View your friends list and online status." },
    PermissionDef { id: "contacts.write", label: "Manage contacts", category: "account", sensitive: true, e2ee: false, device_only_default: false, description: "Send or update friend status when the user operates." },
    PermissionDef { id: "messages.read", label: "Read channel/group messages", category: "messages", sensitive: true, e2ee: false, device_only_default: false, description: "Read messages in authorized channels or groups." },
    PermissionDef { id: "messages.write", label: "Send and edit messages", category: "messages", sensitive: true, e2ee: false, device_only_default: false, description: "Post messages, emotes or responses on behalf of the Plugin to the extent permitted." },
    PermissionDef { id: "messages.before_send", label: "Process information before sending", category: "messages", sensitive: true, e2ee: false, device_only_default: false, description: "Check or format messages before sending according to user consent." },
    PermissionDef { id: "groups.read", label: "Read group information", category: "groups", sensitive: true, e2ee: false, device_only_default: false, description: "View a list of groups, group channels, and group members." },
    PermissionDef { id: "groups.manage", label: "Manage group", category: "groups", sensitive: true, e2ee: false, device_only_default: false, description: "Update settings, channels, or coordinate team members." },
    PermissionDef { id: "servers.read", label: "Read server information", category: "servers", sensitive: true, e2ee: false, device_only_default: false, description: "View a list of server channels, roles, and members." },
    PermissionDef { id: "servers.manage", label: "Server management", category: "servers", sensitive: true, e2ee: false, device_only_default: false, description: "Create/update channels, roles, and server membership administration." },
    PermissionDef { id: "channels.read", label: "Read channel information", category: "channels", sensitive: true, e2ee: false, device_only_default: false, description: "View the list and metadata of allowed channels." },
    PermissionDef { id: "channels.manage", label: "Channel management", category: "channels", sensitive: true, e2ee: false, device_only_default: false, description: "Create, edit or change channel configuration in authorized servers." },
    PermissionDef { id: "calls.observe", label: "Track call events", category: "calls", sensitive: true, e2ee: false, device_only_default: true, description: "Get incoming, started and ended call status (no audio/visual)." },
    PermissionDef { id: "calls.manage", label: "Call control", category: "calls", sensitive: true, e2ee: false, device_only_default: true, description: "Supports creating meeting rooms or call control operations." },
    PermissionDef { id: "calls.media", label: "Access call audio/video", category: "calls", sensitive: true, e2ee: false, device_only_default: true, description: "Access media signals when everyone involved explicitly agrees." },
    PermissionDef { id: "files.read", label: "Read the attached file", category: "files", sensitive: true, e2ee: false, device_only_default: false, description: "Read attachments within the authorized chat or channel." },
    PermissionDef { id: "files.write", label: "Upload and save files", category: "files", sensitive: true, e2ee: false, device_only_default: false, description: "Upload files within the Plugin's capacity limit." },
    PermissionDef { id: "network.outbound", label: "External API connection", category: "network", sensitive: true, e2ee: false, device_only_default: false, description: "Send HTTPS requests to an external server via an anti-SSRF control layer." },
    PermissionDef { id: "webhooks.manage", label: "Manage Webhooks", category: "network", sensitive: true, e2ee: false, device_only_default: false, description: "Create and coordinate Incoming/Outgoing Webhooks within authorized scope." },
    PermissionDef { id: "bots.manage", label: "Register and manage Bots", category: "bots", sensitive: true, e2ee: false, device_only_default: false, description: "Create a Bot account separate from your real user identity." },

    // Four Explicitly Separated E2EE Data Governance Tiers (NEVER conflated, NEVER exposes private keys)
    PermissionDef { id: "e2ee.read_decrypted", label: "Read decoded E2EE content on the device", category: "e2ee", sensitive: true, e2ee: true, device_only_default: true, description: "Read decrypted text locally only in the specified chat (no root key issued)." },
    PermissionDef { id: "e2ee.process_local", label: "Process E2EE content locally on the device", category: "e2ee", sensitive: true, e2ee: true, device_only_default: true, description: "Only process decrypted data in the local sandbox and block sending to external networks." },
    PermissionDef { id: "e2ee.egress_external", label: "Send E2EE content to Plugin's external service", category: "e2ee", sensitive: true, e2ee: true, device_only_default: true, description: "High warning: allows sending decrypted content to a licensed external server." },
    PermissionDef { id: "e2ee.store_external", label: "Store E2EE content on an external server", category: "e2ee", sensitive: true, e2ee: true, device_only_default: true, description: "High warning: allow external service to store data from E2EE conversation." },
];

pub fn find_permission(id: &str) -> Option<&'static PermissionDef> {
    PERMISSION_CATALOG.iter().find(|p| p.id == id)
}

pub fn is_valid_permission(id: &str) -> bool {
    find_permission(id).is_some()
}

pub fn is_sensitive_permission(id: &str) -> bool {
    find_permission(id).map(|p| p.sensitive).unwrap_or(true)
}

/// Permissions that access a specific individual user's private profile, contacts, or decrypted E2EE messages.
/// These grants NEVER carry across users (a server/group admin's or another member's grant cannot grant access
/// to a different member's personal profile, contacts, or E2EE plaintext).
pub fn is_personal_permission(id: &str) -> bool {
    matches!(
        id,
        "profile.read" | "contacts.read" | "contacts.write"
    ) || id.starts_with("e2ee.")
}

pub fn is_valid_slug(s: &str) -> bool {
    let bytes = s.as_bytes();
    (2..=32).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes.iter().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_' || *b == b'-')
}

/// Immutable namespaced plugin identifier: `<publisher_slug>.<plugin_slug>`
pub fn is_valid_plugin_key(key: &str) -> bool {
    let Some((pub_slug, pkg_slug)) = key.split_once('.') else {
        return false;
    };
    !pkg_slug.contains('.') && is_valid_slug(pub_slug) && is_valid_slug(pkg_slug)
}

pub fn is_valid_semver(v: &str) -> bool {
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts.iter().all(|p| !p.is_empty() && p.len() <= 6 && p.bytes().all(|b| b.is_ascii_digit()))
}

pub fn is_safe_relative_path(path: &str) -> bool {
    if path.is_empty() || path.len() > 200 {
        return false;
    }
    if path.starts_with('/') || path.starts_with('\\') || path.contains('\0') || path.contains("..") || path.contains('\\') {
        return false;
    }
    true
}

pub fn is_safe_http_url(url: &str) -> bool {
    let u = url.trim();
    if u.is_empty() {
        return true;
    }
    if u.len() > 1024 {
        return false;
    }
    let lower = u.to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://"))
        && !lower.contains("javascript:")
        && !lower.contains("data:")
        && !lower.contains('\r')
        && !lower.contains('\n')
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PluginResourceLimits {
    #[serde(default = "default_mem_mb")]
    pub max_memory_mb: u32,
    #[serde(default = "default_timeout_ms")]
    pub max_exec_ms: u32,
    #[serde(default = "default_storage_kb")]
    pub max_storage_kb: u32,
    #[serde(default = "default_rpm")]
    pub max_requests_per_min: u32,
}

fn default_mem_mb() -> u32 { 32 }
fn default_timeout_ms() -> u32 { 3000 }
fn default_storage_kb() -> u32 { 512 }
fn default_rpm() -> u32 { 30 }

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PluginManifest {
    pub plugin_key: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub publisher: String,
    #[serde(default = "default_entry")]
    pub entry_point: String,
    #[serde(default = "default_runtime")]
    pub runtime: String, // "js" | "wasm" | "hybrid" | "declarative"
    #[serde(default = "default_sdk_ver")]
    pub sdk_version: String,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub ui_contributions: serde_json::Value,
    #[serde(default)]
    pub commands: Vec<serde_json::Value>,
    #[serde(default)]
    pub event_subscriptions: Vec<String>,
    #[serde(default)]
    pub settings_schema: Vec<serde_json::Value>,
    #[serde(default)]
    pub resource_limits: PluginResourceLimits,
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    #[serde(default)]
    pub allow_custom_logo: Option<bool>,
}

fn default_entry() -> String { "index.js".into() }
fn default_runtime() -> String { "js".into() }
fn default_sdk_ver() -> String { "1.0.0".into() }

pub const MAX_BUNDLE_CODE_BYTES: usize = 2 * 1024 * 1024; // 2 MB
pub const MAX_UI_HTML_BYTES: usize = 512 * 1024;          // 512 KB
pub const MAX_WASM_BYTES: usize = 4 * 1024 * 1024;        // 4 MB

pub fn compute_package_sha256(manifest_json: &str, bundle_code: &str, ui_html: &str, wasm_base64: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(manifest_json.as_bytes());
    hasher.update(b"\n---BUNDLE---\n");
    hasher.update(bundle_code.as_bytes());
    hasher.update(b"\n---UI---\n");
    hasher.update(ui_html.as_bytes());
    hasher.update(b"\n---WASM---\n");
    hasher.update(wasm_base64.as_bytes());
    hex::encode(hasher.finalize())
}

/// Canonical signing payload for cryptographic plugin package verification:
/// `chatw-plugin-sig-v1:<plugin_key>:<version>:<sha256_hex>`
pub fn canonical_package_signing_payload(plugin_key: &str, version: &str, sha256_hex: &str) -> String {
    format!(
        "chatw-plugin-sig-v1:{}:{}:{}",
        plugin_key.trim(),
        version.trim(),
        sha256_hex.trim().to_ascii_lowercase()
    )
}

fn decode_hex_or_b64(raw: &str) -> Result<Vec<u8>, &'static str> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("empty_encoded_key");
    }
    if s.bytes().all(|b| b.is_ascii_hexdigit()) && s.len() % 2 == 0 {
        if let Ok(v) = hex::decode(s) {
            return Ok(v);
        }
    }
    URL_SAFE_NO_PAD
        .decode(s)
        .or_else(|_| B64.decode(s))
        .map_err(|_| "invalid_key_or_signature_encoding")
}

/// Signs canonical package metadata using a P-256 ECDSA 32-byte secret scalar (hex or base64url).
pub fn sign_package_digest(
    secret_key_raw: &[u8; 32],
    plugin_key: &str,
    version: &str,
    sha256_hex: &str,
) -> Result<(String, String), &'static str> {
    let sk = EcdsaSigningKey::from_slice(secret_key_raw).map_err(|_| "invalid_signing_key")?;
    let vk_sec1 = sk.verifying_key().to_encoded_point(false);
    let payload = canonical_package_signing_payload(plugin_key, version, sha256_hex);
    let sig: EcdsaSignature = sk.sign(payload.as_bytes());
    Ok((
        hex::encode(vk_sec1.as_bytes()),
        hex::encode(sig.to_bytes()),
    ))
}

/// Verifies a cryptographic P-256 ECDSA signature (`signature_encoded`) against the publisher's
/// registered public key (`public_key_encoded`, SEC1 compressed/uncompressed hex or base64url)
/// over `chatw-plugin-sig-v1:<plugin_key>:<version>:<sha256_hex>`.
pub fn verify_package_signature(
    public_key_encoded: &str,
    signature_encoded: &str,
    plugin_key: &str,
    version: &str,
    sha256_hex: &str,
) -> Result<(), &'static str> {
    let pk_bytes = decode_hex_or_b64(public_key_encoded)?;
    let vk = EcdsaVerifyingKey::from_sec1_bytes(&pk_bytes).map_err(|_| "invalid_publisher_public_key")?;
    let sig_bytes = decode_hex_or_b64(signature_encoded)?;
    let sig = EcdsaSignature::from_slice(&sig_bytes)
        .or_else(|_| EcdsaSignature::from_der(&sig_bytes))
        .map_err(|_| "invalid_package_signature_format")?;
    let payload = canonical_package_signing_payload(plugin_key, version, sha256_hex);
    vk.verify(payload.as_bytes(), &sig)
        .map_err(|_| "package_signature_verification_failed")
}

pub fn validate_manifest_and_artifacts(
    manifest: &PluginManifest,
    expected_publisher_slug: &str,
    bundle_code: &str,
    ui_html: &str,
    wasm_base64: &str,
    declared_sha256: Option<&str>,
) -> Result<(String, usize, Vec<String>), &'static str> {
    if !is_valid_plugin_key(&manifest.plugin_key) {
        return Err("invalid_plugin_key");
    }
    if !manifest.plugin_key.starts_with(&format!("{expected_publisher_slug}.")) {
        return Err("plugin_key_namespace_mismatch");
    }
    if manifest.name.trim().len() < 2 || manifest.name.trim().len() > 80 {
        return Err("invalid_plugin_name");
    }
    if !is_valid_semver(&manifest.version) {
        return Err("invalid_semver");
    }
    if !is_safe_relative_path(&manifest.entry_point) {
        return Err("invalid_entry_point_path");
    }
    if !matches!(manifest.runtime.as_str(), "js" | "wasm" | "hybrid" | "declarative") {
        return Err("unsupported_runtime");
    }
    if bundle_code.len() > MAX_BUNDLE_CODE_BYTES {
        return Err("bundle_too_large");
    }
    if ui_html.len() > MAX_UI_HTML_BYTES {
        return Err("ui_html_too_large");
    }

    let mut total_size = bundle_code.len() + ui_html.len();
    if !wasm_base64.trim().is_empty() {
        let wasm_bytes = B64.decode(wasm_base64.trim()).map_err(|_| "invalid_wasm_base64")?;
        if wasm_bytes.len() > MAX_WASM_BYTES {
            return Err("wasm_too_large");
        }
        // Validate standard WebAssembly binary magic header: \0asm + version 1
        if wasm_bytes.len() < 8 || &wasm_bytes[..8] != b"\0asm\x01\x00\x00\x00" {
            return Err("invalid_wasm_binary");
        }
        total_size += wasm_bytes.len();
    } else if manifest.runtime == "wasm" {
        return Err("wasm_binary_required");
    }

    let mut clean_perms = Vec::new();
    for p in &manifest.permissions {
        if !is_valid_permission(p) {
            return Err("unknown_permission");
        }
        if !clean_perms.contains(p) {
            clean_perms.push(p.clone());
        }
    }

    if manifest.resource_limits.max_memory_mb > 256
        || manifest.resource_limits.max_exec_ms > 15_000
        || manifest.resource_limits.max_storage_kb > 10_240
    {
        return Err("resource_limits_exceeded");
    }

    let manifest_canonical = serde_json::to_string(manifest).map_err(|_| "invalid_manifest")?;
    let computed_sha = compute_package_sha256(&manifest_canonical, bundle_code, ui_html, wasm_base64);
    if let Some(decl) = declared_sha256.filter(|s| !s.trim().is_empty()) {
        if !decl.trim().eq_ignore_ascii_case(&computed_sha) {
            return Err("sha256_mismatch");
        }
    }

    Ok((computed_sha, total_size, clean_perms))
}

// ============================================================================
// Discord-style Incoming Webhook Payload & Embed Schema Validation
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EmbedAuthor {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EmbedMedia {
    pub url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EmbedFooter {
    pub text: String,
    #[serde(default)]
    pub icon_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub inline: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DiscordEmbed {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub color: Option<u32>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub author: Option<EmbedAuthor>,
    #[serde(default)]
    pub thumbnail: Option<EmbedMedia>,
    #[serde(default)]
    pub image: Option<EmbedMedia>,
    #[serde(default)]
    pub footer: Option<EmbedFooter>,
    #[serde(default)]
    pub fields: Vec<EmbedField>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct WebhookComponentBtn {
    pub label: String,
    #[serde(default)]
    pub style: Option<String>, // "primary" | "secondary" | "danger" | "link"
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub custom_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct IncomingWebhookPayload {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub embeds: Vec<DiscordEmbed>,
    #[serde(default)]
    pub components: Vec<WebhookComponentBtn>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

pub fn validate_incoming_webhook_payload(p: &IncomingWebhookPayload) -> Result<(), &'static str> {
    let has_content = p.content.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false);
    let has_embeds = !p.embeds.is_empty();
    if !has_content && !has_embeds {
        return Err("empty_webhook_payload");
    }
    if let Some(c) = &p.content {
        if c.len() > 4000 {
            return Err("content_too_long");
        }
    }
    if let Some(u) = &p.username {
        if u.len() > 80 {
            return Err("username_too_long");
        }
    }
    if let Some(av) = &p.avatar_url {
        if !is_safe_http_url(av) {
            return Err("invalid_avatar_url");
        }
    }
    if p.embeds.len() > 10 {
        return Err("too_many_embeds");
    }
    if p.components.len() > 5 {
        return Err("too_many_components");
    }
    for btn in &p.components {
        if btn.label.trim().is_empty() || btn.label.len() > 80 {
            return Err("invalid_component_label");
        }
        if let Some(url) = &btn.url {
            if !is_safe_http_url(url) {
                return Err("invalid_component_url");
            }
        }
    }
    let mut total_chars = 0usize;
    for emb in &p.embeds {
        if let Some(t) = &emb.title {
            if t.len() > 256 { return Err("embed_title_too_long"); }
            total_chars += t.len();
        }
        if let Some(d) = &emb.description {
            if d.len() > 4096 { return Err("embed_description_too_long"); }
            total_chars += d.len();
        }
        if let Some(u) = &emb.url {
            if !is_safe_http_url(u) { return Err("invalid_embed_url"); }
        }
        if let Some(a) = &emb.author {
            if a.name.len() > 256 { return Err("embed_author_too_long"); }
            if let Some(u) = &a.url { if !is_safe_http_url(u) { return Err("invalid_embed_url"); } }
            if let Some(i) = &a.icon_url { if !is_safe_http_url(i) { return Err("invalid_embed_url"); } }
            total_chars += a.name.len();
        }
        if let Some(th) = &emb.thumbnail {
            if !is_safe_http_url(&th.url) { return Err("invalid_embed_url"); }
        }
        if let Some(im) = &emb.image {
            if !is_safe_http_url(&im.url) { return Err("invalid_embed_url"); }
        }
        if let Some(f) = &emb.footer {
            if f.text.len() > 2048 { return Err("embed_footer_too_long"); }
            if let Some(i) = &f.icon_url { if !is_safe_http_url(i) { return Err("invalid_embed_url"); } }
            total_chars += f.text.len();
        }
        if emb.fields.len() > 25 {
            return Err("too_many_embed_fields");
        }
        for fld in &emb.fields {
            if fld.name.trim().is_empty() || fld.name.len() > 256 || fld.value.trim().is_empty() || fld.value.len() > 1024 {
                return Err("invalid_embed_field");
            }
            total_chars += fld.name.len() + fld.value.len();
        }
    }
    if total_chars > 6000 {
        return Err("embed_total_too_large");
    }
    Ok(())
}
