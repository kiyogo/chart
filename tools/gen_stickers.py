#!/usr/bin/env python3
"""Generate ChatW's original sticker packs and animated reactions as SVG.

Run from the repository root:  python3 tools/gen_stickers.py
Writes app/public/stickers/<pack>/<expression>.svg, app/public/stickers/fx/*.svg
and web/backend/migrations/0012_more_stickers.sql. Names are English.
"""
import os
import uuid

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "app", "public", "stickers")
INK = "#2B2A3D"
BLUSH = "#FF8FA3"
HEART = "#FF4D6D"
TEAR = "#6EC6FF"
GOLD = "#FFD23F"
FONT = "font-family=\"Nunito, 'Arial Rounded MT Bold', 'Segoe UI', Verdana, sans-serif\" font-weight=\"900\""
REDUCED = "@media (prefers-reduced-motion: reduce){*{animation:none!important}}"


def heart(x, y, s=1.0, fill=HEART, cls=""):
    # CSS animations replace the SVG transform attribute, so animated shapes
    # are wrapped in a group and keep their placement on the inner path.
    path = (f'<path transform="translate({x - 14 * s:.1f} {y - 10 * s:.1f}) scale({s})" '
            f'd="M0 6C0-4 14-6 14 4c0-10 14-8 14 2 0 10-14 18-14 18S0 16 0 6z" fill="{fill}"/>')
    return f'<g class="{cls}">{path}</g>' if cls else path


def star(x, y, r=10, fill=GOLD, cls=""):
    c = f' class="{cls}"' if cls else ""
    return (f'<path{c} d="M{x} {y - r}Q{x + r * 0.18} {y - r * 0.18} {x + r} {y}Q{x + r * 0.18} {y + r * 0.18} {x} {y + r}'
            f'Q{x - r * 0.18} {y + r * 0.18} {x - r} {y}Q{x - r * 0.18} {y - r * 0.18} {x} {y - r}z" fill="{fill}"/>')


def label(text, x=128, y=240, size=34, fill="#FFFFFF", cls=""):
    c = f' class="{cls}"' if cls else ""
    return (f'<text{c} x="{x}" y="{y}" {FONT} font-size="{size}" text-anchor="middle" fill="{fill}" '
            f'stroke="{INK}" stroke-width="7" stroke-linejoin="round" paint-order="stroke">{text}</text>')


# ---------------------------------------------------------------- characters
class Char:
    def __init__(self, slug, name, skin, shade, accent, ey=128, ex=30, my=160, cy=152, beak=False):
        self.slug, self.name, self.skin, self.shade, self.accent = slug, name, skin, shade, accent
        self.ey, self.ex, self.my, self.cy, self.beak = ey, ex, my, cy, beak

    def body(self):
        s, d, a = self.skin, self.shade, self.accent
        if self.slug == "mochi":  # cat
            return (f'<path d="M58 104L70 40Q74 30 84 36L122 66Z" fill="{s}"/><path d="M198 104L186 40Q182 30 172 36L134 66Z" fill="{s}"/>'
                    f'<path d="M70 86L78 50L104 70Z" fill="{a}"/><path d="M186 86L178 50L152 70Z" fill="{a}"/>'
                    f'<ellipse cx="128" cy="138" rx="94" ry="80" fill="{s}"/><ellipse cx="128" cy="150" rx="78" ry="58" fill="{d}" opacity=".35"/>'
                    f'<path d="M44 150h28M46 166l26-6M212 150h-28M210 166l-26-6" stroke="{INK}" stroke-width="3" stroke-linecap="round" opacity=".45"/>')
        if self.slug == "puff":  # bear
            return (f'<circle cx="62" cy="72" r="30" fill="{s}"/><circle cx="194" cy="72" r="30" fill="{s}"/>'
                    f'<circle cx="62" cy="72" r="16" fill="{a}"/><circle cx="194" cy="72" r="16" fill="{a}"/>'
                    f'<ellipse cx="128" cy="140" rx="94" ry="82" fill="{s}"/><ellipse cx="128" cy="168" rx="42" ry="30" fill="{d}"/>'
                    f'<ellipse cx="128" cy="154" rx="11" ry="8" fill="{INK}"/>')
        if self.slug == "bun":  # bunny
            return (f'<ellipse cx="94" cy="58" rx="20" ry="54" fill="{s}" transform="rotate(-12 94 58)"/>'
                    f'<ellipse cx="162" cy="58" rx="20" ry="54" fill="{s}" transform="rotate(12 162 58)"/>'
                    f'<ellipse cx="94" cy="62" rx="9" ry="38" fill="{a}" transform="rotate(-12 94 62)"/>'
                    f'<ellipse cx="162" cy="62" rx="9" ry="38" fill="{a}" transform="rotate(12 162 62)"/>'
                    f'<ellipse cx="128" cy="150" rx="90" ry="76" fill="{s}"/><path d="M120 150q8 7 16 0" fill="{a}"/>')
        if self.slug == "pip":  # chick
            return (f'<path d="M118 52q10-26 18-4q12-20 14 6" stroke="{d}" stroke-width="8" fill="none" stroke-linecap="round"/>'
                    f'<circle cx="128" cy="140" r="92" fill="{s}"/><ellipse cx="128" cy="166" rx="70" ry="44" fill="{d}" opacity=".3"/>'
                    f'<path d="M40 150q-14 6-10 22q14-4 22-14z" fill="{d}"/><path d="M216 150q14 6 10 22q-14-4-22-14z" fill="{d}"/>')
        if self.slug == "leafy":  # frog
            return (f'<circle cx="86" cy="94" r="34" fill="{s}"/><circle cx="170" cy="94" r="34" fill="{s}"/>'
                    f'<ellipse cx="128" cy="152" rx="100" ry="70" fill="{s}"/><ellipse cx="128" cy="176" rx="70" ry="34" fill="{d}" opacity=".35"/>'
                    f'<circle cx="86" cy="96" r="22" fill="#fff"/><circle cx="170" cy="96" r="22" fill="#fff"/>')
        # nimbus: cloud
        return (f'<circle cx="80" cy="146" r="52" fill="{s}"/><circle cx="128" cy="112" r="64" fill="{s}"/><circle cx="182" cy="142" r="54" fill="{s}"/>'
                f'<rect x="34" y="138" width="190" height="64" rx="32" fill="{s}"/><ellipse cx="104" cy="88" rx="22" ry="10" fill="#fff" opacity=".6"/>'
                f'<ellipse cx="128" cy="186" rx="76" ry="14" fill="{d}" opacity=".4"/>')


