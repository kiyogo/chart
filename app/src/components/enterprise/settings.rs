use dioxus::prelude::*;
use serde_json::{Value, json};

use crate::{
    components::icons::{
        BriefcaseIcon, BuildingIcon, CalendarIcon, ChevronDownIcon, ChevronRightIcon, DownloadIcon,
        FileTextIcon, ListChecksIcon, LockIcon, ShieldIcon, SlidersIcon, SparkleIcon,
    },
    services::enterprise::{
        self, EnterpriseConfig, MODULE_DEFS, ent_perm, export_all_json_url, export_audit_csv_url,
        export_reports_csv_url,
    },
};

fn str_val(v: &Value, key: &str, default: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or(default).to_string()
}

fn i64_val(v: &Value, key: &str, default: i64) -> i64 {
    v.get(key).and_then(|x| x.as_i64()).unwrap_or(default)
}

fn bool_val(v: &Value, key: &str, default: bool) -> bool {
    v.get(key).and_then(|x| x.as_bool()).unwrap_or(default)
}

fn with_key(base: &Value, key: &str, val: Value) -> Value {
    let mut obj = base.as_object().cloned().unwrap_or_default();
    obj.insert(key.to_string(), val);
    Value::Object(obj)
}

#[component]
pub fn EnterpriseSettingsPanel(server_id: String, on_changed: EventHandler<()>) -> Element {
    let mut cfg = use_signal(|| Option::<EnterpriseConfig>::None);
    let mut cat = use_signal(|| "general"); // "general" | "hr" | "work" | "time" | "docs" | "security" | "advanced"
    let mut search = use_signal(String::new);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);

    // Editable inputs synced when config loads
    let mut company_name = use_signal(String::new);
    let mut contact_email = use_signal(String::new);
    let mut website = use_signal(String::new);
    let mut timezone = use_signal(|| "Asia/Ho_Chi_Minh".to_string());
    let mut emp_prefix = use_signal(|| "NV-".to_string());
    let mut work_start = use_signal(|| "08:30".to_string());
    let mut work_end = use_signal(|| "17:30".to_string());
    let mut grace_mins = use_signal(|| "15".to_string());
    let mut annual_days = use_signal(|| "12".to_string());
    let mut sick_days = use_signal(|| "5".to_string());
    let mut quota_mb = use_signal(|| "2048".to_string());

    let mut sync_inputs = move |c: &EnterpriseConfig| {
        company_name.set(str_val(&c.general_settings, "company_name", ""));
        contact_email.set(str_val(&c.general_settings, "contact_email", ""));
        website.set(str_val(&c.general_settings, "website", ""));
        timezone.set(str_val(&c.general_settings, "timezone", "Asia/Ho_Chi_Minh"));
        emp_prefix.set(str_val(&c.hr_policy, "code_prefix", "NV-"));
        work_start.set(str_val(&c.attendance_policy, "work_start", "08:30"));
        work_end.set(str_val(&c.attendance_policy, "work_end", "17:30"));
        grace_mins.set(i64_val(&c.attendance_policy, "grace_minutes", 15).to_string());
        annual_days.set(i64_val(&c.leave_policy, "annual_days", 12).to_string());
        sick_days.set(i64_val(&c.leave_policy, "sick_days", 5).to_string());
        quota_mb.set(i64_val(&c.document_policy, "quota_mb", 2048).to_string());
    };

    {
        let sid = server_id.clone();
        use_hook(move || {
            let sid = sid.clone();
            spawn(async move {
                if let Ok(c) = enterprise::get_config(&sid).await {
                    sync_inputs(&c);
                    cfg.set(Some(c));
                }
            });
        });
    }

    let save_patch = use_callback({
        let sid = server_id.clone();
        move |(patch, notice): (Value, &'static str)| {
            let sid = sid.clone();
            spawn(async move {
                match enterprise::update_config(&sid, patch).await {
                    Ok(_) => {
                        msg.set(Some((false, notice.into())));
                        if let Ok(c) = enterprise::get_config(&sid).await {
                            cfg.set(Some(c));
                        }
                        on_changed.call(());
                    }
                    Err(e) => msg.set(Some((true, format!("Không lưu được: {}", e.code)))),
                }
            });
        }
    });

    let q = search().trim().to_lowercase();
    let filtering = !q.is_empty();
    let show_general = (!filtering && cat() == "general")
        || (filtering && "chung giao diện phân hệ module tên múi giờ mật độ sắp xếp".contains(&q));
    let show_hr = (!filtering && cat() == "hr")
        || (filtering && "thành viên nhân sự mã nhân viên khách mời đối tác nghỉ việc".contains(&q));
    let show_work = (!filtering && cat() == "work")
        || (filtering && "công việc dự án kênh tin nhắn deadline nhắc hạn duyệt việc".contains(&q));
    let show_time = (!filtering && cat() == "time")
        || (filtering && "chấm công nghỉ phép ca làm giờ vào tan ca phép năm nghỉ ốm".contains(&q));
    let show_docs = (!filtering && cat() == "docs")
        || (filtering && "phê duyệt biểu mẫu tài liệu dung lượng quota thông báo chia sẻ".contains(&q));
    let show_sec = (!filtering && cat() == "security")
        || (filtering && "bảo mật riêng tư mfa 2fa phiên danh bạ ai trợ lý tóm tắt".contains(&q));
    let show_adv = (!filtering && cat() == "advanced")
        || (filtering && "kiểm toán nhật ký nâng cao lưu trữ retention xuất dữ liệu csv json".contains(&q));

    rsx! {
        div { class: "ent-section",
            div { class: "ent-toolbar",
                span { class: "settings-label", SlidersIcon { size: 16 } " Cài đặt doanh nghiệp chuyên sâu" }
                input {
                    r#type: "text",
                    placeholder: "Tìm thiết lập (ví dụ: ca làm, phép, quota, MFA, AI)",
                    value: "{search}",
                    oninput: move |e| search.set(e.value()),
                }
            }

            if !filtering {
                div { class: "sidebar-filters",
                    for (k, label) in [
                        ("general", "Chung & Giao diện"),
                        ("hr", "Thành viên & Nhân sự"),
                        ("work", "Công việc, Dự án & Kênh"),
                        ("time", "Chấm công & Nghỉ phép"),
                        ("docs", "Phê duyệt, Tài liệu & Thông báo"),
                        ("security", "Bảo mật, Riêng tư & AI"),
                        ("advanced", "Kiểm toán & Nâng cao"),
                    ] {
                        button {
                            key: "{k}",
                            r#type: "button",
                            class: if cat() == k { "filter-chip active" } else { "filter-chip" },
                            onclick: move |_| cat.set(k),
                            "{label}"
                        }
                    }
                }
            }

            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{text}" }
            }

            if let Some(c) = cfg() {
                {
                    let can_ent = c.has(ent_perm::MANAGE_ENTERPRISE);
                    let can_sec = c.is_owner || c.has(ent_perm::MANAGE_SECURITY);
                    let can_ai = c.has(ent_perm::MANAGE_AI);
                    let ordered_mods: Vec<String> = if c.module_order.is_empty() {
                        MODULE_DEFS.iter().map(|(k, _)| k.to_string()).collect()
                    } else {
                        c.module_order.clone()
                    };

                    rsx! {
                        if show_general {
                            div { class: "ent-card",
                                h4 { BuildingIcon { size: 16 } " Thông tin chung & Giao diện doanh nghiệp" }
                                div { class: "ent-form-grid",
                                    label { class: "form-field", span { "Tên pháp nhân / Doanh nghiệp" }
                                        input { r#type: "text", disabled: !can_ent, value: "{company_name}", oninput: move |e| company_name.set(e.value()) }
                                    }
                                    label { class: "form-field", span { "Email liên hệ" }
                                        input { r#type: "text", disabled: !can_ent, value: "{contact_email}", oninput: move |e| contact_email.set(e.value()) }
                                    }
                                    label { class: "form-field", span { "Website" }
                                        input { r#type: "text", disabled: !can_ent, value: "{website}", oninput: move |e| website.set(e.value()) }
                                    }
                                    label { class: "form-field", span { "Múi giờ chuẩn" }
                                        select { disabled: !can_ent, value: "{timezone}", onchange: move |e| timezone.set(e.value()),
                                            option { value: "Asia/Ho_Chi_Minh", "Asia/Ho_Chi_Minh (GMT+7)" }
                                            option { value: "Asia/Tokyo", "Asia/Tokyo (GMT+9)" }
                                            option { value: "UTC", "UTC (GMT+0)" }
                                        }
                                    }
                                }
                                if can_ent {
                                    div { class: "ent-btn-group",
                                        button {
                                            r#type: "button",
                                            class: "btn-primary",
                                            onclick: {
                                                let base = c.general_settings.clone();
                                                move |_| {
                                                    let mut obj = base.as_object().cloned().unwrap_or_default();
                                                    obj.insert("company_name".into(), json!(company_name()));
                                                    obj.insert("contact_email".into(), json!(contact_email()));
                                                    obj.insert("website".into(), json!(website()));
                                                    obj.insert("timezone".into(), json!(timezone()));
                                                    save_patch.call((json!({ "general_settings": Value::Object(obj) }), "Đã lưu cài đặt chung"));
                                                }
                                            },
                                            "Lưu thông tin chung"
                                        }
                                    }
                                }

                                label { class: "settings-row toggle-row",
                                    span { class: "grow",
                                        span { class: "settings-label", "Chế độ hiển thị gọn (Compact Density)" }
                                        span { class: "settings-row-desc", "Tối ưu mật độ hiển thị danh sách nhân sự, bảng công việc và tài liệu" }
                                    }
                                    input {
                                        r#type: "checkbox",
                                        class: "switch",
                                        checked: bool_val(&c.display_settings, "compact", false),
                                        disabled: !can_ent,
                                        onchange: {
                                            let base = c.display_settings.clone();
                                            move |e| {
                                                save_patch.call((json!({ "display_settings": with_key(&base, "compact", json!(e.checked())) }), "Đã lưu giao diện"));
                                            }
                                        },
                                    }
                                }

                                div { class: "settings-group-title", "Phân hệ hiển thị & Thứ tự thanh bên" }
                                for (idx, key) in ordered_mods.iter().enumerate() {
                                    {
                                        let label = enterprise::module_label(key);
                                        let enabled = c.enabled_modules.iter().any(|m| m == key);
                                        let cur_enabled = c.enabled_modules.clone();
                                        let cur_order = ordered_mods.clone();
                                        let cur_order2 = ordered_mods.clone();
                                        let k_str = key.clone();
                                        rsx! {
                                            div { key: "mod-{key}", class: "settings-row toggle-row",
                                                span { class: "grow", span { class: "settings-label", "{label}" } }
                                                div { class: "ent-btn-group",
                                                    if can_ent && idx > 0 {
                                                        button { r#type: "button", class: "btn-ghost", title: "Đưa lên", onclick: move |_| {
                                                            let mut next = cur_order.clone();
                                                            next.swap(idx, idx - 1);
                                                            save_patch.call((json!({ "module_order": next }), "Đã đổi thứ tự phân hệ"));
                                                        }, ChevronRightIcon { size: 13 } }
                                                    }
                                                    if can_ent && idx + 1 < ordered_mods.len() {
                                                        button { r#type: "button", class: "btn-ghost", title: "Đưa xuống", onclick: move |_| {
                                                            let mut next = cur_order2.clone();
                                                            next.swap(idx, idx + 1);
                                                            save_patch.call((json!({ "module_order": next }), "Đã đổi thứ tự phân hệ"));
                                                        }, ChevronDownIcon { size: 13 } }
                                                    }
                                                    input {
                                                        r#type: "checkbox",
                                                        class: "switch",
                                                        checked: enabled,
                                                        disabled: !can_ent || k_str == "overview",
                                                        onchange: move |e| {
                                                            let mut next = cur_enabled.clone();
                                                            if e.checked() {
                                                                if !next.iter().any(|x| x == &k_str) { next.push(k_str.clone()); }
                                                            } else {
                                                                next.retain(|x| x != &k_str);
                                                            }
                                                            save_patch.call((json!({ "enabled_modules": next }), "Đã cập nhật phân hệ"));
                                                        },
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if show_hr {
                            div { class: "ent-card",
                                h4 { BriefcaseIcon { size: 16 } " Chính sách Thành viên & Nhân sự" }
                                div { class: "ent-inline-bar",
                                    label { class: "form-field", span { "Tiền tố mã nhân viên" }
                                        input { r#type: "text", disabled: !can_ent, value: "{emp_prefix}", oninput: move |e| emp_prefix.set(e.value()) }
                                    }
                                    if can_ent {
                                        button { r#type: "button", class: "btn-primary", onclick: {
                                            let base = c.hr_policy.clone();
                                            move |_| {
                                                save_patch.call((json!({ "hr_policy": with_key(&base, "code_prefix", json!(emp_prefix())) }), "Đã lưu quy tắc mã nhân viên"));
                                            }
                                        }, "Lưu tiền tố" }
                                    }
                                }
                                label { class: "settings-row toggle-row",
                                    span { class: "grow",
                                        span { class: "settings-label", "Bắt buộc phê duyệt khi thêm thành viên ngoài" }
                                        span { class: "settings-row-desc", "Khách mời và đối tác cần quản trị viên duyệt trước khi truy cập" }
                                    }
                                    input { r#type: "checkbox", class: "switch", checked: bool_val(&c.hr_policy, "approve_external_guests", true), disabled: !can_ent,
                                        onchange: {
                                            let base = c.hr_policy.clone();
                                            move |e| save_patch.call((json!({ "hr_policy": with_key(&base, "approve_external_guests", json!(e.checked())) }), "Đã lưu chính sách thành viên"))
                                        }
                                    }
                                }
                                label { class: "settings-row toggle-row",
                                    span { class: "grow",
                                        span { class: "settings-label", "Tự động thu hồi quyền công việc khi đánh dấu nghỉ việc" }
                                        span { class: "settings-row-desc", "Ngăn truy cập dữ liệu doanh nghiệp ngay khi trạng thái nhân sự chuyển sang đã nghỉ việc" }
                                    }
                                    input { r#type: "checkbox", class: "switch", checked: bool_val(&c.hr_policy, "auto_revoke_offboarded", true), disabled: !can_ent,
                                        onchange: {
                                            let base = c.hr_policy.clone();
                                            move |e| save_patch.call((json!({ "hr_policy": with_key(&base, "auto_revoke_offboarded", json!(e.checked())) }), "Đã lưu chính sách nghỉ việc"))
                                        }
                                    }
                                }
                            }
                        }

                        if show_work {
                            div { class: "ent-card",
                                h4 { ListChecksIcon { size: 16 } " Chính sách Công việc, Dự án & Kênh" }
                                label { class: "settings-row toggle-row",
                                    span { class: "grow",
                                        span { class: "settings-label", "Yêu cầu bước Chờ duyệt trước khi đóng công việc" }
                                        span { class: "settings-row-desc", "Khuyến nghị kiểm tra chất lượng trước khi chuyển trạng thái hoàn thành" }
                                    }
                                    input { r#type: "checkbox", class: "switch", checked: bool_val(&c.task_policy, "require_review", false), disabled: !can_ent,
                                        onchange: {
                                            let base = c.task_policy.clone();
                                            move |e| save_patch.call((json!({ "task_policy": with_key(&base, "require_review", json!(e.checked())) }), "Đã lưu cài đặt công việc"))
                                        }
                                    }
                                }
                                label { class: "settings-row toggle-row",
                                    span { class: "grow",
                                        span { class: "settings-label", "Tự động nhắc hạn công việc sắp tới" }
                                        span { class: "settings-row-desc", "Gửi thông báo nhắc người phụ trách trước thời hạn 24 giờ" }
                                    }
                                    input { r#type: "checkbox", class: "switch", checked: bool_val(&c.task_policy, "auto_deadline_reminder", true), disabled: !can_ent,
                                        onchange: {
                                            let base = c.task_policy.clone();
                                            move |e| save_patch.call((json!({ "task_policy": with_key(&base, "auto_deadline_reminder", json!(e.checked())) }), "Đã lưu cài đặt nhắc hạn"))
                                        }
                                    }
                                }
                                label { class: "settings-row toggle-row",
                                    span { class: "grow",
                                        span { class: "settings-label", "Bảo vệ trích dẫn tin nhắn từ kênh riêng tư" }
                                        span { class: "settings-row-desc", "Ẩn nội dung tin nhắn gốc khi tạo công việc từ kênh riêng tư" }
                                    }
                                    input { r#type: "checkbox", class: "switch", checked: bool_val(&c.project_policy, "redact_private_channel_source", true), disabled: !can_ent,
                                        onchange: {
                                            let base = c.project_policy.clone();
                                            move |e| save_patch.call((json!({ "project_policy": with_key(&base, "redact_private_channel_source", json!(e.checked())) }), "Đã lưu chính sách kênh & dự án"))
                                        }
                                    }
                                }
                            }
                        }

                        if show_time {
                            div { class: "ent-card",
                                h4 { CalendarIcon { size: 16 } " Chính sách Chấm công & Nghỉ phép" }
                                div { class: "ent-form-grid",
                                    label { class: "form-field", span { "Giờ vào ca chuẩn" }
                                        input { r#type: "time", disabled: !can_ent, value: "{work_start}", oninput: move |e| work_start.set(e.value()) }
                                    }
                                    label { class: "form-field", span { "Giờ tan ca chuẩn" }
                                        input { r#type: "time", disabled: !can_ent, value: "{work_end}", oninput: move |e| work_end.set(e.value()) }
                                    }
                                    label { class: "form-field", span { "Ân hạn đi muộn (phút)" }
                                        input { r#type: "number", disabled: !can_ent, value: "{grace_mins}", oninput: move |e| grace_mins.set(e.value()) }
                                    }
                                    label { class: "form-field", span { "Quỹ phép năm mặc định (ngày)" }
                                        input { r#type: "number", disabled: !can_ent, value: "{annual_days}", oninput: move |e| annual_days.set(e.value()) }
                                    }
                                    label { class: "form-field", span { "Quỹ nghỉ ốm có lương (ngày)" }
                                        input { r#type: "number", disabled: !can_ent, value: "{sick_days}", oninput: move |e| sick_days.set(e.value()) }
                                    }
                                }
                                if can_ent {
                                    div { class: "ent-btn-group",
                                        button { r#type: "button", class: "btn-primary", onclick: {
                                            let att_base = c.attendance_policy.clone();
                                            let lv_base = c.leave_policy.clone();
                                            move |_| {
                                                let mut a = att_base.as_object().cloned().unwrap_or_default();
                                                a.insert("work_start".into(), json!(work_start()));
                                                a.insert("work_end".into(), json!(work_end()));
                                                a.insert("grace_minutes".into(), json!(grace_mins().parse::<i64>().unwrap_or(15)));
                                                let mut l = lv_base.as_object().cloned().unwrap_or_default();
                                                l.insert("annual_days".into(), json!(annual_days().parse::<i64>().unwrap_or(12)));
                                                l.insert("sick_days".into(), json!(sick_days().parse::<i64>().unwrap_or(5)));
                                                save_patch.call((json!({ "attendance_policy": Value::Object(a), "leave_policy": Value::Object(l) }), "Đã lưu chính sách chấm công & nghỉ phép"));
                                            }
                                        }, "Lưu thông số ca làm & quỹ phép" }
                                    }
                                }
                                label { class: "settings-row toggle-row",
                                    span { class: "grow",
                                        span { class: "settings-label", "Cho phép chấm công làm việc từ xa" }
                                        span { class: "settings-row-desc", "Nhân viên có thể chọn hình thức Từ xa hoặc Linh hoạt khi vào ca" }
                                    }
                                    input { r#type: "checkbox", class: "switch", checked: bool_val(&c.attendance_policy, "allow_remote", true), disabled: !can_ent,
                                        onchange: {
                                            let base = c.attendance_policy.clone();
                                            move |e| save_patch.call((json!({ "attendance_policy": with_key(&base, "allow_remote", json!(e.checked())) }), "Đã cập nhật chính sách chấm công"))
                                        }
                                    }
                                }
                            }
                        }

                        if show_docs {
                            div { class: "ent-card",
                                h4 { FileTextIcon { size: 16 } " Phê duyệt, Tài liệu & Thông báo nội bộ" }
                                div { class: "ent-inline-bar",
                                    label { class: "form-field", span { "Hạn mức lưu trữ tài liệu (MB)" }
                                        input { r#type: "number", disabled: !can_ent, value: "{quota_mb}", oninput: move |e| quota_mb.set(e.value()) }
                                    }
                                    if can_ent {
                                        button { r#type: "button", class: "btn-primary", onclick: {
                                            let base = c.document_policy.clone();
                                            move |_| {
                                                let mb = quota_mb().parse::<i64>().unwrap_or(2048).max(10);
                                                save_patch.call((json!({ "document_policy": with_key(&base, "quota_mb", json!(mb)) }), "Đã cập nhật hạn mức lưu trữ"));
                                            }
                                        }, "Lưu hạn mức" }
                                    }
                                }
                                label { class: "settings-row toggle-row",
                                    span { class: "grow",
                                        span { class: "settings-label", "Cho phép người tạo thu hồi đề xuất đang chờ duyệt" }
                                        span { class: "settings-row-desc", "Người gửi có thể rút lại biểu mẫu trước khi cấp cuối phê duyệt" }
                                    }
                                    input { r#type: "checkbox", class: "switch", checked: bool_val(&c.approval_policy, "allow_withdraw", true), disabled: !can_ent,
                                        onchange: {
                                            let base = c.approval_policy.clone();
                                            move |e| save_patch.call((json!({ "approval_policy": with_key(&base, "allow_withdraw", json!(e.checked())) }), "Đã cập nhật quy trình phê duyệt"))
                                        }
                                    }
                                }
                                label { class: "settings-row toggle-row",
                                    span { class: "grow",
                                        span { class: "settings-label", "Chống gửi trùng thông báo tự động" }
                                        span { class: "settings-row-desc", "Gộp các thông báo nhắc việc và phê duyệt trùng lặp trong cùng khung giờ" }
                                    }
                                    input { r#type: "checkbox", class: "switch", checked: bool_val(&c.notification_policy, "dedup_alerts", true), disabled: !can_ent,
                                        onchange: {
                                            let base = c.notification_policy.clone();
                                            move |e| save_patch.call((json!({ "notification_policy": with_key(&base, "dedup_alerts", json!(e.checked())) }), "Đã cập nhật cài đặt thông báo"))
                                        }
                                    }
                                }
                            }
                        }

                        if show_sec {
                            div { class: "ent-card",
                                h4 { LockIcon { size: 16 } " Bảo mật, Quyền riêng tư & Trợ lý AI" }
                                label { class: "settings-row toggle-row",
                                    span { class: "grow",
                                        span { class: "settings-label", "Yêu cầu xác thực đa yếu tố (MFA/2FA) cho vai trò quản lý" }
                                        span { class: "settings-row-desc", "Ghi nhận chính sách bắt buộc bảo mật tài khoản đối với người giữ quyền quản trị" }
                                    }
                                    input { r#type: "checkbox", class: "switch", checked: bool_val(&c.security_policy, "require_mfa", false), disabled: !can_sec,
                                        onchange: {
                                            let base = c.security_policy.clone();
                                            move |e| save_patch.call((json!({ "security_policy": with_key(&base, "require_mfa", json!(e.checked())) }), "Đã cập nhật chính sách MFA"))
                                        }
                                    }
                                }
                                label { class: "settings-row toggle-row",
                                    span { class: "grow",
                                        span { class: "settings-label", "Ẩn thông tin liên hệ nhân sự đối với khách mời" }
                                        span { class: "settings-row-desc", "Khách mời và đối tác không thể tra cứu email hoặc số điện thoại nội bộ" }
                                    }
                                    input { r#type: "checkbox", class: "switch", checked: bool_val(&c.privacy_policy, "hide_contact_from_guests", true), disabled: !can_sec,
                                        onchange: {
                                            let base = c.privacy_policy.clone();
                                            move |e| save_patch.call((json!({ "privacy_policy": with_key(&base, "hide_contact_from_guests", json!(e.checked())) }), "Đã cập nhật quyền riêng tư"))
                                        }
                                    }
                                }
                                label { class: "settings-row toggle-row",
                                    span { class: "grow",
                                        span { class: "settings-label", SparkleIcon { size: 15 } " Cho phép Trợ lý tổng hợp AI doanh nghiệp" }
                                        span { class: "settings-row-desc", "Chỉ tổng hợp dữ liệu nghiệp vụ máy chủ được cấp quyền, không giải mã tin nhắn E2EE" }
                                    }
                                    input {
                                        r#type: "checkbox",
                                        class: "switch",
                                        checked: bool_val(&c.ai_config, "enabled", false),
                                        disabled: !can_ai,
                                        onchange: move |e| {
                                            save_patch.call((json!({ "ai_config": { "enabled": e.checked(), "provider": "local" } }), "Đã cập nhật cấu hình AI"));
                                        },
                                    }
                                }
                            }
                        }

                        if show_adv {
                            div { class: "ent-card",
                                h4 { ShieldIcon { size: 16 } " Kiểm toán, Lưu trữ & Bảo toàn dữ liệu" }
                                label { class: "settings-row toggle-row",
                                    span { class: "grow",
                                        span { class: "settings-label", "Khóa bảo toàn dữ liệu doanh nghiệp (Retention Lock)" }
                                        span { class: "settings-row-desc", "Chặn thao tác tắt chế độ doanh nghiệp khi còn nghĩa vụ lưu trữ hồ sơ và kiểm toán" }
                                    }
                                    input {
                                        r#type: "checkbox",
                                        class: "switch",
                                        checked: c.retention_locked,
                                        disabled: !can_sec,
                                        onchange: move |e| {
                                            save_patch.call((json!({ "retention_locked": e.checked() }), "Đã cập nhật khóa bảo toàn dữ liệu"));
                                        },
                                    }
                                }
                                div { class: "ent-btn-group", style: "margin-top: 8px;",
                                    a { class: "btn-secondary", href: "{export_reports_csv_url(&server_id)}", download: "enterprise-report.csv", DownloadIcon { size: 15 } " Xuất báo cáo CSV" }
                                    a { class: "btn-secondary", href: "{export_audit_csv_url(&server_id)}", download: "enterprise-audit.csv", DownloadIcon { size: 15 } " Xuất nhật ký kiểm toán CSV" }
                                    a { class: "btn-secondary", href: "{export_all_json_url(&server_id)}", download: "enterprise-export.json", DownloadIcon { size: 15 } " Sao lưu toàn bộ dữ liệu (JSON)" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
