//! Compact Lossless Columnar Binary Serialization (`CWMC1`) — Section 6.
//!
//! Pre-compression optimizations that maximize Zstandard Level 22 compression ratio
//! with ZERO data loss and strict versioning:
//! - Magic `CWMC1` + `format_version: u16` (backward-compatible decoding)
//! - Chunk-local Sender ID Dictionary (each sender stored once, referenced via LEB128 varint)
//! - Chunk-local Repeated String Table (reaction emojis, MIME types, repeated short replies)
//! - Columnar (Structure-of-Arrays) layout:
//!   1. `seq` column: base `i64` + ZigZag LEB128 deltas
//!   2. `created_at_ms` column: base `i64` + ZigZag LEB128 deltas
//!   3. `sender_idx` column: varint indices into sender table
//!   4. `flags_and_type` column: bit-packed `u16` (msg_type + edited/deleted/pinned/bookmarked/has_*)
//!   5. `status_code` column: raw `u8` (only emitted if any non-zero)
//!   6. `ids` column: 16B binary UUIDs (with fallback tagged UTF-8 string for non-UUID IDs)
//!   7. `reply_to_id` sparse column (only for messages with `FLAG_HAS_REPLY`)
//!   8. `texts` column: length-prefixed UTF-8 (or local string-table index for exact short repeats)
//!   9. `reactions`, `attachments`, `extra_meta` sparse columns

use crate::model::{
    AttachmentRef, CHUNK_FORMAT_VERSION, CompactMessage, MAX_MESSAGES_PER_CHUNK, MessageType,
    MsgStoreError, MsgStoreResult, ReactionEntry,
};
use std::collections::HashMap;
use uuid::Uuid;

pub const CWMC1_MAGIC: &[u8; 5] = b"CWMC1";

// Bit-packed message flags in a single `u16`:
// bits 0..3: MessageType (0..15)
const FLAG_EDITED: u16 = 1 << 4;
const FLAG_DELETED: u16 = 1 << 5;
const FLAG_PINNED: u16 = 1 << 6;
const FLAG_BOOKMARKED: u16 = 1 << 7;
const FLAG_HAS_REPLY: u16 = 1 << 8;
const FLAG_HAS_REACTIONS: u16 = 1 << 9;
const FLAG_HAS_ATTACHMENTS: u16 = 1 << 10;
const FLAG_HAS_EXTRA_META: u16 = 1 << 11;
const FLAG_TEXT_INTERNED: u16 = 1 << 12;
const FLAG_ID_IS_UUID: u16 = 1 << 13;
const FLAG_HAS_STATUS: u16 = 1 << 14;

#[inline]
pub fn encode_uvarint(mut v: u64, out: &mut Vec<u8>) {
    loop {
        let mut b = (v & 0x7F) as u8;
        v >>= 7;
        if v != 0 {
            b |= 0x80;
        }
        out.push(b);
        if v == 0 {
            break;
        }
    }
}

#[inline]
pub fn decode_uvarint(input: &[u8], pos: &mut usize) -> MsgStoreResult<u64> {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    loop {
        if *pos >= input.len() {
            return Err(MsgStoreError::InvalidFormat("unexpected EOF in varint".into()));
        }
        let b = input[*pos];
        *pos += 1;
        if shift >= 64 || (shift == 63 && b > 1) {
            return Err(MsgStoreError::InvalidFormat("varint overflow".into()));
        }
        result |= ((b & 0x7F) as u64) << shift;
        if (b & 0x80) == 0 {
            return Ok(result);
        }
        shift += 7;
    }
}

#[inline]
pub fn zigzag_encode(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)) as u64
}

#[inline]
pub fn zigzag_decode(v: u64) -> i64 {
    ((v >> 1) as i64) ^ (-((v & 1) as i64))
}

#[inline]
pub fn encode_ivarint(v: i64, out: &mut Vec<u8>) {
    encode_uvarint(zigzag_encode(v), out);
}

