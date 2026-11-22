//! "Kinh doanh": a public page per user (/k/handle) describing them and their
//! products, plus business messaging tools: greeting for new contacts, quick
//! replies (type /shortcut in the composer) and a product catalogue.

use dioxus::prelude::*;
use serde_json::json;

use crate::{
    components::icons::{CopyIcon, GlobeIcon, ImageIcon, PlusIcon, ShareIcon, StoreIcon, TrashIcon},
    models::{BusinessData, BusinessProfile},
    platform,
    services::{library, social},
};

fn biz_error(code: &str) -> &'static str {
    match code {
        "invalid_handle" => "Paths contain 3 to 32 characters: lower case letters, numbers, periods, hyphens, or underscores",
        "handle_taken" => "This link is already taken",
        "invalid_name" => "Invalid display name",
        "invalid_website" => "Website must start with https:// or http://",
        "field_too_long" => "A school is too long",
        "invalid_product_name" => "Invalid product name",
        "product_limit_reached" => "Maximum number of products has been reached",
        "business_profile_required" => "Save the Business page first",
        "invalid_shortcut" => "Shortcuts only contain letters, numbers, or underscores with a maximum of 24 characters",
        "shortcut_taken" => "This shortcut already exists",
        "invalid_text" => "Invalid content",
        _ => "Unable to save, please try again",
    }
}

