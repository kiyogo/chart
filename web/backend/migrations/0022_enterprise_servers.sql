-- Spec 015: Integrated Enterprise Mode inside existing Servers
-- Additive & backwards-compatible migration: never modifies or drops existing user/server/message/channel data.

ALTER TABLE servers
    ADD COLUMN IF NOT EXISTS is_enterprise BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS enterprise_disabled_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS retention_locked BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE server_roles
    ADD COLUMN IF NOT EXISTS enterprise_permissions BIGINT NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS server_enterprise_configs (
    server_id           UUID PRIMARY KEY REFERENCES servers(id) ON DELETE CASCADE,
    enabled_modules     JSONB NOT NULL DEFAULT '["overview","hr","departments","tasks","projects","calendar","meetings","attendance","leave","approvals","documents","announcements","reports"]',
    module_order        JSONB NOT NULL DEFAULT '["overview","hr","departments","tasks","projects","calendar","meetings","attendance","leave","approvals","documents","announcements","reports","ai","automation"]',
    general_settings    JSONB NOT NULL DEFAULT '{}',
    display_settings    JSONB NOT NULL DEFAULT '{}',
    security_policy     JSONB NOT NULL DEFAULT '{}',
    hr_policy           JSONB NOT NULL DEFAULT '{}',
    attendance_policy   JSONB NOT NULL DEFAULT '{}',
    leave_policy        JSONB NOT NULL DEFAULT '{}',
    approval_policy     JSONB NOT NULL DEFAULT '{}',
    task_policy         JSONB NOT NULL DEFAULT '{}',
    project_policy      JSONB NOT NULL DEFAULT '{}',
    document_policy     JSONB NOT NULL DEFAULT '{}',
    notification_policy JSONB NOT NULL DEFAULT '{}',
    ai_config           JSONB NOT NULL DEFAULT '{"enabled":false}',
    privacy_policy      JSONB NOT NULL DEFAULT '{}',
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS enterprise_departments (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id         UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    parent_id         UUID REFERENCES enterprise_departments(id) ON DELETE SET NULL,
    name              TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 80),
    code              TEXT CHECK (code IS NULL OR char_length(code) <= 24),
    description       TEXT CHECK (description IS NULL OR char_length(description) <= 500),
    manager_user_id   UUID REFERENCES users(id) ON DELETE SET NULL,
    linked_channel_id UUID REFERENCES channels(id) ON DELETE SET NULL,
    branch_name       TEXT CHECK (branch_name IS NULL OR char_length(branch_name) <= 80),
    default_role_id   UUID REFERENCES server_roles(id) ON DELETE SET NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (server_id, name)
);
CREATE INDEX IF NOT EXISTS ent_dept_server_idx ON enterprise_departments (server_id, created_at);

CREATE TABLE IF NOT EXISTS enterprise_department_members (
    department_id UUID NOT NULL REFERENCES enterprise_departments(id) ON DELETE CASCADE,
    server_id     UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (department_id, user_id)
);
CREATE INDEX IF NOT EXISTS ent_dept_members_srv_user_idx ON enterprise_department_members (server_id, user_id);

