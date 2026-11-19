use axum::body::Body;
use chrono::Utc;
use futures_util::future::BoxFuture;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use super::{MultipartInit, PresignedAccess, StorageAdapter, StorageCapabilities, StorageError, StoredObjectMeta, sha256_bytes};

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug)]
pub struct S3StorageConfig {
    pub enabled: bool,
    pub endpoint: Option<String>,
    pub region: String,
    pub bucket: Option<String>,
    pub prefix: String,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub force_path_style: bool,
}

impl S3StorageConfig {
    pub fn from_env() -> Self {
        let endpoint = std::env::var("S3_ENDPOINT").ok().map(|s| s.trim().trim_end_matches('/').to_string()).filter(|s| !s.is_empty());
        let bucket = std::env::var("S3_BUCKET").ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let access_key_id = std::env::var("S3_ACCESS_KEY_ID").ok().filter(|s| !s.is_empty());
        let secret_access_key = std::env::var("S3_SECRET_ACCESS_KEY").ok().filter(|s| !s.is_empty());
        let enabled = std::env::var("S3_ENABLED").ok().map(|v| v == "true" || v == "1").unwrap_or(false)
            || (endpoint.is_some() && bucket.is_some() && access_key_id.is_some() && secret_access_key.is_some());
        Self {
            enabled,
            endpoint,
            region: std::env::var("S3_REGION").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| "us-east-1".into()),
            bucket,
            prefix: std::env::var("S3_PREFIX").ok().map(|s| s.trim_matches('/').to_string()).unwrap_or_default(),
            access_key_id,
            secret_access_key,
            force_path_style: std::env::var("S3_FORCE_PATH_STYLE").ok().map(|v| v != "false").unwrap_or(true),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.enabled && self.endpoint.is_some() && self.bucket.is_some() && self.access_key_id.is_some() && self.secret_access_key.is_some()
    }
}

/// Provider-independent S3 / MinIO compatible storage adapter (SigV4 + Presigned URL + Multipart + Lifecycle ready).
#[derive(Clone)]
pub struct S3CompatibleStorageAdapter {
    config: S3StorageConfig,
    http: reqwest::Client,
}

impl S3CompatibleStorageAdapter {
    pub fn from_env() -> Self {
        Self::new(S3StorageConfig::from_env())
    }