CHARS = [
    Char("mochi", "Mochi", "#FFE1BF", "#F6B98A", "#FFA8B8"),
    Char("puff", "Puff", "#C98E5C", "#F1CFA6", "#E9B48A", my=176, cy=150),
    Char("bun", "Bun", "#FBF7F2", "#E9DDD0", "#FFB3C7", ey=138, my=168, cy=160),
    Char("pip", "Pip", "#FFD84D", "#FFB627", "#FF8A3D", my=162, beak=True),
    Char("leafy", "Leafy", "#86D66F", "#4FAE45", "#C9F2A6", ey=96, ex=42, my=170, cy=160),
    Char("nimbus", "Nimbus", "#EAF3FF", "#B9D4F5", "#9EC5F2", ey=136, my=166, cy=158),
]


def eyes(kind, c):
    L, R, y = 128 - c.ex, 128 + c.ex, c.ey
    w = 'stroke-width="7" fill="none" stroke-linecap="round"'
    if kind == "normal":
        return "".join(f'<ellipse cx="{x}" cy="{y}" rx="10" ry="13" fill="{INK}"/><circle cx="{x + 4}" cy="{y - 5}" r="4" fill="#fff"/>' for x in (L, R))
    if kind == "happy":
        return "".join(f'<path d="M{x - 12} {y + 4}Q{x} {y - 12} {x + 12} {y + 4}" stroke="{INK}" {w}/>' for x in (L, R))
    if kind == "down":
        return "".join(f'<path d="M{x - 12} {y - 2}Q{x} {y + 10} {x + 12} {y - 2}" stroke="{INK}" {w}/>' for x in (L, R))
    if kind == "heart":
        return heart(L, y, 1.25, cls="beat") + heart(R, y, 1.25, cls="beat")
    if kind == "wide":
        return "".join(f'<ellipse cx="{x}" cy="{y}" rx="15" ry="17" fill="#fff" stroke="{INK}" stroke-width="4"/><circle cx="{x}" cy="{y + 2}" r="8" fill="{INK}"/><circle cx="{x + 3}" cy="{y - 1}" r="3" fill="#fff"/>' for x in (L, R))
    if kind == "sleepy":
        return "".join(f'<path d="M{x - 12} {y + 2}Q{x} {y + 8} {x + 12} {y + 2}" stroke="{INK}" {w}/>' for x in (L, R))
    if kind == "wink":
        return (f'<ellipse cx="{L}" cy="{y}" rx="10" ry="13" fill="{INK}"/><circle cx="{L + 4}" cy="{y - 5}" r="4" fill="#fff"/>'
                f'<path d="M{R - 12} {y + 2}Q{R} {y - 10} {R + 12} {y + 2}" stroke="{INK}" {w}/>')
    if kind == "small":
        return "".join(f'<ellipse cx="{x}" cy="{y + 6}" rx="7" ry="9" fill="{INK}"/>' for x in (L, R))
    if kind == "sad":
        return ("".join(f'<ellipse cx="{x}" cy="{y + 2}" rx="9" ry="11" fill="{INK}"/><circle cx="{x + 3}" cy="{y - 2}" r="3.5" fill="#fff"/>' for x in (L, R))
                + f'<path d="M{L - 14} {y - 22}L{L + 10} {y - 16}M{R + 14} {y - 22}L{R - 10} {y - 16}" stroke="{INK}" stroke-width="5" stroke-linecap="round"/>')
    if kind == "angry":
        return ("".join(f'<ellipse cx="{x}" cy="{y + 2}" rx="9" ry="10" fill="{INK}"/>' for x in (L, R))
                + f'<path d="M{L - 14} {y - 22}L{L + 12} {y - 10}M{R + 14} {y - 22}L{R - 12} {y - 10}" stroke="{INK}" stroke-width="6" stroke-linecap="round"/>')
    if kind == "hmm":
        return (f'<ellipse cx="{L}" cy="{y}" rx="10" ry="13" fill="{INK}"/><circle cx="{L + 4}" cy="{y - 5}" r="4" fill="#fff"/>'
                f'<path d="M{R - 12} {y}h24" stroke="{INK}" {w}/><path d="M{R - 14} {y - 22}Q{R} {y - 32} {R + 14} {y - 22}" stroke="{INK}" stroke-width="5" fill="none" stroke-linecap="round"/>')
    if kind == "cool":
        return (f'<rect x="{L - 26}" y="{y - 16}" width="50" height="30" rx="12" fill="{INK}"/><rect x="{R - 24}" y="{y - 16}" width="50" height="30" rx="12" fill="{INK}"/>'
                f'<path d="M{L + 24} {y - 6}H{R - 24}" stroke="{INK}" stroke-width="6"/><path d="M{L - 14} {y - 8}l12 0M{R - 12} {y - 8}l12 0" stroke="#fff" stroke-width="4" stroke-linecap="round" opacity=".7"/>')
    return eyes("normal", c)


