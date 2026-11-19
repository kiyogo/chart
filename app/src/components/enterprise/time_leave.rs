use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::icons::{DownloadIcon, PlusIcon},
    services::enterprise::{
        self, AdjustmentItem, AttendanceItem, EnterpriseConfig, LeaveData, MeetingItem, TaskItem,
        export_attendance_csv_url,
    },
};

#[component]
pub fn AttendancePanel(server_id: String, cfg: EnterpriseConfig, refresh_tick: u64) -> Element {
    let mut rows = use_signal(Vec::<AttendanceItem>::new);
    let mut adjustments = use_signal(Vec::<AdjustmentItem>::new);
    let mut show_adj = use_signal(|| false);
    let mut work_mode = use_signal(|| "onsite".to_string());
    let mut note = use_signal(String::new);
    let mut adj_date = use_signal(String::new);
    let mut adj_time = use_signal(|| "08:30".to_string());
    let mut adj_reason = use_signal(String::new);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);

    let can_manage = cfg.has(enterprise::ent_perm::MANAGE_ATTENDANCE);
    let can_export = cfg.has(enterprise::ent_perm::EXPORT_DATA);

    let reload = {
        let sid = server_id.clone();
        move || {
            let sid = sid.clone();
            spawn(async move {
                if let Ok(a) = enterprise::list_attendance(&sid).await { rows.set(a); }
                if let Ok(j) = enterprise::list_adjustments(&sid).await { adjustments.set(j); }
            });
        }
    };

    {
        let sid = server_id.clone();
        use_effect(use_reactive!(|(sid, refresh_tick)| {
            let sid = sid.clone();
            let _ = refresh_tick;
            spawn(async move {
                if let Ok(a) = enterprise::list_attendance(&sid).await { rows.set(a); }
                if let Ok(j) = enterprise::list_adjustments(&sid).await { adjustments.set(j); }
            });
        }));
    }

    let check = {
        let sid = server_id.clone();
        let r = reload.clone();
        move |is_in: bool| {
            let (sid, r, wm, nt) = (sid.clone(), r.clone(), work_mode(), note());
            spawn(async move {
                let res = if is_in {
                    enterprise::check_in(&sid, &wm, &nt).await
                } else {
                    enterprise::check_out(&sid, &nt).await
                };
                match res {
                    Ok(_) => {
                        note.set(String::new());
                        msg.set(Some((false, if is_in { "Đã ghi nhận giờ vào ca".into() } else { "Đã ghi nhận giờ tan ca".into() })));
                        r();
                    }
                    Err(e) => msg.set(Some((true, format!("Chưa chấm công được: {}", e.code)))),
                }
            });
        }
    };

    rsx! {
        div { class: "ent-section",
            div { class: "ent-toolbar",
                div { class: "ent-inline-bar",
                    select { value: "{work_mode}", onchange: move |e| work_mode.set(e.value()),
                        option { value: "onsite", "Tại văn phòng" }
                        option { value: "remote", "Làm từ xa" }
                        option { value: "hybrid", "Linh hoạt" }
                    }
                    input { r#type: "text", placeholder: "Ghi chú ca làm", value: "{note}", oninput: move |e| note.set(e.value()) }
                    button { r#type: "button", class: "btn-primary", onclick: { let c = check.clone(); move |_| c(true) }, "Bấm vào ca" }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| check(false), "Bấm tan ca" }
                }
                div { class: "ent-btn-group",
                    button {
                        r#type: "button",
                        class: "btn-secondary",
                        onclick: move |_| show_adj.set(!show_adj()),
                        PlusIcon { size: 15 } " Điều chỉnh giờ công"
                    }
                    if can_export {
                        a { class: "btn-secondary", href: "{export_attendance_csv_url(&server_id)}", download: "attendance.csv", DownloadIcon { size: 15 } " Xuất CSV" }
                    }
                }
            }
            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{text}" }
            }

            if show_adj() {
                div { class: "ent-card",
                    div { class: "ent-form-grid",
                        label { class: "form-field", span { "Ngày làm việc" }
                            input { r#type: "date", value: "{adj_date}", oninput: move |e| adj_date.set(e.value()) }
                        }
                        label { class: "form-field", span { "Giờ vào ca đề xuất" }
                            input { r#type: "time", value: "{adj_time}", oninput: move |e| adj_time.set(e.value()) }
                        }
                        label { class: "form-field", span { "Lý do điều chỉnh" }
                            input { r#type: "text", placeholder: "Ví dụ: Quên bấm vào ca khi họp khách hàng", value: "{adj_reason}", oninput: move |e| adj_reason.set(e.value()) }
                        }
                    }
                    div { class: "ent-btn-group",
                        button { r#type: "button", class: "btn-primary", onclick: {
                            let sid = server_id.clone();
                            let r = reload.clone();
                            move |_| {
                                let (d, t, rs) = (adj_date(), adj_time(), adj_reason().trim().to_string());
                                if d.is_empty() || rs.len() < 2 { return; }
                                let iso = format!("{d}T{t}:00+07:00");
                                let (sid, r) = (sid.clone(), r.clone());
                                spawn(async move {
                                    match enterprise::request_adjustment(&sid, &d, Some(&iso), None, &rs).await {
                                        Ok(_) => {
                                            adj_reason.set(String::new());
                                            show_adj.set(false);
                                            msg.set(Some((false, "Đã gửi đề xuất điều chỉnh công".into())));
                                            r();
                                        }
                                        Err(e) => msg.set(Some((true, format!("Lỗi gửi đề xuất: {}", e.code)))),
                                    }
                                });
                            }
                        }, "Gửi điều chỉnh" }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| show_adj.set(false), "Đóng" }
                    }
                }
            }

            if !adjustments().is_empty() {
                div { class: "ent-list",
                    for a in adjustments() {
                        {
                            let (sid1, id1, r1) = (server_id.clone(), a.id.clone(), reload.clone());
                            let (sid2, id2, r2) = (server_id.clone(), a.id.clone(), reload.clone());
                            rsx! {
                                div { key: "{a.id}", class: "ent-row-card",
                                    div { class: "grow",
                                        div { class: "ent-row-title", strong { "{a.username}" } span { class: "ent-pill", "{a.work_date}" } span { class: "ent-pill", "{enterprise::vn_approval_status(&a.status)}" } }
                                        div { class: "ent-muted", "{a.reason}" }
                                    }
                                    if can_manage && a.status == "pending" {
                                        div { class: "ent-btn-group",
                                            button { r#type: "button", class: "btn-primary", onclick: move |_| {
                                                let (s, id, r) = (sid1.clone(), id1.clone(), r1.clone());
                                                spawn(async move { if enterprise::decide_adjustment(&s, &id, "approved", "").await.is_ok() { r(); } });
                                            }, "Duyệt" }
                                            button { r#type: "button", class: "btn-secondary", onclick: move |_| {
                                                let (s, id, r) = (sid2.clone(), id2.clone(), r2.clone());
                                                spawn(async move { if enterprise::decide_adjustment(&s, &id, "rejected", "").await.is_ok() { r(); } });
                                            }, "Từ chối" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "ent-table-wrap",
                table { class: "ent-table",
                    thead { tr { th { "Ngày" } th { "Nhân sự" } th { "Vào ca" } th { "Tan ca" } th { "Hình thức" } th { "Trạng thái" } } }
                    tbody {
                        for row in rows() {
                            tr { key: "{row.id}",
                                td { "{row.work_date}" }
                                td { "{row.username}" }
                                td { "{row.check_in_at.as_deref().map(|s| &s[11..19]).unwrap_or(\"-\")}" }
                                td { "{row.check_out_at.as_deref().map(|s| &s[11..19]).unwrap_or(\"-\")}" }
                                td { "{enterprise::vn_work_mode(&row.work_mode)}" }
                                td { span { class: "ent-pill ok", "{enterprise::vn_att_status(&row.status)}" } }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn LeavePanel(server_id: String, my_id: String, cfg: EnterpriseConfig, refresh_tick: u64) -> Element {
    let mut data = use_signal(LeaveData::default);
    let mut show_create = use_signal(|| false);
    let mut leave_type = use_signal(|| "annual".to_string());
    let mut start_date = use_signal(String::new);
    let mut end_date = use_signal(String::new);
    let mut half_day = use_signal(|| "full".to_string());
    let mut reason = use_signal(String::new);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);

    let can_approve = cfg.has(enterprise::ent_perm::APPROVE_LEAVE);

    let reload = {
        let sid = server_id.clone();
        move || {
            let sid = sid.clone();
            spawn(async move {
                if let Ok(d) = enterprise::list_leave(&sid).await { data.set(d); }
            });
        }
    };

    {
        let sid = server_id.clone();
        use_effect(use_reactive!(|(sid, refresh_tick)| {
            let sid = sid.clone();
            let _ = refresh_tick;
            spawn(async move {
                if let Ok(d) = enterprise::list_leave(&sid).await { data.set(d); }
            });
        }));
    }

    rsx! {
        div { class: "ent-section",
            div { class: "ent-toolbar",
                div { class: "ent-chips",
                    span { class: "ent-pill ok", "{data().requests.len()} đơn nghỉ phép" }
                }
                button {
                    r#type: "button",
                    class: "btn-primary",
                    onclick: move |_| show_create.set(!show_create()),
                    PlusIcon { size: 15 } " Tạo đơn xin nghỉ"
                }
            }
            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{text}" }
            }
            div { class: "ent-kpi-grid",
                for b in data().balances {
                    div { key: "{b.leave_type}-{b.year}", class: "ent-kpi-card",
                        div { class: "ent-kpi-head", span { "{enterprise::vn_leave_type(&b.leave_type)} ({b.year})" } }
                        div { class: "ent-kpi-val", "{b.total_days - b.used_days} ngày còn" }
                        div { class: "ent-kpi-sub", "Đã dùng {b.used_days}/{b.total_days} ngày" }
                    }
                }
            }

            if show_create() {
                div { class: "ent-card",
                    div { class: "ent-form-grid",
                        label { class: "form-field", span { "Loại nghỉ phép" }
                            select { value: "{leave_type}", onchange: move |e| leave_type.set(e.value()),
                                option { value: "annual", "Phép năm" }
                                option { value: "sick", "Nghỉ ốm" }
                                option { value: "unpaid", "Nghỉ không lương" }
                            }
                        }
                        label { class: "form-field", span { "Từ ngày" }
                            input { r#type: "date", value: "{start_date}", oninput: move |e| start_date.set(e.value()) }
                        }
                        label { class: "form-field", span { "Đến ngày" }
                            input { r#type: "date", value: "{end_date}", oninput: move |e| end_date.set(e.value()) }
                        }
                        label { class: "form-field", span { "Khung buổi" }
                            select { value: "{half_day}", onchange: move |e| half_day.set(e.value()),
                                option { value: "full", "Cả ngày" }
                                option { value: "morning", "Nửa buổi sáng" }
                                option { value: "afternoon", "Nửa buổi chiều" }
                            }
                        }
                        label { class: "form-field", span { "Lý do xin nghỉ" }
                            input { r#type: "text", placeholder: "Nhập lý do xin nghỉ", value: "{reason}", oninput: move |e| reason.set(e.value()) }
                        }
                    }
                    div { class: "ent-btn-group",
                        button { r#type: "button", class: "btn-primary", onclick: {
                            let sid = server_id.clone();
                            let r = reload.clone();
                            move |_| {
                                if start_date().is_empty() || end_date().is_empty() || reason().trim().is_empty() {
                                    msg.set(Some((true, "Nhập đầy đủ ngày bắt đầu, kết thúc và lý do".into())));
                                    return;
                                }
                                let (sid, r) = (sid.clone(), r.clone());
                                let body = json!({
                                    "leave_type": leave_type(),
                                    "start_date": start_date(),
                                    "end_date": end_date(),
                                    "half_day": half_day(),
                                    "reason": reason().trim(),
                                });
                                spawn(async move {
                                    match enterprise::create_leave(&sid, body).await {
                                        Ok(_) => {
                                            reason.set(String::new());
                                            show_create.set(false);
                                            msg.set(Some((false, "Đã gửi đơn xin nghỉ phép".into())));
                                            r();
                                        }
                                        Err(e) => msg.set(Some((true, format!("Gửi đơn thất bại: {}", e.code)))),
                                    }
                                });
                            }
                        }, "Gửi đơn" }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| show_create.set(false), "Đóng" }
                    }
                }
            }

            div { class: "ent-list",
                for req in data().requests {
                    {
                        let is_mine = req.user_id == my_id;
                        let (s1, id1, r1) = (server_id.clone(), req.id.clone(), reload.clone());
                        let (s2, id2, r2) = (server_id.clone(), req.id.clone(), reload.clone());
                        let (s3, id3, r3) = (server_id.clone(), req.id.clone(), reload.clone());
                        rsx! {
                            div { key: "{req.id}", class: "ent-row-card",
                                div { class: "grow",
                                    div { class: "ent-row-title",
                                        strong { "{req.username}" }
                                        span { class: "ent-pill", "{enterprise::vn_leave_type(&req.leave_type)}" }
                                        span { class: "ent-pill", "{req.start_date} đến {req.end_date} ({req.days_count} ngày)" }
                                        span { class: "ent-pill ok", "{enterprise::vn_approval_status(&req.status)}" }
                                    }
                                    div { class: "ent-muted", "{req.reason}" }
                                }
                                if req.status == "pending" {
                                    div { class: "ent-btn-group",
                                        if can_approve && (!is_mine || cfg.is_owner) {
                                            button { r#type: "button", class: "btn-primary", onclick: move |_| {
                                                let (s, id, r) = (s1.clone(), id1.clone(), r1.clone());
                                                spawn(async move {
                                                    match enterprise::decide_leave(&s, &id, "approved", "").await {
                                                        Ok(_) => r(),
                                                        Err(e) => msg.set(Some((true, format!("Lỗi duyệt: {}", e.code)))),
                                                    }
                                                });
                                            }, "Duyệt" }
                                            button { r#type: "button", class: "btn-secondary", onclick: move |_| {
                                                let (s, id, r) = (s2.clone(), id2.clone(), r2.clone());
                                                spawn(async move { if enterprise::decide_leave(&s, &id, "rejected", "").await.is_ok() { r(); } });
                                            }, "Từ chối" }
                                        }
                                        if is_mine {
                                            button { r#type: "button", class: "btn-ghost", onclick: move |_| {
                                                let (s, id, r) = (s3.clone(), id3.clone(), r3.clone());
                                                spawn(async move { if enterprise::decide_leave(&s, &id, "withdrawn", "").await.is_ok() { r(); } });
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
pub fn MeetingsPanel(server_id: String, refresh_tick: u64) -> Element {
    let mut meetings = use_signal(Vec::<MeetingItem>::new);
    let mut show_create = use_signal(|| false);
    let mut title = use_signal(String::new);
    let mut date_str = use_signal(String::new);
    let mut start_time = use_signal(|| "09:00".to_string());
    let mut end_time = use_signal(|| "10:00".to_string());
    let mut meeting_type = use_signal(|| "video".to_string());
    let mut minutes_edit = use_signal(|| Option::<(String, String)>::None);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);

    let reload = {
        let sid = server_id.clone();
        move || {
            let sid = sid.clone();
            spawn(async move {
                if let Ok(list) = enterprise::list_meetings(&sid).await { meetings.set(list); }
            });
        }
    };

    {
        let sid = server_id.clone();
        use_effect(use_reactive!(|(sid, refresh_tick)| {
            let sid = sid.clone();
            let _ = refresh_tick;
            spawn(async move {
                if let Ok(list) = enterprise::list_meetings(&sid).await { meetings.set(list); }
            });
        }));
    }

    rsx! {
        div { class: "ent-section",
            div { class: "ent-toolbar",
                div { class: "ent-chips",
                    span { class: "ent-pill ok", "{meetings().len()} cuộc họp đã lên lịch" }
                }
                button {
                    r#type: "button",
                    class: "btn-primary",
                    onclick: move |_| show_create.set(!show_create()),
                    PlusIcon { size: 15 } " Đặt lịch họp"
                }
            }
            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{text}" }
            }
            if show_create() {
                div { class: "ent-card",
                    div { class: "ent-form-grid",
                        label { class: "form-field", span { "Tiêu đề cuộc họp" }
                            input { r#type: "text", placeholder: "Nhập tiêu đề cuộc họp", value: "{title}", oninput: move |e| title.set(e.value()) }
                        }
                        label { class: "form-field", span { "Hình thức họp" }
                            select { value: "{meeting_type}", onchange: move |e| meeting_type.set(e.value()),
                                option { value: "video", "Video" }
                                option { value: "standup", "Họp nhanh" }
                                option { value: "review", "Đánh giá" }
                                option { value: "all_hands", "Toàn công ty" }
                            }
                        }
                        label { class: "form-field", span { "Ngày họp" }
                            input { r#type: "date", value: "{date_str}", oninput: move |e| date_str.set(e.value()) }
                        }
                        label { class: "form-field", span { "Giờ bắt đầu" }
                            input { r#type: "time", value: "{start_time}", oninput: move |e| start_time.set(e.value()) }
                        }
                        label { class: "form-field", span { "Giờ kết thúc" }
                            input { r#type: "time", value: "{end_time}", oninput: move |e| end_time.set(e.value()) }
                        }
                    }
                    div { class: "ent-btn-group",
                        button { r#type: "button", class: "btn-primary", onclick: {
                            let sid = server_id.clone();
                            let r = reload.clone();
                            move |_| {
                                let (t, d) = (title().trim().to_string(), date_str());
                                if t.is_empty() || d.is_empty() { return; }
                                let s_iso = format!("{d}T{}:00+07:00", start_time());
                                let e_iso = format!("{d}T{}:00+07:00", end_time());
                                let (sid, r) = (sid.clone(), r.clone());
                                let body = json!({ "title": t, "meeting_type": meeting_type(), "starts_at": s_iso, "ends_at": e_iso });
                                spawn(async move {
                                    match enterprise::create_meeting(&sid, body).await {
                                        Ok(v) => {
                                            title.set(String::new());
                                            show_create.set(false);
                                            let warn = v.get("conflict_warning").and_then(|x| x.as_bool()).unwrap_or(false);
                                            msg.set(Some((false, if warn { "Đã tạo lịch họp (cảnh báo trùng khung giờ)".into() } else { "Đã đặt lịch cuộc họp".into() })));
                                            r();
                                        }
                                        Err(e) => msg.set(Some((true, format!("Không tạo được lịch họp: {}", e.code)))),
                                    }
                                });
                            }
                        }, "Lưu lịch họp" }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| show_create.set(false), "Đóng" }
                    }
                }
            }

            div { class: "ent-dept-grid",
                for m in meetings() {
                    {
                        let mid_m = m.id.clone();
                        let mid_t = m.id.clone();
                        let title_t = format!("Theo dõi sau họp: {}", m.title);
                        let min_cur = m.minutes.clone();
                        let sid_m = server_id.clone();
                        let sid_t = server_id.clone();
                        rsx! {
                            div { key: "{m.id}", class: "ent-card",
                                div { class: "ent-card-head",
                                    div { class: "ent-row-title",
                                        strong { "{m.title}" }
                                        span { class: "ent-pill ok", "{enterprise::vn_meeting_type(&m.meeting_type)}" }
                                        span { class: "ent-pill", "{enterprise::vn_meeting_status(&m.status)}" }
                                    }
                                }
                                div { class: "ent-chips",
                                    span { class: "ent-pill", "Chủ trì: {m.host_name}" }
                                    span { class: "ent-pill", "Bắt đầu: {m.starts_at.chars().take(16).collect::<String>()}" }
                                }
                                if !m.minutes.is_empty() {
                                    div { class: "ent-quote", "{m.minutes}" }
                                }
                                div { class: "ent-btn-group",
                                    button { r#type: "button", class: "btn-secondary", onclick: move |_| minutes_edit.set(Some((mid_m.clone(), min_cur.clone()))), "Biên bản" }
                                    button { r#type: "button", class: "btn-secondary", onclick: move |_| {
                                        let (sid, tt) = (sid_t.clone(), title_t.clone());
                                        let _ = mid_t;
                                        spawn(async move {
                                            match enterprise::create_task(&sid, json!({ "title": tt, "priority": "medium" })).await {
                                                Ok(_) => msg.set(Some((false, "Đã chuyển thành công việc trong bảng tiến độ".into()))),
                                                Err(e) => msg.set(Some((true, format!("Không tạo được việc: {}", e.code)))),
                                            }
                                        });
                                    }, "Tạo công việc" }
                                }
                                if let Some((eid, txt)) = minutes_edit() {
                                    if eid == m.id {
                                        div { class: "ent-sub-drawer",
                                            textarea { rows: 3, value: "{txt}", oninput: move |e| minutes_edit.set(Some((eid.clone(), e.value()))) }
                                            div { class: "ent-btn-group", style: "margin-top: 6px;",
                                                button { r#type: "button", class: "btn-primary", onclick: {
                                                    let sid = sid_m.clone();
                                                    let mid = m.id.clone();
                                                    let r = reload.clone();
                                                    move |_| {
                                                        let val = minutes_edit().map(|x| x.1).unwrap_or_default();
                                                        let (sid, mid, r) = (sid.clone(), mid.clone(), r.clone());
                                                        spawn(async move {
                                                            if enterprise::update_meeting(&sid, &mid, json!({ "minutes": val })).await.is_ok() {
                                                                minutes_edit.set(None);
                                                                r();
                                                            }
                                                        });
                                                    }
                                                }, "Lưu biên bản" }
                                                button { r#type: "button", class: "btn-ghost", onclick: move |_| minutes_edit.set(None), "Đóng" }
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
}

#[component]
pub fn CalendarPanel(server_id: String, refresh_tick: u64) -> Element {
    let mut meetings = use_signal(Vec::<MeetingItem>::new);
    let mut tasks = use_signal(Vec::<TaskItem>::new);
    let mut leave = use_signal(LeaveData::default);
    let mut filter = use_signal(|| "all".to_string()); // "all" | "meetings" | "tasks" | "leave"
    let mut show_create = use_signal(|| false);
    let mut m_title = use_signal(String::new);
    let mut m_date = use_signal(|| "2026-11-20".to_string());
    let mut m_start = use_signal(|| "09:00".to_string());
    let mut m_end = use_signal(|| "10:00".to_string());

    let reload = {
        let sid = server_id.clone();
        move || {
            let sid = sid.clone();
            spawn(async move {
                if let Ok(m) = enterprise::list_meetings(&sid).await { meetings.set(m); }
                if let Ok(t) = enterprise::list_tasks(&sid, "all", "").await { tasks.set(t); }
                if let Ok(l) = enterprise::list_leave(&sid).await { leave.set(l); }
            });
        }
    };

    {
        let sid = server_id.clone();
        use_effect(use_reactive!(|(sid, refresh_tick)| {
            let sid = sid.clone();
            let _ = refresh_tick;
            spawn(async move {
                if let Ok(m) = enterprise::list_meetings(&sid).await { meetings.set(m); }
                if let Ok(t) = enterprise::list_tasks(&sid, "all", "").await { tasks.set(t); }
                if let Ok(l) = enterprise::list_leave(&sid).await { leave.set(l); }
            });
        }));
    }

    let show_m = filter() == "all" || filter() == "meetings";
    let show_t = filter() == "all" || filter() == "tasks";
    let show_l = filter() == "all" || filter() == "leave";

    let due_tasks: Vec<TaskItem> = tasks().into_iter().filter(|t| t.due_date.is_some()).collect();

    rsx! {
        div { class: "ent-section",
            div { class: "ent-toolbar",
                div { class: "ent-segmented",
                    button { r#type: "button", class: if filter() == "all" { "filter-chip active" } else { "filter-chip" }, onclick: move |_| filter.set("all".into()), "Tất cả lịch trình" }
                    button { r#type: "button", class: if filter() == "meetings" { "filter-chip active" } else { "filter-chip" }, onclick: move |_| filter.set("meetings".into()), "Cuộc họp ({meetings().len()})" }
                    button { r#type: "button", class: if filter() == "tasks" { "filter-chip active" } else { "filter-chip" }, onclick: move |_| filter.set("tasks".into()), "Hạn công việc ({due_tasks.len()})" }
                    button { r#type: "button", class: if filter() == "leave" { "filter-chip active" } else { "filter-chip" }, onclick: move |_| filter.set("leave".into()), "Lịch nghỉ phép ({leave().requests.len()})" }
                }
                button {
                    r#type: "button",
                    class: "btn-primary",
                    onclick: move |_| show_create.set(!show_create()),
                    PlusIcon { size: 15 } " Thêm sự kiện lịch"
                }
            }

            if show_create() {
                div { class: "ent-card",
                    div { class: "ent-form-grid",
                        label { class: "form-field", span { "Tên sự kiện / Cuộc họp" }
                            input { r#type: "text", placeholder: "Nhập tiêu đề lịch làm việc", value: "{m_title}", oninput: move |e| m_title.set(e.value()) }
                        }
                        label { class: "form-field", span { "Ngày" }
                            input { r#type: "date", value: "{m_date}", oninput: move |e| m_date.set(e.value()) }
                        }
                        label { class: "form-field", span { "Bắt đầu" }
                            input { r#type: "time", value: "{m_start}", oninput: move |e| m_start.set(e.value()) }
                        }
                        label { class: "form-field", span { "Kết thúc" }
                            input { r#type: "time", value: "{m_end}", oninput: move |e| m_end.set(e.value()) }
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
                                    let t = m_title().trim().to_string();
                                    let d = m_date();
                                    if t.is_empty() || d.is_empty() { return; }
                                    let s_iso = format!("{d}T{}:00+07:00", m_start());
                                    let e_iso = format!("{d}T{}:00+07:00", m_end());
                                    let (sid, r) = (sid.clone(), r.clone());
                                    spawn(async move {
                                        if enterprise::create_meeting(&sid, json!({ "title": t, "meeting_type": "video", "starts_at": s_iso, "ends_at": e_iso })).await.is_ok() {
                                            m_title.set(String::new());
                                            show_create.set(false);
                                            r();
                                        }
                                    });
                                }
                            },
                            "Lưu vào lịch"
                        }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| show_create.set(false), "Đóng" }
                    }
                }
            }

            div { class: "ent-timeline",
                if show_m {
                    for m in meetings() {
                        div { key: "m-{m.id}", class: "ent-timeline-card",
                            div { class: "ent-timeline-date", "{m.starts_at.chars().take(16).collect::<String>()}" }
                            div { class: "grow",
                                div { class: "ent-row-title",
                                    strong { "{m.title}" }
                                    span { class: "ent-pill ok", "Cuộc họp · {enterprise::vn_meeting_type(&m.meeting_type)}" }
                                    span { class: "ent-pill", "{enterprise::vn_meeting_status(&m.status)}" }
                                }
                                div { class: "ent-muted", "Chủ trì: {m.host_name}" }
                            }
                        }
                    }
                }
                if show_t {
                    for t in due_tasks {
                        if let Some(d) = &t.due_date {
                            div { key: "t-{t.id}", class: "ent-timeline-card",
                                div { class: "ent-timeline-date", "{d}" }
                                div { class: "grow",
                                    div { class: "ent-row-title",
                                        strong { "{t.title}" }
                                        span { class: "ent-pill", "Hạn công việc" }
                                        span { class: "ent-pill", "{enterprise::vn_task_status(&t.status)}" }
                                        span { class: "ent-pill", "{enterprise::vn_priority(&t.priority)}" }
                                    }
                                    div { class: "ent-muted",
                                        if let Some(an) = &t.assignee_name { "Phụ trách: {an}" } else { "Chưa phân công" }
                                        if let Some(pn) = &t.project_name { " · Dự án: {pn}" }
                                    }
                                }
                            }
                        }
                    }
                }
                if show_l {
                    for lr in leave().requests {
                        div { key: "l-{lr.id}", class: "ent-timeline-card",
                            div { class: "ent-timeline-date", "{lr.start_date}" }
                            div { class: "grow",
                                div { class: "ent-row-title",
                                    strong { "Nghỉ phép: {lr.username}" }
                                    span { class: "ent-pill", "{enterprise::vn_leave_type(&lr.leave_type)}" }
                                    span { class: "ent-pill ok", "{enterprise::vn_approval_status(&lr.status)}" }
                                }
                                div { class: "ent-muted", "{lr.start_date} đến {lr.end_date} ({lr.days_count} ngày) · {lr.reason}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
