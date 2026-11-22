//! Illustrations, multi-step dialogs scaffolding, the welcome tour and the help page.

use dioxus::prelude::*;

use crate::{
    components::icons::{CheckIcon, CloseIcon, LogoIcon},
    platform,
};

fn trigger_spark_celebration() {
    let _ = dioxus::document::eval("if (window.cwSparkBurst) window.cwSparkBurst();");
}

/// Original animated duotone illustrations used at the top of dialogs & onboarding guides.
#[component]
pub fn Illustration(kind: String) -> Element {
    rsx! {
        svg { class: "illustration cw-living-ill", view_box: "0 0 240 120", "aria-hidden": "true",
            defs {
                linearGradient { id: "ill-g", x1: "0", y1: "0", x2: "1", y2: "1",
                    stop { offset: "0", stop_color: "var(--accent)" }
                    stop { offset: "1", stop_color: "#A78BFA" }
                }
            }
            circle { class: "cw-ill-halo", cx: "120", cy: "62", r: "50", fill: "url(#ill-g)", opacity: ".14" }
            path { class: "cw-ill-spark s1", d: "M40 30l3 7 7 3-7 3-3 7-3-7-7-3 7-3z", fill: "#FBBF24" }
            path { class: "cw-ill-spark s2", d: "M200 76l2.5 5.5 5.5 2.5-5.5 2.5-2.5 5.5-2.5-5.5-5.5-2.5 5.5-2.5z", fill: "#60A5FA" }
            circle { class: "cw-ill-spark s3", cx: "190", cy: "28", r: "4", fill: "#F472B6" }
            g { class: "cw-ill-float",
                match kind.as_str() {
                    "user" => rsx! {
                        rect { x: "78", y: "26", width: "84", height: "70", rx: "16", fill: "url(#ill-g)" }
                        circle { cx: "120", cy: "52", r: "13", fill: "#fff" }
                        path { d: "M96 86c4-12 13-18 24-18s20 6 24 18", fill: "#fff" }
                        rect { x: "150", y: "70", width: "34", height: "22", rx: "8", fill: "#FBBF24" }
                        path { d: "M158 81h18M167 76v10", stroke: "#fff", stroke_width: "4", stroke_linecap: "round" }
                    },
                    "devices" => rsx! {
                        rect { x: "62", y: "30", width: "86", height: "58", rx: "10", fill: "url(#ill-g)" }
                        rect { x: "70", y: "38", width: "70", height: "42", rx: "5", fill: "#fff", opacity: ".9" }
                        rect { x: "92", y: "90", width: "26", height: "6", rx: "3", fill: "url(#ill-g)" }
                        rect { x: "154", y: "44", width: "30", height: "52", rx: "8", fill: "#334155" }
                        rect { x: "159", y: "51", width: "20", height: "34", rx: "3", fill: "#fff" }
                        circle { cx: "105", cy: "59", r: "9", fill: "#22C55E" }
                        path { d: "M100 59l4 4 7-7", stroke: "#fff", stroke_width: "3", fill: "none", stroke_linecap: "round" }
                    },
                    "shield" => rsx! {
                        path { d: "M120 20l38 14v26c0 22-16 38-38 46-22-8-38-24-38-46V34z", fill: "url(#ill-g)" }
                        path { d: "M104 60l11 11 22-23", stroke: "#fff", stroke_width: "7", fill: "none", stroke_linecap: "round", stroke_linejoin: "round" }
                    },
                    "library" => rsx! {
                        rect { x: "70", y: "34", width: "50", height: "50", rx: "14", fill: "url(#ill-g)", transform: "rotate(-10 95 59)" }
                        rect { x: "120", y: "30", width: "50", height: "50", rx: "14", fill: "#F472B6", transform: "rotate(8 145 55)" }
                        circle { cx: "95", cy: "56", r: "13", fill: "#FFD27A" }
                        path { d: "M89 58q6 6 12 0", stroke: "#1B2440", stroke_width: "3", fill: "none", stroke_linecap: "round" }
                        text { x: "131", y: "62", font_family: "Arial, sans-serif", font_weight: "800", font_size: "16", fill: "#fff", "GIF" }
                    },
                    _ => rsx! {
                        path { d: "M92 28h56a14 14 0 0 1 14 14v28a14 14 0 0 1-14 14h-28l-18 13v-13h-10a14 14 0 0 1-14-14V42a14 14 0 0 1 14-14z", fill: "url(#ill-g)" }
                        path { d: "M100 50l7 16 13-13 13 13 7-16", stroke: "#fff", stroke_width: "6", fill: "none", stroke_linecap: "round", stroke_linejoin: "round" }
                    },
                }
            }
        }
    }
}

