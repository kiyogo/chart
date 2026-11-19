# Spec 008: OpenMoji sticker set

## Problem
The user asked for as many stickers and emoji as possible, taken from the internet.
After approval, the OpenMoji 17.0.0 release (`openmoji-svg-color.zip`, 5.1 MB,
CC BY-SA 4.0) was downloaded and verified against the release SHA-256.

## Goals
- Ship the OpenMoji artwork as a built-in sticker set, with English names
  (user rule: sticker and emoji names are English) and search in English and Vietnamese.
- Keep the sticker API light: about 2,000 extra stickers must not grow `/api/stickers/packs`.
- Every SVG served from our origin must be safe to open directly (no script, no external references).
- Give credit as the license requires.

## Acceptance criteria
- AC1 The sticker picker has an "OpenMoji" tab with category chips (Smileys, People, Animals,
  Food, Activities, Travel, Objects, Symbols, Flags, Extras) and at least 2,000 stickers.
- AC2 Searching "mèo", "cat", "cà phê" or "pizza" finds OpenMoji stickers.
- AC3 Sending an OpenMoji sticker works in E2EE DMs and groups (the body is the sticker path) and
  renders on both sides.
- AC4 Typing a message that is exactly a sticker name or keyword ("pizza", "mèo") suggests it.
- AC5 Each SVG passes an allow-list of elements and attributes; the importer rejects anything else.
- AC6 Credit "OpenMoji · CC BY-SA 4.0" is visible in the picker, in the guide FAQ, in
  `app/public/stickers/openmoji/LICENSE.md` and in `THIRD_PARTY_NOTICES.md`.

## Out of scope
- Skin-tone variants (2,035 files): the emoji picker already offers skin tones.
- The 367 OpenMoji-only drawings (private-use code points) and 78 OpenMoji combinations
  without a Unicode name: their names are in OpenMoji's `extras-openmoji.csv` /
  `extras-unicode.csv`, which need a separate download approval.
