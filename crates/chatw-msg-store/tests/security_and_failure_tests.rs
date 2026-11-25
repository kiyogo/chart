//! Comprehensive Security, Correctness, Cross-Engine Interop, Crash-Recovery,
//! Eviction, Search, and Failure-Mode Tests for Spec 164 (`chatw-msg-store`).

use chatw_msg_store::*;
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use uuid::Uuid;

fn make_sample_messages(count: usize, base_ts_ms: i64, base_seq: i64) -> Vec<CompactMessage> {
    let senders = [
        Uuid::from_u128(0x1111_2222_3333_4444_5555_6666_7777_8888).to_string(),
        Uuid::from_u128(0xaaaa_bbbb_cccc_dddd_eeee_ffff_0000_1111).to_string(),
        Uuid::from_u128(0x9999_8888_7777_6666_5555_4444_3333_2222).to_string(),
    ];
    let phrases = [
        "ok",
        "alo",
        "ừ",
        "vâng",
        "dạ",
        "hello",
        "hi",
        "yes",
        "no",
        "Xin chào Nguyễn Văn An! Điện thoại mới chạy mượt quá 🔥👍",
        "Chào bạn, hôm nay họp dự án ChatW lúc 14h30 nhé ✅",
        "おはようございます！今日のミーティングよろしくお願いします 😊",
        "你好，收到文件了，谢谢！没问题 🎉",
        "Để mình kiểm tra lại bản cập nhật Zstd Level 22 rồi báo lại ngay nha",
        "Link tài liệu kỹ thuật: https://github.com/kiyogo/chart",
    ];

    (0..count)
        .map(|i| {
            let sender = senders[i % senders.len()].clone();
            let text = phrases[i % phrases.len()].to_string();
            let ts = base_ts_ms + (i as i64) * 15_000;
            let mut msg = CompactMessage {
                id: Uuid::from_u128(0x1000_0000_0000_0000 + (base_seq as u128) + i as u128)
                    .to_string(),
                seq: base_seq + i as i64,
                sender_id: sender,
                created_at_ms: ts,
                msg_type: if i % 11 == 0 {
                    MessageType::Image
                } else if text.contains("https://") {
                    MessageType::Link
                } else {
                    MessageType::Text
                },
                text,
                reply_to_id: if i > 0 && i % 7 == 0 {
                    Some(
                        Uuid::from_u128(0x1000_0000_0000_0000 + (base_seq as u128) + (i - 1) as u128)
                            .to_string(),
                    )
                } else {
                    None
                },
                is_edited: i % 13 == 0,
                deleted_for_all: false,
                is_pinned: i % 29 == 0,
                is_bookmarked: i % 31 == 0,
                status_code: 0,
                reactions: if i % 5 == 0 {
                    vec![ReactionEntry {
                        emoji: if i % 2 == 0 { "👍" } else { "❤️" }.into(),
                        user_id: senders[(i + 1) % senders.len()].clone(),
                        created_at_ms: ts + 3_000,
                    }]
                } else {
                    Vec::new()
                },
                attachments: if i % 11 == 0 {
                    vec![AttachmentRef {
                        ref_id: Uuid::from_u128(0xabcd_0000 + i as u128).to_string(),
                        mime_type: "image/avif".into(),
                        file_name: format!("photo_{i}.avif"),
                        byte_size: 84_210,
                        width: 1920,
                        height: 1080,
                        duration_ms: 0,
                        key_b64: Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into()),
                        nonce_b64: Some("AAAAAAAAAAAAAAAA".into()),
                    }]
                } else {
                    Vec::new()
                },
                extra_meta: if i % 17 == 0 {
                    Some(r#"{"forwarded":true}"#.into())
                } else {
                    None
                },
            };
            if i == 3 {
                // Include decomposed Unicode combining marks to verify NFC/NFD Vietnamese search!
                msg.text = "Nguye\u{0302}\u{0303}n kie\u{0302}\u{0309}m tra đie\u{0323}\u{0302}n thoa\u{0323}i".into();
            }
            msg
        })
        .collect()
}