def mouth(kind, c):
    y = c.my
    st = f'stroke="{INK}" stroke-width="7" fill="none" stroke-linecap="round" stroke-linejoin="round"'
    if c.beak:
        if kind in ("big", "o", "open"):
            return f'<path d="M114 {y - 6}L128 {y - 14}L142 {y - 6}Z" fill="#FF8A3D"/><path d="M116 {y}L128 {y + 14}L140 {y}Z" fill="#E86A1F"/>'
        return f'<path d="M114 {y - 4}L128 {y - 12}L142 {y - 4}L128 {y + 6}Z" fill="#FF8A3D"/>'
    if kind == "smile":
        return f'<path d="M110 {y}Q128 {y + 18} 146 {y}" {st}/>'
    if kind == "big":
        return (f'<path d="M104 {y - 4}Q128 {y + 38} 152 {y - 4}Z" fill="{INK}" stroke="{INK}" stroke-width="4" stroke-linejoin="round"/>'
                f'<ellipse cx="128" cy="{y + 12}" rx="12" ry="7" fill="#FF7A90"/>')
    if kind == "open":
        return f'<path d="M110 {y - 2}Q128 {y + 26} 146 {y - 2}Z" fill="{INK}"/>'
    if kind == "o":
        return f'<ellipse cx="128" cy="{y + 4}" rx="10" ry="13" fill="{INK}"/>'
    if kind == "small_o":
        return f'<ellipse cx="128" cy="{y + 4}" rx="6" ry="7" fill="{INK}"/>'
    if kind == "frown":
        return f'<path d="M112 {y + 12}Q128 {y - 4} 144 {y + 12}" {st}/>'
    if kind == "flat":
        return f'<path d="M114 {y + 6}H142" {st}/>'
    if kind == "side":
        return f'<path d="M116 {y + 8}Q132 {y + 4} 146 {y - 2}" {st}/>'
    if kind == "smirk":
        return f'<path d="M110 {y + 2}Q132 {y + 16} 150 {y - 6}" {st}/>'
    if kind == "wavy":
        return f'<path d="M108 {y + 6}q5-7 10 0t10 0t10 0t10 0" {st}/>'
    if kind == "cat":
        return f'<path d="M110 {y}Q119 {y + 12} 128 {y}Q137 {y + 12} 146 {y}" {st}/>'
    return mouth("smile", c)


def cheeks(c, strong=False):
    o = ".85" if strong else ".55"
    return "".join(f'<ellipse cx="{x}" cy="{c.cy}" rx="{16 if strong else 13}" ry="8" fill="{BLUSH}" opacity="{o}"/>' for x in (128 - c.ex - 26, 128 + c.ex + 26))


def paw(x, y, c, r=20, cls=""):
    k = f' class="{cls}"' if cls else ""
    return f'<g{k}><circle cx="{x}" cy="{y}" r="{r}" fill="{c.skin}" stroke="{INK}" stroke-width="4"/><path d="M{x - 7} {y - 6}v7M{x} {y - 8}v8M{x + 7} {y - 6}v7" stroke="{INK}" stroke-width="3" stroke-linecap="round" opacity=".5"/></g>'


