use dioxus::prelude::*;
use serde_json::Value;

use crate::{
    components::icons::{
        BarChartIcon, BriefcaseIcon, CalendarIcon, ClipboardCheckIcon, DownloadIcon,
        ListChecksIcon, MegaphoneIcon, RefreshIcon, ShieldIcon,
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
                    Err(e) => err.set(Some(format!("Không tải được tổng quan: {}", e.code))),
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
                    Err(e) => err.set(Some(format!("Chấm công chưa thành công: {}", e.code))),
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
                        div { class: "ent-kpi-head", span { "Việc cần làm" } ListChecksIcon { size: 16 } }
                        div { class: "ent-kpi-val", "{ov.my_open_tasks}" }
                        div { class: "ent-kpi-sub", "{ov.my_overdue_tasks} việc trễ hạn" }
                    }
                    button { r#type: "button", class: "ent-kpi-card", onclick: move |_| on_navigate.call("meetings".into()),
                        div { class: "ent-kpi-head", span { "Lịch họp hôm nay" } CalendarIcon { size: 16 } }
                        div { class: "ent-kpi-val", "{ov.today_meetings}" }
                        div { class: "ent-kpi-sub", "Cuộc họp đang mở" }
                    }
                    button { r#type: "button", class: "ent-kpi-card", onclick: move |_| on_navigate.call("announcements".into()),
                        div { class: "ent-kpi-head", span { "Thông báo mới" } MegaphoneIcon { size: 16 } }
                        div { class: "ent-kpi-val", "{ov.unread_announcements}" }
                        div { class: "ent-kpi-sub", "Tin nội bộ chưa đọc" }
                    }
                    button { r#type: "button", class: "ent-kpi-card", onclick: move |_| on_navigate.call("approvals".into()),
                        div { class: "ent-kpi-head", span { "Đề xuất chờ duyệt" } ClipboardCheckIcon { size: 16 } }
                        div { class: "ent-kpi-val", "{ov.pending_approvals}" }
                        div { class: "ent-kpi-sub", "{ov.pending_leave} đơn nghỉ phép chờ" }
                    }
                    button { r#type: "button", class: "ent-kpi-card", onclick: move |_| on_navigate.call("hr".into()),
                        div { class: "ent-kpi-head", span { "Nhân sự hoạt động" } BriefcaseIcon { size: 16 } }
                        div { class: "ent-kpi-val", "{ov.active_employees}" }
                        div { class: "ent-kpi-sub", "{ov.departments_count} phòng ban" }
                    }
                    button { r#type: "button", class: "ent-kpi-card", onclick: move |_| on_navigate.call("projects".into()),
                        div { class: "ent-kpi-head", span { "Dự án đang chạy" } BarChartIcon { size: 16 } }
                        div { class: "ent-kpi-val", "{ov.active_projects}" }
                        div { class: "ent-kpi-sub", "{ov.done_tasks}/{ov.total_tasks} việc hoàn tất" }
                    }
                }

                if cfg.enabled_modules.iter().any(|m| m == "attendance") {
                    div { class: "ent-card",
                        div { class: "ent-card-head",
                            h4 { "Chấm công nhanh hôm nay" }
                            if let Some(att) = &ov.my_attendance {
                                span { class: "ent-pill", "{att.work_date} · {enterprise::vn_work_mode(&att.work_mode)}" }
                            }
                        }
                        if let Some(att) = &ov.my_attendance {
                            div { class: "ent-muted-row",
                                if let Some(ci) = &att.check_in_at {
                                    span { "Vào ca: {ci.chars().take(19).collect::<String>()}" }
                                }
                                if let Some(co) = &att.check_out_at {
                                    span { "Tan ca: {co.chars().take(19).collect::<String>()}" }
                                }
                                if att.late_minutes > 0 {
                                    span { class: "ent-pill warn", "Muộn {att.late_minutes} phút" }
                                }
                                if att.overtime_minutes > 0 {
                                    span { class: "ent-pill ok", "Tăng ca {att.overtime_minutes} phút" }
                                }
                            }
                        } else {
                            p { class: "ent-muted", "Bạn chưa bấm giờ vào ca hôm nay. Giờ được ghi nhận theo đồng hồ máy chủ." }
                        }
                        div { class: "ent-inline-bar",
                            select { value: "{work_mode}", onchange: move |e| work_mode.set(e.value()),
                                option { value: "onsite", "Tại văn phòng" }
                                option { value: "remote", "Từ xa" }
                                option { value: "hybrid", "Linh hoạt" }
                            }
                            input {
                                r#type: "text",
                                placeholder: "Ghi chú ca làm (tùy chọn)",
                                value: "{check_note}",
                                oninput: move |e| check_note.set(e.value()),
                            }
                            button { r#type: "button", class: "btn-primary", disabled: busy(), onclick: move |_| do_check.call(true), "Vào ca" }
                            button { r#type: "button", class: "btn-secondary", disabled: busy(), onclick: move |_| do_check.call(false), "Tan ca" }
                        }
                    }
                }
            } else {
                div { class: "ent-empty", "Đang đồng bộ số liệu không gian doanh nghiệp" }
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
                    Err(e) => err.set(Some(format!("Không xem được báo cáo: {}", e.code))),
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
                h3 { class: "ent-section-title", BarChartIcon { size: 18 } " Báo cáo vận hành & Kiểm soát" }
                if can_export {
                    div { class: "ent-btn-group",
                        a { class: "btn-secondary", href: "{export_reports_csv_url(&server_id)}", download: "enterprise-report.csv", DownloadIcon { size: 15 } " Xuất báo cáo CSV" }
                        a { class: "btn-secondary", href: "{export_audit_csv_url(&server_id)}", download: "enterprise-audit.csv", DownloadIcon { size: 15 } " Nhật ký CSV" }
                        a { class: "btn-secondary", href: "{export_all_json_url(&server_id)}", download: "enterprise-export.json", DownloadIcon { size: 15 } " Toàn bộ JSON" }
                    }
                }
            }
            if let Some(m) = err() {
                div { class: "status-badge error", "{m}" }
            }
            if let Some(r) = rep() {
                div { class: "ent-two-col",
                    div { class: "ent-card",
                        h4 { "Tổng hợp chỉ số thực tế" }
                        div { class: "ent-stat-list",
                            div { class: "ent-stat-row", span { "Nhân sự đang hoạt động" } strong { "{r[\"active_members\"].as_i64().unwrap_or(0)}" } }
                            div { class: "ent-stat-row", span { "Công việc quá hạn" } strong { "{r[\"overdue_tasks\"].as_i64().unwrap_or(0)}" } }
                            div { class: "ent-stat-row", span { "Dung lượng tài liệu" } strong { "{r[\"total_doc_bytes\"].as_i64().unwrap_or(0) / 1024} KB" } }
                        }
                        if let Some(arr) = r["tasks_by_status"].as_array() {
                            div { class: "ent-sub-label", "Công việc theo trạng thái" }
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
                        h4 { "Khối lượng theo phòng ban" }
                        if let Some(depts) = r["department_workload"].as_array() {
                            if depts.is_empty() {
                                p { class: "ent-muted", "Chưa có phòng ban nào được thiết lập." }
                            } else {
                                div { class: "ent-stat-list",
                                    for d in depts {
                                        div { class: "ent-stat-row",
                                            span { "{d[\"name\"].as_str().unwrap_or(\"\")}" }
                                            span { class: "ent-muted", "{d[\"employees\"].as_i64().unwrap_or(0)} nhân sự · {d[\"open_tasks\"].as_i64().unwrap_or(0)} việc mở" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if can_audit {
                div { class: "ent-card",
                    div { class: "ent-card-head",
                        h4 { ShieldIcon { size: 16 } " Nhật ký thao tác doanh nghiệp" }
                        div { class: "ent-inline-bar",
                            input { r#type: "text", placeholder: "Lọc theo hành động hoặc đối tượng", value: "{audit_q}", oninput: move |e| audit_q.set(e.value()) }
                            button { r#type: "button", class: "btn-secondary", onclick: reload_audits, RefreshIcon { size: 15 } " Lọc" }
                        }
                    }
                    div { class: "ent-table-wrap",
                        table { class: "ent-table",
                            thead {
                                tr { th { "Thời gian" } th { "Người thực hiện" } th { "Hành động" } th { "Đối tượng" } }
                            }
                            tbody {
                                for a in audits() {
                                    tr { key: "{a.id}",
                                        td { "{a.created_at.chars().take(19).collect::<String>()}" }
                                        td { "{a.actor_name.as_deref().unwrap_or(\"Hệ thống\")}" }
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
}
