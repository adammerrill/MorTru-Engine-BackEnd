-- Migration 0004: USDA rural eligibility by census tract GEOID.
-- Source: USDA RD / Census TIGER (updated with each decennial Census + interim runs).
-- pct_eligible: percentage of tract area outside all USDA-ineligible polygons.

CREATE TABLE IF NOT EXISTS usda_rural_eligibility (
    geoid            TEXT              NOT NULL PRIMARY KEY,  -- 11-digit census tract
    fips_code        TEXT              NOT NULL,              -- 5-digit county FIPS
    state_abbr       TEXT              NOT NULL,
    is_sfh_eligible  BOOLEAN           NOT NULL,
    is_mfh_eligible  BOOLEAN           NOT NULL,
    pct_eligible     DOUBLE PRECISION,
    source_version   TEXT              NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_usda_rural_fips
    ON usda_rural_eligibility (fips_code);
