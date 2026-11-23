//! Spec 105: Clean Simple HTML Legal Documents & Product Vision
//! (Điều khoản dịch vụ, Chính sách bảo mật, Tiêu chuẩn cộng đồng, Tầm nhìn sản phẩm)
//! Pure simple HTML document flow, zero clutter, zero colons, zero parentheses.

use dioxus::prelude::*;
use crate::components::icons::CloseIcon;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LegalDocKind {
    Terms,
    Privacy,
    Community,
    Vision,
}

impl LegalDocKind {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Terms => "Điều khoản dịch vụ",
            Self::Privacy => "Chính sách bảo mật",
            Self::Community => "Tiêu chuẩn cộng đồng",
            Self::Vision => "Tầm nhìn sản phẩm",
        }
    }
}

pub struct LegalItem {
    pub title: &'static str,
    pub detail: &'static str,
}

pub fn get_legal_items(kind: LegalDocKind) -> Vec<LegalItem> {
    match kind {
        LegalDocKind::Terms => vec![
            LegalItem {
                title: "Thứ nhất. Quyền riêng tư là tuyệt đối vĩnh viễn và không thể đảo ngược!",
                detail: "Mọi quyền lợi và sự riêng tư của người dùng là tối thượng, bất khả xâm phạm dưới mọi hình thức và hoàn cảnh.",
            },
            LegalItem {
                title: "Thứ hai. Từ chối cung cấp bất kì thông tin cho bên thứ ba nào.",
                detail: "Không hợp tác, không mua bán, không chuyển giao bất kỳ dữ liệu nào cho bất kỳ cá nhân, tổ chức hay bên thứ ba nào.",
            },
            LegalItem {
                title: "Thứ ba. Ẩn danh tuyệt đối.",
                detail: "Không thu thập số điện thoại, họ tên thật, định vị GPS, danh bạ hay bất kỳ dấu vết định danh cá nhân nào.",
            },
            LegalItem {
                title: "Thứ tư. Mã hóa đầu cuối thực chất.",
                detail: "Chỉ người gửi và người nhận mới có chìa khóa giải mã nội dung. Máy chủ hoàn toàn mù và không thể đọc tin nhắn.",
            },
            LegalItem {
                title: "Thứ năm. Toàn bộ dữ liệu máy chủ luôn được mã hóa, chỉ người có mật khẩu mới mở được, khi bị tấn công hệ thống sẽ tự động ngăn chặn và bảo vệ dữ liệu.",
                detail: "Hệ thống bảo vệ chủ động, cách ly vùng nguy hiểm và phong tỏa kho lưu trữ ngay khi phát hiện dấu hiệu can thiệp bất hợp pháp.",
            },
            LegalItem {
                title: "Thứ sáu. Zero-logs tuyệt đối.",
                detail: "Không ghi nhật ký truy cập, không lưu lịch sử cuộc gọi, tin nhắn hay hành vi. Dữ liệu khi xóa là biến mất vĩnh viễn không thể phục hồi.",
            },
            LegalItem {
                title: "Thứ bảy. Quyền sở hữu thuộc trọn về người dùng.",
                detail: "Khóa mật mã được lưu trên thiết bị của bạn. Bạn là chủ nhân duy nhất nắm giữ quyền truy cập và định đoạt dữ liệu của chính mình.",
            },
            LegalItem {
                title: "Thứ tám. Mã nguồn mở và minh bạch.",
                detail: "Cam kết không cửa sau, không mã độc ngầm, kiến trúc mở sẵn sàng cho cộng đồng kiểm toán bảo mật độc lập.",
            },
            LegalItem {
                title: "Thứ chín. Phi thương mại hóa và không thao túng.",
                detail: "Không hiển thị quảng cáo, không bán dữ liệu hành vi, không sử dụng thuật toán can thiệp hay dẫn dắt tâm lý người dùng.",
            },
            LegalItem {
                title: "Thứ mười. Không khoan nhượng với phá hoại.",
                detail: "Mọi hành vi tấn công hạ tầng, cố tình phát tán mã độc, đánh cắp khóa mã hóa hoặc xâm phạm hệ thống đều bị chặn đứng và loại bỏ vĩnh viễn.",
            },
        ],
        LegalDocKind::Privacy => vec![
            LegalItem {
                title: "Thu thập dữ liệu bằng không",
                detail: "ChatW không lưu số điện thoại, không truy cập danh bạ, không lưu địa chỉ IP và không gắn cookie theo dõi người dùng.",
            },
            LegalItem {
                title: "Mật mã phía máy khách",
                detail: "Khóa mật mã sinh trực tiếp trên trình duyệt hoặc thiết bị của bạn. Khóa riêng tư không bao giờ gửi qua mạng.",
            },
            LegalItem {
                title: "Từ chối chia sẻ bên thứ ba",
                detail: "Tuyệt đối không chia sẻ, không nhúng mã theo dõi phân tích hành vi và không tích hợp với bất kỳ nhà cung cấp dữ liệu thương mại nào.",
            },
            LegalItem {
                title: "Xóa sạch không vết tích",
                detail: "Khi bạn chọn xóa tin nhắn hoặc tài khoản, toàn bộ dữ liệu trên thiết bị và máy chủ chuyển tiếp bị tiêu hủy tức thì, không thùng rác lưu trữ.",
            },
            LegalItem {
                title: "Tự vệ chủ động khi bị tấn công",
                detail: "Hệ thống tự động kích hoạt phòng thủ đa tầng, khóa bảo vệ cơ sở dữ liệu và cách ly vùng nguy hiểm khi phát hiện xâm nhập.",
            },
        ],
        LegalDocKind::Community => vec![
            LegalItem {
                title: "Tôn trọng quyền riêng tư của người khác",
                detail: "Cấm rình mò, nghe lén, chụp lén hoặc phát tán hình ảnh, tin nhắn riêng tư của người khác khi chưa có sự đồng ý rõ ràng.",
            },
            LegalItem {
                title: "Nghiêm cấm mã độc và lừa đảo",
                detail: "Nghiêm cấm gửi liên kết lừa đảo, phát tán virus, trojan hoặc thực hiện các thủ đoạn chiếm đoạt tài sản.",
            },
            LegalItem {
                title: "Không spam và không tấn công từ chối dịch vụ",
                detail: "Không sử dụng bot spam hàng loạt, không gửi rác gây nghẽn băng thông và không phá hoại trải nghiệm liên lạc của cộng đồng.",
            },
            LegalItem {
                title: "Bảo vệ trẻ em tuyệt đối",
                detail: "Không khoan nhượng với bất kỳ hành vi lạm dụng, đe dọa hoặc xâm hại trẻ em. Tài khoản vi phạm sẽ bị loại bỏ vĩnh viễn ngay lập tức.",
            },
            LegalItem {
                title: "Giao tiếp văn minh và trách nhiệm",
                detail: "Tự do thảo luận trên tinh thần tôn trọng. Cấm đe dọa bạo lực, khủng bố hoặc kích động gây hại cho sự an nguy của con người.",
            },
        ],
        LegalDocKind::Vision => vec![
            LegalItem {
                title: "Quyền riêng tư là nền tảng của tự do",
                detail: "Mọi cuộc trò chuyện cá nhân đều phải được bảo vệ một cách tự nhiên mà không cần người dùng phải là chuyên gia kỹ thuật.",
            },
            LegalItem {
                title: "Bảo mật bằng toán học thay vì niềm tin",
                detail: "Chúng tôi không yêu cầu bạn phải tin vào thiện chí của bất kỳ ai. Kiến trúc mật mã học đảm bảo máy chủ không bao giờ có khả năng giải mã nội dung.",
            },
            LegalItem {
                title: "Tối giản và thuần khiết",
                detail: "Không thuật toán gây nghiện, không quảng cáo, không thu thập thói quen. Ứng dụng chỉ tập trung phục vụ mục đích duy nhất là truyền tải thông điệp nhanh nhất và an toàn nhất.",
            },
            LegalItem {
                title: "Người dùng là chủ nhân duy nhất của dữ liệu",
                detail: "Thiết bị của bạn giữ chìa khóa, dữ liệu của bạn do bạn toàn quyền định đoạt. Khi bạn xóa, dữ liệu biến mất mãi mãi.",
            },
            LegalItem {
                title: "Công nghệ độc lập và minh bạch",
                detail: "Xây dựng trên nền tảng mã nguồn mở, không phụ thuộc vào các tập đoàn công nghệ lớn, vì một không gian mạng tự do và an toàn cho tất cả mọi người.",
            },
        ],
    }
}