#[test]
fn roundtrip_lossless_zstd22_e2ee_and_cross_engine_compatibility() {
    let root_key = [0x42u8; 32];
    let key = derive_chunk_storage_key(&root_key, "acc-1", ScopeKind::Dm, "peer-2", 1).unwrap();
    let registry = DictionaryRegistry::new();
    let now_ms = 1_760_000_000_000i64;
    let old_base_ms = now_ms - (10 * 86_400 * 1000);
    let msgs = make_sample_messages(500, old_base_ms, 100);

    let req = SealChunkRequest {
        chunk_id: "chunk-001",
        account_id: "acc-1",
        scope_type: ScopeKind::Dm,
        scope_id: "peer-2",
        chunk_version: 1,
        access_epoch: 1,
        key_version: 1,
        candidate_dict_id: DICT_ID_PUBLIC_CHAT_V1,
        now_ms,
        require_space_savings: true,
        zstd_opts: Zstd22Options::default(),
    };

    let sealed = seal_and_verify_message_chunk(&msgs, &req, &key, &registry).expect("seal ok");
    assert_eq!(sealed.metadata.compression_level, 22);
    assert!(sealed.metadata.encrypted_size < sealed.metadata.original_size);
    assert!(sealed.metadata.serialized_size < sealed.metadata.original_size);

    let restored =
        open_and_verify_message_chunk(&sealed.metadata, &sealed.sealed_bytes, &key, &registry)
            .expect("open ok");
    assert_eq!(restored, msgs);

    // Cross-engine RFC 8878 Level 22 compatibility check when both native-zstd & pure-zstd are compiled!
    #[cfg(all(feature = "native-zstd", feature = "pure-zstd", not(target_arch = "wasm32")))]
    {
        let serialized = serialize_chunk_messages(&msgs).unwrap();
        let dict = &public_chat_dict_v1().bytes;

        // Native libzstd Level 22 -> Pure-Rust structured-zstd decode (with AND without dictionary)
        let nat_no_dict =
            compress_zstd22_native(&serialized, None, Zstd22Options::default()).unwrap();
        let pure_dec =
            decompress_zstd_pure(&nat_no_dict, None, DecompressBounds::default()).unwrap();
        assert_eq!(pure_dec, serialized);

        let nat_dict =
            compress_zstd22_native(&serialized, Some(dict), Zstd22Options::default()).unwrap();
        let pure_dec_dict =
            decompress_zstd_pure(&nat_dict, Some(dict), DecompressBounds::default()).unwrap();
        assert_eq!(pure_dec_dict, serialized);

        // Pure-Rust structured-zstd Level 22 -> Native libzstd decode (with AND without dictionary)
        let pure_no_dict =
            compress_zstd22_pure(&serialized, None, Zstd22Options::default()).unwrap();
        let nat_dec =
            decompress_zstd_native(&pure_no_dict, None, DecompressBounds::default()).unwrap();
        assert_eq!(nat_dec, serialized);

        let pure_dict =
            compress_zstd22_pure(&serialized, Some(dict), Zstd22Options::default()).unwrap();
        let nat_dec_dict =
            decompress_zstd_native(&pure_dict, Some(dict), DecompressBounds::default()).unwrap();
        assert_eq!(nat_dec_dict, serialized);
    }
}

