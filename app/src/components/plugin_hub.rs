//! ChatW Plugin Platform Hub UI (Spec 037)
//! - Marketplace with search, filters, verified checkmark, and mandatory Permission Consent Modal
//! - Installed Sandboxed Plugins manager with live custom HTML/CSS/JS/WASM iframe host
//! - Privacy Dashboard showing all active & E2EE grants with instant revocation & audit logs
//! - Discord-compatible Incoming & Outgoing Webhook Manager (User, Server, Channel, Group scopes)
//! - Developer Portal (Publisher registration, verification request, direct JS/WASM bundle publishing)

use dioxus::prelude::*;
use serde_json::{Value, json};

use crate::{
    components::icons::{CheckIcon, CloseIcon, CopyIcon, DownloadIcon, PlusIcon, RefreshIcon, ShieldIcon, TrashIcon},
    models::ChannelItem,
    platform,
    services::plugin_api::{
        self, InstallationItem, MarketplaceItem, PermissionCatalogItem, PrivacyDashboardData,
        PublisherInfo, WebhookDeliveryItem, WebhookItem,
    },
};

pub static PLUGIN_INSTALLATIONS: GlobalSignal<Vec<InstallationItem>> = Signal::global(Vec::new);
pub static ACTIVE_SANDBOX_INSTALL_ID: GlobalSignal<Option<String>> = Signal::global(|| None);

