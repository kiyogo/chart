use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::icons::{CloseIcon, PlusIcon},
    models::ServerMemberItem,
    services::{
        api,
        enterprise::{self, EnterpriseConfig, ProjectItem, TaskCommentItem, TaskItem},
    },
};

#[component]
pub fn TasksPanel(server_id: String, cfg: EnterpriseConfig, refresh_tick: u64) -> Element {
    let mut tasks = use_signal(Vec::<TaskItem>::new);
    let mut projects = use_signal(Vec::<ProjectItem>::new);
    let mut members = use_signal(Vec::<ServerMemberItem>::new);
    let mut mode = use_signal(|| "kanban".to_string()); // "kanban" | "list" | "calendar"
    let mut view = use_signal(|| "all".to_string()); // "all" | "mine" | "assigned" | "overdue"
    let mut q = use_signal(String::new);
    let mut show_create = use_signal(|| false);
    let mut open_task = use_signal(|| Option::<String>::None);
    let mut comments = use_signal(Vec::<TaskCommentItem>::new);
    let mut comment_input = use_signal(String::new);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);

    let mut title = use_signal(String::new);
    let mut desc = use_signal(String::new);
    let mut priority = use_signal(|| "medium".to_string());
    let mut due_date = use_signal(String::new);
    let mut assignee = use_signal(String::new);
    let mut project_id = use_signal(String::new);

    let can_create = cfg.has(enterprise::ent_perm::CREATE_TASKS);
    let can_assign = cfg.has(enterprise::ent_perm::ASSIGN_TASKS);

    let reload = use_callback({
        let sid = server_id.clone();
        move |_: ()| {
            let sid = sid.clone();
            let v = view();
            let qv = q();
            spawn(async move {
                if let Ok(t) = enterprise::list_tasks(&sid, &v, &qv).await {
                    tasks.set(t);
                }
                if let Ok(p) = enterprise::list_projects(&sid).await {
                    projects.set(p);
                }
                if let Ok(m) = api::servers_get_members(&sid).await {
                    members.set(m);
                }
            });
        }
    });

    {
        let sid = server_id.clone();
        use_effect(use_reactive!(|(sid, refresh_tick)| {
            let sid = sid.clone();
            let _ = refresh_tick;
            let v = view();
            let qv = q();
            spawn(async move {
                if let Ok(t) = enterprise::list_tasks(&sid, &v, &qv).await {
                    tasks.set(t);
                }
                if let Ok(p) = enterprise::list_projects(&sid).await {
                    projects.set(p);
                }
                if let Ok(m) = api::servers_get_members(&sid).await {
                    members.set(m);
                }
            });
        }));
    }

    let create_task = {
        let sid = server_id.clone();
        move |_| {
            let t = title().trim().to_string();
            if t.is_empty() {
                return;
            }
            let sid = sid.clone();
            let body = json!({
                "title": t,
                "description": desc(),
                "priority": priority(),
                "due_date": if due_date().is_empty() { serde_json::Value::Null } else { json!(due_date()) },
                "assignee_id": if assignee().is_empty() { serde_json::Value::Null } else { json!(assignee()) },
                "project_id": if project_id().is_empty() { serde_json::Value::Null } else { json!(project_id()) },
            });
            spawn(async move {
                match enterprise::create_task(&sid, body).await {
                    Ok(_) => {
                        title.set(String::new());
                        desc.set(String::new());
                        due_date.set(String::new());
                        show_create.set(false);
                        msg.set(Some((false, "Đã giao công việc mới".into())));
                        reload.call(());
                    }
                    Err(e) => msg.set(Some((true, format!("Không tạo được việc: {}", e.code)))),
                }
            });
        }
    };

    let set_status = use_callback({
        let sid = server_id.clone();
        move |(tid, st): (String, &'static str)| {
            let sid = sid.clone();
            spawn(async move {
                match enterprise::update_task(&sid, &tid, json!({ "status": st })).await {
                    Ok(_) => reload.call(()),
                    Err(e) => msg.set(Some((true, format!("Không cập nhật được: {}", e.code)))),
                }
            });
        }
    });

    let load_comments = use_callback({
        let sid = server_id.clone();
        move |tid: String| {
            let sid = sid.clone();
            open_task.set(Some(tid.clone()));
            spawn(async move {
                if let Ok(c) = enterprise::list_task_comments(&sid, &tid).await {
                    comments.set(c);
                }
            });
        }
    });

    rsx! {
        div { class: "ent-section",
            div { class: "ent-toolbar",
                div { class: "ent-inline-bar",
                    div { class: "ent-segmented",
                        button { r#type: "button", class: if mode() == "kanban" { "filter-chip active" } else { "filter-chip" }, onclick: move |_| mode.set("kanban".into()), "Kanban" }
                        button { r#type: "button", class: if mode() == "list" { "filter-chip active" } else { "filter-chip" }, onclick: move |_| mode.set("list".into()), "Danh sách" }
                        button { r#type: "button", class: if mode() == "calendar" { "filter-chip active" } else { "filter-chip" }, onclick: move |_| mode.set("calendar".into()), "Theo hạn" }
                    }
                    select { value: "{view}", onchange: move |e| { view.set(e.value()); reload.call(()); },
                        option { value: "all", "Tất cả việc" }
                        option { value: "mine", "Việc của tôi" }
                        option { value: "assigned", "Tôi đã giao" }
                        option { value: "overdue", "Quá hạn" }
                    }
                    input { r#type: "text", placeholder: "Tìm công việc", value: "{q}", oninput: move |e| q.set(e.value()) }
                    button { r#type: "button", class: "btn-secondary", onclick: move |_| reload.call(()), "Lọc" }
                }
                if can_create {
                    button {
                        r#type: "button",
                        class: "btn-primary",
                        onclick: move |_| show_create.set(!show_create()),
                        PlusIcon { size: 15 } " Giao việc mới"
                    }
                }
            }

            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{text}" }
            }

            if can_create && show_create() {
                div { class: "ent-card",
                    div { class: "ent-form-grid",
                        label { class: "form-field", span { "Tiêu đề công việc" }
                            input { r#type: "text", placeholder: "Nhập tiêu đề công việc", value: "{title}", oninput: move |e| title.set(e.value()) }
                        }
                        label { class: "form-field", span { "Dự án liên kết" }
                            select { value: "{project_id}", onchange: move |e| project_id.set(e.value()),
                                option { value: "", "Chưa chọn dự án" }
                                for p in projects() { option { key: "{p.id}", value: "{p.id}", "{p.name}" } }
                            }
                        }
                        if can_assign {
                            label { class: "form-field", span { "Người thực hiện" }
                                select { value: "{assignee}", onchange: move |e| assignee.set(e.value()),
                                    option { value: "", "Chưa phân công" }
                                    for m in members() { option { key: "{m.user_id}", value: "{m.user_id}", "{m.username}" } }
                                }
                            }
                        }
                        label { class: "form-field", span { "Mức ưu tiên" }
                            select { value: "{priority}", onchange: move |e| priority.set(e.value()),
                                option { value: "low", "Thấp" }
                                option { value: "medium", "Trung bình" }
                                option { value: "high", "Cao" }
                                option { value: "urgent", "Khẩn cấp" }
                            }
                        }
                        label { class: "form-field", span { "Hạn hoàn thành" }
                            input { r#type: "date", value: "{due_date}", oninput: move |e| due_date.set(e.value()) }
                        }
                        label { class: "form-field", span { "Mô tả chi tiết" }
                            input { r#type: "text", placeholder: "Yêu cầu đầu ra hoặc tiêu chí hoàn thành", value: "{desc}", oninput: move |e| desc.set(e.value()) }
                        }
                    }
                    div { class: "ent-btn-group",
                        button { r#type: "button", class: "btn-primary", onclick: create_task, "Lưu công việc" }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| show_create.set(false), "Đóng" }
                    }
                }
            }

            if mode() == "kanban" {
                div { class: "ent-kanban",
                    for (st, st_label) in [("todo", "Cần làm"), ("in_progress", "Đang làm"), ("review", "Chờ duyệt"), ("done", "Hoàn thành")] {
                        {
                            let col_tasks: Vec<TaskItem> = tasks().into_iter().filter(|t| t.status == st).collect();
                            rsx! {
                                div { key: "{st}", class: "ent-kanban-col",
                                    div { class: "ent-kanban-head", strong { "{st_label}" } span { class: "ent-pill", "{col_tasks.len()}" } }
                                    for t in col_tasks {
                                        {
                                            let tid1 = t.id.clone();
                                            let tid2 = t.id.clone();
                                            let tid3 = t.id.clone();
                                            rsx! {
                                                div { key: "{t.id}", class: "ent-kanban-card",
                                                    div { class: "ent-row-title",
                                                        strong { "{t.title}" }
                                                        span { class: "ent-pill", "{enterprise::vn_priority(&t.priority)}" }
                                                    }
                                                    if let Some(ds) = &t.description {
                                                        if !ds.is_empty() {
                                                            div { class: "ent-muted", style: "margin-top: 4px;", "{ds}" }
                                                        }
                                                    }
                                                    div { class: "ent-chips", style: "margin-top: 6px;",
                                                        if let Some(pn) = &t.project_name { span { class: "ent-pill", "{pn}" } }
                                                        if let Some(an) = &t.assignee_name { span { class: "ent-pill", "@{an}" } }
                                                        if let Some(d) = &t.due_date { span { class: "ent-pill", "Hạn {d}" } }
                                                    }
                                                    div { class: "ent-btn-group", style: "margin-top: 8px;",
                                                        if st == "todo" {
                                                            button { r#type: "button", class: "btn-secondary", onclick: move |_| set_status.call((tid2.clone(), "in_progress")), "Nhận việc" }
                                                        }
                                                        if st == "in_progress" {
                                                            button { r#type: "button", class: "btn-secondary", onclick: move |_| set_status.call((tid3.clone(), "review")), "Gửi duyệt" }
                                                        }
                                                        if st != "done" {
                                                            button { r#type: "button", class: "btn-primary", onclick: move |_| set_status.call((tid1.clone(), "done")), "Hoàn tất" }
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
            } else {
                div { class: "ent-list",
                    for t in tasks() {
                        {
                            let tid_c = t.id.clone();
                            let tid_s = t.id.clone();
                            let is_open = open_task().as_deref() == Some(t.id.as_str());
                            let sid_c = server_id.clone();
                            rsx! {
                                div { key: "{t.id}", class: "ent-card",
                                    div { class: "ent-row-card", style: "border: none; padding: 0; background: transparent;",
                                        div { class: "grow",
                                            div { class: "ent-row-title",
                                                strong { "{t.title}" }
                                                span { class: "ent-pill", "{enterprise::vn_priority(&t.priority)}" }
                                                span { class: "ent-pill ok", "{enterprise::vn_task_status(&t.status)}" }
                                                if let Some(pn) = &t.project_name { span { class: "ent-pill", "{pn}" } }
                                                if let Some(ch) = &t.source_channel_name { span { class: "ent-pill", "Từ #{ch}" } }
                                            }
                                            if let Some(prev) = &t.source_message_preview {
                                                div { class: "ent-quote", "{prev}" }
                                            }
                                            div { class: "ent-muted",
                                                if let Some(an) = &t.assignee_name { "Phụ trách: {an}" } else { "Chưa gán" }
                                                if let Some(d) = &t.due_date { " · Hạn: {d}" }
                                                " · {t.comment_count} bình luận"
                                            }
                                        }
                                        div { class: "ent-btn-group",
                                            select { value: "{t.status}", onchange: move |e| {
                                                let s = e.value();
                                                let st = match s.as_str() { "in_progress" => "in_progress", "review" => "review", "done" => "done", "archived" => "archived", _ => "todo" };
                                                set_status.call((tid_s.clone(), st));
                                            },
                                                option { value: "todo", "Cần làm" }
                                                option { value: "in_progress", "Đang làm" }
                                                option { value: "review", "Chờ duyệt" }
                                                option { value: "done", "Hoàn thành" }
                                                option { value: "archived", "Lưu trữ" }
                                            }
                                            button { r#type: "button", class: "btn-secondary", onclick: move |_| {
                                                if is_open { open_task.set(None); } else { load_comments.call(tid_c.clone()); }
                                            }, "Thảo luận" }
                                        }
                                    }
                                    if is_open {
                                        div { class: "ent-sub-drawer",
                                            for c in comments() {
                                                div { key: "{c.id}", class: "ent-comment",
                                                    strong { "{c.author_name}: " }
                                                    span { "{c.content}" }
                                                }
                                            }
                                            div { class: "ent-inline-bar", style: "margin-top: 6px;",
                                                input { r#type: "text", placeholder: "Viết trao đổi công việc", value: "{comment_input}", oninput: move |e| comment_input.set(e.value()) }
                                                button { r#type: "button", class: "btn-primary", onclick: {
                                                    let sid = sid_c.clone();
                                                    let tid = t.id.clone();
                                                    move |_| {
                                                        let txt = comment_input().trim().to_string();
                                                        if txt.is_empty() { return; }
                                                        let (sid, tid) = (sid.clone(), tid.clone());
                                                        spawn(async move {
                                                            if enterprise::add_task_comment(&sid, &tid, &txt).await.is_ok() {
                                                                comment_input.set(String::new());
                                                                if let Ok(c) = enterprise::list_task_comments(&sid, &tid).await {
                                                                    comments.set(c);
                                                                }
                                                            }
                                                        });
                                                    }
                                                }, "Gửi" }
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
pub fn ProjectsPanel(server_id: String, cfg: EnterpriseConfig, refresh_tick: u64) -> Element {
    let mut projects = use_signal(Vec::<ProjectItem>::new);
    let mut show_create = use_signal(|| false);
    let mut name = use_signal(String::new);
    let mut desc = use_signal(String::new);
    let mut priority = use_signal(|| "medium".to_string());
    let mut due_date = use_signal(String::new);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);

    let can_manage = cfg.has(enterprise::ent_perm::MANAGE_PROJECTS);

    let reload = {
        let sid = server_id.clone();
        move || {
            let sid = sid.clone();
            spawn(async move {
                if let Ok(p) = enterprise::list_projects(&sid).await {
                    projects.set(p);
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
                if let Ok(p) = enterprise::list_projects(&sid).await {
                    projects.set(p);
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
                "description": desc(),
                "priority": priority(),
                "due_date": if due_date().is_empty() { serde_json::Value::Null } else { json!(due_date()) },
            });
            spawn(async move {
                match enterprise::create_project(&sid, body).await {
                    Ok(_) => {
                        name.set(String::new());
                        desc.set(String::new());
                        show_create.set(false);
                        msg.set(Some((false, "Đã khởi tạo dự án".into())));
                        r();
                    }
                    Err(e) => msg.set(Some((true, format!("Không tạo được dự án: {}", e.code)))),
                }
            });
        }
    };

    rsx! {
        div { class: "ent-section",
            div { class: "ent-toolbar",
                div { class: "ent-chips",
                    span { class: "ent-pill ok", "{projects().len()} dự án doanh nghiệp" }
                }
                if can_manage {
                    button {
                        r#type: "button",
                        class: "btn-primary",
                        onclick: move |_| show_create.set(!show_create()),
                        PlusIcon { size: 15 } " Tạo dự án"
                    }
                }
            }
            if let Some((is_err, text)) = msg() {
                div { class: if is_err { "status-badge error" } else { "status-badge success" }, "{text}" }
            }
            if can_manage && show_create() {
                div { class: "ent-card",
                    div { class: "ent-form-grid",
                        label { class: "form-field", span { "Tên dự án" }
                            input { r#type: "text", placeholder: "Nhập tên dự án", value: "{name}", oninput: move |e| name.set(e.value()) }
                        }
                        label { class: "form-field", span { "Mô tả mục tiêu" }
                            input { r#type: "text", placeholder: "Mô tả ngắn gọn mục tiêu dự án", value: "{desc}", oninput: move |e| desc.set(e.value()) }
                        }
                        label { class: "form-field", span { "Mức ưu tiên" }
                            select { value: "{priority}", onchange: move |e| priority.set(e.value()),
                                option { value: "low", "Thấp" }
                                option { value: "medium", "Trung bình" }
                                option { value: "high", "Cao" }
                                option { value: "urgent", "Khẩn cấp" }
                            }
                        }
                        label { class: "form-field", span { "Thời hạn dự kiến" }
                            input { r#type: "date", value: "{due_date}", oninput: move |e| due_date.set(e.value()) }
                        }
                    }
                    div { class: "ent-btn-group",
                        button { r#type: "button", class: "btn-primary", onclick: create, "Lưu dự án" }
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| show_create.set(false), "Đóng" }
                    }
                }
            }
            div { class: "ent-dept-grid",
                for p in projects() {
                    {
                        let pct = if p.task_total > 0 { (p.task_done * 100 / p.task_total).min(100) } else { 0 };
                        let pid = p.id.clone();
                        let sid = server_id.clone();
                        let r = reload.clone();
                        rsx! {
                            div { key: "{p.id}", class: "ent-card",
                                div { class: "ent-card-head",
                                    div { class: "ent-row-title",
                                        strong { "{p.name}" }
                                        span { class: "ent-pill ok", "{enterprise::vn_project_status(&p.status)}" }
                                        span { class: "ent-pill", "{enterprise::vn_priority(&p.priority)}" }
                                    }
                                    if can_manage {
                                        select { value: "{p.status}", onchange: move |e| {
                                            let (sid, pid, r, v) = (sid.clone(), pid.clone(), r.clone(), e.value());
                                            spawn(async move {
                                                if enterprise::update_project(&sid, &pid, json!({ "status": v })).await.is_ok() { r(); }
                                            });
                                        },
                                            option { value: "planning", "Lập kế hoạch" }
                                            option { value: "active", "Đang chạy" }
                                            option { value: "on_hold", "Tạm dừng" }
                                            option { value: "completed", "Hoàn thành" }
                                            option { value: "archived", "Lưu trữ" }
                                        }
                                    }
                                }
                                if let Some(d) = &p.description {
                                    if !d.is_empty() { div { class: "ent-muted", "{d}" } }
                                }
                                div { class: "ent-chips",
                                    span { class: "ent-pill", "{p.task_done}/{p.task_total} công việc ({pct}%)" }
                                    if let Some(due) = &p.due_date { span { class: "ent-pill", "Hạn {due}" } }
                                    if let Some(ch) = &p.linked_channel_name { span { class: "ent-pill ok", "Kênh #{ch}" } }
                                }
                                div { class: "ent-progress", div { class: "ent-progress-fill", style: "width: {pct}%;" } }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn CreateTaskFromMsgDialog(
    server_id: String,
    channel_id: String,
    message_id: String,
    preview: String,
    on_close: EventHandler<()>,
    on_created: EventHandler<()>,
) -> Element {
    let default_title: String = preview.chars().take(72).collect();
    let mut title = use_signal(move || if default_title.is_empty() { "Công việc từ tin nhắn".to_string() } else { default_title });
    let mut description = use_signal(|| preview.clone());
    let mut priority = use_signal(|| "medium".to_string());
    let mut due_date = use_signal(String::new);
    let mut assignee = use_signal(String::new);
    let mut project_id = use_signal(String::new);
    let mut members = use_signal(Vec::<ServerMemberItem>::new);
    let mut projects = use_signal(Vec::<ProjectItem>::new);
    let mut err = use_signal(|| Option::<String>::None);
    let mut saving = use_signal(|| false);

    {
        let sid = server_id.clone();
        use_hook(move || {
            let sid = sid.clone();
            spawn(async move {
                if let Ok(m) = api::servers_get_members(&sid).await { members.set(m); }
                if let Ok(p) = enterprise::list_projects(&sid).await { projects.set(p); }
            });
        });
    }

    rsx! {
        div { class: "modal-overlay", role: "dialog", aria_modal: "true", onclick: move |_| on_close.call(()),
            div { class: "modal-dialog", onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h3 { class: "modal-title", "Tạo công việc từ tin nhắn kênh" }
                    button { r#type: "button", class: "icon-btn", aria_label: "Đóng", onclick: move |_| on_close.call(()), CloseIcon {} }
                }
                div { class: "modal-body",
                    label { class: "form-field", span { class: "form-label", "Tiêu đề công việc" }
                        input { r#type: "text", value: "{title}", oninput: move |e| title.set(e.value()) }
                    }
                    label { class: "form-field", span { class: "form-label", "Nội dung chi tiết" }
                        textarea { rows: 3, value: "{description}", oninput: move |e| description.set(e.value()) }
                    }
                    div { class: "ent-form-grid",
                        label { class: "form-field", span { class: "form-label", "Người thực hiện" }
                            select { value: "{assignee}", onchange: move |e| assignee.set(e.value()),
                                option { value: "", "Chưa phân công" }
                                for m in members() { option { key: "{m.user_id}", value: "{m.user_id}", "{m.username}" } }
                            }
                        }
                        label { class: "form-field", span { class: "form-label", "Dự án" }
                            select { value: "{project_id}", onchange: move |e| project_id.set(e.value()),
                                option { value: "", "Không gắn dự án" }
                                for p in projects() { option { key: "{p.id}", value: "{p.id}", "{p.name}" } }
                            }
                        }
                        label { class: "form-field", span { class: "form-label", "Mức ưu tiên" }
                            select { value: "{priority}", onchange: move |e| priority.set(e.value()),
                                option { value: "low", "Thấp" }
                                option { value: "medium", "Trung bình" }
                                option { value: "high", "Cao" }
                                option { value: "urgent", "Khẩn cấp" }
                            }
                        }
                        label { class: "form-field", span { class: "form-label", "Hạn hoàn thành" }
                            input { r#type: "date", value: "{due_date}", oninput: move |e| due_date.set(e.value()) }
                        }
                    }
                    if let Some(e) = err() { div { class: "status-badge error", "{e}" } }
                    div { style: "display: flex; justify-content: flex-end; gap: 8px;",
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Hủy" }
                        button {
                            r#type: "button",
                            class: "btn-primary",
                            disabled: saving() || title().trim().is_empty(),
                            onclick: move |_| {
                                if saving() { return; }
                                saving.set(true);
                                let sid = server_id.clone();
                                let cid = channel_id.clone();
                                let mid = message_id.clone();
                                let body = json!({
                                    "title": title().trim(),
                                    "description": description(),
                                    "priority": priority(),
                                    "due_date": if due_date().is_empty() { serde_json::Value::Null } else { json!(due_date()) },
                                    "assignee_id": if assignee().is_empty() { serde_json::Value::Null } else { json!(assignee()) },
                                    "project_id": if project_id().is_empty() { serde_json::Value::Null } else { json!(project_id()) },
                                    "source_channel_id": cid,
                                    "source_message_id": mid,
                                });
                                spawn(async move {
                                    match enterprise::create_task(&sid, body).await {
                                        Ok(_) => on_created.call(()),
                                        Err(e) => {
                                            err.set(Some(format!("Không tạo được việc: {}", e.code)));
                                            saving.set(false);
                                        }
                                    }
                                });
                            },
                            if saving() { "Đang lưu" } else { "Tạo công việc" }
                        }
                    }
                }
            }
        }
    }
}
