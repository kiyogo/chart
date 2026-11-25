use axum::body::Body;
use futures_util::future::BoxFuture;
use std::path::{Path, PathBuf};

use super::{
    MultipartInit, StorageAdapter, StorageCapabilities, StorageError, StoredObjectMeta,
    atomic_write, read_file_range, safe_join_relative, sha256_bytes, sha256_file, stream_file,
};

/// Content-Addressable Storage (CAS) on local disk (`{cas_dir}/{aa}/{bb}/{sha256}.{ext}`).
pub struct LocalFileStorageAdapter {
    cas_dir: PathBuf,
}

impl LocalFileStorageAdapter {
    pub fn new(cas_dir: PathBuf) -> Self {
        Self { cas_dir }
    }

    pub fn root(&self) -> &Path {
        &self.cas_dir
    }

    pub fn cas_key_for_sha(sha256: &[u8; 32], ext: &str) -> String {
        let hex_hash = hex::encode(sha256);
        let safe_ext: String = ext
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .take(12)
            .collect();
        let suffix = if safe_ext.is_empty() { "bin".to_string() } else { safe_ext };
        format!("{}/{}/{}.{}", &hex_hash[0..2], &hex_hash[2..4], hex_hash, suffix)
    }

    fn chunks_dir(&self, transfer_id: &str) -> Result<PathBuf, StorageError> {
        safe_join_relative(&self.cas_dir.join("_multipart"), transfer_id)
    }
}

impl StorageAdapter for LocalFileStorageAdapter {
    fn capabilities(&self) -> StorageCapabilities {
        StorageCapabilities {
            backend_name: "local_cas",
            supports_stream: true,
            supports_multipart: true,
            supports_presign: false,
            supports_range: true,
            supports_ttl: false,
            supports_peer_transfer: false,
            supports_dedup: true,
            durable: true,
            max_chunk_bytes: 8 * 1024 * 1024,
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
            let path = safe_join_relative(&self.cas_dir, key)?;
            let sha = sha256_bytes(&bytes);
            if !path.exists() {
                atomic_write(&path, &bytes).await?;
            }
            Ok(StoredObjectMeta {
                backend: "local_cas".into(),
                storage_key: key.to_string(),
                byte_size: bytes.len() as u64,
                sha256_hex: hex::encode(sha),
                content_type: content_type.to_string(),
            })
        })
    }

    fn get_object<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Vec<u8>, StorageError>> {
        Box::pin(async move {
            let path = safe_join_relative(&self.cas_dir, key)?;
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
            let path = safe_join_relative(&self.cas_dir, key)?;
            read_file_range(&path, start, end_inclusive).await
        })
    }

    fn get_object_stream<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Body, StorageError>> {
        Box::pin(async move {
            let path = safe_join_relative(&self.cas_dir, key)?;
            stream_file(path).await
        })
    }

    fn delete_object<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, StorageError>> {
        Box::pin(async move {
            let path = safe_join_relative(&self.cas_dir, key)?;
            match tokio::fs::remove_file(path).await {
                Ok(()) => Ok(true),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(e) => Err(StorageError::Io(e)),
            }
        })
    }

    fn object_exists<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, StorageError>> {
        Box::pin(async move { Ok(safe_join_relative(&self.cas_dir, key)?.exists()) })
    }

    fn get_metadata<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<StoredObjectMeta, StorageError>> {
        Box::pin(async move {
            let path = safe_join_relative(&self.cas_dir, key)?;
            let (byte_size, sha) = sha256_file(&path).await?;
            Ok(StoredObjectMeta {
                backend: "local_cas".into(),
                storage_key: key.to_string(),
                byte_size,
                sha256_hex: hex::encode(sha),
                content_type: "application/octet-stream".into(),
            })
        })
    }

    fn get_content_hash<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<[u8; 32], StorageError>> {
        Box::pin(async move {
            let path = safe_join_relative(&self.cas_dir, key)?;
            Ok(sha256_file(&path).await?.1)
        })
    }

    fn begin_multipart_transfer<'a>(
        &'a self,
        key: &'a str,
        _content_type: &'a str,
    ) -> BoxFuture<'a, Result<MultipartInit, StorageError>> {
        Box::pin(async move {
            let upload_id = uuid::Uuid::new_v4().to_string();
            let dir = self.chunks_dir(&upload_id)?;
            tokio::fs::create_dir_all(&dir).await?;
            Ok(MultipartInit {
                upload_id,
                storage_key: key.to_string(),
            })
        })
    }

    fn upload_chunk<'a>(
        &'a self,
        transfer_id: &'a str,
        chunk_index: u32,
        bytes: Vec<u8>,
    ) -> BoxFuture<'a, Result<String, StorageError>> {
        Box::pin(async move {
            let dir = self.chunks_dir(transfer_id)?;
            let chunk_path = dir.join(format!("{chunk_index:08}.part"));
            let hash = hex::encode(sha256_bytes(&bytes));
            atomic_write(&chunk_path, &bytes).await?;
            Ok(hash)
        })
    }
}

impl LocalFileStorageAdapter {
    /// Assemble sequential uploaded chunks cleanly without wiping staging until caller verifies integrity.
    pub async fn assemble_chunks(&self, transfer_id: &str, total_chunks: u32) -> Result<Vec<u8>, StorageError> {
        let dir = self.chunks_dir(transfer_id)?;
        let mut out = Vec::new();
        for idx in 0..total_chunks {
            let part = dir.join(format!("{idx:08}.part"));
            let chunk = tokio::fs::read(&part).await?;
            out.extend_from_slice(&chunk);
        }
        Ok(out)
    }

    /// Clean up an abandoned or aborted multipart upload staging directory.
    pub async fn abort_chunks(&self, transfer_id: &str) -> Result<(), StorageError> {
        let dir = self.chunks_dir(transfer_id)?;
        if dir.exists() {
            let _ = tokio::fs::remove_dir_all(&dir).await;
        }
        Ok(())
    }
}
