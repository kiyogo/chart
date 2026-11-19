use std::collections::HashSet;

use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::{
        avatar::Avatar,
        icons::{BriefcaseIcon, NetworkIcon, PlusIcon, TrashIcon},
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
    let mut selected = use_signal(HashSet::<String>::new);
    let mut editing_uid = use_signal(|| Option::<String>::None);
    let mut handover_from = use_signal(|| Option::<String>::None);
    let mut handover_to = use_signal(String::new);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);

    // Form fields for add / edit employee
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
            spawn(async move {
                if let Ok(list) = enterprise::list_employees(&sid, &qv, &sv, &dv).await {
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
            spawn(async move {
                if let Ok(list) = enterprise::list_employees(&sid, &qv, &sv, &dv).await {
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
                msg.set(Some((true, "Vui lòng chọn thành viên và nhập họ tên".into())));
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
                        msg.set(Some((false, "Đã lưu hồ sơ nhân sự".into())));
                        editing_uid.set(None);
                        load.call(());
                    }
                    Err(e) => msg.set(Some((true, format!("Lỗi lưu hồ sơ: {}", e.code)))),
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
                    msg.set(Some((false, "Đã cập nhật trạng thái hàng loạt".into())));
                    load.call(());
                }
            });
        }
    });

    rsx! {
        div { class: "ent-section",
            div { class: "ent-toolbar",
                h3 { class: "ent-section-title", BriefcaseIcon { size: 18 } " Danh bạ nhân sự ({employees().len()})" }
                div { class: "ent-inline-bar",
                    input { r#type: "text", placeholder: "Tìm tên, mã NV, chức danh", value: "{q}", oninput: move |e| q.set(e.value()) }
                    select { value: "{status_filter}", onchange: move |e| { status_filter.set(e.value()); load.call(()); },
                        option { value: "", "Mọi trạng thái" }
                        option { value: "active", "Đang làm việc" }
                        option { value: "onboarding", "Đang thử việc" }
                        option { value: "suspended", "Tạm khóa" }
                        option { value: "offboarded", "Đã nghỉ việc" }
                    }
                    select { value: "{dept_filter}", onchange: move |e| { dept_filter.set(e.value()); load.call(()); },
                        option { value: "", "Mọi phòng ban" }
                        for d in depts() {
                            option { key: "{d.id}", value: "{d.id}", "{d.name}" }
                        }
                    }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| load.call(()), "Tìm" }
                    if can_manage {
                        button {
                            r#type: "button",
                            class: "btn-primary",
                            onclick: move |_| {
                                f_uid.set(String::new());
                                f_name.set(String::new());
                                f_code.set(String::new());
                                f_title.set(String::new());
                                editing_uid.set(Some("new".into()));
                            },
                            PlusIcon { size: 15 } " Thêm hồ sơ"
                        }
                    }
                }
            }

            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{text}" }
            }

            if can_manage && !selected().is_empty() {
                div { class: "ent-card ent-inline-bar",
                    strong { "Đã chọn {selected().len()} nhân sự:" }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| bulk_status.call("active"), "Kích hoạt" }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| bulk_status.call("suspended"), "Tạm khóa" }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| selected.set(HashSet::new()), "Bỏ chọn" }
                }
            }

            if let Some(mode) = editing_uid() {
                div { class: "ent-card",
                    div { class: "ent-card-head",
                        h4 { if mode == "new" { "Thêm / Cập nhật hồ sơ nhân sự" } else { "Chỉnh sửa hồ sơ nhân sự" } }
                        button { r#type: "button", class: "btn-ghost", onclick: move |_| editing_uid.set(None), "Đóng" }
                    }
                    div { class: "ent-form-grid",
                        if mode == "new" {
                            label { class: "form-field", span { "Thành viên máy chủ" }
                                select { value: "{f_uid}", onchange: move |e| {
                                    let id = e.value();
                                    f_uid.set(id.clone());
                                    if f_name().is_empty() {
                                        if let Some(m) = members.read().iter().find(|x| x.user_id == id) {
                                            f_name.set(m.nickname.clone().unwrap_or_else(|| m.username.clone()));
                                        }
                                    }
                                },
                                    option { value: "", "Chọn thành viên" }
                                    for m in members() {
                                        option { key: "{m.user_id}", value: "{m.user_id}", "{m.username}" }
                                    }
                                }
                            }
                        }
                        label { class: "form-field", span { "Họ và tên" } input { r#type: "text", value: "{f_name}", oninput: move |e| f_name.set(e.value()) } }
                        label { class: "form-field", span { "Mã nhân viên" } input { r#type: "text", value: "{f_code}", oninput: move |e| f_code.set(e.value()) } }
                        label { class: "form-field", span { "Chức danh" } input { r#type: "text", value: "{f_title}", oninput: move |e| f_title.set(e.value()) } }
                        label { class: "form-field", span { "Phòng ban" }
                            select { value: "{f_dept}", onchange: move |e| f_dept.set(e.value()),
                                option { value: "", "Chưa gán" }
                                for d in depts() { option { key: "{d.id}", value: "{d.id}", "{d.name}" } }
                            }
                        }
                        label { class: "form-field", span { "Loại thành viên" }
                            select { value: "{f_kind}", onchange: move |e| f_kind.set(e.value()),
                                option { value: "employee", "Nhân viên" }
                                option { value: "partner", "Đối tác" }
                                option { value: "guest", "Khách mời" }
                            }
                        }
                        label { class: "form-field", span { "Hình thức" }
                            select { value: "{f_mode}", onchange: move |e| f_mode.set(e.value()),
                                option { value: "onsite", "Tại văn phòng" }
                                option { value: "remote", "Từ xa" }
                                option { value: "hybrid", "Linh hoạt" }
                            }
                        }
                        label { class: "form-field", span { "Trạng thái" }
                            select { value: "{f_status}", onchange: move |e| f_status.set(e.value()),
                                option { value: "active", "Đang làm việc" }
                                option { value: "onboarding", "Đang thử việc" }
                                option { value: "suspended", "Tạm khóa" }
                                option { value: "offboarded", "Đã nghỉ việc" }
                            }
                        }
                        label { class: "form-field", span { "Email công việc" } input { r#type: "text", value: "{f_email}", oninput: move |e| f_email.set(e.value()) } }
                        label { class: "form-field", span { "Điện thoại công việc" } input { r#type: "text", value: "{f_phone}", oninput: move |e| f_phone.set(e.value()) } }
                        label { class: "form-field", span { "Loại hợp đồng" } input { r#type: "text", value: "{f_contract}", oninput: move |e| f_contract.set(e.value()) } }
                    }
                    label { class: "form-field", span { "Ghi chú nội bộ" } textarea { rows: 2, value: "{f_notes}", oninput: move |e| f_notes.set(e.value()) } }
                    div { class: "ent-btn-group", style: "margin-top: 8px;",
                        button { r#type: "button", class: "btn-primary", onclick: save_emp, "Lưu hồ sơ" }
                    }
                }
            }

            if let Some(from_id) = handover_from() {
                div { class: "ent-card",
                    div { class: "ent-card-head",
                        h4 { "Bàn giao công việc & tài liệu khi nghỉ việc" }
                        button { r#type: "button", class: "btn-ghost", onclick: move |_| handover_from.set(None), "Hủy" }
                    }
                    div { class: "ent-inline-bar",
                        select { value: "{handover_to}", onchange: move |e| handover_to.set(e.value()),
                            option { value: "", "Chọn người nhận bàn giao" }
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
                                                msg.set(Some((false, "Đã bàn giao xong và thu hồi quyền truy cập".into())));
                                                load.call(());
                                            }
                                            Err(e) => msg.set(Some((true, format!("Bàn giao thất bại: {}", e.code)))),
                                        }
                                    });
                                }
                            },
                            "Xác nhận bàn giao"
                        }
                    }
                }
            }

            div { class: "ent-list",
                for emp in employees() {
                    {
                        let e_edit = emp.clone();
                        let uid = emp.user_id.clone();
                        let uid_cb = uid.clone();
                        let checked = selected().contains(&uid);
                        rsx! {
                            div { key: "{emp.id}", class: "ent-row-card",
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
                                Avatar { name: emp.full_name.clone(), avatar_url: emp.avatar_url.clone(), size: 36 }
                                div { class: "grow",
                                    div { class: "ent-row-title",
                                        strong { "{emp.full_name}" }
                                        span { class: "ent-muted", " @{emp.username}" }
                                        if let Some(code) = &emp.employee_code {
                                            span { class: "ent-pill", "{code}" }
                                        }
                                        span { class: "ent-pill", "{emp.employment_status}" }
                                    }
                                    div { class: "ent-muted",
                                        "{emp.job_title.as_deref().unwrap_or(\"Thành viên\")}"
                                        if let Some(dn) = &emp.department_name { " · {dn}" }
                                        " · {emp.work_mode}"
                                        if let Some(em) = &emp.work_email { " · {em}" }
                                        if let Some(ph) = &emp.work_phone { " · {ph}" }
                                    }
                                }
                                if can_manage {
                                    div { class: "ent-btn-group",
                                        button { r#type: "button", class: "btn-secondary", onclick: move |_| open_edit(e_edit.clone()), "Sửa" }
                                        button { r#type: "button", class: "btn-ghost", onclick: move |_| handover_from.set(Some(emp.user_id.clone())), "Bàn giao" }
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
    let mut name = use_signal(String::new);
    let mut code = use_signal(String::new);
    let mut branch = use_signal(String::new);
    let mut parent_id = use_signal(String::new);
    let mut mgr_id = use_signal(String::new);
    let mut add_uid = use_signal(String::new);
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
                "branch_name": branch(),
                "parent_id": if parent_id().is_empty() { serde_json::Value::Null } else { json!(parent_id()) },
                "manager_user_id": if mgr_id().is_empty() { serde_json::Value::Null } else { json!(mgr_id()) },
            });
            spawn(async move {
                match enterprise::create_department(&sid, body).await {
                    Ok(_) => {
                        name.set(String::new());
                        code.set(String::new());
                        branch.set(String::new());
                        msg.set(Some((false, "Đã tạo phòng ban mới".into())));
                        r();
                    }
                    Err(e) => msg.set(Some((true, format!("Không tạo được phòng ban: {}", e.code)))),
                }
            });
        }
    };

    rsx! {
        div { class: "ent-section",
            h3 { class: "ent-section-title", NetworkIcon { size: 18 } " Sơ đồ phòng ban & Cơ cấu tổ chức" }
            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{text}" }
            }
            if can_manage {
                div { class: "ent-card",
                    div { class: "ent-inline-bar",
                        input { r#type: "text", placeholder: "Tên phòng ban", value: "{name}", oninput: move |e| name.set(e.value()) }
                        input { r#type: "text", placeholder: "Mã PB", style: "max-width: 110px;", value: "{code}", oninput: move |e| code.set(e.value()) }
                        input { r#type: "text", placeholder: "Chi nhánh", value: "{branch}", oninput: move |e| branch.set(e.value()) }
                        select { value: "{parent_id}", onchange: move |e| parent_id.set(e.value()),
                            option { value: "", "Cấp cao nhất" }
                            for d in depts() { option { key: "{d.id}", value: "{d.id}", "Trực thuộc: {d.name}" } }
                        }
                        select { value: "{mgr_id}", onchange: move |e| mgr_id.set(e.value()),
                            option { value: "", "Trưởng bộ phận" }
                            for m in members() { option { key: "{m.user_id}", value: "{m.user_id}", "{m.username}" } }
                        }
                        button { r#type: "button", class: "btn-primary", onclick: create, PlusIcon { size: 15 } " Tạo phòng ban" }
                    }
                }
            }

            div { class: "ent-list",
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
                            div { key: "{d.id}", class: if d.parent_id.is_some() { "ent-row-card ent-indent" } else { "ent-row-card" },
                                div { class: "grow",
                                    div { class: "ent-row-title",
                                        strong { "{d.name}" }
                                        if let Some(c) = &d.code { span { class: "ent-pill", "{c}" } }
                                        if let Some(b) = &d.branch_name { span { class: "ent-pill", "{b}" } }
                                        if let Some(pn) = parent_name { span { class: "ent-muted", "trực thuộc {pn}" } }
                                    }
                                    div { class: "ent-muted",
                                        "{d.member_count} thành viên"
                                        if let Some(mn) = &d.manager_name { " · Trưởng phòng: {mn}" }
                                        if let Some(ch) = &d.linked_channel_name { " · Kênh #{ch}" }
                                    }
                                }
                                if can_manage {
                                    div { class: "ent-btn-group",
                                        select { value: "{add_uid}", onchange: move |e| {
                                            let u = e.value();
                                            add_uid.set(u.clone());
                                            if u.is_empty() { return; }
                                            let (s, d_id, r) = (sid_add.clone(), did_add.clone(), r1.clone());
                                            spawn(async move {
                                                if enterprise::add_department_member(&s, &d_id, &u).await.is_ok() {
                                                    add_uid.set(String::new());
                                                    r();
                                                }
                                            });
                                        },
                                            option { value: "", "+ Thành viên" }
                                            for m in members() { option { key: "{m.user_id}", value: "{m.user_id}", "{m.username}" } }
                                        }
                                        button { r#type: "button", class: "icon-btn danger", title: "Xóa phòng ban", onclick: move |_| {
                                            let (s, d_id, r) = (sid_del.clone(), did_del.clone(), r2.clone());
                                            spawn(async move {
                                                if enterprise::delete_department(&s, &d_id).await.is_ok() { r(); }
                                            });
                                        }, TrashIcon { size: 15 } }
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
