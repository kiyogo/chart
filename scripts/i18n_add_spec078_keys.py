#!/usr/bin/env python3
"""
Spec 078 incremental i18n updater:
Appends Lightbox zoom/rotate labels to app/web/locales/_keys.json and translates
ONLY those delta keys across all 197 world language packs (curated Vietnamese).
"""
import concurrent.futures
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from i18n_generate_all_locales import post_translate, sanitize_translation

NEW_EXACT_VI = {
    "Zoom in": "Phóng to",
    "Reset zoom": "Đặt lại thu phóng",
    "Rotate image": "Xoay ảnh",
}

LOCALES_DIR = "app/web/locales"
KEYS_PATH = os.path.join(LOCALES_DIR, "_keys.json")


def update_one_language(lang_obj, add_exact):
    code = lang_obj["code"]
    api_tl = lang_obj["api"]
    path = os.path.join(LOCALES_DIR, f"{code}.json")
    pack = json.load(open(path, encoding="utf-8"))

    if code == "en":
        tr_exact = list(add_exact)
    elif code == "vi":
        tr_exact = [NEW_EXACT_VI[k] for k in add_exact]
    else:
        res = post_translate(list(add_exact), api_tl) if add_exact else []
        tr_exact = [sanitize_translation(add_exact[i], res[i], False) for i in range(len(add_exact))]

    pack["exact"].extend(tr_exact)
    with open(path, "w", encoding="utf-8") as fp:
        json.dump(pack, fp, ensure_ascii=False, separators=(",", ":"))
    return code


def main():
    keys = json.load(open(KEYS_PATH, encoding="utf-8"))
    langs = json.load(open("scripts/i18n_world_languages.json", encoding="utf-8"))
    existing = set(keys["exact"])
    add_exact = [k for k in NEW_EXACT_VI.keys() if k not in existing]
    if not add_exact:
        print("All Spec 078 keys are already present.")
        return

    print(f"Adding {len(add_exact)} keys across {len(langs)} languages...")
    keys["exact"].extend(add_exact)
    with open(KEYS_PATH, "w", encoding="utf-8") as fp:
        json.dump(keys, fp, ensure_ascii=False, separators=(",", ":"))

    done = 0
    failed = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=16) as pool:
        futs = {pool.submit(update_one_language, lo, add_exact): lo["code"] for lo in langs}
        for f in concurrent.futures.as_completed(futs):
            c = futs[f]
            try:
                f.result()
                done += 1
                if done % 50 == 0 or done == len(langs):
                    print(f"  progress: {done}/{len(langs)}")
            except Exception as e:
                failed.append((c, str(e)))
                print(f"  ERROR {c}: {e}")

    if failed:
        print("FAILED:", failed)
        sys.exit(1)

    n_ex = len(keys["exact"])
    for lo in langs:
        p = json.load(open(os.path.join(LOCALES_DIR, f"{lo['code']}.json"), encoding="utf-8"))
        assert len(p["exact"]) == n_ex, f"Parity mismatch for {lo['code']}"
    print(f"SUCCESS: 100% parity across all {len(langs)} packs ({n_ex} exact).")


if __name__ == "__main__":
    main()