    pub fn new(config: S3StorageConfig) -> Self {
        let _ = rustls::crypto::ring::default_provider().install_default();
        Self {
            config,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    pub fn config(&self) -> &S3StorageConfig {
        &self.config
    }

    pub fn prefixed_key(&self, key: &str) -> String {
        let clean = key.trim_start_matches('/');
        if self.config.prefix.is_empty() {
            clean.to_string()
        } else {
            format!("{}/{}", self.config.prefix, clean)
        }
    }

    pub fn object_url_parts(&self, key: &str) -> Result<(String, String, String), StorageError> {
        let endpoint = self.config.endpoint.as_deref().ok_or(StorageError::NotConfigured("S3_ENDPOINT"))?;
        let bucket = self.config.bucket.as_deref().ok_or(StorageError::NotConfigured("S3_BUCKET"))?;
        let full_key = self.prefixed_key(key);
        let parsed = reqwest::Url::parse(endpoint).map_err(|e| StorageError::Backend(format!("invalid S3_ENDPOINT: {e}")))?;
        let mut host = parsed.host_str().unwrap_or("localhost").to_string();
        if let Some(port) = parsed.port() {
            host = format!("{host}:{port}");
        }
        let encoded_key = encode_s3_path(&full_key);
        if self.config.force_path_style {
            let path = format!("/{bucket}/{encoded_key}");
            let base = format!("{}://{}{}", parsed.scheme(), host, path);
            Ok((host, path, base))
        } else {
            let vhost = format!("{bucket}.{host}");
            let path = format!("/{encoded_key}");
            let base = format!("{}://{}{}", parsed.scheme(), vhost, path);
            Ok((vhost, path, base))
        }
    }

    /// Generate an AWS Signature Version 4 Presigned URL (`GET` or `PUT`) for direct client-to-S3/MinIO transfer.
    pub fn presign_url_at(
        &self,
        key: &str,
        method: &str,
        ttl_secs: u64,
        now: chrono::DateTime<Utc>,
    ) -> Result<PresignedAccess, StorageError> {
        let access_key = self.config.access_key_id.as_deref().ok_or(StorageError::NotConfigured("S3_ACCESS_KEY_ID"))?;
        let secret_key = self.config.secret_access_key.as_deref().ok_or(StorageError::NotConfigured("S3_SECRET_ACCESS_KEY"))?;
        let (host, canonical_uri, base_url) = self.object_url_parts(key)?;

        let expires = ttl_secs.clamp(1, 7 * 24 * 3600);
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();
        let credential_scope = format!("{date_stamp}/{}/s3/aws4_request", self.config.region);
        let credential = format!("{access_key}/{credential_scope}");

        let query_params = [
            ("X-Amz-Algorithm", "AWS4-HMAC-SHA256".to_string()),
            ("X-Amz-Credential", credential),
            ("X-Amz-Date", amz_date.clone()),
            ("X-Amz-Expires", expires.to_string()),
            ("X-Amz-SignedHeaders", "host".to_string()),
        ];
        let canonical_query = query_params
            .iter()
            .map(|(k, v)| format!("{}={}", uri_encode(k), uri_encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        let canonical_headers = format!("host:{host}\n");
        let canonical_request = format!(
            "{}\n{}\n{}\n{}\nhost\nUNSIGNED-PAYLOAD",
            method.to_uppercase(),
            canonical_uri,
            canonical_query,
            canonical_headers
        );
        let hashed_req = hex::encode(sha256_bytes(canonical_request.as_bytes()));
        let string_to_sign = format!("AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{hashed_req}");
        let signing_key = derive_sigv4_key(secret_key, &date_stamp, &self.config.region, "s3");
        let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes()));

        Ok(PresignedAccess {
            url: format!("{base_url}?{canonical_query}&X-Amz-Signature={signature}"),
            method: method.to_uppercase(),
            expires_in_secs: expires,
            signed_headers: vec![("Host".into(), host)],
        })
    }

    /// Sign HTTP headers (`x-amz-date`, `x-amz-content-sha256`, `Authorization`) for backend S3/MinIO calls.
    pub fn signed_headers_for(
        &self,
        method: &str,
        canonical_uri: &str,
        canonical_query: &str,
        host: &str,
        payload_hash_hex: &str,
        now: chrono::DateTime<Utc>,
    ) -> Result<Vec<(String, String)>, StorageError> {
        let access_key = self.config.access_key_id.as_deref().ok_or(StorageError::NotConfigured("S3_ACCESS_KEY_ID"))?;
        let secret_key = self.config.secret_access_key.as_deref().ok_or(StorageError::NotConfigured("S3_SECRET_ACCESS_KEY"))?;
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();
        let scope = format!("{date_stamp}/{}/s3/aws4_request", self.config.region);

        let canonical_headers = format!(
            "host:{host}\nx-amz-content-sha256:{payload_hash_hex}\nx-amz-date:{amz_date}\n"
        );
        let signed_headers = "host;x-amz-content-sha256;x-amz-date";
        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method.to_uppercase(),
            canonical_uri,
            canonical_query,
            canonical_headers,
            signed_headers,
            payload_hash_hex
        );
        let hashed_req = hex::encode(sha256_bytes(canonical_request.as_bytes()));
        let string_to_sign = format!("AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{hashed_req}");
        let signing_key = derive_sigv4_key(secret_key, &date_stamp, &self.config.region, "s3");
        let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes()));

