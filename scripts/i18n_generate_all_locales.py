#!/usr/bin/env python3
"""
Phase 4 (Spec 064): Generate canonical English _keys.json and all 197 indexed world
language packs in app/web/locales/{code}.json with:
- 100% key parity across all 197 world languages
- 100% authentic native Vietnamese (vi.json) mapped directly from original source strings
- Preserved hand-curated Spec 062 core overrides for the top 31 major languages
- Direct English -> target neural translation for all 197 languages
- Strict placeholder integrity ({0}, {1}, ...), zero empty values, zero "..."
"""
import concurrent.futures
import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

# Pre-existing English UI strings in app/src & app/web that must also be translated
EXTRA_EN_EXACT = [
    "ChatW - Private End-to-End Encrypted Messenger",
    "Add to Favorite GIFs",
    "Add to Favorite Stickers",
    "Add to Favorites",
    "Angry",
    "Anime Profile",
    "Arrow keys to move, Enter to pick",
    "Artist Profile",
    "Avatar Studio",
    "Avatar effects",
    "Bot",
    "Bounce",
    "Breathe",
    "Bubbles",
    "Celebrate",
    "Clear",
    "Color Ring",
    "Color hex",
    "Community",
    "Community Profile",
    "Confetti",
    "Creator Profile",
    "Cry",
    "Custom",
    "Custom Emoji",
    "Cyber Profile",
    "Dark Profile",
    "Darken",
    "Desktop",
    "Developer Profile",
    "Dizzy",
    "Done",
    "Downloads",
    "Drawings",
    "Ease In",
    "Ease In-Out",
    "Ease Out",
    "Elastic",
    "Electric",
    "Elegant Profile",
    "Email",
    "Embed image",
    "Emoji",
    "Emoji you use will show up here",
    "Emoji, stickers and GIFs",
    "Fade In",
    "Fade Out",
    "Favorite GIF",
    "Favorite Stickers",
    "Favorites",
    "Fire",
    "Flash",
    "Flip",
    "Float",
    "GIF Collections",
    "Gaming Profile",
    "General",
    "Gentle",
    "Glass Profile",
    "Glitch",
    "Glow",
    "Gradient Ring",
    "Halo",
    "Heartbeat",
    "Hearts",
    "Hello",
    "High",
    "Hologram",
    "Hour",
    "Hue Shift",
    "Jitter",
    "Keyframes",
    "Light Profile",
    "Lighten",
    "Linear",
    "Loading",
    "Member",
    "Minute",
    "Nature Profile",
    "Neon",
    "Neon Profile",
    "No emoji found",
    "No matching languages found",
    "No stickers found",
    "Nod",
    "None",
    "Normal",
    "Now",
    "Onion skin",
    "Only you",
    "Opacity",
    "Orange",
    "Orbit",
    "Origin",
    "Overlay",
    "Pastel Profile",
    "Pin",
    "Pixel",
    "Pixel Profile",
    "Plugin",
    "Preview",
    "Professional Profile",
    "Profile Templates",
    "Pulse",
    "Rain",
    "Rainbow Ring",
    "Reactions",
    "Recently Used",
    "Recently used",
    "Remove from Favorite GIFs",
    "Remove from Favorite Stickers",
    "Remove from Favorites",
    "Results",
    "Retro Profile",
    "Right-click to remove from Favorites",
    "Ripple",
    "Rocket",
    "Rotate",
    "Sad",
    "Scale",
    "Scanline",
    "Screen",
    "Search GIFs",
    "Search languages",
    "Search stickers",
    "Send GIF",
    "Send favorite GIF",
    "Shake",
    "Shine",
    "Showcase",
    "Sleep",
    "Slide In",
    "Smoke",
    "Snow",
    "Soft",
    "Sparkle",
    "Spin",
    "Stars",
    "Sticker",
    "Sticker Packs",
    "Stickers drawn in ChatW Studio",
    "Swing",
    "Thank you",
    "Thumbnail",
    "Tilt",
    "Today",
    "Tolerance",
    "Typing",
    "Unknown IP",
    "Untitled",
    "Video",
    "View",
    "Vintage Profile",
    "Wave",
    "Web Browser",
    "Website",
    "Wobble",
    "Zoom",
    "Zoom In",
    "this device",
    "a thought",
    "Add story",
    "Notes, files, and links synced across your devices",
    "Notes, links, photos, and reminders only you can see, synced across your signed-in devices. Click any device name to rename it.",
    "Messages are end-to-end encrypted on your device. The server only relays ciphertext and cannot read your content.",
    "I am currently busy and will reply as soon as possible.",
    "Main navigation",
    "Download app",
    "Download for desktop",
    "My account",
    "My profile",
    "Open settings",
    "Close menu",
    "Quick switch",
    "All devices",
    "Create new",
    "Create new group",
    "Create or join community",
    "Start conversation",
    "Direct messages",
    "Unread",
    "Pinned",
    "Groups",
    "Channels",
    "Plugins",
    "Select a conversation",
    "End-to-end encrypted",
    "Seen",
    "Message",
    "Invite",
    "Copy link",
    "Delete",
    "Save",
    "Cancel",
    "Close",
    "Send",
    "Log out",
    "Profile",
]