#[test]
fn strict_7_day_boundary_and_incremental_checkpoint_crash_recovery() {
    let now_ms = 1_760_000_000_000i64;
    let six_days_23h_ago = now_ms - (7 * 86_400 - 3600) * 1000;
    let seven_days_1s_ago = now_ms - (7 * 86_400 + 1) * 1000;

    assert!(!is_message_eligible_for_compression(six_days_23h_ago, now_ms));
    assert!(is_message_eligible_for_compression(seven_days_1s_ago, now_ms));

    // Mix of old (>= 9 days) and fresh (< 7 days) messages
    let mut msgs = make_sample_messages(600, now_ms - 12 * 86_400 * 1000, 1);
    let fresh = make_sample_messages(100, now_ms - 2 * 86_400 * 1000, 601);
    msgs.extend(fresh);

    let profile = DeviceCapabilityProfile::default();
    let signals = ResourceSignals::default();
    let mut limits = IncrementalTickLimits::for_device(&profile, &signals);
    limits.target_chunk_bytes = 32 * 1024; // small target for multi-tick test
    limits.max_chunks_per_tick = 1;

    let mut cp = CompressionCheckpoint::new(ScopeKind::Dm, "peer-x", now_ms);

    // Tick 1 seals ONLY 1 small chunk (incremental, never bulk!)
    let b1 = plan_incremental_batches(ScopeKind::Dm, "peer-x", &msgs, now_ms, &limits, &cp);
    assert_eq!(b1.len(), 1);
    assert!(b1[0].seq_end <= 600, "fresh <7d messages must NEVER be included");

    // Simulate crash -> checkpoint persisted with Pause & Backoff
    cp.pause(now_ms);
    assert!(plan_incremental_batches(ScopeKind::Dm, "peer-x", &msgs, now_ms, &limits, &cp).is_empty());
    cp.resume(now_ms);

    cp.record_failure_with_backoff(now_ms);
    assert!(plan_incremental_batches(ScopeKind::Dm, "peer-x", &msgs, now_ms + 5_000, &limits, &cp).is_empty());
    assert!(!plan_incremental_batches(ScopeKind::Dm, "peer-x", &msgs, now_ms + 60_000, &limits, &cp).is_empty());

    // Record sealed chunk 1 and verify deduplication prevents re-compressing it
    cp.record_sealed(b1[0].seq_start, b1[0].seq_end, now_ms + 60_000);
    let b2 = plan_incremental_batches(ScopeKind::Dm, "peer-x", &msgs, now_ms + 61_000, &limits, &cp);
    assert_eq!(b2.len(), 1);
    assert_eq!(b2[0].seq_start, b1[0].seq_end + 1);
    assert!(b2[0].seq_end <= 600);
}

