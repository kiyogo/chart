#!/usr/bin/env python3
"""
Spec 084 Font Asset Fetcher (SIL OFL / Open Source Fonts)
1. Downloads Monocraft v4.2.1 (IdreesInc/Monocraft, SIL OFL) and compresses Regular + Bold
   into compact WOFF2 files in app/public/fonts/ for the Minecraft font option.
2. Downloads self-hosted Vietnamese + multilingual WOFF2 subsets for 14 diverse SIL OFL fonts
   from Fontsource CDN and emits clean `@font-face` CSS with exact `unicode-range` blocks
   into app/assets/fonts_generated.css (imported or appended into app/assets/main.css).
3. Also caches raw rounded multi-script SIL OFL WOFF2 bases in scratch dir for
   scripts/build_chatw_lgbt_font.py to sculpt our brand-new original `ChatW LGBT` typeface.
"""
import concurrent.futures
import io
import json
import os
import re
import sys
import urllib.request
import zipfile

from fontTools.ttLib import TTFont

OUT_DIR = "app/public/fonts"
SCRATCH_BASES = "/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/scratch/lgbt_bases"
os.makedirs(OUT_DIR, exist_ok=True)
os.makedirs(SCRATCH_BASES, exist_ok=True)

HEADERS = {"User-Agent": "ChatW-FontBuilder/1.0"}


def fetch_bytes(url: str) -> bytes:
    req = urllib.request.Request(url, headers=HEADERS)
    with urllib.request.urlopen(req, timeout=30) as resp:
        return resp.read()


def build_minecraft_woff2():
    """Download IdreesInc/Monocraft v4.2.1 TTF archive and convert Regular + Bold to WOFF2."""
    reg_out = os.path.join(OUT_DIR, "minecraft-400.woff2")
    bold_out = os.path.join(OUT_DIR, "minecraft-700.woff2")
    if os.path.exists(reg_out) and os.path.exists(bold_out):
        print("Minecraft WOFF2 already built.")
        return

    url = "https://github.com/IdreesInc/Monocraft/releases/download/v4.2.1/Monocraft-ttf.zip"
    print("Downloading Monocraft v4.2.1 (SIL OFL)...")
    data = fetch_bytes(url)
    with zipfile.ZipFile(io.BytesIO(data)) as zf:
        names = zf.namelist()
        print("  Monocraft zip entries:", names)
        reg_name = next((n for n in names if n.endswith("Monocraft.ttf")), None)
        bold_name = next((n for n in names if "Bold.ttf" in n and "Italic" not in n), None)
        if not reg_name:
            reg_name = next((n for n in names if n.endswith(".ttf") and "Regular" in n), names[0])
        if not bold_name:
            bold_name = next((n for n in names if "SemiBold.ttf" in n and "Italic" not in n), reg_name)

        for src_name, dst_path in [(reg_name, reg_out), (bold_name, bold_out)]:
            raw_ttf = zf.read(src_name)
            font = TTFont(io.BytesIO(raw_ttf))
            font.flavor = "woff2"
            font.save(dst_path)
            print(f"  Built {dst_path} ({os.path.getsize(dst_path):,} bytes from {src_name})")


# (pkg_name, css_family_name, weights, subsets_to_include)
FONTSOURCE_FAMILIES = [
    ("plus-jakarta-sans", "ChatW Modern", [400, 600, 700], ["latin", "latin-ext", "vietnamese", "cyrillic-ext"]),
    ("nunito", "ChatW Rounded", [400, 700], ["latin", "latin-ext", "vietnamese", "cyrillic", "cyrillic-ext"]),
    ("quicksand", "ChatW Geometric", [500, 700], ["latin", "latin-ext", "vietnamese"]),
    ("be-vietnam-pro", "ChatW Humanist", [400, 600, 700], ["latin", "latin-ext", "vietnamese"]),
    ("bricolage-grotesque", "ChatW Expressive", [400, 700], ["latin", "latin-ext", "vietnamese"]),
    ("lora", "ChatW Serif", [400, 700], ["latin", "latin-ext", "vietnamese", "cyrillic", "cyrillic-ext"]),
    ("playfair-display", "ChatW Classic", [500, 700], ["latin", "latin-ext", "vietnamese", "cyrillic"]),
    ("roboto-slab", "ChatW Slab", [400, 700], ["latin", "latin-ext", "vietnamese", "cyrillic", "greek"]),
    ("jetbrains-mono", "ChatW Mono", [400, 600], ["latin", "latin-ext", "vietnamese", "cyrillic", "greek"]),
    ("vt323", "ChatW Retro", [400], ["latin", "latin-ext", "vietnamese"]),
    ("chakra-petch", "ChatW Futuristic", [500, 700], ["latin", "latin-ext", "vietnamese", "thai"]),
    ("patrick-hand", "ChatW Handwriting", [400], ["latin", "latin-ext", "vietnamese"]),
    ("dancing-script", "ChatW Calligraphy", [500, 700], ["latin", "latin-ext", "vietnamese"]),
    ("lexend", "ChatW Readable", [400, 600], ["latin", "latin-ext", "vietnamese"]),
]