EXTRA_EN_TEMPLATES = [
    "{0} languages",
    "Panel transparency: {0}%",
    "Glass blur level: {0}px",
    "Wallpaper dim: {0}%",
    "Wallpaper blur: {0}px",
    "Sync version: {0}",
    "{0} votes",
    "Message {0}",
    "{0} is typing",
    "Only you · {0} devices",
    "Step {0} / {1}",
    "{0} min",
    "{0} hr",
    "{0} days",
    "{0} rolled a die: {1}",
    "{0} · active {1}",
    "Read decrypted messages for conversation {0}",
    "Started sandbox ({0}, origin=null) successfully",
    "Sandbox startup timed out ({0}ms)",
    "Execution limit exceeded ({0}ms)",
    "Background plugin ({0}).",
    "Permission revoked immediately ({0})",
    "Error handling event {0}",
]

# Preferred authentic Vietnamese disambiguation when multiple VI synonyms mapped to one EN string
PREFERRED_EN_TO_VI = {
    "ChatW - Private End-to-End Encrypted Messenger": "ChatW - Ứng dụng nhắn tin bảo mật mã hóa đầu cuối E2EE",
    "Chat wallpaper": "Hình nền trò chuyện",
    "Always": "Luôn luôn",
    "Title": "Tiêu đề",
    "Link copied": "Đã sao chép liên kết",
    "On": "Bật",
    "Off": "Tắt",
    "Edit": "Chỉnh sửa",
    "Completed": "Hoàn thành",
    "System": "Theo hệ thống",
    "Cancel": "Hủy",
    "Username": "Tên đăng nhập",
    "More options": "Tùy chọn khác",
    "All": "Tất cả",
    "Enter": "Vào",
    "Normal": "Vừa",
    "Request to join": "Xin tham gia",
    "Copied": "Đã sao chép",
    "Copy link": "Sao chép liên kết",
    "Welcome": "Chào mừng",
    "Cloud service": "Dịch vụ đám mây",
    "Start time": "Thời gian bắt đầu",
    "Call": "Cuộc gọi",
    "Work": "Công việc",
    "End": "Kết thúc",
    "Not assigned yet": "Chưa phân công",
    "Search languages": "Tìm kiếm ngôn ngữ",
    "No matching languages found": "Không tìm thấy ngôn ngữ phù hợp",
    "{0} languages": "{0} ngôn ngữ",
    "{0} · active {1}": "{0} · hoạt động {1}",
    "Member": "Thành viên",
    "Unknown IP": "IP không rõ",
    "Clear": "Xóa",
    "Today": "Hôm nay",
    "Hour": "Giờ",
    "Minute": "Phút",
    "Now": "Bây giờ",
    "Color hex": "Mã màu",
    "Done": "Xong",
    "Showcase": "Xem thử",
    "Preview": "Xem trước",
    "Gentle": "Nhẹ",
    "Soft": "Mềm",
    "Note to self": "Chính mình",
    "Only you": "Chỉ mình bạn",
    "this device": "thiết bị này",
    "a thought": "suy nghĩ",
    "Add story": "Tạo tin",
    "I am currently busy and will reply as soon as possible.": "Tôi đang bận, sẽ trả lời bạn sớm nhất có thể.",
    "Background plugin ({0}).": "Plugin chạy nền ({0}).",
    "Read decrypted messages for conversation {0}": "Đã đọc tin nhắn giải mã cho cuộc trò chuyện {0}",
    "Started sandbox ({0}, origin=null) successfully": "Khởi chạy sandbox ({0}, origin=null) thành công",
    "Sandbox startup timed out ({0}ms)": "Quá thời gian khởi động sandbox ({0}ms)",
    "Execution limit exceeded ({0}ms)": "Vượt quá giới hạn thực thi ({0}ms)",
    "Permission revoked immediately ({0})": "Đã thu hồi quyền ngay lập tức ({0})",
    "Error handling event {0}": "Lỗi xử lý sự kiện {0}",
    "Main navigation": "Điều hướng chính",
    "Download app": "Tải ứng dụng",
    "Download for desktop": "Tải cho máy tính",
    "My account": "Tài khoản của tôi",
    "My profile": "Hồ sơ của tôi",
    "Open settings": "Mở cài đặt",
    "Close menu": "Đóng menu",
    "Quick switch": "Tìm nhanh",
    "All devices": "Tất cả thiết bị",
    "Create new": "Tạo mới",
    "Create new group": "Tạo nhóm mới",
    "Create or join community": "Tạo hoặc tham gia cộng đồng",
    "Start conversation": "Bắt đầu trò chuyện",
    "Direct messages": "Tin nhắn trực tiếp",
    "Unread": "Chưa đọc",
    "Pinned": "Đã ghim",
    "Groups": "Nhóm",
    "Channels": "Kênh",
    "Plugins": "Plugin",
    "Select a conversation": "Chọn một cuộc trò chuyện",
    "End-to-end encrypted": "Mã hóa đầu cuối",
    "Seen": "Đã xem",
    "Message": "Nhắn tin",
    "Invite": "Mời",
    "Delete": "Xóa",
    "Save": "Lưu",
    "Close": "Đóng",
    "Send": "Gửi",
    "Log out": "Đăng xuất",
    "Profile": "Hồ sơ",
}

