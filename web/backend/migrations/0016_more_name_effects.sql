-- Spec 009: 10 more name effects. Rollback: restore the 0010 list (after resetting rows to 'none').
ALTER TABLE users DROP CONSTRAINT IF EXISTS users_name_effect_check;
ALTER TABLE users ADD CONSTRAINT users_name_effect_check CHECK (name_effect IN (
    'none', 'gradient', 'shimmer', 'rainbow', 'neon', 'wave',
    'glow', 'sparkle', 'glitch', 'pulse', 'typing', 'reveal', 'cycle', 'outline', 'holo', 'breathe'
));
