use axum::body::Body;
use futures_util::future::BoxFuture;
use std::path::PathBuf;

use super::{
    StorageAdapter, StorageCapabilities, StorageError, StoredObjectMeta, atomic_write,
    read_file_range, safe_join_relative, sha256_bytes, sha256_file, stream_file,
};

/// Opt-in E2EE encrypted server backup for DM media/vault payloads (server only holds opaque ciphertext).
pub struct EncryptedBackupStorageAdapter {
    backup_dir: PathBuf,
}

impl EncryptedBackupStorageAdapter {
    pub fn new(backup_dir: PathBuf) -> Self {
        Self { backup_dir }
    }

    pub fn root(&self) -> &PathBuf {
        &self.backup_dir
    }
}

impl StorageAdapter for EncryptedBackupStorageAdapter {
    fn capabilities(&self) -> StorageCapabilities {
        StorageCapabilities {
            backend_name: "encrypted_backup",
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
            let path = safe_join_relative(&self.backup_dir, key)?;
            let sha = sha256_bytes(&bytes);
            atomic_write(&path, &bytes).await?;
            Ok(StoredObjectMeta {
                backend: "encrypted_backup".into(),
                storage_key: key.to_string(),
                byte_size: bytes.len() as u64,
                sha256_hex: hex::encode(sha),
                content_type: content_type.to_string(),
            })
        })
    }

    fn get_object<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Vec<u8>, StorageError>> {
        Box::pin(async move {
            let path = safe_join_relative(&self.backup_dir, key)?;
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
            let path = safe_join_relative(&self.backup_dir, key)?;
            read_file_range(&path, start, end_inclusive).await
        })
    }

    fn get_object_stream<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Body, StorageError>> {
        Box::pin(async move {
            let path = safe_join_relative(&self.backup_dir, key)?;
            stream_file(path).await
        })
    }

    fn delete_object<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, StorageError>> {
        Box::pin(async move {
            let path = safe_join_relative(&self.backup_dir, key)?;
            match tokio::fs::remove_file(path).await {
                Ok(()) => Ok(true),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(e) => Err(StorageError::Io(e)),
            }
        })
    }

    fn object_exists<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, StorageError>> {
        Box::pin(async move { Ok(safe_join_relative(&self.backup_dir, key)?.exists()) })
    }

    fn get_metadata<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<StoredObjectMeta, StorageError>> {
        Box::pin(async move {
            let path = safe_join_relative(&self.backup_dir, key)?;
            let (byte_size, sha) = sha256_file(&path).await?;
            Ok(StoredObjectMeta {
                backend: "encrypted_backup".into(),
                storage_key: key.to_string(),
                byte_size,
                sha256_hex: hex::encode(sha),
                content_type: "application/octet-stream".into(),
            })
        })
    }

    fn get_content_hash<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<[u8; 32], StorageError>> {
        Box::pin(async move {
            let path = safe_join_relative(&self.backup_dir, key)?;
            Ok(sha256_file(&path).await?.1)
        })
    }
}