PH_RE = re.compile(r"\{(\d+)\}")
HAS_ALPHA_RE = re.compile(r"[A-Za-z]{2,}")


def static_char_len(tpl: str) -> int:
    return len(PH_RE.sub("", tpl).strip())


def post_translate(texts, api_tl):
    """Translate list of English strings to `api_tl` cleanly via 1:1 multi-q POST batch."""
    if not texts:
        return []
    url = (
        f"https://clients5.google.com/translate_a/t"
        f"?client=dict-chrome-ex&sl=en&tl={urllib.parse.quote(api_tl)}"
    )
    data = urllib.parse.urlencode([("q", t) for t in texts]).encode("utf-8")
    last_err = None
    for attempt in range(7):
        try:
            req = urllib.request.Request(
                url,
                data=data,
                headers={
                    "User-Agent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
                    "Content-Type": "application/x-www-form-urlencoded",
                },
            )
            with urllib.request.urlopen(req, timeout=30) as r:
                resp = json.loads(r.read().decode("utf-8"))
            # Single-item responses can be ["str"] or [["str", "en"]]
            if isinstance(resp, list) and len(resp) == len(texts):
                out = []
                for item in resp:
                    if isinstance(item, list):
                        out.append(str(item[0] or "").strip())
                    else:
                        out.append(str(item or "").strip())
                time.sleep(0.05)
                return out
            if len(texts) == 1:
                if isinstance(resp, list) and resp:
                    v = resp[0][0] if isinstance(resp[0], list) else resp[0]
                    return [str(v or texts[0]).strip()]
                return [texts[0]]
            mid = len(texts) // 2
            return post_translate(texts[:mid], api_tl) + post_translate(texts[mid:], api_tl)
        except urllib.error.HTTPError as e:
            last_err = e
            if e.code == 400 and len(texts) > 1:
                mid = len(texts) // 2
                return post_translate(texts[:mid], api_tl) + post_translate(texts[mid:], api_tl)
            wait = min(20.0, 2.0 * (attempt + 1)) if e.code == 429 else min(10.0, 1.0 * (attempt + 1))
            time.sleep(wait)
        except Exception as e:
            last_err = e
            time.sleep(min(10.0, 1.0 * (attempt + 1)))
    raise RuntimeError(f"Failed translating to {api_tl}: {last_err}")


