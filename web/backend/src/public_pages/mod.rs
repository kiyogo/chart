//! Server-rendered public pages (no login, SEO friendly):
//! * `/k/{handle}` — business page ("Kinh doanh")
//! * `/c/{server_id}` — verified public community that opted into search indexing
//! * `/sitemap.xml` — includes the pages above
//! * `/admin` — the separate admin web (static assets)

use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use uuid::Uuid;

use crate::auth::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/k/{handle}", get(business_page))
        .route("/c/{server_id}", get(community_page))
        .route("/sitemap.xml", get(sitemap))
        .route("/admin", get(admin_index))
        .route("/admin/", get(admin_index))
        .route("/admin/admin.js", get(admin_js))
        .route("/admin/admin.css", get(admin_css))
}

pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#39;"),
            _ => o.push(c),
        }
    }
    o
}

const PAGE_CSS: &str = r#"
*{box-sizing:border-box;margin:0;padding:0}body{font-family:Inter,"Segoe UI",Roboto,"Noto Sans",Arial,sans-serif;background:#E9EDF3;color:#171C26;line-height:1.5}
.wrap{max-width:920px;margin:0 auto;padding:16px}.card{background:#fff;border-radius:18px;overflow:hidden;margin-bottom:12px}
.cover{height:220px;background:linear-gradient(135deg,#2764E7,#7C3AED);background-size:cover;background-position:center}
.head{padding:0 24px 24px;display:flex;gap:16px;align-items:flex-end;margin-top:-44px;flex-wrap:wrap}.avatar{width:96px;height:96px;border-radius:50%;border:5px solid #fff;background:#2764E7;color:#fff;display:flex;align-items:center;justify-content:center;font-size:36px;font-weight:700;overflow:hidden}.avatar img{width:100%;height:100%;object-fit:cover}
h1{font-size:28px;letter-spacing:-.02em}.muted{color:#5B6475;font-size:14px}.chip{display:inline-block;font-size:12px;font-weight:600;padding:2px 10px;border-radius:999px;background:#EAF0FD;color:#1B4FC4;margin-right:6px}
.chip.ok{background:#E7F6EE;color:#0E7A4D}.section{padding:20px 24px}.section h2{font-size:18px;margin-bottom:10px}.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(200px,1fr));gap:12px}
.product{background:#F3F5F8;border-radius:14px;overflow:hidden}.product img{width:100%;aspect-ratio:4/3;object-fit:cover;display:block}.product div{padding:10px 12px}.price{font-weight:700;color:#1B4FC4}
.btn{display:inline-flex;align-items:center;gap:8px;background:#2764E7;color:#fff;text-decoration:none;font-weight:600;padding:10px 18px;border-radius:12px}.row{display:flex;gap:10px;flex-wrap:wrap}
.msg{display:flex;gap:10px;padding:8px 0}.msg b{display:block;font-size:14px}.msg p{white-space:pre-wrap;overflow-wrap:anywhere;font-size:15px}.ch{font-weight:700;margin:16px 0 4px;color:#5B6475}
footer{text-align:center;color:#868E9C;font-size:13px;padding:16px}
@media (prefers-color-scheme:dark){body{background:#0C0E12;color:#E8EAED}.card{background:#191C22}.product{background:#21252C}.muted{color:#A4ABB6}.avatar{border-color:#191C22}}
"#;

fn page(title: &str, description: &str, canonical: &str, image: Option<&str>, json_ld: &str, robots: &str, body: &str) -> Html<String> {
    let img = image.map(esc).unwrap_or_else(|| "/og-image.svg".into());
    Html(format!(
        r#"<!DOCTYPE html><html lang="vi"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>{t}</title><meta name="description" content="{d}"><meta name="robots" content="{robots}"><link rel="canonical" href="{c}">
<meta property="og:type" content="website"><meta property="og:title" content="{t}"><meta property="og:description" content="{d}"><meta property="og:url" content="{c}"><meta property="og:image" content="{img}">
<meta name="twitter:card" content="summary_large_image"><meta name="twitter:title" content="{t}"><meta name="twitter:description" content="{d}"><meta name="twitter:image" content="{img}">
<link rel="icon" href="/logo.svg" type="image/svg+xml"><style>{css}</style><script type="application/ld+json">{ld}</script></head>
<body><div class="wrap">{body}<footer>Trang được tạo bằng ChatW · <a href="/">Mở ChatW</a></footer></div></body></html>"#,
        t = esc(title),
        d = esc(description),
        c = esc(canonical),
        css = PAGE_CSS,
        ld = json_ld.replace("</", "<\\/"),
    ))
}

fn not_found() -> Response {
    (StatusCode::NOT_FOUND, Html("<!DOCTYPE html><meta charset=utf-8><title>Không tìm thấy</title><p style='font-family:sans-serif;padding:24px'>Không tìm thấy trang này.</p>")).into_response()
}

async fn business_page(State(state): State<AppState>, Path(handle): Path<String>) -> Response {
    let Ok(Some((_uid, username, p, products))) = crate::business::public_profile(&state.db, &handle).await else { return not_found() };
    let origin = &state.config.public_origin;
    let canonical = format!("{origin}/k/{}", p.handle);
    let desc = p.description.clone().unwrap_or_else(|| format!("{} trên ChatW", p.name));
    let short: String = desc.chars().take(160).collect();
    let cover = p.cover_url.as_deref().map(|c| format!("{origin}{c}"));
    let ld = serde_json::json!({
        "@context": "https://schema.org", "@type": "Organization", "name": p.name, "url": canonical,
        "description": short, "address": p.address, "sameAs": p.website.clone().into_iter().collect::<Vec<_>>(),
        "makesOffer": products.iter().map(|x| serde_json::json!({"@type": "Offer", "name": x.name, "price": x.price, "description": x.description})).collect::<Vec<_>>()
    })
    .to_string();
    let mut body = String::new();
    body.push_str(r#"<div class="card">"#);
    match &p.cover_url {
        Some(c) => body.push_str(&format!(r#"<div class="cover" style="background-image:url('{}')"></div>"#, esc(c))),
        None => body.push_str(r#"<div class="cover"></div>"#),
    }
    let initial = p.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
    body.push_str(&format!(
        r#"<div class="head"><div class="avatar">{}</div><div><h1>{}</h1><div class="muted">@{}{}</div></div></div>"#,
        esc(&initial),
        esc(&p.name),
        esc(&p.handle),
        p.category.as_deref().map(|c| format!(" · {}", esc(c))).unwrap_or_default()
    ));
    body.push_str(r#"<div class="section">"#);
    if p.verification_status == "verified" {
        body.push_str(r#"<span class="chip ok">Đã xác minh</span>"#);
    }
    body.push_str(r#"<span class="chip">Kinh doanh trên ChatW</span>"#);
    if let Some(d) = &p.description {
        body.push_str(&format!(r#"<p style="margin-top:12px;white-space:pre-wrap">{}</p>"#, esc(d)));
    }
    body.push_str(r#"<div class="row" style="margin-top:16px">"#);
    body.push_str(&format!(r#"<a class="btn" href="/?dm={}">Nhắn tin trên ChatW</a>"#, esc(&username)));
    if let Some(w) = &p.website {
        body.push_str(&format!(r#"<a class="btn" style="background:#334155" href="{}" rel="nofollow noopener" target="_blank">Website</a>"#, esc(w)));
    }
    body.push_str("</div>");
    for (label, v) in [("Địa chỉ", &p.address), ("Giờ mở cửa", &p.hours), ("Liên hệ", &p.contact)] {
        if let Some(v) = v {
            body.push_str(&format!(r#"<p class="muted" style="margin-top:8px"><b>{label}:</b> {}</p>"#, esc(v)));
        }
    }
    body.push_str("</div></div>");
    if !products.is_empty() {
        body.push_str(r#"<div class="card"><div class="section"><h2>Sản phẩm và dịch vụ</h2><div class="grid">"#);
        for x in &products {
            body.push_str(r#"<div class="product">"#);
            if let Some(img) = &x.image_url {
                body.push_str(&format!(r#"<img src="{}" alt="{}" loading="lazy">"#, esc(img), esc(&x.name)));
            }
            body.push_str(&format!(
                r#"<div><b>{}</b>{}{}</div></div>"#,
                esc(&x.name),
                x.price.as_deref().map(|pr| format!(r#"<div class="price">{}</div>"#, esc(pr))).unwrap_or_default(),
                x.description.as_deref().map(|d| format!(r#"<p class="muted">{}</p>"#, esc(d))).unwrap_or_default()
            ));
        }
        body.push_str("</div></div></div>");
    }
    page(&format!("{} · ChatW", p.name), &short, &canonical, cover.as_deref(), &ld, "index, follow", &body).into_response()
}

async fn community_page(State(state): State<AppState>, Path(server_id): Path<Uuid>) -> Response {
    let Ok(Some(s)) = sqlx::query!(
        "SELECT id, name, description, icon_url, banner_url FROM servers WHERE id = $1 AND is_public AND verification_status = 'verified' AND public_index",
        server_id
    )
    .fetch_optional(&state.db)
    .await
    else {
        return not_found();
    };
    let channels = sqlx::query!(
        "SELECT id, name FROM channels WHERE server_id = $1 AND channel_type IN ('text', 'announcement') ORDER BY position LIMIT 10",
        server_id
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    let origin = &state.config.public_origin;
    let canonical = format!("{origin}/c/{server_id}");
    let desc = s.description.clone().unwrap_or_else(|| format!("Cộng đồng {} trên ChatW", s.name));
    let short: String = desc.chars().take(160).collect();
    let ld = serde_json::json!({ "@context": "https://schema.org", "@type": "DiscussionForumPosting", "headline": s.name, "url": canonical, "text": short }).to_string();
    let mut body = String::new();
    body.push_str(r#"<div class="card"><div class="cover"></div>"#);
    let initial = s.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
    body.push_str(&format!(r#"<div class="head"><div class="avatar">{}</div><div><h1>{}</h1><div class="muted">{}</div></div></div>"#, esc(&initial), esc(&s.name), esc(&short)));
    body.push_str(r#"<div class="section"><span class="chip ok">Cộng đồng công khai đã xác minh</span></div></div>"#);
    for ch in channels {
        let msgs = sqlx::query!(
            r#"SELECT m.content, m.created_at, u.username AS "username!" FROM channel_messages m JOIN users u ON u.id = m.sender_id
               WHERE m.channel_id = $1 ORDER BY m.created_at DESC LIMIT 30"#,
            ch.id
        )
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
        if msgs.is_empty() {
            continue;
        }
        body.push_str(&format!(r#"<div class="card"><div class="section"><div class="ch"># {}</div>"#, esc(&ch.name)));
        for m in msgs.iter().rev() {
            body.push_str(&format!(
                r#"<div class="msg"><div class="avatar" style="width:36px;height:36px;border:0;font-size:15px">{}</div><div><b>{} <span class="muted">{}</span></b><p>{}</p></div></div>"#,
                esc(&m.username.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()),
                esc(&m.username),
                m.created_at.format("%d/%m/%Y"),
                esc(&m.content)
            ));
        }
        body.push_str("</div></div>");
    }
    let _ = (&s.icon_url, &s.banner_url);
    page(&format!("{} · Cộng đồng ChatW", s.name), &short, &canonical, None, &ld, "index, follow", &body).into_response()
}

async fn sitemap(State(state): State<AppState>) -> Response {
    let origin = state.config.public_origin.clone();
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    xml.push_str(&format!("  <url><loc>{}/</loc><changefreq>daily</changefreq><priority>1.0</priority></url>\n", esc(&origin)));
    if let Ok(rows) = sqlx::query!("SELECT handle, updated_at FROM business_profiles WHERE enabled ORDER BY updated_at DESC LIMIT 5000")
        .fetch_all(&state.db)
        .await
    {
        for r in rows {
            xml.push_str(&format!(
                "  <url><loc>{}/k/{}</loc><lastmod>{}</lastmod><changefreq>weekly</changefreq></url>\n",
                esc(&origin),
                esc(&r.handle),
                r.updated_at.format("%Y-%m-%d")
            ));
        }
    }
    if let Ok(rows) = sqlx::query!("SELECT id FROM servers WHERE is_public AND verification_status = 'verified' AND public_index LIMIT 5000")
        .fetch_all(&state.db)
        .await
    {
        for r in rows {
            xml.push_str(&format!("  <url><loc>{}/c/{}</loc><changefreq>daily</changefreq></url>\n", esc(&origin), r.id));
        }
    }
    xml.push_str("</urlset>\n");
    ([(header::CONTENT_TYPE, "application/xml; charset=utf-8")], xml).into_response()
}

async fn admin_index() -> Response {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8"), (header::HeaderName::from_static("x-robots-tag"), "noindex, nofollow")],
        include_str!("../../../admin/index.html"),
    )
        .into_response()
}

async fn admin_js() -> Response {
    ([(header::CONTENT_TYPE, "text/javascript; charset=utf-8")], include_str!("../../../admin/admin.js")).into_response()
}

async fn admin_css() -> Response {
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], include_str!("../../../admin/admin.css")).into_response()
}