#[component]
pub fn LegalContent(kind: LegalDocKind) -> Element {
    let items = get_legal_items(kind);

    rsx! {
        div { class: "legal-simple-doc",
            for item in items.iter() {
                div { class: "legal-simple-item",
                    h3 { class: "legal-simple-title", "{item.title}" }
                    p { class: "legal-simple-detail", "{item.detail}" }
                }
            }
        }
    }
}

#[component]
pub fn LegalPanel(kind: LegalDocKind) -> Element {
    rsx! {
        div { class: "settings-legal-panel",
            LegalContent { kind }
        }
    }
}

#[component]
pub fn LegalModal(
    doc: Option<LegalDocKind>,
    on_close: EventHandler<()>,
) -> Element {
    if let Some(kind) = doc {
        rsx! {
            div {
                class: "legal-modal-backdrop",
                onclick: move |_| on_close.call(()),
                div {
                    class: "legal-modal-card",
                    onclick: move |e| e.stop_propagation(),
                    div { class: "legal-modal-top",
                        h3 { class: "legal-modal-title", "{kind.title()}" }
                        button {
                            r#type: "button",
                            class: "legal-modal-close-btn",
                            aria_label: "Đóng",
                            onclick: move |_| on_close.call(()),
                            CloseIcon { size: 18 }
                        }
                    }
                    div { class: "legal-modal-scrollable",
                        LegalContent { kind }
                    }
                    div { class: "legal-modal-bottom",
                        button {
                            r#type: "button",
                            class: "btn-primary legal-modal-ok-btn",
                            onclick: move |_| on_close.call(()),
                            "Đã hiểu và đồng ý"
                        }
                    }
                }
            }
        }
    } else {
        rsx! {}
    }
}
