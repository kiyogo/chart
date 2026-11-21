use std::net::IpAddr;

use futures_util::future::BoxFuture;
use reqwest::Method;
use serde_json::{Value, json};

use crate::storage::{StorageAdapter, s3::{S3CompatibleStorageAdapter, S3StorageConfig}};

use super::{CloudBackupItem, CloudError, CloudQuotaInfo, CloudStorageProvider, ProviderCapabilities};

pub fn all_provider_catalog() -> Vec<ProviderCapabilities> {
    let gdrive_ok = std::env::var("GOOGLE_DRIVE_CLIENT_ID")
        .ok()
        .is_some_and(|v| !v.is_empty() && !v.starts_with("dev-") && !v.starts_with("your-"));
    let onedrive_ok = std::env::var("ONEDRIVE_CLIENT_ID")
        .ok()
        .is_some_and(|v| !v.is_empty() && !v.starts_with("your-"));
    let dropbox_ok = std::env::var("DROPBOX_APP_KEY")
        .ok()
        .is_some_and(|v| !v.is_empty() && !v.starts_with("your-"));

    vec![
        ProviderCapabilities {
            provider_id: "google_drive",
            display_name: "Google Drive",
            auth_kind: "oauth2_pkce",
            oauth_configured: gdrive_ok,
            supports_resumable_upload: true,
            supports_resumable_download: true,
            supports_incremental_chunks: true,
            supports_quota: true,
            supports_folder_isolation: true,
            requires_https: true,
            max_file_bytes: 5 * 1024 * 1024 * 1024 * 1024,
            setup_requirements: vec![
                "GOOGLE_DRIVE_CLIENT_ID (OAuth Client ID trên Google Cloud Console)",
                "Google Drive API enabled + OAuth Consent Screen + Privacy/ToS URL",
                "Phạm vi tối thiểu: drive.file / drive.appdata",
            ],
        },
        ProviderCapabilities {
            provider_id: "onedrive",
            display_name: "Microsoft OneDrive",
            auth_kind: "oauth2_pkce",
            oauth_configured: onedrive_ok,
            supports_resumable_upload: true,
            supports_resumable_download: true,
            supports_incremental_chunks: true,
            supports_quota: true,
            supports_folder_isolation: true,
            requires_https: true,
            max_file_bytes: 250 * 1024 * 1024 * 1024,
            setup_requirements: vec![
                "ONEDRIVE_CLIENT_ID (Microsoft Entra App Registration)",
                "Microsoft Graph scope tối thiểu: Files.ReadWrite.AppFolder offline_access",
            ],
        },
        ProviderCapabilities {
            provider_id: "dropbox",
            display_name: "Dropbox",
            auth_kind: "oauth2_pkce",
            oauth_configured: dropbox_ok,
            supports_resumable_upload: true,
            supports_resumable_download: true,
            supports_incremental_chunks: true,
            supports_quota: true,
            supports_folder_isolation: true,
            requires_https: true,
            max_file_bytes: 350 * 1024 * 1024 * 1024,
            setup_requirements: vec![
                "DROPBOX_APP_KEY (Dropbox Developer App - App Folder)",
                "Trạng thái Production approval nếu phát hành cho nhiều người dùng ngoài nhóm test",
            ],
        },
        ProviderCapabilities {
            provider_id: "nextcloud",
            display_name: "Nextcloud",
            auth_kind: "credentials",
            oauth_configured: true,
            supports_resumable_upload: false,
            supports_resumable_download: true,
            supports_incremental_chunks: true,
            supports_quota: true,
            supports_folder_isolation: true,
            requires_https: true,
            max_file_bytes: 50 * 1024 * 1024 * 1024,
            setup_requirements: vec!["URL HTTPS Nextcloud + App Password (khuyến nghị thay vì mật khẩu chính)"],
        },
        ProviderCapabilities {
            provider_id: "webdav",
            display_name: "WebDAV",
            auth_kind: "credentials",
            oauth_configured: true,
            supports_resumable_upload: false,
            supports_resumable_download: true,
            supports_incremental_chunks: true,
            supports_quota: true,
            supports_folder_isolation: true,
            requires_https: true,
            max_file_bytes: 50 * 1024 * 1024 * 1024,
            setup_requirements: vec!["URL HTTPS WebDAV + tài khoản có quyền đọc/ghi/tạo thư mục"],
        },
        ProviderCapabilities {
            provider_id: "s3",
            display_name: "Amazon S3",
            auth_kind: "credentials",
            oauth_configured: true,
            supports_resumable_upload: true,
            supports_resumable_download: true,
            supports_incremental_chunks: true,
            supports_quota: false,
            supports_folder_isolation: true,
            requires_https: true,
            max_file_bytes: 5 * 1024 * 1024 * 1024 * 1024,
            setup_requirements: vec!["Endpoint HTTPS, Region, Bucket, Access Key, Secret Key"],
        },
        ProviderCapabilities {
            provider_id: "r2",
            display_name: "Cloudflare R2",
            auth_kind: "credentials",
            oauth_configured: true,
            supports_resumable_upload: true,
            supports_resumable_download: true,
            supports_incremental_chunks: true,
            supports_quota: false,
            supports_folder_isolation: true,
            requires_https: true,
            max_file_bytes: 5 * 1024 * 1024 * 1024 * 1024,
            setup_requirements: vec!["R2 S3 Endpoint HTTPS, Bucket, Access Key, Secret Key"],
        },
        ProviderCapabilities {
            provider_id: "minio",
            display_name: "MinIO / S3 tương thích",
            auth_kind: "credentials",
            oauth_configured: true,
            supports_resumable_upload: true,
            supports_resumable_download: true,
            supports_incremental_chunks: true,
            supports_quota: false,
            supports_folder_isolation: true,
            requires_https: true,
            max_file_bytes: 5 * 1024 * 1024 * 1024 * 1024,
            setup_requirements: vec!["Endpoint S3/MinIO, Bucket, Access Key, Secret Key"],
        },
        ProviderCapabilities {
            provider_id: "nas",
            display_name: "NAS cá nhân",
            auth_kind: "credentials",
            oauth_configured: true,
            supports_resumable_upload: false,
            supports_resumable_download: true,
            supports_incremental_chunks: true,
            supports_quota: true,
            supports_folder_isolation: true,
            requires_https: true,
            max_file_bytes: 50 * 1024 * 1024 * 1024,
            setup_requirements: vec!["Địa chỉ WebDAV/HTTPS của NAS và tài khoản App Password"],
        },
        ProviderCapabilities {
            provider_id: "icloud",
            display_name: "Apple iCloud",
            auth_kind: "icloud_platform",
            oauth_configured: false,
            supports_resumable_upload: false,
            supports_resumable_download: false,
            supports_incremental_chunks: true,
            supports_quota: false,
            supports_folder_isolation: true,
            requires_https: true,
            max_file_bytes: 50 * 1024 * 1024 * 1024,
            setup_requirements: vec![
                "Chỉ khả dụng qua Apple CloudKit / Ubiquity Container trên ứng dụng macOS & iOS đã ký Apple Developer",
            ],
        },
    ]
}

