# Spec 105: Clean Simple HTML Legal Documents & Product Vision (Điều khoản, Chính sách, Tiêu chuẩn, Tầm nhìn sản phẩm)

## 1. Problem Statement
The user requested refactoring the legal documents with extreme simplicity:
1. **Remove all hero header boxes** at the top ("Chuẩn mực chung / Tiêu chuẩn cộng đồng / Quy tắc ứng xử..." and the big icon/badge/title/subtitle container).
2. **Remove all "Chuẩn mực 1", "Nguyên tắc 1" tags/badges**.
3. **Write in pure, clean, simple HTML** (clean flat typography, no green checkmarks, no heavy badges, uncluttered document layout).
4. **Remove all parenthetical explanations** like `(...)` (e.g. `(Zero Data Collection)`, `(E2EE)`, `(phishing)`, `(No Backdoors)`, etc.).
5. **Remove all colons `:`** across all titles, items, and descriptions.
6. **Add "Tầm nhìn sản phẩm" (Product Vision)** as an essential core document across Settings -> About and Login screens.

## 2. Acceptance Criteria
- **AC1**: The top hero header box (`.legal-doc-hero`) is completely eliminated from all legal views.
- **AC2**: All badges/tags like "Chuẩn mực 1", "Nguyên tắc 1", checkmark icons are removed.
- **AC3**: Rendered using simple, clean, flat HTML structure (`.legal-simple-doc`, `.legal-simple-item`, `.legal-simple-title`, `.legal-simple-detail`).
- **AC4**: 0 parenthetical annotations `(...)` and 0 colons `:` in any of the documents.
- **AC5**: New document `LegalDocKind::Vision` ("Tầm nhìn sản phẩm") added with 5 powerful vision pillars.
- **AC6**: Settings -> About integrates 6 clean subtabs: `Tổng quan`, `Điều khoản dịch vụ`, `Chính sách bảo mật`, `Tiêu chuẩn cộng đồng`, `Tầm nhìn sản phẩm`, `Hướng dẫn`.
- **AC7**: Login screen provides 1-click access to all 4 documents in clean modal view.
- **AC8**: Playwright E2E automated test verifies removal of hero banner, removal of colons/parentheses, presence of Product Vision, and flawless rendering.