#[test]
fn security_tamper_replay_rollback_cross_account_padding_and_compression_bomb() {
    let root_a = [0x11u8; 32];
    let root_b = [0x22u8; 32];
    let key_a = derive_chunk_storage_key(&root_a, "acc-a", ScopeKind::Dm, "peer-1", 1).unwrap();
    let key_b = derive_chunk_storage_key(&root_b, "acc-b", ScopeKind::Dm, "peer-1", 1).unwrap();
    let registry = DictionaryRegistry::new();
    let now_ms = 1_760_000_000_000i64;
    let msgs = make_sample_messages(100, now_ms - 10 * 86_400 * 1000, 1);

    let req = SealChunkRequest {
        chunk_id: "cid-sec-1",
        account_id: "acc-a",
        scope_type: ScopeKind::Dm,
        scope_id: "peer-1",
        chunk_version: 2,
        access_epoch: 3,
        key_version: 1,
        candidate_dict_id: DICT_ID_PUBLIC_CHAT_V1,
        now_ms,
        require_space_savings: true,
        zstd_opts: Zstd22Options::default(),
    };
    let sealed = seal_and_verify_message_chunk(&msgs, &req, &key_a, &registry).unwrap();

    // 1. Cross-account isolation: Account B key MUST fail!
    assert!(open_and_verify_message_chunk(&sealed.metadata, &sealed.sealed_bytes, &key_b, &registry).is_err());

    // 2. Tamper test: flipping 1 bit in ciphertext MUST fail integrity/AEAD check!
    let mut tampered = sealed.sealed_bytes.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 0x01;
    assert!(open_and_verify_message_chunk(&sealed.metadata, &tampered, &key_a, &registry).is_err());

    // 3. Rollback / Replay test: changing chunk_version or access_epoch or scope_id in metadata MUST fail AAD!
    let mut rollback_meta = sealed.metadata.clone();
    rollback_meta.chunk_version = 1;
    assert!(open_and_verify_message_chunk(&rollback_meta, &sealed.sealed_bytes, &key_a, &registry).is_err());

    let mut wrong_epoch = sealed.metadata.clone();
    wrong_epoch.access_epoch = 2;
    assert!(open_and_verify_message_chunk(&wrong_epoch, &sealed.sealed_bytes, &key_a, &registry).is_err());

    let mut wrong_scope = sealed.metadata.clone();
    wrong_scope.scope_id = "peer-other".into();
    assert!(open_and_verify_message_chunk(&wrong_scope, &sealed.sealed_bytes, &key_a, &registry).is_err());

    // 4. Side-channel padding resistance: slightly different payloads map to identical padded bucket sizes!
    let p1 = pad_for_side_channel_resistance(&[1u8; 300]);
    let p2 = pad_for_side_channel_resistance(&[2u8; 347]);
    assert_eq!(p1.len(), p2.len());
    assert_eq!(p1.len() % 256, 0);

    // 5. Missing dictionary handling
    let empty_reg = DictionaryRegistry::new();
    let mut missing_dict_meta = sealed.metadata.clone();
    missing_dict_meta.dictionary_id = 9999;
    assert!(open_and_verify_message_chunk(&missing_dict_meta, &sealed.sealed_bytes, &key_a, &empty_reg).is_err());

    // 6. Compression Bomb protection: highly compressible 8 MiB zeros compressed to ~100 bytes
    // MUST be rejected when bounds are strict!
    let bomb_plain = vec![0u8; 8 * 1024 * 1024];
    let bomb_zstd = compress_zstd22(&bomb_plain, None, None).unwrap();
    assert!(bomb_zstd.len() < 4096);
    let strict_bounds = DecompressBounds {
        max_output_bytes: 512 * 1024,
        max_expansion_ratio: 50,
        max_window_log: 24,
    };
    let err = decompress_zstd_bounded(&bomb_zstd, None, Some(strict_bounds)).unwrap_err();
    assert!(matches!(err, MsgStoreError::DecompressionBomb { .. }));

    // 7. Malformed fuzz-like inputs MUST NEVER panic
    for garbage in [
        &b""[..],
        &b"CWMC1"[..],
        &b"CWMC1\x01\x00\xff\xff\xff\xff\xff"[..],
        &b"CWEC01\x01\x00short"[..],
        &b"CWSE1\x01\x00bad"[..],
        &[0x28, 0xb5, 0x2f, 0xfd, 0x00, 0xff, 0xff],
    ] {
        let _ = deserialize_chunk_messages(garbage);
        let _ = decompress_zstd_bounded(garbage, None, None);
        let _ = open_and_verify_message_chunk(&sealed.metadata, garbage, &key_a, &registry);
    }

    // 8. Resource-constrained Level 22 guard defers cleanly instead of OOM/downgrading!
    let tight_opts = Zstd22Options {
        max_window_log: 22,
        ram_budget_bytes: Some(1024 * 1024), // 1 MiB impossibly small -> must return ResourceConstrained
        content_checksum: true,
    };
    let serialized = serialize_chunk_messages(&msgs).unwrap();
    let res = compress_zstd22(&serialized, None, Some(tight_opts));
    assert!(matches!(res, Err(MsgStoreError::ResourceConstrained(_))));
}

