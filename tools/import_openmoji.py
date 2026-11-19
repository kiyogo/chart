#!/usr/bin/env python3
"""Import OpenMoji colour SVGs as a built-in ChatW sticker set.

OpenMoji (https://openmoji.org, https://github.com/hfg-gmuend/openmoji) is
licensed CC BY-SA 4.0. This tool reads the release archive
`openmoji-svg-color.zip` directly (nothing is extracted to disk), keeps the
standard Unicode emoji without skin-tone variants plus single code point
extras that have a Unicode name, checks every SVG against a strict allow-list,
minifies it and writes:

  app/public/stickers/openmoji/<HEX>.svg
  app/public/stickers/openmoji/catalog.json   (+ catalog.json.gz, served precompressed)
  app/public/stickers/openmoji/LICENSE.md

Sticker names are English (Unicode CLDR short names from the ibus emoji
dictionaries). English and Vietnamese CLDR keywords are kept for search and
folded the same way as `composer::fold` in the client.

Usage, from the repository root:
  python3 -I tools/import_openmoji.py --zip /path/to/openmoji-svg-color.zip

Needs python3-gi (GLib) to read /usr/share/ibus/dicts/emoji-{en,vi}.dict.
"""

import argparse
import gzip
import hashlib
import json
import re
import sys
import unicodedata
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path

RELEASE = "17.0.0"
RELEASE_SHA256 = "59b0cd9f6fe033818fc02585cea42bea9fba5d68a4d3a3639bfe5d5cc3805689"
MAX_SVG_BYTES = 64 * 1024

SVG_NS = "http://www.w3.org/2000/svg"
HEX_NAME = re.compile(r"^([0-9A-F]{4,6}(?:-[0-9A-F]{4,6})*)\.svg$")

ALLOWED_TAGS = {
    "svg", "g", "path", "rect", "circle", "ellipse", "line", "polyline", "polygon",
    "defs", "clipPath", "linearGradient", "radialGradient", "stop",
}
ALLOWED_ATTRS = {
    "viewBox", "id", "d", "x", "y", "width", "height", "cx", "cy", "r", "rx", "ry",
    "x1", "y1", "x2", "y2", "points", "transform", "fill", "fill-opacity", "fill-rule",
    "stroke", "stroke-width", "stroke-linecap", "stroke-linejoin", "stroke-miterlimit",
    "stroke-dasharray", "stroke-dashoffset", "stroke-opacity", "clip-path", "clip-rule",
    "opacity", "paint-order", "color", "display", "offset", "stop-color", "stop-opacity",
    "gradientUnits", "gradientTransform",
}
# Harmless attributes that do nothing in a standalone image (markers are never defined).
DROPPED_ATTRS = {"version", "class", "marker-start", "marker-mid", "marker-end"}
SAFE_VALUE = re.compile(r"^[\w#%(),.+\- ]*$")
LOCAL_URL = re.compile(r"url\(#([\w-]+)\)")

# (id, label, cover hexcode) in picker order. Labels stay English like the emoji picker.
CATEGORIES = [
    ("smileys", "Smileys & Emotion", "1F600"),
    ("people", "People & Body", "1F44B"),
    ("nature", "Animals & Nature", "1F43C"),
    ("food", "Food & Drink", "1F354"),
    ("activities", "Activities", "26BD"),
    ("travel", "Travel & Places", "1F680"),
    ("objects", "Objects", "1F4A1"),
    ("symbols", "Symbols", "2764"),
    ("flags", "Flags", "1F3C1"),
    ("extras", "Extras", "1F12F"),
]
CLDR_GROUPS = {
    "Smileys & Emotion": "smileys",
    "People & Body": "people",
    "Component": "people",
    "Animals & Nature": "nature",
    "Food & Drink": "food",
    "Activities": "activities",
    "Travel & Places": "travel",
    "Objects": "objects",
    "Symbols": "symbols",
    "Flags": "flags",
}

# Same folding as `composer::fold` in the client: lowercase, Vietnamese marks removed.
_FOLD = {}
for base, chars in {
    "a": "àáảãạăằắẳẵặâầấẩẫậ",
    "e": "èéẻẽẹêềếểễệ",
    "i": "ìíỉĩị",
    "o": "òóỏõọôồốổỗộơờớởỡợ",
    "u": "ùúủũụưừứửữự",
    "y": "ỳýỷỹỵ",
    "d": "đ",
}.items():
    for ch in chars:
        _FOLD[ord(ch)] = base


def fold(s):
    return s.lower().translate(_FOLD)


def clean_text(s):
    return " ".join(s.replace("’", "'").split())


def norm(s):
    return s.replace("️", "")


def hex_to_str(h):
    return "".join(chr(int(x, 16)) for x in h.split("-"))


