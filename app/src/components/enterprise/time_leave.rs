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
                        msg.set(Some((false, if is_in { "Recorded shift time".into() } else { "The end of shift time has been recorded".into() })));
                        r();
                    }
                    Err(e) => msg.set(Some((true, format!("Unable to clock time: {}", e.code)))),
                }
            });
        }
    };

    rsx! {
        div { class: "ent-section",
            div { class: "ent-toolbar",
                div { class: "ent-inline-bar",
                    select { value: "{work_mode}", onchange: move |e| work_mode.set(e.value()),
                        option { value: "onsite", "At the office" }
                        option { value: "remote", "Work remotely" }
                        option { value: "hybrid", "Flexible" }
                    }
                    input { r#type: "text", placeholder: "Shift notes", value: "{note}", oninput: move |e| note.set(e.value()) }
                    button { r#type: "button", class: "btn-primary", onclick: { let c = check.clone(); move |_| c(true) }, "Click on shift" }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| check(false), "Click to end shift" }
                }
                div { class: "ent-btn-group",
                    button {
                        r#type: "button",
                        class: "btn-secondary",
                        onclick: move |_| show_adj.set(!show_adj()),
                        PlusIcon { size: 15 } " Adjust working hours"
                    }
                    if can_export {
                        a { class: "btn-secondary", href: "{export_attendance_csv_url(&server_id)}", download: "attendance.csv", DownloadIcon { size: 15 } " Export CSV" }
                    }
                }
            }
            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{text}" }
            }

            if show_adj() {
                div { class: "ent-card",
                    div { class: "ent-form-grid",
                        label { class: "form-field", span { "Working day" }
                            input { r#type: "date", value: "{adj_date}", oninput: move |e| adj_date.set(e.value()) }
                        }
                        label { class: "form-field", span { "Time for proposed shift" }
                            input { r#type: "time", value: "{adj_time}", oninput: move |e| adj_time.set(e.value()) }
                        }
                        label { class: "form-field", span { "Reason for adjustment" }
                            input { r#type: "text", placeholder: "For example: Forgot to click on shift during a client meeting", value: "{adj_reason}", oninput: move |e| adj_reason.set(e.value()) }
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
                                            msg.set(Some((false, "The public adjustment proposal has been sent".into())));
                                            r();
                                        }
                                        Err(e) => msg.set(Some((true, format!("Error sending proposal: {}", e.code)))),
                                    }
                                });
                            }
                        }, "Submit corrections" }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| show_adj.set(false), "Close" }
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
                                            }, "Browse" }
                                            button { r#type: "button", class: "btn-secondary", onclick: move |_| {
                                                let (s, id, r) = (sid2.clone(), id2.clone(), r2.clone());
                                                spawn(async move { if enterprise::decide_adjustment(&s, &id, "rejected", "").await.is_ok() { r(); } });
                                            }, "Refuse" }
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
                    thead { tr { th { "Date" } th { "Human resources" } th { "Clock in" } th { "Clock out" } th { "Form" } th { "Status" } } }
                    tbody {
                        for row in rows() {
                            tr { key: "{row.id}",
                                td { "{row.work_date}" }
                                td { "{row.username}" }
                                td { "{row.check_in_at.as_deref().and_then(|s| s.get(11..19)).unwrap_or(\"-\")}" }
                                td { "{row.check_out_at.as_deref().and_then(|s| s.get(11..19)).unwrap_or(\"-\")}" }
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
                    span { class: "ent-pill ok", "{data().requests.len()} leave application" }
                }
                button {
                    r#type: "button",
                    class: "btn-primary",
                    onclick: move |_| show_create.set(!show_create()),
                    PlusIcon { size: 15 } " Create a leave application"
                }
            }
            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{text}" }
            }
            div { class: "ent-kpi-grid",
                for b in data().balances {
                    div { key: "{b.leave_type}-{b.year}", class: "ent-kpi-card",
                        div { class: "ent-kpi-head", span { "{enterprise::vn_leave_type(&b.leave_type)} ({b.year})" } }
                        div { class: "ent-kpi-val", "{b.total_days - b.used_days} days left" }
                        div { class: "ent-kpi-sub", "Used {b.used_days}/{b.total_days} days" }
                    }
                }
            }

            if show_create() {
                div { class: "ent-card",
                    div { class: "ent-form-grid",
                        label { class: "form-field", span { "Type of leave" }
                            select { value: "{leave_type}", onchange: move |e| leave_type.set(e.value()),
                                option { value: "annual", "Permit year" }
                                option { value: "sick", "Sick leave" }
                                option { value: "unpaid", "Leave without pay" }
                            }
                        }
                        label { class: "form-field", span { "From date" }
                            input { r#type: "date", value: "{start_date}", oninput: move |e| start_date.set(e.value()) }
                        }
                        label { class: "form-field", span { "Come day" }
                            input { r#type: "date", value: "{end_date}", oninput: move |e| end_date.set(e.value()) }
                        }
                        label { class: "form-field", span { "Session frame" }
                            select { value: "{half_day}", onchange: move |e| half_day.set(e.value()),
                                option { value: "full", "All day" }
                                option { value: "morning", "Half morning" }
                                option { value: "afternoon", "Half afternoon" }
                            }
                        }
                        label { class: "form-field", span { "Reason for leave" }
                            input { r#type: "text", placeholder: "Enter the reason for leaving", value: "{reason}", oninput: move |e| reason.set(e.value()) }
                        }
                    }
                    div { class: "ent-btn-group",
                        button { r#type: "button", class: "btn-primary", onclick: {
                            let sid = server_id.clone();
                            let r = reload.clone();
                            move |_| {
                                if start_date().is_empty() || end_date().is_empty() || reason().trim().is_empty() {
                                    msg.set(Some((true, "Enter the full start date, end date and reason".into())));
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
                                            msg.set(Some((false, "Leave application has been sent".into())));
                                            r();
                                        }
                                        Err(e) => msg.set(Some((true, format!("Application submission failed: {}", e.code)))),
                                    }
                                });
                            }
                        }, "Submit application" }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| show_create.set(false), "Close" }
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
                                        span { class: "ent-pill", "{req.start_date} to {req.end_date} ({req.days_count} days)" }
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
                                                        Err(e) => msg.set(Some((true, format!("Browsing error: {}", e.code)))),
                                                    }
                                                });
                                            }, "Browse" }
                                            button { r#type: "button", class: "btn-secondary", onclick: move |_| {
                                                let (s, id, r) = (s2.clone(), id2.clone(), r2.clone());
                                                spawn(async move { if enterprise::decide_leave(&s, &id, "rejected", "").await.is_ok() { r(); } });
                                            }, "Refuse" }
                                        }
                                        if is_mine {
                                            button { r#type: "button", class: "btn-ghost", onclick: move |_| {
                                                let (s, id, r) = (s3.clone(), id3.clone(), r3.clone());
                                                spawn(async move { if enterprise::decide_leave(&s, &id, "withdrawn", "").await.is_ok() { r(); } });
                                            }, "Withdrawal" }
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
    let mut channels = use_signal(Vec::<crate::models::ChannelItem>::new);
    let mut show_create = use_signal(|| false);
    let mut title = use_signal(String::new);
    let mut date_str = use_signal(String::new);
    let mut start_time = use_signal(|| "09:00".to_string());
    let mut end_time = use_signal(|| "10:00".to_string());
    let mut meeting_type = use_signal(|| "video".to_string());
    let mut voice_ch_id = use_signal(String::new);
    let mut waiting_room = use_signal(|| false);
    let mut minutes_edit = use_signal(|| Option::<(String, String)>::None);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);

    let reload = {
        let sid = server_id.clone();
        move || {
            let sid = sid.clone();
            spawn(async move {
                if let Ok(list) = enterprise::list_meetings(&sid).await { meetings.set(list); }
                if let Ok(chs) = crate::services::api::servers_get_channels(&sid).await { channels.set(chs); }
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
                if let Ok(chs) = crate::services::api::servers_get_channels(&sid).await { channels.set(chs); }
            });
        }));
    }

    rsx! {
        div { class: "ent-section",
            div { class: "ent-toolbar",
                div { class: "ent-chips",
                    span { class: "ent-pill ok", "{meetings().len()} scheduled meeting" }
                }
                button {
                    r#type: "button",
                    class: "btn-primary",
                    onclick: move |_| show_create.set(!show_create()),
                    PlusIcon { size: 15 } " Schedule a meeting"
                }
            }
            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{text}" }
            }
            if show_create() {
                div { class: "ent-card",
                    div { class: "ent-form-grid",
                        label { class: "form-field", span { "Meeting title" }
                            input { r#type: "text", placeholder: "Enter a meeting title", value: "{title}", oninput: move |e| title.set(e.value()) }
                        }
                        label { class: "form-field", span { "Meeting format" }
                            select { value: "{meeting_type}", onchange: move |e| meeting_type.set(e.value()),
                                option { value: "video", "Video" }
                                option { value: "standup", "Quick meeting" }
                                option { value: "review", "Review" }
                                option { value: "all_hands", "Whole company" }
                            }
                        }
                        label { class: "form-field", span { "Meeting date" }
                            input { r#type: "date", value: "{date_str}", oninput: move |e| date_str.set(e.value()) }
                        }
                        label { class: "form-field", span { "Start time" }
                            input { r#type: "time", value: "{start_time}", oninput: move |e| start_time.set(e.value()) }
                        }
                        label { class: "form-field", span { "End time" }
                            input { r#type: "time", value: "{end_time}", oninput: move |e| end_time.set(e.value()) }
                        }
                        label { class: "form-field", span { "Linked voice room" }
                            select { value: "{voice_ch_id}", onchange: move |e| voice_ch_id.set(e.value()),
                                option { value: "", "Auto" }
                                for ch in channels() {
                                    if ch.channel_type == "voice" {
                                        option { key: "{ch.id}", value: "{ch.id}", "{ch.name}" }
                                    }
                                }
                            }
                        }
                        label { style: "display: flex; align-items: center; gap: 6px; padding-top: 18px;",
                            input { r#type: "checkbox", checked: waiting_room(), onchange: move |e| waiting_room.set(e.checked()) }
                            "Turn on browsing lounge"
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
                                let body = json!({
                                    "title": t,
                                    "meeting_type": meeting_type(),
                                    "starts_at": s_iso,
                                    "ends_at": e_iso,
                                    "channel_id": if voice_ch_id().is_empty() { serde_json::Value::Null } else { json!(voice_ch_id()) },
                                    "waiting_room": waiting_room(),
                                });
                                spawn(async move {
                                    match enterprise::create_meeting(&sid, body).await {
                                        Ok(v) => {
                                            title.set(String::new());
                                            show_create.set(false);
                                            let warn = v.get("conflict_warning").and_then(|x| x.as_bool()).unwrap_or(false);
                                            msg.set(Some((false, if warn { "Created meeting schedule (warning about overlapping time frames)".into() } else { "Meeting scheduled".into() })));
                                            r();
                                        }
                                        Err(e) => msg.set(Some((true, format!("Unable to create meeting schedule: {}", e.code)))),
                                    }
                                });
                            }
                        }, "Save meeting schedule" }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| show_create.set(false), "Close" }
                    }
                }
            }

            div { class: "ent-dept-grid",
                for m in meetings() {
                    {
                        let mid_m = m.id.clone();
                        let mid_t = m.id.clone();
                        let title_t = format!("Post-meeting follow-up: {}", m.title);
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
                                    span { class: "ent-pill", "Chair: {m.host_name}" }
                                    span { class: "ent-pill", "Start: {m.starts_at.chars().take(16).collect::<String>()}" }
                                }
                                if !m.minutes.is_empty() {
                                    div { class: "ent-quote", "{m.minutes}" }
                                }
                                div { class: "ent-btn-group",
                                    button { r#type: "button", class: "btn-secondary", onclick: move |_| minutes_edit.set(Some((mid_m.clone(), min_cur.clone()))), "Minutes" }
                                    button { r#type: "button", class: "btn-secondary", onclick: move |_| {
                                        let (sid, tt) = (sid_t.clone(), title_t.clone());
                                        let _ = mid_t;
                                        spawn(async move {
                                            match enterprise::create_task(&sid, json!({ "title": tt, "priority": "medium" })).await {
                                                Ok(_) => msg.set(Some((false, "Converted to work in progress table".into()))),
                                                Err(e) => msg.set(Some((true, format!("Unable to create job: {}", e.code)))),
                                            }
                                        });
                                    }, "Create jobs" }
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
                                                }, "Save minutes" }
                                                button { r#type: "button", class: "btn-ghost", onclick: move |_| minutes_edit.set(None), "Close" }
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
    let mut horizon = use_signal(|| "month".to_string()); // "day" | "week" | "month" | "all"
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
                div { class: "ent-inline-bar",
                    select {
                        class: "cw-dropdown",
                        aria_label: "Date",
                        value: "{horizon}",
                        onchange: move |e| horizon.set(e.value()),
                        option { value: "day", selected: horizon() == "day", "Date" }
                        option { value: "week", selected: horizon() == "week", "Week" }
                        option { value: "month", selected: horizon() == "month", "Month" }
                    }
                    select {
                        class: "cw-dropdown",
                        aria_label: "All",
                        value: "{filter}",
                        onchange: move |e| filter.set(e.value()),
                        option { value: "all", selected: filter() == "all", "All" }
                        option { value: "meetings", selected: filter() == "meetings", "Meeting ({meetings().len()})" }
                        option { value: "tasks", selected: filter() == "tasks", "Job term ({due_tasks.len()})" }
                        option { value: "leave", selected: filter() == "leave", "Leave ({leave().requests.len()})" }
                    }
                }
                button {
                    r#type: "button",
                    class: "btn-primary",
                    onclick: move |_| show_create.set(!show_create()),
                    PlusIcon { size: 15 } " Add calendar events"
                }
            }

            if show_create() {
                div { class: "ent-card",
                    div { class: "ent-form-grid",
                        label { class: "form-field", span { "Event/Meeting name" }
                            input { r#type: "text", placeholder: "Enter the schedule title", value: "{m_title}", oninput: move |e| m_title.set(e.value()) }
                        }
                        label { class: "form-field", span { "Date" }
                            input { r#type: "date", value: "{m_date}", oninput: move |e| m_date.set(e.value()) }
                        }
                        label { class: "form-field", span { "Get started" }
                            input { r#type: "time", value: "{m_start}", oninput: move |e| m_start.set(e.value()) }
                        }
                        label { class: "form-field", span { "End" }
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
                            "Save to calendar"
                        }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| show_create.set(false), "Close" }
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
                                    span { class: "ent-pill ok", "Meeting · {enterprise::vn_meeting_type(&m.meeting_type)}" }
                                    span { class: "ent-pill", "{enterprise::vn_meeting_status(&m.status)}" }
                                }
                                div { class: "ent-muted", "Chair: {m.host_name}" }
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
                                        span { class: "ent-pill", "Work term" }
                                        span { class: "ent-pill", "{enterprise::vn_task_status(&t.status)}" }
                                        span { class: "ent-pill", "{enterprise::vn_priority(&t.priority)}" }
                                    }
                                    div { class: "ent-muted",
                                        if let Some(an) = &t.assignee_name { "In charge: {an}" } else { "Not assigned yet" }
                                        if let Some(pn) = &t.project_name { " · Project: {pn}" }
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
                                    strong { "Leave: {lr.username}" }
                                    span { class: "ent-pill", "{enterprise::vn_leave_type(&lr.leave_type)}" }
                                    span { class: "ent-pill ok", "{enterprise::vn_approval_status(&lr.status)}" }
                                }
                                div { class: "ent-muted", "{lr.start_date} to {lr.end_date} ({lr.days_count} days) · {lr.reason}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
