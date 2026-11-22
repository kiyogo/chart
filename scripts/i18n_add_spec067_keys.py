#!/usr/bin/env python3
"""
Spec 067 incremental i18n updater:
Appends all newly introduced Settings Redesign, About & Developer Mode keys to
app/web/locales/_keys.json and translates ONLY those delta keys across all 197
world language packs (with hand-curated native Vietnamese translations).
"""
import concurrent.futures
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from i18n_generate_all_locales import post_translate, sanitize_translation

NEW_EXACT_VI = {
    "Privacy & security": "Quyền riêng tư & bảo mật",
    "Chat & notifications": "Trò chuyện & thông báo",
    "Data & backup": "Dữ liệu & sao lưu",
    "About": "Giới thiệu",
    "Theme & look": "Giao diện & hiển thị",
    "Personalize": "Cá nhân hóa",
    "Advanced export": "Xuất dữ liệu nâng cao",
    "Encryption keys": "Khóa mã hóa",
    "Sync & recovery": "Đồng bộ & khôi phục",
    "Diagnostics": "Chẩn đoán",
    "End-to-end encryption": "Mã hóa đầu cuối",
    "Private end-to-end encrypted messenger": "Ứng dụng nhắn tin riêng tư mã hóa đầu cuối",
    "App information": "Thông tin ứng dụng",
    "App name": "Tên ứng dụng",
    "Version": "Phiên bản",
    "Build number": "Số bản dựng",
    "Platform": "Nền tảng",
    "Web (WebAssembly)": "Web (WebAssembly)",
    "Encryption": "Mã hóa",
    "Double Ratchet • X25519 • AES-256-GCM": "Double Ratchet • X25519 • AES-256-GCM",
    "Privacy model": "Mô hình riêng tư",
    "Zero-knowledge blind relay": "Chuyển tiếp mù không lưu khóa",
    "Languages": "Ngôn ngữ",
    "197 languages": "197 ngôn ngữ",
    "License": "Giấy phép",
    "Open Source (MIT)": "Mã nguồn mở (MIT)",
    "Developer options": "Tùy chọn nhà phát triển",
    "Developer mode": "Chế độ nhà phát triển",
    "Show the Developer menu with advanced data export, encryption keys, sync recovery, extensions, and diagnostics.": "Hiển thị mục Nhà phát triển gồm xuất dữ liệu nâng cao, khóa mã hóa, đồng bộ khôi phục, tiện ích và chẩn đoán.",
    "Enabled": "Đang bật",
    "Open Developer": "Mở Nhà phát triển",
    "Support & diagnostics": "Hỗ trợ & chẩn đoán",
    "User guide & shortcuts": "Hướng dẫn & phím tắt",
    "Open guide": "Mở hướng dẫn",
    "Interactive tour": "Giới thiệu nhanh",
    "Start tour": "Bắt đầu xem",
    "System summary": "Thông tin hệ thống",
    "Copy diagnostics": "Sao chép chẩn đoán",
    "Runtime diagnostics": "Chẩn đoán hệ thống",
    "Account ID": "ID tài khoản",
    "Encryption status": "Trạng thái mã hóa",
    "Maintenance": "Bảo trì",
    "Diagnostic report": "Báo cáo chẩn đoán",
    "Appearance defaults": "Giao diện mặc định",
    "Turn off developer mode": "Tắt chế độ nhà phát triển",
}

NEW_TEMPLATES_VI = {
    "Tap {0} more times to enable developer mode": "Nhấn thêm {0} lần để bật chế độ nhà phát triển",
}

KEEP_VERBATIM = {
    "Double Ratchet • X25519 • AES-256-GCM",
}

LOCALES_DIR = "app/web/locales"
KEYS_PATH = os.path.join(LOCALES_DIR, "_keys.json")