EXPR = [
    # key, English name, eyes, mouth, extras(c) -> svg
    ("hello", "Hello", "normal", "smile"),
    ("love", "Love", "heart", "smile"),
    ("lol", "LOL", "happy", "big"),
    ("ok", "OK", "wink", "smile"),
    ("wow", "Wow", "wide", "o"),
    ("sad", "Sad", "sad", "frown"),
    ("cry", "Cry", "down", "wavy"),
    ("sleepy", "Sleepy", "sleepy", "small_o"),
    ("party", "Party", "happy", "open"),
    ("thanks", "Thanks", "happy", "smile"),
    ("hmm", "Hmm", "hmm", "side"),
    ("angry", "Angry", "angry", "flat"),
    ("cool", "Cool", "cool", "smirk"),
    ("shy", "Shy", "small", "wavy"),
    ("yay", "Yay", "happy", "open"),
    ("sorry", "Sorry", "down", "small_o"),
]


def extras(key, c):
    if key == "hello":
        return f'<g class="wave">{paw(214, 110, c, 22)}</g>' + label("Hi!", 200, 64, 36, cls="pop")
    if key == "love":
        return heart(206, 56, 1.4, cls="rise") + heart(50, 70, 1.0, fill="#FF8FA3", cls="rise d1") + heart(222, 104, 0.8, cls="rise d2")
    if key == "lol":
        drops = "".join(f'<path d="M{x} {c.ey + 6}q-8 14 0 20q8-6 0-20z" fill="{TEAR}"/>' for x in (128 - c.ex - 20, 128 + c.ex + 20))
        return drops + label("LOL", 128, 52, 40, fill="#FFD23F", cls="pop")
    if key == "ok":
        return (f'<g class="wave"><rect x="186" y="150" width="42" height="40" rx="14" fill="{c.skin}" stroke="{INK}" stroke-width="4"/>'
                f'<rect x="196" y="112" width="18" height="44" rx="9" fill="{c.skin}" stroke="{INK}" stroke-width="4"/></g>' + label("OK", 64, 64, 38, cls="pop"))
    if key == "wow":
        return star(210, 60, 14, cls="tw") + star(46, 74, 10, cls="tw d1") + star(222, 112, 8, cls="tw d2") + label("Wow", 128, 50, 36, cls="pop")
    if key == "sad":
        return f'<path d="M{128 - c.ex} {c.ey + 16}q-7 13 0 19q7-6 0-19z" fill="{TEAR}" class="drip"/>'
    if key == "cry":
        return "".join(f'<path d="M{x - 6} {c.ey + 8}q-4 30 4 60h6q-6-30-2-60z" fill="{TEAR}" opacity=".9" class="drip"/>' for x in (128 - c.ex, 128 + c.ex + 6))
    if key == "sleepy":
        return label("Z", 196, 74, 34, fill="#9EC5F2", cls="zz") + label("z", 218, 48, 24, fill="#9EC5F2", cls="zz d1") + label("z", 232, 28, 18, fill="#9EC5F2", cls="zz d2")
    if key == "party":
        conf = "".join(f'<g class="fall d{i % 3}"><rect x="{x}" y="{y}" width="10" height="16" rx="3" fill="{f}" transform="rotate({r} {x} {y})"/></g>'
                       for i, (x, y, f, r) in enumerate([(40, 40, "#FF4D6D", 20), (70, 20, "#FFD23F", -30), (190, 30, "#4F7BFF", 40), (220, 70, "#3DDC97", -20), (30, 100, "#B388FF", 10), (226, 120, "#FF8A3D", 30)]))
        return conf + f'<path d="M78 66L110 8L132 62Z" fill="#4F7BFF"/><path d="M90 44l26 6M84 56l36 6" stroke="#FFD23F" stroke-width="6"/><circle cx="110" cy="8" r="8" fill="#FF4D6D"/>'
    if key == "thanks":
        return heart(128, 214, 2.0, cls="beat") + label("Thanks!", 128, 50, 34, fill="#FFB3C7", cls="pop")
    if key == "hmm":
        return (f'<g class="tw"><circle cx="206" cy="60" r="26" fill="#fff" stroke="{INK}" stroke-width="4"/><circle cx="182" cy="96" r="7" fill="#fff" stroke="{INK}" stroke-width="3"/></g>'
                f'<text x="206" y="72" {FONT} font-size="34" text-anchor="middle" fill="{INK}">?</text>' + paw(70, 190, c, 18))
    if key == "angry":
        return (f'<ellipse cx="128" cy="{c.ey - 20}" rx="70" ry="22" fill="#FF5A5A" opacity=".22"/>'
                f'<path class="beat" d="M196 52l10 10M216 52l-10 10M196 72l10-10M216 72l-10-10" stroke="#FF3B3B" stroke-width="7" stroke-linecap="round"/>' + label("Hmph", 70, 56, 30, fill="#FF8080", cls="pop"))
    if key == "cool":
        return star(212, 66, 14, fill="#fff", cls="tw") + star(44, 92, 9, fill="#fff", cls="tw d1")
    if key == "shy":
        return heart(212, 74, 0.9, fill="#FF8FA3", cls="rise") + heart(46, 92, 0.7, fill="#FF8FA3", cls="rise d1")
    if key == "yay":
        return (paw(40, 86, c, 20, "wave") + paw(216, 86, c, 20, "wave d1") + star(128, 34, 12, cls="tw") + star(70, 40, 8, cls="tw d1")
                + star(186, 40, 8, cls="tw d2") + label("Yay!", 128, 244, 34, fill="#FFD23F", cls="pop"))
    if key == "sorry":
        return f'<path d="M196 76q-10 16 0 24q10-8 0-24z" fill="{TEAR}" class="drip"/>' + label("Sorry", 128, 244, 32, fill="#C7D7FF", cls="pop")
    return ""


