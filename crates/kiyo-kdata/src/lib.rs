pub mod cloud_chunk;
pub mod error;
pub mod format;
pub mod model;
pub mod zip_compat;

pub use cloud_chunk::{
    AccountCloudKeys, KCHUNK_MAGIC, SealedCloudChunk, derive_account_cloud_keys, open_cloud_chunk,
    open_cloud_manifest, seal_cloud_chunk, seal_cloud_manifest,
};
pub use error::KdataError;
pub use format::{
    FOOTER_LEN, HEADER_LEN, KdataStreamWriter, KdfParams, OpenedKdataArchive, ParsedKdataHeader,
    probe_kdata_header, stream_read_kdata,
};
pub use model::{
    BackupSyncCursor, CIPHER_XCHACHA20_POLY1305_BE32, DEFAULT_CHUNK_SIZE, ExportedAccountInfo,
    ExportedMessageRecord, ImportLimits, KDATA_FOOTER_MAGIC, KDATA_FORMAT_VERSION, KDATA_MAGIC,
    KDATA_SCHEMA_VERSION, KdataEntryKind, KdataIndexEntry, KdataManifest, MissingMediaReport,
    sanitize_relative_path,
};
pub use zip_compat::{
    ExtractedZipEntry, ZipEncryptionMode, ZipEntryInput, read_zip_archive, write_zip_archive,
};

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::collections::HashMap;
    use std::io::Cursor;
    use uuid::Uuid;

    fn sample_manifest_and_entries(backup_id: Uuid) -> (KdataManifest, Vec<(KdataIndexEntry, Vec<u8>)>) {
        let user_id = Uuid::new_v4();
        let account_json = br#"{"username":"kiyo_user","display_name":"Kiyo"}"#.to_vec();
        let msg_json = br#"[{"id":"11111111-1111-1111-1111-111111111111","content":"hello"}]"#.to_vec();
        let media_bytes = vec![0x42u8; 72 * 1024]; // spans multiple chunks when chunk_size = 16 KiB

        let items = vec![
            (KdataEntryKind::Account, "account/profile.json", "application/json", account_json),
            (KdataEntryKind::MessagesJson, "messages/dm_001.json", "application/json", msg_json),
            (KdataEntryKind::MediaBlob, "media/photo.avif", "image/avif", media_bytes),
        ];

        let mut pairs = Vec::new();
        let mut total_plain = 0u64;
        for (i, (kind, path, mime, data)) in items.into_iter().enumerate() {
            total_plain += data.len() as u64;
            let sha = hex::encode(Sha256::digest(&data));
            let idx = KdataIndexEntry {
                entry_id: i as u32,
                kind,
                logical_path: path.to_string(),
                mime_type: mime.to_string(),
                plain_size: data.len() as u64,
                sha256_hex: sha,
                scope_type: Some("dm".into()),
                scope_id: None,
                ref_id: None,
                availability_state: Some("backed_up_encrypted".into()),
            };
            pairs.push((idx, data));
        }

        let manifest = KdataManifest {
            backup_id,
            snapshot_id: Uuid::new_v4(),
            format_version: KDATA_FORMAT_VERSION,
            schema_version: KDATA_SCHEMA_VERSION,
            app_version: "0.1.0".into(),
            owner_user_id: user_id,
            created_at: "2026-11-20T11:30:00Z".into(),
            base_snapshot_id: None,
            incremental: false,
            checksum_algo: "sha256".into(),
            entries_count: pairs.len() as u64,
            total_plain_bytes: total_plain,
            total_media_bytes: 72 * 1024,
            sync_cursor: BackupSyncCursor {
                dm_seq: 10,
                group_seq: 5,
                channel_seq: 2,
            },
            account: ExportedAccountInfo {
                user_id,
                username: Some("kiyo_user".into()),
                display_name: Some("Kiyo".into()),
                avatar_url: None,
                banner_url: None,
                bio: None,
                status_text: None,
                preferences: serde_json::json!({}),
            },
            missing_media: Vec::new(),
            complete: true,
            notes: Vec::new(),
        };
        (manifest, pairs)
    }

    fn build_kdata(backup_id: Uuid, password: &str) -> Vec<u8> {
        let (manifest, pairs) = sample_manifest_and_entries(backup_id);
        let index: Vec<KdataIndexEntry> = pairs.iter().map(|(i, _)| i.clone()).collect();
        let mut out = Vec::new();
        let mut writer = KdataStreamWriter::begin(
            &mut out,
            password,
            &manifest,
            &index,
            16 * 1024, // 16 KiB chunks to exercise multi-frame BE32 streaming
            KdfParams::fast_for_tests(),
            1795147800,
        )
        .expect("begin writer");
        for (entry, data) in &pairs {
            writer.write_entry_bytes(entry.entry_id, data).expect("write entry");
        }
        writer.finish().expect("finish");
        out
    }

    #[test]
    fn roundtrip_and_tamper_truncation_wrong_password_tests() {
        let bid = Uuid::new_v4();
        let pw = "Strong-Kiyo-Passphrase#2026";
        let archive = build_kdata(bid, pw);
        assert!(archive.starts_with(KDATA_MAGIC));

        // 1. Valid stream read
        let mut collected: HashMap<u32, Vec<u8>> = HashMap::new();
        let opened = stream_read_kdata(
            &mut Cursor::new(&archive),
            pw,
            &ImportLimits::default(),
            |entry, chunk, _done| {
                collected.entry(entry.entry_id).or_default().extend_from_slice(chunk);
                Ok(())
            },
        )
        .expect("valid read");
        assert_eq!(opened.manifest.backup_id, bid);
        assert_eq!(collected.get(&2).unwrap().len(), 72 * 1024);
        assert!(opened.frame_count >= 4);

        // 2. Wrong password rejected
        let wrong = stream_read_kdata(&mut Cursor::new(&archive), "wrong-pw", &ImportLimits::default(), |_, _, _| Ok(()));
        assert!(matches!(wrong, Err(KdataError::WrongPasswordOrTampered)));

        // 3. Flip 1 byte in middle (data frame) -> rejected
        let mut flipped = archive.clone();
        let mid = flipped.len() / 2;
        flipped[mid] ^= 0x01;
        assert!(stream_read_kdata(&mut Cursor::new(&flipped), pw, &ImportLimits::default(), |_, _, _| Ok(())).is_err());

        // 4. Flip 1 byte in header -> rejected
        let mut hdr_flip = archive.clone();
        hdr_flip[20] ^= 0xFF;
        assert!(stream_read_kdata(&mut Cursor::new(&hdr_flip), pw, &ImportLimits::default(), |_, _, _| Ok(())).is_err());

        // 5. Truncated archive -> rejected
        let truncated = &archive[..archive.len() - 10];
        assert!(stream_read_kdata(&mut Cursor::new(truncated), pw, &ImportLimits::default(), |_, _, _| Ok(())).is_err());

        // 6. Trailing byte injection -> rejected
        let mut extended = archive.clone();
        extended.push(0x00);
        assert!(stream_read_kdata(&mut Cursor::new(&extended), pw, &ImportLimits::default(), |_, _, _| Ok(())).is_err());
    }

    #[test]
    fn cloud_chunk_isolation_and_integrity() {
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        let keys_a = derive_account_cloud_keys(b"secret-a", &user_a).unwrap();
        let keys_b = derive_account_cloud_keys(b"secret-a", &user_b).unwrap();

        let payload = b"shared media bytes across two users";
        let sealed_a = seal_cloud_chunk(&keys_a, &user_a, payload).unwrap();
        let sealed_b = seal_cloud_chunk(&keys_b, &user_b, payload).unwrap();
        // Zero cross-account dedup leakage!
        assert_ne!(sealed_a.chunk_id_hex, sealed_b.chunk_id_hex);

        let opened = open_cloud_chunk(&keys_a, &user_a, &sealed_a.chunk_id_hex, &sealed_a.wire_bytes).unwrap();
        assert_eq!(opened, payload);

        // User B cannot decrypt or claim User A's chunk
        assert!(open_cloud_chunk(&keys_b, &user_b, &sealed_a.chunk_id_hex, &sealed_a.wire_bytes).is_err());
    }

    #[test]
    fn zip_aes256_and_security_guards() {
        let mut buf = Cursor::new(Vec::new());
        let entries = [ZipEntryInput { path: "messages/hello.txt", bytes: b"hello world" }];
        buf = write_zip_archive(buf, &entries, ZipEncryptionMode::Aes256("zip-secret-2026")).unwrap();
        let bytes = buf.into_inner();
        let len = bytes.len() as u64;

        // Wrong password rejected
        assert!(read_zip_archive(Cursor::new(&bytes), len, Some("bad"), &ImportLimits::default()).is_err());
        // Correct password succeeds
        let got = read_zip_archive(Cursor::new(&bytes), len, Some("zip-secret-2026"), &ImportLimits::default()).unwrap();
        assert_eq!(got[0].bytes, b"hello world");

        // Path traversal rejected
        assert!(sanitize_relative_path("../etc/passwd").is_err());
        assert!(sanitize_relative_path("/abs/path").is_err());
        assert!(sanitize_relative_path("a\\b").is_err());

        // ZIP bomb ratio cap guard
        let big = vec![b'A'; 512 * 1024]; // highly compressible -> tiny ZIP
        let bomb_buf = write_zip_archive(Cursor::new(Vec::new()), &[ZipEntryInput { path: "bomb.txt", bytes: &big }], ZipEncryptionMode::UnencryptedAuthorized).unwrap().into_inner();
        let strict_limits = ImportLimits { max_zip_expansion_ratio: 5, ..Default::default() };
        assert!(matches!(
            read_zip_archive(Cursor::new(&bomb_buf), bomb_buf.len() as u64, None, &strict_limits),
            Err(KdataError::ResourceLimitExceeded(_))
        ));
    }
}
