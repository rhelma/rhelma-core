# MNI Approved Training Streams v1

## Purpose
Approved Training Streams (ATS) define how training samples may be selected for distillation.
ATS are the bridge between the Data Constitution and future training jobs.

## ATS Policy Object (canonical JSON)
- `stream_id` (string)
- `version` (u32)
- `name` (string)
- `owner` (string, optional)
- `tiers_allowed` (array of strings): `public_commons`, `consent_based`
- `sensitive_gate` (object):
  - `allow` (bool)
  - `require_jury_approval` (bool)
- `content_filters` (object):
  - `min_reputation` (u32, optional)
  - `include_tags` (array, optional)
  - `exclude_tags` (array, optional)
  - `max_age_days` (u32, optional)
- `sampling` (object):
  - `max_samples` (u32)
  - `strategy` (string): `recent`, `balanced`, `random`
- `created_at_unix` (i64)
- `updated_at_unix` (i64)

## Governance
- ATS create/update requires admin token and is audit-logged.
- ATS versions are append-only; updates create a new version.