fn is_forbidden_ssrf_ip(ip: IpAddr, allow_lan_rfc1918: bool, allow_loopback_test: bool) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            if v4.is_unspecified() || v4.is_link_local() || v4.is_multicast() || v4.is_broadcast() {
                return true; // always block 169.254.169.254 metadata, 0.0.0.0, etc.
            }
            if v4.is_loopback() {
                return !allow_loopback_test;
            }
            if v4.is_private() {
                return !(allow_lan_rfc1918 || allow_loopback_test);
            }
            false
        }
        IpAddr::V6(v6) => {
            if v6.is_unspecified() || v6.is_multicast() || v6.is_unicast_link_local() {
                return true;
            }
            if v6.is_loopback() {
                return !allow_loopback_test;
            }
            // Unique local fc00::/7
            if (v6.segments()[0] & 0xfe00) == 0xfc00 {
                return !(allow_lan_rfc1918 || allow_loopback_test);
            }
            false
        }
    }
}

/// Validates external cloud URLs against insecure plaintext HTTP and SSRF internal targets.
pub fn validate_remote_endpoint(raw_url: &str, allow_lan_nas: bool) -> Result<reqwest::Url, CloudError> {
    let url = reqwest::Url::parse(raw_url.trim())
        .map_err(|e| CloudError::InvalidConfig(format!("URL không hợp lệ: {e}")))?;

    let allow_loopback_test = std::env::var("ALLOW_LOOPBACK_CLOUD_TEST")
        .ok()
        .is_some_and(|v| v == "1" || v == "true");

    let scheme = url.scheme();
    let host = url
        .host_str()
        .ok_or_else(|| CloudError::InvalidConfig("Thiếu host trong URL".into()))?;
    let host_lower = host.to_ascii_lowercase();

    let is_lan_http_allowed = if allow_lan_nas && scheme == "http" {
        if host_lower.ends_with(".local") {
            true
        } else if let Ok(ip) = host.parse::<IpAddr>() {
            match ip {
                IpAddr::V4(v4) => v4.is_private(),
                IpAddr::V6(v6) => (v6.segments()[0] & 0xfe00) == 0xfc00,
            }
        } else {
            false
        }
    } else {
        false
    };

    if scheme != "https" {
        if scheme == "http" && allow_loopback_test && (host_lower == "127.0.0.1" || host_lower == "localhost") {
            // Permitted strictly in controlled integration tests against local loopback test server.
        } else if is_lan_http_allowed {
            // Permitted explicitly for local LAN NAS endpoints configured by user (`allow_lan = true`).
        } else {
            return Err(CloudError::TlsRequired);
        }
    }

    if host_lower == "metadata.google.internal" || host_lower.ends_with(".internal") || host_lower.ends_with(".local") {
        if !(allow_lan_nas && host_lower.ends_with(".local")) {
            return Err(CloudError::SsrfBlocked(host.to_string()));
        }
    }
    if (host_lower == "localhost" || host_lower == "127.0.0.1" || host_lower == "::1") && !allow_loopback_test {
        return Err(CloudError::SsrfBlocked(host.to_string()));
    }
    let host_unbracketed = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = host_unbracketed.parse::<IpAddr>() {
        if is_forbidden_ssrf_ip(ip, allow_lan_nas, allow_loopback_test) {
            return Err(CloudError::SsrfBlocked(host.to_string()));
        }
    }
    Ok(url)
}

/// Builds a redirect-disabled (`redirect::Policy::none()`) HTTP client and resolves DNS upfront
/// to prevent 302 redirect SSRF and DNS rebinding against private/metadata IP ranges.
async fn build_safe_cloud_http_client(
    url: &reqwest::Url,
    allow_lan_nas: bool,
) -> Result<reqwest::Client, CloudError> {
    let allow_loopback_test = std::env::var("ALLOW_LOOPBACK_CLOUD_TEST")
        .ok()
        .is_some_and(|v| v == "1" || v == "true");
    let host = url
        .host_str()
        .ok_or_else(|| CloudError::InvalidConfig("Thiếu host trong URL".into()))?
        .trim_start_matches('[')
        .trim_end_matches(']');
    let port = url
        .port_or_known_default()
        .ok_or_else(|| CloudError::InvalidConfig("Cổng URL không hợp lệ".into()))?;

    let mut builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(15));

    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_forbidden_ssrf_ip(ip, allow_lan_nas, allow_loopback_test) {
            return Err(CloudError::SsrfBlocked(host.to_string()));
        }
    } else if let Ok(addrs) = tokio::net::lookup_host((host, port)).await {
        let resolved: Vec<std::net::SocketAddr> = addrs.collect();
        for addr in &resolved {
            if is_forbidden_ssrf_ip(addr.ip(), allow_lan_nas, allow_loopback_test) {
                return Err(CloudError::SsrfBlocked(format!("{} -> {}", host, addr.ip())));
            }
        }
        if let Some(first) = resolved.first() {
            builder = builder.resolve(host, *first);
        }
    }

    builder
        .build()
        .map_err(|e| CloudError::Network(e.to_string()))
}

fn sanitize_remote_key(key: &str) -> Result<String, CloudError> {
    let clean = key.trim().trim_start_matches('/');
    if clean.is_empty() || clean.contains("..") || clean.contains('\\') || clean.contains('\0') {
        return Err(CloudError::InvalidConfig("remote_key không hợp lệ".into()));
    }
    Ok(clean.to_string())
}

