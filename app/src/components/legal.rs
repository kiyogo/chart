//! Spec 104: Terms of Service, Privacy Policy & Community Standards
//! (Điều khoản dịch vụ, Chính sách bảo mật, Tiêu chuẩn cộng đồng)
//! Steel manifesto style: "ngắn nhất có thể một là một hai là hai đúng".

use dioxus::prelude::*;
use crate::components::icons::{CheckIcon, CloseIcon, FileTextIcon, LockIcon, ShieldIcon};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LegalDocKind {
    Terms,
    Privacy,
    Community,
}

impl LegalDocKind {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Terms => "Điều khoản dịch vụ",
            Self::Privacy => "Chính sách bảo mật",
            Self::Community => "Tiêu chuẩn cộng đồng",
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self {
            Self::Terms => "Cam kết thép và nguyên tắc bất biến của hệ thống nhắn tin bảo mật ChatW",
            Self::Privacy => "Chính sách bảo vệ quyền riêng tư tuyệt đối và mô hình Zero-Knowledge",
            Self::Community => "Quy tắc ứng xử và tiêu chuẩn văn minh bảo vệ môi trường chung",
        }
    }

    pub fn badge(&self) -> &'static str {
        match self {
            Self::Terms => "Nguyên tắc bất biến",
            Self::Privacy => "Zero-Knowledge",
            Self::Community => "Chuẩn mực chung",
        }
    }
}

pub struct LegalItem {
    pub order: &'static str,
    pub title: &'static str,
    pub detail: &'static str,
}

