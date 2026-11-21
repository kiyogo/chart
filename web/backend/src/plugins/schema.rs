//! Official ChatW Plugin Permission Catalog, Manifest Schema & Discord Embed Validators.

use base64::{Engine, engine::general_purpose::STANDARD as B64};
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
    PermissionDef { id: "ui.page", label: "Tạo trang riêng", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Hiển thị giao diện riêng của Plugin trong khu vực nội dung chính." },
    PermissionDef { id: "ui.sidebar", label: "Mở ở cột bên trái", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Hiển thị giao diện gọn của Plugin song song với khung trò chuyện." },
    PermissionDef { id: "ui.dialog", label: "Mở hộp thoại", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Hiển thị cửa sổ hộp thoại tương tác trong sandbox." },
    PermissionDef { id: "ui.message_action", label: "Thêm nút vào tin nhắn", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Bổ sung nút hành động hoặc mục trình đơn ngữ cảnh trên tin nhắn." },
    PermissionDef { id: "ui.composer", label: "Mở rộng khung soạn tin", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Thêm nút thao tác nhanh hoặc bảng mở rộng cạnh ô nhập tin." },
    PermissionDef { id: "ui.server_tab", label: "Trang quản trị máy chủ", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Bổ sung tab quản lý hoặc bảng điều khiển trong máy chủ." },
    PermissionDef { id: "ui.group_tab", label: "Trang mở rộng nhóm", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Bổ sung giao diện tiện ích trong nhóm trò chuyện." },
    PermissionDef { id: "ui.profile_tab", label: "Thẻ mở rộng hồ sơ", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Hiển thị tab hoặc thẻ thông tin trong hồ sơ người dùng." },
    PermissionDef { id: "ui.call_panel", label: "Tiện ích trong cuộc gọi", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Thêm nút điều khiển hoặc giao diện phụ trợ trong cuộc gọi." },
    PermissionDef { id: "commands.register", label: "Đăng ký lệnh gõ tắt", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Đăng ký các lệnh /command trong khung soạn tin." },
    PermissionDef { id: "theme.apply", label: "Tùy chỉnh giao diện ChatW", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Cho phép áp dụng bảng màu, phông chữ và CSS chủ đề có thể khôi phục." },
    PermissionDef { id: "storage.local", label: "Lưu trữ cục bộ riêng", category: "storage", sensitive: false, e2ee: false, device_only_default: false, description: "Lưu trữ dữ liệu khóa-giá trị riêng của Plugin trên thiết bị." },
    PermissionDef { id: "storage.sync", label: "Đồng bộ dữ liệu Plugin", category: "storage", sensitive: false, e2ee: false, device_only_default: false, description: "Đồng bộ cấu hình và dữ liệu không nhạy cảm giữa các thiết bị." },
    PermissionDef { id: "events.lifecycle", label: "Sự kiện vòng đời Plugin", category: "events", sensitive: false, e2ee: false, device_only_default: false, description: "Nhận thông báo khi Plugin được bật, tắt, cập nhật hoặc đổi cài đặt." },
    PermissionDef { id: "notifications.send", label: "Gửi thông báo", category: "ui", sensitive: false, e2ee: false, device_only_default: false, description: "Hiển thị thông báo trong ứng dụng cho người dùng." },
    PermissionDef { id: "clipboard.write", label: "Sao chép vào bộ nhớ tạm", category: "device", sensitive: false, e2ee: false, device_only_default: true, description: "Sao chép văn bản khi người dùng tương tác với Plugin." },

    // Sensitive Data & Scope Management (Require Platform Approval + User Consent)
    PermissionDef { id: "profile.read", label: "Đọc thông tin hồ sơ", category: "account", sensitive: true, e2ee: false, device_only_default: false, description: "Đọc tên hiển thị, định danh công khai và trạng thái tài khoản." },
    PermissionDef { id: "contacts.read", label: "Đọc danh sách bạn bè", category: "account", sensitive: true, e2ee: false, device_only_default: false, description: "Xem danh sách bạn bè và trạng thái trực tuyến." },
    PermissionDef { id: "contacts.write", label: "Quản lý danh bạ", category: "account", sensitive: true, e2ee: false, device_only_default: false, description: "Gửi hoặc cập nhật trạng thái kết bạn khi người dùng thao tác." },
    PermissionDef { id: "messages.read", label: "Đọc tin nhắn kênh/nhóm", category: "messages", sensitive: true, e2ee: false, device_only_default: false, description: "Đọc tin nhắn trong các kênh hoặc nhóm thuộc phạm vi được cấp quyền." },
    PermissionDef { id: "messages.write", label: "Gửi và chỉnh sửa tin nhắn", category: "messages", sensitive: true, e2ee: false, device_only_default: false, description: "Đăng tin nhắn, biểu cảm hoặc phản hồi thay mặt Plugin trong phạm vi cho phép." },
    PermissionDef { id: "messages.before_send", label: "Xử lý tin trước khi gửi", category: "messages", sensitive: true, e2ee: false, device_only_default: false, description: "Kiểm tra hoặc định dạng tin nhắn trước khi gửi theo sự đồng ý của người dùng." },
    PermissionDef { id: "groups.read", label: "Đọc thông tin nhóm", category: "groups", sensitive: true, e2ee: false, device_only_default: false, description: "Xem danh sách nhóm, kênh nhóm và thành viên nhóm." },
    PermissionDef { id: "groups.manage", label: "Quản lý nhóm", category: "groups", sensitive: true, e2ee: false, device_only_default: false, description: "Cập nhật cài đặt, kênh hoặc điều phối thành viên nhóm." },
    PermissionDef { id: "servers.read", label: "Đọc thông tin máy chủ", category: "servers", sensitive: true, e2ee: false, device_only_default: false, description: "Xem danh sách kênh, vai trò và thành viên của máy chủ." },
    PermissionDef { id: "servers.manage", label: "Quản lý máy chủ", category: "servers", sensitive: true, e2ee: false, device_only_default: false, description: "Tạo/cập nhật kênh, vai trò và quản trị thành viên máy chủ." },
    PermissionDef { id: "channels.read", label: "Đọc thông tin kênh", category: "channels", sensitive: true, e2ee: false, device_only_default: false, description: "Xem danh sách và siêu dữ liệu các kênh được cho phép." },
    PermissionDef { id: "channels.manage", label: "Quản lý kênh", category: "channels", sensitive: true, e2ee: false, device_only_default: false, description: "Tạo, sửa hoặc đổi cấu hình kênh trong máy chủ được cấp quyền." },
    PermissionDef { id: "calls.observe", label: "Theo dõi sự kiện cuộc gọi", category: "calls", sensitive: true, e2ee: false, device_only_default: true, description: "Nhận trạng thái cuộc gọi đến, bắt đầu và kết thúc (không có âm thanh/hình ảnh)." },
    PermissionDef { id: "calls.manage", label: "Điều khiển cuộc gọi", category: "calls", sensitive: true, e2ee: false, device_only_default: true, description: "Hỗ trợ tạo phòng họp hoặc thao tác điều khiển cuộc gọi." },
    PermissionDef { id: "calls.media", label: "Truy cập âm thanh/hình ảnh cuộc gọi", category: "calls", sensitive: true, e2ee: false, device_only_default: true, description: "Truy cập tín hiệu media khi mọi người tham gia đồng ý rõ ràng." },
    PermissionDef { id: "files.read", label: "Đọc tệp đính kèm", category: "files", sensitive: true, e2ee: false, device_only_default: false, description: "Đọc tệp đính kèm trong phạm vi cuộc trò chuyện hoặc kênh được cấp quyền." },
    PermissionDef { id: "files.write", label: "Tải lên và lưu tệp", category: "files", sensitive: true, e2ee: false, device_only_default: false, description: "Tải lên tệp trong hạn mức dung lượng của Plugin." },
    PermissionDef { id: "network.outbound", label: "Kết nối API bên ngoài", category: "network", sensitive: true, e2ee: false, device_only_default: false, description: "Gửi yêu cầu HTTPS ra máy chủ bên ngoài qua lớp kiểm soát chống SSRF." },
    PermissionDef { id: "webhooks.manage", label: "Quản lý Webhook", category: "network", sensitive: true, e2ee: false, device_only_default: false, description: "Tạo và điều phối Incoming/Outgoing Webhook trong phạm vi được cấp quyền." },
    PermissionDef { id: "bots.manage", label: "Đăng ký và quản lý Bot", category: "bots", sensitive: true, e2ee: false, device_only_default: false, description: "Tạo tài khoản Bot tách biệt với danh tính người dùng thật." },

    // Four Explicitly Separated E2EE Data Governance Tiers (NEVER conflated, NEVER exposes private keys)
    PermissionDef { id: "e2ee.read_decrypted", label: "Đọc nội dung E2EE đã giải mã trên thiết bị", category: "e2ee", sensitive: true, e2ee: true, device_only_default: true, description: "Đọc văn bản đã giải mã cục bộ chỉ trong cuộc trò chuyện được chỉ định (không cấp khóa gốc)." },
    PermissionDef { id: "e2ee.process_local", label: "Xử lý nội dung E2EE cục bộ trên thiết bị", category: "e2ee", sensitive: true, e2ee: true, device_only_default: true, description: "Chỉ xử lý dữ liệu đã giải mã ngay trong sandbox cục bộ và chặn gửi ra mạng ngoài." },
    PermissionDef { id: "e2ee.egress_external", label: "Gửi nội dung E2EE ra dịch vụ ngoài của Plugin", category: "e2ee", sensitive: true, e2ee: true, device_only_default: true, description: "Cảnh báo cao: cho phép gửi nội dung đã giải mã tới máy chủ ngoài được cấp phép." },
    PermissionDef { id: "e2ee.store_external", label: "Lưu trữ nội dung E2EE trên máy chủ ngoài", category: "e2ee", sensitive: true, e2ee: true, device_only_default: true, description: "Cảnh báo rất cao: cho phép dịch vụ ngoài lưu trữ dữ liệu từ cuộc trò chuyện E2EE." },
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
