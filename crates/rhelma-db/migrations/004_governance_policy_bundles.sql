-- 004_governance_policy_bundles.sql
-- Governance schema additions (policy bundles + succession records).

CREATE TABLE IF NOT EXISTS policy_bundles (
  bundle_id TEXT PRIMARY KEY,
  version TEXT NOT NULL,
  hash TEXT NOT NULL,
  quorum_signatures JSONB NOT NULL DEFAULT '[]',
  issuer TEXT,
  issued_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMP WITH TIME ZONE,
  payload JSONB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_policy_bundles_issued_at ON policy_bundles(issued_at DESC);
CREATE INDEX IF NOT EXISTS idx_policy_bundles_version ON policy_bundles(version);

CREATE TABLE IF NOT EXISTS succession_records (
  creator_id TEXT NOT NULL,
  record_version TEXT NOT NULL,
  successor_id TEXT NOT NULL,
  signature TEXT NOT NULL,
  activated BOOLEAN NOT NULL DEFAULT false,
  created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
  activated_at TIMESTAMP WITH TIME ZONE,
  PRIMARY KEY (creator_id, record_version)
);

CREATE INDEX IF NOT EXISTS idx_succession_records_creator ON succession_records(creator_id);
CREATE INDEX IF NOT EXISTS idx_succession_records_activated ON succession_records(activated);