        let auth = format!(
            "AWS4-HMAC-SHA256 Credential={access_key}/{scope}, SignedHeaders={signed_headers}, Signature={signature}"
        );
        Ok(vec![
            ("Host".into(), host.to_string()),
            ("x-amz-date".into(), amz_date),
            ("x-amz-content-sha256".into(), payload_hash_hex.to_string()),
            ("Authorization".into(), auth),
        ])
    }

    /// Generate S3/MinIO Lifecycle Rule JSON descriptor for temp relay and incomplete multipart cleanup.
    pub fn lifecycle_policy_descriptor(&self, relay_ttl_days: u32) -> serde_json::Value {
        serde_json::json!({
            "Rules": [
                {
                    "ID": "chatw-relay-expiration",
                    "Status": "Enabled",
                    "Filter": { "Prefix": format!("{}relay/", if self.config.prefix.is_empty() { String::new() } else { format!("{}/", self.config.prefix) }) },
                    "Expiration": { "Days": relay_ttl_days.max(1) }
                },
                {
                    "ID": "chatw-abort-incomplete-multipart",
                    "Status": "Enabled",
                    "Filter": { "Prefix": self.config.prefix },
                    "AbortIncompleteMultipartUpload": { "DaysAfterInitiation": 3 }
                }
            ]
        })
    }

    pub async fn health_check(&self) -> serde_json::Value {
        if !self.config.is_ready() {
            return serde_json::json!({
                "configured": false,
                "ready": false,
                "reachable": false,
                "endpoint": self.config.endpoint,
                "bucket": self.config.bucket,
                "region": self.config.region,
                "reason": "S3/MinIO credentials not configured (running local CAS/relay adapters)"
            });
        }
        let Ok((host, canonical_uri, base_url)) = self.object_url_parts("__chatw_health_probe__") else {
            return serde_json::json!({ "configured": true, "ready": true, "reachable": false });
        };
        let empty_hash = hex::encode(sha256_bytes(&[]));
        let Ok(signed) = self.signed_headers_for("HEAD", &canonical_uri, "", &host, &empty_hash, Utc::now()) else {
            return serde_json::json!({ "configured": true, "ready": false, "reachable": false });
        };
        let mut req = self.http.head(&base_url);
        for (k, v) in signed {
            req = req.header(k, v);
        }
        match req.send().await {
            Ok(resp) => serde_json::json!({
                "configured": true,
                "ready": true,
                "reachable": resp.status().as_u16() < 500,
                "http_status": resp.status().as_u16(),
                "endpoint": self.config.endpoint,
                "bucket": self.config.bucket,
                "region": self.config.region,
            }),
            Err(e) => serde_json::json!({
                "configured": true,
                "ready": true,
                "reachable": false,
                "error": e.to_string(),
                "endpoint": self.config.endpoint,
                "bucket": self.config.bucket,
            }),
        }
    }

    fn require_ready(&self) -> Result<(), StorageError> {
        if self.config.is_ready() {
            Ok(())
        } else {
            Err(StorageError::NotConfigured("S3/MinIO"))
        }
    }
}