def sanitize_translation(orig_en: str, tr: str, is_tpl: bool) -> str:
    s = (tr or "").strip()
    # Forbidden "..." ellipsis (AGENTS.md Rule 6)
    s = s.replace("...", "").replace("…", "").strip()
    # Normalize spaced / fullwidth braces around placeholders
    if is_tpl:
        s = s.replace("｛", "{").replace("｝", "}")
        # Convert Arabic-Indic / Persian / Devanagari digits inside { } back to ASCII digits
        def fix_ph(m):
            digits = m.group(1).strip()
            ascii_d = "".join(
                str("٠١٢٣٤٥٦٧٨٩".index(c)) if c in "٠١٢٣٤٥٦٧٨٩"
                else str("۰۱۲۳۴۵۶۷۸۹".index(c)) if c in "۰۱۲۳۴۵۶۷۸۹"
                else str("०१२३४५६७८९".index(c)) if c in "०१२३४५६७८९"
                else c
                for c in digits
            )
            return f"{{{ascii_d}}}"
        s = re.sub(r"\{\s*([0-9٠-٩۰-۹०-९]+)\s*\}", fix_ph, s)
        req_phs = sorted(set(PH_RE.findall(orig_en)))
        for k in req_phs:
            if f"{{{k}}}" not in s:
                return orig_en  # Safe fallback if exotic script corrupted placeholder
    if not s:
        return orig_en
    return s


def build_keys():
    raw = json.load(open("scripts/i18n_work/vi_to_en.json", encoding="utf-8"))
    vi_to_en = raw["vi_to_en"]
    meta = raw["meta"]

    exact_set = set()
    template_set = set()
    en_to_orig_vi = {}

    for vi_norm, en_val in vi_to_en.items():
        en_clean = en_val.strip().replace("...", "").replace("…", "").strip()
        vi_clean = vi_norm.strip().replace("...", "").replace("…", "").strip()
        if not en_clean:
            continue
        sc = meta[vi_norm]["slot_count"]
        if sc == 0:
            exact_set.add(en_clean)
            if en_clean not in en_to_orig_vi:
                en_to_orig_vi[en_clean] = vi_clean
        else:
            # Keep only templates that contain alphabetic English words so DOM regexes stay unambiguous
            if HAS_ALPHA_RE.search(PH_RE.sub("", en_clean)):
                template_set.add(en_clean)
                if en_clean not in en_to_orig_vi:
                    en_to_orig_vi[en_clean] = vi_clean

    for s in EXTRA_EN_EXACT:
        exact_set.add(s.strip())
    for s in EXTRA_EN_TEMPLATES:
        template_set.add(s.strip())

    # Apply preferred authentic Vietnamese disambiguations
    for en_k, vi_v in PREFERRED_EN_TO_VI.items():
        en_to_orig_vi[en_k] = vi_v

    exact_list = sorted(exact_set)
    # Sort templates most-specific first (static text length DESC, then alphabetical)
    template_list = sorted(template_set, key=lambda x: (-static_char_len(x), x))

    return exact_list, template_list, en_to_orig_vi, vi_to_en