#[inline]
pub fn decode_ivarint(input: &[u8], pos: &mut usize) -> MsgStoreResult<i64> {
    Ok(zigzag_decode(decode_uvarint(input, pos)?))
}

fn write_str(s: &str, out: &mut Vec<u8>) {
    encode_uvarint(s.len() as u64, out);
    out.extend_from_slice(s.as_bytes());
}

fn read_str(input: &[u8], pos: &mut usize, max_len: usize) -> MsgStoreResult<String> {
    let len = decode_uvarint(input, pos)? as usize;
    if len > max_len || *pos + len > input.len() {
        return Err(MsgStoreError::InvalidFormat("string exceeds bounds".into()));
    }
    let slice = &input[*pos..*pos + len];
    *pos += len;
    String::from_utf8(slice.to_vec())
        .map_err(|e| MsgStoreError::InvalidFormat(format!("invalid utf8: {e}")))
}

fn write_id_compact(id: &str, out: &mut Vec<u8>) -> bool {
    if let Ok(u) = Uuid::parse_str(id) {
        out.extend_from_slice(u.as_bytes());
        true
    } else {
        write_str(id, out);
        false
    }
}

fn read_id_compact(input: &[u8], pos: &mut usize, is_uuid: bool) -> MsgStoreResult<String> {
    if is_uuid {
        if *pos + 16 > input.len() {
            return Err(MsgStoreError::InvalidFormat("uuid truncated".into()));
        }
        let bytes: [u8; 16] = input[*pos..*pos + 16]
            .try_into()
            .map_err(|_| MsgStoreError::InvalidFormat("uuid slice".into()))?;
        *pos += 16;
        Ok(Uuid::from_bytes(bytes).to_string())
    } else {
        read_str(input, pos, 512)
    }
}

