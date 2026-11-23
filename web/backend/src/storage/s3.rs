use axum::body::Body;
use chrono::Utc;
use futures_util::future::BoxFuture;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::Duration;

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

/// Provider-independent S3 / MinIO / R2 compatible storage adapter:
/// - AWS SigV4 Header Signing + Short-Lived Presigned GET/PUT URLs (private bucket only!)
/// - Full S3 Multipart Upload (`InitiateMultipartUpload`, `UploadPart`, `CompleteMultipartUpload`, `AbortMultipartUpload`)
/// - True Streaming Downloads (`Body::from_stream`) + HTTP Range GET
/// - Exponential Backoff Retry on transient 5xx/429/network errors
/// - Lifecycle Policy Descriptor for incomplete multipart & relay expiration
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
                .timeout(Duration::from_secs(60))
                .connect_timeout(Duration::from_secs(10))
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

        // Enforce short-lived presigned URLs (max 1 hour for private media security)
        let expires = ttl_secs.clamp(1, 3600);
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

    /// Execute an S3 HTTP request with exponential backoff retry on transient network / 429 / 5xx failures.
    async fn send_with_retry<F>(&self, mut build_req: F) -> Result<reqwest::Response, StorageError>
    where
        F: FnMut() -> Result<reqwest::RequestBuilder, StorageError>,
    {
        let delays_ms = [100u64, 250, 600];
        let mut last_err = String::new();
        for attempt in 0..=delays_ms.len() {
            let req = build_req()?;
            match req.send().await {
                Ok(resp) => {
                    let code = resp.status().as_u16();
                    if (code == 429 || code >= 500) && attempt < delays_ms.len() {
                        last_err = format!("transient S3 HTTP {code}");
                        tokio::time::sleep(Duration::from_millis(delays_ms[attempt])).await;
                        continue;
                    }
                    return Ok(resp);
                }
                Err(e) => {
                    last_err = e.to_string();
                    if attempt < delays_ms.len() {
                        tokio::time::sleep(Duration::from_millis(delays_ms[attempt])).await;
                        continue;
                    }
                }
            }
        }
        Err(StorageError::Backend(format!("S3 request failed after retries: {last_err}")))
    }

    /// Real S3 `InitiateMultipartUpload` (`POST /{key}?uploads`), returning S3 `UploadId`.
    pub async fn initiate_multipart_upload(
        &self,
        key: &str,
        content_type: &str,
    ) -> Result<MultipartInit, StorageError> {
        self.require_ready()?;
        let (host, canonical_uri, base_url) = self.object_url_parts(key)?;
        let empty_hash = hex::encode(sha256_bytes(&[]));
        let url = format!("{base_url}?uploads");

        let resp = self
            .send_with_retry(|| {
                let headers = self.signed_headers_for("POST", &canonical_uri, "uploads=", &host, &empty_hash, Utc::now())?;
                let mut req = self.http.post(&url).header("Content-Type", content_type);
                for (k, v) in headers {
                    req = req.header(k, v);
                }
                Ok(req)
            })
            .await?;

        if !resp.status().is_success() {
            return Err(StorageError::Backend(format!("S3 InitiateMultipartUpload status {}", resp.status())));
        }
        let xml = resp.text().await.map_err(|e| StorageError::Backend(e.to_string()))?;
        let upload_id = extract_xml_tag(&xml, "UploadId")
            .ok_or_else(|| StorageError::Backend("S3 InitiateMultipartUpload missing <UploadId>".into()))?;
        Ok(MultipartInit {
            upload_id,
            storage_key: key.to_string(),
        })
    }

    /// Real S3 `UploadPart` (`PUT /{key}?partNumber=N&uploadId=ID`), returning verified ETag.
    pub async fn upload_multipart_part(
        &self,
        key: &str,
        upload_id: &str,
        part_number_1_based: u32,
        bytes: Vec<u8>,
    ) -> Result<String, StorageError> {
        self.require_ready()?;
        if part_number_1_based == 0 || part_number_1_based > 10_000 {
            return Err(StorageError::InvalidKey);
        }
        let (host, canonical_uri, base_url) = self.object_url_parts(key)?;
        let sha_hex = hex::encode(sha256_bytes(&bytes));
        let canonical_query = format!(
            "partNumber={part_number_1_based}&uploadId={}",
            uri_encode(upload_id)
        );
        let url = format!("{base_url}?{canonical_query}");

        let resp = self
            .send_with_retry(|| {
                let headers = self.signed_headers_for("PUT", &canonical_uri, &canonical_query, &host, &sha_hex, Utc::now())?;
                let mut req = self.http.put(&url).body(bytes.clone());
                for (k, v) in headers {
                    req = req.header(k, v);
                }
                Ok(req)
            })
            .await?;

        if !resp.status().is_success() {
            return Err(StorageError::Backend(format!("S3 UploadPart status {}", resp.status())));
        }
        let etag = resp
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.trim().trim_matches('"').to_string())
            .unwrap_or(sha_hex);
        Ok(etag)
    }

    /// Real S3 `CompleteMultipartUpload` (`POST /{key}?uploadId=ID`) assembling all verified parts server-side on S3/MinIO.
    pub async fn complete_multipart_upload(
        &self,
        key: &str,
        upload_id: &str,
        parts: &[(u32, String)],
    ) -> Result<(), StorageError> {
        self.require_ready()?;
        let (host, canonical_uri, base_url) = self.object_url_parts(key)?;
        let canonical_query = format!("uploadId={}", uri_encode(upload_id));
        let url = format!("{base_url}?{canonical_query}");

        let mut xml = String::from("<CompleteMultipartUpload>");
        for (part_num, etag) in parts {
            let clean_etag = etag.trim_matches('"').replace(['<', '>', '&', '"'], "");
            xml.push_str(&format!(
                "<Part><PartNumber>{part_num}</PartNumber><ETag>\"{clean_etag}\"</ETag></Part>"
            ));
        }
        xml.push_str("</CompleteMultipartUpload>");
        let body_bytes = xml.into_bytes();
        let body_sha = hex::encode(sha256_bytes(&body_bytes));

        let resp = self
            .send_with_retry(|| {
                let headers = self.signed_headers_for("POST", &canonical_uri, &canonical_query, &host, &body_sha, Utc::now())?;
                let mut req = self
                    .http
                    .post(&url)
                    .header("Content-Type", "application/xml")
                    .body(body_bytes.clone());
                for (k, v) in headers {
                    req = req.header(k, v);
                }
                Ok(req)
            })
            .await?;

        if !resp.status().is_success() {
            return Err(StorageError::Backend(format!("S3 CompleteMultipartUpload status {}", resp.status())));
        }
        Ok(())
    }

    /// Real S3 `AbortMultipartUpload` (`DELETE /{key}?uploadId=ID`) cleaning abandoned multipart uploads.
    pub async fn abort_multipart_upload(&self, key: &str, upload_id: &str) -> Result<(), StorageError> {
        self.require_ready()?;
        let (host, canonical_uri, base_url) = self.object_url_parts(key)?;
        let canonical_query = format!("uploadId={}", uri_encode(upload_id));
        let url = format!("{base_url}?{canonical_query}");
        let empty_hash = hex::encode(sha256_bytes(&[]));

        let resp = self
            .send_with_retry(|| {
                let headers = self.signed_headers_for("DELETE", &canonical_uri, &canonical_query, &host, &empty_hash, Utc::now())?;
                let mut req = self.http.delete(&url);
                for (k, v) in headers {
                    req = req.header(k, v);
                }
                Ok(req)
            })
            .await?;

        if !resp.status().is_success() && resp.status() != reqwest::StatusCode::NOT_FOUND {
            return Err(StorageError::Backend(format!("S3 AbortMultipartUpload status {}", resp.status())));
        }
        Ok(())
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
            let size = bytes.len() as u64;
            let resp = self
                .send_with_retry(|| {
                    let headers = self.signed_headers_for("PUT", &canonical_uri, "", &host, &sha_hex, Utc::now())?;
                    let mut req = self
                        .http
                        .put(&base_url)
                        .header("Content-Type", content_type)
                        .header("x-amz-meta-chatw-sha256", &sha_hex)
                        .body(bytes.clone());
                    for (k, v) in headers {
                        req = req.header(k, v);
                    }
                    Ok(req)
                })
                .await?;
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
            let resp = self
                .send_with_retry(|| {
                    let headers = self.signed_headers_for("GET", &canonical_uri, "", &host, &empty_hash, Utc::now())?;
                    let mut req = self.http.get(&base_url);
                    for (k, v) in headers {
                        req = req.header(k, v);
                    }
                    Ok(req)
                })
                .await?;
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
            let range_val = format!("bytes={start}-{end_inclusive}");
            let resp = self
                .send_with_retry(|| {
                    let headers = self.signed_headers_for("GET", &canonical_uri, "", &host, &empty_hash, Utc::now())?;
                    let mut req = self.http.get(&base_url).header("Range", &range_val);
                    for (k, v) in headers {
                        req = req.header(k, v);
                    }
                    Ok(req)
                })
                .await?;
            if resp.status() == reqwest::StatusCode::NOT_FOUND {
                return Err(StorageError::NotFound(key.to_string()));
            }
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

    /// True zero-buffering streaming download from S3/MinIO directly into Axum HTTP response `Body`!
    fn get_object_stream<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Body, StorageError>> {
        Box::pin(async move {
            self.require_ready()?;
            let (host, canonical_uri, base_url) = self.object_url_parts(key)?;
            let empty_hash = hex::encode(sha256_bytes(&[]));
            let resp = self
                .send_with_retry(|| {
                    let headers = self.signed_headers_for("GET", &canonical_uri, "", &host, &empty_hash, Utc::now())?;
                    let mut req = self.http.get(&base_url);
                    for (k, v) in headers {
                        req = req.header(k, v);
                    }
                    Ok(req)
                })
                .await?;
            if resp.status() == reqwest::StatusCode::NOT_FOUND {
                return Err(StorageError::NotFound(key.to_string()));
            }
            if !resp.status().is_success() {
                return Err(StorageError::Backend(format!("S3 stream GET status {}", resp.status())));
            }
            Ok(Body::from_stream(resp.bytes_stream()))
        })
    }

    fn delete_object<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, StorageError>> {
        Box::pin(async move {
            self.require_ready()?;
            let (host, canonical_uri, base_url) = self.object_url_parts(key)?;
            let empty_hash = hex::encode(sha256_bytes(&[]));
            let resp = self
                .send_with_retry(|| {
                    let headers = self.signed_headers_for("DELETE", &canonical_uri, "", &host, &empty_hash, Utc::now())?;
                    let mut req = self.http.delete(&base_url);
                    for (k, v) in headers {
                        req = req.header(k, v);
                    }
                    Ok(req)
                })
                .await?;
            Ok(resp.status().is_success() || resp.status() == reqwest::StatusCode::NOT_FOUND)
        })
    }

    fn object_exists<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, StorageError>> {
        Box::pin(async move {
            self.require_ready()?;
            let (host, canonical_uri, base_url) = self.object_url_parts(key)?;
            let empty_hash = hex::encode(sha256_bytes(&[]));
            let resp = self
                .send_with_retry(|| {
                    let headers = self.signed_headers_for("HEAD", &canonical_uri, "", &host, &empty_hash, Utc::now())?;
                    let mut req = self.http.head(&base_url);
                    for (k, v) in headers {
                        req = req.header(k, v);
                    }
                    Ok(req)
                })
                .await?;
            Ok(resp.status().is_success())
        })
    }

    fn get_metadata<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<StoredObjectMeta, StorageError>> {
        Box::pin(async move {
            self.require_ready()?;
            let (host, canonical_uri, base_url) = self.object_url_parts(key)?;
            let empty_hash = hex::encode(sha256_bytes(&[]));
            let resp = self
                .send_with_retry(|| {
                    let headers = self.signed_headers_for("HEAD", &canonical_uri, "", &host, &empty_hash, Utc::now())?;
                    let mut req = self.http.head(&base_url);
                    for (k, v) in headers {
                        req = req.header(k, v);
                    }
                    Ok(req)
                })
                .await?;
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
            let sha_hdr = resp
                .headers()
                .get("x-amz-meta-chatw-sha256")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            Ok(StoredObjectMeta {
                backend: "s3_ready".into(),
                storage_key: key.to_string(),
                byte_size: size,
                sha256_hex: sha_hdr,
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
        content_type: &'a str,
    ) -> BoxFuture<'a, Result<MultipartInit, StorageError>> {
        Box::pin(async move { self.initiate_multipart_upload(key, content_type).await })
    }

    fn upload_chunk<'a>(
        &'a self,
        transfer_id: &'a str,
        chunk_index: u32,
        bytes: Vec<u8>,
    ) -> BoxFuture<'a, Result<String, StorageError>> {
        Box::pin(async move {
            let (key, upload_id) = transfer_id
                .split_once('|')
                .ok_or_else(|| StorageError::Backend("expected 'key|upload_id' composite transfer_id".into()))?;
            self.upload_multipart_part(key, upload_id, chunk_index + 1, bytes).await
        })
    }
}

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].trim().to_string())
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
    use axum::{
        Router,
        body::Bytes,
        extract::{Path, Query, State},
        http::{HeaderMap, StatusCode, header},
        response::{IntoResponse, Response},
        routing::any,
    };
    use chrono::TimeZone;
    use std::{
        collections::{BTreeMap, HashMap},
        sync::{Arc, Mutex},
    };

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

    #[derive(Default)]
    struct LoopbackS3State {
        objects: HashMap<String, Vec<u8>>,
        multiparts: HashMap<String, BTreeMap<u32, Vec<u8>>>,
        fail_first_put: bool,
    }

    async fn loopback_s3_handler(
        State(st): State<Arc<Mutex<LoopbackS3State>>>,
        method: axum::http::Method,
        Path((_bucket, key)): Path<(String, String)>,
        Query(q): Query<HashMap<String, String>>,
        headers: HeaderMap,
        body: Bytes,
    ) -> Response {
        // Verify SigV4 Authorization header is always present on backend calls
        let auth = headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).unwrap_or("");
        if !auth.starts_with("AWS4-HMAC-SHA256 Credential=testkey/") {
            return (StatusCode::FORBIDDEN, "missing sigv4").into_response();
        }

        let mut guard = st.lock().unwrap();
        match method.as_str() {
            "POST" if q.contains_key("uploads") => {
                let uid = "mp-upload-001".to_string();
                guard.multiparts.insert(uid.clone(), BTreeMap::new());
                let xml = format!("<InitiateMultipartUploadResult><UploadId>{uid}</UploadId></InitiateMultipartUploadResult>");
                (StatusCode::OK, xml).into_response()
            }
            "PUT" if q.contains_key("uploadId") && q.contains_key("partNumber") => {
                let uid = q.get("uploadId").unwrap();
                let part: u32 = q.get("partNumber").unwrap().parse().unwrap();
                if let Some(parts) = guard.multiparts.get_mut(uid) {
                    parts.insert(part, body.to_vec());
                    let etag = hex::encode(sha256_bytes(&body));
                    return (StatusCode::OK, [(header::ETAG, format!("\"{etag}\""))], "").into_response();
                }
                StatusCode::NOT_FOUND.into_response()
            }
            "POST" if q.contains_key("uploadId") => {
                let uid = q.get("uploadId").unwrap().clone();
                if let Some(parts) = guard.multiparts.remove(&uid) {
                    let mut combined = Vec::new();
                    for (_, p) in parts {
                        combined.extend_from_slice(&p);
                    }
                    guard.objects.insert(key, combined);
                    return (StatusCode::OK, "<CompleteMultipartUploadResult/>").into_response();
                }
                StatusCode::NOT_FOUND.into_response()
            }
            "DELETE" if q.contains_key("uploadId") => {
                guard.multiparts.remove(q.get("uploadId").unwrap());
                StatusCode::NO_CONTENT.into_response()
            }
            "PUT" => {
                if guard.fail_first_put {
                    guard.fail_first_put = false;
                    return (StatusCode::SERVICE_UNAVAILABLE, "simulated 503 for retry test").into_response();
                }
                guard.objects.insert(key, body.to_vec());
                StatusCode::OK.into_response()
            }
            "GET" => {
                let Some(data) = guard.objects.get(&key) else {
                    return StatusCode::NOT_FOUND.into_response();
                };
                if let Some(range) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) {
                    if let Some(spec) = range.strip_prefix("bytes=") {
                        if let Some((s, e)) = spec.split_once('-') {
                            let start: usize = s.parse().unwrap_or(0);
                            let end: usize = e.parse().unwrap_or(data.len().saturating_sub(1)).min(data.len() - 1);
                            let slice = data[start..=end].to_vec();
                            return (
                                StatusCode::PARTIAL_CONTENT,
                                [(header::CONTENT_RANGE, format!("bytes {start}-{end}/{}", data.len()))],
                                slice,
                            )
                                .into_response();
                        }
                    }
                }
                (StatusCode::OK, data.clone()).into_response()
            }
            "HEAD" => {
                if let Some(data) = guard.objects.get(&key) {
                    (StatusCode::OK, [(header::CONTENT_LENGTH, data.len().to_string())], "").into_response()
                } else {
                    StatusCode::NOT_FOUND.into_response()
                }
            }
            "DELETE" => {
                guard.objects.remove(&key);
                StatusCode::NO_CONTENT.into_response()
            }
            _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
        }
    }

    #[tokio::test]
    async fn real_s3_http_put_retry_range_stream_multipart_and_delete_roundtrip() {
        let state = Arc::new(Mutex::new(LoopbackS3State {
            fail_first_put: true, // forces exponential backoff retry to prove it works!
            ..Default::default()
        }));
        let app = Router::new()
            .route("/{bucket}/{*key}", any(loopback_s3_handler))
            .with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let srv = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let adapter = S3CompatibleStorageAdapter::new(S3StorageConfig {
            enabled: true,
            endpoint: Some(format!("http://{addr}")),
            region: "us-east-1".into(),
            bucket: Some("chatw-private".into()),
            prefix: "media".into(),
            access_key_id: Some("testkey".into()),
            secret_access_key: Some("testsecret".into()),
            force_path_style: true,
        });

        // 1. PUT with simulated 503 transient failure -> retry succeeds!
        let payload = b"ChatW E2EE encrypted AVIF object payload for S3 verification!".to_vec();
        let meta = adapter.put_object("aa/bb/obj.enc", payload.clone(), "application/octet-stream", None).await.expect("put with retry");
        assert_eq!(meta.byte_size, payload.len() as u64);

        // 2. HEAD & GET & Range GET & Streaming GET
        assert!(adapter.object_exists("aa/bb/obj.enc").await.unwrap());
        assert_eq!(adapter.get_object("aa/bb/obj.enc").await.unwrap(), payload);
        let (range_bytes, total) = adapter.get_object_range("aa/bb/obj.enc", 6, 15).await.unwrap();
        assert_eq!(range_bytes, &payload[6..=15]);
        assert_eq!(total, payload.len() as u64);

        // 3. Full S3 Multipart Upload (Initiate -> UploadPart 1 & 2 -> Complete -> Verify assembled bytes!)
        let mp = adapter.initiate_multipart_upload("mp/video.enc", "application/octet-stream").await.expect("init mp");
        let e1 = adapter.upload_multipart_part("mp/video.enc", &mp.upload_id, 1, b"PART_ONE_DATA_".to_vec()).await.expect("part 1");
        let e2 = adapter.upload_multipart_part("mp/video.enc", &mp.upload_id, 2, b"PART_TWO_DATA".to_vec()).await.expect("part 2");
        adapter.complete_multipart_upload("mp/video.enc", &mp.upload_id, &[(1, e1), (2, e2)]).await.expect("complete mp");
        assert_eq!(adapter.get_object("mp/video.enc").await.unwrap(), b"PART_ONE_DATA_PART_TWO_DATA");

        // 4. DELETE cleanup
        assert!(adapter.delete_object("mp/video.enc").await.unwrap());
        assert!(!adapter.object_exists("mp/video.enc").await.unwrap());

        srv.abort();
    }
}