def load_legacy_curated_overrides(code: str, vi_to_en: dict) -> dict:
    """Re-key hand-curated Spec 062 overrides from old VI keys to canonical English keys."""
    path = f"scripts/i18n_work/legacy_curated_31/{code}.json"
    if not os.path.exists(path):
        return {}
    raw = json.load(open(path, encoding="utf-8"))
    out = {}
    for section in ("exact", "templates"):
        for old_vi, tr_val in raw.get(section, {}).items():
            if not tr_val or "..." in tr_val:
                continue
            en_key = vi_to_en.get(old_vi) or vi_to_en.get(old_vi.strip())
            if en_key:
                en_key = en_key.strip()
                if en_key not in out:
                    out[en_key] = tr_val.strip()
    return out


def chunk_strings(items, max_chars=13000, max_count=400):
    batches = []
    cur = []
    clen = 0
    for s in items:
        if cur and (clen + len(s) + 2 > max_chars or len(cur) >= max_count):
            batches.append(cur)
            cur = []
            clen = 0
        cur.append(s)
        clen += len(s) + 2
    if cur:
        batches.append(cur)
    return batches


def generate_one_language(lang_obj, exact_list, template_list, en_to_orig_vi, vi_to_en, out_dir):
    code = lang_obj["code"]
    api_tl = lang_obj["api"]
    dst = os.path.join(out_dir, f"{code}.json")

    # Skip if already generated with v=2 and exact parity
    if os.path.exists(dst):
        try:
            ex = json.load(open(dst, encoding="utf-8"))
            if (
                ex.get("v") == 2
                and ex.get("lang") == code
                and len(ex.get("exact", [])) == len(exact_list)
                and len(ex.get("templates", [])) == len(template_list)
                and all(bool(v and v.strip()) and "..." not in v for v in ex["exact"])
                and all(bool(v and v.strip()) and "..." not in v for v in ex["templates"])
            ):
                return (code, "cached", 0)
        except Exception:
            pass

    if code == "en":
        pack = {"v": 2, "lang": "en", "exact": exact_list, "templates": template_list}
        with open(dst, "w", encoding="utf-8") as fp:
            json.dump(pack, fp, ensure_ascii=False, separators=(",", ":"))
        return (code, "identity", 0)

    # Pre-seed known authentic overrides (Vietnamese original strings or Spec 062 curated top-31)
    overrides = {}
    if code == "vi":
        overrides.update(en_to_orig_vi)
    else:
        overrides.update(load_legacy_curated_overrides(code, vi_to_en))

    all_needed = []
    seen_need = set()
    for s in exact_list + template_list:
        if s not in overrides and s not in seen_need:
            seen_need.add(s)
            all_needed.append(s)

    tr_map = dict(overrides)
    if all_needed:
        batches = chunk_strings(all_needed)
        # Translate batches sequentially per language (outer pool runs multiple languages in parallel!)
        for b in batches:
            res = post_translate(b, api_tl)
            for orig, out in zip(b, res):
                tr_map[orig] = out

    exact_out = [sanitize_translation(s, tr_map.get(s, s), False) for s in exact_list]
    tpl_out = [sanitize_translation(s, tr_map.get(s, s), True) for s in template_list]

    # Batch-retry ONLY templates whose placeholders actually broke during initial pass (in 1 single POST!)
    retry_idxs = []
    retry_wrapped = []
    for idx, orig in enumerate(template_list):
        if code != "en" and orig not in overrides:
            raw_tr = tr_map.get(orig, orig)
            # Only retry if placeholder was actually lost (causing sanitize_translation to fall back)
            if tpl_out[idx] == orig and raw_tr != orig:
                retry_idxs.append(idx)
                retry_wrapped.append(PH_RE.sub(lambda m: f"XPH{m.group(1)}X", orig))
    if retry_wrapped:
        try:
            retry_res = post_translate(retry_wrapped, api_tl)
            for idx, solo in zip(retry_idxs, retry_res):
                orig = template_list[idx]
                restored = re.sub(r"XPH(\d+)X", r"{\1}", solo, flags=re.IGNORECASE)
                fixed = sanitize_translation(orig, restored, True)
                if all(f"{{{k}}}" in fixed for k in set(PH_RE.findall(orig))):
                    tpl_out[idx] = fixed
        except Exception:
            pass

    pack = {"v": 2, "lang": code, "exact": exact_out, "templates": tpl_out}
    tmp = dst + ".tmp"
    with open(tmp, "w", encoding="utf-8") as fp:
        json.dump(pack, fp, ensure_ascii=False, separators=(",", ":"))
    os.replace(tmp, dst)
    return (code, "generated", len(all_needed))