ANIM_CSS = (
    "<style>.bob{animation:bob 1.8s ease-in-out infinite;transform-origin:128px 220px}"
    "@keyframes bob{0%,100%{transform:translateY(0)}50%{transform:translateY(-8px)}}"
    ".wave{animation:wave 1s ease-in-out infinite;transform-box:fill-box;transform-origin:50% 100%}"
    "@keyframes wave{0%,100%{transform:rotate(0)}50%{transform:rotate(-18deg)}}"
    ".beat{animation:beat 1s ease-in-out infinite;transform-box:fill-box;transform-origin:center}"
    "@keyframes beat{0%,100%{transform:scale(1)}50%{transform:scale(1.18)}}"
    ".rise{animation:rise 2.4s ease-in infinite}@keyframes rise{0%{transform:translateY(10px);opacity:0}30%{opacity:1}100%{transform:translateY(-30px);opacity:0}}"
    ".tw{animation:tw 1.6s ease-in-out infinite;transform-box:fill-box;transform-origin:center}@keyframes tw{0%,100%{transform:scale(1);opacity:1}50%{transform:scale(.6);opacity:.5}}"
    ".pop{animation:pop 1.8s ease-in-out infinite;transform-box:fill-box;transform-origin:center}@keyframes pop{0%,100%{transform:scale(1)}40%{transform:scale(1.12)}}"
    ".drip{animation:drip 1.4s ease-in infinite}@keyframes drip{0%{transform:translateY(0);opacity:1}100%{transform:translateY(14px);opacity:0}}"
    ".zz{animation:zz 2.4s ease-in-out infinite}@keyframes zz{0%{transform:translate(0,8px);opacity:0}40%{opacity:1}100%{transform:translate(8px,-10px);opacity:0}}"
    ".fall{animation:fall 2s linear infinite}@keyframes fall{0%{transform:translateY(-10px)}100%{transform:translateY(24px)}}"
    ".d1{animation-delay:.35s}.d2{animation-delay:.7s}" + REDUCED + "</style>"
)


def sticker(c, key, eye, mo, animated):
    face = eyes(eye, c) + mouth("cat" if (c.slug == "mochi" and mo == "smile") else mo, c)
    blush = cheeks(c, strong=key in ("shy", "love", "thanks"))
    inner = c.body() + blush + face
    body = f'<g class="bob">{inner}</g>' if animated else inner
    extra = extras(key, c)
    css = ANIM_CSS if animated else ""
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">{css}{body}{extra}</svg>'


# ------------------------------------------------------------- reactions (GIF)
def fx(name, body, css):
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><style>{css}{REDUCED}</style>{body}</svg>'


def big_text(t, size=86, fill="#FFD23F", y=150, cls=""):
    return label(t, 128, y, size, fill=fill, cls=cls)