CREATE TABLE IF NOT EXISTS enterprise_employees (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id         UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    employee_code     TEXT CHECK (employee_code IS NULL OR char_length(employee_code) <= 32),
    member_kind       TEXT NOT NULL DEFAULT 'employee' CHECK (member_kind IN ('employee', 'guest', 'partner', 'bot')),
    full_name         TEXT NOT NULL CHECK (char_length(full_name) BETWEEN 1 AND 100),
    work_email        TEXT CHECK (work_email IS NULL OR char_length(work_email) <= 120),
    work_phone        TEXT CHECK (work_phone IS NULL OR char_length(work_phone) <= 40),
    job_title         TEXT CHECK (job_title IS NULL OR char_length(job_title) <= 80),
    position_level    TEXT CHECK (position_level IS NULL OR char_length(position_level) <= 40),
    department_id     UUID REFERENCES enterprise_departments(id) ON DELETE SET NULL,
    manager_user_id   UUID REFERENCES users(id) ON DELETE SET NULL,
    branch            TEXT CHECK (branch IS NULL OR char_length(branch) <= 80),
    work_location     TEXT CHECK (work_location IS NULL OR char_length(work_location) <= 100),
    work_mode         TEXT NOT NULL DEFAULT 'onsite' CHECK (work_mode IN ('onsite', 'remote', 'hybrid')),
    contract_type     TEXT CHECK (contract_type IS NULL OR char_length(contract_type) <= 50),
    employment_status TEXT NOT NULL DEFAULT 'active' CHECK (employment_status IN ('active', 'onboarding', 'suspended', 'offboarded')),
    start_date        TEXT,
    end_date          TEXT,
    timezone          TEXT NOT NULL DEFAULT 'Asia/Ho_Chi_Minh',
    work_schedule     TEXT NOT NULL DEFAULT '08:30-17:30',
    skills            TEXT[] NOT NULL DEFAULT '{}',
    internal_notes    TEXT CHECK (internal_notes IS NULL OR char_length(internal_notes) <= 2000),
    custom_fields     JSONB NOT NULL DEFAULT '{}',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (server_id, user_id)
);
CREATE INDEX IF NOT EXISTS ent_emp_server_dept_idx ON enterprise_employees (server_id, department_id, employment_status);

CREATE TABLE IF NOT EXISTS enterprise_projects (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id         UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    department_id     UUID REFERENCES enterprise_departments(id) ON DELETE SET NULL,
    name              TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 120),
    description       TEXT CHECK (description IS NULL OR char_length(description) <= 2000),
    status            TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('planning', 'active', 'on_hold', 'completed', 'archived')),
    priority          TEXT NOT NULL DEFAULT 'medium' CHECK (priority IN ('low', 'medium', 'high', 'urgent')),
    manager_user_id   UUID REFERENCES users(id) ON DELETE SET NULL,
    linked_channel_id UUID REFERENCES channels(id) ON DELETE SET NULL,
    start_date        TEXT,
    due_date          TEXT,
    milestones        JSONB NOT NULL DEFAULT '[]',
    member_ids        UUID[] NOT NULL DEFAULT '{}',
    created_by        UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_proj_server_idx ON enterprise_projects (server_id, status, created_at DESC);

