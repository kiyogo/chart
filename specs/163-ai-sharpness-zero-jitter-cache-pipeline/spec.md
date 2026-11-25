# Spec 163 — Stronger AI Super-Resolution, Zero-Jitter Instant Image Send, Silent Sender Cache Handoff & Cross-Platform Adaptive Storage Cache (with 15-Day Server Cold Compression Auto-Restore)

## Problem Statement
1. **AI Sharpening Strength**: Current WebGL2 finalize shader uses conservative single-scale 1px CAS (`uColor1.z = 0.50` for `NomosUniSpanX2`) clipped tightly to 3×3 `[mnRgb, mxRgb]`, leaving upscaled 2×/4× photos softer than desired.
2. **Message-Stream Jitter on Image Send**: Sending an image causes messages above in the stream to jump/jitter multiple times because:
   - `PendingSenderMedia` uses a different row height/pill badge and is removed before inserting the real message.
   - `messages.rs` keys rows by `msg.id`, so `local-{uuid}` -> `{uuid}` unmounts and remounts `<RichImage>` from scratch.
   - `scroll_chat_to_bottom(false)` runs smooth scroll animations that collide with `ResizeObserver` / `MutationObserver` instant pinning and duplicate ack/WS scroll calls.
   - `senderOriginalSession` mints a second `blob:` URL instead of reusing the already-decoded device file preview URL.
3. **Uncompressed Upload & Silent Local Cache Handoff**:
   - Sender must immediately view the exact local file from their device (zero badge/spinner overlay, completely seamless), upload original bytes uncompressed to the server, save into sender's local app cache, and silently switch to the cached copy without any UI flash or status indicator.
4. **Receiver Persistent Cache & 15-Day Cold Compression Recovery**:
   - On Web, `core_bridge.js` only stored downloaded images in volatile RAM (`blobPool`), bypassing persistent OPFS/IndexedDB caching.
   - When the server compresses an image unused for >= 15 days (`cold_compressed_at IS NOT NULL`), responses must signal `X-ChatW-Cold-Compressed: 1` (and `;cold15d=1` over binary RPC) so any sender or receiver who lost local cache/device file or switched to a new device automatically downloads the server copy, runs strong AI sharpening, and caches the restored result locally.
5. **Cross-Platform Device-Storage-Aware Adaptive Cache Quota**:
   - Must dynamically bound cache size proportional to real free disk/quota space across Web (`navigator.storage.estimate()`, including Auto=0 mode), Linux/macOS/Android/iOS (`statvfs`), and Windows (`GetDiskFreeSpaceExW` instead of a hardcoded 16 GiB constant), with automatic W-TinyLFU + size-aware eviction when storage tightens or temporary files expire.

## Acceptance Criteria
1. Dual-scale edge-aware luminance sharpening (1px CAS + 2px micro-clarity) boosts crispness significantly across WebGL2 neural SR (SPAN/Real-ESRGAN/AnimeVideoV3), FSR1 RCAS, and CPU/Worker CAS while preserving skin tones and avoiding chromatic speckles/halos.
2. Sending an image in DM, Group, or Channel renders the sender's local file immediately at locked exact dimensions with zero visible "optimizing/sending" badge, uploads uncompressed, saves to local cache, and transitions silently with zero unmount/remount and zero jitter of messages above.
3. Receiver downloads from server -> persists into local app cache (OPFS/IDB on Web, local CAS on Desktop/Mobile) respecting real device free-space quota; subsequent opens load directly from local cache.
4. Images untouched on server for >= 15 days compress into AVIF and emit `X-ChatW-Cold-Compressed: 1`; on cache miss / lost original / new device, client detects cold compression and automatically runs strong AI sharpening + caches the output.
5. All workspace tests (`chatw-media-core`, `kiyo-kdata`, `chatw --lib --tests`), `cargo check -p chatw-app`, `node --check app/web/*.js`, and `./app/build-web.sh` pass 100%, and changes are pushed to `origin main`.