// ---------------------------------------------------------------- WebDAV / Nextcloud / NAS
pub struct WebDavProvider {
    pub kind: &'static str,
    pub base_url: reqwest::Url,
    pub username: String,
    pub password: String,
    pub folder: String,
    pub allow_lan: bool,
}

impl WebDavProvider {
    pub fn from_config(kind: &'static str, cfg: &Value) -> Result<Self, CloudError> {
        let raw_url = cfg["endpoint"].as_str().ok_or_else(|| CloudError::InvalidConfig("Thiếu endpoint WebDAV".into()))?;
        let allow_lan = kind == "nas" && cfg["allow_lan"].as_bool().unwrap_or(false);
        let parsed = validate_remote_endpoint(raw_url, allow_lan)?;
        let username = cfg["username"].as_str().unwrap_or("").trim().to_string();
        let password = cfg["password"].as_str().unwrap_or("").to_string();
        let folder = cfg["folder"]
            .as_str()
            .unwrap_or("KIYO_Backups")
            .trim_matches('/')
            .to_string();
        Ok(Self {
            kind,
            base_url: parsed,
            username,
            password,
            folder: if folder.is_empty() { "KIYO_Backups".into() } else { folder },
            allow_lan,
        })
    }

    async fn safe_client(&self) -> Result<reqwest::Client, CloudError> {
        build_safe_cloud_http_client(&self.base_url, self.allow_lan).await
    }

    fn url_for(&self, rel: &str) -> Result<String, CloudError> {
        let mut base = self.base_url.to_string();
        if !base.ends_with('/') {
            base.push('/');
        }
        let clean_rel = rel.trim_start_matches('/');
        if clean_rel.is_empty() {
            Ok(format!("{base}{}/", self.folder))
        } else {
            Ok(format!("{base}{}/{}", self.folder, clean_rel))
        }
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if !self.username.is_empty() {
            req.basic_auth(&self.username, Some(&self.password))
        } else if !self.password.is_empty() {
            req.bearer_auth(&self.password)
        } else {
            req
        }
    }

    async fn ensure_dir(&self, http: &reqwest::Client, dir_rel: &str) -> Result<(), CloudError> {
        let parts: Vec<&str> = dir_rel.trim_matches('/').split('/').filter(|s| !s.is_empty()).collect();
        let mkcol = Method::from_bytes(b"MKCOL").map_err(|_| CloudError::Unsupported("MKCOL"))?;
        let root_url = self.url_for("")?;
        let r0 = self
            .apply_auth(http.request(mkcol.clone(), &root_url))
            .send()
            .await
            .map_err(|e| CloudError::Network(e.to_string()))?;
        if r0.status().as_u16() == 401 || r0.status().as_u16() == 403 {
            return Err(CloudError::AuthRequired);
        }
        let mut acc = String::new();
        for p in parts {
            if !acc.is_empty() {
                acc.push('/');
            }
            acc.push_str(p);
            let url = format!("{}{}/", self.url_for("")?, acc);
            let _ = self.apply_auth(http.request(mkcol.clone(), &url)).send().await;
        }
        Ok(())
    }
}

fn extract_xml_tag_u64(xml: &str, tag_suffix: &str) -> Option<u64> {
    let close_pat = format!(":{tag_suffix}>");
    let open_end = xml.find(&close_pat).or_else(|| xml.find(&format!("<{tag_suffix}>")))?;
    let after = &xml[open_end..];
    let gt = after.find('>')?;
    let rest = &after[gt + 1..];
    let lt = rest.find('<')?;
    rest[..lt].trim().parse::<u64>().ok()
}

fn extract_hrefs_from_propfind(xml: &str, folder: &str) -> Vec<CloudBackupItem> {
    let mut items = Vec::new();
    let mut pos = 0;
    while let Some(start) = xml[pos..].find("href>") {
        let after = &xml[pos + start + 5..];
        let Some(end) = after.find('<') else { break };
        let href = after[..end].trim();
        pos = pos + start + 5 + end;
        if href.ends_with('/') {
            continue;
        }
        if let Some(idx) = href.find(folder) {
            let rel = href[idx + folder.len()..].trim_start_matches('/').to_string();
            if !rel.is_empty() {
                let name = rel.rsplit('/').next().unwrap_or(&rel).to_string();
                items.push(CloudBackupItem {
                    remote_key: rel,
                    name,
                    byte_size: 0,
                    etag_or_sha256: None,
                    modified_at: None,
                });
            }
        }
    }
    items
}

