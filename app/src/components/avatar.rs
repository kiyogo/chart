use dioxus::prelude::*;

use crate::platform;

const PALETTE: [&str; 8] = [
    "#2563EB", // Blue
    "#7C3AED", // Violet
    "#059669", // Emerald
    "#D97706", // Amber
    "#E11D48", // Rose
    "#0891B2", // Cyan
    "#4F46E5", // Indigo
    "#0D9488", // Teal
];

/// Exact JS-compatible 32-bit UTF-16 hash from `web/frontend/src/components/Avatar.tsx`.
pub fn avatar_bg_color(name: &str) -> &'static str {
    let mut hash: i32 = 0;
    for unit in name.encode_utf16() {
        hash = hash
            .wrapping_shl(5)
            .wrapping_sub(hash)
            .wrapping_add(unit as i32);
    }
    let idx = (hash.unsigned_abs() as usize) % PALETTE.len();
    PALETTE[idx]
}

pub fn avatar_initial(name: &str) -> String {
    let trimmed = name.trim();
    match trimmed.chars().next() {
        Some(c) => c.to_uppercase().to_string(),
        None => "?".to_string(),
    }
}

pub fn avatar_font_size(size: u32) -> &'static str {
    if size <= 28 {
        "11px"
    } else if size <= 36 {
        "14px"
    } else if size <= 42 {
        "16px"
    } else {
        "24px"
    }
}

#[component]
pub fn Avatar(
    #[props(default)] name: String,
    #[props(default)] username: Option<String>,
    #[props(default)] user_id: Option<String>,
    #[props(default)] avatar_url: Option<String>,
    #[props(default = 36)] size: u32,
    #[props(default)] class: String,
) -> Element {
    let effective_name = if !name.is_empty() {
        name
    } else {
        username.unwrap_or_default()
    };
    let url_clean = avatar_url.clone().filter(|u| !u.is_empty());
    let init_blob = url_clean
        .as_deref()
        .and_then(|u| platform::get_cached_blob_url_sync(u, "", ""));
    let init_last = if init_blob.is_some() { url_clean.clone() } else { None };
    let mut img_failed = use_signal(|| false);
    let mut blob_src = use_signal(move || init_blob);
    let mut last_url = use_signal(move || init_last);

    if *last_url.read() != url_clean {
        last_url.set(url_clean.clone());
        img_failed.set(false);
        if let Some(ref u) = url_clean {
            if let Some(hit) = platform::get_cached_blob_url_sync(u, "", "") {
                blob_src.set(Some(hit));
            } else if u.starts_with("blob:") || u.starts_with("https://") || u.starts_with("http://") {
                blob_src.set(Some(u.clone()));
            } else {
                blob_src.set(None);
                let target = u.clone();
                spawn(async move {
                    if let Some(burl) = platform::resolve_blob_media_url(&target, "NORMAL").await {
                        if last_url.peek().as_deref() == Some(target.as_str()) {
                            blob_src.set(Some(burl));
                        }
                    } else if last_url.peek().as_deref() == Some(target.as_str()) {
                        img_failed.set(true);
                    }
                });
            }
        } else {
            blob_src.set(None);
        }
    }

    let cls = if class.is_empty() {
        "user-avatar ".to_string()
    } else {
        format!("user-avatar {class}")
    };

    if let Some(resolved) = blob_src()
        && !img_failed() {
            let style = format!(
                "width: {size}px; height: {size}px; min-width: {size}px; min-height: {size}px; object-fit: cover;"
            );
            return rsx! {
                img {
                    src: "{resolved}",
                    alt: "{effective_name}",
                    class: "{cls}",
                    style: "{style}",
                    onerror: move |_| img_failed.set(true),
                }
            };
        }

    if url_clean.is_some() && blob_src().is_none() && !img_failed() {
        let style = format!(
            "width: {size}px; height: {size}px; min-width: {size}px; min-height: {size}px; border-radius: 50%;"
        );
        return rsx! {
            div {
                class: "{cls} skeleton-circle skeleton-shimmer",
                style: "{style}",
                role: "status",
                aria_label: "Loading avatar",
            }
        };
    }

    let bg = avatar_bg_color(&effective_name);
    let fs = avatar_font_size(size);
    let initial = avatar_initial(&effective_name);
    let style = format!(
        "width: {size}px; height: {size}px; min-width: {size}px; min-height: {size}px; background-color: {bg}; font-size: {fs};"
    );

    rsx! {
        div {
            class: "{cls}",
            style: "{style}",
            aria_hidden: "true",
            "{initial}"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avatar_hash_matches_js() {
        // Verified against JS: ((hash << 5) - hash + charCode) | 0
        // "alice" => 92903040 % 8 = 0 => "#2563EB"
        assert_eq!(avatar_bg_color("alice"), "#2563EB");
        // "bob" => 97717 % 8 = 5 => "#0891B2"
        assert_eq!(avatar_bg_color("bob"), "#0891B2");
        assert_eq!(avatar_initial("  alice "), "A");
        assert_eq!(avatar_initial(""), "?");
        assert_eq!(avatar_font_size(28), "11px");
        assert_eq!(avatar_font_size(36), "14px");
        assert_eq!(avatar_font_size(42), "16px");
        assert_eq!(avatar_font_size(64), "24px");
    }
}
