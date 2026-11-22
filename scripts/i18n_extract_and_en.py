#!/usr/bin/env python3
"""
Phase 1 (Spec 064): Extract 100% of Vietnamese UI string literals (plain + interpolated)
from app/src, app/web, and user-facing web/backend/src, and build a verified canonical
English <-> authentic Vietnamese mapping in scripts/i18n_work/vi_to_en.json.
"""
import concurrent.futures
import glob
import json
import os
import re
import sys
import time
import urllib.parse
import urllib.request

VI_RE = re.compile(
    r"[àáạảãâầấậẩẫăằắặẳẵèéẹẻẽêềếệểễìíịỉĩòóọỏõôồốộổỗơờớợởỡùúụủũưừứựửữỳýỵỷỹđ"
    r"ÀÁẠẢÃÂẦẤẬẨẪĂẰẮẶẲẴÈÉẸẺẼÊỀẾỆỂỄÌÍỊỈĨÒÓỌỎÕÔỒỐỘỔỖƠỜỚỢỞỠÙÚỤỦŨƯỪỨỰỬỮỲÝỴỶỸĐ]"
)

# ASCII-only Vietnamese UI literals confirmed by manual context audit
ASCII_VI_OVERRIDES = {
    "Cam": "Orange",
    "Cao": "High",
    "CH Play": "Google Play",
    "Dung sai": "Tolerance",
    "Ghim": "Pin",
    "Giao ngay": "Assign now",
    "Kinh doanh": "Business",
    "Qua Wi-Fi": "Wi-Fi only",
    "Tan ca": "Clock out",
    "Tham gia": "Join",
    "Tham gia ngay": "Join now",
    "Trang Kinh doanh": "Business Page",
    "Xem": "View",
    "Xem trang": "View page",
    "Xin tham gia": "Request to join",
    "Xoay": "Rotate",
    "Xong": "Done",
}