#[test]
fn vietnamese_multilingual_encrypted_search_mutations_and_tombstones() {
    let root_key = [0x77u8; 32];
    let search_key = derive_search_index_key(&root_key, "acc-1", 1).unwrap();
    let other_key = derive_search_index_key(&[0x88u8; 32], "acc-2", 1).unwrap();

    let now_ms = 1_760_000_000_000i64;
    let msgs = make_sample_messages(200, now_ms - 10 * 86_400 * 1000, 1);

    let mut idx = LocalSearchIndex::new();
    idx.set_scope_coverage("dm:peer-2", SearchCoverageState::IndexedComplete);
    for (i, m) in msgs.iter().enumerate() {
        idx.upsert_message(ScopeKind::Dm, "peer-2", None, None, i as u32, m);
    }

    // Vietnamese accent-insensitive search ("xin chao" finds "Xin chào", "nguyen" finds "Nguyễn" AND decomposed NFD!)
    let r1 = idx
        .search(
            &SearchQuery {
                text: "xin chao nguyen".into(),
                exact_diacritics: false,
                phrase_match: false,
                prefix_match: true,
                filters: SearchFilters::default(),
                offset: 0,
                limit: 20,
            },
            None,
        )
        .unwrap();
    assert!(!r1.hits.is_empty());
    assert!(!r1.from_result_cache);

    // Second identical query hits Search Result Cache!
    let r1_cached = idx
        .search(
            &SearchQuery {
                text: "xin chao nguyen".into(),
                exact_diacritics: false,
                phrase_match: false,
                prefix_match: true,
                filters: SearchFilters::default(),
                offset: 0,
                limit: 20,
            },
            None,
        )
        .unwrap();
    assert!(r1_cached.from_result_cache);
    assert_eq!(r1_cached.total_matched, r1.total_matched);

    // "dien thoai" matches both precomposed "Điện thoại" and decomposed NFD "đie\u{0323}\u{0302}n thoa\u{0323}i"!
    let r_dt = idx
        .search(
            &SearchQuery {
                text: "dien thoai".into(),
                exact_diacritics: false,
                phrase_match: true,
                prefix_match: false,
                filters: SearchFilters::default(),
                offset: 0,
                limit: 50,
            },
            None,
        )
        .unwrap();
    assert!(r_dt.hits.iter().any(|h| h.message_id == msgs[3].id));

    // Japanese & Chinese & Emoji search
    let r_jp = idx
        .search(
            &SearchQuery {
                text: "ミーティング".into(),
                exact_diacritics: false,
                phrase_match: false,
                prefix_match: false,
                filters: SearchFilters::default(),
                offset: 0,
                limit: 10,
            },
            None,
        )
        .unwrap();
    assert!(!r_jp.hits.is_empty());

    let r_emoji = idx
        .search(
            &SearchQuery {
                text: "🔥".into(),
                exact_diacritics: false,
                phrase_match: false,
                prefix_match: false,
                filters: SearchFilters::default(),
                offset: 0,
                limit: 10,
            },
            None,
        )
        .unwrap();
    assert!(!r_emoji.hits.is_empty());

    // Update chunk locator WITHOUT re-tokenizing
    assert!(idx.update_chunk_locator(&msgs[3].id, "chunk-007", 3));
    let r_loc = idx
        .search(
            &SearchQuery {
                text: "dien thoai".into(),
                exact_diacritics: false,
                phrase_match: true,
                prefix_match: false,
                filters: SearchFilters::default(),
                offset: 0,
                limit: 50,
            },
            None,
        )
        .unwrap();
    let hit3 = r_loc.hits.iter().find(|h| h.message_id == msgs[3].id).unwrap();
    assert_eq!(hit3.chunk_id.as_deref(), Some("chunk-007"));
    assert_eq!(hit3.chunk_msg_offset, 3);

    // Delete message + verify tombstone prevents resurrection even if old chunk re-upserts it!
    idx.delete_message(&msgs[3].id);
    idx.upsert_message(ScopeKind::Dm, "peer-2", None, Some("chunk-007"), 3, &msgs[3]);
    let r_after_del = idx
        .search(
            &SearchQuery {
                text: "dien thoai".into(),
                exact_diacritics: false,
                phrase_match: true,
                prefix_match: false,
                filters: SearchFilters::default(),
                offset: 0,
                limit: 50,
            },
            None,
        )
        .unwrap();
    assert!(!r_after_del.hits.iter().any(|h| h.message_id == msgs[3].id));

    // Cooperative cancellation check
    let cancel = AtomicBool::new(true);
    let cancelled = idx.search(
        &SearchQuery {
            text: "chatw".into(),
            exact_diacritics: false,
            phrase_match: false,
            prefix_match: true,
            filters: SearchFilters::default(),
            offset: 0,
            limit: 10,
        },
        Some(&cancel),
    );
    assert!(matches!(cancelled, Err(MsgStoreError::Cancelled)));

    // Encrypted search shard round-trip & cross-account protection
    let sealed_shard = idx
        .seal_encrypted_shard(&search_key, "acc-1", "dm:peer-2", 1)
        .unwrap();
    // Plaintext terms MUST NOT appear in encrypted shard!
    let as_str = String::from_utf8_lossy(&sealed_shard);
    assert!(!as_str.contains("dien thoai"));
    assert!(!as_str.contains("Xin chào"));

    // Wrong account key fails!
    assert!(LocalSearchIndex::open_encrypted_shard(&other_key, "acc-2", "dm:peer-2", 1, &sealed_shard).is_err());

    // Correct key restores exact index!
    let mut restored_idx =
        LocalSearchIndex::open_encrypted_shard(&search_key, "acc-1", "dm:peer-2", 1, &sealed_shard)
            .unwrap();
    assert_eq!(restored_idx.doc_count(), idx.doc_count());
    let r_restored = restored_idx
        .search(
            &SearchQuery {
                text: "chatw".into(),
                exact_diacritics: false,
                phrase_match: false,
                prefix_match: true,
                filters: SearchFilters::default(),
                offset: 0,
                limit: 10,
            },
            None,
        )
        .unwrap();
    assert!(!r_restored.hits.is_empty());

    // Apply mutations (edit + recall + pin + reaction) to immutable chunk
    let mut chunk_msgs = msgs.clone();
    let applied = apply_mutations_to_chunk(
        &mut chunk_msgs,
        &[
            ChunkMutation {
                mutation_seq: 1,
                scope_type: ScopeKind::Dm,
                scope_id: "peer-2".into(),
                message_id: msgs[0].id.clone(),
                chunk_id: Some("chunk-001".into()),
                op: MutationOp::Edit,
                actor_id: msgs[0].sender_id.clone(),
                new_text: Some("Nội dung đã sửa".into()),
                emoji: None,
                created_at_ms: now_ms,
            },
            ChunkMutation {
                mutation_seq: 2,
                scope_type: ScopeKind::Dm,
                scope_id: "peer-2".into(),
                message_id: msgs[1].id.clone(),
                chunk_id: Some("chunk-001".into()),
                op: MutationOp::Recall,
                actor_id: msgs[1].sender_id.clone(),
                new_text: None,
                emoji: None,
                created_at_ms: now_ms + 1000,
            },
        ],
    );
    assert_eq!(applied, 2);
    assert_eq!(chunk_msgs[0].text, "Nội dung đã sửa");
    assert!(chunk_msgs[0].is_edited);
    assert!(chunk_msgs[1].deleted_for_all);
    assert!(chunk_msgs[1].text.is_empty());
}

