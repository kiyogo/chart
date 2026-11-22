use dioxus::prelude::*;
use serde_json::{Value, json};

use crate::{
    components::icons::{DownloadIcon, VicoAiIcon},
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
pub fn EnterpriseSettingsPanel(
    server_id: String,
    on_changed: EventHandler<()>,
    #[props(default)] active_section: Option<&'static str>,
) -> Element {
    let mut cfg = use_signal(|| Option::<EnterpriseConfig>::None);
    let mut cat = use_signal(|| "general");
    let mut msg = use_signal(|| Option::<(bool, String)>::None);

    let mut company_name = use_signal(String::new);
    let mut contact_email = use_signal(String::new);
    let mut contact_phone = use_signal(String::new);
    let mut website = use_signal(String::new);
    let mut timezone = use_signal(|| "Asia/Ho_Chi_Minh".to_string());
    let mut date_format = use_signal(|| "DD/MM/YYYY".to_string());
    let mut emp_prefix = use_signal(|| "NV".to_string());
    let mut work_start = use_signal(|| "08:30".to_string());
    let mut work_end = use_signal(|| "17:30".to_string());
    let mut grace_mins = use_signal(|| "15".to_string());
    let mut annual_days = use_signal(|| "12".to_string());
    let mut sick_days = use_signal(|| "5".to_string());
    let mut quota_mb = use_signal(|| "2048".to_string());
    let mut mod_drag_from = use_signal(|| Option::<usize>::None);
    let mut mod_drag_over = use_signal(|| Option::<usize>::None);

    let mut sync_inputs = move |c: &EnterpriseConfig| {
        company_name.set(str_val(&c.general_settings, "company_name", ""));
        contact_email.set(str_val(&c.general_settings, "contact_email", ""));
        contact_phone.set(str_val(&c.general_settings, "contact_phone", ""));
        website.set(str_val(&c.general_settings, "website", ""));
        timezone.set(str_val(&c.general_settings, "timezone", "Asia/Ho_Chi_Minh"));
        date_format.set(str_val(&c.general_settings, "date_format", "DD/MM/YYYY"));
        emp_prefix.set(str_val(&c.hr_policy, "code_prefix", "NV"));
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
                    Err(e) => msg.set(Some((true, format!("Unable to save: {}", e.code)))),
                }
            });
        }
    });

    let current_sec = active_section.unwrap_or_else(|| cat());

    rsx! {
        div { class: "ent-section",
            if active_section.is_none() {
                div { class: "ent-toolbar",
                    div { class: "sidebar-filters",
                        for (k, label) in [
                            ("general", "Setup"),
                            ("hr", "Human resources"),
                            ("work", "Work"),
                            ("time", "Timekeeping"),
                            ("docs", "Approval"),
                            ("security", "Security"),
                            ("advanced", "Auditing"),
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
                        if current_sec == "general" {
                            div { class: "settings-group",
                                div { class: "ent-card",
                                    div { class: "ent-form-grid",
                                        label { class: "form-field", span { "Business name" }
                                            input { r#type: "text", disabled: !can_ent, value: "{company_name}", oninput: move |e| company_name.set(e.value()) }
                                        }
                                        label { class: "form-field", span { "Contact email" }
                                            input { r#type: "text", disabled: !can_ent, value: "{contact_email}", oninput: move |e| contact_email.set(e.value()) }
                                        }
                                        label { class: "form-field", span { "Website" }
                                            input { r#type: "text", disabled: !can_ent, value: "{website}", oninput: move |e| website.set(e.value()) }
                                        }
                                        label { class: "form-field", span { "Contact phone" }
                                            input { r#type: "text", disabled: !can_ent, value: "{contact_phone}", oninput: move |e| contact_phone.set(e.value()) }
                                        }
                                        label { class: "form-field", span { "Time zone" }
                                            select { disabled: !can_ent, value: "{timezone}", onchange: move |e| timezone.set(e.value()),
                                                option { value: "Asia/Ho_Chi_Minh", "Vietnam GMT+7" }
                                                option { value: "Asia/Tokyo", "Tokyo GMT+9" }
                                                option { value: "UTC", "UTC International" }
                                            }
                                        }
                                        label { class: "form-field", span { "Date format" }
                                            select { disabled: !can_ent, value: "{date_format}", onchange: move |e| date_format.set(e.value()),
                                                option { value: "DD/MM/YYYY", "DD/MM/YYYY" }
                                                option { value: "YYYY-MM-DD", "YYYY-MM-DD" }
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
                                                        obj.insert("contact_phone".into(), json!(contact_phone()));
                                                        obj.insert("website".into(), json!(website()));
                                                        obj.insert("timezone".into(), json!(timezone()));
                                                        obj.insert("date_format".into(), json!(date_format()));
                                                        save_patch.call((json!({ "general_settings": Value::Object(obj) }), "Settings saved"));
                                                    }
                                                },
                                                "Save"
                                            }
                                        }
                                    }
                                }
                            }

                            div { class: "settings-group",
                                h2 { class: "settings-group-title", "Appearance" }
                                div { class: "settings-list",
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "Compact mode" } }
                                        input {
                                            r#type: "checkbox",
                                            class: "switch",
                                            checked: bool_val(&c.display_settings, "compact", false),
                                            disabled: !can_ent,
                                            onchange: {
                                                let base = c.display_settings.clone();
                                                move |e| {
                                                    save_patch.call((json!({ "display_settings": with_key(&base, "compact", json!(e.checked())) }), "Interface saved"));
                                                }
                                            },
                                        }
                                    }
                                }
                            }

                            div { class: "settings-group",
                                h2 { class: "settings-group-title", "Sidebar" }
                                div { class: "settings-list", "data-drag-group": "ent-sidebar-mods",
                                    for (idx, key) in ordered_mods.iter().enumerate() {
                                        {
                                            let label = enterprise::module_label(key);
                                            let enabled = c.enabled_modules.iter().any(|m| m == key);
                                            let cur_enabled = c.enabled_modules.clone();
                                            let cur_order = ordered_mods.clone();
                                            let k_str = key.clone();
                                            let dragging_now = mod_drag_from() == Some(idx);
                                            let over_cls = match (mod_drag_from(), mod_drag_over()) {
                                                (Some(from), Some(ov)) if ov == idx && from != idx => {
                                                    if idx < from { " drag-over-before" } else { " drag-over-after" }
                                                }
                                                _ => "",
                                            };
                                            let row_cls = format!(
                                                "settings-row toggle-row{}{}{}",
                                                if can_ent { " draggable-item" } else { "" },
                                                if dragging_now { " is-dragging" } else { "" },
                                                over_cls
                                            );
                                            rsx! {
                                                div {
                                                    key: "mod-{key}",
                                                    class: "{row_cls}",
                                                    draggable: if can_ent { "true" } else { "false" },
                                                    "data-drag-item": if can_ent { "true" } else { "false" },
                                                    "data-drag-key": "{key}",
                                                    ondragstart: move |_| {
                                                        if can_ent {
                                                            mod_drag_from.set(Some(idx));
                                                            mod_drag_over.set(None);
                                                        }
                                                    },
                                                    ondragover: move |e| {
                                                        if can_ent && mod_drag_from().is_some() {
                                                            e.prevent_default();
                                                            if mod_drag_over() != Some(idx) {
                                                                mod_drag_over.set(Some(idx));
                                                            }
                                                        }
                                                    },
                                                    ondrop: move |e| {
                                                        e.prevent_default();
                                                        if can_ent {
                                                            if let Some(from) = mod_drag_from() {
                                                                if from != idx && from < cur_order.len() && idx < cur_order.len() {
                                                                    let mut next = cur_order.clone();
                                                                    let item = next.remove(from);
                                                                    next.insert(idx, item);
                                                                    save_patch.call((json!({ "module_order": next }), "Changed order"));
                                                                }
                                                            }
                                                        }
                                                        mod_drag_from.set(None);
                                                        mod_drag_over.set(None);
                                                    },
                                                    ondragend: move |_| {
                                                        mod_drag_from.set(None);
                                                        mod_drag_over.set(None);
                                                    },
                                                    span { class: "grow", span { class: "settings-label", "{label}" } }
                                                    div { class: "ent-btn-group",
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
                                                                save_patch.call((json!({ "enabled_modules": next }), "Updated"));
                                                            },
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if current_sec == "hr" {
                            div { class: "settings-group",
                                div { class: "ent-card",
                                    div { class: "ent-inline-bar",
                                        label { class: "form-field", span { "Employee code" }
                                            input { r#type: "text", disabled: !can_ent, value: "{emp_prefix}", oninput: move |e| emp_prefix.set(e.value()) }
                                        }
                                        if can_ent {
                                            button { r#type: "button", class: "btn-primary", onclick: {
                                                let base = c.hr_policy.clone();
                                                move |_| {
                                                    save_patch.call((json!({ "hr_policy": with_key(&base, "code_prefix", json!(emp_prefix())) }), "Saved"));
                                                }
                                            }, "Save" }
                                        }
                                    }
                                }
                                div { class: "settings-list",
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "Browse external members" } }
                                        input { r#type: "checkbox", class: "switch", checked: bool_val(&c.hr_policy, "approve_external_guests", true), disabled: !can_ent,
                                            onchange: {
                                                let base = c.hr_policy.clone();
                                                move |e| save_patch.call((json!({ "hr_policy": with_key(&base, "approve_external_guests", json!(e.checked())) }), "Saved"))
                                            }
                                        }
                                    }
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "Revocation of rights upon termination of employment" } }
                                        input { r#type: "checkbox", class: "switch", checked: bool_val(&c.hr_policy, "auto_revoke_offboarded", true), disabled: !can_ent,
                                            onchange: {
                                                let base = c.hr_policy.clone();
                                                move |e| save_patch.call((json!({ "hr_policy": with_key(&base, "auto_revoke_offboarded", json!(e.checked())) }), "Saved"))
                                            }
                                        }
                                    }
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "Create your own profile when joining" } }
                                        input { r#type: "checkbox", class: "switch", checked: bool_val(&c.hr_policy, "auto_onboard_profile", true), disabled: !can_ent,
                                            onchange: {
                                                let base = c.hr_policy.clone();
                                                move |e| save_patch.call((json!({ "hr_policy": with_key(&base, "auto_onboard_profile", json!(e.checked())) }), "Saved"))
                                            }
                                        }
                                    }
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "Limit the number of invitation links" } }
                                        input { r#type: "checkbox", class: "switch", checked: bool_val(&c.hr_policy, "limit_invite_uses", true), disabled: !can_ent,
                                            onchange: {
                                                let base = c.hr_policy.clone();
                                                move |e| save_patch.call((json!({ "hr_policy": with_key(&base, "limit_invite_uses", json!(e.checked())) }), "Saved"))
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if current_sec == "work" {
                            div { class: "settings-group",
                                div { class: "settings-list",
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "Browse before completing" } }
                                        input { r#type: "checkbox", class: "switch", checked: bool_val(&c.task_policy, "require_review", false), disabled: !can_ent,
                                            onchange: {
                                                let base = c.task_policy.clone();
                                                move |e| save_patch.call((json!({ "task_policy": with_key(&base, "require_review", json!(e.checked())) }), "Saved"))
                                            }
                                        }
                                    }
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "Work deadline reminder" } }
                                        input { r#type: "checkbox", class: "switch", checked: bool_val(&c.task_policy, "auto_deadline_reminder", true), disabled: !can_ent,
                                            onchange: {
                                                let base = c.task_policy.clone();
                                                move |e| save_patch.call((json!({ "task_policy": with_key(&base, "auto_deadline_reminder", json!(e.checked())) }), "Saved"))
                                            }
                                        }
                                    }
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "Hide private channel quotes" } }
                                        input { r#type: "checkbox", class: "switch", checked: bool_val(&c.project_policy, "redact_private_channel_source", true), disabled: !can_ent,
                                            onchange: {
                                                let base = c.project_policy.clone();
                                                move |e| save_patch.call((json!({ "project_policy": with_key(&base, "redact_private_channel_source", json!(e.checked())) }), "Saved"))
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if current_sec == "time" {
                            div { class: "settings-group",
                                div { class: "ent-card",
                                    div { class: "ent-form-grid",
                                        label { class: "form-field", span { "Shift time now" }
                                            input { r#type: "time", disabled: !can_ent, value: "{work_start}", oninput: move |e| work_start.set(e.value()) }
                                        }
                                        label { class: "form-field", span { "Off-duty time" }
                                            input { r#type: "time", disabled: !can_ent, value: "{work_end}", oninput: move |e| work_end.set(e.value()) }
                                        }
                                        label { class: "form-field", span { "Allowed lateness (minutes)" }
                                            input { r#type: "number", disabled: !can_ent, value: "{grace_mins}", oninput: move |e| grace_mins.set(e.value()) }
                                        }
                                        label { class: "form-field", span { "Annual leave (days)" }
                                            input { r#type: "number", disabled: !can_ent, value: "{annual_days}", oninput: move |e| annual_days.set(e.value()) }
                                        }
                                        label { class: "form-field", span { "Sick leave (days)" }
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
                                                    save_patch.call((json!({ "attendance_policy": Value::Object(a), "leave_policy": Value::Object(l) }), "Saved"));
                                                }
                                            }, "Save" }
                                        }
                                    }
                                }
                                div { class: "settings-list",
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "Timekeeping remotely" } }
                                        input { r#type: "checkbox", class: "switch", checked: bool_val(&c.attendance_policy, "allow_remote", true), disabled: !can_ent,
                                            onchange: {
                                                let base = c.attendance_policy.clone();
                                                move |e| save_patch.call((json!({ "attendance_policy": with_key(&base, "allow_remote", json!(e.checked())) }), "Saved"))
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if current_sec == "docs" {
                            div { class: "settings-group",
                                div { class: "ent-card",
                                    div { class: "ent-inline-bar",
                                        label { class: "form-field", span { "Storage limit (MB)" }
                                            input { r#type: "number", disabled: !can_ent, value: "{quota_mb}", oninput: move |e| quota_mb.set(e.value()) }
                                        }
                                        if can_ent {
                                            button { r#type: "button", class: "btn-primary", onclick: {
                                                let base = c.document_policy.clone();
                                                move |_| {
                                                    let mb = quota_mb().parse::<i64>().unwrap_or(2048).max(10);
                                                    save_patch.call((json!({ "document_policy": with_key(&base, "quota_mb", json!(mb)) }), "Saved"));
                                                }
                                            }, "Save" }
                                        }
                                    }
                                }
                                div { class: "settings-list",
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "Withdraw proposal pending approval" } }
                                        input { r#type: "checkbox", class: "switch", checked: bool_val(&c.approval_policy, "allow_withdraw", true), disabled: !can_ent,
                                            onchange: {
                                                let base = c.approval_policy.clone();
                                                move |e| save_patch.call((json!({ "approval_policy": with_key(&base, "allow_withdraw", json!(e.checked())) }), "Saved"))
                                            }
                                        }
                                    }
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "Anti-duplicate notifications" } }
                                        input { r#type: "checkbox", class: "switch", checked: bool_val(&c.notification_policy, "dedup_alerts", true), disabled: !can_ent,
                                            onchange: {
                                                let base = c.notification_policy.clone();
                                                move |e| save_patch.call((json!({ "notification_policy": with_key(&base, "dedup_alerts", json!(e.checked())) }), "Saved"))
                                            }
                                        }
                                    }
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "Check the upload file format" } }
                                        input { r#type: "checkbox", class: "switch", checked: bool_val(&c.document_policy, "verify_magic_bytes", true), disabled: !can_ent,
                                            onchange: {
                                                let base = c.document_policy.clone();
                                                move |e| save_patch.call((json!({ "document_policy": with_key(&base, "verify_magic_bytes", json!(e.checked())) }), "Saved"))
                                            }
                                        }
                                    }
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "Save document version history" } }
                                        input { r#type: "checkbox", class: "switch", checked: bool_val(&c.document_policy, "keep_versions", true), disabled: !can_ent,
                                            onchange: {
                                                let base = c.document_policy.clone();
                                                move |e| save_patch.call((json!({ "document_policy": with_key(&base, "keep_versions", json!(e.checked())) }), "Saved"))
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if current_sec == "security" {
                            div { class: "settings-group",
                                div { class: "settings-list",
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "Two-step authentication for management" } }
                                        input { r#type: "checkbox", class: "switch", checked: bool_val(&c.security_policy, "require_mfa", false), disabled: !can_sec,
                                            onchange: {
                                                let base = c.security_policy.clone();
                                                move |e| save_patch.call((json!({ "security_policy": with_key(&base, "require_mfa", json!(e.checked())) }), "Saved"))
                                            }
                                        }
                                    }
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "Hide contact from guests" } }
                                        input { r#type: "checkbox", class: "switch", checked: bool_val(&c.privacy_policy, "hide_contact_from_guests", true), disabled: !can_sec,
                                            onchange: {
                                                let base = c.privacy_policy.clone();
                                                move |e| save_patch.call((json!({ "privacy_policy": with_key(&base, "hide_contact_from_guests", json!(e.checked())) }), "Saved"))
                                            }
                                        }
                                    }
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "Requires a trusted device when synchronizing" } }
                                        input { r#type: "checkbox", class: "switch", checked: bool_val(&c.security_policy, "require_trusted_device", true), disabled: !can_sec,
                                            onchange: {
                                                let base = c.security_policy.clone();
                                                move |e| save_patch.call((json!({ "security_policy": with_key(&base, "require_trusted_device", json!(e.checked())) }), "Saved"))
                                            }
                                        }
                                    }
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow",
                                            span { class: "settings-label", VicoAiIcon { size: 15 } " Allow Vico AI to synthesize" }
                                        }
                                        input {
                                            r#type: "checkbox",
                                            class: "switch",
                                            checked: bool_val(&c.ai_config, "enabled", false),
                                            disabled: !can_ai,
                                            onchange: move |e| {
                                                save_patch.call((json!({ "ai_config": { "enabled": e.checked(), "provider": "local" } }), "Saved"));
                                            },
                                        }
                                    }
                                }
                            }
                        }

                        if current_sec == "advanced" {
                            div { class: "settings-group",
                                div { class: "settings-list",
                                    label { class: "settings-row toggle-row",
                                        span { class: "grow", span { class: "settings-label", "Data security lock" } }
                                        input {
                                            r#type: "checkbox",
                                            class: "switch",
                                            checked: c.retention_locked,
                                            disabled: !can_sec,
                                            onchange: move |e| {
                                                save_patch.call((json!({ "retention_locked": e.checked() }), "Saved"));
                                            },
                                        }
                                    }
                                }
                                div { class: "ent-btn-group", style: "margin-top: 8px;",
                                    a { class: "btn-secondary", href: "{export_reports_csv_url(&server_id)}", download: "enterprise-report.csv", DownloadIcon { size: 15 } " Export CSV report" }
                                    a { class: "btn-secondary", href: "{export_audit_csv_url(&server_id)}", download: "enterprise-audit.csv", DownloadIcon { size: 15 } " CSV audit export" }
                                    a { class: "btn-secondary", href: "{export_all_json_url(&server_id)}", download: "enterprise-export.json", DownloadIcon { size: 15 } " Export JSON data" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
