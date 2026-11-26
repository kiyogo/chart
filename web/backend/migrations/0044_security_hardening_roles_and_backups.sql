-- Spec 169: Least-privilege database access for sessions and user_key_backups
-- Prevents the runtime app role (`chatw_app`) from directly reading/dumping `user_key_backups`
-- or writing/forging rows in `sessions`, while preserving per-user authenticated backup RPCs.

CREATE OR REPLACE FUNCTION public.fetch_user_key_backup(p_user_id UUID)
RETURNS TABLE (
    wrapped_keys_b64 TEXT,
    kdf_salt_b64 TEXT,
    kdf_iterations INT,
    kdf_alg TEXT,
    kdf_mem_kib INT,
    kdf_time_cost INT,
    kdf_parallelism INT,
    root_key_wrapped_b64 TEXT,
    fp_b64 TEXT,
    updated_at TIMESTAMPTZ
)
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    SELECT
        b.wrapped_keys_b64,
        b.kdf_salt_b64,
        b.kdf_iterations,
        b.kdf_alg,
        b.kdf_mem_kib,
        b.kdf_time_cost,
        b.kdf_parallelism,
        b.root_key_wrapped_b64,
        encode(b.key_fingerprint, 'base64') AS fp_b64,
        b.updated_at
    FROM public.user_key_backups b
    WHERE b.user_id = p_user_id;
$$;

CREATE OR REPLACE FUNCTION public.upsert_user_key_backup(
    p_user_id UUID,
    p_wrapped_keys_b64 TEXT,
    p_kdf_salt_b64 TEXT,
    p_kdf_iterations INT,
    p_key_fingerprint BYTEA,
    p_kdf_alg TEXT,
    p_kdf_mem_kib INT,
    p_kdf_time_cost INT,
    p_kdf_parallelism INT,
    p_root_key_wrapped_b64 TEXT
)
RETURNS VOID
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    INSERT INTO public.user_key_backups
        (user_id, wrapped_keys_b64, kdf_salt_b64, kdf_iterations, key_fingerprint,
         kdf_alg, kdf_mem_kib, kdf_time_cost, kdf_parallelism, root_key_wrapped_b64, updated_at)
    VALUES
        (p_user_id, p_wrapped_keys_b64, p_kdf_salt_b64, p_kdf_iterations, p_key_fingerprint,
         p_kdf_alg, p_kdf_mem_kib, p_kdf_time_cost, p_kdf_parallelism, p_root_key_wrapped_b64, now())
    ON CONFLICT (user_id) DO UPDATE SET
        wrapped_keys_b64 = EXCLUDED.wrapped_keys_b64,
        kdf_salt_b64 = EXCLUDED.kdf_salt_b64,
        kdf_iterations = EXCLUDED.kdf_iterations,
        key_fingerprint = EXCLUDED.key_fingerprint,
        kdf_alg = EXCLUDED.kdf_alg,
        kdf_mem_kib = EXCLUDED.kdf_mem_kib,
        kdf_time_cost = EXCLUDED.kdf_time_cost,
        kdf_parallelism = EXCLUDED.kdf_parallelism,
        root_key_wrapped_b64 = EXCLUDED.root_key_wrapped_b64,
        updated_at = now();
$$;

REVOKE ALL ON FUNCTION public.fetch_user_key_backup(UUID) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.upsert_user_key_backup(UUID, TEXT, TEXT, INT, BYTEA, TEXT, INT, INT, INT, TEXT) FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'chatw_app') THEN
        REVOKE ALL ON TABLE public.sessions FROM chatw_app;
        REVOKE ALL ON TABLE public.user_key_backups FROM chatw_app;
        GRANT EXECUTE ON FUNCTION public.fetch_user_key_backup(UUID) TO chatw_app;
        GRANT EXECUTE ON FUNCTION public.upsert_user_key_backup(UUID, TEXT, TEXT, INT, BYTEA, TEXT, INT, INT, INT, TEXT) TO chatw_app;
    END IF;
END $$;
