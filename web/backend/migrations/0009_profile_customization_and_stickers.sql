-- 0009: Profile customization (theme colors, avatar decoration, profile effect)
-- and a working built-in sticker pack.

ALTER TABLE users ADD COLUMN IF NOT EXISTS profile_color TEXT
    CHECK (profile_color IS NULL OR profile_color ~ '^#[0-9A-Fa-f]{6}$');
ALTER TABLE users ADD COLUMN IF NOT EXISTS profile_accent TEXT
    CHECK (profile_accent IS NULL OR profile_accent ~ '^#[0-9A-Fa-f]{6}$');
ALTER TABLE users ADD COLUMN IF NOT EXISTS avatar_decoration TEXT NOT NULL DEFAULT 'none'
    CHECK (avatar_decoration IN ('none', 'halo', 'crown', 'sprout', 'stars', 'cat'));
ALTER TABLE users ADD COLUMN IF NOT EXISTS profile_effect TEXT NOT NULL DEFAULT 'none'
    CHECK (profile_effect IN ('none', 'sparkle', 'snow', 'aurora', 'bubbles', 'confetti'));

-- The 0008 seed stored emoji characters in file_url (not images) and a cover on an
-- external host blocked by the CSP. Replace them with the original SVG stickers
-- shipped by the client under /stickers/.
DELETE FROM stickers WHERE pack_id = '00000000-0000-0000-0000-000000000001';
UPDATE sticker_packs
SET name = 'Bóng Chat',
    description = 'Bộ sticker gốc của ChatW',
    cover_url = '/stickers/bong-hello.svg'
WHERE id = '00000000-0000-0000-0000-000000000001';
INSERT INTO stickers (pack_id, name, file_url) VALUES
('00000000-0000-0000-0000-000000000001', 'Xin chào', '/stickers/bong-hello.svg'),
('00000000-0000-0000-0000-000000000001', 'Yêu quá', '/stickers/bong-love.svg'),
('00000000-0000-0000-0000-000000000001', 'Cười lớn', '/stickers/bong-laugh.svg'),
('00000000-0000-0000-0000-000000000001', 'Tuyệt vời', '/stickers/bong-ok.svg'),
('00000000-0000-0000-0000-000000000001', 'Bất ngờ', '/stickers/bong-wow.svg'),
('00000000-0000-0000-0000-000000000001', 'Buồn', '/stickers/bong-sad.svg'),
('00000000-0000-0000-0000-000000000001', 'Buồn ngủ', '/stickers/bong-sleepy.svg'),
('00000000-0000-0000-0000-000000000001', 'Ăn mừng', '/stickers/bong-party.svg'),
('00000000-0000-0000-0000-000000000001', 'Cảm ơn', '/stickers/bong-thanks.svg'),
('00000000-0000-0000-0000-000000000001', 'Suy nghĩ', '/stickers/bong-think.svg');