impl CloudStorageProvider for WebDavProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        all_provider_catalog()
            .into_iter()
            .find(|c| c.provider_id == self.kind)
            .unwrap_or_else(|| all_provider_catalog().into_iter().find(|c| c.provider_id == "webdav").unwrap())
    }

    fn verify_connection<'a>(
        &'a self,
        _http: &'a reqwest::Client,
    ) -> BoxFuture<'a, Result<(Option<String>, CloudQuotaInfo), CloudError>> {
        Box::pin(async move {
            let safe_http = self.safe_client().await?;
            let propfind = Method::from_bytes(b"PROPFIND").map_err(|_| CloudError::Unsupported("PROPFIND"))?;
            let url = self.base_url.to_string();
            let resp = self
                .apply_auth(safe_http.request(propfind, &url).header("Depth", "0"))
                .send()
                .await
                .map_err(|e| CloudError::Network(e.to_string()))?;
            let status = resp.status().as_u16();
            if (300..400).contains(&status) {
                return Err(CloudError::SsrfBlocked("WebDAV redirect is forbidden".into()));
            }
            if status == 401 || status == 403 {
                return Err(CloudError::AuthRequired);
            }
            if status >= 400 {
                return Err(CloudError::Remote(format!("WebDAV HTTP {status}")));
            }
            let body = resp.text().await.unwrap_or_default();
            let free = extract_xml_tag_u64(&body, "quota-available-bytes");
            let used = extract_xml_tag_u64(&body, "quota-used-bytes");
            let total = match (used, free) {
                (Some(u), Some(f)) => Some(u.saturating_add(f)),
                _ => None,
            };
            self.ensure_dir(&safe_http, "").await?;
            Ok((
                if self.username.is_empty() { None } else { Some(self.username.clone()) },
                CloudQuotaInfo {
                    total_bytes: total,
                    used_bytes: used,
                    free_bytes: free,
                },
            ))
        })
    }

    fn list_backups<'a>(
        &'a self,
        _http: &'a reqwest::Client,
        _prefix: &'a str,
    ) -> BoxFuture<'a, Result<Vec<CloudBackupItem>, CloudError>> {
        Box::pin(async move {
            let safe_http = self.safe_client().await?;
            let propfind = Method::from_bytes(b"PROPFIND").map_err(|_| CloudError::Unsupported("PROPFIND"))?;
            let url = self.url_for("")?;
            let resp = self
                .apply_auth(safe_http.request(propfind, &url).header("Depth", "infinity"))
                .send()
                .await
                .map_err(|e| CloudError::Network(e.to_string()))?;
            let st = resp.status().as_u16();
            if (300..400).contains(&st) {
                return Err(CloudError::SsrfBlocked("WebDAV redirect is forbidden".into()));
            }
            if st == 401 || st == 403 {
                return Err(CloudError::AuthRequired);
            }
            if st == 404 {
                return Ok(Vec::new());
            }
            let xml = resp.text().await.unwrap_or_default();
            Ok(extract_hrefs_from_propfind(&xml, &self.folder))
        })
    }

    fn upload_backup<'a>(
        &'a self,
        _http: &'a reqwest::Client,
        remote_key: &'a str,
        bytes: Vec<u8>,
    ) -> BoxFuture<'a, Result<CloudBackupItem, CloudError>> {
        Box::pin(async move {
            let safe_http = self.safe_client().await?;
            let clean = sanitize_remote_key(remote_key)?;
            if let Some((parent, _)) = clean.rsplit_once('/') {
                self.ensure_dir(&safe_http, parent).await?;
            } else {
                self.ensure_dir(&safe_http, "").await?;
            }
            let sha = hex::encode(crate::storage::sha256_bytes(&bytes));
            let len = bytes.len() as u64;
            let url = self.url_for(&clean)?;
            let resp = self
                .apply_auth(safe_http.put(&url).body(bytes))
                .send()
                .await
                .map_err(|e| CloudError::Network(e.to_string()))?;
            let st = resp.status().as_u16();
            if (300..400).contains(&st) {
                return Err(CloudError::SsrfBlocked("WebDAV redirect is forbidden".into()));
            }
            if st == 401 || st == 403 {
                return Err(CloudError::AuthRequired);
            }
            if st == 507 {
                return Err(CloudError::QuotaExceeded);
            }
            if !resp.status().is_success() {
                return Err(CloudError::Remote(format!("WebDAV PUT HTTP {st}")));
            }
            Ok(CloudBackupItem {
                remote_key: clean.clone(),
                name: clean.rsplit('/').next().unwrap_or(&clean).to_string(),
                byte_size: len,
                etag_or_sha256: Some(sha),
                modified_at: Some(chrono::Utc::now().to_rfc3339()),
            })
        })
    }

    fn download_backup<'a>(
        &'a self,
        _http: &'a reqwest::Client,
        remote_key: &'a str,
    ) -> BoxFuture<'a, Result<Vec<u8>, CloudError>> {
        Box::pin(async move {
            let safe_http = self.safe_client().await?;
            let clean = sanitize_remote_key(remote_key)?;
            let url = self.url_for(&clean)?;
            let resp = self
                .apply_auth(safe_http.get(&url))
                .send()
                .await
                .map_err(|e| CloudError::Network(e.to_string()))?;
            let st = resp.status().as_u16();
            if (300..400).contains(&st) {
                return Err(CloudError::SsrfBlocked("WebDAV redirect is forbidden".into()));
            }
            if st == 401 || st == 403 {
                return Err(CloudError::AuthRequired);
            }
            if !resp.status().is_success() {
                return Err(CloudError::Remote(format!("WebDAV GET HTTP {st}")));
            }
            resp.bytes().await.map(|b| b.to_vec()).map_err(|e| CloudError::Network(e.to_string()))
        })
    }

    fn delete_backup<'a>(
        &'a self,
        _http: &'a reqwest::Client,
        remote_key: &'a str,
    ) -> BoxFuture<'a, Result<bool, CloudError>> {
        Box::pin(async move {
            let safe_http = self.safe_client().await?;
            let clean = sanitize_remote_key(remote_key)?;
            let url = self.url_for(&clean)?;
            let resp = self
                .apply_auth(safe_http.delete(&url))
                .send()
                .await
                .map_err(|e| CloudError::Network(e.to_string()))?;
            if resp.status().as_u16() == 404 {
                return Ok(false);
            }
            Ok(resp.status().is_success())
        })
    }
}

// ---------------------------------------------------------------- S3 / R2 / MinIO user cloud adapter
pub struct S3UserCloudProvider {
    pub kind: &'static str,
    pub adapter: S3CompatibleStorageAdapter,
}

impl S3UserCloudProvider {
    pub fn from_config(kind: &'static str, cfg: &Value) -> Result<Self, CloudError> {
        let raw_endpoint = cfg["endpoint"].as_str().ok_or_else(|| CloudError::InvalidConfig("Thiếu S3 endpoint".into()))?;
        let allow_lan = cfg["allow_lan"].as_bool().unwrap_or(false);
        let parsed = validate_remote_endpoint(raw_endpoint, allow_lan)?;
        let bucket = cfg["bucket"].as_str().map(str::trim).filter(|s| !s.is_empty()).ok_or_else(|| CloudError::InvalidConfig("Thiếu S3 bucket".into()))?;
        let access_key = cfg["access_key"].as_str().filter(|s| !s.is_empty()).ok_or_else(|| CloudError::InvalidConfig("Thiếu Access Key".into()))?;
        let secret_key = cfg["secret_key"].as_str().filter(|s| !s.is_empty()).ok_or_else(|| CloudError::InvalidConfig("Thiếu Secret Key".into()))?;
        let region = cfg["region"].as_str().filter(|s| !s.is_empty()).unwrap_or("us-east-1");
        let prefix = cfg["prefix"].as_str().unwrap_or("kiyo-backups").trim_matches('/');

        let s3_cfg = S3StorageConfig {
            enabled: true,
            endpoint: Some(parsed.as_str().trim_end_matches('/').to_string()),
            region: region.to_string(),
            bucket: Some(bucket.to_string()),
            prefix: prefix.to_string(),
            access_key_id: Some(access_key.to_string()),
            secret_access_key: Some(secret_key.to_string()),
            force_path_style: cfg["force_path_style"].as_bool().unwrap_or(true),
        };
        Ok(Self {
            kind,
            adapter: S3CompatibleStorageAdapter::new(s3_cfg),
        })
    }
}

