-- Spec 037: Full ChatW Plugin Platform, Sandbox Runtime, Permissions, Discord Webhooks, Sync & Marketplace

CREATE TABLE IF NOT EXISTS plugin_publishers (
    id UUID PRIMARY KEY,
    user_id UUID UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    slug TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    website_url TEXT,
    bio TEXT,
    verification_status TEXT NOT NULL DEFAULT 'unverified',
    verified_at TIMESTAMPTZ,
    verified_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS plugin_packages (
    id UUID PRIMARY KEY,
    plugin_key TEXT NOT NULL UNIQUE,
    publisher_id UUID NOT NULL REFERENCES plugin_publishers(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    summary TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL DEFAULT 'utility',
    icon_url TEXT NOT NULL DEFAULT '',
    screenshots JSONB NOT NULL DEFAULT '[]'::jsonb,
    video_url TEXT,
    platforms JSONB NOT NULL DEFAULT '["web","linux","windows","macos"]'::jsonb,
    is_public BOOLEAN NOT NULL DEFAULT true,
    is_builtin BOOLEAN NOT NULL DEFAULT false,
    latest_version TEXT NOT NULL DEFAULT '1.0.0',
    install_count BIGINT NOT NULL DEFAULT 0,
    rating_avg DOUBLE PRECISION NOT NULL DEFAULT 0,
    rating_count INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_plugin_packages_cat ON plugin_packages(category, is_public);
CREATE INDEX IF NOT EXISTS idx_plugin_packages_pub ON plugin_packages(publisher_id);

CREATE TABLE IF NOT EXISTS plugin_versions (
    id UUID PRIMARY KEY,
    package_id UUID NOT NULL REFERENCES plugin_packages(id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    changelog TEXT NOT NULL DEFAULT '',
    runtime TEXT NOT NULL DEFAULT 'js',
    sdk_version TEXT NOT NULL DEFAULT '1.0.0',
    manifest JSONB NOT NULL DEFAULT '{}'::jsonb,
    bundle_code TEXT NOT NULL DEFAULT '',
    wasm_base64 TEXT NOT NULL DEFAULT '',
    ui_html TEXT NOT NULL DEFAULT '',
    sha256 TEXT NOT NULL,
    signature TEXT,
    size_bytes INT NOT NULL DEFAULT 0,
    requested_permissions JSONB NOT NULL DEFAULT '[]'::jsonb,
    sensitive_approval_status TEXT NOT NULL DEFAULT 'not_required',
    approved_permissions JSONB NOT NULL DEFAULT '[]'::jsonb,
    approved_by UUID REFERENCES users(id) ON DELETE SET NULL,
    approved_at TIMESTAMPTZ,
    revoked BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(package_id, version)
);

CREATE INDEX IF NOT EXISTS idx_plugin_versions_pkg ON plugin_versions(package_id, created_at DESC);

CREATE TABLE IF NOT EXISTS plugin_installations (
    id UUID PRIMARY KEY,
    package_id UUID NOT NULL REFERENCES plugin_packages(id) ON DELETE CASCADE,
    version_id UUID NOT NULL REFERENCES plugin_versions(id) ON DELETE RESTRICT,
    scope_type TEXT NOT NULL,
    scope_id UUID NOT NULL,
    installed_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id TEXT,
    enabled BOOLEAN NOT NULL DEFAULT false,
    display_order INT NOT NULL DEFAULT 0,
    ui_slot TEXT NOT NULL DEFAULT 'sidebar',
    allowed_channels JSONB NOT NULL DEFAULT '[]'::jsonb,
    blocked_channels JSONB NOT NULL DEFAULT '[]'::jsonb,
    allowed_roles JSONB NOT NULL DEFAULT '[]'::jsonb,
    settings JSONB NOT NULL DEFAULT '{}'::jsonb,
    sync_version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(package_id, scope_type, scope_id)
);

CREATE INDEX IF NOT EXISTS idx_plugin_installations_scope ON plugin_installations(scope_type, scope_id);
CREATE INDEX IF NOT EXISTS idx_plugin_installations_user ON plugin_installations(installed_by);

CREATE TABLE IF NOT EXISTS plugin_grants (
    id UUID PRIMARY KEY,
    installation_id UUID NOT NULL REFERENCES plugin_installations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    permission TEXT NOT NULL,
    scope_type TEXT NOT NULL DEFAULT 'global',
    scope_id UUID,
    device_only BOOLEAN NOT NULL DEFAULT false,
    expires_at TIMESTAMPTZ,
    granted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_plugin_grants_inst ON plugin_grants(installation_id, user_id);
CREATE INDEX IF NOT EXISTS idx_plugin_grants_user ON plugin_grants(user_id, revoked_at);

CREATE TABLE IF NOT EXISTS plugin_storage (
    installation_id UUID NOT NULL REFERENCES plugin_installations(id) ON DELETE CASCADE,
    owner_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    storage_key TEXT NOT NULL,
    value JSONB NOT NULL DEFAULT 'null'::jsonb,
    is_secret BOOLEAN NOT NULL DEFAULT false,
    ciphertext BYTEA,
    size_bytes INT NOT NULL DEFAULT 0,
    syncable BOOLEAN NOT NULL DEFAULT true,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY(installation_id, owner_user_id, storage_key)
);

CREATE TABLE IF NOT EXISTS plugin_webhooks (
    id UUID PRIMARY KEY,
    direction TEXT NOT NULL,
    scope_type TEXT NOT NULL,
    scope_id UUID NOT NULL,
    channel_id UUID,
    installation_id UUID REFERENCES plugin_installations(id) ON DELETE SET NULL,
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    avatar_url TEXT,
    token_hash BYTEA,
    token_hint TEXT NOT NULL DEFAULT '',
    target_url TEXT,
    secret_ciphertext BYTEA,
    subscribed_events JSONB NOT NULL DEFAULT '[]'::jsonb,
    enabled BOOLEAN NOT NULL DEFAULT true,
    rate_limit_per_min INT NOT NULL DEFAULT 30,
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_plugin_webhooks_scope ON plugin_webhooks(direction, scope_type, scope_id);
CREATE INDEX IF NOT EXISTS idx_plugin_webhooks_channel ON plugin_webhooks(channel_id);

CREATE TABLE IF NOT EXISTS plugin_webhook_deliveries (
    id UUID PRIMARY KEY,
    webhook_id UUID NOT NULL REFERENCES plugin_webhooks(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    event_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    http_status INT,
    attempt_count INT NOT NULL DEFAULT 0,
    request_bytes INT NOT NULL DEFAULT 0,
    response_preview TEXT NOT NULL DEFAULT '',
    error_message TEXT NOT NULL DEFAULT '',
    payload_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    next_retry_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ,
    UNIQUE(webhook_id, event_id)
);

CREATE INDEX IF NOT EXISTS idx_plugin_wh_deliv_retry ON plugin_webhook_deliveries(status, next_retry_at);
CREATE INDEX IF NOT EXISTS idx_plugin_wh_deliv_wh ON plugin_webhook_deliveries(webhook_id, created_at DESC);

CREATE TABLE IF NOT EXISTS plugin_reviews (
    id UUID PRIMARY KEY,
    package_id UUID NOT NULL REFERENCES plugin_packages(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rating INT NOT NULL CHECK (rating BETWEEN 1 AND 5),
    comment TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(package_id, user_id)
);

CREATE TABLE IF NOT EXISTS plugin_reports (
    id UUID PRIMARY KEY,
    package_id UUID NOT NULL REFERENCES plugin_packages(id) ON DELETE CASCADE,
    reporter_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reason TEXT NOT NULL,
    details TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'open',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS plugin_audit_logs (
    id UUID PRIMARY KEY,
    installation_id UUID,
    package_id UUID,
    actor_user_id UUID,
    scope_type TEXT NOT NULL DEFAULT 'user',
    scope_id UUID,
    action TEXT NOT NULL,
    permission TEXT,
    detail JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_plugin_audit_actor ON plugin_audit_logs(actor_user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_plugin_audit_inst ON plugin_audit_logs(installation_id, created_at DESC);

CREATE TABLE IF NOT EXISTS plugin_sync_cursors (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    sync_seq BIGINT NOT NULL DEFAULT 1,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS plugin_bots (
    id UUID PRIMARY KEY,
    installation_id UUID NOT NULL REFERENCES plugin_installations(id) ON DELETE CASCADE,
    bot_id UUID NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(installation_id, bot_id)
);