FX = {
    "lol": ("LOL", "".join(label(ch, 64 + i * 64, 150, 92, fill=f, cls=f"b d{i}") for i, (ch, f) in enumerate(zip("LOL", ["#FFD23F", "#FF8A3D", "#FF4D6D"]))),
            ".b{animation:b .9s ease-in-out infinite}@keyframes b{0%,100%{transform:translateY(0)}50%{transform:translateY(-18px)}}.d1{animation-delay:.15s}.d2{animation-delay:.3s}"),
    "omg": ("OMG", big_text("OMG!", 78, "#FF4D6D", cls="s"), ".s{animation:s .35s linear infinite;transform-box:fill-box;transform-origin:center}@keyframes s{0%,100%{transform:rotate(-4deg)}50%{transform:rotate(4deg)}}"),
    "gg": ("GG", big_text("GG", 110, "#3DDC97", cls="g"), ".g{animation:g 1.4s ease-in-out infinite;transform-box:fill-box;transform-origin:center}@keyframes g{0%,100%{transform:scale(1);opacity:1}50%{transform:scale(1.12);opacity:.85}}"),
    "plus-one": ("+1", big_text("+1", 120, "#4F7BFF", cls="p"), ".p{animation:p 1s cubic-bezier(.3,1.6,.5,1) infinite;transform-box:fill-box;transform-origin:center}@keyframes p{0%{transform:scale(.6)}40%{transform:scale(1.15)}100%{transform:scale(1)}}"),
    "thank-you": ("Thank You", big_text("Thank you", 46, "#FFB3C7", 150) + heart(70, 80, 1.6, cls="r") + heart(186, 70, 1.2, fill="#FF8FA3", cls="r d1") + heart(128, 210, 1.4, cls="r d2"),
                  ".r{animation:r 2s ease-in infinite}@keyframes r{0%{transform:translateY(20px);opacity:0}30%{opacity:1}100%{transform:translateY(-40px);opacity:0}}.d1{animation-delay:.6s}.d2{animation-delay:1.2s}"),
    "confetti": ("Confetti", "".join(f'<g class="c d{i % 4}"><rect x="{x}" y="-20" width="12" height="20" rx="3" fill="{f}" transform="rotate({r} {x} 0)"/></g>'
                                     for i, (x, f, r) in enumerate([(20, "#FF4D6D", 10), (56, "#FFD23F", -20), (92, "#4F7BFF", 30), (128, "#3DDC97", -10), (164, "#B388FF", 20), (200, "#FF8A3D", -30), (236, "#6EC6FF", 15), (38, "#3DDC97", 25), (110, "#FF4D6D", -25), (182, "#FFD23F", 5)])),
                 ".c{animation:c 2.2s linear infinite}@keyframes c{0%{transform:translateY(0) rotate(0)}100%{transform:translateY(300px) rotate(360deg)}}.d1{animation-delay:.5s}.d2{animation-delay:1s}.d3{animation-delay:1.5s}"),
    "fireworks": ("Fireworks", '<rect width="256" height="256" rx="40" fill="#141B33"/>' + "".join(
        f'<g transform="translate({x} {y})"><circle r="40" fill="none" stroke="{f}" stroke-width="6" stroke-dasharray="4 14" class="f d{i}"/><circle r="6" fill="{f}" class="f d{i}"/></g>'
        for i, (x, y, f) in enumerate([(80, 90, "#FFD23F"), (176, 110, "#FF4D6D"), (120, 180, "#6EC6FF")])),
        ".f{animation:f 1.8s ease-out infinite;transform-box:fill-box;transform-origin:center}@keyframes f{0%{transform:scale(.1);opacity:1}80%{opacity:1}100%{transform:scale(1.3);opacity:0}}.d1{animation-delay:.6s}.d2{animation-delay:1.2s}"),
    "heart-rain": ("Heart Rain", "".join(heart(x, -20, s, fill=f, cls=f"h d{i % 4}") for i, (x, s, f) in enumerate([(30, 1.2, HEART), (80, 0.8, "#FF8FA3"), (128, 1.5, HEART), (176, 1.0, "#FFB3C7"), (226, 1.3, HEART), (56, 1.0, "#FFB3C7"), (200, 0.7, HEART)])),
                   ".h{animation:h 2.4s linear infinite}@keyframes h{0%{transform:translateY(0)}100%{transform:translateY(300px)}}.d1{animation-delay:.6s}.d2{animation-delay:1.2s}.d3{animation-delay:1.8s}"),
    "clap": ("Clap", '<g class="l"><rect x="58" y="70" width="64" height="110" rx="30" fill="#FFD27A" stroke="#2B2A3D" stroke-width="5"/><path d="M76 100v30M90 92v38M104 100v30" stroke="#2B2A3D" stroke-width="4" stroke-linecap="round" opacity=".35"/></g><g class="r"><rect x="134" y="70" width="64" height="110" rx="30" fill="#FFC15E" stroke="#2B2A3D" stroke-width="5"/><path d="M152 100v30M166 92v38M180 100v30" stroke="#2B2A3D" stroke-width="4" stroke-linecap="round" opacity=".35"/></g>'
             '<path d="M128 40v-24M96 50l-12-18M160 50l12-18" stroke="#FFD23F" stroke-width="8" stroke-linecap="round" class="k"/>',
             ".l{animation:l .5s ease-in-out infinite}.r{animation:r .5s ease-in-out infinite}@keyframes l{50%{transform:translateX(10px) rotate(6deg)}}@keyframes r{50%{transform:translateX(-10px) rotate(-6deg)}}.k{animation:k .5s infinite}@keyframes k{0%,60%{opacity:0}70%,100%{opacity:1}}"),
    "fire": ("Fire", '<g class="fl"><path d="M128 20C150 70 200 92 196 156c-3 50-38 80-68 80s-68-28-68-78c0-40 26-58 34-86 12 22 20 34 34 38 6-30 0-60-4-90z" fill="#FF6A2B"/>'
             '<path d="M128 110c16 26 40 40 38 72-2 26-20 40-38 40s-38-14-38-40c0-22 14-30 18-46 8 12 14 18 22 20 2-14 0-30-2-46z" fill="#FFD23F"/></g>',
             ".fl{animation:fl .6s ease-in-out infinite alternate;transform-origin:128px 236px}@keyframes fl{0%{transform:scale(1,1) skewX(0)}100%{transform:scale(1.04,.96) skewX(3deg)}}"),
    "sparkles": ("Sparkles", star(128, 120, 60, cls="t") + star(52, 60, 22, fill="#FFF3B0", cls="t d1") + star(206, 70, 26, fill="#FFF3B0", cls="t d2") + star(200, 200, 18, cls="t d1") + star(56, 196, 16, cls="t d2"),
                 ".t{animation:t 1.4s ease-in-out infinite;transform-box:fill-box;transform-origin:center}@keyframes t{0%,100%{transform:scale(1) rotate(0)}50%{transform:scale(.7) rotate(20deg)}}.d1{animation-delay:.45s}.d2{animation-delay:.9s}"),
    "rainbow": ("Rainbow", "".join(f'<path d="M{24 + i * 14} 196A{104 - i * 14} {104 - i * 14} 0 0 1 {232 - i * 14} 196" fill="none" stroke="{f}" stroke-width="14" stroke-linecap="round" pathLength="100" class="a d{i}"/>'
                                   for i, f in enumerate(["#FF4D6D", "#FF8A3D", "#FFD23F", "#3DDC97", "#4F7BFF", "#B388FF"])),
                ".a{stroke-dasharray:100;animation:a 2.4s ease-in-out infinite}@keyframes a{0%{stroke-dashoffset:100}50%,100%{stroke-dashoffset:0}}"
                + "".join(f".d{i}{{animation-delay:{i * 0.1:.1f}s}}" for i in range(6))),
    "loading": ("Loading", "".join(f'<circle cx="{64 + i * 64}" cy="128" r="22" fill="{f}" class="o d{i}"/>' for i, f in enumerate(["#4F7BFF", "#B388FF", "#FF4D6D"])),
                ".o{animation:o 1s ease-in-out infinite}@keyframes o{0%,100%{transform:translateY(0)}50%{transform:translateY(-30px)}}.d1{animation-delay:.15s}.d2{animation-delay:.3s}"),
    "check": ("Check", '<circle cx="128" cy="128" r="100" fill="#3DDC97"/><path d="M78 132l34 34 66-72" fill="none" stroke="#fff" stroke-width="22" stroke-linecap="round" stroke-linejoin="round" pathLength="100" class="k"/>',
              ".k{stroke-dasharray:100;animation:k 1.6s ease-out infinite}@keyframes k{0%{stroke-dashoffset:100}50%,100%{stroke-dashoffset:0}}"),
    "hundred": ("100", big_text("100", 96, "#FF4D6D", 140, "h") + '<path d="M50 176h156M70 196h116" stroke="#FF4D6D" stroke-width="10" stroke-linecap="round" class="u"/>',
                ".h{animation:h 1.2s ease-in-out infinite;transform-box:fill-box;transform-origin:center}@keyframes h{0%,100%{transform:rotate(-4deg)}50%{transform:rotate(4deg) scale(1.06)}}.u{animation:u 1.2s ease-in-out infinite}@keyframes u{50%{transform:translateY(6px)}}"),
    "good-morning": ("Good Morning", '<g class="ry">' + "".join(f'<rect x="124" y="20" width="8" height="30" rx="4" fill="#FFB627" transform="rotate({a} 128 128)"/>' for a in range(0, 360, 45)) + '</g>'
                     '<circle cx="128" cy="128" r="56" fill="#FFD23F"/>' + label("Good morning", 128, 236, 28, "#FFF3B0"),
                     ".ry{animation:ry 6s linear infinite;transform-origin:128px 128px}@keyframes ry{to{transform:rotate(360deg)}}"),
    "good-night": ("Good Night", '<rect width="256" height="256" rx="40" fill="#141B33"/><path d="M156 52a78 78 0 1 0 50 132 64 64 0 1 1-50-132z" fill="#FFE39A"/>'
                   + star(60, 60, 10, "#fff", "t") + star(210, 40, 8, "#fff", "t d1") + star(80, 190, 7, "#fff", "t d2") + label("Good night", 128, 236, 28, "#C7D7FF"),
                   ".t{animation:t 1.8s ease-in-out infinite;transform-box:fill-box;transform-origin:center}@keyframes t{0%,100%{opacity:1}50%{opacity:.2}}.d1{animation-delay:.6s}.d2{animation-delay:1.2s}"),
    "yes": ("Yes", big_text("YES!", 84, "#3DDC97", cls="n"), ".n{animation:n .7s ease-in-out infinite;transform-box:fill-box;transform-origin:center}@keyframes n{0%,100%{transform:translateY(0)}50%{transform:translateY(12px)}}"),
    "no": ("No", big_text("NO", 104, "#FF4D6D", cls="n"), ".n{animation:n .5s ease-in-out infinite;transform-box:fill-box;transform-origin:center}@keyframes n{0%,100%{transform:translateX(0)}25%{transform:translateX(-14px)}75%{transform:translateX(14px)}}"),
    "hug": ("Hug", heart(128, 118, 5.4, cls="b") + label("Hug", 128, 236, 34, "#FFB3C7"),
            ".b{animation:b 1.1s ease-in-out infinite;transform-box:fill-box;transform-origin:center}@keyframes b{0%,100%{transform:scale(1)}30%{transform:scale(1.12)}60%{transform:scale(.96)}}"),
    "wow": ("Wow", "".join(f'<rect x="124" y="14" width="8" height="34" rx="4" fill="#FFD23F" transform="rotate({a} 128 128)" class="z"/>' for a in range(0, 360, 30)) + big_text("WOW", 66, "#FF8A3D", 150),
            ".z{animation:z 1s ease-out infinite}@keyframes z{0%{opacity:0}50%{opacity:1}100%{opacity:0}}"),
    "party-popper": ("Party Popper", '<path d="M40 220l46-120 72 72z" fill="#FFB627" stroke="#2B2A3D" stroke-width="5" stroke-linejoin="round"/>'
                     + "".join(f'<circle cx="{x}" cy="{y}" r="{r}" fill="{f}" class="p d{i % 3}"/>' for i, (x, y, r, f) in enumerate([(150, 70, 10, "#FF4D6D"), (196, 96, 8, "#4F7BFF"), (176, 40, 7, "#3DDC97"), (214, 150, 9, "#B388FF"), (120, 40, 6, "#FFD23F")]))
                     + '<path d="M120 112q30-50 70-56M136 132q46-20 84-6" fill="none" stroke="#FF4D6D" stroke-width="6" stroke-linecap="round" class="p"/>',
                     ".p{animation:p 1.2s ease-out infinite}@keyframes p{0%{transform:translate(-20px,20px);opacity:0}40%{opacity:1}100%{transform:translate(10px,-10px);opacity:0}}.d1{animation-delay:.3s}.d2{animation-delay:.6s}"),
}


