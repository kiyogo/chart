-- Clean channel, category, department, project, and folder names:
-- remove hyphens/ampersands and enforce sentence case.

UPDATE server_categories SET name = 'Không gian doanh nghiệp' WHERE name = 'Không gian Doanh nghiệp';
UPDATE server_categories SET name = 'Phòng họp trực tuyến' WHERE name = 'Phòng họp Trực tuyến';

UPDATE channels SET name = 'Thông báo nội bộ' WHERE name = 'thông-báo-nội-bộ';
UPDATE channels SET name = 'Ban điều hành' WHERE name = 'ban-điều-hành';
UPDATE channels SET name = 'Nhân sự hành chính' WHERE name = 'nhân-sự-hành-chính';
UPDATE channels SET name = 'Kỹ thuật sản phẩm' WHERE name = 'kỹ-thuật-sản-phẩm';
UPDATE channels SET name = 'Kinh doanh vận hành' WHERE name = 'kinh-doanh-vận-hành';
UPDATE channels SET name = 'Tự động hóa' WHERE name = 'tự-động-hóa';
UPDATE channels SET name = 'Phòng họp điều hành' WHERE name = 'Phòng họp Điều hành';
UPDATE channels SET name = 'Phòng họp nhanh' WHERE name = 'Phòng họp Nhanh';

UPDATE enterprise_departments SET name = 'Ban điều hành' WHERE name = 'Ban Điều hành';
UPDATE enterprise_departments SET name = 'Phòng kỹ thuật sản phẩm' WHERE name = 'Phòng Kỹ thuật & Sản phẩm';
UPDATE enterprise_departments SET name = 'Phòng nhân sự hành chính' WHERE name = 'Phòng Nhân sự & Hành chính';
UPDATE enterprise_departments SET name = 'Phòng kinh doanh vận hành' WHERE name = 'Phòng Kinh doanh & Vận hành';

UPDATE enterprise_projects SET name = 'Vận hành chuyển đổi số doanh nghiệp' WHERE name = 'Vận hành & Chuyển đổi số Doanh nghiệp';
UPDATE enterprise_projects SET name = 'Phát triển nền tảng bảo mật hệ thống' WHERE name = 'Phát triển Nền tảng & Bảo mật Hệ thống';

UPDATE enterprise_folders SET name = 'Quy chế và sổ tay nhân sự' WHERE name = 'Quy chế & Sổ tay Nhân sự';
UPDATE enterprise_folders SET name = 'Tài liệu dự án và kỹ thuật' WHERE name = 'Tài liệu Dự án & Kỹ thuật';
UPDATE enterprise_folders SET name = 'Biểu mẫu và hợp đồng' WHERE name = 'Biểu mẫu & Hợp đồng';