const TOUR: [(&str, &str, &str); 6] = [
    ("chat", "Welcome to ChatW", "Private messaging, multi-channel communities, voice and video calling. Let's take a quick look at what you can do."),
    ("shield", "Direct messages are end-to-end encrypted", "The private key is located on your device. The server only forwards the ciphertext. Compare lock fingerprints in the Info panel to verify friends."),
    ("user", "Profiles have their own imprint", "Theme color, avatar frame, name effect, nameplate, animation effect. You can also draw yourself or upload a photo to decorate."),
    ("library", "Your emojis, stickers and GIFs", "Open the menu in the compose box. Add your own emojis, stickers, and GIFs in Settings › Library and use them in every conversation."),
    ("devices", "Device control", "See which devices are remotely signed in and signed out in Settings › My Account."),
    ("chat", "Tip: Ctrl + K", "Press Ctrl + K anywhere to jump to a chat, group, or community."),
];

#[component]
pub fn WelcomeTour(on_close: EventHandler<()>) -> Element {
    let mut step = use_signal(|| 0usize);
    let (art, title, body) = TOUR[step()];
    let last = step() + 1 == TOUR.len();
    rsx! {
        div { class: "modal-overlay", onclick: move |_| on_close.call(()),
            div { class: "modal-dialog story-dialog", role: "dialog", aria_modal: "true", aria_label: "Introducing ChatW",
                onclick: move |e| e.stop_propagation(),
                onkeydown: move |e| match e.key() {
                    Key::Escape => on_close.call(()),
                    Key::ArrowRight if !last => step.set(step() + 1),
                    Key::ArrowLeft if step() > 0 => step.set(step() - 1),
                    _ => {}
                },
                div { class: "story-art",
                    button { r#type: "button", class: "icon-btn story-close", aria_label: "Skip tour", onclick: move |_| on_close.call(()), CloseIcon {} }
                    div { class: "story-step-badge", "Step {step() + 1} / {TOUR.len()}" }
                    div { key: "{step()}", class: "cw-step-enter",
                        Illustration { kind: art.to_string() }
                    }
                }
                div { class: "story-progress-track",
                    div {
                        class: "story-progress-fill",
                        style: "width: {((step() + 1) * 100) / TOUR.len()}%;",
                    }
                }
                div { key: "body-{step()}", class: "story-body cw-step-enter",
                    h2 { class: "story-title", "{title}" }
                    p { class: "story-text", "{body}" }
                    div { class: "story-dots", role: "tablist",
                        for i in 0..TOUR.len() {
                            button { key: "{i}", r#type: "button", class: if i == step() { "story-dot active" } else { "story-dot" }, aria_label: "Step {i + 1}", onclick: move |_| step.set(i) }
                        }
                    }
                }
                div { class: "story-actions",
                    if step() > 0 {
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| step.set(step() - 1), "Back" }
                    } else {
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Skip" }
                    }
                    button {
                        r#type: "button",
                        class: "btn-primary",
                        onclick: move |_| {
                            if last {
                                trigger_spark_celebration();
                                on_close.call(());
                            } else {
                                step.set(step() + 1);
                            }
                        },
                        if last { "Start exploring" } else { "Continue" }
                    }
                }
            }
        }
    }
}

const SHORTCUTS: [(&str, &str); 11] = [
    ("Ctrl + K", "Quickly find chats, groups, communities"),
    ("@ then type the name", "Mention a person's name; Tab or Enter to insert"),
    (": then type the emoji name", "Emoji suggestions, for example :fire"),
    ("/ then type the shortcut", "Insert Quick Reply by Business"),
    ("Double-click next to the message", "Reply to that message"),
    ("Enter", "Send a message"),
    ("Shift + Enter", "Down line"),
    ("Esc", "Close menus, dialog boxes, records"),
    ("↑ ↓ ← →", "Navigate the emoji panel"),
    ("Ctrl + K then type \"Settings\"", "Open settings"),
    ("Tab", "Move between buttons"),
];

const GUIDES: [(&str, &str, &[&str]); 16] = [
    ("user", "Avatar Studio: rotating avatars and 40 effects", &[
        "Open your profile › Edit profile › Avatar Studio tab.",
        "Choose an effect like Spin, Orbit, Hearts, Glitch, adjust Speed, Intensity, Color and Rotation Direction.",
        "Trigger: Always run, On hover or On click. Set Number of iterations if you don't want to run forever.",
        "Click Save. Friends see effects on their profile, chat list, and friends list.",
    ]),
    ("chat", "Demo Gallery: try before using", &[
        "Settings › Demo.",
        "See all Avatar effects, Name effects, Nameplates, Profile effects and Chat backgrounds.",
        "Click Apply: the changes are actually saved to the profile or settings.",
    ]),
    ("user", "Profile information and viewing permissions", &[
        "Edit profile › Information: add name, job, place of residence, website, interests.",
        "Each section has its own permissions: Everyone, Friends, or Only Me. The server sends the item only to authorized users.",
        "Custom status can be automatically deleted after 30 minutes, 1 hour, 4 hours, end of today or 7 days.",
        "Animated animated GIF or WebP cover images up to 8 MB in size.",
    ]),
    ("chat", "Favorite stickers, GIFs and emojis", &[
        "Hover over the sticker or GIF in the menu and press ☆ to add it to favorites.",
        "The recent tab holds the 24 most recent stickers you sent.",
        "Click on a sticker in a message to add it to favorites, resend it, or find similar stickers.",
        "Emoji: right-click on the emoji, or press ☆ at the foot of the menu. Favorites list syncs between devices.",
    ]),
    ("devices", "Sync settings between devices", &[
        "Settings › Customization › Sync settings with other devices.",
        "Interface, suggestions, notifications and favorites list follow the account.",
        "The app lock PIN never leaves the device.",
        "Two devices edit together: the last record is kept, neither device overwrites newer data without re-reading it.",
    ]),
    ("lock", "Nickname, private notes and save messages", &[
        "Open a conversation's Info panel › Nicknames and private notes.",
        "The nickname replaces the person's name on your list; only you see.",
        "Hover over a message and click the device icon to save a copy to \"Yourself\".",
        "Find quick settings using the Find settings box at the top of the Settings page.",
    ]),
    ("devices", "\"Yourself\": notes between devices", &[
        "Open \"Yourself\" at the top of the Messages list; the message is encrypted with your key.",
        "Each message records the name of the device sent. Click the device name to change the name.",
        "Options menu to change wallpaper or delete; recreate using the button at the top of the list.",
    ]),
    ("chat", "Pin, categorize, turn off notifications", &[
        "Hover over a conversation and click the options button.",
        "Pin to top, tag categories, turn off notifications, move group to Community, block.",
        "Manage tags using the category tags button above the list.",
    ]),
    ("user", "Manage group", &[
        "In groups, click the options button › Manage groups.",
        "Enable/disable membership rights: send messages, vote, take notes, remind appointments, pin, change name, change photo.",
        "Join: approve new members, invite link, request pending approval.",
        "The group leader appoints a deputy or transfers the group leader's authority.",
    ]),
    ("shield", "Lock ChatW with PIN code", &[
        "Settings › PIN Lock › set a 4 to 8 digit PIN code.",
        "Choose inactivity time before locking, or locking every time you open it.",
        "Forgot your PIN: sign out on the lock screen then sign in again.",
    ]),
    ("library", "Plugins and theme customization", &[
        "Click the Plugin icon on the left navigation bar or go to Settings › Plugins.",
        "Enable existing Plugins or create new Plugins.",
        "Each Plugin can make a Chat Bot, change the interface, add pages and enable HUD.",
    ]),
    ("shield", "Verify encryption with friends", &[
        "Open the chat, click the Information panel icon in the right corner.",
        "Open the \"Security and encryption\" section to view the lock fingerprint.",
        "Compare each 4-letter block with friends via face-to-face or call.",
    ]),
    ("user", "Profile decoration", &[
        "Click avatar in the lower left corner › My profile › Edit profile.",
        "Decoration tab: theme colors, avatar frames, profile effects.",
        "Effects tab: avatar rotation, rhythm, name effects, and nameplates.",
        "Custom tab: upload a photo or draw in the app, choose a motion, then save.",
    ]),
    ("library", "Add your own emoji, stickers, GIFs", &[
        "Settings › Gallery, select Sticker, Emoji or GIF.",
        "Emoji needs an English name consisting of letters, numbers, and underscores, for example party_cat.",
        "Use it right in the menu in the compose box, in every conversation.",
    ]),
    ("devices", "Sign out of the lost device", &[
        "Settings › Account › Signed in devices.",
        "Click \"Sign out\" on an unfamiliar device, or sign out all other devices.",
        "The new device will have its own encryption key; Back up the key in Security and encryption.",
    ]),
    ("chat", "Communities and channels", &[
        "Left column › Community › + button to create or join using invite code.",
        "In the community: create categories, text channels, and voice channels.",
        "Pin the community to the left column in Settings › Customization.",
    ]),
];

const FAQ: [(&str, &str); 9] = [
    ("Where does the OpenMoji sticker set come from?", "More than 2,000 stickers in the OpenMoji tab are the work of the OpenMoji open source project, used under a CC BY-SA 4.0 license. Names and search keywords come from Unicode CLDR data, so you can search in both Vietnamese and English."),
    ("Why doesn't automatic reply work when I turn off my phone?", "Private messages are end-to-end encrypted so the server cannot write its own reply. Automatic replies run on your device when ChatW is open."),
    ("What is the effect of hiding \"Seen\"?", "Other people don't know you've read it, and you won't see they've read it either. Change in Settings › Chat."),
    ("How do groups or communities show up in Search?", "Turn on Public in Group Management or Community Settings. ChatW team verifies before showing to everyone."),
    ("Why is there a message \"Unable to decode\"?", "Each browser has its own encryption key. Messages sent to the old key cannot be read on the new device. Restore the key backup in Security and encryption."),
    ("Are group messages end-to-end encrypted?", "Not yet. Currently only direct messages are end-to-end encrypted; Community groups and channels are saved by the server in their normal form."),
    ("Does ChatW save emails or phone numbers?", "No. You sign in with Google; The server only stores an anonymous identifier issued by Google."),
    ("Who sees my profile effects?", "People see your profile. Viewers can turn off animations in Settings › Customization."),
    ("How are file uploads limited?", "Sticker maximum 512 KB; Emojis, GIFs and profile decorations up to 2 MB; The server checks the actual format of the file."),
];

const BEGINNER_CHECKLIST: [(&str, &str, &str); 5] = [
    ("theme", "Customize the interface according to your own style", "Change theme color, background image, and frame transparency in Settings › Appearance"),
    ("self", "Send the first note to \"Yourself\"", "Quickly save photos, links, and notes separately between your phone and computer"),
    ("friend", "Make friends or create your first community", "Click the Friends or Community icon in the left sidebar to connect now"),
    ("backup", "Secure security key backup", "Go to Settings › Security or Cloud to back up your private key"),
    ("shortcut", "Try opening a quick search with Ctrl + K", "Jump instantly to any chat, channel, or setting"),
];

const CHECKLIST_STORAGE_KEY: &str = "chatw_beginner_checklist_v1";

#[component]
pub fn GuideSettings(on_replay_tour: EventHandler<()>) -> Element {
    let mut query = use_signal(String::new);
    let mut done_ids = use_signal(|| {
        platform::storage_get(CHECKLIST_STORAGE_KEY)
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
    });
    let done_count = BEGINNER_CHECKLIST
        .iter()
        .filter(|(id, _, _)| done_ids.read().iter().any(|x| x == id))
        .count();
    let pct = (done_count * 100) / BEGINNER_CHECKLIST.len();
    let q = crate::components::composer::fold(query().trim());
    let matches = |parts: &[&str]| q.is_empty() || parts.iter().any(|p| crate::components::composer::fold(p).contains(&q));
    let guides: Vec<_> = GUIDES.iter().filter(|(_, t, st)| matches(&[t]) || matches(st)).collect();
    let faq: Vec<_> = FAQ.iter().filter(|(qq, a)| matches(&[qq, a])).collect();
    let shortcuts: Vec<_> = SHORTCUTS.iter().filter(|(k, d)| matches(&[k, d])).collect();
    rsx! {
        div { class: "search-field help-search",
            crate::components::icons::SearchIcon { size: 16 }
            input { r#type: "search", placeholder: "Find it in the instructions", aria_label: "Find it in the instructions", value: "{query}", oninput: move |e| query.set(e.value()) }
        }
        if !q.is_empty() && guides.is_empty() && faq.is_empty() && shortcuts.is_empty() {
            div { class: "sidebar-empty", "There are no suitable instructions" }
        }
        div { class: "guide-hero cw-spotlight cw-stagger",
            Illustration { kind: "chat".to_string() }
            div {
                div { class: "cw-guide-hero-title", "Get started with ChatW" }
                div { class: "cw-guide-hero-sub", "Review the step-by-step introduction at any time for a quick 1-minute familiarization." }
                button { r#type: "button", class: "btn-primary", onclick: move |_| on_replay_tour.call(()), LogoIcon { size: 16 } span { "Review introduction" } }
            }
        }
        if q.is_empty() {
            div { class: "settings-group cw-checklist-wrap cw-stagger",
                div { class: "cw-checklist-head",
                    div { class: "settings-group-title", "Roadmap for newcomers" }
                    span { class: "cw-checklist-counter", "{done_count}/{BEGINNER_CHECKLIST.len()} completed" }
                }
                div { class: "cw-checklist-bar",
                    div { class: "cw-checklist-bar-fill", style: "width: {pct}%;" }
                }
                div { class: "cw-checklist-list",
                    for (id, title, desc) in BEGINNER_CHECKLIST.iter().copied() {
                        {
                            let checked = done_ids.read().iter().any(|x| x == id);
                            rsx! {
                                button {
                                    key: "{id}",
                                    r#type: "button",
                                    class: if checked { "cw-checklist-item done cw-spotlight" } else { "cw-checklist-item cw-spotlight" },
                                    aria_pressed: "{checked}",
                                    onclick: move |_| {
                                        let mut cur = done_ids();
                                        if let Some(pos) = cur.iter().position(|x| x == id) {
                                            cur.remove(pos);
                                        } else {
                                            cur.push(id.to_string());
                                            trigger_spark_celebration();
                                        }
                                        platform::storage_set(CHECKLIST_STORAGE_KEY, &cur.join(","));
                                        done_ids.set(cur);
                                    },
                                    span { class: if checked { "cw-check-circle checked" } else { "cw-check-circle" },
                                        if checked { CheckIcon { size: 14 } }
                                    }
                                    span { class: "cw-checklist-copy",
                                        span { class: "settings-label", "{title}" }
                                        span { class: "settings-sublabel", "{desc}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        div { class: "settings-group",
            div { class: "settings-group-title", "Guide" }
            div { class: "guide-grid",
                for (art, title, steps) in guides.iter().copied().copied() {
                    div { key: "{title}", class: "guide-card cw-spotlight",
                        Illustration { kind: art.to_string() }
                        div { class: "settings-label", "{title}" }
                        ol { class: "guide-steps",
                            for s in steps.iter() { li { key: "{s}", "{s}" } }
                        }
                    }
                }
            }
        }
        div { class: "settings-group",
            div { class: "settings-group-title", "Shortcuts" }
            div { class: "settings-list",
                for (k, d) in shortcuts.iter().copied().copied() {
                    div { key: "{k}", class: "settings-row", span { class: "settings-label", "{d}" } span { class: "kbd", "{k}" } }
                }
            }
        }
        div { class: "settings-group",
            div { class: "settings-group-title", "Frequently asked questions" }
            for (q, a) in faq.iter().copied().copied() {
                details { key: "{q}", class: "faq-item",
                    summary { "{q}" }
                    p { "{a}" }
                }
            }
        }
    }
}
