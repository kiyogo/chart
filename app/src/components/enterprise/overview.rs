use dioxus::prelude::*;
use serde_json::Value;

use crate::{
    components::icons::{
        BarChartIcon, BriefcaseIcon, CalendarIcon, ClipboardCheckIcon, DownloadIcon,
        ListChecksIcon, MegaphoneIcon, RefreshIcon,
    },
    services::enterprise::{
        self, EnterpriseAuditItem, EnterpriseConfig, EnterpriseOverview, export_all_json_url,
        export_audit_csv_url, export_reports_csv_url,
    },
};

#[component]
pub fn OverviewPanel(
    server_id: String,
    cfg: EnterpriseConfig,
    refresh_tick: u64,
    on_navigate: EventHandler<String>,
) -> Element {
    let mut data = use_signal(|| Option::<EnterpriseOverview>::None);
    let mut err = use_signal(|| Option::<String>::None);
    let mut check_note = use_signal(String::new);
    let mut work_mode = use_signal(|| "onsite".to_string());
    let mut busy = use_signal(|| false);

    {
        let sid = server_id.clone();
        use_effect(use_reactive!(|(sid, refresh_tick)| {
            let sid = sid.clone();
            let _ = refresh_tick;
            spawn(async move {
                match enterprise::get_overview(&sid).await {
                    Ok(v) => {
                        data.set(Some(v));
                        err.set(None);
                    }
                    Err(e) => err.set(Some(format!("Unable to load overview: {}", e.code))),
                }
            });
        }));
    }

    let do_check = use_callback({
        let sid = server_id.clone();
        move |is_in: bool| {
            if busy() {
                return;
            }
            busy.set(true);
            let sid = sid.clone();
            let mode = work_mode();
            let note = check_note();
            spawn(async move {
                let res = if is_in {
                    enterprise::check_in(&sid, &mode, &note).await
                } else {
                    enterprise::check_out(&sid, &note).await
                };
                match res {
                    Ok(_) => {
                        check_note.set(String::new());
                        if let Ok(v) = enterprise::get_overview(&sid).await {
                            data.set(Some(v));
                        }
                        err.set(None);
                    }
                    Err(e) => err.set(Some(format!("Attendance failed: {}", e.code))),
                }
                busy.set(false);
            });
        }
    });

    rsx! {
        div { class: "ent-section",
            if let Some(m) = err() {
                div { class: "status-badge error", "{m}" }
            }
            if let Some(ov) = data() {
                div { class: "ent-kpi-grid",
                    button { r#type: "button", class: "ent-kpi-card", onclick: move |_| on_navigate.call("tasks".into()),
                        div { class: "ent-kpi-head", span { "Things to do" } ListChecksIcon { size: 16 } }
                        div { class: "ent-kpi-val", "{ov.my_open_tasks}" }
                        div { class: "ent-kpi-sub", "{ov.my_overdue_tasks} lateness" }
                    }
                    button { r#type: "button", class: "ent-kpi-card", onclick: move |_| on_navigate.call("meetings".into()),
                        div { class: "ent-kpi-head", span { "Meeting schedule today" } CalendarIcon { size: 16 } }
                        div { class: "ent-kpi-val", "{ov.today_meetings}" }
                        div { class: "ent-kpi-sub", "The meeting is open" }
                    }
                    button { r#type: "button", class: "ent-kpi-card", onclick: move |_| on_navigate.call("announcements".into()),
                        div { class: "ent-kpi-head", span { "New announcement" } MegaphoneIcon { size: 16 } }
                        div { class: "ent-kpi-val", "{ov.unread_announcements}" }
                        div { class: "ent-kpi-sub", "Unread internal news" }
                    }
                    button { r#type: "button", class: "ent-kpi-card", onclick: move |_| on_navigate.call("approvals".into()),
                        div { class: "ent-kpi-head", span { "Proposal awaiting approval" } ClipboardCheckIcon { size: 16 } }
                        div { class: "ent-kpi-val", "{ov.pending_approvals}" }
                        div { class: "ent-kpi-sub", "{ov.pending_leave} pending leave application" }
                    }
                    button { r#type: "button", class: "ent-kpi-card", onclick: move |_| on_navigate.call("hr".into()),
                        div { class: "ent-kpi-head", span { "Operational personnel" } BriefcaseIcon { size: 16 } }
                        div { class: "ent-kpi-val", "{ov.active_employees}" }
                        div { class: "ent-kpi-sub", "{ov.departments_count} departments" }
                    }
                    button { r#type: "button", class: "ent-kpi-card", onclick: move |_| on_navigate.call("projects".into()),
                        div { class: "ent-kpi-head", span { "Project is running" } BarChartIcon { size: 16 } }
                        div { class: "ent-kpi-val", "{ov.active_projects}" }
                        div { class: "ent-kpi-sub", "{ov.done_tasks}/{ov.total_tasks} job completed" }
                    }
                }

                if cfg.enabled_modules.iter().any(|m| m == "attendance") {
                    div { class: "ent-card",
                        div { class: "ent-toolbar",
                            div { class: "ent-chips",
                                if let Some(att) = &ov.my_attendance {
                                    span { class: "ent-pill ok", "Today's shift: {att.work_date}" }
                                    span { class: "ent-pill", "{enterprise::vn_work_mode(&att.work_mode)}" }
                                    if let Some(ci) = &att.check_in_at {
                                        span { class: "ent-pill", "On shift: {ci.chars().take(19).collect::<String>()}" }
                                    }
                                    if let Some(co) = &att.check_out_at {
                                        span { class: "ent-pill", "Tan ca: {co.chars().take(19).collect::<String>()}" }
                                    }
                                    if att.late_minutes > 0 {
                                        span { class: "ent-pill warn", "{att.late_minutes} minutes late" }
                                    }
                                    if att.overtime_minutes > 0 {
                                        span { class: "ent-pill ok", "Overtime {att.overtime_minutes} minutes" }
                                    }
                                } else {
                                    span { class: "ent-pill", "Haven't clocked in today's shift (server time)" }
                                }
                            }
                            div { class: "ent-inline-bar",
                                select { value: "{work_mode}", onchange: move |e| work_mode.set(e.value()),
                                    option { value: "onsite", "At the office" }
                                    option { value: "remote", "From afar" }
                                    option { value: "hybrid", "Flexible" }
                                }
                                input {
                                    r#type: "text",
                                    placeholder: "Shift notes",
                                    value: "{check_note}",
                                    oninput: move |e| check_note.set(e.value()),
                                }
                                button { r#type: "button", class: "btn-primary", disabled: busy(), onclick: move |_| do_check.call(true), "Clock in" }
                                button { r#type: "button", class: "btn-secondary", disabled: busy(), onclick: move |_| do_check.call(false), "Clock out" }
                            }
                        }
                    }
                }

                div { class: "ent-two-col",
                    div { class: "ent-card",
                        div { class: "ent-stat-list",
                            div { class: "ent-stat-row",
                                span { "Progress of work completion" }
                                strong { "{ov.done_tasks} / {ov.total_tasks}" }
                            }
                            div { class: "ent-stat-row",
                                span { "Departments & Specialized Departments" }
                                strong { "{ov.departments_count} departments" }
                            }
                            div { class: "ent-stat-row",
                                span { "Leave application & Approval pending" }
                                strong { "{ov.pending_leave + ov.pending_approvals} request" }
                            }
                        }
                    }
                    div { class: "ent-card",
                        div { class: "ent-chips",
                            span { class: "ent-pill", "/task <title>" }
                            span { class: "ent-pill", "/checkin [note]" }
                            span { class: "ent-pill", "/checkout [note]" }
                            span { class: "ent-pill", "/leave <reason>" }
                            span { class: "ent-pill", "/meet <title>" }
                            span { class: "ent-pill", "/approve <proposal>" }
                            span { class: "ent-pill", "/announce <content>" }
                            span { class: "ent-pill", "/workspace" }
                        }
                        div { class: "ent-muted",
                            "Type the above quick commands directly in any chat channel of the business server to assign tasks, time clock, schedule meetings or send proposals for approval."
                        }
                    }
                }
            } else {
                div { class: "ent-empty", "Synchronizing business space data" }
            }
        }
    }
}

#[component]
pub fn ReportsPanel(server_id: String, cfg: EnterpriseConfig, refresh_tick: u64) -> Element {
    let mut rep = use_signal(|| Option::<Value>::None);
    let mut audits = use_signal(Vec::<EnterpriseAuditItem>::new);
    let mut audit_q = use_signal(String::new);
    let mut err = use_signal(|| Option::<String>::None);

    let can_audit = cfg.has(enterprise::ent_perm::VIEW_AUDIT) || cfg.has(enterprise::ent_perm::MANAGE_SECURITY);
    let can_export = cfg.has(enterprise::ent_perm::EXPORT_DATA);

    {
        let sid = server_id.clone();
        use_effect(use_reactive!(|(sid, refresh_tick)| {
            let sid = sid.clone();
            let _ = refresh_tick;
            let q = audit_q();
            spawn(async move {
                match enterprise::reports(&sid, None).await {
                    Ok(v) => {
                        rep.set(Some(v));
                        err.set(None);
                    }
                    Err(e) => err.set(Some(format!("Unable to view report: {}", e.code))),
                }
                if can_audit {
                    if let Ok(a) = enterprise::audit_log(&sid, &q).await {
                        audits.set(a);
                    }
                }
            });
        }));
    }

    let reload_audits = {
        let sid = server_id.clone();
        move |_| {
            let sid = sid.clone();
            let q = audit_q();
            spawn(async move {
                if let Ok(a) = enterprise::audit_log(&sid, &q).await {
                    audits.set(a);
                }
            });
        }
    };

    rsx! {
        div { class: "ent-section",
            div { class: "ent-toolbar",
                if can_audit {
                    div { class: "ent-inline-bar",
                        input { r#type: "text", placeholder: "Filter logs by action or object", value: "{audit_q}", oninput: move |e| audit_q.set(e.value()) }
                        button { r#type: "button", class: "btn-secondary", onclick: reload_audits, RefreshIcon { size: 15 } " Filter logs" }
                    }
                } else {
                    div {}
                }
                if can_export {
                    div { class: "ent-btn-group",
                        a { class: "btn-secondary", href: "{export_reports_csv_url(&server_id)}", download: "enterprise-report.csv", DownloadIcon { size: 15 } " Export CSV report" }
                        a { class: "btn-secondary", href: "{export_audit_csv_url(&server_id)}", download: "enterprise-audit.csv", DownloadIcon { size: 15 } " CSV log" }
                        a { class: "btn-secondary", href: "{export_all_json_url(&server_id)}", download: "enterprise-export.json", DownloadIcon { size: 15 } " Full JSON" }
                    }
                }
            }
            if let Some(m) = err() {
                div { class: "status-badge error", "{m}" }
            }
            if let Some(r) = rep() {
                div { class: "ent-two-col",
                    div { class: "ent-card",
                        div { class: "ent-stat-list",
                            div { class: "ent-stat-row", span { "Personnel is active" } strong { "{r[\"active_members\"].as_i64().unwrap_or(0)}" } }
                            div { class: "ent-stat-row", span { "Overdue work" } strong { "{r[\"overdue_tasks\"].as_i64().unwrap_or(0)}" } }
                            div { class: "ent-stat-row", span { "Document storage capacity" } strong { "{r[\"total_doc_bytes\"].as_i64().unwrap_or(0) / 1024} KB" } }
                        }
                        if let Some(arr) = r["tasks_by_status"].as_array() {
                            div { class: "ent-chips",
                                for item in arr {
                                    if let Some(pair) = item.as_array() {
                                        span { class: "ent-pill", "{enterprise::vn_task_status(pair[0].as_str().unwrap_or(\"\"))}: {pair[1].as_i64().unwrap_or(0)}" }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "ent-card",
                        if let Some(depts) = r["department_workload"].as_array() {
                            if depts.is_empty() {
                                p { class: "ent-muted", "No departments have been established yet." }
                            } else {
                                div { class: "ent-stat-list",
                                    for d in depts {
                                        div { class: "ent-stat-row",
                                            span { "{d[\"name\"].as_str().unwrap_or(\"\")}" }
                                            span { class: "ent-muted", "{d[\"employees\"].as_i64().unwrap_or(0)} personnel · {d[\"open_tasks\"].as_i64().unwrap_or(0)} job opening" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if can_audit {
                div { class: "ent-table-wrap",
                    table { class: "ent-table",
                        thead {
                            tr { th { "Time" } th { "Implementer" } th { "Action" } th { "Object" } }
                        }
                        tbody {
                            for a in audits() {
                                tr { key: "{a.id}",
                                    td { "{a.created_at.chars().take(19).collect::<String>()}" }
                                    td { "{a.actor_name.as_deref().unwrap_or(\"System\")}" }
                                    td { span { class: "ent-pill", "{a.action}" } }
                                    td { "{a.target.as_deref().unwrap_or(\"-\")}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