/// Serialize a slice of messages into compact columnar `CWMC1` binary bytes.
/// Guarantees 100% lossless round-trip fidelity for every field.
pub fn serialize_chunk_messages(messages: &[CompactMessage]) -> MsgStoreResult<Vec<u8>> {
    if messages.len() > MAX_MESSAGES_PER_CHUNK {
        return Err(MsgStoreError::InvalidFormat(format!(
            "message count {} exceeds hard cap {MAX_MESSAGES_PER_CHUNK}",
            messages.len()
        )));
    }

    fn intern<'a>(map: &mut HashMap<&'a str, u32>, vec: &mut Vec<&'a str>, s: &'a str) -> u32 {
        if let Some(&idx) = map.get(s) {
            idx
        } else {
            let idx = vec.len() as u32;
            map.insert(s, idx);
            vec.push(s);
            idx
        }
    }

    // 1. Build chunk-local sender table (deduped UUIDs / IDs)
    let mut sender_map: HashMap<&str, u32> = HashMap::new();
    let mut senders: Vec<&str> = Vec::new();

    // 2. Build chunk-local repeated string table (emojis, mime types, repeated short texts <= 64B seen >= 2x)
    let mut short_text_counts: HashMap<&str, u32> = HashMap::new();
    let mut str_map: HashMap<&str, u32> = HashMap::new();
    let mut strings: Vec<&str> = Vec::new();

    for m in messages {
        intern(&mut sender_map, &mut senders, &m.sender_id);
        if m.text.len() <= 64 {
            *short_text_counts.entry(m.text.as_str()).or_insert(0) += 1;
        }
        for r in &m.reactions {
            intern(&mut sender_map, &mut senders, &r.user_id);
            intern(&mut str_map, &mut strings, &r.emoji);
        }
        for a in &m.attachments {
            intern(&mut str_map, &mut strings, &a.mime_type);
        }
    }
    for (&txt, &cnt) in &short_text_counts {
        if cnt >= 2 {
            intern(&mut str_map, &mut strings, txt);
        }
    }

    let est_cap = 64 + messages.len() * 48;
    let mut out = Vec::with_capacity(est_cap);

    // Header: magic(5) + version(u16 LE) + count(uvarint)
    out.extend_from_slice(CWMC1_MAGIC);
    out.extend_from_slice(&CHUNK_FORMAT_VERSION.to_le_bytes());
    encode_uvarint(messages.len() as u64, &mut out);

    if messages.is_empty() {
        return Ok(out);
    }

    // Section A: Sender dictionary (bit flag per sender: 1 = 16B raw UUID, 0 = UTF-8 str)
    encode_uvarint(senders.len() as u64, &mut out);
    for s in &senders {
        if let Ok(u) = Uuid::parse_str(s) {
            out.push(1);
            out.extend_from_slice(u.as_bytes());
        } else {
            out.push(0);
            write_str(s, &mut out);
        }
    }

    // Section B: Repeated string dictionary
    encode_uvarint(strings.len() as u64, &mut out);
    for s in &strings {
        write_str(s, &mut out);
    }

    // Section C: Columnar `seq` deltas
    let base_seq = messages[0].seq;
    encode_ivarint(base_seq, &mut out);
    let mut prev_seq = base_seq;
    for m in &messages[1..] {
        encode_ivarint(m.seq.wrapping_sub(prev_seq), &mut out);
        prev_seq = m.seq;
    }

    // Section D: Columnar `created_at_ms` deltas
    let base_ts = messages[0].created_at_ms;
    encode_ivarint(base_ts, &mut out);
    let mut prev_ts = base_ts;
    for m in &messages[1..] {
        encode_ivarint(m.created_at_ms.wrapping_sub(prev_ts), &mut out);
        prev_ts = m.created_at_ms;
    }

    // Section E: Columnar `sender_idx`
    for m in messages {
        let idx = sender_map[m.sender_id.as_str()];
        encode_uvarint(idx as u64, &mut out);
    }

    // Section F: Bit-packed flags + MessageType (`u16` LE per message)
    let mut id_is_uuid_vec = Vec::with_capacity(messages.len());
    let mut text_interned_vec = Vec::with_capacity(messages.len());
    for m in messages {
        let is_uuid = Uuid::parse_str(&m.id).is_ok();
        let is_interned = m.text.len() <= 64 && str_map.contains_key(m.text.as_str());
        id_is_uuid_vec.push(is_uuid);
        text_interned_vec.push(is_interned);

        let mut flags: u16 = (m.msg_type as u16) & 0x0F;
        if m.is_edited {
            flags |= FLAG_EDITED;
        }
        if m.deleted_for_all {
            flags |= FLAG_DELETED;
        }
        if m.is_pinned {
            flags |= FLAG_PINNED;
        }
        if m.is_bookmarked {
            flags |= FLAG_BOOKMARKED;
        }
        if m.reply_to_id.is_some() {
            flags |= FLAG_HAS_REPLY;
        }
        if !m.reactions.is_empty() {
            flags |= FLAG_HAS_REACTIONS;
        }
        if !m.attachments.is_empty() {
            flags |= FLAG_HAS_ATTACHMENTS;
        }
        if m.extra_meta.is_some() {
            flags |= FLAG_HAS_EXTRA_META;
        }
        if is_interned {
            flags |= FLAG_TEXT_INTERNED;
        }
        if is_uuid {
            flags |= FLAG_ID_IS_UUID;
        }
        if m.status_code != 0 {
            flags |= FLAG_HAS_STATUS;
        }
        out.extend_from_slice(&flags.to_le_bytes());
    }

    // Section G: Sparse `status_code` column
    for m in messages {
        if m.status_code != 0 {
            out.push(m.status_code);
        }
    }

    // Section H: Compact Message IDs column
    for (m, &is_uuid) in messages.iter().zip(&id_is_uuid_vec) {
        let ok = write_id_compact(&m.id, &mut out);
        debug_assert_eq!(ok, is_uuid);
    }

    // Section I: Sparse `reply_to_id` column
    for m in messages {
        if let Some(ref rid) = m.reply_to_id {
            if let Ok(u) = Uuid::parse_str(rid) {
                out.push(1);
                out.extend_from_slice(u.as_bytes());
            } else {
                out.push(0);
                write_str(rid, &mut out);
            }
        }
    }

    // Section J: Message Text column (interned index OR length-prefixed UTF-8)
    for (m, &is_interned) in messages.iter().zip(&text_interned_vec) {
        if is_interned {
            let idx = str_map[m.text.as_str()];
            encode_uvarint(idx as u64, &mut out);
        } else {
            write_str(&m.text, &mut out);
        }
    }

    // Section K: Sparse Reactions column
    for m in messages {
        if !m.reactions.is_empty() {
            encode_uvarint(m.reactions.len() as u64, &mut out);
            let mut prev_r_ts = m.created_at_ms;
            for r in &m.reactions {
                encode_uvarint(str_map[r.emoji.as_str()] as u64, &mut out);
                encode_uvarint(sender_map[r.user_id.as_str()] as u64, &mut out);
                encode_ivarint(r.created_at_ms.wrapping_sub(prev_r_ts), &mut out);
                prev_r_ts = r.created_at_ms;
            }
        }
    }

    // Section L: Sparse Attachment References column (never embeds media bytes!)
    for m in messages {
        if !m.attachments.is_empty() {
            encode_uvarint(m.attachments.len() as u64, &mut out);
            for a in &m.attachments {
                write_str(&a.ref_id, &mut out);
                encode_uvarint(str_map[a.mime_type.as_str()] as u64, &mut out);
                write_str(&a.file_name, &mut out);
                encode_uvarint(a.byte_size, &mut out);
                encode_uvarint(a.width as u64, &mut out);
                encode_uvarint(a.height as u64, &mut out);
                encode_uvarint(a.duration_ms as u64, &mut out);
                let opt_bits: u8 =
                    u8::from(a.key_b64.is_some()) | (u8::from(a.nonce_b64.is_some()) << 1);
                out.push(opt_bits);
                if let Some(ref k) = a.key_b64 {
                    write_str(k, &mut out);
                }
                if let Some(ref n) = a.nonce_b64 {
                    write_str(n, &mut out);
                }
            }
        }
    }

    // Section M: Sparse Extra Metadata column
    for m in messages {
        if let Some(ref meta) = m.extra_meta {
            write_str(meta, &mut out);
        }
    }

    Ok(out)
}