impl CloudStorageProvider for S3UserCloudProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        all_provider_catalog()
            .into_iter()
            .find(|c| c.provider_id == self.kind)
            .unwrap_or_else(|| all_provider_catalog().into_iter().find(|c| c.provider_id == "s3").unwrap())
    }

    fn verify_connection<'a>(
        &'a self,
        _http: &'a reqwest::Client,
    ) -> BoxFuture<'a, Result<(Option<String>, CloudQuotaInfo), CloudError>> {
        Box::pin(async move {
            let probe_key = "_kiyo_probe.txt";
            self.adapter
                .put_object(probe_key, b"kiyo-probe".to_vec(), "text/plain", None)
                .await
                .map_err(|e| CloudError::Remote(e.to_string()))?;
            let _ = self.adapter.delete_object(probe_key).await;
            Ok((self.adapter.config().bucket.clone(), CloudQuotaInfo::default()))
        })
    }

    fn list_backups<'a>(
        &'a self,
        _http: &'a reqwest::Client,
        _prefix: &'a str,
    ) -> BoxFuture<'a, Result<Vec<CloudBackupItem>, CloudError>> {
        Box::pin(async move { Ok(Vec::new()) })
    }

    fn upload_backup<'a>(
        &'a self,
        _http: &'a reqwest::Client,
        remote_key: &'a str,
        bytes: Vec<u8>,
    ) -> BoxFuture<'a, Result<CloudBackupItem, CloudError>> {
        Box::pin(async move {
            let clean = sanitize_remote_key(remote_key)?;
            let meta = self
                .adapter
                .put_object(&clean, bytes, "application/octet-stream", None)
                .await
                .map_err(|e| CloudError::Remote(e.to_string()))?;
            Ok(CloudBackupItem {
                remote_key: clean.clone(),
                name: clean.rsplit('/').next().unwrap_or(&clean).to_string(),
                byte_size: meta.byte_size,
                etag_or_sha256: Some(meta.sha256_hex),
                modified_at: Some(chrono::Utc::now().to_rfc3339()),
            })
        })
    }

    fn download_backup<'a>(
        &'a self,
        _http: &'a reqwest::Client,
        remote_key: &'a str,
    ) -> BoxFuture<'a, Result<Vec<u8>, CloudError>> {
        Box::pin(async move {
            let clean = sanitize_remote_key(remote_key)?;
            self.adapter
                .get_object(&clean)
                .await
                .map_err(|e| CloudError::Remote(e.to_string()))
        })
    }

    fn delete_backup<'a>(
        &'a self,
        _http: &'a reqwest::Client,
        remote_key: &'a str,
    ) -> BoxFuture<'a, Result<bool, CloudError>> {
        Box::pin(async move {
            let clean = sanitize_remote_key(remote_key)?;
            self.adapter
                .delete_object(&clean)
                .await
                .map_err(|e| CloudError::Remote(e.to_string()))
        })
    }
}

// ---------------------------------------------------------------- OAuth Cloud Providers (Google Drive, OneDrive, Dropbox)
pub struct OAuthCloudProvider {
    pub kind: &'static str,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub use_app_data: bool,
}

