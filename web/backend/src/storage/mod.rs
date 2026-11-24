pub mod dedup_engine;
pub mod encrypted_backup;
pub mod existing;
pub mod local_cas;
pub mod migration;
pub mod permissions;
pub mod references;
pub mod router;
pub mod s3;
pub mod temp_relay;

use axum::body::{Body, Bytes};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fmt,
    path::{Component, Path, PathBuf},
    sync::Arc,
};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageCapabilities {
    pub backend_name: &'static str,
    pub supports_stream: bool,
    pub supports_multipart: bool,
    pub supports_presign: bool,
    pub supports_range: bool,
    pub supports_ttl: bool,
    pub supports_peer_transfer: bool,
    pub supports_dedup: bool,
    pub durable: bool,
    pub max_chunk_bytes: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredObjectMeta {
    pub backend: String,
    pub storage_key: String,
    pub byte_size: u64,
    pub sha256_hex: String,
    pub content_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PresignedAccess {
    pub url: String,
    pub method: String,
    pub expires_in_secs: u64,
    pub signed_headers: Vec<(String, String)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultipartInit {
    pub upload_id: String,
    pub storage_key: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CleanupReport {
    pub scanned: u64,
    pub deleted: u64,
    pub skipped_pinned: u64,
    pub freed_bytes: u64,
}

#[derive(Debug)]
pub enum StorageError {
    NotFound(String),
    InvalidKey,
    IntegrityMismatch,
    UnsupportedOperation {
        adapter: &'static str,
        operation: &'static str,
    },
    NotConfigured(&'static str),
    Io(std::io::Error),
    Backend(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(k) => write!(f, "object not found: {k}"),
            Self::InvalidKey => write!(f, "invalid storage key"),
            Self::IntegrityMismatch => write!(f, "sha256 integrity mismatch"),
            Self::UnsupportedOperation { adapter, operation } => {
                write!(f, "operation '{operation}' is unsupported by '{adapter}'")
            }
            Self::NotConfigured(name) => write!(f, "adapter not configured: {name}"),
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Backend(m) => write!(f, "storage backend error: {m}"),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        if e.kind() == std::io::ErrorKind::NotFound {
            Self::NotFound(e.to_string())
        } else {
            Self::Io(e)
        }
    }
}

/// Unified provider-independent storage interface with explicit capability reporting.
pub trait StorageAdapter: Send + Sync {
    fn capabilities(&self) -> StorageCapabilities;

    fn put_object<'a>(
        &'a self,
        key: &'a str,
        bytes: Vec<u8>,
        content_type: &'a str,
        ttl_secs: Option<u64>,
    ) -> BoxFuture<'a, Result<StoredObjectMeta, StorageError>>;

    fn get_object<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Vec<u8>, StorageError>>;

    fn get_object_range<'a>(
        &'a self,
        key: &'a str,
        start: u64,
        end_inclusive: u64,
    ) -> BoxFuture<'a, Result<(Vec<u8>, u64), StorageError>>;

    fn get_object_stream<'a>(
        &'a self,
        key: &'a str,
    ) -> BoxFuture<'a, Result<Body, StorageError>>;

    fn delete_object<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, StorageError>>;

    fn object_exists<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, StorageError>>;

    fn get_metadata<'a>(
        &'a self,
        key: &'a str,
    ) -> BoxFuture<'a, Result<StoredObjectMeta, StorageError>>;

    fn get_content_hash<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<[u8; 32], StorageError>>;

    fn create_temporary_access<'a>(
        &'a self,
        _key: &'a str,
        _method: &'a str,
        _ttl_secs: u64,
    ) -> BoxFuture<'a, Result<PresignedAccess, StorageError>> {
        let adapter = self.capabilities().backend_name;
        Box::pin(async move {
            Err(StorageError::UnsupportedOperation {
                adapter,
                operation: "create_temporary_access",
            })
        })
    }

    fn revoke_temporary_access<'a>(
        &'a self,
        _token_or_key: &'a str,
    ) -> BoxFuture<'a, Result<(), StorageError>> {
        let adapter = self.capabilities().backend_name;
        Box::pin(async move {
            Err(StorageError::UnsupportedOperation {
                adapter,
                operation: "revoke_temporary_access",
            })
        })
    }

    fn begin_multipart_transfer<'a>(
        &'a self,
        _key: &'a str,
        _content_type: &'a str,
    ) -> BoxFuture<'a, Result<MultipartInit, StorageError>> {
        let adapter = self.capabilities().backend_name;
        Box::pin(async move {
            Err(StorageError::UnsupportedOperation {
                adapter,
                operation: "begin_multipart_transfer",
            })
        })
    }

    fn upload_chunk<'a>(
        &'a self,
        _transfer_id: &'a str,
        _chunk_index: u32,
        _bytes: Vec<u8>,
    ) -> BoxFuture<'a, Result<String, StorageError>> {
        let adapter = self.capabilities().backend_name;
        Box::pin(async move {
            Err(StorageError::UnsupportedOperation {
                adapter,
                operation: "upload_chunk",
            })
        })
    }

    fn download_chunk<'a>(
        &'a self,
        key: &'a str,
        chunk_index: u32,
        chunk_size: usize,
    ) -> BoxFuture<'a, Result<(Vec<u8>, u64), StorageError>> {
        Box::pin(async move {
            let start = (chunk_index as u64) * (chunk_size as u64);
            let end = start + (chunk_size as u64).saturating_sub(1);
            self.get_object_range(key, start, end).await
        })
    }

    fn verify_integrity<'a>(
        &'a self,
        key: &'a str,
        expected_sha256: &'a [u8; 32],
    ) -> BoxFuture<'a, Result<bool, StorageError>> {
        Box::pin(async move {
            let actual = self.get_content_hash(key).await?;
            Ok(&actual == expected_sha256)
        })
    }

    fn cleanup_expired_objects<'a>(&'a self) -> BoxFuture<'a, Result<CleanupReport, StorageError>> {
        Box::pin(async move { Ok(CleanupReport::default()) })
    }
}

