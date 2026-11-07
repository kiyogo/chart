use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Bad request or invalid format: {0}")]
    BadRequest(&'static str),

    #[error("Invalid cryptographic signature")]
    InvalidSignature,

    #[error("Decryption failed")]
    DecryptionFailed,

    #[error("Key generation error: {0}")]
    KeyError(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, CoreError>;
