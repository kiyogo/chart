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
    let mut selected = use_signal(HashSet::<String>::new);
    let mut editing_uid = use_signal(|| Option::<String>::None);
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
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| load.call(()), "Lọc" }
                }
                if can_manage {
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
                        PlusIcon { size: 15 } " Thêm hồ sơ"
                    }
                }
            }

            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{text}" }
            }

            if can_manage && !selected().is_empty() {
                div { class: "ent-card ent-inline-bar",
                    span { class: "ent-pill", "Đã chọn {selected().len()} nhân sự" }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| bulk_status.call("active"), "Kích hoạt" }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| bulk_status.call("suspended"), "Tạm khóa" }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| selected.set(HashSet::new()), "Bỏ chọn" }
                }
            }

            if let Some(mode) = editing_uid() {
                div { class: "ent-card",
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
                    div { class: "ent-btn-group",
                        button { r#type: "button", class: "btn-primary", onclick: save_emp, "Lưu hồ sơ" }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| editing_uid.set(None), "Đóng" }
                    }
                }
            }

            if let Some(from_id) = handover_from() {
                div { class: "ent-card",
                    div { class: "ent-inline-bar",
                        select { value: "{handover_to}", onchange: move |e| handover_to.set(e.value()),
                            option { value: "", "Chọn người nhận bàn giao công việc & tài liệu" }
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
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| handover_from.set(None), "Hủy" }
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
                                            "{emp.job_title.as_deref().unwrap_or(\"Thành viên\")}"
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
                                        button { r#type: "button", class: "btn-secondary", onclick: move |_| open_edit(e_edit.clone()), "Chỉnh sửa" }
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
    let mut show_create = use_signal(|| false);
    let mut name = use_signal(String::new);
    let mut code = use_signal(String::new);
    let mut desc = use_signal(String::new);
    let mut branch = use_signal(String::new);
    let mut parent_id = use_signal(String::new);
    let mut mgr_id = use_signal(String::new);
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
                "description": desc(),
                "branch_name": branch(),
                "parent_id": if parent_id().is_empty() { serde_json::Value::Null } else { json!(parent_id()) },
                "manager_user_id": if mgr_id().is_empty() { serde_json::Value::Null } else { json!(mgr_id()) },
            });
            spawn(async move {
                match enterprise::create_department(&sid, body).await {
                    Ok(_) => {
                        name.set(String::new());
                        code.set(String::new());
                        desc.set(String::new());
                        branch.set(String::new());
                        show_create.set(false);
                        msg.set(Some((false, "Đã tạo phòng ban mới".into())));
                        r();
                    }
                    Err(e) => msg.set(Some((true, format!("Không tạo được phòng ban: {}", e.code)))),
                }
            });
        }
    };

    let total_assigned: i64 = depts().iter().map(|d| d.member_count).sum();

    rsx! {
        div { class: "ent-section",
            div { class: "ent-toolbar",
                div { class: "ent-chips",
                    span { class: "ent-pill ok", "{depts().len()} phòng ban" }
                    span { class: "ent-pill", "{total_assigned} nhân sự trực thuộc" }
                }
                if can_manage {
                    button {
                        r#type: "button",
                        class: "btn-primary",
                        onclick: move |_| show_create.set(!show_create()),
                        PlusIcon { size: 15 } " Tạo phòng ban"
                    }
                }
            }

            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{text}" }
            }

            if can_manage && show_create() {
                div { class: "ent-card",
                    div { class: "ent-form-grid",
                        label { class: "form-field", span { "Tên phòng ban" }
                            input { r#type: "text", placeholder: "Ví dụ: Phòng Pháp chế & Tuân thủ", value: "{name}", oninput: move |e| name.set(e.value()) }
                        }
                        label { class: "form-field", span { "Mã phòng ban" }
                            input { r#type: "text", placeholder: "Ví dụ: LEGAL", value: "{code}", oninput: move |e| code.set(e.value()) }
                        }
                        label { class: "form-field", span { "Chi nhánh" }
                            input { r#type: "text", placeholder: "Ví dụ: Trụ sở chính", value: "{branch}", oninput: move |e| branch.set(e.value()) }
                        }
                        label { class: "form-field", span { "Đơn vị cấp trên" }
                            select { value: "{parent_id}", onchange: move |e| parent_id.set(e.value()),
                                option { value: "", "Cấp cao nhất" }
                                for d in depts() { option { key: "{d.id}", value: "{d.id}", "{d.name}" } }
                            }
                        }
                        label { class: "form-field", span { "Trưởng bộ phận" }
                            select { value: "{mgr_id}", onchange: move |e| mgr_id.set(e.value()),
                                option { value: "", "Chưa chọn" }
                                for m in members() { option { key: "{m.user_id}", value: "{m.user_id}", "{m.username}" } }
                            }
                        }
                        label { class: "form-field", span { "Chức năng & Nhiệm vụ" }
                            input { r#type: "text", placeholder: "Mô tả chức năng chính của bộ phận", value: "{desc}", oninput: move |e| desc.set(e.value()) }
                        }
                    }
                    div { class: "ent-btn-group",
                        button { r#type: "button", class: "btn-primary", onclick: create, "Lưu phòng ban" }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| show_create.set(false), "Đóng" }
                    }
                }
            }

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
                                        button { r#type: "button", class: "icon-btn danger", title: "Xóa phòng ban", onclick: move |_| {
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
                                    span { class: "ent-pill", "{d.member_count} thành viên" }
                                    if let Some(mn) = &d.manager_name {
                                        span { class: "ent-pill", "Trưởng bộ phận: {mn}" }
                                    }
                                    if let Some(ch) = &d.linked_channel_name {
                                        span { class: "ent-pill ok", "Kênh #{ch}" }
                                    }
                                    if let Some(pn) = parent_name {
                                        span { class: "ent-pill", "Trực thuộc: {pn}" }
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
                                            option { value: "", "+ Gán thành viên vào phòng ban" }
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