/// Shared bundle of all backend storage adapters instantiated from server config.
#[derive(Clone)]
pub struct StorageRegistry {
    pub existing: Arc<existing::ExistingStorageAdapter>,
    pub local_cas: Arc<local_cas::LocalFileStorageAdapter>,
    pub temp_relay: Arc<temp_relay::TemporaryRelayStorageAdapter>,
    pub encrypted_backup: Arc<encrypted_backup::EncryptedBackupStorageAdapter>,
    pub s3: Arc<s3::S3CompatibleStorageAdapter>,
}

impl StorageRegistry {
    pub fn from_uploads_dir(uploads_dir: &Path) -> Self {
        Self {
            existing: Arc::new(existing::ExistingStorageAdapter::new(uploads_dir.to_path_buf())),
            local_cas: Arc::new(local_cas::LocalFileStorageAdapter::new(uploads_dir.join("cas"))),
            temp_relay: Arc::new(temp_relay::TemporaryRelayStorageAdapter::new(uploads_dir.join("relay"))),
            encrypted_backup: Arc::new(encrypted_backup::EncryptedBackupStorageAdapter::new(
                uploads_dir.join("encrypted_backup"),
            )),
            s3: Arc::new(s3::S3CompatibleStorageAdapter::from_env()),
        }
    }

    pub fn capabilities_list(&self) -> Vec<StorageCapabilities> {
        vec![
            self.existing.capabilities(),
            self.local_cas.capabilities(),
            self.temp_relay.capabilities(),
            self.encrypted_backup.capabilities(),
            self.s3.capabilities(),
        ]
    }
}

pub fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

pub async fn sha256_file(path: &Path) -> Result<(u64, [u8; 32]), StorageError> {
    let mut f = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut total: u64 = 0;
    loop {
        let n = f.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        total += n as u64;
    }
    Ok((total, hasher.finalize().into()))
}

pub fn safe_join_relative(root: &Path, rel: &str) -> Result<PathBuf, StorageError> {
    let clean = rel.trim().trim_start_matches('/');
    if clean.is_empty() || clean.contains('\0') {
        return Err(StorageError::InvalidKey);
    }
    let rel_path = Path::new(clean);
    if rel_path.is_absolute() {
        return Err(StorageError::InvalidKey);
    }
    let mut out = root.to_path_buf();
    for comp in rel_path.components() {
        match comp {
            Component::Normal(p) => out.push(p),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(StorageError::InvalidKey);
            }
        }
    }
    Ok(out)
}

pub async fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), StorageError> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let tmp = path.with_extension(format!("tmp.{}", uuid::Uuid::new_v4().simple()));
    tokio::fs::write(&tmp, bytes).await?;
    if let Err(e) = tokio::fs::rename(&tmp, path).await {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(StorageError::Io(e));
    }
    Ok(())
}

pub async fn read_file_range(path: &Path, start: u64, end_inclusive: u64) -> Result<(Vec<u8>, u64), StorageError> {
    let mut file = tokio::fs::File::open(path).await?;
    let total_size = file.metadata().await?.len();
    if total_size == 0 {
        return Ok((Vec::new(), 0));
    }
    if start >= total_size || end_inclusive < start {
        return Err(StorageError::InvalidKey);
    }
    let clamped_end = end_inclusive.min(total_size - 1);
    let len = (clamped_end - start + 1).min(16 * 1024 * 1024) as usize;
    file.seek(std::io::SeekFrom::Start(start)).await?;
    let mut buf = vec![0u8; len];
    let mut read_total = 0usize;
    while read_total < len {
        let n = file.read(&mut buf[read_total..]).await?;
        if n == 0 {
            break;
        }
        read_total += n;
    }
    buf.truncate(read_total);
    Ok((buf, total_size))
}

pub async fn stream_file(path: PathBuf) -> Result<Body, StorageError> {
    let file = tokio::fs::File::open(&path).await?;
    let stream = futures_util::stream::try_unfold(file, |mut f| async move {
        let mut buf = vec![0u8; 64 * 1024];
        let n = f.read(&mut buf).await?;
        if n == 0 {
            Ok::<Option<(Bytes, tokio::fs::File)>, std::io::Error>(None)
        } else {
            buf.truncate(n);
            Ok(Some((Bytes::from(buf), f)))
        }
    });
    Ok(Body::from_stream(stream))
}
