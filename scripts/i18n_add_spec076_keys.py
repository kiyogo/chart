#!/usr/bin/env python3
"""
Spec 076 incremental i18n updater:
Appends all newly introduced Spec 069–076 exact keys and templates to
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
    "Offline: message queued and will send automatically when reconnected": "Đang ngoại tuyến: tin nhắn đã lưu hàng đợi và sẽ tự gửi khi có mạng",
    "This device's encryption key does not match your account. Sync from your trusted device or reset keys in Settings > Security": "Khóa mã hóa trên thiết bị này không khớp với tài khoản của bạn. Hãy đồng bộ từ thiết bị tin cậy hoặc đặt lại khóa trong Cài đặt › Bảo mật",
    "This device's encryption key does not match your account. Sync from your trusted device or reset keys in Settings > Privacy & security > Security": "Khóa mã hóa trên thiết bị này không khớp với tài khoản của bạn. Hãy đồng bộ từ thiết bị tin cậy hoặc đặt lại khóa trong Cài đặt › Riêng tư & bảo mật › Bảo mật",
    "Session expired. Please sign in again": "Phiên đăng nhập đã hết hạn. Vui lòng đăng nhập lại",
    "Recipient has not set up an encryption key yet": "Người nhận chưa thiết lập khóa mã hóa",
    "Recipient encryption key changed. Please verify their safety number": "Khóa mã hóa của người nhận đã thay đổi. Vui lòng xác minh mã bảo mật",
    "Scroll to bottom": "Cuộn xuống cuối",
    "Scroll to latest messages": "Cuộn tới tin nhắn mới nhất",
    "Seek voice message": "Tua tin nhắn thoại",
    "Seek voice position": "Tua vị trí giọng nói",
    "Download original": "Tải bản gốc",
    "Key mismatch": "Khóa không khớp",
    "This device's encryption key does not match your account. Sync from your trusted device or reset this device's key to resume messaging.": "Khóa mã hóa trên thiết bị này không khớp với tài khoản của bạn. Hãy đồng bộ từ thiết bị tin cậy hoặc đặt lại khóa trên thiết bị này để tiếp tục nhắn tin.",
    "Sync from trusted device": "Đồng bộ từ thiết bị tin cậy",
    "Reset device key": "Đặt lại khóa thiết bị",
    "Generating a new key lets this device send and receive new messages immediately. Old messages encrypted with a previous key cannot be decrypted without that key or a backup.": "Tạo khóa mới giúp thiết bị này gửi và nhận tin nhắn mới ngay lập tức. Tin nhắn cũ mã hóa bằng khóa trước đó sẽ không thể giải mã nếu không có khóa cũ hoặc bản sao lưu.",
    "Confirm reset key": "Xác nhận đặt lại khóa",
    "Device encryption key reset. You can send messages now.": "Đã đặt lại khóa mã hóa thiết bị. Bạn có thể gửi tin nhắn ngay bây giờ.",
    "Two-factor code required to reset keys on this device": "Cần mã xác minh hai bước để đặt lại khóa trên thiết bị này",
    "An encrypted key backup exists on your account. Sync from your trusted device or enter your backup password to unlock.": "Tài khoản của bạn đã có bản sao lưu khóa mã hóa. Hãy đồng bộ từ thiết bị tin cậy hoặc nhập mật khẩu sao lưu để mở khóa.",
    "This device was revoked. Please sign in again.": "Thiết bị này đã bị thu hồi. Vui lòng đăng nhập lại.",
}

NEW_TEMPLATES_VI = {
    "Unable to send ({0})": "Không thể gửi ({0})",
    "Unable to reset device key ({0})": "Không thể đặt lại khóa thiết bị ({0})",
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
        to_tr = list(add_exact) + [s.replace("{0}", "ZX0ZX") for s in add_tpl]
        res = post_translate(to_tr, api_tl) if to_tr else []
        tr_map = {}
        idx = 0
        for s in add_exact:
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
        print("All Spec 076 keys are already present in _keys.json.")
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

    n_ex = len(keys["exact"])
    n_tp = len(keys["templates"])
    bad = []
    for lo in langs:
        c = lo["code"]
        p = json.load(open(os.path.join(LOCALES_DIR, f"{c}.json"), encoding="utf-8"))
        if len(p["exact"]) != n_ex or len(p["templates"]) != n_tp:
            bad.append((c, len(p["exact"]), len(p["templates"])))
    if bad:
        print("PARITY MISMATCH:", bad[:10])
        sys.exit(1)
    print(f"SUCCESS: 100% parity across all {len(langs)} packs ({n_ex} exact, {n_tp} templates).")


if __name__ == "__main__":
    main()
