CREATE TABLE users (
  tenant_id VARCHAR(256) NOT NULL,
  id UUID PRIMARY KEY,
  email TEXT NOT NULL,
  name TEXT NOT NULL,
  roles JSONB DEFAULT '[]',
  created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
  UNIQUE (tenant_id, email),
  -- Required for composite foreign keys (tenant-scoped references).
  -- Example: user_credentials references users(tenant_id, id).
  UNIQUE (tenant_id, id)
);

CREATE INDEX idx_users_tenant_id ON users(tenant_id);
CREATE INDEX idx_users_tenant_email ON users(tenant_id, email);