def main():
    packs_sql = []
    pack_ns = uuid.UUID("6a1d7c9e-3a41-4c0b-9f1e-2a7c4d5b8e10")
    order = 3
    total = 0
    for c in CHARS:
        os.makedirs(os.path.join(OUT, c.slug), exist_ok=True)
        pid = str(uuid.uuid5(pack_ns, c.slug))
        rows = []
        for key, name, eye, mo in EXPR:
            svg = sticker(c, key, eye, mo, animated=False)
            open(os.path.join(OUT, c.slug, f"{key}.svg"), "w").write(svg)
            rows.append((f"{c.name} {name}", f"/stickers/{c.slug}/{key}.svg", False))
            total += 1
        packs_sql.append((pid, f"{c.name} Friends", f"Original ChatW stickers: {c.name}", f"/stickers/{c.slug}/hello.svg", rows, order))
        order += 1
    # Animated pack: Mochi and Pip in motion.
    os.makedirs(os.path.join(OUT, "motion"), exist_ok=True)
    rows = []
    for c in (CHARS[0], CHARS[3], CHARS[5]):
        for key, name, eye, mo in EXPR:
            if key in ("hello", "love", "lol", "wow", "cry", "sleepy", "party", "thanks", "yay", "angry"):
                svg = sticker(c, key, eye, mo, animated=True)
                fname = f"{c.slug}-{key}.svg"
                open(os.path.join(OUT, "motion", fname), "w").write(svg)
                rows.append((f"{c.name} {name}", f"/stickers/motion/{fname}", True))
                total += 1
    packs_sql.append((str(uuid.uuid5(pack_ns, "motion")), "Friends Motion", "Animated ChatW stickers", "/stickers/motion/mochi-hello.svg", rows, order))
    # Reactions (GIF tab)
    os.makedirs(os.path.join(OUT, "fx"), exist_ok=True)
    gifs = []
    for slug, (name, body, css) in FX.items():
        open(os.path.join(OUT, "fx", f"{slug}.svg"), "w").write(fx(name, body, css))
        gifs.append((name, f"/stickers/fx/{slug}.svg"))
    # Migration
    lines = ["-- Spec 007: many more original stickers (generated by tools/gen_stickers.py).",
             "-- Sticker names are always English. Built-in packs have no author."]
    for pid, pname, desc, cover, rows, _ in packs_sql:
        lines.append(f"INSERT INTO sticker_packs (id, name, description, cover_url) VALUES ('{pid}', '{pname}', '{desc}', '{cover}') "
                     "ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, description = EXCLUDED.description, cover_url = EXCLUDED.cover_url;")
        lines.append(f"DELETE FROM stickers WHERE pack_id = '{pid}';")
        vals = ",\n".join(f"('{pid}', '{n}', '{u}', {'TRUE' if a else 'FALSE'})" for n, u, a in rows)
        lines.append(f"INSERT INTO stickers (pack_id, name, file_url, is_animated) VALUES\n{vals};")
    open(os.path.join(ROOT, "web", "backend", "migrations", "0012_more_stickers.sql"), "w").write("\n".join(lines) + "\n")
    # Rust list of built-in reactions for the GIF tab
    rs = "\n".join(f'    ("{n}", "{u}"),' for n, u in gifs)
    open(os.path.join(ROOT, "app", "src", "components", "fx_gifs.rs"), "w").write(
        "//! Built-in animated reactions (generated by tools/gen_stickers.py). Names are English.\n\n"
        f"pub const FX_GIFS: &[(&str, &str)] = &[\n{rs}\n];\n")
    print("stickers", total, "packs", len(packs_sql), "reactions", len(gifs))


if __name__ == "__main__":
    main()
