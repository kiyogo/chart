pub mod hr_dept;
pub mod ops;
pub mod overview;
pub mod settings;
pub mod tasks_projects;
pub mod time_leave;

use dioxus::prelude::*;

use crate::{
    components::{
        icons::{
            BackIcon, BarChartIcon, BriefcaseIcon, BuildingIcon, CalendarIcon, ClipboardCheckIcon,
            FileTextIcon, ListChecksIcon, MegaphoneIcon, NetworkIcon, PlugIcon, VicoAiIcon, VideoIcon,
        },
        server_members_panel::{ServerHeaderTools, ServerMembersPanel},
    },
    models::ServerMemberItem,
    services::{
        enterprise::{EnterpriseConfig, module_label},
        social::ServerRole,
    },
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
        "ai" => rsx! { VicoAiIcon { size: 16 } },
        "automation" => rsx! { PlugIcon { size: 16 } },
        _ => rsx! { BuildingIcon { size: 16 } },
    }
}

#[component]
pub fn EnterpriseWorkspace(
    server_id: String,
    server_name: String,
    owner_id: String,
    my_id: String,
    module: String,
    cfg: EnterpriseConfig,
    refresh_tick: u64,
    members: Vec<ServerMemberItem>,
    roles: Vec<ServerRole>,
    members_open: bool,
    on_toggle_members: EventHandler<()>,
    on_open_profile: EventHandler<String>,
    on_profile_click: EventHandler<(String, f64, f64)>,
    on_jump_channel_msg: EventHandler<(String, String)>,
    on_select_module: EventHandler<String>,
    on_close: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "chat-main-container",
            div { class: "chat-column ent-workspace",
                header { class: "chat-header ent-header",
                    button { r#type: "button", class: "icon-btn back-btn", aria_label: "Return to the sidebar", onclick: move |_| on_close.call(()), BackIcon {} }
                    div { class: "chat-header-identity",
                        span { style: "color: var(--accent); display: flex;", {module_icon(&module)} }
                        div { class: "chat-header-info",
                            h2 { class: "chat-header-title", "{module_label(&module)}" }
                        }
                    }
                    div { class: "chat-header-actions",
                        ServerHeaderTools {
                            server_id: server_id.clone(),
                            server_name: server_name.clone(),
                            is_enterprise: true,
                            members: members.clone(),
                            members_open: members_open,
                            on_toggle_members: on_toggle_members,
                            on_open_profile: on_open_profile,
                            on_jump_channel_msg: on_jump_channel_msg,
                            on_open_ent_module: Some(on_select_module),
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

            if members_open {
                div {
                    class: "panel-resizer resizer-right",
                    "data-resizer": "details",
                    title: "Drag to resize the member list",
                }
                ServerMembersPanel {
                    owner_id: owner_id,
                    members: members,
                    roles: roles,
                    on_profile_click: on_profile_click,
                    on_close: on_toggle_members,
                }
            }
        }
    }
}