def load_ibus(path):
    import gi

    gi.require_version("GLib", "2.0")
    from gi.repository import GLib

    data = Path(path).read_bytes()
    variant = GLib.Variant.new_from_bytes(GLib.VariantType.new("(sqv)"), GLib.Bytes.new(data), False)
    magic, _version, items = variant.unpack()
    if magic != "IBusEmojiData":
        raise SystemExit(f"{path}: not an ibus emoji dictionary")
    out = {}
    for order, t in enumerate(items):
        emoji, count = t[2], t[3]
        annotations = list(t[4:4 + count])
        description = t[4 + count] if len(t) > 4 + count else ""
        group = t[5 + count] if len(t) > 5 + count else ""
        out.setdefault(norm(emoji), (description, annotations, group, order))
    return out


def load_emoji_data(path):
    """Category and order of each emoji in the client's own emoji picker."""
    out = {}
    category = None
    for line in Path(path).read_text(encoding="utf-8").splitlines():
        m = re.search(r'id: "(\w+)"', line)
        if m:
            category = m.group(1)
        for emoji, _kw in re.findall(r'\("([^"]+)", "([^"]*)"\)', line):
            if category:
                out.setdefault(norm(emoji), (category, len(out)))
    return out


def sentence_case(name):
    name = clean_text(name)
    return name[:1].upper() + name[1:]


def check_value(value):
    for ref in LOCAL_URL.findall(value):
        value = value.replace(f"url(#{ref})", "")
    if not SAFE_VALUE.match(value) or "url(" in value or "javascript" in value.lower() or "expression" in value.lower():
        raise ValueError(f"unsafe attribute value {value[:40]!r}")


def sanitize_svg(raw):
    """Parse with an allow-list and return a minified SVG string."""
    if len(raw) > MAX_SVG_BYTES:
        raise ValueError("file too large")
    text = raw.decode("utf-8")
    if "<!" in text or "<?xml-stylesheet" in text:
        raise ValueError("doctype, entity or comment")
    root = ET.fromstring(text)
    referenced = set()
    for el in root.iter():
        tag = el.tag
        if not tag.startswith("{" + SVG_NS + "}") or tag.split("}", 1)[1] not in ALLOWED_TAGS:
            raise ValueError(f"element {tag}")
        # Stray text outside <text> is never drawn (one OpenMoji file has a typo); it is dropped below.
        for key, value in list(el.attrib.items()):
            if key.startswith("{") or key in DROPPED_ATTRS:
                del el.attrib[key]
                continue
            if key not in ALLOWED_ATTRS:
                raise ValueError(f"attribute {key}")
            check_value(value)
            referenced.update(LOCAL_URL.findall(value))
    if root.tag != "{%s}svg" % SVG_NS or "viewBox" not in root.attrib:
        raise ValueError("not an svg root")

    def tidy(parent):
        children = []
        for child in list(parent):
            tidy(child)
            child.text = child.tail = None
            if "id" in child.attrib and child.attrib["id"] not in referenced:
                del child.attrib["id"]
            for key in ("d", "points"):
                if key in child.attrib:
                    child.attrib[key] = " ".join(child.attrib[key].split())
            is_group = child.tag == "{%s}g" % SVG_NS
            if is_group and not child.attrib:
                children.extend(list(child))  # unwrap attribute-less groups
            elif is_group and len(child) == 0:
                continue  # empty group
            else:
                children.append(child)
        for child in list(parent):
            parent.remove(child)
        parent.extend(children)

    tidy(root)
    root.text = root.tail = None
    root.attrib.pop("id", None)
    out = ET.tostring(root, encoding="unicode", short_empty_elements=True)
    # Re-check what we are about to publish.
    again = ET.fromstring(out)
    for el in again.iter():
        if el.tag.split("}", 1)[1] not in ALLOWED_TAGS:
            raise ValueError("post-check failed")
    if re.search(r"\son\w+=|script|href|<!", out, re.IGNORECASE):
        raise ValueError("post-check failed")
    return out


LICENSE_MD = """# OpenMoji stickers

The emoji artwork in this folder comes from **OpenMoji**, the open-source emoji
and icon project: https://openmoji.org (release {release},
https://github.com/hfg-gmuend/openmoji).

License: Creative Commons Attribution-ShareAlike 4.0 International
(CC BY-SA 4.0), https://creativecommons.org/licenses/by-sa/4.0/

Changes made for ChatW by `tools/import_openmoji.py`: every SVG was checked
against an allow-list of elements and attributes and minified (unused ids and
empty groups removed, whitespace collapsed). The drawings themselves are
unchanged. These adapted files are shared under the same license, CC BY-SA 4.0.

Sticker names and search keywords in `catalog.json` come from Unicode CLDR
annotations (English and Vietnamese), Copyright © Unicode, Inc., used under the
Unicode License v3; see THIRD_PARTY_NOTICES.md at the root of the repository.
"""


