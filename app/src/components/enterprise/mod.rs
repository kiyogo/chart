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
        FileTextIcon, ListChecksIcon, MegaphoneIcon, NetworkIcon, PlugIcon, SparkleIcon, VideoIcon,
    },
    services::enterprise::{EnterpriseConfig, module_label},
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
    let _ = server_name;

    rsx! {
        div { class: "chat-main-container",
            div { class: "chat-column ent-workspace",
                header { class: "chat-header ent-header",
                    button { r#type: "button", class: "icon-btn back-btn", aria_label: "Quay lại thanh bên", onclick: move |_| on_close.call(()), BackIcon {} }
                    div { class: "chat-header-identity",
                        span { style: "color: var(--accent); display: flex;", {module_icon(&module)} }
                        div { class: "chat-header-info",
                            h2 { class: "chat-header-title", "{module_label(&module)}" }
                        }
                    }
                }

                div { class: "ent-body",
                    match module.as_str() {
                        "hr" => rsx! { hr_dept::HrPanel { server_id, cfg, refresh_tick } },
                        "departments" => rsx! { hr_dept::DepartmentsPanel { server_id, cfg, refresh_tick } },
                        "tasks" => rsx! { tasks_projects::TasksPanel { server_id, cfg, refresh_tick } },
                        "projects" => rsx! { tasks_projects::ProjectsPanel { server_id, cfg, refresh_tick } },
                        "attendance" => rsx! { time_leave::AttendancePanel { server_id, cfg, refresh_tick } },
                        "leave" => rsx! { time_leave::LeavePanel { server_id, my_id, cfg, refresh_tick } },
                        "meetings" => rsx! { time_leave::MeetingsPanel { server_id, refresh_tick } },
                        "calendar" => rsx! { time_leave::CalendarPanel { server_id, refresh_tick } },
                        "approvals" => rsx! { ops::ApprovalsPanel { server_id, my_id, cfg, refresh_tick } },
                        "documents" => rsx! { ops::DocumentsPanel { server_id, cfg, refresh_tick } },
                        "announcements" => rsx! { ops::AnnouncementsPanel { server_id, cfg, refresh_tick } },
                        "reports" => rsx! { overview::ReportsPanel { server_id, cfg, refresh_tick } },
                        "ai" => rsx! { ops::AiPanel { server_id, cfg } },
                        "automation" => rsx! { ops::AutomationPanel { server_id, cfg, refresh_tick } },
                        _ => rsx! { overview::OverviewPanel { server_id, cfg, refresh_tick, on_navigate: on_select_module } },
                    }
                }
            }
        }
    }
}
