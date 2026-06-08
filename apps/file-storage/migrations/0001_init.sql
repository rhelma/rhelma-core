-- file-storage-service schema (v5.2)
--
-- Notes:
-- - IDs are UUID.
-- - Soft delete: status='deleted' with deleted_at.

CREATE TABLE IF NOT EXISTS files (
    id UUID PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    region TEXT NOT NULL,
    original_name TEXT NOT NULL,
    content_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    checksum TEXT NOT NULL,
    storage_backend TEXT NOT NULL,
    storage_path TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    created_by TEXT,
    deleted_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_files_tenant_id ON files (tenant_id);
CREATE INDEX IF NOT EXISTS idx_files_status ON files (status);
CREATE INDEX IF NOT EXISTS idx_files_created_at ON files (created_at);

CREATE TABLE IF NOT EXISTS file_audit_log (
    id BIGSERIAL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    file_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    actor TEXT,
    details TEXT,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_file_id ON file_audit_log (file_id);
CREATE INDEX IF NOT EXISTS idx_audit_created_at ON file_audit_log (created_at);
