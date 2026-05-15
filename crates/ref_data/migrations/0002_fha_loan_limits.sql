-- Migration 0002: FHA single-family loan limits by county and year.
-- Source: HUD Mortgagee Letter (annual, effective January 1).
-- Primary key: (fips_code, effective_year) — one row per county per year.

CREATE TABLE IF NOT EXISTS fha_loan_limits (
    fips_code       TEXT        NOT NULL,
    state_abbr      TEXT        NOT NULL,
    county_name     TEXT        NOT NULL,
    limit_type      TEXT        NOT NULL,  -- 'Floor' | 'Standard' | 'HighCost'
    limit_1_unit    BIGINT      NOT NULL,  -- Cents (integer, 1/100 of a dollar)
    limit_2_unit    BIGINT      NOT NULL,
    limit_3_unit    BIGINT      NOT NULL,
    limit_4_unit    BIGINT      NOT NULL,
    effective_year  SMALLINT    NOT NULL,
    PRIMARY KEY (fips_code, effective_year)
);

CREATE INDEX IF NOT EXISTS idx_fha_limits_fips
    ON fha_loan_limits (fips_code);