def update_one_language(lang_obj, add_exact, add_tpl):
    code = lang_obj["code"]
    api_tl = lang_obj["api"]
    path = os.path.join(LOCALES_DIR, f"{code}.json")
    pack = json.load(open(path, encoding="utf-8"))

    if code == "en":
        tr_exact = list(add_exact)
        tr_tpl = list(add_tpl)
    elif code == "vi":
        tr_exact = [NEW_EXACT_VI[k] for k in add_exact]
        tr_tpl = [NEW_TEMPLATES_VI[k] for k in add_tpl]
    else:
        to_tr = [s for s in add_exact if s not in KEEP_VERBATIM] + [
            s.replace("{0}", "ZX0ZX") for s in add_tpl
        ]
        res = post_translate(to_tr, api_tl) if to_tr else []
        tr_map = {}
        idx = 0
        for s in add_exact:
            if s in KEEP_VERBATIM:
                tr_map[s] = s
            else:
                tr_map[s] = sanitize_translation(s, res[idx], False)
                idx += 1
        tpl_map = {}
        for s in add_tpl:
            raw_t = res[idx].replace("ZX0ZX", "{0}").replace("zx0zx", "{0}")
            idx += 1
            tpl_map[s] = sanitize_translation(s, raw_t, True)
        tr_exact = [tr_map[k] for k in add_exact]
        tr_tpl = [tpl_map[k] for k in add_tpl]

    pack["exact"].extend(tr_exact)
    pack["templates"].extend(tr_tpl)
    with open(path, "w", encoding="utf-8") as fp:
        json.dump(pack, fp, ensure_ascii=False, separators=(",", ":"))
    return code


def main():
    keys = json.load(open(KEYS_PATH, encoding="utf-8"))
    langs = json.load(open("scripts/i18n_world_languages.json", encoding="utf-8"))

    existing_exact = set(keys["exact"])
    existing_tpl = set(keys["templates"])
    add_exact = [k for k in NEW_EXACT_VI.keys() if k not in existing_exact]
    add_tpl = [k for k in NEW_TEMPLATES_VI.keys() if k not in existing_tpl]

    if not add_exact and not add_tpl:
        print("All Spec 067 keys are already present in _keys.json.")
        return

    print(f"Adding {len(add_exact)} exact keys and {len(add_tpl)} template keys across {len(langs)} languages...")
    keys["exact"].extend(add_exact)
    keys["templates"].extend(add_tpl)
    with open(KEYS_PATH, "w", encoding="utf-8") as fp:
        json.dump(keys, fp, ensure_ascii=False, separators=(",", ":"))

    done = 0
    failed = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=16) as pool:
        futs = {pool.submit(update_one_language, lo, add_exact, add_tpl): lo["code"] for lo in langs}
        for f in concurrent.futures.as_completed(futs):
            c = futs[f]
            try:
                f.result()
                done += 1
                if done % 40 == 0 or done == len(langs):
                    print(f"  progress: {done}/{len(langs)}")
            except Exception as e:
                failed.append((c, str(e)))
                print(f"  ERROR {c}: {e}")

    if failed:
        print("FAILED:", failed)
        sys.exit(1)

    # Verify 100% parity across all 197 packs
    n_ex = len(keys["exact"])
    n_tp = len(keys["templates"])
    bad = []
    for lo in langs:
        c = lo["code"]
        p = json.load(open(os.path.join(LOCALES_DIR, f"{c}.json"), encoding="utf-8"))
        if len(p["exact"]) != n_ex or len(p["templates"]) != n_tp:
            bad.append((c, len(p["exact"]), len(p["templates"])))
        if any(not v or not v.strip() or "..." in v for v in p["exact"] + p["templates"]):
            bad.append((c, "empty_or_ellipsis"))
    if bad:
        print("PARITY ERROR:", bad[:10])
        sys.exit(1)
    print(f"SUCCESS: 197/197 languages synced ({n_ex} exact + {n_tp} templates, 0 empty, 0 '...').")


if __name__ == "__main__":
    main()
