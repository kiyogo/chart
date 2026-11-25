# Spec 164 — Incremental Zstd-22 Message Compression, Intelligent Adaptive Storage, Smart Cache & Encrypted Full-Text Search

## 1. Problem Statement
ChatW currently stores client-side cached messages as uncompressed, unencrypted JSON arrays (truncated at 500 messages on Web and unbounded on Native), lacks message-level Zstandard Level 22 incremental chunk archival, lacks a multi-tier Hot/Warm/Cold storage lifecycle for messages, lacks an encrypted local full-text search engine (with Vietnamese diacritic support and cross-chunk jump-to-message), and lacks a unified cross-platform adaptive storage budget + resource scheduler.

## 2. Core Objectives & Hard Invariants
1. **Strict 7-Day Compression Age Boundary**:
   - Messages `< 7 days` old (`< 604,800s`): sent, received, E2EE-encrypted, synced, and stored completely normally with zero compression latency.
   - Messages `>= 7 days` old: eligible for incremental lossless chunking + **Zstandard Level 22** compression (never silently downgraded below Level 22; defers or shrinks chunk size within safe bounds when device resources are constrained).
   - Zero data loss: 100% lossless round-trip preserving exact IDs, timestamps, sender IDs, reply refs, reactions, flags, attachments, and metadata. Zero AI summarization/alteration.
   - Never replaces storage unless compressed + padded + encrypted output strictly saves space.
2. **Incremental Background Processing (Zero Bulk Spikes)**:
   - Distributed over time with bounded per-tick limits (max messages, max bytes, max CPU ms, max concurrency), backpressure (active calls, low battery, thermal, scrolling/typing), crash-safe checkpoints, pause/resume, exponential backoff retry, deduplication, and multi-device distributed leases + OCC (`manifest_version` CAS).
   - Never compresses all history at once or on startup; never re-compresses whole conversations on every new message.
3. **E2EE & Cryptographic Security**:
   - All E2EE message compression happens strictly on authorized user devices (decrypt -> binary pack -> Zstd-22 -> side-channel-mitigating padding -> AES-256-GCM AEAD with domain-separated HKDF subkey & fresh CSPRNG 96-bit nonce + full metadata AAD -> verify decrypt/decompress -> upload -> atomic manifest commit).
   - Zero plaintext messages, plaintext search indices, search queries, or usable decryption keys ever sent to server or written to logs.
   - Strict scope isolation: never mixes conversations, accounts, groups, channels, or incompatible history-access epochs in a single chunk or private dictionary.
   - Decompression bomb protection: strict windowLog cap, hard decompressed byte ceiling, max expansion ratio cap, and max message count check BEFORE allocation.
4. **Compact Lossless Pre-Compression + Zstd Dictionary**:
   - Columnar binary format `CWMC1` (versioned) using sender ID table mapping, ZigZag varint timestamp/seq deltas, bit-packed flags, and local repeated-string table.
   - Versioned public multilingual/Vietnamese chat Zstd dictionary (`dict_id = 1`, BLAKE3-verified) plus support for E2EE per-scope dictionaries (only used when measurably improving compression; old dictionaries retained while referenced).
5. **Hot / Warm / Cold Architecture + Intelligent Local Storage & Adaptive Quota**:
   - **Hot**: `< 7d` messages + L1 bounded RAM LRU cache (zeroized/wiped on lock/logout).
   - **Warm**: Compressed encrypted chunks in L2 local storage (Web OPFS/IDB, Native filesystem) + server local CAS / fast tier.
   - **Cold**: Older/infrequently accessed chunks archived to S3/MinIO cold prefix/storage-class, lazy-fetched and lazy-decompressed on demand.
   - **Adaptive Storage Budget** across Web, Android, iOS, Windows, Linux, macOS: tiered free-space curve (`<1GB` -> `>250GB`), reserved OS headroom, hysteresis (`±12%` deadband) to prevent oscillation, modes (`Auto`, `SpaceSaver`, `HighPerformance`, `Custom` clamped safely), W-TinyLFU + Segmented LRU + cost-aware eviction protecting unsynced/unique/pinned data.
6. **High-Performance Encrypted Full-Text Search (Vietnamese + Multilingual)**:
   - Local encrypted sharded inverted index (`CWSE1`) updated incrementally on send/receive/edit/delete/recall/chunk-seal (never full rebuilds; never decompresses entire history to search).
   - Full Unicode NFD/NFC + Vietnamese accented AND accent-insensitive matching (`xin chao` -> `xin chào`, `nguyen` -> `Nguyễn`, `dien thoai` -> `điện thoại`), prefix, exact, phrase, CJK bigrams, emoji, BM25 + recency ranking, filters (sender, conversation, group, server, channel, date range, type, attachment, image, video, link, pinned, bookmarked), search result cache, pagination, cancellation, coverage status, and cross-chunk lazy `jump_to_message`.
   - Encrypted search index shard sync across devices (zero-knowledge server storage).
7. **Mutations Over Compressed Chunks & Compatibility**:
   - Append-only `message_mutations` log + tombstones supporting edit, recall, delete, pin, reaction, bookmark over immutable compressed chunks without rewriting full history; bounded compaction when dead-ratio exceeds threshold; full backward compatibility with existing ChatW features and `.kdata` backup/restore.
8. **Minimalist Storage Settings UI & PWA Offline Access**:
   - Consistent flat minimalist UI inside existing `StorageSettingsPanel` (total/messages/media/files/cache/offline/search-index/saved breakdown, 4 storage modes, safe clear cache) and real message search in `SearchPage` + in-chat details panels.
   - Offline App Shell caching in `sw.js` + `manifest.json` + persistent multi-scope caching so cached conversations and local search work offline.
9. **Verification, Benchmarks & Security Audits**:
   - Real multi-scale benchmarks (1K, 10K, 100K, 1M+ messages; chunk sizes 64KB..16MB; eviction algorithms LRU/LFU/TinyLFU/W-TinyLFU/SLRU/Weighted; serialization vs raw vs Zstd-22 vs Zstd-22+dict vs padded+encrypted; search latency P50/P95/P99).
   - Comprehensive failure-mode tests (network drop, crash recovery, quota exceeded, corrupted chunk/dict/index, compression bomb, replay/rollback/tamper, cross-account isolation, concurrent multi-device leases, edit/delete during compression/search, logout wipe, migration rollback).