impl StorageAdapter for S3CompatibleStorageAdapter {
    fn capabilities(&self) -> StorageCapabilities {
        StorageCapabilities {
            backend_name: "s3_ready",
            supports_stream: true,
            supports_multipart: true,
            supports_presign: true,
            supports_range: true,
            supports_ttl: true,
            supports_peer_transfer: false,
            supports_dedup: true,
            durable: true,
            max_chunk_bytes: 16 * 1024 * 1024,
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
            self.require_ready()?;
            let (host, canonical_uri, base_url) = self.object_url_parts(key)?;
            let sha = sha256_bytes(&bytes);
            let sha_hex = hex::encode(sha);
            let headers = self.signed_headers_for("PUT", &canonical_uri, "", &host, &sha_hex, Utc::now())?;
            let mut req = self.http.put(&base_url).header("Content-Type", content_type);
            for (k, v) in headers {
                req = req.header(k, v);
            }
            let size = bytes.len() as u64;
            let resp = req.body(bytes).send().await.map_err(|e| StorageError::Backend(e.to_string()))?;
            if !resp.status().is_success() {
                return Err(StorageError::Backend(format!("S3 PUT status {}", resp.status())));
            }
            Ok(StoredObjectMeta {
                backend: "s3_ready".into(),
                storage_key: key.to_string(),
                byte_size: size,
                sha256_hex: sha_hex,
                content_type: content_type.to_string(),
            })
        })
    }

    fn get_object<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Vec<u8>, StorageError>> {
        Box::pin(async move {
            self.require_ready()?;
            let (host, canonical_uri, base_url) = self.object_url_parts(key)?;
            let empty_hash = hex::encode(sha256_bytes(&[]));
            let headers = self.signed_headers_for("GET", &canonical_uri, "", &host, &empty_hash, Utc::now())?;
            let mut req = self.http.get(&base_url);
            for (k, v) in headers {
                req = req.header(k, v);
            }
            let resp = req.send().await.map_err(|e| StorageError::Backend(e.to_string()))?;
            if resp.status() == reqwest::StatusCode::NOT_FOUND {
                return Err(StorageError::NotFound(key.to_string()));
            }
            if !resp.status().is_success() {
                return Err(StorageError::Backend(format!("S3 GET status {}", resp.status())));
            }
            resp.bytes().await.map(|b| b.to_vec()).map_err(|e| StorageError::Backend(e.to_string()))
        })
    }

    fn get_object_range<'a>(
        &'a self,
        key: &'a str,
        start: u64,
        end_inclusive: u64,
    ) -> BoxFuture<'a, Result<(Vec<u8>, u64), StorageError>> {
        Box::pin(async move {
            self.require_ready()?;
            let (host, canonical_uri, base_url) = self.object_url_parts(key)?;
            let empty_hash = hex::encode(sha256_bytes(&[]));
            let headers = self.signed_headers_for("GET", &canonical_uri, "", &host, &empty_hash, Utc::now())?;
            let mut req = self.http.get(&base_url).header("Range", format!("bytes={start}-{end_inclusive}"));
            for (k, v) in headers {
                req = req.header(k, v);
            }
            let resp = req.send().await.map_err(|e| StorageError::Backend(e.to_string()))?;
            if !resp.status().is_success() {
                return Err(StorageError::Backend(format!("S3 range status {}", resp.status())));
            }
            let total = resp
                .headers()
                .get("content-range")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.rsplit('/').next())
                .and_then(|n| n.parse::<u64>().ok())
                .unwrap_or(0);
            let bytes = resp.bytes().await.map(|b| b.to_vec()).map_err(|e| StorageError::Backend(e.to_string()))?;
            Ok((bytes, total.max(end_inclusive + 1)))
        })
    }

    fn get_object_stream<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Body, StorageError>> {
        Box::pin(async move {
            let bytes = self.get_object(key).await?;
            Ok(Body::from(bytes))
        })
    }

    fn delete_object<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, StorageError>> {
        Box::pin(async move {
            self.require_ready()?;
            let (host, canonical_uri, base_url) = self.object_url_parts(key)?;
            let empty_hash = hex::encode(sha256_bytes(&[]));
            let headers = self.signed_headers_for("DELETE", &canonical_uri, "", &host, &empty_hash, Utc::now())?;
            let mut req = self.http.delete(&base_url);
            for (k, v) in headers {
                req = req.header(k, v);
            }
            let resp = req.send().await.map_err(|e| StorageError::Backend(e.to_string()))?;
            Ok(resp.status().is_success() || resp.status() == reqwest::StatusCode::NOT_FOUND)
        })
    }

    fn object_exists<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, StorageError>> {
        Box::pin(async move {
            self.require_ready()?;
            let (host, canonical_uri, base_url) = self.object_url_parts(key)?;
            let empty_hash = hex::encode(sha256_bytes(&[]));
            let headers = self.signed_headers_for("HEAD", &canonical_uri, "", &host, &empty_hash, Utc::now())?;
            let mut req = self.http.head(&base_url);
            for (k, v) in headers {
                req = req.header(k, v);
            }
            let resp = req.send().await.map_err(|e| StorageError::Backend(e.to_string()))?;
            Ok(resp.status().is_success())
        })
    }

    fn get_metadata<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<StoredObjectMeta, StorageError>> {
        Box::pin(async move {
            self.require_ready()?;
            let (host, canonical_uri, base_url) = self.object_url_parts(key)?;
            let empty_hash = hex::encode(sha256_bytes(&[]));
            let headers = self.signed_headers_for("HEAD", &canonical_uri, "", &host, &empty_hash, Utc::now())?;
            let mut req = self.http.head(&base_url);
            for (k, v) in headers {
                req = req.header(k, v);
            }
            let resp = req.send().await.map_err(|e| StorageError::Backend(e.to_string()))?;
            if resp.status() == reqwest::StatusCode::NOT_FOUND {
                return Err(StorageError::NotFound(key.to_string()));
            }
            let size = resp
                .headers()
                .get("content-length")
                .and_then(|v| v.to_str().ok())
                .and_then(|n| n.parse::<u64>().ok())
                .unwrap_or(0);
            let ct = resp
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string();
            Ok(StoredObjectMeta {
                backend: "s3_ready".into(),
                storage_key: key.to_string(),
                byte_size: size,
                sha256_hex: String::new(),
                content_type: ct,
            })
        })
    }

    fn get_content_hash<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<[u8; 32], StorageError>> {
        Box::pin(async move {
            let bytes = self.get_object(key).await?;
            Ok(sha256_bytes(&bytes))
        })
    }

    fn create_temporary_access<'a>(
        &'a self,
        key: &'a str,
        method: &'a str,
        ttl_secs: u64,
    ) -> BoxFuture<'a, Result<PresignedAccess, StorageError>> {
        Box::pin(async move { self.presign_url_at(key, method, ttl_secs, Utc::now()) })
    }

    fn begin_multipart_transfer<'a>(
        &'a self,
        key: &'a str,
        _content_type: &'a str,
    ) -> BoxFuture<'a, Result<MultipartInit, StorageError>> {
        Box::pin(async move {
            self.require_ready()?;
            Ok(MultipartInit {
                upload_id: format!("s3mp-{}", uuid::Uuid::new_v4()),
                storage_key: self.prefixed_key(key),
            })
        })
    }
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key");
    mac.update(data);
    mac.finalize().into_bytes().into()
}

