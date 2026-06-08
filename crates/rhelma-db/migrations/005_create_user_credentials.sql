-- Create credentials table for password-based auth.
-- This keeps user profile data (users table) separate from secrets.

CREATE TABLE IF NOT EXISTS user_credentials (
    tenant_id VARCHAR(256) NOT NULL,
    user_id UUID NOT NULL,
    password_hash TEXT NOT NULL,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ,
    PRIMARY KEY (tenant_id, user_id),
    CONSTRAINT fk_user_credentials_user
        FOREIGN KEY (tenant_id, user_id)
        REFERENCES users(tenant_id, id)
        ON DELETE CASCADE
);

-- Fast lookups by user.
CREATE INDEX IF NOT EXISTS idx_user_credentials_user_id
    ON user_credentials(user_id);