impl OAuthCloudProvider {
    pub fn from_config(kind: &'static str, cfg: &Value) -> Result<Self, CloudError> {
        let access_token = cfg["access_token"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or(CloudError::AuthRequired)?
            .to_string();
        let refresh_token = cfg["refresh_token"].as_str().map(str::to_string);
        let use_app_data = cfg["use_app_data"].as_bool().unwrap_or(false);
        Ok(Self {
            kind,
            access_token,
            refresh_token,
            use_app_data,
        })
    }
}

impl CloudStorageProvider for OAuthCloudProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        all_provider_catalog()
            .into_iter()
            .find(|c| c.provider_id == self.kind)
            .unwrap()
    }

    fn verify_connection<'a>(
        &'a self,
        http: &'a reqwest::Client,
    ) -> BoxFuture<'a, Result<(Option<String>, CloudQuotaInfo), CloudError>> {
        Box::pin(async move {
            match self.kind {
                "google_drive" => {
                    let resp = http
                        .get("https://www.googleapis.com/drive/v3/about?fields=user(emailAddress),storageQuota(limit,usage)")
                        .bearer_auth(&self.access_token)
                        .send()
                        .await
                        .map_err(|e| CloudError::Network(e.to_string()))?;
                    if resp.status().as_u16() == 401 {
                        return Err(CloudError::TokenExpired);
                    }
                    if resp.status().as_u16() == 403 {
                        return Err(CloudError::PermissionRevoked);
                    }
                    let v: Value = resp.json().await.map_err(|e| CloudError::Remote(e.to_string()))?;
                    let email = v["user"]["emailAddress"].as_str().map(str::to_string);
                    let total = v["storageQuota"]["limit"].as_str().and_then(|s| s.parse::<u64>().ok());
                    let used = v["storageQuota"]["usage"].as_str().and_then(|s| s.parse::<u64>().ok());
                    let free = total.and_then(|t| used.map(|u| t.saturating_sub(u)));
                    Ok((email, CloudQuotaInfo { total_bytes: total, used_bytes: used, free_bytes: free }))
                }
                "onedrive" => {
                    let resp = http
                        .get("https://graph.microsoft.com/v1.0/me/drive?$select=owner,quota")
                        .bearer_auth(&self.access_token)
                        .send()
                        .await
                        .map_err(|e| CloudError::Network(e.to_string()))?;
                    if resp.status().as_u16() == 401 {
                        return Err(CloudError::TokenExpired);
                    }
                    let v: Value = resp.json().await.map_err(|e| CloudError::Remote(e.to_string()))?;
                    let email = v["owner"]["user"]["email"].as_str().or(v["owner"]["user"]["displayName"].as_str()).map(str::to_string);
                    let total = v["quota"]["total"].as_u64();
                    let used = v["quota"]["used"].as_u64();
                    let free = v["quota"]["remaining"].as_u64();
                    Ok((email, CloudQuotaInfo { total_bytes: total, used_bytes: used, free_bytes: free }))
                }
                "dropbox" => {
                    let resp = http
                        .post("https://api.dropboxapi.com/2/users/get_space_usage")
                        .bearer_auth(&self.access_token)
                        .send()
                        .await
                        .map_err(|e| CloudError::Network(e.to_string()))?;
                    if resp.status().as_u16() == 401 {
                        return Err(CloudError::TokenExpired);
                    }
                    let v: Value = resp.json().await.map_err(|e| CloudError::Remote(e.to_string()))?;
                    let used = v["used"].as_u64();
                    let total = v["allocation"]["allocated"].as_u64();
                    let free = total.and_then(|t| used.map(|u| t.saturating_sub(u)));
                    Ok((None, CloudQuotaInfo { total_bytes: total, used_bytes: used, free_bytes: free }))
                }
                _ => Err(CloudError::Unsupported("verify_connection")),
            }
        })
    }

    fn refresh_authorization<'a>(
        &'a self,
        http: &'a reqwest::Client,
    ) -> BoxFuture<'a, Result<Option<Value>, CloudError>> {
        Box::pin(async move {
            let Some(ref rt) = self.refresh_token else {
                return Err(CloudError::AuthRequired);
            };
            match self.kind {
                "google_drive" => {
                    let cid = std::env::var("GOOGLE_DRIVE_CLIENT_ID").map_err(|_| CloudError::NotConfiguredByAdmin("GOOGLE_DRIVE_CLIENT_ID"))?;
                    let csec = std::env::var("GOOGLE_DRIVE_CLIENT_SECRET").unwrap_or_default();
                    let mut params = vec![
                        ("client_id", cid.as_str()),
                        ("refresh_token", rt.as_str()),
                        ("grant_type", "refresh_token"),
                    ];
                    if !csec.is_empty() {
                        params.push(("client_secret", csec.as_str()));
                    }
                    let resp = http.post("https://oauth2.googleapis.com/token").form(&params).send().await.map_err(|e| CloudError::Network(e.to_string()))?;
                    if !resp.status().is_success() {
                        return Err(CloudError::PermissionRevoked);
                    }
                    let v: Value = resp.json().await.map_err(|e| CloudError::Remote(e.to_string()))?;
                    Ok(Some(v))
                }
                "onedrive" => {
                    let cid = std::env::var("ONEDRIVE_CLIENT_ID").map_err(|_| CloudError::NotConfiguredByAdmin("ONEDRIVE_CLIENT_ID"))?;
                    let resp = http.post("https://login.microsoftonline.com/common/oauth2/v2.0/token").form(&[
                        ("client_id", cid.as_str()),
                        ("refresh_token", rt.as_str()),
                        ("grant_type", "refresh_token"),
                    ]).send().await.map_err(|e| CloudError::Network(e.to_string()))?;
                    if !resp.status().is_success() {
                        return Err(CloudError::PermissionRevoked);
                    }
                    let v: Value = resp.json().await.map_err(|e| CloudError::Remote(e.to_string()))?;
                    Ok(Some(v))
                }
                "dropbox" => {
                    let cid = std::env::var("DROPBOX_APP_KEY").map_err(|_| CloudError::NotConfiguredByAdmin("DROPBOX_APP_KEY"))?;
                    let resp = http.post("https://api.dropboxapi.com/oauth2/token").form(&[
                        ("client_id", cid.as_str()),
                        ("refresh_token", rt.as_str()),
                        ("grant_type", "refresh_token"),
                    ]).send().await.map_err(|e| CloudError::Network(e.to_string()))?;
                    if !resp.status().is_success() {
                        return Err(CloudError::PermissionRevoked);
                    }
                    let v: Value = resp.json().await.map_err(|e| CloudError::Remote(e.to_string()))?;
                    Ok(Some(v))
                }
                _ => Ok(None),
            }
        })
    }

    fn revoke_authorization<'a>(
        &'a self,
        http: &'a reqwest::Client,
    ) -> BoxFuture<'a, Result<(), CloudError>> {
        Box::pin(async move {
            match self.kind {
                "google_drive" => {
                    let _ = http
                        .post("https://oauth2.googleapis.com/revoke")
                        .form(&[("token", self.access_token.as_str())])
                        .send()
                        .await;
                }
                "dropbox" => {
                    let _ = http
                        .post("https://api.dropboxapi.com/2/auth/token/revoke")
                        .bearer_auth(&self.access_token)
                        .send()
                        .await;
                }
                _ => {}
            }
            Ok(())
        })
    }

    fn list_backups<'a>(
        &'a self,
        http: &'a reqwest::Client,
        prefix: &'a str,
    ) -> BoxFuture<'a, Result<Vec<CloudBackupItem>, CloudError>> {
        Box::pin(async move {
            match self.kind {
                "google_drive" => {
                    let spaces = if self.use_app_data { "appDataFolder" } else { "drive" };
                    let q = format!("name contains '{}' and trashed = false", prefix.replace('\'', ""));
                    let list_url = format!(
                        "https://www.googleapis.com/drive/v3/files?spaces={}&q={}&fields={}",
                        urlencoding::encode(spaces),
                        urlencoding::encode(&q),
                        urlencoding::encode("files(id,name,size,sha256Checksum,modifiedTime)"),
                    );
                    let resp = http
                        .get(&list_url)
                        .bearer_auth(&self.access_token)
                        .send()
                        .await
                        .map_err(|e| CloudError::Network(e.to_string()))?;
                    if resp.status().as_u16() == 401 {
                        return Err(CloudError::TokenExpired);
                    }
                    let v: Value = resp.json().await.map_err(|e| CloudError::Remote(e.to_string()))?;
                    let mut out = Vec::new();
                    if let Some(arr) = v["files"].as_array() {
                        for f in arr {
                            out.push(CloudBackupItem {
                                remote_key: f["id"].as_str().unwrap_or_default().to_string(),
                                name: f["name"].as_str().unwrap_or_default().to_string(),
                                byte_size: f["size"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0),
                                etag_or_sha256: f["sha256Checksum"].as_str().map(str::to_string),
                                modified_at: f["modifiedTime"].as_str().map(str::to_string),
                            });
                        }
                    }
                    Ok(out)
                }
                "onedrive" => {
                    let resp = http
                        .get("https://graph.microsoft.com/v1.0/me/drive/special/approot/children?$select=id,name,size,lastModifiedDateTime")
                        .bearer_auth(&self.access_token)
                        .send()
                        .await
                        .map_err(|e| CloudError::Network(e.to_string()))?;
                    if resp.status().as_u16() == 401 {
                        return Err(CloudError::TokenExpired);
                    }
                    let v: Value = resp.json().await.map_err(|e| CloudError::Remote(e.to_string()))?;
                    let mut out = Vec::new();
                    if let Some(arr) = v["value"].as_array() {
                        for f in arr {
                            out.push(CloudBackupItem {
                                remote_key: f["name"].as_str().unwrap_or_default().to_string(),
                                name: f["name"].as_str().unwrap_or_default().to_string(),
                                byte_size: f["size"].as_u64().unwrap_or(0),
                                etag_or_sha256: None,
                                modified_at: f["lastModifiedDateTime"].as_str().map(str::to_string),
                            });
                        }
                    }
                    Ok(out)
                }
                "dropbox" => {
                    let resp = http
                        .post("https://api.dropboxapi.com/2/files/list_folder")
                        .bearer_auth(&self.access_token)
                        .json(&json!({ "path": "", "recursive": true }))
                        .send()
                        .await
                        .map_err(|e| CloudError::Network(e.to_string()))?;
                    if resp.status().as_u16() == 401 {
                        return Err(CloudError::TokenExpired);
                    }
                    let v: Value = resp.json().await.map_err(|e| CloudError::Remote(e.to_string()))?;
                    let mut out = Vec::new();
                    if let Some(arr) = v["entries"].as_array() {
                        for f in arr {
                            if f[".tag"].as_str() == Some("file") {
                                let path = f["path_display"].as_str().unwrap_or_default().trim_start_matches('/');
                                out.push(CloudBackupItem {
                                    remote_key: path.to_string(),
                                    name: f["name"].as_str().unwrap_or_default().to_string(),
                                    byte_size: f["size"].as_u64().unwrap_or(0),
                                    etag_or_sha256: f["content_hash"].as_str().map(str::to_string),
                                    modified_at: f["server_modified"].as_str().map(str::to_string),
                                });
                            }
                        }
                    }
                    Ok(out)
                }
                _ => Ok(Vec::new()),
            }
        })
    }

    fn upload_backup<'a>(
        &'a self,
        http: &'a reqwest::Client,
        remote_key: &'a str,
        bytes: Vec<u8>,
    ) -> BoxFuture<'a, Result<CloudBackupItem, CloudError>> {
        Box::pin(async move {
            let clean = sanitize_remote_key(remote_key)?;
            let len = bytes.len() as u64;
            let sha = hex::encode(crate::storage::sha256_bytes(&bytes));
            match self.kind {
                "google_drive" => {
                    let meta = if self.use_app_data {
                        json!({ "name": clean.replace('/', "_"), "parents": ["appDataFolder"] })
                    } else {
                        json!({ "name": clean.replace('/', "_") })
                    };
                    let init = http
                        .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=resumable")
                        .bearer_auth(&self.access_token)
                        .header("X-Upload-Content-Length", len.to_string())
                        .json(&meta)
                        .send()
                        .await
                        .map_err(|e| CloudError::Network(e.to_string()))?;
                    if init.status().as_u16() == 401 {
                        return Err(CloudError::TokenExpired);
                    }
                    let upload_url = init
                        .headers()
                        .get("location")
                        .and_then(|v| v.to_str().ok())
                        .ok_or_else(|| CloudError::Remote("Missing Drive upload location".into()))?
                        .to_string();
                    let put = http
                        .put(&upload_url)
                        .header("Content-Type", "application/octet-stream")
                        .body(bytes)
                        .send()
                        .await
                        .map_err(|e| CloudError::Network(e.to_string()))?;
                    if put.status().as_u16() == 507 || put.status().as_u16() == 403 {
                        return Err(CloudError::QuotaExceeded);
                    }
                    let v: Value = put.json().await.map_err(|e| CloudError::Remote(e.to_string()))?;
                    let file_id = v["id"].as_str().unwrap_or(&clean).to_string();
                    Ok(CloudBackupItem {
                        remote_key: file_id,
                        name: clean,
                        byte_size: len,
                        etag_or_sha256: Some(sha),
                        modified_at: Some(chrono::Utc::now().to_rfc3339()),
                    })
                }
                "onedrive" => {
                    let safe_name = clean.replace('/', "_");
                    let sess_url = format!(
                        "https://graph.microsoft.com/v1.0/me/drive/special/approot:/{}:/createUploadSession",
                        urlencoding::encode(&safe_name)
                    );
                    let init = http
                        .post(&sess_url)
                        .bearer_auth(&self.access_token)
                        .json(&json!({ "item": { "@microsoft.graph.conflictBehavior": "replace" } }))
                        .send()
                        .await
                        .map_err(|e| CloudError::Network(e.to_string()))?;
                    if init.status().as_u16() == 401 {
                        return Err(CloudError::TokenExpired);
                    }
                    let sv: Value = init.json().await.map_err(|e| CloudError::Remote(e.to_string()))?;
                    let upload_url = sv["uploadUrl"].as_str().ok_or_else(|| CloudError::Remote("Missing OneDrive uploadUrl".into()))?;
                    let range = if len == 0 {
                        "bytes */0".to_string()
                    } else {
                        format!("bytes 0-{}/{}", len - 1, len)
                    };
                    let put = http
                        .put(upload_url)
                        .header("Content-Range", range)
                        .body(bytes)
                        .send()
                        .await
                        .map_err(|e| CloudError::Network(e.to_string()))?;
                    if !put.status().is_success() {
                        return Err(CloudError::Remote(format!("OneDrive upload {}", put.status())));
                    }
                    Ok(CloudBackupItem {
                        remote_key: safe_name,
                        name: clean,
                        byte_size: len,
                        etag_or_sha256: Some(sha),
                        modified_at: Some(chrono::Utc::now().to_rfc3339()),
                    })
                }
                "dropbox" => {
                    let dbx_path = format!("/{}", clean);
                    let arg = json!({ "path": dbx_path, "mode": "overwrite", "autorename": false });
                    let resp = http
                        .post("https://content.dropboxapi.com/2/files/upload")
                        .bearer_auth(&self.access_token)
                        .header("Dropbox-API-Arg", arg.to_string())
                        .header("Content-Type", "application/octet-stream")
                        .body(bytes)
                        .send()
                        .await
                        .map_err(|e| CloudError::Network(e.to_string()))?;
                    if resp.status().as_u16() == 401 {
                        return Err(CloudError::TokenExpired);
                    }
                    if resp.status().as_u16() == 507 {
                        return Err(CloudError::QuotaExceeded);
                    }
                    if !resp.status().is_success() {
                        return Err(CloudError::Remote(format!("Dropbox upload {}", resp.status())));
                    }
                    Ok(CloudBackupItem {
                        remote_key: clean.clone(),
                        name: clean,
                        byte_size: len,
                        etag_or_sha256: Some(sha),
                        modified_at: Some(chrono::Utc::now().to_rfc3339()),
                    })
                }
                _ => Err(CloudError::Unsupported("upload_backup")),
            }
        })
    }

    fn download_backup<'a>(
        &'a self,
        http: &'a reqwest::Client,
        remote_key: &'a str,
    ) -> BoxFuture<'a, Result<Vec<u8>, CloudError>> {
        Box::pin(async move {
            match self.kind {
                "google_drive" => {
                    let url = format!("https://www.googleapis.com/drive/v3/files/{}?alt=media", urlencoding::encode(remote_key));
                    let resp = http.get(&url).bearer_auth(&self.access_token).send().await.map_err(|e| CloudError::Network(e.to_string()))?;
                    if resp.status().as_u16() == 401 { return Err(CloudError::TokenExpired); }
                    resp.bytes().await.map(|b| b.to_vec()).map_err(|e| CloudError::Network(e.to_string()))
                }
                "onedrive" => {
                    let url = format!("https://graph.microsoft.com/v1.0/me/drive/special/approot:/{}:/content", urlencoding::encode(remote_key));
                    let resp = http.get(&url).bearer_auth(&self.access_token).send().await.map_err(|e| CloudError::Network(e.to_string()))?;
                    if resp.status().as_u16() == 401 { return Err(CloudError::TokenExpired); }
                    resp.bytes().await.map(|b| b.to_vec()).map_err(|e| CloudError::Network(e.to_string()))
                }
                "dropbox" => {
                    let arg = json!({ "path": format!("/{}", remote_key.trim_start_matches('/')) });
                    let resp = http
                        .post("https://content.dropboxapi.com/2/files/download")
                        .bearer_auth(&self.access_token)
                        .header("Dropbox-API-Arg", arg.to_string())
                        .send()
                        .await
                        .map_err(|e| CloudError::Network(e.to_string()))?;
                    if resp.status().as_u16() == 401 { return Err(CloudError::TokenExpired); }
                    resp.bytes().await.map(|b| b.to_vec()).map_err(|e| CloudError::Network(e.to_string()))
                }
                _ => Err(CloudError::Unsupported("download_backup")),
            }
        })
    }

    fn delete_backup<'a>(
        &'a self,
        http: &'a reqwest::Client,
        remote_key: &'a str,
    ) -> BoxFuture<'a, Result<bool, CloudError>> {
        Box::pin(async move {
            match self.kind {
                "google_drive" => {
                    let url = format!("https://www.googleapis.com/drive/v3/files/{}", urlencoding::encode(remote_key));
                    let r = http.delete(&url).bearer_auth(&self.access_token).send().await.map_err(|e| CloudError::Network(e.to_string()))?;
                    Ok(r.status().is_success())
                }
                "onedrive" => {
                    let url = format!("https://graph.microsoft.com/v1.0/me/drive/special/approot:/{}", urlencoding::encode(remote_key));
                    let r = http.delete(&url).bearer_auth(&self.access_token).send().await.map_err(|e| CloudError::Network(e.to_string()))?;
                    Ok(r.status().is_success())
                }
                "dropbox" => {
                    let r = http
                        .post("https://api.dropboxapi.com/2/files/delete_v2")
                        .bearer_auth(&self.access_token)
                        .json(&json!({ "path": format!("/{}", remote_key.trim_start_matches('/')) }))
                        .send()
                        .await
                        .map_err(|e| CloudError::Network(e.to_string()))?;
                    Ok(r.status().is_success())
                }
                _ => Ok(false),
            }
        })
    }
}

