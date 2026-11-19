use axum::body::Body;
use futures_util::future::BoxFuture;
use std::path::PathBuf;

use super::{
    StorageAdapter, StorageCapabilities, StorageError, StoredObjectMeta, atomic_write,
    read_file_range, safe_join_relative, sha256_bytes, sha256_file, stream_file,
};

/// Reads and serves legacy `{uploads_dir}/{attachments,media,avatars,banners,stickers}/...` files safely.
pub struct ExistingStorageAdapter {
    uploads_dir: PathBuf,
}

impl ExistingStorageAdapter {
    pub fn new(uploads_dir: PathBuf) -> Self {
        Self { uploads_dir }
    }

    pub fn uploads_root(&self) -> &PathBuf {
        &self.uploads_dir
    }
}

impl StorageAdapter for ExistingStorageAdapter {
    fn capabilities(&self) -> StorageCapabilities {
        StorageCapabilities {
            backend_name: "legacy",
            supports_stream: true,
            supports_multipart: false,
            supports_presign: false,
            supports_range: true,
            supports_ttl: false,
            supports_peer_transfer: false,
            supports_dedup: false,
            durable: true,
            max_chunk_bytes: 0,
        }
    }

    fn put_object<'a>(
        &'a self,
        key: &'a str,
        bytes: Vec<u8>,
        content_type: &'a str,
        _ttl_secs: Option<u64>,
    ) -> BoxFuture<'a, Result<StoredObjectMeta, StorageError>> {
        Box::pin(async move {
            let path = safe_join_relative(&self.uploads_dir, key)?;
            let sha = sha256_bytes(&bytes);
            atomic_write(&path, &bytes).await?;
            Ok(StoredObjectMeta {
                backend: "legacy".into(),
                storage_key: key.to_string(),
                byte_size: bytes.len() as u64,
                sha256_hex: hex::encode(sha),
                content_type: content_type.to_string(),
            })
        })
    }

    fn get_object<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Vec<u8>, StorageError>> {
        Box::pin(async move {
            let path = resolve_existing_path(&self.uploads_dir, key)?;
            Ok(tokio::fs::read(path).await?)
        })
    }

    fn get_object_range<'a>(
        &'a self,
        key: &'a str,
        start: u64,
        end_inclusive: u64,
    ) -> BoxFuture<'a, Result<(Vec<u8>, u64), StorageError>> {
        Box::pin(async move {
            let path = resolve_existing_path(&self.uploads_dir, key)?;
            read_file_range(&path, start, end_inclusive).await
        })
    }

    fn get_object_stream<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Body, StorageError>> {
        Box::pin(async move {
            let path = resolve_existing_path(&self.uploads_dir, key)?;
            stream_file(path).await
        })
    }

    fn delete_object<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, StorageError>> {
        Box::pin(async move {
            let path = resolve_existing_path(&self.uploads_dir, key)?;
            match tokio::fs::remove_file(path).await {
                Ok(()) => Ok(true),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(e) => Err(StorageError::Io(e)),
            }
        })
    }

    fn object_exists<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, StorageError>> {
        Box::pin(async move {
            Ok(resolve_existing_path(&self.uploads_dir, key)
                .map(|p| p.exists())
                .unwrap_or(false))
        })
    }

    fn get_metadata<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<StoredObjectMeta, StorageError>> {
        Box::pin(async move {
            let path = resolve_existing_path(&self.uploads_dir, key)?;
            let (byte_size, sha) = sha256_file(&path).await?;
            Ok(StoredObjectMeta {
                backend: "legacy".into(),
                storage_key: key.to_string(),
                byte_size,
                sha256_hex: hex::encode(sha),
                content_type: "application/octet-stream".into(),
            })
        })
    }

    fn get_content_hash<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<[u8; 32], StorageError>> {
        Box::pin(async move {
            let path = resolve_existing_path(&self.uploads_dir, key)?;
            Ok(sha256_file(&path).await?.1)
        })
    }
}

/// Supports both relative keys ("attachments/uuid.bin") and legacy absolute paths saved under uploads_dir.
fn resolve_existing_path(uploads_dir: &std::path::Path, key: &str) -> Result<PathBuf, StorageError> {
    let candidate = PathBuf::from(key);
    if candidate.is_absolute() {
        let canon_root = uploads_dir.canonicalize().unwrap_or_else(|_| uploads_dir.to_path_buf());
        let canon_target = candidate.canonicalize().map_err(StorageError::Io)?;
        if canon_target.starts_with(&canon_root) {
            return Ok(canon_target);
        }
        return Err(StorageError::InvalidKey);
    }
    safe_join_relative(uploads_dir, key)
}