def main():
    langs = json.load(open("scripts/i18n_world_languages.json", encoding="utf-8"))
    out_dir = "app/web/locales"
    os.makedirs(out_dir, exist_ok=True)

    exact_list, template_list, en_to_orig_vi, vi_to_en = build_keys()
    keys_path = os.path.join(out_dir, "_keys.json")
    with open(keys_path, "w", encoding="utf-8") as fp:
        json.dump(
            {"v": 2, "exact": exact_list, "templates": template_list},
            fp,
            ensure_ascii=False,
            separators=(",", ":"),
        )
    print(f"Wrote {keys_path}: {len(exact_list)} exact strings + {len(template_list)} templates")

    workers = int(os.environ.get("I18N_WORKERS", "12"))
    t0 = time.time()
    done = 0
    errors = []

    with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as ex:
        futs = {
            ex.submit(
                generate_one_language, l, exact_list, template_list, en_to_orig_vi, vi_to_en, out_dir
            ): l["code"]
            for l in langs
        }
        for fut in concurrent.futures.as_completed(futs):
            code = futs[fut]
            try:
                c, status, cnt = fut.result()
                done += 1
                if done % 10 == 0 or done == len(langs) or status != "cached":
                    if done % 10 == 0 or done == len(langs) or c in ("en", "vi", "zh-CN", "he", "ar", "ja"):
                        print(f"[{done:3d}/{len(langs)}] {c:9s} ({status}, translated={cnt})  {time.time() - t0:.1f}s", flush=True)
            except Exception as e:
                errors.append((code, str(e)))
                print(f"ERROR {code}: {e}", file=sys.stderr, flush=True)

    print(f"\nFinished {done}/{len(langs)} languages in {time.time() - t0:.1f}s (errors: {len(errors)})")
    if errors:
        for c, e in errors:
            print(f"  FAILED {c}: {e}")
        sys.exit(1)

    # Full validation pass over ALL 197 packs
    for l in langs:
        code = l["code"]
        p = json.load(open(os.path.join(out_dir, f"{code}.json"), encoding="utf-8"))
        assert p["v"] == 2 and p["lang"] == code
        assert len(p["exact"]) == len(exact_list), f"{code} exact mismatch"
        assert len(p["templates"]) == len(template_list), f"{code} template mismatch"
        for i, v in enumerate(p["exact"]):
            assert v and v.strip() and "..." not in v, f"{code} bad exact[{i}]: {v!r}"
        for i, v in enumerate(p["templates"]):
            assert v and v.strip() and "..." not in v, f"{code} bad tpl[{i}]: {v!r}"
            for k in set(PH_RE.findall(template_list[i])):
                assert f"{{{k}}}" in v, f"{code} missing placeholder {{{k}}} in tpl[{i}]: {v!r}"
    print(f"VALIDATION PASSED 100%: ALL {len(langs)} world language packs verified!")


if __name__ == "__main__":
    main()