# Disambiguated / hand-curated overrides (higher priority than existing en.json & MT)
CURATED_VI_TO_EN = {
    "Xem thử": "Showcase",
    "Xem trước": "Preview",
    "Nhẹ": "Gentle",
    "Mềm": "Soft",
    "Vào ca": "Clock in",
    "Tắt khóa": "Disable lock",
    "Tên đăng nhập": "Username",
    "Chỉnh sửa": "Edit",
    "Không lưu": "Not stored",
    "Không yêu cầu": "Not required",
    "Phương thức đăng nhập": "Sign-in method",
    "Đã đăng nhập thiết bị": "Signed-in devices",
    "Tên tài khoản": "Account name",
    "Mã danh": "Identifier",
    "Đổi ảnh bìa": "Change banner",
    "Đổi ảnh đại diện": "Change avatar",
    "Tài liệu bảo mật": "Security guide",
    "Trò chuyện riêng tư sẵn sàng": "Private chat ready",
    "Mọi tin nhắn trực tiếp đều được mã hóa đầu cuối trên thiết bị của bạn.": "All direct messages are end-to-end encrypted on your device.",
    "Thêm bạn bè": "Add friends",
    "Tìm theo tên tài khoản để nhắn tin ngay": "Find by username to message right away",
    "Tạo nhóm trò chuyện": "Create group chat",
    "Trò chuyện và gọi nhóm nhiều người": "Chat and group call with multiple people",
    "Tùy biến giao diện": "Customize appearance",
    "Đổi chủ đề màu, hình nền và hiệu ứng": "Change color themes, wallpapers, and effects",
    "Hướng dẫn người mới": "Getting started guide",
    "Xem giới thiệu nhanh các tính năng nổi bật": "Take a quick tour of key features",
    "Chia sẻ suy nghĩ": "Share a thought",
    "Đăng tin": "Post story",
    "Đã bình chọn": "Voted",
    "Đã gửi một sticker": "Sent a sticker",
    "Tháng {0}, {1}": "{0}/{1}",
    "Mã màu": "Color hex",
    "Hôm nay": "Today",
    "Giờ": "Hour",
    "Phút": "Minute",
    "Bây giờ": "Now",
    "Thiết bị": "Devices",
    "Bạn": "You",
    "Không xác định": "Unknown",
    "Quản trị viên": "Administrator",
    "Đang mở": "Open",
    "Hoàn thành": "Completed",
    "Tất cả": "All",
    "Cài đặt": "Settings",
    "Ngôn ngữ": "Language",
    "Giao diện": "Appearance",
    "Đám mây": "Cloud",
    "Trò chuyện": "Chat",
    "Đồng bộ": "Sync",
    "Sao lưu": "Backup",
    "Lưu trữ": "Storage",
    "Tài khoản": "Account",
    "Quyền riêng tư": "Privacy",
    "Bảo mật": "Security",
    "Khóa ứng dụng": "App lock",
    "Thông báo": "Notifications",
    "Trả lời tự động": "Auto reply",
    "Tùy chỉnh": "Customize",
    "Thư viện": "Library",
    "Vẽ hình": "Draw",
    "Kho đồ": "Inventory",
    "Tiện ích": "Extensions",
    "Hướng dẫn": "Guide",
    "Chính mình": "Note to self",
    "Ghi chú cho chính mình": "Note to self",
    "Tin nhắn": "Messages",
    "Bạn bè": "Friends",
    "Cộng đồng": "Communities",
    "Họp nhóm": "Meetings",
    "Mới": "Feed",
    "Tìm kiếm": "Search",
    "Tìm cài đặt": "Search settings",
    "Tìm cuộc trò chuyện": "Search conversations",
    "Xác nhận": "Confirm",
    "Hủy": "Cancel",
    "Đóng": "Close",
    "Lưu": "Save",
    "Xóa": "Delete",
    "Gửi": "Send",
    "Sao chép": "Copy",
    "Đã sao chép": "Copied",
    "Tải ảnh của bạn": "Upload your image",
    "Khôi phục giao diện mặc định": "Reset default appearance",
    "Độ trong suốt khung trắng: {0}%": "Panel transparency: {0}%",
    "Mức độ làm mờ kính: {0}px": "Glass blur level: {0}px",
    "Độ tối phủ lên ảnh: {0}%": "Wallpaper dim: {0}%",
    "Làm mờ hình nền: {0}px": "Wallpaper blur: {0}px",
    "Phiên bản đồng bộ: {0}": "Sync version: {0}",
    "{0} tháng {1} năm {2}": "{0} month {1}, {2}",
    "{0} tháng {1}": "{0}/{1}",
    "{0} lượt bình chọn": "{0} votes",
    "Nhắn tin tới {0}": "Message {0}",
    "{0} đang soạn tin": "{0} is typing",
    "Chỉ mình bạn · {0} thiết bị": "Only you · {0} devices",
    "Bước {0} / {1}": "Step {0} / {1}",
    "{0} phút": "{0} min",
    "{0} giờ": "{0} hr",
    "{0} ngày": "{0} days",
    "Kết quả xúc xắc của {0}: {1}": "{0} rolled a die: {1}",
    "Bỏ qua giới thiệu": "Skip tour",
    "Tên tài khoản thử nghiệm": "Test username",
    "Tùy chọn khác": "More options",
    "Tạo bình chọn": "Create poll",
    "Tạo ghi chú": "Create note",
    "Tạo nhắc hẹn": "Create reminder",
    "Bình chọn": "Poll",
    "Đêm sao": "Starry night",
    "Đại dương": "Ocean",
    "Hoàng hôn": "Sunset",
    "Cực quang": "Aurora",
    "Bạc hà": "Mint",
    "Chấm bi": "Dots",
    "Bong bóng": "Bubble",
    "Giấy": "Paper",
    "Không": "None",
    "Sáng": "Light",
    "Xám tro": "Ash",
    "Tối": "Dark",
    "Đen huyền": "Obsidian",
    "Theo hệ thống": "System",
    "Xanh dương": "Blue",
    "Tím": "Violet",
    "Xanh lá": "Green",
    "Hồng": "Rose",
    "Xanh ngọc": "Cyan",
    "Gọn": "Compact",
    "Vừa": "Normal",
    "Rộng": "Wide",
    "Hẹp": "Narrow",
    "Toàn màn hình": "Full width",
    "Tròn": "Round",
    "Vuông": "Square",
    "Chiếc lá": "Leaf",
    "Bo góc mềm": "Squircle",
    "Hệ thống": "System",
    "Bo tròn": "Rounded",
    "Có chân": "Serif",
    "Đơn cách": "Monospace",
    "Dễ đọc": "Readable",
    "Chữ và màu nền tự cân bằng rất dễ đọc.": "Text and background auto-balance for easy reading.",
    "Đẹp và rõ nét ở mọi kiểu giao diện.": "Crisp and clear in every theme style.",
    "Màu tùy chọn": "Custom color",
    "Mặc định": "Default",
    "Tìm kiếm ngôn ngữ": "Search languages",
}
CURATED_VI_TO_EN.update(ASCII_VI_OVERRIDES)