CREATE TABLE IF NOT EXISTS enterprise_tasks (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id         UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    project_id        UUID REFERENCES enterprise_projects(id) ON DELETE SET NULL,
    department_id     UUID REFERENCES enterprise_departments(id) ON DELETE SET NULL,
    parent_task_id    UUID REFERENCES enterprise_tasks(id) ON DELETE CASCADE,
    title             TEXT NOT NULL CHECK (char_length(title) BETWEEN 1 AND 160),
    description       TEXT CHECK (description IS NULL OR char_length(description) <= 4000),
    status            TEXT NOT NULL DEFAULT 'todo' CHECK (status IN ('todo', 'in_progress', 'review', 'done', 'archived')),
    priority          TEXT NOT NULL DEFAULT 'medium' CHECK (priority IN ('low', 'medium', 'high', 'urgent')),
    assignee_id       UUID REFERENCES users(id) ON DELETE SET NULL,
    participant_ids   UUID[] NOT NULL DEFAULT '{}',
    watcher_ids       UUID[] NOT NULL DEFAULT '{}',
    labels            TEXT[] NOT NULL DEFAULT '{}',
    checklist         JSONB NOT NULL DEFAULT '[]',
    depends_on_ids    UUID[] NOT NULL DEFAULT '{}',
    due_date          TEXT,
    recurrence        TEXT NOT NULL DEFAULT 'none' CHECK (recurrence IN ('none', 'daily', 'weekly', 'monthly')),
    source_channel_id UUID REFERENCES channels(id) ON DELETE SET NULL,
    source_message_id UUID REFERENCES channel_messages(id) ON DELETE SET NULL,
    created_by        UUID REFERENCES users(id) ON DELETE SET NULL,
    completed_at      TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_task_server_status_idx ON enterprise_tasks (server_id, status, assignee_id);
CREATE INDEX IF NOT EXISTS ent_task_server_due_idx ON enterprise_tasks (server_id, due_date);

CREATE TABLE IF NOT EXISTS enterprise_task_comments (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id        UUID NOT NULL REFERENCES enterprise_tasks(id) ON DELETE CASCADE,
    author_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content        TEXT NOT NULL CHECK (char_length(content) BETWEEN 1 AND 2000),
    attachment_url TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_task_comment_idx ON enterprise_task_comments (task_id, created_at);

CREATE TABLE IF NOT EXISTS enterprise_task_activity (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id    UUID NOT NULL REFERENCES enterprise_tasks(id) ON DELETE CASCADE,
    actor_id   UUID REFERENCES users(id) ON DELETE SET NULL,
    action     TEXT NOT NULL,
    details    TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_task_act_idx ON enterprise_task_activity (task_id, created_at DESC);

CREATE TABLE IF NOT EXISTS enterprise_meetings (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id         UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    department_id     UUID REFERENCES enterprise_departments(id) ON DELETE SET NULL,
    project_id        UUID REFERENCES enterprise_projects(id) ON DELETE SET NULL,
    channel_id        UUID REFERENCES channels(id) ON DELETE SET NULL,
    title             TEXT NOT NULL CHECK (char_length(title) BETWEEN 1 AND 120),
    description       TEXT CHECK (description IS NULL OR char_length(description) <= 1500),
    meeting_type      TEXT NOT NULL DEFAULT 'video' CHECK (meeting_type IN ('audio', 'video', 'standup', 'review', 'all_hands')),
    host_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    co_host_ids       UUID[] NOT NULL DEFAULT '{}',
    participant_ids   UUID[] NOT NULL DEFAULT '{}',
    guest_emails      TEXT[] NOT NULL DEFAULT '{}',
    waiting_room      BOOLEAN NOT NULL DEFAULT FALSE,
    recording_allowed BOOLEAN NOT NULL DEFAULT FALSE,
    starts_at         TIMESTAMPTZ NOT NULL,
    ends_at           TIMESTAMPTZ NOT NULL,
    timezone          TEXT NOT NULL DEFAULT 'Asia/Ho_Chi_Minh',
    recurrence        TEXT NOT NULL DEFAULT 'none' CHECK (recurrence IN ('none', 'daily', 'weekly', 'monthly')),
    notes             TEXT NOT NULL DEFAULT '',
    minutes           TEXT NOT NULL DEFAULT '',
    status            TEXT NOT NULL DEFAULT 'scheduled' CHECK (status IN ('scheduled', 'live', 'ended', 'cancelled')),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (ends_at > starts_at)
);
CREATE INDEX IF NOT EXISTS ent_meeting_server_time_idx ON enterprise_meetings (server_id, starts_at);

CREATE TABLE IF NOT EXISTS enterprise_attendance (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id        UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id          UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    work_date        TEXT NOT NULL,
    check_in_at      TIMESTAMPTZ,
    check_out_at     TIMESTAMPTZ,
    work_mode        TEXT NOT NULL DEFAULT 'onsite' CHECK (work_mode IN ('onsite', 'remote', 'hybrid')),
    status           TEXT NOT NULL DEFAULT 'present' CHECK (status IN ('present', 'late', 'early_leave', 'remote', 'overtime')),
    late_minutes     INT NOT NULL DEFAULT 0,
    early_minutes    INT NOT NULL DEFAULT 0,
    overtime_minutes INT NOT NULL DEFAULT 0,
    note             TEXT,
    verified_by      UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (server_id, user_id, work_date)
);
CREATE INDEX IF NOT EXISTS ent_att_server_date_idx ON enterprise_attendance (server_id, work_date DESC, user_id);

CREATE TABLE IF NOT EXISTS enterprise_attendance_adjustments (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id    UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    work_date    TEXT NOT NULL,
    requested_in TIMESTAMPTZ,
    requested_out TIMESTAMPTZ,
    reason       TEXT NOT NULL CHECK (char_length(reason) BETWEEN 2 AND 500),
    status       TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected')),
    reviewer_id  UUID REFERENCES users(id) ON DELETE SET NULL,
    reviewed_at  TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_att_adj_srv_idx ON enterprise_attendance_adjustments (server_id, status, created_at DESC);

CREATE TABLE IF NOT EXISTS enterprise_leave_balances (
    server_id  UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    year       INT NOT NULL,
    leave_type TEXT NOT NULL,
    total_days DOUBLE PRECISION NOT NULL DEFAULT 12.0,
    used_days  DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    PRIMARY KEY (server_id, user_id, year, leave_type)
);

CREATE TABLE IF NOT EXISTS enterprise_leave_requests (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id     UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    leave_type    TEXT NOT NULL CHECK (char_length(leave_type) BETWEEN 1 AND 32),
    start_date    TEXT NOT NULL,
    end_date      TEXT NOT NULL,
    half_day      TEXT NOT NULL DEFAULT 'full' CHECK (half_day IN ('full', 'morning', 'afternoon')),
    days_count    DOUBLE PRECISION NOT NULL CHECK (days_count > 0),
    reason        TEXT NOT NULL CHECK (char_length(reason) BETWEEN 1 AND 500),
    status        TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected', 'withdrawn')),
    approver_id   UUID REFERENCES users(id) ON DELETE SET NULL,
    decision_note TEXT,
    decided_at    TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_leave_srv_idx ON enterprise_leave_requests (server_id, status, created_at DESC);

CREATE TABLE IF NOT EXISTS enterprise_form_templates (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id      UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name           TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 80),
    category       TEXT NOT NULL CHECK (category IN ('leave', 'purchase', 'payment', 'equipment', 'it_support', 'travel', 'attendance_fix', 'project_proposal', 'access_request', 'custom')),
    description    TEXT CHECK (description IS NULL OR char_length(description) <= 300),
    fields         JSONB NOT NULL DEFAULT '[]',
    workflow_steps JSONB NOT NULL DEFAULT '[]',
    created_by     UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_form_tpl_srv_idx ON enterprise_form_templates (server_id);

CREATE TABLE IF NOT EXISTS enterprise_approval_requests (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    template_id     UUID REFERENCES enterprise_form_templates(id) ON DELETE SET NULL,
    title           TEXT NOT NULL CHECK (char_length(title) BETWEEN 1 AND 120),
    category        TEXT NOT NULL,
    requester_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    department_id   UUID REFERENCES enterprise_departments(id) ON DELETE SET NULL,
    form_data       JSONB NOT NULL DEFAULT '{}',
    attachment_urls TEXT[] NOT NULL DEFAULT '{}',
    status          TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected', 'needs_revision', 'withdrawn')),
    current_step    INT NOT NULL DEFAULT 0,
    total_steps     INT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_app_req_srv_idx ON enterprise_approval_requests (server_id, status, created_at DESC);

CREATE TABLE IF NOT EXISTS enterprise_approval_decisions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    request_id  UUID NOT NULL REFERENCES enterprise_approval_requests(id) ON DELETE CASCADE,
    step_index  INT NOT NULL,
    approver_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    decision    TEXT NOT NULL CHECK (decision IN ('approved', 'rejected', 'revision', 'withdrawn')),
    comment     TEXT,
    decided_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_app_dec_req_idx ON enterprise_approval_decisions (request_id, decided_at);

CREATE TABLE IF NOT EXISTS enterprise_folders (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id     UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    parent_id     UUID REFERENCES enterprise_folders(id) ON DELETE CASCADE,
    scope         TEXT NOT NULL DEFAULT 'company' CHECK (scope IN ('company', 'department', 'project', 'personal')),
    department_id UUID REFERENCES enterprise_departments(id) ON DELETE SET NULL,
    project_id    UUID REFERENCES enterprise_projects(id) ON DELETE SET NULL,
    owner_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name          TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 80),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_folder_srv_idx ON enterprise_folders (server_id, scope, parent_id);

CREATE TABLE IF NOT EXISTS enterprise_documents (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id     UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    folder_id     UUID REFERENCES enterprise_folders(id) ON DELETE SET NULL,
    scope         TEXT NOT NULL DEFAULT 'company' CHECK (scope IN ('company', 'department', 'project', 'personal')),
    department_id UUID REFERENCES enterprise_departments(id) ON DELETE SET NULL,
    project_id    UUID REFERENCES enterprise_projects(id) ON DELETE SET NULL,
    owner_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title         TEXT NOT NULL CHECK (char_length(title) BETWEEN 1 AND 120),
    file_url      TEXT NOT NULL CHECK (file_url LIKE '/uploads/%' AND position('..' IN file_url) = 0),
    file_name     TEXT NOT NULL,
    file_size     BIGINT NOT NULL DEFAULT 0,
    mime_type     TEXT NOT NULL DEFAULT 'application/octet-stream',
    pinned        BOOLEAN NOT NULL DEFAULT FALSE,
    starred_by    UUID[] NOT NULL DEFAULT '{}',
    trashed_at    TIMESTAMPTZ,
    version       INT NOT NULL DEFAULT 1,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_doc_srv_idx ON enterprise_documents (server_id, scope, trashed_at, updated_at DESC);

CREATE TABLE IF NOT EXISTS enterprise_document_versions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES enterprise_documents(id) ON DELETE CASCADE,
    version     INT NOT NULL,
    file_url    TEXT NOT NULL,
    file_size   BIGINT NOT NULL DEFAULT 0,
    uploaded_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS enterprise_document_shares (
    token       TEXT PRIMARY KEY CHECK (char_length(token) BETWEEN 12 AND 64),
    server_id   UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    document_id UUID NOT NULL REFERENCES enterprise_documents(id) ON DELETE CASCADE,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_by  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS enterprise_announcements (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id       UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    scope           TEXT NOT NULL DEFAULT 'company' CHECK (scope IN ('company', 'department', 'project')),
    department_id   UUID REFERENCES enterprise_departments(id) ON DELETE SET NULL,
    project_id      UUID REFERENCES enterprise_projects(id) ON DELETE SET NULL,
    author_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title           TEXT NOT NULL CHECK (char_length(title) BETWEEN 1 AND 140),
    content         TEXT NOT NULL CHECK (char_length(content) BETWEEN 1 AND 4000),
    priority        TEXT NOT NULL DEFAULT 'normal' CHECK (priority IN ('normal', 'important', 'urgent')),
    pinned          BOOLEAN NOT NULL DEFAULT FALSE,
    scheduled_at    TIMESTAMPTZ,
    attachment_urls TEXT[] NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_ann_srv_idx ON enterprise_announcements (server_id, pinned DESC, created_at DESC);

CREATE TABLE IF NOT EXISTS enterprise_announcement_reads (
    announcement_id UUID NOT NULL REFERENCES enterprise_announcements(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    read_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (announcement_id, user_id)
);

CREATE TABLE IF NOT EXISTS enterprise_integrations (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id      UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    provider       TEXT NOT NULL CHECK (provider IN ('webhook', 'github', 'gitlab', 'jira', 'google_calendar', 'outlook', 'google_drive', 'onedrive', 'rest_api', 'crm', 'hrm', 'erp')),
    name           TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 80),
    status         TEXT NOT NULL DEFAULT 'unconfigured' CHECK (status IN ('configured', 'unconfigured', 'error')),
    endpoint_url   TEXT,
    has_secret     BOOLEAN NOT NULL DEFAULT FALSE,
    secret_hash    TEXT,
    events         TEXT[] NOT NULL DEFAULT '{}',
    config_meta    JSONB NOT NULL DEFAULT '{}',
    last_tested_at TIMESTAMPTZ,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_integ_srv_idx ON enterprise_integrations (server_id);

CREATE TABLE IF NOT EXISTS enterprise_workflows (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id     UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name          TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 80),
    trigger_event TEXT NOT NULL CHECK (trigger_event IN ('task_assigned', 'task_completed', 'leave_decided', 'approval_decided', 'member_joined', 'announcement_posted')),
    action_type   TEXT NOT NULL CHECK (action_type IN ('post_channel_message', 'notify_user', 'webhook')),
    action_config JSONB NOT NULL DEFAULT '{}',
    enabled       BOOLEAN NOT NULL DEFAULT TRUE,
    run_count     BIGINT NOT NULL DEFAULT 0,
    last_run_at   TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_wf_srv_idx ON enterprise_workflows (server_id, enabled);

CREATE TABLE IF NOT EXISTS enterprise_workflow_runs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id UUID NOT NULL REFERENCES enterprise_workflows(id) ON DELETE CASCADE,
    server_id   UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    status      TEXT NOT NULL CHECK (status IN ('ok', 'skipped', 'error')),
    detail      TEXT NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_wf_run_idx ON enterprise_workflow_runs (server_id, created_at DESC);
