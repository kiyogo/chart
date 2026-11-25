//! Real Benchmark Suite for Spec 165 (Sections 36, 37, 38, 43):
//! 1. Plain SQLite vs SQLCipher (AES-256) write/read latency & disk footprint overhead.
//! 2. Inline SQLite BLOB vs External Encrypted Chunk File across 2 KiB .. 256 KiB.
//! 3. Unified SQLite + SQLCipher + FTS5 Vietnamese full-text search scaling up to 1,000,000 messages!
//!
//! Run with:
//! `CARGO_INCREMENTAL=0 cargo run --release -p chatw-msg-store --features "native-zstd,sqlite-sqlcipher" --example benchmark_spec165_storage`

use std::time::Instant;

use chatw_msg_store::{
    CompactMessage, EncryptedSqliteStore, MessageType, ScopeKind, SearchCoverageState,
    SearchFilters, SearchQuery, SqliteChunkRefRecord,
};

fn build_vietnamese_batch(start: usize, count: usize) -> Vec<CompactMessage> {
    let templates = [
        "Hôm nay họp dự án đường sắt cao tốc Bắc Nam lúc 14h30 tại phòng họp số 3",
        "Đã gửi báo cáo tài chính quý 4 và bảng đối chiếu công nợ khách hàng nhé",
        "Chiều nay đi uống cà phê sữa đá ở quận 1 không mọi người ơi",
        "Mình vừa đẩy bản cập nhật Zstandard cấp 22 và mã hóa đầu cuối lên máy chủ",
        "Cảm ơn anh rất nhiều, hẹn gặp lại ở Đà Nẵng vào cuối tuần này",
        "Kiểm tra lại tốc độ tìm kiếm tiếng Việt có dấu và không dấu trên một triệu tin nhắn",
        "Hệ thống lưu trữ thông minh tự động tối ưu dung lượng theo từng thiết bị",
        "Tệp đính kèm hợp đồng bản chính đã được đồng bộ ngoại tuyến an toàn",
    ];
    let senders = ["alice", "bob", "charlie", "duong", "minh"];
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let idx = start + i;
        let base = templates[idx % templates.len()];
        let text = if idx == 777_777 {
            "Bí mật đặc biệt kiểm thử triệu tin nhắn: mật khẩu đường sắt Đà Nẵng 2026".to_string()
        } else {
            format!("{base} #{idx}")
        };
        out.push(CompactMessage {
            id: format!("m-{idx:07}"),
            seq: idx as i64 + 1,
            sender_id: senders[idx % senders.len()].to_string(),
            created_at_ms: 1_700_000_000_000 + (idx as i64) * 1_000,
            msg_type: MessageType::Text,
            text,
            reply_to_id: None,
            is_edited: false,
            deleted_for_all: false,
            is_pinned: idx % 1000 == 0,
            is_bookmarked: false,
            status_code: 0,
            reactions: Vec::new(),
            attachments: Vec::new(),
            extra_meta: None,
        });
    }
    out
}

fn dir_size(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    if let Ok(rd) = std::fs::read_dir(path) {
        for e in rd.flatten() {
            if let Ok(m) = e.metadata() {
                if m.is_file() {
                    total += m.len();
                } else if m.is_dir() {
                    total += dir_size(&e.path());
                }
            }
        }
    }
    total
}

