-- Spec 161: Security constraints, cascade consistency, and missing FK/GC/query indexes.

-- 1. Ensure sessions.id is non-null & unique/indexed for per-session revocation/binding.
UPDATE sessions SET id = gen_random_uuid() WHERE id IS NULL;
ALTER TABLE sessions ALTER COLUMN id SET NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS sessions_id_uidx ON sessions (id);

-- 2. Fix conflicting/unsafe ON DELETE actions.
ALTER TABLE plugin_installations
    DROP CONSTRAINT IF EXISTS plugin_installations_version_id_fkey;
ALTER TABLE plugin_installations
    ADD CONSTRAINT plugin_installations_version_id_fkey
    FOREIGN KEY (version_id) REFERENCES plugin_versions(id) ON DELETE CASCADE;

ALTER TABLE media_cdc_recipes
    DROP CONSTRAINT IF EXISTS media_cdc_recipes_chunk_blake3_fkey;
ALTER TABLE media_cdc_recipes
    ADD CONSTRAINT media_cdc_recipes_chunk_blake3_fkey
    FOREIGN KEY (chunk_blake3) REFERENCES media_cdc_chunks(chunk_blake3) ON DELETE CASCADE;

ALTER TABLE sticker_packs
    DROP CONSTRAINT IF EXISTS sticker_packs_author_id_fkey;
ALTER TABLE sticker_packs
    ADD CONSTRAINT sticker_packs_author_id_fkey
    FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE CASCADE;

-- 3. Critical media_objects & media_references FK/GC indexes.
CREATE INDEX IF NOT EXISTS media_references_object_idx ON media_references (object_id) WHERE object_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS media_references_opt_obj_idx ON media_references (optimized_object_id) WHERE optimized_object_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS media_references_thumb_obj_idx ON media_references (thumb_object_id) WHERE thumb_object_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS media_references_forward_ref_idx ON media_references (forward_from_ref_id) WHERE forward_from_ref_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS media_references_owner_plain_sha_idx ON media_references (owner_id, sha256_plain) WHERE sha256_plain IS NOT NULL;

CREATE INDEX IF NOT EXISTS media_objects_variant_of_idx ON media_objects (variant_of) WHERE variant_of IS NOT NULL;
CREATE INDEX IF NOT EXISTS media_objects_backend_key_idx ON media_objects (storage_backend, storage_key);
CREATE INDEX IF NOT EXISTS media_objects_backend_created_idx ON media_objects (storage_backend, created_at);

-- 4. Messaging & sync hot-path indexes.
CREATE INDEX IF NOT EXISTS messages_sender_seq_idx ON messages (sender_id, seq);
CREATE INDEX IF NOT EXISTS messages_receiver_seq_idx ON messages (receiver_id, seq);
CREATE INDEX IF NOT EXISTS messages_reply_to_idx ON messages (reply_to_id) WHERE reply_to_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS channel_messages_sender_idx ON channel_messages (sender_id);
CREATE INDEX IF NOT EXISTS channel_messages_reply_to_idx ON channel_messages (reply_to_id) WHERE reply_to_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS channel_messages_chan_created_idx ON channel_messages (channel_id, created_at DESC);

CREATE INDEX IF NOT EXISTS group_messages_sender_idx ON group_messages (sender_id);
CREATE INDEX IF NOT EXISTS group_messages_reply_to_idx ON group_messages (reply_to_id) WHERE reply_to_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS group_messages_bot_idx ON group_messages (bot_id) WHERE bot_id IS NOT NULL;

-- 5. Security, audit, transfer GC, social & enterprise FK indexes.
CREATE INDEX IF NOT EXISTS audit_logs_created_desc_idx ON audit_logs (created_at DESC);
CREATE INDEX IF NOT EXISTS transfer_sessions_status_exp_idx ON transfer_sessions (status, expires_at);
CREATE INDEX IF NOT EXISTS twofa_challenges_user_idx ON twofa_challenges (user_id);
CREATE INDEX IF NOT EXISTS twofa_challenges_exp_idx ON twofa_challenges (expires_at);
CREATE INDEX IF NOT EXISTS passkey_challenges_user_idx ON passkey_challenges (user_id);
CREATE INDEX IF NOT EXISTS passkey_challenges_exp_idx ON passkey_challenges (expires_at);
CREATE INDEX IF NOT EXISTS cloud_oauth_states_user_idx ON cloud_oauth_states (user_id);
CREATE INDEX IF NOT EXISTS cloud_oauth_states_exp_idx ON cloud_oauth_states (expires_at);

CREATE INDEX IF NOT EXISTS feed_posts_repost_idx ON feed_posts (repost_of_id) WHERE repost_of_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS server_member_roles_role_idx ON server_member_roles (role_id);
CREATE INDEX IF NOT EXISTS profile_guestbook_author_idx ON profile_guestbook (author_id);
CREATE INDEX IF NOT EXISTS forum_threads_author_idx ON forum_threads (author_id);

CREATE INDEX IF NOT EXISTS enterprise_tasks_parent_idx ON enterprise_tasks (parent_task_id) WHERE parent_task_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS enterprise_tasks_project_idx ON enterprise_tasks (project_id) WHERE project_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS enterprise_tasks_source_msg_idx ON enterprise_tasks (source_message_id) WHERE source_message_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS enterprise_docs_folder_idx ON enterprise_documents (folder_id) WHERE folder_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS enterprise_doc_versions_doc_idx ON enterprise_document_versions (document_id);
CREATE INDEX IF NOT EXISTS enterprise_doc_shares_doc_idx ON enterprise_document_shares (document_id);
CREATE INDEX IF NOT EXISTS enterprise_workflow_runs_wf_idx ON enterprise_workflow_runs (workflow_id);
