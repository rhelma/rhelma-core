-- 0001_init.sql
-- Minimal bootstrap for Rhelma's shared schema migrations.
--
-- Why this is here:
-- - Other migrations in this directory rely on `gen_random_uuid()`.
-- - Postgres provides it via the `pgcrypto` extension.
--
-- This statement is idempotent and safe to run in any environment.

CREATE EXTENSION IF NOT EXISTS pgcrypto;