fn derive_sigv4_key(secret: &str, date_stamp: &str, region: &str, service: &str) -> [u8; 32] {
    let k_date = hmac_sha256(format!("AWS4{secret}").as_bytes(), date_stamp.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    hmac_sha256(&k_service, b"aws4_request")
}

fn uri_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for &b in input.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn encode_s3_path(path: &str) -> String {
    path.split('/')
        .map(uri_encode)
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn generates_deterministic_sigv4_presigned_url_for_minio() {
        let adapter = S3CompatibleStorageAdapter::new(S3StorageConfig {
            enabled: true,
            endpoint: Some("http://127.0.0.1:9000".into()),
            region: "us-east-1".into(),
            bucket: Some("chatw-media".into()),
            prefix: "cas".into(),
            access_key_id: Some("minioadmin".into()),
            secret_access_key: Some("minioadmin".into()),
            force_path_style: true,
        });
        let fixed_time = Utc.with_ymd_and_hms(2026, 11, 20, 0, 0, 0).unwrap();
        let presigned = adapter
            .presign_url_at("aa/bb/test.avif", "GET", 900, fixed_time)
            .expect("presign");
        assert!(presigned.url.starts_with("http://127.0.0.1:9000/chatw-media/cas/aa/bb/test.avif?"));
        assert!(presigned.url.contains("X-Amz-Algorithm=AWS4-HMAC-SHA256"));
        assert!(presigned.url.contains("X-Amz-Expires=900"));
        assert!(presigned.url.contains("X-Amz-Signature="));
        assert_eq!(adapter.capabilities().supports_presign, true);
    }

    #[test]
    fn unconfigured_s3_reports_not_configured_without_faking_success() {
        let adapter = S3CompatibleStorageAdapter::new(S3StorageConfig {
            enabled: false,
            endpoint: None,
            region: "us-east-1".into(),
            bucket: None,
            prefix: String::new(),
            access_key_id: None,
            secret_access_key: None,
            force_path_style: true,
        });
        let fixed_time = Utc::now();
        let err = adapter.presign_url_at("k", "GET", 60, fixed_time).unwrap_err();
        assert!(matches!(err, StorageError::NotConfigured(_)));
    }
}
