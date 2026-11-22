//! Ctrl/Cmd + K quick switcher: jump to any conversation, group or community.

use dioxus::prelude::*;

use crate::{
    components::{avatar::Avatar, icons::{CompassIcon, SearchIcon, SettingsIcon}},
    models::{FriendItem, GroupItem, ServerItem},
};

#[derive(Clone, PartialEq)]
enum Target {
    Friend(FriendItem),
    Group(GroupItem),
    Server(ServerItem),
    Settings,
}

impl Target {
    fn label(&self) -> String {
        match self {
            Target::Friend(f) => f.username.clone(),
            Target::Group(g) => g.name.clone(),
            Target::Server(s) => s.name.clone(),
            Target::Settings => "Settings".to_string(),
        }
    }
    fn kind(&self) -> &'static str {
        match self {
            Target::Friend(_) => "Direct messages",
            Target::Group(_) => "Groups",
            Target::Server(_) => "Communities",
            Target::Settings => "Application",
        }
    }
}

#[component]
pub fn QuickSwitcher(
    friends: Vec<FriendItem>,
    groups: Vec<GroupItem>,
    servers: Vec<ServerItem>,
    on_close: EventHandler<()>,
    on_pick_friend: EventHandler<FriendItem>,
    on_pick_group: EventHandler<GroupItem>,
    on_pick_server: EventHandler<ServerItem>,
    on_open_settings: EventHandler<()>,
) -> Element {
    let mut query = use_signal(String::new);
    let mut cursor = use_signal(|| 0usize);

    use_effect(move || {
        let _ = dioxus::document::eval("setTimeout(() => { const el = document.getElementById('quick-switch-input'); if (el) el.focus(); }, 0);");
    });

    let q = query().trim().to_lowercase();
    let mut all: Vec<Target> = Vec::new();
    all.extend(friends.into_iter().map(Target::Friend));
    all.extend(groups.into_iter().map(Target::Group));
    all.extend(servers.into_iter().map(Target::Server));
    all.push(Target::Settings);
    let mut results: Vec<Target> = all
        .into_iter()
        .filter(|t| q.is_empty() || t.label().to_lowercase().contains(&q))
        .collect();
    // Names that start with the query first.
    results.sort_by_key(|t| !t.label().to_lowercase().starts_with(&q));
    results.truncate(30);
    let count = results.len();
    let cur = cursor().min(count.saturating_sub(1));

    let pick = move |t: Target| match t {
        Target::Friend(f) => on_pick_friend.call(f),
        Target::Group(g) => on_pick_group.call(g),
        Target::Server(s) => on_pick_server.call(s),
        Target::Settings => on_open_settings.call(()),
    };
    let results_for_key = results.clone();

    rsx! {
        div { class: "modal-overlay quick-overlay", onclick: move |_| on_close.call(()),
            div {
                class: "quick-switcher",
                role: "dialog",
                aria_label: "Quick switch",
                onclick: move |e| e.stop_propagation(),
                div { class: "quick-input",
                    SearchIcon { size: 18 }
                    input {
                        id: "quick-switch-input",
                        r#type: "search",
                        placeholder: "Go to a chat, group, or community",
                        aria_label: "Quick switch",
                        autocomplete: "off",
                        value: "{query}",
                        oninput: move |e| { query.set(e.value()); cursor.set(0); },
                        onkeydown: move |e| match e.key() {
                            Key::Escape => on_close.call(()),
                            Key::ArrowDown => { e.prevent_default(); if count > 0 { cursor.set((cur + 1) % count); } }
                            Key::ArrowUp => { e.prevent_default(); if count > 0 { cursor.set((cur + count - 1) % count); } }
                            Key::Enter => { e.prevent_default(); if let Some(t) = results_for_key.get(cur).cloned() { pick(t); } }
                            _ => {}
                        },
                    }
                    span { class: "kbd", "Esc" }
                }
                div { class: "quick-results", role: "listbox",
                    if results.is_empty() {
                        div { class: "picker-empty", "No results" }
                    }
                    for (i, t) in results.into_iter().enumerate() {
                        {
                            let label = t.label();
                            let kind = t.kind();
                            let t2 = t.clone();
                            rsx! {
                                button {
                                    key: "{kind}-{label}-{i}",
                                    r#type: "button",
                                    role: "option",
                                    aria_selected: "{i == cur}",
                                    class: if i == cur { "quick-item active" } else { "quick-item" },
                                    onmouseenter: move |_| cursor.set(i),
                                    onclick: move |_| pick(t2.clone()),
                                    match &t {
                                        Target::Friend(f) => rsx! { Avatar { name: f.username.clone(), avatar_url: f.avatar_url.clone(), size: 32 } },
                                        Target::Group(g) => rsx! { Avatar { name: g.name.clone(), avatar_url: g.avatar_url.clone(), size: 32 } },
                                        Target::Server(_) => rsx! { span { class: "quick-icon", CompassIcon { size: 18 } } },
                                        Target::Settings => rsx! { span { class: "quick-icon", SettingsIcon {} } },
                                    }
                                    span { class: "quick-label", "{label}" }
                                    span { class: "quick-kind", "{kind}" }
                                }
                            }
                        }
                    }
                }
                div { class: "quick-footer",
                    span { span { class: "kbd", "↑" } " " span { class: "kbd", "↓" } " move" }
                    span { span { class: "kbd", "Enter" } " open" }
                    span { span { class: "kbd", "Ctrl K" } " Open this table" }
                }
            }
        }
    }
}
