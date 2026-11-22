//! Unified Plugin Settings Entrypoint (Spec 058).
//! Renders EXCLUSIVELY the single sandboxed `PluginHubPanel` across both
//! `Cài đặt -> Tiện ích` and the left rail `Plugin` button.

use dioxus::prelude::*;

use crate::{
    components::plugin_hub::PluginHubPanel,
    models::BotItem,
    services::api,
};

#[component]
pub fn PluginsSettings(
    my_username: String,
    #[props(default)] on_open_bot_chat: Option<EventHandler<BotItem>>,
) -> Element {
    let _ = on_open_bot_chat;
    let uid = api::auth_me_cached_id().unwrap_or_default();
    rsx! {
        div { class: "plugins-unified-settings",
            PluginHubPanel {
                my_user_id: uid,
                my_username: my_username,
            }
        }
    }
}
