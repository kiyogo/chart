-- Add avatar_url column to users table for Google profile picture
ALTER TABLE users ADD COLUMN IF NOT EXISTS avatar_url TEXT;
