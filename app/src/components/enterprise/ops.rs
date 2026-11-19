use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::icons::{PlusIcon, ShareIcon, TrashIcon},
    platform,
    services::{
        api,
        enterprise::{
            self, AnnouncementItem, ApprovalsData, AutomationData, DocumentsData, EnterpriseConfig,
        },
        library,
    },
};

#[component]
pub fn ApprovalsPanel(server_id: String, my_id: String, cfg: EnterpriseConfig, refresh_tick: u64) -> Element {
    let mut data = use_signal(ApprovalsData::default);
    let mut show_create = use_signal(|| false);
    let mut show_tpl_create = use_signal(|| false);
    let mut new_tpl_name = use_signal(String::new);
    let mut new_tpl_cat = use_signal(|| "custom".to_string());
    let mut new_tpl_steps = use_signal(|| "Trưởng bộ phận, Ban điều hành".to_string());
    let mut tpl_id = use_signal(String::new);
    let mut title = use_signal(String::new);
    let mut category = use_signal(|| "purchase".to_string());
    let mut detail = use_signal(String::new);
    let mut comment = use_signal(String::new);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);

    let can_approve = cfg.has(enterprise::ent_perm::APPROVE_REQUESTS);
    let can_manage_tpl = cfg.has(enterprise::ent_perm::MANAGE_ENTERPRISE);

    let reload = {
        let sid = server_id.clone();
        move || {
            let sid = sid.clone();
            spawn(async move {
                if let Ok(v) = enterprise::list_approvals(&sid).await { data.set(v); }
            });
        }
    };

    {
        let sid = server_id.clone();
        use_effect(use_reactive!(|(sid, refresh_tick)| {
            let sid = sid.clone();
            let _ = refresh_tick;
            spawn(async move {
                if let Ok(v) = enterprise::list_approvals(&sid).await { data.set(v); }
            });
        }));
    }

    rsx! {
        div { class: "ent-section",
            div { class: "ent-toolbar",
                div { class: "ent-chips",
                    for t in data().templates {
                        {
                            let tid = t.id.clone();
                            let tname = t.name.clone();
                            let tcat = t.category.clone();
                            rsx! {
                                button {
                                    key: "{t.id}",
                                    r#type: "button",
                                    class: if tpl_id() == tid { "filter-chip active" } else { "filter-chip" },
                                    onclick: move |_| {
                                        tpl_id.set(tid.clone());
                                        category.set(tcat.clone());
                                        title.set(tname.clone());
                                        show_create.set(true);
                                    },
                                    "{t.name}"
                                }
                            }
                        }
                    }
                }
                div { class: "ent-btn-group",
                    if can_manage_tpl {
                        button {
                            r#type: "button",
                            class: "btn-secondary",
                            onclick: move |_| show_tpl_create.set(!show_tpl_create()),
                            PlusIcon { size: 15 } " Tạo mẫu biểu mẫu"
                        }
                    }
                    button {
                        r#type: "button",
                        class: "btn-primary",
                        onclick: move |_| show_create.set(!show_create()),
                        PlusIcon { size: 15 } " Gửi đề xuất"
                    }
                }
            }

            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{text}" }
            }

            if can_manage_tpl && show_tpl_create() {
                div { class: "ent-card",
                    div { class: "ent-form-grid",
                        label { class: "form-field", span { "Tên mẫu biểu mẫu" }
                            input { r#type: "text", placeholder: "Ví dụ: Đề nghị công tác nước ngoài", value: "{new_tpl_name}", oninput: move |e| new_tpl_name.set(e.value()) }
                        }
                        label { class: "form-field", span { "Phân loại" }
                            select { value: "{new_tpl_cat}", onchange: move |e| new_tpl_cat.set(e.value()),
                                option { value: "custom", "Tùy chỉnh" }
                                option { value: "purchase", "Mua sắm" }
                                option { value: "payment", "Thanh toán" }
                                option { value: "equipment", "Thiết bị" }
                                option { value: "travel", "Công tác" }
                                option { value: "access_request", "Cấp quyền" }
                            }
                        }
                        label { class: "form-field", span { "Các cấp phê duyệt (ngăn cách bằng dấu phẩy)" }
                            input { r#type: "text", placeholder: "Trưởng bộ phận, Kế toán, Ban điều hành", value: "{new_tpl_steps}", oninput: move |e| new_tpl_steps.set(e.value()) }
                        }
                    }
                    div { class: "ent-btn-group",
                        button {
                            r#type: "button",
                            class: "btn-primary",
                            onclick: {
                                let sid = server_id.clone();
                                let r = reload.clone();
                                move |_| {
                                    let n = new_tpl_name().trim().to_string();
                                    if n.is_empty() { return; }
                                    let steps: Vec<serde_json::Value> = new_tpl_steps()
                                        .split(',')
                                        .map(str::trim)
                                        .filter(|s| !s.is_empty())
                                        .map(|s| json!({ "label": s, "mode": "sequential" }))
                                        .collect();
                                    let (sid, r) = (sid.clone(), r.clone());
                                    spawn(async move {
                                        match enterprise::create_form_template(&sid, json!({
                                            "name": n,
                                            "category": new_tpl_cat(),
                                            "fields": [{ "key": "summary", "label": "Nội dung chi tiết", "required": true }],
                                            "workflow_steps": steps,
                                        })).await {
                                            Ok(_) => {
                                                new_tpl_name.set(String::new());
                                                show_tpl_create.set(false);
                                                msg.set(Some((false, "Đã tạo mẫu biểu mẫu nhiều cấp".into())));
                                                r();
                                            }
                                            Err(e) => msg.set(Some((true, format!("Không tạo được mẫu: {}", e.code)))),
                                        }
                                    });
                                }
                            },
                            "Lưu mẫu"
                        }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| show_tpl_create.set(false), "Đóng" }
                    }
                }
            }

            if show_create() {
                div { class: "ent-card",
                    div { class: "ent-form-grid",
                        label { class: "form-field", span { "Mẫu biểu mẫu" }
                            select { value: "{tpl_id}", onchange: move |e| {
                                let id = e.value();
                                tpl_id.set(id.clone());
                                if let Some(t) = data().templates.iter().find(|x| x.id == id) {
                                    category.set(t.category.clone());
                                    if title().is_empty() { title.set(t.name.clone()); }
                                }
                            },
                                option { value: "", "Biểu mẫu tùy chỉnh" }
                                for t in data().templates { option { key: "{t.id}", value: "{t.id}", "{t.name}" } }
                            }
                        }
                        label { class: "form-field", span { "Tiêu đề đề xuất" }
                            input { r#type: "text", placeholder: "Nhập tiêu đề đề xuất", value: "{title}", oninput: move |e| title.set(e.value()) }
                        }
                        label { class: "form-field", span { "Phân loại" }
                            select { value: "{category}", onchange: move |e| category.set(e.value()),
                                option { value: "purchase", "Mua sắm" }
                                option { value: "payment", "Thanh toán" }
                                option { value: "equipment", "Thiết bị" }
                                option { value: "it_support", "Hỗ trợ IT" }
                                option { value: "travel", "Công tác" }
                                option { value: "access_request", "Cấp quyền" }
                            }
                        }
                        label { class: "form-field", span { "Nội dung chi tiết" }
                            input { r#type: "text", placeholder: "Mô tả chi tiết lý do và hạng mục cần duyệt", value: "{detail}", oninput: move |e| detail.set(e.value()) }
                        }
                    }
                    div { class: "ent-btn-group",
                        button { r#type: "button", class: "btn-primary", onclick: {
                            let sid = server_id.clone();
                            let r = reload.clone();
                            move |_| {
                                let t = title().trim().to_string();
                                if t.is_empty() { return; }
                                let (sid, r) = (sid.clone(), r.clone());
                                let body = json!({
                                    "template_id": if tpl_id().is_empty() { serde_json::Value::Null } else { json!(tpl_id()) },
                                    "title": t,
                                    "category": category(),
                                    "form_data": { "summary": detail() },
                                });
                                spawn(async move {
                                    match enterprise::submit_approval(&sid, body).await {
                                        Ok(_) => {
                                            title.set(String::new());
                                            detail.set(String::new());
                                            show_create.set(false);
                                            msg.set(Some((false, "Đã gửi yêu cầu phê duyệt".into())));
                                            r();
                                        }
                                        Err(e) => msg.set(Some((true, format!("Gửi yêu cầu thất bại: {}", e.code)))),
                                    }
                                });
                            }
                        }, "Gửi phê duyệt" }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| show_create.set(false), "Đóng" }
                    }
                }
            }

            div { class: "ent-list",
                for req in data().requests {
                    {
                        let is_mine = req.requester_id == my_id;
                        let (s1, id1, r1) = (server_id.clone(), req.id.clone(), reload.clone());
                        let (s2, id2, r2) = (server_id.clone(), req.id.clone(), reload.clone());
                        let (s3, id3, r3) = (server_id.clone(), req.id.clone(), reload.clone());
                        let (s4, id4, r4) = (server_id.clone(), req.id.clone(), reload.clone());
                        rsx! {
                            div { key: "{req.id}", class: "ent-row-card",
                                div { class: "grow",
                                    div { class: "ent-row-title",
                                        strong { "{req.title}" }
                                        span { class: "ent-pill", "{enterprise::vn_category(&req.category)}" }
                                        span { class: "ent-pill", "Bước {req.current_step + 1}/{req.total_steps}" }
                                        span { class: "ent-pill ok", "{enterprise::vn_approval_status(&req.status)}" }
                                    }
                                    div { class: "ent-muted",
                                        "Người gửi: {req.requester_name}"
                                        if let Some(s) = req.form_data.get("summary").and_then(|v| v.as_str()) {
                                            if !s.is_empty() { " · {s}" }
                                        }
                                    }
                                }
                                if matches!(req.status.as_str(), "pending" | "needs_revision") {
                                    div { class: "ent-btn-group",
                                        if can_approve {
                                            input { r#type: "text", placeholder: "Ghi chú duyệt", style: "max-width: 140px;", value: "{comment}", oninput: move |e| comment.set(e.value()) }
                                            button { r#type: "button", class: "btn-primary", onclick: move |_| {
                                                let (s, id, r, c) = (s1.clone(), id1.clone(), r1.clone(), comment());
                                                spawn(async move { if enterprise::decide_approval(&s, &id, "approved", &c).await.is_ok() { r(); } });
                                            }, "Duyệt" }
                                            button { r#type: "button", class: "btn-secondary", onclick: move |_| {
                                                let (s, id, r, c) = (s4.clone(), id4.clone(), r4.clone(), comment());
                                                spawn(async move { if enterprise::decide_approval(&s, &id, "revision", &c).await.is_ok() { r(); } });
                                            }, "Yêu cầu sửa" }
                                            button { r#type: "button", class: "btn-secondary", onclick: move |_| {
                                                let (s, id, r, c) = (s2.clone(), id2.clone(), r2.clone(), comment());
                                                spawn(async move { if enterprise::decide_approval(&s, &id, "rejected", &c).await.is_ok() { r(); } });
                                            }, "Từ chối" }
                                        }
                                        if is_mine {
                                            button { r#type: "button", class: "btn-ghost", onclick: move |_| {
                                                let (s, id, r) = (s3.clone(), id3.clone(), r3.clone());
                                                spawn(async move { if enterprise::decide_approval(&s, &id, "withdrawn", "").await.is_ok() { r(); } });
                                            }, "Thu hồi" }
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
}

#[component]
pub fn DocumentsPanel(server_id: String, cfg: EnterpriseConfig, refresh_tick: u64) -> Element {
    let mut data = use_signal(DocumentsData::default);
    let mut scope = use_signal(String::new);
    let mut trashed = use_signal(|| false);
    let mut q = use_signal(String::new);
    let mut show_folder = use_signal(|| false);
    let mut folder_name = use_signal(String::new);
    let mut share_link = use_signal(|| Option::<String>::None);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);

    let can_manage = cfg.has(enterprise::ent_perm::MANAGE_DOCUMENTS);

    let reload = use_callback({
        let sid = server_id.clone();
        move |_: ()| {
            let sid = sid.clone();
            let (sc, tr, qv) = (scope(), trashed(), q());
            spawn(async move {
                if let Ok(d) = enterprise::list_documents(&sid, &sc, tr, &qv).await { data.set(d); }
            });
        }
    });

    {
        let sid = server_id.clone();
        use_effect(use_reactive!(|(sid, refresh_tick)| {
            let sid = sid.clone();
            let _ = refresh_tick;
            let (sc, tr, qv) = (scope(), trashed(), q());
            spawn(async move {
                if let Ok(d) = enterprise::list_documents(&sid, &sc, tr, &qv).await { data.set(d); }
            });
        }));
    }

    rsx! {
        div { class: "ent-section",
            div { class: "ent-toolbar",
                div { class: "ent-inline-bar",
                    select { value: "{scope}", onchange: move |e| { scope.set(e.value()); reload.call(()); },
                        option { value: "", "Mọi phạm vi" }
                        option { value: "company", "Công ty" }
                        option { value: "department", "Phòng ban" }
                        option { value: "project", "Dự án" }
                        option { value: "personal", "Cá nhân" }
                    }
                    button { r#type: "button", class: if trashed() { "filter-chip active" } else { "filter-chip" }, onclick: move |_| { trashed.set(!trashed()); reload.call(()); }, "Thùng rác" }
                    input { r#type: "text", placeholder: "Tìm tài liệu", value: "{q}", oninput: move |e| q.set(e.value()) }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| reload.call(()), "Lọc" }
                }
                div { class: "ent-btn-group",
                    if can_manage {
                        button {
                            r#type: "button",
                            class: "btn-secondary",
                            onclick: move |_| show_folder.set(!show_folder()),
                            PlusIcon { size: 15 } " Thư mục mới"
                        }
                    }
                    if can_manage || scope() == "personal" {
                        input { id: "ent-doc-upload", r#type: "file", style: "display: none;", onchange: {
                            let sid = server_id.clone();
                            move |_| {
                                let sid = sid.clone();
                                let sc = if scope().is_empty() { "company".to_string() } else { scope() };
                                spawn(async move {
                                    if let Ok((bytes, mime)) = platform::read_selected_file_raw("ent-doc-upload").await {
                                        let size = bytes.len() as i64;
                                        match library::upload_media(bytes, &mime).await {
                                            Ok(url) => {
                                                let body = json!({
                                                    "title": format!("Tài liệu {}", chrono::Utc::now().format("%d-%m-%Y %H:%M")),
                                                    "scope": sc,
                                                    "file_url": url,
                                                    "file_name": format!("document.{}", if mime.contains("pdf") { "pdf" } else { "bin" }),
                                                    "file_size": size,
                                                    "mime_type": mime,
                                                });
                                                match enterprise::create_document(&sid, body).await {
                                                    Ok(_) => { msg.set(Some((false, "Đã tải tài liệu lên kho".into()))); reload.call(()); }
                                                    Err(e) => msg.set(Some((true, format!("Lưu tài liệu thất bại: {}", e.code)))),
                                                }
                                            }
                                            Err(_) => msg.set(Some((true, "Không tải tệp lên máy chủ được".into()))),
                                        }
                                    }
                                });
                            }
                        } }
                        label { r#for: "ent-doc-upload", class: "btn-primary", role: "button", tabindex: "0", PlusIcon { size: 15 } " Tải tài liệu" }
                    }
                }
            }

            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{text}" }
            }
            if let Some(link) = share_link() {
                div { class: "ent-card ent-inline-bar",
                    span { "Liên kết chia sẻ có thời hạn: " }
                    code { "{link}" }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| platform::copy_to_clipboard(&link), "Sao chép" }
                }
            }

            if can_manage && show_folder() {
                div { class: "ent-card ent-inline-bar",
                    input { r#type: "text", placeholder: "Nhập tên thư mục mới", value: "{folder_name}", oninput: move |e| folder_name.set(e.value()) }
                    button { r#type: "button", class: "btn-primary", onclick: {
                        let sid = server_id.clone();
                        move |_| {
                            let n = folder_name().trim().to_string();
                            if n.is_empty() { return; }
                            let sc = if scope().is_empty() { "company".to_string() } else { scope() };
                            let sid = sid.clone();
                            spawn(async move {
                                if enterprise::create_folder(&sid, &n, &sc).await.is_ok() {
                                    folder_name.set(String::new());
                                    show_folder.set(false);
                                    reload.call(());
                                }
                            });
                        }
                    }, "Lưu thư mục" }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| show_folder.set(false), "Đóng" }
                }
            }

            if !data().folders.is_empty() {
                div { class: "ent-dept-grid",
                    for f in data().folders {
                        div { key: "{f.id}", class: "ent-card",
                            div { class: "ent-row-title",
                                strong { "{f.name}" }
                                span { class: "ent-pill ok", "{enterprise::vn_doc_scope(&f.scope)}" }
                            }
                        }
                    }
                }
            }

            div { class: "ent-list",
                for d in data().documents {
                    {
                        let (s_sh, d_sh) = (server_id.clone(), d.id.clone());
                        let (s_tr, d_tr) = (server_id.clone(), d.id.clone());
                        let (s_pin, d_pin) = (server_id.clone(), d.id.clone());
                        let (s_ver, d_ver) = (server_id.clone(), d.id.clone());
                        let act_tr = if trashed() { "restore" } else { "trash" };
                        let act_pin = if d.pinned { "unpin" } else { "pin" };
                        rsx! {
                            div { key: "{d.id}", class: "ent-row-card",
                                div { class: "grow",
                                    div { class: "ent-row-title",
                                        strong { "{d.title}" }
                                        span { class: "ent-pill", "{enterprise::vn_doc_scope(&d.scope)}" }
                                        span { class: "ent-pill", "v{d.version}" }
                                        if d.pinned { span { class: "ent-pill ok", "Đã ghim" } }
                                    }
                                    div { class: "ent-muted",
                                        "Người tải: {d.owner_name} · {d.file_name} · {d.file_size / 1024} KB"
                                    }
                                }
                                div { class: "ent-btn-group",
                                    a { class: "btn-secondary", href: "{platform::resolve_asset_url(&d.file_url)}", target: "_blank", "Mở tệp" }
                                    if can_manage && !trashed() {
                                        button { r#type: "button", class: "btn-secondary", onclick: move |_| {
                                            let (s, id) = (s_pin.clone(), d_pin.clone());
                                            spawn(async move {
                                                if enterprise::update_document(&s, &id, json!({ "action": act_pin })).await.is_ok() { reload.call(()); }
                                            });
                                        }, if d.pinned { "Bỏ ghim" } else { "Ghim" } }
                                        button { r#type: "button", class: "btn-secondary", onclick: move |_| {
                                            let (s, id) = (s_ver.clone(), d_ver.clone());
                                            spawn(async move {
                                                if enterprise::update_document(&s, &id, json!({ "action": "bump_version" })).await.is_ok() { reload.call(()); }
                                            });
                                        }, "+ Phiên bản" }
                                    }
                                    button { r#type: "button", class: "btn-secondary", onclick: move |_| {
                                        let (s, id) = (s_sh.clone(), d_sh.clone());
                                        spawn(async move {
                                            if let Ok(v) = enterprise::update_document(&s, &id, json!({ "action": "share", "expires_hours": 24 })).await {
                                                if let Some(tok) = v.get("token").and_then(|x| x.as_str()) {
                                                    share_link.set(Some(format!("/api/enterprise/shared/{tok}")));
                                                }
                                            }
                                        });
                                    }, ShareIcon { size: 14 } " Chia sẻ 24h" }
                                    button { r#type: "button", class: "btn-ghost", onclick: move |_| {
                                        let (s, id) = (s_tr.clone(), d_tr.clone());
                                        spawn(async move {
                                            if enterprise::update_document(&s, &id, json!({ "action": act_tr })).await.is_ok() { reload.call(()); }
                                        });
                                    }, TrashIcon { size: 14 } if trashed() { " Khôi phục" } else { " Xóa tạm" } }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AnnouncementsPanel(server_id: String, cfg: EnterpriseConfig, refresh_tick: u64) -> Element {
    let mut rows = use_signal(Vec::<AnnouncementItem>::new);
    let mut show_create = use_signal(|| false);
    let mut title = use_signal(String::new);
    let mut content = use_signal(String::new);
    let mut priority = use_signal(|| "normal".to_string());
    let mut pinned = use_signal(|| false);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);

    let can_post = cfg.has(enterprise::ent_perm::POST_ANNOUNCEMENTS);

    let reload = {
        let sid = server_id.clone();
        move || {
            let sid = sid.clone();
            spawn(async move {
                if let Ok(list) = enterprise::list_announcements(&sid).await { rows.set(list); }
            });
        }
    };

    {
        let sid = server_id.clone();
        use_effect(use_reactive!(|(sid, refresh_tick)| {
            let sid = sid.clone();
            let _ = refresh_tick;
            spawn(async move {
                if let Ok(list) = enterprise::list_announcements(&sid).await { rows.set(list); }
            });
        }));
    }

    rsx! {
        div { class: "ent-section",
            div { class: "ent-toolbar",
                div { class: "ent-chips",
                    span { class: "ent-pill ok", "{rows().len()} thông báo chính thức" }
                }
                if can_post {
                    button {
                        r#type: "button",
                        class: "btn-primary",
                        onclick: move |_| show_create.set(!show_create()),
                        PlusIcon { size: 15 } " Đăng thông báo"
                    }
                }
            }
            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{text}" }
            }
            if can_post && show_create() {
                div { class: "ent-card",
                    div { class: "ent-inline-bar",
                        input { r#type: "text", placeholder: "Tiêu đề thông báo", value: "{title}", oninput: move |e| title.set(e.value()) }
                        select { value: "{priority}", onchange: move |e| priority.set(e.value()),
                            option { value: "normal", "Bình thường" }
                            option { value: "important", "Quan trọng" }
                            option { value: "urgent", "Khẩn cấp" }
                        }
                        label { style: "display: flex; gap: 6px; align-items: center;",
                            input { r#type: "checkbox", checked: pinned(), onchange: move |e| pinned.set(e.checked()) }
                            "Ghim đầu bảng"
                        }
                    }
                    textarea { rows: 3, placeholder: "Nội dung thông báo", value: "{content}", oninput: move |e| content.set(e.value()) }
                    div { class: "ent-btn-group",
                        button { r#type: "button", class: "btn-primary", onclick: {
                            let sid = server_id.clone();
                            let r = reload.clone();
                            move |_| {
                                let (t, c) = (title().trim().to_string(), content().trim().to_string());
                                if t.is_empty() || c.is_empty() { return; }
                                let (sid, r) = (sid.clone(), r.clone());
                                spawn(async move {
                                    match enterprise::create_announcement(&sid, json!({ "title": t, "content": c, "priority": priority(), "pinned": pinned() })).await {
                                        Ok(_) => {
                                            title.set(String::new());
                                            content.set(String::new());
                                            show_create.set(false);
                                            msg.set(Some((false, "Đã phát hành thông báo nội bộ".into())));
                                            r();
                                        }
                                        Err(e) => msg.set(Some((true, format!("Lỗi đăng thông báo: {}", e.code)))),
                                    }
                                });
                            }
                        }, "Phát hành" }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| show_create.set(false), "Đóng" }
                    }
                }
            }
            div { class: "ent-list",
                for a in rows() {
                    {
                        let (sid, id, r) = (server_id.clone(), a.id.clone(), reload.clone());
                        rsx! {
                            div { key: "{a.id}", class: "ent-card",
                                div { class: "ent-row-card", style: "border: none; padding: 0; background: transparent;",
                                    div { class: "grow",
                                        div { class: "ent-row-title",
                                            strong { "{a.title}" }
                                            span { class: "ent-pill", "{enterprise::vn_ann_priority(&a.priority)}" }
                                            if a.pinned { span { class: "ent-pill ok", "Đã ghim" } }
                                        }
                                        p { style: "margin: 6px 0;", "{a.content}" }
                                        div { class: "ent-muted", "Đăng bởi {a.author_name} · {a.read_count} người đã xác nhận đọc" }
                                    }
                                    if !a.read_by_me {
                                        button { r#type: "button", class: "btn-secondary", onclick: move |_| {
                                            let (sid, id, r) = (sid.clone(), id.clone(), r.clone());
                                            spawn(async move { if enterprise::mark_announcement_read(&sid, &id).await.is_ok() { r(); } });
                                        }, "Xác nhận đã đọc" }
                                    } else {
                                        span { class: "ent-pill ok", "Đã đọc" }
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

#[component]
pub fn AiPanel(server_id: String, cfg: EnterpriseConfig) -> Element {
    let mut scope = use_signal(|| "overview".to_string());
    let mut summary = use_signal(|| Option::<String>::None);
    let mut note = use_signal(|| Option::<String>::None);
    let mut err = use_signal(|| Option::<String>::None);

    let ai_enabled = cfg.ai_config.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);

    rsx! {
        div { class: "ent-section",
            if !ai_enabled {
                div { class: "ent-card",
                    div { class: "ent-inline-bar",
                        crate::components::icons::VicoAiIcon { size: 22 }
                        p { class: "ent-muted", "Vico AI chỉ hoạt động trong không gian này khi quản trị viên bật trong Cài đặt máy chủ và không bao giờ đọc tin nhắn cá nhân mã hóa đầu cuối." }
                    }
                }
            } else {
                div { class: "ent-card",
                    div { class: "ent-inline-bar",
                        crate::components::icons::VicoAiIcon { size: 22 }
                        select { value: "{scope}", onchange: move |e| scope.set(e.value()),
                            option { value: "overview", "Tổng quan toàn công ty" }
                            option { value: "tasks", "Tiến độ công việc" }
                        }
                        button { r#type: "button", class: "btn-primary", onclick: {
                            let sid = server_id.clone();
                            move |_| {
                                let (sid, sc) = (sid.clone(), scope());
                                spawn(async move {
                                    match enterprise::ai_summarize(&sid, &sc, None).await {
                                        Ok(v) => {
                                            summary.set(v.get("summary").and_then(|x| x.as_str()).map(str::to_string));
                                            note.set(v.get("e2ee_note").and_then(|x| x.as_str()).map(str::to_string));
                                            err.set(None);
                                        }
                                        Err(e) => err.set(Some(format!("Vico AI không chạy tổng hợp được: {}", e.code))),
                                    }
                                });
                            }
                        }, "Tóm tắt bằng Vico AI" }
                    }
                    if let Some(e) = err() { div { class: "status-badge error", "{e}" } }
                    if let Some(s) = summary() {
                        div { class: "ent-quote", style: "margin-top: 10px;", strong { "{s}" } }
                    }
                    if let Some(n) = note() {
                        div { class: "ent-muted", style: "margin-top: 6px;", "{n}" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AutomationPanel(server_id: String, cfg: EnterpriseConfig, refresh_tick: u64) -> Element {
    let mut data = use_signal(AutomationData::default);
    let mut show_integ = use_signal(|| false);
    let mut show_wf = use_signal(|| false);
    let mut integ_name = use_signal(String::new);
    let mut integ_provider = use_signal(|| "webhook".to_string());
    let mut integ_url = use_signal(String::new);
    let mut wf_name = use_signal(String::new);
    let mut wf_trigger = use_signal(|| "task_assigned".to_string());
    let mut wf_channel = use_signal(String::new);
    let mut channels = use_signal(Vec::<crate::models::ChannelItem>::new);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);

    let can_integ = cfg.has(enterprise::ent_perm::MANAGE_INTEGRATIONS);
    let can_wf = cfg.has(enterprise::ent_perm::MANAGE_AUTOMATION);

    let reload = {
        let sid = server_id.clone();
        move || {
            let sid = sid.clone();
            spawn(async move {
                if let Ok(a) = enterprise::list_automation(&sid).await { data.set(a); }
                if let Ok(c) = api::servers_get_channels(&sid).await { channels.set(c); }
            });
        }
    };

    {
        let sid = server_id.clone();
        use_effect(use_reactive!(|(sid, refresh_tick)| {
            let sid = sid.clone();
            let _ = refresh_tick;
            spawn(async move {
                if let Ok(a) = enterprise::list_automation(&sid).await { data.set(a); }
                if let Ok(c) = api::servers_get_channels(&sid).await { channels.set(c); }
            });
        }));
    }

    rsx! {
        div { class: "ent-section",
            div { class: "ent-toolbar",
                div { class: "ent-chips",
                    span { class: "ent-pill ok", "{data().workflows.len()} quy trình tự động" }
                    span { class: "ent-pill", "{data().integrations.len()} kết nối tích hợp" }
                }
                div { class: "ent-btn-group",
                    if can_integ {
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| show_integ.set(!show_integ()), PlusIcon { size: 15 } " Thêm kết nối" }
                    }
                    if can_wf {
                        button { r#type: "button", class: "btn-primary", onclick: move |_| show_wf.set(!show_wf()), PlusIcon { size: 15 } " Thêm quy trình" }
                    }
                }
            }
            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{text}" }
            }

            if can_integ && show_integ() {
                div { class: "ent-card",
                    div { class: "ent-inline-bar",
                        select { value: "{integ_provider}", onchange: move |e| integ_provider.set(e.value()),
                            option { value: "webhook", "Webhook" }
                            option { value: "github", "GitHub" }
                            option { value: "jira", "Jira" }
                            option { value: "google_calendar", "Google Calendar" }
                            option { value: "rest_api", "REST API" }
                        }
                        input { r#type: "text", placeholder: "Tên kết nối", value: "{integ_name}", oninput: move |e| integ_name.set(e.value()) }
                        input { r#type: "text", placeholder: "https://endpoint", value: "{integ_url}", oninput: move |e| integ_url.set(e.value()) }
                        button { r#type: "button", class: "btn-primary", onclick: {
                            let sid = server_id.clone();
                            let r = reload.clone();
                            move |_| {
                                let n = integ_name().trim().to_string();
                                if n.is_empty() { return; }
                                let (sid, r) = (sid.clone(), r.clone());
                                spawn(async move {
                                    match enterprise::create_integration(&sid, json!({
                                        "provider": integ_provider(),
                                        "name": n,
                                        "endpoint_url": integ_url(),
                                    })).await {
                                        Ok(_) => { integ_name.set(String::new()); integ_url.set(String::new()); show_integ.set(false); r(); }
                                        Err(e) => msg.set(Some((true, format!("Không lưu được tích hợp: {}", e.code)))),
                                    }
                                });
                            }
                        }, "Lưu kết nối" }
                    }
                }
            }

            if can_wf && show_wf() {
                div { class: "ent-card",
                    div { class: "ent-inline-bar",
                        input { r#type: "text", placeholder: "Tên quy trình", value: "{wf_name}", oninput: move |e| wf_name.set(e.value()) }
                        select { value: "{wf_trigger}", onchange: move |e| wf_trigger.set(e.value()),
                            option { value: "task_assigned", "Khi giao công việc" }
                            option { value: "task_completed", "Khi hoàn thành việc" }
                            option { value: "leave_decided", "Khi duyệt nghỉ phép" }
                            option { value: "approval_decided", "Khi duyệt đề xuất" }
                            option { value: "announcement_posted", "Khi có thông báo mới" }
                        }
                        select { value: "{wf_channel}", onchange: move |e| wf_channel.set(e.value()),
                            option { value: "", "Chọn kênh gửi thông báo" }
                            for ch in channels() {
                                if ch.channel_type == "text" {
                                    option { key: "{ch.id}", value: "{ch.id}", "#{ch.name}" }
                                }
                            }
                        }
                        button { r#type: "button", class: "btn-primary", onclick: {
                            let sid = server_id.clone();
                            let r = reload.clone();
                            move |_| {
                                let (n, cid) = (wf_name().trim().to_string(), wf_channel());
                                if n.is_empty() || cid.is_empty() { return; }
                                let (sid, r) = (sid.clone(), r.clone());
                                spawn(async move {
                                    match enterprise::create_workflow(&sid, json!({
                                        "name": n,
                                        "trigger_event": wf_trigger(),
                                        "action_type": "post_channel_message",
                                        "action_config": { "channel_id": cid },
                                    })).await {
                                        Ok(_) => { wf_name.set(String::new()); show_wf.set(false); r(); }
                                        Err(e) => msg.set(Some((true, format!("Không tạo được quy trình: {}", e.code)))),
                                    }
                                });
                            }
                        }, "Lưu quy trình" }
                    }
                }
            }

            if !data().integrations.is_empty() {
                div { class: "ent-list",
                    for i in data().integrations {
                        {
                            let (sid, id, r) = (server_id.clone(), i.id.clone(), reload.clone());
                            rsx! {
                                div { key: "{i.id}", class: "ent-row-card",
                                    div { class: "grow",
                                        div { class: "ent-row-title",
                                            strong { "{i.name}" }
                                            span { class: "ent-pill", "{i.provider}" }
                                            span { class: "ent-pill ok", "{i.status}" }
                                        }
                                    }
                                    button { r#type: "button", class: "btn-secondary", onclick: move |_| {
                                        let (sid, id, r) = (sid.clone(), id.clone(), r.clone());
                                        spawn(async move {
                                            match enterprise::test_integration(&sid, &id).await {
                                                Ok(_) => { msg.set(Some((false, "Kiểm tra kết nối thành công".into()))); r(); }
                                                Err(e) => msg.set(Some((true, format!("Kiểm tra thất bại: {}", e.code)))),
                                            }
                                        });
                                    }, "Kiểm tra kết nối" }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "ent-list",
                for w in data().workflows {
                    {
                        let (sid, id, r) = (server_id.clone(), w.id.clone(), reload.clone());
                        rsx! {
                            div { key: "{w.id}", class: "ent-row-card",
                                div { class: "grow",
                                    div { class: "ent-row-title",
                                        strong { "{w.name}" }
                                        span { class: "ent-pill", "{w.trigger_event}" }
                                        span { class: "ent-muted", "Đã chạy {w.run_count} lần" }
                                    }
                                }
                                button { r#type: "button", class: "btn-secondary", onclick: move |_| {
                                    let (sid, id, r) = (sid.clone(), id.clone(), r.clone());
                                    spawn(async move { if enterprise::toggle_workflow(&sid, &id).await.is_ok() { r(); } });
                                }, if w.enabled { "Đang bật" } else { "Đang tắt" } }
                            }
                        }
                    }
                }
            }
        }
    }
}