def lex_rust_string_literals(src: str):
    """
    Accurate Rust string literal lexer.
    Yields (start_idx, end_idx, raw_inner_content, is_raw, hashes) for double-quoted
    string literals outside comments, char literals, and byte strings.
    """
    n = len(src)
    i = 0
    while i < n:
        c = src[i]
        # Line comment
        if c == "/" and i + 1 < n and src[i + 1] == "/":
            i += 2
            while i < n and src[i] != "\n":
                i += 1
            continue
        # Block comment (nested)
        if c == "/" and i + 1 < n and src[i + 1] == "*":
            depth = 1
            i += 2
            while i < n and depth > 0:
                if src[i] == "/" and i + 1 < n and src[i + 1] == "*":
                    depth += 1
                    i += 2
                elif src[i] == "*" and i + 1 < n and src[i + 1] == "/":
                    depth -= 1
                    i += 2
                else:
                    i += 1
            continue
        # Byte string b"..." or br#"..."#
        if c == "b" and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_")):
            if i + 1 < n and src[i + 1] == '"':
                i += 2
                while i < n:
                    if src[i] == "\\":
                        i += 2
                    elif src[i] == '"':
                        i += 1
                        break
                    else:
                        i += 1
                continue
            if i + 1 < n and src[i + 1] == "r":
                j = i + 2
                while j < n and src[j] == "#":
                    j += 1
                if j < n and src[j] == '"':
                    hashes = j - (i + 2)
                    end_delim = '"' + ("#" * hashes)
                    k = src.find(end_delim, j + 1)
                    i = (k + len(end_delim)) if k != -1 else n
                    continue
        # Raw string r"..." or r#"..."#
        if c == "r" and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_")):
            j = i + 1
            while j < n and src[j] == "#":
                j += 1
            if j < n and src[j] == '"':
                hashes = j - (i + 1)
                end_delim = '"' + ("#" * hashes)
                k = src.find(end_delim, j + 1)
                if k != -1:
                    yield (i, k + len(end_delim), src[j + 1 : k], True, hashes)
                    i = k + len(end_delim)
                    continue
        # Char literal vs lifetime ('a vs 'x' or '\n')
        if c == "'":
            if i + 2 < n and src[i + 1] == "\\":
                k = i + 3
                while k < n and src[k] != "'" and k < i + 12:
                    k += 1
                if k < n and src[k] == "'":
                    i = k + 1
                    continue
            elif i + 2 < n and src[i + 2] == "'":
                i += 3
                continue
            i += 1
            continue
        # Normal double-quoted string literal
        if c == '"':
            start = i
            i += 1
            buf = []
            while i < n:
                ch = src[i]
                if ch == "\\":
                    if i + 1 < n:
                        buf.append(src[i : i + 2])
                        i += 2
                        continue
                elif ch == '"':
                    i += 1
                    yield (start, i, "".join(buf), False, 0)
                    break
                buf.append(ch)
                i += 1
            continue
        i += 1


def unescape_rust(raw: str) -> str:
    # Unescape standard Rust escapes we care about for UI text
    out = []
    i = 0
    n = len(raw)
    while i < n:
        if raw[i] == "\\" and i + 1 < n:
            nxt = raw[i + 1]
            if nxt == '"':
                out.append('"')
                i += 2
            elif nxt == "\\":
                out.append("\\")
                i += 2
            elif nxt == "n":
                out.append("\n")
                i += 2
            elif nxt == "t":
                out.append("\t")
                i += 2
            elif nxt == "r":
                out.append("\r")
                i += 2
            else:
                out.append(raw[i : i + 2])
                i += 2
        else:
            out.append(raw[i])
            i += 1
    return "".join(out)


