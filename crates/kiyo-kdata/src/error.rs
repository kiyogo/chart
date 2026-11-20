use std::fmt;

#[derive(Debug)]
pub enum KdataError {
    InvalidMagic,
    UnsupportedVersion(u16),
    UnsupportedCipher(u8),
    InvalidHeader(String),
    AuthenticationFailed,
    WrongPasswordOrTampered,
    IntegrityMismatch(String),
    UnsafePath(String),
    ResourceLimitExceeded(String),
    FormatViolation(String),
    Io(std::io::Error),
    Zip(String),
}

impl fmt::Display for KdataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "Tệp không đúng định dạng .kdata hợp lệ (sai magic bytes)"),
            Self::UnsupportedVersion(v) => write!(f, "Phiên bản định dạng .kdata ({v}) chưa được hỗ trợ"),
            Self::UnsupportedCipher(c) => write!(f, "Thuật toán mã hóa ({c}) không được hỗ trợ"),
            Self::InvalidHeader(m) => write!(f, "Header .kdata không hợp lệ: {m}"),
            Self::AuthenticationFailed => write!(f, "Xác thực AEAD thất bại: dữ liệu có thể bị thay đổi hoặc sai khóa"),
            Self::WrongPasswordOrTampered => write!(f, "Mật khẩu giải mã không đúng hoặc tệp .kdata đã bị chỉnh sửa"),
            Self::IntegrityMismatch(m) => write!(f, "Kiểm tra toàn vẹn thất bại: {m}"),
            Self::UnsafePath(p) => write!(f, "Đường dẫn không an toàn bị chặn: {p}"),
            Self::ResourceLimitExceeded(m) => write!(f, "Vượt giới hạn tài nguyên an toàn: {m}"),
            Self::FormatViolation(m) => write!(f, "Cấu trúc bản sao lưu bị hỏng hoặc không hợp lệ: {m}"),
            Self::Io(e) => write!(f, "Lỗi đọc/ghi luồng dữ liệu: {e}"),
            Self::Zip(e) => write!(f, "Lỗi xử lý kho lưu trữ ZIP: {e}"),
        }
    }
}

impl std::error::Error for KdataError {}

impl From<std::io::Error> for KdataError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