#[test]
fn adaptive_budget_hysteresis_l1_wipe_prefetch_and_scheduler_priority() {
    let mut profile = DeviceCapabilityProfile {
        platform: PlatformKind::Android,
        total_ram_mb: 4096,
        available_ram_mb: 2048,
        cpu_cores: 8,
        total_disk_bytes: 64 * 1024 * 1024 * 1024,
        free_disk_bytes: 15 * 1024 * 1024 * 1024,
        os_quota_bytes: None,
        is_ssd: true,
        is_mobile: true,
    };
    let signals = ResourceSignals::default();

    let b1 = compute_dynamic_storage_budget(&profile, StorageMode::Auto, None, None, &signals);
    assert!(b1.total_local_budget_bytes >= 512 * 1024 * 1024);
    assert!(b1.total_local_budget_bytes <= b1.effective_safe_ceiling_bytes);

    // Slight free space jitter (±5%) MUST NOT change budget thanks to ±12% hysteresis!
    profile.free_disk_bytes = 14_500 * 1024 * 1024;
    let b2 = compute_dynamic_storage_budget(
        &profile,
        StorageMode::Auto,
        None,
        Some(b1.total_local_budget_bytes),
        &signals,
    );
    assert_eq!(b2.total_local_budget_bytes, b1.total_local_budget_bytes);

    // Custom mode exceeding safe ceiling MUST be clamped strictly!
    profile.free_disk_bytes = 600 * 1024 * 1024;
    let b_custom = compute_dynamic_storage_budget(
        &profile,
        StorageMode::Custom,
        Some(10_000),
        None,
        &signals,
    );
    assert!(b_custom.total_local_budget_bytes <= b_custom.effective_safe_ceiling_bytes);
    assert!(b_custom.total_local_budget_bytes < 600 * 1024 * 1024);

    // L1 cache bounds & logout wipe
    let mut l1 = L1ChunkCache::new(2 * 1024 * 1024);
    l1.put("c1".into(), make_sample_messages(20, 1000, 1), 15, false);
    assert!(l1.get("c1").is_some());
    l1.wipe_sensitive();
    assert!(l1.get("c1").is_none());
    assert_eq!(l1.stats().0, 0);

    // Resource scheduler suspends background compression during active video call!
    let call_signals = ResourceSignals {
        active_voice_or_video_call: true,
        ..Default::default()
    };
    let call_budget = ResourceScheduler::compute_budget(&profile, &call_signals);
    assert!(!call_budget.allow_compression);
    assert!(!call_budget.allow_prefetch);

    let mut sched = ResourceScheduler::new();
    sched.enqueue(ScheduledTask {
        task_id: "compress-1".into(),
        priority: TaskPriority::BackgroundCompress,
        enqueued_at_ms: 1,
        estimated_cpu_ms: 40,
        estimated_ram_bytes: 8 * 1024 * 1024,
        scope_key: Some("dm:1".into()),
    });
    sched.enqueue(ScheduledTask {
        task_id: "send-1".into(),
        priority: TaskPriority::MessageRealtime,
        enqueued_at_ms: 2,
        estimated_cpu_ms: 1,
        estimated_ram_bytes: 1024,
        scope_key: Some("dm:1".into()),
    });
    // Realtime message MUST pop first even during call; compression stays deferred!
    assert_eq!(sched.pop_next_allowed(&call_budget).unwrap().task_id, "send-1");
    assert!(sched.pop_next_allowed(&call_budget).is_none());

    // Prefetch planner respects scroll direction & skips during calls
    let chunks: Vec<String> = (0..6).map(|i| format!("c{i}")).collect();
    let local: HashSet<String> = ["c3".to_string()].into_iter().collect();
    let plan = PrefetchPlanner::plan_for_conversation(
        "dm:1",
        &chunks,
        3,
        ScrollDirection::OlderHistory,
        &local,
        &b1,
        &signals,
    );
    assert_eq!(plan.front().unwrap().chunk_id, "c2");
    assert!(
        PrefetchPlanner::plan_for_conversation(
            "dm:1",
            &chunks,
            3,
            ScrollDirection::OlderHistory,
            &local,
            &b1,
            &call_signals,
        )
        .is_empty()
    );
}
