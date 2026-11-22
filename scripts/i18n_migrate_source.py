#!/usr/bin/env python3
"""
Phase 2 (Spec 064): Migrate all Vietnamese UI string literals across app/src/**/*.rs
and user-facing web/backend/src/**/*.rs to 100% canonical English using the verified
scripts/i18n_work/vi_to_en.json catalog.
"""
import glob
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from i18n_extract_and_en import (
    ASCII_VI_OVERRIDES,
    VI_RE,
    is_sql_query,
    lex_rust_string_literals,
    split_rust_placeholders,
    unescape_rust,
)

# Polish any awkward automated English strings before writing to source code
EN_POLISH_OVERRIDES = {
    "Tự động chỉnh vừa mắt hoặc chọn kích thước bên dưới": "Auto-scale for comfortable reading or choose a size below",
    "Chìa khóa bảo mật chỉ lưu trên máy này và không gửi lên mạng. Bạn có thể lưu lại để chuyển sang máy khác.": "Your security key is stored only on this device and never sent over the network. You can back it up to transfer to another device.",
    "Kéo để thay đổi kích thước danh mục": "Drag to resize sidebar",
    "Không thể xử lý tệp ảnh": "Unable to process image file",
    "Không thể xử lý ảnh bìa": "Unable to process banner image",
    "Không thể cập nhật ảnh đại diện": "Unable to update avatar",
    "Không thể tải ảnh bìa": "Unable to upload banner image",
    "Đã lưu ảnh bìa hồ sơ": "Profile banner saved",
    "Thoát tài khoản khỏi máy này. Tin nhắn của bạn vẫn an toàn.": "Sign out of this device. Your messages remain safe.",
    "Áp dụng cho nút chính, bong bóng tin nhắn đã gửi và mục đang chọn.": "Applies to primary buttons, sent bubbles, and selected items.",
    "Mã hóa hai đầu chuẩn cao nhất": "Highest-grade end-to-end encryption",
}


def escape_rust_string(s: str) -> str:
    out = []
    for ch in s:
        if ch == "\\":
            out.append("\\\\")
        elif ch == '"':
            out.append('\\"')
        elif ch == "\n":
            out.append("\\n")
        elif ch == "\r":
            out.append("\\r")
        elif ch == "\t":
            out.append("\\t")
        else:
            out.append(ch)
    return "".join(out)


def restore_slots(en_template: str, slots: list) -> str:
    res = en_template
    # Replace placeholders carefully via unique temporary tokens to avoid collision if a slot contains {1}
    for i in range(len(slots)):
        res = res.replace(f"{{{i}}}", f"\x00SLOT{i}\x00")
    for i, span in enumerate(slots):
        res = res.replace(f"\x00SLOT{i}\x00", span)
    return res


def migrate_file(path: str, vi_to_en: dict) -> int:
    src = open(path, encoding="utf-8").read()
    edits = []
    for start, end, inner, is_raw, hashes in lex_rust_string_literals(src):
        if is_raw:
            continue
        val = unescape_rust(inner)
        if is_sql_query(val):
            continue
        # Preserve messages.rs date formatter lines handled manually with English month names
        if path == "app/src/components/messages.rs" and "tháng" in val:
            continue
        has_vi = bool(VI_RE.search(val))
        is_ascii_vi = val in ASCII_VI_OVERRIDES
        if not has_vi and not is_ascii_vi:
            continue
        norm, slots = split_rust_placeholders(val)
        en_tpl = EN_POLISH_OVERRIDES.get(norm) or vi_to_en.get(norm)
        if not en_tpl:
            print(f"WARNING missing mapping in {path}: {norm!r}")
            continue
        en_val = restore_slots(en_tpl, slots)
        replacement = '"' + escape_rust_string(en_val) + '"'
        edits.append((start, end, replacement))

    if not edits:
        return 0

    out = []
    last = 0
    for start, end, repl in edits:
        out.append(src[last:start])
        out.append(repl)
        last = end
    out.append(src[last:])
    with open(path, "w", encoding="utf-8") as fp:
        fp.write("".join(out))
    return len(edits)


def main():
    data = json.load(open("scripts/i18n_work/vi_to_en.json", encoding="utf-8"))
    vi_to_en = data["vi_to_en"]
    vi_to_en.update(EN_POLISH_OVERRIDES)
    # Persist polished mapping back into vi_to_en.json so Phase 4 locale generator uses exact same English keys!
    data["vi_to_en"] = vi_to_en
    with open("scripts/i18n_work/vi_to_en.json", "w", encoding="utf-8") as fp:
        json.dump(data, fp, ensure_ascii=False, indent=2)

    skip_files = {
        "app/src/i18n.rs",
        "app/src/components/icons.rs",
        "app/src/components/brand_logos.rs",
        "app/src/components/emoji_data.rs",
        "web/backend/src/users/mod.rs",
    }
    targets = [
        f
        for pat in ("app/src/**/*.rs", "web/backend/src/**/*.rs")
        for f in sorted(glob.glob(pat, recursive=True))
        if f not in skip_files
    ]

    total_files = 0
    total_edits = 0
    for f in targets:
        n = migrate_file(f, vi_to_en)
        if n > 0:
            total_files += 1
            total_edits += n
    print(f"Migrated {total_edits} string literals across {total_files} Rust files to canonical English!")


if __name__ == "__main__":
    main()
