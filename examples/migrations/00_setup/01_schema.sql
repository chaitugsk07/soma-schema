-- 00_setup/01_schema.sql
-- Idempotent project bootstrap. Runs UNCONDITIONALLY before every up(), untracked.
-- Every statement MUST be idempotent: use IF NOT EXISTS, CREATE OR REPLACE, etc.
-- This is the chicken-before-the-egg: it runs before the tracking table is created.

CREATE SCHEMA IF NOT EXISTS example;

CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- A shared updated_at trigger function (CREATE OR REPLACE is idempotent).
CREATE OR REPLACE FUNCTION example.fn_update_timestamp()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END;
$$;
