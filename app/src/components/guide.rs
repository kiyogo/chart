//! Illustrations, multi-step dialogs scaffolding, the welcome tour and the help page.

use dioxus::prelude::*;

use crate::components::icons::{CloseIcon, LogoIcon};

/// Original duotone illustrations used at the top of dialogs.
#[component]
pub fn Illustration(kind: String) -> Element {
    rsx! {
        svg { class: "illustration", view_box: "0 0 240 120", "aria-hidden": "true",
            defs {
                linearGradient { id: "ill-g", x1: "0", y1: "0", x2: "1", y2: "1",
                    stop { offset: "0", stop_color: "var(--accent)" }
                    stop { offset: "1", stop_color: "#A78BFA" }
                }
            }
            circle { cx: "120", cy: "62", r: "50", fill: "url(#ill-g)", opacity: ".14" }
            path { d: "M40 30l3 7 7 3-7 3-3 7-3-7-7-3 7-3z", fill: "#FBBF24" }
            path { d: "M200 76l2.5 5.5 5.5 2.5-5.5 2.5-2.5 5.5-2.5-5.5-5.5-2.5 5.5-2.5z", fill: "#60A5FA" }
            circle { cx: "190", cy: "28", r: "4", fill: "#F472B6" }
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

const TOUR: [(&str, &str, &str); 6] = [
    ("chat", "Chào mừng đến ChatW", "Nhắn tin riêng tư, cộng đồng nhiều kênh, gọi thoại và video. Cùng xem nhanh những điều bạn có thể làm."),
    ("shield", "Tin nhắn trực tiếp được mã hóa đầu cuối", "Khóa riêng tư nằm trên thiết bị của bạn. Máy chủ chỉ chuyển tiếp bản mã. So sánh vân tay khóa trong bảng Thông tin để xác minh bạn bè."),
    ("user", "Hồ sơ mang dấu ấn riêng", "Màu chủ đề, khung avatar, hiệu ứng tên, bảng tên, hiệu ứng động. Bạn còn có thể tự vẽ hoặc tải ảnh lên để trang trí."),
    ("library", "Emoji, sticker và GIF của bạn", "Mở bảng chọn ở ô soạn tin. Thêm emoji, sticker, GIF riêng trong Cài đặt › Thư viện và dùng ở mọi cuộc trò chuyện."),
    ("devices", "Kiểm soát thiết bị", "Xem các thiết bị đang đăng nhập và đăng xuất từ xa trong Cài đặt › Tài khoản của tôi."),
    ("chat", "Mẹo: Ctrl + K", "Nhấn Ctrl + K ở bất kỳ đâu để nhảy tới một cuộc trò chuyện, nhóm hoặc cộng đồng."),
];

#[component]
pub fn WelcomeTour(on_close: EventHandler<()>) -> Element {
    let mut step = use_signal(|| 0usize);
    let (art, title, body) = TOUR[step()];
    let last = step() + 1 == TOUR.len();
    rsx! {
        div { class: "modal-overlay", onclick: move |_| on_close.call(()),
            div { class: "modal-dialog story-dialog", role: "dialog", aria_modal: "true", aria_label: "Giới thiệu ChatW",
                onclick: move |e| e.stop_propagation(),
                onkeydown: move |e| match e.key() {
                    Key::Escape => on_close.call(()),
                    Key::ArrowRight if !last => step.set(step() + 1),
                    Key::ArrowLeft if step() > 0 => step.set(step() - 1),
                    _ => {}
                },
                div { class: "story-art",
                    button { r#type: "button", class: "icon-btn story-close", aria_label: "Bỏ qua giới thiệu", onclick: move |_| on_close.call(()), CloseIcon {} }
                    Illustration { kind: art.to_string() }
                }
                div { class: "story-body",
                    h2 { class: "story-title", "{title}" }
                    p { class: "story-text", "{body}" }
                    div { class: "story-dots", role: "tablist",
                        for i in 0..TOUR.len() {
                            button { key: "{i}", r#type: "button", class: if i == step() { "story-dot active" } else { "story-dot" }, aria_label: "Bước {i + 1}", onclick: move |_| step.set(i) }
                        }
                    }
                }
                div { class: "story-actions",
                    if step() > 0 {
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| step.set(step() - 1), "Quay lại" }
                    } else {
                        button { r#type: "button", class: "btn-secondary", onclick: move |_| on_close.call(()), "Bỏ qua" }
                    }
                    button { r#type: "button", class: "btn-primary", onclick: move |_| if last { on_close.call(()) } else { step.set(step() + 1) },
                        if last { "Bắt đầu" } else { "Tiếp tục" }
                    }
                }
            }
        }
    }
}

const SHORTCUTS: [(&str, &str); 11] = [
    ("Ctrl + K", "Tìm nhanh cuộc trò chuyện, nhóm, cộng đồng"),
    ("@ rồi gõ tên", "Nhắc tên một người; Tab hoặc Enter để chèn"),
    (": rồi gõ tên emoji", "Gợi ý emoji, ví dụ :fire"),
    ("/ rồi gõ phím tắt", "Chèn trả lời nhanh của Kinh doanh"),
    ("Bấm đúp cạnh tin nhắn", "Trả lời tin nhắn đó"),
    ("Enter", "Gửi tin nhắn"),
    ("Shift + Enter", "Xuống dòng"),
    ("Esc", "Đóng bảng chọn, hộp thoại, hồ sơ"),
    ("↑ ↓ ← →", "Di chuyển trong bảng emoji"),
    ("Ctrl + K rồi gõ \"Cài đặt\"", "Mở cài đặt"),
    ("Tab", "Di chuyển giữa các nút"),
];

const GUIDES: [(&str, &str, &[&str]); 16] = [
    ("user", "Avatar Studio: avatar xoay và 40 hiệu ứng", &[
        "Mở hồ sơ của bạn › Chỉnh sửa hồ sơ › thẻ Avatar Studio.",
        "Chọn một hiệu ứng (Spin, Orbit, Hearts, Glitch...), chỉnh Tốc độ, Cường độ, Màu và Hướng xoay.",
        "Kích hoạt: Luôn chạy, Khi rê chuột hoặc Khi nhấp. Đặt Số lần lặp nếu không muốn chạy mãi.",
        "Bấm Lưu. Bạn bè thấy hiệu ứng trên hồ sơ, danh sách trò chuyện và danh sách bạn bè.",
    ]),
    ("chat", "Demo Gallery: thử trước khi dùng", &[
        "Cài đặt › Demo Gallery.",
        "Chọn nhóm: Avatar, Hiệu ứng tên, Bảng tên, Hiệu ứng hồ sơ hoặc Hình nền chat.",
        "Đổi chế độ xem Desktop / Mobile / Gọn, hoặc xem như khi giảm chuyển động.",
        "Bấm Áp dụng: thay đổi được lưu thật vào hồ sơ hoặc cài đặt.",
    ]),
    ("user", "Thông tin hồ sơ và quyền xem", &[
        "Chỉnh sửa hồ sơ › Thông tin: thêm danh xưng, công việc, nơi sống, website, sở thích.",
        "Mỗi mục có quyền riêng: Mọi người, Bạn bè hoặc Chỉ mình tôi. Máy chủ chỉ gửi mục đó cho người được phép.",
        "Trạng thái tùy chỉnh có thể tự xóa sau 30 phút, 1 giờ, 4 giờ, hết hôm nay hoặc 7 ngày.",
        "Ảnh bìa GIF hoặc WebP động giữ nguyên chuyển động (tối đa 8 MB).",
    ]),
    ("chat", "Sticker, GIF và emoji yêu thích", &[
        "Rê chuột lên sticker hoặc GIF trong bảng chọn và bấm ☆ để thêm vào Favorites.",
        "Thẻ Recently Used giữ 24 sticker bạn gửi gần nhất.",
        "Bấm vào sticker trong tin nhắn: Add to Favorite Stickers, Send Again hoặc Find Similar Stickers.",
        "Emoji: nhấp chuột phải vào emoji, hoặc bấm ☆ ở chân bảng chọn. Danh sách yêu thích đồng bộ giữa các thiết bị.",
    ]),
    ("devices", "Đồng bộ cài đặt giữa các thiết bị", &[
        "Cài đặt › Tùy chỉnh › Đồng bộ cài đặt với các thiết bị khác (bật sẵn).",
        "Giao diện, gợi ý, thông báo và danh sách yêu thích đi theo tài khoản.",
        "Mã PIN khóa ứng dụng không bao giờ rời khỏi thiết bị.",
        "Hai thiết bị cùng sửa: bản ghi sau cùng được giữ, không thiết bị nào ghi đè dữ liệu mới hơn mà không đọc lại.",
    ]),
    ("lock", "Biệt danh, ghi chú riêng và lưu tin nhắn", &[
        "Mở bảng Thông tin của một cuộc trò chuyện › Biệt danh và ghi chú riêng.",
        "Biệt danh thay tên người đó trong danh sách của bạn; chỉ bạn thấy.",
        "Rê chuột lên một tin nhắn và bấm biểu tượng thiết bị để lưu bản sao vào \"Chính mình\" (vẫn mã hóa đầu cuối).",
        "Tìm cài đặt nhanh bằng ô Tìm cài đặt ở đầu trang Cài đặt.",
    ]),
    ("devices", "\"Chính mình\": ghi chú giữa các thiết bị", &[
        "Mở \"Chính mình\" ở đầu danh sách Tin nhắn; tin nhắn được mã hóa bằng khóa của bạn.",
        "Mỗi tin ghi tên thiết bị đã gửi, ví dụ Linux 1, Windows 2. Bấm tên thiết bị để đổi tên.",
        "Menu ⋯ để đổi hình nền hoặc xóa; tạo lại bằng nút ở đầu danh sách.",
    ]),
    ("chat", "Ghim, thẻ phân loại, tắt thông báo", &[
        "Rê chuột lên một cuộc trò chuyện (hoặc nhấp chuột phải) rồi bấm ⋯.",
        "Ghim lên đầu, gắn thẻ phân loại, tắt thông báo, chuyển nhóm sang Cộng đồng, chặn.",
        "Lọc theo thẻ bằng nút Phân loại phía trên danh sách.",
    ]),
    ("user", "Quản lý nhóm", &[
        "Trong nhóm bấm ⋯ › Quản lý nhóm.",
        "Bật / tắt quyền thành viên: gửi tin, bình chọn, ghi chú, nhắc hẹn, ghim, đổi tên, đổi ảnh.",
        "Tham gia: duyệt thành viên mới, liên kết mời (sao chép, chia sẻ, đổi), yêu cầu chờ duyệt.",
        "Trưởng nhóm bổ nhiệm phó nhóm hoặc chuyển quyền trưởng nhóm.",
    ]),
    ("shield", "Khóa ChatW bằng mã PIN", &[
        "Cài đặt › Khóa ứng dụng › đặt mã PIN 4 đến 8 số.",
        "Chọn thời gian không hoạt động trước khi khóa, hoặc khóa mỗi lần mở.",
        "Quên mã PIN: đăng xuất trên màn hình khóa rồi đăng nhập lại.",
    ]),
    ("library", "Tạo bot của riêng bạn", &[
        "Cài đặt › Bot › nhập tên bot và tên hiển thị.",
        "Lưu token (chỉ hiện một lần), rồi gọi getUpdates và sendMessage từ code của bạn.",
        "Thêm bot vào nhóm bạn quản lý; công khai bot sau khi được xác minh.",
    ]),
    ("shield", "Xác minh mã hóa với bạn bè", &[
        "Mở cuộc trò chuyện, bấm biểu tượng bảng Thông tin ở góc phải.",
        "Mở mục \"Bảo mật và mã hóa\" để xem vân tay khóa.",
        "So sánh từng khối 4 ký tự với bạn bè qua gặp mặt hoặc cuộc gọi.",
    ]),
    ("user", "Trang trí hồ sơ", &[
        "Bấm avatar ở góc dưới cột trái › Hồ sơ của tôi › Chỉnh sửa hồ sơ.",
        "Tab Trang trí: màu chủ đề, khung avatar, hiệu ứng hồ sơ.",
        "Tab Hiệu ứng: avatar xoay, nhịp, hiệu ứng tên và bảng tên.",
        "Tab Tự tạo: tải ảnh lên hoặc vẽ trong app, chọn chuyển động rồi lưu.",
    ]),
    ("library", "Thêm emoji, sticker, GIF của riêng bạn", &[
        "Cài đặt › Thư viện, chọn Sticker, Emoji hoặc GIF.",
        "Emoji cần tên tiếng Anh (chữ, số, gạch dưới), ví dụ party_cat.",
        "Dùng ngay trong bảng chọn ở ô soạn tin, trong mọi cuộc trò chuyện.",
    ]),
    ("devices", "Đăng xuất thiết bị bị mất", &[
        "Cài đặt › Tài khoản của tôi › Thiết bị đã đăng nhập.",
        "Bấm \"Đăng xuất\" ở thiết bị lạ, hoặc đăng xuất tất cả thiết bị khác.",
        "Thiết bị mới sẽ có khóa mã hóa riêng; hãy sao lưu khóa trong Bảo mật và mã hóa.",
    ]),
    ("chat", "Cộng đồng và kênh", &[
        "Cột trái › Cộng đồng › nút + để tạo hoặc tham gia bằng mã mời.",
        "Trong cộng đồng: tạo danh mục, kênh văn bản và kênh thoại.",
        "Ghim cộng đồng lên cột trái trong Cài đặt › Tùy chỉnh.",
    ]),
];

const FAQ: [(&str, &str); 9] = [
    ("Bộ sticker OpenMoji lấy từ đâu?", "Hơn 2.000 sticker trong thẻ OpenMoji là tác phẩm của dự án mã nguồn mở OpenMoji (openmoji.org), dùng theo giấy phép CC BY-SA 4.0. Tên và từ khóa tìm kiếm lấy từ dữ liệu Unicode CLDR, nên bạn tìm được bằng cả tiếng Việt lẫn tiếng Anh."),
    ("Vì sao trả lời tự động không chạy khi tôi tắt máy?", "Tin nhắn riêng được mã hóa đầu cuối nên máy chủ không thể tự viết trả lời. Trả lời tự động chạy trên thiết bị của bạn khi ChatW đang mở."),
    ("Ẩn \"Đã xem\" có tác dụng gì?", "Người khác không biết bạn đã đọc, và bạn cũng không thấy họ đã đọc. Đổi trong Cài đặt › Trò chuyện."),
    ("Làm sao để nhóm hoặc cộng đồng hiện trong Tìm kiếm?", "Bật Công khai trong Quản lý nhóm hoặc Cài đặt cộng đồng. Đội ngũ ChatW xác minh trước khi hiện cho mọi người."),
    ("Vì sao có tin nhắn \"Không thể giải mã\"?", "Mỗi trình duyệt có khóa mã hóa riêng. Tin gửi tới khóa cũ không đọc được ở thiết bị mới. Hãy khôi phục bản sao lưu khóa trong Bảo mật và mã hóa."),
    ("Tin nhắn nhóm có được mã hóa đầu cuối không?", "Chưa. Hiện chỉ tin nhắn trực tiếp được mã hóa đầu cuối; nhóm và kênh cộng đồng được máy chủ lưu ở dạng thường."),
    ("ChatW có lưu email hay số điện thoại không?", "Không. Bạn đăng nhập bằng Google; máy chủ chỉ lưu mã định danh ẩn danh do Google cấp."),
    ("Ai thấy được hiệu ứng hồ sơ của tôi?", "Mọi người xem hồ sơ của bạn. Người xem có thể tắt hiệu ứng động trong Cài đặt › Tùy chỉnh."),
    ("Tệp tải lên được giới hạn thế nào?", "Sticker tối đa 512 KB; emoji, GIF và ảnh trang trí hồ sơ tối đa 2 MB; máy chủ kiểm tra định dạng thật của tệp."),
];

#[component]
pub fn GuideSettings(on_replay_tour: EventHandler<()>) -> Element {
    let mut query = use_signal(String::new);
    let q = crate::components::composer::fold(query().trim());
    let matches = |parts: &[&str]| q.is_empty() || parts.iter().any(|p| crate::components::composer::fold(p).contains(&q));
    let guides: Vec<_> = GUIDES.iter().filter(|(_, t, st)| matches(&[t]) || matches(st)).collect();
    let faq: Vec<_> = FAQ.iter().filter(|(qq, a)| matches(&[qq, a])).collect();
    let shortcuts: Vec<_> = SHORTCUTS.iter().filter(|(k, d)| matches(&[k, d])).collect();
    rsx! {
        div { class: "search-field help-search",
            crate::components::icons::SearchIcon { size: 16 }
            input { r#type: "search", placeholder: "Tìm trong hướng dẫn (ví dụ: avatar, sticker, PIN)", aria_label: "Tìm trong hướng dẫn", value: "{query}", oninput: move |e| query.set(e.value()) }
        }
        if !q.is_empty() && guides.is_empty() && faq.is_empty() && shortcuts.is_empty() {
            div { class: "sidebar-empty", "Không có hướng dẫn phù hợp" }
        }
        div { class: "guide-hero",
            Illustration { kind: "chat".to_string() }
            div {
                div { class: "settings-group-title", "Bắt đầu với ChatW" }
                div { class: "settings-group-desc", "Xem lại phần giới thiệu từng bước bất cứ lúc nào." }
                button { r#type: "button", class: "btn-primary", onclick: move |_| on_replay_tour.call(()), LogoIcon { size: 16 } span { "Xem lại giới thiệu" } }
            }
        }
        div { class: "settings-group",
            div { class: "settings-group-title", "Hướng dẫn" }
            div { class: "guide-grid",
                for (art, title, steps) in guides.iter().copied().copied() {
                    div { key: "{title}", class: "guide-card",
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
            div { class: "settings-group-title", "Phím tắt" }
            div { class: "settings-list",
                for (k, d) in shortcuts.iter().copied().copied() {
                    div { key: "{k}", class: "settings-row", span { class: "settings-label", "{d}" } span { class: "kbd", "{k}" } }
                }
            }
        }
        div { class: "settings-group",
            div { class: "settings-group-title", "Câu hỏi thường gặp" }
            for (q, a) in faq.iter().copied().copied() {
                details { key: "{q}", class: "faq-item",
                    summary { "{q}" }
                    p { "{a}" }
                }
            }
        }
    }
}