def split_rust_placeholders(s: str):
    """
    Replace Rust format/RSX interpolation spans `{...}` with positional `{0}`, `{1}`, ...
    Properly respects escaped `{{` and `}}`, nested braces, and quotes inside expressions.
    Returns (normalized_template, list_of_original_spans).
    """
    out = []
    slots = []
    i = 0
    n = len(s)
    while i < n:
        if s[i] == "{":
            if i + 1 < n and s[i + 1] == "{":
                out.append("{{")
                i += 2
                continue
            # Find matching closing brace
            j = i + 1
            depth = 1
            in_str = False
            while j < n and depth > 0:
                ch = s[j]
                if in_str:
                    if ch == "\\":
                        j += 2
                        continue
                    elif ch == '"':
                        in_str = False
                else:
                    if ch == '"':
                        in_str = True
                    elif ch == "{":
                        depth += 1
                    elif ch == "}":
                        depth -= 1
                j += 1
            if depth == 0:
                span = s[i:j]
                idx = len(slots)
                slots.append(span)
                out.append(f"{{{idx}}}")
                i = j
                continue
        elif s[i] == "}" and i + 1 < n and s[i + 1] == "}":
            out.append("}}")
            i += 2
            continue
        out.append(s[i])
        i += 1
    return ("".join(out), slots)


def is_sql_query(s: str) -> bool:
    u = s.strip().upper()
    return u.startswith(("SELECT ", "INSERT ", "UPDATE ", "DELETE ", "CREATE ", "ALTER ", "WITH "))


def translate_batch(texts, sl="vi", tl="en"):
    """Batch translate list of strings using newline-safe sentinel escaping."""
    if not texts:
        return []
    # Escape internal newlines & tabs with tokens that survive MT
    encoded = [t.replace("\n", " __NL__ ").replace("\t", " __TAB__ ") for t in texts]
    payload = "\n".join(encoded)
    url = (
        f"https://translate.googleapis.com/translate_a/single"
        f"?client=gtx&sl={urllib.parse.quote(sl)}&tl={urllib.parse.quote(tl)}&dt=t"
    )
    data = urllib.parse.urlencode({"q": payload}).encode("utf-8")
    for attempt in range(5):
        try:
            req = urllib.request.Request(
                url,
                data=data,
                headers={
                    "User-Agent": "Mozilla/5.0",
                    "Content-Type": "application/x-www-form-urlencoded",
                },
            )
            with urllib.request.urlopen(req, timeout=20) as r:
                resp = json.loads(r.read().decode("utf-8"))
            joined = "".join(seg[0] for seg in resp[0] if seg and seg[0])
            parts = joined.split("\n")
            if len(parts) == len(texts):
                return [
                    p.replace(" __NL__ ", "\n").replace("__NL__", "\n").replace(" __TAB__ ", "\t").replace("__TAB__", "\t").strip()
                    for p in parts
                ]
            # Fallback: line mismatch -> bisect batch recursively!
            if len(texts) == 1:
                return [joined.replace(" __NL__ ", "\n").replace("__NL__", "\n").strip()]
            mid = len(texts) // 2
            return translate_batch(texts[:mid], sl, tl) + translate_batch(texts[mid:], sl, tl)
        except Exception as e:
            if attempt == 4:
                raise RuntimeError(f"Translation failed for batch ({len(texts)}): {e}")
            time.sleep(0.8 * (attempt + 1))