fn main() {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let base = std::env::temp_dir().join(format!("chatw_bench165_{}_{ts}", std::process::id()));
    std::fs::create_dir_all(&base).unwrap();
    let root_key = [0x42u8; 32];

    println!("=============================================================================");
    println!("  CHATW SPEC 165 — REAL STORAGE, SQLCIPHER & 1M-MESSAGE FTS5 BENCHMARK");
    println!("=============================================================================\n");

    // ── Benchmark 1: Plain SQLite vs SQLCipher (AES-256) Encryption Overhead (20,000 messages) ──
    println!("[1] Plain SQLite vs SQLCipher (AES-256 + FTS5) — 20,000 Vietnamese Messages");
    let sample_20k = build_vietnamese_batch(0, 20_000);

    let plain_dir = base.join("plain");
    let mut plain_db = EncryptedSqliteStore::open_plain_for_benchmark(
        &plain_dir.join("plain.db"),
        &plain_dir.join("chunks"),
        "bench-plain",
    )
    .unwrap();
    let t0 = Instant::now();
    for chunk in sample_20k.chunks(2_000) {
        plain_db
            .upsert_messages_batch(ScopeKind::Dm, "dm-bench", Some("chk-0"), chunk, SearchCoverageState::IndexedComplete)
            .unwrap();
    }
    let plain_write_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let q_vn = SearchQuery {
        text: "duong sat cao toc".into(),
        exact_diacritics: false,
        phrase_match: false,
        prefix_match: true,
        filters: SearchFilters::default(),
        offset: 0,
        limit: 30,
    };
    let tq0 = Instant::now();
    let r_plain = plain_db.search_fts5(&q_vn).unwrap();
    let plain_search_ms = tq0.elapsed().as_secs_f64() * 1000.0;
    let plain_bytes = dir_size(&plain_dir);
    drop(plain_db);

    let cipher_dir = base.join("cipher");
    let mut cipher_db = EncryptedSqliteStore::open(&cipher_dir, "bench-cipher", &root_key, 1).unwrap();
    let t1 = Instant::now();
    for chunk in sample_20k.chunks(2_000) {
        cipher_db
            .upsert_messages_batch(ScopeKind::Dm, "dm-bench", Some("chk-0"), chunk, SearchCoverageState::IndexedComplete)
            .unwrap();
    }
    let cipher_write_ms = t1.elapsed().as_secs_f64() * 1000.0;
    let tq1 = Instant::now();
    let r_cipher = cipher_db.search_fts5(&q_vn).unwrap();
    let cipher_search_ms = tq1.elapsed().as_secs_f64() * 1000.0;
    let cipher_bytes = dir_size(&cipher_dir);
    assert_eq!(r_plain.hits.len(), r_cipher.hits.len());

    println!("  Plain SQLite + FTS5    : write={plain_write_ms:7.1} ms | search={plain_search_ms:5.2} ms | disk={:.2} MiB", plain_bytes as f64 / 1_048_576.0);
    println!("  SQLCipher AES-256+FTS5 : write={cipher_write_ms:7.1} ms | search={cipher_search_ms:5.2} ms | disk={:.2} MiB", cipher_bytes as f64 / 1_048_576.0);
    println!("  Encryption Overhead    : write={:+.1}% | disk={:+.1}%\n",
        ((cipher_write_ms / plain_write_ms) - 1.0) * 100.0,
        ((cipher_bytes as f64 / plain_bytes as f64) - 1.0) * 100.0,
    );

    // ── Benchmark 2: SQLite Inline BLOB vs External Encrypted Chunk File (2 KiB .. 256 KiB) ──
    println!("[2] SQLCipher Inline BLOB vs External Encrypted File (50 chunks per size)");
    println!("  {:>10} | {:>13} | {:>13} | {:>13} | {:>13}", "Chunk Size", "Inline W(ms)", "ExtFile W(ms)", "Inline R(ms)", "ExtFile R(ms)");
    for &sz in &[2 * 1024usize, 8 * 1024, 16 * 1024, 64 * 1024, 256 * 1024] {
        let payload: Vec<u8> = (0..sz).map(|i| ((i * 31 + 17) & 0xFF) as u8).collect();
        let b3 = blake3::hash(&payload).to_hex().to_string();

        let dir_in = base.join(format!("blob_in_{sz}"));
        let db_in = EncryptedSqliteStore::open(&dir_in, "in", &root_key, 1).unwrap();
        let tw0 = Instant::now();
        for i in 0..50 {
            let rec = SqliteChunkRefRecord {
                chunk_id: format!("in-{sz}-{i}"),
                scope_key: "dm:1".into(),
                chunk_version: 1,
                seq_start: i * 100,
                seq_end: i * 100 + 99,
                message_count: 100,
                original_size: (sz * 4) as i64,
                compressed_size: sz as i64,
                encrypted_size: sz as i64,
                dictionary_id: 1,
                ciphertext_blake3: b3.clone(),
                storage_mode: "inline_blob".into(),
                file_rel_path: None,
                pinned_offline: false,
            };
            db_in.store_chunk_inline(&rec, &payload).unwrap();
        }
        let in_w = tw0.elapsed().as_secs_f64() * 1000.0;
        let tr0 = Instant::now();
        for i in 0..50 {
            let got = db_in.read_chunk_bytes(&format!("in-{sz}-{i}")).unwrap().unwrap();
            assert_eq!(got.len(), sz);
        }
        let in_r = tr0.elapsed().as_secs_f64() * 1000.0;
        drop(db_in);

        let dir_ex = base.join(format!("blob_ex_{sz}"));
        let db_ex = EncryptedSqliteStore::open(&dir_ex, "ex", &root_key, 1).unwrap();
        let tw1 = Instant::now();
        for i in 0..50 {
            let rec = SqliteChunkRefRecord {
                chunk_id: format!("ex-{sz}-{i}"),
                scope_key: "dm:1".into(),
                chunk_version: 1,
                seq_start: i * 100,
                seq_end: i * 100 + 99,
                message_count: 100,
                original_size: (sz * 4) as i64,
                compressed_size: sz as i64,
                encrypted_size: sz as i64,
                dictionary_id: 1,
                ciphertext_blake3: b3.clone(),
                storage_mode: "external_file".into(),
                file_rel_path: None,
                pinned_offline: false,
            };
            db_ex.store_chunk_external(&rec, &payload).unwrap();
        }
        let ex_w = tw1.elapsed().as_secs_f64() * 1000.0;
        let tr1 = Instant::now();
        for i in 0..50 {
            let got = db_ex.read_chunk_bytes(&format!("ex-{sz}-{i}")).unwrap().unwrap();
            assert_eq!(got.len(), sz);
        }
        let ex_r = tr1.elapsed().as_secs_f64() * 1000.0;
        drop(db_ex);

        println!(
            "  {:>7} KiB | {:>13.2} | {:>13.2} | {:>13.2} | {:>13.2}",
            sz / 1024, in_w, ex_w, in_r, ex_r
        );
    }
    println!();

    // ── Benchmark 3: 1,000,000-Message Encrypted SQLCipher + FTS5 Vietnamese Search! ──
    println!("[3] 1,000,000-Message Encrypted SQLCipher + FTS5 Vietnamese Full-Text Search");
    let million_dir = base.join("million_fts5");
    let mut mdb = EncryptedSqliteStore::open(&million_dir, "acc-1m", &root_key, 1).unwrap();
    let total_msgs = 1_000_000usize;
    let batch_sz = 25_000usize;

    let templates = [
        "Hôm nay họp dự án đường sắt cao tốc Bắc Nam lúc 14h30 tại phòng họp số 3",
        "Đã gửi báo cáo tài chính quý 4 và bảng đối chiếu công nợ khách hàng nhé",
        "Chiều nay đi uống cà phê sữa đá ở quận 1 không mọi người ơi",
        "Mình vừa đẩy bản cập nhật Zstandard cấp 22 và mã hóa đầu cuối lên máy chủ",
        "Cảm ơn anh rất nhiều, hẹn gặp lại ở Đà Nẵng vào cuối tuần này",
        "Kiểm tra lại tốc độ tìm kiếm tiếng Việt có dấu và không dấu trên triệu tin nhắn",
        "Hệ thống lưu trữ thông minh tự động tối ưu dung lượng theo từng thiết bị",
        "Tệp đính kèm hợp đồng bản chính đã được đồng bộ ngoại tuyến an toàn",
    ];
    let senders = ["alice", "bob", "charlie", "duong", "minh"];

    let ti0 = Instant::now();
    for start in (0..total_msgs).step_by(batch_sz) {
        let mut owned: Vec<(String, &'static str, i64, String)> = Vec::with_capacity(batch_sz);
        for i in start..(start + batch_sz).min(total_msgs) {
            let txt = if i == 777_777 {
                "Bí mật đặc biệt kiểm thử triệu tin nhắn: mật khẩu đường sắt Đà Nẵng 2026".to_string()
            } else {
                format!("{} #{}", templates[i % templates.len()], i)
            };
            owned.push((
                format!("m-{i:07}"),
                senders[i % senders.len()],
                1_700_000_000_000 + (i as i64) * 1000,
                txt,
            ));
        }
        let refs: Vec<(&str, &str, i64, &str)> = owned
            .iter()
            .map(|(id, s, ts, t)| (id.as_str(), *s, *ts, t.as_str()))
            .collect();
        let chunk_tag = format!("chk-{:04}", start / batch_sz);
        mdb.bulk_index_fts_only("dm:million", &chunk_tag, &refs).unwrap();
    }
    let idx_secs = ti0.elapsed().as_secs_f64();
    let stats = mdb.stats();
    println!(
        "  Indexed {} docs into AES-256 SQLCipher+FTS5 in {:.2}s ({:.0} msgs/sec) | DB size: {:.2} MiB",
        stats.fts_doc_count,
        idx_secs,
        total_msgs as f64 / idx_secs,
        stats.db_file_bytes as f64 / 1_048_576.0
    );

    // Needle-in-a-haystack Vietnamese accent-insensitive query across 1,000,000 messages!
    let q_needle = SearchQuery {
        text: "mat khau duong sat da nang 2026".into(),
        exact_diacritics: false,
        phrase_match: false,
        prefix_match: true,
        filters: SearchFilters::default(),
        offset: 0,
        limit: 20,
    };
    let ts1 = Instant::now();
    let r_needle = mdb.search_fts5(&q_needle).unwrap();
    let needle_ms = ts1.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(r_needle.hits.len(), 1);
    assert_eq!(r_needle.hits[0].message_id, "m-0777777");
    println!("  Needle query ('mat khau duong sat da nang 2026'): {:6.2} ms | hits={} | chunk={:?}",
        needle_ms, r_needle.hits.len(), r_needle.hits[0].chunk_id);

    // Exact phrase + accented Vietnamese query across 1,000,000 messages!
    let q_phrase = SearchQuery {
        text: "cà phê sữa đá".into(),
        exact_diacritics: true,
        phrase_match: true,
        prefix_match: false,
        filters: SearchFilters::default(),
        offset: 0,
        limit: 30,
    };
    let ts2 = Instant::now();
    let r_phrase = mdb.search_fts5(&q_phrase).unwrap();
    let phrase_ms = ts2.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(r_phrase.hits.len(), 30);
    println!("  Exact accented phrase ('cà phê sữa đá', top 30) : {:6.2} ms | hits={} | has_more={}",
        phrase_ms, r_phrase.hits.len(), r_phrase.has_more);

    // Filtered query (sender + prefix) across 1,000,000 messages!
    let q_filt = SearchQuery {
        text: "zstand".into(),
        exact_diacritics: false,
        phrase_match: false,
        prefix_match: true,
        filters: SearchFilters {
            sender_id: Some("alice".into()),
            ..Default::default()
        },
        offset: 0,
        limit: 30,
    };
    let ts3 = Instant::now();
    let r_filt = mdb.search_fts5(&q_filt).unwrap();
    let filt_ms = ts3.elapsed().as_secs_f64() * 1000.0;
    assert!(!r_filt.hits.is_empty());
    println!("  Filtered prefix ('zstand' + sender='alice')     : {:6.2} ms | hits={}",
        filt_ms, r_filt.hits.len());

    drop(mdb);
    let _ = std::fs::remove_dir_all(&base);
    println!("\nBenchmark complete (temp dir cleaned).");
}
