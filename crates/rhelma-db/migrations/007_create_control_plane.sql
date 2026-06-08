-- 007_create_control_plane.sql
-- Control Plane (realms + nodes + routes + heartbeats)
-- NOTE: Requires pgcrypto (enabled in 0001_init.sql).

CREATE TABLE IF NOT EXISTS control_realms (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  slug text NOT NULL UNIQUE,
  name text NOT NULL,
  status text NOT NULL DEFAULT 'active',
  metadata jsonb NOT NULL DEFAULT '{}'::jsonb,

  owner_tenant_id text,
  owner_user_id uuid,

  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),

  CONSTRAINT control_realms_owner_fk
    FOREIGN KEY (owner_tenant_id, owner_user_id)
    REFERENCES users(tenant_id, id)
    ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS control_nodes (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  realm_id uuid NULL REFERENCES control_realms(id) ON DELETE SET NULL,

  name text NOT NULL,
  region text NOT NULL,

  public_base_url text,
  internal_base_url text,

  status text NOT NULL DEFAULT 'offline',
  last_seen_at timestamptz,

  capabilities jsonb NOT NULL DEFAULT '{}'::jsonb,
  version text,

  api_key_hash text,
  api_key_hint text,

  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS control_nodes_unique_name_region
  ON control_nodes(name, region);

CREATE TABLE IF NOT EXISTS control_node_heartbeats (
  node_id uuid NOT NULL REFERENCES control_nodes(id) ON DELETE CASCADE,
  ts timestamptz NOT NULL DEFAULT now(),
  cpu_percent real,
  mem_percent real,
  disk_percent real,
  rps real,
  checks jsonb NOT NULL DEFAULT '{}'::jsonb,
  PRIMARY KEY (node_id, ts)
);

CREATE INDEX IF NOT EXISTS control_node_heartbeats_ts_idx
  ON control_node_heartbeats(ts DESC);

CREATE TABLE IF NOT EXISTS control_realm_routes (
  realm_id uuid NOT NULL REFERENCES control_realms(id) ON DELETE CASCADE,
  service text NOT NULL,
  node_id uuid NOT NULL REFERENCES control_nodes(id) ON DELETE CASCADE,
  weight int NOT NULL DEFAULT 100,
  status text NOT NULL DEFAULT 'active',
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (realm_id, service, node_id)
);

CREATE INDEX IF NOT EXISTS control_realm_routes_service_idx
  ON control_realm_routes(service);

CREATE TABLE IF NOT EXISTS control_realm_members (
  realm_id uuid NOT NULL REFERENCES control_realms(id) ON DELETE CASCADE,
  tenant_id text NOT NULL,
  user_id uuid NOT NULL,
  role text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (realm_id, tenant_id, user_id),
  CONSTRAINT control_realm_members_user_fk
    FOREIGN KEY (tenant_id, user_id)
    REFERENCES users(tenant_id, id)
    ON DELETE CASCADE
);