def clean_en_output(orig_vi: str, tr_en: str, slot_count: int) -> str:
    s = tr_en.strip()
    # Preserve leading/trailing spaces of original UI fragments like " · Project: {0}"
    lead = orig_vi[: len(orig_vi) - len(orig_vi.lstrip(" "))]
    trail = orig_vi[len(orig_vi.rstrip(" ")) :]
    # Remove any forbidden "..." ellipsis (AGENTS.md Rule 6)
    s = s.replace("...", "").replace("…", "").strip()
    # Normalize spaced placeholders like "{ 0 }" -> "{0}"
    for k in range(slot_count):
        s = re.sub(rf"\{{\s*{k}\s*\}}", f"{{{k}}}", s)
    # Restore common brand/technical casing
    for bad, good in [
        ("chatw", "ChatW"),
        ("Chatw", "ChatW"),
        ("vico ai", "Vico AI"),
        ("Vico ai", "Vico AI"),
        ("e2ee", "E2EE"),
        ("webrtc", "WebRTC"),
        ("webdav", "WebDAV"),
        ("github", "GitHub"),
        ("dropbox", "Dropbox"),
        ("onedrive", "OneDrive"),
        ("nextcloud", "Nextcloud"),
        ("google drive", "Google Drive"),
        ("google", "Google"),
    ]:
        s = re.sub(rf"\b{re.escape(bad)}\b", good, s)
    return f"{lead}{s}{trail}"