# Multi-script joyful rounded bases for compiling our bespoke original `ChatW LGBT` font
LGBT_BASE_SPECS = [
    ("baloo-2", [500, 700], ["latin", "latin-ext", "vietnamese", "devanagari"]),
    ("nunito", [600, 800], ["cyrillic", "cyrillic-ext"]),
    ("baloo-bhaijaan-2", [500, 700], ["arabic"]),
    ("mali", [500, 700], ["thai"]),
    ("fredoka", [500, 700], ["hebrew"]),
    ("baloo-da-2", [500, 700], ["bengali"]),
    ("baloo-thambi-2", [500, 700], ["tamil"]),
    ("baloo-tamma-2", [500, 700], ["kannada"]),
    ("baloo-chettan-2", [500, 700], ["malayalam"]),
    ("baloo-paaji-2", [500, 700], ["gurmukhi"]),
    ("baloo-bhai-2", [500, 700], ["gujarati"]),
    ("baloo-tammudu-2", [500, 700], ["telugu"]),
]


def parse_fontsource_css(pkg: str, weight: int):
    url = f"https://cdn.jsdelivr.net/npm/@fontsource/{pkg}@latest/{weight}.css"
    css = fetch_bytes(url).decode("utf-8")
    blocks = []
    for m in re.finditer(r"/\*\s*([^*]+?)\s*\*/\s*@font-face\s*\{([^}]+)\}", css, re.S):
        comment = m.group(1).strip()
        body = m.group(2)
        # comment looks like "nunito-vietnamese-400-normal"
        sub_m = re.search(rf"{re.escape(pkg)}-([a-z0-9-]+?)-{weight}-normal", comment)
        subset = sub_m.group(1) if sub_m else ""
        file_m = re.search(r"url\(\./files/([^)]+\.woff2)\)", body)
        ur_m = re.search(r"unicode-range:\s*([^;]+);", body)
        if subset and file_m:
            blocks.append({
                "subset": subset,
                "file": file_m.group(1),
                "unicode_range": ur_m.group(1).strip() if ur_m else "",
            })
    return blocks


def download_file(url: str, dst: str):
    if os.path.exists(dst) and os.path.getsize(dst) > 500:
        return os.path.getsize(dst)
    data = fetch_bytes(url)
    with open(dst, "wb") as f:
        f.write(data)
    return len(data)


def main():
    build_minecraft_woff2()

    css_rules = [
        "/* Auto-generated by scripts/fetch_open_fonts.py (Spec 084 — SIL OFL self-hosted web fonts) */",
        "@font-face {",
        "  font-family: 'ChatW Minecraft';",
        "  font-style: normal;",
        "  font-display: swap;",
        "  font-weight: 400;",
        "  src: url('/fonts/minecraft-400.woff2') format('woff2');",
        "}",
        "@font-face {",
        "  font-family: 'ChatW Minecraft';",
        "  font-style: normal;",
        "  font-display: swap;",
        "  font-weight: 700;",
        "  src: url('/fonts/minecraft-700.woff2') format('woff2');",
        "}",
    ]

    download_jobs = []
    manifest_lgbt = []

    print("Fetching Fontsource CSS manifests...")
    for pkg, family, weights, wanted_subsets in FONTSOURCE_FAMILIES:
        for w in weights:
            blocks = parse_fontsource_css(pkg, w)
            for b in blocks:
                if b["subset"] not in wanted_subsets:
                    continue
                local_name = b["file"]
                dst = os.path.join(OUT_DIR, local_name)
                url = f"https://cdn.jsdelivr.net/npm/@fontsource/{pkg}@latest/files/{local_name}"
                download_jobs.append((url, dst))
                rule = [
                    "@font-face {",
                    f"  font-family: '{family}';",
                    "  font-style: normal;",
                    "  font-display: swap;",
                    f"  font-weight: {w};",
                    f"  src: url('/fonts/{local_name}') format('woff2');",
                ]
                if b["unicode_range"]:
                    rule.append(f"  unicode-range: {b['unicode_range']};")
                rule.append("}")
                css_rules.append("\n".join(rule))

    print("Fetching multi-script rounded bases for ChatW LGBT compiler...")
    for pkg, weights, wanted_subsets in LGBT_BASE_SPECS:
        for w in weights:
            try:
                blocks = parse_fontsource_css(pkg, w)
            except Exception as e:
                print(f"  WARN {pkg} {w}: {e}")
                continue
            for b in blocks:
                if b["subset"] not in wanted_subsets:
                    continue
                local_name = b["file"]
                dst = os.path.join(SCRATCH_BASES, local_name)
                url = f"https://cdn.jsdelivr.net/npm/@fontsource/{pkg}@latest/files/{local_name}"
                download_jobs.append((url, dst))
                manifest_lgbt.append({
                    "pkg": pkg,
                    "weight": w,
                    "subset": b["subset"],
                    "file": dst,
                    "unicode_range": b["unicode_range"],
                })

    print(f"Downloading {len(download_jobs)} WOFF2 files in parallel...")
    total_bytes = 0
    with concurrent.futures.ThreadPoolExecutor(max_workers=12) as pool:
        futs = {pool.submit(download_file, u, d): d for u, d in download_jobs}
        for f in concurrent.futures.as_completed(futs):
            d = futs[f]
            sz = f.result()
            total_bytes += sz

    with open(os.path.join(SCRATCH_BASES, "manifest.json"), "w", encoding="utf-8") as fp:
        json.dump(manifest_lgbt, fp, indent=2)

    with open(os.path.join(SCRATCH_BASES, "open_fonts.css"), "w", encoding="utf-8") as fp:
        fp.write("\n".join(css_rules) + "\n")

    print(f"DONE: {len(download_jobs)} files ({total_bytes / 1024:.1f} KB), {len(manifest_lgbt)} LGBT base subsets ready.")


if __name__ == "__main__":
    main()