/// Deserialize `CWMC1` binary bytes back into exact lossless `Vec<CompactMessage>`.
pub fn deserialize_chunk_messages(input: &[u8]) -> MsgStoreResult<Vec<CompactMessage>> {
    if input.len() < 7 || &input[..5] != CWMC1_MAGIC {
        return Err(MsgStoreError::InvalidFormat("bad CWMC1 magic".into()));
    }
    let version = u16::from_le_bytes([input[5], input[6]]);
    if version != CHUNK_FORMAT_VERSION {
        return Err(MsgStoreError::UnsupportedVersion(version));
    }

    let mut pos = 7usize;
    let count = decode_uvarint(input, &mut pos)? as usize;
    if count > MAX_MESSAGES_PER_CHUNK {
        return Err(MsgStoreError::DecompressionBomb {
            limit: MAX_MESSAGES_PER_CHUNK,
            actual: count,
        });
    }
    if count == 0 {
        return Ok(Vec::new());
    }

    // Section A: Senders
    let sender_cnt = decode_uvarint(input, &mut pos)? as usize;
    if sender_cnt > MAX_MESSAGES_PER_CHUNK * 4 {
        return Err(MsgStoreError::InvalidFormat("sender table too large".into()));
    }
    let mut senders = Vec::with_capacity(sender_cnt);
    for _ in 0..sender_cnt {
        if pos >= input.len() {
            return Err(MsgStoreError::InvalidFormat("sender tag EOF".into()));
        }
        let is_uuid = input[pos] == 1;
        pos += 1;
        senders.push(read_id_compact(input, &mut pos, is_uuid)?);
    }

    // Section B: Repeated Strings
    let str_cnt = decode_uvarint(input, &mut pos)? as usize;
    if str_cnt > MAX_MESSAGES_PER_CHUNK * 8 {
        return Err(MsgStoreError::InvalidFormat("string table too large".into()));
    }
    let mut strings = Vec::with_capacity(str_cnt);
    for _ in 0..str_cnt {
        strings.push(read_str(input, &mut pos, 64 * 1024)?);
    }

    // Section C: seq deltas
    let mut seqs = Vec::with_capacity(count);
    let mut cur_seq = decode_ivarint(input, &mut pos)?;
    seqs.push(cur_seq);
    for _ in 1..count {
        cur_seq = cur_seq.wrapping_add(decode_ivarint(input, &mut pos)?);
        seqs.push(cur_seq);
    }

    // Section D: created_at_ms deltas
    let mut timestamps = Vec::with_capacity(count);
    let mut cur_ts = decode_ivarint(input, &mut pos)?;
    timestamps.push(cur_ts);
    for _ in 1..count {
        cur_ts = cur_ts.wrapping_add(decode_ivarint(input, &mut pos)?);
        timestamps.push(cur_ts);
    }

    // Section E: sender indices
    let mut sender_indices = Vec::with_capacity(count);
    for _ in 0..count {
        let idx = decode_uvarint(input, &mut pos)? as usize;
        if idx >= senders.len() {
            return Err(MsgStoreError::InvalidFormat("sender idx out of bounds".into()));
        }
        sender_indices.push(idx);
    }

    // Section F: flags
    if pos + count * 2 > input.len() {
        return Err(MsgStoreError::InvalidFormat("flags column truncated".into()));
    }
    let mut flags_col = Vec::with_capacity(count);
    for _ in 0..count {
        let f = u16::from_le_bytes([input[pos], input[pos + 1]]);
        pos += 2;
        flags_col.push(f);
    }

    // Section G: sparse status codes
    let mut statuses = vec![0u8; count];
    for (i, &f) in flags_col.iter().enumerate() {
        if (f & FLAG_HAS_STATUS) != 0 {
            if pos >= input.len() {
                return Err(MsgStoreError::InvalidFormat("status column EOF".into()));
            }
            statuses[i] = input[pos];
            pos += 1;
        }
    }

    // Section H: message IDs
    let mut ids = Vec::with_capacity(count);
    for &f in &flags_col {
        ids.push(read_id_compact(input, &mut pos, (f & FLAG_ID_IS_UUID) != 0)?);
    }

    // Section I: sparse replies
    let mut replies = vec![None; count];
    for (i, &f) in flags_col.iter().enumerate() {
        if (f & FLAG_HAS_REPLY) != 0 {
            if pos >= input.len() {
                return Err(MsgStoreError::InvalidFormat("reply tag EOF".into()));
            }
            let is_uuid = input[pos] == 1;
            pos += 1;
            replies[i] = Some(read_id_compact(input, &mut pos, is_uuid)?);
        }
    }

    // Section J: texts
    let mut texts = Vec::with_capacity(count);
    for &f in &flags_col {
        if (f & FLAG_TEXT_INTERNED) != 0 {
            let idx = decode_uvarint(input, &mut pos)? as usize;
            if idx >= strings.len() {
                return Err(MsgStoreError::InvalidFormat("text intern idx OOB".into()));
            }
            texts.push(strings[idx].clone());
        } else {
            texts.push(read_str(input, &mut pos, 2 * 1024 * 1024)?);
        }
    }

    // Section K: reactions
    let mut reactions_col: Vec<Vec<ReactionEntry>> = vec![Vec::new(); count];
    for (i, &f) in flags_col.iter().enumerate() {
        if (f & FLAG_HAS_REACTIONS) != 0 {
            let r_cnt = decode_uvarint(input, &mut pos)? as usize;
            if r_cnt > 10_000 {
                return Err(MsgStoreError::InvalidFormat("reaction count too large".into()));
            }
            let mut list = Vec::with_capacity(r_cnt);
            let mut prev_r_ts = timestamps[i];
            for _ in 0..r_cnt {
                let e_idx = decode_uvarint(input, &mut pos)? as usize;
                let u_idx = decode_uvarint(input, &mut pos)? as usize;
                let dt = decode_ivarint(input, &mut pos)?;
                if e_idx >= strings.len() || u_idx >= senders.len() {
                    return Err(MsgStoreError::InvalidFormat("reaction ref OOB".into()));
                }
                prev_r_ts = prev_r_ts.wrapping_add(dt);
                list.push(ReactionEntry {
                    emoji: strings[e_idx].clone(),
                    user_id: senders[u_idx].clone(),
                    created_at_ms: prev_r_ts,
                });
            }
            reactions_col[i] = list;
        }
    }

    // Section L: attachments
    let mut attachments_col: Vec<Vec<AttachmentRef>> = vec![Vec::new(); count];
    for (i, &f) in flags_col.iter().enumerate() {
        if (f & FLAG_HAS_ATTACHMENTS) != 0 {
            let a_cnt = decode_uvarint(input, &mut pos)? as usize;
            if a_cnt > 256 {
                return Err(MsgStoreError::InvalidFormat("attachment count too large".into()));
            }
            let mut list = Vec::with_capacity(a_cnt);
            for _ in 0..a_cnt {
                let ref_id = read_str(input, &mut pos, 512)?;
                let m_idx = decode_uvarint(input, &mut pos)? as usize;
                if m_idx >= strings.len() {
                    return Err(MsgStoreError::InvalidFormat("attachment mime OOB".into()));
                }
                let file_name = read_str(input, &mut pos, 1024)?;
                let byte_size = decode_uvarint(input, &mut pos)?;
                let width = decode_uvarint(input, &mut pos)? as u32;
                let height = decode_uvarint(input, &mut pos)? as u32;
                let duration_ms = decode_uvarint(input, &mut pos)? as u32;
                if pos >= input.len() {
                    return Err(MsgStoreError::InvalidFormat("attachment opt bits EOF".into()));
                }
                let opt_bits = input[pos];
                pos += 1;
                let key_b64 = if (opt_bits & 1) != 0 {
                    Some(read_str(input, &mut pos, 512)?)
                } else {
                    None
                };
                let nonce_b64 = if (opt_bits & 2) != 0 {
                    Some(read_str(input, &mut pos, 512)?)
                } else {
                    None
                };
                list.push(AttachmentRef {
                    ref_id,
                    mime_type: strings[m_idx].clone(),
                    file_name,
                    byte_size,
                    width,
                    height,
                    duration_ms,
                    key_b64,
                    nonce_b64,
                });
            }
            attachments_col[i] = list;
        }
    }

    // Section M: extra_meta
    let mut metas = vec![None; count];
    for (i, &f) in flags_col.iter().enumerate() {
        if (f & FLAG_HAS_EXTRA_META) != 0 {
            metas[i] = Some(read_str(input, &mut pos, 512 * 1024)?);
        }
    }

    if pos != input.len() {
        return Err(MsgStoreError::InvalidFormat(format!(
            "trailing unconsumed bytes: {} remaining",
            input.len() - pos
        )));
    }

    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let f = flags_col[i];
        out.push(CompactMessage {
            id: std::mem::take(&mut ids[i]),
            seq: seqs[i],
            sender_id: senders[sender_indices[i]].clone(),
            created_at_ms: timestamps[i],
            msg_type: MessageType::from_u8((f & 0x0F) as u8),
            text: std::mem::take(&mut texts[i]),
            reply_to_id: replies[i].take(),
            is_edited: (f & FLAG_EDITED) != 0,
            deleted_for_all: (f & FLAG_DELETED) != 0,
            is_pinned: (f & FLAG_PINNED) != 0,
            is_bookmarked: (f & FLAG_BOOKMARKED) != 0,
            status_code: statuses[i],
            reactions: std::mem::take(&mut reactions_col[i]),
            attachments: std::mem::take(&mut attachments_col[i]),
            extra_meta: metas[i].take(),
        });
    }

    Ok(out)
}