def main():
    ET.register_namespace("", SVG_NS)
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--zip", required=True, help="openmoji-svg-color.zip from the OpenMoji release")
    ap.add_argument("--sha256", default=RELEASE_SHA256, help="expected SHA-256 of the archive ('' to skip)")
    ap.add_argument("--ibus-en", default="/usr/share/ibus/dicts/emoji-en.dict")
    ap.add_argument("--ibus-vi", default="/usr/share/ibus/dicts/emoji-vi.dict")
    ap.add_argument("--emoji-data", default="app/src/components/emoji_data.rs")
    ap.add_argument("--out", default="app/public/stickers/openmoji")
    args = ap.parse_args()

    archive = Path(args.zip).read_bytes()
    digest = hashlib.sha256(archive).hexdigest()
    if args.sha256 and digest != args.sha256:
        raise SystemExit(f"checksum mismatch: {digest}")

    en = load_ibus(args.ibus_en)
    vi = load_ibus(args.ibus_vi)
    picker = load_emoji_data(args.emoji_data)
    cat_index = {c[0]: i for i, c in enumerate(CATEGORIES)}

    out_dir = Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)
    written = set()
    items = []
    skipped = {"skin tone": 0, "openmoji extra": 0, "no name": 0, "rejected": 0, "duplicate": 0}
    seen = set()

    with zipfile.ZipFile(args.zip) as z:
        for info in z.infolist():
            m = HEX_NAME.match(info.filename)
            if not m or info.is_dir():
                skipped["rejected"] += 1
                continue
            hexcode = m.group(1)
            s = hex_to_str(hexcode)
            key = norm(s)
            if any(0x1F3FB <= ord(c) <= 0x1F3FF for c in s):
                skipped["skin tone"] += 1
                continue
            if any(0xE000 <= ord(c) <= 0xF8FF or ord(c) >= 0xF0000 for c in s):
                skipped["openmoji extra"] += 1
                continue
            if key in seen:
                skipped["duplicate"] += 1
                continue
            if key in en:
                name = en[key][0]
                category = picker[key][0] if key in picker else CLDR_GROUPS.get(en[key][2], "symbols")
                order = picker[key][1] if key in picker else 100000 + en[key][3]
            elif len(s) == 1 and unicodedata.name(s, ""):
                name = unicodedata.name(s).lower()
                category, order = "extras", 200000 + ord(s)
            else:
                skipped["no name"] += 1
                continue
            if info.file_size > MAX_SVG_BYTES:
                skipped["rejected"] += 1
                continue
            try:
                svg = sanitize_svg(z.read(info))
            except (ValueError, ET.ParseError, UnicodeDecodeError) as e:
                print(f"rejected {info.filename}: {e}", file=sys.stderr)
                skipped["rejected"] += 1
                continue
            seen.add(key)
            name = sentence_case(name)[:64]
            words = []
            for w in [name] + (en[key][1] if key in en else []) + (vi[key][1] + [vi[key][0]] if key in vi else []):
                f = fold(clean_text(w))
                if f and f not in words:
                    words.append(f)
            (out_dir / f"{hexcode}.svg").write_text(svg, encoding="utf-8")
            written.add(f"{hexcode}.svg")
            items.append((cat_index[category], order, hexcode, name, "|".join(words[1:])))

    # Remove artwork from earlier imports that is no longer part of the set.
    for old in out_dir.glob("*.svg"):
        if HEX_NAME.match(old.name) and old.name not in written:
            old.unlink()

    items.sort()
    covers = {h for _, _, h, _, _ in items}
    categories = []
    for cid, label, cover in CATEGORIES:
        members = [h for c, _, h, _, _ in items if CATEGORIES[c][0] == cid]
        if members:
            categories.append([cid, label, cover if cover in covers else members[0]])
    used = [c[0] for c in categories]
    catalog = {
        "v": 1,
        "source": f"OpenMoji {RELEASE}",
        "license": "CC BY-SA 4.0",
        "categories": categories,
        "items": [[h, n, used.index(CATEGORIES[c][0]), k] for c, _, h, n, k in items],
    }
    blob = json.dumps(catalog, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    (out_dir / "catalog.json").write_bytes(blob)
    (out_dir / "catalog.json.gz").write_bytes(gzip.compress(blob, compresslevel=9, mtime=0))
    (out_dir / "LICENSE.md").write_text(LICENSE_MD.format(release=RELEASE), encoding="utf-8")

    per_cat = {}
    for c, _, _, _, _ in items:
        per_cat[CATEGORIES[c][0]] = per_cat.get(CATEGORIES[c][0], 0) + 1
    print(f"stickers: {len(items)}  catalog: {len(blob)} bytes")
    print("per category:", per_cat)
    print("skipped:", skipped)


if __name__ == "__main__":
    main()
