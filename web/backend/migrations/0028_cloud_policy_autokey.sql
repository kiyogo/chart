-- Spec 027 Extension: Optional sealed passphrase for unattended scheduled cloud backups
ALTER TABLE cloud_backup_policies
    ADD COLUMN IF NOT EXISTS auto_key_enc BYTEA;