pub fn get_legal_items(kind: LegalDocKind) -> Vec<LegalItem> {
    match kind {
        LegalDocKind::Terms => vec![
            LegalItem {
                order: "Thứ nhất",
                title: "Quyền riêng tư là tuyệt đối vĩnh viễn và không thể đảo ngược!",
                detail: "Mọi quyền lợi và sự riêng tư của người dùng là tối thượng, bất khả xâm phạm dưới mọi hình thức và hoàn cảnh.",
            },
            LegalItem {
                order: "Thứ hai",
                title: "Từ chối cung cấp bất kì thông tin cho bên thứ ba nào.",
                detail: "Không hợp tác, không mua bán, không chuyển giao bất kỳ dữ liệu nào cho bất kỳ cá nhân, tổ chức hay bên thứ ba nào.",
            },
            LegalItem {
                order: "Thứ ba",
                title: "Ẩn danh tuyệt đối.",
                detail: "Không thu thập số điện thoại, họ tên thật, định vị GPS, danh bạ hay bất kỳ dấu vết định danh cá nhân nào.",
            },
            LegalItem {
                order: "Thứ tư",
                title: "Mã hóa đầu cuối (E2EE) thực chất.",
                detail: "Chỉ người gửi và người nhận mới có chìa khóa giải mã nội dung. Máy chủ hoàn toàn mù (Blind Relay) và không thể đọc tin nhắn.",
            },
            LegalItem {
                order: "Thứ năm",
                title: "Bảo vệ máy chủ và mã hóa toàn bộ dữ liệu.",
                detail: "Toàn bộ dữ liệu máy chủ luôn được mã hóa, chỉ người có mật khẩu mới mở được, khi bị tấn công hệ thống sẽ tự động ngăn chặn và bảo vệ dữ liệu.",
            },
            LegalItem {
                order: "Thứ sáu",
                title: "Zero-logs tuyệt đối.",
                detail: "Không ghi nhật ký truy cập, không lưu lịch sử cuộc gọi, tin nhắn hay hành vi. Dữ liệu khi xóa là biến mất vĩnh viễn không thể phục hồi.",
            },
            LegalItem {
                order: "Thứ bảy",
                title: "Quyền sở hữu thuộc trọn về người dùng.",
                detail: "Khóa mật mã được lưu trên thiết bị của bạn. Bạn là chủ nhân duy nhất nắm giữ quyền truy cập và định đoạt dữ liệu của chính mình.",
            },
            LegalItem {
                order: "Thứ tám",
                title: "Mã nguồn mở và minh bạch.",
                detail: "Cam kết không cửa sau (No Backdoors), không mã độc ngầm, kiến trúc mở sẵn sàng cho cộng đồng kiểm toán bảo mật độc lập.",
            },
            LegalItem {
                order: "Thứ chín",
                title: "Phi thương mại hóa và không thao túng.",
                detail: "Không hiển thị quảng cáo, không bán dữ liệu hành vi, không sử dụng thuật toán can thiệp hay dẫn dắt tâm lý người dùng.",
            },
            LegalItem {
                order: "Thứ mười",
                title: "Không khoan nhượng với phá hoại.",
                detail: "Mọi hành vi tấn công hạ tầng, cố tình phát tán mã độc, đánh cắp khóa mã hóa hoặc xâm phạm hệ thống đều bị chặn đứng và loại bỏ vĩnh viễn.",
            },
        ],
        LegalDocKind::Privacy => vec![
            LegalItem {
                order: "Nguyên tắc 1",
                title: "Thu thập dữ liệu: Bằng 0 (Zero Data Collection)",
                detail: "ChatW không lưu số điện thoại, không truy cập danh bạ, không lưu địa chỉ IP và không gắn cookie theo dõi người dùng.",
            },
            LegalItem {
                order: "Nguyên tắc 2",
                title: "Mật mã phía máy khách (Client-Side Cryptography)",
                detail: "Khóa X25519, Ed25519 và Double Ratchet sinh trực tiếp trên trình duyệt hoặc thiết bị của bạn. Khóa riêng tư không bao giờ gửi qua mạng.",
            },
            LegalItem {
                order: "Nguyên tắc 3",
                title: "Từ chối chia sẻ bên thứ ba",
                detail: "Tuyệt đối không chia sẻ, nhúng mã theo dõi phân tích hành vi (analytics) hay tích hợp với bất kỳ nhà cung cấp dữ liệu thương mại nào.",
            },
            LegalItem {
                order: "Nguyên tắc 4",
                title: "Xóa sạch không vết tích",
                detail: "Khi bạn chọn xóa tin nhắn hoặc tài khoản, toàn bộ dữ liệu trên thiết bị và máy chủ chuyển tiếp bị tiêu hủy tức thì, không thùng rác lưu trữ.",
            },
            LegalItem {
                order: "Nguyên tắc 5",
                title: "Tự vệ chủ động khi bị tấn công",
                detail: "Hệ thống tự động kích hoạt phòng thủ đa tầng, khóa bảo vệ cơ sở dữ liệu và cách ly vùng nguy hiểm khi phát hiện xâm nhập.",
            },
        ],
        LegalDocKind::Community => vec![
            LegalItem {
                order: "Chuẩn mực 1",
                title: "Tôn trọng quyền riêng tư của người khác",
                detail: "Cấm rình mò, nghe lén, chụp lén hoặc phát tán hình ảnh, tin nhắn riêng tư của người khác khi chưa có sự đồng ý rõ ràng.",
            },
            LegalItem {
                order: "Chuẩn mực 2",
                title: "Nghiêm cấm mã độc và lừa đảo",
                detail: "Nghiêm cấm gửi liên kết lừa đảo (phishing), phát tán virus, trojan hoặc thực hiện các thủ đoạn chiếm đoạt tài sản.",
            },
            LegalItem {
                order: "Chuẩn mực 3",
                title: "Không spam và không tấn công từ chối dịch vụ",
                detail: "Không sử dụng bot spam hàng loạt, gửi rác gây nghẽn băng thông hoặc phá hoại trải nghiệm liên lạc của cộng đồng.",
            },
            LegalItem {
                order: "Chuẩn mực 4",
                title: "Bảo vệ trẻ em tuyệt đối",
                detail: "Không khoan nhượng với bất kỳ hành vi lạm dụng, đe dọa hoặc xâm hại trẻ em. Tài khoản vi phạm sẽ bị loại bỏ vĩnh viễn ngay lập tức.",
            },
            LegalItem {
                order: "Chuẩn mực 5",
                title: "Giao tiếp văn minh và trách nhiệm",
                detail: "Tự do thảo luận trên tinh thần tôn trọng; cấm đe dọa bạo lực, khủng bố hoặc kích động gây hại cho sự an nguy của con người.",
            },
        ],
    }
}

#[component]
pub fn LegalContent(kind: LegalDocKind) -> Element {
    let items = get_legal_items(kind);

    rsx! {
        div { class: "legal-doc-view",
            div { class: "legal-doc-hero",
                div { class: "legal-doc-hero-icon",
                    match kind {
                        LegalDocKind::Terms => rsx! { FileTextIcon { size: 28 } },
                        LegalDocKind::Privacy => rsx! { ShieldIcon { size: 28 } },
                        LegalDocKind::Community => rsx! { LockIcon { size: 28 } },
                    }
                }
                div { class: "legal-doc-hero-meta",
                    div { class: "legal-doc-badge", "{kind.badge()}" }
                    h2 { class: "legal-doc-title", "{kind.title()}" }
                    p { class: "legal-doc-subtitle", "{kind.subtitle()}" }
                }
            }

            div { class: "legal-items-grid",
                for item in items.iter() {
                    div { class: "legal-item-card",
                        div { class: "legal-item-header",
                            span { class: "legal-item-order", "{item.order}" }
                            span { class: "legal-item-check", CheckIcon { size: 14 } }
                        }
                        div { class: "legal-item-title", "{item.title}" }
                        div { class: "legal-item-detail", "{item.detail}" }
                    }
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
                        div { class: "legal-modal-heading",
                            span { class: "legal-modal-tag", "{kind.badge()}" }
                            h3 { class: "legal-modal-title", "{kind.title()}" }
                        }
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