def main():
    os.makedirs("scripts/i18n_work", exist_ok=True)

    # Load existing hand-curated en.json (Spec 062)
    existing_en = {}
    if os.path.exists("app/web/locales/en.json"):
        raw_en = json.load(open("app/web/locales/en.json", encoding="utf-8"))
        existing_en.update(raw_en.get("exact", {}))
        existing_en.update(raw_en.get("templates", {}))

    # Merge curated overrides on top of existing_en
    curated = dict(existing_en)
    curated.update(CURATED_VI_TO_EN)

    # 1. Scan all frontend Rust files (except i18n.rs, icons.rs, brand_logos.rs, emoji_data.rs)
    skip_files = {
        "app/src/i18n.rs",
        "app/src/components/icons.rs",
        "app/src/components/brand_logos.rs",
        "app/src/components/emoji_data.rs",
    }
    rust_files = [
        f for f in sorted(glob.glob("app/src/**/*.rs", recursive=True))
        if f not in skip_files
    ]
    # Also include user-facing backend files (excluding users/mod.rs test fixture)
    backend_files = [
        f for f in sorted(glob.glob("web/backend/src/**/*.rs", recursive=True))
        if f != "web/backend/src/users/mod.rs"
    ]

    extracted = {}  # normalized_vi -> { "slots_examples": [...], "files": set(), "is_template": bool }

    for f in rust_files + backend_files:
        src = open(f, encoding="utf-8").read()
        for start, end, inner, is_raw, hashes in lex_rust_string_literals(src):
            if is_raw:
                continue  # Raw strings are JS/CSS/SVG/HTML snippets; handled explicitly
            val = unescape_rust(inner)
            if is_sql_query(val):
                continue
            has_vi = bool(VI_RE.search(val))
            is_ascii_vi = val in ASCII_VI_OVERRIDES
            if not has_vi and not is_ascii_vi:
                continue
            norm, slots = split_rust_placeholders(val)
            info = extracted.setdefault(norm, {"slot_count": len(slots), "files": set(), "samples": []})
            info["files"].add(f)
            if len(info["samples"]) < 2:
                info["samples"].append(val)

    # Also include JS strings from custom_pickers.js & plugin_runtime.js
    js_vi_strings = [
        "Mã màu", "Tháng {0}, {1}", "Xóa", "Hôm nay", "Giờ", "Phút", "Bây giờ",
        "Đã đọc tin nhắn giải mã cho cuộc trò chuyện {0}",
        "E2EE_EGRESS_DENIED: plugin chỉ được xử lý E2EE cục bộ",
        "Khởi chạy sandbox ({0}, origin=null) thành công",
        "Lỗi khởi chạy Plugin",
        "Quá thời gian khởi động sandbox ({0}ms)",
        "Đã dừng Plugin do treo khi khởi động",
        "Vượt quá giới hạn thực thi ({0}ms)",
        "Đã dừng Plugin do treo hoặc quá thời gian xử lý",
        "Plugin chưa cung cấp nội dung giao diện.",
        "Đã thu hồi quyền ngay lập tức ({0})",
        "Lỗi xử lý sự kiện {0}",
    ]
    for s in js_vi_strings:
        sc = len(re.findall(r"\{\d+\}", s))
        extracted.setdefault(s, {"slot_count": sc, "files": {"app/web"}, "samples": [s]})

    print(f"Total unique normalized Vietnamese strings extracted: {len(extracted)}")

    # Split into curated vs needs MT
    vi_to_en = {}
    to_translate = []
    for vi_norm in sorted(extracted.keys()):
        if vi_norm in curated:
            vi_to_en[vi_norm] = curated[vi_norm]
        elif vi_norm.strip() in curated:
            lead = vi_norm[: len(vi_norm) - len(vi_norm.lstrip(" "))]
            trail = vi_norm[len(vi_norm.rstrip(" ")) :]
            vi_to_en[vi_norm] = f"{lead}{curated[vi_norm.strip()]}{trail}"
        else:
            to_translate.append(vi_norm)

    print(f"Already curated: {len(vi_to_en)} | Needs vi->en neural translation: {len(to_translate)}")

    # Load cached vi->en if resuming
    cache_path = "scripts/i18n_work/vi_to_en_mt_cache.json"
    mt_cache = json.load(open(cache_path, encoding="utf-8")) if os.path.exists(cache_path) else {}
    pending = [s for s in to_translate if s not in mt_cache]
    print(f"Cached MT: {len(to_translate) - len(pending)} | Pending MT: {len(pending)}")

    # Chunk pending strings by ~3200 chars each
    batches = []
    cur_batch = []
    cur_len = 0
    for s in pending:
        if cur_batch and (cur_len + len(s) + 2 > 3200 or len(cur_batch) >= 55):
            batches.append(cur_batch)
            cur_batch = []
            cur_len = 0
        cur_batch.append(s)
        cur_len += len(s) + 2
    if cur_batch:
        batches.append(cur_batch)

    if batches:
        print(f"Translating {len(pending)} strings in {len(batches)} batches (vi -> en)...")
        t0 = time.time()
        with concurrent.futures.ThreadPoolExecutor(max_workers=10) as ex:
            futs = {ex.submit(translate_batch, b, "vi", "en"): b for b in batches}
            done = 0
            for fut in concurrent.futures.as_completed(futs):
                b = futs[fut]
                res = fut.result()
                for orig, en in zip(b, res):
                    mt_cache[orig] = en
                done += 1
                if done % 10 == 0 or done == len(batches):
                    print(f"  Batch {done}/{len(batches)} ({time.time() - t0:.1f}s)")
        with open(cache_path, "w", encoding="utf-8") as fp:
            json.dump(mt_cache, fp, ensure_ascii=False, indent=2)

    # Assemble & validate all vi -> en entries
    broken_placeholders = []
    for vi_norm in to_translate:
        sc = extracted[vi_norm]["slot_count"]
        en_clean = clean_en_output(vi_norm, mt_cache[vi_norm], sc)
        # Verify all placeholders {0}..{sc-1} are intact
        missing = [k for k in range(sc) if f"{{{k}}}" not in en_clean]
        if missing:
            # Retry single string individually with explicit spaces around {k}
            solo = translate_batch([vi_norm], "vi", "en")[0]
            en_clean = clean_en_output(vi_norm, solo, sc)
            missing = [k for k in range(sc) if f"{{{k}}}" not in en_clean]
            if missing:
                broken_placeholders.append((vi_norm, en_clean, missing))
        vi_to_en[vi_norm] = en_clean

    # Check if any en output accidentally still has Vietnamese diacritics
    still_vi = [(k, v) for k, v in vi_to_en.items() if VI_RE.search(v)]
    print(f"Broken placeholders: {len(broken_placeholders)}")
    for item in broken_placeholders[:10]:
        print("  BROKEN PH:", item)
    print(f"Still has VI chars in EN output: {len(still_vi)}")
    for item in still_vi[:15]:
        print("  STILL VI:", item)

    # Save full mapping + metadata
    out_obj = {
        "vi_to_en": vi_to_en,
        "meta": {
            k: {"slot_count": v["slot_count"], "files": sorted(v["files"])}
            for k, v in extracted.items()
        },
    }
    with open("scripts/i18n_work/vi_to_en.json", "w", encoding="utf-8") as fp:
        json.dump(out_obj, fp, ensure_ascii=False, indent=2)
    print(f"Saved scripts/i18n_work/vi_to_en.json ({len(vi_to_en)} entries)")


if __name__ == "__main__":
    main()