pub fn instantiate_provider(provider_id: &str, cfg: &Value) -> Result<Box<dyn CloudStorageProvider>, CloudError> {
    match provider_id {
        "nextcloud" => Ok(Box::new(WebDavProvider::from_config("nextcloud", cfg)?)),
        "webdav" => Ok(Box::new(WebDavProvider::from_config("webdav", cfg)?)),
        "nas" => Ok(Box::new(WebDavProvider::from_config("nas", cfg)?)),
        "s3" => Ok(Box::new(S3UserCloudProvider::from_config("s3", cfg)?)),
        "r2" => Ok(Box::new(S3UserCloudProvider::from_config("r2", cfg)?)),
        "minio" => Ok(Box::new(S3UserCloudProvider::from_config("minio", cfg)?)),
        "google_drive" => Ok(Box::new(OAuthCloudProvider::from_config("google_drive", cfg)?)),
        "onedrive" => Ok(Box::new(OAuthCloudProvider::from_config("onedrive", cfg)?)),
        "dropbox" => Ok(Box::new(OAuthCloudProvider::from_config("dropbox", cfg)?)),
        "icloud" => Err(CloudError::Unsupported("iCloud sử dụng Apple CloudKit bản địa trên macOS/iOS")),
        other => Err(CloudError::InvalidConfig(format!("Không hỗ trợ provider {other}"))),
    }
}
