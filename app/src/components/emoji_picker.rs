use dioxus::prelude::*;

pub const EMOJIS: [&str; 24] = [
    "😀", "😂", "🥹", "❤️", "👍", "🎉",
    "😭", "😍", "🔥", "😊", "🥳", "🙏",
    "👏", "✨", "🚀", "💯", "😎", "🙌",
    "🤝", "🤩", "😴", "🤔", "✌️", "💪",
];

#[component]
pub fn EmojiPicker(
    on_select: EventHandler<String>,
    #[props(default)] _on_close: Option<EventHandler<()>>,
) -> Element {
    rsx! {
        div {
            class: "emoji-popup",
            role: "dialog",
            aria_label: "Chọn emoji",
            onclick: move |evt| evt.stop_propagation(),
            for emoji in EMOJIS {
                button {
                    r#type: "button",
                    class: "emoji-btn",
                    aria_label: "{emoji}",
                    onclick: move |_| on_select.call(emoji.to_string()),
                    "{emoji}"
                }
            }
        }
    }
}