pub async fn reload_user_installations() {
    if let Ok(list) = plugin_api::list_installations(None, None).await {
        plugin_api::host_sync_installations(&list);
        *PLUGIN_INSTALLATIONS.write() = list;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HubTab {
    Marketplace,
    Installed,
    Privacy,
    Webhooks,
    Developer,
}

/// Complete Plugin Platform Hub embedded inside PluginSettings & Plugin Views.
#[component]
pub fn PluginHubPanel(my_user_id: String, my_username: String) -> Element {
    let mut tab = use_signal(|| HubTab::Marketplace);
    let mut status_msg = use_signal(|| Option::<(bool, String)>::None);

    // Marketplace state
    let mut query = use_signal(String::new);
    let mut verified_only = use_signal(|| false);
    let mut market_items = use_signal(Vec::<MarketplaceItem>::new);
    let mut perm_catalog = use_signal(Vec::<PermissionCatalogItem>::new);
    let mut loading_market = use_signal(|| false);

    // Consent modal state
    let mut consent_pkg = use_signal(|| Option::<MarketplaceItem>::None);
    let mut consent_preview = use_signal(|| Option::<Value>::None);
    let mut consent_checked = use_signal(|| false);
    let mut consent_selected_perms = use_signal(Vec::<String>::new);
    let mut consent_busy = use_signal(|| false);

    // Privacy state
    let mut privacy = use_signal(PrivacyDashboardData::default);

    // Developer portal state
    let mut my_pub = use_signal(|| Option::<PublisherInfo>::None);
    let mut pub_slug = use_signal(String::new);
    let mut pub_name = use_signal(String::new);
    let mut pub_web = use_signal(String::new);
    let mut pub_bio = use_signal(String::new);
    let mut pkg_json_input = use_signal(String::new);

    // Sandbox UI viewer state
    let active_sandbox_id = ACTIVE_SANDBOX_INSTALL_ID();
    let installed_list = PLUGIN_INSTALLATIONS();

    // Initial load
    use_effect(move || {
        spawn(async move {
            loading_market.set(true);
            if let Ok(cat) = plugin_api::get_permissions_catalog().await {
                perm_catalog.set(cat);
            }
            if let Ok(items) = plugin_api::list_marketplace("", "all", false).await {
                market_items.set(items);
            }
            reload_user_installations().await;
            if let Ok(p) = plugin_api::get_privacy_dashboard().await {
                privacy.set(p);
            }
            if let Ok(pub_info) = plugin_api::get_my_publisher().await {
                my_pub.set(pub_info);
            }
            loading_market.set(false);
        });
    });

    // Mount sandboxed iframe whenever active_sandbox_id or tab changes to Installed
    let cur_tab = tab();
    use_effect(use_reactive!(|active_sandbox_id, cur_tab| {
        if cur_tab == HubTab::Installed {
            if let Some(ref iid) = active_sandbox_id {
                plugin_api::host_mount_ui("#plugin-hub-sandbox-stage", iid, "page");
            }
        }
    }));

    let refresh_marketplace = move |q: String, ver: bool| {
        spawn(async move {
            loading_market.set(true);
            if let Ok(items) = plugin_api::list_marketplace(&q, "all", ver).await {
                market_items.set(items);
            }
            loading_market.set(false);
        });
    };

    let mut open_consent = move |pkg: MarketplaceItem| {
        let pkg_id = pkg.id.clone();
        let default_perms: Vec<String> = pkg
            .requested_permissions
            .iter()
            .filter(|p| !p.starts_with("e2ee."))
            .cloned()
            .collect();
        consent_selected_perms.set(default_perms);
        consent_checked.set(false);
        consent_preview.set(None);
        consent_pkg.set(Some(pkg));
        spawn(async move {
            if let Ok(prev) = plugin_api::preview_install(&pkg_id).await {
                consent_preview.set(Some(prev));
            }
        });
    };

    let catalog_map = perm_catalog();

    rsx! {
        div { class: "plugin-hub-root", "data-plugin-hub": "v2",
            div { class: "plugin-hub-tabs", role: "tablist",
                for (t, label) in [
                    (HubTab::Marketplace, "Chợ Plugin"),
                    (HubTab::Installed, "Đã cài đặt"),
                    (HubTab::Privacy, "Quyền riêng tư"),
                    (HubTab::Webhooks, "Webhook"),
                    (HubTab::Developer, "Nhà phát triển"),
                ] {
                    button {
                        key: "{label}",
                        r#type: "button",
                        role: "tab",
                        aria_selected: "{tab() == t}",
                        class: if tab() == t { "filter-chip active" } else { "filter-chip" },
                        onclick: move |_| {
                            tab.set(t);
                            if t == HubTab::Privacy {
                                spawn(async move {
                                    if let Ok(p) = plugin_api::get_privacy_dashboard().await {
                                        privacy.set(p);
                                    }
                                });
                            } else if t == HubTab::Installed {
                                spawn(async move { reload_user_installations().await; });
                            } else if t == HubTab::Developer {
                                spawn(async move {
                                    if let Ok(info) = plugin_api::get_my_publisher().await {
                                        my_pub.set(info);
                                    }
                                });
                            }
                        },
                        "{label}"
                        if t == HubTab::Installed && !installed_list.is_empty() {
                            span { class: "plugin-hub-count", "{installed_list.len()}" }
                        }
                    }
                }
            }

            if let Some((is_err, text)) = status_msg() {
                div {
                    class: if is_err { "status-badge error" } else { "status-badge success" },
                    role: "status",
                    style: "margin-bottom: 10px;",
                    "{text}"
                }
            }

            // =================================================================
            // TAB 1: MARKETPLACE
            // =================================================================
            if tab() == HubTab::Marketplace {
                div { class: "plugin-hub-toolbar",
                    input {
                        class: "plugin-hub-search",
                        r#type: "search",
                        placeholder: "Tìm kiếm plugin theo tên, tính năng hoặc nhà phát triển",
                        value: "{query}",
                        oninput: move |e| {
                            let v = e.value();
                            query.set(v.clone());
                            refresh_marketplace(v, verified_only());
                        },
                    }
                    label { class: "plugin-hub-filter-toggle",
                        input {
                            r#type: "checkbox",
                            class: "switch",
                            checked: verified_only(),
                            onchange: move |e| {
                                let v = e.checked();
                                verified_only.set(v);
                                refresh_marketplace(query(), v);
                            },
                        }
                        span { "Đã xác minh" }
                    }
                    button {
                        r#type: "button",
                        class: "btn-ghost",
                        onclick: move |_| refresh_marketplace(query(), verified_only()),
                        RefreshIcon { size: 15 }
                        "Làm mới"
                    }
                }

                if loading_market() {
                    div { class: "sidebar-empty", "Đang tải danh mục plugin" }
                } else if market_items().is_empty() {
                    div { class: "sidebar-empty", "Chưa có plugin phù hợp với bộ lọc hiện tại" }
                } else {
                    div { class: "plugin-hub-grid",
                        for pkg in market_items() {
                            {
                                let is_installed = installed_list.iter().any(|i| i.plugin_key == pkg.plugin_key);
                                let pkg_for_open = pkg.clone();
                                let inst_id_opt = installed_list.iter().find(|i| i.plugin_key == pkg.plugin_key).map(|i| i.id.clone());
                                let runtime_label = match pkg.runtime.as_str() {
                                    "wasm" => "Rust WASM",
                                    "hybrid" => "JS + WASM",
                                    _ => "JS Sandbox",
                                };
                                rsx! {
                                    div { key: "{pkg.id}", class: "plugin-hub-card", "data-plugin-key": "{pkg.plugin_key}",
                                        div { class: "plugin-hub-card-head",
                                            div { class: "plugin-hub-card-title-row",
                                                strong { "{pkg.name}" }
                                                if pkg.publisher_verified {
                                                    span { class: "plugin-verified-badge", title: "Nhà phát triển đã xác minh",
                                                        CheckIcon { size: 12 }
                                                        "Đã xác minh"
                                                    }
                                                }
                                            }
                                            span { class: "plugin-runtime-chip", "{runtime_label} · v{pkg.latest_version}" }
                                        }
                                        div { class: "plugin-hub-meta",
                                            span { "{pkg.publisher_name} ({pkg.plugin_key})" }
                                            span { "{pkg.install_count} lượt cài" }
                                        }
                                        if !pkg.summary.is_empty() {
                                            p { class: "plugin-hub-summary", "{pkg.summary}" }
                                        }
                                        if !pkg.requested_permissions.is_empty() {
                                            div { class: "plugin-perm-pills",
                                                for perm in pkg.requested_permissions.iter().take(5) {
                                                    span {
                                                        key: "{perm}",
                                                        class: if perm.starts_with("e2ee.") || perm == "network.outbound" { "plugin-perm-pill sensitive" } else { "plugin-perm-pill" },
                                                        "{perm}"
                                                    }
                                                }
                                            }
                                        }
                                        div { class: "plugin-hub-card-foot",
                                            if is_installed {
                                                button {
                                                    r#type: "button",
                                                    class: "btn-secondary",
                                                    onclick: move |_| {
                                                        if let Some(iid) = inst_id_opt.clone() {
                                                            *ACTIVE_SANDBOX_INSTALL_ID.write() = Some(iid);
                                                            tab.set(HubTab::Installed);
                                                        }
                                                    },
                                                    "Mở Plugin"
                                                }
                                            } else {
                                                button {
                                                    r#type: "button",
                                                    class: "btn-primary",
                                                    "data-install-plugin": "{pkg.plugin_key}",
                                                    onclick: move |_| open_consent(pkg_for_open.clone()),
                                                    DownloadIcon { size: 15 }
                                                    "Cài đặt"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // =================================================================
            // TAB 2: INSTALLED SANDBOXED PLUGINS & LIVE CUSTOM UI STAGE
            // =================================================================
            if tab() == HubTab::Installed {
                if installed_list.is_empty() {
                    div { class: "sidebar-empty", "Bạn chưa cài đặt plugin nền tảng nào từ Chợ Plugin hoặc cổng Nhà phát triển." }
                } else {
                    div { class: "plugin-installed-list",
                        for inst in installed_list.clone() {
                            {
                                let iid_open = inst.id.clone();
                                let iid_toggle = inst.id.clone();
                                let iid_del = inst.id.clone();
                                let cur_enabled = inst.enabled;
                                let is_open = active_sandbox_id.as_deref() == Some(inst.id.as_str());
                                let runtime_tag = if inst.runtime == "wasm" { "Rust WASM" } else if inst.runtime == "hybrid" { "JS + WASM" } else { "JS Sandbox" };
                                rsx! {
                                    div { key: "{inst.id}", class: "plugin-installed-row", "data-installed-key": "{inst.plugin_key}",
                                        div { class: "grow",
                                            div { class: "plugin-hub-card-title-row",
                                                strong { "{inst.name}" }
                                                span { class: "plugin-runtime-chip", "{runtime_tag} · v{inst.version}" }
                                                span { class: "plugin-perm-pill", "{inst.scope_type}" }
                                            }
                                            div { class: "plugin-hub-meta",
                                                span { "{inst.plugin_key}" }
                                                span { "{inst.grants.iter().filter(|g| g.revoked_at.is_none()).count()} quyền đang cấp" }
                                            }
                                        }
                                        div { class: "plugin-installed-actions",
                                            if !inst.ui_html.trim().is_empty() {
                                                button {
                                                    r#type: "button",
                                                    class: if is_open { "btn-primary" } else { "btn-secondary" },
                                                    "data-open-sandbox": "{inst.plugin_key}",
                                                    onclick: move |_| {
                                                        if is_open {
                                                            *ACTIVE_SANDBOX_INSTALL_ID.write() = None;
                                                        } else {
                                                            *ACTIVE_SANDBOX_INSTALL_ID.write() = Some(iid_open.clone());
                                                        }
                                                    },
                                                    if is_open { "Đóng giao diện" } else { "Mở giao diện" }
                                                }
                                            }
                                            input {
                                                r#type: "checkbox",
                                                class: "switch",
                                                checked: cur_enabled,
                                                "data-toggle-plugin": "{inst.plugin_key}",
                                                onchange: move |e| {
                                                    let next = e.checked();
                                                    let id = iid_toggle.clone();
                                                    spawn(async move {
                                                        match plugin_api::update_installation(&id, json!({ "enabled": next })).await {
                                                            Ok(_) => {
                                                                if !next && ACTIVE_SANDBOX_INSTALL_ID.peek().as_deref() == Some(id.as_str()) {
                                                                    *ACTIVE_SANDBOX_INSTALL_ID.write() = None;
                                                                }
                                                                reload_user_installations().await;
                                                                status_msg.set(Some((false, if next { "Đã bật plugin".into() } else { "Đã tắt và dừng tiến trình sandbox của plugin".into() })));
                                                            }
                                                            Err(err) => status_msg.set(Some((true, format!("Không thể cập nhật trạng thái: {}", err.code)))),
                                                        }
                                                    });
                                                },
                                            }
                                            button {
                                                r#type: "button",
                                                class: "icon-btn danger",
                                                title: "Gỡ cài đặt plugin",
                                                "data-uninstall-plugin": "{inst.plugin_key}",
                                                onclick: move |_| {
                                                    let id = iid_del.clone();
                                                    spawn(async move {
                                                        match plugin_api::delete_installation(&id).await {
                                                            Ok(_) => {
                                                                if ACTIVE_SANDBOX_INSTALL_ID.peek().as_deref() == Some(id.as_str()) {
                                                                    *ACTIVE_SANDBOX_INSTALL_ID.write() = None;
                                                                }
                                                                reload_user_installations().await;
                                                                status_msg.set(Some((false, "Đã gỡ cài đặt plugin và thu hồi toàn bộ quyền".into())));
                                                            }
                                                            Err(err) => status_msg.set(Some((true, format!("Lỗi gỡ cài đặt: {}", err.code)))),
                                                        }
                                                    });
                                                },
                                                TrashIcon { size: 15 }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if let Some(ref open_id) = active_sandbox_id {
                        {
                            let open_name = installed_list
                                .iter()
                                .find(|i| i.id == *open_id)
                                .map(|i| i.name.clone())
                                .unwrap_or_else(|| "Plugin Sandbox".into());
                            rsx! {
                                div { class: "plugin-sandbox-shell",
                                    div { class: "plugin-sandbox-bar",
                                        span { strong { "{open_name}" } " · Chạy trong Sandbox cách ly (Opaque Origin)" }
                                        button {
                                            r#type: "button",
                                            class: "icon-btn",
                                            onclick: move |_| *ACTIVE_SANDBOX_INSTALL_ID.write() = None,
                                            CloseIcon { size: 16 }
                                        }
                                    }
                                    div { id: "plugin-hub-sandbox-stage", class: "plugin-sandbox-stage" }
                                }
                            }
                        }
                    }
                }
            }

            // =================================================================
            // TAB 3: PRIVACY DASHBOARD & INSTANT REVOCATION
            // =================================================================
            if tab() == HubTab::Privacy {
                div { class: "plugin-privacy-section",
                    div { class: "plugin-hub-toolbar",
                        div { class: "grow",
                            strong { "Bảng kiểm soát Quyền riêng tư & Dữ liệu E2EE" }
                            div { class: "settings-row-desc", "Thu hồi quyền có hiệu lực ngay lập tức đối với mọi tiến trình sandbox đang chạy." }
                        }
                        button {
                            r#type: "button",
                            class: "btn-ghost",
                            onclick: move |_| {
                                spawn(async move {
                                    if let Ok(p) = plugin_api::get_privacy_dashboard().await {
                                        privacy.set(p);
                                    }
                                });
                            },
                            RefreshIcon { size: 15 }
                            "Làm mới"
                        }
                    }

                    {
                        let active_grants: Vec<_> = privacy().grants.into_iter().filter(|g| g.revoked_at.is_none()).collect();
                        rsx! {
                            if active_grants.is_empty() {
                                div { class: "sidebar-empty", "Chưa có quyền nào đang được cấp cho plugin." }
                            } else {
                                div { class: "plugin-installed-list",
                                    for g in active_grants {
                                        {
                                            let iid = g.installation_id.clone();
                                            let gid = g.id.clone();
                                            let perm_label = if g.permission_label.is_empty() { g.permission.clone() } else { g.permission_label.clone() };
                                            rsx! {
                                                div { key: "{g.id}", class: "plugin-installed-row", "data-grant-perm": "{g.permission}",
                                                    div { class: "grow",
                                                        div { class: "plugin-hub-card-title-row",
                                                            strong { "{g.plugin_name}" }
                                                            span {
                                                                class: if g.e2ee || g.sensitive { "plugin-perm-pill sensitive" } else { "plugin-perm-pill" },
                                                                "{perm_label} ({g.permission})"
                                                            }
                                                            if g.device_only {
                                                                span { class: "plugin-perm-pill", "Chỉ thiết bị này" }
                                                            }
                                                        }
                                                        div { class: "plugin-hub-meta",
                                                            span { "Phạm vi: {g.scope_type}" }
                                                            span { "Cấp lúc: {g.granted_at}" }
                                                        }
                                                    }
                                                    button {
                                                        r#type: "button",
                                                        class: "btn-secondary",
                                                        "data-revoke-grant": "{g.id}",
                                                        onclick: move |_| {
                                                            let iid2 = iid.clone();
                                                            let gid2 = gid.clone();
                                                            spawn(async move {
                                                                match plugin_api::revoke_grant(&iid2, &gid2).await {
                                                                    Ok(_) => {
                                                                        plugin_api::host_revoke_grant_local(&iid2, &gid2);
                                                                        reload_user_installations().await;
                                                                        if let Ok(p) = plugin_api::get_privacy_dashboard().await {
                                                                            privacy.set(p);
                                                                        }
                                                                        status_msg.set(Some((false, "Đã thu hồi quyền ngay lập tức".into())));
                                                                    }
                                                                    Err(err) => status_msg.set(Some((true, format!("Không thể thu hồi quyền: {}", err.code)))),
                                                                }
                                                            });
                                                        },
                                                        "Thu hồi ngay"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if !privacy().recent_activity.is_empty() {
                        div { class: "settings-group-title", style: "margin-top: 14px;", "Nhật ký kiểm toán gần đây" }
                        div { class: "plugin-audit-list",
                            for row in privacy().recent_activity.into_iter().take(15) {
                                div { key: "{row.id}", class: "plugin-audit-row",
                                    span { class: "plugin-audit-action", "{row.action}" }
                                    if let Some(p) = row.permission {
                                        span { class: "plugin-perm-pill", "{p}" }
                                    }
                                    span { class: "plugin-hub-meta", "{row.created_at}" }
                                }
                            }
                        }
                    }
                }
            }

            // =================================================================
            // TAB 4: WEBHOOK MANAGER
            // =================================================================
            if tab() == HubTab::Webhooks {
                ScopedWebhookManager {
                    scope_type: "user".to_string(),
                    scope_id: my_user_id.clone(),
                    channel_id: None,
                    channels: Vec::new(),
                    can_manage: true,
                }
            }

            // =================================================================
            // TAB 5: DEVELOPER PORTAL (Publisher Registration + JS/WASM Publish)
            // =================================================================
            if tab() == HubTab::Developer {
                div { class: "plugin-dev-portal",
                    if let Some(pub_info) = my_pub() {
                        div { class: "plugin-installed-row",
                            div { class: "grow",
                                div { class: "plugin-hub-card-title-row",
                                    strong { "{pub_info.display_name}" }
                                    span { class: "plugin-runtime-chip", "@{pub_info.slug}" }
                                    if pub_info.verified {
                                        span { class: "plugin-verified-badge", CheckIcon { size: 12 } "Đã xác minh" }
                                    } else {
                                        span { class: "plugin-perm-pill", "Trạng thái: {pub_info.verification_status}" }
                                    }
                                }
                                div { class: "settings-row-desc", "Tất cả plugin xuất bản bắt buộc sử dụng định danh {pub_info.slug}.<tên_plugin>." }
                            }
                            if !pub_info.verified && pub_info.verification_status != "pending" {
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    onclick: move |_| {
                                        spawn(async move {
                                            match plugin_api::request_publisher_verification().await {
                                                Ok(_) => {
                                                    if let Ok(info) = plugin_api::get_my_publisher().await {
                                                        my_pub.set(info);
                                                    }
                                                    status_msg.set(Some((false, "Đã gửi yêu cầu xác minh nhà phát triển".into())));
                                                }
                                                Err(err) => status_msg.set(Some((true, format!("Không thể gửi yêu cầu: {}", err.code)))),
                                            }
                                        });
                                    },
                                    "Yêu cầu xác minh"
                                }
                            }
                        }

                        div { class: "plugin-dev-publish-box",
                            strong { "Xuất bản gói Plugin (JSON tạo từ CLI `chatw-plugin pack` — hỗ trợ JS/TS và Rust WASM)" }
                            textarea {
                                rows: "8",
                                class: "plugin-json-area",
                                placeholder: "Dán nội dung tệp .chatw-plugin.json (manifest, bundle_code, ui_html, wasm_base64, sha256)",
                                value: "{pkg_json_input}",
                                oninput: move |e| pkg_json_input.set(e.value()),
                            }
                            div { class: "inline-form",
                                button {
                                    r#type: "button",
                                    class: "btn-primary",
                                    "data-dev-publish-btn": "true",
                                    onclick: move |_| {
                                        let raw = pkg_json_input();
                                        let parsed: Result<Value, _> = serde_json::from_str(&raw);
                                        match parsed {
                                            Ok(payload) => {
                                                spawn(async move {
                                                    match plugin_api::publish_package(payload).await {
                                                        Ok(resp) => {
                                                            let key = resp.get("plugin_key").and_then(|v| v.as_str()).unwrap_or("");
                                                            pkg_json_input.set(String::new());
                                                            refresh_marketplace(query(), verified_only());
                                                            status_msg.set(Some((false, format!("Đã xuất bản gói plugin {key} thành công"))));
                                                        }
                                                        Err(err) => status_msg.set(Some((true, format!("Xuất bản thất bại: {}", err.code)))),
                                                    }
                                                });
                                            }
                                            Err(_) => status_msg.set(Some((true, "JSON gói plugin không hợp lệ".into()))),
                                        }
                                    },
                                    "Xuất bản lên Chợ Plugin"
                                }
                            }
                        }
                    } else {
                        div { class: "plugin-dev-publish-box",
                            strong { "Đăng ký hồ sơ Nhà phát triển (Namespace cố định)" }
                            div { class: "settings-row-desc", "Định danh nhà phát triển (slug) là bất biến để chống giả mạo và ghi đè Plugin ID." }
                            div { class: "form-grid-2",
                                label { class: "form-field",
                                    span { class: "form-label", "Namespace (slug)" }
                                    input {
                                        placeholder: "ví dụ: acme",
                                        value: "{pub_slug}",
                                        oninput: move |e| pub_slug.set(e.value()),
                                    }
                                }
                                label { class: "form-field",
                                    span { class: "form-label", "Tên hiển thị" }
                                    input {
                                        placeholder: "Tên tổ chức hoặc tác giả",
                                        value: "{pub_name}",
                                        oninput: move |e| pub_name.set(e.value()),
                                    }
                                }
                            }
                            label { class: "form-field",
                                span { class: "form-label", "Trang web (tùy chọn)" }
                                input {
                                    placeholder: "https://example.com",
                                    value: "{pub_web}",
                                    oninput: move |e| pub_web.set(e.value()),
                                }
                            }
                            label { class: "form-field",
                                span { class: "form-label", "Giới thiệu ngắn" }
                                input {
                                    placeholder: "Mô tả các plugin bạn phát triển",
                                    value: "{pub_bio}",
                                    oninput: move |e| pub_bio.set(e.value()),
                                }
                            }
                            button {
                                r#type: "button",
                                class: "btn-primary",
                                onclick: move |_| {
                                    let sl = pub_slug();
                                    let nm = pub_name();
                                    let wb = pub_web();
                                    let bi = pub_bio();
                                    spawn(async move {
                                        match plugin_api::register_publisher(&sl, &nm, &wb, &bi).await {
                                            Ok(info) => {
                                                my_pub.set(Some(info));
                                                status_msg.set(Some((false, "Đã đăng ký hồ sơ Nhà phát triển".into())));
                                            }
                                            Err(err) => status_msg.set(Some((true, format!("Không thể đăng ký: {}", err.code)))),
                                        }
                                    });
                                },
                                "Tạo hồ sơ Nhà phát triển"
                            }
                        }
                    }
                }
            }

            // =================================================================
            // MANDATORY PERMISSION CONSENT DIALOG (Never Auto-Install / Auto-Grant)
            // =================================================================
            if let Some(pkg) = consent_pkg() {
                {
                    let pkg_id = pkg.id.clone();
                    let pkg_name = pkg.name.clone();
                    let req_perms = pkg.requested_permissions.clone();
                    let preview_obj = consent_preview();
                    let has_e2ee = req_perms.iter().any(|p| p.starts_with("e2ee."));
                    let sens_approved = preview_obj
                        .as_ref()
                        .and_then(|v| v.get("sensitive_approval_status"))
                        .and_then(|v| v.as_str())
                        .map(|s| s == "approved" || s == "not_required")
                        .unwrap_or(pkg.sensitive_approval_status != "pending");
                    rsx! {
                        div { class: "modal-overlay", onclick: move |_| consent_pkg.set(None),
                            div {
                                class: "modal-dialog plugin-consent-modal",
                                role: "dialog",
                                aria_modal: "true",
                                "data-consent-modal": "{pkg.plugin_key}",
                                onclick: move |e| e.stop_propagation(),
                                div { class: "modal-header",
                                    div {
                                        h2 { class: "modal-title", "Xác nhận quyền cài đặt: {pkg_name}" }
                                        div { class: "settings-row-desc", "{pkg.plugin_key} · v{pkg.latest_version} · {pkg.publisher_name}" }
                                    }
                                    button { r#type: "button", class: "icon-btn", onclick: move |_| consent_pkg.set(None), CloseIcon { size: 18 } }
                                }
                                div { class: "modal-body",
                                    if !pkg.publisher_verified {
                                        div { class: "status-badge warning", "Nhà phát triển chưa được xác minh tích xanh. Chỉ cài đặt nếu bạn tin tưởng nguồn gói này." }
                                    }
                                    if has_e2ee {
                                        div { class: "status-badge error",
                                            ShieldIcon { size: 15 }
                                            " Plugin yêu cầu quyền dữ liệu nhạy cảm / E2EE. Khóa riêng tư E2EE KHÔNG BAO GIỜ được chia sẻ với plugin."
                                        }
                                    }
                                    if !sens_approved {
                                        div { class: "status-badge warning", "Các quyền nhạy cảm của phiên bản này chưa được quản trị viên duyệt nên sẽ không thể cấp." }
                                    }

                                    div { class: "settings-group-title", "Quyền plugin yêu cầu" }
                                    div { class: "plugin-consent-perm-list",
                                        for perm in req_perms {
                                            {
                                                let meta = catalog_map.iter().find(|c| c.id == perm);
                                                let label = meta.map(|m| m.label.clone()).unwrap_or_else(|| perm.clone());
                                                let desc = meta.map(|m| m.description.clone()).unwrap_or_default();
                                                let is_sens = meta.map(|m| m.sensitive || m.e2ee).unwrap_or(perm.starts_with("e2ee."));
                                                let is_e2ee = perm.starts_with("e2ee.");
                                                let checked = consent_selected_perms().contains(&perm);
                                                let perm_c = perm.clone();
                                                rsx! {
                                                    label { key: "{perm}", class: "plugin-consent-perm-item",
                                                        input {
                                                            r#type: "checkbox",
                                                            class: "switch",
                                                            checked: checked,
                                                            disabled: is_e2ee || (is_sens && !sens_approved),
                                                            onchange: move |e| {
                                                                let on = e.checked();
                                                                consent_selected_perms.with_mut(|list| {
                                                                    if on {
                                                                        if !list.contains(&perm_c) { list.push(perm_c.clone()); }
                                                                    } else {
                                                                        list.retain(|x| x != &perm_c);
                                                                    }
                                                                });
                                                            },
                                                        }
                                                        div { class: "grow",
                                                            div { class: "plugin-hub-card-title-row",
                                                                strong { "{label}" }
                                                                if is_sens {
                                                                    span { class: "plugin-perm-pill sensitive", "Nhạy cảm" }
                                                                }
                                                            }
                                                            if !desc.is_empty() {
                                                                div { class: "settings-row-desc", "{desc}" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    label { class: "plugin-consent-confirm-row",
                                        input {
                                            r#type: "checkbox",
                                            checked: consent_checked(),
                                            "data-consent-checkbox": "true",
                                            onchange: move |e| consent_checked.set(e.checked()),
                                        }
                                        span { "Tôi đã kiểm tra danh sách quyền và đồng ý cài đặt plugin này trong sandbox cách ly." }
                                    }
                                }
                                div { class: "modal-footer",
                                    button { r#type: "button", class: "btn-secondary", onclick: move |_| consent_pkg.set(None), "Hủy" }
                                    button {
                                        r#type: "button",
                                        class: "btn-primary",
                                        disabled: !consent_checked() || consent_busy(),
                                        "data-consent-submit": "true",
                                        onclick: move |_| {
                                            let pid = pkg_id.clone();
                                            let grants: Vec<Value> = consent_selected_perms()
                                                .into_iter()
                                                .map(|p| json!({ "permission": p, "scope_type": "global" }))
                                                .collect();
                                            consent_busy.set(true);
                                            spawn(async move {
                                                let res = plugin_api::create_installation(json!({
                                                    "package_id": pid,
                                                    "scope_type": "user",
                                                    "consent_confirmed": true,
                                                    "enable": true,
                                                    "grants": grants,
                                                })).await;
                                                consent_busy.set(false);
                                                match res {
                                                    Ok(v) => {
                                                        let new_id = v.get("id").and_then(|x| x.as_str()).map(String::from);
                                                        consent_pkg.set(None);
                                                        reload_user_installations().await;
                                                        if let Some(iid) = new_id {
                                                            *ACTIVE_SANDBOX_INSTALL_ID.write() = Some(iid);
                                                        }
                                                        tab.set(HubTab::Installed);
                                                        status_msg.set(Some((false, "Đã cài đặt và khởi chạy plugin trong Sandbox".into())));
                                                    }
                                                    Err(err) => {
                                                        status_msg.set(Some((true, format!("Cài đặt bị từ chối: {}", err.code))));
                                                    }
                                                }
                                            });
                                        },
                                        "Xác nhận cài đặt"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Reusable Scoped Webhook Manager (Discord-compatible Incoming & Outgoing Webhooks).
/// Works for User, Server, Channel, and Group scopes!
#[component]
pub fn ScopedWebhookManager(
    scope_type: String,
    scope_id: String,
    #[props(default)] channel_id: Option<String>,
    #[props(default)] channels: Vec<ChannelItem>,
    #[props(default = true)] can_manage: bool,
) -> Element {
    let mut webhooks = use_signal(Vec::<WebhookItem>::new);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);
    let mut direction = use_signal(|| "incoming".to_string());
    let mut name = use_signal(String::new);
    let mut target_url = use_signal(String::new);
    let mut selected_ch = use_signal(move || channel_id.clone().unwrap_or_default());
    let mut revealed_secret = use_signal(|| Option::<(String, String)>::None);
    let mut deliveries_for = use_signal(|| Option::<(String, Vec<WebhookDeliveryItem>)>::None);

    let st_eff = scope_type.clone();
    let sid_eff = scope_id.clone();
    use_effect(use_reactive!(|st_eff, sid_eff| {
        let st = st_eff.clone();
        let sid = sid_eff.clone();
        spawn(async move {
            let res = if st == "user" {
                plugin_api::list_webhooks(None, None).await
            } else {
                plugin_api::list_webhooks(Some(&st), Some(&sid)).await
            };
            if let Ok(list) = res {
                webhooks.set(list);
            }
        });
    }));

    let st_reload = scope_type.clone();
    let sid_reload = scope_id.clone();
    let reload = move || {
        let st = st_reload.clone();
        let sid = sid_reload.clone();
        spawn(async move {
            let res = if st == "user" {
                plugin_api::list_webhooks(None, None).await
            } else {
                plugin_api::list_webhooks(Some(&st), Some(&sid)).await
            };
            if let Ok(list) = res {
                webhooks.set(list);
            }
        });
    };

    let st_create = scope_type.clone();
    let sid_create = scope_id.clone();
    let r_create = reload.clone();

    rsx! {
        div { class: "webhook-mgr-root",
            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, role: "status", "{text}" }
            }

            if let Some((label, val)) = revealed_secret() {
                div { class: "webhook-secret-banner",
                    div { class: "grow",
                        strong { "{label} · Chỉ hiển thị một lần" }
                        code { class: "webhook-secret-code", "{val}" }
                    }
                    button {
                        r#type: "button",
                        class: "btn-secondary",
                        onclick: move |_| {
                            platform::copy_to_clipboard(&val);
                            msg.set(Some((false, "Đã sao chép vào khay nhớ tạm".into())));
                        },
                        CopyIcon { size: 15 }
                        "Sao chép"
                    }
                    button {
                        r#type: "button",
                        class: "icon-btn",
                        onclick: move |_| revealed_secret.set(None),
                        CloseIcon { size: 15 }
                    }
                }
            }

            if can_manage && scope_type != "user" {
                div { class: "plugin-dev-publish-box",
                    strong { "Tạo Webhook mới" }
                    div { class: "form-grid-2",
                        label { class: "form-field",
                            span { class: "form-label", "Loại Webhook" }
                            select {
                                value: "{direction}",
                                onchange: move |e| direction.set(e.value()),
                                option { value: "incoming", "Nhận tin từ hệ thống ngoài" }
                                option { value: "outgoing", "Gửi sự kiện ra ngoài" }
                            }
                        }
                        label { class: "form-field",
                            span { class: "form-label", "Tên hiển thị Webhook" }
                            input {
                                placeholder: "Ví dụ: GitHub, Cảnh báo",
                                value: "{name}",
                                oninput: move |e| name.set(e.value()),
                            }
                        }
                    }
                    if !channels.is_empty() {
                        label { class: "form-field",
                            span { class: "form-label", "Kênh đích" }
                            select {
                                value: "{selected_ch}",
                                onchange: move |e| selected_ch.set(e.value()),
                                for ch in channels.iter().filter(|c| c.channel_type == "text") {
                                    option { key: "{ch.id}", value: "{ch.id}", "#{ch.name}" }
                                }
                            }
                        }
                    }
                    if direction() == "outgoing" {
                        label { class: "form-field",
                            span { class: "form-label", "Đường dẫn nhận HTTPS" }
                            input {
                                placeholder: "https://api.example.com/chatw-hook",
                                value: "{target_url}",
                                oninput: move |e| target_url.set(e.value()),
                            }
                        }
                    }
                    button {
                        r#type: "button",
                        class: "btn-primary",
                        onclick: move |_| {
                            let dir = direction();
                            let nm = name().trim().to_string();
                            if nm.len() < 2 {
                                msg.set(Some((true, "Vui lòng nhập tên Webhook ít nhất 2 ký tự".into())));
                                return;
                            }
                            let eff_ch = if !selected_ch().is_empty() {
                                Some(selected_ch())
                            } else {
                                channels.iter().find(|c| c.channel_type == "text").map(|c| c.id.clone())
                            };
                            let (actual_scope_type, actual_scope_id) = if st_create == "server" {
                                if let Some(ref cid) = eff_ch {
                                    ("channel".to_string(), cid.clone())
                                } else {
                                    (st_create.clone(), sid_create.clone())
                                }
                            } else {
                                (st_create.clone(), sid_create.clone())
                            };
                            let mut payload = json!({
                                "direction": dir,
                                "scope_type": actual_scope_type,
                                "scope_id": actual_scope_id,
                                "channel_id": eff_ch,
                                "name": nm,
                                "rate_limit_per_min": 30,
                            });
                            if dir == "outgoing" {
                                payload["target_url"] = json!(target_url().trim());
                                payload["subscribed_events"] = json!(if st_create == "group" { vec!["onGroupMessage"] } else { vec!["onChannelMessage"] });
                            }
                            let reload_fn = r_create.clone();
                            spawn(async move {
                                match plugin_api::create_webhook(payload).await {
                                    Ok(resp) => {
                                        name.set(String::new());
                                        target_url.set(String::new());
                                        if let Some(url) = resp.get("execute_url").and_then(|v| v.as_str()) {
                                            revealed_secret.set(Some(("URL gửi Incoming Webhook".into(), url.to_string())));
                                        } else if let Some(sec) = resp.get("signing_secret").and_then(|v| v.as_str()) {
                                            revealed_secret.set(Some(("Khóa bí mật ký HMAC-SHA256".into(), sec.to_string())));
                                        }
                                        reload_fn();
                                        msg.set(Some((false, "Đã tạo Webhook thành công".into())));
                                    }
                                    Err(err) => msg.set(Some((true, format!("Không thể tạo Webhook: {}", err.code)))),
                                }
                            });
                        },
                        PlusIcon { size: 15 }
                        "Tạo Webhook"
                    }
                }
            } else if scope_type == "user" {
                div { class: "settings-row-desc", style: "margin-bottom: 10px;", "Để tạo Webhook gửi tin vào kênh hoặc nhóm, hãy mở Cài đặt Máy chủ hoặc Quản lý Nhóm -> mục Plugin & Webhook." }
            }

            if webhooks().is_empty() {
                div { class: "sidebar-empty", "Chưa có Webhook nào trong phạm vi này." }
            } else {
                div { class: "plugin-installed-list",
                    for wh in webhooks() {
                        {
                            let wid_test = wh.id.clone();
                            let wid_rot = wh.id.clone();
                            let wid_log = wh.id.clone();
                            let wid_del = wh.id.clone();
                            let wid_tg = wh.id.clone();
                            let wh_enabled = wh.enabled;
                            let r_del = reload.clone();
                            let r_tg = reload.clone();
                            let r_rot = reload.clone();
                            rsx! {
                                div { key: "{wh.id}", class: "plugin-installed-row",
                                    div { class: "grow",
                                        div { class: "plugin-hub-card-title-row",
                                            strong { "{wh.name}" }
                                            span { class: "plugin-runtime-chip", "{wh.direction}" }
                                            span { class: "plugin-perm-pill", "{wh.token_hint}" }
                                        }
                                        div { class: "plugin-hub-meta",
                                            if let Some(ref u) = wh.target_url {
                                                span { "{u}" }
                                            } else {
                                                span { "Phạm vi: {wh.scope_type} · Giới hạn {wh.rate_limit_per_min}/phút" }
                                            }
                                        }
                                    }
                                    if can_manage {
                                        div { class: "plugin-installed-actions",
                                            button {
                                                r#type: "button",
                                                class: "btn-secondary",
                                                onclick: move |_| {
                                                    let id = wid_test.clone();
                                                    spawn(async move {
                                                        match plugin_api::test_webhook(&id).await {
                                                            Ok(_) => msg.set(Some((false, "Đã gửi thử Webhook thành công".into()))),
                                                            Err(err) => msg.set(Some((true, format!("Gửi thử thất bại: {}", err.code)))),
                                                        }
                                                    });
                                                },
                                                "Gửi thử"
                                            }
                                            button {
                                                r#type: "button",
                                                class: "btn-ghost",
                                                onclick: move |_| {
                                                    let id = wid_rot.clone();
                                                    let rfn = r_rot.clone();
                                                    spawn(async move {
                                                        match plugin_api::rotate_webhook_token(&id).await {
                                                            Ok(v) => {
                                                                if let Some(u) = v.get("execute_url").and_then(|x| x.as_str()) {
                                                                    revealed_secret.set(Some(("URL mới sau khi đổi Token".into(), u.to_string())));
                                                                } else if let Some(s) = v.get("signing_secret").and_then(|x| x.as_str()) {
                                                                    revealed_secret.set(Some(("Khóa HMAC mới".into(), s.to_string())));
                                                                }
                                                                rfn();
                                                                msg.set(Some((false, "Đã xoay vòng Token Webhook — Token cũ đã bị vô hiệu hóa".into())));
                                                            }
                                                            Err(err) => msg.set(Some((true, format!("Không thể đổi Token: {}", err.code)))),
                                                        }
                                                    });
                                                },
                                                "Đổi Token"
                                            }
                                            button {
                                                r#type: "button",
                                                class: "btn-ghost",
                                                onclick: move |_| {
                                                    let id = wid_log.clone();
                                                    spawn(async move {
                                                        if let Ok(list) = plugin_api::list_webhook_deliveries(&id).await {
                                                            deliveries_for.set(Some((id, list)));
                                                        }
                                                    });
                                                },
                                                "Nhật ký"
                                            }
                                            input {
                                                r#type: "checkbox",
                                                class: "switch",
                                                checked: wh_enabled,
                                                onchange: move |e| {
                                                    let on = e.checked();
                                                    let id = wid_tg.clone();
                                                    let rfn = r_tg.clone();
                                                    spawn(async move {
                                                        let _ = plugin_api::update_webhook(&id, json!({ "enabled": on })).await;
                                                        rfn();
                                                    });
                                                },
                                            }
                                            button {
                                                r#type: "button",
                                                class: "icon-btn danger",
                                                onclick: move |_| {
                                                    let id = wid_del.clone();
                                                    let rfn = r_del.clone();
                                                    spawn(async move {
                                                        let _ = plugin_api::delete_webhook(&id).await;
                                                        rfn();
                                                        msg.set(Some((false, "Đã xóa Webhook".into())));
                                                    });
                                                },
                                                TrashIcon { size: 15 }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some((_, logs)) = deliveries_for() {
                div { class: "plugin-dev-publish-box", style: "margin-top: 12px;",
                    div { class: "plugin-hub-card-head",
                        strong { "Lịch sử gửi Webhook" }
                        button { r#type: "button", class: "icon-btn", onclick: move |_| deliveries_for.set(None), CloseIcon { size: 15 } }
                    }
                    if logs.is_empty() {
                        div { class: "sidebar-empty", "Chưa có lượt gửi nào." }
                    } else {
                        for d in logs.into_iter().take(15) {
                            div { key: "{d.id}", class: "plugin-audit-row",
                                span { class: "plugin-perm-pill", "{d.status}" }
                                span { "{d.event_type}" }
                                if let Some(code) = d.http_status { span { "HTTP {code}" } }
                                if let Some(ms) = d.latency_ms { span { "{ms}ms" } }
                                if let Some(err) = d.last_error { span { style: "color: var(--danger);", "{err}" } }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Combined Server / Group Admin Plugin & Webhook Tab (`ServerSettings` and `GroupManage`).
#[component]
pub fn ScopedPluginAndWebhookTab(
    scope_type: String,
    scope_id: String,
    #[props(default)] channels: Vec<ChannelItem>,
    can_manage: bool,
) -> Element {
    let mut scoped_installs = use_signal(Vec::<InstallationItem>::new);
    let mut market = use_signal(Vec::<MarketplaceItem>::new);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);

    let st_e = scope_type.clone();
    let sid_e = scope_id.clone();
    use_effect(use_reactive!(|st_e, sid_e| {
        let st = st_e.clone();
        let sid = sid_e.clone();
        spawn(async move {
            if let Ok(list) = plugin_api::list_installations(Some(&st), Some(&sid)).await {
                scoped_installs.set(list);
            }
            if let Ok(m) = plugin_api::list_marketplace("", "all", false).await {
                market.set(m);
            }
        });
    }));

    rsx! {
        div { class: "scoped-plugin-tab",
            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, role: "status", "{text}" }
            }

            div { class: "settings-group-title", "Plugin đã cài trong phạm vi này" }
            if scoped_installs().is_empty() {
                div { class: "sidebar-empty", "Chưa cài đặt plugin nào cho phạm vi này." }
            } else {
                div { class: "plugin-installed-list",
                    for inst in scoped_installs() {
                        {
                            let iid = inst.id.clone();
                            let st_r = scope_type.clone();
                            let sid_r = scope_id.clone();
                            rsx! {
                                div { key: "{inst.id}", class: "plugin-installed-row",
                                    div { class: "grow",
                                        strong { "{inst.name}" }
                                        div { class: "plugin-hub-meta", "{inst.plugin_key} · v{inst.version}" }
                                    }
                                    if can_manage {
                                        button {
                                            r#type: "button",
                                            class: "icon-btn danger",
                                            onclick: move |_| {
                                                let id = iid.clone();
                                                let st = st_r.clone();
                                                let sid = sid_r.clone();
                                                spawn(async move {
                                                    let _ = plugin_api::delete_installation(&id).await;
                                                    if let Ok(list) = plugin_api::list_installations(Some(&st), Some(&sid)).await {
                                                        scoped_installs.set(list);
                                                    }
                                                });
                                            },
                                            TrashIcon { size: 15 }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if can_manage && !market().is_empty() {
                div { class: "settings-group-title", style: "margin-top: 12px;", "Thêm nhanh Plugin từ Chợ" }
                div { class: "plugin-hub-grid",
                    for pkg in market().into_iter().take(6) {
                        {
                            let pid = pkg.id.clone();
                            let st_i = scope_type.clone();
                            let sid_i = scope_id.clone();
                            let safe_grants: Vec<Value> = pkg
                                .requested_permissions
                                .iter()
                                .filter(|p| !p.starts_with("e2ee."))
                                .map(|p| json!({ "permission": p, "scope_type": st_i.clone(), "scope_id": sid_i.clone() }))
                                .collect();
                            rsx! {
                                div { key: "{pkg.id}", class: "plugin-hub-card",
                                    div { class: "plugin-hub-card-title-row",
                                        strong { "{pkg.name}" }
                                        if pkg.publisher_verified {
                                            span { class: "plugin-verified-badge", CheckIcon { size: 12 } "Đã xác minh" }
                                        }
                                    }
                                    div { class: "plugin-hub-meta", "{pkg.plugin_key}" }
                                    button {
                                        r#type: "button",
                                        class: "btn-secondary",
                                        onclick: move |_| {
                                            let p = pid.clone();
                                            let st = st_i.clone();
                                            let sid = sid_i.clone();
                                            let g = safe_grants.clone();
                                            spawn(async move {
                                                let res = plugin_api::create_installation(json!({
                                                    "package_id": p,
                                                    "scope_type": st,
                                                    "scope_id": sid,
                                                    "consent_confirmed": true,
                                                    "enable": true,
                                                    "grants": g,
                                                })).await;
                                                match res {
                                                    Ok(_) => {
                                                        if let Ok(list) = plugin_api::list_installations(Some(&st), Some(&sid)).await {
                                                            scoped_installs.set(list);
                                                        }
                                                        msg.set(Some((false, "Đã cài đặt plugin vào phạm vi thành công".into())));
                                                    }
                                                    Err(err) => msg.set(Some((true, format!("Không thể cài plugin: {}", err.code)))),
                                                }
                                            });
                                        },
                                        "Cài vào phạm vi này"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "settings-group-title", style: "margin-top: 16px;", "Webhook" }
            ScopedWebhookManager {
                scope_type: scope_type,
                scope_id: scope_id,
                channel_id: None,
                channels: channels,
                can_manage: can_manage,
            }
        }
    }
}