#[component]
pub fn BusinessSettings() -> Element {
    let mut data = use_signal(BusinessData::default);
    let mut loaded = use_signal(|| false);
    let mut form = use_signal(BusinessProfile::default);
    let mut verify = use_signal(|| false);
    let mut msg = use_signal(|| Option::<(bool, String)>::None);
    let mut origin = use_signal(String::new);
    let mut p_name = use_signal(String::new);
    let mut p_price = use_signal(String::new);
    let mut p_desc = use_signal(String::new);
    let mut p_img = use_signal(|| Option::<String>::None);
    let mut q_short = use_signal(String::new);
    let mut q_text = use_signal(String::new);
    let mut confirm_delete = use_signal(|| false);
    use_hook(move || {
        spawn(async move {
            origin.set(platform::app_origin().await);
            if let Ok(d) = social::business_get().await {
                match d.profile.clone() {
                    Some(p) => form.set(p),
                    None => form.with_mut(|p| p.enabled = true),
                }
                data.set(d);
            }
            loaded.set(true);
        });
    });
    let has_profile = data.read().profile.is_some();
    let f = form();
    let link = format!("{}/k/{}", origin(), f.handle.trim().to_lowercase());
    let save = move |_| {
        let f = form();
        let body = json!({
            "handle": f.handle.trim().to_lowercase(), "name": f.name.trim(), "category": f.category, "description": f.description,
            "address": f.address, "hours": f.hours, "website": f.website, "contact": f.contact, "cover_url": f.cover_url,
            "enabled": f.enabled, "greeting": f.greeting, "away_message": f.away_message, "request_verification": verify(),
        });
        spawn(async move {
            match social::business_save(body).await {
                Ok(d) => {
                    if let Some(p) = d.profile.clone() {
                        form.set(p);
                    }
                    data.set(d);
                    msg.set(Some((false, "Business page saved".into())));
                }
                Err(e) => msg.set(Some((true, biz_error(&e.code).into()))),
            }
        });
    };
    let mut set = move |apply: fn(&mut BusinessProfile, String), v: String| form.with_mut(|p| apply(p, v));
    let opt = |v: String| if v.trim().is_empty() { None } else { Some(v) };

    rsx! {
        div { class: "settings-group",
            div { class: "biz-hero",
                span { class: "biz-hero-icon", StoreIcon { size: 22 } }
                div {
                    div { class: "settings-group-title", "Do business on ChatW" }
                    p { class: "settings-group-desc", "Each Business enabled account has a public link that introduces you, your products, opening hours and contact information. Public pages are indexed by Google and other search engines." }
                }
            }
        }
        if !loaded() {
            div { class: "sidebar-empty", "Loading" }
        } else {
            if let Some((is_err, m)) = msg() { div { class: if is_err { "status-badge error" } else { "status-badge success" }, role: "status", "{m}" } }
            div { class: "settings-group",
                div { class: "settings-group-title", "Public page" }
                div { class: "form-grid",
                    label { class: "form-field", span { "Path" }
                        input { r#type: "text", maxlength: "32", placeholder: "vd: tiemhoa.mai", value: "{f.handle}", oninput: move |e| set(|p, v| p.handle = v, e.value()) }
                    }
                    label { class: "form-field", span { "Display name" }
                        input { r#type: "text", maxlength: "80", placeholder: "Apricot flower shop", value: "{f.name}", oninput: move |e| set(|p, v| p.name = v, e.value()) }
                    }
                    label { class: "form-field", span { "Field" }
                        input { r#type: "text", maxlength: "60", placeholder: "Fresh flowers, gifts", value: "{f.category.clone().unwrap_or_default()}", oninput: move |e| { let v = opt(e.value()); form.with_mut(|p| p.category = v) } }
                    }
                    label { class: "form-field", span { "Opening hours" }
                        input { r#type: "text", maxlength: "120", placeholder: "8:00 - 21:00 daily", value: "{f.hours.clone().unwrap_or_default()}", oninput: move |e| { let v = opt(e.value()); form.with_mut(|p| p.hours = v) } }
                    }
                    label { class: "form-field", span { "Address" }
                        input { r#type: "text", maxlength: "200", value: "{f.address.clone().unwrap_or_default()}", oninput: move |e| { let v = opt(e.value()); form.with_mut(|p| p.address = v) } }
                    }
                    label { class: "form-field", span { "Contact" }
                        input { r#type: "text", maxlength: "120", placeholder: "Phone number or email", value: "{f.contact.clone().unwrap_or_default()}", oninput: move |e| { let v = opt(e.value()); form.with_mut(|p| p.contact = v) } }
                    }
                    label { class: "form-field", span { "Website" }
                        input { r#type: "url", maxlength: "200", placeholder: "https://", value: "{f.website.clone().unwrap_or_default()}", oninput: move |e| { let v = opt(e.value()); form.with_mut(|p| p.website = v) } }
                    }
                    label { class: "form-field wide", span { "Introduction" }
                        textarea { rows: "3", maxlength: "2000", value: "{f.description.clone().unwrap_or_default()}", oninput: move |e| { let v = opt(e.value()); form.with_mut(|p| p.description = v) } }
                    }
                }
                div { class: "inline-form",
                    input { id: "biz-cover-file", r#type: "file", accept: "image/png,image/jpeg,image/webp,image/avif", style: "display: none;",
                        onchange: move |_| { spawn(async move {
                            match platform::read_selected_file_raw("biz-cover-file").await {
                                Ok((bytes, mime)) => match library::upload_media(bytes, &mime).await {
                                    Ok(url) => form.with_mut(|p| p.cover_url = Some(url)),
                                    Err(_) => msg.set(Some((true, "Unable to upload cover photo".into()))),
                                },
                                Err(e) if e == "too_large" => msg.set(Some((true, "Photo larger than 2 MB".into()))),
                                Err(_) => {}
                            }
                        }); },
                    }
                    label { r#for: "biz-cover-file", class: "btn-secondary", role: "button", tabindex: "0", ImageIcon { size: 16 } "Cover photo" }
                    if let Some(c) = f.cover_url.clone().filter(|c| platform::safe_asset_path(c)) {
                        span { class: "banner-preview", style: "background-image: url('{platform::resolve_asset_url(&c)}');" }
                    }
                }
                label { class: "settings-row toggle-row",
                    span { class: "grow", span { class: "settings-label", "The page is active" } span { class: "settings-row-desc", "Turn off to hide the public page without deleting data" } }
                    input { r#type: "checkbox", class: "switch", checked: f.enabled, onchange: move |e| form.with_mut(|p| p.enabled = e.checked()) }
                }
                label { class: "settings-row toggle-row",
                    span { class: "grow", span { class: "settings-label", "Submit a verification request" } span { class: "settings-row-desc", "Verified pages have a ✓ mark and appear in the Search section" } }
                    input { r#type: "checkbox", class: "switch", checked: verify(), onchange: move |e| verify.set(e.checked()) }
                }
                if has_profile {
                    div { class: "invite-link-box",
                        GlobeIcon { size: 16 }
                        code { class: "invite-link-text", "{link}" }
                        button { r#type: "button", class: "btn-secondary", onclick: { let l = link.clone(); move |_| { platform::copy_to_clipboard(&l); msg.set(Some((false, "Link copied".into()))); } }, CopyIcon {} "Copy" }
                        button { r#type: "button", class: "btn-secondary", onclick: { let l = link.clone(); let n = f.name.clone(); move |_| platform::share_link(&n, &l) }, ShareIcon { size: 16 } "Share" }
                        button { r#type: "button", class: "btn-ghost", onclick: { let l = link.clone(); move |_| platform::open_external_url(&l) }, "View page" }
                    }
                }
                button { r#type: "button", class: "btn-primary", disabled: f.handle.trim().is_empty() || f.name.trim().is_empty(), onclick: save,
                    if has_profile { "Save changes" } else { "Turn on Business" }
                }
            }
            if has_profile {
                div { class: "settings-group",
                    div { class: "settings-group-title", "Business messages" }
                    label { class: "form-field", span { "New guest greetings" }
                        textarea { rows: "2", maxlength: "1000", placeholder: "Hello, thank you for sending a message to Mai Flower Shop!", value: "{f.greeting.clone().unwrap_or_default()}", oninput: move |e| { let v = opt(e.value()); form.with_mut(|p| p.greeting = v) } }
                    }
                    label { class: "form-field", span { "After-hours messages" }
                        textarea { rows: "2", maxlength: "1000", value: "{f.away_message.clone().unwrap_or_default()}", oninput: move |e| { let v = opt(e.value()); form.with_mut(|p| p.away_message = v) } }
                    }
                    button { r#type: "button", class: "btn-secondary", onclick: save, "Save the message" }
                }
                div { class: "settings-group",
                    div { class: "settings-group-title", "Quick reply" }
                    p { class: "settings-group-desc", "Type / and the keyboard shortcut in the compose box to quickly insert a reply, for example /gia." }
                    for q in data.read().quick_replies.iter().cloned() {
                        {
                            let id = q.id.clone();
                            rsx! {
                                div { key: "{q.id}", class: "member-row",
                                    code { class: "quick-short", "/{q.shortcut}" }
                                    span { class: "grow", "{q.text}" }
                                    button { r#type: "button", class: "icon-btn danger", aria_label: "Delete quick reply /{q.shortcut}",
                                        onclick: move |_| { let id = id.clone(); spawn(async move { if let Ok(d) = social::business_reply_delete(&id).await { data.set(d); } }); },
                                        TrashIcon { size: 16 }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "form-grid",
                        label { class: "form-field", span { "Shortcuts" } input { r#type: "text", maxlength: "24", placeholder: "gia", value: "{q_short}", oninput: move |e| q_short.set(e.value()) } }
                        label { class: "form-field wide", span { "Content" } textarea { rows: "2", maxlength: "1000", value: "{q_text}", oninput: move |e| q_text.set(e.value()) } }
                    }
                    button { r#type: "button", class: "btn-secondary", disabled: q_short().trim().is_empty() || q_text().trim().is_empty(),
                        onclick: move |_| { let (s, t) = (q_short(), q_text()); spawn(async move {
                            match social::business_reply_add(&s, &t).await {
                                Ok(d) => { data.set(d); q_short.set(String::new()); q_text.set(String::new()); }
                                Err(e) => msg.set(Some((true, biz_error(&e.code).into()))),
                            }
                        }); },
                        PlusIcon { size: 16 } "Add quick replies"
                    }
                }
                div { class: "settings-group",
                    div { class: "settings-group-title", "Products and services" }
                    div { class: "product-grid",
                        for pr in data.read().products.iter().cloned() {
                            {
                                let id = pr.id.clone();
                                rsx! {
                                    div { key: "{pr.id}", class: "product-card",
                                        if let Some(img) = pr.image_url.clone().filter(|u| platform::safe_asset_path(u)) {
                                            img { class: "product-img", src: "{platform::resolve_asset_url(&img)}", alt: "{pr.name}", loading: "lazy" }
                                        }
                                        div { class: "product-name", "{pr.name}" }
                                        if let Some(price) = pr.price.as_ref() { div { class: "product-price", "{price}" } }
                                        if let Some(d) = pr.description.as_ref() { div { class: "product-desc", "{d}" } }
                                        button { r#type: "button", class: "icon-btn danger", aria_label: "Delete {pr.name}",
                                            onclick: move |_| { let id = id.clone(); spawn(async move { if let Ok(d) = social::business_product_delete(&id).await { data.set(d); } }); },
                                            TrashIcon { size: 16 }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "form-grid",
                        label { class: "form-field", span { "Name" } input { r#type: "text", maxlength: "120", value: "{p_name}", oninput: move |e| p_name.set(e.value()) } }
                        label { class: "form-field", span { "Price" } input { r#type: "text", maxlength: "40", placeholder: "150,000 VND", value: "{p_price}", oninput: move |e| p_price.set(e.value()) } }
                        label { class: "form-field wide", span { "Description" } textarea { rows: "2", maxlength: "1000", value: "{p_desc}", oninput: move |e| p_desc.set(e.value()) } }
                    }
                    div { class: "inline-form",
                        input { id: "biz-product-file", r#type: "file", accept: "image/png,image/jpeg,image/webp,image/avif", style: "display: none;",
                            onchange: move |_| { spawn(async move {
                                if let Ok((bytes, mime)) = platform::read_selected_file_raw("biz-product-file").await {
                                    match library::upload_media(bytes, &mime).await {
                                        Ok(url) => p_img.set(Some(url)),
                                        Err(_) => msg.set(Some((true, "Unable to upload product images".into()))),
                                    }
                                }
                            }); },
                        }
                        label { r#for: "biz-product-file", class: "btn-secondary", role: "button", tabindex: "0", ImageIcon { size: 16 } if p_img().is_some() { "Photo selected" } else { "Product photo" } }
                        button { r#type: "button", class: "btn-primary", disabled: p_name().trim().is_empty(),
                            onclick: move |_| {
                                let body = json!({ "name": p_name().trim(), "price": opt(p_price()), "description": opt(p_desc()), "image_url": p_img() });
                                spawn(async move {
                                    match social::business_product_add(body).await {
                                        Ok(d) => { data.set(d); p_name.set(String::new()); p_price.set(String::new()); p_desc.set(String::new()); p_img.set(None); }
                                        Err(e) => msg.set(Some((true, biz_error(&e.code).into()))),
                                    }
                                });
                            },
                            PlusIcon { size: 16 } "Add products"
                        }
                    }
                }
                div { class: "danger-zone",
                    div { class: "settings-group-title", "Turn off Business completely" }
                    p { class: "settings-group-desc", "Delete public pages, products, and quick replies. Cannot be undone." }
                    if confirm_delete() {
                        div { class: "inline-form",
                            button { r#type: "button", class: "btn-secondary", onclick: move |_| confirm_delete.set(false), "Cancel" }
                            button { r#type: "button", class: "btn-danger", onclick: move |_| { confirm_delete.set(false); spawn(async move {
                                if social::business_delete().await.is_ok() { data.set(BusinessData::default()); form.set(BusinessProfile::default()); msg.set(Some((false, "Business page removed".into()))); }
                            }); }, TrashIcon { size: 16 } "Delete permanently" }
                        }
                    } else {
                        button { r#type: "button", class: "btn-danger-outline", onclick: move |_| confirm_delete.set(true), "Delete the Business page" }
                    }
                }
            }
        }
    }
}
