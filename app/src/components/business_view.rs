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
        "invalid_handle" => "Đường dẫn gồm 3 đến 32 ký tự: chữ thường không dấu, số, dấu chấm, gạch ngang hoặc gạch dưới",
        "handle_taken" => "Đường dẫn này đã có người dùng",
        "invalid_name" => "Tên hiển thị không hợp lệ",
        "invalid_website" => "Website phải bắt đầu bằng https:// hoặc http://",
        "field_too_long" => "Một trường quá dài",
        "invalid_product_name" => "Tên sản phẩm không hợp lệ",
        "product_limit_reached" => "Đã đạt số sản phẩm tối đa",
        "business_profile_required" => "Hãy lưu trang Kinh doanh trước",
        "invalid_shortcut" => "Phím tắt chỉ gồm chữ không dấu, số hoặc gạch dưới tối đa 24 ký tự",
        "shortcut_taken" => "Phím tắt này đã có",
        "invalid_text" => "Nội dung không hợp lệ",
        _ => "Không lưu được, vui lòng thử lại",
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
                    msg.set(Some((false, "Đã lưu trang Kinh doanh".into())));
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
                    div { class: "settings-group-title", "Kinh doanh trên ChatW" }
                    p { class: "settings-group-desc", "Mỗi tài khoản bật Kinh doanh có một đường dẫn công khai giới thiệu bạn, sản phẩm, giờ mở cửa và cách liên hệ. Trang công khai được Google và các công cụ tìm kiếm lập chỉ mục." }
                }
            }
        }
        if !loaded() {
            div { class: "sidebar-empty", "Đang tải" }
        } else {
            if let Some((is_err, m)) = msg() { div { class: if is_err { "status-badge error" } else { "status-badge success" }, role: "status", "{m}" } }
            div { class: "settings-group",
                div { class: "settings-group-title", "Trang công khai" }
                div { class: "form-grid",
                    label { class: "form-field", span { "Đường dẫn" }
                        input { r#type: "text", maxlength: "32", placeholder: "vd: tiemhoa.mai", value: "{f.handle}", oninput: move |e| set(|p, v| p.handle = v, e.value()) }
                    }
                    label { class: "form-field", span { "Tên hiển thị" }
                        input { r#type: "text", maxlength: "80", placeholder: "Tiệm hoa Mai", value: "{f.name}", oninput: move |e| set(|p, v| p.name = v, e.value()) }
                    }
                    label { class: "form-field", span { "Lĩnh vực" }
                        input { r#type: "text", maxlength: "60", placeholder: "Hoa tươi, quà tặng", value: "{f.category.clone().unwrap_or_default()}", oninput: move |e| { let v = opt(e.value()); form.with_mut(|p| p.category = v) } }
                    }
                    label { class: "form-field", span { "Giờ mở cửa" }
                        input { r#type: "text", maxlength: "120", placeholder: "8:00 - 21:00 hằng ngày", value: "{f.hours.clone().unwrap_or_default()}", oninput: move |e| { let v = opt(e.value()); form.with_mut(|p| p.hours = v) } }
                    }
                    label { class: "form-field", span { "Địa chỉ" }
                        input { r#type: "text", maxlength: "200", value: "{f.address.clone().unwrap_or_default()}", oninput: move |e| { let v = opt(e.value()); form.with_mut(|p| p.address = v) } }
                    }
                    label { class: "form-field", span { "Liên hệ" }
                        input { r#type: "text", maxlength: "120", placeholder: "Số điện thoại hoặc email", value: "{f.contact.clone().unwrap_or_default()}", oninput: move |e| { let v = opt(e.value()); form.with_mut(|p| p.contact = v) } }
                    }
                    label { class: "form-field", span { "Website" }
                        input { r#type: "url", maxlength: "200", placeholder: "https://", value: "{f.website.clone().unwrap_or_default()}", oninput: move |e| { let v = opt(e.value()); form.with_mut(|p| p.website = v) } }
                    }
                    label { class: "form-field wide", span { "Giới thiệu" }
                        textarea { rows: "3", maxlength: "2000", value: "{f.description.clone().unwrap_or_default()}", oninput: move |e| { let v = opt(e.value()); form.with_mut(|p| p.description = v) } }
                    }
                }
                div { class: "inline-form",
                    input { id: "biz-cover-file", r#type: "file", accept: "image/png,image/jpeg,image/webp,image/avif", style: "display: none;",
                        onchange: move |_| { spawn(async move {
                            match platform::read_selected_file_raw("biz-cover-file").await {
                                Ok((bytes, mime)) => match library::upload_media(bytes, &mime).await {
                                    Ok(url) => form.with_mut(|p| p.cover_url = Some(url)),
                                    Err(_) => msg.set(Some((true, "Không tải ảnh bìa lên được".into()))),
                                },
                                Err(e) if e == "too_large" => msg.set(Some((true, "Ảnh lớn hơn 2 MB".into()))),
                                Err(_) => {}
                            }
                        }); },
                    }
                    label { r#for: "biz-cover-file", class: "btn-secondary", role: "button", tabindex: "0", ImageIcon { size: 16 } "Ảnh bìa" }
                    if let Some(c) = f.cover_url.clone().filter(|c| platform::safe_asset_path(c)) {
                        span { class: "banner-preview", style: "background-image: url('{platform::resolve_asset_url(&c)}');" }
                    }
                }
                label { class: "settings-row toggle-row",
                    span { class: "grow", span { class: "settings-label", "Trang đang hoạt động" } span { class: "settings-row-desc", "Tắt để ẩn trang công khai mà không xóa dữ liệu" } }
                    input { r#type: "checkbox", class: "switch", checked: f.enabled, onchange: move |e| form.with_mut(|p| p.enabled = e.checked()) }
                }
                label { class: "settings-row toggle-row",
                    span { class: "grow", span { class: "settings-label", "Gửi yêu cầu xác minh" } span { class: "settings-row-desc", "Trang đã xác minh có dấu ✓ và hiện trong mục Tìm kiếm" } }
                    input { r#type: "checkbox", class: "switch", checked: verify(), onchange: move |e| verify.set(e.checked()) }
                }
                if has_profile {
                    div { class: "invite-link-box",
                        GlobeIcon { size: 16 }
                        code { class: "invite-link-text", "{link}" }
                        button { r#type: "button", class: "btn-secondary", onclick: { let l = link.clone(); move |_| { platform::copy_to_clipboard(&l); msg.set(Some((false, "Đã sao chép đường dẫn".into()))); } }, CopyIcon {} "Sao chép" }
                        button { r#type: "button", class: "btn-secondary", onclick: { let l = link.clone(); let n = f.name.clone(); move |_| platform::share_link(&n, &l) }, ShareIcon { size: 16 } "Chia sẻ" }
                        button { r#type: "button", class: "btn-ghost", onclick: { let l = link.clone(); move |_| platform::open_external_url(&l) }, "Xem trang" }
                    }
                }
                button { r#type: "button", class: "btn-primary", disabled: f.handle.trim().is_empty() || f.name.trim().is_empty(), onclick: save,
                    if has_profile { "Lưu thay đổi" } else { "Bật Kinh doanh" }
                }
            }
            if has_profile {
                div { class: "settings-group",
                    div { class: "settings-group-title", "Tin nhắn kinh doanh" }
                    label { class: "form-field", span { "Lời chào khách mới" }
                        textarea { rows: "2", maxlength: "1000", placeholder: "Chào bạn, cảm ơn đã nhắn cho Tiệm hoa Mai!", value: "{f.greeting.clone().unwrap_or_default()}", oninput: move |e| { let v = opt(e.value()); form.with_mut(|p| p.greeting = v) } }
                    }
                    label { class: "form-field", span { "Tin nhắn ngoài giờ" }
                        textarea { rows: "2", maxlength: "1000", value: "{f.away_message.clone().unwrap_or_default()}", oninput: move |e| { let v = opt(e.value()); form.with_mut(|p| p.away_message = v) } }
                    }
                    button { r#type: "button", class: "btn-secondary", onclick: save, "Lưu tin nhắn" }
                }
                div { class: "settings-group",
                    div { class: "settings-group-title", "Trả lời nhanh" }
                    p { class: "settings-group-desc", "Gõ / và phím tắt trong ô soạn tin để chèn nhanh câu trả lời, ví dụ /gia." }
                    for q in data.read().quick_replies.iter().cloned() {
                        {
                            let id = q.id.clone();
                            rsx! {
                                div { key: "{q.id}", class: "member-row",
                                    code { class: "quick-short", "/{q.shortcut}" }
                                    span { class: "grow", "{q.text}" }
                                    button { r#type: "button", class: "icon-btn danger", aria_label: "Xóa trả lời nhanh /{q.shortcut}",
                                        onclick: move |_| { let id = id.clone(); spawn(async move { if let Ok(d) = social::business_reply_delete(&id).await { data.set(d); } }); },
                                        TrashIcon { size: 16 }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "form-grid",
                        label { class: "form-field", span { "Phím tắt" } input { r#type: "text", maxlength: "24", placeholder: "gia", value: "{q_short}", oninput: move |e| q_short.set(e.value()) } }
                        label { class: "form-field wide", span { "Nội dung" } textarea { rows: "2", maxlength: "1000", value: "{q_text}", oninput: move |e| q_text.set(e.value()) } }
                    }
                    button { r#type: "button", class: "btn-secondary", disabled: q_short().trim().is_empty() || q_text().trim().is_empty(),
                        onclick: move |_| { let (s, t) = (q_short(), q_text()); spawn(async move {
                            match social::business_reply_add(&s, &t).await {
                                Ok(d) => { data.set(d); q_short.set(String::new()); q_text.set(String::new()); }
                                Err(e) => msg.set(Some((true, biz_error(&e.code).into()))),
                            }
                        }); },
                        PlusIcon { size: 16 } "Thêm trả lời nhanh"
                    }
                }
                div { class: "settings-group",
                    div { class: "settings-group-title", "Sản phẩm và dịch vụ" }
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
                                        button { r#type: "button", class: "icon-btn danger", aria_label: "Xóa {pr.name}",
                                            onclick: move |_| { let id = id.clone(); spawn(async move { if let Ok(d) = social::business_product_delete(&id).await { data.set(d); } }); },
                                            TrashIcon { size: 16 }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "form-grid",
                        label { class: "form-field", span { "Tên" } input { r#type: "text", maxlength: "120", value: "{p_name}", oninput: move |e| p_name.set(e.value()) } }
                        label { class: "form-field", span { "Giá" } input { r#type: "text", maxlength: "40", placeholder: "150.000đ", value: "{p_price}", oninput: move |e| p_price.set(e.value()) } }
                        label { class: "form-field wide", span { "Mô tả" } textarea { rows: "2", maxlength: "1000", value: "{p_desc}", oninput: move |e| p_desc.set(e.value()) } }
                    }
                    div { class: "inline-form",
                        input { id: "biz-product-file", r#type: "file", accept: "image/png,image/jpeg,image/webp,image/avif", style: "display: none;",
                            onchange: move |_| { spawn(async move {
                                if let Ok((bytes, mime)) = platform::read_selected_file_raw("biz-product-file").await {
                                    match library::upload_media(bytes, &mime).await {
                                        Ok(url) => p_img.set(Some(url)),
                                        Err(_) => msg.set(Some((true, "Không tải ảnh sản phẩm lên được".into()))),
                                    }
                                }
                            }); },
                        }
                        label { r#for: "biz-product-file", class: "btn-secondary", role: "button", tabindex: "0", ImageIcon { size: 16 } if p_img().is_some() { "Đã chọn ảnh" } else { "Ảnh sản phẩm" } }
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
                            PlusIcon { size: 16 } "Thêm sản phẩm"
                        }
                    }
                }
                div { class: "danger-zone",
                    div { class: "settings-group-title", "Tắt hẳn Kinh doanh" }
                    p { class: "settings-group-desc", "Xóa trang công khai, sản phẩm và trả lời nhanh. Không thể hoàn tác." }
                    if confirm_delete() {
                        div { class: "inline-form",
                            button { r#type: "button", class: "btn-secondary", onclick: move |_| confirm_delete.set(false), "Hủy" }
                            button { r#type: "button", class: "btn-danger", onclick: move |_| { confirm_delete.set(false); spawn(async move {
                                if social::business_delete().await.is_ok() { data.set(BusinessData::default()); form.set(BusinessProfile::default()); msg.set(Some((false, "Đã xóa trang Kinh doanh".into()))); }
                            }); }, TrashIcon { size: 16 } "Xóa vĩnh viễn" }
                        }
                    } else {
                        button { r#type: "button", class: "btn-danger-outline", onclick: move |_| confirm_delete.set(true), "Xóa trang Kinh doanh" }
                    }
                }
            }
        }
    }
}
