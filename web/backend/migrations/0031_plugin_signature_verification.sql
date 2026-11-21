-- Spec 051: Cryptographic Publisher Public Key & Signature Verification Metadata

ALTER TABLE plugin_publishers
    ADD COLUMN IF NOT EXISTS public_key TEXT;

ALTER TABLE plugin_versions
    ADD COLUMN IF NOT EXISTS signature_verified BOOLEAN NOT NULL DEFAULT false;
