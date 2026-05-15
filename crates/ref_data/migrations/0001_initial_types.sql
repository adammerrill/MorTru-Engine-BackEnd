-- Migration 0001: PostgreSQL ENUM types shared across ref_data tables.
-- SQLite: these are TEXT columns; enum constraint enforced at the Rust layer.
-- Run once; idempotent via DO/EXCEPTION block.

DO $$ BEGIN
  CREATE TYPE fha_limit_type AS ENUM ('Floor', 'Standard', 'HighCost');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
  CREATE TYPE program_code AS ENUM (
    'conventional', 'home_ready', 'home_possible', 'home_one',
    'fha', 'fha_dpa', 'va', 'va_jumbo', 'usda', 'bond', 'jumbo', 'non_qm'
  );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
  CREATE TYPE cbsa_designation AS ENUM ('Metropolitan', 'Micropolitan', 'Rural');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;
