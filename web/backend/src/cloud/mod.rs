pub mod incremental;
pub mod providers;
pub mod router;

use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
use futures_util::future::BoxFuture;
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::fmt;

use crate::error::{AppError, AppResult};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub provider_id: &'static str,
    pub display_name: &'static str,
    pub auth_kind: &'static str, // "oauth2_pkce" | "credentials" | "icloud_platform"
    pub oauth_configured: bool,
    pub supports_resumable_upload: bool,
    pub supports_resumable_download: bool,
    pub supports_incremental_chunks: bool,
    pub supports_quota: bool,
    pub supports_folder_isolation: bool,
    pub requires_https: bool,
    pub max_file_bytes: u64,
    pub setup_requirements: Vec<&'static str>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CloudQuotaInfo {
    pub total_bytes: Option<u64>,
    pub used_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloudBackupItem {
    pub remote_key: String,
    pub name: String,
    pub byte_size: u64,
    pub etag_or_sha256: Option<String>,
    pub modified_at: Option<String>,
}

#[derive(Debug)]
pub enum CloudError {
    AuthRequired,
    TokenExpired,
    PermissionRevoked,
    QuotaExceeded,
    NotConfiguredByAdmin(&'static str),
    SsrfBlocked(String),
    TlsRequired,
    InvalidConfig(String),
    ChecksumMismatch,
    Unsupported(&'static str),
    Network(String),
    Remote(String),
}

impl fmt::Display for CloudError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthRequired => write!(f, "Need to re-authenticate Cloud account"),
            Self::TokenExpired => write!(f, "Cloud login session has expired"),
            Self::PermissionRevoked => write!(f, "The user has revoked Cloud access"),
            Self::QuotaExceeded => write!(f, "Cloud account has run out of free space"),
            Self::NotConfiguredByAdmin(msg) => write!(f, "Administrator has not configured provider information: {msg}"),
            Self::SsrfBlocked(host) => write!(f, "Address blocked because of intranet security policy/SSRF: {host}"),
            Self::TlsRequired => write!(f, "Requires a valid HTTPS/TLS connection"),
            Self::InvalidConfig(m) => write!(f, "Invalid configuration: {m}"),
            Self::ChecksumMismatch => write!(f, "Cloud backup file checksum does not match"),
            Self::Unsupported(op) => write!(f, "The provider does not support this operation: {op}"),
            Self::Network(m) => write!(f, "Network error when connecting to Cloud: {m}"),
            Self::Remote(m) => write!(f, "Error from Cloud server: {m}"),
        }
    }
}

impl std::error::Error for CloudError {}

impl From<CloudError> for AppError {
    fn from(e: CloudError) -> Self {
        match e {
            CloudError::AuthRequired | CloudError::TokenExpired | CloudError::PermissionRevoked => {
                AppError::Unauthorized
            }
            CloudError::SsrfBlocked(_) | CloudError::TlsRequired | CloudError::InvalidConfig(_) => {
                AppError::BadRequest("invalid_cloud_endpoint_or_config")
            }
            CloudError::NotConfiguredByAdmin(_) => AppError::BadRequest("cloud_provider_not_configured"),
            CloudError::QuotaExceeded => AppError::PayloadTooLarge,
            _ => AppError::BadRequest("cloud_operation_failed"),
        }
    }
}

/// Unified Cloud Storage Provider interface (Google Drive, OneDrive, Dropbox, Nextcloud, WebDAV, S3/R2/MinIO, NAS).
pub trait CloudStorageProvider: Send + Sync {
    fn capabilities(&self) -> ProviderCapabilities;

    fn verify_connection<'a>(
        &'a self,
        http: &'a reqwest::Client,
    ) -> BoxFuture<'a, Result<(Option<String>, CloudQuotaInfo), CloudError>>;

    fn refresh_authorization<'a>(
        &'a self,
        _http: &'a reqwest::Client,
    ) -> BoxFuture<'a, Result<Option<serde_json::Value>, CloudError>> {
        Box::pin(async move { Ok(None) })
    }

    fn revoke_authorization<'a>(
        &'a self,
        _http: &'a reqwest::Client,
    ) -> BoxFuture<'a, Result<(), CloudError>> {
        Box::pin(async move { Ok(()) })
    }

    fn list_backups<'a>(
        &'a self,
        http: &'a reqwest::Client,
        prefix: &'a str,
    ) -> BoxFuture<'a, Result<Vec<CloudBackupItem>, CloudError>>;

    fn upload_backup<'a>(
        &'a self,
        http: &'a reqwest::Client,
        remote_key: &'a str,
        bytes: Vec<u8>,
    ) -> BoxFuture<'a, Result<CloudBackupItem, CloudError>>;

    fn download_backup<'a>(
        &'a self,
        http: &'a reqwest::Client,
        remote_key: &'a str,
    ) -> BoxFuture<'a, Result<Vec<u8>, CloudError>>;

    fn delete_backup<'a>(
        &'a self,
        http: &'a reqwest::Client,
        remote_key: &'a str,
    ) -> BoxFuture<'a, Result<bool, CloudError>>;

    fn verify_backup<'a>(
        &'a self,
        http: &'a reqwest::Client,
        remote_key: &'a str,
        expected_sha256_hex: &'a str,
    ) -> BoxFuture<'a, Result<bool, CloudError>> {
        Box::pin(async move {
            let bytes = self.download_backup(http, remote_key).await?;
            let actual = hex::encode(crate::storage::sha256_bytes(&bytes));
            Ok(actual == expected_sha256_hex)
        })
    }

    fn get_quota<'a>(
        &'a self,
        http: &'a reqwest::Client,
    ) -> BoxFuture<'a, Result<CloudQuotaInfo, CloudError>> {
        Box::pin(async move {
            let (_, q) = self.verify_connection(http).await?;
            Ok(q)
        })
    }
}

fn cred_cipher(session_secret: &[u8]) -> Aes256Gcm {
    let hk = Hkdf::<Sha256>::new(None, session_secret);
    let mut key = [0u8; 32];
    hk.expand(b"chatw-cloud-credentials-v1", &mut key).expect("32-byte key");
    Aes256Gcm::new_from_slice(&key).expect("aes256")
}

pub fn seal_cloud_config(session_secret: &[u8], value: &serde_json::Value) -> AppResult<Vec<u8>> {
    let plain = serde_json::to_vec(value).map_err(|_| AppError::Internal)?;
    let mut nonce = [0u8; 12];
    getrandom::fill(&mut nonce).map_err(|_| AppError::Internal)?;
    let ct = cred_cipher(session_secret)
        .encrypt(&Nonce::from(nonce), plain.as_slice())
        .map_err(|_| AppError::Internal)?;
    let mut out = Vec::with_capacity(12 + ct.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

pub fn open_cloud_config(session_secret: &[u8], sealed: &[u8]) -> AppResult<serde_json::Value> {
    if sealed.len() < 13 {
        return Err(AppError::Internal);
    }
    let nonce: [u8; 12] = sealed[..12].try_into().map_err(|_| AppError::Internal)?;
    let plain = cred_cipher(session_secret)
        .decrypt(&Nonce::from(nonce), &sealed[12..])
        .map_err(|_| AppError::Internal)?;
    serde_json::from_slice(&plain).map_err(|_| AppError::Internal)
}
