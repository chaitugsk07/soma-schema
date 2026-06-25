-- 00_setup/01_schema.sql
-- Idempotent bootstrap. Runs unconditionally before every up(), untracked.
CREATE SCHEMA IF NOT EXISTS vault;
CREATE EXTENSION IF NOT EXISTS pgcrypto;
