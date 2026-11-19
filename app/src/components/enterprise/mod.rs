pub mod hr_dept;
pub mod ops;
pub mod overview;
pub mod settings;
pub mod tasks_projects;
pub mod time_leave;

use dioxus::prelude::*;

use crate::{
    components::icons::{
        BackIcon, BarChartIcon, BriefcaseIcon, BuildingIcon, CalendarIcon, ClipboardCheckIcon,
        FileTextIcon, ListChecksIcon, MegaphoneIcon, NetworkIcon, PlugIcon, RefreshIcon,
        SearchIcon, SparkleIcon, VideoIcon,
    },
    services::enterprise::{self, EnterpriseConfig, SearchResultItem, module_label},
};

pub fn module_icon(key: &str) -> Element {
    match key {
        "hr" => rsx! { BriefcaseIcon { size: 16 } },
        "departments" => rsx! { NetworkIcon { size: 16 } },
        "tasks" => rsx! { ListChecksIcon { size: 16 } },
        "projects" => rsx! { BriefcaseIcon { size: 16 } },
        "calendar" => rsx! { CalendarIcon { size: 16 } },
        "meetings" => rsx! { VideoIcon { size: 16 } },
        "attendance" => rsx! { CalendarIcon { size: 16 } },
        "leave" => rsx! { ClipboardCheckIcon { size: 16 } },
        "approvals" => rsx! { ClipboardCheckIcon { size: 16 } },
        "documents" => rsx! { FileTextIcon { size: 16 } },
        "announcements" => rsx! { MegaphoneIcon { size: 16 } },
        "reports" => rsx! { BarChartIcon { size: 16 } },
        "ai" => rsx! { SparkleIcon { size: 16 } },
        "automation" => rsx! { PlugIcon { size: 16 } },
        _ => rsx! { BuildingIcon { size: 16 } },
    }
}

#[component]
pub fn EnterpriseWorkspace(
    server_id: String,
    server_name: String,
    my_id: String,
    module: String,
    cfg: EnterpriseConfig,
    refresh_tick: u64,
    on_select_module: EventHandler<String>,
    on_close: EventHandler<()>,
) -> Element {
    let mut query = use_signal(String::new);
    let mut hits = use_signal(Vec::<SearchResultItem>::new);
    let mut local_tick = use_signal(|| 0u64);

    let effective_tick = refresh_tick + local_tick();
    let sid_search = server_id.clone();

    rsx! {
        div { class: "ent-workspace",
            header { class: "chat-header ent-header",
                button { r#type: "button", class: "icon-btn back-btn", aria_label: "Quay lại thanh bên", onclick: move |_| on_close.call(()), BackIcon {} }
                div { class: "chat-header-identity",
                    span { style: "color: var(--accent); display: flex;", {module_icon(&module)} }
                    div { class: "chat-header-info",
                        h2 { class: "chat-header-title", "{module_label(&module)}" }
                        span { class: "chat-header-meta", "{server_name} · Không gian doanh nghiệp" }
                    }
                }
                div { class: "chat-header-actions",
                    div { class: "ent-search-box",
                        SearchIcon { size: 15 }
                        input {
                            r#type: "text",
                            placeholder: "Tìm trong doanh nghiệp",
                            value: "{query}",
                            oninput: {
                                let sid = sid_search.clone();
                                move |e| {
                                    let val = e.value();
                                    query.set(val.clone());
                                    let sid = sid.clone();
                                    spawn(async move {
                                        if val.trim().is_empty() {
                                            hits.set(Vec::new());
                                        } else if let Ok(res) = enterprise::search(&sid, &val).await {
                                            hits.set(res);
                                        }
                                    });
                                }
                            },
                        }
                        if !hits().is_empty() {
                            div { class: "ent-search-popover",
                                for h in hits() {
                                    {
                                        let m = h.module.clone();
                                        rsx! {
                                            button { key: "{h.kind}-{h.id}", r#type: "button", class: "ent-search-hit",
                                                onclick: move |_| {
                                                    hits.set(Vec::new());
                                                    query.set(String::new());
                                                    on_select_module.call(m.clone());
                                                },
                                                strong { "{h.title}" }
                                                span { class: "ent-muted", "{h.subtitle}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    button { r#type: "button", class: "icon-btn", title: "Làm mới dữ liệu", onclick: move |_| local_tick.set(local_tick() + 1), RefreshIcon { size: 16 } }
                    button { r#type: "button", class: "btn-ghost", onclick: move |_| on_close.call(()), "Kênh chat" }
                }
            }

            div { class: "ent-body",
                match module.as_str() {
                    "hr" => rsx! { hr_dept::HrPanel { server_id, cfg, refresh_tick: effective_tick } },
                    "departments" => rsx! { hr_dept::DepartmentsPanel { server_id, cfg, refresh_tick: effective_tick } },
                    "tasks" => rsx! { tasks_projects::TasksPanel { server_id, cfg, refresh_tick: effective_tick } },
                    "projects" => rsx! { tasks_projects::ProjectsPanel { server_id, cfg, refresh_tick: effective_tick } },
                    "attendance" => rsx! { time_leave::AttendancePanel { server_id, cfg, refresh_tick: effective_tick } },
                    "leave" => rsx! { time_leave::LeavePanel { server_id, my_id, cfg, refresh_tick: effective_tick } },
                    "meetings" => rsx! { time_leave::MeetingsPanel { server_id, refresh_tick: effective_tick } },
                    "calendar" => rsx! { time_leave::CalendarPanel { server_id, refresh_tick: effective_tick } },
                    "approvals" => rsx! { ops::ApprovalsPanel { server_id, my_id, cfg, refresh_tick: effective_tick } },
                    "documents" => rsx! { ops::DocumentsPanel { server_id, cfg, refresh_tick: effective_tick } },
                    "announcements" => rsx! { ops::AnnouncementsPanel { server_id, cfg, refresh_tick: effective_tick } },
                    "reports" => rsx! { overview::ReportsPanel { server_id, cfg, refresh_tick: effective_tick } },
                    "ai" => rsx! { ops::AiPanel { server_id, cfg } },
                    "automation" => rsx! { ops::AutomationPanel { server_id, cfg, refresh_tick: effective_tick } },
                    _ => rsx! { overview::OverviewPanel { server_id, cfg, refresh_tick: effective_tick, on_navigate: on_select_module } },
                }
            }
        }
    }
}
