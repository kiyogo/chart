use std::collections::HashSet;

use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::{
        avatar::Avatar,
        icons::{PlusIcon, TrashIcon},
    },
    models::ServerMemberItem,
    services::{
        api,
        enterprise::{self, DepartmentItem, EmployeeItem, EnterpriseConfig},
    },
};

#[component]
pub fn HrPanel(server_id: String, cfg: EnterpriseConfig, refresh_tick: u64) -> Element {
    let mut employees = use_signal(Vec::<EmployeeItem>::new);
    let mut depts = use_signal(Vec::<DepartmentItem>::new);
    let mut members = use_signal(Vec::<ServerMemberItem>::new);
    let mut q = use_signal(String::new);
    let mut status_filter = use_signal(String::new);
    let mut dept_filter = use_signal(String::new);
    let mut sort_by = use_signal(|| "name".to_string());
    let mut selected = use_signal(HashSet::<String>::new);
    let mut editing_uid = use_signal(|| Option::<String>::None);
    let mut show_import = use_signal(|| false);
    let mut import_text = use_signal(String::new);
    let mut quick_task_uid = use_signal(|| Option::<(String, String)>::None);
    let mut quick_task_title = use_signal(String::new);
    let mut handover_from = use_signal(|| Option::<String>::None);
    let mut handover_to = use_signal(String::new);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);

    let mut f_uid = use_signal(String::new);
    let mut f_name = use_signal(String::new);
    let mut f_code = use_signal(String::new);
    let mut f_kind = use_signal(|| "employee".to_string());
    let mut f_title = use_signal(String::new);
    let mut f_dept = use_signal(String::new);
    let mut f_email = use_signal(String::new);
    let mut f_phone = use_signal(String::new);
    let mut f_mode = use_signal(|| "onsite".to_string());
    let mut f_status = use_signal(|| "active".to_string());
    let mut f_contract = use_signal(String::new);
    let mut f_notes = use_signal(String::new);

    let can_manage = cfg.has(enterprise::ent_perm::MANAGE_HR);

    let load = use_callback({
        let sid = server_id.clone();
        move |_: ()| {
            let sid = sid.clone();
            let qv = q();
            let sv = status_filter();
            let dv = dept_filter();
            let sb = sort_by();
            spawn(async move {
                if let Ok(list) = enterprise::list_employees_sorted(&sid, &qv, &sv, &dv, &sb).await {
                    employees.set(list);
                }
                if let Ok(dl) = enterprise::list_departments(&sid).await {
                    depts.set(dl);
                }
                if let Ok(ml) = api::servers_get_members(&sid).await {
                    members.set(ml);
                }
            });
        }
    });

    {
        let sid = server_id.clone();
        use_effect(use_reactive!(|(sid, refresh_tick)| {
            let sid = sid.clone();
            let _ = refresh_tick;
            let qv = q();
            let sv = status_filter();
            let dv = dept_filter();
            let sb = sort_by();
            spawn(async move {
                if let Ok(list) = enterprise::list_employees_sorted(&sid, &qv, &sv, &dv, &sb).await {
                    employees.set(list);
                }
                if let Ok(dl) = enterprise::list_departments(&sid).await {
                    depts.set(dl);
                }
                if let Ok(ml) = api::servers_get_members(&sid).await {
                    members.set(ml);
                }
            });
        }));
    }

    let mut open_edit = move |e: EmployeeItem| {
        f_uid.set(e.user_id.clone());
        f_name.set(e.full_name.clone());
        f_code.set(e.employee_code.unwrap_or_default());
        f_kind.set(e.member_kind.clone());
        f_title.set(e.job_title.unwrap_or_default());
        f_dept.set(e.department_id.unwrap_or_default());
        f_email.set(e.work_email.unwrap_or_default());
        f_phone.set(e.work_phone.unwrap_or_default());
        f_mode.set(e.work_mode.clone());
        f_status.set(e.employment_status.clone());
        f_contract.set(e.contract_type.unwrap_or_default());
        f_notes.set(e.internal_notes.unwrap_or_default());
        editing_uid.set(Some(e.user_id));
    };

    let save_emp = {
        let sid = server_id.clone();
        move |_| {
            let sid = sid.clone();
            let uid = f_uid();
            if uid.is_empty() || f_name().trim().is_empty() {
                msg.set(Some((true, "Please select a member and enter your full name".into())));
                return;
            }
            let body = json!({
                "user_id": uid,
                "full_name": f_name(),
                "employee_code": f_code(),
                "member_kind": f_kind(),
                "job_title": f_title(),
                "department_id": if f_dept().is_empty() { serde_json::Value::Null } else { json!(f_dept()) },
                "work_email": f_email(),
                "work_phone": f_phone(),
                "work_mode": f_mode(),
                "employment_status": f_status(),
                "contract_type": f_contract(),
                "internal_notes": f_notes(),
            });
            spawn(async move {
                match enterprise::upsert_employee(&sid, body).await {
                    Ok(_) => {
                        msg.set(Some((false, "Personnel records saved".into())));
                        editing_uid.set(None);
                        load.call(());
                    }
                    Err(e) => msg.set(Some((true, format!("Error saving record: {}", e.code)))),
                }
            });
        }
    };

    let bulk_status = use_callback({
        let sid = server_id.clone();
        move |st: &'static str| {
            let ids: Vec<String> = selected().into_iter().collect();
            if ids.is_empty() {
                return;
            }
            let sid = sid.clone();
            spawn(async move {
                if enterprise::bulk_employees(&sid, json!({ "user_ids": ids, "employment_status": st })).await.is_ok() {
                    selected.set(HashSet::new());
                    msg.set(Some((false, "Bulk status updated".into())));
                    load.call(());
                }
            });
        }
    });

    rsx! {
        div { class: "ent-section",
            div { class: "ent-toolbar",
                div { class: "ent-inline-bar",
                    input { r#type: "text", placeholder: "Find name, employee code, title", value: "{q}", oninput: move |e| q.set(e.value()) }
                    select { value: "{status_filter}", onchange: move |e| { status_filter.set(e.value()); load.call(()); },
                        option { value: "", "Every state" }
                        option { value: "active", "Working" }
                        option { value: "onboarding", "On probation" }
                        option { value: "suspended", "Temporarily locked" }
                        option { value: "offboarded", "Retired" }
                    }
                    select { value: "{dept_filter}", onchange: move |e| { dept_filter.set(e.value()); load.call(()); },
                        option { value: "", "Every department" }
                        for d in depts() {
                            option { key: "{d.id}", value: "{d.id}", "{d.name}" }
                        }
                    }
                    select { value: "{sort_by}", onchange: move |e| { sort_by.set(e.value()); load.call(()); },
                        option { value: "name", "Sort: Full name" }
                        option { value: "code", "Sort: Code NV" }
                        option { value: "recent", "Sort: Newest" }
                    }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| load.call(()), "Filter" }
                }
                if can_manage {
                    div { class: "ent-btn-group",
                        button {
                            r#type: "button",
                            class: "btn-secondary",
                            onclick: move |_| show_import.set(!show_import()),
                            "Import list"
                        }
                        button {
                            r#type: "button",
                            class: "btn-primary",
                            onclick: move |_| {
                                if editing_uid().is_some() {
                                    editing_uid.set(None);
                                } else {
                                    f_uid.set(String::new());
                                    f_name.set(String::new());
                                    f_code.set(String::new());
                                    f_title.set(String::new());
                                    editing_uid.set(Some("new".into()));
                                }
                            },
                            PlusIcon { size: 15 } " Add profile"
                        }
                    }
                }
            }

            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{text}" }
            }

            if can_manage && show_import() {
                div { class: "ent-card",
                    label { class: "form-field",
                        span { "Enter the list of personnel (each line: account, full name, title, department code)" }
                        textarea {
                            rows: 3,
                            placeholder: "alice, Nguyen Van A, System Engineer, ENG",
                            value: "{import_text}",
                            oninput: move |e| import_text.set(e.value()),
                        }
                    }
                    div { class: "ent-btn-group",
                        button {
                            r#type: "button",
                            class: "btn-primary",
                            onclick: {
                                let sid = server_id.clone();
                                move |_| {
                                    let rows: Vec<serde_json::Value> = import_text()
                                        .lines()
                                        .filter_map(|line| {
                                            let parts: Vec<&str> = line.split(',').map(str::trim).collect();
                                            let uname = parts.first().copied().unwrap_or("");
                                            if uname.is_empty() {
                                                return None;
                                            }
                                            Some(json!({
                                                "username": uname,
                                                "full_name": parts.get(1).copied().filter(|s| !s.is_empty()),
                                                "job_title": parts.get(2).copied().filter(|s| !s.is_empty()),
                                                "department_code": parts.get(3).copied().filter(|s| !s.is_empty()),
                                            }))
                                        })
                                        .collect();
                                    if rows.is_empty() {
                                        msg.set(Some((true, "Enter at least 1 valid account line".into())));
                                        return;
                                    }
                                    let sid = sid.clone();
                                    spawn(async move {
                                        match enterprise::import_employees(&sid, json!({ "rows": rows })).await {
                                            Ok(v) => {
                                                let imp = v.get("imported").and_then(|x| x.as_i64()).unwrap_or(0);
                                                import_text.set(String::new());
                                                show_import.set(false);
                                                msg.set(Some((false, format!("Imported {imp} personnel records"))));
                                                load.call(());
                                            }
                                            Err(e) => msg.set(Some((true, format!("Error importing list: {}", e.code)))),
                                        }
                                    });
                                }
                            },
                            "Perform import"
                        }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| show_import.set(false), "Close" }
                    }
                }
            }

            if let Some((target_uid, target_name)) = quick_task_uid() {
                div { class: "ent-card ent-inline-bar",
                    span { class: "ent-pill ok", "Assign work to {target_name}" }
                    input {
                        r#type: "text",
                        placeholder: "Enter the title of the job to be assigned",
                        value: "{quick_task_title}",
                        oninput: move |e| quick_task_title.set(e.value()),
                    }
                    button {
                        r#type: "button",
                        class: "btn-primary",
                        onclick: {
                            let sid = server_id.clone();
                            let tuid = target_uid.clone();
                            move |_| {
                                let t = quick_task_title().trim().to_string();
                                if t.is_empty() { return; }
                                let (sid, tuid) = (sid.clone(), tuid.clone());
                                spawn(async move {
                                    match enterprise::create_task(&sid, json!({ "title": t, "assignee_id": tuid, "priority": "medium" })).await {
                                        Ok(_) => {
                                            quick_task_title.set(String::new());
                                            quick_task_uid.set(None);
                                            msg.set(Some((false, "The job was delivered successfully".into())));
                                        }
                                        Err(e) => msg.set(Some((true, format!("Unable to deliver work: {}", e.code)))),
                                    }
                                });
                            }
                        },
                        "Assign now"
                    }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| quick_task_uid.set(None), "Cancel" }
                }
            }

            if can_manage && !selected().is_empty() {
                div { class: "ent-card ent-inline-bar",
                    span { class: "ent-pill", "Selected {selected().len()} personnel" }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| bulk_status.call("active"), "Activate" }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| bulk_status.call("suspended"), "Temporarily locked" }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| selected.set(HashSet::new()), "Deselect" }
                }
            }

            if let Some(mode) = editing_uid() {
                div { class: "ent-card",
                    div { class: "ent-form-grid",
                        if mode == "new" {
                            label { class: "form-field", span { "Server member" }
                                select { value: "{f_uid}", onchange: move |e| {
                                    let id = e.value();
                                    f_uid.set(id.clone());
                                    if f_name().is_empty() {
                                        if let Some(m) = members.read().iter().find(|x| x.user_id == id) {
                                            f_name.set(m.nickname.clone().unwrap_or_else(|| m.username.clone()));
                                        }
                                    }
                                },
                                    option { value: "", "Select member" }
                                    for m in members() {
                                        option { key: "{m.user_id}", value: "{m.user_id}", "{m.username}" }
                                    }
                                }
                            }
                        }
                        label { class: "form-field", span { "Full name" } input { r#type: "text", value: "{f_name}", oninput: move |e| f_name.set(e.value()) } }
                        label { class: "form-field", span { "Employee code" } input { r#type: "text", value: "{f_code}", oninput: move |e| f_code.set(e.value()) } }
                        label { class: "form-field", span { "Title" } input { r#type: "text", value: "{f_title}", oninput: move |e| f_title.set(e.value()) } }
                        label { class: "form-field", span { "Departments" }
                            select { value: "{f_dept}", onchange: move |e| f_dept.set(e.value()),
                                option { value: "", "Not assigned yet" }
                                for d in depts() { option { key: "{d.id}", value: "{d.id}", "{d.name}" } }
                            }
                        }
                        label { class: "form-field", span { "Membership type" }
                            select { value: "{f_kind}", onchange: move |e| f_kind.set(e.value()),
                                option { value: "employee", "Staff" }
                                option { value: "partner", "Partner" }
                                option { value: "guest", "Guest" }
                            }
                        }
                        label { class: "form-field", span { "Form" }
                            select { value: "{f_mode}", onchange: move |e| f_mode.set(e.value()),
                                option { value: "onsite", "At the office" }
                                option { value: "remote", "From afar" }
                                option { value: "hybrid", "Flexible" }
                            }
                        }
                        label { class: "form-field", span { "Status" }
                            select { value: "{f_status}", onchange: move |e| f_status.set(e.value()),
                                option { value: "active", "Working" }
                                option { value: "onboarding", "On probation" }
                                option { value: "suspended", "Temporarily locked" }
                                option { value: "offboarded", "Retired" }
                            }
                        }
                        label { class: "form-field", span { "Work email" } input { r#type: "text", value: "{f_email}", oninput: move |e| f_email.set(e.value()) } }
                        label { class: "form-field", span { "Work phone" } input { r#type: "text", value: "{f_phone}", oninput: move |e| f_phone.set(e.value()) } }
                        label { class: "form-field", span { "Contract type" } input { r#type: "text", value: "{f_contract}", oninput: move |e| f_contract.set(e.value()) } }
                    }
                    label { class: "form-field", span { "Internal notes" } textarea { rows: 2, value: "{f_notes}", oninput: move |e| f_notes.set(e.value()) } }
                    div { class: "ent-btn-group",
                        button { r#type: "button", class: "btn-primary", onclick: save_emp, "Save profile" }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| editing_uid.set(None), "Close" }
                    }
                }
            }

            if let Some(from_id) = handover_from() {
                div { class: "ent-card",
                    div { class: "ent-inline-bar",
                        select { value: "{handover_to}", onchange: move |e| handover_to.set(e.value()),
                            option { value: "", "Select the recipient to hand over work & documents" }
                            for m in members() {
                                if m.user_id != from_id {
                                    option { key: "{m.user_id}", value: "{m.user_id}", "{m.username}" }
                                }
                            }
                        }
                        button {
                            r#type: "button",
                            class: "btn-primary",
                            onclick: {
                                let sid = server_id.clone();
                                let fid = from_id.clone();
                                move |_| {
                                    let to_id = handover_to();
                                    if to_id.is_empty() {
                                        return;
                                    }
                                    let sid = sid.clone();
                                    let fid = fid.clone();
                                    spawn(async move {
                                        match enterprise::handover_employee(&sid, &fid, json!({ "handover_to": to_id, "mark_offboarded": true })).await {
                                            Ok(_) => {
                                                handover_from.set(None);
                                                msg.set(Some((false, "Completed handover and revoked access rights".into())));
                                                load.call(());
                                            }
                                            Err(e) => msg.set(Some((true, format!("Handover failed: {}", e.code)))),
                                        }
                                    });
                                }
                            },
                            "Confirm handover"
                        }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| handover_from.set(None), "Cancel" }
                    }
                }
            }

            div { class: "ent-dept-grid",
                for emp in employees() {
                    {
                        let e_edit = emp.clone();
                        let uid = emp.user_id.clone();
                        let uid_cb = uid.clone();
                        let checked = selected().contains(&uid);
                        rsx! {
                            div { key: "{emp.id}", class: "ent-card",
                                div { class: "ent-row-card", style: "padding: 0; background: transparent;",
                                    if can_manage {
                                        input {
                                            r#type: "checkbox",
                                            checked,
                                            onchange: move |ev| {
                                                selected.with_mut(|s| {
                                                    if ev.checked() { s.insert(uid_cb.clone()); } else { s.remove(&uid_cb); }
                                                });
                                            },
                                        }
                                    }
                                    Avatar { name: emp.full_name.clone(), avatar_url: emp.avatar_url.clone(), size: 38 }
                                    div { class: "grow",
                                        div { class: "ent-row-title",
                                            strong { "{emp.full_name}" }
                                            span { class: "ent-muted", "@{emp.username}" }
                                        }
                                        div { class: "ent-muted",
                                            "{emp.job_title.as_deref().unwrap_or(\"Member\")}"
                                            if let Some(dn) = &emp.department_name { " · {dn}" }
                                        }
                                    }
                                }
                                div { class: "ent-chips",
                                    if let Some(code) = &emp.employee_code {
                                        span { class: "ent-pill", "{code}" }
                                    }
                                    span { class: "ent-pill ok", "{enterprise::vn_emp_status(&emp.employment_status)}" }
                                    span { class: "ent-pill", "{enterprise::vn_work_mode(&emp.work_mode)}" }
                                    if let Some(em) = &emp.work_email {
                                        if !em.is_empty() { span { class: "ent-pill", "{em}" } }
                                    }
                                    if let Some(ph) = &emp.work_phone {
                                        if !ph.is_empty() { span { class: "ent-pill", "{ph}" } }
                                    }
                                }
                                if can_manage {
                                    div { class: "ent-btn-group",
                                        button { r#type: "button", class: "btn-secondary", onclick: move |_| open_edit(e_edit.clone()), "Edit" }
                                        button {
                                            r#type: "button",
                                            class: "btn-secondary",
                                            onclick: {
                                                let u = emp.user_id.clone();
                                                let n = emp.full_name.clone();
                                                move |_| quick_task_uid.set(Some((u.clone(), n.clone())))
                                            },
                                            "Assign work"
                                        }
                                        button { r#type: "button", class: "btn-ghost", onclick: move |_| handover_from.set(Some(emp.user_id.clone())), "Handover" }
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
pub fn DepartmentsPanel(server_id: String, cfg: EnterpriseConfig, refresh_tick: u64) -> Element {
    let mut depts = use_signal(Vec::<DepartmentItem>::new);
    let mut members = use_signal(Vec::<ServerMemberItem>::new);
    let mut channels = use_signal(Vec::<crate::models::ChannelItem>::new);
    let mut view_mode = use_signal(|| "grid".to_string()); // "grid" | "orgchart"
    let mut show_create = use_signal(|| false);
    let mut name = use_signal(String::new);
    let mut code = use_signal(String::new);
    let mut desc = use_signal(String::new);
    let mut branch = use_signal(String::new);
    let mut parent_id = use_signal(String::new);
    let mut mgr_id = use_signal(String::new);
    let mut linked_ch_id = use_signal(String::new);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);

    let can_manage = cfg.has(enterprise::ent_perm::MANAGE_DEPARTMENTS);

    let reload = {
        let sid = server_id.clone();
        move || {
            let sid = sid.clone();
            spawn(async move {
                if let Ok(list) = enterprise::list_departments(&sid).await {
                    depts.set(list);
                }
                if let Ok(ml) = api::servers_get_members(&sid).await {
                    members.set(ml);
                }
                if let Ok(chs) = api::servers_get_channels(&sid).await {
                    channels.set(chs);
                }
            });
        }
    };

    {
        let sid = server_id.clone();
        use_effect(use_reactive!(|(sid, refresh_tick)| {
            let sid = sid.clone();
            let _ = refresh_tick;
            spawn(async move {
                if let Ok(list) = enterprise::list_departments(&sid).await {
                    depts.set(list);
                }
                if let Ok(ml) = api::servers_get_members(&sid).await {
                    members.set(ml);
                }
                if let Ok(chs) = api::servers_get_channels(&sid).await {
                    channels.set(chs);
                }
            });
        }));
    }

    let create = {
        let sid = server_id.clone();
        let r = reload.clone();
        move |_| {
            let n = name().trim().to_string();
            if n.is_empty() {
                return;
            }
            let sid = sid.clone();
            let r = r.clone();
            let body = json!({
                "name": n,
                "code": code(),
                "description": desc(),
                "branch_name": branch(),
                "parent_id": if parent_id().is_empty() { serde_json::Value::Null } else { json!(parent_id()) },
                "manager_user_id": if mgr_id().is_empty() { serde_json::Value::Null } else { json!(mgr_id()) },
                "linked_channel_id": if linked_ch_id().is_empty() { serde_json::Value::Null } else { json!(linked_ch_id()) },
            });
            spawn(async move {
                match enterprise::create_department(&sid, body).await {
                    Ok(_) => {
                        name.set(String::new());
                        code.set(String::new());
                        desc.set(String::new());
                        branch.set(String::new());
                        linked_ch_id.set(String::new());
                        show_create.set(false);
                        msg.set(Some((false, "New department created".into())));
                        r();
                    }
                    Err(e) => msg.set(Some((true, format!("Unable to create department: {}", e.code)))),
                }
            });
        }
    };

    let total_assigned: i64 = depts().iter().map(|d| d.member_count).sum();

    rsx! {
        div { class: "ent-section",
            div { class: "ent-toolbar",
                div { class: "ent-inline-bar",
                    select {
                        class: "cw-dropdown",
                        aria_label: "List of departments",
                        value: "{view_mode}",
                        onchange: move |e| view_mode.set(e.value()),
                        option { value: "grid", selected: view_mode() == "grid", "List of departments" }
                        option { value: "orgchart", selected: view_mode() == "orgchart", "Organizational chart" }
                    }
                    span { class: "ent-pill ok", "{depts().len()} departments" }
                    span { class: "ent-pill", "{total_assigned} affiliated personnel" }
                }
                if can_manage {
                    button {
                        r#type: "button",
                        class: "btn-primary",
                        onclick: move |_| show_create.set(!show_create()),
                        PlusIcon { size: 15 } " Create departments"
                    }
                }
            }

            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{text}" }
            }

            if can_manage && show_create() {
                div { class: "ent-card",
                    div { class: "ent-form-grid",
                        label { class: "form-field", span { "Department name" }
                            input { r#type: "text", placeholder: "For example: Technical Department", value: "{name}", oninput: move |e| name.set(e.value()) }
                        }
                        label { class: "form-field", span { "Department code" }
                            input { r#type: "text", placeholder: "For example: ENG", value: "{code}", oninput: move |e| code.set(e.value()) }
                        }
                        label { class: "form-field", span { "Branch" }
                            input { r#type: "text", placeholder: "For example: Head office", value: "{branch}", oninput: move |e| branch.set(e.value()) }
                        }
                        label { class: "form-field", span { "Superior unit" }
                            select { value: "{parent_id}", onchange: move |e| parent_id.set(e.value()),
                                option { value: "", "Highest level" }
                                for d in depts() { option { key: "{d.id}", value: "{d.id}", "{d.name}" } }
                            }
                        }
                        label { class: "form-field", span { "Head of department" }
                            select { value: "{mgr_id}", onchange: move |e| mgr_id.set(e.value()),
                                option { value: "", "Not selected yet" }
                                for m in members() { option { key: "{m.user_id}", value: "{m.user_id}", "{m.username}" } }
                            }
                        }
                        label { class: "form-field", span { "Linked chat channel" }
                            select { value: "{linked_ch_id}", onchange: move |e| linked_ch_id.set(e.value()),
                                option { value: "", "Do not link channels" }
                                for ch in channels() {
                                    if ch.channel_type == "text" {
                                        option { key: "{ch.id}", value: "{ch.id}", "#{ch.name}" }
                                    }
                                }
                            }
                        }
                        label { class: "form-field", span { "Functions & Duties" }
                            input { r#type: "text", placeholder: "Describe the main function of the department", value: "{desc}", oninput: move |e| desc.set(e.value()) }
                        }
                    }
                    div { class: "ent-btn-group",
                        button { r#type: "button", class: "btn-primary", onclick: create, "Save department" }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| show_create.set(false), "Close" }
                    }
                }
            }

            if view_mode() == "orgchart" {
                div { class: "ent-list",
                    for root in depts().into_iter().filter(|d| d.parent_id.is_none()) {
                        {
                            let children: Vec<DepartmentItem> = depts().into_iter().filter(|c| c.parent_id.as_deref() == Some(root.id.as_str())).collect();
                            rsx! {
                                div { key: "org-{root.id}", class: "ent-card",
                                    div { class: "ent-row-title",
                                        strong { "{root.name}" }
                                        if let Some(c) = &root.code { if !c.is_empty() { span { class: "ent-pill ok", "{c}" } } }
                                        span { class: "ent-pill", "{root.member_count} personnel" }
                                        if let Some(m) = &root.manager_name { span { class: "ent-pill", "Manager: {m}" } }
                                        if let Some(ch) = &root.linked_channel_name { span { class: "ent-pill ok", "#{ch}" } }
                                    }
                                    if !children.is_empty() {
                                        div { class: "ent-sub-drawer", style: "margin-top: 8px; padding-left: 16px;",
                                            for child in children {
                                                div { key: "org-c-{child.id}", class: "ent-row-card",
                                                    div { class: "ent-row-title",
                                                        strong { "↳ {child.name}" }
                                                        if let Some(c) = &child.code { if !c.is_empty() { span { class: "ent-pill", "{c}" } } }
                                                        span { class: "ent-pill", "{child.member_count} personnel" }
                                                        if let Some(m) = &child.manager_name { span { class: "ent-pill", "Manager: {m}" } }
                                                        if let Some(ch) = &child.linked_channel_name { span { class: "ent-pill ok", "#{ch}" } }
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
            } else {
                div { class: "ent-dept-grid",
                for d in depts() {
                    {
                        let sid_add = server_id.clone();
                        let did_add = d.id.clone();
                        let sid_del = server_id.clone();
                        let did_del = d.id.clone();
                        let r1 = reload.clone();
                        let r2 = reload.clone();
                        let parent_name = d.parent_id.as_ref().and_then(|pid| depts.read().iter().find(|x| &x.id == pid).map(|x| x.name.clone()));
                        rsx! {
                            div { key: "{d.id}", class: "ent-card",
                                div { class: "ent-card-head",
                                    div { class: "ent-row-title",
                                        strong { "{d.name}" }
                                        if let Some(c) = &d.code {
                                            if !c.is_empty() { span { class: "ent-pill ok", "{c}" } }
                                        }
                                        if let Some(b) = &d.branch_name {
                                            if !b.is_empty() { span { class: "ent-pill", "{b}" } }
                                        }
                                    }
                                    if can_manage {
                                        button { r#type: "button", class: "icon-btn danger", title: "Delete department", onclick: move |_| {
                                            let (s, d_id, r) = (sid_del.clone(), did_del.clone(), r2.clone());
                                            spawn(async move {
                                                if enterprise::delete_department(&s, &d_id).await.is_ok() { r(); }
                                            });
                                        }, TrashIcon { size: 15 } }
                                    }
                                }
                                if let Some(ds) = &d.description {
                                    if !ds.is_empty() {
                                        div { class: "ent-muted", "{ds}" }
                                    }
                                }
                                div { class: "ent-chips",
                                    span { class: "ent-pill", "{d.member_count} members" }
                                    if let Some(mn) = &d.manager_name {
                                        span { class: "ent-pill", "Head of department: {mn}" }
                                    }
                                    if let Some(ch) = &d.linked_channel_name {
                                        span { class: "ent-pill ok", "Channel #{ch}" }
                                    }
                                    if let Some(pn) = parent_name {
                                        span { class: "ent-pill", "Affiliate: {pn}" }
                                    }
                                }
                                if can_manage {
                                    div { class: "ent-inline-bar",
                                        select { value: "", onchange: move |e| {
                                            let u = e.value();
                                            if u.is_empty() { return; }
                                            let (s, d_id, r) = (sid_add.clone(), did_add.clone(), r1.clone());
                                            spawn(async move {
                                                if enterprise::add_department_member(&s, &d_id, &u).await.is_ok() {
                                                    r();
                                                }
                                            });
                                        },
                                            option { value: "", "+ Assign members to departments" }
                                            for m in members() { option { key: "{m.user_id}", value: "{m.user_id}", "{m.username}" } }
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
