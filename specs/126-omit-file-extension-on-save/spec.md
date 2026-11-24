# Spec 126: Omit File Extension When Saving and Displaying Media Filenames

## Problem Statement
When saving, uploading, or displaying media files (such as images, videos, audio), the system retained the raw file extension in the stored filename (e.g., `hinh-nen-may-tinh-anime-4k (94).jpg`).
In the media lightbox header and download attributes, this raw extension was shown directly: `hinh-nen-may-tinh-anime-4k (94).jpg`.
The user requested:
*"khi lưu không được dùng lưu luôn đuôi file nữa nhé"* (When saving, do not save/include the file extension as well).

## Goals & Acceptance Criteria
1. **Strip File Extension on Save / Upload**:
   - In `app/web/media_optimizer.js`: apply `stripFileExtension(name)` when creating `fileName` in `processSingleFile`, `pickInstantPreview`, `optimizePastedBytes`, and instant previews.
   - In `app/src/storage/media_opt.rs`: implement `strip_file_extension(name)` to strip trailing extensions when processing media payloads.
   - The MIME type (`mime`) and dedicated format extension (`ext`) remain fully tracked in separate metadata fields for binary sniffer and decoder pipelines.
2. **Strip File Extension on Display & Lightbox**:
   - In `app/src/components/rich.rs`:
     - Lightbox topbar filename (`.lightbox-filename`) displays the clean name without extension (e.g. `hinh-nen-may-tinh-anime-4k (94)` instead of `hinh-nen-may-tinh-anime-4k (94).jpg`).
     - Lightbox download attribute (`dl_name`) uses the clean base name without redundant extension.
   - In `app/src/components/details_panel.rs`: media tile preview titles display clean base names.
   - Backward compatible with existing stored messages that already contain `.jpg`, `.png`, etc.
3. **Verification**:
   - Automated E2E test verifying that uploading/viewing `hinh-nen-may-tinh-anime-4k (94).jpg` renders the title as `hinh-nen-may-tinh-anime-4k (94)` in `.lightbox-filename` with zero extension.
   - All regression suites pass 100%.
